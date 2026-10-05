// ---------- results: one card per loop (odds, members, decks), the schedule and affinity buttons ----------
import { $, esc } from './dom.js';
import { CARDS, CARD_BY_ID, SK } from './data.js';
import { ui } from './state.js';
import { umaAv, cardAv, skillIcon } from './images.js';
import { cardTitle } from './cards.js';
import { closePop, markOwn, popHasFocus } from './popover.js';
import { openSched } from './agenda.js';
import { openWhy } from './affinity.js';
export const fmtIn = (p) => p > 0 ? '1 in ' + (1 / p >= 100 ? Math.round(1 / p).toLocaleString() : (1 / p).toFixed(1)) : 'never';
const runs90 = (p) => p > 0 ? Math.ceil(Math.log(0.1) / Math.log(1 - p)).toLocaleString() : '—';
const loopKey = (l) => l.members.map((m, k) => m.id + ':' + l.outfits[k].outfits.map(o => o.id).join('/')).join(' ');
// one result: its odds, the four umas, the deck(s), the schedule and affinity buttons. ctx: {rank (1-based, 0 when
// only one result is shown), top: the #1 result (to say what differs from it), proven}
function loopCard(l, meta, ctx) {
  const n = meta.targets.length;
  const card = document.createElement('article'); card.className = 'loop' + (ctx.rank <= 1 ? ' first' : '');
  const ref = ctx.rank > 1 ? ctx.top : null, sameLoop = ref && loopKey(ref.loop) === loopKey(l);
  card.innerHTML = `<div class="lhead">${ctx.rank ? `<span class="rank">#${ctx.rank}</span>` : ''}<span class="odds"><b>${fmtIn(l.p_all)}</b> runs to land all ${n}</span>
    <span class="sub">${n > 1 ? `at least ${n - 1} of ${n}: <span class="num">${fmtIn(l.p_minus1)}</span> · ` : ''}90% sure within <span class="num">${runs90(l.p_all)}</span> runs</span>${ctx.proven ? '<span class="badge" title="The search ranks decks and loops by multiplying each target&#39;s odds; nothing can beat this one on that ranking, given the race schedules found for the loops it solved. The odds shown are exact for the model.">Proven best</span>' : ''}</div>`;
  if (meta.deck) {
    const D = meta.decks, M = l.members;
    const key = (d) => d.cards.join('|');
    const same = !D || D.every(d => key(d) === key(meta.deck));
    const refCards = ref ? new Set(ref.deck.cards) : null;   // #2 and #3: the cards of their main deck that #1's doesn't have are marked
    const lastBor = (d, idx) => [...idx].sort((a, b) => /\(borrowed\)$/.test(d.cards[a]) - /\(borrowed\)$/.test(d.cards[b]));   // borrowed card in the sixth slot, as in the game
    const chip = (d, ci, mark) => { const nm = d.cards[ci]; const pals = new Set(d.pal_cards || []);
      const bor = /\(borrowed\)$/.test(nm); const base = nm.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, '').replace(/ \(link \d: [^()]*\)$/, '');
      const endAt = (d.end_chain_at || [])[ci] || 0, pick = (d.choices || [])[ci] || '';   // pick: the choice to prioritise, e.g. "link 1: Uma Stan"
      const tagPick = pick ? ' · prioritise ' + pick : '', isNew = mark && refCards && !refCards.has(nm);
      const c = (d.card_ids && CARD_BY_ID.get(d.card_ids[ci])) || CARDS.find(x => x.name === base); const t = c ? cardTitle(c) : { nm: base, ti: '' };
      const tag = bor ? 'borrowed' : pals.has(base) ? 'pal' : '', full = t.nm + (t.ti ? ' · ' + t.ti : '') + (tag ? ` (${tag})` : '') + tagPick + (isNew ? ' · not in #1’s deck' : '');
      return `<button type="button" class="dc${bor ? ' bor' : pals.has(base) ? ' pal' : ''}${isNew ? ' new' : ''}" data-cid="${c ? c.id : ''}" data-name="${esc(t.nm)}"${endAt ? ` data-end="${endAt}"` : ''}` +
        ` title="${esc(full)}" aria-label="${esc(full)}" aria-haspopup="dialog" aria-expanded="false">` +
        `<span class="dn">${esc(full)}</span>${isNew ? '<span class="dnew" aria-hidden="true">new</span>' : ''}${bor ? '<i>borrowed</i>' : ''}${pick ? `<i>prioritise ${esc(pick)}</i>` : ''}${endAt ? `<i>chain end: link ${endAt}</i>` : ''}</button>`; };
    // per target the deck can hint: the gold's odds, then the white's (a gold hint counts for the white too, so the
    // white's number is the chance of at least the white): the two numbers the landing odds depend on; each with its icon
    const icon = (g, gold) => { const i = skillIcon(g, gold); return i ? i.outerHTML : ''; };
    const oddsOf = (d) => { const odds = d.odds.filter(o => o.white > 0.005 || o.gold > 0.005).map(o => { const s = SK.get(o.group) || { w: o.group, gd: null };
        const h = 1 - (1 - o.white) * (1 - o.gold), parts = [];
        if (o.gold > 0.005) parts.push(`<span class="dsk">${icon(o.group, true)}${esc(s.gd || s.w)} <b>${Math.round(o.gold * 100)}%</b></span>`);
        if (h - o.gold > 0.005) parts.push(`<span class="dsk" title="${o.gold > 0.005 ? esc(s.w) + ' or its gold' : ''}">${icon(o.group, false)}${esc(s.w)} <b>${Math.round(h * 100)}%</b></span>`);
        return parts.join(''); }).join('');
      return odds ? `<div class="dodds"><span class="dlab">Hint odds from this deck:</span>${odds}</div>` : '<div class="dodds">This deck gives none of the targets.</div>'; };
    const block = (d, title) => { const dk = document.createElement('div'); dk.className = 'deck';
      dk.innerHTML = `<div class="row between"><b>${title}</b>${ref && [...d.cards].some(c => !refCards.has(c)) ? '<span class="hint"><span class="dnew">new</span> not in #1’s deck</span>' : ''}</div><div class="dcards">${lastBor(d, d.cards.map((_, ci) => ci)).map(ci => chip(d, ci, true)).join('')}</div>` + oddsOf(d);
      return dk; };
    if (same) card.appendChild(block(meta.deck, D ? 'Deck for all four' : 'Deck'));
    else {
      // the deck most members share, then each other member's swaps against it, one row each: "X swaps A → B"
      const users = M.filter((m, k) => key(D[k]) === key(meta.deck)).map(m => m.name);
      card.appendChild(block(meta.deck, 'Deck for ' + users.join(', ')));
      const seen = new Set();
      M.forEach((m, k) => { if (key(D[k]) === key(meta.deck) || seen.has(key(D[k]))) return; seen.add(key(D[k]));
        const d = D[k]; const who = M.filter((_, j) => key(D[j]) === key(d)).map(x => x.name);
        const whoT = who.length === 1 ? `${esc(who[0])}’s deck` : `The deck for ${who.map(esc).join(', ')}`; const out = meta.deck.cards.map((_, ci) => ci).filter(ci => !d.cards.includes(meta.deck.cards[ci]));
        const inn = d.cards.map((_, ci) => ci).filter(ci => !meta.deck.cards.includes(d.cards[ci]));
        const dk = document.createElement('div'); dk.className = 'deck swap';
        dk.innerHTML = `<div class="swrow"><b>${whoT}</b><span class="swt">swaps</span><span class="swc">${lastBor(meta.deck, out).map(ci => chip(meta.deck, ci)).join('')}</span>` +
          `<span class="swa" aria-hidden="true">→</span><span class="vh">for</span><span class="swc">${lastBor(d, inn).map(ci => chip(d, ci)).join('')}</span></div>` + oddsOf(d);
        card.appendChild(dk); });
    }
    card.querySelectorAll('.deck .dc').forEach(el => { const c = CARD_BY_ID.get(+el.dataset.cid); el.prepend(cardAv(c || null, 96)); });
  }
  let mem;
  if (sameLoop) { mem = document.createElement('p'); mem.className = 'same'; mem.textContent = 'Same loop as #1 (the same umas and outfits); only the deck differs.'; }
  else {
    mem = document.createElement('div'); mem.className = 'members';
    l.members.forEach((m, k) => {
      const o = l.outfits[k]; const d = document.createElement('div'); d.className = 'mem' + (o.covers.length ? ' covers' : '');
      const outfit = o.outfits.length ? o.outfits.map(x => x.label).join(' or ') : 'any outfit';
      d.innerHTML = `<button type="button" class="head memb" aria-haspopup="dialog" aria-expanded="false"><span class="nm"></span></button><span class="of"></span><span class="sk"></span>` +
        `<span class="own" title="The odds that every target lands in a run with her as the trainee">as trainee: <span class="num">${fmtIn(l.per_trainee[k])}</span></span>`;
      d.dataset.uma = m.id; d.dataset.outfit = o.outfits.length === 1 ? o.outfits[0].id : ''; d.dataset.outfitLabel = outfit;
      d.querySelector('.nm').textContent = m.name; d.querySelector('.of').textContent = outfit;
      d.querySelector('.head').prepend(umaAv(m.id, o.outfits.length === 1 ? o.outfits[0].id : null, 38));
      // targets on her own outfit's skill list (she has them when she is the trainee): the reason for the blue top border
      const sk = d.querySelector('.sk');
      if (o.covers.length) { const cap = document.createElement('span'); cap.className = 'skl'; cap.textContent = 'On her outfit:'; sk.appendChild(cap);
 }
      for (const c of o.covers) { const s = SK.get(c.group) || { w: c.group, gd: null }; const sp = document.createElement('span');
        sp.className = 'spark' + (c.level === 2 ? ' g' : ''); sp.textContent = c.level === 2 ? (s.gd || s.w) : s.w; sp.title = (c.level === 2 ? 'Gold skill' : 'Skill') + ' on her own outfit: she has it when she is the trainee'; sk.appendChild(sp); }
      mem.appendChild(d);
    });
  }
  const firstDeck = card.querySelector('.deck');   // the loop (members, odds) reads before the decks that serve it
  card.insertBefore(mem, firstDeck);
  const acts = document.createElement('div'); acts.className = 'row lacts';
  const sb = document.createElement('button'); sb.type = 'button'; sb.className = 'btn sbtn'; sb.textContent = 'Race schedule'; sb.onclick = () => openSched(l); acts.appendChild(sb);
  if (l.detail) { const ab = document.createElement('button'); ab.type = 'button'; ab.className = 'btn sbtn'; ab.textContent = 'Affinity'; ab.onclick = () => openWhy(l); acts.appendChild(ab); }
  card.appendChild(acts);
  return card;
}
export function renderDeck(res, meta) {
  // focus inside the old result (an open panel's Owned switch, say) would fall back to the page top: keep it in the results
  const box = $('results'), refocus = popHasFocus() || box.contains(document.activeElement);
  closePop(); box.textContent = '';
  ui.headline = fmtIn(res.top[0].loop.p_all);   // only results with loops reach this (search.js keeps errors off the page)
  const many = res.top.length > 1;
  res.top.forEach((t, i) => box.appendChild(loopCard(t.loop, { ...meta, deck: t.deck, decks: t.decks || null }, { rank: many ? i + 1 : 0, top: res.top[0], proven: i === 0 && res.proven })));
  if (refocus) $('rcol').focus({ preventScroll: true });
  markOwn();   // grey out anything marked not owned since the search (needs the cards on the page)
}
