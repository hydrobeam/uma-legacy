// ---------- results: one card per loop (odds, members, decks), the schedule and affinity buttons ----------
import { $, esc } from './dom.js';
import { CARDS, CARD_BY_ID, SK } from './data.js';
import { ui } from './state.js';
import { umaAv, cardAv, skillIcon } from './images.js';
import { cardTitle } from './cards.js';
import { closePop, markOwn, popHasFocus } from './popover.js';
import { openSched } from './agenda.js';
import { openWhy } from './affinity.js';
import { openOdds } from './odds.js';
export const fmtIn = (p) => p > 0 ? '1 in ' + (1 / p >= 100 ? Math.round(1 / p).toLocaleString() : (1 / p).toFixed(1)) : 'never';
const loopKey = (l) => l.members.map((m, k) => m.id + ':' + l.outfits[k].outfits.map(o => o.id).join('/')).join(' ');
// one result: its odds, the three or four umas, the deck(s), the schedule and affinity buttons. ctx: {rank (1-based, 0 when
// only one result is shown), top: the #1 result (to say what differs from it)}
function loopCard(l, meta, ctx) {
  const n = meta.targets.length, forced = l.members.filter(m => (meta.forced || []).includes(m.id)).map(m => m.id);   // umas the job required
  const card = document.createElement('article'); card.className = 'loop' + (ctx.rank <= 1 ? ' first' : '');
  const ref = ctx.rank > 1 ? ctx.top : null, sameLoop = ref && loopKey(ref.loop) === loopKey(l);
  card.innerHTML = `<div class="lhead">${ctx.rank ? `<span class="rank">#${ctx.rank}</span>` : ''}<span class="odds"><b>${fmtIn(l.p_all)}</b> runs to land all ${n}</span>${forced.length ? '<span class="lkey" title="Marked must include under Umas: the search had to put her in the loop"><i aria-hidden="true"></i>Must include</span>' : ''}</div>`;
  if (meta.deck) {
    const D = meta.decks, M = l.members;
    const key = (d) => d.cards.join('|');
    const same = !D || D.every(d => key(d) === key(meta.deck));
    const lastBor = (d, idx) => [...idx].sort((a, b) => /\(borrowed\)$/.test(d.cards[a]) - /\(borrowed\)$/.test(d.cards[b]));   // borrowed card in the sixth slot, as in the game
    const chip = (d, ci) => { const nm = d.cards[ci]; const pals = new Set(d.pal_cards || []);
      const bor = /\(borrowed\)$/.test(nm); const base = nm.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, '');
      const c = CARD_BY_ID.get(d.card_ids[ci]) || CARDS.find(x => x.name === base);   // plain pals have a name but no entry in card_ids
      // pick: the skill to prioritise in IT: the search's choice at a chain link ("link 1: Uma Stan"), or what a pal's
      // credited event choice needs (derived from the event data: c.pri)
      const endAt = (d.end_chain_at || [])[ci] || 0, pick = (d.choices || [])[ci] || (c && c.pri ? c.pri.join(', ') : '');
      const tagPick = pick ? ' · prioritise ' + pick : '';
      const t = c ? cardTitle(c) : { nm: base, ti: '' };
      const tag = bor ? 'borrowed' : pals.has(base) ? 'pal' : '', full = t.nm + (t.ti ? ' · ' + t.ti : '') + (tag ? ` (${tag})` : '') + tagPick;
      return `<button type="button" class="dc${bor ? ' bor' : pals.has(base) ? ' pal' : ''}" data-cid="${c ? c.id : ''}" data-name="${esc(t.nm)}"${endAt ? ` data-end="${endAt}"` : ''}` +
        ` title="${esc(full)}" aria-label="${esc(full)}" aria-haspopup="dialog" aria-expanded="false">` +
        `<span class="dn">${esc(full)}</span>${pick ? `<i>prioritise ${esc(pick)}</i>` : ''}${endAt ? `<i>chain end: link ${endAt}</i>` : ''}</button>`; };
    // per target the deck can hint: the gold's odds, then the white's (a gold hint counts for the white too, so the
    // white's number is the chance of at least the white): the two numbers the landing odds depend on; each with its icon
    const icon = (g, gold) => { const i = skillIcon(g, gold); return i ? i.outerHTML : ''; };
    // shown on demand (the (i) button by the deck's title): one tile per hint: icon, name and percentage over a bar
    // (the icons alone don't tell skills apart: the game draws one icon per effect type)
    const oddsOf = (d) => { const tile = (g, gold, name, p, tip) => { const pc = Math.round(p * 100);
        return `<span class="dsk${gold ? ' g' : ''}" title="${esc(tip || name)}: ${pc}%">${icon(g, gold)}<span class="dnm">${esc(name)}</span><span class="dbar" aria-hidden="true"><span style="width:${pc}%"></span></span><b>${pc}%</b></span>`; };
      const odds = d.odds.filter(o => o.white > 0.005 || o.gold > 0.005).map(o => { const s = SK.get(o.group) || { w: o.group, gd: null };
        const h = 1 - (1 - o.white) * (1 - o.gold), parts = [];
        if (o.gold > 0.005) parts.push(tile(o.group, true, s.gd || s.w, o.gold));
        if (h - o.gold > 0.005) parts.push(tile(o.group, false, s.w, h, o.gold > 0.005 ? s.w + ' or its gold' : ''));
        return parts.join(''); }).join('');
      return `<div class="dodds" hidden><span class="dlab">Hint odds from this deck</span>${odds || '<span>This deck gives none of the targets.</span>'}</div>`; };
    // what a swap changes: the hint odds that move against the shared deck (the gold's, then at least the white's),
    // largest change first; two shown, a click on the list shows them all (and again folds it)
    const deltas = (d) => { const base = new Map(meta.deck.odds.map(o => [o.group, o])), out = [];
      const h = (o) => o ? 1 - (1 - o.white) * (1 - o.gold) : 0;
      for (const o of d.odds) { const b = base.get(o.group), s = SK.get(o.group) || { w: o.group, gd: null };
        if (Math.abs(o.gold - (b ? b.gold : 0)) >= 0.005) out.push({ g: o.group, gold: true, nm: s.gd || s.w, a: b ? b.gold : 0, z: o.gold });
        if (Math.abs(h(o) - h(b)) >= 0.005 && (h(o) - o.gold > 0.005 || h(b) - (b ? b.gold : 0) > 0.005)) out.push({ g: o.group, gold: false, nm: s.w, a: h(b), z: h(o) }); }
      out.sort((x, y) => Math.abs(y.z - y.a) - Math.abs(x.z - x.a));
      if (!out.length) return '';
      const pc = (p) => Math.round(p * 100) + '%', one = (x) => `<span class="swd1${x.z < x.a ? ' down' : ''}">${icon(x.g, x.gold)}${esc(x.nm)} <b>${pc(x.a)} → ${pc(x.z)}</b></span>`;
      if (out.length <= 2) return `<span class="swd">${out.map(one).join('')}</span>`;
      return `<button type="button" class="swd more" aria-expanded="false">${out.map((x, i) => i < 2 ? one(x) : one(x).replace('class="swd1', 'class="swd1 extra')).join('')}<span class="swmore" data-n="${out.length - 2}">+${out.length - 2} more</span></button>`; };
    const info = '<button type="button" class="oinfo" aria-expanded="false" aria-label="Hint odds from this deck" title="Hint odds from this deck">i</button>';
    // a member's portrait (her outfit's when the loop fixes one), with her name as its tooltip
    // a member's portrait: a button opening her panel, as her name at the top does
    const face = (k, size) => { const of = l.outfits[k].outfits, av = umaAv(M[k].id, of.length === 1 ? of[0].id : null, size), nm = M[k].name + (of.length === 1 ? ' ' + of[0].label : '');
      return `<button type="button" class="facebtn" data-uma="${M[k].id}" title="${esc(nm)}" aria-label="${esc(nm)}" aria-haspopup="dialog" aria-expanded="false">${av.outerHTML}</button>`; };
    const block = (d, title) => { const dk = document.createElement('div'); dk.className = 'deck';
      dk.innerHTML = `<div class="dtitle"><b>${title}</b></div>${info}<div class="dcards">${lastBor(d, d.cards.map((_, ci) => ci)).map(ci => chip(d, ci)).join('')}</div>` + oddsOf(d);
      return dk; };
    if (same) card.appendChild(block(meta.deck, D ? `Deck for all ${M.length === 3 ? 'three' : 'four'}` : 'Deck'));
    else {
      // the deck most members share, then each other member's swaps against it, one row each: "X swaps A → B"
      const users = M.map((m, k) => k).filter(k => key(D[k]) === key(meta.deck));
      card.appendChild(block(meta.deck, `Deck for <span class="dwho">${users.map(k => face(k, 42)).join('')}</span><span class="vh">${esc(users.map(k => M[k].name).join(', '))}</span>`));
      const seen = new Set();
      // a label where the swap rows start: each row is another uma's deck, given by what it swaps from the deck above
      if (M.some((m, k) => key(D[k]) !== key(meta.deck))) { const h = document.createElement('p'); h.className = 'swhead'; h.textContent = 'Swaps'; card.appendChild(h); }
      M.forEach((m, k) => { if (key(D[k]) === key(meta.deck) || seen.has(key(D[k]))) return; seen.add(key(D[k]));
        const d = D[k]; const who = M.filter((_, j) => key(D[j]) === key(d)).map(x => x.name);
        // who: her character icon (her outfit's when the loop fixes one), the words for screen readers and the tooltip
        const whoT = who.length === 1 ? `${who[0]}’s deck` : who.slice(0, -1).join(', ') + ' and ' + who.at(-1);   // "X’s deck swaps …", "X and Y swap …"
        const faces = M.map((x, j) => j).filter(j => key(D[j]) === key(d)).map(j => face(j, 64)).join('');   // each portrait its own tooltip
        const out = meta.deck.cards.map((_, ci) => ci).filter(ci => !d.cards.includes(meta.deck.cards[ci]));
        const inn = d.cards.map((_, ci) => ci).filter(ci => !meta.deck.cards.includes(d.cards[ci]));
        const dk = document.createElement('div'); dk.className = 'deck swap';
        dk.innerHTML = `<div class="swrow"><span class="swho">${faces}<span class="vh">${esc(whoT)}</span></span><span class="swt">${who.length === 1 ? 'swaps' : 'swap'}</span><span class="swc">${lastBor(meta.deck, out).map(ci => chip(meta.deck, ci)).join('')}</span>` +
          `<span class="swa" aria-hidden="true">→</span><span class="vh">for</span><span class="swc">${lastBor(d, inn).map(ci => chip(d, ci)).join('')}</span>${deltas(d)}</div>${info}` + oddsOf(d);
        card.appendChild(dk); });
    }
    card.querySelectorAll('.swd.more').forEach(b => b.onclick = () => { const open = b.getAttribute('aria-expanded') !== 'true';
      b.setAttribute('aria-expanded', String(open)); const m = b.querySelector('.swmore'); m.textContent = open ? 'Show less' : `+${m.dataset.n} more`; });
    card.querySelectorAll('.deck .oinfo').forEach(b => b.onclick = () => { const o = b.closest('.deck').querySelector('.dodds');
      o.hidden = !o.hidden; b.setAttribute('aria-expanded', String(!o.hidden)); });
    card.querySelectorAll('.deck .dc').forEach(el => { const c = CARD_BY_ID.get(+el.dataset.cid); el.prepend(cardAv(c || null, 96)); });
  }
  let mem;
  if (sameLoop) { mem = document.createElement('p'); mem.className = 'same'; mem.textContent = 'Same loop as #1 (the same umas and outfits); only the deck differs.'; }
  else {
    mem = document.createElement('div'); mem.className = 'members' + (l.members.length === 3 ? ' n3' : '');
    l.members.forEach((m, k) => {
      const o = l.outfits[k]; const d = document.createElement('div'); d.className = 'mem';
      const outfit = o.outfits.length ? o.outfits.map(x => x.label).join(' or ') : 'any outfit';
      d.innerHTML = `<button type="button" class="head memb" aria-haspopup="dialog" aria-expanded="false"><span class="nm"></span></button><span class="of"></span><span class="sk"></span>` +
        `<span class="own" title="The odds that every target lands in a run with her as the trainee">as trainee: <span class="num">${fmtIn(l.per_trainee[k])}</span></span>`;
      d.dataset.uma = m.id; d.dataset.outfit = o.outfits.length === 1 ? o.outfits[0].id : ''; d.dataset.outfitLabel = outfit;
      d.querySelector('.nm').textContent = m.name; d.querySelector('.of').textContent = outfit;
      if (forced.includes(m.id)) { d.classList.add('forced'); const t = document.createElement('span'); t.className = 'vh'; t.textContent = ' (must include)'; d.querySelector('.nm').after(t); }
      d.querySelector('.head').prepend(umaAv(m.id, o.outfits.length === 1 ? o.outfits[0].id : null, 38));
      // targets on her own outfit's skill list (she has them when she is the trainee)
      const sk = d.querySelector('.sk');
      // each its icon and name
      for (const c of o.covers) { const s = SK.get(c.group) || { w: c.group, gd: null }; const sp = document.createElement('span');
        sp.className = 'spark' + (c.level === 2 ? ' g' : '');
        const ic = skillIcon(c.group, c.level === 2); if (ic) sp.appendChild(ic);
        sp.append(c.level === 2 ? (s.gd || s.w) : s.w); sk.appendChild(sp); }
      mem.appendChild(d);
    });
  }
  const firstDeck = card.querySelector('.deck');   // the loop (members, odds) reads before the decks that serve it
  card.insertBefore(mem, firstDeck);
  const acts = document.createElement('div'); acts.className = 'row lacts';
  // the chance of each count of targets landed (the solver's, exact over the landed sets), as the career planner's
  if (l.landed) { const ob = document.createElement('button'); ob.type = 'button'; ob.className = 'btn sbtn'; ob.textContent = 'Odds'; ob.setAttribute('aria-haspopup', 'dialog');
    ob.onclick = () => openOdds({ title: 'Odds of this loop', n, ex: l.landed, pAll: l.p_all, expected: l.landed.reduce((s, x, k) => s + k * x, 0) }); acts.appendChild(ob); }
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
  res.top.forEach((t, i) => box.appendChild(loopCard(t.loop, { ...meta, deck: t.deck, decks: t.decks || null }, { rank: many ? i + 1 : 0, top: res.top[0] })));
  if (refocus) $('rcol').focus({ preventScroll: true });
  markOwn();   // grey out anything marked not owned since the search (needs the cards on the page)
}
