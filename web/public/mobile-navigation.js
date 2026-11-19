(() => {
 'use strict';
 if(!window.VibefyMobile?.postMessage&&!window.webkit?.messageHandlers?.vibefyMobile)return;
 if(window.vibefyMobileNavigationReady)return;window.vibefyMobileNavigationReady=true;
 document.documentElement.dataset.mobileApp='true';
 document.addEventListener('click',event=>{
  const anchor=event.target?.closest?.('a[href]');if(!anchor)return;const url=new URL(anchor.href,location.origin);
  if(anchor.matches('.brand,[data-app-home]')||(url.origin===location.origin&&['/','/index.html','/download'].includes(url.pathname))){event.preventDefault();event.stopImmediatePropagation();document.querySelector('.toolbar-home')?.click();}
 },true);
})();
