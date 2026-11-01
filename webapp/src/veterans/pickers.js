// ---------- the veterans' editors: the uma picker, the skills prompt, the G1 wins window, the "+" list ----------
import { signal } from '@preact/signals-core';
import { $, $dialog, $input } from '../dom.js';
import { careerTables, gameData } from '../data.js';
import { umaAv } from '../images.js';
import { winsAgenda } from '../agenda.js';
import { must } from '../must.ts';
import { NEW_STARS } from './store.ts';
import { umaName, skillName, targetGroups } from './names.js';
/** @import { NodeRef, NodeRecord, Sparks } from './store.ts' */

const allTargetsAt = (stars) => Object.fromEntries(targetGroups().map((g) => [g, stars]));

// ---------- dialogs ----------
for (const d of document.querySelectorAll('dialog')) {
  for (const b of d.querySelectorAll('.dclose')) b.onclick = () => d.close();
}
// the uma picker: every uma (or those `only` lets through), filterable; onPick(chara id)
/** @type {((chara: number) => void) | null} */
let onUmaPicked = null;
/** @param {string} title @param {(chara: number) => void} onPick @param {((u: import('../data.js').Uma) => boolean)|null} [only] */
export function openUmaPicker(title, onPick, only = null) {
  onUmaPicked = onPick;
  $('pick-title').textContent = title;
  $input('pick-filter').value = '';
  drawUmas($('pick-list'), '', pickUma, only);
  $dialog('pick-dlg').showModal();
  $input('pick-filter').focus();
}
function pickUma(id) {
  $dialog('pick-dlg').close();
  const onPick = onUmaPicked;
  onUmaPicked = null;
  onPick?.(id);
}
$input('pick-filter').oninput = () => filterUmas($('pick-list'), $input('pick-filter').value);
// the picker's list: every uma `only` lets through, the filter applied by hiding (typing doesn't rebuild it)
/** @param {HTMLElement} box @param {string} filter @param {(chara: number) => void} onPick @param {((u: import('../data.js').Uma) => boolean)|null} [only] */
export function drawUmas(box, filter, onPick, only = null) {
  box.textContent = '';
  for (const u of [...gameData().umas].sort((a, b) => a.name.localeCompare(b.name))) {
    if (only && !only(u)) continue;
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'upick';
    b.append(umaAv(u.id, null, 40));
    const s = document.createElement('span');
    s.textContent = u.name;
    b.append(s);
    b._nm = u.name.toLowerCase();
    b.onclick = () => onPick(u.id);
    box.appendChild(b);
  }
  const none = document.createElement('p');
  none.className = 'hint';
  none.textContent = 'No uma matches.';
  box.appendChild(none);
  filterUmas(box, filter);
}
/** @param {HTMLElement} box @param {string} filter */
export function filterUmas(box, filter) {
  const f = filter.trim().toLowerCase();
  let n = 0;
  for (const b of box.querySelectorAll('.upick')) {
    b.hidden = !!f && !must(b._nm, 'the picker names each button').includes(f);
    n += Number(!b.hidden);
  }
  must(box.querySelector('p.hint'), 'the picker has its no-match hint').hidden = n > 0;
}
// after an uma is picked for a node that carries sparks: whether to give her the target skills. onChoice(sparks | null:
// keep what she has)
/** @param {string} name @param {boolean} hasSparks @param {(sparks: Sparks|null) => void} onChoice @param {boolean} [withParents] */
export function askSkills(name, hasSparks, onChoice, withParents = false) {
  const n = targetGroups().length;
  $('ask-title').textContent = `Add skills to ${name}?`;
  $('ask-text').textContent =
    `Her sparks of your ${n} target skills decide what she passes on. A fully looped parent has all of them` +
    (withParents ? ', and so do her own two parents: the same choice goes on them.' : '.') +
    ' You can change the stars of each afterwards.';
  const box = $('ask-btns');
  box.textContent = '';
  // No skills first and the default (a new uma carries nothing until you say so)
  const choices = [
    { label: 'No skills', sparks: () => ({}), primary: true },
    { label: `All ${n} at 2★`, sparks: () => allTargetsAt(2) },
    { label: `All ${n} at 3★`, sparks: () => allTargetsAt(3) },
    { label: `All ${n} at 1★`, sparks: () => allTargetsAt(1) },
    ...(hasSparks ? [{ label: 'Keep her current skills', sparks: () => null }] : []),
  ];
  for (const { label, sparks, primary } of choices) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = `btn${primary ? ' primary' : ''}`;
    b.textContent = label;
    b.onclick = () => {
      $dialog('ask-dlg').close();
      onChoice(sparks());
    };
    box.appendChild(b);
  }
  $dialog('ask-dlg').showModal();
  must(box.querySelector('.primary'), 'the prompt is made with its first choice').focus();
}
// the G1 wins of one node, on the schedule window's calendar (the editor in agenda.js): each win in its slot, click it to
// remove it; an empty slot where a G1 is held lists the races held then. Each edit lands on her record as it is then
// (`at`). A pool veteran's wins are saved on each edit, and the page searches again when the dialog closes
// (`winsOpen`). opts (the veteran analysis' draft): {offer, info, onClose} (see winsAgenda). winAt {race index: turn} is
// where a win shows, for display only (the solver reads `wins`).
const allRaces = () =>
  careerTables().races.map((name, ix) => ({ ix, name, turns: (careerTables().race_turns || [])[ix] || [] }));
