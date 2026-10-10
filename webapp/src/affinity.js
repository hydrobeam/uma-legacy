// ---------- affinity popup (loop planner): a career's affinity, legacy by legacy, as on the career page ----------
// The loop's careers as the game's pill switches: one per trainee, the most affinity first; in a four-uma loop each
// trainee heads three careers, so a second row picks her parents (both faces and that career's affinity); under them the chosen career's table (afftable.js): each parent, then her two parents, with their
// relation points, the G1 wins they share (expected: from the schedules' win chances, x3) and the affinity, and the
// total. In a three-uma rotation the trainee is her own grandparent twice (her earlier veteran): no relation points
// with her (a trio counts distinct umas), only the G1s it shares with its child.
import { $ } from './dom.js';
import { umaAv } from './images.js';
import { affEl, affTable } from './afftable.js';
import { pickOutfit } from './roster.js';
export function openWhy(l) {
  const D = l.detail, M = l.members, body = $('why-body'); body.textContent = '';
  const nm = (i) => M[i].short || M[i].name;
  const outfitOf = (i) => pickOutfit(l.members[i].id, l.outfits[i].outfits).list[0]?.id ?? null;
  const total = (L) => L.slots.reduce((a, x) => a + x.aff, 0);
  const head = document.createElement('div'); head.className = 'row between dhead';
  head.innerHTML = '<h3 id="why-title">Affinity</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('why-dlg').close(); head.appendChild(close);
  body.appendChild(head);
  // the careers to pick from: one pill per trainee (a rotation's three); in a four-uma loop each trainee heads three
  // careers (her parents two of the other three), so a second row picks her parents
  const pick = document.createElement('div'); pick.className = 'afpick'; pick.setAttribute('role', 'radiogroup'); pick.setAttribute('aria-label', 'Trainee');
  const pars = document.createElement('div'); pars.className = 'afpick afpar'; pars.setAttribute('role', 'radiogroup'); pars.setAttribute('aria-label', 'Parents');
  const view = document.createElement('div'); view.className = 'afview';
  body.append(pick, pars, view);
  const byT = new Map();   // trainee -> her careers, the most affinity first
  for (const i of D.lineages.map((L, i) => i).sort((a, b) => total(D.lineages[b]) - total(D.lineages[a]))) {
    const t = D.lineages[i].t; if (!byT.has(t)) byT.set(t, []); byT.get(t).push(i); }
  const radios = (box, items) => box.addEventListener('keydown', (e) => { const list = items(), k = list.findIndex(([, b]) => b === document.activeElement); if (k < 0) return;
    const to = { ArrowRight: k + 1, ArrowDown: k + 1, ArrowLeft: k - 1, ArrowUp: k - 1 }[e.key]; if (to === undefined) return; e.preventDefault();
    const [i, b] = list[(to + list.length) % list.length]; show(i); b.focus(); });
  const tBtns = [...byT].map(([t, ls]) => { const L = D.lineages[ls[0]], b = document.createElement('button'); b.type = 'button'; b.setAttribute('role', 'radio');
    b.setAttribute('aria-label', ls.length > 1 ? `${M[t].name} as the trainee` : `${M[t].name}, parents ${M[L.p[0]].name} and ${M[L.p[1]].name}`); b.title = b.getAttribute('aria-label');
    b.append(umaAv(M[t].id, outfitOf(t), 28), Object.assign(document.createElement('span'), { className: 'afpn', textContent: nm(t) }));
    const a = affEl(total(L)); if (a) b.appendChild(a);   // her best career's affinity
    b.onclick = () => show(ls[0]); pick.appendChild(b); return [ls[0], b, t]; });
  let pBtns = [];
  radios(pick, () => tBtns); radios(pars, () => pBtns);
  const mark = (list, on) => list.forEach(([, b, k]) => { const yes = on(k); b.setAttribute('aria-checked', String(yes)); b.setAttribute('aria-pressed', String(yes)); b.tabIndex = yes ? 0 : -1; });
  function show(i) {
    const L = D.lineages[i], ls = byT.get(L.t);
    mark(tBtns, (t) => t === L.t);
    // her parents, when she heads more than one career: both faces and that career's affinity
    pars.hidden = ls.length < 2; pars.textContent = ''; if (ls.length > 1) pars.append(Object.assign(document.createElement('span'), { className: 'aflab', textContent: 'Parents' }));
    pBtns = ls.length < 2 ? [] : ls.map(j => { const C = D.lineages[j], b = document.createElement('button'); b.type = 'button'; b.setAttribute('role', 'radio');
      b.setAttribute('aria-label', `parents ${M[C.p[0]].name} and ${M[C.p[1]].name}`); b.title = `${M[C.t].name}: ${b.getAttribute('aria-label')}`;
      b.append(umaAv(M[C.p[0]].id, outfitOf(C.p[0]), 24), umaAv(M[C.p[1]].id, outfitOf(C.p[1]), 24));
      const a = affEl(total(C)); if (a) b.appendChild(a);
      b.onclick = () => show(j); pars.appendChild(b); return [j, b, j]; });
    mark(pBtns, (j) => j === i);
    const own = (w) => w === L.t ? 'her own earlier veteran' : null;
    view.replaceChildren(affTable(L.p.map(p => { const x = L.slots.find(s => s.kind === 'parent' && s.who[0] === p);
      return { face: umaAv(M[p].id, outfitOf(p), 30), name: M[p].name, rel: x.rel, race: x.race, aff: x.aff,
        gps: L.slots.filter(s => s.kind === 'grandparent' && s.who[0] === p).map(g => ({ face: umaAv(M[g.who[1]].id, outfitOf(g.who[1]), 24), name: M[g.who[1]].name,
          tag: own(g.who[1]), rel: g.rel, race: g.race, aff: g.aff })) }; }), `${M[L.t].name}: her six legacies`));
  }
  show(tBtns[0][0]);
  $('why-dlg').showModal();
}
