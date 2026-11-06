'use strict';
// The presentation and player have explicit URLs on every device.
window.vibefyPhone = /iPhone|iPod|Android.*Mobile|Windows Phone/i.test(navigator.userAgent);
document.documentElement.dataset.device = window.vibefyPhone ? 'phone' : 'desktop';
