// ---------- the veteran card: the game's "Umamusume Details" screen ----------
// Her portrait and outfit, her aptitude grades (Track / Distance / Style, the game's letters), then three tabs as in the
// game: Skills (the target skills she learned), Inspiration (her sparks as the game's spark bars, stars under each, then
// "Legacy Origin": her two parents' sparks) and Career Info (her support cards, her two legacies, the G1s she ran and
// won). No stats (the model has none).
// The game's own art where the build shipped it (images.py: grade letters, spark bars and bullets, skill tiles), the
// same shapes in CSS without it. Her sparks stay editable (vets.js sparksEl: click for stars, x removes, + adds).
import { el } from './dom.js';
import { SK, CARD_BY_ID, CARDS } from './data.js';
import { srcFor, umaAv, skillIcon, cardAv } from './images.js';
import { umaName, sparksEl } from './vets.js';
/** @import { LineageNode } from './vets.js' */
import { borLast } from './cards.js';

let tab = 'insp'; // the tab on show, kept across redraws (Inspiration first: it is where her sparks are entered)
// a spark bar's name on one line, its type shrunk until it fits (the game's way), down to 9 px; panes on show only
// Each name takes the largest size f0 - 0.5k that fits, or the first one at or under 9 px (a step-by-step shrink). All
// names are sized together: every round writes each unsettled name's next size to try, then reads them all (one layout
// per round, not one per step per name), halving the range of steps left (the width only grows with the size).
const fitting = new Set(); // roots with a pass already queued (resize fires many times a frame)
/** @param {ParentNode} [root] */
export function fitSparks(root = document) {
  if (fitting.has(root)) return;
  fitting.add(root);
  requestAnimationFrame(() => {
    fitting.delete(root);
    const names = [...root.querySelectorAll('.ud .vskn')];
    for (const e of names) e.style.fontSize = '';
    const fits = (e) => !(e.scrollWidth > e.clientWidth + 0.5);
    // per name that overflows (and is on show): base size f0 in px, the steps known not to fit (lo) and known to fit, or
    // the floor (hi); step k is the size f0 - 0.5k, and k = 0 is the CSS size
    /** @type {{e: HTMLElement, f0: number, lo: number, hi: number, mid?: number}[]} */
    const shrinking = names
      .filter((e) => e.clientWidth)
      .map((e) => ({ e, f0: parseFloat(getComputedStyle(e).fontSize) }))
      .filter((x) => !fits(x.e))
      .map((x) => {
        let floorStep = 0;
        while (x.f0 - 0.5 * floorStep > 9) floorStep++;
        return { ...x, lo: 0, hi: floorStep };
      });
    // the answer is the smallest step that fits, else hi
    let open = shrinking.filter((x) => x.hi - x.lo > 1);
    while (open.length) {
      for (const x of open) {
        x.mid = (x.lo + x.hi) >> 1;
        x.e.style.fontSize = `${x.f0 - 0.5 * x.mid}px`;
      }
      for (const x of open) {
        if (fits(x.e)) x.hi = x.mid;
        else x.lo = x.mid;
      }
      open = open.filter((x) => x.hi - x.lo > 1);
    }
    for (const x of shrinking) if (x.hi > 0) x.e.style.fontSize = `${x.f0 - 0.5 * x.hi}px`;
  });
}
addEventListener('resize', () => fitSparks());
const GRADES = 'GFEDCBAS';
// aptitude rows; `at` indexes the solver's `apt` order: short, mile, middle, long, turf, dirt, front, pace, late, end
const APT_ROWS = [
  {
    label: 'Track',
    cells: [
      { at: 4, name: 'Turf' },
      { at: 5, name: 'Dirt' },
    ],
  },
  {
    label: 'Distance',
    cells: [
      { at: 0, name: 'Sprint' },
      { at: 1, name: 'Mile' },
      { at: 2, name: 'Medium' },
      { at: 3, name: 'Long' },
    ],
  },
  {
    label: 'Style',
    cells: [
      { at: 6, name: 'Front' },
      { at: 7, name: 'Pace' },
      { at: 8, name: 'Late' },
      { at: 9, name: 'End' },
    ],
  },
];
// a grade letter (code 1..8 = G..S): the game's sprite, or the letter
function gradeEl(g) {
  const s = srcFor('ui', `grade_${g - 1}`),
    letter = GRADES[g - 1] || '?';
  if (s) {
    const im = new Image();
    im.className = 'udg';
    im.src = s;
    im.alt = letter;
    return im;
  }
  return el('b', `udg txt g${letter}`, letter);
}
// a turn (0-71: 24 half-months a year) as "Classic Late Apr"
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const when = (turn) =>
  `${['Junior', 'Classic', 'Senior'][Math.floor(turn / 24)]} ${turn % 2 ? 'Late ' : 'Early '}${MONTHS[Math.floor((turn % 24) / 2)]}`;

