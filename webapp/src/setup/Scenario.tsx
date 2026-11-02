import type { RefObject } from 'preact';
import { batch } from '@preact/signals';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import type { Scenario } from '../model/data.ts';
import { fields, setRank } from '../state/settings.ts';
import { LIST, artUrl, chooseScenario, goldNames, label, scenarioOf } from '../state/scenario.ts';
import { skillIconSrc, srcFor } from '../model/images.ts';
import { SegmentedControl } from '../ui/SegmentedControl.tsx';
import type { Segment } from '../ui/SegmentedControl.tsx';
import type { RankClass } from '../gen/solver.ts';
import { popover } from '../ui/Popover.tsx';
import { ScenLogo } from '../ui/Images.tsx';
import { menuKeys, opensOnArrow, useDropdown, useFocusChecked } from '../ui/dropdown.ts';
import { setLabel, setRow } from './Shell.tsx';
import type { Page } from './Shell.tsx';
import { cx, tv } from 'tailwind-variants/lite';

// a guaranteed gold: its icon, named by its tooltip (the name as small text only when there is no icon), so every
// tile stays logo + one line
function GoldIcon({ group, name }: { group: number; name: string }) {
  const src = skillIconSrc(group, true);
  return src ? (
    <img class="size-4 shrink-0" data-icon="skill" alt={name} loading="eager" src={src} title={name} />
  ) : (
    <span class="text-caption text-ink font-semibold" title={name}>
      {name}
    </span>
  );
}
// One scenario's tile in the grid: its logo at the left over the game's scenario art at the right (a fade between), its
// hint rate and guaranteed golds under it; the chosen one in the game's bright green rim; the same hover text as the
// button shows for the chosen one. Narrow (the menu 360 px or under: a phone) its padding and line tighten.
function ScenarioTile({ sc, checked, onPick }: { sc: Scenario; checked: boolean; onPick: (v: string) => void }) {
  const golds = goldNames(sc),
    art = artUrl(sc);
  return (
    <button
      type="button"
      class="art-stile border-gm-edge text-gm-ink hover:border-gm-bright2 focus-visible:border-gm-bright2 on:border-gm-bright2 on:ring-gm-bright2/25 on:border-2 on:ring-2 relative flex min-h-28 min-w-0 flex-col items-start justify-start gap-1 overflow-hidden rounded-lg border px-3 py-2 text-left @max-[360px]:px-2 @max-[360px]:py-1.5"
      role="menuitemradio"
      data-v={sc.v}
      tabIndex={-1}
      style={art ? { '--scen-art': art } : undefined}
      aria-label={label(sc)}
      title={label(sc)}
      aria-checked={checked}
      onClick={() => onPick(sc.v)}
    >
      <ScenLogo sc={sc} tile cls="h-16 w-[58%]" />
      <span class="text-note text-gm-ink2 leading-title @max-[360px]:text-caption inline-flex flex-wrap items-center justify-start gap-x-1.5 gap-y-0.5 font-bold @max-[360px]:flex-nowrap @max-[360px]:gap-x-1">
        {sc.mult}
        {/* "(estimate)", shortened to "(est.)" when the tiles are narrow (phone) so the line stays one line */}
        {sc.est ? (
          <span>
            (<span class="@max-[380px]:hidden">estimate</span>
            <abbr class="hidden no-underline @max-[380px]:inline" title="estimate">
              est.
            </abbr>
            )
          </span>
        ) : null}
        {golds.length ? (
          // biome-ignore lint/a11y/useSemanticElements: a fieldset can't sit inside a button; the icons are named as one group
          <span
            class="inline-flex items-center gap-0.75"
            role="group"
            aria-label="Guaranteed in every run"
            title="Guaranteed in every run"
          >
            {golds.map(([group, name]) => (
              <GoldIcon key={group} group={group} name={name} />
            ))}
          </span>
        ) : null}
      </span>
    </button>
  );
}
// The scenario button: the game's scenario select art behind its logo at the right, on a pale fade; it lifts under the
// pointer. The loop page's is the setup column's width and shorter (its setup fits a 900 px screen).
const scenButton = tv({
  base: 'border-gm-edge art-scen text-ink shadow-raised hover:shadow-lifted active:inset-shadow-pressed relative inline-grid max-w-full items-center justify-items-start overflow-hidden rounded-lg border px-2 py-0.5 transition duration-120 hover:-translate-y-px active:translate-y-0 active:shadow-none motion-reduce:transition-none motion-reduce:hover:translate-y-0 max-md:min-h-11',
  variants: { page: { career: 'min-h-11 w-52.5', loop: 'min-h-9.5 w-full' } },
  defaultVariants: { page: 'career' },
});
/** the scenario picker: a setup row of the page's (its label, the button), the menu hung under the whole list */
export function ScenarioPicker({ page }: { page: Page }) {
  const [open, setOpen] = useState(false),
    button = useRef<HTMLButtonElement>(null),
    menu = useRef<HTMLDivElement>(null);
  const close = useRef(() => setOpen(false)).current,
    inside = useRef([menu]).current; // the button and the grid: a click on the label closes it
  useDropdown(open, close, button, inside);
  useFocusChecked(open, menu);
  // after a pick, the focus goes back to the button once it shows the new scenario (its title is the game tooltip's)
  const refocus = useRef(false);
  useLayoutEffect(() => {
    if (!refocus.current) return;
    refocus.current = false;
    button.current?.focus();
  });
  const sc = scenarioOf(fields.scen.value);
  const pick = (v: string) => {
    refocus.current = true;
    batch(() => {
      if (v !== fields.scen.peek()) chooseScenario(v);
      setOpen(false);
    });
  };
  // the focus leaving the picker (the layout's #scen-dd, around both) closes it
  const onFocusOut = (e: FocusEvent & { currentTarget: HTMLElement }) => {
    const to = e.relatedTarget as Node | null;
    if (to && !e.currentTarget.parentElement?.contains(to)) setOpen(false);
  };
  return (
    <>
      <div class={setRow[page]} onFocusOut={onFocusOut}>
        <span class={setLabel.beside} id="x-scen-l">
          Scenario
        </span>
        <button
          class={scenButton({ page })}
          id="x-scen"
          ref={button}
          type="button"
          aria-haspopup="menu"
          aria-expanded={open}
          aria-controls="scen-menu"
          aria-label={`Scenario: ${sc.name}`}
          title={label(sc)}
          data-value={sc.v}
          style={{ '--scen-art': artUrl(sc) || 'none' }}
          onClick={() => setOpen(!open)}
          onKeyDown={opensOnArrow(open, setOpen)}
        >
          <ScenLogo key={sc.v} sc={sc} cls={page === 'loop' ? 'h-7.5 w-28' : 'h-9 w-28'} />
        </button>
      </div>
      <div
        class={popover({
          frame: 'panel',
          class: cx(
            '@container absolute inset-x-0 top-[calc(100%+4px)] z-20 grid auto-rows-fr gap-1.5 p-2',
            // five tiles: two rows of two and one centred, all one size
            LIST.length === 5 ? 'grid-cols-4 *:not-last:col-span-2 *:last:col-[2/span_2]' : 'grid-cols-2',
          ),
        })}
        id="scen-menu"
        role="menu"
        aria-labelledby="x-scen-l"
        ref={menu}
        hidden={!open}
        onKeyDown={(e) => menuKeys(e, 2)}
        onFocusOut={onFocusOut}
      >
        {LIST.map((s) => (
          <ScenarioTile key={s.v} sc={s} checked={s.v === sc.v} onPick={pick} />
        ))}
      </div>
    </>
  );
}

