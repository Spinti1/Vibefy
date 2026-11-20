const {test}=require('node:test');
const assert=require('node:assert/strict'),fs=require('node:fs'),path=require('node:path'),vm=require('node:vm');
const read=name=>fs.readFileSync(path.join(__dirname,'../public',name),'utf8');
function importer(){
 const nodes=new Map(),calls=[],saved=[];
 const node=id=>{if(!nodes.has(id))nodes.set(id,{value:'',textContent:'',disabled:false});return nodes.get(id);};
 const track={id:'abcdefghijk',title:'Demo track',artist:'Demo artist'};
 const bridge={save:c=>saved.push(c),toast(){}};
 vm.runInNewContext(read('web-library.js'),{document:{getElementById:node,addEventListener(){}},window:{vibefyWebLibrary:bridge,addEventListener(){}},crypto:{},URL,AbortSignal,fetch:async(url)=>{calls.push(url);return {ok:true,json:async()=>String(url).startsWith('/api/import/spotify')?{title:'Spotify demo',tracks:[track],next:null}:String(url)==='/api/match'?{track}:{title:'YouTube demo',tracks:[track]}};}});
 return {node,calls,saved,async submit(platform,url){node(platform+'-playlist-url').value=url;const button={disabled:false};await node('import-'+platform+'-form').onsubmit({preventDefault(){},submitter:button});assert.equal(button.disabled,false);}};
}
test('each import accepts only its own platform and rejects unsupported URLs before network requests',async()=>{
 const p=importer();
 await p.submit('spotify','https://music.youtube.com/playlist?list=PLabcdefghijklmnop');
 await p.submit('youtube','https://open.spotify.com/playlist/1234567890123456789012');
 await p.submit('youtube','https://music.youtube.com.evil.test/playlist?list=PLabcdefghijklmnop');
 assert.equal(p.calls.length,0);assert.equal(p.saved.length,0);
 assert.match(p.node('spotify-import-status').textContent,/Only public playlists/);
 assert.match(p.node('youtube-import-status').textContent,/YouTube playlist link/);
});
test('Spotify import matches recordings while YouTube import preserves source IDs',async()=>{
 const p=importer();
 await p.submit('spotify','https://open.spotify.com/playlist/1234567890123456789012');
 await p.submit('youtube','https://music.youtube.com/playlist?list=PLabcdefghijklmnop');
 assert.deepEqual(p.saved.map(c=>c.id),['spotify:1234567890123456789012','youtube:PLabcdefghijklmnop']);
 assert.ok(p.calls.includes('/api/match'));assert.equal(p.saved[0].tracks[0].id,'abcdefghijk');
});
test('Home uses a single account entry and dedicated import dialogs without a provider switch',()=>{
 const html=read('app.html');
 assert.ok(!html.includes('import-platform'));assert.ok(!html.includes('home-connect'));
 assert.equal((html.match(/id="header-connect"/g)||[]).length,1);
 for(const name of ['spotify','youtube'])assert.ok(html.includes('data-dialog="import-'+name+'"'));
 assert.doesNotMatch(read('player.js'),/title:'Liked songs'|function toggleLike|function liked/);
 // The old backup schema remains readable, including previously saved likes.
 assert.match(read('player.js'),/clean\.likes=value\.likes\.map\(normalizeTrack\)/);
});
