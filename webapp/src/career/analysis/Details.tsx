// ---------- her card in Try a run: the game's "Umamusume Details" screen ----------
// Her portrait and outfit, her actions (the popup's: `actions`), her aptitude grades (Track / Distance / Style, the
// game's letters), then three tabs as in the game: Skills (the target skills she learned), Inspiration (her sparks as
// the game's spark bars, stars under each, then "Legacy Origin": her two parents' sparks) and Career Info (her support
// cards, her two legacies, the G1s she ran and won). No stats (the model has none).
// The tab on show and the races' fold are the analysis store's (they outlive the card). Her draft keeps one identity
// (career/analysis/state.ts says why), so her card's `spec` is made anew with every record (every edit), and the parts
// that read a signal (the tabs, the races' fold) draw her again from it rather than skip her as
// unchanged. Her sparks are edited through the store (`draftRef`).
import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { gameData } from '../../data.ts';
import { settings } from '../../settings.ts';
import { popOpener } from '../../popover.ts';
import { umaName } from '../../veterans/names.ts';
import { borrowedLast, deckCards, fitSparks } from '../../details.ts';
import type { DeckSlot } from '../../details.ts';
import { openSkill } from '../view/SkillDetails.tsx';
import { InspView } from '../../ui/InspView.tsx';
import { SegmentedControl } from '../../ui/SegmentedControl.tsx';
import { GRADE_LETTER, gradeAt } from '../../grades.ts';
import { CardAv, Grade, SkillIcon, UmaAv } from '../../ui/Images.tsx';
import type { DeepReadonly } from '../../store.ts';
import type { CareerReport } from '../job.ts';
import { draftRef } from '../../veterans/store.ts';
import type { Draft, VetRecord } from '../../veterans/store.ts';
import { detailsTab as tab, racesShown, showTab, toggleRaces } from './state.ts';
import type { DetailsTab } from './state.ts';
import { Button } from '../../ui/Button.tsx';
import { cx } from 'tailwind-variants/lite';

type Career = DeepReadonly<CareerReport>;

