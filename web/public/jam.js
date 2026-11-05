'use strict';
// Shared desktop/browser protocol. Cloudflare relays metadata, never audio or account tokens.
(() => {
 const $=id=>document.getElementById(id), BASE='https://vibefy-jam.pex-am.workers.dev';
 let socket,host=false,token=null,code='',timer,retryTimer,attempt=0,stopping=true,rtt=0,lastPing=0;
 const state=window.vibefyJam={following:false,queue:[]};
 const valid=c=>/^[A-HJ-NP-Z2-9]{12}$/.test(c);
 const bridge=()=>window.vibefyPlayerBridge;
 const status=s=>{$('jam-status').textContent=s;};
 function controls(active){$('jam-host').disabled=active;$('jam-join').disabled=active;$('jam-leave').hidden=!active;$('jam-indicator').hidden=!active;}
 function parse(raw){let c=raw.trim();if(c.includes('://')){const u=new URL(c);if(u.origin!=='https://usevibefy.online'||u.pathname!=='/app')throw Error();c=u.searchParams.get('jam')||'';}c=c.replace(/[- ]/g,'').toUpperCase();if(!valid(c))throw Error();return c;}
 function remote(t){if(!t||!/^vibefy:youtube:[A-Za-z0-9_-]{11}$/.test(t.uri))return null;const id=t.uri.slice(15);return {id,title:t.title,artist:(t.artists||[]).map(a=>a.name).join(', '),album:t.album_name||t.album,duration:(t.duration_ms||0)/1000,image:`https://i.ytimg.com/vi/${id}/hqdefault.jpg`};}
 function local(t){return t&&/^[A-Za-z0-9_-]{11}$/.test(t.id)?{uri:'vibefy:youtube:'+t.id,title:t.title,artists:[{name:t.artist||''}],album:t.album||'',duration_ms:Math.round((t.duration||0)*1000)}:null;}
 function send(p){if(socket?.readyState===WebSocket.OPEN){const s=JSON.stringify(p);if(s.length<=23000)socket.send(s);}}
 function broadcast(){if(!host)return;const s=bridge().snapshot(),t=local(s.track);let queue=(bridge().queue?.()||[]).map(local).filter(Boolean);const p={type:'state',snapshot:t?{track:t,position_ms:Math.min(86400000,Math.max(0,Math.round(s.position*1000))),playing:!s.paused}:null,queue};while(JSON.stringify(p).length>23000&&queue.length)queue.pop();send(p);}
 function leave(announce=true){stopping=true;clearInterval(timer);clearTimeout(retryTimer);if(host)send({type:'end'});socket?.close();socket=null;const followed=state.following;host=false;token=null;state.following=false;if(followed)bridge()?.followMode(false);controls(false);$('jam-invite-row').hidden=true;if(announce)status('Session ended.');}
 function connect(){
  if(stopping)return;
  const ws=socket=new WebSocket(BASE.replace('https:','wss:')+'/rooms/'+code);
  ws.onopen=()=>send({type:'hello',protocol:2,host_token:token,name:'Vibefy web'});
  ws.onmessage=e=>{if(ws!==socket||typeof e.data!=='string'||e.data.length>64000)return;let p;try{p=JSON.parse(e.data);}catch{return;}if(!p||typeof p!=='object')return;
   if(p.type==='welcome'){attempt=0;state.following=!host;if(!host)bridge().followMode(true);status(host?'Hosting · share the link or code':'Following host');clearInterval(timer);timer=setInterval(()=>{broadcast();if(Date.now()-lastPing>10000){lastPing=Date.now();send({type:'ping',echo:lastPing});}},1000);broadcast();return;}
   if(p.type==='pong'&&p.echo===lastPing){rtt=Math.min(1500,Math.max(0,(Date.now()-lastPing)/2));return;}
   if(p.type==='state'){state.queue=Array.isArray(p.queue)?p.queue.slice(0,100).map(remote).filter(Boolean):[];bridge().refreshQueue?.();if(host){status(`Hosting · ${Number(p.listeners)||0} listeners`);return;}const s=p.snapshot,t=remote(s?.track);if(!s||!t){status('Waiting for the host to play a YouTube recording…');bridge().pause?.();return;}if(typeof s.playing!=='boolean'||!Number.isFinite(s.position_ms)||s.position_ms<0||s.position_ms>86400000)return;status('Following host');bridge().follow(t,(s.position_ms+(s.playing?rtt:0))/1000,!s.playing);return;}
   if(p.type==='add'&&host){const t=remote(p.track);if(t)bridge().enqueue(t);broadcast();return;}
   if(p.type==='notice'||p.type==='error')status(String(p.message||'Jam notification').slice(0,200));
  };
  ws.onclose=e=>{if(ws!==socket||stopping)return;clearInterval(timer);if(e.code===1000||e.code===1008||++attempt>5){leave(false);status('Jam ended or expired. You can start or join another session.');return;}status('Connection interrupted · reconnecting…');retryTimer=setTimeout(connect,Math.min(15000,attempt*2000));};
  ws.onerror=()=>status('Jam connection unavailable. Retrying…');
 }
 $('jam-host').onclick=async()=>{leave(false);controls(true);status('Creating session…');try{const res=await fetch(BASE+'/rooms',{method:'POST',signal:AbortSignal.timeout(12000)});if(!res.ok)throw Error('Cannot create a Jam right now. Try again later.');const p=await res.json();code=parse(p.code);if(!/^[a-f0-9]{64}$/.test(p.host_token))throw Error('Invalid Jam response');token=p.host_token;host=true;stopping=false;attempt=0;$('jam-invite').value='https://usevibefy.online/app?jam='+code;$('jam-invite-row').hidden=false;connect();}catch(e){leave(false);status(e.message);}};
 $('jam-join').onclick=()=>{let next;try{next=parse($('jam-code').value);}catch{return status('Paste a Vibefy invitation link or its 12-character code.');}leave(false);code=next;stopping=false;attempt=0;controls(true);status('Connecting…');connect();};
 $('jam-leave').onclick=()=>leave();
 $('jam-copy').onclick=async()=>{try{await navigator.clipboard.writeText($('jam-invite').value);status(`Invitation copied · code ${code.match(/.{4}/g).join('-')}`);}catch{status('Select the invitation and copy it.');}};
 state.add=t=>{if(!state.following||socket?.readyState!==WebSocket.OPEN)return false;const track=local(t);if(!track)return false;send({type:'add',track});return true;};
 const invite=new URLSearchParams(location.search).get('jam');if(invite&&valid(invite)){$('jam-code').value=invite;status('Invitation ready. Press Join to connect.');document.querySelector('[data-dialog="jam"]').click();}
 window.addEventListener('pagehide',()=>leave(false));
})();
