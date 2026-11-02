// ---------- her card in Try a run: the game's "Umamusume Details" screen ----------
// Her portrait and outfit, her actions (the popup's: `actions`), her aptitude grades (Track / Distance / Style, the
// game's letters), then three tabs as in the game: Skills (the target skills she learned), Inspiration (her sparks as
// the game's spark bars, stars under each, then "Legacy Origin": her two parents' sparks) and Career Info (her support
// cards, her two legacies, the G1s she ran and won). No stats (the model has none).
// The tab on show and the races' fold are the analysis store's (they outlive the card). Her draft keeps one identity
// (career/analysis/state.ts says why), so her card's `spec` is made anew with every record (every edit), and the parts
// that read a signal (the tabs, the races' fold) draw her again from it rather than skip her as
// unchanged. Her sparks are edited through the store (`draftRef`).
import type { ComponentChildren, RefObject } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { gameData } from '../../model/data.ts';
import { settings } from '../../settings.ts';
import { umaName } from '../../veterans/names.ts';
import { borrowedLast, deckCards, fitSparks } from '../../details.ts';
import type { DeckSlot } from '../../details.ts';
import { openSkill } from '../view/SkillDetails.tsx';
import { InspView } from '../../ui/InspView.tsx';
import { usePopOpener } from '../../ui/CardPop.tsx';
import { chip, FRIENDS_TAG } from '../../ui/deckChip.ts';
import { skillTile } from '../../ui/skillTile.ts';
import { SegmentedControl } from '../../ui/SegmentedControl.tsx';
import { gradeOf } from '../../model/grades.ts';
import { Aptitude, Grade } from '../../gen/solver.ts';
import { CardAv, GradeBadge, SkillIcon, UmaAv } from '../../ui/Images.tsx';
import type { DeepReadonly } from '../../store.ts';
import type { CareerReport } from '../job.ts';
import { draftRef, parentsPlaced } from '../../veterans/store.ts';
import type { Draft, VetRecord } from '../../veterans/store.ts';
import { detailsTab as tab, racesShown, showTab, toggleRaces } from './state.ts';
import type { DetailsTab } from './state.ts';
import { Button } from '../../ui/Button.tsx';
import { cx } from 'tailwind-variants/lite';

type Career = DeepReadonly<CareerReport>;

// aptitude rows, as the game lays them out
const APT_ROWS = [
  {
    label: 'Track',
    cells: [
      { aptitude: Aptitude.Turf, name: 'Turf' },
      { aptitude: Aptitude.Dirt, name: 'Dirt' },
    ],
  },
  {
    label: 'Distance',
    cells: [
      { aptitude: Aptitude.Sprint, name: 'Sprint' },
      { aptitude: Aptitude.Mile, name: 'Mile' },
      { aptitude: Aptitude.Medium, name: 'Medium' },
      { aptitude: Aptitude.Long, name: 'Long' },
    ],
  },
  {
    label: 'Style',
    cells: [
      { aptitude: Aptitude.FrontRunner, name: 'Front' },
      { aptitude: Aptitude.PaceChaser, name: 'Pace' },
      { aptitude: Aptitude.LateSurger, name: 'Late' },
      { aptitude: Aptitude.EndCloser, name: 'End' },
    ],
  },
];
// a turn (0-71: 24 half-months a year) as "Classic Late Apr"
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const when = (turn: number) =>
  `${['Junior', 'Classic', 'Senior'][Math.floor(turn / 24)]} ${turn % 2 ? 'Late ' : 'Early '}${MONTHS[Math.floor((turn % 24) / 2)]}`;

// her card: the game's white panel under its green bar (its top edge the bar's), its parts a column; the head her
// portrait in its white mat and cream ring, the actions and her name over the pale hero gradient (on a phone the
// actions on a row of their own under the portrait and name)
const CARD =
  'bg-gm-panel border-gm-edge shadow-panel font-game text-gm-ink grid min-w-0 gap-2.5 rounded-b-2xl border border-t-0 pb-3';
const HEAD =
  'art-hero border-gm-line grid grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-3.5 border-b px-4 pt-3.5 pb-2.5 max-sm:grid-cols-[auto_minmax(0,1fr)]';
// her aptitudes: a label column, then four grade cells a row (on a phone the label over its four cells)
const APT_ROW =
  'grid grid-cols-[74px_repeat(4,minmax(0,1fr))] items-center gap-1.25 max-sm:grid-cols-4 max-sm:gap-x-0.75 max-sm:gap-y-0.5';