/**
 * @typedef {object} DetailsSpec
 * @property {LineageNode & {parents: LineageNode[], bought?: Object<number, string>}} d  the veteran
 * @property {string|null} outfit  her outfit's label
 * @property {number[]|null} apt  her ten grades at the end of this career (codes 1..8 = G..S; null: unknown)
 * @property {{ix: number, name: string, p: number, turns: number[]}[]} races  the G1s her schedule runs, each with her
 *   chance of winning it at least once
 * @property {(ix: number) => string} raceName
 * @property {() => void} onEdit  after an edit of her sparks
 * @property {HTMLElement} winsBtn  the G1 wins editor's button
 * @property {(j: number) => string} parentLabel  a parent's place ("#3")
 * @property {import('./career.js').CareerTarget[]} [targets]  the career's per-target report
 * @property {import('./results.js').ResultDeck|null} [deck]  her deck
 * @property {{v: import('./vets.js').Veteran, label: string, open: () => void}[]} [legacies]  her two parents, each with
 *   her place and what opens her node
 */
/** @param {DetailsSpec} spec */
export function detailsEl({
  d,
  outfit,
  apt,
  races,
  raceName,
  onEdit,
  winsBtn,
  parentLabel,
  targets = [],
  deck = null,
  legacies = [],
}) {
  const card = el('div', 'ud');
  // her portrait, outfit and name
  const top = el('div', 'udtop');
  const por = el('div', 'udpor');
  const face = umaAv(d.chara, d.outfit, 96);
  face.setAttribute('role', 'img');
  face.setAttribute('aria-label', outfit ? `${umaName(d.chara)} ${outfit}` : umaName(d.chara));
  por.appendChild(face);
  top.appendChild(por);
  const nm = el('div', 'udname');
  if (outfit) nm.appendChild(el('div', 'udout', /^\[.*\]$/.test(outfit) ? outfit : `[${outfit}]`));
  nm.appendChild(el('div', 'udnm', umaName(d.chara)));
  top.appendChild(nm);
  card.appendChild(top);
  // her aptitudes
  if (apt) {
    const g = el('div', 'udapt');
    g.setAttribute('role', 'table');
    g.setAttribute('aria-label', 'Her aptitudes');
    for (const { label, cells } of APT_ROWS) {
      const row = el('div', 'udarow');
      row.setAttribute('role', 'row');
      const h = el('span', 'udal', label);
      h.setAttribute('role', 'rowheader');
      row.appendChild(h);
      for (const { at, name } of cells) {
        const c = el('span', 'udac');
        c.setAttribute('role', 'cell');
        c.append(el('span', 'udan', name), gradeEl(apt[at]));
        c.title = `${name} ${GRADES[apt[at] - 1]}`;
        row.appendChild(c);
      }
      g.appendChild(row);
    }
    card.appendChild(g);
  }
  // the tabs: each pane is built from the card's inputs
  const tabs = el('div', 'udtabs');
  tabs.setAttribute('role', 'tablist');
  tabs.setAttribute('aria-label', 'Her details');
  const panes = el('div', 'udpanes');
  const paneInput = { d, races, raceName, onEdit, winsBtn, parentLabel, targets, deck, legacies };
  const defs = [
    { key: 'skills', label: 'Skills', make: skillsPane },
    { key: 'insp', label: 'Inspiration', make: inspPane },
    { key: 'info', label: 'Career Info', make: infoPane },
  ];
  const shownTabs = []; // {key, button, pane} per tab, in order
  for (const { key, label, make } of defs) {
    const b = el('button', 'udtab', label);
    b.type = 'button';
    b.id = `udtab-${key}`;
    b.setAttribute('role', 'tab');
    b.dataset.k = `sim:tab:${key}`;
    b.setAttribute('aria-selected', String(tab === key));
    b.setAttribute('aria-controls', `udpane-${key}`);
    b.tabIndex = tab === key ? 0 : -1;
    const pane = make(paneInput);
    pane.id = `udpane-${key}`;
    pane.classList.add('udpane');
    pane.setAttribute('role', 'tabpanel');
    pane.setAttribute('aria-labelledby', b.id);
    pane.hidden = tab !== key;
    b.onclick = () => {
      tab = key;
      for (const other of shownTabs) {
        const on = other.key === key;
        other.button.setAttribute('aria-selected', String(on));
        other.button.tabIndex = on ? 0 : -1;
        other.pane.hidden = !on;
      }
      fitSparks(card);
    };
    b.onkeydown = (e) => {
      const i = defs.findIndex((x) => x.key === key),
        step = e.key === 'ArrowRight' ? 1 : e.key === 'ArrowLeft' ? -1 : 0;
      if (!step) return;
      e.preventDefault();
      const next = shownTabs[(i + step + shownTabs.length) % shownTabs.length].button;
      next.click();
      next.focus();
    };
    shownTabs.push({ key, button: b, pane });
    tabs.appendChild(b);
    panes.appendChild(pane);
  }
  card.append(tabs, panes);
  return card;
}

