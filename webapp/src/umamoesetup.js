// ---------- uma.moe: the key, and the listings the planner borrows from ----------
// One button beside Next career ("Borrow from uma.moe", then "uma.moe · 61 veterans") opens a popup: the key until one
// is saved, then the status, what was searched, uma.moe's orders and every listing as a card (a card opens her in the
// veteran popup, with her trainer's followers and Copy ID), Refresh and Disconnect.
// With a key set, the veterans uma.moe lists for the target set (parent with at least n - 1 of the n targets herself,
// followable trainers, best covered first: one request, kept 24 h) join the pool as borrowed veterans (vets.js
// `setListings`), so every search and plan weighs them like any other borrow. A request goes out only when the target
// set has no answer kept (on saving the key, after a target change) or on Refresh; edits to veterans or settings never
// send one. uma.moe lists Global veterans: on the Grand Masters-era data it is asked only for the targets Global has
// (`targetFactors`).
import { $, $dialog, $input, el } from './dom.js';
import { ERA } from './saved.ts';
import { settings } from './settings.ts';
import { vets, setListings, umaName } from './vets.js';
import { parseListing } from './umadump.js';
import { vetCell } from './vetcell.js';
import { wireMenu } from './sortmenu.js';
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
import { must } from './must.ts';

/**
 * What the popup needs from the career page.
 * @typedef {object} UmamoeCtx
 * @property {(v: import('./vets.js').Veteran) => void} open  her veteran popup (sparks, parents, her trainer's Copy ID)
 */
/** @type {UmamoeCtx} */
let ctx;
/** @type {{sig: string, text: string, short: string, shownText: string, shownShort: string, min: number, n: number}}
 *  the target set the pool's listings are for ('' none), the popup's status and the button's (`shown*`: the answer on
 *  show's, put back when a request for another set is dropped), and what was searched */
const now = { sig: '', text: '', short: '', shownText: '', shownShort: '', min: 0, n: 0 };
// the request under way (`asking`, for the target set `pending`): any later call that changes or keeps the listings,
// or a forgotten key, makes it stale, so an older answer never lands over a newer one
let asking = 0,
  pending = '';

const sigOf = () => `${ERA}|${settings.value.targets.map((t) => t.group).join(',')}|${fullTrainers() ? 'full' : ''}`;
// the query for the target set: parents carrying at least n - 1 of the n targets herself, the first 100
function query() {
  const factors = targetFactors(settings.value.targets.map((t) => t.group));
  return factors.length ? { factors, min: Math.max(1, factors.length - 1), page: 0, full: fullTrainers() } : null;
}
// how long ago the answer was fetched: "just now", "12 min ago", "3 h ago" (a kept answer is reused for 24 h)
const ago = (at) => {
  const min = (Date.now() - at) / 60e3;
  return min < 1 ? 'just now' : min < 60 ? `${Math.round(min)} min ago` : `${Math.round(min / 60)} h ago`;
};
// the popup's orders, uma.moe's own: its search's order (best covered first), then by her rank, her affinity score, her
// race wins, her white sparks' stars, the latest updated; each starts best (or latest) first, and picking the chosen one
// again reverses it, as the umas' and cards' sort menus do; the choice is kept in this browser
/** @type {Record<string, [string, string, (v: import('./vets.js').Veteran) => number]>} */
const SORTS = {
  match: ['Best match', 'uma.moe’s order: the best covered first', (v) => -(order.get(v.id) ?? 1e9)],
  rank: ['Rank', 'Her career rank', (v) => v.rank || 0],
  affinity: ['Affinity', 'Her affinity with her parents, as uma.moe scores it', (v) => v.lender?.affinity || 0],
  wins: ['Wins', 'Her race wins', (v) => v.lender?.wins || 0],
  sparks: ['Sparks', 'Her white sparks’ stars, summed', (v) => v.lender?.sparkStars || 0],
  updated: ['Updated', 'The latest updated first', (v) => Date.parse(v.lender?.updated || '') || 0],
};
const SORT_KEY = 'llp:umamoe-sort';
let [sortBy, reversed] = (() => {
  try {
    const [k, dir] = (localStorage.getItem(SORT_KEY) || '').split(':');
    return k in SORTS ? [k, dir === 'rev'] : ['match', false];
  } catch {
    return ['match', false];
  }
})();
function paintSortMenu() {
  const arrow = reversed ? '↑' : '↓',
    btn = $('um-sort');
  btn.textContent = `${SORTS[sortBy][0]} ${arrow}`;
  btn.title = `${SORTS[sortBy][1]}${reversed ? ', reversed' : ''}`;
  btn.setAttribute('aria-label', `${btn.textContent}, order veterans`);
  for (const b of $('um-sort-menu').querySelectorAll('button')) {
    const k = must(b.dataset.k, 'a menu item names its key'),
      on = k === sortBy;
    b.setAttribute('aria-checked', String(on));
    b.textContent = SORTS[k][0] + (on ? ` ${arrow}` : '');
    b.title = on ? 'Click to reverse' : SORTS[k][1];
  }
}
/** @type {Map<string, number>} each listing's place in uma.moe's answer */
const order = new Map();

