'use strict';
// The shared room lives within the normal library/search interface.
(() => {
 const $=id=>document.getElementById(id),room=$('jam-room'),source=$('now-dialog');
 let active=false,lastQueue='',lastPeers='',chosen=null;
 const volumeNodes=['volume-toggle','volume'].map(id=>({node:$(id),parent:$(id).parentNode,next:$(id).nextSibling}));
 const label=(tag,value,cls)=>{const n=document.createElement(tag);n.textContent=value;if(cls)n.className=cls;return n;};
 function activate(value){
  if(active===value)return;active=value;lastQueue='';lastPeers='';document.body.classList.toggle('jam-active',value);room.hidden=!value;
  if(source.open)source.close();
  (value?$('jam-source-slot'):document.body).append(source);
  $('jam-volume-controls').hidden=!value;
  for(const part of volumeNodes){if(value)$('jam-volume-inputs').append(part.node);else part.parent.insertBefore(part.node,part.next);}
  if(!value){document.body.classList.remove('jam-guest');if(document.fullscreenElement===room)document.exitFullscreen?.().catch(()=>{});}
  if(value){$('jam-dialog').close();if(window.vibefyPlayerBridge.snapshot().track)source.show();}
  else if($('jam-song-dialog').open)$('jam-song-dialog').close();
 }
 function update(packet){
  if(!active)return;
  const jam=window.vibefyJam,t=packet.track;
  document.body.classList.toggle('jam-guest',!jam.host);
  $('jam-room-role').textContent=jam.host?'You’re hosting':'Listening together';
  const people=Array.isArray(packet.participants)?packet.participants.length:(Number(packet.listeners)||0)+(packet.host_present===false?0:1);
  $('jam-room-count').textContent=people+(people===1?' person':' people');
  $('jam-room-status').textContent=packet.host_present===false?'Waiting for the host to reconnect…':packet.snapshot?.playing?'Playing together':t?'Host paused':'Choose a song or add one to the queue';
  $('jam-room-title').textContent=t?.title||'Your shared session';
  $('jam-room-artist').textContent=t?.artist||'Search music above. Songs you choose join this queue.';
  const artwork=window.vibefyWebLibrary.art(t?.image);if($('jam-room-cover').getAttribute('src')!==artwork)$('jam-room-cover').src=artwork;
  const peers=Array.isArray(packet.participants)?packet.participants.slice(0,25).sort((a,b)=>Number(b.role==='host')-Number(a.role==='host')):[];const peersKey=JSON.stringify(peers);if(peersKey!==lastPeers){lastPeers=peersKey;
  const roster=$('jam-people');roster.replaceChildren();
  for(const [i,p]of peers.entries()){
   const item=label('span','', 'jam-person'),name=String(p.name||'Listener').slice(0,60);
   item.append(label('span',p.role==='host'?'H':String(i+1),'jam-avatar'),label('span',name+(p.role==='host'?' · host':'')));roster.append(item);
  }
  }
  const next=Array.isArray(packet.queue)?packet.queue:[],key=JSON.stringify(next);
  if(!jam.host)$('next').disabled=!jam.sharedControls||!next.length;
  if(key===lastQueue)return;lastQueue=key;
  const list=$('jam-shared-tracks');list.replaceChildren();
  if(!next.length)list.append(label('p','The queue is empty. Search for a song to add it.','fine-print'));
  next.forEach((track,i)=>{const b=document.createElement('button');b.className='jam-queue-track';b.disabled=!jam.sharedControls;b.setAttribute('aria-label','Play '+track.title+' for everyone');const image=document.createElement('img');image.src=window.vibefyWebLibrary.art(track.image);image.alt='';image.loading='lazy';const details=label('span','');details.append(label('strong',track.title),label('small',track.artist));b.append(label('span',String(i+1),'jam-track-number'),image,details);b.onclick=()=>{if(!jam.playQueue(i))window.vibefyWebLibrary.toast('The queue changed. Try again.');};list.append(b);});
 }
 $('jam-room-leave').onclick=()=>window.vibefyJam.leave();
 $('jam-room-expand').hidden=typeof room.requestFullscreen!=='function';
 $('jam-room-expand').onclick=async()=>{try{if(document.fullscreenElement===room)await document.exitFullscreen();else await room.requestFullscreen();}catch{window.vibefyWebLibrary.toast('Fullscreen is unavailable in this browser.');}};
 document.addEventListener('fullscreenchange',()=>{const expanded=document.fullscreenElement===room;$('jam-room-expand').setAttribute('aria-label',expanded?'Exit Jam fullscreen':'Fullscreen Jam');});
 $('jam-room-invite').onclick=async()=>{try{const invite=location.origin+'/web?jam='+window.vibefyJam.code;await navigator.clipboard.writeText(invite);window.vibefyWebLibrary.toast('Jam invitation copied.');}catch{document.querySelector('[data-dialog="jam"]').click();}};
 $('jam-song-add').onclick=()=>{if(!chosen)return;const jam=window.vibefyJam;if(jam.host)window.vibefyPlayerBridge.addQueue(chosen);else if(!jam.add(chosen))window.vibefyWebLibrary.toast('This source cannot be queued right now.');$('jam-song-dialog').close();};
 window.vibefyJamRoom={activate,update,songOptions:track=>{chosen=track;$('jam-song-title').textContent=track.title;$('jam-song-artist').textContent=track.artist;$('jam-song-dialog').showModal();}};
})();
