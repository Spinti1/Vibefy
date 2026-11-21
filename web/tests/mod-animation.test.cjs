const {test}=require('node:test'),assert=require('node:assert/strict'),fs=require('node:fs'),vm=require('node:vm');
const source=fs.readFileSync(require('node:path').join(__dirname,'../public/mobile-mods.js'),'utf8');
function animation(){
 const body={append(node){node.parentNode=this;}},modal={id:'now-dialog',matches:()=>true,append(node){node.parentNode=this;},getBoundingClientRect:()=>({left:100,top:50})};
 const nodes={seek:{value:100,max:200,getClientRects:()=>[1],getBoundingClientRect:()=>({left:150,top:300,width:400})},'mini-seek':{value:100,max:200,getClientRects:()=>[1],getBoundingClientRect:()=>({left:150,top:300,width:400})},'mini-progress':{getClientRects:()=>[],getBoundingClientRect:()=>({width:0})}};
 const c={phone:false,timer:null,prefs:{catjam:true,nyan:true,size:90,tempo:135},cat:{style:{}},nyan:{style:{},getAttribute(){return this.src;}},$:id=>nodes[id],window:{vibefyPlayerBridge:{snapshot:()=>c.snapshot},vibefyPreferences:{}},snapshot:{track:{duration:200},position:100,paused:false},document:{hidden:false,body,querySelectorAll:()=>c.dialogs},dialogs:[],matchMedia:()=>({matches:false}),innerWidth:800,setTimeout:()=>1,Math,Number,Boolean};
 vm.createContext(c);vm.runInContext(source.slice(source.indexOf(' function schedule()'),source.indexOf(" document.addEventListener('vibefy-playback'")),c);return {...c,context:c,modal,nodes,frame:()=>vm.runInContext('frame()',c)};
}
test('cats render on the visible mini seek bar while the closed now-playing dialog is ignored',()=>{
 const p=animation();p.frame();assert.equal(p.cat.hidden,false);assert.equal(p.nyan.hidden,false);assert.equal(p.nyan.style.left,'327px');assert.equal(p.nyan.style.top,'284px');assert.equal(p.cat.parentNode,p.context.document.body);
});
test('modal close restores animation and modal artwork uses coordinates relative to the dialog',()=>{
 const p=animation();p.context.dialogs=[{...p.modal,id:'mods-dialog'}];p.frame();assert.equal(p.cat.hidden,true);
 p.context.dialogs=[];p.frame();assert.equal(p.cat.hidden,false);
 p.context.dialogs=[p.modal];p.frame();assert.equal(p.nyan.style.left,'227px');assert.equal(p.nyan.style.top,'234px');assert.equal(p.cat.parentNode,p.modal);
});
test('paused songs show installed artwork but do not start an idle animation loop',()=>{
 const p=animation();p.context.snapshot.paused=true;p.frame();assert.equal(p.cat.hidden,false);assert.equal(p.nyan.hidden,false);assert.equal(p.context.timer,null);
 p.context.document.hidden=true;p.frame();assert.equal(p.cat.hidden,true);assert.equal(p.nyan.hidden,true);
});

test('the phone progress indicator uses song duration, not its normalized maximum of one',()=>{
 const p=animation();p.nodes.seek.getClientRects=()=>[];p.nodes['mini-seek'].getClientRects=()=>[];p.nodes['mini-progress'].max=1;p.nodes['mini-progress'].getClientRects=()=>[1];p.nodes['mini-progress'].getBoundingClientRect=()=>({left:0,top:400,width:300});p.frame();assert.equal(p.nyan.style.left,'127px');
});

test('phone catalogue ignores an old Cat Jam installation and removes WaveLink without changing other preferences',()=>{
 const nodes=new Map();const make=()=>({dataset:{},style:{},children:[],classList:{add(){},toggle(){}},setAttribute(){},getAttribute(){return this.src;},append(...children){this.children.push(...children);for(const c of children)c.parentNode=this;},prepend(){},after(){},replaceChildren(){this.children=[];},addEventListener(){}});
 const node=id=>{if(!nodes.has(id))nodes.set(id,make());return nodes.get(id);};
 const window={vibefyNativeInfo:{platform:'ios'},vibefyUI:{icon:make,labelIcon(){}},vibefyPlayerBridge:{snapshot:()=>({track:null,paused:true})},addEventListener(){},dispatchEvent(){}};
 const document={body:make(),hidden:false,getElementById:node,querySelector:make,querySelectorAll:()=>[],createElement:make,addEventListener(){}};
 vm.runInNewContext(source,{window,document,localStorage:{getItem:()=>JSON.stringify({wavelink:true,catjam:true,nyan:true,juicevault:true,size:110})},matchMedia:()=>({matches:false}),Image:function(){return make();},MutationObserver:class{observe(){}},Event,Boolean,Number,Math,JSON,setTimeout:()=>1});
 assert.deepEqual(node('mod-catalogue').children.map(c=>c.dataset.mod),['juicevault','nyan']);assert.equal(window.vibefyBrowserMods.catjam,false);assert.equal(window.vibefyBrowserMods.nyan,true);assert.equal(window.vibefyBrowserMods.size,110);assert.equal('wavelink' in window.vibefyBrowserMods,false);
 assert.ok(!document.body.children.some(n=>n.className==='browser-mod cat-jam'));
});

test('inline Jam uses its visible seek bar even though its dialog is non-modal',()=>{
 const p=animation();p.nodes['now-dialog']={open:true};p.nodes['mini-seek'].getClientRects=()=>[];p.frame();assert.equal(p.nyan.hidden,false);assert.equal(p.nyan.style.left,'327px');assert.equal(p.nyan.style.position,'fixed');
});

test('disabled mods and hidden pages do not read player state or measure layout',()=>{
 const p=animation();p.context.window.vibefyPlayerBridge.snapshot=()=>{throw Error('Unnecessary snapshot');};
 p.context.prefs.catjam=false;p.context.prefs.nyan=false;p.frame();
 assert.equal(p.cat.hidden,true);assert.equal(p.nyan.hidden,true);
 vm.runInContext('schedule()',p.context);assert.equal(p.context.timer,null);
 p.context.prefs.nyan=true;p.context.document.hidden=true;p.frame();
 vm.runInContext('schedule()',p.context);assert.equal(p.context.timer,null);
});
