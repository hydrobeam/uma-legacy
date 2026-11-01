// The lineage tree: the trainee and the two legacies with their parents, and the career's odds with Try a run.
import { $, $dialog, el } from '../../dom.js';
import { umaAv } from '../../images.js';
import { affEl } from '../../afftable.js';
import { vets, plan, umaName } from '../../vets.js';
import { renderSimulation } from '../analysis.js';
import { must } from '../../must.ts';
import { idxOf, shown, parentAt } from '../job.js';
import { btn, tile, sectionEl } from './parts.js';
import { nodeDialog, openNode, fillNode } from './NodeDialog.js';
import { oddsBtn, deckDialog, openDeck, fillDeck, targetsEl, schedBtn } from './Sections.js';

// a node of the pedigree: what it is (its header), then who; ghost: the planner's pick, not yours
function pnode(kind, label, ghost) {
  const n = el('div', `pn ${kind}${ghost ? ' ghost' : ''}`);
  const h = el('div', 'pnh');
  h.appendChild(el('span', 'pnl', label));
  n.appendChild(h);
  return n;
}
// The tree, as the game's legacy display: portraits only (the trainee; Legacy 1 blue and Legacy 2 pink, each the parent
// with her own two parents), each with its legacy number and affinity; a portrait opens the node popup (her sparks,
// G1 wins, choose or hand back the place, change who she is). The popup is refilled after every edit (`fillNode`).
// a click anywhere in a node's box opens it, as its portrait does (not on the box's own buttons or deck cards)
const wholeBox = (box, t) => {
  box.classList.add('pnclick');
  box.addEventListener('click', (e) => {
    if (!e.target.closest('button, a, .deck, input')) t.click();
  });
};
function traineeEl(r) {
  const id = plan.trainee || r?.trainee.id;
  const n = pnode('trainee', plan.trainee ? 'Your pick' : "Planner's pick", !plan.trainee);
  // the affinity she holds from her six legacies (its breakdown is in her popup)
  if (r && r.trainee.id === id) {
    const tot = legacyAff(r),
      a = must(affEl(tot, 'aff tot'), 'her legacies sum to an affinity');
    a.title = `Affinity ${Math.round(tot)}, her six legacies: click her for the breakdown`;
    must(n.querySelector('.pnh'), "pnode() makes the place's head").appendChild(a);
  }
  const sub = id && r && r.trainee.id === id ? r.trainee.outfits[0]?.label || null : null;
  const t = id
    ? tile('node:t', umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 96), umaName(id), sub)
    : tile('node:t', el('span', 'av empty'), 'Any uma', 'The planner tries every uma you own.');
  t.setAttribute('aria-label', `Trainee: ${id ? umaName(id) : 'the planner picks'}. Details`);
  t.onclick = () => openNode({ kind: 't' });
  n.appendChild(t);
  wholeBox(n, t);
  // her deck and her race schedule, each in its popup
  if (r && r.trainee.id === id) {
    const row = el('div', 'row tdeck');
    const db = btn('Deck', '', () => openDeck(r));
    db.id = 'deck-open';
    db.setAttribute('aria-haspopup', 'dialog');
    row.append(db, schedBtn(r));
    n.appendChild(row);
  }
  return n;
}
// her affinity, in all and legacy by legacy (afftable.js), from the report's six slots (career/inherit.rs `slots`);
// Legacy 1 and 2 follow the career's parents (a slot's legacy is her index among your veterans)
const legacyAff = (r) => r.slots.reduce((a, x) => a + x.aff, 0);
function familyEl(k, r) {
  const fam = el('div', 'pf');
  // the game's legacy box: "Legacy 1" blue, "Legacy 2" pink, the parent with her own two parents
  fam.appendChild(el('span', 'pflab', `Legacy ${k + 1}`));
  const at = parentAt(k, r);
  const n = el('div', `pn parent${at?.pick ? ' ghost' : ''}`);
  if (at?.v.borrowed) n.classList.add('bor');
  let t;
  if (at) {
    const v = at.v;
    t = tile(
      `node:p${k}`,
      umaAv(v.chara, v.outfit, 64),
      umaName(v.chara),
      v.lender
        ? `uma.moe · ${v.lender.name || 'trainer'}`
        : `Veteran #${idxOf(v.id) + 1}${v.borrowed ? ' · borrowed' : v.src === 'hand' ? ', made here' : ''}`,
    );
    t.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Details`);
  } else {
    t = tile(`node:p${k}`, el('span', 'av empty'), vets.length >= 2 ? 'The planner picks' : 'No parent yet', null);
    t.setAttribute('aria-label', `Parent ${k + 1}: none yet. Details`);
  }
  t.onclick = () => openNode({ kind: 'p', k });
  n.appendChild(t);
  wholeBox(n, t);
  fam.appendChild(n);
  // her own parents: the trainee's grandparents
  if (at) {
    const gk = el('div', 'pgs');
    at.v.parents.forEach((g, j) => {
      const gn = el('div', 'pn gp');
      const gt = tile(
        `node:p${k}g${j}`,
        umaAv(g.chara || 0, g.outfit, 36),
        g.chara ? umaName(g.chara) : 'Unknown: pick her',
        null,
      );
      gt.setAttribute('aria-label', `Grandparent: ${g.chara ? umaName(g.chara) : 'unknown'}. Details`);
      gt.onclick = () => openNode({ kind: 'g', k, j });
      gn.appendChild(gt);
      wholeBox(gn, gt);
      gk.appendChild(gn);
    });
    fam.appendChild(gk);
  }
  return fam;
}
// The next career as the player sets it up in the game, in the game's order: the odds, then whom to follow (a borrow
// from uma.moe), the trainee, her two legacies, the deck, and each target's chance to land
export function renderTree() {
  const r = shown(),
    box = $('tree');
  box.textContent = '';
  $('lodds').replaceChildren(...(r ? verdictEl(r).childNodes : []));
  const t = sectionEl('ltrainee', 'Trainee');
  t.appendChild(traineeEl(r));
  box.appendChild(t);
  const lg = sectionEl('llegacies', 'Legacies'),
    k = el('div', 'pk');
  k.append(familyEl(0, r), familyEl(1, r));
  lg.appendChild(k);
  box.appendChild(lg);
  if (r && r.trainee.id === (plan.trainee || r.trainee.id)) {
    box.append(targetsEl(r));
    if (deckDialog.made()?.open) fillDeck(r);
  }
  if (nodeDialog.made()?.open) fillNode();
}
// the career's odds of landing every target and Try a run, beside the title
function verdictEl(r) {
  const v = el('div', 'lodds');
  // Try a run: a roll of the veteran this career leaves and her comparison, in a popup
  const tr = btn('Try a run', '', () => {
    renderSimulation();
    $dialog('sim-dlg').showModal();
  });
  tr.id = 'sim-try';
  tr.setAttribute('aria-haspopup', 'dialog');
  tr.title = 'Roll a random result of this career and see whether the veteran it leaves helps your loop';
  v.append(oddsBtn(r), tr);
  return v;
}
