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
// so that effect subscribes to none of them. What it reads and writes of the page, the targets and the veterans store's
// listings, comes in through `connectUmamoe`, called once by the entry: it reads no store itself (.dependency-cruiser.mjs checks it).
import { batch, computed, signal } from '@preact/signals';
import { ERA } from './saved.ts';
import { must } from './must.ts';
import { read, update } from './storage.ts';
import { parseListing } from './umadump.ts';
import {
  savedKey,
  saveKey,
  targetFactors,
  keptAnswer,
  search,
  SearchError,
  fullTrainers,
  setFullTrainers,
} from './umamoe.ts';
import type { ReadonlySignal } from '@preact/signals';
import type { UmamoeListing } from './umadump.ts';
import type { SettingsView } from './settings.ts';
import type { Veteran, VetRecord } from './veterans/store.ts';

/** what the sync reads and writes of the page (career/main.tsx) */
export interface UmPage {
  /** the target skills now */
  targets: () => SettingsView['targets'];
  /** the pool's listings replaced (veterans/store.ts `setListings`) */
  setListings: (list: readonly Veteran[]) => void;
}
let page: UmPage | null = null;
/** the page's side of the sync, once, before any sync */
export function connectUmamoe(to: UmPage): void {
  page = to;
}
const pageNow = () => must(page, 'the entry connects uma.moe (connectUmamoe) before a sync');

/** The target set the pool's listings are for ('' none), the popup's status and the button's (`shown*`: the answer on
 *  show's, put back when a request for another set is dropped), and what was searched. */
interface UmStatus {
  sig: string;
  text: string;
  short: string;
  shownText: string;
  shownShort: string;
  min: number;
  n: number;
}
const status = signal<UmStatus>({ sig: '', text: '', short: '', shownText: '', shownShort: '', min: 0, n: 0 });
/** the listings' status (the popup's line, the button's label) */
export const umStatus: ReadonlySignal<UmStatus> = status;
const keyNow = signal(savedKey());
/** a key is saved */
export const hasKey = computed(() => !!keyNow.value);
const busy = signal(false);
/** a request is out (Refresh waits) */
export const umBusy: ReadonlySignal<boolean> = busy;
const full = signal(fullTrainers());
/** trainers at 1,000 followers included */
export const umFull: ReadonlySignal<boolean> = full;
// the request under way (`asking`, for the target set `pending`): any later call that changes or keeps the listings,
// or a forgotten key, makes it stale, so an older answer never lands over a newer one
let asking = 0,
  pending = '';
// (each a write: storage, then the signal)
const keyIs = (key: string) => {
  saveKey(key);
  keyNow.value = key;
};
const statusPatch = (patch: Partial<UmStatus>) => {
  status.value = { ...status.peek(), ...patch };
};

const sigOf = () =>
  `${ERA}|${pageNow()
    .targets()
    .map((t) => t.group)
    .join(',')}|${full.peek() ? 'full' : ''}`;
