const {test}=require('node:test');
const assert=require('node:assert/strict');
const fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const source=fs.readFileSync(path.join(__dirname,'../public/player.js'),'utf8');
const view=source.match(/function playlistView\([^\n]+/)[0];
const context={};vm.runInNewContext(view,context);
const songs=[{id:'one',title:'Breathe',artist:'Artist B',album:'First',duration:200},{id:'two',title:'Again',artist:'Artist A',album:'Second',duration:100},{id:'three',title:'Breathe',artist:'Artist A',album:'Second',duration:300}];
test('playlist filters search title, artist and album without mutating saved order',()=>{
 assert.deepEqual(Array.from(context.playlistView(songs,'SECOND','original'),v=>v.index),[1,2]);
 assert.deepEqual(Array.from(context.playlistView(songs,' artist b ','title'),v=>v.index),[0]);
 assert.deepEqual(Array.from(context.playlistView(songs,'missing','original')) ,[]);
 assert.deepEqual(songs.map(t=>t.id),['one','two','three']);
});
test('sorting preserves original indexes so displayed and played songs agree',()=>{
 assert.deepEqual(Array.from(context.playlistView(songs,'','duration'),v=>v.index),[1,0,2]);
 assert.deepEqual(Array.from(context.playlistView(songs,'','title'),v=>v.index),[1,0,2]);
 assert.deepEqual(Array.from(context.playlistView(songs,'breathe','artist'),v=>v.track.id),['three','one']);
});
test('new offline shell contains playlist styling and retains player routes',()=>{
 const sw=fs.readFileSync(path.join(__dirname,'../public/sw.js'),'utf8');
 for(const file of ['/web','/phone','/workspace.css','/web-library.js','/mobile-mods.js'])assert.ok(sw.includes("'"+file+"'"));
 assert.match(sw,/vibefy-shell-\d+/);
 for(const file of ['/native-ui.js','/native-ui.css','/assets/ui-icons.svg'])assert.ok(sw.includes("'"+file+"'"));
});

