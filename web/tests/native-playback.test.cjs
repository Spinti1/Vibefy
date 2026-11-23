const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
function page(){const packets=[],events=[],window={vibefyNativeInfo:{backgroundAudio:true},VibefyMobile:{postMessage:p=>packets.push(JSON.parse(p))}};vm.runInNewContext(fs.readFileSync(path.join(__dirname,'../public/native-playback.js'),'utf8'),{window,EventTarget,Event,document:{dispatchEvent:e=>events.push(e)},CustomEvent:class{constructor(type,o){this.type=type;this.detail=o.detail;}}});return{window,api:window.vibefyNativePlayback,packets,events};}
test('native queue is loaded once; pause, seek, volume and modes use the Android media session',()=>{const p=page(),tracks=[{id:'abcdefghijk',title:'One'},{id:'lmnopqrstuv',title:'Two'}];p.api.loadQueue(tracks,0,false,0,false,false);p.api.syncQueue(tracks,0,false,false);assert.equal(p.packets.filter(p=>p.command==='queue').length,1);p.api.pause();p.api.currentTime=42;p.api.volume=.4;p.api.modes(true,true);assert.deepEqual(p.packets.slice(-4).map(p=>p.command),['pause','seek','volume','modes']);assert.equal(p.packets.at(-3).position,42000);assert.equal(p.api.paused,true);});
test('native failures dispatch one error and stale queue snapshots cannot overwrite current playback',()=>{const p=page();let errors=0;p.api.addEventListener('error',()=>errors++);p.api.loadQueue([{id:'abcdefghijk'}],0,false,0,false,false);p.api.loadQueue([{id:'lmnopqrstuv'}],0,true,7,false,false);p.window.vibefyNativePlaybackState({generation:1,id:'abcdefghijk',position:100});assert.equal(p.api.currentTime,7);p.window.vibefyNativePlaybackState({generation:2,error:'Unavailable'});p.window.vibefyNativePlaybackState({generation:2,error:'Unavailable'});assert.equal(errors,1);});
test('Android track transitions update the interface without asking JavaScript to advance the queue',()=>{const p=page();p.api.loadQueue([{id:'abcdefghijk'},{id:'lmnopqrstuv'}],0,false,0,false,false);p.window.vibefyNativePlaybackState({generation:1,id:'lmnopqrstuv',index:1,position:3,duration:180,paused:false,ready:true});assert.equal(p.events.at(-1).detail.id,'lmnopqrstuv');assert.equal(p.api.duration,180);assert.equal(p.packets.filter(p=>p.command==='queue').length,1);assert.throws(()=>p.api.loadQueue([{id:'https://evil.test/audio'}],0,false,0,false,false));});
test('unmatched playlist entries do not block Android playback or renumber native next and previous in the UI',()=>{
 const p=page(),tracks=[{id:'abcdefghijk'},{id:'spotify:unmatched'},{id:'lmnopqrstuv'}];
 p.api.loadQueue(tracks,2,false,0,false,false);
 assert.deepEqual(p.packets.at(-1).tracks,[tracks[0],tracks[2]]);
 assert.equal(p.packets.at(-1).index,1);
 p.api.previous();
 assert.equal(p.packets.at(-1).command,'previous');
 p.window.vibefyNativePlaybackState({generation:1,id:'abcdefghijk',index:0});
 assert.equal(p.events.at(-1).detail.index,0);
 p.api.next();
 assert.equal(p.packets.at(-1).command,'next');
 p.window.vibefyNativePlaybackState({generation:1,id:'lmnopqrstuv',index:1});
 assert.equal(p.events.at(-1).detail.index,2);
});

test('media-session connection failures leave loading state and surface an error',()=>{const p=page();let failed=0;p.api.addEventListener('error',()=>failed++);p.api.loadQueue([{id:'abcdefghijk'}],0,false,0,false,false);p.window.vibefyNativePlaybackState({generation:0,error:'Connection unavailable'});assert.equal(failed,1);});

test('late native callbacks cannot replace a newly selected recording while its source resolves',()=>{const p=page();p.api.loadQueue([{id:'abcdefghijk'}],0,false,0,false,false);p.api.expectTrack('lmnopqrstuv');p.window.vibefyNativePlaybackState({generation:1,id:'abcdefghijk',position:99,ready:true});assert.equal(p.api.currentTime,0);assert.equal(p.events.length,0);});

test('position ticks do not redraw every playlist row or replace the native queue',()=>{const p=page();p.api.loadQueue([{id:'abcdefghijk'}],0,false,0,false,false);p.window.vibefyNativePlaybackState({generation:1,id:'abcdefghijk',index:0,position:1});p.window.vibefyNativePlaybackState({generation:1,id:'abcdefghijk',index:0,position:2});assert.equal(p.events.length,0);assert.equal(p.api.currentTime,2);});
