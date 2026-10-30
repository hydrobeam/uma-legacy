// ---------- uma.moe: the key, and the listings the planner borrows from ----------
// One button beside Next career ("Borrow from uma.moe", then "uma.moe · 61 parents") opens a popup: the key until one is
// saved, then the status, what was searched, the listing the next career borrows (Copy ID), every listing, Refresh and
// Disconnect.
// With a key set, the veterans uma.moe lists for the target set (parent with at least n - 1 of the n targets herself,
// followable trainers, best covered first: one request, kept 24 h) join the pool as borrowed veterans (vets.js
// `setListings`), so every search and plan weighs them like any other borrow. A request goes out only when the target
// set has no answer kept (on saving the key, after a target change) or on Refresh; edits to veterans or settings never
// send one. Global data only (uma.moe lists Global veterans).
import { $, $dialog, $input, el } from './dom.js';
import { ERA } from './settings.js';
import { state } from './state.js';
import { vets, setListings, umaName, sparksEl, blueName, pinkName, uniqueName } from './vets.js';
import { parseListing } from './umadump.js';
import { umaAv } from './images.js';
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
 * @property {(v: import('./vets.js').Veteran) => HTMLElement} copyBtn  her trainer's Copy ID button
 */
/** @type {UmamoeCtx} */
let ctx;
/** @type {{sig: string, text: string, short: string, min: number, n: number}} the target set the pool's listings are
 *  for ('' none), the popup's status and the button's, and what was searched */
const now = { sig: '', text: '', short: '', min: 0, n: 0 };
let asking = 0; // the request under way; a newer one (or a forgotten key) makes it stale
let detail = ''; // the listing whose ancestry is on show ('' the grid)

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
  now.text = `${n} parents · ${ago(a.at)}`;
  now.short = `${n} parents`;
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
  $('um-open').textContent = has ? `uma.moe · ${now.short || now.text}` : 'Borrow from uma.moe';
  $('um-what').textContent = now.sig
    ? `Parents with ${now.min}+ of your ${now.n} targets${fullTrainers() ? '' : ', from trainers you can follow'}`
    : '';
  $input('um-full').checked = fullTrainers();
  // the grid of listings (the next career's first, tagged), or one listing's ancestry
  const b = ctx?.borrowed(),
    list = $('um-list'),
    box = $('um-detail'),
    all = vets.filter((x) => x.src === 'umamoe').sort((x, y) => Number(y === b) - Number(x === b)),
    open = all.find((v) => v.id === detail);
  list.textContent = '';
  box.textContent = '';
  list.hidden = !!open;
  box.hidden = !open;
  if (open) box.append(...ancestry(open));
  else for (const v of all) list.appendChild(card(v, v === b));
}
// a spark as a small chip of its colour: "Speed 3★"
const chip = (kind, name, stars) => el('span', `umsp ${kind}`, `${name} ${stars}★`);
// her main three sparks, as the game's legacy list shows them: blue, pink, green
function mainSparks(v) {
  const row = el('span', 'umsps'),
    first = (o) => Object.entries(o || {}).find(([, st]) => st > 0);
  const bl = first(v.blues),
    pk = first(v.pinks),
    gr = first(v.greens);
  if (bl) row.appendChild(chip('blue', blueName(+bl[0]), bl[1]));
  if (pk) row.appendChild(chip('pink', pinkName(+pk[0]), pk[1]));
  if (gr) row.appendChild(chip('green', uniqueName(+gr[0]) || 'Unique', gr[1]));
  return row;
}
/** @param {import('./vets.js').Lender|undefined} l */
const trainerLine = (l) =>
  l ? `${l.name} · ${l.followers >= 1000 ? 'full' : `${l.followers} follower${l.followers === 1 ? '' : 's'}`}` : '';
// a listing as a card: her face, name, main sparks, trainer and followers; a click shows her ancestry
function card(v, next) {
  const c = el('button', `umcard${next ? ' next' : ''}`),
    txt = el('span', 'umct');
  c.type = 'button';
  txt.append(el('span', 'nm', umaName(v.chara)), mainSparks(v), el('span', 'sub', trainerLine(v.lender)));
  c.append(umaAv(v.chara, v.outfit, 48), txt);
  if (next) c.appendChild(el('span', 'umtag', 'Next career'));
  c.onclick = () => {
    detail = v.id;
    paintUmamoe();
  };
  return c;
}
// her full ancestry: herself and her two parents with their sparks; her trainer with Copy ID
function ancestry(v) {
  const back = el('button', 'btn sm umback', 'Back');
  back.type = 'button';
  back.onclick = () => {
    detail = '';
    paintUmamoe();
  };
  const head = el('div', 'umline'),
    who = el('span', 'umct');
  who.append(el('span', 'nm', umaName(v.chara)), el('span', 'sub', trainerLine(v.lender)));
  head.append(umaAv(v.chara, v.outfit, 56), who, ctx.copyBtn(v));
  const rows = [v, ...v.parents].map((nd, i) => {
    const r = el('div', 'umanc'),
      top = el('div', 'umline');
    top.append(umaAv(nd.chara || 0, nd.outfit, 32), el('span', 'nm', nd.chara ? umaName(nd.chara) : 'Unknown'));
    if (i) top.appendChild(el('span', 'sub', 'parent'));
    r.append(
      top,
      sparksEl(nd, '', () => {}, true),
    );
    return r;
  });
  return [back, head, ...rows];
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
    detail = '';
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
