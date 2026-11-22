'use strict';
(() => {
 const native=()=>!!(window.vibefyMobileShell||window.VibefyMobile||window.webkit?.messageHandlers?.vibefyMobile);
 const standalone=matchMedia('(display-mode: standalone)');
 const installed=()=>standalone.matches||navigator.standalone===true;
 if(native())return;
 const ios=/iPhone|iPad|iPod/.test(navigator.userAgent)||(navigator.platform==='MacIntel'&&navigator.maxTouchPoints>1);
 const phone=ios||window.vibefyPhone||/Android/.test(navigator.userAgent);
 if(!phone)return;
 const key='vibefy-install-dismissed-v1';let deferred=null,automatic=true;
 const el=(tag,text)=>{const n=document.createElement(tag);if(text)n.textContent=text;return n;};
 const dialog=el('dialog');dialog.id='install-app-dialog';dialog.setAttribute('aria-labelledby','install-app-title');
 const header=el('div');header.className='dialog-header';const title=el('h2','Keep Vibefy close');title.id='install-app-title';
 const close=el('button','×');close.className='icon-button';close.type='button';close.setAttribute('aria-label','Close installation guide');header.append(title,close);
 const icon=el('img');icon.src='/assets/icon.png';icon.alt='';icon.className='install-app-icon';
 const description=el('p','Add Vibefy to your Home Screen. Open the same music player in its own window, without installing an IPA or APK.');
 const steps=el('ol');steps.className='install-app-steps';
 for(const text of ios?['Open this link in Safari if you are using another app’s built-in browser.','Tap Share, then Add to Home Screen (Adăugați pe ecranul principal).','Keep “Open as Web App” enabled if shown, then tap Add.']:
 ['Open your browser’s menu (⋮).','Choose Install app or Add to Home screen, then confirm.'])steps.append(el('li',text));
 const note=el('p','Sign in and save playlists from the Home Screen app. Your browser may keep its storage separate. Music needs an internet connection; background playback depends on the browser.');note.className='fine-print';
 const status=el('p');status.className='fine-print';status.setAttribute('role','status');
 const actions=el('div');actions.className='install-app-actions';const primary=el('button');primary.className='button primary';primary.type='button';
 const later=el('button','Not now');later.className='button';later.type='button';actions.append(primary,later);
 dialog.append(header,icon,description,steps,note,status,actions);document.body.append(dialog);
 const entry=el('button','Add Vibefy to Home Screen');entry.className='button small';entry.type='button';
 document.querySelector('#settings-dialog .setting-actions')?.prepend(entry);
 const dismiss=()=>{try{localStorage.setItem(key,String(Date.now()));}catch{}dialog.close();};
 function refresh(){const done=installed();entry.hidden=done;if(done&&dialog.open)dialog.close();primary.textContent=deferred?'Install Vibefy':ios?'Got it':'Show installation steps';steps.hidden=!!deferred;}
 function show(){if(native()||installed())return;refresh();if(!dialog.open)dialog.showModal();}
 close.onclick=later.onclick=dismiss;
 dialog.addEventListener('cancel',event=>{event.preventDefault();dismiss();});
 entry.onclick=()=>{document.getElementById('settings-dialog')?.close();show();};
 primary.onclick=async()=>{
  if(!deferred){if(ios){dismiss();return;}steps.hidden=false;status.textContent='If Install is unavailable, use Chrome or Safari to open this address directly.';return;}
  const event=deferred;deferred=null;primary.disabled=true;
  try{await event.prompt();const choice=await event.userChoice;if(choice?.outcome==='accepted')dialog.close();else dismiss();}
  catch{status.textContent='Use your browser menu to add Vibefy to your Home Screen.';}
  finally{primary.disabled=false;refresh();}
 };
 window.addEventListener('beforeinstallprompt',event=>{event.preventDefault();deferred=event;refresh();});
 window.addEventListener('appinstalled',()=>{deferred=null;entry.hidden=true;dialog.close();});
 standalone.addEventListener?.('change',refresh);
 function offer(){if(!automatic||installed()||document.querySelector('dialog[open]'))return;automatic=false;let last=0;try{last=Number(localStorage.getItem(key))||0;}catch{}if(Date.now()-last<7*86400000)return;show();}
 document.querySelectorAll('dialog').forEach(node=>{if(node!==dialog)node.addEventListener('close',offer);});
 refresh();setTimeout(offer,1200);
})();
