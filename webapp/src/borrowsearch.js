// ---------- Find a borrow on uma.moe: the search dialog (Veteran Umamusume, Borrowed tab) ----------
// One request to uma.moe (umamoe.js) lists lineages whose parent carries at least n - 1 of the n targets herself, best
// covered first. The planner then ranks every one by the best next career with her as the borrowed parent (one search
// per lineage, the parent place fixed on her; the pool's other veterans fill the other place), against today's best
// career, and prices the top three in today's plan (careers to a fully looped parent, under its fit, as Compare does).
// uma.moe's own ranking is never used for the order: pairing, affinity and carriers decide (measured 2026-10-23: the
// lineage with the best sparks alone was not the planner's pick). Wider searches (the next page, or one target fewer on
// the parent herself) are one more request each, only on a press.
import { $, $dialog, $input, el } from './dom.js';
import { ERA } from './settings.js';
import { state } from './state.js';
import { UMAS } from './data.js';
import { umaAv } from './images.js';
import { pct } from './odds.js';
import { umaName, jobLegacy } from './vets.js';
import { parseListing } from './umadump.js';
import { savedKey, saveKey, targetFactors, search, SearchError } from './umamoe.js';

/**
 * What the dialog needs from the career page.
 * @typedef {object} BorrowCtx
 * @property {() => any} baseJob  the job of today's best career (career.js `baseOf`); throws when no search can run
 * @property {(job: any) => Promise<any|null>} run  a solver job's answer (`r`); null when the worker restarted
 * @property {() => Promise<any|null>} baseline  today's best career (a CareerReport), without a new borrow
 * @property {() => number|null} planNow  today's plan's V (careers to a fully looped parent), when one is on show
 * @property {(legacy: any) => Promise<number|null>} planWith  V with one more veteran, under today's plan's fit
 * @property {(id: string) => number} poolIndex  a veteran's place in the pool, -1 when not there
 * @property {(i: number) => string} vetLabel  a pool veteran by number and name ("#12 Gold Ship")
 * @property {(v: import('./vets.js').Veteran, use: boolean) => void} add  into the borrow list; `use`: also a parent of
 *   the next career
 * @property {() => string} borrowNote  why the planner would not use a borrowed parent now ('' when it would)
 */
/** @type {BorrowCtx} */
let ctx;

const SHOWN = 10, // rows before "Show all"
  PRICED = 3, // rows priced in the plan
  TIE = 0.25; // careers: the page's tie tolerance

/**
 * The search so far: for one era and target set; listings by id, the last query, and each listing's ranking.
 * @typedef {{p: number, report: any, index: number}} Rank
 * @typedef {{sig: string, n: number, min: number, page: number, pages: number, total: number,
 *   found: Map<string, import('./vets.js').Veteran>, rankKey: string, ranks: Map<string, Rank>, base: Rank|null,
 *   saved: Map<string, number|null>, requests: number, all: boolean}} Session
 */
/** @type {Session|null} */
let session = null;
let busy = false,
  token = 0; // the ranking under way; a newer one (or a closed dialog) makes it stale

const targetSig = () => `${ERA}|${state.targets.map((t) => t.group).join(',')}`;
const msg = (text) => {
  $('um-msg').textContent = text;
};
const outfitName = (chara, outfit) => {
  const label = UMAS.find((u) => u.id === chara)?.outfits.find((o) => o.id === outfit)?.label;
  return label ? `${umaName(chara)} ${label}` : umaName(chara); // (a label carries its brackets)
};
// "3 h ago" / "2 d ago" from an ISO time
const ago = (iso) => {
  const h = (Date.now() - Date.parse(iso)) / 3600e3;
  if (!(h >= 0)) return '';
  return h < 1 ? 'within the hour' : h < 48 ? `${Math.round(h)} h ago` : `${Math.round(h / 24)} d ago`;
};

