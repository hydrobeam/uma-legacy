// ---------- the career page's veterans and lineage plan: storage, the spark editor, the uma and skills prompts ----------
// A veteran: {id, src: 'import' | 'hand', chara, outfit?, borrowed, sparks: {skill group: stars 1-3}, pinks: {aptitude
// group: stars}, wins: [race index], parents: [node, node]}; a parent node: {chara (null: not known), outfit?, sparks,
// pinks, wins}. The plan: {trainee: chara id | null (the planner picks), parents: [veteran id | null, ...]}.
// Saved per data era in localStorage['llp:career'] = {v: 2, global: {vets, plan}, gm: {vets, plan}}.
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { UMAS, NAME, SK, CAREER } from './data.js';
import { state, goldGiven } from './state.js';
import { umaAv, skillIcon, srcFor } from './images.js';
import { winsAgenda } from './agenda.js';

const CKEY = 'llp:career', CV = 2;
// stars of a skill added by hand: 2, the most common star count of a white spark at SS rank (70% at rank score 17,500+,
// 45% below it; hakuraku.moe)
export const NEW_STARS = 2;
function loadAll() {
  try { const o = JSON.parse(localStorage.getItem(CKEY));
    if (o && o.v === CV) return o;
    if (o && o.v === 1) { const n = { v: CV }; for (const e of ['global', 'gm']) if (Array.isArray(o[e])) n[e] = { vets: o[e] }; return n; }   // v1: a list per era
  } catch {}
  return { v: CV };
}
const ALL = loadAll(), MINE = ALL[ERA] || {};
export const vets = (MINE.vets || []).map(v => ({ pinks: {}, ...v, parents: v.parents.map(p => ({ pinks: {}, ...p })) }));
export const plan = MINE.plan || { trainee: null, parents: [null, null] };
export function persist() { ALL[ERA] = { vets, plan }; try { localStorage.setItem(CKEY, JSON.stringify(ALL)); } catch {} }

let onChange = () => {};
export function onVetsChange(fn) { onChange = fn; }
// every edit: saved, then the page redraws and searches again
export function changed() { persist(); onChange(); }

export const umaName = (id) => id ? (NAME.get(id) || 'Uma ' + id) : 'Unknown';
const targetGroups = () => state.targets.map(t => t.group);
export const skillName = (g) => (SK.get(g) || { w: 'Skill ' + g }).w;
export const pinkName = (g) => CAREER.pinks[g] || 'Aptitude ' + g;
export const blueName = (g) => (CAREER.blues || {})[g] || ['', 'Speed', 'Stamina', 'Power', 'Guts', 'Wit'][g] || 'Stat ' + g;
export const uniqueName = (g) => (CAREER.uniques || {})[g] || null;
const starText = (n) => '★'.repeat(n) + '☆'.repeat(3 - n);
export const vetById = (id) => vets.find(v => v.id === id);
const full = (stars) => Object.fromEntries(targetGroups().map(g => [g, stars]));

