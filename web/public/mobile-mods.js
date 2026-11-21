'use strict';
(() => {
 const $=id=>document.getElementById(id),key='vibefy-web-mods-v1';
 const phone=window.vibefyPhone||!!(window.vibefyNativeInfo||window.vibefyMobileShell)||matchMedia('(pointer:coarse) and (max-width:900px)').matches;
 const names=['juicevault','nyan',...(phone?[]:['catjam'])],prefs={juicevault:false,nyan:false,catjam:false,size:90,tempo:135};
 try{const saved=JSON.parse(localStorage.getItem(key)||'{}');for(const n of names)prefs[n]=saved[n]===true;for(const [n,min,max] of [['size',50,180],['tempo',60,200]])if(Number.isFinite(saved[n]))prefs[n]=Math.max(min,Math.min(max,saved[n]));}catch{}
 window.vibefyBrowserMods=prefs;
 const el=(tag,value)=>{const n=document.createElement(tag);n.textContent=value;return n;};
 function save(){try{localStorage.setItem(key,JSON.stringify(prefs));}catch{}}
 const buttons=el('div','');buttons.className='header-source-searches';buttons.setAttribute('aria-label','Additional music sources');document.querySelector('.header-search').after(buttons);
 function sourceButtons(){buttons.replaceChildren();buttons.hidden=!prefs.juicevault;if(prefs.juicevault){const b=el('button','JuiceVault search');b.prepend(window.vibefyUI.icon('music'));b.className='button small';b.type='button';b.onclick=()=>window.vibefySourceSearch('juicevault');buttons.append(b);}window.dispatchEvent(new Event('vibefy-mods'));}
 const catalogue=[['juicevault','JuiceVault','Search the catalogue. Connect your account to import playlists.','https://juicevault.xyz','JuiceVault'],['nyan','Nyan Cat Progress Bar','Original Nyan artwork follows playback progress.','https://github.com/NoahClemmensen/spicetify-nyan-bar-synced','NoahClemmensen'],...(!phone?[['catjam','Cat Jam','Original animation with size and manual tempo controls.','https://github.com/BlafKing/spicetify-cat-jam-synced','BlafKing']]:[])];
 for(const [id,name,description,url,author] of catalogue){
  const card=el('article','');card.className='mod-card';card.dataset.mod=id;
  const heading=el('div','');heading.className='mod-heading';const art=el('span','');art.className='mod-art mod-art-'+id;art.setAttribute('aria-hidden','true');
  if(id==='nyan'){const img=new Image();img.src='/assets/mods/nyan_1.png';img.alt='';art.append(img);}else if(id==='catjam'){const sprite=el('span','');sprite.className='mod-cat-preview';art.append(sprite);}else art.append(window.vibefyUI.icon('music'));
  const copy=el('div',''),state=el('small','');state.className='mod-state';copy.append(el('h3',name),state);heading.append(art,copy);const descriptionNode=el('p',description);descriptionNode.className='mod-description';
  const footer=el('div','');footer.className='mod-footer';const link=el('a','By '+author);link.href=url;link.target='_blank';link.rel='noopener noreferrer';link.append(window.vibefyUI.icon('external-link'));
  const actions=el('div','');actions.className='mod-action-buttons';const settings=el('button','');settings.type='button';settings.className='icon-button mod-settings-trigger';const label=id==='catjam'?'Cat Jam settings':'JuiceVault account';settings.setAttribute('aria-label',label);settings.title=label;settings.append(window.vibefyUI.icon(id==='catjam'?'settings':'user'));settings.onclick=()=>$(id==='catjam'?'cat-settings-dialog':'juice-account-dialog').showModal();const button=el('button','');button.type='button';
  const render=()=>{const active=prefs[id];settings.hidden=!active||!['catjam','juicevault'].includes(id);window.vibefyUI.labelIcon(button,active?'x':'plus',active?'Remove':'Install');button.className='button small'+(active?'':' primary');button.setAttribute('aria-pressed',String(active));button.setAttribute('aria-label',(active?'Remove ':'Install ')+name);state.textContent=active?'Installed':'Not installed';card.classList.toggle('installed',active);};
  render();button.onclick=()=>{prefs[id]=!prefs[id];save();render();sourceButtons();updateSettings();schedule();};actions.append(settings,button);footer.append(link,actions);card.append(heading,descriptionNode,footer);$('mod-catalogue').append(card);
 }
 const cat=el('div',''),nyan=new Image();cat.className='browser-mod cat-jam';nyan.className='browser-mod nyan';nyan.alt='';for(const node of (phone?[nyan]:[cat,nyan])){node.hidden=true;node.setAttribute('aria-hidden','true');document.body.append(node);}let timer=null;
 function updateSettings(){$('cat-controls').hidden=!prefs.catjam;for(const [id,key,suffix] of [['cat-size','size','%'],['cat-tempo','tempo',' BPM']]){$(id).value=prefs[key];$(id+'-label').textContent=prefs[key]+suffix;}}
 for(const [id,key] of [['cat-size','size'],['cat-tempo','tempo']])$(id).oninput=()=>{prefs[key]=Number($(id).value);save();updateSettings();schedule();};
 function schedule(){if(timer===null&&!document.hidden&&(prefs.catjam||prefs.nyan||!cat.hidden||!nyan.hidden))timer=setTimeout(frame,100);}
 function frame(){timer=null;
  // Playback events also arrive with no visual mods installed. Avoid copying
  // player state and measuring the DOM in that overwhelmingly common case.
  if(document.hidden||(!prefs.catjam&&!prefs.nyan)){cat.hidden=true;nyan.hidden=true;return;}
  const s=window.vibefyPlayerBridge.snapshot(),visible=!document.hidden,playing=s.track&&!s.paused;
  const dialogs=[...document.querySelectorAll('dialog[open]')],modal=dialogs.find(d=>d.matches(':modal')),parent=modal||document.body;
  for(const node of (phone?[nyan]:[cat,nyan]))if(node.parentNode!==parent)parent.append(node);
  const blocked=Boolean(modal&&modal.id!=='now-dialog');cat.hidden=!prefs.catjam||!s.track||!visible||blocked;nyan.hidden=!prefs.nyan||!s.track||!visible||blocked;
  const reduced=window.vibefyPreferences?.reducedMotion||matchMedia('(prefers-reduced-motion: reduce)').matches;
  if(!cat.hidden){const scale=prefs.size/100,f=reduced?0:Math.floor(s.position*prefs.tempo/135.48/9.75*325)%325;cat.style.width=112*scale+'px';cat.style.height=86*scale+'px';cat.style.backgroundSize=1792*scale+'px '+1806*scale+'px';cat.style.backgroundPosition=(-(f%16)*112*scale)+'px '+(-Math.floor(f/16)*86*scale)+'px';cat.style.bottom='0px';}
  if(!nyan.hidden){const expanded=modal?.id==='now-dialog'||(!modal&&$('now-dialog')?.open),bars=expanded?[$('seek')]:[$('mini-seek'),$('mini-progress')],bar=bars.find(b=>b&&b.getClientRects().length&&b.getBoundingClientRect().width>0),rect=bar?.getBoundingClientRect();nyan.hidden=!rect;
   if(rect){const f=reduced?1:Math.floor(s.position*116/60*6)%6+1,source='/assets/mods/nyan_'+f+'.png';if(nyan.getAttribute('src')!==source)nyan.src=source;const duration=(bar===$('mini-progress')?Number(s.track.duration):Number(bar.max))||Number(s.track.duration)||1;const position=bar===$('mini-progress')?s.position:Number(bar.value),fraction=Math.max(0,Math.min(1,(Number.isFinite(position)?position:s.position)/duration));const origin=modal?modal.getBoundingClientRect():{left:0,top:0};nyan.style.position=modal?'absolute':'fixed';nyan.style.left=(rect.left+rect.width*fraction-origin.left-23+(modal?.scrollLeft||0))+'px';nyan.style.top=(rect.top-origin.top-16+(modal?.scrollTop||0))+'px';}
  }
  if(visible&&!blocked&&(prefs.catjam||prefs.nyan)&&playing&&!reduced)schedule();
 }
 document.addEventListener('vibefy-playback',schedule);window.addEventListener('vibefy-preferences',schedule);document.querySelectorAll('dialog').forEach(d=>d.addEventListener('close',schedule));document.addEventListener('visibilitychange',()=>{if(timer!==null){clearTimeout(timer);timer=null;}frame();});document.addEventListener('animationend',schedule,true);window.addEventListener('resize',schedule);window.addEventListener('scroll',schedule,true);for(const id of ['seek','mini-seek'])$(id).addEventListener('input',schedule);new MutationObserver(schedule).observe(document.body,{subtree:true,attributes:true,attributeFilter:['open']});sourceButtons();updateSettings();frame();
})();
