// Career planner page: entry point. build.py bundles this and the modules it imports into dist/career.html.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (vets.js) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
import { $ } from './dom.js';
import { ERA, SAVED } from './settings.js';
import { UMAS, CARDS, CARD_BY_ID, SK, CAREER, loadData } from './data.js';
import { state, goldGiven, writeSaved } from './state.js';
import { run, killEngine } from './engine.js';
import { onSave } from './save.js';
import { EXTRA, jobTargets, ownedCards, rcardsOff } from './job.js';
import { umaAv, skillIcon, cardAv } from './images.js';
import { initScenario, eraScenario, scenarioOf } from './scenario.js';
import { initRoster, renderRoster, noOutfit } from './roster.js';
import { initCards, cardTitle } from './cards.js';
import { initPopover, markOwn } from './popover.js';
import { openSched } from './agenda.js';
import { initSetup } from './setup.js';
import './gmtip.js';   // the game-style tooltips (both planner pages)
import { affEl, affTable as affTableOf } from './afftable.js';
import { initTargets, renderTargets } from './targets.js';
import { vets, plan, onVetsChange, changed, persist, vetById, umaName, skillName, sparksEl, winsBtn, changeUma, askSkills,
  newVet, importVets, addVets, removeVet, clearVets, legaciesJob, openUmaPicker, drawUmas, openWins } from './vets.js';
import { parseExport, parseTree } from './umadump.js';
import { detailsEl, inspView, fitSparks } from './details.js';

// under 1%: two significant figures (0.0061%, 0.013%), so long odds that differ don't print the same
const pct = (p) => { const x = p * 100; if (p === 0) return '0%';
  if (x < 1e-6) return '<0.000001%';
  return (x >= 10 ? x.toFixed(0) : x >= 1 ? x.toFixed(1) : x.toFixed(Math.min(6, 1 - Math.floor(Math.log10(x))))) + '%'; };
const fmtIn = (p) => p > 0 ? '1 in ' + (1 / p >= 100 ? Math.round(1 / p).toLocaleString() : (1 / p).toFixed(1)) : 'never';
const tName = (g) => { const s = SK.get(g) || { w: 'Skill ' + g, gd: null }; return goldGiven(g) && s.gd ? s.gd : s.w; };
const owned = (u) => state.roster[u.id] !== 'out' && !noOutfit(u);
const el = (tag, cls, text) => { const e = document.createElement(tag); if (cls) e.className = cls; if (text !== undefined) e.textContent = text; return e; };
const btn = (label, cls, fn) => { const b = el('button', 'btn' + (cls ? ' ' + cls : ''), label); b.type = 'button'; b.onclick = fn; return b; };

