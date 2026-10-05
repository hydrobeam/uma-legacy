// ---------- roster core: the tiles, their order and state, and their markup ----------
// No DOM and no page state: shared by the page (roster.js), the inline script that orders the pre-rendered roster before
// the first paint (rosterearly.js) and the build (prerender.mjs writes the Global roster into the HTML), so all three
// produce the same tiles.
export const ST_LABEL = { own: 'owned', out: 'not owned', req: 'must include' };
// one tile per outfit (an uma without separate outfits: one tile)
export function rosterTiles(umas) {
  const tiles = [];
  for (const u of umas) for (const o of (u.outfits && u.outfits.length ? u.outfits : [null]))
    tiles.push({ u, o, rel: o ? o.rel : 0, key: u.name + ' ' + (o ? o.label : '') });
  return tiles;
}
// a tile's comparison fields, from its data (tiles) or from its element's attributes (the pre-rendered roster)
const fields = (t) => t.dataset ? { rel: +t.dataset.rel, oid: +(t.dataset.o || 0), key: t.dataset.key } : { rel: t.rel, oid: t.o ? t.o.id : 0, key: t.key };
// release (oldest first, ties by outfit id) or name (A to Z); 'desc' reverses the whole order
export function compareTiles(rsort, rdir) {
  const dir = rdir === 'desc' ? -1 : 1;
  return (a, b) => { const x = fields(a), y = fields(b);
    return dir * (rsort === 'name' ? x.key.localeCompare(y.key) : (x.rel - y.rel) || (x.oid - y.oid)); };
}
// owned / not owned / must include, from the saved roster ({uma id: 'out' | 'req'}) and outfits ({outfit id: false | 'req'})
export function tileState(roster, outfits, umaId, outfitId) {
  if (roster[umaId] === 'out' || (outfitId && outfits[outfitId] === false)) return 'out';
  return (outfitId ? outfits[outfitId] === 'req' : roster[umaId] === 'req') ? 'req' : 'own';
}
export const tileLabel = (name, outfitLabel, st) => name + (outfitLabel ? ' ' + outfitLabel : '') + ': ' + ST_LABEL[st];
export function initials(name) { const w = name.replace(/[^A-Za-z .]/g, '').split(/[ .]+/).filter(Boolean); return ((w[0] || '?')[0] + (w[1] ? w[1][0] : (w[0] || '')[1] || '')).toUpperCase(); }
const esc = (s) => String(s).replace(/&/g, '&amp;').replace(/</g, '&lt;').replace(/>/g, '&gt;').replace(/"/g, '&quot;');
// a tile's markup. src: its portrait (outfit icon, else the uma's; null: initials); eager: in the first screenful
export function tileHTML(t, { src, eager, state }) {
  const { u, o } = t, label = esc(tileLabel(u.name, o && o.label, state));
  const img = src ? `<img alt="" loading="${eager ? 'eager' : 'lazy'}"${eager ? ' fetchpriority="high"' : ''} src="${esc(src)}">` : esc(initials(u.name));
  return `<div class="ut" data-uma="${u.id}" data-o="${o ? o.id : ''}" data-rel="${t.rel}" data-key="${esc(t.key)}" data-s="${state}">` +
    `<button class="up" type="button" aria-label="${label}" title="${label}">` +
    `<span class="av" style="width: 96px; height: 96px; font-size: 36px; --h: ${(u.id * 47) % 360};">${img}</span></button>` +
    `<div class="un">${esc(u.name)}</div>` + (o ? `<div class="uol">${esc(o.label)}</div>` : '') + '</div>';
}
export const EAGER = 8;   // the first two rows are on screen at load (phone and desktop): their portraits load at once