// the query for the target set: parents carrying at least n - 1 of the n targets herself, the first 100
function query() {
  const factors = targetFactors(
    pageNow()
      .targets()
      .map((t) => t.group),
  );
  return factors.length ? { factors, min: Math.max(1, factors.length - 1), page: 0, full: full.peek() } : null;
}
// how long ago the answer was fetched: "just now", "12 min ago", "3 h ago" (a kept answer is reused for 24 h)
const ago = (at: number): string => {
  const min = (Date.now() - at) / 60e3;
  return min < 1 ? 'just now' : min < 60 ? `${Math.round(min)} min ago` : `${Math.round(min / 60)} h ago`;
};
// the popup's orders, uma.moe's own: its search's order (best covered first), then by her rank, her affinity score, her
// race wins, her white sparks' stars, the latest updated; each starts best (or latest) first, and picking the chosen one
// again reverses it, as the umas' and cards' sort menus do; the choice is kept in this browser
/** the orders, in the menu's order */
export const UM_SORTS = ['match', 'rank', 'affinity', 'wins', 'sparks', 'updated'] as const;
export type UmSort = (typeof UM_SORTS)[number];
export const SORTS: Record<UmSort, [string, string, (v: VetRecord) => number]> = {
  match: ['Best match', 'uma.moe’s order: the best covered first', (v) => -(order.get(v.id) ?? 1e9)],
  rank: ['Rank', 'Her career rank', (v) => v.rank || 0],
  affinity: ['Affinity', 'Her affinity with her parents, as uma.moe scores it', (v) => v.lender?.affinity || 0],
  wins: ['Wins', 'Her race wins', (v) => v.lender?.wins || 0],
  sparks: ['Sparks', 'Her white sparks’ stars, summed', (v) => v.lender?.sparkStars || 0],
  updated: ['Updated', 'The latest updated first', (v) => Date.parse(v.lender?.updated || '') || 0],
};
const isUmSort = (k: string | undefined): k is UmSort => k !== undefined && Object.hasOwn(SORTS, k);
const SORT_KEY = 'llp:umamoe-sort';
const [keptSort, keptDir] = (read(SORT_KEY) || '').split(':'); // (storage blocked: by match)
const sort = signal<{ by: UmSort; reversed: boolean }>(
  isUmSort(keptSort) ? { by: keptSort, reversed: keptDir === 'rev' } : { by: 'match', reversed: false },
);
/** the popup's order */
export const umSort: ReadonlySignal<{ by: UmSort; reversed: boolean }> = sort;
/** an order picked: the one in use again reverses it; kept in this browser */
export function pickUmSort(k: UmSort): void {
  const now = sort.peek(),
    next = { by: k, reversed: k === now.by && !now.reversed };
  update(SORT_KEY, () => `${k}${next.reversed ? ':rev' : ''}`); // (storage blocked: for this visit only)
  sort.value = next;
}
/** each listing's place in uma.moe's answer */
const order = new Map<string, number>();

function take(
  a: { items: readonly UmamoeListing[]; total: number; at: number },
  q: { min: number; factors: readonly (readonly number[])[] },
): void {
  const read = a.items.map(parseListing),
    listed = read.map(({ veteran }) => veteran),
    unknownPinks = read.reduce((count, { unknownPinks }) => count + unknownPinks, 0);
  order.clear();
  for (const [i, v] of listed.entries()) order.set(v.id, i);
  const n = a.items.length,
    // (a pink of an aptitude the solver doesn't know is left out of her record: umadump.ts)
    leftOut = unknownPinks ? `, ${unknownPinks} unknown pink${unknownPinks === 1 ? '' : 's'} left out` : '';
  batch(() => {
    pageNow().setListings(listed);
    statusPatch({
      text: `${n} veterans found, searched ${ago(a.at)}${leftOut}`,
      shownText: `${n} veterans found, searched ${ago(a.at)}${leftOut}`,
      short: `${n} veterans`,
      shownShort: `${n} veterans`,
      min: q.min,
      n: q.factors.length,
    });
  });
}

// the pool's listings for the target set as it is now: the kept answer at once, else one request (`fresh`: always one)
export async function syncListings(fresh = false): Promise<void> {
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
        pageNow().setListings([]);
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
/** a key saved: its listings (kept, or one request) */
export function saveUmKey(key: string): void {
  if (!key) return;
  keyIs(key);
  syncListings();
}
/** Disconnect: the key forgotten, the listings gone */
export function forgetUmKey(): void {
  pending = ''; // so syncListings stales a request still out
  batch(() => {
    keyIs('');
    statusPatch({ text: '', short: '' });
  });
  syncListings();
}
/** Include max borrows: kept, and the listings for that choice */
export function setUmFull(on: boolean): void {
  setFullTrainers(on);
  full.value = on;
  syncListings();
}
