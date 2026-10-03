// ---------- character icons, card art and skill icons, shipped with the page (from the game's own files) ----------
import { SHIPPED, SKILLICON, NAME } from './data.js';
for (const [v, k] of [['--friend-frame', 'ui:friend_frame']]) if (SHIPPED[k]) document.documentElement.style.setProperty(v, `url(data:image/webp;base64,${SHIPPED[k]})`);
export function srcFor(kind, id) { const b = SHIPPED[kind + ':' + id]; return b ? 'data:image/webp;base64,' + b : null; }
export function skillIcon(group) { const s = srcFor('skill', SKILLICON[group]); if (!s) return null;
  const im = new Image(); im.className = 'sic'; im.alt = ''; im.src = s; return im; }
function initials(name) { const w = name.replace(/[^A-Za-z .]/g, '').split(/[ .]+/).filter(Boolean); return ((w[0] || '?')[0] + (w[1] ? w[1][0] : (w[0] || '')[1] || '')).toUpperCase(); }
export function umaAv(charaId, outfitId, size) {
  const el = document.createElement('span'); el.className = 'av'; el.style.width = el.style.height = size + 'px'; el.style.fontSize = Math.round(size * 0.38) + 'px';
  el.style.setProperty('--h', String((charaId * 47) % 360)); el.textContent = initials(NAME.get(charaId) || '?');
  const tries = [outfitId && srcFor('outfit', outfitId), srcFor('uma', charaId)].filter(Boolean);
  // the image goes in at once (embedded data, no network); initials come back only if every source fails
  const next = () => { const s = tries.shift(); el.textContent = ''; if (!s) { el.textContent = initials(NAME.get(charaId) || '?'); return; }
    const im = new Image(); im.alt = ''; im.decoding = 'sync'; im.onerror = next; im.src = s; el.appendChild(im); };
  if (tries.length) next(); return el;
}
const KABBR = { speed: 'SPD', stamina: 'STA', power: 'POW', guts: 'GUT', wit: 'WIT', pal: 'PAL', group: 'GRP', other: '·' };
export function cardAv(c, size) {
  const el = document.createElement('span'); el.className = 'cav'; el.style.width = el.style.height = size + 'px'; el.style.fontSize = Math.max(7, Math.round(size * 0.3)) + 'px';
  const kind = (c && c.kind) || 'other'; el.style.setProperty('--k', `var(--k-${kind})`);
  el.style.setProperty('--rr', c ? `var(--r-${({ 3: 'ssr', 2: 'sr', 1: 'r' })[c.rar] || 'r'})` : 'transparent');
  el.textContent = KABBR[kind] || '·'; el.title = c ? c.name : '';
  const s = c && srcFor('card', c.id);
  if (s) { const im = new Image(); im.alt = ''; im.decoding = 'sync'; im.src = s; el.textContent = ''; el.classList.add('art'); el.appendChild(im);
    im.onerror = () => { im.remove(); el.classList.remove('art'); el.textContent = KABBR[kind] || '·'; };
    const ty = SHIPPED['ui:type_' + kind]; if (ty) { const i = document.createElement('i'); i.className = 'ctype'; i.title = kind; i.style.backgroundImage = `url(data:image/webp;base64,${ty})`; el.appendChild(i); } }
  return el;
}
