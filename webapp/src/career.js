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
import { vets, plan, onVetsChange, changed, persist, vetById, umaName, skillName, pinkName, sparksEl, winsBtn, openWins, changeUma, askSkills,
  newVet, importVets, removeVet, clearVets, legaciesJob, openUmaPicker, drawUmas } from './vets.js';
import { parseExport } from './umadump.js';

const pct = (p) => p === 0 ? '0%' : (p * 100 >= 10 ? (p * 100).toFixed(0) : p * 100 >= 1 ? (p * 100).toFixed(1) : (p * 100).toFixed(2)) + '%';
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
  else n.textContent = status.text;
}
// every change: redraw now, search again after a short pause (a change during a search stops it)
function schedule() {
  clearTimeout(timer); myId++;
  if (searching || simulation.state === 'busy' || ahead.state === 'busy') killEngine();
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
  // a place left open: the K best careers by odds, and the planner's pick among them by the veterans they leave (rollout)
  const job = { cmd: 'career', targets: jobTargets(), exclude, exclude_outfits, extra: EXTRA, owned_cards: ownedCards(), borrow: state.borrow,
    star_dist: state.rank.split(',').map(Number), scenario: state.scen || 'unity', ...rcardsOff(), legacies: legaciesJob(), trainees: plan.trainee ? [plan.trainee] : [],
    fix: plan.parents.filter(Boolean).map(idxOf), per_trainee: false, top: all ? 1 : ROLL_K, ...(all ? {} : { rollout: ROLL_K }) };
  // today's best career over everything (the rollout's floor), when a place is set and it is known for these settings
  if (!all && (job.trainees.length || job.fix.length) && baselineCache?.key === baseKey(job)) job.p_now = baselineCache.result.p_all;
  return job;
}
const ROLL_K = 6;
// the job of today's best career over everything (no place set, one result, no rollout): the comparison's baseline
const baseOf = (job) => ({ ...job, exclude: UMAS.filter(u => !owned(u)).map(u => u.id), trainees: [], fix: [], per_trainee: false, top: 1, rollout: undefined, p_now: undefined });
const baseKey = (job) => JSON.stringify(baseOf(job));
// after the main result: the outcome look ahead (a second run), and the entered veteran with its comparison (on demand)
let ahead = { state: 'none', r: null };
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
      const R = m.r.rollout, rc = R && !R.error && !(R.candidates_proven && R.shortlist_proven) ? ' The pick\'s check of next careers ran out of time: it uses the best found.' : '';
      const cert = (m.r.proven_top ? 'Checked on exact odds: nothing else does better.' : 'The exact check ran out of time: the order may not be final.') + rc;
      const n = m.r.candidates;
      status = { kind: 'done', text: (n > 1 ? `Weighed ${n.toLocaleString()} careers in ${(m.ms / 1000).toFixed(1)} s. ` : `Scored in ${(m.ms / 1000).toFixed(1)} s. `) + cert };
      const best = m.r.results[0], r = m.r.rollout?.choice || best;
      simulation = r ? { state: 'ready', draft: draftOf(r), r: null, baseline: null, error: null } : { state: 'none' };
      ahead = r ? { state: 'busy', r: null } : { state: 'none' };
      if (best && !job.trainees.length && !job.fix.length) baselineCache = { key: baseKey(job), result: best };
    }
  } catch (e) { if (id !== myId) return; status = { kind: 'error', text: e.message }; last = null; simulation = { state: 'none' }; }
  searching = false; clearInterval(tick); renderAll(); paint();
  if (ahead.state === 'busy') lookAhead();
}
// the outcome look ahead for the career on show; today's best is the main result when the planner had every place.
// It shares the one worker with the comparison: a comparison cancelled while both were queued takes it down too, and
// it runs again (aheadId tells the run that counts).
let aheadId = 0;
async function lookAhead() {
  const id = myId, aid = ++aheadId, r = shown(); if (!r) return;
  const best = last.r.results[0], open = !plan.trainee && !plan.parents[0] && !plan.parents[1];
  const { rollout: _r, p_now: _p, ...plain } = last.job;
  const job2 = { ...plain, trainees: [r.trainee.id], fix: r.parents, per_trainee: false, top: 1, lookahead: true, ...(open ? { p_now: best.p_all, p_minus1_now: best.p_minus1 } : {}) };
  ahead = { state: 'busy', r: null };
  try {
    const m2 = await run(job2);
    if (id !== myId || aid !== aheadId) return;
    ahead = m2.r.lookahead ? { state: 'done', r: m2.r.lookahead } : { state: 'error', text: m2.r.error || m2.r.lookahead?.error || 'No look ahead.' };
  } catch (e) { if (id !== myId || aid !== aheadId) return; ahead = { state: 'error', text: e.message }; }
  renderSimulation();
}

