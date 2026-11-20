'use strict';
(() => {
 const $=id=>document.getElementById(id),key='vibefy-web-preferences';
 const prefs={reducedMotion:matchMedia('(prefers-reduced-motion: reduce)').matches,compactRows:false,autoNext:true,rememberQueue:true,lyricsSize:24};
 try{const saved=JSON.parse(localStorage.getItem(key)||'{}');for(const k of ['reducedMotion','compactRows','autoNext','rememberQueue'])if(typeof saved[k]==='boolean')prefs[k]=saved[k];if(Number.isFinite(saved.lyricsSize))prefs.lyricsSize=Math.max(18,Math.min(40,saved.lyricsSize));}catch{}
 window.vibefyPreferences=prefs;
 function apply(){document.body.classList.toggle('reduce-motion',prefs.reducedMotion);document.body.classList.toggle('compact-rows',prefs.compactRows);document.documentElement.style.setProperty('--web-lyrics-size',prefs.lyricsSize+'px');$('pref-lyricsSize-value').textContent=prefs.lyricsSize+' px';}
 function save(){try{localStorage.setItem(key,JSON.stringify(prefs));if(!prefs.rememberQueue)sessionStorage.removeItem('vibefy-resume');}catch{}apply();window.dispatchEvent(new Event('vibefy-preferences'));}
 for(const k of ['reducedMotion','compactRows','autoNext','rememberQueue']){const input=$('pref-'+k);input.checked=prefs[k];input.onchange=()=>{prefs[k]=input.checked;save();};}
 $('pref-lyricsSize').value=prefs.lyricsSize;$('pref-lyricsSize').oninput=()=>{prefs.lyricsSize=Number($('pref-lyricsSize').value);save();};apply();
 $('clear-data-export').onclick=()=>$('export-library').click();
 $('confirm-clear-data').onclick=async()=>{
  const button=$('confirm-clear-data'),status=$('clear-data-status');button.disabled=true;status.textContent='Clearing this site’s data…';
  try{
   // Server header clears browser-managed first-party data, including HttpOnly cookies.
   const response=await fetch('/api/site-data/clear',{method:'POST',headers:{'X-Vibefy-Action':'clear-site-data'},credentials:'same-origin',signal:AbortSignal.timeout(15000)});
   if(!response.ok)throw Error('Site data could not be cleared. Please retry.');
   window.vibefyJam?.leave?.();window.vibefyPlayerBridge?.pause?.();window.vibefyYouTube?.forget?.();window.vibefyJuice?.forget?.();
   // Explicit fallback for browsers with partial Clear-Site-Data support.
   localStorage.clear();sessionStorage.clear();
   if('caches'in window)for(const name of await caches.keys())await caches.delete(name);
   if('serviceWorker'in navigator)for(const registration of await navigator.serviceWorker.getRegistrations())await registration.unregister();
   if(window.indexedDB?.databases)for(const db of await indexedDB.databases())if(db.name)await new Promise((resolve,reject)=>{const request=indexedDB.deleteDatabase(db.name);const timer=setTimeout(()=>reject(Error('Some browser data is still open. Close other Vibefy tabs and retry.')),3000);request.onsuccess=()=>{clearTimeout(timer);resolve();};request.onerror=request.onblocked=()=>{clearTimeout(timer);reject(Error('Close other Vibefy tabs, then clear this site’s data in your browser settings.'));};});
   for(const cookie of document.cookie.split(';')){const name=cookie.split('=')[0].trim();if(name)document.cookie=name+'=; Max-Age=0; Path=/; SameSite=Lax; Secure';}
   location.replace('/web');
  }catch(error){status.textContent=error.message+' Browser support varies; your browser’s site settings can finish a full reset.';}finally{button.disabled=false;}
 };
})();
