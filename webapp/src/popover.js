// ---------- deck card panel: name, role, ownership (the same setting as the card picker) ----------
import { signal } from '@preact/signals-core';
import { esc } from './dom.js';
import { gameData } from './data.js';
import { settings, setOwned } from './settings.ts';
import { goldGiven } from './job.js';
import { umaState, paintRoster } from './roster.js';
import { cardTitle, cardGives, giftsHTML, paintCards, chainEndAt } from './cards.js';
import { must } from './must.ts';
// The panel lives in the results column (positioned against it), so it scrolls with the result it belongs to when the
// column scrolls on its own (desktop: the column is sticky and scrolls inside the window).
// Keyboard: opening moves the focus into the panel; Escape, or Tab past either end, closes it and returns the focus to
// the card or name that opened it.
const pop = document.createElement('div');
pop.className = 'cpop';
pop.hidden = true;
pop.setAttribute('role', 'dialog');
pop.tabIndex = -1;
must(document.getElementById('rcol'), 'both planner pages have the results column').appendChild(pop);
/** @type {HTMLElement | null} */
let popFor = null; // the card or name button whose panel is open
// An opener drawn by a component (all of the loop page's, initPopover({drawn: true}); the career page's deck popup,
// initPopover({drawn: predicate})) draws its own aria-expanded from `popOpener`, and its own greying (`off`) from the
// settings, so nothing here writes into it. Openers built with el() get both written here, as before.
let drawnAll = false,
  drawn = /** @type {(button: HTMLElement) => boolean} */ (() => false);
