// The career's sections in the tree: the odds, the deck (its button and popup), the race schedule's button, the
// targets with what gives each, and the chance of each count of them.
import { assert } from '../../must.ts';
import { settings } from '../../state/settings.ts';
import { srcFor } from '../../model/images.ts';
import { scenarioOf } from '../../state/scenario.ts';
import { usePopOpener } from '../../results/CardPop.tsx';
import { pct, deckHint, fmtIn } from '../../model/odds.ts';
import { starsOf } from '../../veterans/names.ts';
import { borrowedLast, deckCards } from '../../results/details.ts';
import type { DeckSlot } from '../../results/details.ts';
import { cardTitle, chainEndNote } from '../../state/cards.ts';
import { CardAv, ScenLogo, SkillIcon, UmaAv } from '../../ui/Images.tsx';
import { Dialog, DialogBody, DialogClose, DialogFoot, DialogTitle } from '../../ui/Dialog.tsx';
import { legacyVet, shown } from '../current.ts';
import { tName, tableRows } from './parts.ts';
import { deckOpen, openDeck, deckClosed } from './dialogs.ts';
import { Guard } from './Guard.tsx';
import { openSkill } from './SkillDetails.tsx';
import { openSched } from '../../results/Schedule.tsx';
import { Button } from '../../ui/Button.tsx';
import { panel } from '../../ui/Panel.tsx';
import { GameBar } from '../../ui/GameBar.tsx';
import { chip } from '../../results/deckChip.ts';
import { skillTile } from './skillTile.ts';
import { cx } from 'tailwind-variants/lite';

