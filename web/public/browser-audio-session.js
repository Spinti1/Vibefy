'use strict';
// Configure Safari's playback category before the first media start. An
// embedded provider still owns its own playback and background restrictions.
(() => {
  if (window.vibefyNativePlayback) return;
  const audio = document.getElementById('audio');
  if (!audio) return;
  const configurePlayback = () => {
    try {
      if (navigator.audioSession) navigator.audioSession.type = 'playback';
    } catch { /* Unsupported session settings must never interrupt playback. */ }
  };
  configurePlayback();
  audio.addEventListener('play', configurePlayback);
})();
