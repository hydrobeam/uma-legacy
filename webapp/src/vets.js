// ---------- the career page's veterans and lineage plan: storage, the spark editor, the uma and skills prompts ----------
/**
 * A spark map: group -> stars (1-3). Keys are object keys, so strings once read back with Object.keys / entries.
 * @typedef {Object<number, number>} Sparks
 *
 * One uma of a lineage: a veteran's parent, or the fields a veteran shares with one.
 * @typedef {object} LineageNode
 * @property {number|null} chara  her character id (null: not known yet)
 * @property {number} [outfit]  her outfit (card) id, when known
 * @property {Sparks} sparks  white sparks: skill group -> stars
 * @property {Sparks} pinks  pink sparks: aptitude group -> stars
 * @property {Sparks} [blues]  blue sparks: stat group (1 Speed .. 5 Wit) -> stars
 * @property {Sparks} [greens]  unique spark: outfit card id -> stars
 * @property {number[]} wins  G1 wins, as race indices into CAREER.races
 * @property {Object<number, number>} [winAt]  race index -> the turn its win shows at (display only; the solver reads `wins`)
 *
 * A veteran of the pool: a lineage node with an identity, her source and her two parents.
 * @typedef {LineageNode & {id: string, src: string, borrowed: boolean, parents: LineageNode[]}} Veteran
 *   src: where she came from: 'import' | 'tree' (uma.moe) | 'hand' | 'sample' | 'entered' (older saves: 'simulated')
 *
 * The places of the next career: each is chosen here or left to the planner (null).
 * @typedef {object} LineagePlan
 * @property {number|null} trainee  the trainee's character id
 * @property {(string|null)[]} parents  the two parent places, by veteran id
 */
// Saved per data era in localStorage['llp:career'] = {v: 2, global: {vets, plan}, gm: {vets, plan}}.
import { $, $dialog, $input } from './dom.js';
import { ERA } from './settings.js';
import { UMAS, NAME, SK, CAREER } from './data.js';
import { state, goldGiven } from './state.js';
import { umaAv, skillIcon, srcFor } from './images.js';
import { winsAgenda } from './agenda.js';

const STORAGE_KEY = 'llp:career', STORAGE_VERSION = 2;
// stars of a skill added by hand: 2, the most common star count of a white spark at SS rank (70% at rank score 17,500+,
// 45% below it; hakuraku.moe)
export const NEW_STARS = 2;
function loadAll() {
  try { const saved = JSON.parse(localStorage.getItem(STORAGE_KEY));
    if (saved && saved.v === STORAGE_VERSION) return saved;
    if (saved && saved.v === 1) {   // v1: a list of veterans per era
      const upgraded = { v: STORAGE_VERSION };
      for (const era of ['global', 'gm']) if (Array.isArray(saved[era])) upgraded[era] = { vets: saved[era] };
      return upgraded;
    }
  } catch {}
  return { v: STORAGE_VERSION };
}
const ALL = loadAll(), MINE = ALL[ERA] || {};
/** @type {Veteran[]} every veteran of this era; a veteran's number on the page is her index here plus one */
export const vets = (MINE.vets || []).map(v => ({ pinks: {}, ...v, parents: v.parents.map(p => ({ pinks: {}, ...p })) }));
/** @type {LineagePlan} */
export const plan = MINE.plan || { trainee: null, parents: [null, null] };
// Saving: `persist` writes now; an edit's `persistSoon` writes once after the next frame (edits in between share it), and
// a write still due goes out when the page is hidden or left (a reload or close loses nothing). `rev` counts the edits
// (each `persistSoon`, and a change of uma before its skills prompt answers): `legaciesJob` is rebuilt only after one.
let persistDue = false, rev = 0;
export function persist() { persistDue = false; ALL[ERA] = { vets, plan }; try { localStorage.setItem(STORAGE_KEY, JSON.stringify(ALL)); } catch {} }
export function persistSoon() {
  rev++;
  if (document.visibilityState === 'hidden') { persist(); return; }
  if (persistDue) return;
  persistDue = true; requestAnimationFrame(() => setTimeout(() => { if (persistDue) persist(); }));
}
const flushPersist = () => { if (persistDue) persist(); };
addEventListener('pagehide', flushPersist);
document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') flushPersist(); });

