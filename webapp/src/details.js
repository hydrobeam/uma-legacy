// ---------- the veteran card: the game's "Umamusume Details" screen ----------
// Her portrait and outfit, her aptitude grades (Track / Distance / Style, the game's letters), then three tabs as in the
// game: Skills (the target skills she learned), Inspiration (her sparks as the game's spark bars, stars under each, then
// "Legacy Origin": her two parents' sparks) and Career Info (the G1s she ran and won). No stats (the model has none).
// The game's own art where the build shipped it (images.py: grade letters, spark bars and bullets, skill tiles), the
// same shapes in CSS without it. Her sparks stay editable (vets.js sparksEl: click for stars, x removes, + adds).
import { el } from './dom.js';
import { SK, CARD_BY_ID, CARDS } from './data.js';
import { srcFor, umaAv, skillIcon, cardAv } from './images.js';
import { umaName, sparksEl } from './vets.js';

let tab = 'insp';
// a spark bar's name on one line, its type shrunk until it fits (the game's way), down to 9 px; panes on show only
// Each name takes the largest size f0 - 0.5k that fits, or the first one at or under 9 px (a step-by-step shrink). All
// names are sized together: every round writes each unsettled name's next size to try, then reads them all (one layout
// per round, not one per step per name), halving the range of steps left (the width only grows with the size).
const fitting = new Set();   // roots with a pass already queued (resize fires many times a frame)
export function fitSparks(root = document) {
  if (fitting.has(root)) return;
  fitting.add(root);
  requestAnimationFrame(() => {
    fitting.delete(root);
    const es = [...root.querySelectorAll('.ud .vskn')];
    for (const e of es) e.style.fontSize = '';
    const fits = (e) => !(e.scrollWidth > e.clientWidth + 0.5);
    // per name: base size f0, the steps that are known to fit (hi) and known not to (lo); k = 0 is the CSS size
    const st = es.filter(e => e.clientWidth).map(e => ({ e, f0: parseFloat(getComputedStyle(e).fontSize) }))
      .filter(x => !fits(x.e)).map(x => { let kmax = 0; while (x.f0 - 0.5 * kmax > 9) kmax++; return { ...x, lo: 0, hi: kmax }; });
    // steps lo (doesn't fit) .. hi (fits, or the floor): the answer is the smallest step that fits, else hi
    let open = st.filter(x => x.hi - x.lo > 1);
    while (open.length) {
      for (const x of open) { x.mid = (x.lo + x.hi) >> 1; x.e.style.fontSize = (x.f0 - 0.5 * x.mid) + 'px'; }
      for (const x of open) { if (fits(x.e)) x.hi = x.mid; else x.lo = x.mid; }
      open = open.filter(x => x.hi - x.lo > 1);
    }
    for (const x of st) if (x.hi > 0) x.e.style.fontSize = (x.f0 - 0.5 * x.hi) + 'px';
  });
}
addEventListener('resize', () => fitSparks());   // the tab on show, kept across redraws (Inspiration first: it is where her sparks are entered)
const GRADES = 'GFEDCBAS';
// aptitude rows (index into the solver's `apt` order: short, mile, middle, long, turf, dirt, front, pace, late, end)
const APT_ROWS = [['Track', [[4, 'Turf'], [5, 'Dirt']]], ['Distance', [[0, 'Sprint'], [1, 'Mile'], [2, 'Medium'], [3, 'Long']]],
  ['Style', [[6, 'Front'], [7, 'Pace'], [8, 'Late'], [9, 'End']]]];
// a grade letter (code 1..8 = G..S): the game's sprite, or the letter
function gradeEl(g) {
  const s = srcFor('ui', 'grade_' + (g - 1)), letter = GRADES[g - 1] || '?';
  if (s) { const im = new Image(); im.className = 'udg'; im.src = s; im.alt = letter; return im; }
  return el('b', 'udg txt g' + letter, letter);
}
const when = (t) => `${['Junior', 'Classic', 'Senior'][Math.floor(t / 24)]} ${(t % 2 ? 'Late ' : 'Early ')}${['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'][Math.floor((t % 24) / 2)]}`;

