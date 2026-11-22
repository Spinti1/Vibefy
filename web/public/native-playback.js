'use strict';
// Android Media3 owns audio, notification controls and queue advancement.
(() => {
 if(!window.vibefyNativeInfo?.backgroundAudio||!window.VibefyMobile?.postMessage)return;
 const listeners=new EventTarget();let state={paused:true,position:0,duration:0,ready:false},source='',expectedTrack='',generation=0,signature='',volume=.8;
 function send(command,fields={}){window.VibefyMobile.postMessage(JSON.stringify({action:'nativeMedia',command,...fields}));}
 const emit=type=>listeners.dispatchEvent(new Event(type));
 const supported=t=>t&&/^(?:[A-Za-z0-9_-]{11}|jv:[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12})$/.test(t.id||'');
 const api={
  addEventListener:(...args)=>listeners.addEventListener(...args),
  get paused(){return state.paused;},get currentTime(){return state.position||0;},
  set currentTime(value){if(!Number.isFinite(value))return;state.position=Math.max(0,Math.min(86400,value));send('seek',{position:Math.round(state.position*1000)});},
  get duration(){return state.duration||0;},get readyState(){return state.ready?4:1;},get error(){return state.error||null;},
  getAttribute:key=>key==='src'?source:null,
  removeAttribute:key=>{if(key==='src'){source='';signature='';send('clear');}},load(){},
  play(){state.paused=false;send('play');emit('playing');return Promise.resolve();},
  pause(){state.paused=true;send('pause');emit('pause');},
  get volume(){return volume;},set volume(value){if(Number.isFinite(value)){volume=Math.max(0,Math.min(1,value));send('volume',{value:volume});}},
  expectTrack:id=>{expectedTrack=id;},
  loadQueue(tracks,index,paused,position,shuffle,repeat){
   if(!tracks.length||tracks.length>10000||!tracks.every(supported))throw Error('This queue contains a source unavailable to native audio.');
   source=tracks[index].id;expectedTrack='';signature=JSON.stringify(tracks.map(t=>t.id));state={id:source,index,position,duration:tracks[index].duration||0,paused,ready:false};generation++;
   send('queue',{tracks,index,paused,position:Math.round(position*1000),shuffle,repeat,generation,autoNext:!window.vibefyJam?.following&&window.vibefyPreferences?.autoNext!==false});emit(paused?'pause':'waiting');
  },
  syncQueue(tracks,index,shuffle,repeat){if(!source||signature===JSON.stringify(tracks.map(t=>t.id)))return;if(tracks.length&&tracks.every(supported))api.loadQueue(tracks,index,state.paused,state.position,shuffle,repeat);},
  modes:(shuffle,repeat)=>send('modes',{shuffle,repeat}),
  next:()=>send('next'),previous:()=>send('previous'),resync:()=>send('state'),
 };
 window.vibefyNativePlayback=api;
 window.vibefyNativePlaybackState=next=>{
  if(!next||(next.generation<generation&&!(next.generation===0&&next.error&&!next.id)))return;
  if(next.id&&expectedTrack&&next.id!==expectedTrack)return;if(next.id===expectedTrack)expectedTrack='';
  const old=state;state={...state,...next};if(next.id)source=next.id;
  if(next.error){if(old.error!==next.error)emit('error');return;}
  document.dispatchEvent(new CustomEvent('vibefy-native-track',{detail:state}));
  if(state.ready&&(!old.ready||old.id!==state.id||old.duration!==state.duration))emit('loadedmetadata');
  if(old.paused!==state.paused||old.playing!==state.playing)emit(state.paused?'pause':state.playing?'playing':'waiting');
  // The service advances itself, including when JavaScript is suspended.
  if(next.ended&&!old.ended)emit('pause');
  emit('timeupdate');
 };
 send('state');
})();