// ---------- status line ----------
let searching = false, t0 = 0, tick = null, timer = null, myId = 0;
let status = { kind: 'loading', text: 'Loading…' };
let last = null;   // the last result: {r, job}
function paint() {
  const n = $('run-note');
  $('rhead').dataset.kind = searching ? 'busy' : status.kind;
  $('results').classList.toggle('stale', !searching && ['error', 'empty'].includes(status.kind));
  if (searching) { const s = Math.floor((performance.now() - t0) / 1000);
    n.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Searching…<span aria-hidden="true">' + (s >= 1 ? ` ${s} s` : '') + '</span></span>'; }
  else if (planS.state === 'busy' && status.kind === 'done') { const s = Math.floor((performance.now() - planS.t0) / 1000);
    n.textContent = status.text + ' ';
    const sp = el('span'); sp.innerHTML = '<span class="spin" aria-hidden="true"></span>';
    sp.append('Planning: the next career…', el('span', '', s >= 1 ? ` ${s} s` : '')); sp.lastChild.setAttribute('aria-hidden', 'true');
    n.appendChild(sp); }
  else n.textContent = status.text + (status.kind === 'done' ? planNote() : '');
}
// the plan's line: finished, or stopped by its time limit, or failed
function planNote() {
  if (planS.state === 'error') return ` Plan not made: ${planS.text}`;
  if (planS.state !== 'done') return '';
  return ` Planned in ${(planS.ms / 1000).toFixed(1)} s.`;
}
// every change: redraw now, search again after a short pause (a change during a search stops it)
function schedule() {
  clearTimeout(timer); myId++;
  if (searching || simulation.state === 'busy' || planS.state === 'busy') killEngine();
  simulationId++;
  searching = true; t0 = performance.now(); paint();
  timer = setTimeout(search, 600);
}

// ---------- the job ----------
const idxOf = (id) => vets.findIndex(v => v.id === id);
class Blocked extends Error { constructor(kind, text) { super(text); this.kind = kind; } }
function buildJob() {
  if (!state.targets.length) throw new Blocked('empty', 'No target skills: add them with Target skills above.');
  const fixed = plan.parents.filter(Boolean).map(vetById);
  if (vets.length < 2) throw new Blocked('empty', 'Choose both parents, or import your veterans and let the planner pick.');
  if (fixed.length === 2 && fixed[0].chara === fixed[1].chara) throw new Blocked('error', `Both parents are ${umaName(fixed[0].chara)}: a career takes two different umas.`);
  if (fixed.length === 2 && fixed[0].borrowed && fixed[1].borrowed) throw new Blocked('error', 'Both parents are borrowed: a career can borrow one.');
  if (plan.trainee && fixed.some(v => v.chara === plan.trainee)) throw new Blocked('error', `${umaName(plan.trainee)} can't be her own parent: choose another trainee or parent.`);
  const exclude = UMAS.filter(u => !owned(u) && u.id !== plan.trainee).map(u => u.id);
  if (!plan.trainee && exclude.length >= UMAS.length) throw new Blocked('empty', 'No uma is owned: mark yours with Umas above.');
  const exclude_outfits = Object.entries(state.outfits).filter(([, v]) => v === false).map(([k]) => +k);
  const all = plan.trainee && fixed.length === 2;
  // a place left open: the K best careers by odds (the plan then picks among them and a few more, `runPlan`)
  const job = { cmd: 'career', targets: jobTargets(), exclude, exclude_outfits, extra: EXTRA, owned_cards: ownedCards(), borrow: true,
    star_dist: state.rank.split(',').map(Number), scenario: state.scen || 'unity', ...rcardsOff(), legacies: legaciesJob(), trainees: plan.trainee ? [plan.trainee] : [],
    fix: plan.parents.filter(Boolean).map(idxOf), per_trainee: false, top: all ? 1 : ROLL_K };
  return job;
}
// an 8/8 veteran: a spark of every target (with 8/8 parents: a fully looped parent)
const isFull = (nd) => !!nd && state.targets.length > 0 && state.targets.every(t => (nd.sparks || {})[t.group] > 0);
const ROLL_K = 6;
// the job of today's best career over everything (no place set, one result, no rollout): the comparison's baseline
const baseOf = (job) => ({ ...job, exclude: UMAS.filter(u => !owned(u)).map(u => u.id), trainees: [], fix: [], per_trainee: false, top: 1, rollout: undefined, p_now: undefined });
const baseKey = (job) => JSON.stringify(baseOf(job));
// after the main result: the plan (a second run: the planner's candidates and its pick), and the entered veteran with
// its comparison (on demand)
let planS = { state: 'none', r: null };
const keyOf = (r) => r.trainee.id + ':' + r.parents.join(',');
let simulation = { state: 'none', draft: null, r: null, baseline: null, error: null };
let simulationId = 0;
let baselineCache = null;
function comparisonJob() { return JSON.parse(baseKey(last.job)); }
async function search() {
  const id = ++myId; simulationId++;
  let job;
  try { job = buildJob(); } catch (e) { searching = false; status = { kind: e.kind || 'error', text: e.message }; last = null; simulation = { state: 'none' }; renderAll(); paint(); return; }
  searching = true; t0 = performance.now(); clearInterval(tick); tick = setInterval(paint, 1000); paint();
  try {
    const m = await run(job);
    if (id !== myId) return;
    if (m.r.error) { status = { kind: 'error', text: 'Not searched: ' + m.r.error }; last = null; simulation = { state: 'none' }; }
    else {
      last = { r: m.r, job, plan: JSON.stringify(plan) };
      const cert = m.r.proven_top ? 'The best odds are checked exact: no career does better.' : 'The exact check of the best odds ran out of time: their order may not be final.';
      const n = m.r.candidates;
      status = { kind: 'done', text: (n > 1 ? `Weighed ${n.toLocaleString()} careers in ${(m.ms / 1000).toFixed(1)} s. ` : `Scored in ${(m.ms / 1000).toFixed(1)} s. `) + cert };
      const best = m.r.results[0], r = best;
      simulation = r ? { state: 'ready', draft: draftOf(r), r: null, baseline: null, error: null, forKey: keyOf(r) } : { state: 'none' };
      // a trainee you chose leaves the plan nothing to pick: no plan
      planS = r && !plan.trainee ? { state: 'busy', r: null, t0: performance.now() } : { state: 'none' };
      if (best && !job.trainees.length && !job.fix.length) baselineCache = { key: baseKey(job), result: best };
    }
  } catch (e) { if (id !== myId) return; status = { kind: 'error', text: e.message }; last = null; simulation = { state: 'none' }; }
  searching = false; clearInterval(tick); renderAll(); paint();
  if (planS.state === 'busy') runPlan();
}
// The plan: the same job with `next` (solver: sim/plan.rs `index_rule`, exact odds, no simulation): every owned uma's
// best career scored by the careers it saves toward a fully looped parent (the cheapest trio). Its pick replaces the
// best-odds career on show. It shares the one worker with the comparison: a comparison cancelled while both were queued
// takes it down too, and it runs again (planId tells the run that counts).
let planId = 0;
async function runPlan() {
  const id = myId, pid = ++planId; if (!last) return;
  const { rollout: _r, p_now: _p, ...plain } = last.job;
  const job2 = { ...plain, next: {} };
  planS = { state: 'busy', r: null, t0: performance.now() };
  clearInterval(tick); tick = setInterval(paint, 1000); paint();
  try {
    const m2 = await run(job2);
    if (id !== myId || pid !== planId) return;
    const P = m2.r.next;
    planS = P && !P.error && P.options?.length ? { state: 'done', r: P, ms: m2.ms } : { state: 'error', text: m2.r.error || P?.error || 'no option found.' };
  } catch (e) { if (id !== myId || pid !== planId) return; planS = { state: 'error', text: e.message }; }
  clearInterval(tick);
  // the plan's pick goes on show: the veteran you got follows it (a comparison for the old career goes)
  const r = shown();
  if (r && simulation.draft && simulation.forKey !== keyOf(r)) { cancelCompare();
    simulation = { state: 'ready', draft: draftOf(r), r: null, baseline: simulation.baseline, error: null, forKey: keyOf(r) }; }
  renderAll(); paint();
}

// ---------- the pedigree ----------
// the career on show: the top result while it belongs to the current plan
// (the plan's pick once it is made, else the highest odds)
const shown = () => last && last.plan === JSON.stringify(plan) && last.r.results[0] ? (planS.state === 'done' && planS.r.choice || last.r.results[0]) : null;
// the plan's row for a career (null before the plan is made)
const planRow = (r) => planS.state === 'done' ? planS.r.options.find(o => keyOf(o) === keyOf(r)) || null : null;
// an option's role: the best odds now, a loop member, or an uma of the best career's lineage or of your veterans
const roleText = (o) => o.role === 'plan' ? 'in the plan\'s trio' : o.role === 'finish' ? 'the plan\'s finisher' : o.role === 'loop' ? 'loop member' : o.role === 'candidate' ? 'candidate' : 'builder';
// an option's net saving: the careers her veteran saves toward a fully looped parent on average, minus the one it costs
const netText = (x) => (x >= 0 ? '+' : '−') + Math.abs(x).toFixed(1);
// an option's score: one career deep, plus (for the few highest and the best odds) what her veteran opens for the next
const scoreOf = (o) => o.score ?? o.index;
const fmtN = (x) => Math.round(x).toLocaleString();
// which veteran sits in parent place k: the chosen one, else the planner's pick in the result on show
function parentAt(k, r) {
  if (plan.parents[k]) return { v: vetById(plan.parents[k]), pick: false };
  if (!r) return null;
  const fixedI = plan.parents.map(id => id ? idxOf(id) : -1), other = fixedI[1 - k];
  const free = r.parents.filter(i => i !== other);
  const i = other >= 0 ? free[0] : r.parents[k];
  return i === undefined ? null : { v: vets[i], pick: true };
}
// a legacy's affinity in the career on show (slot of veteran i, or of her parent j)
const affOf = (r, i, j = null) => { if (!r) return null; const s = r.slots.find(x => x.legacy === i && (j === null ? x.kind === 'parent' : x.gp === j)); return s ? s.aff : null; };
// a node of the pedigree: saddlecloth number (the six legacies 1-6), what it is, who, sparks, actions
function pnode(kind, gate, label, ghost) {
  const n = el('div', `pn ${kind}` + (ghost ? ' ghost' : ''));
  if (gate) n.style.setProperty('--gate', `var(--gate${gate})`), n.style.setProperty('--gate-ink', `var(--gate${gate}-ink)`);
  const h = el('div', 'pnh');
  if (gate) h.appendChild(el('span', 'gate', String(gate)));
  h.appendChild(el('span', 'pnl', label));
  n.appendChild(h);
  return n;
}
// The tree, as the game's legacy display: portraits only (the trainee; Legacy 1 blue and Legacy 2 pink, each the parent
// with her own two parents), each with its legacy number and affinity; a portrait opens the node popup (her sparks,
// G1 wins, choose or hand back the place, change who she is). The popup is refilled after every edit (`fillNode`).
// a click anywhere in a node's box opens it, as its portrait does (not on the box's own buttons or deck cards)
const wholeBox = (box, t) => { box.classList.add('pnclick'); box.addEventListener('click', (e) => { if (!e.target.closest('button, a, .deck, input')) t.click(); }); };
function tile(key, face, name, sub, extra) {
  const b = el('button', 'ptile'); b.type = 'button'; b.dataset.k = key; b.setAttribute('aria-haspopup', 'dialog');
  b.appendChild(face); const nm = el('span', 'tnm'); nm.appendChild(el('span', 'nm', name)); if (sub) nm.appendChild(el('span', 'sub', sub)); if (extra) nm.appendChild(extra);
  b.appendChild(nm); return b;
}
function traineeEl(r) {
  const id = plan.trainee || (r && r.trainee.id);
  const n = pnode('trainee', 0, plan.trainee ? 'Trainee' : 'Trainee · planner\'s pick', !plan.trainee);
  // the affinity she holds from her six legacies (its breakdown is in her popup)
  if (r && r.trainee.id === id) {
    const tot = legacyAff(r), a = affEl(tot, 'aff tot'); a.title = `Affinity ${Math.round(tot)}, her six legacies: click her for the breakdown`;
    n.querySelector('.pnh').appendChild(a);
  }
  const sub = id && r && r.trainee.id === id ? (r.trainee.outfits.length ? r.trainee.outfits.map(o => o.label).join(' or ') : 'any outfit') : null;
  const t = id ? tile('node:t', umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 96), umaName(id), sub)
    : tile('node:t', el('span', 'av empty'), 'Any uma', 'The planner tries every uma you own.');
  t.setAttribute('aria-label', `Trainee: ${id ? umaName(id) : 'the planner picks'}. Details`); t.onclick = () => openNode({ kind: 't' });
  n.appendChild(t); wholeBox(n, t);
  // her career's deck (opens here, under her) and race schedule (its window), behind buttons
  if (r && r.trainee.id === id) {
    const row = el('div', 'row tdeck');
    const db = btn(deckOpen ? 'Hide the deck' : 'Deck', '', () => { deckOpen = !deckOpen; renderTree(); $('deck-open')?.focus(); });
    db.id = 'deck-open'; db.dataset.k = 'deck-open'; db.setAttribute('aria-expanded', String(deckOpen)); db.setAttribute('aria-controls', 'tree-deck');
    row.append(db, schedBtn(r)); n.appendChild(row);
    if (deckOpen) { const dk = deckEl(r, 'Deck'); dk.id = 'tree-deck'; n.appendChild(dk); }
  }
  return n;
}
// affinity: the game's heart and the number (named "Affinity N" for screen readers and the tooltip)
// her affinity, legacy by legacy (afftable.js), from the report's six slots (career.rs slots()); Legacy 1 and 2 follow
// the career's parents (a slot's legacy is her index among your veterans)
const legacyAff = (r) => r.slots.reduce((a, x) => a + x.aff, 0);
function affTable(r) {
  const nm = (x) => x.name || umaName(x.chara);
  return affTableOf(r.parents.map((ix) => r.slots.find(x => x.kind === 'parent' && x.legacy === ix)).filter(Boolean).map(p => ({
    face: umaAv(p.chara || 0, null, 30), name: nm(p), rel: p.rel, race: p.race, aff: p.aff,
    gps: r.slots.filter(x => x.kind === 'grandparent' && x.legacy === p.legacy).sort((a, b) => a.gp - b.gp)
      .map(g => ({ face: umaAv(g.chara || 0, null, 24), name: nm(g), rel: g.rel, race: g.race, aff: g.aff })) })));
}
function familyEl(k, r) {
  const fam = el('div', 'pf');
  // the game's legacy box: "Legacy 1" blue, "Legacy 2" pink, the parent with her own two parents
  fam.appendChild(el('span', 'pflab', `Legacy ${k + 1}`));
  const at = parentAt(k, r), gate = k * 3 + 1;
  const n = pnode('parent', 0, (at && at.pick ? 'Planner\'s pick' : at ? 'Your pick' : `Parent ${k + 1}`) + (at && at.v.borrowed ? ' · guest' : ''), at && at.pick);
  if (at && at.v.borrowed) n.classList.add('bor');
  let t;
  if (at) {
    const v = at.v, i = idxOf(v.id), a = affOf(r, i);
    if (a !== null) n.querySelector('.pnh').appendChild(affEl(a));
    t = tile('node:p' + k, umaAv(v.chara, v.outfit, 72), umaName(v.chara), `Veteran #${i + 1}` + (v.src === 'hand' ? ', made here' : ''));
    t.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Details`);
  } else { t = tile('node:p' + k, el('span', 'av empty'), vets.length >= 2 ? 'The planner picks' : 'No parent yet', null); t.setAttribute('aria-label', `Parent ${k + 1}: none yet. Details`); }
  t.onclick = () => openNode({ kind: 'p', k });
  n.appendChild(t); wholeBox(n, t); fam.appendChild(n);
  // her own parents: the trainee's grandparents
  if (at) {
    const gk = el('div', 'pk');
    at.v.parents.forEach((g, j) => {
      const gn = pnode('gp', 0, 'Grandparent', false);
      const a = affOf(r, idxOf(at.v.id), j); if (a !== null) gn.querySelector('.pnh').appendChild(affEl(a));
      const gt = tile(`node:p${k}g${j}`, umaAv(g.chara || 0, g.outfit, 48), g.chara ? umaName(g.chara) : 'Unknown: pick her', null);
      gt.setAttribute('aria-label', `Grandparent: ${g.chara ? umaName(g.chara) : 'unknown'}. Details`); gt.onclick = () => openNode({ kind: 'g', k, j });
      gn.appendChild(gt); wholeBox(gn, gt); gk.appendChild(gn);
    });
    fam.appendChild(gk);
  }
  return fam;
}
let deckOpen = false;   // the deck under the trainee: closed at first
// the node popup (game style): one dialog, refilled on every redraw while open
let nodeDlg = null, nodeOpen = null;
function openNode(spec) {
  if (!nodeDlg) {
    nodeDlg = el('dialog', 'gmdlg nodedlg'); nodeDlg.id = 'node-dlg'; nodeDlg.setAttribute('aria-labelledby', 'node-title');
    const h = el('h3', 'gmdhdr'); h.id = 'node-title'; const body = el('div', 'gmdbody'); body.id = 'node-body';
    const f = el('div', 'gmdfoot'); const c = el('button', 'btn gmclose', 'Close'); c.type = 'button'; c.onclick = () => nodeDlg.close(); f.appendChild(c);
    nodeDlg.append(h, body, f);
    nodeDlg.addEventListener('close', () => { const sp = nodeOpen; nodeOpen = null; const key = sp && (sp.kind === 't' ? 'node:t' : sp.kind === 'p' ? 'node:p' + sp.k : sp.kind === 'v' ? 'node:v' + sp.id : `node:p${sp.k}g${sp.j}`);
      const e = key && document.querySelector(`[data-k="${key}"]`); if (e && !document.querySelector('dialog[open]')) e.focus(); });
    document.body.appendChild(nodeDlg);
  }
  nodeOpen = spec; fillNode(); if (!nodeDlg.open) nodeDlg.showModal();
}
// a dialog of its own from the popup (the uma or parent picker): the popup closes first
const fromNode = (fn) => () => { if (nodeDlg?.open) nodeDlg.close(); fn(); };
function fillNode() {
  if (!nodeOpen || !nodeDlg) return;
  const r = shown(), body = $('node-body'); body.textContent = ''; const sp = nodeOpen;
  const acts = el('div', 'row tacts');
  if (sp.kind === 't') {
    $('node-title').textContent = plan.trainee ? 'Trainee' : 'Trainee · planner\'s pick';
    const id = plan.trainee || (r && r.trainee.id);
    if (id) { const w = el('div', 'twho'); w.append(umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 72), el('div', 'nm', umaName(id))); body.appendChild(w); }
    else body.appendChild(el('p', 'tempty', 'The planner tries every uma you own.'));
    if (id && r && r.trainee.id === id) body.appendChild(affTable(r));
    acts.appendChild(btn(plan.trainee ? 'Change trainee' : 'Choose trainee', 'primary', fromNode(() => openUmaPicker('Trainee: which uma?', (u) => { plan.trainee = u; changed(); }, owned))));
    if (plan.trainee) acts.appendChild(btn('Let the planner pick', '', () => { plan.trainee = null; changed(); }));
  } else if (sp.kind === 'p') {
    const k = sp.k, at = parentAt(k, r);
    $('node-title').textContent = `Legacy ${k + 1} · parent ${k + 1}` + (at && at.pick ? ' · planner\'s pick' : '');
    if (at) {
      const v = at.v, i = idxOf(v.id), w = el('div', 'twho');
      const pb = el('button', 'facebtn'); pb.type = 'button'; pb.append(umaAv(v.chara, v.outfit, 64)); pb.title = 'Choose another parent'; pb.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Choose another parent`);
      pb.onclick = fromNode(() => openParentPicker(k));
      const nm = el('div', 'tnm'); nm.append(el('div', 'nm', umaName(v.chara)), el('div', 'sub', `Veteran #${i + 1}` + (v.src === 'hand' ? ', made here' : '') + (v.borrowed ? ', borrowed' : '')));
      w.append(pb, nm, winsBtn(v, umaName(v.chara), 'p' + k)); body.append(w);
      // her Inspiration view, as the card's: her sparks, then her parents' (the trainee's grandparents), all editable
      const iv = el('div', 'ud udbare'); iv.appendChild(inspView({ d: v, key: 'p' + k, onEdit: changed, parentKey: (j) => `p${k}g${j}`, onEditParent: changed, header: false }));
      body.appendChild(iv);
    } else body.appendChild(el('p', 'tempty', vets.length >= 2 ? 'The planner picks from your veterans.' : 'No parent yet.'));
    acts.appendChild(btn(plan.parents[k] ? 'Change parent' : 'Choose parent', 'primary big', fromNode(() => openParentPicker(k))));
    if (plan.parents[k]) acts.appendChild(btn('Let the planner pick', '', () => { plan.parents[k] = null; changed(); }));
  } else if (sp.kind === 'v') {
    // a veteran of yours (the pool, Career Info's legacies): her sparks and her parents', editable in place
    const i = idxOf(sp.id), v = vets[i]; if (!v) { nodeDlg.close(); return; }
    $('node-title').textContent = `Veteran #${i + 1}` + (v.borrowed ? ' · borrowed' : '');
    const w = el('div', 'twho'), b = el('button', 'facebtn'); b.type = 'button'; b.append(umaAv(v.chara, v.outfit, 56));
    b.title = 'Change who she is'; b.setAttribute('aria-label', `Veteran #${i + 1}: ${umaName(v.chara)}. Change who she is`); b.onclick = fromNode(() => changeUma(v, `Veteran #${i + 1}`));
    // where she came from (entered from a career here, an uma.moe tree, an import, made here)
    const src = { entered: 'Entered here', simulated: 'Entered here', tree: 'From uma.moe', import: 'Imported', hand: 'Made here' }[v.src];
    w.append(b, el('div', 'nm', umaName(v.chara)), winsBtn(v, `Veteran #${i + 1}, ${umaName(v.chara)}`, 'v' + v.id));
    if (src) w.appendChild(el('span', v.src === 'tree' ? 'srctag' : 'simtag', src));
    body.appendChild(w);
    const iv = el('div', 'ud udbare'); iv.appendChild(inspView({ d: v, key: 'v' + v.id, onEdit: changed, parentKey: (j) => `v${v.id}g${j}`, onEditParent: changed, header: false }));
    body.appendChild(iv);
    // her parents: who they are and their G1 wins
    const pr = el('div', 'vpars');
    v.parents.forEach((g, j) => { const x = el('div', 'twho'), gb = el('button', 'facebtn'); gb.type = 'button'; gb.append(umaAv(g.chara || 0, g.outfit, 36));
      const lab = `Parent ${j + 1} of #${i + 1}`; gb.title = `${lab}: ${umaName(g.chara)}. Change who she is`; gb.setAttribute('aria-label', gb.title); gb.onclick = fromNode(() => changeUma(g, lab));
      x.append(gb, el('span', 'nm', g.chara ? umaName(g.chara) : 'Unknown parent'), winsBtn(g, `${lab}, ${umaName(g.chara)}`, `v${v.id}g${j}`)); pr.appendChild(x); });
    body.appendChild(pr);
  } else {
    const at = parentAt(sp.k, r), g = at && at.v.parents[sp.j];
    $('node-title').textContent = `Legacy ${sp.k + 1} · grandparent`;
    if (!g) { nodeDlg.close(); return; }
    const w = el('div', 'twho'), b = el('button', 'facebtn'); b.type = 'button'; b.append(umaAv(g.chara || 0, g.outfit, 56));
    b.title = 'Change who she is'; b.setAttribute('aria-label', `Grandparent: ${umaName(g.chara)}. Change who she is`); b.onclick = fromNode(() => changeUma(g, 'Grandparent'));
    w.append(b, el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'), winsBtn(g, umaName(g.chara), `p${sp.k}g${sp.j}`));
    const iv = el('div', 'ud udbare'); iv.appendChild(inspView({ d: { ...g, parents: [] }, key: `p${sp.k}g${sp.j}`, onEdit: changed, header: false }));
    // (a copy without parents for the view; her sparks object is hers: edits land on her)
    iv.querySelector('.simskills').replaceChildren(sparksEl(g, `p${sp.k}g${sp.j}`, changed));
    body.append(w, iv);
  }
  // the popup's actions sit in its footer beside Close (no row of their own)
  const foot = nodeDlg.querySelector('.gmdfoot'); foot.querySelectorAll('.nact').forEach(e => e.remove());
  for (const b of [...acts.children]) { b.classList.add('nact'); foot.appendChild(b); }
  fitSparks(nodeDlg);
}
function renderTree() {
  const r = shown(), box = $('tree'); box.textContent = '';
  const t = el('div', 'pt'); t.appendChild(traineeEl(r)); box.appendChild(t);
  const k = el('div', 'pk'); k.append(familyEl(0, r), familyEl(1, r)); box.appendChild(k);
  if (nodeDlg?.open) fillNode();
}

