'use strict';
// Invitations can be opened in the web player or copied to the native app.
(() => {
 const code=new URLSearchParams(location.search).get('jam')||'';
 if(!/^[A-HJ-NP-Z2-9]{12}$/.test(code))return;
 const box=document.createElement('section');box.className='feature-card';box.id='jam-invitation';
 const title=document.createElement('h2');title.textContent='You’re invited to a Vibefy Jam';
 const text=document.createElement('p');text.textContent='Join in the web player, or open Listen Together in the desktop app and paste this code: '+code.match(/.{4}/g).join('-');
 const copy=document.createElement('button');copy.className='button primary small';copy.textContent='Copy Jam code';copy.onclick=async()=>{try{await navigator.clipboard.writeText(code);copy.textContent='Code copied';}catch{copy.textContent='Select and copy the code above';}};
 const web=document.createElement('a');web.className='button';web.textContent='Join in browser';web.href='/web?jam='+code;
 box.append(title,text,copy,web);document.querySelector('main').prepend(box);
})();
