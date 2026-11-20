const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../public/jam.js'),'utf8');
function page(options={}){const nodes=new Map(),connections=[],calls=[],timers=[];const node=id=>{if(options.noIndicator&&id==="jam-indicator")return null;if(!nodes.has(id))nodes.set(id,{hidden:false,disabled:false,value:'',textContent:'',click(){}});return nodes.get(id);};class WS{static OPEN=1;constructor(url){this.url=url;this.readyState=1;this.sent=[];connections.push(this);}send(raw){this.sent.push(JSON.parse(raw));}close(){this.readyState=3;}receive(p){this.onmessage({data:JSON.stringify(p)});}}
 const bridge={snapshot:()=>({track:options.track||{id:'abcdefghijk',title:'Song',artist:'Artist',duration:200},position:12,paused:false}),queue:()=>[],followMode:v=>calls.push(['mode',v]),follow:(...v)=>calls.push(['follow',...v]),enqueue:t=>calls.push(['add',t]),control:p=>{calls.push(['control',p]);return true;},refreshQueue(){},pause(){}};
 const window={vibefyPlayerBridge:bridge,vibefyWebLibrary:{toast:s=>calls.push(["toast",s])},addEventListener(){}};
 vm.runInNewContext(source,{window,document:{getElementById:node,querySelector:()=>node('dialog')},WebSocket:WS,location:{search:options.search||'',origin:'https://vibefy-neon.vercel.app'},URL,URLSearchParams,navigator:{clipboard:{writeText:async()=>{}}},Date,AbortSignal,setInterval:()=>1,clearInterval(){},setTimeout:(fn,ms)=>{const t={fn,ms};timers.push(t);return t;},clearTimeout(t){if(t)t.cancelled=true;},fetch:async()=>({ok:true,json:async()=>({code:'ABCDEFGHJKLM',host_token:'a'.repeat(64)})})});return {node,connections,calls,window,timers};}

test('a browser host uses the shared relay protocol and keeps the host secret out of its link',async()=>{const p=page();await p.node('jam-host').onclick();const ws=p.connections[0];assert.match(ws.url,/wss:\/\/vibefy-neon\.vercel\.app\/api\/jam\/rooms\/ABCDEFGHJKLM$/);ws.onopen();assert.equal(ws.sent[0].protocol,2);assert.equal(ws.sent[0].host_token,'a'.repeat(64));assert.equal(p.node('jam-invite').value,'https://vibefy-neon.vercel.app/web?jam=ABCDEFGHJKLM');ws.receive({type:'welcome',role:'host'});assert.equal(ws.sent.find(v=>v.type==='state').snapshot.track.uri,'vibefy:youtube:abcdefghijk');ws.receive({type:'add',track:{uri:'vibefy:youtube:bbbbbbbbbbb',title:'Next',artists:[{name:'Artist'}],duration_ms:200000}});assert.equal(p.calls.find(c=>c[0]==='add')[1].id,'bbbbbbbbbbb');p.node('jam-leave').onclick();assert.equal(ws.sent.at(-1).type,'end');});

test('a browser guest follows native timing and forwards additions without host credentials',()=>{const p=page();p.node('jam-code').value='abcd-efgh-jklm';p.node('jam-join').onclick();const ws=p.connections[0];ws.onopen();assert.equal(ws.sent[0].host_token,null);ws.receive({type:'welcome',role:'guest'});ws.receive({type:'state',snapshot:{track:{uri:'vibefy:youtube:abcdefghijk',title:'Song',artists:[{name:'Artist'}],duration_ms:200000},position_ms:24000,playing:false},queue:[]});const follow=p.calls.find(c=>c[0]==='follow');assert.equal(follow[1].id,'abcdefghijk');assert.equal(follow[2],24);assert.equal(follow[3],true);assert.equal(p.window.vibefyJam.add({id:'bbbbbbbbbbb',title:'Next'}),true);assert.equal(ws.sent.at(-1).type,'add');p.node('jam-leave').onclick();assert.ok(!ws.sent.some(p=>p.type==='end'));});

