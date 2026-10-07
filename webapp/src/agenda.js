// ---------- race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all of them ----------
// One member (the career page): no "All three" / "All four" tab; `shared` then means a parent won the race (it builds affinity when
// she is a parent herself), and rows marked `skipped` (G1s she could enter but doesn't) show in their slot.
import { $, esc } from './dom.js';
import { RACEIDS } from './data.js';
import { umaAv, srcFor } from './images.js';
const YEARS = ['Junior', 'Classic', 'Senior'];
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const slotOf = (w) => { const m = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(w); return m ? MONTHS.indexOf(m[3]) * 2 + (m[2] === 'L' ? 1 : 0) : -1; };
// a race's banner (by its full or chart name), or null
const banner = (name) => { const id = RACEIDS[name], src = id && srcFor('race', id); return src ? Object.assign(new Image(), { className: 'banner', alt: '', loading: 'lazy', src }) : null; };
// the bar at the foot of a slot: "Early Apr"
function whenBar(sI) { const lb = document.createElement('div'); lb.className = 'ag-when'; const wn = document.createElement('span'); wn.textContent = (sI % 2 ? 'Late ' : 'Early ') + MONTHS[sI >> 1]; lb.appendChild(wn); return lb; }
// the schedule window's calendar: three year columns of 24 half-month slots (the career starts in Junior Year, early
// July); fill(year, slot index, slot element) fills each slot
export function agendaGrid(fill) {
  const ag = document.createElement('div'); ag.className = 'agenda';
  for (let y = 0; y < 3; y++) {
    const col = document.createElement('section'); col.className = 'ag-year y' + y;
    col.innerHTML = `<h4>${YEARS[y]} Year</h4>`;
    const grid = document.createElement('div'); grid.className = 'ag-grid';
    for (let sI = 0; sI < 24; sI++) {
      const slot = document.createElement('div');
      if (y === 0 && sI < 12) { if (sI === 0) { slot.className = 'ag-pre'; slot.textContent = 'The career starts in Junior Year, early July'; grid.appendChild(slot); } continue; }   // one note fills the blank half-year
      slot.className = 'ag-slot';
      fill(y, sI, slot);
      grid.appendChild(slot);
    }
    col.appendChild(grid); ag.appendChild(col);
  }
  return ag;
}
// G1 wins on the same calendar: races = [{ix, name, turns: [0-71], p?}]; each race a button at each of its turns
// (a race held in Classic and Senior is one race: either button toggles it). won(ix) says whether she won it,
// toggle(ix) flips it; not-won races are faded.
export function winsAgenda(races, won, toggle) {
  const at = new Map();
  for (const r of races) for (const t of r.turns) { const k = t; if (!at.has(k)) at.set(k, []); at.get(k).push(r); }
  const btns = [];
  const paint = () => { for (const b of btns) { const w = won(+b.dataset.ix); b.setAttribute('aria-pressed', String(w)); b.classList.toggle('off', !w);
    b.closest('.ag-slot').classList.toggle('on', [...b.closest('.ag-slot').querySelectorAll('.wbtn')].some(x => won(+x.dataset.ix))); } };
  const ag = agendaGrid((y, sI, slot) => {
    const list = at.get(y * 24 + sI) || [];
    if (list.length > 1) slot.classList.add('many');
    for (const r of list) {
      const b = document.createElement('button'); b.type = 'button'; b.className = 'ag-race wbtn'; b.dataset.ix = r.ix; b.dataset.k = 'win:' + r.ix + ':' + (y * 24 + sI);
      const bn = banner(r.name); if (bn) b.appendChild(bn);
      const nm = document.createElement('span'); nm.className = 'ag-nm'; nm.textContent = r.name; b.appendChild(nm);
      const label = r.name + (r.p !== undefined ? ` (${Math.round(r.p * 100)}% to win)` : '');
      b.title = label; b.setAttribute('aria-label', label);
      b.onclick = () => { toggle(r.ix); paint(); };
      slot.appendChild(b); btns.push(b);
    }
    const lb = whenBar(sI);
    if (list.length === 1 && list[0].p !== undefined) { const p = document.createElement('b'); p.textContent = Math.round(list[0].p * 100) + '% win'; lb.appendChild(p); }
    slot.appendChild(lb);
  });
  paint();
  return { el: ag, paint };
}
export function openSched(l) {
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
  keyEl.innerHTML = single ? '<span><i class="sh"></i>A parent won it: builds affinity</span><span><i class="op"></i>No parent won it</span><span><i class="sk"></i>Skipped</span>'
    : '<span><i class="sh"></i>Shared G1: builds affinity</span><span><i class="op"></i>Optional G1: runs alone, no affinity</span>'; tabs.appendChild(keyEl);
  // why a race isn't a sure win: 110 minus the aptitude cost (distance and surface grades) minus the penalty for the k-th
  // race in a row, capped at 100%
  const ORD = ['', '1st', '2nd', '3rd', '4th', '5th', '6th or later'];
  const why = (c) => { if (c.apt === undefined) return ''; const parts = [];
    if (c.apt > 0) parts.push(`aptitude −${c.apt}`);
    if (c.pen > 0) parts.push(`${ORD[Math.min(c.row, 6)]} race in a row −${c.pen}`);
    return `${c.p}% win: 110` + (parts.length ? ', ' + parts.join(', ') : '') + (110 - c.apt - c.pen > 100 ? ', capped at 100' : ''); };
  function cell(r, k) {   // one scheduled race in a slot
    const c = r.cells[k], names = r.race.split('/'), el = document.createElement('div'); el.className = 'ag-race' + (r.shared ? '' : ' opt');
    const bn = banner(names[0]); if (bn) el.appendChild(bn);
    const nmEl = document.createElement('div'); nmEl.className = 'ag-nm'; nmEl.textContent = names.join(' or '); el.appendChild(nmEl);
    const others = M.map((_, j) => j).filter(j => j !== k && r.cells[j]);
    if (c.goal) { const g = document.createElement('span'); g.className = 'ag-goal'; g.textContent = 'Goal'; el.appendChild(g); }
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
      // G1s she could enter at this turn but doesn't (another race, or a rest for the streak penalty)
      const sk = rows.filter(x => x.skipped && x.year === y && slotOf(x.when) === sI);
      if (sk.length) { const k = document.createElement('div'); k.className = 'ag-skip'; k.textContent = 'Skipped: ' + sk.map(x => x.race).join(', ');
        k.title = sk.map(x => x.race + ': ' + (x.instead === 'rest' ? 'she rests this turn (races in a row lower her win odds)' : 'she runs ' + x.instead + ' instead')).join('\n'); slot.appendChild(k); }
      // the turn, and the win odds of the race run there (in the bar, not over the banner's lettering)
      const lb = whenBar(sI);
      if (r) { const p = document.createElement('b'); p.textContent = r.cells[key].p + '% win'; p.title = why(r.cells[key]); lb.appendChild(p); }
      slot.appendChild(lb);
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
