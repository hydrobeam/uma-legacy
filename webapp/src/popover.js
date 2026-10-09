// ---------- deck card panel: name, role, ownership (the same setting as the card picker) ----------
import { esc } from './dom.js';
import { UMAS, CARD_BY_ID } from './data.js';
import { state, goldGiven } from './state.js';
import { save } from './save.js';
import { umaState, paintRoster } from './roster.js';
import { cardTitle, cardGives, paintCards } from './cards.js';
// The panel lives in the results column (positioned against it), so it scrolls with the result it belongs to when the
// column scrolls on its own (desktop: the column is sticky and scrolls inside the window).
// Keyboard: opening moves the focus into the panel; Escape, or Tab past either end, closes it and returns the focus to
// the card or name that opened it.
const pop = document.createElement('div'); pop.className = 'cpop'; pop.hidden = true; pop.setAttribute('role', 'dialog'); pop.tabIndex = -1; document.getElementById('rcol').appendChild(pop);
let popFor = null;
export const popHasFocus = () => !pop.hidden && pop.contains(document.activeElement);
export function closePop() { pop.hidden = true; if (popFor) popFor.setAttribute('aria-expanded', 'false'); popFor = null; }
const focusables = () => [...pop.querySelectorAll('input, a[href], button')];
function back() { const t = popFor; closePop(); if (t && t.isConnected) t.focus(); }
pop.addEventListener('keydown', (e) => {
  if (e.key !== 'Tab') return;
  const f = focusables(), i = f.indexOf(document.activeElement);
  if (!f.length || (e.shiftKey ? i <= 0 : i === f.length - 1)) { e.preventDefault(); back(); }
});
export function markOwn() {
  document.querySelectorAll('.deck .dc').forEach(e => e.classList.toggle('off', !e.classList.contains('bor') && state.cardsOff[+e.dataset.cid] === false));
  document.querySelectorAll('.mem[data-uma]').forEach(e => { const u = UMAS.find(x => x.id === +e.dataset.uma);
    e.classList.toggle('off', !!u && (umaState(u) === 'out' || (e.dataset.outfit && state.outfits[+e.dataset.outfit] === false))); });
}
// beside: a deck card's panel opens to its right (else its left) at its top, so it never covers the hint odds under the
// deck; without room on either side (phones), below it like the others
function placePop(el, beside = false) {
  pop.hidden = false;
  const host = pop.offsetParent || document.body, h = host.getBoundingClientRect(), r = el.getBoundingClientRect(), w = pop.offsetWidth;
  const right = r.right - h.left + 8, left = r.left - h.left - 8 - w;
  if (beside && right + w <= host.clientWidth) { pop.style.left = (host.scrollLeft + right) + 'px'; pop.style.top = (host.scrollTop + r.top - h.top) + 'px'; }
  else if (beside && left >= 0) { pop.style.left = (host.scrollLeft + left) + 'px'; pop.style.top = (host.scrollTop + r.top - h.top) + 'px'; }
  else {
    pop.style.left = (host.scrollLeft + Math.max(0, Math.min(r.left - h.left, host.clientWidth - w))) + 'px';
    pop.style.top = (host.scrollTop + r.bottom - h.top + 6) + 'px';
  }
  (focusables()[0] || pop).focus({ preventScroll: true });
}
// a member's panel: one "Owned" switch. When the loop uses a specific outfit, that outfit is what is owned or not
// (another outfit would be a different trainee); with "any outfit" it is the uma.
function openUmaPop(btn, d = btn.closest('.mem')) {   // d: her member block (a portrait elsewhere in the result passes it)
  closePop(); popFor = btn; btn.setAttribute('aria-expanded', 'true');
  const u = UMAS.find(x => x.id === +d.dataset.uma), oid = d.dataset.outfit ? +d.dataset.outfit : null;
  const owned = () => oid ? umaState(u) !== 'out' && state.outfits[oid] !== false : umaState(u) !== 'out';
  const title = u.name + (oid ? ' ' + d.dataset.outfitLabel : '');
  pop.setAttribute('aria-label', title);
  pop.innerHTML = `<div><b>${esc(u.name)}</b><div class="ctt">${esc(d.dataset.outfitLabel)}</div>` +
    (d.dataset.outfitNote ? `<div class="onote">${esc(d.dataset.outfitNote)}</div>` : '') + `</div>` +
    `<div class="pfoot"><label class="sw"><input type="checkbox" role="switch" data-act="uma"${owned() ? ' checked' : ''}>Owned</label>` +
    `<a class="gtl" href="${umaUrl(u, oid)}" target="_blank" rel="noopener">GameTora ↗︎</a></div>`;
  placePop(btn);
  const sw = pop.querySelector('[data-act="uma"]');
  sw.onchange = () => {
    if (sw.checked) { if (state.roster[u.id] === 'out') delete state.roster[u.id]; if (oid) delete state.outfits[oid];
      else for (const o of u.outfits || []) delete state.outfits[o.id]; }
    // not owned can't also be must include: the uma as a whole drops her must-include marks (an outfit's own mark is
    // replaced by its not-owned one; her other marked outfits still stand)
    else if (oid) state.outfits[oid] = false;
    else { state.roster[u.id] = 'out'; for (const o of u.outfits || []) if (state.outfits[o.id] === 'req') delete state.outfits[o.id]; }
    save(); paintRoster(u.id); markOwn(); sw.checked = owned(); };
}
// GameTora's character page: /characters/<outfit id>-<slug>, the slug from her support cards' url_name (build.py),
// else made from her name; with "any outfit" her original outfit (<chara id>01)
function umaUrl(u, oid) {
  const slug = u.gt || u.name.toLowerCase().replace(/[.']/g, '').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  return `https://gametora.com/umamusume/characters/${oid || u.id * 100 + 1}-${slug}`;
}
// GameTora's card page, by the url_name in its own card list (build.py; group cards have names a slug can't guess).
// Fallback for a card missing from that list: id + character name, lowercase, punctuation dropped.
function gametoraUrl(c) {
  const nm = c.gt || c.id + '-' + c.name.replace(/^\[.*?\]\s*/, '').toLowerCase().replace(/[.']/g, '').replace(/[^a-z0-9]+/g, '-').replace(/^-|-$/g, '');
  return 'https://gametora.com/umamusume/supports/' + nm;
}
function openPop(el) {
  closePop(); popFor = el; el.setAttribute('aria-expanded', 'true');
  const c = CARD_BY_ID.get(+el.dataset.cid), t = c ? cardTitle(c) : { nm: el.dataset.name, ti: '' };
  const bor = el.classList.contains('bor'), own = c && state.cardsOff[c.id] !== false;
  const groups = new Set(state.targets.filter(x => !goldGiven(x.group)).map(x => x.group)), gives = c ? cardGives(c, groups) : [];
  pop.setAttribute('aria-label', t.nm);
  pop.innerHTML = `<div><b>${esc(t.nm)}</b>${t.ti ? `<div class="ctt">${esc(t.ti)}</div>` : ''}</div>` + (bor ? '<div>Borrowed.</div>' : '') +
    (gives.length ? `<div class="pgives">${gives.map(x => `<span class="gv${x.g ? ' g' : ''}" title="${esc(x.tip)}">${esc(x.t)} <small>${esc(x.s)}</small></span>`).join('')}</div>` : '') +
    (el.dataset.end ? `<div>Take the chain-ending choice at link ${el.dataset.end}.</div>` : '') +
    `<div class="pfoot">${c && !bor ? `<label class="sw"><input type="checkbox" role="switch" data-act="own"${own ? ' checked' : ''}>Owned</label>` : '<span></span>'}` +
    (c ? `<a class="gtl" href="${gametoraUrl(c)}" target="_blank" rel="noopener">GameTora ↗︎</a>` : '') + '</div>';
  placePop(el, true);
  const sw = pop.querySelector('[data-act="own"]');
  if (sw) sw.onchange = () => { if (sw.checked) delete state.cardsOff[c.id]; else state.cardsOff[c.id] = false; save(); paintCards(); markOwn(); };
}
export function initPopover() {
  document.addEventListener('click', (e) => { const d = e.target.closest('.deck .dc'), mb = e.target.closest('.mem .memb');
    if (d) { if (popFor === d) closePop(); else openPop(d); return; }
    if (mb) { if (popFor === mb) closePop(); else openUmaPop(mb); return; }
    const fb = e.target.closest('.loop .facebtn');   // a portrait in a deck title or swap row: the same panel
    if (fb) { if (popFor === fb) closePop(); else openUmaPop(fb, fb.closest('.loop').querySelector(`.mem[data-uma="${fb.dataset.uma}"]`)); return; }
    if (!e.target.closest('.cpop')) closePop(); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && popFor) back(); });
}
