'use strict';
// Device routing is a product choice, not an authentication boundary.
// A narrow desktop window must still show the desktop download site.
window.vibefyPhone = /iPhone|iPod|Android.*Mobile|Windows Phone/i.test(navigator.userAgent);
document.documentElement.dataset.device = window.vibefyPhone ? 'phone' : 'desktop';
if (/^\/app(?:\.html)?\/?$/.test(location.pathname) && !window.vibefyPhone) {
  location.replace('/');
} else if (location.pathname === '/' && window.vibefyPhone && !new URLSearchParams(location.search).has('site')) {
  location.replace('/app' + location.search);
}
