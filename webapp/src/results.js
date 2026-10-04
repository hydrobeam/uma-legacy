// ---------- results: one card per loop (odds, members, decks), the schedule and affinity buttons ----------
import { $, esc } from './dom.js';
import { CARDS, CARD_BY_ID, SK } from './data.js';
import { ui } from './state.js';
import { umaAv, cardAv } from './images.js';
import { cardTitle } from './cards.js';
import { paintRunNote } from './search.js';
import { closePop, markOwn } from './popover.js';
import { openSched } from './agenda.js';
import { openWhy } from './affinity.js';
export const fmtIn = (p) => p > 0 ? '1 in ' + (1 / p >= 100 ? Math.round(1 / p).toLocaleString() : (1 / p).toFixed(1)) : 'never';
const runs90 = (p) => p > 0 ? Math.ceil(Math.log(0.1) / Math.log(1 - p)).toLocaleString() : '—';
function loopCard(l, i, meta, proven) {
  const n = meta.targets.length;
  const card = document.createElement('article'); card.className = 'loop' + (i === 0 ? ' first' : '');
  card.innerHTML = `<div class="lhead"><span class="odds">${fmtIn(l.p_all)}<small>runs, all ${n} land</small></span>
    <span class="sub">at least ${n - 1}: ${fmtIn(l.p_minus1)} · 90% chance by ${runs90(l.p_all)} runs</span>${proven ? '<span class="badge" title="The search ranks decks and loops by multiplying each target&#39;s odds; nothing can beat this one on that ranking, given the race schedules found for the loops it solved. The odds shown are exact for the model.">Proven best</span>' : ''}</div>`;
  if (meta.deck) {
    const D = meta.decks, M = l.members;
    const key = (d) => d.cards.join('|');
    const same = !D || D.every(d => key(d) === key(meta.deck));
    const lastBor = (d, idx) => [...idx].sort((a, b) => /\(borrowed\)$/.test(d.cards[a]) - /\(borrowed\)$/.test(d.cards[b]));   // borrowed card in the sixth slot, as in the game
    const chip = (d, ci) => { const nm = d.cards[ci]; const pals = new Set(d.pal_cards || []);
      const bor = /\(borrowed\)$/.test(nm); const base = nm.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, '').replace(/ \(link \d: [^()]*\)$/, '');
      const endAt = (d.end_chain_at || [])[ci] || 0, pick = (d.choices || [])[ci] || '';   // pick: the choice to prioritise, e.g. "link 1: Uma Stan"
      const tagPick = pick ? ' · prioritise ' + pick : '';
      const c = (d.card_ids && CARD_BY_ID.get(d.card_ids[ci])) || CARDS.find(x => x.name === base); const t = c ? cardTitle(c) : { nm: base, ti: '' };
      const tag = bor ? 'borrow' : pals.has(base) ? 'pal' : '', full = t.nm + (t.ti ? ' · ' + t.ti : '') + (tag ? ` (${tag})` : '') + tagPick;
      return `<button type="button" class="dc${bor ? ' bor' : pals.has(base) ? ' pal' : ''}" data-cid="${c ? c.id : ''}" data-name="${esc(t.nm)}"${endAt ? ` data-end="${endAt}"` : ''}` +
        ` title="${esc(full)}" aria-label="${esc(full)}" aria-haspopup="dialog" aria-expanded="false">` +
        `<span class="dn">${esc(full)}</span>${pick ? `<i>prioritise ${esc(pick)}</i>` : ''}${endAt ? `<i>chain end: link ${endAt}</i>` : ''}</button>`; };
    const oddsOf = (d, lead) => { const odds = d.odds.filter(o => o.white > 0.005 || o.gold > 0.005).map(o => { const s = SK.get(o.group) || { w: o.group, gd: null };
        // gold, then any hint at all (white or gold): the two numbers the landing odds depend on
        const h = 1 - (1 - o.white) * (1 - o.gold), parts = [];
        if (o.gold > 0.005) parts.push(`${esc(s.gd || s.w)} <b>${Math.round(o.gold * 100)}%</b>`);
        if (h - o.gold > 0.005) parts.push(o.gold > 0.005 ? `any hint <b>${Math.round(h * 100)}%</b>` : `${esc(s.w)} <b>${Math.round(h * 100)}%</b>`);
        return parts.join(' · '); }).join('</span><span>');
      return odds ? `<div class="dodds"><span>${lead}</span><span>${odds}</span></div>` : '<div class="dodds">This deck gives none of the targets.</div>'; };
    const block = (d, title, lead) => { const dk = document.createElement('div'); dk.className = 'deck';
      dk.innerHTML = `<div class="row between"><b>${title}</b></div><div class="dcards">${lastBor(d, d.cards.map((_, ci) => ci)).map(ci => chip(d, ci)).join('')}</div>` + oddsOf(d, lead);
      return dk; };
    if (same) card.appendChild(block(meta.deck, D ? 'Deck for all four' : 'Deck', 'From the deck, every run: '));
    else {
      // the deck most members share, then each other member's swaps against it
      const users = M.filter((m, k) => key(D[k]) === key(meta.deck)).map(m => m.name);
      card.appendChild(block(meta.deck, 'Deck for ' + users.join(', '), 'From this deck, every run: '));
      const seen = new Set();
      M.forEach((m, k) => { if (key(D[k]) === key(meta.deck) || seen.has(key(D[k]))) return; seen.add(key(D[k]));
        const d = D[k]; const who = M.filter((_, j) => key(D[j]) === key(d)).map(x => x.name);
        const whoT = who.length === 1 ? `${esc(who[0])}'s deck` : `Deck for ${who.map(esc).join(', ')}`; const out = meta.deck.cards.map((_, ci) => ci).filter(ci => !d.cards.includes(meta.deck.cards[ci]));
        const inn = d.cards.map((_, ci) => ci).filter(ci => !meta.deck.cards.includes(d.cards[ci]));
        const dk = document.createElement('div'); dk.className = 'deck swap';
        dk.innerHTML = `<div class="row between"><b>${whoT}</b></div>` +
          `<div class="dcards"><span class="swl o">Out</span>${lastBor(meta.deck, out).map(ci => chip(meta.deck, ci)).join('') || '<span class="hint">nothing</span>'}</div>` +
          `<div class="dcards"><span class="swl i">In</span>${lastBor(d, inn).map(ci => chip(d, ci)).join('') || '<span class="hint">nothing</span>'}</div>` + oddsOf(d, who.length === 1 ? 'From her deck, every run: ' : 'From this deck, every run: ');
        card.appendChild(dk); });
    }
    card.querySelectorAll('.deck .dc').forEach(el => { const c = CARD_BY_ID.get(+el.dataset.cid); el.prepend(cardAv(c || null, 96)); });
  }
  const mem = document.createElement('div'); mem.className = 'members';
  l.members.forEach((m, k) => {
    const o = l.outfits[k]; const d = document.createElement('div'); d.className = 'mem' + (o.covers.length ? ' covers' : '');
    const outfit = o.outfits.length ? o.outfits.map(x => x.label).join(' or ') : 'any outfit';
    d.innerHTML = `<button type="button" class="head memb" aria-haspopup="dialog" aria-expanded="false"><span class="nm"></span></button><span class="of"></span><span class="sk"></span><span class="own">own runs ${fmtIn(l.per_trainee[k])}</span>`;
    d.dataset.uma = m.id; d.dataset.outfit = o.outfits.length === 1 ? o.outfits[0].id : ''; d.dataset.outfitLabel = outfit;
    d.querySelector('.nm').textContent = m.name; d.querySelector('.of').textContent = outfit;
    d.querySelector('.head').prepend(umaAv(m.id, o.outfits.length === 1 ? o.outfits[0].id : null, 38));
    const sk = d.querySelector('.sk');
    for (const c of o.covers) { const s = SK.get(c.group) || { w: c.group, gd: null }; const sp = document.createElement('span');
      sp.className = 'spark' + (c.level === 2 ? ' g' : ''); sp.textContent = c.level === 2 ? (s.gd || s.w) : s.w; sk.appendChild(sp); }
    mem.appendChild(d);
  });
  const firstDeck = card.querySelector('.deck');   // the loop (members, odds) reads before the decks that serve it
  card.insertBefore(mem, firstDeck);
  const acts = document.createElement('div'); acts.className = 'row lacts';
  const sb = document.createElement('button'); sb.type = 'button'; sb.className = 'btn sbtn'; sb.textContent = 'Race schedule'; sb.onclick = () => openSched(l); acts.appendChild(sb);
  if (l.detail) { const ab = document.createElement('button'); ab.type = 'button'; ab.className = 'btn sbtn'; ab.textContent = 'Affinity'; ab.onclick = () => openWhy(l); acts.appendChild(ab); }
  card.appendChild(acts);
  return card;
}
export function renderDeck(res, meta) {
  closePop();
  const box = $('results'); box.textContent = '';
  if (res.error) { const e = document.createElement('p'); e.className = 'err'; e.textContent = res.error; box.appendChild(e); return; }
  if (!res.top.length) { const st = document.createElement('div'); st.className = 'status'; st.textContent = 'No loop fits these settings.'; box.appendChild(st); return; }
  ui.runNote = meta.example ? 'Showing an example: Unity Cup skills, everything owned'
    : `Searched ${res.decks_built.toLocaleString()} decks and ${res.foursomes.toLocaleString()} loops in ${(meta.ms / 1000).toFixed(1)} s`;
  paintRunNote();
  res.top.forEach((t, i) => box.appendChild(loopCard(t.loop, i, { ...meta, deck: t.deck, decks: t.decks || null }, i === 0 && res.proven)));
  markOwn();   // grey out anything marked not owned since the search (needs the cards on the page)
}
