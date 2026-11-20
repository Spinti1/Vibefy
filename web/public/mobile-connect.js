'use strict';
(async()=>{
 const p=new URLSearchParams(location.hash.slice(1));history.replaceState(null,'',location.pathname);
 const status=document.getElementById('status'),button=document.getElementById('connect'),back=document.getElementById('return');
 let popupTimer;const retry=document.getElementById('retry');
 const reset=()=>{clearTimeout(popupTimer);button.disabled=false;retry.hidden=true;status.textContent='Tap Continue with Google to retry. Close any older Google window first.';};retry.onclick=reset;
 const state=p.get('state'),raw=p.get('key'),write=p.get('write')==='1',renew=p.get('renew')==='1';
 const dec=s=>Uint8Array.from(atob(s.replace(/-/g,'+').replace(/_/g,'/')),c=>c.charCodeAt(0));
 const enc=b=>btoa(String.fromCharCode(...b)).replace(/\+/g,'-').replace(/\//g,'_').replace(/=+$/,'');
 try{
  if(!/^[A-Za-z0-9_-]{32}$/.test(state||'')||!/^[A-Za-z0-9_-]{43}$/.test(raw||''))throw Error('Start the connection from the Vibefy app.');
  const key=await crypto.subtle.importKey('raw',dec(raw),'AES-GCM',false,['encrypt']);
  const r=await fetch('/api/google-config',{signal:AbortSignal.timeout(15000)});if(!r.ok)throw Error('Connection is temporarily unavailable. Try again.');const config=await r.json();if(!config.clientId)throw Error('Google connection is unavailable.');
  await new Promise((resolve,reject)=>{const s=document.createElement('script');s.src='https://accounts.google.com/gsi/client';s.onload=resolve;s.onerror=()=>reject(Error('Google could not load. Check your connection.'));document.head.append(s);});
  const read='https://www.googleapis.com/auth/youtube.readonly';
  if(renew&&!config.renewable)throw Error('Persistent Google connection is being configured. Please retry shortly.');
  const oauth=google.accounts.oauth2[renew?'initCodeClient':'initTokenClient']({client_id:config.clientId,scope:'openid profile '+read+(write?' https://www.googleapis.com/auth/youtube.force-ssl':''),include_granted_scopes:false,ux_mode:'popup',select_account:true,error_callback:()=>{clearTimeout(popupTimer);retry.hidden=true;status.textContent='Connection cancelled or popup blocked. Tap Continue with Google to retry.';button.disabled=false;},callback:async grant=>{clearTimeout(popupTimer);retry.hidden=true;
   try{if(renew){if(grant.error||!grant.code)throw Error('Google did not grant access. Try again.');const r=await fetch('/api/google-session/exchange',{method:'POST',headers:{'Content-Type':'application/json','X-Vibefy-Auth':'1'},body:JSON.stringify({code:grant.code}),signal:AbortSignal.timeout(20000)});if(!r.ok)throw Error('Google could not enable persistent access. Retry the connection; your playlists are safe.');grant=await r.json();}
    if(grant.error||!grant.access_token)throw Error('Google did not grant access. Try again.');if(!String(grant.scope||'').split(' ').includes(read))throw Error('Allow YouTube playlist access to connect.');
    const iv=crypto.getRandomValues(new Uint8Array(12)),payload={access_token:grant.access_token,expires_in:Number(grant.expires_in||3600),scope:grant.scope,...(renew?{refresh_token:grant.refresh_token}:{})};
    const data=await crypto.subtle.encrypt({name:'AES-GCM',iv,additionalData:new TextEncoder().encode(state)},key,new TextEncoder().encode(JSON.stringify(payload)));
    back.href='vibefy://connect#'+new URLSearchParams({state,iv:enc(iv),data:enc(new Uint8Array(data))});back.hidden=false;button.hidden=true;status.textContent='Google access is ready. Tap Return to Vibefy to finish connecting and load your playlists.';
   }catch(e){status.textContent=e.message;button.disabled=false;}
  }});
  button.disabled=false;button.onclick=()=>{button.disabled=true;retry.hidden=false;popupTimer=setTimeout(reset,90000);status.textContent='Finish connecting in the Google window…';if(renew)oauth.requestCode();else oauth.requestAccessToken({prompt:'select_account'});};
 }catch(e){status.textContent=e.message;}
})();
