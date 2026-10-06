// ---------- affinity popup: what builds each legacy's affinity (relations + shared G1 wins), per pair and per career ----------
import { $, esc } from './dom.js';
import { umaAv } from './images.js';
export function openWhy(l) {
  const D = l.detail, M = l.members, body = $('why-body'); body.textContent = '';
  const nm = (i) => M[i].short || M[i].name;
  const outfitOf = (i) => (l.outfits[i].outfits.length === 1 ? l.outfits[i].outfits[0].id : null);
  const total = (L) => L.slots.reduce((a, x) => a + x.aff, 0);
  const head = document.createElement('div'); head.className = 'row between dhead';
  head.innerHTML = '<h3 id="why-title" class="row">Affinity <button type="button" class="oinfo" aria-expanded="false" aria-controls="why-math" aria-label="How affinity is calculated" title="How affinity is calculated">i</button></h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('why-dlg').close(); head.appendChild(close);
  body.appendChild(head);
  // in words, then the formula (behind the (i) by the title), in the legacy roles of one career: trainee T, parents
  // P1 and P2, each parent P's own parents GP1 and GP2
  const intro = document.createElement('div'); intro.className = 'afx';
  intro.innerHTML = `<p>A parent's affinity is how well she matches the trainee, the other parent, and her own two parents (each as a trio with the trainee), plus 3 for every G1 she won together with the other parent or with one of her own parents. A grandparent counts only her trio with the trainee and her child, plus 3 for every G1 she won together with that child. The higher a legacy's affinity, the more often it passes a skill on: each spark's chance is its base × (1 + affinity / 100), so affinity 100 doubles it.</p>
    <div class="afx" id="why-math" hidden><p class="hint">Each legacy's affinity, for trainee T with parents P1 and P2 (P is either; P2 the other one) and each parent's own parents GP1 and GP2:</p>
    <p class="fx">Parent P = rel(T–P) + rel(P1–P2) + rel(T–P–GP1) + rel(T–P–GP2)<br>&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;&nbsp;+ 3 × [G1(P–P2) + G1(P–GP1) + G1(P–GP2)]</p>
    <p class="fx">Grandparent GP (under P) = rel(T–P–GP) + 3 × G1(P–GP)</p>
    <p class="hint">rel: the game's relation points of a pair or trio, fixed by who they are. G1: the G1 races both won, which the race schedule makes as many as it can.</p></div>`;
  body.appendChild(intro);
  const ib = head.querySelector('.oinfo'), math = intro.querySelector('#why-math');
  ib.onclick = () => { math.hidden = !math.hidden; ib.setAttribute('aria-expanded', String(!math.hidden)); };
  // an uma as her portrait; hover, focus or tap shows her name beside it
  const face = (i, size = 26) => { const f = document.createElement('span'); f.className = 'aface'; f.tabIndex = 0; f.title = M[i].name;
    f.appendChild(umaAv(M[i].id, outfitOf(i), size)); const n = document.createElement('span'); n.className = 'an'; n.textContent = nm(i); f.appendChild(n); return f.outerHTML; };
  const h1 = document.createElement('h4'); h1.textContent = 'Pairs'; body.appendChild(h1);
  const pt = document.createElement('div'); pt.className = 'tablewrap';
  pt.innerHTML = `<table class="tbl"><thead><tr><th>Pair</th><th>Relation points</th><th>G1s won together</th><th>Race affinity (×3)</th></tr></thead><tbody>` +
    D.pairs.map(p => `<tr><td class="afaces">${face(p.a)}<span class="amp">&amp;</span>${face(p.b)}</td><td>${p.rel}</td><td>${p.wins.toFixed(0)}</td><td>${(3 * p.wins).toFixed(0)}</td></tr>`).join('') + '</tbody></table>';
  body.appendChild(pt);
  const h2 = document.createElement('h4'); h2.textContent = 'The 12 careers (choose one)'; body.appendChild(h2);
  const ct = document.createElement('div'); ct.className = 'tablewrap';
  const tb = document.createElement('table'); tb.className = 'tbl';
  tb.innerHTML = '<thead><tr><th>Trainee</th><th>Parents</th><th>Grandparents</th><th>Affinity</th></tr></thead>';
  const tbody = document.createElement('tbody'); tb.appendChild(tbody); ct.appendChild(tb);
  const split = document.createElement('div'); split.className = 'split'; split.appendChild(ct);
  const view = document.createElement('div'); view.className = 'view'; view.style.display = 'grid'; view.style.gap = '12px'; split.appendChild(view); body.appendChild(split);
  // a career's four grandparents, two under each parent (grouped, the parent named in the group's tooltip)
  const gps = (L) => L.p.map(p => `<span class="agp" title="${esc(nm(p))}\u2019s parents">${L.slots.filter(x => x.kind === 'grandparent' && x.who[0] === p).map(x => face(x.who[1])).join('')}</span>`).join('<span class="amp">·</span>');
  const rows = D.lineages.map((L, i) => {
    const tr = document.createElement('tr'); tr.className = 'pick';
    // the trainee's name is a button (keyboard, assistive tech: pressed = the career shown); the whole row takes clicks too
    tr.innerHTML = `<td><button type="button" class="rowbtn" aria-pressed="false" aria-label="${esc(nm(L.t))}: parents ${esc(nm(L.p[0]))} and ${esc(nm(L.p[1]))}" title="${esc(M[L.t].name)}">${umaAv(M[L.t].id, outfitOf(L.t), 26).outerHTML}<span class="an">${esc(nm(L.t))}</span></button></td><td class="afaces">${face(L.p[0])}<span class="amp">+</span>${face(L.p[1])}</td><td class="afaces">${gps(L)}</td><td>${total(L).toFixed(0)}</td>`;
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
    for (const g of G) tree.appendChild(node('g', g.who[1], '<span class="hint">Grandparent</span><br>' + slot(g)));
    view.appendChild(tree);
  }
  // a tap on a portrait (phones) shows its name; another tap, or a tap elsewhere, hides it
  body.addEventListener('click', (e) => { const f = e.target.closest('.aface'); body.querySelectorAll('.aface.open').forEach(x => { if (x !== f) x.classList.remove('open'); }); if (f) f.classList.toggle('open'); });
  let best = 0; D.lineages.forEach((L, i) => { if (total(L) > total(D.lineages[best])) best = i; }); show(best);
  $('why-dlg').showModal();
}