test('a pasted external URL cannot choose the relay or open a peer address',()=>{const p=page();p.node('jam-code').value='https://usevibefy.online.evil/app?jam=ABCDEFGHJKLM';p.node('jam-join').onclick();assert.equal(p.connections.length,0);assert.match(p.node('jam-status').textContent,/Paste a Vibefy/);});

test('a phone invitation joins even if a decorative connection indicator is missing',()=>{
 const p=page({noIndicator:true,search:'?jam=ABCDEFGHJKLM'});
 assert.equal(p.node('jam-code').value,'ABCDEFGHJKLM');
 p.node('jam-join').onclick();assert.equal(p.connections.length,1);
 const ws=p.connections[0];ws.onopen();ws.receive({type:'welcome',role:'guest'});
 ws.receive({type:'state',listeners:1,snapshot:null,queue:[]});
 assert.match(p.node('jam-status').textContent,/Joined session.*waiting/);
 assert.ok(p.calls.some(c=>c[0]==='toast'&&c[1].startsWith('Joined the Jam')));
});
test('JuiceVault playback and shared controls retain source IDs without relaying credentials',async()=>{
 const id='jv:12345678-1234-1234-1234-123456789abc',uri='vibefy:juicevault:'+id.slice(3);
 const host=page({track:{id,title:'Public recording',artist:'Artist',duration:200}});await host.node('jam-host').onclick();const h=host.connections[0];h.onopen();h.receive({type:'welcome',role:'host'});
 assert.equal(h.sent.find(p=>p.type==='state').snapshot.track.uri,uri);
 const guest=page();guest.node('jam-code').value='ABCDEFGHJKLM';guest.node('jam-join').onclick();const g=guest.connections[0];g.onopen();g.receive({type:'welcome',role:'guest'});
 const track={uri,title:'Public recording',artists:[{name:'Artist'}],duration_ms:200000};
 g.receive({type:'state',shared_controls:true,listeners:1,snapshot:{track,position_ms:45000,playing:false},queue:[track]});
 assert.equal(guest.calls.find(c=>c[0]==='follow')[1].id,id);
 assert.equal(guest.window.vibefyJam.seek(id,60),true);assert.deepEqual(g.sent.at(-1),{type:'control',action:'seek',uri,position_ms:60000});
 assert.equal(guest.window.vibefyJam.playQueue(0),true);assert.equal(g.sent.at(-1).uri,uri);
 assert.equal(guest.window.vibefyJam.playQueue(5),false);assert.equal(guest.window.vibefyJam.seek('jv:../private',2),false);
 h.receive({type:'control',action:'seek',uri,position_ms:60000});assert.ok(host.calls.some(c=>c[0]==='control'));
 assert.doesNotMatch(JSON.stringify(g.sent),/access_token|Authorization|password/);
});
test('legacy hosts without shared control capability remain read-only',()=>{
 const p=page();p.node('jam-code').value='ABCDEFGHJKLM';p.node('jam-join').onclick();const ws=p.connections[0];ws.onopen();ws.receive({type:'welcome',role:'guest'});ws.receive({type:'state',snapshot:null,queue:[]});
 assert.equal(p.window.vibefyJam.seek('abcdefghijk',10),false);
});
test('joining listeners notify the host once per count increase, without repeated state toasts',async()=>{
 const p=page();await p.node('jam-host').onclick();const ws=p.connections[0];ws.onopen();ws.receive({type:'welcome',role:'host'});
 for(const listeners of [0,1,1,0,2])ws.receive({type:'state',listeners,queue:[],snapshot:null});
 assert.deepEqual(p.calls.filter(c=>c[0]==='toast').map(c=>c[1]),['A listener joined your Jam.','2 listeners joined your Jam.']);
 assert.equal(p.node('jam-notice').hidden,false);assert.equal(p.node('jam-notice').textContent,'2 listeners joined your Jam.');
});
test('a stalled handshake closes after twelve seconds and retries through the relay',()=>{
 const p=page();p.node('jam-code').value='ABCDEFGHJKLM';p.node('jam-join').onclick();
 p.timers.find(t=>t.ms===12000).fn();assert.equal(p.connections[0].readyState,3);
 p.connections[0].onclose({code:1006});p.timers.find(t=>t.ms===500).fn();
 assert.match(p.connections[1].url,/^wss:\/\/vibefy-jam\.pex-am\.workers\.dev\/rooms\//);
});
test('replacing a Jam icon preserves the live session indicator',()=>{
 const ui=fs.readFileSync(require('node:path').join(__dirname,'../public/native-ui.js'),'utf8');
 const helper=ui.slice(ui.indexOf('function setIcon('),ui.indexOf('function labelIcon('));
 const indicator={id:'jam-indicator',hidden:true},children=[{oldIcon:true},indicator];
 const node={querySelector:s=>s==='#jam-indicator'?children.find(c=>c.id==='jam-indicator'):null,replaceChildren(...items){children.splice(0,children.length,...items);},append(item){children.push(item);}};
 vm.runInNewContext(helper+';setIcon(node,"users");',{node,icon:name=>({name})});
 assert.equal(children[1],indicator);assert.equal(children[0].name,'users');
});

test('control acknowledgements stay visible after a later clock update and leaving clears permissions',()=>{
 const p=page();p.node('jam-code').value='ABCDEFGHJKLM';p.node('jam-join').onclick();const ws=p.connections[0];ws.onopen();ws.receive({type:'welcome',role:'guest'});
 ws.receive({type:'notice',message:'Seeking the shared track…'});
 ws.receive({type:'state',snapshot:null,queue:[],shared_controls:true,listeners:1});
 assert.equal(p.node('jam-notice').textContent,'Seeking the shared track…');
 p.node('jam-leave').onclick();assert.equal(p.window.vibefyJam.sharedControls,false);assert.equal(p.window.vibefyJam.queue.length,0);
});

test('participant pause reaches the host, ignores stale playing packets and accepts confirmation',()=>{
 const p=page();p.node('jam-code').value='ABCDEFGHJKLM';p.node('jam-join').onclick();const ws=p.connections[0];ws.onopen();ws.receive({type:'welcome',role:'guest'});
 const snapshot={track:{uri:'vibefy:youtube:abcdefghijk',title:'Song',artists:[],duration_ms:200000},position_ms:12000,playing:true};
 ws.receive({type:'state',snapshot,queue:[],shared_controls:true});const before=p.calls.filter(c=>c[0]==='follow').length;
 assert.equal(p.window.vibefyJam.setPlaying('abcdefghijk',false),true);
 assert.equal(ws.sent.at(-1).action,'set_playing');assert.equal(ws.sent.at(-1).playing,false);
 ws.receive({type:'state',snapshot,queue:[],shared_controls:true});assert.equal(p.calls.filter(c=>c[0]==='follow').length,before);
 ws.receive({type:'state',snapshot:{...snapshot,playing:false},queue:[],shared_controls:true});assert.equal(p.calls.at(-1)[3],true);
 assert.equal(p.window.vibefyJam.setPlaying('abcdefghijk','false'),false);
});
test('stale snapshots cannot rewind an optimistic participant seek',()=>{
 const p=page();p.node('jam-code').value='ABCDEFGHJKLM';p.node('jam-join').onclick();const ws=p.connections[0];ws.onopen();ws.receive({type:'welcome',role:'guest'});
 const snapshot={track:{uri:'vibefy:youtube:abcdefghijk',title:'Song',artists:[],duration_ms:200000},position_ms:12000,playing:true};
 ws.receive({type:'state',snapshot,queue:[],shared_controls:true});assert.equal(p.window.vibefyJam.seek('abcdefghijk',75),true);const count=p.calls.filter(c=>c[0]==='follow').length;
 ws.receive({type:'state',snapshot,queue:[],shared_controls:true});assert.equal(p.calls.filter(c=>c[0]==='follow').length,count);
 ws.receive({type:'state',snapshot:{...snapshot,position_ms:75000},queue:[],shared_controls:true});assert.equal(p.calls.at(-1)[2],75);
 // A new song cancels the old intent immediately.
 p.window.vibefyJam.seek('abcdefghijk',100);ws.receive({type:'state',snapshot:{...snapshot,track:{...snapshot.track,uri:'vibefy:youtube:bbbbbbbbbbb'}},queue:[],shared_controls:true});assert.equal(p.calls.at(-1)[1].id,'bbbbbbbbbbb');
});
