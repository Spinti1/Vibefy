const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../public/youtube-account.js'),'utf8');
async function page(){
 const nodes=new Map(),grants=[],calls=[];let config,deny=false;
 const node=id=>{if(!nodes.has(id))nodes.set(id,{textContent:'',hidden:false,replaceChildren(){},append(){}});return nodes.get(id);};
 const oauth={initTokenClient(c){config=c;return {requestAccessToken(options){grants.push(options);queueMicrotask(()=>config.callback({access_token:'test-only-token',expires_in:3600,scope:deny?'':options.scope}));}};},hasGrantedAllScopes(response,scope){return response.scope.split(' ').includes(scope);},revoke(token,done){done();}};
 const google={accounts:{oauth2:oauth}},window={google,vibefyWebLibrary:{collections:()=>[],save(){}},dispatchEvent(){}};
 const fetch=async(url,options={})=>{calls.push({url:String(url),method:options.method||'GET'});return {ok:true,status:200,json:async()=>String(url)==='/api/google-config'?{clientId:'123-demo.apps.googleusercontent.com'}:String(url).includes('/userinfo')?{name:'Demo listener'}:{items:[]}};};
 vm.runInNewContext(source,{document:{getElementById:node,createElement:()=>({}),head:{append(script){queueMicrotask(()=>script.onload());}}},window,google,fetch,URL,Date,Number,String,Error,Event,AbortSignal,queueMicrotask});
 await new Promise(setImmediate);return {node,grants,calls,window,deny(){deny=true;}};
}
test('Continue with Google asks for basic identity and does not read YouTube',async()=>{const p=await page();await p.node('google-connect').onclick();assert.equal(p.grants[0].scope,'openid profile');assert.equal(p.grants[0].include_granted_scopes,false);assert.ok(p.calls.some(c=>c.url==='https://openidconnect.googleapis.com/v1/userinfo'));assert.ok(!p.calls.some(c=>c.url.includes('/youtube/v3/')));assert.match(p.node('account-identity').textContent,/Demo listener/);});
test('playlist access is a separate explicit read permission',async()=>{const p=await page();await p.node('google-connect').onclick();await p.node('youtube-refresh').onclick();assert.equal(p.grants.length,2);assert.equal(p.grants[1].scope,'https://www.googleapis.com/auth/youtube.readonly');assert.ok(p.calls.some(c=>c.url.startsWith('https://www.googleapis.com/youtube/v3/playlists?')));assert.ok(p.calls.every(c=>c.method==='GET'));});
test('denied playlist permission preserves basic sign-in and never calls the private API',async()=>{const p=await page();await p.node('google-connect').onclick();p.deny();await p.node('youtube-refresh').onclick();assert.match(p.node('youtube-status').textContent,/permission was not granted/);assert.equal(p.window.vibefyYouTube.connected,true);assert.ok(!p.calls.some(c=>c.url.includes('/youtube/v3/')));p.node('google-disconnect').onclick();assert.equal(p.window.vibefyYouTube.connected,false);});
