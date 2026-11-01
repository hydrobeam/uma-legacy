// The career's sections in the tree: the odds, the deck (its button and popup), the race schedule's button, the
// targets with what gives each, and the chance of each count of them.
import { useRef } from 'preact/hooks';
import { fmtIn } from '../../dom.ts';
import { settings } from '../../settings.ts';
import { srcFor } from '../../images.ts';
import { scenarioOf } from '../../scenario.ts';
import { popOpener } from '../../popover.ts';
import { pct, deckHint } from '../../odds.ts';
import { vets } from '../../veterans/store.ts';
import { deckCardAt } from '../../details.ts';
import { cardTitle, borLast, chainEndNote } from '../../cards.ts';
import { CardAv, ScenLogo, SkillIcon, UmaAv } from '../../ui/Images.tsx';
import { Dialog } from '../../ui/Dialog.tsx';
import { shown } from '../current.ts';
import { tName, tableRows } from './parts.ts';
import { deckOpen, openDeck, deckClosed } from './dialogs.ts';
import { Guard } from './Guard.tsx';
import { openSkill } from './SkillDetails.tsx';
import { openSched } from '../../ui/Schedule.tsx';

/** the career on show, as the answer store hands it (read-only) */
export type ShownCareer = NonNullable<ReturnType<typeof shown>>;
type CareerReport = ShownCareer;
/** the career's odds of landing all n (each count is under the targets: Counts) */
export function Odds({ r }: { r: CareerReport }) {
  const n = settings.value.targets.length;
  return (
    <span
      class="udodds"
      id="odds-open"
      title={`The chance this career lands all ${n} targets (each count under the targets)`}
    >
      <b>{fmtIn(r.p_all)}</b>
      <span>{` to land all ${n}`}</span>
    </span>
  );
}
/** her deck's button (the deck in its popup) */
export const DeckButton = () => (
  <button type="button" class="btn" id="deck-open" aria-haspopup="dialog" onClick={openDeck}>
    Deck
  </button>
);
// her race schedule: the schedule window (the game's calendar), each race's figure the community table's on her own
// grades (parts.ts tableRows); the G1s she could enter but doesn't aren't shown
const schedOf = (r: CareerReport) => ({
  members: [{ id: r.trainee.id, name: r.trainee.name, short: r.trainee.name }],
  outfits: [{ outfits: r.trainee.outfits }],
  schedule: tableRows(r).filter((x: { run?: boolean }) => x.run !== false),
});
export function SchedButton({ r }: { r: CareerReport }) {
  const ran = schedOf(r).schedule.length;
  return (
    <button
      type="button"
      class="btn"
      id="sched-open"
      title={ran ? `She runs ${ran} G1s: ${(r.g1_wins ?? 0).toFixed(1)} wins expected` : 'No G1s'}
      onClick={() => openSched(schedOf(r))}
    >
      Race schedule
    </button>
  );
}
/** one card of a deck (here the career's, in its popup; Try a run's best career with her: analysis/view.tsx) (a click opens its panel: popover.ts, which reads the data-* and leaves the markup to this) */
export function DeckCard({ r, i }: { r: CareerReport; i: number }) {
  const d = r.deck,
    { base, borrowed: bor, card: c } = deckCardAt(d, i),
    endAt = (d.end_chain_at || [])[i] || 0,
    pick = (d.choices || [])[i] || (c?.pri ? c.pri.join(', ') : ''),
    t = c ? cardTitle(c) : { nm: base, ti: '' },
    tag = bor ? 'borrowed' : c?.pal ? 'pal' : '',
    full =
      t.nm +
      (t.ti ? ` · ${t.ti}` : '') +
      (tag ? ` (${tag})` : '') +
      (pick ? ` · prioritise ${pick}` : '') +
      (endAt ? ` · ${chainEndNote(c, endAt)}` : ''),
    off = !!c && !bor && settings.value.cardsOff[c.id] === false,
    ref = useRef<HTMLButtonElement>(null),
    open = popOpener.value !== null && popOpener.value === ref.current;
  return (
    <button
      ref={ref}
      type="button"
      class={`dc${bor ? ' bor' : c?.pal ? ' pal' : ''}${off ? ' off' : ''}`}
      data-cid={c ? String(c.id) : undefined}
      data-name={t.nm}
      data-end={endAt ? String(endAt) : undefined}
      title={full}
      aria-label={full}
      aria-haspopup="dialog"
      aria-expanded={open}
    >
      <CardAv c={c || null} size={96} titled={false} />
      <span class="dn">{full}</span>
      {pick && <i>{`prioritise ${pick}`}</i>}
      {!!endAt && <i>{chainEndNote(c, endAt)}</i>}
    </button>
  );
}
/** her deck in its popup (the game's dialog): the six cards with their priority and chain notes */
export function DeckDialog() {
  const r = shown(),
    open = deckOpen.value && !!r;
  return (
    <Dialog id="deck-dlg" cls="gmdlg deckdlg" labelledBy="deck-title" open={open} onClose={deckClosed}>
      {r && (
        <>
          <h3 class="gmdhdr" id="deck-title">
            Deck
          </h3>
          <div class="gmdbody">
            <div class="deck" id="tree-deck">
              <div class="dcards">
                <Guard what="The deck popup">
                  {borLast([...r.deck.cards]).map((i: number) => (
                    <DeckCard key={`${i}:${r.deck.cards[i]}`} r={r} i={i} />
                  ))}
                </Guard>
              </div>
            </div>
          </div>
          <div class="gmdfoot">
            <button type="button" class="btn gmclose" onClick={deckClosed}>
              Close
            </button>
          </div>
        </>
      )}
    </Dialog>
  );
}
/** whether a deck card is this module's (popover.ts leaves its markup alone) */

