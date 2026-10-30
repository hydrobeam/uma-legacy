// ---------- uma.moe: the key, and the listings the planner borrows from ----------
// One button beside Next career ("Borrow from uma.moe", then "uma.moe · 61 parents") opens a popup: the key until one is
// saved, then the status, what was searched, every listing as a card (the next career's first; a card opens her in the
// veteran popup, with her trainer's Copy ID), Refresh and Disconnect.
// With a key set, the veterans uma.moe lists for the target set (parent with at least n - 1 of the n targets herself,
// followable trainers, best covered first: one request, kept 24 h) join the pool as borrowed veterans (vets.js
// `setListings`), so every search and plan weighs them like any other borrow. A request goes out only when the target
// set has no answer kept (on saving the key, after a target change) or on Refresh; edits to veterans or settings never
// send one. Global data only (uma.moe lists Global veterans).
import { $, $dialog, $input, el } from './dom.js';
import { ERA } from './settings.js';
import { state } from './state.js';
import { vets, setListings, umaName, blueName, pinkName, uniqueName } from './vets.js';
import { parseListing } from './umadump.js';
import { umaAv, srcFor } from './images.js';
import { CAREER } from './data.js';
import {
  savedKey,
  saveKey,
  targetFactors,
  keptAnswer,
  search,
  SearchError,
  fullTrainers,
  setFullTrainers,
} from './umamoe.js';

/**
 * What the popup needs from the career page.
 * @typedef {object} UmamoeCtx
 * @property {() => import('./vets.js').Veteran|null} borrowed  the uma.moe veteran the next career borrows, if any
 * @property {(v: import('./vets.js').Veteran) => void} open  her veteran popup (sparks, parents, her trainer's Copy ID)
 */
/** @type {UmamoeCtx} */
let ctx;
/** @type {{sig: string, text: string, short: string, min: number, n: number}} the target set the pool's listings are
 *  for ('' none), the popup's status and the button's, and what was searched */
const now = { sig: '', text: '', short: '', min: 0, n: 0 };
let asking = 0; // the request under way; a newer one (or a forgotten key) makes it stale

const sigOf = () => `${ERA}|${state.targets.map((t) => t.group).join(',')}|${fullTrainers() ? 'full' : ''}`;
// the query for the target set: parents carrying at least n - 1 of the n targets herself, the first 100
function query() {
  const factors = targetFactors(state.targets.map((t) => t.group));
  return factors.length ? { factors, min: Math.max(1, factors.length - 1), page: 0, full: fullTrainers() } : null;
}
const ago = (at) => `${Math.max(1, Math.round((Date.now() - at) / 3600e3))} h`;
/** @param {{items: any[], total: number, at: number}} a @param {{min: number, factors: number[][]}} q */
function take(a, q) {
  setListings(a.items.map(parseListing));
  const n = a.items.length;
  now.text = `${n} veterans found, searched ${ago(a.at)} ago`;
  now.short = `${n} veterans`;
  now.min = q.min;
  now.n = q.factors.length;
}

// the popup's contents and the button's label
export function paintUmamoe() {
  if (ERA === 'gm') return;
  const has = !!savedKey();
  $('um-keyrow').hidden = has;
  $('um-on').hidden = !has;
  $('um-msg').textContent = now.text;
  $('um-err').textContent = has ? '' : now.text;
  $('um-open').textContent = has || now.short ? `uma.moe · ${now.short || now.text}` : 'Borrow from uma.moe';
  $('um-what').textContent = now.sig
    ? `Parents with ${now.min}+ of your ${now.n} targets${fullTrainers() ? '' : ', from trainers you can follow'}`
    : '';
  $input('um-full').checked = fullTrainers();
  // the listings as cards, the next career's first (tagged)
  const b = ctx?.borrowed(),
    list = $('um-list');
  list.textContent = '';
  for (const v of vets.filter((x) => x.src === 'umamoe').sort((x, y) => Number(y === b) - Number(x === b)))
    list.appendChild(card(v, v === b));
}
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
  // (the legacy list's own pill for the tag, its star row for the stars)
  const tag = (kind, label, full, stars) => {
    const t = el('span', `umsp ${kind}`),
      src = srcFor('ui', `fstar_${kind}_${Math.min(3, Math.max(1, stars))}`);
    t.appendChild(el('b', '', label));
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
  if (bl) row.appendChild(tag('blue', BLUE_TAG[bl[0]] || '?', blueName(+bl[0]), bl[1]));
  if (pk) row.appendChild(tag('pink', PINK_TAG[pk[0]] || '?', pinkName(+pk[0]), pk[1]));
  if (gr) row.appendChild(tag('green', 'UNIQ', uniqueName(+gr[0]) || 'Unique skill', gr[1]));
  return row;
}
/** @param {import('./vets.js').Lender|undefined} l */
const followersText = (l) =>
  !l ? '' : l.followers >= 1000 ? 'max' : `${l.followers} follower${l.followers === 1 ? '' : 's'}`;