// ---------- the parent picker ----------
let pickK = 0;
function vpTab(t) {
  for (const x of ['vets', 'new', 'auto']) { $('vp-tab-' + x).setAttribute('aria-selected', String(x === t)); $('vp-' + x).hidden = x !== t; }
  if (t === 'new') { $('vp-filter').value = ''; drawNew(); $('vp-filter').focus(); }
}
const drawNew = () => drawUmas($('vp-umas'), $('vp-filter').value, (id) => {
  $('vpick-dlg').close();
  askSkills(umaName(id), false, (sp) => { const v = newVet(id, sp || {}); plan.parents[pickK] = v.id; changed(); }, true);
});
// why veteran v can't go in place k (or '')
function blockedFor(v, k) {
  const o = plan.parents[1 - k] && vetById(plan.parents[1 - k]);
  if (plan.trainee && v.chara === plan.trainee) return 'the trainee herself';
  if (o && o.chara === v.chara) return 'same uma as the other parent';
  if (o && o.borrowed && v.borrowed) return 'the other parent is borrowed';
  return '';
}
function openParentPicker(k) {
  pickK = k; $('vpick-title').textContent = `Choose parent ${k + 1}`;
  const box = $('vp-list'); box.textContent = '';
  vets.forEach((v, i) => {
    const why = blockedFor(v, k), cur = plan.parents[k] === v.id;
    const b = el('button', 'vpt' + (cur ? ' cur' : '') + (v.borrowed ? ' bor' : '')); b.type = 'button'; b.disabled = !!why;
    const top = el('div', 'vpth'); top.append(umaAv(v.chara, v.outfit, 48));
    const nm = el('div'); nm.append(el('div', 'nm', umaName(v.chara)), el('div', 'sub', `#${i + 1}${v.borrowed ? ', borrowed' : ''}${cur ? ', chosen' : ''}`)); top.appendChild(nm);
    b.appendChild(top);
    const tg = state.targets.map(t => t.group), have = tg.filter(g => v.sparks[g]), stars = have.reduce((a, g) => a + v.sparks[g], 0);
    b.appendChild(el('div', 'vpts', `${have.length} of ${tg.length} target skills, ${stars}★`));
    const gp = el('div', 'vptg'); gp.append('Parents: ');
    for (const g of v.parents) { gp.append(umaAv(g.chara || 0, g.outfit, 22)); }
    gp.append(' ' + v.parents.map(g => g.chara ? umaName(g.chara) : 'unknown').join(', '));
    b.appendChild(gp);
    if (why) b.appendChild(el('div', 'vptw', `Can't: ${why}`));
    b.onclick = () => { plan.parents[k] = v.id; $('vpick-dlg').close(); changed(); };
    box.appendChild(b);
  });
  if (!vets.length) box.innerHTML = '<p class="hint">No veterans yet: import them in the paddock below, or make a new one.</p>';
  vpTab(vets.length ? 'vets' : 'new');
  $('vpick-dlg').showModal();
}

