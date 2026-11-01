// The parent picker: a veteran for a parent place, a new one, or the place handed back to the planner.
import { $, $dialog, $input, el } from '../../dom.js';
import { settings } from '../../settings.ts';
import { umaAv } from '../../images.js';
import { vets, plan, changed, vetById, umaName, askSkills, newVet, drawUmas } from '../../vets.js';
import { BORROW_CHOICES, borrowChoice, pairBlock } from '../job.js';

// ---------- the parent picker ----------
export let pickK = 0;
export function vpTab(t) {
  for (const x of ['vets', 'new', 'auto']) {
    $(`vp-tab-${x}`).setAttribute('aria-selected', String(x === t));
    $(`vp-${x}`).hidden = x !== t;
  }
  if (t === 'new') {
    $input('vp-filter').value = '';
    drawNew();
    $input('vp-filter').focus();
  }
}
const drawNew = () =>
  drawUmas($('vp-umas'), $input('vp-filter').value, (id) => {
    $dialog('vpick-dlg').close();
    askSkills(
      umaName(id),
      false,
      (sp) => {
        const v = newVet(id, sp || {});
        plan.parents[pickK] = v.id;
        changed();
      },
      true,
    );
  });
// why veteran v can't go in place k (or '')
const BLOCKED = {
  trainee: 'the trainee herself',
  same: 'same uma as the other parent',
  both: 'the other parent is borrowed',
};
function blockedFor(v, k) {
  const otherId = plan.parents[1 - k];
  const why = pairBlock(plan.trainee, v, otherId && vetById(otherId), settings.value);
  return why === 'setting'
    ? `borrowed (Borrowed parent: ${BORROW_CHOICES[borrowChoice(settings.value)]})`
    : BLOCKED[why] || '';
}
export function openParentPicker(k) {
  pickK = k;
  $('vpick-title').textContent = `Choose parent ${k + 1}`;
  const box = $('vp-list');
  box.textContent = '';
  let headed = false;
  vets.forEach((v, i) => {
    if (v.src === 'umamoe' && !headed) {
      box.appendChild(el('h4', 'vpgh', 'uma.moe'));
      headed = true;
    }
    const why = blockedFor(v, k),
      cur = plan.parents[k] === v.id;
    const b = el('button', `vpt${cur ? ' cur' : ''}${v.borrowed ? ' bor' : ''}`);
    b.type = 'button';
    b.disabled = !!why;
    const top = el('div', 'vpth');
    top.append(umaAv(v.chara, v.outfit, 48));
    const nm = el('div');
    nm.append(
      el('div', 'nm', umaName(v.chara)),
      el(
        'div',
        'sub',
        v.lender
          ? `${v.lender.name}${cur ? ', chosen' : ''}`
          : `#${i + 1}${v.borrowed ? ', borrowed' : ''}${cur ? ', chosen' : ''}`,
      ),
    );
    top.appendChild(nm);
    b.appendChild(top);
    const tg = settings.value.targets.map((t) => t.group),
      have = tg.filter((g) => v.sparks[g]),
      stars = have.reduce((a, g) => a + v.sparks[g], 0);
    b.appendChild(el('div', 'vpts', `${have.length} of ${tg.length} target skills, ${stars}★`));
    const gp = el('div', 'vptg');
    gp.append('Parents: ');
    for (const g of v.parents) {
      gp.append(umaAv(g.chara || 0, g.outfit, 22));
    }
    gp.append(` ${v.parents.map((g) => (g.chara ? umaName(g.chara) : 'unknown')).join(', ')}`);
    b.appendChild(gp);
    if (why) b.appendChild(el('div', 'vptw', `Can't: ${why}`));
    b.onclick = () => {
      plan.parents[k] = v.id;
      plan.fromTable = false; // (chosen by hand: it stays)
      $dialog('vpick-dlg').close();
      changed();
    };
    box.appendChild(b);
  });
  if (!vets.length)
    box.innerHTML = '<p class="hint">No veterans yet: import them in the paddock below, or make a new one.</p>';
  vpTab(vets.length ? 'vets' : 'new');
  $dialog('vpick-dlg').showModal();
}
