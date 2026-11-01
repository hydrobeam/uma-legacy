// The veterans dialog: its two tabs, sort menu, cards and which one the wide pane shows (NodeDialog.tsx's VetPane
// draws her). With it, the setup panel's fold: the first
// veterans arriving fold the setup (`renderPool` writes `folded`, so the two share a module).
import { $, $dialog, el } from '../../dom.js';
import { ERA } from '../../saved.ts';
import { RANKS } from '../../data.js';
import { settings } from '../../settings.ts';
import { scenarioOf } from '../../scenario.js';
import { vets, places } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.js';
import { vetCell } from '../../vetcell.js';
import { wireMenu } from '../../sortmenu.js';
import { umamoeButton } from '../../umamoesetup.js';
import { must } from '../../must.ts';
import { idxOf, BORROW_CHOICES, borrowChoice } from '../job.js';
import { openNode, showPane } from './dialogs.ts';

// ---------- the veterans (pool) ----------
// a veteran as the game's legacy list draws her, her number (an uma.moe one: her trainer) under her, a "Parent 1" tab
// when she is one of the next career's parents; a click shows her beside the list (a wide dialog) or in her popup (her
// sparks editable in place, Yours / Guest, Remove)
function vetCardEl(v, i) {
  const k = places.value.parents.indexOf(v.id),
    tg = settings.value.targets.map((t) => t.group),
    have = tg.filter((g) => v.sparks?.[g]).length,
    c = vetCell(v, {
      name: v.lender ? v.lender.name || 'uma.moe' : `#${i + 1}`,
      label: `Veteran #${i + 1}: ${umaName(v.chara)}, ${have} of ${tg.length} target skills. Her sparks`,
      onClick: () => {
        if (!wideVets()) openNode({ kind: 'v', id: v.id }, () => focusCard(v.id));
        else if (vSel !== v.id) {
          vSel = v.id;
          markSel();
          drawPane();
        }
      },
      tab: k >= 0 ? `Parent ${k + 1}` : undefined,
    });
  c.id = `vet-${i}`;
  c.dataset.k = `node:v${v.id}`;
  return c;
}
// your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog
// (the cards are drawn only while the dialog is open, and when it opens: every edit and result redraws the page)
// Two tabs, as the game's Legacy Select: Veteran Umamusume (yours) and Guests (the ones you can borrow: your borrow list,
// then uma.moe's listings); a veteran keeps her number (her place in the whole list) in either. A wide dialog shows the
// chosen one beside the list (`vSel`); a narrow one opens her popup.
export let poolStale = true,
  vTab = 'own',
  /** @type {string|null} */ vSel = null;
const wideVets = () => matchMedia('(min-width: 760px)').matches;
export const inTab = (v) => !!v.borrowed === (vTab === 'bor');
export function setVTab(t) {
  vTab = t;
  for (const x of ['own', 'bor']) $(`v-tab-${x}`).setAttribute('aria-selected', String(x === t));
  $('vets').setAttribute('aria-labelledby', `v-tab-${t}`);
  renderPool();
}
// the list's orders: the order added (uma.moe's listings in uma.moe's order), how many targets she carries (the
// default; ties in the order added), her rank; each starts as the menu says and picking the chosen one again reverses
// it (as the umas' and cards' sort menus)
/** @type {Record<string, [string, string, (v: import('../../veterans/store.ts').VetRecord, i: number) => number]>} */
const VSORTS = {
  no: ['Added', 'In the order they were added to this page (uma.moe’s: in its own order)', (_v, i) => -i],
  tg: [
    'Targets',
    'The most targets she carries first',
    (v) => settings.value.targets.filter((t) => v.sparks?.[t.group] > 0).length,
  ],
  rank: ['Rank', 'The highest career rank first', (v) => v.rank || 0],
};
let vSort = 'tg',
  vRev = false;
