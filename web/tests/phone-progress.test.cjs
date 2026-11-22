const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../public/player.js'),'utf8');
function progress(duration){
 const nodes=new Map(),node=id=>{if(!nodes.has(id))nodes.set(id,{value:0,max:100});return nodes.get(id);};let observed;
 const c={$:node,media:{duration,currentTime:45},current:()=>({duration:180}),seekingSlider:false,pendingAudioSeek:null,shownPosition:()=>45,time:n=>String(n),lyricLines:[],window:{},navigator:{},Event,document:{dispatchEvent(){observed=node('mini-seek').value;}}};
 vm.createContext(c);vm.runInContext(source.match(/function updatePosition\([^\n]+/)[0],c);return {c,node,observed:()=>observed};
}
test('unknown stream duration uses track metadata and notifies Nyan after the slider is updated',()=>{
 const p=progress(NaN);p.c.updatePosition();assert.equal(p.node('seek').max,180);assert.equal(p.node('mini-seek').max,180);assert.equal(p.node('mini-progress').value,.25);assert.equal(p.observed(),45);
});

test('position is published through MediaSession itself for lock-screen progress',()=>{
 const p=progress(200);let state;p.c.navigator.mediaSession={setPositionState(value){state=value;}};
 p.c.updatePosition();assert.equal(state.position,45);assert.equal(state.duration,200);assert.equal(state.playbackRate,1);
});

test('lyric DOM updates only on line changes, including seeking backwards',()=>{
 const p=progress(200);let scans=0;const nodes=[0,1].map(()=>({classList:{toggle(){}},setAttribute(){},removeAttribute(){}}));
 Object.assign(p.c,{lyricLines:[{at:0},{at:60}],lyricOffset:0,activeLyric:-1,lyricScrollUntil:Infinity});
 p.c.document.querySelectorAll=()=>{scans++;return nodes;};
 p.c.updatePosition();p.c.updatePosition();assert.equal(scans,1);
 p.c.shownPosition=()=>80;p.c.updatePosition();assert.equal(scans,2);
 p.c.shownPosition=()=>20;p.c.updatePosition();assert.equal(scans,3);
});
test('position updates never pull a dragged thumb back to the playing position',()=>{
 const p=progress(200);p.c.seekingSlider=true;p.node('seek').value=120;p.node('mini-seek').value=120;p.c.updatePosition();assert.equal(p.node('seek').value,120);assert.equal(p.node('mini-seek').value,120);assert.equal(p.node('position').textContent,'120');assert.equal(p.node('mini-progress').value,.6);
});

test('hidden queue rendering is deferred until explicitly opened',()=>{
 const calls=[],nodes={};const c={window:{},$:id=>nodes[id]||(nodes[id]={getClientRects:()=>[]}),queue:[{id:'a'},{id:'b'}],index:0,renderTracks:(...args)=>calls.push(args)};
 vm.createContext(c);vm.runInContext(source.match(/function renderQueue\([^\n]+/)[0],c);c.renderQueue();assert.equal(calls.length,0);c.renderQueue(true);assert.equal(calls.length,1);assert.equal(calls[0][1][0].id,'b');
});

test('blocked or failed playback restores a Play state instead of leaving a false playing intent',()=>{
 const c={pendingAudioSeek:{paused:false},playbackTimer:null,playbackError:false,$:()=>({}),audio:{pause(){c.paused=true;}},clearTimeout(){},updatePlay(){c.updates++;},toast(){},updates:0};
 vm.createContext(c);vm.runInContext(source.match(/function audioPlayFailed\([^\n]+/)[0]+'\n'+source.match(/function playbackFailed\([^\n]+/)[0],c);
 c.audioPlayFailed({name:'NotAllowedError'});assert.equal(c.pendingAudioSeek.paused,true);assert.equal(c.updates,1);
 c.playbackFailed();assert.equal(c.pendingAudioSeek,null);assert.equal(c.paused,true);assert.equal(c.updates,2);
});
