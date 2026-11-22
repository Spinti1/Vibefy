const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../public/youtube-account.js'),'utf8');
async function page(options={}){
 options.collections ||= [];
 const nodes=new Map(),grants=[],calls=[],timers=[],responses=[];let config,deny=false,popupError=false;
 const makeNode=()=>({textContent:'',hidden:false,disabled:false,open:false,dataset:{},children:[],events:{},classList:{toggle(){}},setAttribute(){},addEventListener(name,fn){this.events[name]=fn;},replaceChildren(){this.children=[];},append(...children){this.children.push(...children);},showModal(){this.open=true;},close(){this.open=false;},remove(){}});
 const node=id=>{if(!nodes.has(id))nodes.set(id,makeNode());return nodes.get(id);};
 const oauth={initTokenClient(c){config=c;return {requestAccessToken(request){grants.push(request);responses.push({success:()=>c.callback({access_token:'test-only-token',expires_in:3600,scope:deny?'':request.scope}),error:()=>c.error_callback({type:'popup_closed'})});if(!options.hold)queueMicrotask(()=>popupError?c.error_callback({type:'popup_closed'}):c.callback({access_token:'test-only-token',expires_in:3600,scope:deny?'':request.scope}));}};},hasGrantedAllScopes(response,...scopes){return scopes.every(scope=>response.scope.split(' ').includes(scope));},revoke(token,done){done();}};
 const google={accounts:{oauth2:oauth}},window={google,vibefyWebLibrary:{collections:()=>options.collections||[],save(c){options.saved?.push(c);const index=options.collections.findIndex(v=>v.id===c.id);if(index<0)options.collections.push(c);else options.collections[index]=c;},art:value=>value},dispatchEvent(){}};
 const fetch=async(url,request={})=>{calls.push({url:String(url),method:request.method||'GET'});if(options.api&&String(url).includes('/youtube/v3/'))return options.api(new URL(url),request);return {ok:true,status:200,json:async()=>String(url)==='/api/google-config'?{clientId:options.missing?'':'123-demo.apps.googleusercontent.com'}:String(url).includes('/userinfo')?{name:'Demo listener'}:{items:options.playlists||[]}};};
 vm.runInNewContext(source,{crypto:require('node:crypto').webcrypto,Image:function(){return makeNode();},document:{getElementById:node,createElement:makeNode,head:{append(script){queueMicrotask(()=>script.onload());}}},window,google,fetch,URL,Date,Number,String,Error,Event,AbortSignal,queueMicrotask,setTimeout(fn,ms){const t={fn,ms};timers.push(t);return t;},clearTimeout(){}});
 await new Promise(setImmediate);return {node,hasNode:id=>nodes.has(id),grants,calls,window,timers,responses,deny(){deny=true;},closePopup(){popupError=true;}};
}
test('one explicit Connect YouTube click requests read access and loads playlists automatically',async()=>{const p=await page();await p.node('google-connect').onclick();assert.equal(p.grants.length,1);assert.equal(p.grants[0].include_granted_scopes,false);assert.match(p.grants[0].scope,/youtube\.readonly/);assert.doesNotMatch(p.grants[0].scope,/force-ssl/);assert.ok(p.calls.some(c=>c.url.includes('/youtube/v3/playlists?')));assert.match(p.node('account-identity').textContent,/Demo listener/);assert.equal(p.node('header-connect-label').textContent,'YouTube connected');assert.equal(p.node('header-connect').disabled,false);});
test('refresh reuses an approved grant and connected buttons open account without another popup',async()=>{const p=await page();await p.node('google-connect').onclick();await p.node('youtube-refresh').onclick();await p.node('header-connect').onclick();assert.equal(p.grants.length,1);assert.ok(p.calls.every(c=>c.method==='GET'));});
test('denied read permission never marks the account connected or reads the private API',async()=>{const p=await page();p.deny();await p.node('google-connect').onclick();assert.match(p.node('youtube-status').textContent,/permission was not granted/);assert.equal(p.window.vibefyYouTube.connected,false);assert.ok(!p.calls.some(c=>c.url.includes('/youtube/v3/')));assert.equal(p.node('header-connect').disabled,false);});
test('closing the popup clears loading and permits retry',async()=>{const p=await page();p.closePopup();await p.node('google-connect').onclick();assert.match(p.node('youtube-status').textContent,/closed or blocked/);assert.equal(p.node('header-connect').disabled,false);assert.equal(p.node('google-connect-label').textContent,'Continue to Google');});
test('missing publisher configuration exposes retry and does not pretend to connect',async()=>{const p=await page({missing:true});assert.equal(p.node('header-connect').disabled,false);assert.equal(p.node('google-connect').disabled,false);assert.equal(p.node('google-connect-label').textContent,'Retry connection');assert.equal(p.window.vibefyYouTube.connected,false);});
test('disconnect clears the account state without changing the local library',async()=>{const p=await page();await p.node('google-connect').onclick();p.node('google-disconnect').onclick();assert.equal(p.window.vibefyYouTube.connected,false);assert.equal(p.node('header-connect-label').textContent,'Connect YouTube');assert.match(p.node('youtube-status').textContent,/preserved/);});
test('cancelling restores controls and late callbacks cannot cancel a newer connection',async()=>{const p=await page({hold:true});const first=p.node('google-connect').onclick();p.node('account-dialog').events.close();await first;assert.equal(p.node('header-connect').disabled,false);assert.match(p.node('youtube-status').textContent,/cancelled/);const second=p.node('google-connect').onclick();p.responses[0].error();assert.equal(p.node('header-connect').disabled,true);p.responses[1].success();await second;assert.equal(p.window.vibefyYouTube.connected,true);});

