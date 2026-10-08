// ---------- race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all of them ----------
// One member (the career page): no "All three" / "All four" tab; `shared` then means a parent won the race (it builds affinity when
// she is a parent herself).
import { $, esc } from './dom.js';
import { RACEIDS } from './data.js';
import { umaAv, srcFor } from './images.js';
const YEARS = ['Junior', 'Classic', 'Senior'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const slotOf = (w) => { const m = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(w); return m ? MONTHS.indexOf(m[3]) * 2 + (m[2] === 'L' ? 1 : 0) : -1; };
// a race's banner (by its full or chart name), or null
const banner = (name) => { const id = RACEIDS[name], src = id && srcFor('race', id); return src ? Object.assign(new Image(), { className: 'banner', alt: '', loading: 'lazy', src }) : null; };
// the bar at the foot of a slot: "Early Apr", and the win odds of the race run there (`c`: a schedule cell)
function whenBar(sI, c = null) {
  const lb = document.createElement('div'); lb.className = 'ag-when'; const wn = document.createElement('span'); wn.textContent = (sI % 2 ? 'Late ' : 'Early ') + MONTHS[sI >> 1]; lb.appendChild(wn);
  if (c) { const p = document.createElement('b'); p.textContent = c.p + '% win'; p.title = why(c); lb.appendChild(p); }
  return lb;
}
// why a race isn't a sure win: 110 minus the aptitude cost (distance and surface grades) minus the penalty for the k-th
// race in a row, capped at 100%
const ORD = ['', '1st', '2nd', '3rd', '4th', '5th', '6th or later'];
const why = (c) => { if (c.apt === undefined) return ''; const parts = [];
  if (c.apt > 0) parts.push(`aptitude −${c.apt}`);
  if (c.pen > 0) parts.push(`${ORD[Math.min(c.row, 6)]} race in a row −${c.pen}`);
  return `${c.p}% win: 110` + (parts.length ? ', ' + parts.join(', ') : '') + (110 - c.apt - c.pen > 100 ? ', capped at 100' : ''); };
// a race in its slot (the schedule window's cell, and a win in the G1 wins editor): banner, name, Goal badge
function raceCell(tag, bannerName, label, goal) {
  const el = document.createElement(tag); el.className = 'ag-race';
  const bn = banner(bannerName); if (bn) el.appendChild(bn);
  const nm = document.createElement('div'); nm.className = 'ag-nm'; nm.textContent = label; el.appendChild(nm);
  if (goal) { const g = document.createElement('span'); g.className = 'ag-goal'; g.textContent = 'Goal'; el.appendChild(g); }
  return el;
}
const turnName = (t) => `${YEARS[(t / 24) | 0]} Year, ${t % 2 ? 'Late' : 'Early'} ${MONTHS[(t % 24) >> 1]}`;
// the schedule window's calendar: three year columns of 24 half-month slots (the career starts in Junior Year, early
// July); fill(year, slot index, slot element) fills each slot
// On the career page it is the game's Scheduled Races: one year at a time behind the game's year tabs (the year shown
// is kept across redraws; a new window starts at its first year with races), the turns before the debut as the game's grey slots.
let agYear = null;
export function agendaGrid(fill) {
  const gm = !!document.querySelector('.rb');
  const ag = document.createElement('div'); ag.className = 'agenda' + (gm ? ' gm' : '');
  const cols = [];
  for (let y = 0; y < 3; y++) {
    const col = document.createElement('section'); col.className = 'ag-year y' + y;
    col.innerHTML = `<h4>${YEARS[y]} Year</h4>`;
    const grid = document.createElement('div'); grid.className = 'ag-grid';
    for (let sI = 0; sI < 24; sI++) {
      const slot = document.createElement('div');
      if (y === 0 && sI < 12) {
        if (gm) { if (sI >= 6) { slot.className = 'ag-slot pre'; slot.appendChild(whenBar(sI)); grid.appendChild(slot); } }   // the game's Junior Year starts in April
        else if (sI === 0) { slot.className = 'ag-pre'; slot.textContent = 'The career starts in Junior Year, early July'; grid.appendChild(slot); }   // one note fills the blank half-year
        continue;
      }
      slot.className = 'ag-slot';
      fill(y, sI, slot);
      grid.appendChild(slot);
    }
    col.appendChild(grid); ag.appendChild(col); cols.push(col);
  }
  if (!gm) return ag;
  const tabs = document.createElement('div'); tabs.className = 'udtabs ag-tabs'; tabs.setAttribute('role', 'tablist');
  const btns = YEARS.map((n, y) => { const b = document.createElement('button'); b.type = 'button'; b.className = 'udtab'; b.setAttribute('role', 'tab');
    b.dataset.y = y; b.textContent = n + ' Year'; b.onclick = () => pick(y); tabs.appendChild(b); return b; });
  function pick(y) { agYear = y; cols.forEach((c, j) => { c.hidden = j !== y; }); btns.forEach((b, j) => { b.setAttribute('aria-selected', String(j === y)); b.tabIndex = j === y ? 0 : -1; }); }
  tabs.addEventListener('keydown', (e) => { const i = btns.indexOf(document.activeElement); if (i < 0) return;
    const to = { ArrowRight: (i + 1) % 3, ArrowLeft: (i + 2) % 3, Home: 0, End: 2 }[e.key]; if (to === undefined) return; e.preventDefault(); pick(to); btns[to].focus(); });
  // the year first shown: the one kept, else the first with a race in it
  pick(agYear ?? Math.max(0, cols.findIndex(c => c.querySelector('.ag-slot.on'))));
  ag.prepend(tabs);
  return ag;
}
// ---------- the G1 wins editor (a pool veteran's, and the veteran analysis' draft), on the same calendar ----------
// Each win sits in one slot as the schedule window's cell; a click removes it and the slot is empty. An empty slot where a
// G1 is held is a button: a menu of the races held then, and a pick marks one won there. A race held in Classic and Senior
// is one race (one win saddle): it shows once, at the turn picked for it (`at`, kept by the page for display only),
// else the turn her schedule runs it (`info`), else its first turn free of another win (an imported win has no turn).
// o: {races: [{ix, name, turns}], get() -> [ix], set([ix]), at() -> {ix: turn} (created on demand, edited in place),
//   offer(turn) -> [ix] | null (the races the menu lists; null: every race held then),
//   info(ix) -> {turn, cell, parent} | null (the race in her schedule: its turn, its schedule cell with the win odds; a
//   parent won it too), onEdit() after every change}
let menuOpen = null;   // {menu, opener}: one menu at a time
function closeMenu(refocus) {
  if (!menuOpen) return; const { menu, opener } = menuOpen; menuOpen = null;
  menu.hidden = true; opener.setAttribute('aria-expanded', 'false'); if (refocus && opener.isConnected) opener.focus();
}
document.addEventListener('pointerdown', (e) => { if (menuOpen && !menuOpen.menu.contains(e.target) && !menuOpen.opener.contains(e.target)) closeMenu(false); }, true);
export function winsAgenda(o) {
  agYear = null;
  const name = new Map(o.races.map(r => [r.ix, r.name])), turns = new Map(o.races.map(r => [r.ix, r.turns]));
  const held = new Map(); for (const r of o.races) for (const t of r.turns) { if (!held.has(t)) held.set(t, []); held.get(t).push(r.ix); }
  const info = (ix) => (o.info && o.info(ix)) || null;
  const box = document.createElement('div'); box.className = 'winsed';
  const menu = document.createElement('div'); menu.className = 'cpop wmenu'; menu.hidden = true; menu.setAttribute('role', 'dialog'); menu.tabIndex = -1;
  // where each win shows: turn -> [ix] (two wins on one turn only when the record says so; they share the slot)
  function layout() {
    const at = o.at(), place = new Map(), off = [];
    const rank = (ix) => (turns.get(ix) || []).includes(at[ix]) ? 0 : info(ix)?.turn !== undefined ? 1 : 2;
    for (const ix of [...o.get()].sort((a, b) => rank(a) - rank(b) || a - b)) {
      const ts = turns.get(ix) || []; if (!ts.length) { off.push(ix); continue; }
      const pref = [at[ix], info(ix)?.turn, ...ts].filter(t => ts.includes(t));
      const t = pref.find(t => !place.has(t)) ?? pref[0];
      if (!place.has(t)) place.set(t, []); place.get(t).push(ix);
    }
    return { place, off };
  }
  const edit = (wins) => { o.set([...new Set(wins)].sort((a, b) => a - b)); o.onEdit(); };
  const remove = (ix) => { delete o.at()[ix]; edit(o.get().filter(x => x !== ix)); };
  function draw(focusK = null) {
    closeMenu(false);
    const { place, off } = layout(), shownAt = new Map(); for (const [t, l] of place) for (const ix of l) shownAt.set(ix, t);
    const ag = agendaGrid((y, sI, slot) => {
      const t = y * 24 + sI, here = place.get(t) || [];
      if (here.length) slot.classList.add('on');
      if (here.length > 1) slot.classList.add('many');
      for (const ix of here) {
        const nm = name.get(ix), f = info(ix), mine = f && f.turn === t;
        const b = raceCell('button', nm, nm, mine && f.cell.goal); b.type = 'button'; b.classList.add('wbtn'); b.dataset.k = 'win:' + ix; b.dataset.ix = ix;
        b.setAttribute('aria-label', `${nm}, won: remove`);
        b.title = `${nm} · won` + (mine ? ' · ' + (why(f.cell) || f.cell.p + '% win') : '') + (f && f.parent ? ' · a parent won it too' : '') + ' · click to remove';
        b.onclick = () => { remove(ix); draw('add:' + t); };
        slot.appendChild(b);
      }
      const can = here.length ? [] : (o.offer ? o.offer(t) : held.get(t)) || [];
      if (can.length) {
        slot.classList.add('free');
        const a = document.createElement('button'); a.type = 'button'; a.className = 'ag-add'; a.dataset.k = 'add:' + t;
        a.innerHTML = '<span aria-hidden="true">+</span>'; a.setAttribute('aria-label', `${turnName(t)}: add a G1 win`); a.title = 'Add a G1 win';
        a.setAttribute('aria-haspopup', 'dialog'); a.setAttribute('aria-expanded', 'false');
        a.onclick = () => (menuOpen && menuOpen.opener === a ? closeMenu(false) : openMenu(a, t, can, shownAt));
        slot.appendChild(a);
      }
      const f = here.length === 1 ? info(here[0]) : null;
      slot.appendChild(whenBar(sI, f && f.turn === t ? f.cell : null));
    });
    box.textContent = ''; box.appendChild(ag);
    if (off.length) {   // a win with no turn on the calendar (not in the race data): listed, still removable
      const p = document.createElement('p'); p.className = 'hint'; p.textContent = 'Not on the calendar: ';
      for (const ix of off) { const b = document.createElement('button'); b.type = 'button'; b.className = 'btn'; b.dataset.k = 'win:' + ix;
        b.textContent = `${name.get(ix) || 'Race ' + ix} ×`; b.setAttribute('aria-label', `${name.get(ix) || 'Race ' + ix}, won: remove`); b.onclick = () => { remove(ix); draw(); }; p.appendChild(b); }
      box.appendChild(p);
    }
    box.appendChild(menu);
    if (focusK) (box.querySelector(`[data-k="${CSS.escape(focusK)}"]`) || box.querySelector('button'))?.focus();
  }
  // the races held at turn t: her schedule's first, then by name; a race won at another turn moves here
  function openMenu(a, t, can, shownAt) {
    closeMenu(false); menuOpen = { menu, opener: a }; a.setAttribute('aria-expanded', 'true');
    menu.textContent = ''; menu.setAttribute('aria-label', `${turnName(t)}: G1s held`);
    const h = document.createElement('p'); h.className = 'hint'; h.textContent = `${turnName(t)}: which G1 did she win?`; menu.appendChild(h);
    const mine = (ix) => info(ix)?.turn === t;
    for (const ix of [...can].sort((x, y) => mine(y) - mine(x) || name.get(x).localeCompare(name.get(y)))) {
      const b = document.createElement('button'); b.type = 'button'; b.className = 'btn wpick';
      const nm = document.createElement('span'); nm.textContent = name.get(ix); b.appendChild(nm);
      const sub = mine(ix) ? `her schedule · ${info(ix).cell.p}% win` : shownAt.has(ix) ? `won, shown in ${turnName(shownAt.get(ix))}: moves here` : '';
      if (sub) { const s = document.createElement('small'); s.textContent = sub; b.appendChild(s); }
      b.onclick = () => { o.at()[ix] = t; edit([...o.get(), ix]); draw('win:' + ix); };
      menu.appendChild(b);
    }
    // under the slot, inside the editor (it scrolls with the calendar); clamped to the editor's width
    menu.hidden = false;
    const br = box.getBoundingClientRect(), r = a.closest('.ag-slot').getBoundingClientRect(), w = menu.offsetWidth;
    menu.style.left = Math.max(0, Math.min(r.left - br.left, box.clientWidth - w)) + 'px'; menu.style.top = (r.bottom - br.top + 4) + 'px';
    menu.querySelector('.wpick')?.focus();
  }
  // Escape closes the menu (not the dialog) and returns to its slot; Tab past either end does the same
  menu.addEventListener('keydown', (e) => {
    if (e.key === 'Escape') { e.preventDefault(); e.stopPropagation(); closeMenu(true); return; }
    if (e.key !== 'Tab') return;
    const f = [...menu.querySelectorAll('button')], i = f.indexOf(document.activeElement);
    if (!f.length || (e.shiftKey ? i <= 0 : i === f.length - 1)) { e.preventDefault(); closeMenu(true); }
  });
  draw();
  return box;
}
// the race schedule window: one tab per member, her races on the calendar
export function openSched(l) {
  agYear = null;
  const M = l.members, rows = l.schedule, body = $('sched-body'); body.textContent = '';
  const outfitOf = (k) => (l.outfits[k].outfits.length === 1 ? l.outfits[k].outfits[0].id : null);
  const head = document.createElement('div'); head.className = 'row between dhead';   // stays at the top while the dialog scrolls
  head.innerHTML = '<h3 id="sched-title">Race schedule</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('sched-dlg').close(); head.appendChild(close);
  const tabs = document.createElement('div'); tabs.className = 'stabs'; tabs.setAttribute('role', 'tablist');
  const view = document.createElement('div'); view.className = 'sview'; view.id = 'sched-view'; view.setAttribute('role', 'tabpanel');
  // tabs: one in the Tab order (the selected), the arrows (and Home/End) move between them and select
  const tabBtn = (key, label, av) => { const b = document.createElement('button'); b.type = 'button'; b.className = 'stab'; b.setAttribute('role', 'tab');
    b.id = 'stab-' + key; b.setAttribute('aria-controls', 'sched-view');
    if (av) b.appendChild(av); b.appendChild(document.createTextNode(label)); b.onclick = () => show(key); tabs.appendChild(b); return b; };
  const btns = M.map((m, k) => tabBtn(k, m.short || m.name, umaAv(m.id, outfitOf(k), 28)));
  const single = M.length === 1;
  if (!single) btns.push(tabBtn('all', M.length === 3 ? 'All three' : 'All four', null));
  const keyOf = (j) => (j === M.length ? 'all' : j);
  tabs.addEventListener('keydown', (e) => {
    const i = btns.indexOf(document.activeElement); if (i < 0) return;
    const to = { ArrowRight: (i + 1) % btns.length, ArrowLeft: (i - 1 + btns.length) % btns.length, Home: 0, End: btns.length - 1 }[e.key];
    if (to === undefined) return; e.preventDefault(); show(keyOf(to)); btns[to].focus(); });
  const keyEl = document.createElement('span'); keyEl.className = 'agkey';
  keyEl.innerHTML = single ? '<span><i class="sh"></i>A parent won it: builds affinity</span><span><i class="op"></i>No parent won it</span>'
    : '<span><i class="sh"></i>Shared G1: builds affinity</span><span><i class="op"></i>Optional G1: runs alone, no affinity</span>'; tabs.appendChild(keyEl);
  function cell(r, k) {   // one scheduled race in a slot
    const c = r.cells[k], names = r.race.split('/'), el = raceCell('div', names[0], names.join(' or '), c.goal);
    if (!r.shared) el.classList.add('opt');
    const others = M.map((_, j) => j).filter(j => j !== k && r.cells[j]);
    const won = single && r.parents ? r.parents.map((x, j) => x ? `parent ${j + 1}` : '').filter(Boolean) : [];
    el.title = names.join(' or ') + ' · ' + (why(c) || 'win ' + c.p + '%') + (c.goal ? ' · goal' : '') + (single ? (won.length ? ' · won by ' + won.join(' and ') : ' · no parent won it')
      : others.length ? ' · with ' + others.map(j => M[j].name).join(', ') : ' · alone (no affinity, optional)');
    return el;
  }
  function show(key) {
    btns.forEach((b, j) => { const on = keyOf(j) === key; b.setAttribute('aria-selected', on ? 'true' : 'false'); b.tabIndex = on ? 0 : -1; });
    view.setAttribute('aria-labelledby', 'stab-' + key);
    keyEl.hidden = key === 'all';   // the table says shared and optional in its own captions
    view.textContent = '';
    if (key === 'all') { view.appendChild(schedTable(M, rows, outfitOf)); return; }
    const ag = agendaGrid((y, sI, slot) => {
      const r = rows.find(x => x.year === y && x.cells[key] && slotOf(x.when) === sI);
      if (r) { slot.classList.add('on'); if (!r.shared) slot.classList.add('opt'); slot.appendChild(cell(r, key)); }
      // the turn, and the win odds of the race run there (in the bar, not over the banner's lettering)
      slot.appendChild(whenBar(sI, r ? r.cells[key] : null));
    });
    view.appendChild(ag);
  }
  body.append(head, tabs, view); show(0); $('sched-dlg').showModal();
}
// the "All three" / "All four" tab: every member's schedule as one table
// (turns written as on the member tabs, "Early Apr", the year being the group row; the members' columns headed by their
// portrait and short name, the name dropped to the screen reader on a narrow phone so the table fits)
const turn = (w) => { const m = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(w); return m ? (m[2] === 'E' ? 'Early ' : 'Late ') + m[3] : w; };
function schedTable(m, rows, outfitOf) {
  const wrap = document.createElement('div'); wrap.className = 'tablewrap';
  const t = document.createElement('table'); t.className = 'sched';
  const head = '<tr><th>G1</th><th class="when">When</th>' + m.map((x, k) => `<th class="mh" data-k="${k}"><span class="hn">${esc(x.short || x.name)}</span></th>`).join('') + '<th>Runners</th></tr>';
  const body = (list) => { let out = '', y = -1;
    for (const r of list) {
      if (r.year !== y) { y = r.year; out += `<tr class="year"><td colspan="${m.length + 3}">${['Junior', 'Classic', 'Senior'][y]}</td></tr>`; }
      out += `<tr><td>${esc(r.race)}</td><td class="when">${esc(turn(r.when))}</td>` + r.cells.map(c => c ? `<td class="${c.goal ? 'goal' : ''}">${c.goal ? '*' : ''}${c.p}</td>` : '<td class="dot">·</td>').join('')
        + `<td>${r.runners}/${m.length}</td></tr>`;
    } return out; };
  const shared = rows.filter(r => r.shared), opt = rows.filter(r => !r.shared);
  t.innerHTML = `<caption>Shared G1s (these build affinity)</caption><thead>${head}</thead><tbody>${body(shared)}</tbody>`;
  wrap.appendChild(t);
  if (opt.length) {
    const t2 = document.createElement('table'); t2.className = 'sched';
    t2.innerHTML = `<caption>Optional G1s (one runner, no affinity; free to run, skip freely)</caption><thead>${head}</thead><tbody>${body(opt)}</tbody>`;
    wrap.appendChild(t2);
  }
  wrap.querySelectorAll('th.mh').forEach(th => th.prepend(umaAv(m[+th.dataset.k].id, outfitOf(+th.dataset.k), 22)));
  const note = document.createElement('p'); note.className = 'hint'; note.textContent = 'Number = win % at that attempt (aptitude and races in a row included; hover for why). * = goal race. · = not running it. Turns not listed are rest.' + (rows.some(r => r.alts && r.alts.length) ? ' A/B = either race at the same odds; every runner must pick the same one.' : '');
  wrap.appendChild(note);
  return wrap;
}
