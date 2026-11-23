'use strict';
(()=>{
 const api='https://vibefy-jam.pex-am.workers.dev/shares/',idPattern=/^[a-f0-9]{32}$/,bridge=window.vibefyWebLibrary;
 const $=id=>document.getElementById(id),node=(tag,text)=>{const n=document.createElement(tag);if(text)n.textContent=text;return n;};
 const jobs=new Map(),checked=new Map();let incoming=null,preview=null;
 const dialog=node('dialog');dialog.id='shared-playlist-dialog';const header=node('div');header.className='dialog-header';header.append(node('h2','Shared playlist'));const close=node('button','×');close.className='icon-button';close.setAttribute('aria-label','Close shared playlist');close.onclick=()=>dialog.close();header.append(close);dialog.append(header);
 const cover=node('img');cover.className='collection-art';cover.style.cssText='width:112px;height:112px;border-radius:14px;object-fit:cover';cover.alt='';cover.onerror=()=>{cover.onerror=null;cover.src='/assets/icon.png';};const title=node('h3'),details=node('p'),status=node('p');status.setAttribute('role','status');details.className=status.className='fine-print';
 const label=node('label'),follow=node('input');follow.type='checkbox';follow.checked=true;label.append(follow,document.createTextNode(' Keep updated from desktop'));
 const add=node('button','Add playlist');add.className='button primary';add.disabled=true;const native=node('a','Open in Vibefy');native.className='button small';native.hidden=!!window.vibefyMobileShell;
 dialog.append(cover,title,details,label,status,add,native);document.body.append(dialog);
 async function read(id,revision){const r=await fetch(api+id,{headers:revision?{'If-None-Match':'"'+revision+'"'}:{},signal:AbortSignal.timeout(15000)});if(r.status===304)return null;if(!r.ok)throw Error(r.status===404?'Sharing was disabled or expired. Your saved copy is safe.':'The playlist could not update. Your saved copy is safe.');const raw=await r.text();if(new TextEncoder().encode(raw).length>400*1024)throw Error('This shared playlist is too large.');const data=JSON.parse(raw);if(typeof data.title!=='string'||!Array.isArray(data.tracks)||data.tracks.length>2000||!/^[a-f0-9]{64}$/.test(data.revision||''))throw Error('Invalid playlist.');return data;}
 function save(id,data,existing){
  bridge.save({...existing,id:'shared:'+id,title:data.title,tracks:data.tracks,created:existing?.created??Date.now(),shared:{id,revision:data.revision,follow:follow.checked}});
  checked.set(id,Date.now());
  dialog.close();
  bridge.open('shared:'+id);
  bridge.toast('Playlist saved');
 }
 async function invite(scanned){
  const hashId=new URLSearchParams(location.hash.slice(1)).get('share');
  const pathId=!location.hash&&location.pathname?.match(/^\/share\/([a-f0-9]{32})\/?$/)?.[1];
  const id=typeof scanned==='string'?scanned:hashId||pathId;
  if(!idPattern.test(id||''))return;
  const request={id};incoming=request;preview=null;add.disabled=true;
  title.textContent='Loading playlist…';status.textContent='';details.textContent='';
  const existing=bridge.collections().find(c=>c.id==='shared:'+id);
  const nativeAndroid=window.vibefyMobileShell&&window.vibefyNativeInfo?.platform==='android';
  if(nativeAndroid&&existing){
   dialog.close();
   bridge.open(existing.id);
   return;
  }
  follow.checked=existing?.shared?.follow??true;
  const android=/Android/i.test(window.navigator?.userAgent||'');
  native.href=android?'intent://playlist?id='+id+'#Intent;scheme=vibefy;package=online.usevibefy.mobile;S.browser_fallback_url='+encodeURIComponent(location.origin+'/web#share='+id)+';end':'vibefy://playlist?id='+id;
  if(!dialog.open)dialog.showModal();
  try{
   const data=await read(id);
   if(incoming!==request||!dialog.open)return;
   preview=data;
   if(nativeAndroid){
    save(id,data);
    return;
   }
   title.textContent=data.title;details.textContent=data.tracks.length+(data.tracks.length===1?' song':' songs');
   cover.src=bridge.art(data.tracks.find(t=>t.image)?.image);
   add.textContent=existing?'Update saved playlist':'Add playlist';add.disabled=false;
  }catch(e){if(incoming===request)status.textContent=e.message;}
 }
 add.onclick=()=>{if(!preview||!incoming)return;const id=incoming.id;try{save(id,preview,bridge.collections().find(c=>c.id==='shared:'+id));}catch(e){status.textContent=e.message;}};
 const tools=node('div');tools.className='setting-actions';const toggle=node('button'),refresh=node('button','Check for changes'),note=node('span');toggle.className=refresh.className='button small';note.className='fine-print';tools.append(toggle,refresh,note);$('collection-subtitle').after(tools);tools.hidden=true;
 async function sync(c,force=false){const info=c?.shared;if(!info?.follow||jobs.has(info.id)||!force&&Date.now()-(checked.get(info.id)||0)<60000)return;jobs.set(info.id,true);checked.set(info.id,Date.now());try{const data=await read(info.id,info.revision);if(data){const latest=bridge.collections().find(p=>p.id===c.id);if(!latest?.shared?.follow)return;bridge.save({...latest,title:data.title,tracks:data.tracks,shared:{...latest.shared,revision:data.revision}});note.textContent='Updated from desktop';}else note.textContent='Up to date';}catch(e){note.textContent=e.message;}finally{jobs.delete(info.id);}}
 function selected(){const c=bridge.selected();tools.hidden=!c?.shared;if(c?.shared){toggle.textContent=c.shared.follow?'Keep updated: On':'Keep updated: Off';refresh.disabled=!c.shared.follow;note.textContent='';sync(c);}}
 toggle.onclick=()=>{const c=bridge.selected();if(!c?.shared)return;c.shared.follow=!c.shared.follow;bridge.save(c);selected();};refresh.onclick=()=>sync(bridge.selected(),true);
 document.addEventListener('vibefy-collection-open',selected);document.addEventListener('visibilitychange',()=>{if(document.visibilityState==='visible')selected();});window.vibefyPlaylistShares={open:id=>{if(idPattern.test(id||''))return invite(id);}};window.addEventListener('hashchange',invite);invite();
})();
