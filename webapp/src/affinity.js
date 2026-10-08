// ---------- affinity popup (loop planner): a career's affinity, legacy by legacy, as on the career page ----------
// The loop's careers as the game's pill switches (each its trainee, with her two parents), the one with the most
// affinity first; under them the chosen career's table (afftable.js): each parent, then her two parents, with their
// relation points, the G1 wins they share (expected: from the schedules' win chances, x3) and the affinity, and the
// total. In a three-uma rotation the trainee is her own grandparent twice (her earlier veteran): no relation points
// with her (a trio counts distinct umas), only the G1s it shares with its child.
import { $ } from './dom.js';
import { umaAv } from './images.js';
import { affEl, affTable } from './afftable.js';
export function openWhy(l) {
  const D = l.detail, M = l.members, body = $('why-body'); body.textContent = '';
  const nm = (i) => M[i].short || M[i].name;
  const outfitOf = (i) => (l.outfits[i].outfits.length === 1 ? l.outfits[i].outfits[0].id : null);
  const total = (L) => L.slots.reduce((a, x) => a + x.aff, 0);
  const head = document.createElement('div'); head.className = 'row between dhead';
  head.innerHTML = '<h3 id="why-title">Affinity</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('why-dlg').close(); head.appendChild(close);
  body.appendChild(head);
  const pick = document.createElement('div'); pick.className = 'afpick'; pick.setAttribute('role', 'radiogroup'); pick.setAttribute('aria-label', 'Career');
  const view = document.createElement('div'); view.className = 'afview';
  body.append(pick, view);
  const order = D.lineages.map((L, i) => i).sort((a, b) => total(D.lineages[b]) - total(D.lineages[a]));
  const btns = order.map(i => { const L = D.lineages[i], b = document.createElement('button'); b.type = 'button'; b.setAttribute('role', 'radio');
    b.setAttribute('aria-label', `${M[L.t].name}, parents ${M[L.p[0]].name} and ${M[L.p[1]].name}`); b.title = b.getAttribute('aria-label');
    b.append(umaAv(M[L.t].id, outfitOf(L.t), 28), Object.assign(document.createElement('span'), { className: 'afpn', textContent: nm(L.t) }));
    const a = affEl(total(L)); if (a) b.appendChild(a);
    b.onclick = () => show(i); pick.appendChild(b); return [i, b]; });
  pick.addEventListener('keydown', (e) => { const k = btns.findIndex(([, b]) => b === document.activeElement); if (k < 0) return;
    const to = { ArrowRight: k + 1, ArrowDown: k + 1, ArrowLeft: k - 1, ArrowUp: k - 1 }[e.key]; if (to === undefined) return; e.preventDefault();
    const [i, b] = btns[(to + btns.length) % btns.length]; show(i); b.focus(); });
  function show(i) {
    btns.forEach(([j, b]) => { b.setAttribute('aria-checked', String(j === i)); b.setAttribute('aria-pressed', String(j === i)); b.tabIndex = j === i ? 0 : -1; });
    const L = D.lineages[i], own = (w) => w === L.t ? 'her own earlier veteran' : null;
    view.replaceChildren(affTable(L.p.map(p => { const x = L.slots.find(s => s.kind === 'parent' && s.who[0] === p);
      return { face: umaAv(M[p].id, outfitOf(p), 30), name: M[p].name, rel: x.rel, race: x.race, aff: x.aff,
        gps: L.slots.filter(s => s.kind === 'grandparent' && s.who[0] === p).map(g => ({ face: umaAv(M[g.who[1]].id, outfitOf(g.who[1]), 24), name: M[g.who[1]].name,
          tag: own(g.who[1]), rel: g.rel, race: g.race, aff: g.aff })) }; }), `${M[L.t].name}: her six legacies`));
  }
  show(order[0]);
  $('why-dlg').showModal();
}
