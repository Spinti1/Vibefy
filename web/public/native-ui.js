'use strict';
// The native application's own Lucide/Spotifast icon set. No font glyph substitutes.
(() => {
 const names=new Set(['house','library','panel-left','chevron-left','chevron-right','chevron-down','search','shopping-cart','settings','users','user','mic','list-music','skip-back-filled','skip-forward-filled','play-filled','pause-filled','shuffle','repeat','heart','heart-filled','plus','list-plus','music','audio-lines','ellipsis','copy','external-link','x','pencil','trash-2','eye','eye-off','refresh-cw','clock','layout-grid','layout-list','arrow-left','volume-2','volume-x','expand','log-out','youtube']);
 function icon(name){if(!names.has(name))name='music';const svg=document.createElementNS('http://www.w3.org/2000/svg','svg');svg.setAttribute('viewBox','0 0 24 24');svg.setAttribute('aria-hidden','true');svg.setAttribute('focusable','false');svg.classList.add('ui-icon');if(name==='play-filled'||name==='pause-filled'){svg.setAttribute('fill','currentColor');svg.setAttribute('stroke','none');const p=document.createElementNS('http://www.w3.org/2000/svg','path');p.setAttribute('d',name==='play-filled'?'M7 4v16l13-8z':'M5 4h5v16H5zM14 4h5v16h-5z');svg.append(p);}else{const use=document.createElementNS('http://www.w3.org/2000/svg','use');use.setAttribute('href','/assets/ui-icons.svg#'+name);svg.append(use);}return svg;}
 function setIcon(node,name){if(node){const indicator=node.querySelector('#jam-indicator');node.replaceChildren(icon(name));if(indicator)node.append(indicator);}}
 function labelIcon(node,name,label){if(!node)return;const copy=document.createElement('span');copy.textContent=label;node.replaceChildren(icon(name),copy);}
 window.vibefyUI={icon,setIcon,labelIcon};
 document.addEventListener('pointerdown',event=>{document.documentElement.dataset.touchInput=String(event.pointerType==='touch');},{passive:true});
 document.addEventListener('keydown',()=>{document.documentElement.dataset.touchInput='false';});
 const mods=document.querySelector('.header-actions>[data-dialog="mods"]');const search=document.querySelector('.header-search');if(mods&&search){search.after(mods);mods.classList.add('toolbar-mods');}
 document.querySelectorAll('[data-ui-icon]').forEach(node=>setIcon(node,node.dataset.uiIcon));
 for(const [id,name]of Object.entries({'toggle-library':'panel-left','navigate-back':'chevron-left','navigate-forward':'chevron-right','mini-previous':'skip-back-filled','mini-next':'skip-forward-filled','previous':'skip-back-filled','next':'skip-forward-filled','mini-play':'play-filled','now-play':'play-filled','shuffle':'shuffle','repeat':'repeat','mini-expand':'expand','now-add':'list-plus','edit-collection':'ellipsis','play-collection':'play-filled'}))setIcon(document.getElementById(id),name);
 for(const [page,name,label]of [['home','house','Home'],['search','search','Search'],['library','library','Library']])document.querySelectorAll('[data-page="'+page+'"]').forEach(node=>{if(node.classList.contains('toolbar-home')||node.classList.contains('mobile-search-button'))setIcon(node,name);else labelIcon(node,name,label);});
 for(const [dialog,name]of [['jam','users'],['mods','shopping-cart'],['settings','settings'],['lyrics','mic'],['queue','list-music']])document.querySelectorAll('[data-dialog="'+dialog+'"]').forEach(node=>{if(node.classList.contains('rail-button')||node.classList.contains('text-button'))labelIcon(node,name,node.textContent.trim().replace(/^[^A-Za-z]+/,''));else setIcon(node,name);});
 document.querySelectorAll('[data-close]').forEach(node=>{if(node.classList.contains('icon-button'))setIcon(node,node.getAttribute('aria-label')==='Close now playing'?'chevron-down':'x');else labelIcon(node,'x',node.textContent.trim()||'Cancel');});
 document.querySelectorAll('[data-dialog="create"]').forEach(node=>{if(node.classList.contains('icon-button'))setIcon(node,'plus');else labelIcon(node,'plus','Create playlist');});
 document.querySelectorAll('[data-dialog="import-spotify"],[data-dialog="import-youtube"]').forEach(node=>labelIcon(node,'list-plus',node.dataset.dialog==='import-spotify'?'Import Spotify playlist':'Import YouTube playlist'));
 document.querySelectorAll('.search-form').forEach(form=>{const symbol=form.querySelector('span[aria-hidden]');if(symbol)setIcon(symbol,'search');const submit=form.querySelector('button[type="submit"]');if(submit)setIcon(submit,'search');});
 document.querySelectorAll('.rail-filter>span,.playlist-filter>span').forEach(node=>setIcon(node,'search'));
 document.querySelectorAll('.playlist-preview>span').forEach(node=>setIcon(node,'music'));
 labelIcon(document.getElementById('library-back'),'arrow-left','Back to Home');
 labelIcon(document.getElementById('remove-collection'),'trash-2','Remove local copy');
 labelIcon(document.getElementById('toggle-hidden'),'eye','Show hidden playlists');
 for(const [id,name,label]of [['youtube-refresh','refresh-cw','Refresh playlists'],['google-disconnect','log-out','Disconnect'],['export-library','copy','Export backup'],['edit-collection','ellipsis',null],['hide-playlist','eye-off','Hide playlist'],['copy-to-youtube','youtube','Save a YouTube copy'],['add-to-playlist','list-plus','Add song'],['remove-from-playlist','trash-2','Remove song']])if(label)labelIcon(document.getElementById(id),name,label);
 document.getElementById('now-add').setAttribute('aria-label','Add to playlist');
 const restore=document.querySelector('label.file-label');if(restore){const input=restore.querySelector('input');const title=document.createElement('span');title.textContent='Import backup';restore.replaceChildren(icon('copy'),title,input);}
 const table=document.querySelector('.track-table-head');if(table){const duration=table.children[3];setIcon(duration,'clock');}
 const greeting=document.getElementById('home-greeting');if(greeting){const hour=new Date().getHours();greeting.textContent=hour<12?'Good morning':hour<18?'Good afternoon':'Good evening';}
 document.getElementById('home-export').onclick=()=>document.getElementById('export-library').click();
 document.getElementById('home-restore').onclick=()=>document.getElementById('restore-library').click();
 const toggle=document.getElementById('toggle-library');let collapsed=false;try{collapsed=localStorage.getItem('vibefy-web-sidebar')==='collapsed';}catch{}
 function sidebar(){document.body.classList.toggle('library-collapsed',collapsed);toggle.setAttribute('aria-expanded',String(!collapsed));}sidebar();
 toggle.onclick=()=>{collapsed=!collapsed;sidebar();try{localStorage.setItem('vibefy-web-sidebar',collapsed?'collapsed':'open');}catch{}};
})();
