// ---------- a career's affinity, legacy by legacy (both planner pages) ----------
// Each of the six legacies has its own affinity, which weighs its own inheritance rolls: a parent's counts her relation
// with the trainee and the other parent, the trio relations with her own two parents and the G1s she shares with all
// three (3 points each); a grandparent's, her trio relation and the G1s she shares with the parent between them. The
// total is the six summed. legs: [{ face, name, tag?, rel, race, aff, gps: [{ face, name, tag?, rel, race, aff }] }]
// (Legacy 1 first); face an element (her portrait).
import { el } from './dom.js';
// her affinity as the game's heart and the number
export function affEl(affinity, cls = 'aff') {
  if (affinity === null || affinity === undefined) return null;
  const shown = Math.round(affinity), heart = el('span', cls);
  heart.append(el('i', 'heart'), String(shown));
  heart.setAttribute('aria-label', `Affinity ${shown}`); heart.title = `Affinity ${shown}`;
  return heart;
}
export function affTable(legs, caption = 'Affinity') {
  const table = el('table', 'afftab'), whole = (x) => String(Math.round(x));
  table.innerHTML = '<thead><tr><th scope="col"></th><th scope="col">Relation</th><th scope="col">G1 wins shared</th><th scope="col">Affinity</th></tr></thead><tfoot></tfoot>';
  if (caption) table.prepend(Object.assign(el('caption'), { textContent: caption }));
  // one row: who (her face, name, tag, legacy number), then the legacy's relation, shared G1s and affinity
  const row = (cls, who, legacy) => {
    const tr = el('tr', cls), th = el('th'); th.scope = 'row'; th.append(...who);
    tr.append(th, el('td', '', whole(legacy.rel)), el('td', '', whole(legacy.race)), el('td', 'afv'));
    tr.lastChild.appendChild(affEl(legacy.aff));
    return tr;
  };
  // legacyIndex: the parent's place (Legacy 1, 2); a grandparent's row has none
  const who = (legacy, legacyIndex) => [legacy.face, el('span', 'afn', legacy.name),
    ...(legacy.tag ? [el('span', 'aft', legacy.tag)] : []),
    ...(legacyIndex != null ? [el('span', 'afk', `Legacy ${legacyIndex + 1}`)] : [])];
  legs.forEach((parent, k) => {
    const group = el('tbody', `leg${k}`); group.appendChild(row('par', who(parent, k), parent));
    for (const grandparent of parent.gps) group.appendChild(row('gp', who(grandparent), grandparent));
    table.appendChild(group);
  });
  const everyone = legs.flatMap(parent => [parent, ...parent.gps]), sum = (field) => everyone.reduce((total, x) => total + field(x), 0);
  table.tFoot.appendChild(row('tot', [el('span', 'afk', 'Total')], { rel: sum(x => x.rel), race: sum(x => x.race), aff: sum(x => x.aff) }));
  return table;
}
