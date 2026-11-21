'use strict';
(() => {
 // The web host can read public Spotify playlists only. A Spotify tab's login
 // is never read or copied; native phone apps deliberately omit these controls.
 if(window.vibefyMobileShell)return;
 const bridge=window.vibefyWebLibrary;if(!bridge)return;
 const idOf=c=>/^spotify:([A-Za-z0-9]{22})$/.exec(c?.id||'')?.[1];
 const key='vibefy-spotify-follow-v1',period=15*60*1000,jobs=new Map();let prefs={};
 try{prefs=JSON.parse(localStorage.getItem(key)||'{}');if(!prefs||typeof prefs!=='object'||Array.isArray(prefs))prefs={};}catch{}
 const make=(tag,text)=>{const n=document.createElement(tag);if(text)n.textContent=text;return n;};
 const tools=make('div');tools.className='spotify-sync-tools';tools.hidden=true;
 const button=make('button','Sync Spotify playlist');button.className='button small';
 const label=make('label');label.className='spotify-follow-option';const follow=make('input');follow.type='checkbox';label.append(follow,document.createTextNode(' Check for new songs automatically'));
 const status=make('p','Public Spotify playlists only · Adds new songs; keeps your local songs.');status.className='fine-print';status.setAttribute('role','status');tools.append(button,label,status);document.getElementById('collection-subtitle').after(tools);
 const persist=()=>{try{localStorage.setItem(key,JSON.stringify(prefs));}catch{}};
 function update(){const c=bridge.selected(),id=idOf(c);tools.hidden=!id;if(id){follow.checked=prefs[id]?.enabled===true;button.disabled=jobs.has(id);button.textContent=jobs.has(id)?'Syncing Spotify…':'Sync Spotify playlist';}}
 async function sync(c,automatic=false){
  const id=idOf(c);if(!id||c.shared?.follow||jobs.has(id))return;
  const old=prefs[id]||{},now=Date.now();if(now<(Number(old.next)||0)){if(!automatic)status.textContent='Spotify sync is cooling down. Your local playlist is safe; try again later.';return;}
  jobs.set(id,true);update();const say=text=>{if(idOf(bridge.selected())===id)status.textContent=text;};
  const checked=async response=>{if(response.ok)return response.json();const retry=Number(response.headers?.get('Retry-After'));if(response.status===429)prefs[id]={...prefs[id],next:Date.now()+Math.min(86400,Math.max(60,Number.isFinite(retry)?retry:900))*1000};throw Error(response.status===429?'Spotify is limiting requests. Sync will wait before trying again.':'Spotify could not be read. The playlist must be public. Your local playlist is unchanged.');};
  try{
   let offset=0;const remote=[],offsets=new Set();
   do{
    if(offsets.has(offset)||!Number.isInteger(offset)||offset<0||offset>20000)throw Error('Spotify returned incomplete playlist pages. Your local playlist is unchanged.');offsets.add(offset);
    say('Reading Spotify playlist · '+remote.length+' songs');
    const data=await checked(await fetch('/api/import/spotify?id='+id+'&offset='+offset,{signal:AbortSignal.timeout(30000)}));
    if(!Array.isArray(data.tracks)||data.tracks.length>100||!('next' in data))throw Error('Spotify returned an incomplete playlist. Try again later.');
    remote.push(...data.tracks);if(remote.length>10000)throw Error('This playlist exceeds the browser library limit. Your local playlist is unchanged.');offset=data.next;
   }while(offset!==null);
   const known=new Set(c.tracks.map(t=>t.spotifyId).filter(Boolean)),seen=new Set(),additions=[];
   let unmatched=0;
   for(const song of remote){
    const spotifyId=/^spotify:([A-Za-z0-9]{22})$/.exec(song.id||'')?.[1];if(!spotifyId)throw Error('Spotify returned a recording without its identifier. Your local playlist is unchanged.');if(known.has(spotifyId)||seen.has(spotifyId))continue;seen.add(spotifyId);
    say('Matching new songs · '+seen.size);
    const data=await checked(await fetch('/api/match',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(song),signal:AbortSignal.timeout(30000)}));
    if(data.track&&/^[A-Za-z0-9_-]{11}$/.test(data.track.id||''))additions.push({...data.track,spotifyId});else unmatched++;
   }
   // Re-read after network waits so editing/removing a playlist cannot be undone.
   const latest=bridge.collections().find(p=>p.id===c.id);if(!latest)return;
   const tracks=latest.tracks.map(t=>({...t})),byVideo=new Map(tracks.map(t=>[t.id,t]));let count=0;
   for(const track of additions){const present=byVideo.get(track.id);if(present){if(!present.spotifyId)present.spotifyId=track.spotifyId;continue;}if(tracks.length>=10000)throw Error('This playlist is full. Your local playlist is unchanged.');tracks.push(track);byVideo.set(track.id,track);count++;}
   bridge.save({...latest,tracks});prefs[id]={...prefs[id],checked:Date.now(),next:Date.now()+10000};persist();
   say((count?'Added '+count+' new songs from Spotify.':'Spotify playlist is up to date.')+(unmatched?' '+unmatched+' songs could not be matched; they will be retried on the next sync.':''));
   if(!automatic)bridge.toast(count?'Added '+count+' Spotify songs':'Spotify playlist is up to date');
  }catch(error){prefs[id]={...prefs[id],next:Math.max(Number(prefs[id]?.next)||0,Date.now()+60000)};persist();say(error.name==='TimeoutError'?'Spotify sync timed out. Your local playlist is unchanged.':error.message);}
  finally{jobs.delete(id);update();}
 }
 button.onclick=()=>sync(bridge.selected());follow.onchange=()=>{const id=idOf(bridge.selected());if(!id)return;prefs[id]={...prefs[id],enabled:follow.checked};persist();status.textContent=follow.checked?'Checks every 15 minutes while this browser is open. Spotify limits are respected.':'Automatic checks are off. Use Sync Spotify playlist when needed.';};
 function tick(){if(document.visibilityState!=='visible'||jobs.size)return;const now=Date.now();const c=bridge.collections().find(c=>{const p=prefs[idOf(c)];return p?.enabled===true&&now>=Math.max(Number(p.next)||0,(Number(p.checked)||0)+period);});if(c)sync(c,true);}
 document.addEventListener('vibefy-collection-open',update);document.addEventListener('visibilitychange',()=>{if(document.visibilityState==='visible')tick();});setInterval(tick,60000);update();
})();
