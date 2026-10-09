// ---------- support cards: the picker (owned or not, by rarity) and deck options ----------
import { $, esc } from './dom.js';
import { CARDS, SK, SCENARIOS } from './data.js';
import { state, goldGiven } from './state.js';
import { cardAv } from './images.js';
import { save } from './save.js';
import { paintSort, sortMenu } from './sortmenu.js';
const RAR = { 1: 'R', 2: 'SR', 3: 'SSR' };
// rar: append the rarity ("Daitaku Helios SSR"); the card list leaves it out, its frame and section heading show it
export const cardTitle = (c, rar = true) => { const m = /^\[(.*)\] (.*)$/.exec(c.name); return m ? { nm: m[2] + (rar ? ' ' + RAR[c.rar] : ''), ti: m[1] } : { nm: c.name, ti: '' }; };
// how a card gives a target, as a short note and a one-line tooltip: from training with it, from an event, or from its
// n-th story event (they come in order, so a later one needs the earlier ones first); "50%": the event gives it on
// some of its outcomes (one of two: 50%; Anshinzawa SSR's finale: 75%, measured)
const ORD = ['', '1st', '2nd', '3rd', '4th', '5th'];
const share = (p) => p >= 1 ? '' : `${Math.round(p * 100)}%`;
export function cardGives(c, groups) {   // [{t: skill, s: note, tip: tooltip, g: gold}]
  const out = [];
  for (const g of c.whites) if (groups.has(g)) out.push({ t: (SK.get(g) || { w: g }).w, s: 'hint', tip: 'Hinted when training with this card', g: false });
  for (const [g, lv, link, p] of c.ev) if (groups.has(g)) { const sk = SK.get(g) || { w: g, gd: g }, sh = share(p);
    const s = (link ? `${ORD[link] || link + 'th'} event` : 'event') + (sh ? ' · ' + sh : '');
    const tip = (link ? `Its ${ORD[link] || link + 'th'} story event (they come in order)` : 'One of its events') + (sh ? `, ${sh} of the time` : '');
    out.push({ t: lv === 2 ? sk.gd : sk.w, s, tip, g: lv === 2 }); }
  return out;
}
export function renderCards() {
  const groups = new Set(state.targets.filter(t => !goldGiven(t.group)).map(t => t.group));   // what the deck still has to give
  const box = $('cards'); box.textContent = '';
  const sections = [['SSR', c => c.rar === 3], ['SR', c => c.rar === 2], ['R', c => c.rar === 1 && (c.pal || state.rcards)]];   // R pals stay usable without R cards
  const dir = state.cdir === 'desc' ? -1 : 1, nmOf = (c) => { const t = cardTitle(c); return t.nm + ' ' + t.ti; };
  for (const [label, pick] of sections) {
    const list = CARDS.filter(pick).map(c => ({ c, gives: cardGives(c, groups) })).filter(x => x.c.pal || x.gives.length)
      .sort((a, b) => dir * (state.csort === 'name' ? nmOf(a.c).localeCompare(nmOf(b.c)) : (a.c.rel - b.c.rel) || (a.c.id - b.c.id)));
    if (!list.length) continue;
    const sec = document.createElement('div'); sec.className = 'cgroup';
    sec.innerHTML = `<h3>${label}</h3><div class="ctiles"></div>`;
    const chips = sec.querySelector('.ctiles');
    for (const { c, gives } of list) {
      const t = cardTitle(c, false); const b = document.createElement('button'); b.className = 'ct'; b.type = 'button'; b._nm = c.name.toLowerCase();
      b.innerHTML = `<span class="cn">${esc(t.nm)}</span>` + (t.ti ? `<span class="cti">${esc(t.ti)}</span>` : '') +
        `<span class="cg">${gives.length ? gives.map(x => `<span class="gv${x.g ? ' g' : ''}" title="${esc(x.tip)}">${esc(x.t)} <small>${esc(x.s)}</small></span>`).join('')
          : ''}</span>`;
      b.prepend(cardAv(c, 112));
      // a card the chosen scenario bans (Team Sirius in Unity Cup and Grand Concert): the search leaves it out
      const ban = (c.ban || []).includes(state.scen || 'unity');
      if (ban) { b.classList.add('ban'); const n = document.createElement('span'); n.className = 'cban'; n.textContent = 'Not allowed in ' + (SCENARIOS.find(s => s.v === (state.scen || 'unity')) || {}).name; b.appendChild(n); }
      // a toggle: pressed = owned; its name is its visible text (name, title, what it gives), so voice control matches it
      const paint = () => { const own = state.cardsOff[c.id] !== false; b.setAttribute('aria-pressed', own ? 'true' : 'false'); };
      paint(); b._paint = paint;
      b.onclick = () => { if (state.cardsOff[c.id] === false) delete state.cardsOff[c.id]; else state.cardsOff[c.id] = false; save(); paint(); cardCount(); };
      chips.appendChild(b);
    }
    box.appendChild(sec);
  }
  paintSort('cs', 'c');
  filterCards();
}
// after an ownership change: every card's pressed state and the count (the list doesn't depend on ownership)
export function paintCards() {
  for (const b of $('cards').querySelectorAll('.ct')) b._paint();
  cardCount();
}
// the filter hides cards (and a rarity section left empty) instead of rebuilding the list on every keystroke
function filterCards() {
  const f = $('c-filter').value.trim().toLowerCase();
  for (const sec of $('cards').querySelectorAll('.cgroup')) {
    let any = false;
    for (const b of sec.querySelectorAll('.ct')) { b.hidden = !!f && !b._nm.includes(f); any ||= !b.hidden; }
    sec.hidden = !any;
  }
  cardCount();
}
function cardCount() {
  // the cards listed (those that give a target, and pals, matching the filter) and how many of them are owned (beside All / None owned)
  const cs = [...$('cards').querySelectorAll('.ct:not([hidden])')], own = cs.filter(c => c.getAttribute('aria-pressed') === 'true').length;
  $('c-count').textContent = `${own} of ${cs.length} owned`;
}
export function initCards() {
  sortMenu('cs', 'c', () => renderCards());
  $('c-filter').oninput = filterCards;
  $('c-all').onclick = () => { state.cardsOff = {}; save(); paintCards(); };
  $('c-none').onclick = () => { for (const c of CARDS) state.cardsOff[c.id] = false; save(); paintCards(); };
  if (!$('c-top')) return;   // the career page has neither: one career, one deck
  // Results: 1 or 3, the game's pill switch (a radio group: the arrows move and select)
  const top = $('c-top'), pickTop = (v) => { if (v === state.decksTop) return; state.decksTop = v; paintTop(); save(); };
  top.querySelectorAll('button').forEach(b => { b.onclick = () => pickTop(b.dataset.v); });
  top.addEventListener('keydown', (e) => { if (!['ArrowLeft', 'ArrowRight', 'ArrowUp', 'ArrowDown'].includes(e.key)) return; e.preventDefault();
    const bs = [...top.querySelectorAll('button')], n = bs.find(b => b.dataset.v !== state.decksTop); pickTop(n.dataset.v); n.focus(); });
  paintTop();
  $('c-peruma').checked = state.perUma; $('c-peruma').onchange = (e) => { state.perUma = e.target.checked; save(); };
}
export function paintTop() {
  const top = document.getElementById('c-top'); if (!top) return;
  top.querySelectorAll('button').forEach(b => { const on = b.dataset.v === state.decksTop; b.setAttribute('aria-checked', String(on)); b.setAttribute('aria-pressed', String(on)); b.tabIndex = on ? 0 : -1; });
}