// Skills: every target skill she could acquire this career, as the game's skill tiles (name only): acquired in this
// result bright (gold tile for the gold version), the others greyed. A tile opens the game-style skill popup: acquired
// or not, where it can come from (her outfit's own skill, a deck card's gold or white hint, her story events,
// inspiration from the legacies, the scenario) with each chance, and how likely it is acquired and lands as a spark.
const pctS = (p) => (p >= 0.995 ? 'always' : p < 0.005 ? '<1%' : `${Math.round(p * 100)}%`);
function skillsPane({ d, targets }) {
  const p = el('div'),
    grid = el('div', 'udskills');
  let acquired = 0,
    acquirable = 0;
  for (const t of targets) {
    const b = t.buy;
    if (!b || b.gold + b.white <= 0) continue;
    acquirable++;
    // how she got it: bought in this result, else landed as a spark (gold when gold lands more often), else not at all
    const got =
      d.bought?.[t.group] || ((d.sparks[t.group] || 0) > 0 ? (b.land_gold > b.land_white ? 'gold' : 'white') : null);
    const gold = got === 'gold',
      skill = SK.get(t.group) || { w: `Skill ${t.group}`, gd: null };
    // the version shown: the one she got, else the one she would most likely get
    const goldShown = got ? gold : b.gold > b.white;
    const name = goldShown && skill.gd ? skill.gd : skill.w;
    if (got) acquired++;
    const tile = el('button', `udsk${gold ? ' gold' : ''}${got ? '' : ' off'}`);
    tile.type = 'button';
    tile.dataset.k = `sim:sk${t.group}`;
    const ic = skillIcon(t.group, goldShown);
    if (ic) tile.appendChild(ic);
    tile.appendChild(el('span', 'udskn', name));
    tile.setAttribute('aria-haspopup', 'dialog');
    tile.setAttribute('aria-label', `${name}: ${got ? `acquired${gold ? ' (gold)' : ''}` : 'not acquired'}. Details`);
    tile.onclick = () => openSkill({ b, name, got, icon: () => skillIcon(t.group, goldShown) }, tile);
    grid.appendChild(tile);
  }
  if (!acquirable) p.appendChild(el('p', 'udnone', 'No target skill can be acquired in this career.'));
  else {
    p.appendChild(grid);
    p.appendChild(
      el('p', 'udfoot', `${acquired} of ${acquirable} target skills acquired. Tap a skill for where it comes from.`),
    );
  }
  return p;
}
// the game-style skill popup (one dialog, refilled)
let skDlg = null;
// b: the target's buying report; got: 'gold' | 'white' | null (not acquired); icon() makes her skill icon; from: the
// tile that opened it (focused again on close)
function openSkill({ b, name, got, icon }, from) {
  if (!skDlg) {
    skDlg = el('dialog', 'gmdlg');
    skDlg.id = 'sk-dlg';
    skDlg.setAttribute('aria-labelledby', 'sk-title');
    skDlg.addEventListener('close', () => skDlg._from?.focus());
    document.body.appendChild(skDlg);
  }
  skDlg._from = from;
  skDlg.textContent = '';
  const h = el('h3', 'gmdhdr', 'Skill Details');
  h.id = 'sk-title';
  skDlg.appendChild(h);
  const body = el('div', 'gmdbody');
  const tile = el('div', `udsk${got === 'gold' ? ' gold' : ''}${got ? '' : ' off'}`);
  const ic = icon();
  if (ic) tile.appendChild(ic);
  tile.appendChild(el('span', 'udskn', name));
  body.appendChild(tile);
  body.appendChild(
    el(
      'p',
      `gmdstat${got ? ' on' : ''}`,
      got ? `Acquired${got === 'gold' ? ' (gold)' : ''} in this result` : 'Not acquired in this result',
    ),
  );
  // where it comes from, each with its chance (a source under half a percent isn't listed)
  const sources = [];
  if (b.scenario) sources.push({ label: 'The scenario', chance: 1 });
  if (b.outfit === 2) sources.push({ label: 'Her own skill (gold)', chance: 1 });
  else if (b.outfit === 1) sources.push({ label: 'Her own skill', chance: 1 });
  if (b.deck_gold > 0.005) sources.push({ label: 'Deck: a gold hint', chance: b.deck_gold });
  if (b.deck_white > 0.005) sources.push({ label: 'Deck: a hint', chance: b.deck_white });
  if (b.own_event > 0.005) sources.push({ label: 'Her story events', chance: b.own_event });
  if (b.inherit > 0.005) sources.push({ label: 'Inspiration (the legacies)', chance: b.inherit });
  body.appendChild(el('div', 'gmdsub', 'Where it comes from'));
  const ul = el('ul', 'gmdlist');
  for (const { label, chance } of sources) {
    const li = el('li');
    li.append(el('span', '', label), el('b', '', pctS(chance)));
    ul.appendChild(li);
  }
  body.appendChild(ul);
  const sum = el('div', 'gmdlist two');
  for (const { label, chance } of [
    { label: 'Acquired', chance: b.gold + b.white },
    { label: 'Lands as a spark', chance: b.land_gold + b.land_white },
  ]) {
    const x = el('div');
    x.append(el('span', '', label), el('b', '', pctS(chance)));
    sum.appendChild(x);
  }
  body.appendChild(sum);
  skDlg.appendChild(body);
  const close = el('button', 'btn gmclose', 'Close');
  close.type = 'button';
  close.onclick = () => skDlg.close();
  const f = el('div', 'gmdfoot');
  f.appendChild(close);
  skDlg.appendChild(f);
  skDlg.showModal();
  close.focus();
}

