const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const shared=fs.readFileSync('mobile/shared/app-shell.js','utf8'),hosted=fs.readFileSync('web/public/mobile-shell.js','utf8');
function context(kind='android'){
 class Storage {getItem(){return null}setItem(){}removeItem(){}}
 let ready,packet;const bridge={postMessage(raw){packet=typeof raw==='string'?JSON.parse(raw):raw}};
 const window={addEventListener(){}};window.self=window;window.top=window;
 if(kind==='android')window.VibefyMobile=bridge;if(kind==='ios')window.webkit={messageHandlers:{vibefyMobile:bridge}};
 const document={documentElement:null,readyState:'loading',addEventListener(name,fn){if(name==='DOMContentLoaded')ready=fn},getElementById(id){return id==='vibefy-mobile-style'?{}:null},querySelector(){return null},querySelectorAll(){return[]}};
 const c={window,document,location:{origin:'https://vibefy-neon.vercel.app'},Storage,localStorage:new Storage(),sessionStorage:new Storage(),crypto:globalThis.crypto,Uint8Array,btoa,atob,URL,URLSearchParams,TextEncoder,TextDecoder,setTimeout:()=>1,clearTimeout(){}};
 return{c,window,document,get packet(){return packet},ready(){document.documentElement={dataset:{}};ready?.()}};
}
for(const platform of ['ios','android']){
 test(platform+': document-start before HTML root still initializes account bridge and opens system browser',async()=>{const e=context(platform);vm.runInNewContext(shared,e.c);assert.equal(e.window.vibefyMobileShellReady,true);assert.equal(typeof e.window.vibefyMobileAuthorize,'function');e.ready();assert.equal(e.document.documentElement.dataset.mobileApp,'true');const job=e.window.vibefyMobileAuthorize(false);assert.equal(e.packet.action,'connectGoogle');assert.match(e.packet.url,/^https:\/\/vibefy-neon\.vercel\.app\/mobile-connect\.html#/);const rejected=assert.rejects(job,/cancelled/);e.window.vibefyMobileCancelAuth();await rejected;});
 test(platform+': hosted repair recovers the partially initialized 0.2.2 shell',()=>{const e=context(platform);e.window.vibefyMobileShell=true;vm.runInNewContext(hosted,e.c);assert.equal(typeof e.window.vibefyMobileAuthorize,'function');const first=e.window.vibefyMobileAuthorize;vm.runInNewContext(shared,e.c);assert.equal(e.window.vibefyMobileAuthorize,first)});
}
test('hosted repair leaves an ordinary browser unchanged',()=>{const e=context('browser');vm.runInNewContext(hosted,e.c);assert.equal(e.window.vibefyMobileAuthorize,undefined);assert.equal(e.window.vibefyMobileShell,undefined)});
