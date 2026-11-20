const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../public/web-settings.js'),'utf8');
function page(ok=true){
 const nodes=new Map(),store=new Map([['vibefy-web-v1','playlist metadata']]),actions=[],classes=new Map();
 const node=id=>{if(!nodes.has(id))nodes.set(id,{value:24,checked:false,textContent:'',click(){actions.push('export');}});return nodes.get(id);};
 const localStorage={getItem:k=>store.get(k)||null,setItem:(k,v)=>store.set(k,v),removeItem:k=>store.delete(k),clear:()=>{actions.push('clear-local');store.clear();}};
 const window={dispatchEvent(){},vibefyJam:{leave(){actions.push('leave');}},vibefyPlayerBridge:{pause(){actions.push('pause');}},vibefyYouTube:{forget(){actions.push('forget-account');}}};
 vm.runInNewContext(source,{window,document:{getElementById:node,body:{classList:{toggle:(name,value)=>classes.set(name,value)}},documentElement:{style:{setProperty(){}}},cookie:''},matchMedia:()=>({matches:false}),localStorage,sessionStorage:{removeItem(){},clear(){actions.push('clear-session');}},Event,AbortSignal,navigator:{},location:{replace:url=>actions.push(url)},fetch:async(url,options)=>{actions.push([url,options.method,options.headers['X-Vibefy-Action']]);return {ok};}});
 return {node,store,actions,classes};
}
test('opening settings does not clear data; preferences update only their own store',()=>{
 const p=page();assert.deepEqual(p.actions,[]);assert.equal(p.store.get('vibefy-web-v1'),'playlist metadata');
 p.node('pref-compactRows').checked=true;p.node('pref-compactRows').onchange();
 assert.equal(p.classes.get('compact-rows'),true);assert.equal(p.store.get('vibefy-web-v1'),'playlist metadata');
});
test('a failed clear request preserves local playlists and account sessions',async()=>{
 const p=page(false);await p.node('confirm-clear-data').onclick();assert.equal(p.store.get('vibefy-web-v1'),'playlist metadata');assert.ok(!p.actions.includes('forget-account'));assert.match(p.node('clear-data-status').textContent,/could not be cleared/);assert.equal(p.node('confirm-clear-data').disabled,false);
});
test('explicit confirmed reset clears only after the server accepts the action',async()=>{
 const p=page();await p.node('confirm-clear-data').onclick();assert.deepEqual(p.actions[0],['/api/site-data/clear','POST','clear-site-data']);assert.ok(p.actions.includes('clear-local'));assert.ok(p.actions.includes('clear-session'));assert.ok(p.actions.includes('forget-account'));assert.equal(p.actions.at(-1),'/web');
});