// a listing as the game's legacy list draws a veteran: her portrait, her three main sparks under it, then her trainer's
// name and followers; a click opens her veteran popup (her sparks, her parents, Copy ID)
function card(v, next) {
  const c = el('button', `umcell${next ? ' umnext' : ''}`);
  c.type = 'button';
  c.setAttribute('aria-haspopup', 'dialog');
  c.setAttribute('aria-label', `${umaName(v.chara)}, ${v.lender?.name || 'uma.moe'}: her sparks and parents`);
  c.title = `${umaName(v.chara)} · ${v.lender?.name || ''}`;
  const face = el('span', 'umface');
  face.appendChild(umaAv(v.chara, v.outfit, 64));
  // her career-rank badge, top right of her portrait, as in the game (the last rank whose minimum her score reaches)
  const rankId = (CAREER.ranks || []).reduce((at, min, i) => (v.rank >= min ? i + 1 : at), 0),
    badge = rankId && srcFor('ui', `vrank_${rankId}`);
  if (badge) {
    const b = el('img', 'umrank');
    b.src = badge;
    b.alt = 'Rank';
    face.appendChild(b);
  }
  c.append(
    face,
    mainSparks(v),
    el('span', 'nm', v.lender?.name || umaName(v.chara)),
    el('span', 'sub', followersText(v.lender)),
  );
  if (next) c.appendChild(el('span', 'umtag', 'Next'));
  c.onclick = () => ctx.open(v);
  return c;
}

// the pool's listings for the target set as it is now: the kept answer at once, else one request (`fresh`: always one)
/** @param {boolean} [fresh] */
export async function syncListings(fresh = false) {
  const key = savedKey(),
    sig = sigOf(),
    q = query();
  if (ERA === 'gm' || !key || !q) {
    if (now.sig) {
      now.sig = '';
      setListings([]);
    }
    now.text = now.short = !q ? 'No targets' : '';
    paintUmamoe();
    return;
  }
  if (sig === now.sig && !fresh) return;
  const kept = fresh ? null : keptAnswer(ERA, q);
  if (kept) {
    now.sig = sig;
    take(kept, q);
    paintUmamoe();
    return;
  }
  const id = ++asking;
  now.text = now.short = 'searching…';
  paintUmamoe();
  $('um-again').toggleAttribute('disabled', true);
  try {
    const a = await search(key, ERA, q, fresh);
    if (id !== asking) return;
    now.sig = sig;
    take(a, q);
  } catch (e) {
    if (id !== asking) return;
    now.text = now.short = e instanceof SearchError ? e.message : 'Search failed';
    // a refused key is forgotten: the key field comes back, with why, for another
    if (e instanceof SearchError && e.badKey) {
      saveKey('');
      now.short = 'key rejected';
    }
  } finally {
    if (id === asking) $('um-again').toggleAttribute('disabled', false);
    paintUmamoe();
  }
}

/** @param {UmamoeCtx} c */
export function initUmamoe(c) {
  ctx = c;
  if (ERA === 'gm') return; // (the button stays hidden: Global only)
  const d = $dialog('um-dlg');
  $('um-open').hidden = false;
  $('um-open').onclick = () => {
    paintUmamoe();
    d.showModal();
    if (!savedKey()) $input('um-key').focus();
  };
  for (const x of d.querySelectorAll('.dclose')) /** @type {HTMLElement} */ (x).onclick = () => d.close();
  d.addEventListener('click', (e) => {
    if (e.target === d) d.close();
  });
  $('um-save').onclick = () => {
    const key = $input('um-key').value.trim();
    if (!key) return;
    saveKey(key);
    $input('um-key').value = '';
    syncListings();
  };
  $input('um-key').onkeydown = (e) => {
    if (e.key === 'Enter') $('um-save').click();
  };
  $('um-again').onclick = () => syncListings(true);
  $input('um-full').onchange = () => {
    setFullTrainers($input('um-full').checked);
    syncListings();
  };
  $('um-forget').onclick = () => {
    asking++;
    saveKey('');
    now.text = now.short = '';
    syncListings();
  };
  paintUmamoe();
}
