export const CODE = /^[A-HJ-NP-Z2-9]{12}$/;
export const TOKEN = /^[a-f0-9]{64}$/;
export const URI = /^vibefy:(?:youtube:[A-Za-z0-9_-]{11}|juicevault:[0-9a-fA-F]{8}(?:-[0-9a-fA-F]{4}){3}-[0-9a-fA-F]{12})$/;
export function publicCover(value) {
  if(typeof value!=='string'||value.length>2000)return null;
  try{const u=new URL(value);if(u.protocol!=='https:'||u.username||u.password||u.port||u.search||u.hash)return null;
    const h=u.hostname,p=u.pathname,ok=h==='api.juicevault.xyz'?p.startsWith('/cdn/')&&/^[-a-zA-Z0-9_/\.]+$/.test(p):h==='i.ytimg.com'?p.startsWith('/vi/')||p.startsWith('/vi_webp/'):h==='i.scdn.co'?p.startsWith('/image/'):h.endsWith('.googleusercontent.com')||h.endsWith('.ggpht.com');
    return ok?u.href:null;
  }catch{return null;}
}
export function track(value) {
  if (!value || typeof value !== 'object') return null;
  if (!URI.test(value.uri || '')) return null;
  if (typeof value.title !== 'string' || !value.title.trim() || value.title.length > 300) return null;
  if (!Array.isArray(value.artists) || value.artists.length > 12) return null;
  const artists = value.artists.map(a => ({id:'',uri:'',name:String(a?.name || '').slice(0,120)}));
  // JuiceVault's public artwork address is derived from the validated recording
  // UUID. Its optional cache-busting query never needs to cross the relay.
  const cover=[value.cover_url,value.art_url,value.art_small_url].map(publicCover).find(Boolean)||(value.uri.startsWith('vibefy:juicevault:')?'https://api.juicevault.xyz/cdn/music/covers/'+value.uri.slice(18):null);
  return {uri:value.uri,title:value.title,name:value.title,artists,album:String(value.album||'').slice(0,300),album_name:String(value.album_name||'').slice(0,300),duration_ms:Math.min(86400000,Math.max(0,Number(value.duration_ms)||0)),cover_url:cover,art_url:null,art_small_url:null,is_episode:false};
}
// Commands refer to the current server snapshot/queue, never arbitrary URLs.
export function control(value, current, queue) {
  if(value?.action==='set_playing' && current && value.uri===current.track.uri && typeof value.playing==='boolean')
    return {action:'set_playing',uri:value.uri,playing:value.playing};
  if(value?.action==='seek' && current && value.uri===current.track.uri && Number.isInteger(value.position_ms) && value.position_ms>=0 && value.position_ms<=86400000)
    return {action:'seek',uri:value.uri,position_ms:Math.min(value.position_ms,current.track.duration_ms||86400000)};
  if(value?.action==='play_queue' && Number.isInteger(value.index) && value.index>=0 && value.index<queue.length && queue[value.index].uri===value.uri)
    return {action:'play_queue',index:value.index,uri:value.uri};
  return null;
}
export function snapshot(value) {
  if (value === null) return null;
  const t=track(value?.track);
  if (!t || typeof value.playing !== 'boolean' || !Number.isInteger(value.position_ms) || value.position_ms<0 || value.position_ms>86400000) throw new Error('Invalid snapshot');
  return {track:t,position_ms:value.position_ms,playing:value.playing};
}
export function randomCode() {
  const alphabet='ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
  return Array.from(crypto.getRandomValues(new Uint8Array(12)),n=>alphabet[n%32]).join('');
}
export function secret() {return Array.from(crypto.getRandomValues(new Uint8Array(32)),n=>n.toString(16).padStart(2,'0')).join('');}
