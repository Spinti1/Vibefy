export const SHARE_ID=/^[a-f0-9]{32}$/;
export const SHARE_TOKEN=/^[a-f0-9]{64}$/;
export const MAX_BYTES=384*1024, MAX_TRACKS=2000, RETENTION=90*86400000;
export function cleanPlaylist(value){
 if(!value||typeof value.title!=='string'||!value.title.trim()||value.title.length>200||!Array.isArray(value.tracks)||value.tracks.length>MAX_TRACKS)throw Error('Invalid playlist or more than 2,000 songs.');
 const tracks=value.tracks.map(t=>{
  const sc=t?.id?.match(/^scurl:https:\/\/soundcloud\.com\/[A-Za-z0-9_%~-]+\/[A-Za-z0-9_%~-]+$/);
  if(!t||(!sc&&! /^(?:[A-Za-z0-9_-]{11}|spotify:[A-Za-z0-9]{22}|sc:\d{1,20}|jv:[0-9a-fA-F-]{36})$/.test(t.id||''))||String(t.id||'').length>512||typeof t.title!=='string'||t.title.length>300||typeof t.artist!=='string'||t.artist.length>300)throw Error('Invalid song metadata.');
  let image='';try{const u=new URL(t.image);if(u.protocol==='https:'&&!u.username&&!u.password&&!u.port&&u.href.length<=600&&(['i.ytimg.com','i.scdn.co','mosaic.scdn.co','api.juicevault.xyz'].includes(u.hostname)||['.googleusercontent.com','.ggpht.com','.sndcdn.com'].some(s=>u.hostname.endsWith(s))))image=u.href;}catch{}
  return {id:t.id,title:t.title,artist:t.artist,duration:Number.isFinite(t.duration)?Math.max(0,Math.min(86400,t.duration)):0,...(image?{image}:{})};
 });
 const result={title:value.title.trim(),tracks};if(new TextEncoder().encode(JSON.stringify(result)).length>MAX_BYTES)throw Error('Playlist metadata exceeds 384 KiB.');return result;
}
const json=(data,status=200,headers={})=>Response.json(data,{status,headers:{'Cache-Control':'no-store',...headers}});
export async function shareRequest(ctx,env,request){
 const url=new URL(request.url),storage=ctx.storage;
 if(url.pathname==='/reserve'||url.pathname==='/release'){
  const input=await request.json();if(!SHARE_ID.test(input.id))return json({error:'Invalid ID'},400);
  const now=Date.now(),entries=await storage.get('index')||{};if(url.pathname==='/release'){delete entries[input.id];await storage.put('index',entries);return json({ok:true});}for(const [id,expiry]of Object.entries(entries))if(expiry<now)delete entries[id];
  if(!entries[input.id]&&Object.keys(entries).length>=1000)return json({error:'Sharing capacity reached. Try again later.'},503);
  entries[input.id]=now+RETENTION;await storage.put('index',entries);return json({ok:true});
 }
 let meta=await storage.get('meta');
 if(request.method==='GET'){
  if(!meta||meta.revoked||meta.expires<Date.now())return json({error:'This share expired or was disabled. Your saved copy is unchanged.'},404);
  const etag='"'+meta.revision+'"';if(request.headers.get('If-None-Match')===etag)return new Response(null,{status:304,headers:{ETag:etag,'Cache-Control':'no-cache'}});
  let body='';for(let i=0;i<meta.parts;i++)body+=await storage.get('part'+i)||'';
  return new Response(body,{headers:{'Content-Type':'application/json','Cache-Control':'no-cache',ETag:etag}});
 }
 const token=request.headers.get('Authorization')?.replace(/^Bearer /,'');if(!SHARE_TOKEN.test(token||''))return json({error:'Owner authorization required'},401);
 if(!meta){const digest=await crypto.subtle.digest('SHA-256',new TextEncoder().encode(token));const expected=Array.from(new Uint8Array(digest),x=>x.toString(16).padStart(2,'0')).join('').slice(0,32);if(url.pathname.split('/')[2]!==expected)return json({error:'Invalid owner identity'},403);}
 if(meta&&meta.owner!==token)return json({error:'Owner authorization required'},403);
 if(request.method==='POST'&&url.pathname.endsWith('/revoke')){if(meta){await storage.deleteAll();await storage.put('meta',{...meta,revoked:true,parts:0});await env.SHARES.get(env.SHARES.idFromName('__registry')).fetch(new Request('https://share/release',{method:'POST',body:JSON.stringify({id:url.pathname.split('/')[2]})}));}return json({revoked:true});}
 if(meta?.revoked)return json({error:'This share was disabled. Create a new share.'},410);
 if(request.method!=='PUT')return json({error:'Not found'},404);
 let value;try{if(Number(request.headers.get('Content-Length')||0)>MAX_BYTES)return json({error:'Metadata too large'},413);const reader=request.body?.getReader();let size=0,raw='';const decoder=new TextDecoder();if(!reader)return json({error:'Invalid metadata'},400);while(true){const chunk=await reader.read();if(chunk.done)break;size+=chunk.value.byteLength;if(size>MAX_BYTES){await reader.cancel();return json({error:'Metadata too large'},413);}raw+=decoder.decode(chunk.value,{stream:true});}raw+=decoder.decode();value=cleanPlaylist(JSON.parse(raw));}catch(e){return json({error:e.message},400);}
 const registry=env.SHARES.get(env.SHARES.idFromName('__registry'));
 const reserved=await registry.fetch(new Request('https://share/reserve',{method:'POST',body:JSON.stringify({id:url.pathname.split('/')[2]})}));if(!reserved.ok)return reserved;
 const bytes=new TextEncoder().encode(JSON.stringify(value)),hash=await crypto.subtle.digest('SHA-256',bytes),revision=Array.from(new Uint8Array(hash),x=>x.toString(16).padStart(2,'0')).join('');
 // UTF-16 chunks <= 20,000 chars remain below the storage per-value byte limit.
 const body=JSON.stringify({...value,revision}),parts=Math.ceil(body.length/20000),entries={};for(let i=0;i<parts;i++)entries['part'+i]=body.slice(i*20000,(i+1)*20000);
 const expires=Date.now()+RETENTION;entries.meta={owner:token,revision,parts,expires};await storage.put(entries);for(let i=parts;i<(meta?.parts||0);i++)await storage.delete('part'+i);await storage.setAlarm(expires);
 return json({revision,expires,bytes:bytes.length});
}
