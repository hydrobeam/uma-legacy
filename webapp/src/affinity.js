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
/**
 * One career in the solver's affinity detail (`loop.detail.lineages`); uma numbers index the loop's members.
 * @typedef {object} Lineage
 * @property {number} t the trainee
 * @property {[number, number]} p her parents
 * @property {LegacySlot[]} slots her six legacies
 */
/**
 * @typedef {object} LegacySlot
 * @property {'parent' | 'grandparent'} kind
 * @property {number[]} who [parent] or [parent, grandparent]
 * @property {number} rel relation points
 * @property {number} race the G1 wins shared (expected), as points
 * @property {number} aff affinity
 */
const careerTotal = (lineage) => lineage.slots.reduce((sum, slot) => sum + slot.aff, 0);
const button = (label) => { const b = document.createElement('button'); b.type = 'button'; b.setAttribute('role', 'radio'); b.setAttribute('aria-label', label); return b; };
const radioGroup = (className, label) => { const box = document.createElement('div'); box.className = className; box.setAttribute('role', 'radiogroup'); box.setAttribute('aria-label', label); return box; };
// a pill row as a radio group: the arrows move along it (wrapping) and pick. `items()`: its [career, button] pairs now
function arrowKeys(box, items, show) {
  box.addEventListener('keydown', (e) => {
    const list = items(), k = list.findIndex(([, b]) => b === document.activeElement);
    if (k < 0) return;
    const to = { ArrowRight: k + 1, ArrowDown: k + 1, ArrowLeft: k - 1, ArrowUp: k - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    const [career, b] = list[(to + list.length) % list.length]; show(career); b.focus();
  });
}
// pills: [career, button, key]; the one whose key `isOn` accepts is checked (and the one Tab reaches)
const mark = (pills, isOn) => pills.forEach(([, b, key]) => {
  const on = isOn(key); b.setAttribute('aria-checked', String(on)); b.setAttribute('aria-pressed', String(on)); b.tabIndex = on ? 0 : -1;
});
export function openWhy(loop) {
  const lineages = loop.detail.lineages, members = loop.members, body = $('why-body'); body.textContent = '';
  const shortName = (i) => members[i].short || members[i].name;
  const outfitOf = (i) => pickOutfit(loop.members[i].id, loop.outfits[i].outfits).list[0]?.id ?? null;
  const face = (i, size) => umaAv(members[i].id, outfitOf(i), size);
  const head = document.createElement('div'); head.className = 'row between dhead';
  head.innerHTML = '<h3 id="why-title">Affinity</h3>';
  const close = document.createElement('button'); close.className = 'btn'; close.textContent = 'Close'; close.onclick = () => $('why-dlg').close(); head.appendChild(close);
  body.appendChild(head);
  // the careers to pick from: one pill per trainee (a rotation's three); in a four-uma loop each trainee heads three
  // careers (her parents two of the other three), so a second row picks her parents
  const traineeRow = radioGroup('afpick', 'Trainee'), parentRow = radioGroup('afpick afpar', 'Parents');
  const view = document.createElement('div'); view.className = 'afview';
  body.append(traineeRow, parentRow, view);
  const careersOf = new Map();   // trainee -> her careers (lineage indices), the most affinity first
  const byAffinity = lineages.map((_, i) => i).sort((a, b) => careerTotal(lineages[b]) - careerTotal(lineages[a]));
  for (const i of byAffinity) {
    const trainee = lineages[i].t;
    if (!careersOf.has(trainee)) careersOf.set(trainee, []);
    careersOf.get(trainee).push(i);
  }
  const traineePills = [...careersOf].map(([trainee, careers]) => {
    const best = lineages[careers[0]], [p0, p1] = best.p;
    const b = button(careers.length > 1 ? `${members[trainee].name} as the trainee` : `${members[trainee].name}, parents ${members[p0].name} and ${members[p1].name}`);
    b.title = b.getAttribute('aria-label');
    b.append(face(trainee, 28), Object.assign(document.createElement('span'), { className: 'afpn', textContent: shortName(trainee) }));
    const heart = affEl(careerTotal(best)); if (heart) b.appendChild(heart);   // her best career's affinity
    b.onclick = () => show(careers[0]); traineeRow.appendChild(b);
    return [careers[0], b, trainee];
  });
  let parentPills = [];
  arrowKeys(traineeRow, () => traineePills, show); arrowKeys(parentRow, () => parentPills, show);
  // a career's table rows (afftable.js): each parent, then her two parents, with their relation points, shared G1s and affinity
  function legaciesOf(lineage) {
    const own = (who) => who === lineage.t ? 'her own earlier veteran' : null;
    return lineage.p.map(parent => {
      const slot = lineage.slots.find(s => s.kind === 'parent' && s.who[0] === parent), parentFace = face(parent, 30);
      const grandparents = lineage.slots.filter(s => s.kind === 'grandparent' && s.who[0] === parent).map(g => ({
        face: face(g.who[1], 24), name: members[g.who[1]].name, tag: own(g.who[1]), rel: g.rel, race: g.race, aff: g.aff }));
      return { face: parentFace, name: members[parent].name, rel: slot.rel, race: slot.race, aff: slot.aff, gps: grandparents };
    });
  }
  function show(i) {
    const lineage = lineages[i], careers = careersOf.get(lineage.t);
    mark(traineePills, (trainee) => trainee === lineage.t);
    // her parents, when she heads more than one career: both faces and that career's affinity
    parentRow.hidden = careers.length < 2; parentRow.textContent = '';
    if (careers.length > 1) parentRow.append(Object.assign(document.createElement('span'), { className: 'aflab', textContent: 'Parents' }));
    parentPills = careers.length < 2 ? [] : careers.map(j => {
      const career = lineages[j], [p0, p1] = career.p;
      const b = button(`parents ${members[p0].name} and ${members[p1].name}`);
      b.title = `${members[career.t].name}: ${b.getAttribute('aria-label')}`;
      b.append(face(p0, 24), face(p1, 24));
      const heart = affEl(careerTotal(career)); if (heart) b.appendChild(heart);
      b.onclick = () => show(j); parentRow.appendChild(b);
      return [j, b, j];
    });
    mark(parentPills, (j) => j === i);
    view.replaceChildren(affTable(legaciesOf(lineage), `${members[lineage.t].name}: her six legacies`));
  }
  show(traineePills[0][0]);
  $('why-dlg').showModal();
}