let onChange = () => {};
export function onVetsChange(fn) { onChange = fn; }
// every edit: saved, then the page redraws and searches again
export function changed() { persistSoon(); onChange(); }

/** @type {(id: number|null|undefined) => string} */
export const umaName = (id) => id ? (NAME.get(id) || `Uma ${id}`) : 'Unknown';
const targetGroups = () => state.targets.map(t => t.group);
/** @type {(g: number) => string} */
export const skillName = (g) => (SK.get(g) || { w: `Skill ${g}` }).w;
/** @type {(g: number|string) => string} */
export const pinkName = (g) => CAREER.pinks[g] || `Aptitude ${g}`;
/** @type {(g: number|string) => string} */
export const blueName = (g) => CAREER.blues?.[g] || ['', 'Speed', 'Stamina', 'Power', 'Guts', 'Wit'][g] || `Stat ${g}`;
/** @type {(g: number|string) => string|null} */
export const uniqueName = (g) => CAREER.uniques?.[g] || null;
/** @type {(id: string) => Veteran|undefined} */
export const vetById = (id) => vets.find(v => v.id === id);
const allTargetsAt = (stars) => Object.fromEntries(targetGroups().map(g => [g, stars]));

// ---------- dialogs ----------
for (const d of document.querySelectorAll('dialog')) {
  for (const b of d.querySelectorAll('.dclose')) b.onclick = () => d.close();
}
// the uma picker: every uma (or those `only` lets through), filterable; onPick(chara id)
let onUmaPicked = null;
/** @param {string} title @param {(chara: number) => void} onPick @param {((u: import('./data.js').Uma) => boolean)|null} [only] */
export function openUmaPicker(title, onPick, only = null) {
  onUmaPicked = onPick; $('pick-title').textContent = title; $input('pick-filter').value = '';
  drawUmas($('pick-list'), '', pickUma, only); $dialog('pick-dlg').showModal(); $input('pick-filter').focus();
}
function pickUma(id) { $dialog('pick-dlg').close(); const onPick = onUmaPicked; onUmaPicked = null; onPick?.(id); }
$input('pick-filter').oninput = () => filterUmas($('pick-list'), $input('pick-filter').value);
// the picker's list: every uma `only` lets through, the filter applied by hiding (typing doesn't rebuild it)
/** @param {HTMLElement} box @param {string} filter @param {(chara: number) => void} onPick @param {((u: import('./data.js').Uma) => boolean)|null} [only] */
export function drawUmas(box, filter, onPick, only = null) {
  box.textContent = '';
  for (const u of [...UMAS].sort((a, b) => a.name.localeCompare(b.name))) {
    if (only && !only(u)) continue;
    const b = document.createElement('button'); b.type = 'button'; b.className = 'upick'; b.append(umaAv(u.id, null, 40));
    const s = document.createElement('span'); s.textContent = u.name; b.append(s); b._nm = u.name.toLowerCase();
    b.onclick = () => onPick(u.id); box.appendChild(b);
  }
  const none = document.createElement('p'); none.className = 'hint'; none.textContent = 'No uma matches.'; box.appendChild(none);
  filterUmas(box, filter);
}
/** @param {HTMLElement} box @param {string} filter */
export function filterUmas(box, filter) {
  const f = filter.trim().toLowerCase(); let n = 0;
  for (const b of box.querySelectorAll('.upick')) { b.hidden = !!f && !b._nm.includes(f); n += Number(!b.hidden); }
  box.querySelector('p.hint').hidden = n > 0;
}
// after an uma is picked for a node that carries sparks: whether to give her the target skills. onChoice(sparks | null:
// keep what she has)
/** @param {string} name @param {boolean} hasSparks @param {(sparks: Sparks|null) => void} onChoice @param {boolean} [withParents] */
export function askSkills(name, hasSparks, onChoice, withParents = false) {
  const n = targetGroups().length;
  $('ask-title').textContent = `Add skills to ${name}?`;
  $('ask-text').textContent = `Her sparks of your ${n} target skills decide what she passes on. A fully looped parent has all of them` +
    (withParents ? ', and so do her own two parents: the same choice goes on them.' : '.') + ' You can change the stars of each afterwards.';
  const box = $('ask-btns'); box.textContent = '';
  // No skills first and the default (a new uma carries nothing until you say so)
  const choices = [{ label: 'No skills', sparks: () => ({}), primary: true }, { label: `All ${n} at 2★`, sparks: () => allTargetsAt(2) },
    { label: `All ${n} at 3★`, sparks: () => allTargetsAt(3) }, { label: `All ${n} at 1★`, sparks: () => allTargetsAt(1) },
    ...(hasSparks ? [{ label: 'Keep her current skills', sparks: () => null }] : [])];
  for (const { label, sparks, primary } of choices) {
    const b = document.createElement('button'); b.type = 'button'; b.className = `btn${primary ? ' primary' : ''}`; b.textContent = label;
    b.onclick = () => { $dialog('ask-dlg').close(); onChoice(sparks()); }; box.appendChild(b);
  }
  $dialog('ask-dlg').showModal(); box.querySelector('.primary').focus();
}
// the G1 wins of one node, on the schedule window's calendar (the editor in agenda.js): each win in its slot, click it to
// remove it; an empty slot where a G1 is held lists the races held then. A pool veteran's wins are saved on each edit, and
// the page searches again when the dialog closes. opts (the veteran analysis' draft): {offer, info, hint, onEdit, onClose}
// (see winsAgenda). nd.winAt {race index: turn} is where a win shows, for display only (the solver reads nd.wins).
const allRaces = () => CAREER.races.map((name, ix) => ({ ix, name, turns: (CAREER.race_turns || [])[ix] || [] }));
let winsClose = null;
export function openWins(nd, who, opts = {}) {
  const opener = /** @type {HTMLElement} */ (document.activeElement);
  $('wins-title').textContent = `G1 wins: ${who}`;
  const count = () => { $('wins-count').textContent = winsText(nd); };
  $('wins-list').replaceChildren(winsAgenda({ races: allRaces(), get: () => nd.wins, set: (w) => { nd.wins = w; }, at: () => (nd.winAt ||= {}),
    offer: opts.offer, info: opts.info, onEdit: () => { count(); (opts.onEdit || persistSoon)(); } }));
  count();
  // closing: the pool searches again; the focus goes back to the button that opened it (redrawn by then: found by its key)
  winsClose = opts.onClose || (() => { const k = opener?.dataset?.k; changed(); const b = k && document.querySelector(`[data-k="${CSS.escape(k)}"]`); (b || (opener?.isConnected ? opener : null))?.focus(); });
  $dialog('wins-dlg').showModal();
}
/** @type {(nd: LineageNode) => string} */
export const winsText = (nd) => nd.wins.length ? `${nd.wins.length} G1 win${nd.wins.length > 1 ? 's' : ''}` : 'No G1 wins';
$dialog('wins-dlg').addEventListener('close', () => { const f = winsClose; winsClose = null; f?.(); });
// "+" beside a node's sparks: the target skills she lacks, as the game's white spark cells (her blue, pink and unique
// sparks have bars of their own, each opening a picker)
const pop = $('add-pop'); let popFor = null;
function closeAdd() { pop.hidden = true; if (popFor) popFor.setAttribute('aria-expanded', 'false'); popFor = null; }
function openAdd(btn, nd, onEdit = changed) {
  if (popFor === btn) { closeAdd(); return; }
  closeAdd(); popFor = btn; btn.setAttribute('aria-expanded', 'true'); pop.textContent = '';
  const head = (t) => { const h = document.createElement('p'); h.className = 'hint'; h.textContent = t; pop.appendChild(h); };
  const missing = targetGroups().filter(g => !nd.sparks[g]);
  head(missing.length ? 'Target skills she lacks' : 'She has every target skill.');
  const list = document.createElement('div'); list.className = 'addlist';
  for (const g of missing) {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'addsk';
    b.append(Object.assign(document.createElement('span'), { className: 'addn', textContent: skillName(g) }));
    b.onclick = () => { nd.sparks[g] = NEW_STARS; closeAdd(); onEdit(); };
    list.appendChild(b);
  }
  pop.appendChild(list);
  const r = btn.getBoundingClientRect();
  // inside an open (modal) dialog the page around it is inert: the menu moves into that dialog, placed relative to it
  const host = btn.closest('dialog[open]');
  if ((host || document.body) !== pop.parentElement) (host || document.body).appendChild(pop);
  if (host) { const h = host.getBoundingClientRect(); pop.style.left = `${Math.max(4, Math.min(r.left - h.left, h.width - 300))}px`; pop.style.top = `${r.bottom - h.top + host.scrollTop + 6}px`; }
  else { pop.style.left = `${Math.max(8, Math.min(r.left + scrollX, scrollX + innerWidth - 320))}px`; pop.style.top = `${r.bottom + scrollY + 6}px`; }
  pop.hidden = false; pop.focus();
}
document.addEventListener('click', (/** @type {TargetedEvent} */ e) => { if (!pop.hidden && !pop.contains(e.target) && !(popFor?.contains(e.target))) closeAdd(); });
document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !pop.hidden) { const b = popFor; closeAdd(); b?.focus(); } });

