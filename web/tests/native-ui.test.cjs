const {test}=require('node:test');
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const read=name=>fs.readFileSync(path.join(__dirname,'../public/',name),'utf8');
test('all native UI icon references resolve to the bundled SVG sprite',()=>{
 const ui=read('native-ui.js'),sprite=read('assets/ui-icons.svg');const names=vm.runInNewContext(ui.match(/const names=new Set\((\[[^;]+\])\)/)[1]);
 for(const name of names)assert.ok(sprite.includes('id="'+name+'"'),'Missing icon: '+name);
 assert.doesNotMatch(sprite,/<script|foreignObject|onload=|https?:\/\/(?!www\.w3\.org)/);
 assert.ok(read('assets/ui-icons-LICENSE.txt').includes('ISC License'));
});

test('play and pause draw their own visible paths without loading an external SVG',()=>{
 const nodes=()=>({attrs:{},children:[],setAttribute(k,v){this.attrs[k]=v;},classList:{add(){}},append(n){this.children.push(n);}});
 const context={names:new Set(['play-filled','pause-filled']),document:{createElementNS:(ns,tag)=>({...nodes(),tag})}};
 vm.createContext(context);const ui=read('native-ui.js');vm.runInContext(ui.slice(ui.indexOf(' function icon('),ui.indexOf(' function setIcon(')),context);
 for(const name of ['play-filled','pause-filled']){const icon=context.icon(name);assert.equal(icon.attrs.fill,'currentColor');assert.equal(icon.children[0].tag,'path');assert.ok(icon.children[0].attrs.d.length>10);}
});
test('YouTube volume follows the visible official player, including volume chosen before ready',async()=>{
 const volumes=[],cues=[];let plays=0;const player={setVolume:n=>volumes.push(n),unMute(){},cueVideoById:t=>cues.push(t),loadVideoById(){plays++;},getPlayerState:()=>2};
 const window={};const document={hidden:false,getElementById:()=>({open:false}),addEventListener(){},createElement:()=>({}),head:{append(){queueMicrotask(()=>window.onYouTubeIframeAPIReady());}}};
 const YT={Player:function(id,options){queueMicrotask(()=>options.events.onReady({target:player}));return player;}};
 vm.runInNewContext(read('embedded-player.js'),{window,document,YT,location:{origin:'https://example.test'},console,Promise,Number,Math,Error,queueMicrotask,setTimeout:()=>0,clearTimeout(){}});
 const api=window.vibefyEmbedded;api.setVolume(37);await api.load('abcdefghijk');assert.deepEqual(volumes,[37]);assert.equal(cues.length,1);assert.equal(plays,0,'a closed player must only cue, not play');
 api.setVolume(500);api.setVolume(-1);api.setVolume(NaN);assert.deepEqual(volumes,[37,100,0]);
});
test('SoundCloud widget receives volume on readiness and later changes',async()=>{
 const window={},callbacks={},volumes=[];const widget={setVolume:n=>volumes.push(n),getDuration:fn=>fn(1000),play(){},bind:(event,fn)=>callbacks[event]=fn};
 const document={hidden:false,getElementById:id=>id==='soundcloud-frame'?{hidden:true}:null,createElement:()=>({}),head:{append(script){queueMicrotask(()=>script.onload());}},addEventListener(){}};
 const SC={Widget:()=>widget};SC.Widget.Events={READY:'ready',PLAY:'play',PAUSE:'pause',PLAY_PROGRESS:'progress',FINISH:'finish'};
 vm.runInNewContext(read('source-player.js'),{window,document,SC,Number,Math,Error,Promise,queueMicrotask});
 window.vibefySoundCloud.setVolume(44);await window.vibefySoundCloud.load('sc:123');callbacks.ready();window.vibefySoundCloud.setVolume(150);window.vibefySoundCloud.setVolume(Infinity);assert.deepEqual(volumes,[44,100]);
});