// a rank badge: the game's image, else (none, or it fails) its letters
function RankBadge({ rank, text }: { rank: string; text: string }) {
  const src = srcFor('ui', `rank_${rank}`),
    [failed, setFailed] = useState(false);
  return src && !failed ? (
    <span class="inline-flex h-6.5" data-r={rank}>
      <img
        class="group-not-on:opacity-45 group-not-on:saturate-50 h-6.5 w-auto"
        alt=""
        src={src}
        onError={() => setFailed(true)}
      />
    </span>
  ) : (
    <span class="font-bold" data-r={rank}>
      {text}
    </span>
  );
}
// a spark as the rank note's table heads it (white: skill sparks, green: unique sparks)
const SPARK = 'M12,1.5 Q13.3,10.7 22.5,12 Q13.3,13.3 12,22.5 Q10.7,13.3 1.5,12 Q10.7,10.7 12,1.5 Z';
function Spark({
  cls,
  name,
  rim,
  fill,
  stroke,
}: {
  cls: string;
  name: string;
  rim: string;
  fill: string;
  stroke: string;
}) {
  return (
    <svg class={cx('inline size-4.5 align-[-4px]', cls)} viewBox="0 0 24 24" aria-hidden="true">
      <title>{name}</title>
      <path d={SPARK} fill="none" stroke={rim} stroke-width="3.6" stroke-linejoin="round" />
      <path d={SPARK} fill={fill} stroke={stroke} stroke-width="1.4" stroke-linejoin="round" />
    </svg>
  );
}
// the rank note's table: its head cells (the first one the sparks), each row's star count, its shares
const RT = {
  headFirst: 'border-line border-b py-0.5 pr-2 pl-0 text-left font-semibold whitespace-nowrap',
  head: 'border-line border-b px-2 py-0.5 text-right font-bold whitespace-nowrap',
  stars: 'py-0.5 pr-2 pl-0 text-left font-semibold whitespace-nowrap',
  share: 'px-2 py-0.5 text-right whitespace-nowrap',
} as const;
const NOTE_FOOT = 'text-caption text-muted mt-1 block';
// the ? beside the label: what the assumed rank means (the star mix of the legacies' sparks at each); under the label,
// on a phone opening leftwards, inside the screen
const noteLook = popover({
  frame: 'plain',
  class:
    'text-note absolute top-[calc(100%+6px)] left-0 z-40 w-max max-w-[240px] px-2.5 py-2 leading-snug max-md:right-0 max-md:left-auto',
});
function RankNote({ open, at }: { open: boolean; at: RefObject<HTMLSpanElement | null> }) {
  return (
    <span class={noteLook} id="rank-note" role="tooltip" hidden={!open} ref={at}>
      <table class="text-note w-full border-separate border-spacing-0 tabular-nums">
        <thead>
          <tr>
            <th class={RT.headFirst}>
              <Spark cls="" name="Skill (white) sparks" rim="#e6e9ef" fill="#ffffff" stroke="#8a93a3" />
              <Spark cls="ml-px" name="Unique (green) sparks" rim="#98d838" fill="#f5fde0" stroke="#68a808" />
              <span class="sr-only">Skill (white) and unique (green) sparks</span>
            </th>
            <th class={RT.head}>B–S+</th>
            <th class={RT.head}>SS</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th class={RT.stars}>1★</th>
            <td class={RT.share}>50%</td>
            <td class={RT.share}>20%</td>
          </tr>
          <tr>
            <th class={RT.stars}>2★</th>
            <td class={RT.share}>45%</td>
            <td class={RT.share}>70%</td>
          </tr>
          <tr>
            <th class={RT.stars}>3★</th>
            <td class={RT.share}>5%</td>
            <td class={RT.share}>10%</td>
          </tr>
        </tbody>
      </table>
      <span class={NOTE_FOOT}>
        At SS your legacies' white and green sparks have more stars on average, so each inspiration passes them more
        often: about 23% more per roll.
      </span>
      <span class={NOTE_FOOT}>
        The planner doesn't work out the rank a deck would reach: pick the rank your veterans usually get.
      </span>
    </span>
  );
}
const RANK_OPTIONS: Segment<RankClass>[] = [
  {
    v: 'below_ss' as const,
    dataV: 'below_ss',
    label: 'B to S+',
    title: 'Rank score 6,500–17,499',
    body: (
      <>
        <RankBadge rank="b" text="B" />
        <span class="group-not-on:text-muted group-on:text-white">–</span>
        <RankBadge rank="splus" text="S+" />
      </>
    ),
  },
  {
    v: 'ss_and_up' as const,
    dataV: 'ss_and_up',
    label: 'SS',
    title: 'Rank score 17,500+',
    body: <RankBadge rank="ss" text="SS" />,
  },
];
// The rank control: data-value carries the chosen rank class, each option's tooltip its rank score range. The ?'s note
// closes on Escape (the focus back on the ?) or a click elsewhere.
/** the lineage rank control (its row is the page's layout's, #rank-ctl) */
export function RankControl({ page }: { page: Page }) {
  const [noteOpen, setNoteOpen] = useState(false),
    question = useRef<HTMLButtonElement>(null),
    note = useRef<HTMLSpanElement>(null);
  const close = useRef(() => setNoteOpen(false)).current,
    inside = useRef([note]).current;
  useDropdown(noteOpen, close, question, inside);
  const rank = fields.rank.value;
  return (
    <>
      <span class={setLabel.beside}>
        <span id="rank-l">Assumed lineage rank</span>
        <button
          type="button"
          class="group inline-grid place-items-center rounded-full p-0 max-md:-m-3.25 max-md:min-h-11 max-md:min-w-11"
          id="rank-q"
          ref={question}
          aria-expanded={noteOpen}
          aria-controls="rank-note"
          aria-label="What the assumed lineage rank means"
          title="What the assumed lineage rank means"
          onClick={(e) => {
            // as before the port: the click stops here, so a menu or popup open beside it stays open
            e.stopPropagation();
            setNoteOpen(!noteOpen);
          }}
        >
          <span
            class="border-accent bg-panel font-ui text-caption text-accent group-aria-expanded:bg-accent group-aria-expanded:text-panel inline-grid size-4.5 place-items-center rounded-full border leading-none font-bold"
            aria-hidden="true"
          >
            ?
          </span>
        </button>
        <RankNote open={noteOpen} at={note} />
      </span>
      <SegmentedControl
        kind="radio"
        shape="equal"
        size={page === 'loop' ? 'md' : 'lg'}
        id="rank"
        labelledBy="rank-l"
        dataValue={rank}
        options={RANK_OPTIONS}
        value={rank}
        onPick={setRank}
      />
    </>
  );
}
