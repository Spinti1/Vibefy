const {test}=require('node:test'),assert=require('node:assert/strict'),vm=require('node:vm'),fs=require('node:fs');
const source=fs.readFileSync('web/public/spotify-playlist-sync.js','utf8'),spotify='1'.repeat(22),song=id=>({id:'spotify:'+id.repeat(22),title:'Demo '+id,artist:'Artist',duration:180});
function app(phone=false){
 let now=2000000,interval,remote=[song('2'),song('3')],fail=false,collection={id:'spotify:'+spotify,title:'My title',tracks:[{id:'aaaaaaaaaaa',spotifyId:'2'.repeat(22),title:'Kept'},{id:'bbbbbbbbbbb',title:'Local only'}]};const calls=[],events=new Map(),storage=new Map(),saved=[];
 class Node{constructor(tag,text){this.tag=tag;this.textContent=text;this.children=[];}append(...n){this.children.push(...n);}setAttribute(){}after(n){this.afterNode=n;}}
 const subtitle=new Node('p'),document={visibilityState:'visible',createElement:tag=>new Node(tag),createTextNode:t=>({textContent:t}),getElementById:()=>subtitle,addEventListener:(e,f)=>events.set(e,f)};
 const bridge={selected:()=>collection,collections:()=>collection?[collection]:[],save:c=>{collection=c;saved.push(c);},toast(){}};
 vm.runInNewContext(source,{window:{vibefyMobileShell:phone,vibefyWebLibrary:bridge},document,localStorage:{getItem:k=>storage.get(k),setItem:(k,v)=>storage.set(k,v)},Date:{now:()=>now},AbortSignal,setInterval:f=>interval=f,fetch:async(url)=>{calls.push(url);if(fail)return{ok:false,status:429,headers:{get:()=> '120'}};if(String(url).startsWith('/api/import/'))return{ok:true,json:async()=>({tracks:remote,next:null})};return{ok:true,json:async()=>({track:{id:'ccccccccccc',title:'New song',artist:'Artist'}})};}});
 const tools=subtitle.afterNode;return{calls,saved,events,tools,document,get collection(){return collection;},set remote(v){remote=v;},set fail(v){fail=v;},remove:()=>collection=null,advance:ms=>now+=ms,tick:()=>interval?.(),button:tools?.children[0],follow:tools?.children[1]?.children[0]};
}
test('Spotify sync reads only unknown recordings, appends them and preserves local additions/title',async()=>{
 const a=app();await a.button.onclick();assert.equal(a.saved.length,1);assert.deepEqual(a.collection.tracks.map(t=>t.id),['aaaaaaaaaaa','bbbbbbbbbbb','ccccccccccc']);assert.equal(a.collection.title,'My title');assert.equal(a.calls.filter(x=>x==='/api/match').length,1);
 a.advance(11000);await a.button.onclick();assert.equal(a.calls.filter(x=>x==='/api/match').length,1);assert.equal(a.collection.tracks.length,3);
});
test('Spotify rate limits preserve the local copy and suppress repeated requests',async()=>{
 const a=app();a.fail=true;await a.button.onclick();assert.equal(a.saved.length,0);const count=a.calls.length;await a.button.onclick();assert.equal(a.calls.length,count);assert.match(a.tools.children[2].textContent,/cooling down/);
});
test('Spotify automatic checks are opt-in, every 15 minutes, and pause while hidden',async()=>{
 const a=app();const settle=async()=>{for(let i=0;i<5;i++)await new Promise(r=>setImmediate(r));};a.tick();assert.equal(a.calls.length,0);a.follow.checked=true;a.follow.onchange();
 a.document.visibilityState='hidden';a.tick();assert.equal(a.calls.length,0);
 a.document.visibilityState='visible';a.tick();await settle();assert.ok(a.calls.length>0);const count=a.calls.length;
 a.advance(14*60*1000);a.tick();await settle();assert.equal(a.calls.length,count);
 a.advance(60*1000);a.tick();await settle();assert.equal(a.calls.length,count+1);
});
test('native phone apps have no Spotify sync controls or polling',()=>{
 const a=app(true);assert.equal(a.tools,undefined);a.tick();assert.equal(a.calls.length,0);
});
test('invalid recording metadata never claims a successful sync or changes a playlist',async()=>{
 const a=app();a.remote=[{title:'No identity'}];await a.button.onclick();assert.equal(a.saved.length,0);assert.match(a.tools.children[2].textContent,/without its identifier/);
});