// ---------- the career: odds, deck, targets, schedule, after this career, other picks ----------
const sec = (cls, title) => { const s = el('section', 'rbsec ' + cls); s.appendChild(el('h3', '', title)); return s; };
// the odds as the loop planner shows them
// the career's odds in the veteran card's header: all n; a click opens the odds popup (exactly each count)
function oddsBtn(r) {
  const n = state.targets.length, b = el('button', 'udodds'); b.type = 'button'; b.id = 'odds-open'; b.dataset.k = 'odds-open'; b.setAttribute('aria-haspopup', 'dialog');
  b.append(el('b', '', fmtIn(r.p_all)), el('span', '', ` to land all ${n}`));
  b.setAttribute('aria-label', `${fmtIn(r.p_all)} runs to land all ${n}. The odds of every count`);
  b.onclick = () => openOdds(r);
  return b;
}
// the chance of landing exactly k of the n targets: exact over the landed sets (outcome_sets), else each target on its
// own odds (over 12 targets); the all-n and n-1-or-better figures are the solver's exact ones
function landedCounts(r) {
  const n = state.targets.length, ex = new Array(n + 1).fill(0);
  if (Array.isArray(r.outcome_sets)) r.outcome_sets.forEach((p, m) => { let c = 0; for (let x = m; x; x &= x - 1) c++; ex[c] += p; });
  else { let v = [1]; for (const t of r.targets) { const w = new Array(v.length + 1).fill(0); v.forEach((x, i) => { w[i] += x * (1 - t.p); w[i + 1] += x * t.p; }); v = w; } v.forEach((x, i) => { ex[i] = x; }); }
  return ex;
}
let oddsDlg = null;
function openOdds(r) {
  if (!oddsDlg) { oddsDlg = el('dialog', 'gmdlg'); oddsDlg.id = 'odds-dlg'; oddsDlg.setAttribute('aria-labelledby', 'odds-title');
    oddsDlg.addEventListener('close', () => $('odds-open')?.focus()); document.body.appendChild(oddsDlg); }
  const n = state.targets.length, ex = landedCounts(r); oddsDlg.textContent = '';
  const h = el('h3', 'gmdhdr', 'Odds of this career'); h.id = 'odds-title'; oddsDlg.appendChild(h);
  const body = el('div', 'gmdbody');
  // the chance of landing exactly k of the n targets, k = n .. 0 (all n: the solver's exact figure)
  const tb = el('table', 'oddtab'); tb.innerHTML = '<thead><tr><th scope="col">Targets landed</th><th scope="col">Chance</th></tr></thead><tbody></tbody>';
  for (let k = n; k >= 0; k--) {
    const tr = el('tr', k === n ? 'all' : ''); const th = el('th', '', `${k}/${n}`); th.scope = 'row';
    tr.append(th, el('td', '', pct(k === n ? r.p_all : ex[k]))); tb.tBodies[0].appendChild(tr);
  }
  body.append(tb, el('p', 'gmdstat', `${r.expected.toFixed(2)} of ${n} expected.`));
  oddsDlg.appendChild(body);
  const f = el('div', 'gmdfoot'), c = el('button', 'btn gmclose', 'Close'); c.type = 'button'; c.onclick = () => oddsDlg.close(); f.appendChild(c); oddsDlg.appendChild(f);
  oddsDlg.showModal(); c.focus();
}
// the deck as the loop planner shows it: card art and name; a click opens the card's panel (owned or not, GameTora)
function deckEl(r, title = 'Deck', extra = null) {
  const d = r.deck, pals = new Set(d.pal_cards || []);
  const dk = el('div', 'deck'); dk.innerHTML = '<div class="dtitle"><b></b></div><div class="dcards"></div>';
  dk.querySelector('.dtitle b').textContent = title;
  if (extra) dk.querySelector('.dtitle').appendChild(extra);
  const box = dk.querySelector('.dcards');
  d.cards.forEach((name, i) => {
    const bor = /\(borrowed\)$/.test(name), base = name.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, '').replace(/ \(link \d+: [^)]*\)$/, '');
    const id = d.card_ids[i] ?? d.plain_ids[i - d.card_ids.length], c = CARD_BY_ID.get(id) || CARDS.find(x => x.name === base);
    const endAt = (d.end_chain_at || [])[i] || 0, pick = (d.choices || [])[i] || (c && c.pri ? c.pri.join(', ') : '');
    const t = c ? cardTitle(c) : { nm: base, ti: '' }, tag = bor ? 'borrowed' : c && c.pal ? 'pal' : '';
    const full = t.nm + (t.ti ? ' · ' + t.ti : '') + (tag ? ` (${tag})` : '') + (pick ? ' · prioritise ' + pick : '');
    const b = el('button', 'dc' + (bor ? ' bor' : c && c.pal ? ' pal' : '')); b.type = 'button';
    if (c) b.dataset.cid = c.id; b.dataset.name = t.nm; if (endAt) b.dataset.end = endAt;
    b.title = full; b.setAttribute('aria-label', full); b.setAttribute('aria-haspopup', 'dialog'); b.setAttribute('aria-expanded', 'false');
    b.append(cardAv(c || null, 96), el('span', 'dn', full));
    if (pick) b.appendChild(el('i', '', 'prioritise ' + pick));
    if (endAt) b.appendChild(el('i', '', 'chain end: link ' + endAt));
    box.appendChild(b);
  });
  return dk;
}
function targetsEl(r) {
  const s = sec('rtgt', 'Target skills');
  const tb = el('table', 'cartab');
  tb.innerHTML = '<thead><tr><th scope="col">Skill</th><th scope="col" title="Chance the six legacies pass the skill on (two inheritance events)">Inherited</th><th scope="col" title="Legacies (of six) with a spark of it">Sparks in lineage</th><th scope="col" title="Chance the new veteran gets its spark">Lands as a spark</th></tr></thead><tbody></tbody>';
  for (const t of r.targets) {
    // the skill as the game's tile (the card's Skills tab), gold for a gold target
    const row = el('tr'), c0 = el('th'); c0.scope = 'row';
    const tile = el('div', 'udsk' + (t.gold ? ' gold' : '')), ic = skillIcon(t.group, t.gold); if (ic) tile.appendChild(ic);
    tile.appendChild(el('span', 'udskn', tName(t.group))); c0.appendChild(tile);
    row.appendChild(c0);
    for (const v of [t.gold ? 'scenario' : pct(t.q), `${t.carriers} of 6`, pct(t.p)]) row.appendChild(el('td', '', v));
    tb.tBodies[0].appendChild(row);
  }
  s.appendChild(tb);
  return s;
}
// her race schedule: the loop planner's schedule window, from a button beside the deck
// (the races she runs: the G1s she could enter but doesn't aren't shown)
const schedOf = (r) => ({ members: [{ id: r.trainee.id, name: r.trainee.name, short: r.trainee.name }], outfits: [{ outfits: r.trainee.outfits }],
  schedule: (r.schedule || []).filter(x => x.run !== false) });
