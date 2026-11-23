'use strict';
// Official visible YouTube player. No hidden audio-only iframe or challenge bypass.
window.vibefyEmbedded = (() => {
  let player, ready, playable=false, queue, generation=0, volume=80;

  function prepare() {
    if (ready) return ready;
    ready = new Promise((resolve, reject) => {
      const timeout = setTimeout(() => reject(new Error('The official player could not load.')), 15000);
      const createPlayer = () => {
        player = new YT.Player('youtube-player', {
          host: 'https://www.youtube-nocookie.com', width: '100%', height: '100%',
          playerVars: {playsinline: 1, rel: 0, origin: location.origin},
          events: {
            onReady: ({target}) => {
              target.setVolume(volume);
              playable = true;
              clearTimeout(timeout);
              resolve();
            },
            onAutoplayBlocked: () => { if (api.active) api.onBlocked(); },
            onStateChange: ({data}) => {
              if (!api.active) return;
              updateQueue(data);
              api.onUpdate();
              if (data === 0 && (!queue || (queue.nextOutside && player.getPlaylistIndex?.() === queue.ids.length - 1))) api.onEnded();
            },
            onError: ({data}) => {
              if (!api.active) return;
              api.onError(data === 153 ? 'YouTube could not verify this browser player. Open the site directly in a standard browser.' : 'This recording cannot be embedded here. Try another recording or open it on YouTube.');
            }
          }
        });
      };
      if (window.YT?.Player) {
        createPlayer();
        return;
      }
      window.onYouTubeIframeAPIReady = createPlayer;
      const script = document.createElement('script');
      script.src = 'https://www.youtube.com/iframe_api';
      script.onerror = () => {
        clearTimeout(timeout);
        script.remove?.();
        reject(new Error('The official player is unavailable.'));
      };
      document.head.append(script);
    }).catch(error => {
      // A failed warm-up must not poison the next explicit attempt.
      if (!player) ready = undefined;
      throw error;
    });
    return ready;
  }

  function updateQueue(state) {
    if (!queue || (state !== 1 && state !== 5)) return;
    const position = player.getPlaylistIndex?.();
    const ids = player.getPlaylist?.();
    if (!Number.isInteger(position) || ids?.length !== queue.ids.length ||
        ids[position] !== queue.ids[position] || queue.indices[position] === undefined) return;
    api.onTrack(queue.indices[position]);
  }

  const api = {
    active: false,
    prepare,
    onUpdate: () => {}, onEnded: () => {}, onError: () => {}, onBlocked: () => {}, onTrack: () => {},
    get paused(){return !player || player.getPlayerState?.()!==1;},
    get currentTime(){return player?.getCurrentTime?.()||0;},
    set currentTime(value){player?.seekTo?.(value,true);},
    get duration(){return player?.getDuration?.()||0;},
    setVolume(value){if(!Number.isFinite(value))return;volume=Math.round(Math.max(0,Math.min(100,value)));player?.setVolume?.(volume);if(volume>0)player?.unMute?.();},
    pause(){player?.pauseVideo?.();},
    async play(){
      if (!playable) await prepare();
      player?.playVideo?.();
    },
    get queueOrder(){return api.active ? queue?.indices : undefined;},
    setRepeat(value){
      if(api.active && queue)player?.setLoop?.(value);
    },
    move(delta){
      if (!api.active || !queue || !player) return false;
      const index=player.getPlaylistIndex?.();
      if ((delta<0 && index===0 && queue.previousOutside) ||
          (delta>0 && index===queue.ids.length-1 && queue.nextOutside)) return false;
      if (delta > 0) player.nextVideo();
      else player.previousVideo();
      return true;
    },
    loadQueue(id, paused, position, fromMediaSession, order){
      return api.load(id, paused, position, fromMediaSession, order);
    },
    async load(id, paused=false, position=0, fromMediaSession=false, order){
      const ticket=++generation;api.active=true;queue=undefined;
      // Keep a tap's activation when the player has already been prepared.
      if (!playable) await prepare();
      if (ticket!==generation || !api.active) return;
      const startSeconds=Math.max(0,position);
      const cue=paused || (!window.vibefyPhone && !window.matchMedia?.('(max-width:900px)').matches &&
        !document.getElementById('now-dialog').open) || (document.hidden && !fromMediaSession);
      if (order?.ids.length > 1) {
        queue=order;
        player.setLoop(order.repeat);
        if (cue) player.cuePlaylist(order.ids,order.start,startSeconds);
        else player.loadPlaylist(order.ids,order.start,startSeconds);
      } else {
        const item={videoId:id,startSeconds};
        if (cue) player.cueVideoById(item);
        else player.loadVideoById(item);
      }
    }
  };
  // Provider/browser background restrictions remain; Vibefy does not force a pause on tab visibility changes.
  return api;
})();
