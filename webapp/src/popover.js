// ---------- deck card panel: name, role, ownership (the same setting as the card picker) ----------
import { esc } from './dom.js';
import { UMAS, CARD_BY_ID } from './data.js';
import { state } from './state.js';
import { save } from './search.js';
import { umaState, renderRoster } from './roster.js';
import { cardTitle, cardGives, renderCards } from './cards.js';
const pop = document.createElement('div'); pop.className = 'cpop'; pop.hidden = true; pop.setAttribute('role', 'dialog'); document.body.appendChild(pop);
let popFor = null;
export function closePop() { pop.hidden = true; if (popFor) popFor.setAttribute('aria-expanded', 'false'); popFor = null; }
export function markOwn() {
  document.querySelectorAll('.deck .dc').forEach(e => e.classList.toggle('off', !e.classList.contains('bor') && state.cardsOff[+e.dataset.cid] === false));
  document.querySelectorAll('.mem[data-uma]').forEach(e => { const u = UMAS.find(x => x.id === +e.dataset.uma);
    e.classList.toggle('off', !!u && (umaState(u) === 'out' || (e.dataset.outfit && state.outfits[+e.dataset.outfit] === false))); });
}
function placePop(el) {
  pop.hidden = false;
  const r = el.getBoundingClientRect(), w = pop.offsetWidth, vw = document.documentElement.clientWidth;
  pop.style.left = (window.scrollX + Math.max(8, Math.min(r.left, vw - w - 8))) + 'px'; pop.style.top = (window.scrollY + r.bottom + 6) + 'px';
}
// a member's panel: one "Owned" switch. When the loop uses a specific outfit, that outfit is what is owned or not
// (another outfit would be a different trainee); with "any outfit" it is the uma.
function openUmaPop(btn) {
  closePop(); popFor = btn; btn.setAttribute('aria-expanded', 'true');
  const d = btn.closest('.mem'), u = UMAS.find(x => x.id === +d.dataset.uma), oid = d.dataset.outfit ? +d.dataset.outfit : null;
  const owned = () => oid ? umaState(u) !== 'out' && state.outfits[oid] !== false : umaState(u) !== 'out';
  const title = u.name + (oid ? ' ' + d.dataset.outfitLabel : '');
  pop.setAttribute('aria-label', title);
  pop.innerHTML = `<div><b>${esc(u.name)}</b><div class="ctt">${esc(d.dataset.outfitLabel)}</div></div>` +
    `<label class="sw"><input type="checkbox" role="switch" data-act="uma"${owned() ? ' checked' : ''}>Owned</label>`;
  placePop(btn);
  const sw = pop.querySelector('[data-act="uma"]');
  sw.onchange = () => {
    if (sw.checked) { if (state.roster[u.id] === 'out') delete state.roster[u.id]; if (oid) delete state.outfits[oid];
      else for (const o of u.outfits || []) delete state.outfits[o.id]; }
    else if (oid) state.outfits[oid] = false; else state.roster[u.id] = 'out';
    save(); renderRoster(); markOwn(); sw.checked = owned(); };
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
  const groups = new Set(state.targets.filter(x => (x.guaranteed || 0) !== 2).map(x => x.group)), gives = c ? cardGives(c, groups) : [];
  pop.setAttribute('aria-label', t.nm);
  pop.innerHTML = `<div><b>${esc(t.nm)}</b>${t.ti ? `<div class="ctt">${esc(t.ti)}</div>` : ''}</div>` + (bor ? '<div>Borrowed.</div>' : '') +
    (gives.length ? `<div class="pgives">${gives.map(x => `<span class="gv${x.g ? ' g' : ''}">${esc(x.t)} <small>${esc(x.s)}</small></span>`).join('')}</div>` : '') +
    (el.dataset.end ? `<div>Take the chain-ending choice at link ${el.dataset.end}.</div>` : '') +
    `<div class="pfoot">${c && !bor ? `<label class="sw"><input type="checkbox" role="switch" data-act="own"${own ? ' checked' : ''}>Owned</label>` : '<span></span>'}` +
    (c ? `<a class="gtl" href="${gametoraUrl(c)}" target="_blank" rel="noopener">GameTora ↗︎</a>` : '') + '</div>';
  placePop(el);
  const sw = pop.querySelector('[data-act="own"]');
  if (sw) sw.onchange = () => { if (sw.checked) delete state.cardsOff[c.id]; else state.cardsOff[c.id] = false; save(); renderCards(); markOwn(); };
}
export function initPopover() {
  document.addEventListener('click', (e) => { const d = e.target.closest('.deck .dc'), mb = e.target.closest('.mem .memb');
    if (d) { if (popFor === d) closePop(); else openPop(d); return; }
    if (mb) { if (popFor === mb) closePop(); else openUmaPop(mb); return; }
    if (!e.target.closest('.cpop')) closePop(); });
  document.addEventListener('keydown', (e) => { if (e.key === 'Escape' && popFor) { const d = popFor; closePop(); d.focus(); } });
}
