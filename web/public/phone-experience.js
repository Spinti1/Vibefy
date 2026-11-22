'use strict';
// Phone layout uses the same library and playback actions as the full player.
(() => {
 const $=id=>document.getElementById(id),bridge=window.vibefyWebLibrary;
 const phone=matchMedia('(max-width: 900px)');
 const playerFrame=$('youtube-frame');function mountPlayer(){if(phone.matches&&playerFrame.parentElement!==document.body){document.body.append(playerFrame);playerFrame.classList.add('persistent-phone-player');}}mountPlayer();phone.addEventListener('change',mountPlayer);
 document.addEventListener('contextmenu',event=>{if(phone.matches&&event.target.closest?.('.collection-tile,.rail-playlist')){event.preventDefault();event.stopImmediatePropagation();}},true);
 function node(tag,copy,cls){const el=document.createElement(tag);if(copy)el.textContent=copy;if(cls)el.className=cls;return el;}
 function action(label,icon,run){const b=node('button',null,'phone-action');b.type='button';b.setAttribute('aria-label',label);window.vibefyUI.labelIcon(b,icon,label);b.onclick=run;return b;}
 const actions=node('div',null,'phone-playlist-shortcuts');
 const add=action('Add songs','plus',()=>bridge.addSongs());
 const edit=action('Edit details','pencil',()=>$('edit-collection').click());
 actions.append(add,edit);$('playlist-count').before(actions);
 const shuffle=action('Shuffle','shuffle',()=>bridge.shufflePlaylist());shuffle.id='playlist-shuffle';
 $('play-collection').before(shuffle);
 const remove=action('Remove local copy','trash-2',()=>{$('edit-playlist-dialog').close();$('remove-collection').click();});
 remove.classList.add('phone-remove');$('edit-playlist-dialog').append(remove);
 function playlist(){const c=bridge.selected();add.hidden=!!c?.shared?.follow;shuffle.disabled=!c?.tracks?.length;edit.disabled=!!c?.shared?.follow;}
 document.addEventListener('vibefy-collection-open',playlist);window.addEventListener('popstate',playlist);playlist();

 const extras=node('div',null,'phone-now-extras');
 const preview=node('section',null,'phone-lyrics-preview');preview.hidden=true;
 preview.append(node('h3','Lyrics preview'));
 const lines=node('div',null,'phone-preview-lines');preview.append(lines);
 const full=action('Show lyrics','mic',()=>document.querySelector('.now-tools [data-dialog="lyrics"]').click());preview.append(full);
 const artist=action('View artist','user',()=>{const t=window.vibefyPlayerBridge.snapshot().track;if(t?.artistId||/^[A-Za-z0-9_-]{11}$/.test(t?.id||''))document.dispatchEvent(new CustomEvent('vibefy-open-artist',{detail:{id:t.artistId,name:t.artist,videoId:t.id}}));});artist.hidden=true;
 extras.append(preview,artist);$('now-dialog').append(extras);
 const artistName=$('now-artist');artistName.setAttribute('role','button');artistName.tabIndex=0;artistName.onclick=()=>{if(!artist.hidden)artist.onclick();};artistName.addEventListener('keydown',event=>{if(event.key==='Enter'||event.key===' '){event.preventDefault();artistName.onclick();}});
 let trackId=null,request=0,words=[],active=-2;
 function parse(data){const timed=String(data.synced||'').split('\n').flatMap(line=>{const m=line.match(/^\[(\d+):(\d+(?:\.\d+)?)\](.*)$/);return m&&m[3].trim()?[{at:Number(m[1])*60+Number(m[2]),text:m[3]}]:[];});return timed.length?timed:String(data.plain||'').split('\n').filter(x=>x.trim()).slice(0,4).map(text=>({at:null,text}));}
 function paint(position){if(!words.length)return;let i=0;for(let n=0;n<words.length;n++)if(words[n].at!==null&&words[n].at<=position)i=n;const start=Math.max(0,i-1);if(active===i)return;active=i;lines.replaceChildren(...words.slice(start,start+4).map((line,n)=>node('p',line.text,n===i-start?'current':'')));}
 function playback(){if(!phone.matches||document.hidden)return;const snapshot=window.vibefyPlayerBridge.snapshot(),t=snapshot.track;
  if(t?.id!==trackId){trackId=t?.id;const seq=++request;words=[];active=-2;preview.hidden=true;artist.hidden=!t?.artistId&&!/^[A-Za-z0-9_-]{11}$/.test(t?.id||'');artistName.tabIndex=artist.hidden?-1:0;artistName.setAttribute('aria-disabled',String(artist.hidden));if(!t)return;
   artist.querySelector('span').textContent='Explore '+t.artist;
   bridge.lyrics(t).then(data=>{if(seq!==request)return;words=parse(data);preview.hidden=!words.length;paint(window.vibefyPlayerBridge.snapshot().position);}).catch(()=>{if(seq===request)preview.hidden=true;});
  }else paint(snapshot.position);
 }
 document.addEventListener('vibefy-playback',playback);document.addEventListener('visibilitychange',playback);phone.addEventListener('change',playback);playback();
})();