function schedBtn(r) {
  const ran = schedOf(r).schedule.length;
  const b = btn('Race schedule', '', () => openSched(schedOf(r)));
  b.id = 'sched-open'; b.title = ran ? `She runs ${ran} G1s: ${r.g1_wins.toFixed(1)} wins expected` : 'No G1s';
  return b;
}
// the veteran analysis' G1 wins: the G1 wins editor of the pool (a win in its slot, click to remove; an empty slot lists the
// G1s she could enter then). Her schedule's races keep their win odds in the turn bar; a win shows at the turn picked for
// it, else the turn her schedule runs it. A race held in Classic and Senior is one win saddle (one race index).
function openDraftWins(d, r) {
  const rows = r.schedule || [], runs = new Map(), can = new Map();
  for (const w of rows) {
    const ix = CAREER.races.indexOf(w.name || w.race); if (ix < 0) continue;
    if (!can.has(w.turn)) can.set(w.turn, []); can.get(w.turn).push(ix);
    if (w.run !== false && !runs.has(ix)) runs.set(ix, { turn: w.turn, cell: w.cells[0] });
  }
  const parentWon = (ix) => d.parents.some(p => p.wins.includes(ix));
  openWins(d, umaName(d.chara), {
    offer: rows.length ? (t) => can.get(t) || [] : null,
    info: (ix) => { const x = runs.get(ix); return x || parentWon(ix) ? { ...(x || {}), parent: parentWon(ix) } : null; },
    onEdit: editDraft,
    onClose: () => $('sim-wins')?.focus() });   // the button was redrawn by the edits
}
// a career as portraits: trainee (outfit), then her two parents; parents: [{chara, outfit, label, her}]
function facesEl(tid, outfit, parents, outfitLabel = null) {
  const box = el('span', 'cfaces');
  const t = el('span', 'cface tr'); t.append(umaAv(tid, outfit, 32), umaName(tid) + (outfitLabel ? ' ' + outfitLabel : '')); box.appendChild(t);
  box.appendChild(el('span', 'cwith', 'with'));
  parents.forEach((p, j) => { const f = el('span', 'cface'); f.append(umaAv(p.chara, p.outfit, 26), p.label); if (p.her) f.appendChild(el('span', 'her', 'her'));
    box.appendChild(f); if (!j) box.appendChild(el('span', 'cwith', '+')); });
  return box;
}
// a parent of a future career by legacy index (index = vets.length: the veteran she leaves, `who`)
const parentFace = (i, who) => i >= vets.length ? { chara: who.chara, outfit: who.outfit, label: umaName(who.chara), her: true }
  : { chara: vets[i].chara, outfit: vets[i].outfit, label: `#${i + 1} ${umaName(vets[i].chara)}` };
