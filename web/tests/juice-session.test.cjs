const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../public/web-library.js'),'utf8');
async function page(store,unavailable=false){
 const nodes=new Map(),requests=[];const node=id=>{if(!nodes.has(id))nodes.set(id,{value:'',hidden:false,disabled:false,open:false,textContent:'',replaceChildren(){},append(){},close(){this.open=false;},showModal(){this.open=true;}});return nodes.get(id);};
 const window={vibefyBrowserSessions:{juicevault:store},vibefyWebLibrary:{toast(){}},addEventListener(){},dispatchEvent(){}};
 const fetch=async(url,options={})=>{requests.push({url,options});if(unavailable)throw Error('Temporary connection failure');return {ok:true,json:async()=>url.endsWith('/login')?{accessToken:'returned-juice-token'}:{data:[]}};};
 vm.runInNewContext(source,{window,document:{getElementById:node,addEventListener(){}},crypto:{},URL,AbortSignal,fetch,Event});await new Promise(setImmediate);return {window,node,requests};
}
function storage(){return {value:null,async read(){return this.value;},async write(value){this.value=structuredClone(value);},async clear(){this.value=null;}};}
test('JuiceVault login saves no password and restores account and library on reopen',async()=>{
 const store=storage(),p=await page(store);p.node('juice-name').value='Example';p.node('juice-password').value='do-not-save';await p.node('juice-login-form').onsubmit({preventDefault(){},submitter:{}});
 assert.equal(store.value.access_token,'returned-juice-token');assert.equal(store.value.username,'Example');assert.ok(!JSON.stringify(store.value).includes('do-not-save'));assert.equal(p.node('juice-password').value,'');
 const reopened=await page(store);assert.equal(reopened.window.vibefyJuice.connected,true);assert.equal(reopened.node('juice-login-form').hidden,true);assert.ok(reopened.requests.some(r=>r.url.includes('playlists?username=Example')&&r.options.headers.Authorization==='Bearer returned-juice-token'));
});
test('temporary library errors preserve the saved JuiceVault account; explicit logout clears it',async()=>{
 const store=storage();store.value={version:1,access_token:'saved-token',username:'Example'};const p=await page(store,true);assert.equal(p.window.vibefyJuice.connected,true);assert.equal(store.value.access_token,'saved-token');assert.match(p.node('juice-status').textContent,/Account saved/);
 p.window.vibefyJuice.forget();await new Promise(setImmediate);assert.equal(p.window.vibefyJuice.connected,false);assert.equal(store.value,null);
});