// ---------- a node's sparks ----------
// A spark's stars: filled then empty, out of 3 (two spans: the career page's game-style card colours them apart), hidden
// from the screen reader (the control's label says the count).
function starsEl(stars) {
  const box = document.createElement('span'); box.className = 'stars'; box.setAttribute('aria-hidden', 'true');
  const filled = document.createElement('span'); filled.className = 'sf'; filled.textContent = '★'.repeat(stars); filled.style.setProperty('--n', String(stars));
  const empty = document.createElement('span'); empty.className = 'se'; empty.textContent = '☆'.repeat(3 - stars); empty.style.setProperty('--n', String(3 - stars));
  box.append(filled, empty); return box;
}
/**
 * One spark as a chip: a target skill (click: stars 1 -> 2 -> 3 -> 1; the corner x removes) or a pink.
 * @typedef {object} ChipSpec
 * @property {string} label  the spark's name
 * @property {Node|null} icon  the skill's icon, if any
 * @property {number} stars  1-3
 * @property {boolean} pink  an aptitude (pink) spark
 * @property {() => void} onCycle  a click on the chip
 * @property {() => void} onRemove  a click on its corner x
 * @property {string} key  names the controls (data-k), so the page can put the keyboard focus back on the same one after
 *   a redraw
 * @property {boolean} [readOnly]  shown, not editable: no x, no click
 */
