// ---------- the career page's veterans and lineage plan: storage, the spark editor, the uma and skills prompts ----------
// A veteran: {id, src: 'import' | 'hand', chara, outfit?, borrowed, sparks: {skill group: stars 1-3}, pinks: {aptitude
// group: stars}, wins: [race index], parents: [node, node]}; a parent node: {chara (null: not known), outfit?, sparks,
// pinks, wins}. The plan: {trainee: chara id | null (the planner picks), parents: [veteran id | null, ...]}.
// Saved per data era in localStorage['llp:career'] = {v: 2, global: {vets, plan}, gm: {vets, plan}}.
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { UMAS, NAME, SK, CAREER } from './data.js';
import { state } from './state.js';
import { umaAv, skillIcon } from './images.js';
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
$('pick-filter').oninput = () => drawUmas($('pick-list'), $('pick-filter').value, pickUma, pickOnly);
export function drawUmas(box, filter, onPick, only = null) {
  box.textContent = ''; const f = filter.trim().toLowerCase();
  for (const u of [...UMAS].sort((a, b) => a.name.localeCompare(b.name))) {
    if ((f && !u.name.toLowerCase().includes(f)) || (only && !only(u))) continue;
    const b = document.createElement('button'); b.type = 'button'; b.className = 'upick'; b.append(umaAv(u.id, null, 40));
    const s = document.createElement('span'); s.textContent = u.name; b.append(s);
    b.onclick = () => onPick(u.id); box.appendChild(b);
  }
  if (!box.children.length) box.innerHTML = '<p class="hint">No uma matches.</p>';
}
// after an uma is picked for a node that carries sparks: whether to give her the target skills. onChoice(sparks | null:
// keep what she has)
export function askSkills(name, hasSparks, onChoice, withParents = false) {
  const n = targetGroups().length;
  $('ask-title').textContent = `Add skills to ${name}?`;
  $('ask-text').textContent = `Her sparks of your ${n} target skills decide what she passes on. A fully looped parent has all of them` +
    (withParents ? ', and so do her own two parents: the same choice goes on them.' : '.') + ' You can change the stars of each afterwards.';
  const box = $('ask-btns'); box.textContent = '';
  const opts = [[`All ${n} at 2★`, () => full(2), true], [`All ${n} at 3★`, () => full(3)], [`All ${n} at 1★`, () => full(1)],
    ...(hasSparks ? [['Keep her current skills', () => null]] : []), ['No skills', () => ({})]];
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
  $('wins-hint').textContent = opts.hint || 'Single-race G1s she won. Click a win to remove it; click an empty slot (+) for the G1s held then. Each one she shares with a parent or the other parent adds 3 affinity.';
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
// "+" beside a node's sparks: the target skills she lacks, then the aptitudes she has no spark of
const pop = $('add-pop'); let popFor = null;
function closeAdd() { pop.hidden = true; if (popFor) popFor.setAttribute('aria-expanded', 'false'); popFor = null; }
function openAdd(btn, nd, onEdit = changed) {
  if (popFor === btn) { closeAdd(); return; }
  closeAdd(); popFor = btn; btn.setAttribute('aria-expanded', 'true'); pop.textContent = '';
  const head = (t) => { const h = document.createElement('p'); h.className = 'hint'; h.textContent = t; pop.appendChild(h); };
  const miss = targetGroups().filter(g => !nd.sparks[g]);
  head(miss.length ? 'Target skills she lacks' : 'She has every target skill.');
  for (const g of miss) {
    const b = document.createElement('button'); b.type = 'button'; b.className = 'btn addsk';
    const ic = skillIcon(g, false); if (ic) b.appendChild(ic); b.append(skillName(g));
    b.onclick = () => { nd.sparks[g] = NEW_STARS; closeAdd(); onEdit(); };
    pop.appendChild(b);
  }
  const pm = Object.keys(CAREER.pinks).map(Number).filter(g => !nd.pinks[g]);
  if (pm.length) { head('Aptitude (pink) sparks');
    const row = document.createElement('div'); row.className = 'pinkrow';
    for (const g of pm) { const b = document.createElement('button'); b.type = 'button'; b.className = 'btn addpk'; b.textContent = pinkName(g);
      b.onclick = () => { nd.pinks[g] = NEW_STARS; closeAdd(); onEdit(); }; row.appendChild(b); }
    pop.appendChild(row); }
  const r = btn.getBoundingClientRect();
  pop.style.left = Math.max(8, Math.min(r.left + scrollX, scrollX + innerWidth - 320)) + 'px'; pop.style.top = (r.bottom + scrollY + 6) + 'px';
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
  const s = document.createElement('span'); s.className = 'stars'; s.textContent = starText(n); s.setAttribute('aria-hidden', 'true'); b.appendChild(s);
  if (readOnly) { b.setAttribute('aria-label', `${label}${pink ? ' (aptitude)' : ''}, ${n} stars`); c.appendChild(b); return c; }
  b.setAttribute('aria-label', `${label}${pink ? ' (aptitude)' : ''}, ${n} star${n > 1 ? 's' : ''}. Click for ${n % 3 + 1}`);
  b.onclick = cycle; b.dataset.k = key;
  const x = document.createElement('button'); x.type = 'button'; x.className = 'vskx'; x.textContent = '×';
  x.setAttribute('aria-label', 'Remove ' + label); x.title = 'Remove ' + label; x.onclick = remove; x.dataset.k = key + 'x';
  c.append(b, x); return c;
}
export function sparksEl(nd, key = '', onEdit = changed, readOnly = false) {
  const sp = document.createElement('div'); sp.className = 'vsparks';
  for (const g of readOnly ? Object.keys(nd.sparks).map(Number) : targetGroups()) { const n = nd.sparks[g]; if (!n) continue;
    sp.appendChild(chip(skillName(g), skillIcon(g, false), n, false, () => { nd.sparks[g] = n % 3 + 1; onEdit(); }, () => { delete nd.sparks[g]; onEdit(); }, `${key}:s${g}`, readOnly)); }
  for (const [g, n] of Object.entries(nd.pinks || {})) {
    sp.appendChild(chip(pinkName(g), null, n, true, () => { nd.pinks[g] = n % 3 + 1; onEdit(); }, () => { delete nd.pinks[g]; onEdit(); }, `${key}:p${g}`, readOnly)); }
  if (readOnly) return sp;
  const add = document.createElement('button'); add.type = 'button'; add.className = 'vskadd'; add.textContent = '+';
  add.setAttribute('aria-label', 'Add a spark'); add.title = 'Add a spark'; add.setAttribute('aria-haspopup', 'dialog'); add.setAttribute('aria-expanded', 'false');
  add.onclick = () => openAdd(add, nd, onEdit); add.dataset.k = key + ':add';
  sp.appendChild(add);
  return sp;
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
  const v = { id: 'h' + Date.now().toString(36) + Math.random().toString(36).slice(2, 5), src: 'hand', chara, borrowed: false, sparks, pinks: {}, wins: [],
    parents: [0, 1].map(() => ({ chara: null, sparks: { ...sparks }, pinks: {}, wins: [] })) };
  vets.push(v); return v;
}
// imported veterans replace earlier imports; hand-made ones stay (a plan slot holding a replaced one goes back to the planner)
export function importVets(list) {
  for (let i = vets.length - 1; i >= 0; i--) if (vets[i].src === 'import') vets.splice(i, 1);
  vets.unshift(...list); dropMissing(); changed();
}
export function removeVet(i) { vets.splice(i, 1); dropMissing(); changed(); }
export function clearVets() { vets.length = 0; dropMissing(); changed(); }
function dropMissing() { plan.parents = plan.parents.map(id => (id && vetById(id) ? id : null)); }
// the job's legacies (skill groups and stars as pairs; an unknown parent counts with character 0: no relation points)
export function legaciesJob() {
  const pairs = (o) => Object.entries(o || {}).map(([g, s]) => [+g, s]);
  const nd = (n) => ({ chara: n.chara || 0, sparks: pairs(n.sparks), pinks: pairs(n.pinks), wins: n.wins });
  return vets.map(v => ({ ...nd(v), borrowed: !!v.borrowed, parents: v.parents.map(nd) }));
}