/** @param {{items: any[], total: number, at: number}} a @param {{min: number, factors: number[][]}} q */
function take(a, q) {
  const listed = a.items.map(parseListing);
  order.clear();
  for (const [i, v] of listed.entries()) order.set(v.id, i);
  setListings(listed);
  const n = a.items.length;
  now.text = now.shownText = `${n} veterans found, searched ${ago(a.at)}`;
  now.short = now.shownShort = `${n} veterans`;
  now.min = q.min;
  now.n = q.factors.length;
}

// the button shows once you have a veteran of your own (a career needs one of yours beside a borrow)
export function umamoeButton() {
  $('um-open').hidden = !vets.some((v) => !v.borrowed);
}
// the popup's contents and the button's label
function paintUmamoe() {
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
  paintSortMenu();
  // the listings as cards in the chosen order
  const by = SORTS[sortBy][2],
    sign = reversed ? -1 : 1,
    list = $('um-list');
  list.textContent = '';
  for (const v of vets.filter((x) => x.src === 'umamoe').sort((x, y) => sign * (by(y) - by(x))))
    list.appendChild(card(v));
}
// a listing as the game's legacy list draws a veteran, with her trainer's name; a click opens her veteran popup (her
// sparks, her parents, her trainer's followers, Copy ID)
const card = (v) =>
  vetCell(v, {
    name: v.lender?.name || umaName(v.chara),
    label: `${umaName(v.chara)}, ${v.lender?.name || 'uma.moe'}: her sparks and parents`,
    onClick: () => ctx.open(v),
  });

// the pool's listings for the target set as it is now: the kept answer at once, else one request (`fresh`: always one)
/** @param {boolean} [fresh] */
export async function syncListings(fresh = false) {
  const key = savedKey(),
    sig = sigOf(),
    q = query();
  if (!fresh && sig === pending) return; // its request is out
  const id = ++asking,
    wasOut = pending !== '';
  pending = '';
  $('um-again').toggleAttribute('disabled', false);
  if (!key || !q) {
    if (now.sig) {
      now.sig = '';
      setListings([]);
    }
    now.text = now.short = !q ? 'No targets' : '';
    paintUmamoe();
    return;
  }
  if (sig === now.sig && !fresh) {
    // back to the set on show while another's request was out: that one is stale, so its "searching…" goes
    if (wasOut) {
      now.text = now.shownText;
      now.short = now.shownShort;
      paintUmamoe();
    }
    return;
  }
  const kept = fresh ? null : keptAnswer(ERA, q);
  if (kept) {
    now.sig = sig;
    take(kept, q);
    paintUmamoe();
    return;
  }
  pending = sig;
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
    if (id === asking) {
      pending = '';
      $('um-again').toggleAttribute('disabled', false);
    }
    paintUmamoe();
  }
}

/** @param {UmamoeCtx} c */
export function initUmamoe(c) {
  ctx = c;
  const d = $dialog('um-dlg');
  umamoeButton();
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
  for (const k of Object.keys(SORTS)) {
    const b = el('button', '', SORTS[k][0]);
    b.type = 'button';
    b.setAttribute('role', 'menuitemradio');
    b.dataset.k = k;
    $('um-sort-menu').appendChild(b);
  }
  wireMenu('um-sort', (k) => {
    reversed = k === sortBy && !reversed;
    sortBy = k;
    try {
      localStorage.setItem(SORT_KEY, `${k}${reversed ? ':rev' : ''}`);
    } catch {
      // storage blocked: the sort lasts for this visit only
    }
    paintUmamoe();
  });
  $input('um-full').onchange = () => {
    setFullTrainers($input('um-full').checked);
    syncListings();
  };
  $('um-forget').onclick = () => {
    pending = ''; // so syncListings stales a request still out
    saveKey('');
    now.text = now.short = '';
    syncListings();
  };
  paintUmamoe();
}