// ---------- dialogs ----------
for (const d of document.querySelectorAll('dialog')) {
  d.addEventListener('click', (e) => { if (e.target === d) d.close(); });   // backdrop
  d.querySelectorAll('.dclose').forEach(b => b.onclick = () => d.close());
}
// the uma picker: every uma (or those `only` lets through), filterable; onPick(chara id)
let pickCb = null, pickOnly = null;
export function openUmaPicker(title, onPick, only = null) {
  pickCb = onPick; pickOnly = only; $('pick-title').textContent = title; $('pick-filter').value = '';
  drawUmas($('pick-list'), '', pickUma, only); $('pick-dlg').showModal(); $('pick-filter').focus();
}
function pickUma(id) { $('pick-dlg').close(); const cb = pickCb; pickCb = null; cb && cb(id); }
$('pick-filter').oninput = () => filterUmas($('pick-list'), $('pick-filter').value);
// the picker's list: every uma `only` lets through, the filter applied by hiding (typing doesn't rebuild it)
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
export function filterUmas(box, filter) {
  const f = filter.trim().toLowerCase(); let n = 0;
  for (const b of box.querySelectorAll('.upick')) { b.hidden = !!f && !b._nm.includes(f); n += !b.hidden; }
  box.querySelector('p.hint').hidden = n > 0;
}
// after an uma is picked for a node that carries sparks: whether to give her the target skills. onChoice(sparks | null:
// keep what she has)
export function askSkills(name, hasSparks, onChoice, withParents = false) {
  const n = targetGroups().length;
  $('ask-title').textContent = `Add skills to ${name}?`;
  $('ask-text').textContent = `Her sparks of your ${n} target skills decide what she passes on. A fully looped parent has all of them` +
    (withParents ? ', and so do her own two parents: the same choice goes on them.' : '.') + ' You can change the stars of each afterwards.';
  const box = $('ask-btns'); box.textContent = '';
  // No skills first and the default (a new uma carries nothing until you say so)
  const opts = [['No skills', () => ({}), true], [`All ${n} at 2★`, () => full(2)], [`All ${n} at 3★`, () => full(3)], [`All ${n} at 1★`, () => full(1)],
    ...(hasSparks ? [['Keep her current skills', () => null]] : [])];
  for (const [label, val, primary] of opts) {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'btn' + (primary ? ' primary' : ''); b.textContent = label;
    b.onclick = () => { $('ask-dlg').close(); onChoice(val()); }; box.appendChild(b);
  }
  $('ask-dlg').showModal(); box.querySelector('.primary').focus();
}
// the G1 wins of one node, on the schedule window's calendar (the editor in agenda.js): each win in its slot, click it to
// remove it; an empty slot where a G1 is held lists the races held then. A pool veteran's wins are saved on each edit, and
// the page searches again when the dialog closes. opts (the veteran analysis' draft): {offer, info, hint, onEdit, onClose}
// (see winsAgenda). nd.winAt {race index: turn} is where a win shows, for display only (the solver reads nd.wins).
const allRaces = () => CAREER.races.map((name, ix) => ({ ix, name, turns: (CAREER.race_turns || [])[ix] || [] }));
let winsClose = null;
export function openWins(nd, who, opts = {}) {
  const opener = document.activeElement;
  $('wins-title').textContent = 'G1 wins: ' + who;
  const count = () => { $('wins-count').textContent = winsText(nd); };
  $('wins-list').replaceChildren(winsAgenda({ races: allRaces(), get: () => nd.wins, set: (w) => { nd.wins = w; }, at: () => (nd.winAt ||= {}),
    offer: opts.offer, info: opts.info, onEdit: () => { count(); (opts.onEdit || persist)(); } }));
  count();
  // closing: the pool searches again; the focus goes back to the button that opened it (redrawn by then: found by its key)
  winsClose = opts.onClose || (() => { const k = opener?.dataset?.k; changed(); const b = k && document.querySelector(`[data-k="${CSS.escape(k)}"]`); (b || (opener?.isConnected ? opener : null))?.focus(); });
  $('wins-dlg').showModal();
}
export const winsText = (nd) => nd.wins.length ? `${nd.wins.length} G1 win${nd.wins.length > 1 ? 's' : ''}` : 'No G1 wins';
$('wins-dlg').addEventListener('close', () => { const f = winsClose; winsClose = null; f && f(); });
// "+" beside a node's sparks: the target skills she lacks, as the game's white spark cells (her blue, pink and unique
// sparks have bars of their own, each opening a picker)
const pop = $('add-pop'); let popFor = null;
function closeAdd() { pop.hidden = true; if (popFor) popFor.setAttribute('aria-expanded', 'false'); popFor = null; }
function openAdd(btn, nd, onEdit = changed) {
  if (popFor === btn) { closeAdd(); return; }
  closeAdd(); popFor = btn; btn.setAttribute('aria-expanded', 'true'); pop.textContent = '';
  const head = (t) => { const h = document.createElement('p'); h.className = 'hint'; h.textContent = t; pop.appendChild(h); };
  const miss = targetGroups().filter(g => !nd.sparks[g]);
  head(miss.length ? 'Target skills she lacks' : 'She has every target skill.');
  const list = document.createElement('div'); list.className = 'addlist';
  for (const g of miss) {
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
  if (host) { const h = host.getBoundingClientRect(); pop.style.left = Math.max(4, Math.min(r.left - h.left, h.width - 300)) + 'px'; pop.style.top = (r.bottom - h.top + host.scrollTop + 6) + 'px'; }
  else { pop.style.left = Math.max(8, Math.min(r.left + scrollX, scrollX + innerWidth - 320)) + 'px'; pop.style.top = (r.bottom + scrollY + 6) + 'px'; }
  pop.hidden = false; pop.focus();
}
document.addEventListener('click', (e) => { if (!pop.hidden && !pop.contains(e.target) && !(popFor && popFor.contains(e.target))) closeAdd(); });
document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && !pop.hidden) { const b = popFor; closeAdd(); b && b.focus(); } });

