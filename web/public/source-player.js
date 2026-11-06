'use strict';
// Public SoundCloud playback uses its official visible widget, never stream extraction.
window.vibefySoundCloud=(()=>{
 let widget,position=0,duration=0,paused=true,generation=0,ready;
 const api={active:false,onUpdate:()=>{},onEnded:()=>{},get paused(){return paused;},get currentTime(){return position;},set currentTime(v){widget?.seekTo(Math.max(0,v)*1000);},get duration(){return duration;},pause(){widget?.pause();},async play(){widget?.play();},async load(id){
  if(!/^sc:\d{1,20}$/.test(id))throw Error('Invalid SoundCloud track.');const ticket=++generation;api.active=true;
  if(!ready)ready=new Promise((resolve,reject)=>{const script=document.createElement('script');script.src='https://w.soundcloud.com/player/api.js';script.onload=resolve;script.onerror=()=>reject(Error('SoundCloud player could not load.'));document.head.append(script);});await ready;if(ticket!==generation)return;
  let frame=document.getElementById('soundcloud-frame');if(!frame){frame=document.createElement('iframe');frame.id='soundcloud-frame';frame.className='source-playback';frame.title='Official SoundCloud player';frame.allow='autoplay';document.getElementById('youtube-frame').after(frame);}frame.hidden=false;
  const target='https://api.soundcloud.com/tracks/'+id.slice(3);frame.src='https://w.soundcloud.com/player/?url='+encodeURIComponent(target)+'&auto_play=false';widget=SC.Widget(frame);
  widget.bind(SC.Widget.Events.READY,()=>{widget.getDuration(v=>{duration=v/1000;api.onUpdate();});widget.play();});widget.bind(SC.Widget.Events.PLAY,()=>{paused=false;api.onUpdate();});widget.bind(SC.Widget.Events.PAUSE,()=>{paused=true;api.onUpdate();});widget.bind(SC.Widget.Events.PLAY_PROGRESS,e=>{position=e.currentPosition/1000;api.onUpdate();});widget.bind(SC.Widget.Events.FINISH,()=>api.onEnded());
 }};document.addEventListener('visibilitychange',()=>{if(document.hidden&&api.active)api.pause();});return api;
})();