// ---------- the pedigree ----------
// the career on show: the top result while it belongs to the current plan
// (the planner's rollout pick when it chose places, else the highest odds)
const shown = () => last && last.plan === JSON.stringify(plan) && last.r.results[0] ? (last.r.rollout?.choice || last.r.results[0]) : null;
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
    who.append(pb, nm, winsBtn(v, umaName(v.chara)));
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
      who.append(b, el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'), winsBtn(g, umaName(g.chara)));
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
function schedBtn(r) {
  const rows = r.schedule || [], ran = rows.filter(x => x.run !== false).length;
  const b = btn('Race schedule', '', () => openSched({ members: [{ id: r.trainee.id, name: r.trainee.name, short: r.trainee.name }], outfits: [{ outfits: r.trainee.outfits }], schedule: rows }));
  b.id = 'sched-open'; b.title = rows.length ? `She runs ${ran} of the ${rows.length} G1s she can enter: ${r.g1_wins.toFixed(1)} wins expected` : 'No G1s';
  return b;
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
const careerText = (r) => `${r.trainee.name} with ${r.parents.map(i => `#${i + 1} ${umaName(vets[i].chara)}`).join(' + ')}`;
// ---------- after this career: what she could land, and the veteran analysis (the veteran you got) ----------
// The look ahead (a second solver run after the main one) lists her possible outcomes with the best next career each
// gives; it decides nothing. The decision is per veteran: enter the one you got (it starts as the most likely result)
// and Compare runs both choices, running this career again or using her in the next one, on the same terms.
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
function rollDraft() { const r = shown(); if (!r || searching) return; Object.assign(simulation.draft, rolledOutcome(r)); simulation.r = null; simulation.stale = false; compare(); }
// A comparison in flight is for a veteran that is going (a roll, a reset, Compare pressed again): stop it. A wasm call
// can't be interrupted, so the worker goes (run() starts a new one). Returns whether the look ahead went with it.
function cancelCompare() {
  if (simulation.state !== 'busy') return false;
  simulationId++; killEngine(); simulation.state = 'ready';
  return ahead.state === 'busy';
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
  simulation = { ...simulation, draft: draftOf(r), r: null, stale: false, error: null, state: 'ready' }; renderSimulation(); if (lost) lookAhead(); }
// Compare (or a roll): a press while a comparison runs restarts it with the veteran as she is now. The look ahead, if the
// restart took it down, is queued again behind the comparison (runComparison posts its first job before it returns).
function compare() {
  if (searching || !shown()) return;
  const lost = cancelCompare();
  runComparison();
  if (lost) lookAhead();
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
    const current = shown();
    const m = await run({ ...base, fix: [base.legacies.length], legacies: [...base.legacies, draftJob(simulation.draft)],
      policy: { stay_trainee: current.trainee.id, stay_parents: current.parents, p_floor: simulation.baseline.p_all } });
    if (id !== simulationId || source !== myId) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    simulation.r = m.r; simulation.state = 'done';
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
// after this career: how likely she improves your best next career (over what she lands and her stars), as label/value rows
function outcomesEl(L, n) {
  const box = el('div', 'ahead');
  const imp = L.stars_exact ? L.improve_stars : L.improve;
  const dl = el('dl', 'kv');
  const row = (label, value, big = false, title = '') => { const dt = el('dt', '', label), dd = el('dd', big ? 'big' : '', value); if (title) dt.title = title; dl.append(dt, dd); };
  row('Chance she improves your best next career', pct(imp), true, 'Over every set of targets she could land and every star count (stars by the lineage rank, G1 wins by her schedule). The next careers checked are the best found with her landing every target, so this is a floor.');
  row('Your best career today', fmtIn(L.p_now));
  if (imp > 0 && L.f_if_improve) row('When she improves it, the next career averages', fmtIn(L.f_if_improve));
  row(`She lands all ${n}`, pct(L.p_all));
  if (imp > 0 && L.improve_by_count) for (const c of L.improve_by_count.filter(c => c.landed >= n - 3 && c.landed < n)) row(`Lands ${c.landed} of ${n}: improves it`, pct(c.improve));
  if (imp === 0) {
    // the usual reason: today's best career trains her own character, and no one parents her own character
    const r = shown(), best = last?.r.results[0], open = !plan.trainee && !plan.parents[0] && !plan.parents[1];
    row('Why not', open && r && best && best.trainee.id === r.trainee.id ? `Your best career trains ${r.trainee.name}; she can't parent her own character` : 'No result of hers beats your best career today');
  }
  box.appendChild(dl);
  return box;
}
function aheadEl() {
  const box = el('div');
  if (ahead.state === 'busy') { const p = el('p', 'runnote'); p.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Working out what she could land…</span>'; box.appendChild(p); }
  else if (ahead.state === 'done') box.appendChild(outcomesEl(ahead.r, state.targets.length));
  else if (ahead.state === 'error') box.appendChild(el('p', 'hint', ahead.text));
  return box;
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
  const wb = btn(`${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`, 'vwins', () => openWins(d, umaName(d.chara), { races: scheduledRaces(r), onEdit: editDraft }));
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
  if (busy) { const p = el('p', 'runnote'); p.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Comparing: this career again, or her in the next one…</span>'; result.appendChild(p); }
  else if (simulation.error) result.appendChild(el('p', 'note', simulation.error));
  else if (simulation.r) {
    if (simulation.stale) result.appendChild(el('p', 'note', 'You changed her since: press Compare again.'));
    const next = simulation.r.results[0], before = simulation.baseline, n = state.targets.length, policy = simulation.r.policy;
    const verdict = el('p', 'note'); verdict.id = 'sim-verdict';
    if (policy?.stay && policy?.advance) {
      const cost = a => a.expected_careers_estimate === null ? Infinity : a.expected_careers_estimate;
      const stay = cost(policy.stay), advance = cost(policy.advance);
      const better = advance < stay * (1 - 1e-6), worse = stay < advance * (1 - 1e-6);
      verdict.append(el('strong', '', better ? 'Use her in the next career.' : worse ? 'Run this career again.' : 'Neither choice is better.'),
        ' ' + (better || worse ? 'It takes fewer careers on average to land every target.' : 'Both take about as many careers to land every target.'));
      result.appendChild(verdict);
      const est = el('div', 'simodds');
      for (const [label, a] of [['Run this career again', policy.stay], ['Use her in the next career', policy.advance]]) {
        const c = cost(a), card = el('div', 'tc');
        card.append(el('span', 'tl', label), el('b', 'tv', Number.isFinite(c) ? `${c.toFixed(1)} careers` : 'never'), el('span', 'hint', `All ${n} on the next run: ${pct(a.p_all)}; within two runs: ${pct(a.p_within_two)}`));
        est.appendChild(card);
      }
      result.appendChild(est);
      if (!policy.actions_proven || !policy.stay.next_proven || !policy.advance.next_proven) result.appendChild(el('p', 'hint', 'Some searches reached their work limit: this uses the best setups found so far.'));
    } else { verdict.append(el('strong', '', 'No comparison.'), ' ' + (policy?.error || 'The solver returned none.')); result.appendChild(verdict); }
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
  const a = $('ahead-sec'); if (a) a.querySelector('.abody').replaceChildren(aheadEl());
  if (k && k.startsWith('sim:') && !ae.isConnected) document.querySelector(`[data-k="${CSS.escape(k)}"]`)?.focus();
}
// the other careers the planner weighed: only what it picked (the places left open), with "Use this". With the rollout,
// its candidates: each one's odds now and how good the next careers her veterans leave are.
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
function altsEl(res) {
  const box = sec('alts', 'Other picks');
  const ol = el('ol'), R = res.rollout, n = state.targets.length;
  const list = R && R.candidates ? R.candidates : res.results, on = R && R.candidates ? R.pick : 0;
  list.forEach((r, k) => {
    const li = el('li', k === on ? 'on' : '');
    li.append(el('span', 'rank', String(k + 1)), pickWhat(r), el('span', 'alto', R ? `${fmtIn(r.p_all)} now` : `${fmtIn(r.p_all)} · ${pct(r.p_minus1)} at least ${n - 1}`));
    li.appendChild(k === on ? el('span', 'hint', 'shown above') : useBtn(r));
    if (R) {
      // the veterans she leaves: the next career they give on average, and the best next career with her landing everything
      const nl = el('div', 'altn');
      nl.append(`Next career after her: ${fmtIn(r.f_fail)} on average` + (r.gain > 1 + 1e-9 ? ` (×${r.gain.toFixed(2)} today's best)` : ' (no better than today\'s best)'));
      if (r.best_next) { const b = r.best_next, who = { chara: r.trainee.id, outfit: r.trainee.outfits[0]?.id };
        nl.append(' · at best ', facesEl(b.trainee.id, null, [parentFace(vets.length, who), parentFace(b.partner, who)]), ` ${fmtIn(b.p_all)}`); }
      li.appendChild(nl);
    }
    ol.appendChild(li);
  });
  box.appendChild(ol);
  if (R && R.candidates) box.appendChild(el('p', 'hint', `The planner runs the career whose veterans leave the best next careers, not always the one with the best odds now (careers counted under the model). Today's best career, repeated: ${Math.round(R.repeat_careers).toLocaleString()} careers on average; no plan averages fewer than ${Math.round(R.lower_bound).toLocaleString()}${R.p_bar_proven ? '' : ' (bound from the umas checked)'}.`));
  return box;
}
function renderResults() {
  const box = $('results'); box.textContent = '';
  const r = shown(); if (!r) { renderSimulation(); return; }
  box.append(deckEl(r, 'Deck', schedBtn(r)), oddsEl(r));
  const R = last.r.rollout;
  if (R && R.differs && R.choice) { const best = last.r.results[0];
    box.appendChild(el('p', 'sub pickwhy', `Picked over ${careerText(best)} (${fmtIn(best.p_all)} now): the veterans it leaves give better next careers.`)); }
  const top = el('div'); top.id = 'sim-top';
  box.append(top, targetsEl(r));
  const a = sec('rahead', 'After this career'); a.id = 'ahead-sec'; a.appendChild(el('div', 'abody')); box.appendChild(a);
  const open = !plan.trainee || plan.parents.some(p => !p);
  if (open && (R?.candidates?.length > 1 || last.r.results.length > 1)) box.appendChild(altsEl(last.r));
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
    head.append(who, winsBtn(nd, `${label}, ${umaName(nd.chara)}`));
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
function importText(text) {
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
    if (vetsSig() !== sig) { sig = vetsSig(); last = null; simulation = { state: 'none' }; ahead = { state: 'none' }; }
    schedule(); renderAll();
  });
  for (const x of ['vets', 'new', 'auto']) $('vp-tab-' + x).onclick = () => vpTab(x);
  $('vp-filter').oninput = drawNew;
  $('vp-auto-go').onclick = () => { plan.parents[pickK] = null; $('vpick-dlg').close(); changed(); };
  $('v-file').onchange = async (e) => { const f = e.target.files[0]; if (f) importText(await f.text()); e.target.value = ''; };
  $('v-paste').onclick = () => { $('paste-text').value = ''; $('paste-dlg').showModal(); };
  $('paste-load').onclick = () => { if (importText($('paste-text').value)) $('paste-dlg').close(); };
  $('v-clear').onclick = () => { if (vets.length && confirm(`Remove all ${vets.length} veterans?`)) clearVets(); };
  renderAll(); search();
}, (e) => { status = { kind: 'error', text: e.message }; paint(); });