test('toolbar opens the explanation without requesting Google access',async()=>{const p=await page();await p.node('header-connect').onclick();assert.equal(p.node('account-dialog').open,true);assert.equal(p.grants.length,0);assert.ok(!p.calls.some(c=>c.url.includes('/youtube/v3/')));});
test('connected account playlists appear only on Home and the account modal closes',async()=>{const p=await page({playlists:[{id:'PLabcdefghijklmnop',snippet:{title:'Demo playlist',thumbnails:{}},contentDetails:{itemCount:3}}]});await p.node('google-connect').onclick();assert.equal(p.node('youtube-playlists').children.length,0);assert.equal(p.window.vibefyWebLibrary.collections()[0].id,'youtube:PLabcdefghijklmnop');assert.equal(p.hasNode('account-playlists'),false);assert.equal(p.node('account-dialog').open,false);});

test('connection automatically saves complete paginated playlists locally, strips Topic and preserves channel metadata',async()=>{
 const saved=[],items=['abcdefghijk','lmnopqrstuv'];
 const p=await page({saved,api:async url=>({ok:true,status:200,json:async()=>url.pathname.endsWith('/playlists')?{items:[{id:'PLautomatic12345',snippet:{title:'Example'},contentDetails:{itemCount:2}}]}:{items:[{contentDetails:{videoId:items[url.searchParams.has('pageToken')?1:0]},snippet:{title:'A recording',videoOwnerChannelTitle:'Example Band - Topic',videoOwnerChannelId:'UC'+'a'.repeat(22)}}],...(url.searchParams.has('pageToken')?{}:{nextPageToken:'second'})}})});
 await p.node('google-connect').onclick();assert.equal(saved.length,1);assert.equal(saved[0].tracks.length,2);assert.equal(saved[0].tracks[0].artist,'Example Band');assert.equal(saved[0].tracks[0].channelId,'UC'+'a'.repeat(22));assert.equal(p.node('youtube-playlists').children.length,0);assert.ok(p.calls.every(c=>c.method==='GET'),'loading playlists must not create or change account playlists');
});

 test('partial YouTube saves resume the existing copy without duplicate songs',async()=>{
 let creates=0,fail=true;const remote=new Set(),written=[];
 const p=await page({api:async(url,request)=>{
  const body=request.body?JSON.parse(request.body):{};
  if(url.pathname.endsWith('/playlists')&&request.method==='POST'){creates++;return {ok:true,status:200,json:async()=>({id:'PLexamplecopy'})};}
  if(url.pathname.endsWith('/playlists'))return {ok:true,status:200,json:async()=>({items:url.searchParams.get('id')?[{id:url.searchParams.get('id')}]:[]})};
  if(url.pathname.endsWith('/playlistItems')&&request.method==='POST'){
   const id=body.snippet.resourceId.videoId;
   if(id==='bbbbbbbbbbb'&&fail){fail=false;return {ok:false,status:503,json:async()=>({})};}
   written.push(id);remote.add(id);return {ok:true,status:200,json:async()=>({id:'entry'})};
  }
  return {ok:true,status:200,json:async()=>({items:[...remote].map(videoId=>({contentDetails:{videoId}}))})};
 }});
 await p.node('google-connect').onclick();
 const list={id:'local:demo',title:'Example',tracks:['aaaaaaaaaaa','bbbbbbbbbbb','ccccccccccc'].map(id=>({id}))};
 await assert.rejects(p.window.vibefyYouTube.save(list));assert.equal(list.youtubeId,'PLexamplecopy');assert.equal(remote.size,1);
 await p.window.vibefyYouTube.save(list);assert.equal(creates,1);assert.deepEqual(written,['aaaaaaaaaaa','bbbbbbbbbbb','ccccccccccc']);
 await p.window.vibefyYouTube.save(list);assert.equal(written.length,3);
 });

