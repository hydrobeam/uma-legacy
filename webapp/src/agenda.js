// ---------- race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all four ----------
import { $, esc } from './dom.js';
import { RACEIDS } from './data.js';
import { umaAv, srcFor } from './images.js';
const YEARS = ['Junior', 'Classic', 'Senior'];
export function openSched(l) {
  const M = l.members, rows = l.schedule, body = $('sched-body'); body.textContent = '';
  const outfitOf = (k) => (l.outfits[k].outfits.length === 1 ? l.outfits[k].outfits[0].id : null);
  const head = document.createElement('div'); head.className = 'row between';
  head.innerHTML = '<h3 id="sched-title">Race schedule</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('sched-dlg').close(); head.appendChild(close);
  const tabs = document.createElement('div'); tabs.className = 'stabs'; tabs.setAttribute('role', 'tablist');
  const view = document.createElement('div'); view.className = 'sview'; view.id = 'sched-view'; view.setAttribute('role', 'tabpanel');
  // tabs: one in the Tab order (the selected), the arrows (and Home/End) move between them and select
  const tabBtn = (key, label, av) => { const b = document.createElement('button'); b.type = 'button'; b.className = 'stab'; b.setAttribute('role', 'tab');
    b.id = 'stab-' + key; b.setAttribute('aria-controls', 'sched-view');
    if (av) b.appendChild(av); b.appendChild(document.createTextNode(label)); b.onclick = () => show(key); tabs.appendChild(b); return b; };
  const btns = M.map((m, k) => tabBtn(k, m.short || m.name, umaAv(m.id, outfitOf(k), 28)));
  btns.push(tabBtn('all', 'All four', null));
  const keyOf = (j) => (j === M.length ? 'all' : j);
  tabs.addEventListener('keydown', (e) => {
    const i = btns.indexOf(document.activeElement); if (i < 0) return;
    const to = { ArrowRight: (i + 1) % btns.length, ArrowLeft: (i - 1 + btns.length) % btns.length, Home: 0, End: btns.length - 1 }[e.key];
    if (to === undefined) return; e.preventDefault(); show(keyOf(to)); btns[to].focus(); });
  const key = document.createElement('span'); key.className = 'agkey';
  key.innerHTML = '<span><i class="sh"></i>Shared G1: builds affinity</span><span><i class="op"></i>Optional G1: runs alone, no affinity</span>'; tabs.appendChild(key);
  const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
  const slotOf = (w) => { const m = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(w); return m ? MONTHS.indexOf(m[3]) * 2 + (m[2] === 'L' ? 1 : 0) : -1; };
  function cell(r, k) {   // one scheduled race in a slot
    const c = r.cells[k], names = r.race.split('/'), el = document.createElement('div'); el.className = 'ag-race' + (r.shared ? '' : ' opt');
    const id = RACEIDS[names[0]], src = id && srcFor('race', id);
    if (src) el.appendChild(Object.assign(new Image(), { className: 'banner', alt: '', loading: 'lazy', src }));
    const nmEl = document.createElement('div'); nmEl.className = 'ag-nm'; nmEl.textContent = names.join(' or '); el.appendChild(nmEl);
    const others = M.map((_, j) => j).filter(j => j !== k && r.cells[j]);
    const p = document.createElement('span'); p.className = 'ag-p'; p.textContent = c.p + '%'; el.appendChild(p);
    if (c.goal) { const g = document.createElement('span'); g.className = 'ag-goal'; g.textContent = 'Goal'; el.appendChild(g); }
    el.title = names.join(' or ') + ' · win ' + c.p + '%' + (c.goal ? ' · goal' : '') + (others.length ? ' · with ' + others.map(j => M[j].name).join(', ') : ' · alone (no affinity, optional)');
    return el;
  }
  function show(key) {
    btns.forEach((b, j) => { const on = keyOf(j) === key; b.setAttribute('aria-selected', on ? 'true' : 'false'); b.tabIndex = on ? 0 : -1; });
    view.setAttribute('aria-labelledby', 'stab-' + key);
    view.textContent = '';
    if (key === 'all') { view.appendChild(schedTable(M, rows)); return; }
    const ag = document.createElement('div'); ag.className = 'agenda';
    for (let y = 0; y < 3; y++) {
      const col = document.createElement('section'); col.className = 'ag-year y' + y;
      col.innerHTML = `<h4>${YEARS[y]} Year</h4>`;
      const grid = document.createElement('div'); grid.className = 'ag-grid';
      for (let sI = 0; sI < 24; sI++) {
        const slot = document.createElement('div');
        if (y === 0 && sI < 12) { slot.className = 'ag-slot none'; grid.appendChild(slot); continue; }   // the career starts in Junior July
        slot.className = 'ag-slot';
        const r = rows.find(x => x.year === y && x.cells[key] && slotOf(x.when) === sI);
        if (r) { slot.classList.add('on'); if (!r.shared) slot.classList.add('opt'); slot.appendChild(cell(r, key)); }
        const lb = document.createElement('div'); lb.className = 'ag-when'; lb.textContent = (sI % 2 ? 'Late ' : 'Early ') + MONTHS[sI >> 1]; slot.appendChild(lb);
        grid.appendChild(slot);
      }
      col.appendChild(grid); ag.appendChild(col);
    }
    view.appendChild(ag);
  }
  body.append(head, tabs, view); show(0); $('sched-dlg').showModal();
}
// the "All four" tab: every member's schedule as one table
function schedTable(m, rows) {
  const wrap = document.createElement('div'); wrap.className = 'tablewrap';
  const t = document.createElement('table'); t.className = 'sched';
  const head = '<tr><th>G1</th><th class="when">When</th>' + m.map(x => `<th>${esc(x.short)}</th>`).join('') + '<th>Runners</th></tr>';
  const body = (list) => { let out = '', y = -1;
    for (const r of list) {
      if (r.year !== y) { y = r.year; out += `<tr class="year"><td colspan="${m.length + 3}">${['Junior', 'Classic', 'Senior'][y]}</td></tr>`; }
      out += `<tr><td>${esc(r.race)}</td><td class="when">${esc(r.when)}</td>` + r.cells.map(c => c ? `<td class="${c.goal ? 'goal' : ''}">${c.goal ? '*' : ''}${c.p}</td>` : '<td class="dot">·</td>').join('')
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
  const note = document.createElement('p'); note.className = 'hint'; note.textContent = 'Number = win % at that attempt (streak penalty included). * = goal race. · = not running it. Turns not listed are rest.' + (rows.some(r => r.alts && r.alts.length) ? ' A/B = either race at the same odds; every runner must pick the same one.' : '');
  wrap.appendChild(note);
  return wrap;
}
