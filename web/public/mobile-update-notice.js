(() => {
 'use strict';
 if(!window.VibefyMobile?.postMessage||!/Android/i.test(navigator.userAgent))return;
 const version=window.vibefyNativeInfo?.version;
 fetch('/mobile-version.json',{cache:'no-store',signal:AbortSignal.timeout(10000)}).then(r=>{if(!r.ok)throw Error();return r.json();}).then(info=>{
  if(!/^\d+\.\d+\.\d+$/.test(info.version)||!info.androidURL?.startsWith('https://github.com/Spinti1/Vibefy/releases/download/mobile-v'+info.version+'/'))return;
  if(version){const x=version.split('.').map(Number),y=info.version.split('.').map(Number);let newer=false;for(let i=0;i<3;i++){if(y[i]>x[i]){newer=true;break;}if(y[i]<x[i])return;}if(!newer)return;}
  const section=document.createElement('aside');section.className='playlist-workspace';section.setAttribute('role','status');
  const text=document.createElement('p');text.textContent='Vibefy '+info.version+' for Android is available. Install it over your existing app to keep playlists, settings and accounts.';
  const link=document.createElement('a');link.className='button small';link.textContent='Download Android update';link.href=info.androidURL;link.target='_blank';link.rel='noopener noreferrer';section.append(text,link);
  const home=document.getElementById('home-view')||document.querySelector('main');home?.prepend(section);
 }).catch(()=>{});
})();