/** @param {ChipSpec} spec */
function chip({ label, icon, stars, pink, onCycle, onRemove, key, readOnly = false }) {
  const c = document.createElement('span'); c.className = `vsk${pink ? ' pink' : ''}`;
  const b = document.createElement(readOnly ? 'span' : 'button'); if (!readOnly) /** @type {HTMLButtonElement} */ (b).type = 'button'; b.className = 'vskb';
  if (icon) b.appendChild(icon);
  const t = document.createElement('span'); t.className = 'vskn'; t.textContent = label; b.appendChild(t);
  b.appendChild(starsEl(stars));
  if (readOnly) { b.setAttribute('aria-label', `${label}${pink ? ' (aptitude)' : ''}, ${stars} stars`); c.appendChild(b); return c; }
  b.setAttribute('aria-label', `${label}${pink ? ' (aptitude)' : ''}, ${stars} star${stars > 1 ? 's' : ''}. Click for ${stars % 3 + 1}`);
  b.onclick = onCycle; b.dataset.k = key;
  const x = document.createElement('button'); x.type = 'button'; x.className = 'vskx'; x.textContent = '×';
  x.setAttribute('aria-label', `Remove ${label}`); x.title = `Remove ${label}`; x.onclick = onRemove; x.dataset.k = `${key}x`;
  c.append(b, x); return c;
}
// A veteran's sparks as the game's Inspiration cells: her blue (stat), pink (aptitude) and unique (green) sparks first,
// each a bar of its colour (a dashed one when she has none) that opens its picker; then her target skills' white sparks
// (click: stars 1 -> 2 -> 3 -> 1, the corner x removes), then "+" (a target skill she lacks). Sparks of skills that are
// not targets are kept but not shown (read-only: shown). key names the controls (data-k) for the focus after a redraw.
/** @param {LineageNode} nd @param {string} [key] @param {() => void} [onEdit] @param {boolean} [readOnly] @returns {HTMLDivElement} */
export function sparksEl(nd, key = '', onEdit = changed, readOnly = false) {
  const sp = document.createElement('div'); sp.className = 'vsparks';
  for (const kind of ['blue', 'pink', 'green']) { const b = colourBar(nd, kind, key, onEdit, readOnly); if (b) sp.appendChild(b); }
  for (const g of readOnly ? Object.keys(nd.sparks).map(Number) : targetGroups()) {
    const stars = nd.sparks[g]; if (!stars) continue;
    sp.appendChild(chip({ label: skillName(g), icon: skillIcon(g, false), stars, pink: false, key: `${key}:s${g}`, readOnly,
      onCycle: () => { nd.sparks[g] = stars % 3 + 1; onEdit(); }, onRemove: () => { delete nd.sparks[g]; onEdit(); } }));
  }
  if (readOnly) return sp;
  const add = document.createElement('button'); add.type = 'button'; add.className = 'vskadd'; add.textContent = '+';
  add.setAttribute('aria-label', 'Add a target skill spark'); add.title = 'Add a target skill spark'; add.setAttribute('aria-haspopup', 'dialog'); add.setAttribute('aria-expanded', 'false');
  add.onclick = () => openAdd(add, nd, onEdit); add.dataset.k = `${key}:add`;
  sp.appendChild(add);
  return sp;
}
/**
 * Her blue / pink / unique spark: one each (the game gives every veteran one blue and one pink; a unique spark is hers or
 * none).
 * @typedef {object} SparkKind
 * @property {'blues'|'pinks'|'greens'} field  the node's field: blues {stat group: stars}, pinks {aptitude group:
 *   stars}, greens {outfit card id: stars}
 * @property {string} title  the picker's title
 * @property {(group: number|string) => string|null} name  a group's name
 * @property {(nd: LineageNode) => number[]} groups  the groups she can have one of
 */
