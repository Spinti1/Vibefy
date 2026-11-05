'use strict';
// Device routing is a product choice, not an authentication boundary.
// A narrow desktop window must still show the desktop download site.
window.vibefyPhone = /iPhone|iPod|Android.*Mobile|Windows Phone/i.test(navigator.userAgent);
document.documentElement.dataset.device = window.vibefyPhone ? 'phone' : 'desktop';
if (/^\/app(?:\.html)?\/?$/.test(location.pathname) && !window.vibefyPhone) {
  const jam = new URLSearchParams(location.search).get('jam');
  location.replace(jam && /^[A-HJ-NP-Z2-9]{12}$/i.test(jam) ? '/?jam=' + jam.toUpperCase() : '/');
} else if (location.pathname === '/' && window.vibefyPhone && !new URLSearchParams(location.search).has('site')) {
  location.replace('/app' + location.search);
}
