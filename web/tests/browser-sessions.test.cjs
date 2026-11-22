const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const crypto=require('node:crypto').webcrypto,source=fs.readFileSync(path.join(__dirname,'../public/browser-sessions.js'),'utf8');
function indexedStore(){
 const records=new Map();let fail=false;
 const db={createObjectStore(){},close(){},transaction(){const tx={};tx.objectStore=()=>{
  const operation=(kind,name,value)=>{const request={};queueMicrotask(()=>{if(fail){tx.onerror?.();return;}if(kind==='get')request.result=records.get(name);else if(kind==='put')records.set(name,structuredClone(value));else records.delete(name);request.onsuccess?.();tx.oncomplete?.();});return request;};
  return {get:name=>operation('get',name),put:(value,name)=>operation('put',name,value),delete:name=>operation('delete',name)};
 };return tx;}};
 return {records,fail(value){fail=value;},open(){const r={};queueMicrotask(()=>{r.result=db;r.onupgradeneeded?.();r.onsuccess?.();});return r;}};
}
function page(indexedDB){const window={indexedDB,crypto};vm.runInNewContext(source,{window,indexedDB,crypto,TextEncoder,TextDecoder,Uint8Array,JSON,Map,Promise,Error});return window.vibefyBrowserSessions;}
test('both account sessions survive a new page with encrypted records and non-extractable keys',async()=>{
 const db=indexedStore(),first=page(db);const google={access_token:'private-google',refresh_token:'private-refresh'},juice={access_token:'private-juice',username:'Example'};
 await Promise.all([first.google.write(google),first.juicevault.write(juice)]);
 for(const record of db.records.values()){assert.equal(record.key.extractable,false);assert.equal(record.access_token,undefined);assert.ok(record.cipher instanceof ArrayBuffer);}
 const second=page(db);assert.deepEqual(await second.google.read(),google);assert.deepEqual(await second.juicevault.read(),juice);
});
test('disconnect clears only the chosen account and wins over a pending save',async()=>{
 const db=indexedStore(),p=page(db);const save=p.google.write({access_token:'google'}),clear=p.google.clear();await p.juicevault.write({access_token:'juice'});await Promise.all([save,clear]);assert.equal(await p.google.read(),null);assert.equal((await p.juicevault.read()).access_token,'juice');
});
test('account records cannot be swapped and storage errors do not erase them',async()=>{
 const db=indexedStore(),p=page(db);await p.google.write({access_token:'google'});const original=db.records.get('google');db.records.set('juicevault',original);await assert.rejects(p.juicevault.read());
 db.fail(true);await assert.rejects(p.google.read());assert.equal(db.records.get('google'),original);db.fail(false);assert.equal((await p.google.read()).access_token,'google');
});
