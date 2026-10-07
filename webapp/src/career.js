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
import { initCards } from './cards.js';
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
async function search() {
  const id = ++myId;
  let job;
  try { job = buildJob(); } catch (e) { searching = false; status = { kind: e.kind || 'error', text: e.message }; last = null; renderAll(); paint(); return; }
  searching = true; t0 = performance.now(); clearInterval(tick); tick = setInterval(paint, 1000); paint();
  try {
    const m = await run(job);
    if (id !== myId) return;
    if (m.r.error) { status = { kind: 'error', text: 'Not searched: ' + m.r.error }; last = null; }
    else {
      last = { r: m.r, plan: JSON.stringify(plan) };
      const cert = m.r.proven_top ? 'Checked on exact odds: nothing else does better.' : 'The exact check ran out of time: the order may not be final.';
      const n = m.r.candidates;
      status = { kind: 'done', text: (n > 1 ? `Weighed ${n.toLocaleString()} careers in ${(m.ms / 1000).toFixed(1)} s. ` : `Scored in ${(m.ms / 1000).toFixed(1)} s. `) + cert };
    }
  } catch (e) { if (id !== myId) return; status = { kind: 'error', text: e.message }; last = null; }
  searching = false; clearInterval(tick); renderAll(); paint();
}

// ---------- the tree ----------
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
function traineeEl(r) {
  const box = el('div', 'tnode trainee');
  const id = plan.trainee || (r && r.trainee.id);
  box.appendChild(el('div', 'tlab', plan.trainee ? 'Trainee' : 'Trainee · planner\'s pick'));
  if (!plan.trainee) box.classList.add('ghost');
  const who = el('div', 'twho');
  if (id) {
    who.append(umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 88));
    const nm = el('div', 'tnm'); nm.append(el('div', 'nm', umaName(id)));
    if (r && r.trainee.id === id) nm.append(el('div', 'sub', r.trainee.outfits.length ? r.trainee.outfits.map(o => o.label).join(' or ') : 'any outfit'));
    who.append(nm);
  } else who.append(el('div', 'tempty', 'The planner tries every uma you own.'));
  box.appendChild(who);
  const acts = el('div', 'row tacts');
  acts.appendChild(btn(plan.trainee ? 'Change trainee' : 'Choose trainee', 'primary', () => openUmaPicker('Trainee: which uma?', (u) => { plan.trainee = u; changed(); }, owned)));
  if (plan.trainee) acts.appendChild(btn('Let the planner pick', '', () => { plan.trainee = null; changed(); }));
  box.appendChild(acts);
  return box;
}
function parentEl(k, r) {
  const slot = el('div', 'pslot');
  const at = parentAt(k, r);
  const card = el('div', 'tnode parent' + (at && at.pick ? ' ghost' : '') + (at && at.v.borrowed ? ' bor' : ''));
  card.appendChild(el('div', 'tlab', `Parent ${k + 1}` + (at && at.pick ? ' · planner\'s pick' : '') + (at && at.v.borrowed ? ' · borrowed' : '')));
  if (at) {
    const v = at.v, i = idxOf(v.id);
    const who = el('div', 'twho');
    const pb = el('button', 'facebtn'); pb.type = 'button'; pb.append(umaAv(v.chara, v.outfit, 64)); pb.title = 'Choose another parent'; pb.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Choose another parent`);
    pb.onclick = () => openParentPicker(k);
    const nm = el('div', 'tnm'); nm.append(el('div', 'nm', umaName(v.chara)), el('div', 'sub', `Veteran #${i + 1}` + (v.src === 'hand' ? ', made here' : '')));
    who.append(pb, nm, winsBtn(v, umaName(v.chara)));
    card.append(who, sparksEl(v, 'p' + k));
  } else card.appendChild(el('p', 'tempty', vets.length >= 2 ? 'The planner picks from your veterans.' : 'No parent yet.'));
  const acts = el('div', 'row tacts');
  acts.appendChild(btn(plan.parents[k] ? 'Change parent' : 'Choose parent', 'primary big', () => openParentPicker(k)));
  if (plan.parents[k]) acts.appendChild(btn('Let the planner pick', '', () => { plan.parents[k] = null; changed(); }));
  card.appendChild(acts);
  slot.appendChild(card);
  // her own parents: the trainee's grandparents (who they are and what they carry is part of the veteran)
  if (at) {
    const gps = el('div', 'gps');
    at.v.parents.forEach((g, j) => {
      const n = el('div', 'tnode gp');
      n.appendChild(el('div', 'tlab', `Grandparent (parent ${j + 1} of ${umaName(at.v.chara)})`));
      const who = el('div', 'twho');
      const b = el('button', 'facebtn'); b.type = 'button'; b.append(umaAv(g.chara || 0, g.outfit, 40));
      b.title = 'Change who she is'; b.setAttribute('aria-label', `Grandparent: ${umaName(g.chara)}. Change who she is`);
      b.onclick = () => changeUma(g, 'Grandparent');
      who.append(b, el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'), winsBtn(g, umaName(g.chara)));
      n.append(who, sparksEl(g, `p${k}g${j}`));
      gps.appendChild(n);
    });
    slot.appendChild(gps);
  }
  return slot;
}
function renderTree() {
  const r = shown(), box = $('tree'); box.textContent = '';
  box.appendChild(traineeEl(r));
  const ps = el('div', 'tpars'); ps.append(parentEl(0, r), parentEl(1, r)); box.appendChild(ps);
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
  if (!vets.length) box.innerHTML = '<p class="hint">No veterans yet: import them below the tree, or make a new one.</p>';
  vpTab(vets.length ? 'vets' : 'new');
  $('vpick-dlg').showModal();
}

// ---------- results under the tree ----------
function detailEl(r) {
  const box = el('section', 'cdetail');
  const n = state.targets.length;
  const odds = el('div', 'bigodds');
  const o = (big, small) => { const d = el('div', 'bo'); d.append(el('b', '', big), el('span', '', small)); return d; };
  odds.append(o(fmtIn(r.p_all), `${pct(r.p_all)}: all ${n} targets land as sparks`), o(pct(r.p_minus1), `at least ${n - 1} of ${n}`), o(r.expected.toFixed(2), `targets expected, of ${n}`));
  box.appendChild(odds);
  const tb = el('table', 'cartab');
  tb.innerHTML = '<thead><tr><th scope="col">Target skill</th><th scope="col" title="Chance the six legacies pass the skill on (two inheritance events)">Inherited</th><th scope="col" title="Legacies (of six) with a spark of it">Sparks in lineage</th><th scope="col" title="Chance the new veteran gets its spark">Lands as a spark</th></tr></thead><tbody></tbody>';
  for (const t of r.targets) {
    const row = el('tr'), c0 = el('th'); c0.scope = 'row'; const ic = skillIcon(t.group, t.gold); if (ic) c0.appendChild(ic); c0.append(tName(t.group));
    row.appendChild(c0);
    for (const v of [t.gold ? 'scenario' : pct(t.q), `${t.carriers} of 6`, pct(t.p)]) row.appendChild(el('td', '', v));
    tb.tBodies[0].appendChild(row);
  }
  box.appendChild(tb);
  if (r.pinks.length) {
    const p = el('p', 'pinkres'); p.append(el('span', 'sub', 'Aptitudes passed on (at least once): '));
    for (const x of r.pinks) p.append(el('span', 'pchip', `${pinkName(x.group)} ${pct(x.q)}`), ' ');
    box.appendChild(p);
  }
  const dk = el('div', 'cardeck'); dk.appendChild(el('div', 'sub', 'Deck'));
  const ul = el('ul');
  r.deck.cards.forEach((name, i) => { const li = el('li'); const id = r.deck.card_ids[i] ?? r.deck.plain_ids[i - r.deck.card_ids.length];
    li.append(cardAv(CARD_BY_ID.get(id), 28), ' ' + name); ul.appendChild(li); });
  dk.appendChild(ul); box.appendChild(dk);
  return box;
}
// the other careers the planner weighed: only what it picked (the places left open), with "Use this"
function altsEl(res) {
  const box = el('section', 'alts');
  box.appendChild(el('h3', '', 'Other picks'));
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
  box.appendChild(detailEl(r));
  const open = !plan.trainee || plan.parents.some(p => !p);
  if (open && last.r.results.length > 1) box.appendChild(altsEl(last.r));
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