// ---------- the veteran analysis (the veteran you got) ----------
// The decision is per veteran: enter the one you got (she starts as a random result, drawn as Reroll draws) and Compare runs both
// choices, running this career again or using her in the next one, on the plan's terms (expected careers to land).
const copyNode = (v) => ({ chara: v.chara, ...(v.outfit ? { outfit: v.outfit } : {}), sparks: { ...v.sparks }, pinks: { ...v.pinks }, blues: { ...(v.blues || {}) }, greens: { ...(v.greens || {}) }, wins: [...v.wins] });
function scheduledRaces(r) {
  const races = new Map();
  for (const w of (r.schedule || []).filter(w => w.run !== false)) {
    const ix = CAREER.races.indexOf(w.name || w.race); if (ix < 0) continue;
    const prev = races.get(ix);
    races.set(ix, { ix, name: w.name || w.race, p: 1 - (1 - (prev?.p || 0)) * (1 - w.p), turns: [...(prev?.turns || []), w.turn] });
  }
  return [...races.values()];
}
// the skills she bought in a roll, given the sparks it landed (the report's per-target buying and landing chances):
// a landed one is bought, gold with chance land_gold / land; one that didn't land was bought anyway with chance
// (buy - land) / (1 - land), gold with chance (gold - land_gold) / (buy - land)
function rolledBought(r, sparks) {
  const bought = {};
  for (const t of r.targets) { const b = t.buy; if (!b) continue; const L = b.land_gold + b.land_white, B = b.gold + b.white;
    if (sparks[t.group]) bought[t.group] = Math.random() < (L > 0 ? b.land_gold / L : 0) ? 'gold' : 'white';
    else if (L < 1 && Math.random() < Math.max(0, B - L) / (1 - L)) bought[t.group] = Math.random() < (B - L > 0 ? Math.max(0, b.gold - b.land_gold) / (B - L) : 0) ? 'gold' : 'white'; }
  return bought;
}
// her pink spark drawn (MODEL.md): her start grades, each legacy pink firing at each April inspiration (+1 grade, at most S),
// then one aptitude uniform over those at A or S, stars 20/70/10
function rolledPink(r) {
  const ap = r.aptitudes; if (!ap) return { pinks: {}, apt: null };
  const g = ap.start.slice();
  for (let e = 0; e < 2; e++) for (const f of ap.fire) if (Math.random() < f.p) g[f.apt] = Math.min(8, g[f.apt] + 1);
  const as = g.map((x, k) => x >= 7 ? k : -1).filter(k => k >= 0); if (!as.length) return { pinks: {}, apt: g };
  const k = as[Math.floor(Math.random() * as.length)], u = Math.random();
  return { pinks: { [ap.pink_gen[k].group]: u < 0.2 ? 1 : u < 0.9 ? 2 : 3 }, apt: g };
}
// a random result of this career: the set of landed targets drawn from its exact odds, each spark's stars from the rank,
// each G1 she runs won with its chance (it differs every time; the career's first showing is one of these too)
function rolledOutcome(r) {
  const w = state.rank.split(',').map(Number);
  const star = () => { const u = Math.random(); return u < w[0] ? 1 : u < w[0] + w[1] ? 2 : 3; };
  const sparks = {};
  if (Array.isArray(r.outcome_sets)) {
    const u = Math.random(); let c = 0, mask = r.outcome_sets.length - 1;
    for (let i = 0; i < r.outcome_sets.length; i++) { c += r.outcome_sets[i]; if (u < c) { mask = i; break; } }
    state.targets.forEach((t, i) => { if (mask & (1 << i)) sparks[t.group] = star(); });
  } else r.targets.forEach(t => { if (Math.random() < t.p) sparks[t.group] = star(); });   // over 12 targets: each on its own odds
  const wins = scheduledRaces(r).filter(x => Math.random() < x.p).map(x => x.ix).sort((a, b) => a - b);
  const bought = rolledBought(r, sparks), pk = rolledPink(r);
  return { sparks, bought, wins, pinks: pk.pinks, apt: pk.apt, ...rolledBlueGreen(r) };
}
// her blue (stat) spark: one of the five stats, near-equally likely (hakuraku: 20% each, a slight lean to her stronger
// stats); its stars follow that stat's value, which the model doesn't have: the rank band's star mix stands in. Her unique
// (green) spark: always (GameTora: guaranteed for an outfit at 3 stars or more; the page assumes yours are), stars by the
// rank band (hakuraku: as white sparks, not by the skill's level)
function rolledBlueGreen(r) {
  const w = state.rank.split(',').map(Number), star = () => { const u = Math.random(); return u < w[0] ? 1 : u < w[0] + w[1] ? 2 : 3; };
  const blues = { [1 + Math.min(4, Math.floor(Math.random() * 5))]: star() };
  const out = r.trainee.outfits[0]?.id, greens = {};
  if (out && (CAREER.uniques || {})[out]) greens[out] = star();
  return { blues, greens };
}
function rollDraft() { const r = shown(); if (!r || searching) return; Object.assign(simulation.draft, rolledOutcome(r), { winAt: {} }); simulation.r = null; simulation.stale = false; compare(); }
// A comparison in flight is for a veteran that is going (a roll, a reset, Compare pressed again): stop it. A wasm call
// can't be interrupted, so the worker goes (run() starts a new one). Returns whether the plan went with it.
function cancelCompare() {
  if (simulation.state !== 'busy') return false;
  simulationId++; killEngine(); simulation.state = 'ready';
  return planS.state === 'busy';
}
function draftOf(r) { return { chara: r.trainee.id, outfit: r.trainee.outfits[0]?.id, borrowed: false, pinks: {}, ...rolledOutcome(r), winAt: {}, parents: r.parents.map(i => copyNode(vets[i])) }; }
function draftJob(v) {
  const pairs = (o) => Object.entries(o).map(([g, s]) => [+g, s]);
  const node = (v) => ({ chara: v.chara || 0, sparks: pairs(v.sparks), pinks: pairs(v.pinks), wins: [...v.wins] });
  return { ...node(v), borrowed: false, parents: v.parents.map(node) };
}
// an edit to the entered veteran: the comparison on screen is for the old one
function editDraft() { if (simulation.r) simulation.stale = true; renderSimulation(); }
// Compare (or a roll): a press while a comparison runs restarts it with the veteran as she is now. The plan, if the
// restart took it down, is queued again behind the comparison (runComparison posts its first job before it returns).
function compare() {
  if (searching || !shown()) return;
  const lost = cancelCompare();
  runComparison();
  if (lost) runPlan();
}
async function runComparison() {
  const id = ++simulationId, source = myId;
  const base = comparisonJob(), key = JSON.stringify(base);
  simulation.state = 'busy'; simulation.error = null; simulation.stale = false; renderSimulation();
  try {
    if (!simulation.baseline) {
      const open = !last.job.trainees.length && !last.job.fix.length;
      const cached = baselineCache?.key === key ? baselineCache.result : null;
      const before = cached ? { results: [cached] } : open ? last.r : (await run(base)).r;
      if (id !== simulationId || source !== myId) return;
      if (before.error || !before.results?.length) throw new Error(before.error || 'No career found in the current roster.');
      simulation.baseline = before.results[0];
      baselineCache = { key, result: simulation.baseline };
    }
    // the planner's question (exact odds, estimated careers): the best career with her as a parent, and the planner's
    // expected careers to a fully looped parent once its next career runs, without and with her (an n/n: goal step 1 done)
    const legs = [...base.legacies, draftJob(simulation.draft)], her = base.legacies.length;
    const m = await run({ ...base, fix: [her], legacies: legs, top: 1, compare: true });
    if (id !== simulationId || source !== myId) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    // the planner's expected careers to a fully looped parent once its best next career runs, without and with her:
    // W = V0 - (the best positive score) = 1 + E[V0 after that career] (n/n: goal step 1 done)
    const full = isFull(simulation.draft);
    let pb = planS.state === 'done' && !last.job.fix.length ? planS.r : null;
    if (!pb) { const mb = await run({ ...base, next: {}, compare: true }); if (id !== simulationId || source !== myId) return; pb = mb.r.next || null; }
    const ma = await run({ ...base, legacies: legs, next: {}, compare: true }); if (id !== simulationId || source !== myId) return;
    const pa = ma.r.next || null;
    const wOf = (P) => P && P.v0 != null ? P.v0 - Math.max(0, ...P.options.map(o => scoreOf(o) ?? 0)) : null;
    const v0b = wOf(pb), pickA = pa?.options?.find(o => o.chosen) || null, pickB = pb?.options?.find(o => o.chosen) || null;
    // a veteran never costs careers (every plan without her is still there): the two plans are fitted to their own pools,
    // so a rise is the fit's error, read as no change
    let v0a = wOf(pa); if (v0a != null && v0b != null && v0a > v0b) v0a = v0b;
    // the planner's next career keeps its trainee and parents (her legacy index is after them, so they keep theirs)
    const samePick = !!(pickA && pickB && keyOf(pickA) === keyOf(pickB));
    simulation.r = m.r; simulation.full = full; simulation.v0 = [v0b, v0a]; simulation.pickA = pickA; simulation.samePick = samePick;
    simulation.herParent = !!pickA?.parents.includes(her); simulation.state = 'done';
  } catch (e) { if (id !== simulationId || source !== myId) return; simulation.error = e.message; simulation.state = 'error'; }
  renderSimulation();
}
function addDraft() {
  if (searching || !simulation.draft) return;
  const d = structuredClone(simulation.draft); delete d.apt; delete d.bought;
  const v = newVet(d.chara, d.sparks);
  Object.assign(v, d, { src: 'entered', borrowed: false });
  plan.trainee = null; plan.parents = [null, null];
  $('v-msg').textContent = `Added ${umaName(v.chara)} to your veterans. The planner now picks the next career from all of them.`;
  changed();
}
function simulationBody() {
  const box = sec('simulation', 'Veteran analysis'); box.id = 'simulation';
  const d = simulation.draft, r = shown(); if (!d || !r) return box;
  const busy = simulation.state === 'busy';
  // her card, as the game's Umamusume Details screen (details.js; she starts as a random result of this career):
  // aptitudes at the end of this result, Skills / Inspiration (her sparks, editable) / Career Info (her G1s, the wins editor)
  const wb = btn(`${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`, 'vwins', () => openDraftWins(d, r));
  wb.id = 'sim-wins'; wb.dataset.k = 'sim:wins'; wb.setAttribute('aria-haspopup', 'dialog');
  box.appendChild(detailsEl({ d, outfit: r.trainee.outfits[0]?.label || null, apt: d.apt || r.aptitudes?.start || null,
    races: scheduledRaces(r), raceName: (ix) => CAREER.races[ix] || 'G1 ' + ix, onEdit: editDraft, winsBtn: wb, targets: r.targets,
    deck: r.deck, legacies: r.parents.map(i => ({ v: vets[i], label: '#' + (i + 1), open: () => openNode({ kind: 'v', id: vets[i].id }) })),
    parentLabel: (j) => '#' + (r.parents[j] + 1) }));
  const side = el('div', 'simcol');
  const actions = el('div', 'row');
  // Compare and a roll stay usable while a comparison runs: either one restarts it (not during the main search, whose
  // result replaces the career on show)
  const go = btn('Compare', 'primary', compare); go.id = 'sim-compare'; go.dataset.k = 'sim:compare'; go.disabled = searching;
  const roll = btn('Reroll', '', rollDraft); roll.id = 'sim-roll'; roll.dataset.k = 'sim:roll'; roll.disabled = searching;
  const add = btn('Add to veterans', '', addDraft); add.id = 'sim-add'; add.disabled = searching;
  actions.append(go, roll, add);
  const mid = el('div', 'udmid'); mid.append(oddsBtn(r), actions);
  box.querySelector('.ud .udtop')?.insertBefore(mid, box.querySelector('.ud .udname'));
  const result = el('div', 'simresult' + (simulation.stale ? ' stale' : '')); result.id = 'sim-result'; result.setAttribute('role', 'status');
  // the panel beside her card: a mini header, and before any comparison a line saying what goes here
  result.appendChild(el('div', 'simhdr', 'Comparison'));
  if (!busy && !simulation.error && !simulation.r) result.appendChild(el('p', 'simidle', 'Compare/reroll to see if she improves your loop.'));
  if (busy) { const p = el('p', 'runnote'); p.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Comparing: what she changes for the next career…</span>'; result.appendChild(p); }
  else if (simulation.error) result.appendChild(el('p', 'note', simulation.error));
  else if (simulation.r) {
    if (simulation.stale) result.appendChild(el('p', 'note', 'You changed her since: press Compare again.'));
    const before = simulation.baseline, n = state.targets.length, [v0b, v0a] = simulation.v0 || [null, null];
    const next = simulation.r.results[0];
    const saves = v0b != null && v0a != null ? v0b - v0a : null;
    // the verdict in one line, then the two figures it rests on (now -> with her), then the best career with her
    const verdict = el('p', 'simverdict'); verdict.id = 'sim-verdict';
    const looped = simulation.full && d.parents.length === 2 && d.parents.every(isFull);
    // what she does for the loop if you enter her: one short line
    const head = looped ? 'Fully looped parent'
      : simulation.full ? `${n}/${n}: step 1 done`
      : saves != null && saves >= 0.5 ? `Saves ${fmtN(saves)} careers`
      : simulation.herParent ? 'Parent of the next career'
      : next.p_all > before.p_all * (1 + 1e-9) ? 'Better odds, no careers saved'
      : 'No careers saved';
    verdict.append(el('strong', '', head));
    result.appendChild(verdict);
    const figs = el('div', 'simodds');
    // a figure turns green when she makes it better, red when worse (fewer careers, higher odds are better)
    const fig = (label, a, b, dir, title) => { const c = el('div', 'tc'); c.title = title; const v = el('b', 'tv ' + dir, `${a} → ${b}`); c.append(el('span', 'tl', label), v); figs.appendChild(c); };
    const way = (x, y, lowGood) => { if (x == null || y == null) return 'same'; const g = lowGood ? x - y : y - x;   // the gain
      return Math.abs(g) < (lowGood ? 0.5 : 1e-9 * x) ? 'same' : g > 0 ? 'up' : 'down'; };
    fig('Careers to loop', v0b != null ? fmtN(v0b) : '?', v0a != null ? fmtN(v0a) : '?', way(v0b, v0a, true),
      'Expected careers to a fully looped parent once the planner\'s next career runs: without her → with her' + (simulation.pickA ? `. With her the planner's next career is ${simulation.pickA.trainee.name} (${simulation.pickA.parents.map(i => '#' + (i + 1)).join(' + ')}).` : '.'));
    fig('Odds with her as a parent', fmtIn(before.p_all), fmtIn(next.p_all), way(before.p_all, next.p_all, false), `All ${n}: today's best career → the best career with her as a parent (${pct(before.p_all)} → ${pct(next.p_all)}; ${n - 1} or more: ${fmtIn(before.p_minus1)} → ${fmtIn(next.p_minus1)})`);
    result.appendChild(figs);
    const nx = el('div', 'nextc'); nx.append(el('span', 'tl', 'Best career with her'),
      facesEl(next.trainee.id, next.trainee.outfits[0]?.id, next.parents.map(i => parentFace(i, d)), next.trainee.outfits.map(o => o.label).join(' / ') || '(any outfit)'));
    result.appendChild(nx);
    const dd = el('details', 'nextdeck'); dd.appendChild(el('summary', '', 'Show deck')); dd.appendChild(deckEl(next, 'Deck'));
    result.appendChild(dd);
  }
  side.appendChild(result); box.appendChild(side);
  return box;
}
function renderSimulation() {
  const ae = document.activeElement, k = ae?.dataset?.k, top = $('sim-top');
  if (top) { top.replaceChildren(shown() ? simulationBody() : ''); fitSparks(top); }
  if (k && k.startsWith('sim:') && !ae.isConnected) document.querySelector(`[data-k="${CSS.escape(k)}"]`)?.focus();
}
// the options the planner weighed (only what it picks: the places left open), with "Use this": per option its odds now
// and, once the plan is made, the expected careers to land; before that, the K best careers by odds
function pickWhat(r) {
  const what = el('div', 'altw');
  if (!plan.trainee) { what.append(umaAv(r.trainee.id, r.trainee.outfits[0]?.id, 32), ' ' + r.trainee.name); }
  const open = r.parents.filter(i => !plan.parents.includes(vets[i].id));
  if (open.length) { if (!plan.trainee) what.append(', parents '); else what.append('Parents ');
    open.forEach((i, j) => { if (j) what.append(' + '); what.append(umaAv(vets[i].chara, vets[i].outfit, 24), ` #${i + 1} ${umaName(vets[i].chara)}`); }); }
  return what;
}
function useBtn(r) {
  return btn('Use this', '', () => {
    plan.trainee = r.trainee.id; const f = plan.parents.slice();
    const fr = r.parents.filter(i => !f.includes(vets[i].id)); for (let j = 0; j < 2; j++) if (!f[j]) f[j] = vets[fr.shift()].id;
    plan.parents = f; changed(); });
}
const OPT_SHOW = 8;
function optionsEl() {
  const box = sec('alts', 'Options');
  // (today's best by odds outside the places you chose is in the plan's output, not offered here)
  // with the plan: the 8 highest savings (every uma is scored), the best odds always, the rest summed up below
  const P = planS.state === 'done' ? planS.r : null, on = shown();
  const list = P ? P.options.filter((o, i) => i < OPT_SHOW || o.best_odds || o.chosen) : last.r.results;
  const tb = el('table', 'cartab optab');
  tb.innerHTML = '<thead><tr><th scope="col">Career</th><th scope="col">This career</th><th scope="col">Saves per run</th><th scope="col"><span class="sr">Choose</span></th></tr></thead><tbody></tbody>';
  // "Saves per run": what it means, and why the best odds aren't the loop's best, with the plan behind the numbers
  if (P) { const q = el('button', 'btn savewhy', 'What do these mean?'); q.type = 'button'; q.id = 'save-why'; q.dataset.k = 'save-why'; q.setAttribute('aria-haspopup', 'dialog'); q.onclick = () => openSaveWhy(P); box.appendChild(q); }
  for (const r of list) {
    const cur = on && keyOf(r) === keyOf(on), row = el('tr', cur ? 'on' : ''), c0 = el('th'); c0.scope = 'row';
    c0.appendChild(pickWhat(r));
    const tags = el('span', 'otags');
    if (P && r.chosen) tags.appendChild(el('span', 'otag pick', 'planner\'s pick'));
    if (P && r.near) tags.appendChild(el('span', 'otag', 'about the same'));
    if (P && r.best_odds) tags.appendChild(el('span', 'otag', 'best odds now'));
    if (tags.childElementCount) c0.appendChild(tags);
    const sv = el('td', 'orole', P ? (scoreOf(r) == null ? '–' : netText(scoreOf(r))) : planS.state === 'busy' ? 'planning…' : '–');
    if (P && r.index != null) { sv.title = `${roleText(r)}` + (r.lift > 0 ? `: ${pct(r.lift)} of her rolls make the plan cheaper` : '') + (r.two_step != null ? `; ${netText(r.two_step)} from the next career her best rolls open` : '') + (r.near ? '; about the same as the pick' : ''); }
    row.append(c0, el('td', '', fmtIn(r.p_all)), sv);
    const act = el('td');
    act.appendChild(cur ? el('span', 'hint', 'shown above') : useBtn(r));
    row.appendChild(act);
    tb.tBodies[0].appendChild(row);
  }
  const wrap = el('div', 'optwrap'); wrap.appendChild(tb); box.appendChild(wrap);
  const rest = P ? P.options.filter(o => !list.includes(o) && scoreOf(o) != null) : [];
  return box;
}
// "Saves per run" explained, in plain words, with the plan the numbers come from
let saveDlg = null;
function openSaveWhy(P) {
  if (!saveDlg) { saveDlg = el('dialog', 'gmdlg'); saveDlg.id = 'save-dlg'; saveDlg.setAttribute('aria-labelledby', 'save-title');
    saveDlg.addEventListener('close', () => $('save-why')?.focus()); document.body.appendChild(saveDlg); }
  saveDlg.textContent = ''; const n = state.targets.length;
  const h = el('h3', 'gmdhdr', 'Saves per run'); h.id = 'save-title'; saveDlg.appendChild(h);
  const body = el('div', 'gmdbody savebody');
  body.append(el('p', '', `How many careers running her takes off your road to a fully looped parent (an ${n}/${n} whose parents are both ${n}/${n}), averaged over everything she could roll, minus the run itself. +1.1 means she gets you 1.1 careers closer than the run costs you. Below zero, she costs more than she brings.`),
    el('div', 'gmdsub', 'Why not just run the best odds?'),
    el('p', '', `The odds column only says who is most likely to land all ${n} in this one run. A fully looped parent needs ${n}/${n}s of two different umas, then a third career off both of them, and every veteran you make becomes a parent for the runs after it. So a run with slightly worse odds can be the better move when what she leaves behind makes the next runs cheaper. The planner counts both: her own shot, and what her veteran does for the rest.`),
    el('div', 'gmdsub', 'The plan right now'));
  { const pl = P.v0_plan;
    const n = state.targets.length;
    body.appendChild(el('p', 'sub oloop', P.v0 != null && pl ? `Expected careers to a fully looped parent: about ${fmtN(P.v0)} (${n}/${n}s of ${pl.a} and ${pl.b}, then ${pl.c} from both).` : 'Expected careers to a fully looped parent: no plan found.'));
    if (P.members?.length) body.appendChild(el('p', 'sub omembers', P.members.map(m => m.real ? `${m.trainee}: your ${n}/${n}, no careers.`
      : (m.have8 ? `${m.trainee}: a fresh ${n}/${n} (yours is the poorer parent), ` : `${m.trainee}: `) + (m.builds > 0
        ? `${fmtIn(m.p)} now; building pays for about ${m.builds} more lifts (to about ${fmtIn(m.p_switch)}), then run her: about ${fmtN(m.careers)} careers.`
        : `${fmtIn(m.p)}, run her: about ${fmtN(m.careers)} careers.`)).join(' '))); }
  saveDlg.appendChild(body);
  const f = el('div', 'gmdfoot'), c = el('button', 'btn gmclose', 'Close'); c.type = 'button'; c.onclick = () => saveDlg.close(); f.appendChild(c); saveDlg.appendChild(f);
  saveDlg.showModal(); c.focus();
}
function renderResults() {
  const box = $('results'); box.textContent = '';
  const r = shown(); if (!r) { renderSimulation(); return; }
  const top = el('div'); top.id = 'sim-top';
  box.append(top, targetsEl(r));
  const open = !plan.trainee || plan.parents.some(p => !p);
  if (open && ((planS.state === 'done' ? planS.r.options : last.r.results).length > 1)) box.appendChild(optionsEl());
  renderSimulation(); markOwn();
}

// ---------- the veterans (pool) ----------
function vetCardEl(v, i) {
  const card = el('article', 'vet' + (v.borrowed ? ' bor' : '')); card.id = 'vet-' + i;
  const top = el('div', 'vtop');
  top.appendChild(el('span', 'vno', '#' + (i + 1)));
  const seg = el('div', 'seg sm'); seg.setAttribute('role', 'group'); seg.setAttribute('aria-label', `Veteran #${i + 1}: yours or borrowed`);
  for (const [b, l] of [[false, 'Yours'], [true, 'Borrowed']]) {
    const x = el('button', '', l); x.type = 'button'; x.setAttribute('aria-pressed', String(v.borrowed === b));
    x.onclick = () => { if (v.borrowed !== b) { v.borrowed = b; changed(); } }; seg.appendChild(x);
  }
  top.appendChild(seg);
  const k = plan.parents.indexOf(v.id);
  if (k >= 0) top.appendChild(el('span', 'vin', `Parent ${k + 1}`));
  top.appendChild(btn('Remove', 'vdel', () => {
    const index = idxOf(v.id); if (index < 0) return;
    removeVet(index);
    $('v-msg').textContent = `Removed ${umaName(v.chara)} from your veterans.`;
  })).setAttribute('aria-label', `Remove veteran #${i + 1}`);
  card.appendChild(top);
  // her portrait, name, targets and G1 wins, her parents' faces; a click opens her popup (sparks editable in place)
  const tg = state.targets.map(t => t.group), have = tg.filter(g => (v.sparks || {})[g]).length;
  const t = el('button', 'vtile'); t.type = 'button'; t.dataset.k = 'node:v' + v.id; t.setAttribute('aria-haspopup', 'dialog');
  t.setAttribute('aria-label', `Veteran #${i + 1}: ${umaName(v.chara)}, ${have} of ${tg.length} target skills. Her sparks`);
  const nm = el('span', 'tnm'); nm.append(el('span', 'nm', umaName(v.chara)), el('span', 'sub', `${have}/${tg.length} targets · ${v.wins.length ? v.wins.length : 'no'} G1 win${v.wins.length === 1 ? '' : 's'}`));
  const ps = el('span', 'vtps'); for (const g of v.parents) ps.appendChild(umaAv(g.chara || 0, g.outfit, 26));
  t.append(umaAv(v.chara, v.outfit, 48), nm, ps); t.onclick = () => openNode({ kind: 'v', id: v.id });
  card.appendChild(t);
  return card;
}
// your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog
function renderPool() {
  const box = $('vets'); box.textContent = '';
  if (nodeDlg?.open && nodeOpen?.kind === 'v') fillNode();
  vets.forEach((v, i) => box.appendChild(vetCardEl(v, i)));
  if (!vets.length) box.innerHTML = '<p class="hint">No veterans yet. Import an umadump file, or make one with Choose parent.</p>';
  const nb = vets.filter(v => v.borrowed).length;
  $('v-count').textContent = vets.length ? `${vets.length} veteran${vets.length > 1 ? 's' : ''}` + (nb ? `, ${nb} borrowed` : '') + '.' : '';
}
// redraw everything, keeping the keyboard focus on the same control (every edit redraws)
function renderAll() {
  const k = document.activeElement && document.activeElement.dataset ? document.activeElement.dataset.k : null;
  renderTree(); renderResults(); renderPool();
  if (k) { const e = document.querySelector(`[data-k="${CSS.escape(k)}"]`); if (e) e.focus(); }
}

// ---------- settings ----------
function initSettings() {
  $('era-global').setAttribute('aria-pressed', String(ERA === 'global')); $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
  const setEra = (e) => { if (e === ERA) return; eraScenario(e); SAVED.era = e; writeSaved(); persist(); location.reload(); };
  $('era-global').onclick = () => setEra('global'); $('era-gm').onclick = () => setEra('gm');
  initScenario(); initRoster(); initCards(); initTargets();
  renderRoster(); renderTargets();
  initSetup(); initFold();
  // a settings change (targets, umas, cards, scenario): the tree shows the new targets, and the search runs again
  onSave(() => { renderAll(); schedule(); paintFold(); });
}
// the setup panel folds to its ribbon (click it) and a one-line summary; the choice is kept in this browser
let folded = (() => { try { return localStorage.getItem('llp:setup-folded') === '1'; } catch { return false; } })();
function paintFold() {
  const bar = document.querySelector('.rbbar'), b = $('set-fold'), sum = $('set-sum');
  bar.classList.toggle('folded', folded); b.setAttribute('aria-expanded', String(!folded)); sum.hidden = !folded;
  b.title = folded ? 'Show the career setup' : 'Fold the career setup';
  const rank = state.rank === '0.20,0.70,0.10' ? 'SS' : 'B–S+';
  // one small tag per setting (the words as they read; the whole line for the screen reader)
  const parts = [ERA === 'gm' ? 'JP, Grand Masters' : 'Global', scenarioOf(state.scen).name, `Rank ${rank}`, `${state.targets.length} target skills`];
  sum.setAttribute('aria-label', parts.join(' · ')); sum.replaceChildren(...parts.map(p => { const t = el('span', 'sumtag', p); t.setAttribute('aria-hidden', 'true'); return t; }));
}
function initFold() { $('set-fold').onclick = () => { folded = !folded; try { localStorage.setItem('llp:setup-folded', folded ? '1' : '0'); } catch {} paintFold(); }; paintFold(); }

// ---------- start ----------
// Export: the veterans as the page keeps them, the plan, and the solver job the planner sends for them (to replay a
// search outside the page); Import takes the file back
const EXPORT_APP = 'uma-legacy-veterans';
function exportVets() {
  let job = null; try { job = buildJob(); } catch {}
  const data = { app: EXPORT_APP, version: 1, era: ERA, exported: new Date().toISOString(), veterans: vets, plan, job };
  $('vx-text').value = JSON.stringify(data); $('vx-msg').textContent = ''; $('vx-dlg').showModal();
}
// Copy: the clipboard API, else select the text and the old copy command, else leave it selected for Ctrl+C
function copyExport() {
  const t = $('vx-text'), msg = (m) => { $('vx-msg').textContent = m; };
  const fallback = (why) => { t.focus(); t.select();
    let ok = false; try { ok = document.execCommand('copy'); } catch {}
    msg(ok ? 'Copied.' : `Could not copy${why ? ` (${why})` : ''}. The text is selected: press Ctrl+C (or ⌘C).`); };
  if (!navigator.clipboard?.writeText) { fallback('no clipboard access here'); return; }
  navigator.clipboard.writeText(t.value).then(() => msg('Copied.'), (e) => fallback(e?.message || 'clipboard refused'));
}
function downloadExport() {
  const url = URL.createObjectURL(new Blob([$('vx-text').value], { type: 'application/json' }));
  const a = document.createElement('a'); a.href = url; a.download = `uma-legacy-veterans-${ERA}.json`; document.body.appendChild(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  $('vx-msg').textContent = 'Downloaded.';
}
function ownExport(text) { try { const o = JSON.parse(text); return o && o.app === EXPORT_APP && Array.isArray(o.veterans) ? o : null; } catch { return null; } }
function importText(text) {
  const own = ownExport(text);
  if (own) { importVets(own.veterans.map(v => ({ pinks: {}, wins: [], ...v, src: 'import' })));
    $('v-msg').textContent = `Imported ${own.veterans.length} veteran${own.veterans.length === 1 ? '' : 's'} from an export of this page.`; return true; }
  const tree = parseTree(text);
  if (tree) { if (!tree.length) { $('v-msg').textContent = 'No veteran in that uma.moe tree.'; return false; }
    addVets(tree); $('v-msg').textContent = `Added ${tree.map(v => umaName(v.chara)).join(', ')} from an uma.moe tree, with her parents, as #${vets.length - tree.length + 1}${tree.length > 1 ? `-#${vets.length}` : ''} (marked yours: switch to Borrowed if she is a friend's).`; return true; }
  try { const list = parseExport(text); importVets(list); const nb = list.filter(v => v.borrowed).length;
    $('v-msg').textContent = `Imported ${list.length} veteran${list.length > 1 ? 's' : ''}${nb ? ` (${nb} borrowed)` : ''}.`; return true; }
  catch (e) { $('v-msg').textContent = e.message; return false; }
}
document.addEventListener('click', (e) => { if (e.target instanceof HTMLDialogElement && e.target.open) e.target.close(); });   // a backdrop click closes
loadData().then(() => {
  initSettings(); initPopover();
  // A result names parents by their place in the veteran list: adding, removing or importing veterans changes the
  // places, so the result on screen goes; an edit to a veteran keeps it on screen until the new search ends.
  const vetsSig = () => vets.map(v => v.id).join(',');
  let sig = vetsSig();
  onVetsChange(() => {
    if (vetsSig() !== sig) { sig = vetsSig(); last = null; simulation = { state: 'none' }; planS = { state: 'none' }; }
    schedule(); renderAll();
  });
  for (const x of ['vets', 'new', 'auto']) $('vp-tab-' + x).onclick = () => vpTab(x);
  $('vp-filter').oninput = drawNew;
  $('vp-auto-go').onclick = () => { plan.parents[pickK] = null; $('vpick-dlg').close(); changed(); };
  $('v-file').onchange = async (e) => { const f = e.target.files[0]; if (f) importText(await f.text()); e.target.value = ''; };
  $('v-paste').onclick = () => { $('paste-text').value = ''; $('paste-dlg').showModal(); };
  $('v-export').onclick = exportVets; $('vx-copy').onclick = copyExport; $('vx-dl').onclick = downloadExport;
  $('paste-load').onclick = () => { if (importText($('paste-text').value)) $('paste-dlg').close(); };
  $('v-toggle').onclick = () => $('set-vets').showModal();
  $('v-clear').onclick = () => { if (vets.length && confirm(`Remove all ${vets.length} veterans?`)) clearVets(); };
  renderAll(); search();
}, (e) => { status = { kind: 'error', text: e.message }; paint(); });