// d: the veteran (chara, outfit, sparks, pinks, wins, parents); outfit: her outfit's label; apt: her ten grades at the
// end of this career (null: unknown); races: the G1s her schedule runs [{ix, name, turns}]; raceName(ix); onEdit: an edit
// to her sparks; winsBtn: the G1 wins editor's button; parentLabel(j): a parent's place ("#3")
export function detailsEl({ d, outfit, apt, races, raceName, onEdit, winsBtn, parentLabel, targets = [], deck = null, legacies = [] }) {
  const card = el('div', 'ud');
  // her portrait, outfit and name
  const top = el('div', 'udtop');
  const por = el('div', 'udpor'); const face = umaAv(d.chara, d.outfit, 96);
  face.setAttribute('role', 'img'); face.setAttribute('aria-label', outfit ? `${umaName(d.chara)} ${outfit}` : umaName(d.chara));
  por.appendChild(face); top.appendChild(por);
  const nm = el('div', 'udname'); if (outfit) nm.appendChild(el('div', 'udout', /^\[.*\]$/.test(outfit) ? outfit : `[${outfit}]`)); nm.appendChild(el('div', 'udnm', umaName(d.chara)));
  top.appendChild(nm); card.appendChild(top);
  // her aptitudes
  if (apt) {
    const g = el('div', 'udapt'); g.setAttribute('role', 'table'); g.setAttribute('aria-label', 'Her aptitudes');
    for (const [label, cells] of APT_ROWS) {
      const row = el('div', 'udarow'); row.setAttribute('role', 'row');
      const h = el('span', 'udal', label); h.setAttribute('role', 'rowheader'); row.appendChild(h);
      for (const [k, name] of cells) { const c = el('span', 'udac'); c.setAttribute('role', 'cell'); c.append(el('span', 'udan', name), gradeEl(apt[k])); c.title = `${name} ${GRADES[apt[k] - 1]}`; row.appendChild(c); }
      g.appendChild(row);
    }
    card.appendChild(g);
  }
  // the tabs
  const tabs = el('div', 'udtabs'); tabs.setAttribute('role', 'tablist'); tabs.setAttribute('aria-label', 'Her details');
  const panes = el('div', 'udpanes');
  const defs = [['skills', 'Skills', skillsPane], ['insp', 'Inspiration', inspPane], ['info', 'Career Info', infoPane]];
  const btns = [];
  for (const [key, label, make] of defs) {
    const b = el('button', 'udtab', label); b.type = 'button'; b.id = 'udtab-' + key; b.setAttribute('role', 'tab'); b.dataset.k = 'sim:tab:' + key;
    b.setAttribute('aria-selected', String(tab === key)); b.setAttribute('aria-controls', 'udpane-' + key); b.tabIndex = tab === key ? 0 : -1;
    const pane = make({ d, races, raceName, onEdit, winsBtn, parentLabel, targets, deck, legacies }); pane.id = 'udpane-' + key; pane.classList.add('udpane'); pane.setAttribute('role', 'tabpanel');
    pane.setAttribute('aria-labelledby', b.id); pane.hidden = tab !== key;
    b.onclick = () => { tab = key; for (const [x, bb, pp] of btns) { const on = x === key; bb.setAttribute('aria-selected', String(on)); bb.tabIndex = on ? 0 : -1; pp.hidden = !on; } fitSparks(card); };
    b.onkeydown = (e) => { const i = defs.findIndex(x => x[0] === key), n = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
      if (!n) return; e.preventDefault(); const nb = btns[(i + n + btns.length) % btns.length][1]; nb.click(); nb.focus(); };
    btns.push([key, b, pane]); tabs.appendChild(b); panes.appendChild(pane);
  }
  card.append(tabs, panes);
  return card;
}

