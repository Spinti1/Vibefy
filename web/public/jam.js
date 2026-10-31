'use strict';
// Session signalling uses PeerJS Cloud. Music stays with each participant's player;
// only recording metadata, play/pause and timing cross the encrypted data channel.
(() => {
 const $=id=>document.getElementById(id);
 let peer, host=false, connection, timer, pingTimer, connectTimeout, clockOffset=0;
 const guests=new Set();
 const state=window.vibefyJam={following:false};
 function status(message){$('jam-status').textContent=message;}
 function controls(active){$('jam-host').disabled=active;$('jam-join').disabled=active;$('jam-leave').hidden=!active;$('jam-indicator').hidden=!active;}
 function count(){status(host?`Hosting · ${guests.size} listener${guests.size===1?'':'s'}`:'Following host');}
 function packet(value){try{return value&&typeof value==='object'&&value.v===1&&JSON.stringify(value).length<=4096;}catch{return false;}}
 function snapshot(){const s=window.vibefyPlayerBridge.snapshot();return {v:1,type:'state',track:s.track,position:s.position,paused:s.paused,sent:Date.now()};}
 function send(c,p){if(c.open)try{c.send(p);}catch{}}
 function broadcast(){if(!host)return;const s=snapshot();for(const c of guests)send(c,s);}
 async function library(){if(window.Peer)return;if(!state.loading)state.loading=new Promise((resolve,reject)=>{const script=document.createElement('script');script.src='/vendor/peerjs-1.5.5.min.js';script.onload=resolve;script.onerror=()=>reject(new Error('Jam could not load. Please retry.'));document.head.append(script);});await state.loading;}
 function makePeer(id){return new Peer(id,{host:'0.peerjs.com',port:443,path:'/',secure:true,debug:0,config:{iceServers:[{urls:'stun:stun.l.google.com:19302'}]}});}
 function fail(message){leave(false);status(message);}
 function wireErrors(){peer.on('error',()=>fail('Connection unavailable. Try again, or use the same Wi-Fi / VPN.'));peer.on('disconnected',()=>fail('The signalling connection was lost. Rejoin to reconnect.'));}
 function leave(announce=true){host=false;state.following=false;clearInterval(timer);clearInterval(pingTimer);clearTimeout(connectTimeout);for(const c of guests)c.close();guests.clear();connection?.close();connection=null;peer?.destroy();peer=null;window.vibefyPlayerBridge?.followMode(false);controls(false);$('jam-invite-row').hidden=true;$('jam-invite').value='';if(announce)status('Session ended.');}
 $('jam-host').onclick=async()=>{
  try{await library();leave(false);host=true;controls(true);status('Creating session…');const id='vibefy-'+crypto.randomUUID().replaceAll('-','');peer=makePeer(id);wireErrors();
   connectTimeout=setTimeout(()=>fail('The session service did not answer. Please retry.'),15000);
   peer.on('open',()=>{clearTimeout(connectTimeout);$('jam-invite').value=location.origin+'/app?jam='+id;$('jam-invite-row').hidden=false;count();timer=setInterval(broadcast,1000);});
   peer.on('connection',c=>{if(!host||guests.size>=8){c.close();return;}guests.add(c);c.on('open',()=>{send(c,snapshot());count();});c.on('data',p=>{if(packet(p)&&p.type==='ping'&&Number.isFinite(p.t))send(c,{v:1,type:'pong',t:p.t,host:Date.now()});});c.on('close',()=>{guests.delete(c);if(host)count();});c.on('error',()=>{guests.delete(c);if(host)count();});});
  }catch(e){fail(e.message);}
 };
 $('jam-join').onclick=async()=>{
  let id=$('jam-code').value.trim();try{if(id.includes('://')){const u=new URL(id);id=u.searchParams.get('jam')||'';}}catch{id='';}
  if(!/^vibefy-[a-f0-9]{32}$/.test(id))return status('Paste a valid Vibefy web Jam invitation. Desktop LAN codes use a different protocol.');
  try{await library();leave(false);controls(true);status('Connecting to host…');peer=makePeer();wireErrors();connectTimeout=setTimeout(()=>fail('Direct connection failed. Try the same Wi-Fi or a VPN; some networks block WebRTC.'),20000);
   peer.on('open',()=>{connection=peer.connect(id,{reliable:true,serialization:'json'});connection.on('open',()=>{clearTimeout(connectTimeout);state.following=true;window.vibefyPlayerBridge.followMode(true);count();send(connection,{v:1,type:'ping',t:Date.now()});pingTimer=setInterval(()=>send(connection,{v:1,type:'ping',t:Date.now()}),10000);});
    connection.on('data',p=>{if(!packet(p))return;if(p.type==='pong'&&Number.isFinite(p.t)&&Number.isFinite(p.host)&&Date.now()-p.t>=0&&Date.now()-p.t<10000){clockOffset=p.host-(p.t+Date.now())/2;return;}
     if(p.type!=='state'||!Number.isFinite(p.position)||p.position<0||p.position>86400||!Number.isFinite(p.sent)||typeof p.paused!=='boolean')return;
     const elapsed=p.paused?0:Math.max(0,Math.min(5,(Date.now()+clockOffset-p.sent)/1000));window.vibefyPlayerBridge.follow(p.track,p.position+elapsed,p.paused);
    });
    connection.on('close',()=>{if(state.following)fail('The host ended the session.');});connection.on('error',()=>fail('The host connection was interrupted.'));
   });
  }catch(e){fail(e.message);}
 };
 $('jam-leave').onclick=()=>leave();
 $('jam-copy').onclick=async()=>{try{await navigator.clipboard.writeText($('jam-invite').value);status('Invitation copied. Share it only with people you trust.');}catch{status('Select the invitation above and copy it.');}};
 const invitation=new URLSearchParams(location.search).get('jam');if(invitation&&/^vibefy-[a-f0-9]{32}$/.test(invitation)){$('jam-code').value=invitation;$('jam-status').textContent='Invitation ready. Press Join to connect.';document.querySelector('[data-dialog="jam"]').click();}
 window.addEventListener('pagehide',()=>leave(false));
})();