// ---------- a node's sparks ----------
// target skill chips (click: stars 1 -> 2 -> 3 -> 1; the corner x removes), then aptitude chips (pink), then "+".
// Sparks of skills that are not targets are kept but not shown. key: names the controls (data-k), so the page can put the
// keyboard focus back on the same one after a redraw.
function chip(label, icon, n, pink, cycle, remove, key, readOnly = false) {
  const c = document.createElement('span'); c.className = 'vsk' + (pink ? ' pink' : '');
  const b = document.createElement(readOnly ? 'span' : 'button'); if (!readOnly) b.type = 'button'; b.className = 'vskb';
  if (icon) b.appendChild(icon);
  const t = document.createElement('span'); t.className = 'vskn'; t.textContent = label; b.appendChild(t);
  // stars: filled then empty (two spans: the career page's game-style card colours them apart; the text is starText(n))
  const s = document.createElement('span'); s.className = 'stars'; s.setAttribute('aria-hidden', 'true');
  const sf = document.createElement('span'); sf.className = 'sf'; sf.textContent = '★'.repeat(n); sf.style.setProperty('--n', String(n));
  const se = document.createElement('span'); se.className = 'se'; se.textContent = '☆'.repeat(3 - n); se.style.setProperty('--n', String(3 - n)); s.append(sf, se); b.appendChild(s);
  if (readOnly) { b.setAttribute('aria-label', `${label}${pink ? ' (aptitude)' : ''}, ${n} stars`); c.appendChild(b); return c; }
  b.setAttribute('aria-label', `${label}${pink ? ' (aptitude)' : ''}, ${n} star${n > 1 ? 's' : ''}. Click for ${n % 3 + 1}`);
  b.onclick = cycle; b.dataset.k = key;
  const x = document.createElement('button'); x.type = 'button'; x.className = 'vskx'; x.textContent = '×';
  x.setAttribute('aria-label', 'Remove ' + label); x.title = 'Remove ' + label; x.onclick = remove; x.dataset.k = key + 'x';
  c.append(b, x); return c;
}
// A veteran's sparks as the game's Inspiration cells: her blue (stat), pink (aptitude) and unique (green) sparks first,
// each a bar of its colour (a dashed one when she has none) that opens its picker; then her target skills' white sparks
// (click: stars 1 -> 2 -> 3 -> 1, the corner x removes), then "+" (a target skill she lacks). Sparks of skills that are
// not targets are kept but not shown (read-only: shown). key names the controls (data-k) for the focus after a redraw.
export function sparksEl(nd, key = '', onEdit = changed, readOnly = false) {
  const sp = document.createElement('div'); sp.className = 'vsparks';
  for (const kind of ['blue', 'pink', 'green']) { const b = colourBar(nd, kind, key, onEdit, readOnly); if (b) sp.appendChild(b); }
  for (const g of readOnly ? Object.keys(nd.sparks).map(Number) : targetGroups()) { const n = nd.sparks[g]; if (!n) continue;
    sp.appendChild(chip(skillName(g), skillIcon(g, false), n, false, () => { nd.sparks[g] = n % 3 + 1; onEdit(); }, () => { delete nd.sparks[g]; onEdit(); }, `${key}:s${g}`, readOnly)); }
  if (readOnly) return sp;
  const add = document.createElement('button'); add.type = 'button'; add.className = 'vskadd'; add.textContent = '+';
  add.setAttribute('aria-label', 'Add a target skill spark'); add.title = 'Add a target skill spark'; add.setAttribute('aria-haspopup', 'dialog'); add.setAttribute('aria-expanded', 'false');
  add.onclick = () => openAdd(add, nd, onEdit); add.dataset.k = key + ':add';
  sp.appendChild(add);
  return sp;
}
// her blue / pink / unique spark: one each (the game gives every veteran one blue and one pink; a unique spark is hers or
// none). The field: blues {stat group: stars}, pinks {aptitude group: stars}, greens {outfit card id: stars}
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
function colourBar(nd, kind, key, onEdit, readOnly) {
  const K = KIND[kind], cur = Object.entries(nd[K.field] || {}).find(([, n]) => n > 0);
  if (!cur && (readOnly || !K.groups(nd).length)) return null;
  const k = `${key}:${kind}`;
  if (kind === 'green') {
    if (cur) { const g = cur[0], c = chip(uniqueName(g) || 'Unique spark', null, cur[1], false, () => { nd.greens[g] = cur[1] % 3 + 1; onEdit(); }, () => { delete nd.greens[g]; onEdit(); }, k, readOnly);
      c.className = 'vsb green'; const b = c.querySelector('.vskb'); b.setAttribute('aria-label', b.getAttribute('aria-label').replace(/^([^,]+),/, '$1 (unique),')); return c; }
    const c = document.createElement('span'); c.className = 'vsb green none';
    const b = document.createElement('button'); b.type = 'button'; b.className = 'vskb'; b.dataset.k = k; b.setAttribute('aria-label', 'Unique spark: none. Add it at 2 stars');
    b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: '+ Unique spark' }));
    b.onclick = () => { nd.greens = { [nd.outfit]: NEW_STARS }; onEdit(); }; c.appendChild(b); return c;
  }
  if (cur && kind === 'pink') {   // (pinks keep the chip's look and label: "Mile (aptitude), 2 stars")
    const c = chip(pinkName(cur[0]), null, cur[1], true, () => openColour(nd, kind, k, onEdit), () => { delete nd.pinks[cur[0]]; onEdit(); }, k, readOnly);
    const b = c.querySelector('.vskb'); if (!readOnly) { b.setAttribute('aria-haspopup', 'dialog'); b.setAttribute('aria-label', `${pinkName(cur[0])} (aptitude), ${cur[1]} star${cur[1] > 1 ? 's' : ''}. Change`); }
    c.querySelector('.vskx')?.remove(); return c;
  }
  const c = document.createElement('span'); c.className = `vsb ${kind}` + (cur ? '' : ' none');
  const b = document.createElement(readOnly ? 'span' : 'button'); if (!readOnly) { b.type = 'button'; b.dataset.k = k; b.setAttribute('aria-haspopup', 'dialog'); b.onclick = () => openColour(nd, kind, k, onEdit); }
  b.className = 'vskb';
  const label = cur ? K.name(cur[0]) || K.title : `${K.title}: none`;
  b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: label }));
  if (cur) { const n = cur[1], st = document.createElement('span'); st.className = 'stars'; st.setAttribute('aria-hidden', 'true');
    const sf = document.createElement('span'); sf.className = 'sf'; sf.textContent = '★'.repeat(n); sf.style.setProperty('--n', String(n));
    const se = document.createElement('span'); se.className = 'se'; se.textContent = '☆'.repeat(3 - n); se.style.setProperty('--n', String(3 - n)); st.append(sf, se); b.appendChild(st); }
  b.setAttribute('aria-label', cur ? `${label} (${kind === 'blue' ? 'stat' : 'unique'}), ${cur[1]} star${cur[1] > 1 ? 's' : ''}${readOnly ? '' : '. Change'}` : `${K.title}: none. Set it`);
  c.appendChild(b); return c;
}
// the picker (game style): the choices as bars of the spark's colour, its stars (click one), Done (no remove: every
// veteran has one blue and one pink)
let colDlg = null;
function openColour(nd, kind, key, onEdit) {
  const K = KIND[kind];
  if (!colDlg) { colDlg = document.createElement('dialog'); colDlg.className = 'gmdlg coldlg'; colDlg.id = 'col-dlg'; colDlg.setAttribute('aria-labelledby', 'col-title');
    colDlg.addEventListener('close', () => { const e = colDlg._key && document.querySelector(`[data-k="${CSS.escape(colDlg._key)}"]`); if (e) e.focus(); });
    document.body.appendChild(colDlg); }
  colDlg._key = key;
  const draw = () => {
    colDlg.textContent = ''; const cur = Object.entries(nd[K.field] || {}).find(([, n]) => n > 0), g0 = cur ? +cur[0] : null, n0 = cur ? cur[1] : 0;
    const h = document.createElement('h3'); h.className = 'gmdhdr'; h.id = 'col-title'; h.textContent = K.title; colDlg.appendChild(h);
    const body = document.createElement('div'); body.className = 'gmdbody'; const wrap = document.createElement('div'); wrap.className = 'ud udbare'; body.appendChild(wrap);
    const grid = document.createElement('div'); grid.className = 'colgrid vsparks';
    for (const g of K.groups(nd)) {
      const c = document.createElement('span'); c.className = kind === 'pink' ? 'vsk pink' : `vsb ${kind}`;
      const b = document.createElement('button'); b.type = 'button'; b.className = 'vskb'; b.setAttribute('aria-pressed', String(g === g0));
      b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: K.name(g) }));
      b.onclick = () => { nd[K.field] = { [g]: n0 || NEW_STARS }; onEdit(); draw(); }; c.appendChild(b); grid.appendChild(c);
    }
    wrap.appendChild(grid);
    const row = document.createElement('div'); row.className = 'spstars'; row.setAttribute('role', 'group'); row.setAttribute('aria-label', 'Stars');
    for (let k = 1; k <= 3; k++) { const st = document.createElement('button'); st.type = 'button'; st.className = 'spstar' + (k <= n0 ? ' on' : '');
      st.setAttribute('aria-label', `${k} star${k > 1 ? 's' : ''}`); st.setAttribute('aria-pressed', String(k === n0)); st.disabled = g0 === null;
      st.onclick = () => { nd[K.field] = { [g0]: k }; onEdit(); draw(); }; row.appendChild(st); }
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
export function winsBtn(nd, who, key = '') {
  const w = document.createElement('button'); w.type = 'button'; w.className = 'btn vwins'; if (key) w.dataset.k = key + ':wins';
  w.textContent = winsText(nd);
  w.title = 'G1 wins (race affinity): click to edit'; w.onclick = () => openWins(nd, who);
  w.setAttribute('aria-haspopup', 'dialog');
  return w;
}
// change who a node is: pick the uma, then whether to add the target skills
export function changeUma(nd, label) {
  openUmaPicker(`${label}: which uma?`, (id) => {
    nd.chara = id; delete nd.outfit;
    askSkills(umaName(id), Object.keys(nd.sparks).length > 0, (sp) => { if (sp) nd.sparks = sp; changed(); });
  });
}
// a new veteran by hand for a parent place: she gets `sparks`, and so do her parents (a looped parent's own parents carry
// the same sparks), whose umas are not known yet
export function newVet(chara, sparks) {
  const v = { id: 'h' + Date.now().toString(36) + Math.random().toString(36).slice(2, 5), src: 'hand', chara, borrowed: false, sparks, pinks: {}, blues: {}, greens: {}, wins: [],
    parents: [0, 1].map(() => ({ chara: null, sparks: { ...sparks }, pinks: {}, blues: {}, greens: {}, wins: [] })) };
  vets.push(v); return v;
}
// imported veterans replace earlier imports; hand-made ones stay (a plan slot holding a replaced one goes back to the planner)
export function importVets(list) {
  for (let i = vets.length - 1; i >= 0; i--) if (vets[i].src === 'import') vets.splice(i, 1);
  vets.unshift(...list); dropMissing(); changed();
}
// sample veterans to try the planner with (no veterans yet): 18 mob umas drawn at random from the game's (those with a
// portrait), six veterans with two mob parents each, the bare minimum to start a loop: each veteran carries, at 1 star,
// only the targets nothing but inheritance gives (no card hints or events them, no outfit has them, the scenario
// doesn't hand them out); her parents carry nothing. No relation points with anyone (the solver knows no mob), no G1
// wins. Marked as samples; Remove all clears them.
export function sampleVets() {
  const all = CAREER.mobs || [], withArt = all.filter(([id]) => srcFor('mob', id)), pool = (withArt.length >= 18 ? withArt : all).slice();
  for (let i = pool.length - 1; i > 0; i--) { const j = Math.floor(Math.random() * (i + 1)); [pool[i], pool[j]] = [pool[j], pool[i]]; }
  const src = new Set(CAREER.sourced || []), inherit = targetGroups().filter(g => !src.has(g) && !goldGiven(g));
  const node = (id, sparks) => ({ chara: id, sparks, pinks: {}, blues: {}, greens: {}, wins: [] });
  const out = [];
  for (let i = 0; i + 2 < pool.length && out.length < 6; i += 3)
    out.push({ id: 'm' + pool[i][0] + Date.now().toString(36), src: 'sample', borrowed: false, ...node(pool[i][0], Object.fromEntries(inherit.map(g => [g, 1]))),
      parents: [node(pool[i + 1][0], {}), node(pool[i + 2][0], {})] });
  return out;
}
// veterans from an uma.moe tree: added after the others (the numbers you have keep)
export function addVets(list) { vets.push(...list); changed(); }
export function removeVet(i) { vets.splice(i, 1); dropMissing(); changed(); }
export function clearVets() { vets.length = 0; dropMissing(); changed(); }
function dropMissing() { plan.parents = plan.parents.map(id => (id && vetById(id) ? id : null)); }
// the job's legacies (skill groups and stars as pairs; an unknown parent counts with character 0: no relation points)
export function legaciesJob() {
  const pairs = (o) => Object.entries(o || {}).map(([g, s]) => [+g, s]);
  const nd = (n) => ({ chara: n.chara || 0, sparks: pairs(n.sparks), pinks: pairs(n.pinks), wins: n.wins });
  return vets.map(v => ({ ...nd(v), borrowed: !!v.borrowed, parents: v.parents.map(nd) }));
}