// Skills: every target skill she could acquire this career, as the game's skill tiles (name only): acquired in this
// result bright (gold tile for the gold version), the others greyed. A tile opens the game-style skill popup: acquired
// or not, where it can come from (her outfit's own skill, a deck card's gold or white hint, her story events,
// inspiration from the legacies, the scenario) with each chance, and how likely it is acquired and lands as a spark.
const pctS = (p) => p >= 0.995 ? 'always' : p < 0.005 ? '<1%' : Math.round(p * 100) + '%';
function skillsPane({ d, targets }) {
  const p = el('div'), grid = el('div', 'udskills'); let na = 0, n = 0;
  for (const t of targets) {
    const b = t.buy; if (!b || b.gold + b.white <= 0) continue; n++;
    const got = (d.bought || {})[t.group] || ((d.sparks[t.group] || 0) > 0 ? (b.land_gold > b.land_white ? 'gold' : 'white') : null);
    const gold = got === 'gold', s = SK.get(t.group) || { w: 'Skill ' + t.group, gd: null };
    const name = got ? (gold && s.gd ? s.gd : s.w) : (b.gold > b.white && s.gd ? s.gd : s.w);
    if (got) na++;
    const tile = el('button', 'udsk' + (gold ? ' gold' : '') + (got ? '' : ' off')); tile.type = 'button'; tile.dataset.k = 'sim:sk' + t.group;
    const ic = skillIcon(t.group, gold || (!got && b.gold > b.white)); if (ic) tile.appendChild(ic);
    tile.appendChild(el('span', 'udskn', name));
    tile.setAttribute('aria-haspopup', 'dialog'); tile.setAttribute('aria-label', `${name}: ${got ? 'acquired' + (gold ? ' (gold)' : '') : 'not acquired'}. Details`);
    tile.onclick = () => openSkill({ t, b, name, got, icon: () => skillIcon(t.group, gold || (!got && b.gold > b.white)) }, tile);
    grid.appendChild(tile);
  }
  if (!n) p.appendChild(el('p', 'udnone', 'No target skill can be acquired in this career.'));
  else { p.appendChild(grid); p.appendChild(el('p', 'udfoot', `${na} of ${n} target skills acquired. Tap a skill for where it comes from.`)); }
  return p;
}
// the game-style skill popup (one dialog, refilled)
let skDlg = null;
function openSkill({ t, b, name, got, icon }, from) {
  if (!skDlg) {
    skDlg = el('dialog', 'gmdlg'); skDlg.id = 'sk-dlg'; skDlg.setAttribute('aria-labelledby', 'sk-title');
    skDlg.addEventListener('close', () => skDlg._from?.focus());
    document.body.appendChild(skDlg);
  }
  skDlg._from = from; skDlg.textContent = '';
  const h = el('h3', 'gmdhdr', 'Skill Details'); h.id = 'sk-title'; skDlg.appendChild(h);
  const body = el('div', 'gmdbody');
  const tile = el('div', 'udsk' + (got === 'gold' ? ' gold' : '') + (got ? '' : ' off')); const ic = icon(); if (ic) tile.appendChild(ic); tile.appendChild(el('span', 'udskn', name));
  body.appendChild(tile);
  body.appendChild(el('p', 'gmdstat' + (got ? ' on' : ''), got ? `Acquired${got === 'gold' ? ' (gold)' : ''} in this result` : 'Not acquired in this result'));
  const rows = [];
  if (b.scenario) rows.push(['The scenario', 1]);
  if (b.outfit === 2) rows.push(['Her own skill (gold)', 1]); else if (b.outfit === 1) rows.push(['Her own skill', 1]);
  if (b.deck_gold > 0.005) rows.push(['Deck: a gold hint', b.deck_gold]);
  if (b.deck_white > 0.005) rows.push(['Deck: a hint', b.deck_white]);
  if (b.own_event > 0.005) rows.push(['Her story events', b.own_event]);
  if (b.inherit > 0.005) rows.push(['Inspiration (the legacies)', b.inherit]);
  body.appendChild(el('div', 'gmdsub', 'Where it comes from'));
  const ul = el('ul', 'gmdlist');
  for (const [k, v] of rows) { const li = el('li'); li.append(el('span', '', k), el('b', '', pctS(v))); ul.appendChild(li); }
  body.appendChild(ul);
  const sum = el('div', 'gmdlist two');
  for (const [k, v] of [['Acquired', b.gold + b.white], ['Lands as a spark', b.land_gold + b.land_white]]) { const x = el('div'); x.append(el('span', '', k), el('b', '', pctS(v))); sum.appendChild(x); }
  body.appendChild(sum);
  skDlg.appendChild(body);
  const close = el('button', 'btn gmclose', 'Close'); close.type = 'button'; close.onclick = () => skDlg.close();
  const f = el('div', 'gmdfoot'); f.appendChild(close); skDlg.appendChild(f);
  skDlg.showModal(); close.focus();
}

