const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../public/player.js'),'utf8');
function player(){
 const calls=[],events={},nodes=new Map();const $=id=>{if(!nodes.has(id))nodes.set(id,{hidden:false,disabled:false,value:0,textContent:'',max:100,replaceChildren(){},classList:{toggle(){}}});return nodes.get(id);};
 const audio={paused:true,duration:0,currentTime:0,preload:'none',pause(){this.paused=true;},load(){},removeAttribute(){},play(){this.paused=false;calls.push('play');return Promise.resolve();},getAttribute(){return this.src;},addEventListener(name,fn){events[name]=fn;}};
 const media={get duration(){return audio.duration;},get currentTime(){return audio.currentTime;},set currentTime(v){audio.currentTime=v;},get paused(){return audio.paused;},pause:()=>audio.pause(),play:()=>audio.play()};
 const c={nativeAudio:false,nativeAudioSequence:0,calls,events,$,audio,media,queue:[],index:-1,optimisticSeek:null,seekingSlider:false,lyricSequence:0,lyricLines:[],loadedTrackId:null,pendingAudioSeek:null,playbackTimer:null,playbackError:false,window:{vibefyPreferences:{}},embedded:{active:false,pause(){}},soundcloud:{active:false,pause(){}},sessionStorage:{setItem(){}},Event,navigator:{},document:{dispatchEvent(){},hidden:false,querySelectorAll:()=>[]},activeOverlay:'now',text:(tag,text)=>({tag,text}),current:()=>c.queue[c.index],normalizeTrack:t=>t,hideNowPanel(){},openDialog(){},updateNow(){},renderResults(){},renderQueue(){},updatePlay(){},updatePosition(){},advance(){},addQueue(){},toast(){},setupAudio(){},audioPlayFailed(){},playbackFailed(){},setTimeout(){},probeLyrics(){},clearTimeout(){},time:s=>String(s),WebSocket:{OPEN:1},Math,Number,String,Array,Date};
 const load=source.match(/function loadCurrent\([^\n]+/)[0],bridge=source.match(/window\.vibefyPlayerBridge=(\{[\s\S]*?\n\});/)[0];
 vm.createContext(c);vm.runInContext(source.match(/function shownPosition\([^\n]+/)[0]+'\n'+source.match(/function playPause\([^\n]+/)[0]+'\n'+load+'\n'+source.match(/function seekTo\([^\n]+/)[0]+'\n'+source.match(/audio\.addEventListener\('playing'[^\n]+/)[0]+'\n'+bridge,c);
 return c;
}
const jv={id:'jv:12345678-1234-1234-1234-123456789abc',title:'Public recording',artist:'Artist',duration:200};
test('a paused JuiceVault guest loads audio at the shared position without starting playback',()=>{
 const p=player();p.window.vibefyPlayerBridge.follow(jv,42,true);
 assert.equal(p.audio.src,'https://api.juicevault.xyz/music/stream/'+jv.id.slice(3));assert.equal(p.calls.length,0);
 p.window.vibefyPlayerBridge.follow(jv,45,true);p.audio.duration=200;p.events.loadedmetadata();
 assert.equal(p.audio.currentTime,45);assert.equal(p.audio.paused,true);assert.equal(p.pendingAudioSeek,null);
});
test('selecting a filtered shared queue entry skips the correct underlying occurrence',()=>{
 const p=player();p.queue=[{id:'abcdefghijk'}, {id:'sc:123'},jv];p.index=0;
 assert.equal(p.window.vibefyPlayerBridge.control({action:'play_queue',index:0,uri:'vibefy:juicevault:'+jv.id.slice(3)}),true);assert.equal(p.index,2);
 assert.equal(p.window.vibefyPlayerBridge.control({action:'play_queue',index:0,uri:'vibefy:youtube:bbbbbbbbbbb'}),false);
});
test('shared seek requires the current recording and bounds its destination',()=>{
 const p=player();p.queue=[jv];p.index=0;p.audio.duration=200;
 assert.equal(p.window.vibefyPlayerBridge.control({action:'seek',uri:'file:///secret',position_ms:1000}),false);
 assert.equal(p.window.vibefyPlayerBridge.control({action:'seek',uri:'vibefy:juicevault:'+jv.id.slice(3),position_ms:900000}),true);assert.equal(p.audio.currentTime,200);
});

test('joining with the same remembered track still initializes its audio source',()=>{
 const p=player();p.queue=[jv];p.index=0;
 p.window.vibefyPlayerBridge.follow(jv,75,true);
 assert.equal(p.audio.src,'https://api.juicevault.xyz/music/stream/'+jv.id.slice(3));
 assert.equal(p.loadedTrackId,jv.id);p.audio.duration=200;p.events.loadedmetadata();
 assert.equal(p.audio.currentTime,75);assert.equal(p.audio.paused,true);
 p.window.vibefyPlayerBridge.follow(jv,76,true);assert.equal(p.pendingAudioSeek,null);
});

test('a participant seek updates local progress immediately and sends the bounded destination',()=>{
 const p=player();p.queue=[jv];p.index=0;p.audio.duration=200;p.audio.currentTime=10;
 const calls=[];p.window.vibefyJam={following:true,seek:(id,seconds)=>{calls.push([id,seconds]);return true;}};
 p.window.vibefyPlayerBridge.seek(75);assert.equal(p.audio.currentTime,75);assert.equal(calls[0][0],jv.id);assert.equal(calls[0][1],75);
});
test('receiving artwork for the same shared recording updates the cover without reloading its audio',()=>{
 const p=player();p.window.vibefyPlayerBridge.follow(jv,42,true);p.audio.duration=200;p.events.loadedmetadata();
 p.window.vibefyPlayerBridge.follow({...jv,image:'https://api.juicevault.xyz/cdn/covers/example.webp'},43,true);
 assert.equal(p.queue[0].image,'https://api.juicevault.xyz/cdn/covers/example.webp');assert.equal(p.pendingAudioSeek,null);assert.equal(p.audio.currentTime,43);
});

test('guest playback toggles send explicit shared intent and update local playback',()=>{
 const p=player();p.queue=[jv];p.index=0;p.audio.duration=200;p.audio.paused=false;
 const sent=[];p.window.vibefyJam={following:true,setPlaying:(id,playing)=>{sent.push(playing);return true;}};
 vm.runInContext('playPause();playPause();',p);assert.deepEqual(sent,[false,true]);assert.equal(p.audio.paused,false);
 p.window.vibefyJam.setPlaying=()=>false;vm.runInContext('playPause()',p);assert.equal(p.audio.paused,false);
});
test('host shared playback commands are idempotent and require a matching recording',()=>{
 const p=player();p.queue=[jv];p.index=0;p.audio.paused=false;const uri='vibefy:juicevault:'+jv.id.slice(3);
 for(let i=0;i<2;i++)assert.equal(p.window.vibefyPlayerBridge.control({action:'set_playing',uri,playing:false}),true);
 assert.equal(p.audio.paused,true);assert.equal(p.window.vibefyPlayerBridge.control({action:'set_playing',uri:'vibefy:youtube:abcdefghijk',playing:true}),false);
 assert.equal(p.window.vibefyPlayerBridge.control({action:'set_playing',uri,playing:'false'}),false);
});

test('paused snapshots align exactly without repeatedly seeking a stable paused player',()=>{
 const p=player();p.window.vibefyPlayerBridge.follow(jv,40,true);p.audio.duration=200;p.events.loadedmetadata();
 p.window.vibefyPlayerBridge.follow(jv,41.3,true);assert.equal(p.audio.currentTime,41.3);
 p.window.vibefyPlayerBridge.follow(jv,41.3,true);assert.equal(p.audio.paused,true);assert.equal(p.audio.currentTime,41.3);
});

test('an asynchronous YouTube seek never snaps the visible slider back to its old position',()=>{
 const p=player();p.queue=[jv];p.index=0;p.audio.duration=200;p.audio.currentTime=110;p.audio.paused=true;
 let destination;Object.defineProperty(p.media,'currentTime',{get:()=>p.audio.currentTime,set:value=>{destination=value;}});
 p.window.vibefyJam={following:true,seek:()=>true};p.window.vibefyPlayerBridge.seek(67);
 vm.runInContext(source.match(/function updatePosition\([^\n]+/)[0]+'\nupdatePosition();',p);
 assert.equal(destination,67);assert.equal(p.audio.currentTime,110);assert.equal(p.$('seek').value,67);assert.equal(p.$('mini-seek').value,67);
 vm.runInContext('updatePosition();',p);assert.equal(p.$('seek').value,67);
 p.audio.currentTime=67;vm.runInContext('updatePosition();',p);assert.equal(p.$('seek').value,67);assert.equal(p.optimisticSeek,null);
});

test('desktop and phone play the original clip ID while presenting cover artwork',async()=>{
 for(const phone of [false,true]){
  const p=player(),loads=[];p.useEmbedded=true;p.window.vibefyPhone=phone;
  p.embedded.load=(...args)=>{loads.push(args);return Promise.resolve();};
  p.queue=[{id:'abcdefghijk',title:'Clip-only recording',image:'https://i.ytimg.com/vi/abcdefghijk/hqdefault.jpg'}];p.index=0;
  vm.runInContext('loadCurrent(true,42)',p);
  assert.equal(loads[0][0],'abcdefghijk');assert.equal(loads[0][1],true);assert.equal(loads[0][2],42);
  assert.equal(p.$('now-cover').hidden,false);assert.equal(p.embedded.active,true);
 }
});

function nativePlayer(){const p=player(),resolutions=[],plays=[];p.nativeAudio=true;p.embedded.load=()=>{throw Error('Video player must not load');};p.window.vibefyMobileAudio={resolve:id=>new Promise((resolve,reject)=>resolutions.push({id,resolve,reject}))};const begin=source.indexOf('const media = {'),end=source.indexOf("const KEY =",begin);vm.runInContext(source.slice(begin,end).replace('const media =','media ='),p);p.resolutions=resolutions;return p;}
const flush=async()=>{await Promise.resolve();await Promise.resolve();};
test('native YouTube playback resolves only audio and preserves paused Jam position',async()=>{const p=nativePlayer();p.queue=[{id:'abcdefghijk',title:'Clip'}];p.index=0;vm.runInContext('loadCurrent(true,42)',p);assert.equal(p.resolutions[0].id,'abcdefghijk');assert.equal(p.embedded.active,false);assert.equal(p.$('youtube-frame').hidden,true);assert.equal(p.$('now-cover').hidden,false);p.resolutions[0].resolve('https://x.googlevideo.com/audio?itag=140');await flush();assert.equal(p.audio.src,'https://x.googlevideo.com/audio?itag=140');assert.equal(p.audio.paused,true);p.audio.duration=200;p.events.loadedmetadata();assert.equal(p.audio.currentTime,42);assert.equal(p.audio.paused,true)});
test('host pause during source resolution prevents a late automatic resume',async()=>{const p=nativePlayer();p.queue=[{id:'abcdefghijk'}];p.index=0;vm.runInContext('loadCurrent(false,20)',p);p.media.pause();p.resolutions[0].resolve('https://x.googlevideo.com/audio');await flush();assert.equal(p.audio.paused,true);p.audio.duration=200;p.events.loadedmetadata();assert.equal(p.audio.paused,true);assert.equal(p.audio.currentTime,20)});
test('late source responses and failures never start a hidden video or replace a new track',async()=>{const p=nativePlayer();p.queue=[{id:'abcdefghijk'},{id:'zyxwvutsrqp'}];p.index=0;vm.runInContext('loadCurrent()',p);p.index=1;vm.runInContext('loadCurrent()',p);p.resolutions[0].resolve('https://x.googlevideo.com/old');await flush();assert.notEqual(p.audio.src,'https://x.googlevideo.com/old');p.resolutions[1].reject(Error('Unavailable audio'));await flush();assert.equal(p.embedded.active,false);assert.equal(p.$('youtube-frame').hidden,true)});
