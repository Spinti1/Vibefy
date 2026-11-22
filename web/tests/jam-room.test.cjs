const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm'),path=require('node:path');
const read=name=>fs.readFileSync(path.join(__dirname,'../public',name),'utf8');
function room(){
 const nodes=new Map(),classes=new Set();
 class Node {
  constructor(id){this.id=id;this.children=[];this.hidden=false;this.attrs={};this.open=false;}
  get nextSibling(){const list=this.parentNode?.children||[];return list[list.indexOf(this)+1]||null;}
  append(...children){for(const child of children){if(child.parentNode)child.parentNode.children.splice(child.parentNode.children.indexOf(child),1);this.children.push(child);child.parentNode=this;}}
  insertBefore(node,next){if(node.parentNode)node.parentNode.children.splice(node.parentNode.children.indexOf(node),1);const index=next?this.children.indexOf(next):this.children.length;assert.ok(index>=0);this.children.splice(index,0,node);node.parentNode=this;}
  replaceChildren(...children){for(const child of this.children)child.parentNode=null;this.children=[];this.append(...children);}
  close(){this.open=false;}show(){this.open=true;}showModal(){this.open=true;}
  getAttribute(name){return this.attrs[name]??null;}setAttribute(name,value){this.attrs[name]=String(value);}
 }
 const $=id=>{if(!nodes.has(id))nodes.set(id,new Node(id));return nodes.get(id);};
 for(const match of read('app.html').matchAll(/id="([^"]+)"/g))$(match[1]);
 const body=new Node('body');body.classList={toggle(name,value){if(value)classes.add(name);else classes.delete(name);},remove:name=>classes.delete(name)};
 body.append($('now-dialog'));$('mini-tools').append($('volume-toggle'),$('volume-label'),$('volume'),$('mini-expand'));
 const document={body,getElementById:$,createElement:tag=>new Node(tag),addEventListener(){},querySelector(){return {click(){}};},fullscreenElement:null};
 const window={vibefyJam:{host:false,sharedControls:true,leave(){},playQueue:()=>true},vibefyPlayerBridge:{snapshot:()=>({track:{id:'demo'}}),seek(){},addQueue(){}},vibefyWebLibrary:{art:x=>x||'/assets/icon.png',toast(){}}};
 vm.runInNewContext(read('jam-room.js'),{document,window,location:{origin:'https://example.test'},navigator:{clipboard:{writeText:async()=>{}}},Number,String,Array,JSON});
 return {$,document,window,classes};
}
test('Jam moves the original volume controls once and restores their location and handlers on leave',()=>{
 const r=room(),volume=r.$('volume'),mute=r.$('volume-toggle'),handler=()=>{};volume.oninput=handler;
 r.window.vibefyJamRoom.activate(true);r.window.vibefyJamRoom.activate(true);
 assert.equal(volume.parentNode,r.$('jam-volume-inputs'));assert.equal(mute.parentNode,r.$('jam-volume-inputs'));
 assert.equal(r.$('jam-volume-inputs').children.length,2);assert.equal(r.$('jam-volume-controls').hidden,false);assert.ok(r.classes.has('jam-active'));
 r.window.vibefyJamRoom.activate(false);
 assert.equal(r.$('jam-volume-controls').hidden,true);assert.equal(volume.oninput,handler);
 assert.deepEqual(r.$('mini-tools').children.map(n=>n.id),['volume-toggle','volume-label','volume','mini-expand']);assert.ok(!r.classes.has('jam-active'));
});
test('guest Next follows shared-queue availability even when only permission changes',()=>{
 const r=room();r.window.vibefyJamRoom.activate(true);
 const packet={track:{title:'Song',artist:'Artist',duration:200},queue:[{title:'Next',artist:'Artist'}],participants:[],snapshot:{playing:true}};
 r.window.vibefyJamRoom.update(packet);assert.equal(r.$('next').disabled,false);assert.ok(r.classes.has('jam-guest'));
 r.window.vibefyJam.sharedControls=false;r.window.vibefyJamRoom.update(packet);assert.equal(r.$('next').disabled,true);
 r.window.vibefyJam.sharedControls=true;r.window.vibefyJamRoom.update({...packet,queue:[]});assert.equal(r.$('next').disabled,true);
 r.window.vibefyJamRoom.activate(false);assert.ok(!r.classes.has('jam-guest'));
});
test('Jam volume uses the same audio/provider controls, saves the level and restores it after mute',()=>{
 const nodes=new Map(),$=id=>{if(!nodes.has(id))nodes.set(id,{value:0,setAttribute(){}});return nodes.get(id);};
 const output={audio:{volume:0},embedded:[],soundcloud:[],stored:null};
 const c={$,audio:output.audio,embedded:{setVolume:n=>output.embedded.push(n)},soundcloud:{setVolume:n=>output.soundcloud.push(n)},window:{vibefyUI:{setIcon(){}}},localStorage:{getItem:()=>null,setItem:(key,value)=>output.stored=value},Number,Math};
 vm.createContext(c);vm.runInContext(read('player.js').slice(read('player.js').indexOf('let preferredVolume=')),c);
 $('volume').value=37;$('volume').oninput();assert.equal(output.audio.volume,.37);assert.equal($('jam-volume-value').textContent,'37%');
 $('volume-toggle').onclick();assert.equal(output.audio.volume,0);$('volume-toggle').onclick();assert.equal(output.audio.volume,.37);
 vm.runInContext('setVolume(Infinity)',c);
 assert.deepEqual(output.embedded,[80,37,0,37]);assert.deepEqual(output.soundcloud,[80,37,0,37]);assert.equal(output.stored,'37');
});