// Inspiration: "Sparks" (hers, editable), then "Legacy Origin" (her two parents')
function inspPane({ d, onEdit, parentLabel }) {
  const p = el('div');
  p.appendChild(inspView({ d, key: 'sim', onEdit, meClass: 'simwho', parentLabel, parentKey: null }));
  return p;
}
// The game's Inspiration view of a veteran: "Sparks", her sparks as the game's spark bars (stars on their edge), then
// "Legacy Origin": her parents' (each a row with her portrait). key: her sparks' control names; parentKey(j): a parent's
// (null: her parents' sparks read-only); onEdit / onEditParent: an edit. Also the tree's node popups (career.js).
export function inspView({ d, key, onEdit, meClass = '', parentLabel = null, parentKey = null, onEditParent, header = true }) {
  const p = el('div', 'udinsp');
  if (header) p.appendChild(el('div', 'udhdr', 'Sparks'));
  const lin = el('div', 'udlin' + (d.parents?.length ? '' : ' solo'));
  const mine = el('div', 'udrow me' + (meClass ? ' ' + meClass : ''));
  const face = umaAv(d.chara || 0, d.outfit, 64); face.classList.add('udface'); mine.appendChild(face);
  const sp = el('div', 'simskills'); sp.setAttribute('role', 'group'); sp.setAttribute('aria-label', 'Her sparks');
  sp.appendChild(sparksEl(d, key, onEdit)); mine.appendChild(sp); lin.appendChild(mine);
  if (d.parents?.length) {
    const lo = el('div', 'udorigin'); lo.appendChild(el('span', '', 'Legacy Origin')); lin.appendChild(lo);
    d.parents.forEach((g, j) => {
      const row = el('div', 'udrow par');
      const f = umaAv(g.chara || 0, g.outfit, 64); f.classList.add('udface'); f.setAttribute('role', 'img');
      f.setAttribute('aria-label', `${parentLabel ? 'Parent ' + parentLabel(j) : 'Her parent'}: ${umaName(g.chara)}`);
      row.appendChild(f);
      const s = el('div', 'udpsp'); s.appendChild(parentKey ? sparksEl(g, parentKey(j), onEditParent) : sparksEl(g, `simp${j}`, undefined, true)); row.appendChild(s);
      lin.appendChild(row);
    });
  }
  p.appendChild(lin);
  return p;
}

