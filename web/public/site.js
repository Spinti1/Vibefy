'use strict';
// A desktop invitation gives the native app's code; it never starts a web player.
(() => {
 const code=new URLSearchParams(location.search).get('jam')||'';
 if(!/^[A-HJ-NP-Z2-9]{12}$/.test(code))return;
 const box=document.createElement('section');box.className='feature-card';box.id='jam-invitation';
 const title=document.createElement('h2');title.textContent='You’re invited to a Vibefy Jam';
 const text=document.createElement('p');text.textContent='Open Vibefy, select Listen Together in the top bar, then paste this code: '+code.match(/.{4}/g).join('-');
 const copy=document.createElement('button');copy.className='button primary small';copy.textContent='Copy Jam code';copy.onclick=async()=>{try{await navigator.clipboard.writeText(code);copy.textContent='Code copied';}catch{copy.textContent='Select and copy the code above';}};
 box.append(title,text,copy);document.querySelector('main').prepend(box);
})();
