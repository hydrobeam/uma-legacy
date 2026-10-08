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
import { initScenario, eraScenario } from './scenario.js';
import { initRoster, renderRoster, noOutfit } from './roster.js';
import { initCards, cardTitle } from './cards.js';
import { initPopover, markOwn } from './popover.js';
import { openSched } from './agenda.js';
import { initTargets, renderTargets } from './targets.js';
import { vets, plan, onVetsChange, changed, persist, vetById, umaName, skillName, pinkName, sparksEl, winsBtn, changeUma, askSkills,
  newVet, importVets, removeVet, clearVets, legaciesJob, openUmaPicker, drawUmas, openWins } from './vets.js';
import { parseExport } from './umadump.js';

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
  const job = { cmd: 'career', targets: jobTargets(), exclude, exclude_outfits, extra: EXTRA, owned_cards: ownedCards(), borrow: state.borrow,
    star_dist: state.rank.split(',').map(Number), scenario: state.scen || 'unity', ...rcardsOff(), legacies: legaciesJob(), trainees: plan.trainee ? [plan.trainee] : [],
    fix: plan.parents.filter(Boolean).map(idxOf), per_trainee: false, top: all ? 1 : ROLL_K };
  // the goal's progression, when every place is the planner's: after an 8/8 veteran the career uses her (a second 8/8 of
  // another uma), after two of different umas it uses both (its 8/8 would be a fully looped parent)
  const g = goalStage();
  if (g.fix && !plan.trainee && !plan.parents[0] && !plan.parents[1]) { job.fix = g.fix; job.top = 1; }
  return job;
}
// ---------- the goal: a fully looped parent ----------
// A veteran with a spark of every target ("8/8") whose two parents are 8/8 too: as a parent she brings three full
// legacies. A career's P(all) is exactly its chance of an 8/8 veteran, so the steps are picked by it: with no 8/8, the
// plan (building the loop); with one, the best career with her as a parent; with 8/8s of two umas, the best career with
// both as parents (the pair with the most stars); once a fully looped parent exists, the plan again (more stars).
const isFull = (nd) => !!nd && state.targets.length > 0 && state.targets.every(t => (nd.sparks || {})[t.group] > 0);
const starSum = (nd) => state.targets.reduce((a, t) => a + ((nd.sparks || {})[t.group] || 0), 0);
function goalStage() {
  const looped = vets.findIndex(v => isFull(v) && v.parents.length === 2 && v.parents.every(isFull));
  if (looped >= 0) return { step: 'looped', at: looped };
  const full = vets.map((v, i) => ({ v, i })).filter(x => isFull(x.v)).sort((a, b) => starSum(b.v) - starSum(a.v));
  if (!full.length) return { step: 'build' };
  // the best pair of 8/8s: different umas, at most one borrowed, most stars
  let pair = null;
  for (let a = 0; a < full.length && !pair; a++) for (let b = a + 1; b < full.length; b++) {
    const x = full[a].v, y = full[b].v;
    if (x.chara !== y.chara && !(x.borrowed && y.borrowed)) { pair = [full[a].i, full[b].i]; break; }
  }
  if (pair) return { step: 'shot', fix: pair };
  return { step: 'second', fix: [full[0].i], at: full[0].i };
}
function goalEl() {
  const g = goalStage(), box = el('div', 'goalnote'); box.id = 'goal-note';
  const name = (i) => `#${i + 1} ${umaName(vets[i].chara)}`;
  const n = state.targets.length;
  const open = !plan.trainee && !plan.parents[0] && !plan.parents[1];
  const txt = g.step === 'looped' ? `You have a fully looped parent: ${name(g.at)} (${n}/${n} with ${n}/${n} parents). The planner keeps going for more stars.`
    : g.step === 'build' ? `Goal: a fully looped parent (${n}/${n} with ${n}/${n} parents). Step 1: no ${n}/${n} veteran yet, so the planner builds the lineage.`
    : g.step === 'second' ? `Goal: a fully looped parent. Step 2: you have one ${n}/${n} (${name(g.at)}); ${open ? 'this career uses her' : 'a career using her'} as a parent to get a ${n}/${n} of another uma.`
    : `Goal: a fully looped parent. Step 3: you have ${n}/${n}s of two umas (${g.fix.map(name).join(' and ')}); ${open ? 'this career uses both' : 'a career using both'} as parents: its ${n}/${n} would be a fully looped parent.`;
  box.append(el('strong', '', g.step === 'looped' ? 'Fully looped. ' : `Step ${{ build: 1, second: 2, shot: 3 }[g.step]} of 3. `), txt);
  return box;
}
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
      const cert = m.r.proven_top ? 'Checked on exact odds: nothing else does better.' : 'The exact check ran out of time: the order may not be final.';
      const n = m.r.candidates;
      status = { kind: 'done', text: (n > 1 ? `Weighed ${n.toLocaleString()} careers in ${(m.ms / 1000).toFixed(1)} s. ` : `Scored in ${(m.ms / 1000).toFixed(1)} s. `) + cert };
      const best = m.r.results[0], r = best;
      simulation = r ? { state: 'ready', draft: draftOf(r), r: null, baseline: null, error: null, forKey: keyOf(r) } : { state: 'none' };
      const gs = goalStage(), stepped = !!gs.fix && !plan.trainee && !plan.parents[0] && !plan.parents[1];
      // steps 2 and 3 pick by P(all), and a trainee you chose leaves the plan nothing to pick: no plan
      planS = r && !stepped && !plan.trainee ? { state: 'busy', r: null, t0: performance.now() } : { state: 'none' };
      if (best && !job.trainees.length && !job.fix.length) baselineCache = { key: baseKey(job), result: best };
    }
  } catch (e) { if (id !== myId) return; status = { kind: 'error', text: e.message }; last = null; simulation = { state: 'none' }; }
  searching = false; clearInterval(tick); renderAll(); paint();
  if (planS.state === 'busy') runPlan();
}
// The plan: the same job with `next` (solver: sim/plan.rs `stale_rule`, exact, no simulation): candidates are the loop
// planner's loop and the umas in the best career's lineage, each with her best career; one whose best parents are new to
// her (no veteran of hers has them as parents) trains once, the best odds among those; with none, the best career.
// Its pick replaces the quick one on show. It
// shares the one worker with the comparison: a comparison cancelled while both were queued takes it down too, and it
// runs again (planId tells the run that counts).
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
function whyText(r) {
  const P = planS.state === 'done' ? planS.r : null, o = planRow(r); if (!P || !o) return '';
  const pick = P.options.find(x => x.chosen), n = state.targets.length;
  if (P.mode === 'done') return o.chosen ? `The planner's pick: the best odds (you already have a fully looped parent).` : '';
  if (scoreOf(o) == null) return '';
  if (!o.chosen) return `Not the planner's pick: ${netText(scoreOf(o))} careers per run toward a fully looped parent, against ${pick ? netText(scoreOf(pick)) : '–'} for the pick${o.near ? ' (about the same)' : ''}.`;
  const own = o.stage_cost != null ? `her own ${n}/${n} (1 in ${Math.round(1 / o.p_all).toLocaleString()}) counts for her step, and ` : '';
  return `The planner's pick: it brings a fully looped parent the closest, ${netText(scoreOf(o))} careers per run: ${own}${pct(o.lift)} of her rolls make the plan cheaper (a better parent for a step still to run). Repeat it until a roll changes the plan.`;
}
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
function traineeEl(r) {
  const id = plan.trainee || (r && r.trainee.id);
  const n = pnode('trainee', 0, plan.trainee ? 'Trainee' : 'Trainee · planner\'s pick', !plan.trainee);
  // the affinity she holds from her six legacies: parents' and grandparents' parts
  let parts = null;
  if (r && r.trainee.id === id) {
    const sum = (k) => r.slots.filter(x => x.kind === k).reduce((a, x) => a + x.aff, 0), pa = sum('parent'), ga = sum('grandparent');
    const a = el('span', 'aff tot', `Affinity ${Math.round(pa + ga)}`); a.title = `Parents ${Math.round(pa)} + grandparents ${Math.round(ga)}`;
    n.querySelector('.pnh').appendChild(a);
    parts = el('div', 'tsum', `Parents ${Math.round(pa)} · grandparents ${Math.round(ga)}`);
  }
  const who = el('div', 'twho');
  if (id) {
    who.append(umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 96));
    const nm = el('div', 'tnm'); nm.append(el('div', 'nm', umaName(id)));
    if (r && r.trainee.id === id) nm.append(el('div', 'sub', r.trainee.outfits.length ? r.trainee.outfits.map(o => o.label).join(' or ') : 'any outfit'));
    if (parts) nm.append(parts);
    who.append(nm);
  } else who.append(el('div', 'tempty', 'The planner tries every uma you own.'));
  n.appendChild(who);
  const acts = el('div', 'row tacts');
  acts.appendChild(btn(plan.trainee ? 'Change trainee' : 'Choose trainee', 'primary', () => openUmaPicker('Trainee: which uma?', (u) => { plan.trainee = u; changed(); }, owned)));
  if (plan.trainee) acts.appendChild(btn('Let the planner pick', '', () => { plan.trainee = null; changed(); }));
  n.appendChild(acts);
  return n;
}
function affEl(a) { return a === null ? null : el('span', 'aff', `Affinity ${Math.round(a)}`); }
function familyEl(k, r) {
  const fam = el('div', 'pf');
  const at = parentAt(k, r), gate = k * 3 + 1;
  const n = pnode('parent', gate, `Parent ${k + 1}` + (at && at.pick ? ' · planner\'s pick' : '') + (at && at.v.borrowed ? ' · borrowed' : ''), at && at.pick);
  if (at && at.v.borrowed) n.classList.add('bor');
  if (at) {
    const v = at.v, i = idxOf(v.id), a = affOf(r, i);
    if (a !== null) n.querySelector('.pnh').appendChild(affEl(a));
    const who = el('div', 'twho');
    const pb = el('button', 'facebtn'); pb.type = 'button'; pb.append(umaAv(v.chara, v.outfit, 64)); pb.title = 'Choose another parent'; pb.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Choose another parent`);
    pb.onclick = () => openParentPicker(k);
    const nm = el('div', 'tnm'); nm.append(el('div', 'nm', umaName(v.chara)), el('div', 'sub', `Veteran #${i + 1}` + (v.src === 'hand' ? ', made here' : '')));
    who.append(pb, nm, winsBtn(v, umaName(v.chara), 'p' + k));
    n.append(who, sparksEl(v, 'p' + k));
  } else n.appendChild(el('p', 'tempty', vets.length >= 2 ? 'The planner picks from your veterans.' : 'No parent yet.'));
  const acts = el('div', 'row tacts');
  acts.appendChild(btn(plan.parents[k] ? 'Change parent' : 'Choose parent', 'primary big', () => openParentPicker(k)));
  if (plan.parents[k]) acts.appendChild(btn('Let the planner pick', '', () => { plan.parents[k] = null; changed(); }));
  n.appendChild(acts);
  fam.appendChild(n);
  // her own parents: the trainee's grandparents (who they are and what they carry is part of the veteran)
  if (at) {
    const gk = el('div', 'pk');
    at.v.parents.forEach((g, j) => {
      const gn = pnode('gp', gate + 1 + j, 'Grandparent', false);
      const a = affOf(r, idxOf(at.v.id), j); if (a !== null) gn.querySelector('.pnh').appendChild(affEl(a));
      const who = el('div', 'twho');
      const b = el('button', 'facebtn'); b.type = 'button'; b.append(umaAv(g.chara || 0, g.outfit, 40));
      b.title = 'Change who she is'; b.setAttribute('aria-label', `Grandparent: ${umaName(g.chara)}. Change who she is`);
      b.onclick = () => changeUma(g, 'Grandparent');
      who.append(b, el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'), winsBtn(g, umaName(g.chara), `p${k}g${j}`));
      gn.append(who, sparksEl(g, `p${k}g${j}`));
      gk.appendChild(gn);
    });
    fam.appendChild(gk);
  }
  return fam;
}
function renderTree() {
  const r = shown(), box = $('tree'); box.textContent = '';
  const t = el('div', 'pt'); t.appendChild(traineeEl(r)); box.appendChild(t);
  const k = el('div', 'pk'); k.append(familyEl(0, r), familyEl(1, r)); box.appendChild(k);
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
function oddsEl(r) {
  const n = state.targets.length, h = el('div', 'lhead');
  h.innerHTML = `<span class="odds"><b></b> runs to land all ${n}</span><span class="sub">${n - 1} of ${n} or better: <span class="num"></span> · <span class="num"></span> of ${n} expected</span>`;
  h.querySelector('.odds b').textContent = fmtIn(r.p_all);
  const nums = h.querySelectorAll('.sub .num'); nums[0].textContent = fmtIn(r.p_minus1); nums[1].textContent = r.expected.toFixed(2);
  // why the planner shows this career (once the plan is made)
  const why = whyText(r);
  if (why) { const x = el('span', 'sub expc', why); x.id = 'pick-why'; h.appendChild(x); }
  return h;
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
    const row = el('tr'), c0 = el('th'); c0.scope = 'row'; const ic = skillIcon(t.group, t.gold); if (ic) c0.appendChild(ic); c0.append(tName(t.group));
    row.appendChild(c0);
    for (const v of [t.gold ? 'scenario' : pct(t.q), `${t.carriers} of 6`, pct(t.p)]) row.appendChild(el('td', '', v));
    tb.tBodies[0].appendChild(row);
  }
  s.appendChild(tb);
  if (r.pinks.length) {
    const p = el('p', 'pinkres'); p.append(el('span', 'sub', 'Aptitudes passed on (at least once): '));
    for (const x of r.pinks) p.append(el('span', 'pchip', `${pinkName(x.group)} ${pct(x.q)}`), ' ');
    s.appendChild(p);
  }
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
    hint: 'The G1s she won in this career. Click a win to remove it; click an empty slot (+) for the G1s she could enter then (her schedule\'s first, with its win odds). Each one she shares with a parent adds 3 affinity.',
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
// The decision is per veteran: enter the one you got (it starts as the most likely result) and Compare runs both
// choices, running this career again or using her in the next one, on the plan's terms (expected careers to land).
const copyNode = (v) => ({ chara: v.chara, ...(v.outfit ? { outfit: v.outfit } : {}), sparks: { ...v.sparks }, pinks: { ...v.pinks }, wins: [...v.wins] });
function scheduledRaces(r) {
  const races = new Map();
  for (const w of (r.schedule || []).filter(w => w.run !== false)) {
    const ix = CAREER.races.indexOf(w.name || w.race); if (ix < 0) continue;
    const prev = races.get(ix);
    races.set(ix, { ix, name: w.name || w.race, p: 1 - (1 - (prev?.p || 0)) * (1 - w.p), turns: [...(prev?.turns || []), w.turn] });
  }
  return [...races.values()];
}
// the most likely result: the most likely set of landed targets, each at the rank's most likely star count, and the
// G1s she more likely wins than not
function likelyOutcome(r) {
  const w = state.rank.split(',').map(Number), star = w.indexOf(Math.max(...w)) + 1;
  const sparks = {};
  if (Array.isArray(r.outcome_sets)) {
    let mask = 0; r.outcome_sets.forEach((p, m) => { if (p > r.outcome_sets[mask]) mask = m; });
    state.targets.forEach((t, i) => { if (mask & (1 << i)) sparks[t.group] = star; });
  } else r.targets.forEach(t => { if (t.p >= 0.5) sparks[t.group] = star; });
  const wins = scheduledRaces(r).filter(x => x.p >= 0.5).map(x => x.ix).sort((a, b) => a - b);
  return { sparks, wins };
}
// a random result of this career: the set of landed targets drawn from its exact odds, each spark's stars from the rank,
// each G1 she runs won with its chance (your roll, so it differs every time; the default stays the most likely result)
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
  return { sparks, wins };
}
function rollDraft() { const r = shown(); if (!r || searching) return; Object.assign(simulation.draft, rolledOutcome(r), { winAt: {} }); simulation.r = null; simulation.stale = false; compare(); }
// A comparison in flight is for a veteran that is going (a roll, a reset, Compare pressed again): stop it. A wasm call
// can't be interrupted, so the worker goes (run() starts a new one). Returns whether the plan went with it.
function cancelCompare() {
  if (simulation.state !== 'busy') return false;
  simulationId++; killEngine(); simulation.state = 'ready';
  return planS.state === 'busy';
}
function draftOf(r) { return { chara: r.trainee.id, outfit: r.trainee.outfits[0]?.id, borrowed: false, ...likelyOutcome(r), pinks: {}, parents: r.parents.map(i => copyNode(vets[i])) }; }
function draftJob(v) {
  const pairs = (o) => Object.entries(o).map(([g, s]) => [+g, s]);
  const node = (v) => ({ chara: v.chara || 0, sparks: pairs(v.sparks), pinks: pairs(v.pinks), wins: [...v.wins] });
  return { ...node(v), borrowed: false, parents: v.parents.map(node) };
}
// an edit to the entered veteran: the comparison on screen is for the old one
function editDraft() { if (simulation.r) simulation.stale = true; renderSimulation(); }
function resetDraft() { const r = shown(); if (!r) return; const lost = cancelCompare();
  simulation = { ...simulation, draft: draftOf(r), r: null, stale: false, error: null, state: 'ready' }; renderSimulation(); if (lost) runPlan(); }
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
    // exact, the planner's question (no simulation): an n/n is goal step 1 done; else does the best career with her as
    // a parent beat today's best; else does she give one of the plan's candidates better parents (that candidate's best
    // career with her beats her best today, so the planner trains her once with them); else nothing changes yet
    const legs = [...base.legacies, draftJob(simulation.draft)], her = base.legacies.length;
    const m = await run({ ...base, fix: [her], legacies: legs, top: 1, compare: true });
    if (id !== simulationId || source !== myId) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    // the planner's expected careers to a fully looped parent once its best next career runs, without and with her:
    // W = V0 - (the best positive score) = 1 + E[V0 after that career] (n/n: goal step 1 done)
    const full = isFull(simulation.draft);
    let pb = null, pa = null;
    if (!full) {
      pb = planS.state === 'done' && !last.job.fix.length ? planS.r : null;
      if (!pb) { const mb = await run({ ...base, next: {}, compare: true }); if (id !== simulationId || source !== myId) return; pb = mb.r.next || null; }
      const ma = await run({ ...base, legacies: legs, next: {}, compare: true }); if (id !== simulationId || source !== myId) return; pa = ma.r.next || null;
    }
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
  const d = structuredClone(simulation.draft);
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
  // her portrait beside her sparks and G1 wins (she starts as the most likely result of this career)
  const who = el('div', 'simwho'), face = umaAv(d.chara, d.outfit, 96);
  face.setAttribute('role', 'img'); face.setAttribute('aria-label', `${umaName(d.chara)} ${r.trainee.outfits[0]?.label || '(any outfit)'}`);
  const side = el('div', 'simside');
  const controls = el('fieldset', 'simskills'); controls.setAttribute('aria-label', 'Her sparks');
  controls.appendChild(sparksEl(d, 'sim', editDraft));
  // her G1 wins: her schedule's races on the schedule window's calendar
  const wb = btn(`${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`, 'vwins', () => openDraftWins(d, r));
  wb.id = 'sim-wins'; wb.dataset.k = 'sim:wins'; wb.setAttribute('aria-haspopup', 'dialog');
  side.append(controls, wb); who.append(face, side); box.appendChild(who);
  const actions = el('div', 'row');
  // Compare and a roll stay usable while a comparison runs: either one restarts it (not during the main search, whose
  // result replaces the career on show)
  const go = btn('Compare', 'primary', compare); go.id = 'sim-compare'; go.dataset.k = 'sim:compare'; go.disabled = searching;
  const roll = btn('Roll a random result', '', rollDraft); roll.id = 'sim-roll'; roll.dataset.k = 'sim:roll'; roll.disabled = searching;
  const reset = btn('Back to the most likely result', '', resetDraft); reset.id = 'sim-reset'; reset.dataset.k = 'sim:reset';
  const add = btn('Add her to your veterans', '', addDraft); add.id = 'sim-add'; add.disabled = searching;
  actions.append(go, roll, reset, add); box.appendChild(actions);
  const result = el('div', 'simresult' + (simulation.stale ? ' stale' : '')); result.id = 'sim-result'; result.setAttribute('role', 'status');
  if (busy) { const p = el('p', 'runnote'); p.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Comparing: what she changes for the next career…</span>'; result.appendChild(p); }
  else if (simulation.error) result.appendChild(el('p', 'note', simulation.error));
  else if (simulation.r) {
    if (simulation.stale) result.appendChild(el('p', 'note', 'You changed her since: press Compare again.'));
    const before = simulation.baseline, n = state.targets.length, [v0b, v0a] = simulation.v0 || [null, null];
    const next = simulation.r.results[0];
    const saves = v0b != null && v0a != null ? v0b - v0a : null;
    const verdict = el('p', 'note'); verdict.id = 'sim-verdict';
    const looped = simulation.full && d.parents.length === 2 && d.parents.every(isFull);
    const [head, tail] = looped ? [`She's a fully looped parent (${n}/${n} with ${n}/${n} parents).`, 'That is the goal; add her to your veterans.']
      : simulation.full ? [`She's an ${n}/${n}: step 1 of the goal is done.`, `Add her to your veterans and the next career uses her as a parent, for an ${n}/${n} of another uma; the best such career is below.`]
      : saves != null && saves >= 0.5 ? [`She saves ${fmtN(saves)} careers.`, `Expected careers to a fully looped parent after the planner's next career: ${fmtN(v0b)} → ${fmtN(v0a)} with her.` + (simulation.pickA ? (simulation.samePick ? ` The planner's next career doesn't change: ${simulation.pickA.trainee.name} (${simulation.pickA.parents.map(i => '#' + (i + 1)).join(' + ')}) again, ${netText(scoreOf(simulation.pickA))} per run (a gain this small doesn't change what's best).`
        : ` With her the planner's next career is ${simulation.pickA.trainee.name} (${simulation.pickA.parents.map(i => '#' + (i + 1)).join(' + ')}), ${netText(scoreOf(simulation.pickA))} per run.`) : '')]
      // under half a career, but the planner's next career has her as a parent: she lifts its odds more than the plan's cost
      : simulation.herParent ? ['She\'s a parent of the planner\'s next career.', `${simulation.pickA.trainee.name} (${simulation.pickA.parents.map(i => '#' + (i + 1)).join(' + ')}) at ${fmtIn(simulation.pickA.p_all)} (today's best ${fmtIn(before.p_all)}). Expected careers to a fully looped parent after it: ${v0b != null ? fmtN(v0b) : 'unknown'} → ${v0a != null ? fmtN(v0a) : 'unknown'} with her, under half a career saved: the plan's cost rests mostly on the careers still to run.`]
      : next.p_all > before.p_all * (1 + 1e-9) ? ['She raises the best odds but saves under half a career.', `The best career with her as a parent is ${fmtIn(next.p_all)} (today's best ${fmtIn(before.p_all)}); expected careers to a fully looped parent stay about ${v0b != null ? fmtN(v0b) : 'unknown'}, and the planner's next career doesn't use her.`]
      : ['She changes nothing yet.', `Expected careers to a fully looped parent stay ${v0b != null ? fmtN(v0b) : 'unknown'} (the best career with her as a parent is ${fmtIn(next.p_all)}; today's best ${fmtIn(before.p_all)}); she may still matter later, as a grandparent.`];
    verdict.append(el('strong', '', head), ' ' + tail);
    result.appendChild(verdict);
    const odds = el('div', 'simodds');
    for (const [label, a, b] of [[`All ${n}: today's best → next with her`, before.p_all, next.p_all], [`At least ${n - 1}: today's best → next with her`, before.p_minus1, next.p_minus1]]) {
      const card = el('div', 'tc'); card.append(el('span', 'tl', label), el('b', 'tv', `${fmtIn(a)} → ${fmtIn(b)}`), el('span', 'hint', `${pct(a)} → ${pct(b)}`)); odds.appendChild(card);
    }
    result.appendChild(odds);
    const nx = el('div', 'nextc'); nx.append(el('span', 'tl', 'The next career with her: '),
      facesEl(next.trainee.id, next.trainee.outfits[0]?.id, next.parents.map(i => parentFace(i, d)), next.trainee.outfits.map(o => o.label).join(' / ') || '(any outfit)'));
    result.appendChild(nx);
    const dd = el('details', 'nextdeck'); dd.appendChild(el('summary', '', 'Show its deck')); dd.appendChild(deckEl(next, 'Deck of the next career'));
    result.appendChild(dd);
  }
  box.appendChild(result);
  return box;
}
function renderSimulation() {
  const ae = document.activeElement, k = ae?.dataset?.k, top = $('sim-top');
  if (top) top.replaceChildren(shown() ? simulationBody() : '');
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
  // the plan to a fully looped parent, once
  if (P) { const pl = P.v0_plan;
    const n = state.targets.length;
    box.appendChild(el('p', 'sub oloop', P.v0 != null && pl ? `Expected careers to a fully looped parent: about ${fmtN(P.v0)} (${n}/${n}s of ${pl.a} and ${pl.b}, then ${pl.c} from both).` : 'Expected careers to a fully looped parent: no plan found.'));
    if (P.members?.length) box.appendChild(el('p', 'sub omembers', P.members.map(m => m.have8 ? `${m.trainee}: ${n}/${n} done.`
      : m.p_inf && m.builds > 0 ? `${m.trainee}: ${fmtIn(m.p)} now, about ${fmtIn(m.p_inf)} with a full loop lineage; building pays for about ${m.builds} more lifts (to about ${fmtIn(m.p_switch)}), then run her: about ${fmtN(m.careers)} careers.`
      : `${m.trainee}: ${fmtIn(m.p)}, run her: about ${fmtN(m.careers)} careers.`).join(' '))); }
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
  if (rest.length) box.appendChild(el('p', 'sub orest', `${rest.length} more umas scored, ${netText(Math.min(...rest.map(scoreOf)))} to ${netText(Math.max(...rest.map(scoreOf)))} per run.`));
  if (P) box.appendChild(el('p', 'hint', `Saves per run: the careers this career brings a fully looped parent closer, on average over every result she can roll (exact), minus the run itself. The plan is the cheapest trio: 8/8s of two umas, each building her odds toward her full-loop odds while that pays (rate and step fitted to your veterans now), then the third from both. A career counts both ways: her own 8/8 for her step, and her veteran as a parent that makes a step cheaper. The planner runs the highest (near-ties: the better odds) and repeats it until a roll changes the plan. Every odds figure is exact; the expected careers are estimates.`));
  return box;
}
function renderResults() {
  const box = $('results'); box.textContent = '';
  const r = shown(); if (!r) { renderSimulation(); return; }
  box.append(goalEl(), deckEl(r, 'Deck', schedBtn(r)), oddsEl(r));
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
  if (v.src === 'entered' || v.src === 'simulated') top.appendChild(el('span', 'simtag', 'Entered here'));
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
  const node = (nd, kind, label, key) => {
    const n = el('div', 'vnode ' + kind);
    const head = el('div', 'vhead');
    const who = el('button', 'vwho'); who.type = 'button'; who.append(umaAv(nd.chara || 0, nd.outfit, kind === 'self' ? 40 : 28), el('span', 'vnm', nd.chara ? umaName(nd.chara) : 'Unknown parent'));
    who.title = `${label}: ${umaName(nd.chara)}. Click to change who she is`; who.setAttribute('aria-label', who.title);
    who.onclick = () => changeUma(nd, label);
    head.append(who, winsBtn(nd, `${label}, ${umaName(nd.chara)}`, key));
    n.append(head, sparksEl(nd, key));
    return n;
  };
  card.appendChild(node(v, 'self', `Veteran #${i + 1}`, 'v' + v.id));
  const kids = el('div', 'vkids');
  v.parents.forEach((p, j) => kids.appendChild(node(p, 'par', `Parent ${j + 1} of #${i + 1}`, `v${v.id}g${j}`)));
  card.appendChild(kids);
  return card;
}
function renderPool() {
  const box = $('vets'); box.textContent = '';
  vets.forEach((v, i) => box.appendChild(vetCardEl(v, i)));
  if (!vets.length) box.innerHTML = '<p class="hint">No veterans yet. Import an umadump file, or make one with Choose parent.</p>';
  const nb = vets.filter(v => v.borrowed).length;
  $('v-count').textContent = vets.length ? `${vets.length} veteran${vets.length > 1 ? 's' : ''}` + (nb ? `, ${nb} borrowed` : '') : '';
}
// redraw everything, keeping the keyboard focus on the same control (every edit redraws)
function renderAll() {
  const k = document.activeElement && document.activeElement.dataset ? document.activeElement.dataset.k : null;
  renderTree(); renderResults(); renderPool(); counts();
  if (k) { const e = document.querySelector(`[data-k="${CSS.escape(k)}"]`); if (e) e.focus(); }
}

// ---------- settings ----------
function counts() {
  $('o-skills-n').textContent = String(state.targets.length);
  $('c-borrow').checked = !!state.borrow;
}
function initSettings() {
  $('era-global').setAttribute('aria-pressed', String(ERA === 'global')); $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
  const setEra = (e) => { if (e === ERA) return; eraScenario(e); SAVED.era = e; writeSaved(); persist(); location.reload(); };
  $('era-global').onclick = () => setEra('global'); $('era-gm').onclick = () => setEra('gm');
  initScenario(); initRoster(); initCards(); initTargets();
  renderRoster(); renderTargets();
  $('o-skills').onclick = () => $('set-skills').showModal();
  $('o-umas').onclick = () => $('set-umas').showModal();
  $('o-cards').onclick = () => $('set-cards').showModal();
  // a settings change (targets, umas, cards, scenario): the tree shows the new targets, and the search runs again
  onSave(() => { renderAll(); schedule(); });
}

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
  try { const list = parseExport(text); importVets(list); const nb = list.filter(v => v.borrowed).length;
    $('v-msg').textContent = `Imported ${list.length} veteran${list.length > 1 ? 's' : ''}${nb ? ` (${nb} borrowed)` : ''}.`; return true; }
  catch (e) { $('v-msg').textContent = e.message; return false; }
}
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
  $('v-clear').onclick = () => { if (vets.length && confirm(`Remove all ${vets.length} veterans?`)) clearVets(); };
  renderAll(); search();
}, (e) => { status = { kind: 'error', text: e.message }; paint(); });
