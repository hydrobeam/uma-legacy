// ---------- affinity popup: what builds each legacy's affinity (relations + shared G1 wins), per pair and per career ----------
import { $, esc } from './dom.js';
import { umaAv } from './images.js';
export function openWhy(l) {
  const D = l.detail, M = l.members, body = $('why-body'); body.textContent = '';
  const nm = (i) => M[i].short || M[i].name;
  const outfitOf = (i) => (l.outfits[i].outfits.length === 1 ? l.outfits[i].outfits[0].id : null);
  const total = (L) => L.slots.reduce((a, x) => a + x.aff, 0);
  const head = document.createElement('div'); head.className = 'row between dhead';
  head.innerHTML = '<h3 id="why-title">Affinity</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('why-dlg').close(); head.appendChild(close);
  body.appendChild(head);
  const intro = document.createElement('p'); intro.className = 'hint';
  intro.textContent = 'Each legacy\u2019s affinity is its relation points plus 3 for every G1 it won together with its partners: a parent with the other parent and its own parents, a grandparent with its child. Higher affinity, more inheritance. The race schedule exists to raise the race part.';
  body.appendChild(intro);
  const h1 = document.createElement('h4'); h1.textContent = 'Pairs'; body.appendChild(h1);
  const pt = document.createElement('div'); pt.className = 'tablewrap';
  pt.innerHTML = `<table class="tbl"><thead><tr><th>Pair</th><th>Relation points</th><th>G1s won together</th><th>Race affinity (×3)</th></tr></thead><tbody>` +
    D.pairs.map(p => `<tr><td>${esc(nm(p.a))} & ${esc(nm(p.b))}</td><td>${p.rel}</td><td>${p.wins.toFixed(0)}</td><td>${(3 * p.wins).toFixed(0)}</td></tr>`).join('') + '</tbody></table>';
  body.appendChild(pt);
  const h2 = document.createElement('h4'); h2.textContent = 'The 12 careers (choose one)'; body.appendChild(h2);
  const ct = document.createElement('div'); ct.className = 'tablewrap';
  const tb = document.createElement('table'); tb.className = 'tbl';
  tb.innerHTML = '<thead><tr><th>Trainee</th><th>Parents</th><th>Grandparent</th><th>Affinity</th></tr></thead>';
  const tbody = document.createElement('tbody'); tb.appendChild(tbody); ct.appendChild(tb);
  const split = document.createElement('div'); split.className = 'split'; split.appendChild(ct);
  const view = document.createElement('div'); view.className = 'view'; view.style.display = 'grid'; view.style.gap = '12px'; split.appendChild(view); body.appendChild(split);
  const rows = D.lineages.map((L, i) => {
    const tr = document.createElement('tr'); tr.className = 'pick';
    // the trainee's name is a button (keyboard, assistive tech: pressed = the career shown); the whole row takes clicks too
    tr.innerHTML = `<td><button type="button" class="rowbtn" aria-pressed="false" aria-label="${esc(nm(L.t))}: parents ${esc(nm(L.p[0]))} and ${esc(nm(L.p[1]))}">${esc(nm(L.t))}</button></td><td>${esc(nm(L.p[0]))} + ${esc(nm(L.p[1]))}</td><td>${esc(nm(L.x))}</td><td>${total(L).toFixed(0)}</td>`;
    tr.onclick = () => show(i); tbody.appendChild(tr); return tr;
  });
  function show(i) {
    rows.forEach((r, k) => { r.classList.toggle('sel', k === i); r.querySelector('.rowbtn').setAttribute('aria-pressed', String(k === i)); });
    const L = D.lineages[i]; view.textContent = '';
    const tree = document.createElement('div'); tree.className = 'tree';
    const node = (cls, who, html) => { const n = document.createElement('div'); n.className = 'node ' + cls;
      const w = document.createElement('div'); w.className = 'who'; w.appendChild(umaAv(M[who].id, outfitOf(who), 26)); const s = document.createElement('span'); s.textContent = M[who].name; w.appendChild(s);
      n.appendChild(w); const t = document.createElement('div'); t.innerHTML = html; n.appendChild(t); return n; };
    tree.appendChild(node('t', L.t, `<span class="num">Trainee · six legacies, affinity <b>${total(L).toFixed(0)}</b></span>`));
    const slot = (x) => `<span class="num">affinity <b>${x.aff.toFixed(0)}</b><br>${x.rel.toFixed(0)} relations + ${x.race.toFixed(0)} races</span>`;
    const P = L.slots.filter(x => x.kind === 'parent'), G = L.slots.filter(x => x.kind === 'grandparent');
    tree.appendChild(node('p', P[0].who[0], '<span class="hint">Parent</span><br>' + slot(P[0]))); tree.appendChild(node('p', P[1].who[0], '<span class="hint">Parent</span><br>' + slot(P[1])));
    for (const g of G) tree.appendChild(node('g', g.who[1], `<span class="hint">Grandparent (${esc(nm(g.who[0]))})</span><br>` + slot(g)));
    view.appendChild(tree);
  }
  let best = 0; D.lineages.forEach((L, i) => { if (total(L) > total(D.lineages[best])) best = i; }); show(best);
  $('why-dlg').showModal();
}