test('refresh updates an imported YouTube copy without replacing the complete local original',async()=>{
 const local={id:'local:original',youtubeId:'PLexamplecopy',tracks:[{id:'aaaaaaaaaaa'},{id:'bbbbbbbbbbb'}]},remote={id:'youtube:PLexamplecopy',youtubeId:'PLexamplecopy',tracks:[]},saved=[];
 const p=await page({collections:[local,remote],saved,api:async url=>({ok:true,status:200,json:async()=>url.pathname.endsWith('/playlists')?{items:[{id:'PLexamplecopy',snippet:{title:'Example'},contentDetails:{itemCount:1}}]}:{items:[{snippet:{title:'Example',videoOwnerChannelTitle:'Artist'},contentDetails:{videoId:'aaaaaaaaaaa'}}]}})});
 await p.node('google-connect').onclick();assert.equal(saved.length,1);assert.equal(saved[0].id,remote.id);assert.equal(saved[0].tracks.length,1);assert.equal(local.tracks.length,2);
});


test('saving a deleted YouTube copy recreates it once and keeps all local songs',async()=>{
 let creates=0;const remote=new Set(),saved=[];
 const p=await page({saved,api:async(url,request)=>{
  if(url.pathname.endsWith('/playlists')&&request.method==='POST'){creates++;return {ok:true,status:200,json:async()=>({id:'PLrecreated'})};}
  if(url.pathname.endsWith('/playlists'))return {ok:true,status:200,json:async()=>({items:url.searchParams.get('id')==='PLrecreated'?[{id:'PLrecreated'}]:[]})};
  if(request.method==='POST'){const b=JSON.parse(request.body);remote.add(b.snippet.resourceId.videoId);}
  return {ok:true,status:200,json:async()=>({items:[...remote].map(videoId=>({contentDetails:{videoId}}))})};
 }});
 await p.node('google-connect').onclick();
 const list={id:'local:deleted-test',title:'Example',youtubeId:'PLdeleted',tracks:[{id:'aaaaaaaaaaa'},{id:'bbbbbbbbbbb'}]};
 await p.window.vibefyYouTube.save(list);assert.equal(list.youtubeId,'PLrecreated');assert.equal(creates,1);assert.equal(remote.size,2);assert.equal(list.tracks.length,2);
 await p.window.vibefyYouTube.save(list);assert.equal(creates,1);assert.equal(remote.size,2);
});

