export const CODE = /^[A-HJ-NP-Z2-9]{12}$/;
export const TOKEN = /^[a-f0-9]{64}$/;
export function track(value) {
  if (!value || typeof value !== 'object') return null;
  if (!/^vibefy:youtube:[A-Za-z0-9_-]{11}$/.test(value.uri || '')) return null;
  if (typeof value.title !== 'string' || !value.title.trim() || value.title.length > 300) return null;
  if (!Array.isArray(value.artists) || value.artists.length > 12) return null;
  const artists = value.artists.map(a => ({id:'',uri:'',name:String(a?.name || '').slice(0,120)}));
  return {uri:value.uri,title:value.title,name:value.title,artists,album:String(value.album||'').slice(0,300),album_name:String(value.album_name||'').slice(0,300),duration_ms:Math.min(86400000,Math.max(0,Number(value.duration_ms)||0)),cover_url:null,art_url:null,art_small_url:null,is_episode:false};
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
