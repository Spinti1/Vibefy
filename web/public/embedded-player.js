'use strict';
// Official visible YouTube player. No hidden audio-only iframe or challenge bypass.
window.vibefyEmbedded = (() => {
  let player, ready, generation=0;
  const api = {
    active: false,
    onUpdate: () => {}, onEnded: () => {}, onError: () => {},
    get paused(){return !player || player.getPlayerState?.()!==1;},
    get currentTime(){return player?.getCurrentTime?.()||0;},
    set currentTime(value){player?.seekTo?.(value,true);},
    get duration(){return player?.getDuration?.()||0;},
    pause(){player?.pauseVideo?.();},
    async play(){await ready;player?.playVideo?.();},
    async load(id, paused=false, position=0){
      const ticket=++generation;api.active=true;
      if(!ready)ready=new Promise((resolve,reject)=>{
        const timeout=setTimeout(()=>reject(new Error('The official player could not load.')),15000);
        window.onYouTubeIframeAPIReady=()=>{
          player=new YT.Player('youtube-player',{host:'https://www.youtube-nocookie.com',width:'100%',height:'100%',playerVars:{playsinline:1,rel:0,origin:location.origin},events:{
            onReady:()=>{clearTimeout(timeout);resolve();},
            onStateChange:e=>{api.onUpdate();if(e.data===0)api.onEnded();},
            onError:()=>api.onError('This recording cannot be embedded. Try a different recording.')
          }});
        };
        const script=document.createElement('script');script.src='https://www.youtube.com/iframe_api';script.onerror=()=>{clearTimeout(timeout);reject(new Error('The official player is unavailable.'));};document.head.append(script);
      });
      await ready;if(ticket===generation){const item={videoId:id,startSeconds:Math.max(0,position)};if(paused||!document.getElementById('now-dialog').open||document.hidden)player.cueVideoById(item);else player.loadVideoById(item);}
    }
  };
  // Do not emulate background YouTube playback when its visible player is closed.
  document.addEventListener('visibilitychange',()=>{if(document.hidden&&api.active)api.pause();});
  return api;
})();
