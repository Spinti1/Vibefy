(() => {
 'use strict';
 if(window.top!==window.self||location.origin!=='https://vibefy-neon.vercel.app'||window.vibefyMobileShell)return;
 window.vibefyMobileShell=true;
 document.documentElement.dataset.mobileApp='true';
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
 function install(){
  if(!document.getElementById('vibefy-mobile-style')){const style=document.createElement('style');style.id='vibefy-mobile-style';style.textContent=`
   html[data-mobile-app=true]{color-scheme:dark;background:#0e1215;-webkit-tap-highlight-color:transparent;overscroll-behavior:none}
   [data-mobile-app=true] body{overscroll-behavior-y:none}
   [data-mobile-app=true] .app-header,[data-mobile-app=true] .bottom-nav,[data-mobile-app=true] .mini-player{backdrop-filter:none;-webkit-backdrop-filter:none}
   [data-mobile-app=true] button,[data-mobile-app=true] .icon-button{touch-action:manipulation}
   [data-mobile-app=true] .app-header .download-link,[data-mobile-app=true] .rail-bottom{display:none!important}
   [data-mobile-app=true] .collection-tile{content-visibility:auto;contain-intrinsic-size:auto 180px}
   [data-mobile-app=true] input,[data-mobile-app=true] select,[data-mobile-app=true] textarea{font-size:16px}
   [data-mobile-app=true] dialog{max-height:calc(100dvh - 36px);overscroll-behavior:contain}
   [data-mobile-app=true] .bottom-nav{padding-bottom:max(8px,env(safe-area-inset-bottom))}
   [data-mobile-app=true] .app-header{padding-top:max(10px,env(safe-area-inset-top))}
  `;document.head.append(style);}
  const exporter=document.getElementById('export-library');
  if(exporter&&!exporter.dataset.nativeExport){exporter.dataset.nativeExport='true';const previous=exporter.onclick;exporter.onclick=()=>{try{const raw=typeof state==='object'&&state!==null?JSON.stringify(state):localStorage.getItem('vibefy-web-v1');if(!raw||raw.length>6000000)throw Error();const value=JSON.parse(raw);if(!Array.isArray(value.collections)||!Array.isArray(value.likes))throw Error();if(!send({action:'exportBackup',content:raw}))previous?.();}catch{window.vibefyWebLibrary?.toast('No saved library is available to export.');}};}
  const clearButton=document.querySelector('#clear-data-dialog .dialog-header h2');if(clearButton)clearButton.textContent='Clear app data';
  for(const label of document.querySelectorAll('button'))if(label.textContent.trim()==='Clear cookies & site data')label.textContent='Clear app data';
 }
 if(document.readyState==='loading')document.addEventListener('DOMContentLoaded',install,{once:true});else install();
 window.addEventListener('pageshow',install);
})();
