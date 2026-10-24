// ---------- results: one card per loop (odds, members, decks), the odds, schedule and affinity buttons ----------
import { $, esc, fmtIn } from './dom.js';
import { CARDS, CARD_BY_ID, SK } from './data.js';
import { ui } from './state.js';
import { umaAv, cardAv, skillIcon, setRunnerFrom } from './images.js';
import { cardTitle, borLast, chainEndNote } from './cards.js';
import { closePop, markOwn, popHasFocus } from './popover.js';
import { pickOutfit, outfitNote } from './roster.js';
import { openSched } from './agenda.js';
import { openWhy } from './affinity.js';
import { openOdds } from './odds.js';
/**
 * A deck in a result (the solver's `PDeck::out`, deck/pdeck.rs). Parallel lists: `cards[i]` is named with ` (borrowed)` or
 * ` (end chain at link N)` appended where that applies; `card_ids[i]` is its id (plain pals have a name but no entry).
 * @typedef {object} ResultDeck
 * @property {string[]} cards
 * @property {number[]} card_ids
 * @property {string[]} [pal_cards] the cards in it that are pals, by name
 * @property {number[]} [end_chain_at] per card: the chain link to end its story at (0: none)
 * @property {string[]} [choices] per card: the skill to prioritise at its chain link
 * @property {{group: number, white: number, gold: number}[]} odds per target: the chance the deck hints its white, its gold
 * @property {number[]} [plain_ids] the plain pals' ids
 * @property {number} [pals] how many pals it holds
 * @property {string|null} [borrowed] the borrowed card, by name (null: none)
 *
 * A member of a loop (chart.rs `member_json`).
 * @typedef {{id: number, name: string, short: string}} LoopMember
 *
 * A loop in a result (loop_planner/search/report.rs `loop_json`); member k is `members[k]`, with `outfits[k]` and `per_trainee[k]`.
 * @typedef {object} ResultLoop
 * @property {LoopMember[]} members
 * @property {number} p_all  the chance every member lands every target
 * @property {number} p_minus1  the chance each lands all but at most one
 * @property {number} rank_value  what the results are ranked by
 * @property {{outfits: {id: number, label: string}[], covers: {group: number, level: number}[]}[]} outfits  per member: her
 *   outfits that cover a target, and what they cover
 * @property {number[]} per_trainee  per member: her chance of landing every target
 * @property {number[]} [landed]  the chance of landing exactly k targets, k = 0 ..= n, averaged over the loop's careers
 * @property {import('./agenda.js').ScheduleRow[]} schedule  the race schedule (`chart_rows`)
 * @property {{lineages: import('./affinity.js').Lineage[]}} detail  the affinity detail
 *
 * One result of the loop search: the loop and its deck.
 * @typedef {object} LoopResult
 * @property {ResultLoop} loop
 * @property {ResultDeck} deck  the deck most members share (one deck for the loop: that deck)
 * @property {ResultDeck[]} [decks]  per member, when each has her own
 *
 * The loop search's report (loop_planner/search/report.rs), or its refusal (`error`).
 * @typedef {object} LoopReport
 * @property {string} [error]  why it did not search
 * @property {LoopResult[]} top  the best loops, best first
 * @property {boolean} per_uma  a deck per member (false: one deck for the loop)
 * @property {number} decks_built  decks the search built
 * @property {number} foursomes  loop structures the job's sizes allow
 * @property {boolean} proven  the search ended by its stop rule: no unexamined loop can win
 * @property {{rho: number, rho_target: number, complete: boolean}} exact  what the answer rests on (`exact_json`): the
 *   highest cap certified, the one sought, and whether the search ended by its stop rule
 */
/** @param {ResultLoop} loop */
const loopKey = (loop) =>
  loop.members.map((m, k) => `${m.id}:${loop.outfits[k].outfits.map((o) => o.id).join('/')}`).join(' ');
