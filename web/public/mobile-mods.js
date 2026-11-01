'use strict';
(() => {
 const $=id=>document.getElementById(id), key='vibefy-web-mods-v1';
 const names=['nyan','oneko','catjam','peek'];let prefs={nyan:false,oneko:false,catjam:false,peek:false,size:90,tempo:135};
 try{const saved=JSON.parse(localStorage.getItem(key)||'{}');for(const n of names)prefs[n]=saved[n]===true;for(const [n,min,max] of [['size',50,180],['tempo',60,200]])if(Number.isFinite(saved[n]))prefs[n]=Math.max(min,Math.min(max,saved[n]));}catch{}
 function save(){try{localStorage.setItem(key,JSON.stringify(prefs));}catch{}}
 const catalogue=[['nyan','Nyan Cat Progress Bar','Original Nyan artwork on the playback progress.','nyan_1.png'],['oneko','Oneko Cat','A tiny companion that follows your taps.','oneko.png'],['catjam','Cat Jam','Original animation, with size and tempo controls.','nyan_1.png'],['peek','Track Peek','See the next song inside Now Playing.','']];
 const el=(tag,content)=>{const n=document.createElement(tag);n.textContent=content;return n;};
 for(const [id,name,description,asset] of catalogue){const card=el('article','');card.className='mod-card';let art;if(asset&&id!=='catjam'){art=new Image();art.src='/assets/mods/'+asset;art.alt='';}else{art=el('span',id==='catjam'?'♫':'↦');art.setAttribute('aria-hidden','true');}const copy=el('div','');copy.append(el('h3',name),el('p',description));const button=el('button','');const render=()=>{button.textContent=prefs[id]?'Remove':'Install';button.setAttribute('aria-pressed',String(prefs[id]));button.setAttribute('aria-label',(prefs[id]?'Remove ':'Install ')+name);};render();button.onclick=()=>{prefs[id]=!prefs[id];save();render();updateSettings();schedule();};card.append(art,copy,button);$('mod-catalogue').append(card);}
 const importer=el('article','');importer.className='mod-card';const copy=el('div','');copy.append(el('h3','YouTube playlist import'),el('p','Save supported audio playlists on Home. Included.'));const open=el('button','Open');open.onclick=()=>{$('mods-dialog').close();window.vibefyLibraryBridge.import();};importer.append(el('span','▤'),copy,open);$('mod-catalogue').append(importer);
 const cat=el('div',''),neko=el('div',''),nyan=new Image();cat.className='browser-mod cat-jam';neko.className='browser-mod oneko';nyan.className='browser-mod nyan';nyan.alt='';for(const node of [cat,neko,nyan]){node.hidden=true;node.setAttribute('aria-hidden','true');document.body.append(node);}
 const peek=el('button','');peek.id='track-peek';peek.hidden=true;peek.onclick=()=>window.vibefyLibraryBridge.next();$('now-dialog').append(peek);
 let target={x:45,y:120},position={x:45,y:120},timer=null,lastPeek='';
 document.addEventListener('pointerdown',e=>{target={x:e.clientX,y:e.clientY};schedule();},{passive:true});
 function updateSettings(){$('cat-controls').hidden=!prefs.catjam;for(const [id,key,suffix] of [['cat-size','size','%'],['cat-tempo','tempo',' BPM']]){$(id).value=prefs[key];$(id+'-label').textContent=prefs[key]+suffix;}}
 $('cat-size').oninput=()=>{prefs.size=Number($('cat-size').value);save();updateSettings();schedule();};$('cat-tempo').oninput=()=>{prefs.tempo=Number($('cat-tempo').value);save();updateSettings();schedule();};
 function schedule(){if(timer!==null)return;timer=setTimeout(frame,100);}
 function frame(){timer=null;const s=window.vibefyPlayerBridge.snapshot(),dialog=document.querySelector('dialog[open]'),visible=!document.hidden,playing=s.track&&!s.paused;const parent=dialog||document.body;for(const node of [cat,neko,nyan])if(node.parentNode!==parent)parent.append(node);
 cat.hidden=!prefs.catjam||!s.track||!visible||Boolean(dialog&&dialog.id!=='now-dialog');neko.hidden=!prefs.oneko||!visible||!!dialog;nyan.hidden=!prefs.nyan||!s.track||!visible;
 const reduced=matchMedia('(prefers-reduced-motion: reduce)').matches;
 if(!cat.hidden){const scale=prefs.size/100;const f=reduced?0:Math.floor(s.position*prefs.tempo/135.48/9.75*325)%325;cat.style.width=112*scale+'px';cat.style.height=86*scale+'px';cat.style.backgroundSize=1792*scale+'px '+1806*scale+'px';cat.style.backgroundPosition=(-f%16*112*scale)+'px '+(-Math.floor(f/16)*86*scale)+'px';cat.style.bottom=dialog?'0px':'calc(73px + env(safe-area-inset-bottom))';}
 if(!neko.hidden){const dx=target.x-position.x,dy=target.y-position.y,d=Math.hypot(dx,dy),step=Math.min(13,Math.max(0,d-22));if(d>0){position.x+=dx/d*step;position.y+=dy/d*step;}position.x=Math.max(0,Math.min(innerWidth-32,position.x));position.y=Math.max(0,Math.min(innerHeight-32,position.y));neko.style.left=position.x+'px';neko.style.top=position.y+'px';const f=Math.floor(performance.now()/160)%2;const col=d>24?(dx>=0?3:4):2,row=d>24?(dx>=0?f:2+f):0;neko.style.backgroundPosition=(-col*32)+'px '+(-row*32)+'px';}
 if(!nyan.hidden){const bar=dialog?.id==='now-dialog'?$('seek'):$('mini-progress');const rect=bar.getBoundingClientRect();nyan.hidden=!rect.width||Boolean(dialog&&dialog.id!=='now-dialog');const frame=reduced?1:Math.floor(s.position*116/60*6)%6+1;nyan.src='/assets/mods/nyan_'+frame+'.png';nyan.style.left=Math.max(0,Math.min(innerWidth-46,rect.left+rect.width*(s.position/Math.max(1,Number($('seek').max)))-23))+'px';nyan.style.top=rect.top-23+'px';}
 const next=window.vibefyLibraryBridge.peek();peek.hidden=!prefs.peek||!next;peek.disabled=!!window.vibefyJam?.following;if(!peek.hidden&&lastPeek!==next.id){lastPeek=next.id;peek.replaceChildren();const img=new Image();img.src=next.image;img.alt='';const text=el('span','');text.append(el('small','UP NEXT'),el('strong',next.title));peek.append(img,text);}
 const moving=prefs.oneko&&Math.hypot(target.x-position.x,target.y-position.y)>24;
 if(visible&&(moving||((prefs.catjam||prefs.nyan)&&playing&&!reduced)))schedule();
 }
 document.addEventListener('vibefy-playback',schedule);document.querySelectorAll('dialog').forEach(d=>d.addEventListener('close',schedule));
 document.addEventListener('visibilitychange',()=>{if(timer!==null){clearTimeout(timer);timer=null;}frame();});updateSettings();frame();
})();