/** the career on show, as the answer store hands it (read-only) */
export type ShownCareer = NonNullable<ReturnType<typeof shown>>;
type CareerReport = ShownCareer;
/** the career's odds of landing all n of its targets (each count is under the targets: Counts) */
export function Odds({ r }: { r: CareerReport }) {
  const n = r.targets.length;
  return (
    // the page's headline, as the loop page's: one line, the figure large
    <span
      class="text-gm-ink flex items-baseline gap-1.5 leading-tight"
      id="odds-open"
      title={`The chance this career lands all ${n} targets (each count under the targets)`}
    >
      <b class="text-headline font-black">{fmtIn(r.p_all)}</b>
      <span class="text-body font-bold">{` to land all ${n}`}</span>
    </span>
  );
}
/** her deck's button (the deck in its popup) */
export const DeckButton = () => (
  <Button id="deck-open" aria-haspopup="dialog" onClick={openDeck}>
    Deck
  </Button>
);
// her race schedule: the schedule window (the game's calendar), each race's figure the community table's on her own
// grades (parts.ts tableRows); the G1s she could enter but doesn't aren't shown
const schedOf = (r: CareerReport) => ({
  members: [{ k: 0, id: r.trainee.id, name: r.trainee.name, short: r.trainee.name, outfits: r.trainee.outfits }],
  schedule: tableRows(r).filter((x: { run?: boolean }) => x.run !== false),
});
export function SchedButton({ r }: { r: CareerReport }) {
  const ran = schedOf(r).schedule.length;
  return (
    <Button
      id="sched-open"
      title={ran ? `She runs ${ran} G1s: ${(r.g1_wins ?? 0).toFixed(1)} wins expected` : 'No G1s'}
      onClick={() => openSched(schedOf(r))}
    >
      Race schedule
    </Button>
  );
}
/** one card of a deck (here the career's, in its popup; Try a run's best career with her: analysis/view.tsx) (a click opens its panel: results/popover.ts) */
function DeckCard({ slot }: { slot: DeckSlot }) {
  const { base, borrowed: bor, card: c, endAt } = slot,
    pick = slot.choice || (c?.pri ? c.pri.join(', ') : ''),
    t = c ? cardTitle(c) : { nm: base, ti: '' },
    tag = bor ? 'borrowed' : c?.pal ? 'pal' : '',
    full =
      t.nm +
      (t.ti ? ` · ${t.ti}` : '') +
      (tag ? ` (${tag})` : '') +
      (pick ? ` · prioritise ${pick}` : '') +
      (endAt ? ` · ${chainEndNote(c, endAt)}` : ''),
    off = !!c && !bor && settings.value.cardsOff[c.id] === false,
    opener = usePopOpener(true, () => ({
      kind: 'card',
      cid: c ? c.id : null,
      name: t.nm,
      borrowed: bor,
      end: endAt || 0,
    }));
  return (
    // data-tag: borrowed or pal; data-off: greyed, marked not owned (the checks read both)
    // its own <button>: a deck card's chip, its look results/deckChip.ts `chip`, its data marks this list's
    <button
      type="button"
      class={chip({ tag: bor ? 'borrowed' : c?.pal ? 'pal' : 'plain' })}
      data-cid={c ? String(c.id) : undefined}
      data-name={t.nm}
      data-end={endAt ? String(endAt) : undefined}
      data-tag={tag || undefined}
      data-off={off ? '' : undefined}
      title={full}
      aria-label={full}
      aria-haspopup="dialog"
      {...opener}
    >
      <CardAv c={c || null} size={22} titled={false} shape="deck" friend={bor} dim={off ? 'always' : undefined} />
      <span class="group-has-data-[card=art]/chip:hidden">{full}</span>
      {pick && <i class="text-caption text-muted not-italic">{`prioritise ${pick}`}</i>}
      {!!endAt && <i class="text-caption text-muted not-italic">{chainEndNote(c, endAt)}</i>}
    </button>
  );
}
// a deck's cards: six to a row (`row`), three to a row of larger cards (`popup`, the deck popup's) or of smaller ones
// (`mini`, Try a run's comparison panel)
const CARDS = {
  row: 'grid grid-cols-[repeat(6,minmax(0,84px))] items-start gap-2.5 max-md:gap-1.5',
  popup: 'grid grid-cols-[repeat(3,minmax(0,104px))] items-start justify-center gap-2.5 max-md:gap-1.5',
  mini: 'grid grid-cols-[repeat(3,minmax(0,72px))] items-start gap-2.5 max-md:gap-1.5',
} as const;
/** a deck's cards, borrowed last, each a DeckCard (data-deck-cards: the checks find a deck's by it) */
export function DeckCards({ deck, fit }: { deck: Parameters<typeof deckCards>[0]; fit: keyof typeof CARDS }) {
  return (
    <div class={CARDS[fit]} data-deck-cards="">
      {borrowedLast(deckCards(deck)).map((slot) => (
        <DeckCard key={`${slot.i}:${slot.named}`} slot={slot} />
      ))}
    </div>
  );
}
/** her deck in its popup (the game's dialog): the six cards with their priority and chain notes */
export function DeckDialog() {
  const r = shown(),
    open = deckOpen.value && !!r;
  return (
    <Dialog size="md" id="deck-dlg" kind="popup" labelledBy="deck-title" open={open} onClose={deckClosed}>
      {r && (
        <>
          <DialogTitle id="deck-title">Deck</DialogTitle>
          <DialogBody>
            <div class={cx(panel, 'grid gap-2 p-2.5 max-md:p-2')} id="tree-deck">
              <Guard what="The deck popup">
                <DeckCards deck={r.deck} fit="popup" />
              </Guard>
            </div>
          </DialogBody>
          <DialogFoot>
            <DialogClose onClick={deckClosed} />
          </DialogFoot>
        </>
      )}
    </Dialog>
  );
}
// the deck cards that give target group g: by a hint, or by an event (a gold one, at its chain link)
function cardsGiving(r: CareerReport, g: number) {
  const out = [];
  for (const { card: c } of deckCards(r.deck)) {
    if (!c) continue; // (a card the page doesn't know: nothing to say it gives)
    const ways: string[] = [];
    if ((c.whites || []).includes(g)) ways.push('hint');
    for (const [group, rarity, link] of c.ev || [])
      if (group === g) ways.push(`${rarity > 1 ? 'gold ' : ''}event${link ? ` at link ${link}` : ''}`);
    if (ways.length) out.push({ card: c, how: [...new Set(ways)].join(', ') });
  }
  return out;
}
// her six legacies (each parent, then her two parents; an absent one no one, character 0), and whether each carries
// target group g
function legaciesCarrying(r: CareerReport, g: number) {
  const out = [];
  for (const ix of r.parents) {
    const v = legacyVet(ix);
    if (!v) continue; // (she has left the pool under the answer on show)
    out.push({ chara: v.chara || 0, outfit: v.outfit, carries: starsOf(v.sparks, g) > 0 });
    for (const p of v.parents)
      out.push(
        p ? { chara: p.chara || 0, outfit: p.outfit, carries: starsOf(p.sparks, g) > 0 } : { chara: 0, carries: false },
      );
  }
  return out;
}
type Target = CareerReport['targets'][number];
// a target's figures: the legacies' chance, and what else gives it
const FIGURE = 'text-note text-gm-ink2 inline-flex min-w-0 flex-wrap items-center gap-0.5 font-black tabular-nums';
// one target's row: the game's skill tile with her chance to land its spark (tap: Skill Details, where it comes from
// with each chance); her chance to get the skill on the game's bond gauge, gold for its gold skill and white for the
// white alone; the chance her legacies pass it on; what else gives it (the deck's cards with their chance, her outfit,
// her own events, the scenario)
function TargetRow({ r, t, ico }: { r: CareerReport; t: Target; ico: string | null }) {
  const b = t.buy || {},
    gold = t.gold || (b.land_gold || 0) > (b.land_white || 0),
    legacies = legaciesCarrying(r, t.group),
    cards = cardsGiving(r, t.group),
    deckP = deckHint(b.deck_gold || 0, b.deck_white || 0),
    bg = b.gold || 0,
    bw = b.white || 0,
    sc = b.kind === 'scenario' ? scenarioOf(settings.value.scen) : null;
  return (
    // (its cells are the list's grid's: data-target marks the row for the checks)
    <li class="contents" data-target={t.group}>
      {/* its own <button>: a target's row, a skill tile, its look career/view/skillTile.ts `skillTile` */}
      <button
        type="button"
        class={skillTile({ size: 'row', class: 'max-sm:col-span-full max-sm:mt-1.5' })}
        aria-haspopup="dialog"
        aria-label={`${tName(t.group)}: lands ${pct(t.p)}. Where it comes from`}
        onClick={() => openSkill({ b, name: tName(t.group), group: t.group, goldIcon: gold, gold, legacies, cards })}
      >
        <SkillIcon group={t.group} cls="size-6" />
        <span class="text-body min-w-0 flex-1 leading-tight wrap-anywhere">{tName(t.group)}</span>
        <b class="text-heading text-gm-ink shrink-0 font-black tabular-nums" title={`Lands as a spark: ${pct(t.p)}`}>
          {pct(t.p)}
        </b>
      </button>
      {/* the game's bond gauge: its track and its marks at each fifth, gold for the gold skill, white for the white */}
      <span
        class="nine-gauge after:art-gauge-marks relative flex h-4.5 px-1.25 py-1 after:pointer-events-none after:absolute after:inset-x-1.25 after:inset-y-1"
        title={`Acquired: ${pct(bg + bw)} (gold ${pct(bg)}, white only ${pct(bw)})`}
      >
        <span
          class="flex min-w-0 overflow-hidden rounded-full"
          style={{ width: `${Math.round((bg + bw) * 1000) / 10}%` }}
        >
          <i
            class="art-gauge-gold shrink-0"
            style={{ width: `${bg + bw > 0 ? Math.round((bg / (bg + bw)) * 1000) / 10 : 0}%` }}
          />
          <i class="art-gauge-white flex-1" />
        </span>
      </span>
      <span class={FIGURE} title={`Inherited: ${pct(t.q)}`}>
        {ico && <img class="h-auto w-5.75" src={ico} alt="" />}
        <b class="ml-0.5">{String(Math.round(t.q * 100))}</b>
      </span>
      <span class={FIGURE} title={cards.length ? `Deck: ${pct(deckP)}` : undefined} data-deck-figure="">
        {cards.length > 0 && (
          <>
            {cards.map(({ card }) => (
              <CardAv key={card.id} c={card} size={20} />
            ))}
            <b class="ml-0.5">{String(Math.round(deckP * 100))}</b>
          </>
        )}
        {!!b.outfit && (
          <UmaAv
            key={`${r.trainee.id}:${r.trainee.outfits[0]?.id}`}
            uma={r.trainee.id}
            outfit={r.trainee.outfits[0]?.id}
            size={20}
            title={`Her outfit gives its ${b.outfit === 2 ? 'gold skill' : 'white'}`}
          />
        )}
        {(b.own_event || 0) > 0.005 && !b.outfit && (
          <UmaAv
            key={r.trainee.id}
            uma={r.trainee.id}
            outfit={null}
            size={20}
            title={`Her own events: ${pct(b.own_event ?? 0)}`}
          />
        )}
        {sc && <ScenLogo sc={sc} title="The scenario gives it" cls="h-4.5 w-9" />}
      </span>
    </li>
  );
}
/** the targets, lowest first, one aligned row each, then the chance of each count of them */
export function Targets({ r }: { r: CareerReport }) {
  const ico = srcFor('ui', 'ico_legacies');
  return (
    <section class="grid min-w-0 gap-2" data-sec="targets">
      <GameBar>Targets</GameBar>
      {/* one aligned row a target: the tile, the gauge, the legacies' figure, the others' (on a phone the tile over
          the other three) */}
      <ul class="grid grid-cols-[minmax(0,17rem)_minmax(6em,1fr)_3.4em_8em] items-center gap-x-3 gap-y-1.5 max-sm:grid-cols-[minmax(0,1fr)_3.2em_6.5em] max-sm:gap-x-2 max-sm:gap-y-1">
        {[...r.targets]
          .sort((a, b) => a.p - b.p)
          .map((t) => (
            <TargetRow key={t.group} r={r} t={t} ico={ico} />
          ))}
      </ul>
      <Counts r={r} />
    </section>
  );
}
// the chance of landing exactly k of the report's n targets, k = 0 ..= n (the report's: the settings can have changed
// under the answer on show, dimmed until the next one): exact over the landed sets (outcome_sets), else each target on
// its own odds (over 12 targets); the all-n and n-1-or-better figures are the solver's exact ones
const popCount = (mask: number) => {
  let c = 0;
  for (let x = mask; x; x &= x - 1) c++;
  return c;
};
function landedCounts(r: CareerReport): number[] {
  const sets = r.outcome_sets;
  if (Array.isArray(sets)) {
    // sets[mask]: the chance of landing exactly the targets in mask (bit i: target i), one per mask of the n
    assert(sets.length === 2 ** r.targets.length, 'the landed sets are every mask of the targets');
    return Array.from({ length: r.targets.length + 1 }, (_, k) =>
      sets.reduce((sum, p, mask) => (popCount(mask) === k ? sum + p : sum), 0),
    );
  }
  // each target on its own odds: the count's distribution built one target at a time (dist[c]: c landed so far);
  // c landed after this target is c - 1 before it and landing it (`carried`), or c before it and missing it
  let dist = [1];
  for (const t of r.targets) {
    let carried = 0;
    const next = dist.map((x) => {
      const here = carried + x * (1 - t.p);
      carried = x * t.p;
      return here;
    });
    dist = [...next, carried];
  }
  return dist;
}
// the chance of each count of targets (all n first), as columns in the game's gauge colours: the chance over a column
// as tall as its share (all n in gold), the count under it, "1 in X" on hover; under the axis, the game's limit arrow
// where the average roll lands (the expected count, between the columns it falls between)
function Counts({ r }: { r: CareerReport }) {
  const n = r.targets.length,
    ex = landedCounts(r),
    top = Math.max(...ex),
    arrow = srcFor('ui', 'arrow_avg');
  return (
    <div
      class="bg-gm-pane inset-ring-gm-line relative mt-3.5 rounded-2xl px-1.5 pt-2.5 pb-7.5 inset-ring"
      data-counts=""
    >
      {/* (no gap: every column on one baseline) */}
      <div class="grid auto-cols-[minmax(0,1fr)] grid-flow-col">
        {ex
          .map((p, k) => ({ k, p }))
          .reverse()
          .map(({ k, p }) => (
            <span
              key={k}
              class="grid justify-items-center gap-0.75 tabular-nums"
              title={`Exactly ${k} of ${n}: ${fmtIn(p)} (${pct(p)})`}
              data-count={k}
            >
              {/* (a fixed row: every figure on one baseline) */}
              <b class="text-caption text-gm-ink2 h-4.5 leading-4.5 font-bold">{pct(p)}</b>
              <span class="border-gm-tile-edge flex h-13 w-full items-end justify-center border-b-2">
                <i
                  class={cx(
                    'block w-[min(70%,30px)] rounded-t-sm',
                    k === n ? 'art-count-all inset-shadow-count-all' : 'art-count-bar inset-shadow-count-bar',
                  )}
                  style={{ height: `${top > 0 ? Math.max(2, Math.round((p / top) * 100)) : 0}%` }}
                />
              </span>
              <span class="text-caption text-gm-ink2 font-black">{`${k}/${n}`}</span>
            </span>
          ))}
      </div>
      {Number.isFinite(r.expected) && (
        // column n - e from the left (all n first), at its centre: (n - e + 0.5) / (n + 1) of the width
        <span
          class="absolute bottom-1 inline-flex -translate-x-1/2 items-center gap-0.5 whitespace-nowrap"
          style={{ left: `${((n - r.expected + 0.5) / (n + 1)) * 100}%` }}
          title={`The average roll lands ${r.expected.toFixed(2)} of ${n}`}
          data-avg=""
        >
          {arrow && <img class="size-5" src={arrow} alt="" />}
          <b class="text-caption text-count-avg font-black">{`Avg ${r.expected.toFixed(1)}`}</b>
        </span>
      )}
    </div>
  );
}
