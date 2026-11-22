'use strict';
// Direct HTML audio can use Safari's media playback session. An embedded
// provider still owns its own playback and background restrictions.
(() => {
  if (window.vibefyNativePlayback) return;
  const audio = document.getElementById('audio');
  if (!audio) return;
  audio.addEventListener('play', () => {
    try {
      if (navigator.audioSession) navigator.audioSession.type = 'playback';
    } catch { /* Unsupported session settings must never interrupt playback. */ }
  });
})();
