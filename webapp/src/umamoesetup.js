// ---------- uma.moe: the key, and the listings the planner borrows from ----------
// One button beside Next career ("Borrow from uma.moe", then "uma.moe · 61 veterans") opens a popup: the key until one
// is saved, then the status, what was searched, uma.moe's orders and every listing as a card (a card opens her in the
// veteran popup, with her trainer's followers and Copy ID), Refresh and Disconnect (career/view/Umamoe.tsx draws them
// from the signals here).
// With a key set, the veterans uma.moe lists for the target set (parent with at least n - 1 of the n targets herself,
// followable trainers, best covered first: one request, kept 24 h) join the pool as borrowed veterans (veterans/store.ts
// `setListings`), so every search and plan weighs them like any other borrow. A request goes out only when the target
// set has no answer kept (on saving the key, after a target change) or on Refresh; edits to veterans or settings never
// send one. uma.moe lists Global veterans: on the Grand Masters-era data it is asked only for the targets Global has
// (`targetFactors`). `syncListings` runs inside the career page's settings effect: it reads every signal with peek(),
// so that effect subscribes to none of them.
import { batch, computed, signal } from '@preact/signals-core';
import { ERA } from './saved.ts';
import { settings } from './settings.ts';
import { vets, setListings } from './veterans/store.ts';
import { parseListing } from './umadump.js';
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
/** @import { ReadonlySignal } from '@preact/signals-core' */
/** @import { VetRecord } from './veterans/store.ts' */

/**
 * The target set the pool's listings are for ('' none), the popup's status and the button's (`shown*`: the answer on
 * show's, put back when a request for another set is dropped), and what was searched.
 * @typedef {{sig: string, text: string, short: string, shownText: string, shownShort: string, min: number, n: number}} UmStatus
 */
const status = signal(
  /** @type {UmStatus} */ ({ sig: '', text: '', short: '', shownText: '', shownShort: '', min: 0, n: 0 }),
);
/** the listings' status (the popup's line, the button's label) */
export const umStatus = /** @type {ReadonlySignal<UmStatus>} */ (status);
const keyNow = signal(savedKey());
/** a key is saved */
export const hasKey = computed(() => !!keyNow.value);
const busy = signal(false);
/** a request is out (Refresh waits) */
export const umBusy = /** @type {ReadonlySignal<boolean>} */ (busy);
const full = signal(fullTrainers());
/** trainers at 1,000 followers included */
export const umFull = /** @type {ReadonlySignal<boolean>} */ (full);
// the request under way (`asking`, for the target set `pending`): any later call that changes or keeps the listings,
// or a forgotten key, makes it stale, so an older answer never lands over a newer one
let asking = 0,
  pending = '';
// (each a write: storage, then the signal)
const keyIs = (/** @type {string} */ key) => {
  saveKey(key);
  keyNow.value = key;
};
const statusPatch = (/** @type {Partial<UmStatus>} */ patch) => {
  status.value = { ...status.peek(), ...patch };
};

const sigOf = () =>
  `${ERA}|${settings
    .peek()
    .targets.map((t) => t.group)
    .join(',')}|${full.peek() ? 'full' : ''}`;