/** @param {ResultDeck} deck */
const deckKey = (deck) => deck.cards.join('|');
const SHOWN = 0.005; // hint odds and their changes under half a percent are not shown
// the chance of at least the white: a gold hint counts for the white too
const atLeastWhite = (odds) => (odds ? 1 - (1 - odds.white) * (1 - odds.gold) : 0);
const namesOf = (group) => SK.get(group) || { w: group, gd: null };
const iconHTML = (group, gold) => {
  const icon = skillIcon(group, gold);
  return icon ? icon.outerHTML : '';
};
// the deck's card indices among `indices`, the borrowed card last (the sixth slot, as in the game)
const borrowedLast = (deck, indices) => borLast(deck.cards).filter((i) => indices.includes(i));
const allIndices = (list) => list.map((_, i) => i);
// one card of a deck as a button opening its panel: its art, name and title, and what to do with it in IT
/** @param {ResultDeck} deck @param {number} i the card's place in it */
function cardChip(deck, i) {
  const named = deck.cards[i],
    pals = new Set(deck.pal_cards || []);
  const borrowed = /\(borrowed\)$/.test(named),
    base = named.replace(/ \(borrowed\)$/, '').replace(/ \(end chain at link \d\)$/, '');
  const c = CARD_BY_ID.get(deck.card_ids[i]) || CARDS.find((x) => x.name === base); // plain pals have a name but no entry in card_ids
  // pick: the skill to prioritise in IT: the search's choice at a chain link ("link 1: Uma Stan"), or what a pal's
  // credited event choice needs (derived from the event data: c.pri)
  const endAt = (deck.end_chain_at || [])[i] || 0,
    pick = (deck.choices || [])[i] || (c?.pri ? c.pri.join(', ') : '');
  const pickNote = (pick ? ` · prioritise ${pick}` : '') + (endAt ? ` · ${chainEndNote(c, endAt)}` : '');
  const title = c ? cardTitle(c) : { nm: base, ti: '' };
  const tag = borrowed ? 'borrowed' : pals.has(base) ? 'pal' : '',
    full = title.nm + (title.ti ? ` · ${title.ti}` : '') + (tag ? ` (${tag})` : '') + pickNote;
  return (
    `<button type="button" class="dc${borrowed ? ' bor' : pals.has(base) ? ' pal' : ''}" data-cid="${c ? c.id : ''}" data-name="${esc(title.nm)}"${endAt ? ` data-end="${endAt}"` : ''}` +
    ` title="${esc(full)}" aria-label="${esc(full)}" aria-haspopup="dialog" aria-expanded="false">` +
    `<span class="dn">${esc(full)}</span>${pick ? `<i>prioritise ${esc(pick)}</i>` : ''}${endAt ? `<i>${esc(chainEndNote(c, endAt))}</i>` : ''}</button>`
  );
}
// per target the deck can hint: the gold's odds, then the white's (the chance of at least the white): the two numbers
// the landing odds depend on. Shown on demand (the (i) button by the deck's title): one tile per hint: icon, name and
// percentage over a bar (the icons alone don't tell skills apart: the game draws one icon per effect type)
/** @param {ResultDeck} deck */
function hintOddsHTML(deck) {
  const tile = (group, gold, name, p, tip) => {
    const percent = Math.round(p * 100);
    return `<span class="dsk${gold ? ' g' : ''}" title="${esc(tip || name)}: ${percent}%">${iconHTML(group, gold)}<span class="dnm">${esc(name)}</span><span class="dbar" aria-hidden="true"><span style="width:${percent}%"></span></span><b>${percent}%</b></span>`;
  };
  const odds = deck.odds
    .filter((o) => o.white > SHOWN || o.gold > SHOWN)
    .map((o) => {
      const names = namesOf(o.group),
        white = atLeastWhite(o),
        parts = [];
      if (o.gold > SHOWN) parts.push(tile(o.group, true, names.gd || names.w, o.gold));
      if (white - o.gold > SHOWN)
        parts.push(tile(o.group, false, names.w, white, o.gold > SHOWN ? `${names.w} or its gold` : ''));
      return parts.join('');
    })
    .join('');
  return `<div class="dodds" hidden><span class="dlab">Hint odds from this deck</span>${odds || '<span>This deck gives none of the targets.</span>'}</div>`;
}
// what a swap changes: the hint odds that move against the shared deck (the gold's, then at least the white's),
// largest change first; two shown, a click on the list shows them all (and again folds it). who: the members on
// that deck; a hint all of them have on their own outfit's skill list is no change for them (its gold: they have the
// gold; the white: they have at least the white), so it isn't listed (her deck drops it because she doesn't need it)
/** @param {ResultLoop} loop @param {ResultDeck} shared @param {ResultDeck} deck @param {number[]} who */
function swapDeltasHTML(loop, shared, deck, who) {
  const before = new Map(shared.odds.map((o) => [o.group, o])),
    changes = [];
  const ownLevel = (group) =>
    Math.min(...who.map((k) => (loop.outfits[k].covers.find((c) => c.group === group) || { level: 0 }).level));
  for (const o of deck.odds) {
    const b = before.get(o.group),
      names = namesOf(o.group),
      level = ownLevel(o.group);
    if (level < 2 && Math.abs(o.gold - (b ? b.gold : 0)) >= SHOWN)
      changes.push({ group: o.group, gold: true, name: names.gd || names.w, from: b ? b.gold : 0, to: o.gold });
    if (
      level < 1 &&
      Math.abs(atLeastWhite(o) - atLeastWhite(b)) >= SHOWN &&
      (atLeastWhite(o) - o.gold > SHOWN || atLeastWhite(b) - (b ? b.gold : 0) > SHOWN)
    ) {
      changes.push({ group: o.group, gold: false, name: names.w, from: atLeastWhite(b), to: atLeastWhite(o) });
    }
  }
  changes.sort((x, y) => Math.abs(y.to - y.from) - Math.abs(x.to - x.from));
  if (!changes.length) return '';
  const percent = (p) => `${Math.round(p * 100)}%`;
  const one = (x) =>
    `<span class="swd1${x.to < x.from ? ' down' : ''}">${iconHTML(x.group, x.gold)}${esc(x.name)} <b>${percent(x.from)} → ${percent(x.to)}</b></span>`;
  if (changes.length <= 2) return `<span class="swd">${changes.map(one).join('')}</span>`;
  return `<button type="button" class="swd more" aria-expanded="false">${changes.map((x, i) => (i < 2 ? one(x) : one(x).replace('class="swd1', 'class="swd1 extra'))).join('')}<span class="swmore" data-n="${changes.length - 2}">+${changes.length - 2} more</span></button>`;
}
const INFO_BUTTON =
  '<button type="button" class="oinfo" aria-expanded="false" aria-label="Hint odds from this deck" title="Hint odds from this deck">i</button>';
