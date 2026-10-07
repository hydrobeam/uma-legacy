// Career planner page: entry point. build.py bundles this and the modules it imports into dist/career.html.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (vets.js) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
import { $ } from './dom.js';
import { ERA, SAVED } from './settings.js';
import { UMAS, CARDS, CARD_BY_ID, SK, loadData } from './data.js';
import { state, goldGiven, writeSaved } from './state.js';
import { run, killEngine } from './engine.js';
import { onSave } from './save.js';
import { EXTRA, jobTargets, ownedCards, rcardsOff } from './job.js';
import { umaAv, skillIcon, cardAv } from './images.js';
import { initScenario, eraScenario } from './scenario.js';
import { initRoster, renderRoster, noOutfit } from './roster.js';
import { initCards, cardGives } from './cards.js';
import { initTargets, renderTargets } from './targets.js';
import { vets, plan, onVetsChange, changed, persist, vetById, umaName, skillName, pinkName, sparksEl, winsBtn, changeUma, askSkills,
  newVet, importVets, removeVet, clearVets, legaciesJob, openUmaPicker, drawUmas } from './vets.js';
import { parseExport } from './umadump.js';

const pct = (p) => (p * 100 >= 10 ? (p * 100).toFixed(0) : p * 100 >= 1 ? (p * 100).toFixed(1) : (p * 100).toFixed(2)) + '%';
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
  if (searching) killEngine();
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
  return { cmd: 'career', targets: jobTargets(), exclude, exclude_outfits, extra: EXTRA, owned_cards: ownedCards(), borrow: state.borrow,
    scenario: state.scen || 'unity', ...rcardsOff(), legacies: legaciesJob(), trainees: plan.trainee ? [plan.trainee] : [],
    fix: plan.parents.filter(Boolean).map(idxOf), per_trainee: !plan.trainee, top: all ? 1 : 5 };
}
// the look ahead (reroll or move on) for the career on show: a second solver run after the main one
let ahead = { state: 'none', r: null, key: null };
async function search() {
  const id = ++myId;
  let job;
  try { job = buildJob(); } catch (e) { searching = false; status = { kind: e.kind || 'error', text: e.message }; last = null; ahead = { state: 'none' }; renderAll(); paint(); return; }
  searching = true; t0 = performance.now(); clearInterval(tick); tick = setInterval(paint, 1000); paint();
  try {
    const m = await run(job);
    if (id !== myId) return;
    if (m.r.error) { status = { kind: 'error', text: 'Not searched: ' + m.r.error }; last = null; ahead = { state: 'none' }; }
    else {
      last = { r: m.r, plan: JSON.stringify(plan) };
      const cert = m.r.proven_top ? 'Checked on exact odds: nothing else does better.' : 'The exact check ran out of time: the order may not be final.';
      const n = m.r.candidates;
      status = { kind: 'done', text: (n > 1 ? `Weighed ${n.toLocaleString()} careers in ${(m.ms / 1000).toFixed(1)} s. ` : `Scored in ${(m.ms / 1000).toFixed(1)} s. `) + cert };
      ahead = { state: m.r.results.length ? 'busy' : 'none', r: null };
    }
  } catch (e) { if (id !== myId) return; status = { kind: 'error', text: e.message }; last = null; ahead = { state: 'none' }; }
  searching = false; clearInterval(tick); renderAll(); paint();
  if (ahead.state !== 'busy') return;
  // the look ahead for the career on show; today's best over every career is the main result when the planner had every place
  const r = last.r.results[0], open = !plan.trainee && !plan.parents[0] && !plan.parents[1];
  const job2 = { ...job, trainees: [r.trainee.id], fix: r.parents, per_trainee: false, top: 1, lookahead: true, ...(open ? { p_now: r.p_all } : {}) };
  try {
    const m2 = await run(job2);
    if (id !== myId) return;
    ahead = m2.r.lookahead ? { state: 'done', r: m2.r.lookahead } : { state: 'error', text: m2.r.error || 'No look ahead.' };
  } catch (e) { if (id !== myId) return; ahead = { state: 'error', text: e.message }; }
  renderAhead();
}

