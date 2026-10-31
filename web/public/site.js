// No analytics, tracking pixels or remote script execution.
if (/Android|iPhone|iPad|iPod/i.test(navigator.userAgent) && !new URLSearchParams(location.search).has('site')) location.replace('/app');
if ('serviceWorker' in navigator) navigator.serviceWorker.register('/sw.js').catch(() => {});
