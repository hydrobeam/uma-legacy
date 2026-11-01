// The career's sections: odds, deck (and its popup), targets and what gives each, landed counts, the schedule, and
// the results column (the options table and Try a run).
import { $, backdropCloses, el, fmtIn, lazyDialog } from '../../dom.js';
import { settings } from '../../settings.ts';
import { umaAv, skillIcon, cardAv, scenLogo, srcFor } from '../../images.js';
import { scenarioOf } from '../../scenario.js';
import { markOwn } from '../../popover.js';
import { openSched } from '../../agenda.js';
import { pct, deckHint } from '../../odds.js';
import { vets, plan } from '../../vets.js';
import { deckCardAt, openSkill } from '../../details.js';
import { optionsEl } from '../options.js';
import { renderSimulation } from '../analysis.js';
import { answer } from '../state.ts';
import { must } from '../../must.ts';
import { keyOf } from '../keys.js';
import { refFor, shown } from '../job.js';
import { tName, btn, sectionEl, deckEl, tableRows } from './parts.js';

// the career's odds of landing all n (each count is under the targets: countsEl)
export function oddsBtn(r) {
  const n = settings.value.targets.length,
    b = el('span', 'udodds');
  b.id = 'odds-open';
  b.append(el('b', '', fmtIn(r.p_all)), el('span', '', ` to land all ${n}`));
  b.title = `The chance this career lands all ${n} targets (each count under the targets)`;
  return b;
}
// the chance of landing exactly k of the n targets: exact over the landed sets (outcome_sets), else each target on its
// own odds (over 12 targets); the all-n and n-1-or-better figures are the solver's exact ones
const popCount = (mask) => {
  let c = 0;
  for (let x = mask; x; x &= x - 1) c++;
  return c;
};
function landedCounts(r) {
  const n = settings.value.targets.length,
    exactly = new Array(n + 1).fill(0);
  if (Array.isArray(r.outcome_sets)) {
    // outcome_sets[mask]: the chance of landing exactly the targets in mask (bit i: target i)
    r.outcome_sets.forEach((p, mask) => {
      exactly[popCount(mask)] += p;
    });
    return exactly;
  }
  // each target on its own odds: the count's distribution built one target at a time (dist[c]: c landed so far)
  let dist = [1];
  for (const t of r.targets) {
    const next = new Array(dist.length + 1).fill(0);
    dist.forEach((x, c) => {
      next[c] += x * (1 - t.p);
      next[c + 1] += x * t.p;
    });
    dist = next;
  }
  dist.forEach((x, c) => {
    exactly[c] = x;
  });
  return exactly;
}
// her deck in its popup (the game's dialog): the six cards with their priority and chain notes
// deckKey: the answer and deck it holds (`fillDeck`)
/** @type {string | null} */
let deckKey = null;
export const deckDialog = lazyDialog('gmdlg deckdlg', (dialog) => {
  dialog.id = 'deck-dlg';
  dialog.setAttribute('aria-labelledby', 'deck-title');
  dialog.addEventListener('close', () => $('deck-open')?.focus());
  backdropCloses(dialog);
});
export function openDeck(r) {
  fillDeck(r);
  deckDialog.get().showModal();
}
// (refilled while open when a new answer comes: a card marked not owned there searches again)
export function fillDeck(r) {
  // (only when the deck changed: a redraw with the same deck keeps a card's open panel)
  const key = `${keyOf(r)}|${JSON.stringify(r.deck.cards)}`;
  const made = deckDialog.made();
  if (made && deckKey === key && made.childElementCount) return;
  const deckDlg = deckDialog.get();
  deckKey = key;
  deckDlg.textContent = '';
  const h = el('h3', 'gmdhdr', 'Deck');
  h.id = 'deck-title';
  const body = el('div', 'gmdbody'),
    dk = deckEl(r);
  dk.id = 'tree-deck';
  body.appendChild(dk);
  const f = el('div', 'gmdfoot');
  f.appendChild(btn('Close', 'gmclose', () => deckDlg.close()));
  deckDlg.append(h, body, f);
}
// the deck cards that give target group g: by a hint, or by an event (a gold one, at its chain link)
function cardsGiving(r, g) {
  const out = [];
  for (let i = 0; i < r.deck.cards.length; i++) {
    const c = deckCardAt(r.deck, i).card;
    if (!c) continue;
    const ways = [];
    if ((c.whites || []).includes(g)) ways.push('hint');
    for (const [group, rarity, link] of c.ev || [])
      if (group === g) ways.push(`${rarity > 1 ? 'gold ' : ''}event${link ? ` at link ${link}` : ''}`);
    if (ways.length) out.push({ card: c, how: [...new Set(ways)].join(', ') });
  }
  return out;
}
// her six legacies (each parent, then her two parents), and whether each carries target group g
function legaciesCarrying(r, g) {
  const out = [];
  for (const ix of r.parents) {
    const v = vets[ix];
    if (!v) continue;
    out.push({ chara: v.chara, outfit: v.outfit, carries: v.sparks?.[g] > 0 });
    for (const p of v.parents) out.push({ chara: p.chara || 0, outfit: p.outfit, carries: p.sparks?.[g] > 0 });
  }
  return out;
}
// the targets, lowest first, one aligned row each: the game's skill tile with her chance to land its spark (tap: Skill
// Details, where it comes from with each chance); her chance to get the skill on the game's bond gauge, gold for its
// gold skill and white for the white alone (every bar the same length); the chance her legacies pass it on (the game's
// three uma figures); what else gives it (the deck's cards with their chance, her outfit, her own events, the
// scenario). Under them the chance of each count of targets, as columns.
export function targetsEl(r) {
  const s = sectionEl('rtgt', 'Targets'),
    list = el('ul', 'ctgs'),
    ico = srcFor('ui', 'ico_legacies');
  for (const t of [...r.targets].sort((a, b) => a.p - b.p)) {
    const b = t.buy || {},
      gold = t.gold || (b.land_gold || 0) > (b.land_white || 0),
      li = el('li', 'ctg'),
      tile = el('button', 'udsk'),
      legacies = legaciesCarrying(r, t.group),
      cards = cardsGiving(r, t.group),
      deckP = deckHint(b.deck_gold || 0, b.deck_white || 0);
    tile.type = 'button';
    tile.setAttribute('aria-haspopup', 'dialog');
    tile.setAttribute('aria-label', `${tName(t.group)}: lands ${pct(t.p)}. Where it comes from`);
    const icon = skillIcon(t.group, false);
    if (icon) tile.appendChild(icon);
    const lands = el('b', 'tgp', pct(t.p));
    lands.title = `Lands as a spark: ${pct(t.p)}`;
    tile.append(el('span', 'udskn', tName(t.group)), lands);
    tile.onclick = () =>
      openSkill({ b, name: tName(t.group), icon: () => skillIcon(t.group, gold), gold, legacies, cards }, tile);
    // the bar: one pill as long as her chance to get it, gold for its gold skill, the rest white
    const bar = el('span', 'tgbar'),
      bg = b.gold || 0,
      bw = b.white || 0,
      fill = el('span', 'tgfill'),
      g = el('i', 'g');
    fill.style.width = `${Math.round((bg + bw) * 1000) / 10}%`;
    g.style.width = `${bg + bw > 0 ? Math.round((bg / (bg + bw)) * 1000) / 10 : 0}%`;
    fill.append(g, el('i', 'w'));
    bar.appendChild(fill);
    bar.title = `Acquired: ${pct(bg + bw)} (gold ${pct(bg)}, white only ${pct(bw)})`;
    const legs = el('span', 'tgfig');
    if (ico) legs.appendChild(Object.assign(el('img', 'tgico'), { src: ico, alt: '' }));
    legs.appendChild(el('b', '', String(Math.round(t.q * 100))));
    legs.title = `Inherited: ${pct(t.q)}`;
    // what else gives it: the deck's cards with their chance; her outfit, her own events and the scenario outright
    const deck = el('span', 'tgfig tgdk');
    if (cards.length) {
      for (const { card } of cards) deck.appendChild(cardAv(card, 20));
      deck.appendChild(el('b', '', String(Math.round(deckP * 100))));
      deck.title = `Deck: ${pct(deckP)}`;
    }
    if (b.outfit) {
      const o = umaAv(r.trainee.id, r.trainee.outfits[0]?.id, 20);
      o.title = `Her outfit gives its ${b.outfit === 2 ? 'gold skill' : 'white'}`;
      deck.appendChild(o);
    }
    if ((b.own_event || 0) > 0.005 && !b.outfit) {
      const o = umaAv(r.trainee.id, null, 20);
      o.title = `Her own events: ${pct(b.own_event)}`;
      deck.appendChild(o);
    }
    if (b.scenario) {
      const sc = scenLogo(scenarioOf(settings.value.scen));
      sc.title = 'The scenario gives it';
      deck.appendChild(sc);
    }
    li.append(tile, bar, legs, deck);
    list.appendChild(li);
  }
  s.append(list, countsEl(r));
  return s;
}
// the chance of each count of targets (all n first), as columns in the game's gauge colours: the chance over a column
// as tall as its share (all n in gold), the count under it, "1 in X" on hover; under the axis, the game's limit arrow
// where the average roll lands (the expected count, between the columns it falls between)
function countsEl(r) {
  const n = settings.value.targets.length,
    ex = landedCounts(r),
    top = Math.max(...ex),
    chart = el('div', 'tgchart'),
    cols = el('div', 'tgcounts');
  for (let k = n; k >= 0; k--) {
    const c = el('span', `tgc${k === n ? ' all' : ''}`),
      bar = el('span', 'tgcb'),
      col = el('i');
    col.style.height = `${top > 0 ? Math.max(2, Math.round((ex[k] / top) * 100)) : 0}%`;
    bar.appendChild(col);
    c.append(el('b', '', pct(ex[k])), bar, el('span', 'tgck', `${k}/${n}`));
    c.title = `Exactly ${k} of ${n}: ${fmtIn(ex[k])} (${pct(ex[k])})`;
    cols.appendChild(c);
  }
  chart.appendChild(cols);
  if (Number.isFinite(r.expected)) {
    const avg = el('span', 'tgavg'),
      arrow = srcFor('ui', 'arrow_avg');
    // column n - e from the left (all n first), at its centre: (n - e + 0.5) / (n + 1) of the width
    avg.style.left = `${((n - r.expected + 0.5) / (n + 1)) * 100}%`;
    if (arrow) avg.appendChild(Object.assign(el('img', ''), { src: arrow, alt: '' }));
    avg.appendChild(el('b', '', `Avg ${r.expected.toFixed(1)}`));
    avg.title = `The average roll lands ${r.expected.toFixed(2)} of ${n}`;
    chart.appendChild(avg);
  }
  return chart;
}
const schedOf = (r) => ({
  members: [{ id: r.trainee.id, name: r.trainee.name, short: r.trainee.name }],
  outfits: [{ outfits: r.trainee.outfits }],
  schedule: tableRows(r).filter((x) => x.run !== false),
});
export function schedBtn(r) {
  const ran = schedOf(r).schedule.length;
  const b = btn('Race schedule', '', () => openSched(schedOf(r)));
  b.id = 'sched-open';
  b.title = ran ? `She runs ${ran} G1s: ${r.g1_wins.toFixed(1)} wins expected` : 'No G1s';
  return b;
}
export function renderResults() {
  const box = $('results');
  box.textContent = '';
  const r = shown();
  if (!r) {
    renderSimulation();
    return;
  }
  // the table: the plan's careers, or the search's with a place open; with every place chosen, the last plan's (refFor)
  const open = !plan.trainee || plan.parents.some((p) => !p),
    { last, planState } = answer.value;
  if (
    (open &&
      (planState.state === 'done' ? planState.r.options : must(last, 'the sections draw a career on show').r.results)
        .length > 1) ||
    refFor()
  )
    box.appendChild(optionsEl());
  renderSimulation();
  markOwn();
}
