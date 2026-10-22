// ---------- race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all of them ----------
// One member (the career page): no "All three" / "All four" tab; `shared` then means a parent won the race (it builds affinity when
// she is a parent herself).
import { $, $dialog, esc } from './dom.js';
import { RACEIDS } from './data.js';
import { umaAv, srcFor } from './images.js';
import { pickOutfit } from './roster.js';

/**
 * One member's attempt at a scheduled race (the solver's schedule report).
 * @typedef {object} ScheduleCell
 * @property {number} p  her win chance, in percent
 * @property {number} [apt]  the aptitude cost (distance and surface grades), in percentage points
 * @property {number} [pen]  the penalty for racing several turns in a row, in percentage points
 * @property {number} [row]  which race in a row this is (1 = the first)
 * @property {boolean} goal  a goal race of her scenario
 *
 * One scheduled G1, one cell per member (null: she doesn't run it).
 * @typedef {object} ScheduleRow
 * @property {number} year  0 Junior, 1 Classic, 2 Senior
 * @property {string} when  the turn, as "Cl E-Apr" (year, Early or Late, month)
 * @property {string} race  its name; "A/B" when either of two races runs at the same odds
 * @property {boolean} shared  two or more members run it (it builds affinity); one member: a parent won it
 * @property {(ScheduleCell|null)[]} cells
 * @property {boolean[]} [parents]  one member: which of her parents won it
 * @property {number} runners  how many members run it
 * @property {string[]} [alts]  the other race of an "A/B" pair
 */

