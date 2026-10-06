// ---------- support cards: the picker (owned or not, by rarity) and deck options ----------
import { $, esc } from './dom.js';
import { CARDS, SK, SCENARIOS } from './data.js';
import { state, goldGiven } from './state.js';
import { cardAv } from './images.js';
import { save } from './search.js';
import { paintSort, sortMenu } from './sortmenu.js';
const RAR = { 1: 'R', 2: 'SR', 3: 'SSR' };
// rar: append the rarity ("Daitaku Helios SSR"); the card list leaves it out, its frame and section heading show it
export const cardTitle = (c, rar = true) => { const m = /^\[(.*)\] (.*)$/.exec(c.name); return m ? { nm: m[2] + (rar ? ' ' + RAR[c.rar] : ''), ti: m[1] } : { nm: c.name, ti: '' }; };
export function cardGives(c, groups) {   // [{t: skill, s: source note, g: gold}]
  const out = [];
  for (const g of c.whites) if (groups.has(g)) out.push({ t: (SK.get(g) || { w: g }).w, s: 'hint', g: false });
  for (const [g, lv, link, p] of c.ev) if (groups.has(g)) { const sk = SK.get(g) || { w: g, gd: g };
    out.push({ t: lv === 2 ? sk.gd : sk.w, s: link ? 'chain ' + link + (p < 1 ? ', ' + Math.round(p * 100) + '%' : '') : (p < 1 ? Math.round(p * 100) + '%' : 'event'), g: lv === 2 }); }
  return out;
}
export function renderCards() {
  const groups = new Set(state.targets.filter(t => !goldGiven(t.group)).map(t => t.group));   // what the deck still has to give
  const f = $('c-filter').value.trim().toLowerCase();
  const box = $('cards'); box.textContent = '';
  const sections = [['SSR', c => c.rar === 3], ['SR', c => c.rar === 2], ['R', c => c.rar === 1 && (c.pal || state.rcards)]];   // R pals stay usable without R cards
  const dir = state.cdir === 'desc' ? -1 : 1, nmOf = (c) => { const t = cardTitle(c); return t.nm + ' ' + t.ti; };
  for (const [label, pick] of sections) {
    const list = CARDS.filter(pick).map(c => ({ c, gives: cardGives(c, groups) })).filter(x => x.c.pal || x.gives.length)
      .sort((a, b) => dir * (state.csort === 'name' ? nmOf(a.c).localeCompare(nmOf(b.c)) : (a.c.rel - b.c.rel) || (a.c.id - b.c.id)));
    const vis = list.filter(x => !f || x.c.name.toLowerCase().includes(f));
    if (!vis.length) continue;
    const sec = document.createElement('div'); sec.className = 'cgroup';
    sec.innerHTML = `<h3>${label}</h3><div class="ctiles"></div>`;
    const chips = sec.querySelector('.ctiles');
    for (const { c, gives } of vis) {
      const t = cardTitle(c, false); const b = document.createElement('button'); b.className = 'ct'; b.type = 'button';
      b.innerHTML = `<span class="cn">${esc(t.nm)}</span>` + (t.ti ? `<span class="cti">${esc(t.ti)}</span>` : '') +
        `<span class="cg">${gives.length ? gives.map(x => `<span class="gv${x.g ? ' g' : ''}">${esc(x.t)} <small>${esc(x.s)}</small></span>`).join('')
          : ''}</span>`;
      b.prepend(cardAv(c, 112));
      // a card the chosen scenario bans (Team Sirius in Unity Cup and Grand Concert): the search leaves it out
      const ban = (c.ban || []).includes(state.scen || 'unity');
      if (ban) { b.classList.add('ban'); const n = document.createElement('span'); n.className = 'cban'; n.textContent = 'Not allowed in ' + (SCENARIOS.find(s => s.v === (state.scen || 'unity')) || {}).name; b.appendChild(n); }
      // a toggle: pressed = owned; its name is its visible text (name, title, what it gives), so voice control matches it
      const paint = () => { const own = state.cardsOff[c.id] !== false; b.setAttribute('aria-pressed', own ? 'true' : 'false'); };
      paint();
      b.onclick = () => { if (state.cardsOff[c.id] === false) delete state.cardsOff[c.id]; else state.cardsOff[c.id] = false; save(); paint(); cardCount(); };
      chips.appendChild(b);
    }
    box.appendChild(sec);
  }
  paintSort('cs', 'c');
  cardCount();
}
function cardCount() {
  // the cards listed (those that give a target, and pals) and how many of them are owned
  const cs = [...$('cards').querySelectorAll('.ct')], own = cs.filter(c => c.getAttribute('aria-pressed') === 'true').length;
  $('c-count').textContent = `${own} of ${cs.length} listed cards owned`;
}
export function initCards() {
  sortMenu('cs', 'c', () => renderCards());
  $('c-filter').oninput = renderCards;
  $('c-all').onclick = () => { state.cardsOff = {}; save(); renderCards(); };
  $('c-none').onclick = () => { for (const c of CARDS) state.cardsOff[c.id] = false; save(); renderCards(); };
  $('c-borrow').checked = state.borrow; $('c-borrow').onchange = (e) => { state.borrow = e.target.checked; save(); };
  $('c-top').value = state.decksTop; $('c-top').onchange = (e) => { state.decksTop = e.target.value; save(); };
  $('c-peruma').checked = state.perUma; $('c-peruma').onchange = (e) => { state.perUma = e.target.checked; save(); };
}