// Inspiration: "Sparks" (hers, editable), then "Legacy Origin" (her two parents')
function inspPane({ d, onEdit, parentLabel }) {
  const p = el('div');
  p.appendChild(inspView({ d, key: 'sim', onEdit, meClass: 'simwho', parentLabel, parentKey: null }));
  return p;
}
// The game's Inspiration view of a veteran: "Sparks", her sparks as the game's spark bars (stars on their edge), then
// "Legacy Origin": her parents' (each a row with her portrait). Also the tree's node popups (career.js).
/**
 * @typedef {object} InspViewSpec
 * @property {LineageNode & {parents?: LineageNode[]}} d  the veteran shown (her parents: none, or her two)
 * @property {string} key  her sparks' control names
 * @property {() => void} onEdit  after an edit of her sparks
 * @property {string} [meClass]  an extra class on her row
 * @property {((j: number) => string)|null} [parentLabel]  parent j's label (null: "Her parent")
 * @property {((j: number) => string)|null} [parentKey]  parent j's sparks' control names (null: her parents' read-only)
 * @property {() => void} [onEditParent]  after an edit of a parent's sparks
 * @property {boolean} [header]  the "Sparks" heading
 */
/** @param {InspViewSpec} spec */
export function inspView({
  d,
  key,
  onEdit,
  meClass = '',
  parentLabel = null,
  parentKey = null,
  onEditParent,
  header = true,
}) {
  const p = el('div', 'udinsp');
  if (header) p.appendChild(el('div', 'udhdr', 'Sparks'));
  const lin = el('div', `udlin${d.parents?.length ? '' : ' solo'}`);
  const mine = el('div', `udrow me${meClass ? ` ${meClass}` : ''}`);
  const face = umaAv(d.chara || 0, d.outfit, 64);
  face.classList.add('udface');
  mine.appendChild(face);
  const sp = el('div', 'simskills');
  sp.setAttribute('role', 'group');
  sp.setAttribute('aria-label', 'Her sparks');
  sp.appendChild(sparksEl(d, key, onEdit));
  mine.appendChild(sp);
  lin.appendChild(mine);
  if (d.parents?.length) {
    const lo = el('div', 'udorigin');
    lo.appendChild(el('span', '', 'Legacy Origin'));
    lin.appendChild(lo);
    d.parents.forEach((g, j) => {
      const row = el('div', 'udrow par');
      const f = umaAv(g.chara || 0, g.outfit, 64);
      f.classList.add('udface');
      f.setAttribute('role', 'img');
      f.setAttribute('aria-label', `${parentLabel ? `Parent ${parentLabel(j)}` : 'Her parent'}: ${umaName(g.chara)}`);
      row.appendChild(f);
      const s = el('div', 'udpsp');
      s.appendChild(parentKey ? sparksEl(g, parentKey(j), onEditParent) : sparksEl(g, `simp${j}`, undefined, true));
      row.appendChild(s);
      lin.appendChild(row);
    });
  }
  p.appendChild(lin);
  return p;
}

