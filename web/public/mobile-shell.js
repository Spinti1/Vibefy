// Repair and maintain installed mobile clients.
if(window.VibefyMobile?.postMessage||window.webkit?.messageHandlers?.vibefyMobile){
(() => {
 'use strict';
 if(window.top!==window.self||location.origin!=='https://vibefy-neon.vercel.app'||window.vibefyMobileShellReady)return;
 window.vibefyMobileShell=true;
 const resumeKey='vibefy-mobile-resume-v1';
 const remembering=()=>{try{return JSON.parse(localStorage.getItem('vibefy-web-preferences')||'{}').rememberQueue!==false;}catch{return true;}};
 const originalSet=Storage.prototype.setItem,originalRemove=Storage.prototype.removeItem;
 const validResume=raw=>{if(typeof raw!=='string'||raw.length>1000000)return false;try{const v=JSON.parse(raw);return Array.isArray(v.queue)&&v.queue.length<=1000&&Number.isInteger(v.index)&&(v.queue.length===0?v.index===-1:v.index>=0&&v.index<v.queue.length)&&v.queue.every(t=>t&&typeof t.id==='string'&&/^(?:[A-Za-z0-9_-]{11}|jv:[0-9a-fA-F-]{36}|sc:\d{1,20})$/.test(t.id));}catch{return false;}};
 try {
  if(!remembering()){originalRemove.call(localStorage,resumeKey);originalRemove.call(sessionStorage,'vibefy-resume');}
  if(remembering()&&!sessionStorage.getItem('vibefy-resume')) {const held=localStorage.getItem(resumeKey);if(validResume(held))originalSet.call(sessionStorage,'vibefy-resume',held);}
  Storage.prototype.setItem=function(key,value){originalSet.call(this,key,value);if(this===sessionStorage&&key==='vibefy-resume')try{if(remembering()&&validResume(String(value)))originalSet.call(localStorage,resumeKey,String(value));else originalRemove.call(localStorage,resumeKey);}catch{}};
  Storage.prototype.removeItem=function(key){originalRemove.call(this,key);if(this===sessionStorage&&key==='vibefy-resume')try{originalRemove.call(localStorage,resumeKey);}catch{}};
 }catch{}
 function send(packet){try{if(window.VibefyMobile?.postMessage)window.VibefyMobile.postMessage(JSON.stringify(packet));else if(window.webkit?.messageHandlers?.vibefyMobile)window.webkit.messageHandlers.vibefyMobile.postMessage(packet);else return false;return true;}catch{return false;}}

 const sessionJobs=new Map();let sessionSequence=0;
 if(window.vibefyNativeInfo?.secureSession){
  const call=(action,value)=>new Promise((resolve,reject)=>{const id='session-'+(++sessionSequence);const timer=setTimeout(()=>{sessionJobs.delete(id);reject(Error('Secure account storage is unavailable.'));},8000);sessionJobs.set(id,{resolve,reject,timer});if(!send({action,id,...(value?{value}: {})})){clearTimeout(timer);sessionJobs.delete(id);reject(Error('Secure account storage is unavailable.'));}});
  window.vibefyMobileSession={read:()=>call('readSession'),write:value=>call('saveSession',value),clear:()=>call('clearSession')};
  window.vibefyMobileSessionResult=(id,value,error)=>{const job=sessionJobs.get(id);if(!job)return;sessionJobs.delete(id);clearTimeout(job.timer);if(error)job.reject(Error('Secure account storage is unavailable.'));else job.resolve(value);};
 }
 let authJob;
 window.vibefyMobileCancelAuth=()=>{const job=authJob;if(!job)return;authJob=null;clearTimeout(job.timer);job.key.fill(0);job.reject(Error('Connection cancelled. Tap Connect YouTube to try again.'));};
 const enc=bytes=>btoa(String.fromCharCode(...bytes)).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');
 const dec=s=>Uint8Array.from(atob(s.replace(/-/g,'+').replace(/_/g,'/')),c=>c.charCodeAt(0));
 window.vibefyMobileAuthorize=write=>new Promise((resolve,reject)=>{
  if(authJob)return reject(Error('Finish connecting in your browser first.'));
  const key=crypto.getRandomValues(new Uint8Array(32)),state=enc(crypto.getRandomValues(new Uint8Array(24)));
  const job={key,state,resolve,reject};authJob=job;job.timer=setTimeout(()=>{if(authJob===job){authJob=null;reject(Error('Connection expired. Tap Connect YouTube to try again.'));}},300000);
  if(!send({action:'connectGoogle',url:location.origin+'/mobile-connect.html#'+new URLSearchParams({key:enc(key),state,write:write?'1':'0'})})){clearTimeout(job.timer);authJob=null;reject(Error('The browser could not open.'));}
 });
 window.vibefyMobileAuthResult=async raw=>{
  const job=authJob;if(!job)return;
  try{const u=new URL(raw);if(u.protocol!=='vibefy:'||u.hostname!=='connect'||raw.length>16000)return;
   const p=new URLSearchParams(u.hash.slice(1));if(p.get('state')!==job.state)return;
   const key=await crypto.subtle.importKey('raw',job.key,'AES-GCM',false,['decrypt']);
   const plain=await crypto.subtle.decrypt({name:'AES-GCM',iv:dec(p.get('iv')),additionalData:new TextEncoder().encode(job.state)},key,dec(p.get('data')));
   const grant=JSON.parse(new TextDecoder().decode(plain));if(!grant.access_token||!Number.isFinite(grant.expires_in)||grant.expires_in<=0)throw Error();
   authJob=null;clearTimeout(job.timer);job.key.fill(0);job.resolve(grant);
  }catch{authJob=null;clearTimeout(job.timer);job.key.fill(0);job.reject(Error('The connection could not be verified. Please connect again.'));}
 };
 function routeHome(event){
  const anchor=event.target?.closest?.('a[href]');if(!anchor)return;
  const url=new URL(anchor.href,location.origin);
  if(anchor.matches('.brand,[data-app-home]')||(url.origin===location.origin&&['/','/index.html','/download'].includes(url.pathname))){event.preventDefault();event.stopImmediatePropagation();document.querySelector('.toolbar-home')?.click();}
 }
 document.addEventListener('click',routeHome,true);window.vibefyMobileNavigationReady=true;
 window.vibefyMobileShellReady=true;
 function install(){
  if(document.documentElement)document.documentElement.dataset.mobileApp='true';
  if(!document.getElementById('vibefy-mobile-style')){const style=document.createElement('style');style.id='vibefy-mobile-style';style.textContent=`
   html[data-mobile-app=true]{color-scheme:dark;background:#0e1215;-webkit-tap-highlight-color:transparent;overscroll-behavior:none}
   [data-mobile-app=true] body{overscroll-behavior-y:none}
   [data-mobile-app=true] .app-header,[data-mobile-app=true] .bottom-nav,[data-mobile-app=true] .mini-player{backdrop-filter:none;-webkit-backdrop-filter:none}
   [data-mobile-app=true] button,[data-mobile-app=true] .icon-button{touch-action:manipulation}
   [data-mobile-app=true] .app-header .download-link,[data-mobile-app=true] .app-header .github-link,[data-mobile-app=true] .web-label,[data-mobile-app=true] .rail-bottom{display:none!important}
   [data-mobile-app=true] .collection-tile{content-visibility:auto;contain-intrinsic-size:auto 180px}
   [data-mobile-app=true] input,[data-mobile-app=true] select,[data-mobile-app=true] textarea{font-size:16px}
   [data-mobile-app=true] dialog{max-height:calc(100dvh - 36px);overscroll-behavior:contain}
   [data-mobile-app=true] .bottom-nav{padding-bottom:max(8px,env(safe-area-inset-bottom))}
   [data-mobile-app=true] .app-header{padding-top:max(10px,env(safe-area-inset-top))}
  `;document.head.append(style);}
  const exporter=document.getElementById('export-library');
  if(exporter&&!exporter.dataset.nativeExport){exporter.dataset.nativeExport='true';const previous=exporter.onclick;exporter.onclick=()=>{try{const raw=typeof state==='object'&&state!==null?JSON.stringify(state):localStorage.getItem('vibefy-web-v1');if(!raw||raw.length>6000000)throw Error();const value=JSON.parse(raw);if(!Array.isArray(value.collections)||!Array.isArray(value.likes))throw Error();if(!send({action:'exportBackup',content:raw}))previous?.();}catch{window.vibefyWebLibrary?.toast('No saved library is available to export.');}};}
  if(window.vibefyNativeInfo?.platform==='android'&&!document.getElementById('native-update-check')){const area=document.getElementById('settings-dialog');if(area){const check=document.createElement('button');check.id='native-update-check';check.className='button small';check.textContent='Check for app updates';check.onclick=()=>send({action:'checkUpdates'});area.append(check);}}
  const clearButton=document.querySelector('#clear-data-dialog .dialog-header h2');if(clearButton)clearButton.textContent='Clear app data';
  for(const label of document.querySelectorAll('button'))if(label.textContent.trim()==='Clear cookies & site data')label.textContent='Clear app data';
 }
 if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',install,{once:true});else install();
 window.addEventListener('pageshow',install);
})();

}