function paintVSort() {
  const arrow = vRev ? '↑' : '↓',
    b = $('vs');
  b.textContent = `${VSORTS[vSort][0]} ${arrow}`;
  b.title = VSORTS[vSort][1] + (vRev ? ', reversed' : '');
  b.setAttribute('aria-label', `${b.textContent}, order veterans`);
  for (const x of $('vs-menu').querySelectorAll('button')) {
    const on = x.dataset.k === vSort;
    x.setAttribute('aria-checked', String(on));
    const [name, hint] = VSORTS[must(x.dataset.k, 'a menu item names its key')];
    x.textContent = name + (on ? ` ${arrow}` : '');
    x.title = on ? 'Click to reverse' : hint;
  }
}
export function initVSort() {
  for (const k of Object.keys(VSORTS)) {
    const x = el('button', '', VSORTS[k][0]);
    x.type = 'button';
    x.setAttribute('role', 'menuitemradio');
    x.dataset.k = k;
    $('vs-menu').appendChild(x);
  }
  wireMenu('vs', (k) => {
    vRev = k === vSort && !vRev;
    vSort = k;
    drawPool();
  });
}
export function renderPool() {
  // the first veterans arriving fold the setup (as a reload with veterans does), unless you folded or opened it
  if (!folded && vets.value.length) {
    /** @type {string | null} */
    let chosen = null;
    try {
      chosen = localStorage.getItem('llp:setup-folded');
    } catch {
      // storage blocked: as if no fold was chosen
    }
    if (chosen === null) {
      folded = true;
      paintFold();
    }
  }
  umamoeButton();
  const guests = vets.value.filter((v) => v.borrowed).length;
  $('v-tab-own').textContent = `Veteran Umamusume · ${vets.value.length - guests}`;
  $('v-tab-bor').textContent = `Guests · ${guests}`;
  if (!$dialog('set-vets').open) {
    poolStale = true;
    return;
  }
  drawPool();
}
// the cards drawn last, by veteran: one whose record, number, parent tab and the targets are as they were is kept, so an
// edit or a tab switch builds only the cards that changed (300 veterans are about 1,900 elements)
/** @type {Map<string, {sig: string, card: HTMLElement}>} */
const poolCards = new Map();
/** @param {import('../../veterans/store.ts').VetRecord} v @param {number} i */
function poolCard(v, i) {
  const sig = JSON.stringify([v, i, places.value.parents.indexOf(v.id), settings.value.targets]),
    kept = poolCards.get(v.id);
  if (kept?.sig === sig) return kept.card;
  const card = vetCardEl(v, i);
  poolCards.set(v.id, { sig, card });
  return card;
}
// the chosen one's mark (beside the list on a wide dialog)
function markSel() {
  const selId = `vet-${idxOf(vSel)}`;
  for (const c of $('vets').querySelectorAll('.umcell')) {
    const on = c.id === selId;
    c.classList.toggle('sel', on);
    c.setAttribute('aria-current', String(on && wideVets()));
  }
}
// the one beside the list: NodeDialog.tsx's VetPane draws her
const drawPane = () => showPane(vSel && wideVets() ? vSel : null);
// her card, drawn again since her popup opened (an edit in it): the focus goes back to it
const focusCard = (id) => document.getElementById(`vet-${idxOf(id)}`)?.focus();
export function drawPool() {
  const box = $('vets');
  poolStale = false;
  paintVSort();
  const by = VSORTS[vSort][2],
    sign = vRev ? -1 : 1,
    shown = vets.value
      .map((v, i) => /** @type {[import('../../veterans/store.ts').VetRecord, number]} */ ([v, i]))
      .filter(([v]) => inTab(v))
      .sort(([a, i], [b, j]) => sign * (by(b, j) - by(a, i)));
  // Guests: your borrow list, then uma.moe's listings, each under its heading
  /** @type {[string, [import('../../veterans/store.ts').VetRecord, number][]][]} */
  const groups =
    vTab === 'own'
      ? [['', shown]]
      : [
          ['Borrow list', shown.filter(([v]) => v.src !== 'umamoe')],
          ['uma.moe', shown.filter(([v]) => v.src === 'umamoe')],
        ];
  const nodes = [];
  for (const [head, list] of groups) {
    if (!list.length) continue;
    if (head) nodes.push(el('p', 'vsec', head));
    for (const [v, i] of list) nodes.push(poolCard(v, i));
  }
  box.replaceChildren(...nodes);
  const ids = new Set(vets.value.map((v) => v.id));
  for (const id of poolCards.keys()) if (!ids.has(id)) poolCards.delete(id);
  if (!shown.length)
    box.innerHTML =
      vTab === 'own'
        ? '<p class="hint">None yet. Import umadump\'s trained_chara_data.json, or make one with Choose parent.</p>'
        : '<p class="hint">None yet. Import umadump\'s rental_chara_data.json, mark a veteran Guest, or Borrow from uma.moe beside Next career.</p>';
  // the one beside the list: the chosen one while she is in this tab, else the first shown
  if (!shown.some(([v]) => v.id === vSel)) vSel = shown[0]?.[0].id ?? null;
  markSel();
  drawPane();
}
// the setup panel folds to its ribbon (click it) and a one-line summary; the choice is kept in this browser, and until
// one is made it opens folded once there are veterans (the plan's answer, not the settings, is what a return visit is for)
let folded = (() => {
  try {
    const f = localStorage.getItem('llp:setup-folded');
    return f === null ? vets.value.length > 0 : f === '1';
  } catch {
    return vets.value.length > 0;
  }
})();
export function paintFold() {
  const bar = must(document.querySelector('.rbbar'), 'the career page has the setup bar'),
    b = $('set-fold'),
    sum = $('set-sum');
  bar.classList.toggle('folded', folded);
  b.setAttribute('aria-expanded', String(!folded));
  sum.hidden = !folded;
  b.title = folded ? 'Show the career setup' : 'Fold the career setup';
  const rank = settings.value.rank === RANKS.ss ? 'SS' : 'B–S+',
    era = ERA === 'gm' ? 'JP, Grand Masters' : 'Global',
    sc = scenarioOf(settings.value.scen).name;
  // each setting as its label, light, and its value, bold, as the panel's rows read (the whole line for the screen reader)
  // (the borrowed parent only when it isn't the default)
  const bp =
    borrowChoice(settings.value) === 'allow' ? [] : [['Borrowed parent', BORROW_CHOICES[borrowChoice(settings.value)]]];
  sum.setAttribute(
    'aria-label',
    [
      era,
      sc,
      `rank ${rank}`,
      `${settings.value.targets.length} target skills`,
      ...bp.map(([k, v]) => `${k}: ${v}`),
    ].join(' · '),
  );
  sum.replaceChildren(
    ...[
      ['Game data', era],
      ['Scenario', sc],
      ['Rank', rank],
      ['Targets', String(settings.value.targets.length)],
      ...bp,
    ].map(([k, v]) => {
      const p = el('span', 'sumkv');
      p.setAttribute('aria-hidden', 'true');
      p.append(el('span', 'sumk', k), el('b', '', v));
      return p;
    }),
  );
}
// the pink ribbon toggles it (all of it, its edges included); folded, the whole bar (its summary included)
export function initFold() {
  const bar = must(document.querySelector('.rbbar'), 'the career page has the setup bar');
  bar.addEventListener('click', (/** @type {TargetedEvent} */ e) => {
    if (!folded && !e.target.closest('#h-set')) return;
    folded = !folded;
    try {
      localStorage.setItem('llp:setup-folded', folded ? '1' : '0');
    } catch {
      // storage blocked: the fold holds for this visit only
    }
    paintFold();
  });
  paintFold();
}