/** @type {import('@preact/signals-core').Signal<HTMLElement | null>} */
const opener = signal(null);
/** the button whose panel is open (null: none) */
export const popOpener = /** @type {import('@preact/signals-core').ReadonlySignal<HTMLElement | null>} */ (opener);
/** @param {HTMLElement} button @param {boolean} open */
function expanded(button, open) {
  if (!drawn(button)) button.setAttribute('aria-expanded', String(open));
}
export const popHasFocus = () => !pop.hidden && pop.contains(document.activeElement);
export function closePop() {
  pop.hidden = true;
  if (popFor) expanded(popFor, false);
  popFor = null;
  opener.value = null;
}
// the panel opens from `button`
/** @param {HTMLElement} button */
function openFor(button) {
  closePop();
  popFor = button;
  expanded(button, true);
  opener.value = button;
}
// after an ownership change in the panel: the el()-drawn result greys what is no longer owned (a drawn one derives it)
const ownChanged = () => {
  if (!drawnAll) markOwn();
};
const focusables = () => [...pop.querySelectorAll('input, a[href], button')];
// close the panel and focus the button that opened it
function back() {
  const opener = popFor;
  closePop();
  if (opener?.isConnected) opener.focus();
}
pop.addEventListener('keydown', (e) => {
  if (e.key !== 'Tab') return;
  const items = focusables(),
    i = items.indexOf(/** @type {HTMLElement} */ (document.activeElement));
  if (!items.length || (e.shiftKey ? i <= 0 : i === items.length - 1)) {
    e.preventDefault();
    back();
  }
});
export function markOwn() {
  document.querySelectorAll('.deck .dc').forEach((chip) => {
    if (drawn(/** @type {HTMLElement} */ (chip))) return;
    // a chip whose card the page data lacks has no id (data-cid empty or absent) and is never greyed
    const cid = chip.dataset.cid;
    chip.classList.toggle('off', !!cid && !chip.classList.contains('bor') && settings.value.cardsOff[+cid] === false);
  });
  document.querySelectorAll('.mem[data-uma]').forEach((member) => {
    if (drawn(/** @type {HTMLElement} */ (member))) return;
    const u = gameData().umas.find((x) => x.id === +must(member.dataset.uma, 'a member block names her'));
    member.classList.toggle(
      'off',
      !!u &&
        (umaState(u) === 'out' ||
          (!!member.dataset.outfit && settings.value.outfits[+member.dataset.outfit] === false)),
    );
  });
}
const GAP = 8; // px between the panel and the element it belongs to (beside it); below it, 6
// beside: a deck card's panel opens to its right (else its left) at its top, so it never covers the hint odds under the
// deck; without room on either side (phones), below it like the others. Positions are in the host's scrolled box.
function placePop(anchor, beside = false) {
  // (inside a popup, the panel goes into it: the page behind a modal popup is out of reach)
  const dlg = anchor.closest('dialog'),
    home = dlg || document.getElementById('rcol');
  if (pop.parentElement !== home) home.appendChild(pop);
  pop.hidden = false;
  // (in a popup: placed against the screen, under the card, so the popup's edges don't clip it)
  pop.classList.toggle('inpopup', !!dlg);
  if (dlg) {
    const rect = anchor.getBoundingClientRect(),
      width = pop.offsetWidth;
    pop.style.left = `${Math.max(8, Math.min(rect.left, innerWidth - width - 8))}px`;
    pop.style.top = `${Math.min(rect.bottom + 6, innerHeight - pop.offsetHeight - 8)}px`;
    (focusables()[0] || pop).focus({ preventScroll: true });
    return;
  }
  const host = pop.offsetParent || document.body,
    hostRect = host.getBoundingClientRect(),
    rect = anchor.getBoundingClientRect(),
    width = pop.offsetWidth;
  const right = rect.right - hostRect.left + GAP,
    left = rect.left - hostRect.left - GAP - width;
  if (beside && right + width <= host.clientWidth) {
    pop.style.left = `${host.scrollLeft + right}px`;
    pop.style.top = `${host.scrollTop + rect.top - hostRect.top}px`;
  } else if (beside && left >= 0) {
    pop.style.left = `${host.scrollLeft + left}px`;
    pop.style.top = `${host.scrollTop + rect.top - hostRect.top}px`;
  } else {
    pop.style.left = `${host.scrollLeft + Math.max(0, Math.min(rect.left - hostRect.left, host.clientWidth - width))}px`;
    pop.style.top = `${host.scrollTop + rect.bottom - hostRect.top + 6}px`;
  }
  (focusables()[0] || pop).focus({ preventScroll: true });
}
// a member's panel: one "Owned" switch. When the loop uses a specific outfit, that outfit is what is owned or not
// (another outfit would be a different trainee); with "any outfit" it is the uma.
function openUmaPop(btn, member = btn.closest('.mem')) {
  // member: her member block (a portrait elsewhere in the result passes it)
  openFor(btn);
  const uma = +must(member.dataset.uma, 'a member block names her'),
    u = must(
      gameData().umas.find((x) => x.id === uma),
      "a loop's member is an uma of the data",
    ),
    outfitId = member.dataset.outfit ? +member.dataset.outfit : null;
  const owned = () =>
    outfitId ? umaState(u) !== 'out' && settings.value.outfits[outfitId] !== false : umaState(u) !== 'out';
  const title = u.name + (outfitId ? ` ${member.dataset.outfitLabel}` : '');
  pop.setAttribute('aria-label', title);
  pop.innerHTML =
    `<div><b>${esc(u.name)}</b><div class="ctt">${esc(member.dataset.outfitLabel)}</div>` +
    (member.dataset.outfitNote ? `<div class="onote">${esc(member.dataset.outfitNote)}</div>` : '') +
    `</div>` +
    `<div class="pfoot"><label class="sw"><input type="checkbox" role="switch" data-act="uma"${owned() ? ' checked' : ''}>Owned</label>` +
    `<a class="gtl" href="${umaUrl(u, outfitId)}" target="_blank" rel="noopener">GameTora ↗︎</a></div>`;
  placePop(btn);
  /** @type {HTMLInputElement} */
  const toggle = must(pop.querySelector('[data-act="uma"]'), 'the panel is made with its switch');
  toggle.onchange = () => {
    const roster = { ...settings.value.roster },
      outfits = { ...settings.value.outfits };
    if (toggle.checked) {
      if (roster[u.id] === 'out') delete roster[u.id];
      if (outfitId) delete outfits[outfitId];
      else for (const o of u.outfits || []) delete outfits[o.id];
    } else if (outfitId) outfits[outfitId] = false;
    else roster[u.id] = 'out';
    setOwned({ roster, outfits });
    paintRoster(u.id);
    ownChanged();
    toggle.checked = owned();
  };
}
// a name as a GameTora URL slug: lowercase, '.' and "'" dropped, every other run of punctuation a hyphen
const slugOf = (name) =>
  name
    .toLowerCase()
    .replace(/[.']/g, '')
    .replace(/[^a-z0-9]+/g, '-')
    .replace(/^-|-$/g, '');
// GameTora's character page: /characters/<outfit id>-<slug>, the slug from her support cards' url_name
// (export_page.py), else made from her name; with "any outfit" her original outfit (<chara id>01)
function umaUrl(u, outfitId) {
  return `https://gametora.com/umamusume/characters/${outfitId || u.id * 100 + 1}-${u.gt || slugOf(u.name)}`;
}
// GameTora's card page, by the url_name in its own card list (export_page.py; group cards have names a slug can't
// guess); for a card missing from that list, id + character name, lowercase, punctuation dropped.
function gametoraUrl(c) {
  const page = c.gt || `${c.id}-${slugOf(c.name.replace(/^\[.*?\]\s*/, ''))}`; // the name without its [title]
  return `https://gametora.com/umamusume/supports/${page}`;
}
// what to prioritise for IT to take the chain-ending choice the search chose, and when it may not. In the run data IT
// took a prioritised skill's choice in every one of 312 readings where only that choice held one, across card events
// (269 of them Sasami Anshinzawa SSR's finale); the rule is carried over to chain-ending choices here. It took it less
// often when the top choice held a prioritised skill too (41 of 56) or the skill's hint was already Lv 5 from another
// source (16 of 28): small samples, so "may not" (docs/RESEARCH.md section 9)
function chainEndHTML(c, link) {
  const { take, others } = chainEndAt(c, link);
  if (!take) return `<div>Take the chain-ending choice at link ${link}.</div>`;
  const targets = new Set(settings.value.targets.map((x) => x.group)),
    rivals = others.filter((x) => targets.has(x.group)).map((x) => x.name),
    otherNames = others.map((x) => x.name).join(' or ');
  let caveat;
  if (rivals.length)
    caveat = `${rivals.join(' and ')} ${rivals.length > 1 ? 'are' : 'is'} also a target: prioritising both makes the end uncertain.`;
  else if (otherNames)
    caveat = `IT may not take it if ${otherNames} is also prioritised, or if ${take}'s hint is already Lv 5.`;
  else caveat = `IT may not take it if ${take}'s hint is already Lv 5.`;
  return (
    `<div>Prioritise ${esc(take)} in IT: it takes the chain-ending choice at link ${link}.</div>` +
    `<div class="onote">${esc(caveat)}</div>`
  );
}
function openCardPop(chip) {
  openFor(chip);
  const c = gameData().cardById.get(+chip.dataset.cid),
    title = c ? cardTitle(c) : { nm: chip.dataset.name, ti: '' };
  const borrowed = chip.classList.contains('bor'),
    owned = c && settings.value.cardsOff[c.id] !== false;
  const groups = new Set(
      settings.value.targets.filter((x) => !goldGiven(settings.value.scen, x.group)).map((x) => x.group),
    ),
    gifts = c ? cardGives(c, groups) : [];
  pop.setAttribute('aria-label', title.nm);
  pop.innerHTML =
    `<div><b>${esc(title.nm)}</b>${title.ti ? `<div class="ctt">${esc(title.ti)}</div>` : ''}</div>${borrowed ? '<div>Borrowed.</div>' : ''}` +
    (gifts.length ? `<div class="pgives">${giftsHTML(gifts)}</div>` : '') +
    (chip.dataset.end ? chainEndHTML(c, +chip.dataset.end) : '') +
    `<div class="pfoot">${c && !borrowed ? `<label class="sw"><input type="checkbox" role="switch" data-act="own"${owned ? ' checked' : ''}>Owned</label>` : '<span></span>'}` +
    `${c ? `<a class="gtl" href="${gametoraUrl(c)}" target="_blank" rel="noopener">GameTora ↗︎</a>` : ''}</div>`;
  placePop(chip, true);
  /** @type {HTMLInputElement | null} a card of the data that is not borrowed has one */
  const toggle = pop.querySelector('[data-act="own"]');
  if (toggle && c)
    toggle.onchange = () => {
      const { [c.id]: _, ...rest } = settings.value.cardsOff;
      setOwned({ cardsOff: toggle.checked ? rest : { ...rest, [c.id]: false } });
      paintCards();
      ownChanged();
    };
}
// a click on what is already open closes it; on another card or name, its panel opens instead
const toggleFor = (button, open) => {
  if (popFor === button) closePop();
  else open();
};
/** @param {{drawn?: boolean | ((button: HTMLElement) => boolean)}} [page] drawn: the page's result is components, all
 *  of it (true) or the openers it names (see `drawn` above) */
export function initPopover(page = {}) {
  const { drawn: d = false } = page;
  drawnAll = d === true;
  drawn = typeof d === 'function' ? d : () => d;
  document.addEventListener('click', (/** @type {TargetedEvent} */ e) => {
    const chip = e.target.closest('.deck .dc'),
      name = e.target.closest('.mem .memb');
    if (chip) {
      toggleFor(chip, () => openCardPop(chip));
      return;
    }
    if (name) {
      toggleFor(name, () => openUmaPop(name));
      return;
    }
    const face = e.target.closest('.loop .facebtn'); // a portrait in a deck title or swap row: the same panel
    if (face) {
      // a "Same loop as #1" card has no member blocks: #1's (the same umas and outfits) stand in
      const mem = `.mem[data-uma="${face.dataset.uma}"]`;
      toggleFor(face, () =>
        openUmaPop(
          face,
          must(face.closest('.loop'), 'a portrait button is in a loop card').querySelector(mem) ||
            document.querySelector(`.loop.first ${mem}`),
        ),
      );
      return;
    }
    if (!e.target.closest('.cpop')) closePop();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && popFor) back();
  });
}
