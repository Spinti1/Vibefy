'use strict';
// Account grants are separate from playlist backups. Each encrypted record and
// its non-extractable key are committed together; no password is stored.
(() => {
  if (!window.indexedDB || !window.crypto?.subtle) return;
  const databaseName='vibefy-accounts-v1',storeName='accounts',jobs=new Map();
  let opening;
  function database(){
    if(opening)return opening;
    opening=new Promise((resolve,reject)=>{
      const request=indexedDB.open(databaseName,1);
      request.onupgradeneeded=()=>request.result.createObjectStore(storeName);
      request.onsuccess=()=>{const db=request.result;db.onversionchange=()=>{db.close();opening=null;};resolve(db);};
      request.onerror=request.onblocked=()=>{opening=null;reject(Error('Account storage is unavailable.'));};
    });
    return opening;
  }
  async function transaction(name,action,value){
    const db=await database();
    return new Promise((resolve,reject)=>{
      const tx=db.transaction(storeName,action==='read'?'readonly':'readwrite'),store=tx.objectStore(storeName);
      let result;const request=action==='read'?store.get(name):action==='write'?store.put(value,name):store.delete(name);
      request.onsuccess=()=>{result=request.result;};
      tx.oncomplete=()=>resolve(result);
      tx.onerror=tx.onabort=()=>reject(Error('Account storage could not complete the request.'));
    });
  }
  const aad=name=>new TextEncoder().encode('vibefy-session-v1:'+name);
  function account(name){
    const run=work=>{const next=(jobs.get(name)||Promise.resolve()).catch(()=>{}).then(work);jobs.set(name,next);return next;};
    return {
      read:()=>run(async()=>{
        const record=await transaction(name,'read');if(!record)return null;
        const plain=await crypto.subtle.decrypt({name:'AES-GCM',iv:record.iv,additionalData:aad(name)},record.key,record.cipher);
        if(plain.byteLength>32768)throw Error('Saved account is invalid.');
        return JSON.parse(new TextDecoder().decode(plain));
      }),
      write:value=>run(async()=>{
        const plain=new TextEncoder().encode(JSON.stringify(value));if(plain.byteLength>32768)throw Error('Account record is too large.');
        const key=await crypto.subtle.generateKey({name:'AES-GCM',length:256},false,['encrypt','decrypt']),iv=crypto.getRandomValues(new Uint8Array(12));
        const cipher=await crypto.subtle.encrypt({name:'AES-GCM',iv,additionalData:aad(name)},key,plain);
        await transaction(name,'write',{key,iv,cipher});
      }),
      clear:()=>run(()=>transaction(name,'clear'))
    };
  }
  window.vibefyBrowserSessions={google:account('google'),juicevault:account('juicevault')};
})();
