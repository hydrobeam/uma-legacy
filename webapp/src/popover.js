// ---------- deck card panel: name, role, ownership (the same setting as the card picker) ----------
import { esc } from './dom.js';
import { UMAS, CARD_BY_ID } from './data.js';
import { state, goldGiven } from './state.js';
import { save } from './save.js';
import { umaState, paintRoster } from './roster.js';
import { cardTitle, cardGives, giftsHTML, paintCards } from './cards.js';
// The panel lives in the results column (positioned against it), so it scrolls with the result it belongs to when the
// column scrolls on its own (desktop: the column is sticky and scrolls inside the window).
// Keyboard: opening moves the focus into the panel; Escape, or Tab past either end, closes it and returns the focus to
// the card or name that opened it.
const pop = document.createElement('div');
pop.className = 'cpop';
pop.hidden = true;
pop.setAttribute('role', 'dialog');
pop.tabIndex = -1;
document.getElementById('rcol').appendChild(pop);
let popFor = null; // the card or name button whose panel is open
export const popHasFocus = () => !pop.hidden && pop.contains(document.activeElement);
export function closePop() {
  pop.hidden = true;
  if (popFor) popFor.setAttribute('aria-expanded', 'false');
  popFor = null;
}
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
    chip.classList.toggle('off', !chip.classList.contains('bor') && state.cardsOff[+chip.dataset.cid] === false);
  });
  document.querySelectorAll('.mem[data-uma]').forEach((member) => {
    const u = UMAS.find((x) => x.id === +member.dataset.uma);
    member.classList.toggle(
      'off',
      !!u && (umaState(u) === 'out' || (member.dataset.outfit && state.outfits[+member.dataset.outfit] === false)),
    );
  });
}
const GAP = 8; // px between the panel and the element it belongs to (beside it); below it, 6
// beside: a deck card's panel opens to its right (else its left) at its top, so it never covers the hint odds under the
// deck; without room on either side (phones), below it like the others. Positions are in the host's scrolled box.
function placePop(anchor, beside = false) {
  pop.hidden = false;
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
  closePop();
  popFor = btn;
  btn.setAttribute('aria-expanded', 'true');
  const u = UMAS.find((x) => x.id === +member.dataset.uma),
    outfitId = member.dataset.outfit ? +member.dataset.outfit : null;
  const owned = () => (outfitId ? umaState(u) !== 'out' && state.outfits[outfitId] !== false : umaState(u) !== 'out');
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
  const toggle = pop.querySelector('[data-act="uma"]');
  toggle.onchange = () => {
    if (toggle.checked) {
      if (state.roster[u.id] === 'out') delete state.roster[u.id];
      if (outfitId) delete state.outfits[outfitId];
      else for (const o of u.outfits || []) delete state.outfits[o.id];
    } else if (outfitId) state.outfits[outfitId] = false;
    else state.roster[u.id] = 'out';
    save();
    paintRoster(u.id);
    markOwn();
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
// GameTora's character page: /characters/<outfit id>-<slug>, the slug from her support cards' url_name (build.py),
// else made from her name; with "any outfit" her original outfit (<chara id>01)
function umaUrl(u, outfitId) {
  return `https://gametora.com/umamusume/characters/${outfitId || u.id * 100 + 1}-${u.gt || slugOf(u.name)}`;
}
// GameTora's card page, by the url_name in its own card list (build.py; group cards have names a slug can't guess).
// Fallback for a card missing from that list: id + character name, lowercase, punctuation dropped.
function gametoraUrl(c) {
  const page = c.gt || `${c.id}-${slugOf(c.name.replace(/^\[.*?\]\s*/, ''))}`; // the name without its [title]
  return `https://gametora.com/umamusume/supports/${page}`;
}
function openCardPop(chip) {
  closePop();
  popFor = chip;
  chip.setAttribute('aria-expanded', 'true');
  const c = CARD_BY_ID.get(+chip.dataset.cid),
    title = c ? cardTitle(c) : { nm: chip.dataset.name, ti: '' };
  const borrowed = chip.classList.contains('bor'),
    owned = c && state.cardsOff[c.id] !== false;
  const groups = new Set(state.targets.filter((x) => !goldGiven(x.group)).map((x) => x.group)),
    gifts = c ? cardGives(c, groups) : [];
  pop.setAttribute('aria-label', title.nm);
  pop.innerHTML =
    `<div><b>${esc(title.nm)}</b>${title.ti ? `<div class="ctt">${esc(title.ti)}</div>` : ''}</div>${borrowed ? '<div>Borrowed.</div>' : ''}` +
    (gifts.length ? `<div class="pgives">${giftsHTML(gifts)}</div>` : '') +
    (chip.dataset.end ? `<div>Take the chain-ending choice at link ${chip.dataset.end}.</div>` : '') +
    `<div class="pfoot">${c && !borrowed ? `<label class="sw"><input type="checkbox" role="switch" data-act="own"${owned ? ' checked' : ''}>Owned</label>` : '<span></span>'}` +
    `${c ? `<a class="gtl" href="${gametoraUrl(c)}" target="_blank" rel="noopener">GameTora ↗︎</a>` : ''}</div>`;
  placePop(chip, true);
  /** @type {HTMLInputElement} */
  const toggle = pop.querySelector('[data-act="own"]');
  if (toggle)
    toggle.onchange = () => {
      if (toggle.checked) delete state.cardsOff[c.id];
      else state.cardsOff[c.id] = false;
      save();
      paintCards();
      markOwn();
    };
}
// a click on what is already open closes it; on another card or name, its panel opens instead
const toggleFor = (button, open) => {
  if (popFor === button) closePop();
  else open();
};
export function initPopover() {
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
      toggleFor(face, () =>
        openUmaPop(face, face.closest('.loop').querySelector(`.mem[data-uma="${face.dataset.uma}"]`)),
      );
      return;
    }
    if (!e.target.closest('.cpop')) closePop();
  });
  document.addEventListener('keydown', (e) => {
    if (e.key === 'Escape' && popFor) back();
  });
}