const APT_LABEL = 'text-body font-bold max-sm:col-span-full max-sm:text-note max-sm:leading-tight';
const APT_CELL =
  'bg-gm-cell border-gm-line inset-shadow-grade text-body flex min-w-0 items-center justify-between gap-0.5 rounded-md border py-0.5 pr-1.25 pl-2 font-bold max-sm:text-caption max-sm:pr-0.5 max-sm:pl-1.5 max-sm:tracking-tight';
/** what the card shows of her and her career */
export interface DetailsSpec {
  /** her draft */
  d: DeepReadonly<Draft>;
  /** the career she came from */
  r: Career;
  /** the G1s her schedule runs, each with her chance of winning it at least once */
  races: { ix: number; name: string; p: number; turns: number[] }[];
  raceName: (ix: number) => string;
  /** the target skills' groups, in the job's order */
  targets: readonly number[];
  /** her two parents' records, each with her number and what opens her node */
  legacies: { v: VetRecord; label: string; open: () => void }[];
  /** a parent's number on the page ("#3"), by her legacy index (career/current.ts `legacyNumber`) */
  numberOf: (i: number) => string;
}
/** her card (data-details; its head data-part="head"): `actions` beside her portrait, `winsButton` (the G1 wins
 *  editor's) in Career Info */
export function Details({
  spec,
  actions,
  winsButton,
  class: place,
}: {
  spec: DetailsSpec;
  actions: ComponentChildren;
  winsButton: ComponentChildren;
  /** where it sits: layout utilities only */
  class?: string;
}) {
  const { d, r } = spec,
    outfit = r.trainee.outfits[0]?.label || null,
    apt = d.apt || (r.aptitudes?.start.kind === 'known' ? r.aptitudes.start.start : null),
    card = useRef<HTMLDivElement>(null);
  return (
    <div class={cx(CARD, place)} ref={card} data-details="">
      <div class={HEAD} data-part="head">
        <div class="bg-gm-panel shadow-card-face grid rounded-full p-1">
          <UmaAv
            key={`${d.chara}:${d.outfit}`}
            uma={d.chara}
            outfit={d.outfit}
            size={96}
            label={outfit ? `${umaName(d.chara)} ${outfit}` : umaName(d.chara)}
          />
        </div>
        <div class="grid min-w-0 content-center justify-items-center gap-2 max-sm:col-span-full max-sm:row-start-2">
          {actions}
        </div>
        <div class="min-w-0 text-right">
          {outfit && <div class="text-title font-bold">{/^\[.*\]$/.test(outfit) ? outfit : `[${outfit}]`}</div>}
          <div class="text-display max-sm:text-heading leading-tight font-black wrap-anywhere">{umaName(d.chara)}</div>
        </div>
      </div>
      {apt && (
        // biome-ignore lint/a11y/useSemanticElements: today's markup (a grid of grade cells read as a table)
        <div class="grid gap-1.25 px-3.5 max-sm:px-2" role="table" aria-label="Her aptitudes">
          {APT_ROWS.map(({ label, cells }) => (
            // biome-ignore lint/a11y/useSemanticElements lint/a11y/useFocusableInteractive: (the table's rows, as above; a row is no control)
            <div key={label} class={APT_ROW} role="row">
              {/* biome-ignore lint/a11y/useSemanticElements lint/a11y/useFocusableInteractive: (its header cell) */}
              <span class={APT_LABEL} role="rowheader">
                {label}
              </span>
              {cells.map(({ aptitude, name }) => {
                const grade = gradeOf(apt, aptitude);
                return (
                  // biome-ignore lint/a11y/useSemanticElements: (its cells)
                  <span key={aptitude} class={APT_CELL} role="cell" title={`${name} ${Grade[grade]}`}>
                    <span class="truncate">{name}</span>
                    <GradeBadge g={grade} />
                  </span>
                );
              })}
            </div>
          ))}
        </div>
      )}
      <DetailTabs spec={spec} winsButton={winsButton} card={card} />
    </div>
  );
}

/** her two parents' legacy boxes for career `r`, each veteran found by `vetAt` from her legacy index and numbered by
 *  `numberOf` (career/current.ts `legacyVet`, `legacyNumber`; `open`: her node popup). One who has left the pool is
 *  left out: while a pool
 *  veteran's G1 wins window is open the answer on show waits, so the pool can change under it (an uma.moe listing
 *  landing) */