// ---------- the pedigree ----------
// the career on show: the top result while it belongs to the current plan
const shown = () => last && last.plan === JSON.stringify(plan) && last.r.results[0] ? last.r.results[0] : null;
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
  const who = el('div', 'twho');
  if (id) {
    who.append(umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 96));
    const nm = el('div', 'tnm'); nm.append(el('div', 'nm', umaName(id)));
    if (r && r.trainee.id === id) nm.append(el('div', 'sub', r.trainee.outfits.length ? r.trainee.outfits.map(o => o.label).join(' or ') : 'any outfit'));
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
  askSkills(umaName(id), false, (sp) => { const v = newVet(id, sp || {}); plan.parents[pickK] = v.id; changed(); });
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

// ---------- the career: tote board, deck, targets, calendar, look ahead, other picks ----------
const sec = (cls, kicker, title) => { const s = el('section', 'rbsec ' + cls); const h = el('h3'); h.append(el('span', 'rbk', kicker), ' ' + title); s.appendChild(h); return s; };
function toteEl(r) {
  const n = state.targets.length, box = el('div', 'tote'); box.setAttribute('role', 'group'); box.setAttribute('aria-label', 'Odds');
  const cell = (big, small, main) => { const d = el('div', 'tc' + (main ? ' main' : '')); d.append(el('span', 'tl', small), el('b', 'tv', big)); return d; };
  box.append(cell(fmtIn(r.p_all), `All ${n} land · ${pct(r.p_all)}`, true), cell(pct(r.p_minus1), `At least ${n - 1}`), cell(r.expected.toFixed(2), `Expected of ${n}`));
  return box;
}
// the deck as a race card: numbered, the card's art, its name, and what it gives toward the targets
function deckEl(r) {
  const s = sec('rcard', 'Race card', 'Deck');
  const groups = new Set(state.targets.filter(t => !goldGiven(t.group)).map(t => t.group));
  const ol = el('ol');
  r.deck.cards.forEach((name, i) => {
    const id = r.deck.card_ids[i] ?? r.deck.plain_ids[i - r.deck.card_ids.length], c = CARD_BY_ID.get(id);
    const li = el('li'); li.style.setProperty('--gate', `var(--gate${i + 1})`); li.style.setProperty('--gate-ink', `var(--gate${i + 1}-ink)`);
    li.append(el('span', 'gate', String(i + 1)), cardAv(c, 64));
    const tx = el('div', 'rct'); tx.append(el('div', 'nm', name));
    const gv = el('div', 'rcg');
    for (const x of c ? cardGives(c, groups) : []) { const sp = el('span', 'gv' + (x.g ? ' g' : ''), x.t); sp.title = x.tip; sp.appendChild(el('small', '', ' ' + x.s)); gv.appendChild(sp); }
    if (!gv.children.length) gv.appendChild(el('span', 'hint', c && c.pal ? 'Pal: friendship and its own events' : 'Fills a slot'));
    tx.appendChild(gv); li.appendChild(tx); ol.appendChild(li);
  });
  s.appendChild(ol);
  if (r.deck.borrowed) s.appendChild(el('p', 'hint', `Borrowed: ${r.deck.borrowed}.`));
  return s;
}
function targetsEl(r) {
  const s = sec('rtgt', 'Form', 'Target skills');
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
// her G1s by year: when, race, win chance, career goal, and which parents won it (each shared win is affinity later)
function calendarEl(r) {
  const s = sec('rcal', 'Calendar', 'Race schedule');
  const rows = r.schedule || [];
  const shared = rows.filter(x => x.parents.some(Boolean)).length;
  s.appendChild(el('p', 'hint', rows.length ? `She runs the G1s her parents won where she can win them (each one she shares with a parent adds affinity when she is a parent herself): ${rows.length} G1s, ${shared} shared with a parent, ${r.g1_wins.toFixed(1)} wins expected. Career goals are marked.` : 'No G1s scheduled.'));
  const yrs = el('div', 'cyears');
  ['Junior', 'Classic', 'Senior'].forEach((y, yi) => {
    const yr = el('div', 'cyear'); yr.appendChild(el('div', 'cyl', y));
    const list = el('div', 'cyr');
    for (const x of rows.filter(x => x.year === yi)) {
      const c = el('div', 'race' + (x.goal ? ' goal' : ''));
      const top = el('div', 'rw'); top.append(el('span', 'rwhen', x.when.split(' ')[1].replace('-', ' ')));
      for (const [k, on] of x.parents.entries()) if (on) { const g = el('span', 'gate sm', String(k * 3 + 1)); g.style.setProperty('--gate', `var(--gate${k * 3 + 1})`); g.style.setProperty('--gate-ink', `var(--gate${k * 3 + 1}-ink)`); g.title = `Parent ${k + 1} won it`; top.appendChild(g); }
      c.append(top, el('div', 'rn', x.race), el('div', 'rp', `${pct(x.p)} win` + (x.goal ? ' · goal' : '')));
      list.appendChild(c);
    }
    if (!list.children.length) list.appendChild(el('span', 'hint', 'No G1s'));
    yr.appendChild(list); yrs.appendChild(yr);
  });
  s.appendChild(yrs);
  return s;
}
const careers = (x) => x === null || x === undefined || !isFinite(x) ? 'never' : Math.round(x).toLocaleString();
// the look ahead: what she lands decides whether to move on with her or run this career again
function aheadBody(L, n) {
  const box = el('div', 'ahead');
  const head = el('div', 'tote small');
  const cell = (big, small) => { const d = el('div', 'tc'); d.append(el('span', 'tl', small), el('b', 'tv', big)); return d; };
  head.append(cell(careers(L.careers_this), 'Careers to an all-target veteran: this career, then the table'), cell(careers(L.careers_now), 'Repeating today\'s best career'), cell(pct(L.improve), 'Chance she improves the next career'));
  box.appendChild(head);
  if (L.improve === 0) box.appendChild(el('p', 'note', 'Whatever she lands, she can\'t beat your best career today (your veterans already carry more stars than a new spark is likely to). Moving on or rerolling makes no difference: keep running the best career.'));
  const tb = el('table', 'cartab ahtab');
  tb.innerHTML = '<thead><tr><th scope="col">She lands</th><th scope="col">Chance</th><th scope="col" title="Best next career with her as a parent (at least today\'s best)">Next career</th><th scope="col">Then</th></tr></thead><tbody></tbody>';
  // nothing she lands beats today's best: both choices cost the same
  const either = L.improve === 0 ? ['Either', 'mix'] : null;
  const verdict = (share) => either || (share >= 0.999 ? ['Reroll', 'rr'] : share <= 0.001 ? ['Move on', 'mo'] : [`Mostly ${share >= 0.5 ? 'reroll' : 'move on'} (${pct(share >= 0.5 ? share : 1 - share)})`, 'mix']);
  const row = (label, p, f, v) => { const tr = el('tr'); const th = el('th', '', label); th.scope = 'row'; tr.appendChild(th);
    tr.append(el('td', '', pct(p)), el('td', '', f === null ? '—' : fmtIn(f))); const td = el('td'); td.appendChild(el('span', 'verdict ' + v[1], v[0])); tr.appendChild(td); tb.tBodies[0].appendChild(tr); };
  row(`All ${n}`, L.p_all, null, ['Done', 'done']);
  for (const c of L.by_count) {
    if (c.landed === n - 1) {
      // all but one: which one she misses decides
      for (const m of L.miss_one.filter(m => m.p > 0).sort((a, b) => b.p - a.p)) row(`All but ${tName(m.group)}`, m.p, m.f, either || (m.reroll ? ['Reroll', 'rr'] : ['Move on', 'mo']));
    } else row(`${c.landed} of ${n}`, c.p, c.f, verdict(c.reroll));
  }
  box.appendChild(tb);
  if (L.setups.length) {
    const p = el('p', 'hint'); p.append('With her as a parent and every target landed, the best next careers: ');
    p.append(L.setups.slice(0, 3).map(s => `${s.trainee.name} with #${s.partner + 1} ${umaName(vets[s.partner]?.chara)} (${fmtIn(s.p_all)})`).join('; ') + '.');
    box.appendChild(p);
  }
  box.appendChild(el('p', 'hint', 'How it decides: moving on with her costs 1 ÷ (the next career\'s chance) careers on average; running this career once more and keeping the better veteran costs one career plus the same for the better of the two. Her stars follow the rank above; her G1 wins, her schedule. The next career\'s chances are for the best setups found for her landing every target, so they are a floor.'));
  return box;
}
function renderAhead() {
  const s = $('ahead-sec'); if (!s) return;
  const body = s.querySelector('.abody'); body.textContent = '';
  if (ahead.state === 'busy') { const p = el('p', 'runnote'); p.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Looking one career ahead…</span>'; body.appendChild(p); }
  else if (ahead.state === 'done') body.appendChild(aheadBody(ahead.r, state.targets.length));
  else if (ahead.state === 'error') body.appendChild(el('p', 'hint', ahead.text));
}
// the other careers the planner weighed: only what it picked (the places left open), with "Use this"
function altsEl(res) {
  const box = sec('alts', 'Also running', 'Other picks');
  const ol = el('ol');
  res.results.forEach((r, k) => {
    const li = el('li', k === 0 ? 'on' : '');
    const what = el('div', 'altw');
    if (!plan.trainee) { what.append(umaAv(r.trainee.id, r.trainee.outfits[0]?.id, 32), ' ' + r.trainee.name); }
    const open = r.parents.filter(i => !plan.parents.includes(vets[i].id));
    if (open.length) { if (!plan.trainee) what.append(', parents '); else what.append('Parents ');
      open.forEach((i, j) => { if (j) what.append(' + '); what.append(umaAv(vets[i].chara, vets[i].outfit, 24), ` #${i + 1} ${umaName(vets[i].chara)}`); }); }
    li.append(el('span', 'rank', String(k + 1)), what, el('span', 'alto', `${fmtIn(r.p_all)} · ${pct(r.p_minus1)} at least ${state.targets.length - 1}`));
    li.appendChild(k === 0 ? el('span', 'hint', 'shown above') : btn('Use this', '', () => {
      plan.trainee = r.trainee.id; const f = plan.parents.slice();
      const fr = r.parents.filter(i => !f.includes(vets[i].id)); for (let j = 0; j < 2; j++) if (!f[j]) f[j] = vets[fr.shift()].id;
      plan.parents = f; changed(); }));
    ol.appendChild(li);
  });
  box.appendChild(ol);
  return box;
}
function renderResults() {
  const box = $('results'); box.textContent = '';
  const r = shown(); if (!r) return;
  box.appendChild(toteEl(r));
  const grid = el('div', 'rbgrid');
  grid.append(deckEl(r), targetsEl(r));
  box.appendChild(grid);
  box.appendChild(calendarEl(r));
  const a = sec('rahead', 'Next leg', 'After this career: move on or reroll?'); a.id = 'ahead-sec'; a.appendChild(el('div', 'abody')); box.appendChild(a);
  const open = !plan.trainee || plan.parents.some(p => !p);
  if (open && last.r.results.length > 1) box.appendChild(altsEl(last.r));
  renderAhead();
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
  top.appendChild(btn('Remove', 'vdel', () => removeVet(i))).setAttribute('aria-label', `Remove veteran #${i + 1}`);
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
  $('o-umas-n').textContent = `${UMAS.filter(owned).length} owned`;
  $('o-cards-n').textContent = `${CARDS.filter(c => state.cardsOff[c.id] !== false).length} owned`;
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
  initSettings();
  onVetsChange(() => { renderAll(); schedule(); });
  for (const x of ['vets', 'new', 'auto']) $('vp-tab-' + x).onclick = () => vpTab(x);
  $('vp-filter').oninput = drawNew;
  $('vp-auto-go').onclick = () => { plan.parents[pickK] = null; $('vpick-dlg').close(); changed(); };
  $('v-file').onchange = async (e) => { const f = e.target.files[0]; if (f) importText(await f.text()); e.target.value = ''; };
  $('v-paste').onclick = () => { $('paste-text').value = ''; $('paste-dlg').showModal(); };
  $('paste-load').onclick = () => { if (importText($('paste-text').value)) $('paste-dlg').close(); };
  $('v-clear').onclick = () => { if (vets.length && confirm(`Remove all ${vets.length} veterans?`)) clearVets(); };
  renderAll(); search();
}, (e) => { status = { kind: 'error', text: e.message }; paint(); });