// A deck's i-th card, from the solver's deck report: its name with the report's notes taken off (base), whether it is the
// borrowed card, and the card's data (null when unknown). The report lists the deck's cards with ids (card_ids) and
// then the plain ones (plain_ids), in the order of `cards`.
/** @param {import('./results.js').ResultDeck} deck @param {number} i @returns {{base: string, borrowed: boolean, card: import('./data.js').Card|undefined}} */
export function deckCardAt(deck, i) {
  const name = deck.cards[i];
  const borrowed = /\(borrowed\)$/.test(name),
    base = name
      .replace(/ \(borrowed\)$/, '')
      .replace(/ \(end chain at link \d\)$/, '')
      .replace(/ \(link \d+: [^)]*\)$/, '');
  const id = deck.card_ids[i] ?? deck.plain_ids?.[i - deck.card_ids.length];
  return { base, borrowed, card: CARD_BY_ID.get(id) || CARDS.find((x) => x.name === base) };
}
let showRaces = false;
// Career Info, as the game's: her support cards, her two legacies (each with her parents, the borrowed one a guest),
// and her career record (G1 races entered and won; the race-by-race record behind a button, with the G1 wins editor)
function infoPane({ d, races, raceName, winsBtn, deck, legacies }) {
  const p = el('div');
  if (deck) {
    p.appendChild(el('div', 'udsub', 'Support Cards'));
    const row = el('div', 'udcards deck');
    borLast(deck.cards).forEach((i) => {
      const { base, borrowed: bor, card: c } = deckCardAt(deck, i);
      const cell = el('button', `udcard dc${bor ? ' bor' : ''}`);
      cell.type = 'button';
      cell.appendChild(cardAv(c || null, 64));
      cell.title = base + (bor ? ' (borrowed)' : '');
      if (c) cell.dataset.cid = String(c.id);
      cell.dataset.name = base;
      cell.setAttribute('aria-label', `${base + (bor ? ' (borrowed)' : '')}. Card details`);
      cell.setAttribute('aria-haspopup', 'dialog');
      if (bor) cell.appendChild(el('span', 'udfr', 'Friends'));
      row.appendChild(cell);
    });
    p.appendChild(row);
  }
  if (legacies.length) {
    p.appendChild(el('div', 'udsub', 'Legacy Umamusume'));
    const lg = el('div', 'udlegs');
    legacies.forEach(({ v, label, open }, j) => {
      const box = el('div', `udleg l${j + 1}`);
      box.appendChild(el('span', 'udlegh', `Legacy ${j + 1}`));
      const tree = el('div', 'udlegt');
      const me = el('button', 'udlegp');
      me.type = 'button';
      me.setAttribute('aria-haspopup', 'dialog');
      me.setAttribute('aria-label', `Legacy ${j + 1}: ${umaName(v.chara)} (${label}). Her sparks`);
      if (open) me.onclick = open;
      const f = umaAv(v.chara, v.outfit, 64);
      me.appendChild(f);
      me.appendChild(el('span', 'udlegn', `${label} ${umaName(v.chara)}`));
      if (v.borrowed) me.appendChild(el('span', 'udguest', 'Guest'));
      const gps = el('div', 'udleggp');
      for (const g of v.parents) {
        const x = el('div', 'udleggx');
        const gf = umaAv(g.chara || 0, g.outfit, 40);
        gf.setAttribute('role', 'img');
        gf.setAttribute('aria-label', umaName(g.chara));
        x.append(gf, el('span', '', umaName(g.chara)));
        gps.appendChild(x);
      }
      tree.append(me, gps);
      box.appendChild(tree);
      lg.appendChild(box);
      // the whole box opens her popup, as her portrait does
      if (open) {
        box.classList.add('pnclick');
        box.addEventListener('click', (/** @type {TargetedEvent} */ e) => {
          if (!e.target.closest('button')) me.click();
        });
      }
    });
    p.appendChild(lg);
  }
  p.appendChild(el('div', 'udsub', 'Career'));
  const ran = races.length,
    won = d.wins.length,
    inSched = new Set(races.map((r) => r.ix));
  const rec = el('div', 'udrec');
  rec.append(
    el('span', 'udrecl', 'Career Record'),
    el('span', 'udstat', `G1 races: ${ran}`),
    el('span', 'udstat', `Wins: ${won}`),
  );
  p.appendChild(rec);
  const tg = el('button', 'btn udmore', showRaces ? 'Hide the races' : 'Show the races');
  tg.type = 'button';
  tg.setAttribute('aria-expanded', String(showRaces));
  tg.dataset.k = 'sim:races';
  const list = el('ol', 'udraces');
  list.hidden = !showRaces;
  list.id = 'ud-races';
  tg.setAttribute('aria-controls', 'ud-races');
  tg.onclick = () => {
    showRaces = !showRaces;
    list.hidden = !showRaces;
    tg.textContent = showRaces ? 'Hide the races' : 'Show the races';
    tg.setAttribute('aria-expanded', String(showRaces));
  };
  // her schedule's G1s at their first turn, then the wins outside it (no turn: entered on the page)
  const entries = races
    .map((r) => ({ turn: r.turns[0], name: r.name, won: d.wins.includes(r.ix), ix: r.ix }))
    .concat(d.wins.filter((ix) => !inSched.has(ix)).map((ix) => ({ turn: null, name: raceName(ix), won: true, ix })));
  entries.sort((a, b) => (a.turn ?? 99) - (b.turn ?? 99));
  for (const r of entries) {
    const li = el('li', `udrace${r.won ? ' won' : ''}`);
    li.append(
      el('span', 'udrw', r.turn == null ? 'Entered here' : when(r.turn)),
      el('span', 'udgr', 'G1'),
      el('span', 'udrn', r.name),
      el('span', 'udres', r.won ? '1st' : '–'),
    );
    /** @type {HTMLElement} */ (li.lastChild).setAttribute('aria-label', r.won ? 'won' : 'not won');
    list.appendChild(li);
  }
  const act = el('div', 'udact');
  act.append(tg, winsBtn);
  p.appendChild(act);
  p.appendChild(list);
  return p;
}