// a member's portrait (her outfit's when the loop fixes one): a button opening her panel, as her name at the top does,
// with her name as its tooltip
/** @param {ResultLoop} loop @param {number} k the member @param {number} size in px */
function faceButton(loop, k, size) {
  const member = loop.members[k],
    outfits = loop.outfits[k].outfits,
    one = outfits.length === 1;
  const avatar = umaAv(member.id, one ? outfits[0].id : null, size),
    name = member.name + (one ? ` ${outfits[0].label}` : '');
  return `<button type="button" class="facebtn" data-uma="${member.id}" title="${esc(name)}" aria-label="${esc(name)}" aria-haspopup="dialog" aria-expanded="false">${avatar.outerHTML}</button>`;
}
/** @param {ResultDeck} deck @param {string} title markup */
function deckBlock(deck, title) {
  const block = document.createElement('div');
  block.className = 'deck';
  block.innerHTML = `<div class="dtitle"><b>${title}</b></div>${INFO_BUTTON}<div class="dcards">${borrowedLast(
    deck,
    allIndices(deck.cards),
  )
    .map((i) => cardChip(deck, i))
    .join('')}</div>${hintOddsHTML(deck)}`;
  return block;
}
// another member's deck by its changes to the shared one, one row: her face (each with her name as its tooltip), the
// cards that go (greyed) ➜ the cards that come in, and what it does to the odds. `who`: the members on this deck
/** @param {ResultLoop} loop @param {ResultDeck} shared @param {ResultDeck} deck @param {number[]} who */
function swapRow(loop, shared, deck, who) {
  const names = who.map((k) => loop.members[k].name);
  const whose = `${names.length === 1 ? names[0] : `${names.slice(0, -1).join(', ')} and ${names.at(-1)}`}’s deck`; // "X’s deck", "X and Y’s deck"
  const faces = who.map((k) => faceButton(loop, k, 64)).join('');
  const out = allIndices(shared.cards).filter((i) => !deck.cards.includes(shared.cards[i]));
  const comingIn = allIndices(deck.cards).filter((i) => !shared.cards.includes(deck.cards[i]));
  const row = document.createElement('div');
  row.className = 'deck swap';
  row.innerHTML =
    `<div class="swrow"><span class="swho">${faces}<span class="vh">${esc(whose)}:</span></span>` +
    `<div class="swcards"><span class="swc swout"><span class="vh">out:</span>${borrowedLast(shared, out)
      .map((i) => cardChip(shared, i))
      .join('')}</span>` +
    `<span class="swa" aria-hidden="true">${ARROW}</span><span class="swc swin"><span class="vh">in:</span>${borrowedLast(
      deck,
      comingIn,
    )
      .map((i) => cardChip(deck, i))
      .join('')}</span></div>${swapDeltasHTML(loop, shared, deck, who)}</div>${INFO_BUTTON}${hintOddsHTML(deck)}`;
  return row;
}
// the deck(s) of a result into its card: one deck for all, or the deck most members share and the swaps from it
/** @param {HTMLElement} card @param {ResultLoop} loop @param {ResultDeck} shared @param {ResultDeck[]|null} decks per member */
function appendDecks(card, loop, shared, decks) {
  const members = loop.members;
  if (!decks || decks.every((d) => deckKey(d) === deckKey(shared))) {
    card.appendChild(deckBlock(shared, decks ? `Deck for all ${members.length === 3 ? 'three' : 'four'}` : 'Deck'));
  } else {
    const users = allIndices(members).filter((k) => deckKey(decks[k]) === deckKey(shared));
    card.appendChild(
      deckBlock(
        shared,
        `Deck for <span class="dwho">${users.map((k) => faceButton(loop, k, 42)).join('')}</span><span class="vh">${esc(users.map((k) => members[k].name).join(', '))}</span>`,
      ),
    );
    // a label where the other decks start: each row below is another uma's deck, by its changes to the deck above
    const heading = document.createElement('p');
    heading.className = 'swhead';
    heading.textContent = 'Swaps';
    card.appendChild(heading);
    const seen = new Set();
    members.forEach((_, k) => {
      const deck = decks[k];
      if (deckKey(deck) === deckKey(shared) || seen.has(deckKey(deck))) return;
      seen.add(deckKey(deck));
      card.appendChild(
        swapRow(
          loop,
          shared,
          deck,
          allIndices(members).filter((j) => deckKey(decks[j]) === deckKey(deck)),
        ),
      );
    });
  }
  card.querySelectorAll('.swd.more').forEach((b) => {
    b.onclick = () => {
      const open = b.getAttribute('aria-expanded') !== 'true';
      b.setAttribute('aria-expanded', String(open));
      const more = b.querySelector('.swmore');
      more.textContent = open ? 'Show less' : `+${more.dataset.n} more`;
    };
  });
  card.querySelectorAll('.deck .oinfo').forEach((b) => {
    b.onclick = () => {
      const odds = b.closest('.deck').querySelector('.dodds');
      odds.hidden = !odds.hidden;
      b.setAttribute('aria-expanded', String(!odds.hidden));
    };
  });
  card.querySelectorAll('.deck .dc').forEach((chip) => {
    const c = CARD_BY_ID.get(+chip.dataset.cid);
    chip.prepend(cardAv(c || null, 96));
  });
}
// the loop's members: face and name (a button opening her panel), the outfit named, the targets on her own outfit's
// skill list (she has them when she is the trainee) and the odds with her as the trainee
/** @param {ResultLoop} loop */
function membersEl(loop) {
  const box = document.createElement('div');
  box.className = `members${loop.members.length === 3 ? ' n3' : ''}`;
  loop.members.forEach((m, k) => {
    const offered = loop.outfits[k],
      member = document.createElement('div');
    member.className = 'mem';
    // one outfit named, never "any outfit" (her popup says when any would do, or which others are as good)
    const pick = pickOutfit(m.id, offered.outfits),
      outfit = pick.list[0]?.label || '';
    member.innerHTML =
      `<button type="button" class="head memb" aria-haspopup="dialog" aria-expanded="false"><span class="nm"></span></button><span class="of"></span><span class="sk"></span>` +
      `<span class="own" title="The odds that every target lands in a run with her as the trainee">as trainee: <span class="num">${fmtIn(loop.per_trainee[k])}</span></span>`;
    member.dataset.uma = String(m.id);
    member.dataset.outfit = String(offered.outfits.length === 1 ? offered.outfits[0].id : '');
    member.dataset.outfitLabel = outfit;
    member.dataset.outfitNote = outfitNote(pick);
    member.querySelector('.nm').textContent = m.name;
    member.querySelector('.of').textContent = outfit;
    member.querySelector('.head').prepend(umaAv(m.id, pick.list[0]?.id ?? null, 38));
    const skills = member.querySelector('.sk');
    for (const cover of offered.covers) {
      // each its icon and name
      const names = namesOf(cover.group),
        gold = cover.level === 2,
        spark = document.createElement('span');
      spark.className = `spark${gold ? ' g' : ''}`;
      const icon = skillIcon(cover.group, gold);
      if (icon) spark.appendChild(icon);
      spark.append(gold ? names.gd || names.w : names.w);
      skills.appendChild(spark);
    }
    box.appendChild(member);
  });
  return box;
}
const smallButton = (text) => {
  const b = document.createElement('button');
  b.type = 'button';
  b.className = 'btn sbtn';
  b.textContent = text;
  return b;
};
// the result's dialogs: the odds of each count landed (the solver's, exact over the landed sets, as the career
// planner's), the race schedule and the affinity
/** @param {ResultLoop} loop @param {number} n how many targets */
function actionsEl(loop, n) {
  const row = document.createElement('div');
  row.className = 'row lacts';
  if (loop.landed) {
    const odds = smallButton('Odds');
    odds.setAttribute('aria-haspopup', 'dialog');
    odds.onclick = () =>
      openOdds({
        title: 'Odds of this loop',
        n,
        ex: loop.landed,
        pAll: loop.p_all,
        expected: loop.landed.reduce((sum, p, k) => sum + k * p, 0),
      });
    row.appendChild(odds);
  }
  const schedule = smallButton('Race schedule');
  schedule.onclick = () => openSched(loop);
  row.appendChild(schedule);
  if (loop.detail) {
    const affinity = smallButton('Affinity');
    affinity.onclick = () => openWhy(loop);
    row.appendChild(affinity);
  }
  return row;
}
// one result: its odds, the three or four umas, the deck(s), the schedule and affinity buttons. ctx: {rank (1-based, 0 when
// only one result is shown), top: the #1 result (to say what differs from it)}
/**
 * @param {ResultLoop} loop
 * @param {RenderMeta & {deck: ResultDeck, decks: ResultDeck[]|null}} meta
 * @param {{rank: number, top: LoopResult}} ctx
 */
