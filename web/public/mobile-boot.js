'use strict';
// Both desktop and phone browsers use the same local library and player.
{
  (async () => {
    for (const name of ['embedded-player', 'source-player', 'player', 'youtube-account', 'web-library', 'mobile-mods', 'jam']) {
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
