// ---------- character icons, card art and skill icons: files in img/ next to the page (from the game's own files) ----------
// All lazy: the browser fetches an image only when it scrolls near the view (loading is set before src).
import { SHIPPED, IMG_EXT, IMG_BASE, SKILLICON, SKILLICON_GOLD, NAME } from './data.js';
export function srcFor(kind, id) { return SHIPPED.has(kind + ':' + id) ? `${IMG_BASE}${kind}/${id}.${IMG_EXT[kind]}` : null; }
// the UI sprites (--btn-w, --spark-white, ...) are CSS variables the build writes into the stylesheet (build.py SPRITES)
export function skillIcon(group, gold = false) { const s = srcFor('skill', (gold ? SKILLICON_GOLD : SKILLICON)[group]); if (!s) return null;
  const im = new Image(); im.className = 'sic'; im.alt = ''; im.loading = 'lazy'; im.src = s; return im; }
function initials(name) { const w = name.replace(/[^A-Za-z .]/g, '').split(/[ .]+/).filter(Boolean); return ((w[0] || '?')[0] + (w[1] ? w[1][0] : (w[0] || '')[1] || '')).toUpperCase(); }
// eager: an image in the first screenful (the roster's first rows): fetched at once, at high priority, not lazily
export function umaAv(charaId, outfitId, size, eager = false) {
  const el = document.createElement('span'); el.className = 'av'; el.style.width = el.style.height = size + 'px'; el.style.fontSize = Math.round(size * 0.38) + 'px';
  el.style.setProperty('--h', String((charaId * 47) % 360)); el.textContent = initials(NAME.get(charaId) || '?');
  const tries = [outfitId && srcFor('outfit', outfitId), srcFor('uma', charaId), srcFor('mob', charaId)].filter(Boolean);   // a mob (sample veteran) has her own portrait
  // with a source, the disc is left blank (its colour) until the portrait loads; the initials show only when there is no
  // art or every source fails
  const next = () => { const s = tries.shift(); el.textContent = ''; if (!s) { el.textContent = initials(NAME.get(charaId) || '?'); return; }
    const im = new Image(); im.alt = ''; im.loading = eager ? 'eager' : 'lazy'; if (eager) im.fetchPriority = 'high'; im.onerror = next; im.src = s; el.appendChild(im); };
  if (tries.length) next(); return el;
}
const KABBR = { speed: 'SPD', stamina: 'STA', power: 'POW', guts: 'GUT', wit: 'WIT', pal: 'PAL', group: 'GRP', other: '·' };
export function cardAv(c, size) {
  const el = document.createElement('span'); el.className = 'cav'; el.style.width = el.style.height = size + 'px'; el.style.fontSize = Math.max(7, Math.round(size * 0.3)) + 'px';
  const kind = (c && c.kind) || 'other'; el.style.setProperty('--k', `var(--k-${kind})`);
  el.style.setProperty('--rr', c ? `var(--r-${({ 3: 'ssr', 2: 'sr', 1: 'r' })[c.rar] || 'r'})` : 'transparent');
  el.textContent = KABBR[kind] || '·'; el.title = c ? c.name : '';
  const s = c && srcFor('card', c.id);
  if (s) { const im = new Image(); im.alt = ''; im.loading = 'lazy'; im.src = s; el.textContent = ''; el.classList.add('art'); el.appendChild(im);
    im.onerror = () => { im.remove(); el.classList.remove('art'); el.textContent = KABBR[kind] || '·'; };
    const ty = srcFor('ui', 'type_' + kind); if (ty) { const i = document.createElement('i'); i.className = 'ctype'; i.title = kind; i.style.backgroundImage = `url(${ty})`; el.appendChild(i); } }
  return el;
}
// scenario logo (game title art, img/scen/{id}; drawn whole inside its box, aspect kept); without it, or if it fails
// to load, a badge with the scenario's name on a light shade of its colour
export function scenLogo(sc) {
  const el = document.createElement('span'); el.className = 'slogo'; el.setAttribute('aria-hidden', 'true');
  const badge = () => { el.classList.remove('art'); el.textContent = ''; const b = document.createElement('b'); b.textContent = sc.name; b.style.background = sc.tint; b.style.color = '#18202e';   /* dark text on the light tint, in either theme */ el.appendChild(b); };
  const s = srcFor('scen', sc.id);
  if (!s) { badge(); return el; }
  const im = new Image(); im.alt = ''; im.onerror = badge; im.src = s; el.classList.add('art'); el.appendChild(im);
  return el;
}
