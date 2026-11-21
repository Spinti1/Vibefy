const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const {playlistId}=require('../public/playlist-scanner.js'),id='a'.repeat(32);
test('scanner accepts only known Vibefy playlist shares, never arbitrary QR navigation',()=>{
 for(const host of ['vibefy-neon.vercel.app','usevibefy.online','www.usevibefy.online'])assert.equal(playlistId(`https://${host}/web#share=${id}`),id);
 assert.equal(playlistId(`vibefy://playlist?id=${id}`),id);
 for(const url of [`https://evil.test/web#share=${id}`,`https://vibefy-neon.vercel.app.evil.test/web#share=${id}`,`https://evil@vibefy-neon.vercel.app/web#share=${id}`,`http://vibefy-neon.vercel.app/web#share=${id}`,`https://vibefy-neon.vercel.app:444/web#share=${id}`,`https://vibefy-neon.vercel.app/mobile-connect.html#share=${id}`,`vibefy://connect?id=${id}`,'javascript:alert(1)','a'.repeat(513)])assert.equal(playlistId(url),null,url);
});
function scanner(){
 const events=new Map(),nodes=new Map(),timers=[],imports=[];let cameraResolve,decoded=null,stops=0,requests=0;
 class Node{
  constructor(tag){this.tag=tag;this.children=[];this.events=new Map();this.readyState=2;this.videoWidth=640;this.videoHeight=480;}
  append(...children){this.children.push(...children);for(const n of children)if(n.id)nodes.set(n.id,n);}
  before(n){nodes.set(n.id,n);}setAttribute(){}showModal(){this.open=true;}close(){this.open=false;this.events.get('close')?.();}addEventListener(n,f){this.events.set(n,f);}play(){return Promise.resolve();}getContext(){return{drawImage(){},getImageData:()=>({data:new Uint8Array(4),width:1,height:1})};}
 }
 const document={body:new Node('body'),head:new Node('head'),visibilityState:'visible',querySelector:()=>new Node('toolbar'),createElement:t=>new Node(t),addEventListener:(n,f)=>events.set(n,f)};
 const camera={getTracks:()=>[{stop(){stops++;}}]},window={vibefyPlaylistShares:{open:id=>imports.push(id)},jsQR:()=>decoded,addEventListener:(n,f)=>events.set(n,f)};
 vm.runInNewContext(fs.readFileSync('web/public/playlist-scanner.js','utf8'),{document,window,URL,URLSearchParams,Uint8Array,Promise,navigator:{mediaDevices:{getUserMedia:options=>{requests++;assert.equal(options.audio,false);return new Promise(resolve=>cameraResolve=()=>resolve(camera));}}},setTimeout:f=>{timers.push(f);return timers.length;},clearTimeout(){}});
 return{nodes,events,document,timers,imports,get requests(){return requests;},get stops(){return stops;},allow:()=>cameraResolve(),decode:value=>decoded={data:value},decoder:()=>document.head.children[0].onload()};
}
const settle=async()=>{for(let i=0;i<5;i++)await new Promise(r=>setImmediate(r));};
test('camera is lazy, stops after valid scan and previews within the app',async()=>{
 const a=scanner();assert.equal(a.requests,0);a.nodes.get('scan-playlist').onclick();assert.equal(a.requests,1);a.allow();await settle();a.decode(`https://vibefy-neon.vercel.app/web#share=${id}`);a.decoder();await settle();assert.deepEqual(a.imports,[id]);assert.equal(a.stops,1);assert.equal(a.nodes.get('playlist-scanner').open,false);
});
test('camera permission resolving after cancel cannot reactivate capture',async()=>{
 const a=scanner();a.nodes.get('scan-playlist').onclick();a.nodes.get('playlist-scanner').close();a.allow();await settle();assert.equal(a.stops,1);assert.deepEqual(a.imports,[]);
});
test('camera stops when app is backgrounded and never follows untrusted QR links',async()=>{
 const a=scanner();a.nodes.get('scan-playlist').onclick();a.allow();await settle();a.decode('https://evil.test/');a.decoder();await settle();assert.deepEqual(a.imports,[]);a.document.visibilityState='hidden';a.events.get('visibilitychange')();assert.equal(a.stops,1);
});
