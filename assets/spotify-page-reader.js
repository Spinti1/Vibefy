// Read metadata from Spotify's rendered UI. No fetch, XHR, cookies or storage access.
(request => {
  const text = el => (el?.textContent || '').trim();
  const visible = el => !!el && el.getClientRects().length > 0;
  const linkId = (el, kind) => {
    try {
      const u = new URL(el?.getAttribute('href') || '', location.origin);
      if (u.origin !== 'https://open.spotify.com') return null;
      const match = u.pathname.match(new RegExp('^/' + kind + '/([^/]+)$'));
      return match ? decodeURIComponent(match[1]) : null;
    } catch (_) { return null; }
  };
  const image = root => {
    let src = root?.querySelector('img')?.getAttribute('src') || null;
    if (src?.match(/^https:\/\/i\.scdn\.co\/image\/ab67616d00004851[0-9a-f]{32}$/))
      src = src.replace('ab67616d00004851', 'ab67616d0000b273');
    return src ? [{url:src}] : [];
  };
  if (location.origin !== 'https://open.spotify.com') return {ready:false};
  if (request.kind === 'identity') {
    const widget = document.querySelector('[data-testid="user-widget-link"],[data-testid="user-widget-avatar"]');
    if (!visible(widget)) return {ready:false};
    const menus = [...document.querySelectorAll('[role="menu"]')].filter(visible);
    const links = menus.flatMap(m => [...m.querySelectorAll('a[href]')]);
    if (widget.matches('a[href]')) links.push(widget);
    const profile = links.find(a => linkId(a,'user'));
    if (profile) {
      const id = linkId(profile,'user');
      const name = widget.getAttribute('aria-label') || widget.getAttribute('title') || id;
      return {ready:true,id,display_name:name};
    }
    // Opening the account menu only exposes the profile link; no sign-in input.
    if (!menus.length && request.openMenu) widget.click();
    return {ready:false};
  }
  const main = document.querySelector('main');
  if (!main) return {ready:false};
  let root, grid, rows, total = null, collapsed = 0, positions = [];
  if (request.kind === 'library') {
    const sidebar = document.getElementById('Desktop_LeftSidebar_Id');
    const sidebarGrid = sidebar?.querySelector('[role="grid"]');
    root = visible(sidebarGrid) && sidebarGrid.getAttribute('aria-rowcount') !== '0' ? sidebar : main;
    if (!root) return {ready:false};
    if (root.querySelector('[role="progressbar"],[aria-busy="true"]')) return {ready:false};
    grid = root.querySelector('[role="grid"]');
    const declared = grid?.getAttribute('aria-rowcount');
    if (declared !== null && declared !== undefined && /^\d+$/.test(declared)) total = Number(declared);
    positions = [...(grid?.querySelectorAll('[role="row"][aria-rowindex]') || [])].filter(row => text(row)).map(row => Number(row.getAttribute('aria-rowindex'))-1);
    collapsed = root.querySelectorAll('[role="treeitem"][aria-expanded="false"]').length;
    rows = [...root.querySelectorAll('a[href]')].filter(a => linkId(a,'playlist') && !a.closest('[data-testid="footer-div"]')).map(a => {
      const card = a.closest('[role="row"],[data-testid="card"]') || a.parentElement;
      const name = text(card?.querySelector('[dir="auto"]')) || a.getAttribute('aria-label') || text(a);
      return {id:linkId(a,'playlist'),name,images:image(card),owner:{id:null},collaborative:false};
    }).filter(row => /^[a-zA-Z0-9]{22}$/.test(row.id) && row.name);
    // A missing grid is a loading/error page, never an empty library.
    if (!grid && !rows.length) return {ready:false};
    // Responsive/loading placeholders must not become a successful empty import.
    if (total === 0 && !rows.length) return {ready:false};
  } else if (request.kind === 'collection') {
    const expected = request.id === 'liked' ? '/collection/tracks' : '/playlist/' + request.id;
    if (location.pathname !== expected) return {ready:false};
    grid = main.querySelector('[data-testid="playlist-tracklist"]');
    if (!grid) return {ready:false};
    root = grid;
    const count = Number(grid.getAttribute('aria-rowcount'));
    if (!Number.isInteger(count) || count < 1 || count > 20001) return {error:'unsupported-count'};
    total = count - 1;
    rows = [...grid.querySelectorAll('[data-testid="tracklist-row"]')].map(row => {
      const index = Number(row.closest('[role="row"][aria-rowindex]')?.getAttribute('aria-rowindex')) - 2;
      const a = row.querySelector('a[data-testid="internal-track-link"],a[href*="/track/"]');
      const id = linkId(a,'track');
      if (!id) return {index,item:null};
      const artists = [...row.querySelectorAll('a[href*="/artist/"]')].map(a => ({id:linkId(a,'artist'),name:text(a),uri:'spotify:artist:'+linkId(a,'artist')}));
      const album = row.querySelector('a[href*="/album/"]');
      const durationText = text([...row.querySelectorAll('[role="gridcell"]')].at(-1));
      const duration = durationText.match(/(?:^|\s)(\d{1,3}):([0-5]\d)(?::([0-5]\d))?(?:\s|$)/);
      let seconds = 0;
      if (duration) seconds = duration[3] ? +duration[1]*3600 + +duration[2]*60 + +duration[3] : +duration[1]*60 + +duration[2];
      return {index,item:{id,uri:'spotify:track:'+id,type:'track',name:text(a),artists,duration_ms:seconds*1000,
        explicit:!!row.querySelector('[aria-label="Explicit"],[title="Explicit"],[data-testid="explicit-indicator"]') || [...row.querySelectorAll('[data-encore-id="tagIcon"]')].some(el => text(el)==='E'),
        album:{id:linkId(album,'album') || '',name:text(album),images:image(row)},is_local:false}};
    });
  } else return {error:'unsupported-kind'};
  // Scroll only the rendered list's own viewport, without changing filters or songs.
  let viewport = grid || root;
  while (viewport && !(viewport.scrollHeight > viewport.clientHeight + 2 && /auto|scroll/.test(getComputedStyle(viewport).overflowY))) viewport = viewport.parentElement;
  const atEnd = !viewport || viewport.scrollTop + viewport.clientHeight >= viewport.scrollHeight - 4;
  if (request.reset && viewport) viewport.scrollTop = 0;
  if (request.scroll && viewport) viewport.scrollTop = Math.min(viewport.scrollHeight, viewport.scrollTop + Math.max(100,viewport.clientHeight*0.75));
  const owner = [...main.querySelectorAll('a[href*="/user/"]')].find(a => linkId(a,'user'));
  return {ready:true,rows,total,positions,atEnd,collapsed,title:text(main.querySelector('h1')),images:image(main),
    owner:{id:owner ? linkId(owner,'user') : null},path:location.pathname};
})