export function openBorrowSearch() {
  const d = $dialog('um-dlg');
  $input('um-key').value = savedKey();
  $input('um-key').type = 'password';
  $('um-show').textContent = 'Show';
  if (session && session.sig !== targetSig()) session = null;
  const gm = ERA === 'gm';
  $('um-go').toggleAttribute('disabled', gm || busy);
  if (gm) msg('uma.moe lists Global veterans: switch the data to Global to search it.');
  else if (!busy) msg(session ? '' : state.targets.length ? '' : 'Add target skills first.');
  draw();
  d.showModal();
}

// Search: the first request for this target set, or (once one has answered) the ranking again on the pool as it is now,
// without a request; `wider`: the next page, else one target fewer on the parent herself (one more request)
async function go(wider = false) {
  if (busy) return;
  const key = $input('um-key').value.trim();
  if (!key) {
    msg('Enter your uma.moe API key first.');
    $input('um-key').focus();
    return;
  }
  saveKey(key);
  $('um-forget').hidden = false;
  const factors = targetFactors(state.targets.map((t) => t.group));
  if (!factors.length) {
    msg('Add target skills first.');
    return;
  }
  if (!session || session.sig !== targetSig()) {
    const n = factors.length;
    session = {
      sig: targetSig(),
      n,
      min: Math.max(1, n - 1),
      page: -1,
      pages: 1,
      total: 0,
      found: new Map(),
      rankKey: '',
      ranks: new Map(),
      base: null,
      saved: new Map(),
      requests: 0,
      all: false,
    };
  }
  const s = session;
  if (s.page >= 0 && !wider) {
    setBusy(true);
    try {
      await rank(s);
    } finally {
      setBusy(false);
    }
    return;
  }
  // the next query: the first, the next page, or one target fewer on the parent herself
  const q =
    s.page < 0
      ? { min: s.min, page: 0 }
      : s.page + 1 < s.pages
        ? { min: s.min, page: s.page + 1 }
        : { min: s.min - 1, page: 0 };
  if (q.min < 1) return;
  setBusy(true);
  msg('Asking uma.moe…');
  try {
    const a = await search(key, ERA, { factors, ...q });
    if (session !== s) return;
    if (!a.cached) s.requests++;
    Object.assign(s, q, { pages: a.pages, total: a.total });
    for (const item of a.items) {
      const v = parseListing(item);
      s.found.set(v.id, v);
    }
    await rank(s);
  } catch (e) {
    msg(e instanceof SearchError ? e.message : `The search failed: ${e.message}`);
  } finally {
    setBusy(false);
  }
}
function setBusy(b) {
  busy = b;
  $('um-go').toggleAttribute('disabled', b);
  draw();
}

// every listing found, ranked by the best next career with her as a parent (only those not ranked on this pool yet)
/** @param {Session} s */
async function rank(s) {
  const id = ++token;
  let base;
  try {
    base = { ...ctx.baseJob(), borrowed_parent: undefined };
  } catch (e) {
    msg(`Not ranked: ${e.message}`);
    return;
  }
  const key = JSON.stringify(base);
  if (s.rankKey !== key) {
    s.rankKey = key;
    s.ranks.clear();
    s.saved.clear();
    s.base = null;
  }
  // each listing's legacy index: hers in the pool when already added, else after the pool
  const listed = [...s.found.values()],
    fresh = listed.filter((v) => ctx.poolIndex(v.id) < 0),
    n = base.legacies.length,
    legacies = [...base.legacies, ...fresh.map(jobLegacy)],
    at = (v) => (ctx.poolIndex(v.id) >= 0 ? ctx.poolIndex(v.id) : n + fresh.indexOf(v));
  if (!s.base) {
    const b = await ctx.baseline();
    if (id !== token) return;
    s.base = b ? { p: b.p_all, report: b, index: -1 } : null;
  }
  const todo = listed.filter((v) => !s.ranks.has(v.id));
  for (let k = 0; k < todo.length; k++) {
    msg(`Ranking ${k + 1} of ${todo.length} with the planner…`);
    const v = todo[k],
      index = at(v);
    const r = await ctx.run({ ...base, legacies, fix: [index], top: 1 });
    if (id !== token) return;
    if (r === null) {
      msg('Stopped: the planner started again. Search to finish the ranking (no new request).');
      return;
    }
    const best = r.results?.[0];
    s.ranks.set(v.id, { p: best ? best.p_all : 0, report: best || null, index });
    if (k % 10 === 9) draw();
  }
  draw();
  // the top rows in today's plan (one already in the pool is in today's plan)
  const now = ctx.planNow();
  if (now != null)
    for (const v of ordered(s).slice(0, PRICED)) {
      if (s.saved.has(v.id) || ctx.poolIndex(v.id) >= 0) continue;
      msg(`Pricing ${umaName(v.chara)} in the plan…`);
      const withHer = await ctx.planWith(jobLegacy(v));
      if (id !== token) return;
      s.saved.set(v.id, withHer == null ? null : now - withHer);
      draw();
    }
  msg(summary(s, now));
}
/** @param {Session} s */
const ordered = (s) =>
  [...s.found.values()].filter((v) => s.ranks.has(v.id)).sort((a, b) => s.ranks.get(b.id).p - s.ranks.get(a.id).p);