/** @type {(() => void) | null} */
let winsClose = null;
// a pool veteran's wins window is open: the page holds its search until it closes
const editingWins = signal(false);
export const winsOpen = /** @type {import('@preact/signals-core').ReadonlySignal<boolean>} */ (editingWins);
/** @param {NodeRef} at @param {string} who @param {{offer?: any, info?: any, onClose?: () => void}} [opts] */
export function openWins(at, who, opts = {}) {
  const opener = /** @type {HTMLElement} */ (document.activeElement);
  $('wins-title').textContent = `G1 wins: ${who}`;
  const now = () => at.node() || { wins: [], winAt: {} };
  const count = () => {
    $('wins-count').textContent = winsText(now());
  };
  $('wins-list').replaceChildren(
    winsAgenda({
      races: allRaces(),
      get: () => now().wins,
      at: () => now().winAt || {},
      offer: opts.offer,
      info: opts.info,
      onWins: (wins, winAt) => {
        at.edit(() => ({ wins, winAt }));
        count();
      },
    }),
  );
  count();
  // closing: the focus goes back to the button that opened it (the views keep it)
  winsClose =
    opts.onClose ||
    (() => {
      editingWins.value = false;
      if (opener?.isConnected) opener.focus();
    });
  if (!opts.onClose) editingWins.value = true;
  $dialog('wins-dlg').showModal();
}
/** @type {(nd: {wins: readonly number[]}) => string} */
export const winsText = (nd) =>
  nd.wins.length ? `${nd.wins.length} G1 win${nd.wins.length > 1 ? 's' : ''}` : 'No G1 wins';
$dialog('wins-dlg').addEventListener('close', () => {
  const f = winsClose;
  winsClose = null;
  f?.();
});
// "+" beside a node's sparks: the target skills she lacks, as the game's white spark cells (her blue, pink and unique
// sparks have bars of their own, each opening a picker)
const pop = $('add-pop');
/** @type {HTMLElement | null} */
let popFor = null,
  popDrawn = false; // popFor is a component's: it draws its aria-expanded from `addOpener`, nothing here writes it
const addFor = signal(/** @type {HTMLElement | null} */ (null));
/** the "+" whose list is open (null: none) */
export const addOpener = /** @type {import('@preact/signals-core').ReadonlySignal<HTMLElement | null>} */ (addFor);
function closeAdd() {
  pop.hidden = true;
  if (popFor && !popDrawn) popFor.setAttribute('aria-expanded', 'false');
  popFor = null;
  addFor.value = null;
}
/** @param {HTMLElement} btn @param {NodeRef} at @param {boolean} [drawn] btn is a component's */
export function openAdd(btn, at, drawn = false) {
  if (popFor === btn) {
    closeAdd();
    return;
  }
  closeAdd();
  popFor = btn;
  popDrawn = drawn;
  if (!drawn) btn.setAttribute('aria-expanded', 'true');
  addFor.value = btn;
  pop.textContent = '';
  const head = (t) => {
    const h = document.createElement('p');
    h.className = 'hint';
    h.textContent = t;
    pop.appendChild(h);
  };
  const sparks = /** @type {Record<number, number>} */ (at.node()?.sparks || {}),
    missing = targetGroups().filter((g) => !sparks[g]);
  head(missing.length ? 'Target skills she lacks' : 'She has every target skill.');
  const list = document.createElement('div');
  list.className = 'addlist';
  for (const g of missing) {
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'addsk';
    b.append(Object.assign(document.createElement('span'), { className: 'addn', textContent: skillName(g) }));
    b.onclick = () => {
      closeAdd();
      at.edit((now) => ({ sparks: { ...now.sparks, [g]: NEW_STARS } }));
    };
    list.appendChild(b);
  }
  pop.appendChild(list);
  const r = btn.getBoundingClientRect();
  // inside an open (modal) dialog the page around it is inert: the menu moves into that dialog, placed relative to it
  const host = btn.closest('dialog[open]');
  if ((host || document.body) !== pop.parentElement) (host || document.body).appendChild(pop);
  if (host) {
    const h = host.getBoundingClientRect();
    pop.style.left = `${Math.max(4, Math.min(r.left - h.left, h.width - 300))}px`;
    pop.style.top = `${r.bottom - h.top + host.scrollTop + 6}px`;
  } else {
    pop.style.left = `${Math.max(8, Math.min(r.left + scrollX, scrollX + innerWidth - 320))}px`;
    pop.style.top = `${r.bottom + scrollY + 6}px`;
  }
  pop.hidden = false;
  pop.focus();
}
document.addEventListener('click', (/** @type {TargetedEvent} */ e) => {
  if (!pop.hidden && !pop.contains(e.target) && !popFor?.contains(e.target)) closeAdd();
});
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape' && !pop.hidden) {
    const b = popFor;
    closeAdd();
    b?.focus();
  }
});
// change who a node is: pick the uma (saved, and the page searches again), then whether to add the target skills (none
// chosen, as Escape on the prompt: she keeps her sparks)
/** @param {NodeRef} at @param {string} label */
export function changeUma(at, label) {
  openUmaPicker(`${label}: which uma?`, (id) => {
    at.edit(() => ({ chara: id, outfit: undefined }));
    const sparks = at.node()?.sparks || {};
    askSkills(umaName(id), Object.keys(sparks).length > 0, (sp) => {
      if (sp) at.edit(() => ({ sparks: sp }));
    });
  });
}