// aptitude rows; `at` indexes the solver's `apt` order: short, mile, middle, long, turf, dirt, front, pace, late, end
const APT_ROWS = [
  {
    label: 'Track',
    cells: [
      { at: 4, name: 'Turf' },
      { at: 5, name: 'Dirt' },
    ],
  },
  {
    label: 'Distance',
    cells: [
      { at: 0, name: 'Sprint' },
      { at: 1, name: 'Mile' },
      { at: 2, name: 'Medium' },
      { at: 3, name: 'Long' },
    ],
  },
  {
    label: 'Style',
    cells: [
      { at: 6, name: 'Front' },
      { at: 7, name: 'Pace' },
      { at: 8, name: 'Late' },
      { at: 9, name: 'End' },
    ],
  },
];
// a turn (0-71: 24 half-months a year) as "Classic Late Apr"
const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
const when = (turn: number) =>
  `${['Junior', 'Classic', 'Senior'][Math.floor(turn / 24)]} ${turn % 2 ? 'Late ' : 'Early '}${MONTHS[Math.floor((turn % 24) / 2)]}`;

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
/** her card: `actions` beside her portrait, `winsButton` (the G1 wins editor's) in Career Info */
export function Details({
  spec,
  actions,
  winsButton,
}: {
  spec: DetailsSpec;
  actions: ComponentChildren;
  winsButton: ComponentChildren;
}) {
  const { d, r } = spec,
    outfit = r.trainee.outfits[0]?.label || null,
    apt = d.apt || (r.aptitudes && 'start' in r.aptitudes ? r.aptitudes.start : null);
  return (
    <div class="ud">
      <div class="udtop">
        <div class="udpor">
          <UmaAv
            key={`${d.chara}:${d.outfit}`}
            uma={d.chara}
            outfit={d.outfit}
            size={96}
            label={outfit ? `${umaName(d.chara)} ${outfit}` : umaName(d.chara)}
          />
        </div>
        <div class="udmid">{actions}</div>
        <div class="udname">
          {outfit && <div class="udout">{/^\[.*\]$/.test(outfit) ? outfit : `[${outfit}]`}</div>}
          <div class="udnm">{umaName(d.chara)}</div>
        </div>
      </div>
      {apt && (
        // biome-ignore lint/a11y/useSemanticElements: today's markup (a grid of grade cells read as a table)
        <div class="udapt" role="table" aria-label="Her aptitudes">
          {APT_ROWS.map(({ label, cells }) => (
            // biome-ignore lint/a11y/useSemanticElements lint/a11y/useFocusableInteractive: (the table's rows, as above; a row is no control)
            <div key={label} class="udarow" role="row">
              {/* biome-ignore lint/a11y/useSemanticElements lint/a11y/useFocusableInteractive: (its header cell) */}
              <span class="udal" role="rowheader">
                {label}
              </span>
              {cells.map(({ at, name }) => {
                const grade = gradeAt(apt, at);
                return (
                  // biome-ignore lint/a11y/useSemanticElements: (its cells)
                  <span key={at} class="udac" role="cell" title={`${name} ${GRADE_LETTER[grade]}`}>
                    <span class="udan">{name}</span>
                    <Grade g={grade} />
                  </span>
                );
              })}
            </div>
          ))}
        </div>
      )}
      <DetailTabs spec={spec} winsButton={winsButton} />
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
// the tabs and their panes, all three built up front as in the game; once a switch is drawn, the spark names now on show
// shrink to fit (details.ts fitSparks)
function DetailTabs({ spec, winsButton }: { spec: DetailsSpec; winsButton: ComponentChildren }) {
  const list = useRef<HTMLDivElement>(null),
    onShow = tab.value;
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per tab drawn (`onShow` is a module signal's value, which Biome reads as constant)
  useLayoutEffect(() => {
    const card = list.current?.closest<HTMLElement>('.ud');
    if (card) fitSparks(card);
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
        meClass="simwho"
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
      <div class="udpanes" ref={list}>
        {DEFS.map(({ key }) => (
          <div
            key={key}
            id={`udpane-${key}`}
            class="udpane"
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

// Skills: every target skill she could acquire this career, as the game's skill tiles (name only): acquired in this
// result bright (gold tile for the gold version), the others greyed. A tile opens the game-style skill popup
// (view/SkillDetails.tsx): acquired or not, where it can come from, and how likely it is acquired and lands as a spark.
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
        class={cx('udsk', gold ? 'gold' : '', got ? '' : 'off')}
        data-k={`sim:sk${t.group}`}
        aria-haspopup="dialog"
        aria-label={`${name}: ${got ? `acquired${gold ? ' (gold)' : ''}` : 'not acquired'}. Details`}
        onClick={() => openSkill({ b, name, got: got as 'gold' | 'white' | null, group: t.group, goldIcon: goldShown })}
      >
        <SkillIcon group={t.group} gold={goldShown} />
        <span class="udskn">{name}</span>
      </button>,
    ];
  });
  if (!tiles.length) return <p class="udnone">No target skill can be acquired in this career.</p>;
  return (
    <>
      <div class="udskills">{tiles}</div>
      <p class="udfoot">{`${acquired} of ${tiles.length} target skills acquired. Tap a skill for where it comes from.`}</p>
    </>
  );
}

/** one support card of her deck (a click opens its panel: popover.ts, which reads the data-* and leaves the markup to
 *  this: its greying and aria-expanded are drawn here) */
function DeckCell({ slot }: { slot: DeckSlot }) {
  const { base, borrowed: bor, card: c, endAt } = slot, // (endAt: the popover's chain-end advice)
    off = !!c && !bor && settings.value.cardsOff[c.id] === false,
    ref = useRef<HTMLButtonElement>(null),
    open = popOpener.value !== null && popOpener.value === ref.current,
    named = base + (bor ? ' (borrowed)' : '');
  return (
    <button
      ref={ref}
      type="button"
      class={cx('udcard', 'dc', bor ? 'bor' : '', off ? 'off' : '')}
      title={named}
      data-cid={c ? String(c.id) : undefined}
      data-name={base}
      data-end={endAt ? String(endAt) : undefined}
      aria-label={`${named}. Card details`}
      aria-haspopup="dialog"
      aria-expanded={open}
    >
      <CardAv c={c || null} size={64} />
      {bor && <span class="udfr">Friends</span>}
    </button>
  );
}
// one of her two legacies: her portrait (it opens her node), her place and name, her two parents; the whole box opens
// her popup, as her portrait does
function Legacy({ v, label, open, j }: DetailsSpec['legacies'][number] & { j: number }) {
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents lint/a11y/noStaticElementInteractions: the box is the pointer's larger target; her portrait is the keyboard's button
    <div
      class={cx('udleg', j ? 'l2' : 'l1', 'pnclick')}
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest('button')) open();
      }}
    >
      <span class="udlegh">{`Legacy ${j + 1}`}</span>
      <div class="udlegt">
        <button
          type="button"
          class="udlegp"
          aria-haspopup="dialog"
          aria-label={`Legacy ${j + 1}: ${umaName(v.chara)} (${label}). Her sparks`}
          onClick={open}
        >
          <UmaAv key={`${v.chara}:${v.outfit}`} uma={v.chara} outfit={v.outfit} size={64} />
          <span class="udlegn">{`${label} ${umaName(v.chara)}`}</span>
          {v.borrowed && <span class="udguest">Guest</span>}
        </button>
        <div class="udleggp">
          {v.parents.map((g, k) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: her two parents, in their fixed places
            <div key={k} class="udleggx">
              <UmaAv
                key={`${g.chara}:${g.outfit}`}
                uma={g.chara || 0}
                outfit={g.outfit}
                size={40}
                label={umaName(g.chara)}
              />
              <span>{umaName(g.chara)}</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}
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
          <div class="udsub">Support Cards</div>
          <div class="udcards deck">
            {borrowedLast(deckCards(deck)).map((slot) => (
              <DeckCell key={`${slot.i}:${slot.named}`} slot={slot} />
            ))}
          </div>
        </>
      )}
      {legacies.length > 0 && (
        <>
          <div class="udsub">Legacy Umamusume</div>
          <div class="udlegs">
            {legacies.map((leg, j) => (
              <Legacy key={leg.v.id} {...leg} j={j} />
            ))}
          </div>
        </>
      )}
      <div class="udsub">Career</div>
      <div class="udrec">
        <span class="udrecl">Career Record</span>
        <span class="udstat">{`G1 races: ${races.length}`}</span>
        <span class="udstat">{`Wins: ${d.wins.length}`}</span>
      </div>
      <div class="udact">
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
      <ol class="udraces" hidden={!racesShown.value} id="ud-races">
        {entries.map((x) => (
          <li key={x.ix} class={cx('udrace', x.won ? 'won' : '')}>
            <span class="udrw">{x.turn == null ? 'Entered here' : when(x.turn)}</span>
            <span class="udgr">G1</span>
            <span class="udrn">{x.name}</span>
            {/* biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (details.ts infoPane): the label reads the result; a role for it is recorded, not changed here */}
            <span class="udres" aria-label={x.won ? 'won' : 'not won'}>
              {x.won ? '1st' : '–'}
            </span>
          </li>
        ))}
      </ol>
    </>
  );
}