export const legaciesOf = (
  r: Career,
  vetAt: (i: number) => VetRecord | undefined,
  numberOf: (i: number) => string,
  open: (id: string) => void,
): DetailsSpec['legacies'] =>
  r.parents.flatMap((i) => {
    const v = vetAt(i);
    return v ? [{ v, label: numberOf(i), open: () => open(v.id) }] : [];
  });

const DEFS = [
  { key: 'skills', label: 'Skills' },
  { key: 'insp', label: 'Inspiration' },
  { key: 'info', label: 'Career Info' },
] as const;
// the tabs and their panes (on the game's grey pane), all three built up front as in the game; once a switch is drawn,
// the spark names now on show in her `card` shrink to fit (details.ts fitSparks)
function DetailTabs({
  spec,
  winsButton,
  card,
}: {
  spec: DetailsSpec;
  winsButton: ComponentChildren;
  card: RefObject<HTMLDivElement | null>;
}) {
  const onShow = tab.value;
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per tab drawn (`onShow` is a module signal's value, which Biome reads as constant)
  useLayoutEffect(() => {
    if (card.current) fitSparks(card.current);
  }, [onShow]);
  // (her draft's parents are the career's, in its order: draftOf)
  const legacyIndex = spec.r.parents;
  const pane = (key: DetailsTab) =>
    key === 'skills' ? (
      <SkillsPane spec={spec} />
    ) : key === 'insp' ? (
      // Inspiration: "Sparks" (hers, editable), then "Legacy Origin" (her two parents', read-only)
      <InspView
        nd={spec.d}
        parents={spec.d.parents}
        k="sim"
        at={draftRef()}
        targets={spec.targets}
        header
        parentLabel={(j) => spec.numberOf(legacyIndex[j])}
      />
    ) : (
      <InfoPane spec={spec} winsButton={winsButton} />
    );
  return (
    <>
      <SegmentedControl<DetailsTab>
        kind="tab"
        shape="equal"
        size="xl"
        class="mx-3 mt-1 max-sm:mx-2"
        label="Her details"
        value={tab.value}
        onPick={showTab}
        options={DEFS.map(({ key, label }) => ({
          v: key,
          id: `udtab-${key}`,
          dataK: `sim:tab:${key}`,
          controls: `udpane-${key}`,
          body: label,
        }))}
      />
      <div class="bg-gm-pane mx-3 min-w-0 rounded-lg p-2.5 max-sm:mx-2 max-sm:p-2">
        {DEFS.map(({ key }) => (
          <div
            key={key}
            id={`udpane-${key}`}
            class="grid min-w-0 gap-2"
            role="tabpanel"
            aria-labelledby={`udtab-${key}`}
            hidden={tab.value !== key}
          >
            {pane(key)}
          </div>
        ))}
      </div>
    </>
  );
}

// Skills: every target skill she could acquire this career, as the game's skill tiles (name only), two to a row (one on
// a phone): acquired in this result bright (gold tile for the gold version), the others greyed (data-off). A tile opens the game-style skill popup
// (view/SkillDetails.tsx): acquired or not, where it can come from, and how likely it is acquired and lands as a spark.
const PANE_NOTE = 'text-ui text-gm-ink2';
function SkillsPane({ spec: { d, r } }: { spec: DetailsSpec }) {
  let acquired = 0;
  const tiles = r.targets.flatMap((t) => {
    const b = t.buy;
    if (!b || b.gold + b.white <= 0) return [];
    // how she got it: bought in this result, else landed as a spark (gold when gold lands more often), else not at all
    const got =
      d.bought?.[t.group] || ((d.sparks[t.group] || 0) > 0 ? (b.land_gold > b.land_white ? 'gold' : 'white') : null);
    const gold = got === 'gold',
      skill = gameData().skillNames.get(t.group) || { w: `Skill ${t.group}`, gd: null };
    // the version shown: the one she got, else the one she would most likely get
    const goldShown = got ? gold : b.gold > b.white;
    const name = goldShown && skill.gd ? skill.gd : skill.w;
    if (got) acquired++;
    return [
      <button
        key={t.group}
        type="button"
        class={skillTile({ tone: !got ? 'off' : gold ? 'gold' : 'white', size: 'card' })}
        data-k={`sim:sk${t.group}`}
        data-off={got ? undefined : ''}
        aria-haspopup="dialog"
        aria-label={`${name}: ${got ? `acquired${gold ? ' (gold)' : ''}` : 'not acquired'}. Details`}
        onClick={() => openSkill({ b, name, got: got as 'gold' | 'white' | null, group: t.group, goldIcon: goldShown })}
      >
        <SkillIcon group={t.group} gold={goldShown} dim={!got} cls="size-7.5" />
        <span class="text-body leading-tight wrap-anywhere">{name}</span>
      </button>,
    ];
  });
  if (!tiles.length) return <p class={PANE_NOTE}>No target skill can be acquired in this career.</p>;
  return (
    <>
      <div class="grid grid-cols-2 gap-1.5 max-sm:grid-cols-1">{tiles}</div>
      <p
        class={PANE_NOTE}
      >{`${acquired} of ${tiles.length} target skills acquired. Tap a skill for where it comes from.`}</p>
    </>
  );
}

/** one support card of her deck, the decks' chip (its art alone; a borrowed one's in the friend frame, or without art
 *  ringed in pink with a Friends tag): a click opens its panel (ui/CardPop.tsx `usePopOpener`; endAt, its chain-end
 *  advice). data-tag: borrowed; data-off: greyed, marked not owned */
function DeckCell({ slot }: { slot: DeckSlot }) {
  const { base, borrowed: bor, card: c, endAt } = slot,
    off = !!c && !bor && settings.value.cardsOff[c.id] === false,
    named = base + (bor ? ' (borrowed)' : ''),
    opener = usePopOpener(true, () => ({
      kind: 'card',
      cid: c ? c.id : null,
      name: base,
      borrowed: bor,
      end: endAt || 0,
    }));
  return (
    <button
      type="button"
      class={chip({ tag: bor ? 'borrowed' : 'plain', friends: bor })}
      title={named}
      data-cid={c ? String(c.id) : undefined}
      data-name={base}
      data-end={endAt ? String(endAt) : undefined}
      data-tag={bor ? 'borrowed' : undefined}
      data-off={off ? '' : undefined}
      aria-label={`${named}. Card details`}
      aria-haspopup="dialog"
      {...opener}
    >
      <CardAv c={c || null} size={22} titled={false} shape="deck" friend={bor} dim={off ? 'always' : undefined} />
      {bor && <span class={FRIENDS_TAG}>Friends</span>}
    </button>
  );
}
// one of her two legacies, a box in Legacy 1's blue (both, as the old sheet drew them) lit under the pointer: her
// portrait (it opens her node), her place and name, her two parents joined by a bracket; the whole box opens her popup,
// as her portrait does
// her two parents, a column joined to her portrait by a bracket at its left
const GRANDPARENTS =
  'before:border-gm-edge after:border-gm-edge relative grid gap-2 before:absolute before:top-[22%] before:bottom-[22%] before:-left-3 before:rounded-l-sm before:border-2 before:border-r-0 after:absolute after:top-1/2 after:-left-4.5 after:w-1.5 after:border-t-2';
function Legacy({ v, label, open, j }: DetailsSpec['legacies'][number] & { j: number }) {
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents lint/a11y/noStaticElementInteractions: the box is the pointer's larger target; her portrait is the keyboard's button
    <div
      class="border-legacy-blue bg-gm-cell hover:shadow-node-hover relative min-w-0 cursor-pointer rounded-lg border-3 px-2 pt-5.5 pb-2"
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest('button')) open();
      }}
    >
      <span class="bg-legacy-blue text-ui absolute top-0 left-0 rounded-tl-sm rounded-br-lg pt-px pr-3 pb-0.5 pl-2 font-black text-white">{`Legacy ${j + 1}`}</span>
      <div class="grid grid-cols-[auto_minmax(0,1fr)] items-center gap-4.5">
        <button
          type="button"
          class="focus-visible:outline-accent grid cursor-pointer justify-items-center gap-0.5 rounded-lg p-0.5 text-center focus-visible:outline-3 focus-visible:outline-offset-2"
          aria-haspopup="dialog"
          aria-label={`Legacy ${j + 1}: ${umaName(v.chara)} (${label}). Her sparks`}
          onClick={open}
        >
          <UmaAv key={`${v.chara}:${v.outfit}`} uma={v.chara} outfit={v.outfit} size={64} />
          <span class="text-note max-w-23 leading-tight font-bold wrap-anywhere">{`${label} ${umaName(v.chara)}`}</span>
          {v.borrowed && <span class="bg-legacy-pink text-caption rounded-full px-2 font-black text-white">Guest</span>}
        </button>
        <div class={GRANDPARENTS}>
          {parentsPlaced(v.parents).map(([g, k]) => (
            <div key={k} class="text-note flex min-w-0 items-center gap-1.5 font-bold">
              <UmaAv
                key={`${g.chara}:${g.outfit}`}
                uma={g.chara || 0}
                outfit={g.outfit}
                size={40}
                label={umaName(g.chara)}
              />
              <span class="wrap-anywhere">{umaName(g.chara)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
// Career Info's parts: a heading ruled off to its right; a race of her record, a row (on a phone narrower) and its
// result, a win in the game's orange
const SUB = 'text-body after:bg-gm-line flex items-center gap-2 font-black after:h-px after:flex-1';
const RACE =
  'bg-gm-cell border-gm-line text-body grid grid-cols-[112px_auto_minmax(0,1fr)_40px] items-center gap-2 rounded-md border px-2.5 py-1.25 font-bold max-sm:grid-cols-[84px_auto_minmax(0,1fr)_30px] max-sm:gap-1.5';
const RESULT = { won: 'text-gm-won text-right font-black', lost: 'text-gm-lost text-right font-black' } as const;
// Career Info, as the game's: her support cards, her two legacies (each with her parents, the borrowed one a guest),
// and her career record (G1 races entered and won; the race-by-race record behind a button, with the G1 wins editor)
function InfoPane({
  spec: { d, r, races, raceName, legacies },
  winsButton,
}: {
  spec: DetailsSpec;
  winsButton: ComponentChildren;
}) {
  const deck = r.deck,
    inSched = new Set(races.map((x) => x.ix));
  // her schedule's G1s at their first turn, then the wins outside it (no turn: entered on the page)
  const entries = races
    .map((x) => ({ turn: x.turns[0] as number | null, name: x.name, won: d.wins.includes(x.ix), ix: x.ix }))
    .concat(d.wins.filter((ix) => !inSched.has(ix)).map((ix) => ({ turn: null, name: raceName(ix), won: true, ix })));
  entries.sort((a, b) => (a.turn ?? 99) - (b.turn ?? 99));
  return (
    <>
      {deck && (
        <>
          <div class={SUB}>Support Cards</div>
          <div class="grid grid-cols-[repeat(6,minmax(0,64px))] justify-center gap-1.5" data-deck-cards="">
            {borrowedLast(deckCards(deck)).map((slot) => (
              <DeckCell key={`${slot.i}:${slot.named}`} slot={slot} />
            ))}
          </div>
        </>
      )}
      {legacies.length > 0 && (
        <>
          <div class={SUB}>Legacy Umamusume</div>
          <div class="grid grid-cols-2 gap-2 max-sm:grid-cols-1">
            {legacies.map((leg, j) => (
              <Legacy key={leg.v.id} {...leg} j={j} />
            ))}
          </div>
        </>
      )}
      <div class={SUB}>Career</div>
      <div class="bg-gm-cell border-gm-line flex flex-wrap items-center gap-x-3.5 gap-y-1.5 rounded-md border px-2.5 py-1.25 font-bold">
        <span class="bg-gm-pane text-note rounded-full px-2.5 py-px">Career Record</span>
        <span>{`G1 races: ${races.length}`}</span>
        <span>{`Wins: ${d.wins.length}`}</span>
      </div>
      <div class="flex flex-wrap justify-end gap-2">
        <Button
          class="mr-auto"
          aria-expanded={racesShown.value}
          data-k="sim:races"
          aria-controls="ud-races"
          onClick={toggleRaces}
        >
          {racesShown.value ? 'Hide the races' : 'Show the races'}
        </Button>
        {winsButton}
      </div>
      <ol class="grid gap-1" hidden={!racesShown.value} id="ud-races">
        {entries.map((x) => (
          <li key={x.ix} class={RACE}>
            <span class="text-note text-gm-ink2">{x.turn == null ? 'Entered here' : when(x.turn)}</span>
            <span class="bg-gm-g1 text-caption rounded-sm px-1.5 py-px font-black text-white">G1</span>
            <span class="wrap-anywhere">{x.name}</span>
            {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (details.ts infoPane): the label reads the result; a role for it is recorded, not changed here */}
            <span class={x.won ? RESULT.won : RESULT.lost} aria-label={x.won ? 'won' : 'not won'}>
              {x.won ? '1st' : '–'}
            </span>
          </li>
        ))}
      </ol>
    </>
  );
}