// Career Info, as the game's: her support cards, her two legacies (each with her parents, the borrowed one a guest),
// and her career record (G1 races entered and won; the race-by-race record behind a button, with the G1 wins editor)
let showRaces = false;
function infoPane({ d, races, raceName, winsBtn, deck, legacies, parentLabel }) {
  const p = el('div');
  if (deck) {
    p.appendChild(el('div', 'udsub', 'Support Cards'));
    const row = el('div', 'udcards deck');
    deck.cards.forEach((nm, i) => {
      const bor = /\(borrowed\)$/.test(nm), base = nm.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, '').replace(/ \(link \d+: [^)]*\)$/, '');
      const id = deck.card_ids[i] ?? deck.plain_ids?.[i - deck.card_ids.length], c = CARD_BY_ID.get(id) || CARDS.find(x => x.name === base);
      const cell = el('button', 'udcard dc' + (bor ? ' bor' : '')); cell.type = 'button'; cell.appendChild(cardAv(c || null, 64)); cell.title = base + (bor ? ' (borrowed)' : '');
      if (c) cell.dataset.cid = c.id; cell.dataset.name = base; cell.setAttribute('aria-label', base + (bor ? ' (borrowed)' : '') + '. Card details'); cell.setAttribute('aria-haspopup', 'dialog');
      if (bor) cell.appendChild(el('span', 'udfr', 'Friends'));
      row.appendChild(cell);
    });
    p.appendChild(row);
  }
  if (legacies.length) {
    p.appendChild(el('div', 'udsub', 'Legacy Umamusume'));
    const lg = el('div', 'udlegs');
    legacies.forEach(({ v, label, open }, j) => {
      const box = el('div', 'udleg l' + (j + 1)); box.appendChild(el('span', 'udlegh', `Legacy ${j + 1}`));
      const tree = el('div', 'udlegt');
      const me = el('button', 'udlegp'); me.type = 'button'; me.setAttribute('aria-haspopup', 'dialog'); me.setAttribute('aria-label', `Legacy ${j + 1}: ${umaName(v.chara)} (${label}). Her sparks`);
      if (open) me.onclick = open;
      const f = umaAv(v.chara, v.outfit, 64);
      me.appendChild(f); me.appendChild(el('span', 'udlegn', `${label} ${umaName(v.chara)}`)); if (v.borrowed) me.appendChild(el('span', 'udguest', 'Guest'));
      const gps = el('div', 'udleggp');
      for (const g of v.parents) { const x = el('div', 'udleggx'); const gf = umaAv(g.chara || 0, g.outfit, 40); gf.setAttribute('role', 'img'); gf.setAttribute('aria-label', umaName(g.chara)); x.append(gf, el('span', '', umaName(g.chara))); gps.appendChild(x); }
      tree.append(me, gps); box.appendChild(tree); lg.appendChild(box);
      // the whole box opens her popup, as her portrait does
      if (open) { box.classList.add('pnclick'); box.addEventListener('click', (e) => { if (!e.target.closest('button')) me.click(); }); }
    });
    p.appendChild(lg);
  }
  p.appendChild(el('div', 'udsub', 'Career'));
  const ran = races.length, won = d.wins.length, inSched = new Set(races.map(r => r.ix));
  const rec = el('div', 'udrec'); rec.append(el('span', 'udrecl', 'Career Record'), el('span', 'udstat', `G1 races: ${ran}`), el('span', 'udstat', `Wins: ${won}`));
  p.appendChild(rec);
  const tg = el('button', 'btn udmore', showRaces ? 'Hide the races' : 'Show the races'); tg.type = 'button'; tg.setAttribute('aria-expanded', String(showRaces)); tg.dataset.k = 'sim:races';
  const list = el('ol', 'udraces'); list.hidden = !showRaces; list.id = 'ud-races';
  tg.setAttribute('aria-controls', 'ud-races');
  tg.onclick = () => { showRaces = !showRaces; list.hidden = !showRaces; tg.textContent = showRaces ? 'Hide the races' : 'Show the races'; tg.setAttribute('aria-expanded', String(showRaces)); };
  const rows = races.map(r => ({ t: r.turns[0], name: r.name, won: d.wins.includes(r.ix), ix: r.ix }))
    .concat(d.wins.filter(ix => !inSched.has(ix)).map(ix => ({ t: null, name: raceName(ix), won: true, ix })));
  rows.sort((a, b) => (a.t ?? 99) - (b.t ?? 99));
  for (const r of rows) {
    const li = el('li', 'udrace' + (r.won ? ' won' : ''));
    li.append(el('span', 'udrw', r.t == null ? 'Entered here' : when(r.t)), el('span', 'udgr', 'G1'), el('span', 'udrn', r.name), el('span', 'udres', r.won ? '1st' : '–'));
    li.lastChild.setAttribute('aria-label', r.won ? 'won' : 'not won');
    list.appendChild(li);
  }
  const act = el('div', 'udact'); act.append(tg, winsBtn); p.appendChild(act);
  p.appendChild(list);
  return p;
}
