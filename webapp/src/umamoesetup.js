// ---------- uma.moe on the Borrowed tab: the key, and the listings the planner borrows from ----------
// With a key set, the veterans uma.moe lists for the target set (parent with at least n - 1 of the n targets herself,
// followable trainers, best covered first: one request, kept 24 h) join the pool as borrowed veterans (vets.js
// `setListings`), so every search and plan weighs them like any other borrow and the answer names the trainer to follow.
// A request goes out only when the target set has no answer kept (on saving the key, after a target change) or on
// Search again; edits to veterans or settings never send one. Global data only (uma.moe lists Global veterans).
import { $, $input } from './dom.js';
import { ERA } from './settings.js';
import { state } from './state.js';
import { setListings } from './vets.js';
import { parseListing } from './umadump.js';
import { savedKey, saveKey, targetFactors, keptAnswer, search, SearchError } from './umamoe.js';

/** @type {{sig: string, text: string}} the target set the pool's listings are for ('' none), and the status line */
const now = { sig: '', text: '' };
let asking = 0; // the request under way; a newer one (or a forgotten key) makes it stale

const sigOf = () => `${ERA}|${state.targets.map((t) => t.group).join(',')}`;
// the query for the target set: parents carrying at least n - 1 of the n targets herself, the first 100
function query() {
  const factors = targetFactors(state.targets.map((t) => t.group));
  return factors.length ? { factors, min: Math.max(1, factors.length - 1), page: 0 } : null;
}
const ago = (at) => `${Math.max(1, Math.round((Date.now() - at) / 3600e3))} h`;
/** @param {{items: any[], total: number, at: number}} a */
function take(a) {
  setListings(a.items.map(parseListing));
  const n = a.items.length;
  now.text = `${n} to borrow · ${ago(a.at)}`;
}

function paint() {
  const has = !!savedKey();
  $('um-keyrow').hidden = has;
  $('um-on').hidden = !has;
  $('um-msg').textContent = now.text;
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
    now.text = !q ? 'No targets' : '';
    paint();
    return;
  }
  if (sig === now.sig && !fresh) return;
  const kept = fresh ? null : keptAnswer(ERA, q);
  if (kept) {
    now.sig = sig;
    take(kept);
    paint();
    return;
  }
  const id = ++asking;
  now.text = 'Searching…';
  paint();
  $('um-again').toggleAttribute('disabled', true);
  try {
    const a = await search(key, ERA, q, fresh);
    if (id !== asking) return;
    now.sig = sig;
    take(a);
  } catch (e) {
    if (id !== asking) return;
    now.text = e instanceof SearchError ? e.message : 'Search failed';
  } finally {
    if (id === asking) $('um-again').toggleAttribute('disabled', false);
    paint();
  }
}

export function initUmamoe() {
  if (ERA === 'gm') return; // (the row stays hidden: Global only)
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
  $('um-forget').onclick = () => {
    asking++;
    saveKey('');
    now.text = '';
    syncListings();
  };
  paint();
}
