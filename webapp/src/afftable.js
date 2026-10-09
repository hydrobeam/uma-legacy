// ---------- a career's affinity, legacy by legacy (both planner pages) ----------
// Each of the six legacies has its own affinity, which weighs its own inheritance rolls: a parent's counts her relation
// with the trainee and the other parent, the trio relations with her own two parents and the G1s she shares with all
// three (3 points each); a grandparent's, her trio relation and the G1s she shares with the parent between them. The
// total is the six summed. legs: [{ face, name, tag?, rel, race, aff, gps: [{ face, name, tag?, rel, race, aff }] }]
// (Legacy 1 first); face an element (her portrait).
import { el } from './dom.js';
// her affinity as the game's heart and the number
export function affEl(a, cls = 'aff') { if (a === null || a === undefined) return null; const e = el('span', cls); e.append(el('i', 'heart'), String(Math.round(a)));
  e.setAttribute('aria-label', `Affinity ${Math.round(a)}`); e.title = `Affinity ${Math.round(a)}`; return e; }
export function affTable(legs, caption = 'Affinity') {
  const t = el('table', 'afftab'), R = (x) => String(Math.round(x));
  t.innerHTML = '<thead><tr><th scope="col"></th><th scope="col">Relation</th><th scope="col">G1 wins shared</th><th scope="col">Affinity</th></tr></thead><tfoot></tfoot>';
  if (caption) t.prepend(Object.assign(el('caption'), { textContent: caption }));
  const row = (cls, who, x) => { const tr = el('tr', cls), th = el('th'); th.scope = 'row'; th.append(...who);
    tr.append(th, el('td', '', R(x.rel)), el('td', '', R(x.race)), el('td', 'afv')); tr.lastChild.appendChild(affEl(x.aff)); return tr; };
  const who = (x, k) => [x.face, el('span', 'afn', x.name), ...(x.tag ? [el('span', 'aft', x.tag)] : []), ...(k != null ? [el('span', 'afk', `Legacy ${k + 1}`)] : [])];
  legs.forEach((p, k) => { const tb = el('tbody', 'leg' + k); tb.appendChild(row('par', who(p, k), p));
    for (const g of p.gps) tb.appendChild(row('gp', who(g), g)); t.appendChild(tb); });
  const all = legs.flatMap(p => [p, ...p.gps]), sum = (f) => all.reduce((a, x) => a + f(x), 0);
  t.tFoot.appendChild(row('tot', [el('span', 'afk', 'Total')], { rel: sum(x => x.rel), race: sum(x => x.race), aff: sum(x => x.aff) }));
  return t;
}
