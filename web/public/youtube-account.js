'use strict';
// Native clients keep unexpired grants in protected platform storage; browsers use memory.
(() => {
 const $=id=>document.getElementById(id),readScope='https://www.googleapis.com/auth/youtube.readonly',writeScope='https://www.googleapis.com/auth/youtube.force-ssl',identityScope='openid profile';
 let client,token='',expires=0,writable=false,pending,expiryTimer,loading=false,initializing,displayName='',generation=0,grantedScopes='',restored=false,restoreAttempt;
 let sessionWrites=Promise.resolve(),sessionSaveFailed=false,sessionReadFailed=false;
 const storage=()=>window.vibefyMobileSession;
 function persist(){if(!storage()||!connected())return Promise.resolve();const record={version:1,access_token:token,expires_at:expires,scope:grantedScopes,display_name:displayName};sessionWrites=sessionWrites.catch(()=>{}).then(()=>storage().write(record)).then(()=>{sessionSaveFailed=false;},error=>{sessionSaveFailed=true;throw error;});return sessionWrites;}
 function clearStored(){if(storage())sessionWrites=sessionWrites.catch(()=>{}).then(()=>storage().clear()).catch(()=>{});}
 function applyGrant(record){if(!record||record.version!==1||typeof record.access_token!=='string'||record.access_token.length<1||record.access_token.length>8192||typeof record.expires_at!=='number'||record.expires_at<=Date.now()+5000||record.expires_at>Date.now()+86400000||!String(record.scope||'').split(' ').includes(readScope))return false;token=record.access_token;expires=record.expires_at;grantedScopes=String(record.scope);writable=grantedScopes.split(' ').includes(writeScope);displayName=String(record.display_name||'').slice(0,256);armExpiry();return true;}
 function armExpiry(){clearTimeout(expiryTimer);expiryTimer=setTimeout(()=>{clearStored();update();status('Google access needs renewal. Connect again to refresh your YouTube library. Your playlists are preserved.');},Math.max(0,expires-Date.now()));}
 function restore(){if(!storage()||restored)return Promise.resolve();if(restoreAttempt)return restoreAttempt;const ticket=generation;restoreAttempt=(async()=>{let record;try{record=await storage().read();}catch{sessionReadFailed=true;status('Saved account could not be read. Your playlists are safe; try restarting Vibefy.');return;}restored=true;if(ticket!==generation||pending||connected())return;if(!applyGrant(record)){if(record)clearStored();return;}update();try{await refresh();}catch(error){status(error.message);}})().finally(()=>{restoreAttempt=null;});return restoreAttempt;}

 const connected=()=>!!token&&Date.now()<expires;
 const status=message=>{$('youtube-status').textContent=message;};
 function update(){
  const active=connected(),compact=window.vibefyPhone||window.matchMedia?.('(max-width:900px)').matches;
  for(const name of ['header']){const button=$(name+'-connect');button.disabled=loading||(!client&&!active);button.classList.toggle('connected',active);button.setAttribute('aria-label',active?'YouTube connected. Open account':loading?'Connecting YouTube':'Connect YouTube');$(name+'-connect-label').textContent=loading?'Connecting…':active?(compact?'Connected':'YouTube connected'):(compact?'Sign in':'Connect YouTube');}
  $('google-connect').disabled=loading||!client;$('google-connect').hidden=active;$('google-connect-label').textContent=loading?'Connecting…':'Continue to Google';
  $('google-disconnect').hidden=!active;$('youtube-refresh').hidden=!active;$('youtube-refresh').disabled=loading;
  $('account-identity').textContent=active?(displayName||'YouTube connected'):'Not connected';
  $('home-account-status').textContent=loading?'Finish connecting in the Google window…':active?'YouTube connected · Your local playlists and backups stay available.':'Saved on this device. Use the top bar to connect YouTube and load your playlists.';
  window.dispatchEvent(new Event('vibefy-account'));
 }
 function disconnect(){generation++;clearStored();clearTimeout(expiryTimer);token='';expires=0;writable=false;displayName='';grantedScopes='';$('youtube-playlists').replaceChildren();status('Disconnected. Your imported and local playlists are preserved.');update();}
 function finishJob(error,response,job=pending){
  if(!job||pending!==job)return;pending=null;clearTimeout(job.timer);
  if(error){window.vibefyMobileCancelAuth?.();job.reject(error);return;}
  if(!response?.access_token||response.error){job.reject(Error('Google access was not granted. You can still use Vibefy as a guest.'));return;}
  if(!String(response.scope||'').split(' ').includes(readScope)){job.reject(Error('YouTube playlist permission was not granted. Your local library is unchanged.'));return;}
  grantedScopes=String(response.scope||'');token=response.access_token;expires=Date.now()+Math.max(0,Number(response.expires_in||3600)-60)*1000;writable=String(response.scope||'').split(' ').includes(writeScope);
  armExpiry();persist().catch(()=>status('Connected for this session. Secure storage could not save the account; try connecting again.'));
  if(job.write&&!writable)job.reject(Error('Permission to save YouTube playlists was not granted. Your local playlist is safe.'));else job.resolve(response);
 }
 function authorize(write=false){return new Promise((resolve,reject)=>{
  if(!client)return reject(Error('Google connection is still loading. Retry in a moment.'));if(pending)return reject(Error('Finish the open Google window first.'));
  const job={resolve,reject,write};pending=job;job.timer=setTimeout(()=>finishJob(Error('Google did not finish connecting. Close the popup and try again.'),null,job),90000);
  if(window.vibefyMobileAuthorize){clearTimeout(job.timer);job.timer=setTimeout(()=>finishJob(Error('Connection expired. Try again.'),null,job),310000);window.vibefyMobileAuthorize(write).then(r=>finishJob(null,r,job),e=>finishJob(e,null,job));return;}
  if(window.vibefyMobileShell||window.VibefyMobile||window.webkit?.messageHandlers?.vibefyMobile){finishJob(Error('The app connection could not initialize. Restart Vibefy or install the latest mobile update.'),null,job);return;}
  try{const oauth=google.accounts.oauth2.initTokenClient({client_id:client.client_id,scope:readScope,include_granted_scopes:false,callback:response=>finishJob(null,response,job),error_callback:()=>finishJob(Error('Google sign-in was closed or blocked. Allow the popup and try again.'),null,job)});oauth.requestAccessToken({scope:write?readScope+' '+writeScope:identityScope+' '+readScope,include_granted_scopes:false,prompt:''});}catch{finishJob(Error('Google could not open. Allow popups for Vibefy and try again.'),null,job);}
 });}
 async function api(path,query={},method='GET',body){
  if(!connected())throw Error('Connect YouTube again to renew your access.');const url=new URL('https://www.googleapis.com/youtube/v3/'+path);for(const [k,v]of Object.entries(query))url.searchParams.set(k,v);
  const r=await fetch(url,{method,headers:{Authorization:'Bearer '+token,...(body?{'Content-Type':'application/json'}:{})},body:body?JSON.stringify(body):undefined,signal:AbortSignal.timeout(15000)});
  if(r.status===401){disconnect();throw Error('Your Google session expired. Connect again.');}if(!r.ok)throw Error(r.status===403?'YouTube could not authorize this action. Check Google account access or try again later.':'YouTube could not complete the request. Try again.');return r.status===204?{}:r.json();
 }
 async function importPlaylist(p,button){
  const ticket=generation;button.disabled=true;status('Importing '+(p.snippet?.title||'playlist')+'…');
  try{const tracks=[];let page='';do{const data=await api('playlistItems',{part:'snippet,contentDetails',playlistId:p.id,maxResults:50,...(page?{pageToken:page}:{})});for(const i of data.items||[]){const s=i.snippet||{},id=i.contentDetails?.videoId;if(!/^[A-Za-z0-9_-]{11}$/.test(id)||['Private video','Deleted video'].includes(s.title))continue;tracks.push({id,title:s.title,artist:s.videoOwnerChannelTitle||'',album:'YouTube',duration:0,image:s.thumbnails?.medium?.url||''});}page=data.nextPageToken||'';}while(page&&tracks.length<10000);
   if(ticket!==generation||!connected())return;window.vibefyWebLibrary.save({id:'youtube:'+p.id,title:p.snippet?.title||'YouTube playlist',tracks,youtubeId:p.id});for(const box of [$('youtube-playlists')])for(const card of [...box.children])if(card.dataset.playlistId===p.id)card.remove();status('Saved on Home. '+tracks.length+' songs imported.');window.vibefyWebLibrary.open('youtube:'+p.id);window.vibefyWebLibrary.toast('Playlist ready · '+tracks.length+' songs');
  }catch(error){status(error.message);}finally{button.disabled=false;}
 }
 async function refresh(){
  const ticket=generation;status('Loading your YouTube playlists…');const playlists=[];let page='';do{const data=await api('playlists',{part:'snippet,contentDetails',mine:'true',maxResults:50,...(page?{pageToken:page}:{})});if(ticket!==generation||!connected())return;playlists.push(...(data.items||[]));page=data.nextPageToken||'';}while(page&&playlists.length<1000);
  for(const box of [$('youtube-playlists')]){box.replaceChildren();for(const p of playlists){if(window.vibefyWebLibrary.collections().some(c=>c.youtubeId===p.id||c.id==='youtube:'+p.id))continue;const b=document.createElement('button');b.className='collection-tile';b.type='button';b.dataset.playlistId=p.id;const img=new Image();img.src=window.vibefyWebLibrary.art(p.snippet?.thumbnails?.medium?.url);img.alt='';img.className='collection-art';img.loading='lazy';const title=document.createElement('strong');title.textContent=p.snippet?.title||'YouTube playlist';const note=document.createElement('small');note.textContent=(p.contentDetails?.itemCount||0)+' songs · Open playlist';b.append(img,title,note);b.onclick=()=>importPlaylist(p,b);box.append(b);}}
  status(playlists.length?playlists.length+' playlists found. Choose the ones to add to Home.':'No playlists found in this YouTube account. Your local library is available on Home.');
 }
 async function connect(){
  if(connected()||!client){if(!$('account-dialog').open)$('account-dialog').showModal();return;}if(loading)return;if(!$('account-dialog').open)$('account-dialog').showModal();loading=true;status('Finish connecting in the Google window…');update();
  try{const response=await authorize();if(String(response.scope||'').split(' ').includes('openid')){try{const r=await fetch('https://openidconnect.googleapis.com/v1/userinfo',{headers:{Authorization:'Bearer '+token},signal:AbortSignal.timeout(10000)});if(r.ok){const profile=await r.json();displayName=String(profile.name||'YouTube connected');}}catch{ /* Playlist access remains usable without profile metadata. */ }}await persist().catch(()=>{});update();await refresh();$('account-dialog').close();window.dispatchEvent(new Event('vibefy-youtube-ready'));}catch(error){status(error.message);}finally{loading=false;update();if(sessionSaveFailed)status('Connected, but secure storage could not save the account. Reopening may require connecting again.');}
 }
 $('google-connect').onclick=connect;
 // Show the testing and permission explanation before opening Google's consent window.
 $('header-connect').onclick=()=>{if(!$('account-dialog').open)$('account-dialog').showModal();};
 $('account-dialog').addEventListener('close',()=>{if(pending&&!connected())finishJob(Error('Connection cancelled. Connect again when you are ready.'));});
 $('google-disconnect').onclick=()=>{if(token&&window.google?.accounts?.oauth2)google.accounts.oauth2.revoke(token,()=>{});disconnect();};
 $('youtube-refresh').onclick=async()=>{if(!connected())return connect();if(loading)return;loading=true;update();try{await refresh();}catch(error){status(error.message);}finally{loading=false;update();}};
 window.vibefyYouTube={forget:disconnect,get connected(){return connected();},connect,
  async save(collection){if(!writable||!connected())await authorize(true);const created=await api('playlists',{part:'snippet,status'},'POST',{snippet:{title:collection.title,description:'Created with Vibefy'},status:{privacyStatus:'unlisted'}});collection.youtubeId=created.id;window.vibefyWebLibrary.save(collection);for(const t of collection.tracks)await this.add(collection.youtubeId,t);return created.id;},
  async add(id,track){if(!/^[A-Za-z0-9_-]{11}$/.test(track.id))return;if(!writable||!connected())await authorize(true);await api('playlistItems',{part:'snippet'},'POST',{snippet:{playlistId:id,resourceId:{kind:'youtube#video',videoId:track.id}}});},
  async rename(id,title){if(!writable||!connected())await authorize(true);const data=await api('playlists',{part:'snippet',id});const p=data.items?.[0];if(!p)throw Error('This playlist is unavailable.');await api('playlists',{part:'snippet'},'PUT',{id,snippet:{title,description:p.snippet.description||'',...(p.snippet.defaultLanguage?{defaultLanguage:p.snippet.defaultLanguage}:{})}});}
 };
 function unavailable(){if(connected()){update();$('header-connect').disabled=false;status('YouTube connected. Your library could not refresh; check your connection.');return;}status('Google connection could not load. Retry below, or continue with local playlists.');for(const id of ['header-connect'])$(id).disabled=false;$('google-connect').hidden=false;$('google-connect').disabled=false;$('google-connect-label').textContent='Retry connection';$('google-connect').onclick=()=>initialize();}
 function initialize(){
  if(initializing)return initializing;
  const restoreTask=restore();
  initializing=(async()=>{ $('google-connect').disabled=true;status('Preparing Google connection…');const r=await fetch('/api/google-config',{signal:AbortSignal.timeout(10000)});if(!r.ok)throw Error();const data=await r.json();if(!data.clientId)throw Error('Google connection is unavailable.');
   if(!window.vibefyMobileAuthorize&&!window.google?.accounts?.oauth2)await new Promise((resolve,reject)=>{const script=document.createElement('script');script.src='https://accounts.google.com/gsi/client';const timer=setTimeout(()=>{script.remove();reject(Error('Google connection timed out.'));},10000);script.onload=()=>{clearTimeout(timer);resolve();};script.onerror=()=>{clearTimeout(timer);script.remove();reject(Error('Google connection could not load.'));};document.head.append(script);});
   client={client_id:data.clientId};$('google-connect').onclick=connect;update();status('');await restoreTask;if(sessionReadFailed)status('Saved account could not be read. Your playlists are safe; try restarting Vibefy.');
  })().catch(async()=>{await restoreTask;unavailable();}).finally(()=>{initializing=null;});return initializing;
 }
 update();initialize();
})();
