// ---------- a veteran as the game's legacy list draws her ----------
// Her portrait in its ring (her head over the top), her career-rank badge, her three main sparks under it as the list's
// tags with their stars, a name under them; a label tab over her when she has a role (the game's "Trainee"). Used by
// the uma.moe popup's listings and by Veteran Umamusume.
import { el } from './dom.js';
import { careerTables } from './data.js';
import { umaAv, srcFor } from './images.js';
import { umaName, blueName, pinkName, uniqueName } from './vets.js';

// her main sparks as the game's legacy list shows them under a portrait: blue (her stat), pink (her aptitude), green
// (her unique skill), each a short tag of its colour with its stars under it
const BLUE_TAG = { 1: 'SPD', 2: 'STA', 3: 'PWR', 4: 'GUTS', 5: 'WIT' },
  PINK_TAG = {
    11: 'TURF',
    12: 'DIRT',
    21: 'FRONT',
    22: 'PACE',
    23: 'LATE',
    24: 'END',
    31: 'SPRT',
    32: 'MILE',
    33: 'MED',
    34: 'LONG',
  };
function mainSparks(v) {
  const row = el('span', 'umsps'),
    first = (o) => Object.entries(o || {}).find(([, st]) => st > 0);
  // (the legacy list's own tag image, its lettering included, and its star row; the letters alone where the image
  // isn't shipped)
  const tag = (kind, label, full, stars, image) => {
    const t = el('span', `umsp ${kind}`),
      src = srcFor('ui', `fstar_${kind}_${Math.min(3, Math.max(1, stars))}`),
      face = srcFor('ui', image);
    if (face) {
      const f = el('img', 'umtagimg');
      f.src = face;
      f.alt = label;
      t.appendChild(f);
    } else t.appendChild(el('b', '', label));
    if (src) {
      const st = el('img', 'umst');
      st.src = src;
      st.alt = `${stars}★`;
      t.appendChild(st);
    } else t.appendChild(el('i', '', '★'.repeat(stars)));
    t.title = `${full} ${stars}★`;
    return t;
  };
  const bl = first(v.blues),
    pk = first(v.pinks),
    gr = first(v.greens);
  if (bl) row.appendChild(tag('blue', BLUE_TAG[bl[0]] || '?', blueName(+bl[0]), bl[1], `ftag_blue_${bl[0]}`));
  if (pk) row.appendChild(tag('pink', PINK_TAG[pk[0]] || '?', pinkName(+pk[0]), pk[1], `ftag_pink_${pk[0]}`));
  if (gr) row.appendChild(tag('green', 'UNIQ', uniqueName(+gr[0]) || 'Unique skill', gr[1], 'ftag_green'));
  return row;
}
/**
 * @param {import('./vets.js').Veteran} v
 * @param {{name: string, label: string, onClick: () => void, tab?: string, sub?: string}} o  the name under her, the
 *   button's accessible label, what a click does, a role tab over her ("Parent 1"), a line under her name
 */
export function vetCell(v, { name, label, onClick, tab, sub }) {
  const c = el('button', 'umcell');
  c.type = 'button';
  c.setAttribute('aria-haspopup', 'dialog');
  c.setAttribute('aria-label', label);
  c.title = `${umaName(v.chara)} · ${name}`;
  const face = el('span', 'umface'),
    portrait = v.outfit && srcFor('vet', v.outfit);
  // her outfit's portrait as the game's legacy list draws a veteran (in its ring, her head over the top), else her icon
  if (portrait) {
    const p = el('img', 'umvet');
    p.src = portrait;
    p.alt = '';
    p.loading = 'lazy';
    p.onerror = () => p.replaceWith(umaAv(v.chara, v.outfit, 88));
    face.appendChild(p);
  } else face.appendChild(umaAv(v.chara, v.outfit, 88));
  // her career-rank badge, top right of her portrait, as in the game (the last rank whose minimum her score reaches)
  const rank = v.rank;
  const rankId = rank ? (careerTables().ranks || []).reduce((at, min, i) => (rank >= min ? i + 1 : at), 0) : 0,
    badge = rankId && srcFor('ui', `vrank_${rankId}`);
  if (badge) {
    const b = el('img', 'umrank');
    b.src = badge;
    b.alt = 'Rank';
    face.appendChild(b);
  }
  if (tab) face.appendChild(el('span', 'umtab', tab));
  c.append(face, mainSparks(v), el('span', 'nm', name));
  if (sub) c.appendChild(el('span', 'sub', sub));
  c.onclick = onClick;
  return c;
}