/** @param {Session} s @param {number|null} now */
function summary(s, now) {
  const req = s.requests ? `${s.requests} request${s.requests === 1 ? '' : 's'} to uma.moe` : 'from the 24 h cache';
  const plan = now == null ? ' The plan on show has no estimate to price them in (choose no places to get one).' : '';
  return `${s.found.size} lineage${s.found.size === 1 ? '' : 's'} ranked (${req}).${plan}`;
}

function draw() {
  const box = $('um-res');
  box.textContent = '';
  const note = ctx?.borrowNote();
  if (note) box.appendChild(el('p', 'hint', note));
  const s = session;
  if (!s?.ranks.size) return;
  const p0 = s.base?.p;
  if (p0 != null) box.appendChild(el('p', 'hint', `Today's best career: ${pct(p0)} to land every target.`));
  const rows = ordered(s),
    list = el('ol', 'umlist');
  for (const v of s.all ? rows : rows.slice(0, SHOWN)) list.appendChild(row(v, s, p0));
  box.appendChild(list);
  const more = el('div', 'row');
  if (!s.all && rows.length > SHOWN)
    more.appendChild(
      button(`Show all ${rows.length}`, '', () => {
        s.all = true;
        draw();
      }),
    );
  const wider =
    s.page + 1 < s.pages
      ? `Next ${Math.min(100, s.total - (s.page + 1) * 100)} of ${s.total} (1 more request)`
      : s.min > 1
        ? `Also parents with ${s.min - 1} of ${s.n} targets on herself (1 more request)`
        : '';
  if (wider) more.appendChild(button(wider, '', () => go(true))).toggleAttribute('disabled', busy);
  if (more.children.length) box.appendChild(more);
  box.appendChild(
    el(
      'p',
      'hint',
      s.page + 1 < s.pages || s.min > 1
        ? `Searched: parents carrying at least ${s.min} of the ${s.n} targets herself. A lineage with fewer on her can still win through her parents.`
        : `Searched every listed lineage that carries a target on its parent.`,
    ),
  );
}
const button = (label, cls, fn) => {
  const b = el('button', `btn${cls ? ` ${cls}` : ''}`, label);
  b.type = 'button';
  b.onclick = fn;
  return b;
};
// her best career's odds against today's best, relative
const versus = (gain) => {
  const k = Math.round(gain * 100);
  return k > 0 ? `+${k}% on today` : k < 0 ? `${-k}% below today` : "about today's";
};
/** @param {import('./vets.js').Veteran} v @param {Session} s @param {number|undefined} p0 */
function row(v, s, p0) {
  const r = s.ranks.get(v.id),
    li = el('li', 'umrow'),
    tg = state.targets.map((t) => t.group),
    carried = (nd) => tg.filter((g) => nd.sparks?.[g] > 0).length,
    [own, a, b] = [v, ...v.parents].map(carried);
  const faces = el('span', 'umfaces');
  faces.append(umaAv(v.chara, v.outfit, 48));
  const ps = el('span', 'vtps');
  for (const g of v.parents) ps.appendChild(umaAv(g.chara || 0, g.outfit, 26));
  faces.append(ps);
  const body = el('div', 'umbody');
  body.append(
    el('div', 'nm', outfitName(v.chara, v.outfit)),
    el(
      'div',
      'sub',
      `Targets: ${own} of ${tg.length} on her, ${a} and ${b} on her parents (${own + a + b} of ${3 * tg.length}).`,
    ),
  );
  if (r.report) {
    // (the other parent is one of the pool's: a career borrows one)
    const other = r.report.parents.find((i) => i !== r.index),
      gain = p0 ? r.p / p0 - 1 : null;
    body.append(
      el(
        'div',
        'umodds',
        `Best next career: ${r.report.trainee.name}, parents her and ${ctx.vetLabel(other)}: ${pct(r.p)}${gain == null ? '' : ` (${versus(gain)})`}.`,
      ),
    );
    const aff = r.report.slots?.find((x) => x.kind === 'parent' && x.legacy === r.index)?.aff;
    if (aff >= 200)
      body.append(
        el(
          'div',
          'sub',
          `Affinity ${aff}: inheritance runs about 3% below the model at 200 and over, so these odds are a little high.`,
        ),
      );
  } else body.append(el('div', 'umodds', 'No career with her as a parent (her uma is every candidate trainee).'));
  if (s.saved.has(v.id)) {
    const d = s.saved.get(v.id);
    body.append(
      el(
        'div',
        'sub',
        d == null
          ? 'Not priced in the plan.'
          : Math.abs(d) < TIE
            ? 'In the plan: about the same careers to a fully looped parent (within 0.25).'
            : d > 0
              ? `In the plan: saves about ${d.toFixed(1)} careers to a fully looped parent.`
              : `In the plan: about ${(-d).toFixed(1)} careers more (the estimate moves when the pool does).`,
      ),
    );
  }
  const l = v.lender;
  body.append(
    el(
      'div',
      'sub',
      `Trainer ${l.name || '(no name)'} · ${l.followers} follower${l.followers === 1 ? '' : 's'}${l.updated ? ` · updated ${ago(l.updated)}` : ''}`,
    ),
  );
  const acts = el('div', 'row umacts'),
    inPool = ctx.poolIndex(v.id) >= 0;
  acts.append(
    button('Copy trainer ID', '', async (e) => {
      const t = /** @type {HTMLButtonElement} */ (e.currentTarget);
      try {
        await navigator.clipboard.writeText(l.id);
        t.textContent = 'Copied';
      } catch {
        t.textContent = l.id;
      }
    }),
  );
  if (inPool) acts.append(el('span', 'hint', 'In your borrow list.'));
  else
    acts.append(
      button('Add to borrow list', '', () => {
        ctx.add(structuredClone(v), false);
        msg(`Added ${umaName(v.chara)} to your borrowed veterans.`);
        draw();
      }),
      button('Use in the next career', 'primary', () => {
        ctx.add(structuredClone(v), true);
        $dialog('um-dlg').close();
      }),
    );
  body.append(acts);
  li.append(faces, body);
  return li;
}

/** @param {BorrowCtx} c */
export function initBorrowSearch(c) {
  ctx = c;
  const d = $dialog('um-dlg');
  for (const x of d.querySelectorAll('.dclose')) /** @type {HTMLElement} */ (x).onclick = () => d.close();
  d.addEventListener('close', () => {
    token++; // a ranking under way stops drawing (the worker finishes its current job)
    if (busy) setBusy(false);
  });
  $('um-go').onclick = () => go();
  $('um-show').onclick = () => {
    const k = $input('um-key'),
      hide = k.type === 'text';
    k.type = hide ? 'password' : 'text';
    $('um-show').textContent = hide ? 'Show' : 'Hide';
  };
  $('um-forget').hidden = !savedKey();
  $('um-forget').onclick = () => {
    saveKey('');
    $input('um-key').value = '';
    $('um-forget').hidden = true;
    msg('Key forgotten.');
  };
}