function loopCard(loop, meta, ctx) {
  const n = meta.targets.length;
  const card = document.createElement('article');
  card.className = `loop${ctx.rank <= 1 ? ' first' : ''}`;
  const sameLoop = ctx.rank > 1 && loopKey(ctx.top.loop) === loopKey(loop);
  card.innerHTML = `<div class="lhead">${ctx.rank ? `<span class="rank">#${ctx.rank}</span>` : ''}<span class="odds"><b>${fmtIn(loop.p_all)}</b> runs to land all ${n}</span></div>`;
  if (meta.deck) appendDecks(card, loop, meta.deck, meta.decks);
  let members;
  if (sameLoop) {
    members = document.createElement('p');
    members.className = 'same';
    members.textContent = 'Same loop as #1 (the same umas and outfits); only the deck differs.';
  } else members = membersEl(loop);
  card.insertBefore(members, card.querySelector('.deck')); // the loop (members, odds) reads before the decks that serve it
  card.appendChild(actionsEl(loop, n));
  return card;
}
// the game's green arrow (white-edged, dark green outline) between the card that goes and the one that comes in
const ARROW =
  '<svg viewBox="0 0 30 24" width="30" height="24" focusable="false"><path d="M2 8h13V2.5L28 12 15 21.5V16H2z" fill="#7ecf08" stroke="#3c7a00" stroke-width="2" stroke-linejoin="round"/>' +
  '<path d="M4.5 10h12.5V7.3L24 12" fill="none" stroke="#fff" stroke-opacity=".55" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"/></svg>';
/**
 * What a result is drawn for.
 * @typedef {object} RenderMeta
 * @property {{group: number}[]} targets  the job's targets
 */
/** @param {LoopReport} res a report with at least one loop @param {RenderMeta} meta */
export function renderDeck(res, meta) {
  // focus inside the old result (an open panel's Owned switch, say) would fall back to the page top: keep it in the results
  const box = $('results'),
    refocus = popHasFocus() || box.contains(document.activeElement);
  closePop();
  box.textContent = '';
  ui.headline = fmtIn(res.top[0].loop.p_all); // only results with loops reach this (search.js keeps errors off the page)
  setRunnerFrom(res.top[0].loop.members.map((m) => m.id)); // the runner while the next search works: one of this loop's umas
  const many = res.top.length > 1;
  res.top.forEach((t, i) => {
    box.appendChild(
      loopCard(t.loop, { ...meta, deck: t.deck, decks: t.decks || null }, { rank: many ? i + 1 : 0, top: res.top[0] }),
    );
  });
  if (refocus) $('rcol').focus({ preventScroll: true });
  markOwn(); // grey out anything marked not owned since the search (needs the cards on the page)
}
