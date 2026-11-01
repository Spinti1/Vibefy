'use strict';
// Do not initialize media, Spotify or Jam on a desktop browser.
if (window.vibefyPhone) {
  (async () => {
    for (const name of ['embedded-player', 'player', 'spotify', 'mobile-mods', 'jam']) {
      await new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = '/' + name + '.js';
        script.onload = resolve;
        script.onerror = reject;
        document.body.append(script);
      });
    }
  })().catch(() => {
    const notice = document.getElementById('toast');
    notice.hidden = false;
    notice.textContent = 'The player could not load. Check your connection and reload.';
  });
}