// the query for the target set: parents carrying at least n - 1 of the n targets herself, the first 100
function query() {
  const factors = targetFactors(settings.peek().targets.map((t) => t.group));
  return factors.length ? { factors, min: Math.max(1, factors.length - 1), page: 0, full: full.peek() } : null;
}
// how long ago the answer was fetched: "just now", "12 min ago", "3 h ago" (a kept answer is reused for 24 h)
const ago = (at) => {
  const min = (Date.now() - at) / 60e3;
  return min < 1 ? 'just now' : min < 60 ? `${Math.round(min)} min ago` : `${Math.round(min / 60)} h ago`;
};
// the popup's orders, uma.moe's own: its search's order (best covered first), then by her rank, her affinity score, her
// race wins, her white sparks' stars, the latest updated; each starts best (or latest) first, and picking the chosen one
// again reverses it, as the umas' and cards' sort menus do; the choice is kept in this browser
/** @type {Record<string, [string, string, (v: VetRecord) => number]>} */
export const SORTS = {
  match: ['Best match', 'uma.moe’s order: the best covered first', (v) => -(order.get(v.id) ?? 1e9)],
  rank: ['Rank', 'Her career rank', (v) => v.rank || 0],
  affinity: ['Affinity', 'Her affinity with her parents, as uma.moe scores it', (v) => v.lender?.affinity || 0],
  wins: ['Wins', 'Her race wins', (v) => v.lender?.wins || 0],
  sparks: ['Sparks', 'Her white sparks’ stars, summed', (v) => v.lender?.sparkStars || 0],
  updated: ['Updated', 'The latest updated first', (v) => Date.parse(v.lender?.updated || '') || 0],
};
const SORT_KEY = 'llp:umamoe-sort';
const sort = signal(
  (() => {
    try {
      const [k, dir] = (localStorage.getItem(SORT_KEY) || '').split(':');
      return k in SORTS ? { by: k, reversed: dir === 'rev' } : { by: 'match', reversed: false };
    } catch {
      return { by: 'match', reversed: false };
    }
  })(),
);
/** the popup's order */
export const umSort = /** @type {ReadonlySignal<{by: string, reversed: boolean}>} */ (sort);
/** an order picked: the one in use again reverses it; kept in this browser @param {string} k */
export function pickUmSort(k) {
  const now = sort.peek(),
    next = { by: k, reversed: k === now.by && !now.reversed };
  try {
    localStorage.setItem(SORT_KEY, `${k}${next.reversed ? ':rev' : ''}`);
  } catch {
    // storage blocked: the sort lasts for this visit only
  }
  sort.value = next;
}
/** @type {Map<string, number>} each listing's place in uma.moe's answer */
const order = new Map();
/** the listings in the popup's order */
export const listings = computed(() => {
  const { by, reversed } = sort.value,
    score = SORTS[by][2],
    sign = reversed ? -1 : 1;
  return vets.value.filter((v) => v.src === 'umamoe').sort((x, y) => sign * (score(y) - score(x)));
});

/** @param {{items: any[], total: number, at: number}} a @param {{min: number, factors: number[][]}} q */
function take(a, q) {
  const listed = a.items.map(parseListing);
  order.clear();
  for (const [i, v] of listed.entries()) order.set(v.id, i);
  const n = a.items.length;
  batch(() => {
    setListings(listed);
    statusPatch({
      text: `${n} veterans found, searched ${ago(a.at)}`,
      shownText: `${n} veterans found, searched ${ago(a.at)}`,
      short: `${n} veterans`,
      shownShort: `${n} veterans`,
      min: q.min,
      n: q.factors.length,
    });
  });
}

// the pool's listings for the target set as it is now: the kept answer at once, else one request (`fresh`: always one)
/** @param {boolean} [fresh] */
export async function syncListings(fresh = false) {
  const key = keyNow.peek(),
    sig = sigOf(),
    q = query();
  if (!fresh && sig === pending) return; // its request is out
  const id = ++asking,
    wasOut = pending !== '';
  pending = '';
  busy.value = false;
  if (!key || !q) {
    batch(() => {
      if (status.peek().sig) {
        statusPatch({ sig: '' });
        setListings([]);
      }
      statusPatch({ text: !q ? 'No targets' : '', short: !q ? 'No targets' : '' });
    });
    return;
  }
  if (sig === status.peek().sig && !fresh) {
    // back to the set on show while another's request was out: that one is stale, so its "searching…" goes
    if (wasOut) statusPatch({ text: status.peek().shownText, short: status.peek().shownShort });
    return;
  }
  const kept = fresh ? null : keptAnswer(ERA, q);
  if (kept) {
    batch(() => {
      statusPatch({ sig });
      take(kept, q);
    });
    return;
  }
  pending = sig;
  batch(() => {
    statusPatch({ text: 'searching…', short: 'searching…' });
    busy.value = true;
  });
  try {
    const a = await search(key, ERA, q, fresh);
    if (id !== asking) return;
    batch(() => {
      statusPatch({ sig });
      take(a, q);
    });
  } catch (e) {
    if (id !== asking) return;
    const text = e instanceof SearchError ? e.message : 'Search failed';
    batch(() => {
      statusPatch({ text, short: text });
      // a refused key is forgotten: the key field comes back, with why, for another
      if (e instanceof SearchError && e.badKey) {
        keyIs('');
        statusPatch({ short: 'key rejected' });
      }
    });
  } finally {
    if (id === asking) {
      pending = '';
      busy.value = false;
    }
  }
}
/** a key saved: its listings (kept, or one request) @param {string} key */
export function saveUmKey(key) {
  if (!key) return;
  keyIs(key);
  syncListings();
}
/** Disconnect: the key forgotten, the listings gone */
export function forgetUmKey() {
  pending = ''; // so syncListings stales a request still out
  batch(() => {
    keyIs('');
    statusPatch({ text: '', short: '' });
  });
  syncListings();
}
/** Include max borrows: kept, and the listings for that choice @param {boolean} on */
export function setUmFull(on) {
  setFullTrainers(on);
  full.value = on;
  syncListings();
}
