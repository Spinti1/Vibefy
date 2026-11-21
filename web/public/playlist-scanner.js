'use strict';
(() => {
 // QR contents are untrusted. Import only Vibefy's playlist-share identifier;
 // never navigate to an arbitrary scanned URL or pass it to the native bridge.
 function playlistId(raw) {
  if(typeof raw!=='string'||raw.length>512)return null;
  try {
   const u=new URL(raw),id=u.protocol==='vibefy:'?u.searchParams.get('id'):new URLSearchParams(u.hash.slice(1)).get('share');
   if(u.username||u.password||u.port||!(/^[a-f0-9]{32}$/).test(id||''))return null;
   if(u.protocol==='vibefy:')return u.hostname==='playlist'&&['','/'].includes(u.pathname)?id:null;
   return u.protocol==='https:'&&['vibefy-neon.vercel.app','usevibefy.online','www.usevibefy.online'].includes(u.hostname)&&['/web','/phone','/app','/web/','/phone/','/app/'].includes(u.pathname)?id:null;
  }catch{return null;}
 }
 if(typeof module!=='undefined'&&module.exports)module.exports={playlistId};
 if(typeof document==='undefined')return;
 const make=(tag,text)=>{const n=document.createElement(tag);if(text)n.textContent=text;return n;};
 const button=make('button','Scan playlist QR code');button.id='scan-playlist';button.className='button scan-playlist';button.type='button';
 const toolbar=document.querySelector('#home-view .library-toolbar');if(!toolbar)return;toolbar.before(button);
 const dialog=make('dialog');dialog.id='playlist-scanner';dialog.setAttribute('aria-labelledby','scanner-title');
 const heading=make('div');heading.className='dialog-header';const title=make('h2','Scan a playlist');title.id='scanner-title';const close=make('button','×');close.className='icon-button';close.setAttribute('aria-label','Close scanner');close.onclick=()=>dialog.close();heading.append(title,close);
 const hint=make('p','Open a playlist’s “Share to phone · QR” on desktop and point your camera at its code.');hint.className='fine-print';
 const frame=make('div');frame.className='scanner-frame';const video=make('video');video.muted=true;video.autoplay=true;video.playsInline=true;video.setAttribute('playsinline','');video.setAttribute('aria-label','Live QR camera');frame.append(video);
 const status=make('p');status.setAttribute('role','status');status.className='fine-print';
 const retry=make('button','Try camera again');retry.className='button';retry.hidden=true;retry.onclick=start;
 const privacy=make('p','Camera access is only used while scanning. Images stay on your device and are never uploaded.');privacy.className='fine-print';
 dialog.append(heading,hint,frame,status,retry,privacy);document.body.append(dialog);
 const canvas=make('canvas'),context=canvas.getContext('2d',{willReadFrequently:true});let stream=null,timer=null,epoch=0,decoder;
 function stop(){epoch++;clearTimeout(timer);timer=null;stream?.getTracks().forEach(t=>t.stop());stream=null;video.srcObject=null;canvas.width=canvas.height=0;}
 function loadDecoder(){return decoder??=new Promise((resolve,reject)=>{const script=make('script');script.src='/vendor/jsqr-1.4.0.min.js';script.onload=()=>resolve(window.jsQR);script.onerror=()=>{script.remove();decoder=null;reject(Error('decoder-unavailable'));};document.head.append(script);});}
 async function start(){
  stop();const run=epoch;retry.hidden=true;status.textContent='Allow camera access to scan your playlist…';
  if(window.vibefyMobileShell&&window.vibefyNativeInfo&&!window.vibefyNativeInfo.qrCamera){status.textContent='Update Vibefy to use the in-app camera scanner.';return;}
  if(!navigator.mediaDevices?.getUserMedia){status.textContent='Camera scanning is unavailable in this version. Update Vibefy or open the playlist’s shared link.';retry.hidden=false;return;}
  try {
   const camera=await navigator.mediaDevices.getUserMedia({audio:false,video:{facingMode:{ideal:'environment'},width:{ideal:640},height:{ideal:640}}});
   if(run!==epoch||!dialog.open){camera.getTracks().forEach(t=>t.stop());return;}
   stream=camera;video.srcObject=camera;await video.play();const decode=await loadDecoder();if(run!==epoch)return;
   status.textContent='Point your camera at a Vibefy playlist QR code.';
   function scan(){
    if(run!==epoch||!dialog.open)return;
    if(video.readyState>=2&&video.videoWidth&&context){
     const scale=Math.min(1,640/Math.max(video.videoWidth,video.videoHeight));canvas.width=Math.round(video.videoWidth*scale);canvas.height=Math.round(video.videoHeight*scale);
     context.drawImage(video,0,0,canvas.width,canvas.height);const data=context.getImageData(0,0,canvas.width,canvas.height),code=decode(data.data,data.width,data.height,{inversionAttempts:'dontInvert'});
     if(code){const id=playlistId(code.data);if(id){stop();dialog.close();window.vibefyPlaylistShares?.open(id);return;}status.textContent='This is not a Vibefy playlist QR code. Scan the code from “Share to phone · QR”.';}
    }
    timer=setTimeout(scan,200);
   }
   scan();
  }catch(error){if(run!==epoch)return;stop();status.textContent=error.name==='NotAllowedError'?'Camera access was not allowed. Enable Camera for Vibefy in your device settings, then try again.':error.name==='NotFoundError'?'No camera was found on this device.':'The camera could not start. Close other camera apps and try again.';retry.hidden=false;}
 }
 button.onclick=()=>{dialog.showModal();start();};dialog.addEventListener('close',stop);dialog.addEventListener('cancel',stop);
 document.addEventListener('visibilitychange',()=>{if(document.visibilityState==='hidden'&&dialog.open){stop();retry.hidden=false;status.textContent='Camera paused. Tap “Try camera again” to continue.';}});
 window.addEventListener('pagehide',stop);
})();