// the deck cards that give target group g: by a hint, or by an event (a gold one, at its chain link)
function cardsGiving(r: CareerReport, g: number) {
  const out = [];
  for (let i = 0; i < r.deck.cards.length; i++) {
    const c = deckCardAt(r.deck, i).card;
    if (!c) continue;
    const ways: string[] = [];
    if ((c.whites || []).includes(g)) ways.push('hint');
    for (const [group, rarity, link] of c.ev || [])
      if (group === g) ways.push(`${rarity > 1 ? 'gold ' : ''}event${link ? ` at link ${link}` : ''}`);
    if (ways.length) out.push({ card: c, how: [...new Set(ways)].join(', ') });
  }
  return out;
}
// her six legacies (each parent, then her two parents), and whether each carries target group g
function legaciesCarrying(r: CareerReport, g: number) {
  const out = [];
  for (const ix of r.parents) {
    const v = vets.value[ix];
    if (!v) continue;
    out.push({ chara: v.chara || 0, outfit: v.outfit, carries: v.sparks?.[g] > 0 });
    for (const p of v.parents) out.push({ chara: p.chara || 0, outfit: p.outfit, carries: p.sparks?.[g] > 0 });
  }
  return out;
}
type Target = CareerReport['targets'][number];
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
    sc = b.scenario ? scenarioOf(settings.value.scen) : null;
  return (
    <li class="ctg">
      <button
        type="button"
        class="udsk"
        aria-haspopup="dialog"
        aria-label={`${tName(t.group)}: lands ${pct(t.p)}. Where it comes from`}
        onClick={() => openSkill({ b, name: tName(t.group), group: t.group, goldIcon: gold, gold, legacies, cards })}
      >
        <SkillIcon group={t.group} />
        <span class="udskn">{tName(t.group)}</span>
        <b class="tgp" title={`Lands as a spark: ${pct(t.p)}`}>
          {pct(t.p)}
        </b>
      </button>
      <span class="tgbar" title={`Acquired: ${pct(bg + bw)} (gold ${pct(bg)}, white only ${pct(bw)})`}>
        <span class="tgfill" style={{ width: `${Math.round((bg + bw) * 1000) / 10}%` }}>
          <i class="g" style={{ width: `${bg + bw > 0 ? Math.round((bg / (bg + bw)) * 1000) / 10 : 0}%` }} />
          <i class="w" />
        </span>
      </span>
      <span class="tgfig" title={`Inherited: ${pct(t.q)}`}>
        {ico && <img class="tgico" src={ico} alt="" />}
        <b>{String(Math.round(t.q * 100))}</b>
      </span>
      <span class="tgfig tgdk" title={cards.length ? `Deck: ${pct(deckP)}` : undefined}>
        {cards.length > 0 && (
          <>
            {cards.map(({ card }) => (
              <CardAv key={card.id} c={card} size={20} />
            ))}
            <b>{String(Math.round(deckP * 100))}</b>
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
        {sc && <ScenLogo sc={sc} title="The scenario gives it" />}
      </span>
    </li>
  );
}
/** the targets, lowest first, one aligned row each, then the chance of each count of them */
export function Targets({ r }: { r: CareerReport }) {
  const ico = srcFor('ui', 'ico_legacies');
  return (
    <section class="rbsec rtgt">
      <h3>Targets</h3>
      <ul class="ctgs">
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
// the chance of landing exactly k of the n targets: exact over the landed sets (outcome_sets), else each target on its
// own odds (over 12 targets); the all-n and n-1-or-better figures are the solver's exact ones
const popCount = (mask: number) => {
  let c = 0;
  for (let x = mask; x; x &= x - 1) c++;
  return c;
};
function landedCounts(r: CareerReport) {
  const n = settings.value.targets.length,
    exactly: number[] = new Array(n + 1).fill(0);
  if (Array.isArray(r.outcome_sets)) {
    // outcome_sets[mask]: the chance of landing exactly the targets in mask (bit i: target i)
    r.outcome_sets.forEach((p: number, mask: number) => {
      exactly[popCount(mask)] += p;
    });
    return exactly;
  }
  // each target on its own odds: the count's distribution built one target at a time (dist[c]: c landed so far)
  let dist = [1];
  for (const t of r.targets) {
    const next: number[] = new Array(dist.length + 1).fill(0);
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
// the chance of each count of targets (all n first), as columns in the game's gauge colours: the chance over a column
// as tall as its share (all n in gold), the count under it, "1 in X" on hover; under the axis, the game's limit arrow
// where the average roll lands (the expected count, between the columns it falls between)
function Counts({ r }: { r: CareerReport }) {
  const n = settings.value.targets.length,
    ex = landedCounts(r),
    top = Math.max(...ex),
    arrow = srcFor('ui', 'arrow_avg');
  return (
    <div class="tgchart">
      <div class="tgcounts">
        {ex
          .map((_, k) => n - k)
          .map((k) => (
            <span
              key={k}
              class={`tgc${k === n ? ' all' : ''}`}
              title={`Exactly ${k} of ${n}: ${fmtIn(ex[k])} (${pct(ex[k])})`}
            >
              <b>{pct(ex[k])}</b>
              <span class="tgcb">
                <i style={{ height: `${top > 0 ? Math.max(2, Math.round((ex[k] / top) * 100)) : 0}%` }} />
              </span>
              <span class="tgck">{`${k}/${n}`}</span>
            </span>
          ))}
      </div>
      {Number.isFinite(r.expected) && (
        // column n - e from the left (all n first), at its centre: (n - e + 0.5) / (n + 1) of the width
        <span
          class="tgavg"
          style={{ left: `${((n - r.expected + 0.5) / (n + 1)) * 100}%` }}
          title={`The average roll lands ${r.expected.toFixed(2)} of ${n}`}
        >
          {arrow && <img src={arrow} alt="" />}
          <b>{`Avg ${r.expected.toFixed(1)}`}</b>
        </span>
      )}
    </div>
  );
}