// a turn is one of 72 half-months: 24 per year (the year t / 24; its slot t % 24: month (t % 24) / 2, odd slots Late)
const YEARS = ['Junior', 'Classic', 'Senior'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
// a schedule row's `when` ("Cl E-Apr") as {late, month}, or null when it isn't one
const parseWhen = (when) => { const m = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(when); return m ? { late: m[2] === 'L', month: m[3] } : null; };
// its slot in the year (0-23), or -1
const slotOf = (when) => { const w = parseWhen(when); return w ? MONTHS.indexOf(w.month) * 2 + (w.late ? 1 : 0) : -1; };
// a race's banner (by its full or chart name), or null
const banner = (name) => { const id = RACEIDS[name], src = id && srcFor('race', id); return src ? Object.assign(new Image(), { className: 'banner', alt: '', loading: 'lazy', src }) : null; };
// the bar at the foot of a slot: "Early Apr", and the win odds of the race run there
function whenBar(slot, /** @type {ScheduleCell|null} */ cell = null) {
  const bar = document.createElement('div'); bar.className = 'ag-when';
  const label = document.createElement('span'); label.textContent = (slot % 2 ? 'Late ' : 'Early ') + MONTHS[slot >> 1]; bar.appendChild(label);
  if (cell) { const p = document.createElement('b'); p.textContent = `${cell.p}% win`; p.title = why(cell); bar.appendChild(p); }
  return bar;
}
// why a race isn't a sure win: 110 minus the aptitude cost (distance and surface grades) minus the penalty for the k-th
// race in a row, capped at 100% ('' when the cell doesn't say)
const ORD = ['', '1st', '2nd', '3rd', '4th', '5th', '6th or later'];
const why = (/** @type {ScheduleCell} */ cell) => {
  if (cell.apt === undefined) return '';
  const parts = [];
  if (cell.apt > 0) parts.push(`aptitude −${cell.apt}`);
  if (cell.pen > 0) parts.push(`${ORD[Math.min(cell.row, 6)]} race in a row −${cell.pen}`);
  const costs = parts.length ? `, ${parts.join(', ')}` : '', capped = 110 - cell.apt - cell.pen > 100 ? ', capped at 100' : '';
  return `${cell.p}% win: 110${costs}${capped}`;
};
// a race in its slot (the schedule window's cell, and a win in the G1 wins editor): banner (by `bannerName`), name, Goal badge
function raceCell(tag, { bannerName, label, goal }) {
  const el = document.createElement(tag); el.className = 'ag-race';
  const bn = banner(bannerName); if (bn) el.appendChild(bn);
  const nm = document.createElement('div'); nm.className = 'ag-nm'; nm.textContent = label; el.appendChild(nm);
  if (goal) { const g = document.createElement('span'); g.className = 'ag-goal'; g.textContent = 'Goal'; el.appendChild(g); }
  return el;
}
const turnName = (turn) => `${YEARS[(turn / 24) | 0]} Year, ${turn % 2 ? 'Late' : 'Early'} ${MONTHS[(turn % 24) >> 1]}`;
// the schedule window's calendar: three year columns of 24 half-month slots (the career starts in Junior Year, early
// July); fill(year, slot index, slot element) fills each slot
// On the planner pages (html.game) it is the game's Scheduled Races, all three years side by side: each year 4 x 6 slots, Junior's
// first three rows the game's grey cells for April to June (labelled as the game labels them, each half-month twice).
/** @param {(year: number, slot: number, cell: HTMLElement) => void} fill @returns {HTMLDivElement} */
export function agendaGrid(fill) {
  const game = document.documentElement.classList.contains('game');
  const ag = document.createElement('div'); ag.className = `agenda${game ? ' game' : ''}`;
  for (let y = 0; y < 3; y++) {
    const col = document.createElement('section'); col.className = `ag-year y${y}`;
    col.innerHTML = `<h4>${YEARS[y]} Year</h4>`;
    const grid = document.createElement('div'); grid.className = 'ag-grid';
    if (game && y === 0) for (let i = 0; i < 12; i++) {   // April to June, before the career: 4 cells a month
      const slot = document.createElement('div'); slot.className = 'ag-slot pre';
      const bar = document.createElement('div'); bar.className = 'ag-when'; const label = document.createElement('span');
      label.textContent = (i % 4 < 2 ? 'Early ' : 'Late ') + MONTHS[3 + (i >> 2)]; bar.appendChild(label); slot.appendChild(bar); grid.appendChild(slot);
    }
    for (let slotIndex = 0; slotIndex < 24; slotIndex++) {
      const slot = document.createElement('div');
      if (y === 0 && slotIndex < 12) {   // Junior January to June: one note fills the blank half-year
        if (!game && slotIndex === 0) { slot.className = 'ag-pre'; slot.textContent = 'The career starts in Junior Year, early July'; grid.appendChild(slot); }
        continue;
      }
      slot.className = 'ag-slot';
      fill(y, slotIndex, slot);
      grid.appendChild(slot);
    }
    col.appendChild(grid); ag.appendChild(col);
  }
  return ag;
}
// ---------- the G1 wins editor (a pool veteran's, and the veteran analysis' draft), on the same calendar ----------
// Each win sits in one slot as the schedule window's cell; a click removes it and the slot is empty. An empty slot where a
// G1 is held is a button: a menu of the races held then, and a pick marks one won there. A race held in Classic and Senior
// is one race (one win saddle): it shows once, at the turn picked for it (`at`, kept by the page for display only),
// else the turn her schedule runs it (`info`), else its first turn free of another win (an imported win has no turn).
/**
 * @typedef {object} WinsEditorOptions
 * @property {{ix: number, name: string, turns: number[]}[]} races  every G1: its race index, name and the turns it is held
 * @property {() => number[]} get  the race indices she won
 * @property {(wins: number[]) => void} set  replaces them (sorted, no repeats)
 * @property {() => Object<number, number>} at  race index -> the turn its win shows at (created on demand, edited in place)
 * @property {((turn: number) => number[]|null)|null} [offer]  the races the menu lists at a turn (null: every race held then)
 * @property {((ix: number) => {turn?: number, cell?: ScheduleCell, parent: boolean}|null)|null} [info]  the race in her
 *   schedule: its turn and its schedule cell with the win odds; `parent`: a parent won it too
 * @property {() => void} onEdit  after every change
 */
let menuOpen = null;   // {menu, opener}: one menu at a time
function closeMenu(refocus) {
  if (!menuOpen) return; const { menu, opener } = menuOpen; menuOpen = null;
  menu.hidden = true; opener.setAttribute('aria-expanded', 'false'); if (refocus && opener.isConnected) opener.focus();
}
document.addEventListener('pointerdown', (e) => { if (menuOpen && !menuOpen.menu.contains(e.target) && !menuOpen.opener.contains(e.target)) closeMenu(false); }, true);
/** @param {WinsEditorOptions} opts */
export function winsAgenda(opts) {
  const raceName = new Map(opts.races.map(r => [r.ix, r.name])), raceTurns = new Map(opts.races.map(r => [r.ix, r.turns]));
  const heldAt = new Map();   // turn -> the races held then
  for (const r of opts.races) for (const t of r.turns) { if (!heldAt.has(t)) heldAt.set(t, []); heldAt.get(t).push(r.ix); }
  const info = (ix) => (opts.info?.(ix)) || null;
  const box = document.createElement('div'); box.className = 'winsed';
  const menu = document.createElement('div'); menu.className = 'cpop wmenu'; menu.hidden = true; menu.setAttribute('role', 'dialog'); menu.tabIndex = -1;
  // where each win shows: byTurn, turn -> [ix] (two wins on one turn only when the record says so; they share the slot);
  // offCalendar, the wins of races with no turn
  function layout() {
    const at = opts.at(), byTurn = new Map(), offCalendar = [];
    // wins placed by the page first, then those her schedule runs, then the rest (each in race order)
    const rank = (ix) => (raceTurns.get(ix) || []).includes(at[ix]) ? 0 : info(ix)?.turn !== undefined ? 1 : 2;
    for (const ix of [...opts.get()].sort((a, b) => rank(a) - rank(b) || a - b)) {
      const turns = raceTurns.get(ix) || []; if (!turns.length) { offCalendar.push(ix); continue; }
      const preferred = [at[ix], info(ix)?.turn, ...turns].filter(t => turns.includes(t));
      const t = preferred.find(t => !byTurn.has(t)) ?? preferred[0];
      if (!byTurn.has(t)) byTurn.set(t, []); byTurn.get(t).push(ix);
    }
    return { byTurn, offCalendar };
  }
  const edit = (wins) => { opts.set([...new Set(wins)].sort((a, b) => a - b)); opts.onEdit(); };
  const remove = (ix) => { delete opts.at()[ix]; edit(opts.get().filter(x => x !== ix)); };
  // a win in its slot: a button that removes it
  function winButton(ix, turn) {
    const name = raceName.get(ix), scheduled = info(ix), inSchedule = scheduled && scheduled.turn === turn;
    const b = raceCell('button', { bannerName: name, label: name, goal: inSchedule && scheduled.cell.goal });
    b.type = 'button'; b.classList.add('wbtn'); b.dataset.k = `win:${ix}`; b.dataset.ix = ix;
    b.setAttribute('aria-label', `${name}, won: remove`);
    const odds = inSchedule ? ` · ${why(scheduled.cell) || `${scheduled.cell.p}% win`}` : '';
    b.title = `${name} · won${odds}${scheduled?.parent ? ' · a parent won it too' : ''} · click to remove`;
    b.onclick = () => { remove(ix); draw(`add:${turn}`); };
    return b;
  }
  // an empty slot's "+": the menu of the races held then
  function addButton(turn, offered, shownAt) {
    const a = document.createElement('button'); a.type = 'button'; a.className = 'ag-add'; a.dataset.k = `add:${turn}`;
    a.innerHTML = '<span aria-hidden="true">+</span>'; a.setAttribute('aria-label', `${turnName(turn)}: add a G1 win`); a.title = 'Add a G1 win';
    a.setAttribute('aria-haspopup', 'dialog'); a.setAttribute('aria-expanded', 'false');
    a.onclick = () => (menuOpen && menuOpen.opener === a ? closeMenu(false) : openMenu(a, turn, offered, shownAt));
    return a;
  }
  // a win with no turn on the calendar (not in the race data): listed, still removable
  function offCalendarNote(offCalendar) {
    const p = document.createElement('p'); p.className = 'hint'; p.textContent = 'Not on the calendar: ';
    for (const ix of offCalendar) {
      const name = raceName.get(ix) || `Race ${ix}`;
      const b = document.createElement('button'); b.type = 'button'; b.className = 'btn'; b.dataset.k = `win:${ix}`;
      b.textContent = `${name} ×`; b.setAttribute('aria-label', `${name}, won: remove`); b.onclick = () => { remove(ix); draw(); }; p.appendChild(b);
    }
    return p;
  }
  // redraw the calendar; focusKey: the control (data-k) to focus afterwards
  function draw(focusKey = null) {
    closeMenu(false);
    const { byTurn, offCalendar } = layout(), shownAt = new Map(); for (const [t, list] of byTurn) for (const ix of list) shownAt.set(ix, t);
    const ag = agendaGrid((y, slotIndex, slot) => {
      const turn = y * 24 + slotIndex, here = byTurn.get(turn) || [];
      if (here.length) slot.classList.add('on');
      if (here.length > 1) slot.classList.add('many');
      for (const ix of here) slot.appendChild(winButton(ix, turn));
      const offered = here.length ? [] : (opts.offer ? opts.offer(turn) : heldAt.get(turn)) || [];
      if (offered.length) { slot.classList.add('free'); slot.appendChild(addButton(turn, offered, shownAt)); }
      const only = here.length === 1 ? info(here[0]) : null;
      slot.appendChild(whenBar(slotIndex, only && only.turn === turn ? only.cell : null));
    });
    box.textContent = ''; box.appendChild(ag);
    if (offCalendar.length) box.appendChild(offCalendarNote(offCalendar));
    box.appendChild(menu);
    if (focusKey) (box.querySelector(`[data-k="${CSS.escape(focusKey)}"]`) || box.querySelector('button'))?.focus();
  }
  // the races held at turn t: her schedule's first, then by name; a race won at another turn moves here
  function openMenu(opener, turn, offered, shownAt) {
    closeMenu(false); menuOpen = { menu, opener }; opener.setAttribute('aria-expanded', 'true');
    menu.textContent = ''; menu.setAttribute('aria-label', `${turnName(turn)}: G1s held`);
    const h = document.createElement('p'); h.className = 'hint'; h.textContent = `${turnName(turn)}: which G1 did she win?`; menu.appendChild(h);
    const inSchedule = (ix) => info(ix)?.turn === turn;
    for (const ix of [...offered].sort((x, y) => Number(inSchedule(y)) - Number(inSchedule(x)) || raceName.get(x).localeCompare(raceName.get(y)))) {
      // each race as its banner (the name stays for the screen reader and when the banner is missing)
      const b = document.createElement('button'); b.type = 'button'; b.className = 'btn wpick'; b.title = raceName.get(ix);
      const bn = banner(raceName.get(ix)); if (bn) { bn.loading = 'eager'; b.appendChild(bn); b.classList.add('art'); }
      const nm = document.createElement('span'); nm.className = 'wpn'; nm.textContent = raceName.get(ix); b.appendChild(nm);
      const sub = inSchedule(ix) ? `her schedule · ${info(ix).cell.p}% win` : shownAt.has(ix) ? `won, shown in ${turnName(shownAt.get(ix))}: moves here` : '';
      if (sub) { const s = document.createElement('small'); s.textContent = sub; b.appendChild(s); }
      b.onclick = () => { opts.at()[ix] = turn; edit([...opts.get(), ix]); draw(`win:${ix}`); };
      menu.appendChild(b);
    }
    // by the slot, inside the editor (it scrolls with the calendar); clamped to the editor's width
    menu.hidden = false;
    const boxRect = box.getBoundingClientRect(), slotRect = opener.closest('.ag-slot').getBoundingClientRect(), width = menu.offsetWidth;
    menu.style.left = `${Math.max(0, Math.min(slotRect.left - boxRect.left, box.clientWidth - width))}px`;
    // below the slot, or above it when it would run past the editor's foot (no scrolling to reach it)
    const below = slotRect.bottom - boxRect.top + 4, above = slotRect.top - boxRect.top - menu.offsetHeight - 4;
    menu.style.top = `${below + menu.offsetHeight > box.clientHeight && above >= 0 ? above : below}px`;
    menu.querySelector('.wpick')?.focus();
  }
  // Escape closes the menu (not the dialog) and returns to its slot; Tab past either end does the same
  menu.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); closeMenu(true); return; }
    if (e.key !== 'Tab') return;
    const buttons = [...menu.querySelectorAll('button')], i = buttons.indexOf(/** @type {HTMLButtonElement} */ (document.activeElement));
    if (!buttons.length || (e.shiftKey ? i <= 0 : i === buttons.length - 1)) { e.preventDefault(); closeMenu(true); }
  });
  draw();
  return box;
}
// the race schedule window: one tab per member, her races on the calendar
// loop: {members: [{id, name, short?}], outfits: [{outfits}] (per member), schedule: ScheduleRow[]}
/** @param {{members: {id: number, name: string, short?: string}[], outfits: {outfits: {id: number, label: string}[]}[], schedule: ScheduleRow[]}} loop */
export function openSched(loop) {
  const members = loop.members, schedule = loop.schedule, body = $('sched-body'); body.textContent = '';
  const outfitOf = (k) => pickOutfit(loop.members[k].id, loop.outfits[k].outfits).list[0]?.id ?? null;
  const head = document.createElement('div'); head.className = 'row between dhead';   // stays at the top while the dialog scrolls
  head.innerHTML = '<h3 id="sched-title">Race schedule</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $dialog('sched-dlg').close(); head.appendChild(close);
  const tabs = document.createElement('div'); tabs.className = 'stabs'; tabs.setAttribute('role', 'tablist');
  const view = document.createElement('div'); view.className = 'sview'; view.id = 'sched-view'; view.setAttribute('role', 'tabpanel');
  // tabs: one in the Tab order (the selected), the arrows (and Home/End) move between them and select
  // a tab's key: the member's index, or 'all'
  const tabBtn = (key, label, avatar) => { const b = document.createElement('button'); b.type = 'button'; b.className = 'stab'; b.setAttribute('role', 'tab');
    b.id = `stab-${key}`; b.setAttribute('aria-controls', 'sched-view');
    if (avatar) b.appendChild(avatar); b.appendChild(document.createTextNode(label)); b.onclick = () => show(key); tabs.appendChild(b); return b; };
  const btns = members.map((m, k) => tabBtn(k, m.short || m.name, umaAv(m.id, outfitOf(k), 28)));
  const single = members.length === 1;
  if (!single) btns.push(tabBtn('all', members.length === 3 ? 'All three' : 'All four', null));
  const tabKey = (j) => (j === members.length ? 'all' : j);
  tabs.addEventListener('keydown', (e) => {
    const i = btns.indexOf(/** @type {HTMLButtonElement} */ (document.activeElement)); if (i < 0) return;
    const to = { ArrowRight: (i + 1) % btns.length, ArrowLeft: (i - 1 + btns.length) % btns.length, Home: 0, End: btns.length - 1 }[e.key];
    if (to === undefined) return; e.preventDefault(); show(tabKey(to)); btns[to].focus(); });
  const keyEl = document.createElement('span'); keyEl.className = 'agkey';
  keyEl.innerHTML = single ? '<span><i class="sh"></i>A parent won it: builds affinity</span><span><i class="op"></i>No parent won it</span>'
    : '<span><i class="sh"></i>Shared G1: builds affinity</span><span><i class="op"></i>Optional G1: runs alone, no affinity</span>'; tabs.appendChild(keyEl);
  // one scheduled race in member k's slot
  function cell(/** @type {ScheduleRow} */ row, k) {
    const c = row.cells[k], names = row.race.split('/');
    const el = raceCell('div', { bannerName: names[0], label: names.join(' or '), goal: c.goal });
    if (!row.shared) el.classList.add('opt');
    const others = members.map((_, j) => j).filter(j => j !== k && row.cells[j]);
    const wonBy = single && row.parents ? row.parents.map((won, j) => won ? `parent ${j + 1}` : '').filter(Boolean) : [];
    // with whom she runs it (one member: which parent won it)
    const company = single ? (wonBy.length ? ` · won by ${wonBy.join(' and ')}` : ' · no parent won it')
      : others.length ? ` · with ${others.map(j => members[j].name).join(', ')}` : ' · alone (no affinity, optional)';
    el.title = `${names.join(' or ')} · ${why(c) || `win ${c.p}%`}${c.goal ? ' · goal' : ''}${company}`;
    return el;
  }
  function show(key) {
    btns.forEach((b, j) => { const on = tabKey(j) === key; b.setAttribute('aria-selected', on ? 'true' : 'false'); b.tabIndex = on ? 0 : -1; });
    view.setAttribute('aria-labelledby', `stab-${key}`);
    keyEl.hidden = key === 'all';   // the table says shared and optional in its own captions
    view.textContent = '';
    if (key === 'all') { view.appendChild(schedTable(members, schedule, outfitOf)); return; }
    const ag = agendaGrid((y, slotIndex, slot) => {
      const row = schedule.find(x => x.year === y && x.cells[key] && slotOf(x.when) === slotIndex);
      if (row) { slot.classList.add('on'); if (!row.shared) slot.classList.add('opt'); slot.appendChild(cell(row, key)); }
      // the turn, and the win odds of the race run there (in the bar, not over the banner's lettering)
      slot.appendChild(whenBar(slotIndex, row ? row.cells[key] : null));
    });
    view.appendChild(ag);
  }
  body.append(head, tabs, view); show(0); $dialog('sched-dlg').showModal();
}
// the "All three" / "All four" tab: every member's schedule as one table
// (turns written as on the member tabs, "Early Apr", the year being the group row; the members' columns headed by their
// portrait and short name, the name dropped to the screen reader on a narrow phone so the table fits)
const turnLabel = (when) => { const w = parseWhen(when); return w ? (w.late ? 'Late ' : 'Early ') + w.month : when; };
function schedTable(members, /** @type {ScheduleRow[]} */ schedule, outfitOf) {
  const wrap = document.createElement('div'); wrap.className = 'tablewrap';
  const t = document.createElement('table'); t.className = 'sched';
  const head = `<tr><th>G1</th><th class="when">When</th>${members.map((x, k) => `<th class="mh" data-k="${k}"><span class="hn">${esc(x.short || x.name)}</span></th>`).join('')}<th>Runners</th></tr>`;
  // a member's cell: her win %, * for a goal race, its reason on hover; · when she doesn't run it
  const memberCell = (c) => c ? `<td class="${c.goal ? 'goal' : ''}"${why(c) ? ` title="${esc(why(c))}"` : ''}>${c.goal ? '*' : ''}${c.p}</td>` : '<td class="dot">·</td>';
  // the rows, a year row before each year's first
  const body = (rows) => { let out = '', year = -1;
    for (const r of rows) {
      if (r.year !== year) { year = r.year; out += `<tr class="year"><td colspan="${members.length + 3}">${YEARS[year]}</td></tr>`; }
      out += `<tr><td>${esc(r.race)}</td><td class="when">${esc(turnLabel(r.when))}</td>${r.cells.map(memberCell).join('')}<td>${r.runners}/${members.length}</td></tr>`;
    } return out; };
  const shared = schedule.filter(r => r.shared), optional = schedule.filter(r => !r.shared);
  t.innerHTML = `<caption>Shared G1s (these build affinity)</caption><thead>${head}</thead><tbody>${body(shared)}</tbody>`;
  wrap.appendChild(t);
  if (optional.length) {
    const t2 = document.createElement('table'); t2.className = 'sched';
    t2.innerHTML = `<caption>Optional G1s (one runner, no affinity; free to run, skip freely)</caption><thead>${head}</thead><tbody>${body(optional)}</tbody>`;
    wrap.appendChild(t2);
  }
  for (const th of wrap.querySelectorAll('th.mh')) th.prepend(umaAv(members[+th.dataset.k].id, outfitOf(+th.dataset.k), 22));
  const note = document.createElement('p'); note.className = 'hint';
  note.textContent = `Number = win % at that attempt (aptitude and races in a row included; hover for why). * = goal race. · = not running it. Turns not listed are rest or a goal race that isn't a G1.${schedule.some(r => r.alts?.length) ? ' A/B = either race at the same odds; every runner must pick the same one.' : ''}`;
  wrap.appendChild(note);
  return wrap;
}
