//! Small, opt-in playlist snapshots. Owner keys remain in the OS-protected store.
use anyhow::{Result,ensure};
use serde::{Serialize,Deserialize};
use sha2::{Sha256,Digest};
use crate::{addons::Collection,personal_library::ContextData};
const HOST:&str="https://vibefy-jam.pex-am.workers.dev/shares/";
#[derive(Clone,Serialize,Deserialize)]
struct Binding {id:String,owner:String,fingerprint:String,updated:u64}
#[derive(Clone)]
pub struct Panel {pub source:String,pub title:String,pub url:String,pub bytes:usize,pub qr:Vec<bool>,pub qr_width:usize}
fn key(source:&str)->String{format!("playlist-share-{:x}",Sha256::digest(source.as_bytes()))}
fn payload(c:&Collection)->Result<serde_json::Value>{
 ensure!(c.tracks.len()<=2000,"QR sharing supports up to 2,000 songs per playlist.");
 let mut tracks=Vec::new();
 for t in &c.tracks {
  let id=if let Some(id)=t.uri.strip_prefix("vibefy:youtube:").or_else(||t.uri.strip_prefix("youtube:track:")){id.to_owned()}
  else if let Some(id)=t.uri.strip_prefix("spotify:track:"){format!("spotify:{id}")}
  else if let Some(id)=t.uri.strip_prefix("vibefy:juicevault:"){format!("jv:{id}")}
  else if let Some(url)=t.uri.strip_prefix("vibefy:soundcloud:"){format!("scurl:{url}")}
  else if let Some(url)=crate::addons::direct_url(&t.uri){let u=reqwest::Url::parse(&url)?;if u.host_str()==Some("youtu.be"){u.path().trim_start_matches('/').to_owned()}else if let Some((_,id))=u.query_pairs().find(|(k,_)|k=="v"){id.into_owned()}else{anyhow::bail!("This playlist contains a source that cannot be shared to mobile yet.")}}
  else{anyhow::bail!("This playlist contains a source that cannot be shared to mobile yet.")};
  tracks.push(serde_json::json!({"id":id,"title":t.title,"artist":t.artist_names(),"duration":t.duration_ms as f64/1000.0,"image":t.cover_url.as_deref().or(t.art_url.as_deref())}));
 }
 let value=serde_json::json!({"title":c.title,"tracks":tracks});ensure!(serde_json::to_vec(&value)?.len()<=384*1024,"Playlist metadata exceeds the 384 KiB sharing limit.");Ok(value)
}
async fn binding(c:&Collection,context:&ContextData)->Result<Option<Binding>>{
 let bytes=crate::music_accounts::protected_named(&context.config,key(&c.source),None).await?;
 bytes.map(|b|serde_json::from_slice(&b).map_err(Into::into)).transpose()
}
pub async fn publish(c:Collection,create:bool,revoke:bool,context:ContextData)->Result<crate::personal_library::Output>{
 let existing=binding(&c,&context).await?;
 if revoke {
  if let Some(b)=existing {let response=context.http.client().map_err(anyhow::Error::msg)?.post(format!("{HOST}{}/revoke",b.id)).bearer_auth(&b.owner).timeout(std::time::Duration::from_secs(15)).send().await?;ensure!(response.status().is_success(),"Cannot disable sharing right now. Retry shortly.");crate::music_accounts::protected_named(&context.config,key(&c.source),Some(None)).await?;}
  return Ok(crate::personal_library::Output::PhoneShared(None));
 }
 if existing.is_none()&&!create{return Ok(crate::personal_library::Output::PhoneShared(None));}
 let mut b=existing.unwrap_or_else(||{let owner=format!("{:032x}{:032x}",rand::random::<u128>(),rand::random::<u128>());let id=format!("{:x}",Sha256::digest(owner.as_bytes()))[..32].to_owned();Binding{id,owner,fingerprint:String::new(),updated:0}});
 let value=payload(&c)?;let bytes=serde_json::to_vec(&value)?;let fingerprint=format!("{:x}",Sha256::digest(&bytes));let now=crate::playlist_tools::now();
 if b.fingerprint!=fingerprint||now.saturating_sub(b.updated)>7*86400 {
  // Save the owner key before sending, so an interrupted response cannot orphan a live link.
  crate::music_accounts::protected_named(&context.config,key(&c.source),Some(Some(serde_json::to_vec(&b)?))).await?;
  let response=context.http.client().map_err(anyhow::Error::msg)?.put(format!("{HOST}{}",b.id)).bearer_auth(&b.owner).json(&value).timeout(std::time::Duration::from_secs(20)).send().await?;
  ensure!(response.status().is_success(),"Playlist sharing could not update (HTTP {}). Your local playlist is safe.",response.status().as_u16());
  b.fingerprint=fingerprint;b.updated=now;
  crate::music_accounts::protected_named(&context.config,key(&c.source),Some(Some(serde_json::to_vec(&b)?))).await?;
 }
 let url=format!("https://vibefy-neon.vercel.app/web#share={}",b.id);let qr=qrcode::QrCode::new(url.as_bytes())?;let qr_width=qr.width();let modules=qr.into_colors().into_iter().map(|v|v==qrcode::Color::Dark).collect();
 Ok(crate::personal_library::Output::PhoneShared(Some(Panel{source:c.source,title:c.title,url,bytes:bytes.len(),qr:modules,qr_width})))
}
pub async fn sync(collections:Vec<Collection>,context:ContextData)->Result<crate::personal_library::Output>{
 for c in collections {if binding(&c,&context).await?.is_some(){let child=ContextData{progress:Box::new(|_|{}),root:context.root.clone(),config:context.config.clone(),proxy:context.proxy.clone(),http:context.http.clone()};publish(c,false,false,child).await?;}}
 Ok(crate::personal_library::Output::PhoneSynced)
}
#[cfg(test)]mod tests{use super::*;#[test]fn no_account_or_audio_fields_in_snapshot(){let value=payload(&Collection{title:"Demo".into(),..Default::default()}).unwrap();assert_eq!(value.as_object().unwrap().len(),2);assert!(key("private/source").starts_with("playlist-share-"));assert!(!key("private/source").contains("private"));}}
