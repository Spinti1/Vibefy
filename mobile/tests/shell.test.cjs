const {test}=require('node:test');const assert=require('node:assert/strict');const vm=require('node:vm');const fs=require('node:fs');
const script=fs.readFileSync(require('node:path').join(__dirname,'../shared/app-shell.js'),'utf8');
const key='vibefy-mobile-resume-v1';
function page(options={}){
 class Storage {constructor(){this.items=new Map()}getItem(k){return this.items.get(k)??null}setItem(k,v){this.items.set(String(k),String(v))}removeItem(k){this.items.delete(k)}}
 const localStorage=new Storage(),sessionStorage=new Storage();if(options.resume)localStorage.setItem(key,options.resume);if(options.preferences)localStorage.setItem('vibefy-web-preferences',JSON.stringify(options.preferences));localStorage.setItem('account-data','untouched');
 const window={addEventListener(){}};window.top=window;window.self=options.frame?{}:window;
 const context={Storage,window,localStorage,sessionStorage,location:{origin:options.origin||'https://vibefy-neon.vercel.app'},document:{documentElement:{dataset:{}},readyState:'loading',addEventListener(){}}};
 vm.runInNewContext(script,context);return context;
}
const queue=JSON.stringify({queue:[{id:'abcdefghijk',title:'Track',artist:'Artist'}],index:0,collection:'local-test'});
test('restores metadata queue before page initialization',()=>{const p=page({resume:queue});assert.equal(p.sessionStorage.getItem('vibefy-resume'),queue);assert.equal(p.localStorage.getItem('account-data'),'untouched')});
test('persists new queue and explicit empty queue without touching accounts',()=>{const p=page();p.sessionStorage.setItem('vibefy-resume',queue);assert.equal(p.localStorage.getItem(key),queue);const empty=JSON.stringify({queue:[],index:-1});p.sessionStorage.setItem('vibefy-resume',empty);assert.equal(p.localStorage.getItem(key),empty);assert.equal(p.localStorage.getItem('account-data'),'untouched')});
test('malformed queue does not restore or resurrect a stale queue',()=>{for(const resume of ['{','[]',JSON.stringify({queue:[{id:'https://bad.example'}],index:0}),JSON.stringify({queue:[{id:'abcdefghijk'}],index:9})])assert.equal(page({resume}).sessionStorage.getItem('vibefy-resume'),null);const p=page({resume:queue});p.sessionStorage.setItem('vibefy-resume','bad');assert.equal(p.localStorage.getItem(key),null)});
test('clear queue also removes persistent checkpoint',()=>{const p=page({resume:queue});p.sessionStorage.removeItem('vibefy-resume');assert.equal(p.localStorage.getItem(key),null)});
test('unrelated session keys and iframe/foreign origins are not bridged',()=>{const p=page();p.sessionStorage.setItem('token','private');assert.equal(p.localStorage.getItem('token'),null);for(const opts of [{frame:true},{origin:'https://other.example'}]){const q=page(opts);assert.equal(q.window.vibefyMobileShell,undefined);q.sessionStorage.setItem('vibefy-resume',queue);assert.equal(q.localStorage.getItem(key),null)}});

test('queue persistence respects opt out',()=>{const p=page({resume:queue,preferences:{rememberQueue:false}});assert.equal(p.localStorage.getItem(key),null);assert.equal(p.sessionStorage.getItem('vibefy-resume'),null);p.sessionStorage.setItem('vibefy-resume',queue);assert.equal(p.localStorage.getItem(key),null)});

test('native export includes live unsaved metadata when storage quota was reached',()=>{
 class Storage {constructor(){this.items=new Map()}getItem(k){return this.items.get(k)||null}setItem(k,v){this.items.set(k,v)}removeItem(k){this.items.delete(k)}}
 const localStorage=new Storage(),sessionStorage=new Storage();localStorage.setItem('vibefy-web-v1',JSON.stringify({likes:[],collections:[]}));
 const live={likes:[],collections:[{id:'new-local',title:'New playlist',tracks:[]}]};let packet;const button={dataset:{},onclick(){throw Error('fallback unexpected')}};
 const window={addEventListener(){},VibefyMobile:{postMessage(value){packet=JSON.parse(value)}}};window.top=window;window.self=window;
 const document={documentElement:{dataset:{}},readyState:'complete',addEventListener(){},head:{append(){}},createElement(){return{}},getElementById(id){return id==='export-library'?button:null},querySelector(){return null},querySelectorAll(){return[]}};
 vm.runInNewContext(script,{Storage,window,document,localStorage,sessionStorage,state:live,location:{origin:'https://vibefy-neon.vercel.app'}});
 button.onclick();assert.deepEqual(JSON.parse(packet.content),live);assert.equal(packet.action,'exportBackup');
});