/** @type {{blue: SparkKind, pink: SparkKind, green: SparkKind}} */
const KIND = { blue: { field: 'blues', title: 'Blue spark', name: blueName, groups: () => Object.keys(CAREER.blues || { 1: 1, 2: 1, 3: 1, 4: 1, 5: 1 }).map(Number) },
  pink: { field: 'pinks', title: 'Pink spark', name: pinkName, groups: (nd) => pinkChoices(nd) },
  green: { field: 'greens', title: 'Unique spark', name: uniqueName, groups: (nd) => nd.outfit && uniqueName(nd.outfit) ? [nd.outfit] : [] } };
// the aptitudes a pink can come from: those she ends at A or S (a pink is drawn from them only): her end-of-career
// grades when known (the veteran analysis), else her uma's base grades; the pink she has always stays a choice
const APT_OF = { 31: 0, 32: 1, 33: 2, 34: 3, 11: 4, 12: 5, 21: 6, 22: 7, 23: 8, 24: 9 };
function pinkChoices(nd) {
  const grades = nd.apt || UMAS.find(u => u.id === nd.chara)?.apt;
  const all = Object.keys(CAREER.pinks).map(Number), have = Object.keys(nd.pinks || {}).map(Number);
  return grades ? all.filter(g => grades[APT_OF[g]] >= 7 || have.includes(g)) : all;
}
// her spark of one kind, as [group (an object key: a string), stars], or undefined (none)
const currentSpark = (nd, kind) => Object.entries(nd[KIND[kind].field] || {}).find(([, stars]) => stars > 0);
// her spark of one kind as its coloured bar: a chip for a unique or a pink she has, else a bar that opens the picker
function colourBar(nd, kind, key, onEdit, readOnly) {
  const spec = KIND[kind], cur = currentSpark(nd, kind);
  if (!cur && (readOnly || !spec.groups(nd).length)) return null;
  const k = `${key}:${kind}`;
  if (kind === 'green') {
    if (cur) {
      const [group, stars] = cur;
      const c = chip({ label: uniqueName(group) || 'Unique spark', icon: null, stars, pink: false, key: k, readOnly,
        onCycle: () => { nd.greens[group] = stars % 3 + 1; onEdit(); }, onRemove: () => { delete nd.greens[group]; onEdit(); } });
      c.className = 'vsb green'; const b = c.querySelector('.vskb'); b.setAttribute('aria-label', b.getAttribute('aria-label').replace(/^([^,]+),/, '$1 (unique),')); return c;
    }
    const c = document.createElement('span'); c.className = 'vsb green none';
    const b = document.createElement('button'); b.type = 'button'; b.className = 'vskb'; b.dataset.k = k; b.setAttribute('aria-label', 'Unique spark: none. Add it at 2 stars');
    b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: '+ Unique spark' }));
    b.onclick = () => { nd.greens = { [nd.outfit]: NEW_STARS }; onEdit(); }; c.appendChild(b); return c;
  }
  if (cur && kind === 'pink') {   // (pinks keep the chip's look and label: "Mile (aptitude), 2 stars")
    const [group, stars] = cur;
    const c = chip({ label: pinkName(group), icon: null, stars, pink: true, key: k, readOnly,
      onCycle: () => openColour(nd, kind, k, onEdit), onRemove: () => { delete nd.pinks[group]; onEdit(); } });
    const b = c.querySelector('.vskb'); if (!readOnly) { b.setAttribute('aria-haspopup', 'dialog'); b.setAttribute('aria-label', `${pinkName(group)} (aptitude), ${stars} star${stars > 1 ? 's' : ''}. Change`); }
    c.querySelector('.vskx')?.remove(); return c;
  }
  // a blue (she has one or not), or no pink yet
  const c = document.createElement('span'); c.className = `vsb ${kind}${cur ? '' : ' none'}`;
  const b = document.createElement(readOnly ? 'span' : 'button'); if (!readOnly) { /** @type {HTMLButtonElement} */ (b).type = 'button'; b.dataset.k = k; b.setAttribute('aria-haspopup', 'dialog'); b.onclick = () => openColour(nd, kind, k, onEdit); }
  b.className = 'vskb';
  const label = cur ? spec.name(cur[0]) || spec.title : `${spec.title}: none`;
  b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: label }));
  if (cur) b.appendChild(starsEl(cur[1]));
  b.setAttribute('aria-label', cur ? `${label} (${kind === 'blue' ? 'stat' : 'unique'}), ${cur[1]} star${cur[1] > 1 ? 's' : ''}${readOnly ? '' : '. Change'}` : `${spec.title}: none. Set it`);
  c.appendChild(b); return c;
}
// the picker (game style): the choices as bars of the spark's colour, its stars (click one), Done (no remove: every
// veteran has one blue and one pink)
let colDlg = null;
function openColour(nd, kind, key, onEdit) {
  const spec = KIND[kind];
  if (!colDlg) { colDlg = document.createElement('dialog'); colDlg.className = 'gmdlg coldlg'; colDlg.id = 'col-dlg'; colDlg.setAttribute('aria-labelledby', 'col-title');
    colDlg.addEventListener('close', () => { const e = colDlg._key && document.querySelector(`[data-k="${CSS.escape(colDlg._key)}"]`); if (e) e.focus(); });
    document.body.appendChild(colDlg); }
  colDlg._key = key;
  const draw = () => {
    colDlg.textContent = '';
    // her spark now: its group (null: none yet, so no stars to pick) and stars (0: none)
    const cur = currentSpark(nd, kind), curGroup = cur ? +cur[0] : null, curStars = cur ? cur[1] : 0;
    const h = document.createElement('h3'); h.className = 'gmdhdr'; h.id = 'col-title'; h.textContent = spec.title; colDlg.appendChild(h);
    const body = document.createElement('div'); body.className = 'gmdbody'; const wrap = document.createElement('div'); wrap.className = 'ud udbare'; body.appendChild(wrap);
    const grid = document.createElement('div'); grid.className = 'colgrid vsparks';
    for (const g of spec.groups(nd)) {
      const c = document.createElement('span'); c.className = kind === 'pink' ? 'vsk pink' : `vsb ${kind}`;
      const b = document.createElement('button'); b.type = 'button'; b.className = 'vskb'; b.setAttribute('aria-pressed', String(g === curGroup));
      b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: spec.name(g) }));
      b.onclick = () => { nd[spec.field] = { [g]: curStars || NEW_STARS }; onEdit(); draw(); }; c.appendChild(b); grid.appendChild(c);
    }
    wrap.appendChild(grid);
    const row = document.createElement('div'); row.className = 'spstars'; row.setAttribute('role', 'group'); row.setAttribute('aria-label', 'Stars');
    for (let k = 1; k <= 3; k++) { const st = document.createElement('button'); st.type = 'button'; st.className = `spstar${k <= curStars ? ' on' : ''}`;
      st.setAttribute('aria-label', `${k} star${k > 1 ? 's' : ''}`); st.setAttribute('aria-pressed', String(k === curStars)); st.disabled = curGroup === null;
      st.onclick = () => { nd[spec.field] = { [curGroup]: k }; onEdit(); draw(); }; row.appendChild(st); }
    body.appendChild(row);
    if (kind === 'blue') body.appendChild(Object.assign(document.createElement('p'), { className: 'gmdstat', textContent: 'A 3★ blue needs that stat at 600 or more (rank B) by the end of the career.' }));
    if (kind === 'pink') body.appendChild(Object.assign(document.createElement('p'), { className: 'gmdstat', textContent: nd.apt ? 'The aptitudes she ends this career at A or S.' : 'Her uma\'s aptitudes at A or S (a pink needs A or better).' }));
    colDlg.appendChild(body);
    const f = document.createElement('div'); f.className = 'gmdfoot';
    const done = document.createElement('button'); done.type = 'button'; done.className = 'btn gmclose'; done.textContent = 'Done'; done.onclick = () => colDlg.close();
    f.appendChild(done); colDlg.appendChild(f);
  };
  draw(); colDlg.showModal(); colDlg.querySelector('.gmclose:last-child')?.focus();
}
/** @param {LineageNode} nd @param {string} who her name in the editor's title @param {string} [key] @returns {HTMLButtonElement} */
export function winsBtn(nd, who, key = '') {
  const w = document.createElement('button'); w.type = 'button'; w.className = 'btn vwins'; if (key) w.dataset.k = `${key}:wins`;
  w.textContent = winsText(nd);
  w.title = 'G1 wins (race affinity): click to edit'; w.onclick = () => openWins(nd, who);
  w.setAttribute('aria-haspopup', 'dialog');
  return w;
}
// change who a node is: pick the uma, then whether to add the target skills
/** @param {LineageNode} nd @param {string} label */
export function changeUma(nd, label) {
  openUmaPicker(`${label}: which uma?`, (id) => {
    nd.chara = id; delete nd.outfit; rev++;
    askSkills(umaName(id), Object.keys(nd.sparks).length > 0, (sp) => { if (sp) nd.sparks = sp; changed(); });
  });
}
// a new veteran by hand for a parent place: she gets `sparks`, and so do her parents (a looped parent's own parents carry
// the same sparks), whose umas are not known yet
/** @param {number|null} chara @param {Sparks} sparks @returns {Veteran} */
export function newVet(chara, sparks) {
  const v = { id: `h${Date.now().toString(36)}${Math.random().toString(36).slice(2, 5)}`, src: 'hand', chara, borrowed: false, sparks, pinks: {}, blues: {}, greens: {}, wins: [],
    parents: [0, 1].map(() => ({ chara: null, sparks: { ...sparks }, pinks: {}, blues: {}, greens: {}, wins: [] })) };
  vets.push(v); return v;
}
// an import never loses one of your veterans: yours merge by id (a veteran already here takes the file's version in
// her place, new ones go after your other imports), while borrowed ones replace the earlier borrow list (it is who
// lends today), after all of yours so your numbers keep. `all` (an Export of this page, holding both kinds): every
// earlier import goes. Hand-made ones stay (a plan slot holding a removed one goes back to the planner).
// Returns { added, updated } for yours and { borrowed, dropped } for the borrow list.
/** @param {Veteran[]} list @param {boolean} [all] @returns {{added: number, updated: number, borrowed: number, dropped: number}} */
export function importVets(list, all = false) {
  const isImport = (v) => v.src === 'import';
  if (all) { for (let i = vets.length - 1; i >= 0; i--) if (isImport(vets[i])) vets.splice(i, 1); }
  const own = list.filter(v => !v.borrowed), borrowed = list.filter(v => v.borrowed), fresh = [];
  let updated = 0, dropped = 0;
  for (const v of own) { const old = vetById(v.id); if (old) { Object.assign(old, v); updated++; } else fresh.push(v); }
  // new ones of yours go right after your last import
  const afterOwnImports = vets.reduce((at, v, i) => (isImport(v) && !v.borrowed ? i + 1 : at), 0);
  vets.splice(afterOwnImports, 0, ...fresh);
  if (borrowed.length) {
    for (let i = vets.length - 1; i >= 0; i--) if (isImport(vets[i]) && vets[i].borrowed) { vets.splice(i, 1); dropped++; }
    vets.push(...borrowed);
  }
  dropMissing(); changed();
  return { added: fresh.length, updated, borrowed: borrowed.length, dropped };
}
// sample veterans to try the planner with (no veterans yet): 18 mob umas drawn at random from the game's (those with a
// portrait), six veterans with two mob parents each, the bare minimum to start a loop: each veteran carries, at 1 star,
// only the targets nothing but inheritance gives (no card hints or events them, no outfit has them, the scenario
// doesn't hand them out); her parents carry nothing. No relation points with anyone (the solver knows no mob), no G1
// wins. Marked as samples; Remove all clears them.
/** @returns {Veteran[]} */
export function sampleVets() {
  const all = CAREER.mobs || [], withArt = all.filter(([id]) => srcFor('mob', id)), pool = (withArt.length >= 18 ? withArt : all).slice();
  for (let i = pool.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [pool[i], pool[j]] = [pool[j], pool[i]]; }
  const sourced = new Set(CAREER.sourced || []), inherit = targetGroups().filter(g => !sourced.has(g) && !goldGiven(g));
  const node = (id, sparks) => ({ chara: id, sparks, pinks: {}, blues: {}, greens: {}, wins: [] });
  const out = [];
  for (let i = 0; i + 2 < pool.length && out.length < 6; i += 3)
    out.push({ id: `m${pool[i][0]}${Date.now().toString(36)}`, src: 'sample', borrowed: false, ...node(pool[i][0], Object.fromEntries(inherit.map(g => [g, 1]))),
      parents: [node(pool[i + 1][0], {}), node(pool[i + 2][0], {})] });
  return out;
}
// veterans from an uma.moe tree: added after the others (the numbers you have keep)
/** @param {Veteran[]} list */
export function addVets(list) { vets.push(...list); changed(); }
/** @param {number} i her index in `vets` */
export function removeVet(i) { vets.splice(i, 1); dropMissing(); changed(); }
// every veteran, or those `drop` picks
/** @param {(vet: Veteran) => boolean} [drop] */
export function clearVets(drop = () => true) { for (let i = vets.length - 1; i >= 0; i--) if (drop(vets[i])) vets.splice(i, 1); dropMissing(); changed(); }
function dropMissing() { plan.parents = plan.parents.map(id => (id && vetById(id) ? id : null)); }
// the job's legacies (skill groups and stars as pairs; an unknown parent counts with character 0: no relation points)
// (the same array until the next edit, `rev`: callers don't change it)
let legaciesCache = null, legaciesRev = -1;
/**
 * A veteran as the solver's job takes her (sparks as [group, stars] pairs; an unknown parent's character 0).
 * @typedef {{chara: number, sparks: [number, number][], pinks: [number, number][], wins: number[]}} JobNode
 * @typedef {JobNode & {borrowed: boolean, parents: JobNode[]}} JobLegacy
 * @returns {JobLegacy[]}
 */
export function legaciesJob() {
  if (legaciesCache && legaciesRev === rev) return legaciesCache;
  const pairs = (sparks) => Object.entries(sparks || {}).map(([group, stars]) => [+group, stars]);
  const jobNode = (n) => ({ chara: n.chara || 0, sparks: pairs(n.sparks), pinks: pairs(n.pinks), wins: n.wins });
  legaciesRev = rev;
  legaciesCache = vets.map(v => ({ ...jobNode(v), borrowed: !!v.borrowed, parents: v.parents.map(jobNode) }));
  return legaciesCache;
}