test('updating an existing copy appends only new songs and preserves YouTube-only songs',async()=>{
 let creates=0;const remote=new Set(['aaaaaaaaaaa','ccccccccccc']),writes=[];
 const p=await page({api:async(url,request)=>{
  if(url.pathname.endsWith('/playlists')){if(request.method==='POST')creates++;return {ok:true,status:200,json:async()=>({items:url.searchParams.get('id')?[{id:'PLexisting'}]:[]})};}
  if(request.method==='POST'){const id=JSON.parse(request.body).snippet.resourceId.videoId;writes.push(id);remote.add(id);}
  assert.notEqual(request.method,'DELETE');
  return {ok:true,status:200,json:async()=>({items:[...remote].map(videoId=>({contentDetails:{videoId}}))})};
 }});
 await p.node('google-connect').onclick();const list={id:'local:additions',title:'Example',youtubeId:'PLexisting',tracks:[{id:'aaaaaaaaaaa'},{id:'bbbbbbbbbbb'},{id:'bbbbbbbbbbb'}]};
 await p.window.vibefyYouTube.save(list);assert.equal(creates,0);assert.deepEqual(writes,['bbbbbbbbbbb']);assert.equal(remote.size,3);
 await p.window.vibefyYouTube.save(list);assert.equal(writes.length,1);
});

for(const failure of [403,429,503,'network','malformed'])test(`existing copy is retained without recreation on ${failure}`,async()=>{
 let creates=0;
 const p=await page({api:async(url,request)=>{
  if(request.method==='POST'){creates++;return {ok:true,status:200,json:async()=>({id:'PLunexpected'})};}
  if(url.searchParams.has('id')){
   if(failure==='network')throw Error('Network unavailable');
   return {ok:failure==='malformed',status:typeof failure==='number'?failure:200,json:async()=>({})};
  }
  return {ok:true,status:200,json:async()=>({items:[]})};
 }});
 await p.node('google-connect').onclick();const list={id:'local:guard',title:'Example',youtubeId:'PLexisting',tracks:[{id:'aaaaaaaaaaa'}]};
 await assert.rejects(p.window.vibefyYouTube.save(list));assert.equal(list.youtubeId,'PLexisting');assert.equal(creates,0);assert.equal(list.tracks.length,1);
});


test('an uncertain creation is recovered by its saved marker without another create request',async()=>{
 let creates=0,description='',exists=true;
 const p=await page({api:async(url,request)=>{
  if(request.method==='POST'&&url.pathname.endsWith('/playlists')){creates++;description=JSON.parse(request.body).snippet.description;throw Error('Response lost');}
  if(url.searchParams.get('mine')==='true')return {ok:true,status:200,json:async()=>({items:exists?[{id:'PLrecovered',snippet:{description}}]:[]})};
  return {ok:true,status:200,json:async()=>({items:[]})};
 }});
 await p.node('google-connect').onclick();const list={id:'local:uncertain',title:'Example',tracks:[]};
 await assert.rejects(p.window.vibefyYouTube.save(list));assert.equal(list.youtubeCreating,true);assert.equal(creates,1);
 await p.window.vibefyYouTube.save(list);assert.equal(list.youtubeId,'PLrecovered');assert.equal(creates,1);assert.equal(list.youtubeCreating,undefined);
});

test('an unconfirmed creation is not blindly repeated when the library has no confirmed copy',async()=>{
 let creates=0;
 const p=await page({api:async(url,request)=>{
  if(request.method==='POST'){creates++;throw Error('Response lost');}
  return {ok:true,status:200,json:async()=>({items:[]})};
 }});
 await p.node('google-connect').onclick();const list={id:'local:uncertain-none',title:'Example',tracks:[]};
 await assert.rejects(p.window.vibefyYouTube.save(list));await assert.rejects(p.window.vibefyYouTube.save(list),/not confirmed/);assert.equal(creates,1);assert.equal(list.youtubeCreating,true);
});
