import type { RefObject } from 'preact';
import { batch } from '@preact/signals';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { RANKS } from '../data.ts';
import type { Scenario } from '../data.ts';
import { fields, setRank } from '../settings.ts';
import { LIST, artUrl, chooseScenario, goldNames, label, scenarioOf } from '../scenario.ts';
import { skillIconSrc, srcFor } from '../images.ts';
import { RadioPills } from './RadioPills.tsx';
import { ScenLogo } from './Images.tsx';
import { menuKeys, opensOnArrow, useDropdown, useFocusChecked } from './dropdown.ts';

// a guaranteed gold: its icon, named by its tooltip (the name as small text only when there is no icon), so every
// tile stays logo + one line
function GoldIcon({ group, name }: { group: number; name: string }) {
  const src = skillIconSrc(group, true);
  return src ? (
    <img class="sic" alt={name} loading="eager" src={src} title={name} />
  ) : (
    <span class="sgold" title={name}>
      {name}
    </span>
  );
}
// one scenario's tile in the grid: its logo, hint rate and guaranteed golds; the same hover text as the button shows
// for the chosen one
function ScenarioTile({ sc, checked, onPick }: { sc: Scenario; checked: boolean; onPick: (v: string) => void }) {
  const golds = goldNames(sc),
    art = artUrl(sc);
  return (
    <button
      type="button"
      class="stile"
      role="menuitemradio"
      data-v={sc.v}
      tabIndex={-1}
      style={art ? { '--scen-art': art } : undefined}
      aria-label={label(sc)}
      title={label(sc)}
      aria-checked={checked}
      onClick={() => onPick(sc.v)}
    >
      <ScenLogo sc={sc} />
      <span class="ssub">
        {sc.mult}
        {/* "(estimate)", shortened to "(est.)" when the tiles are narrow (phone) so the line stays one line */}
        {sc.est ? (
          <span class="sest">
            (<span class="sest-l">estimate</span>
            <abbr class="sest-s" title="estimate">
              est.
            </abbr>
            )
          </span>
        ) : null}
        {golds.length ? (
          // biome-ignore lint/a11y/useSemanticElements: a fieldset can't sit inside a button; the icons are named as one group
          <span class="sgolds" role="group" aria-label="Guaranteed in every run" title="Guaranteed in every run">
            {golds.map(([group, name]) => (
              <GoldIcon key={group} group={group} name={name} />
            ))}
          </span>
        ) : null}
      </span>
    </button>
  );
}
/** the scenario picker */
export function ScenarioPicker() {
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
  // the focus leaving the picker (the template's #scen-dd, around both) closes it
  const onFocusOut = (e: FocusEvent & { currentTarget: HTMLElement }) => {
    const to = e.relatedTarget as Node | null;
    if (to && !e.currentTarget.parentElement?.contains(to)) setOpen(false);
  };
  return (
    <>
      <div class="gctl" onFocusOut={onFocusOut}>
        <span id="x-scen-l">Scenario</span>
        <button
          class="scenbtn"
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
          <ScenLogo key={sc.v} sc={sc} />
        </button>
      </div>
      <div
        class={LIST.length === 5 ? 'scenpick five' : 'scenpick'}
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
    <span class="rk img" data-r={rank}>
      <img alt="" src={src} onError={() => setFailed(true)} />
    </span>
  ) : (
    <span class="rk" data-r={rank}>
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
    <svg class={`spk ${cls}`} viewBox="0 0 24 24" aria-hidden="true">
      <title>{name}</title>
      <path d={SPARK} fill="none" stroke={rim} stroke-width="3.6" stroke-linejoin="round" />
      <path d={SPARK} fill={fill} stroke={stroke} stroke-width="1.4" stroke-linejoin="round" />
    </svg>
  );
}
// the ? beside the label: what the assumed rank means (the star mix of the legacies' sparks at each)
function RankNote({ open, at }: { open: boolean; at: RefObject<HTMLSpanElement | null> }) {
  return (
    <span class="gpop" id="rank-note" role="tooltip" hidden={!open} ref={at}>
      <table class="rtab">
        <thead>
          <tr>
            <th>
              <Spark cls="w" name="Skill (white) sparks" rim="#e6e9ef" fill="#ffffff" stroke="#8a93a3" />
              <Spark cls="g" name="Unique (green) sparks" rim="#98d838" fill="#f5fde0" stroke="#68a808" />
              <span class="vh">Skill (white) and unique (green) sparks</span>
            </th>
            <th>B–S+</th>
            <th>SS</th>
          </tr>
        </thead>
        <tbody>
          <tr>
            <th>1★</th>
            <td>50%</td>
            <td>20%</td>
          </tr>
          <tr>
            <th>2★</th>
            <td>45%</td>
            <td>70%</td>
          </tr>
          <tr>
            <th>3★</th>
            <td>5%</td>
            <td>10%</td>
          </tr>
        </tbody>
      </table>
      <span class="rfoot">
        At SS your legacies' white and green sparks have more stars on average, so each inspiration passes them more
        often: about 23% more per roll.
      </span>
      <span class="rfoot">
        The planner doesn't work out the rank a deck would reach: pick the rank your veterans usually get.
      </span>
    </span>
  );
}
const RANK_PILLS = [
  {
    v: RANKS.below,
    dataV: RANKS.below,
    label: 'B to S+',
    title: 'Rank score 6,500–17,499',
    body: (
      <>
        <RankBadge rank="b" text="B" />
        <span class="rkdash">–</span>
        <RankBadge rank="splus" text="S+" />
      </>
    ),
  },
  { v: RANKS.ss, dataV: RANKS.ss, label: 'SS', title: 'Rank score 17,500+', body: <RankBadge rank="ss" text="SS" /> },
];
// The rank control: data-value carries the chosen star mix, each option's tooltip its rank score range. The ?'s note
// closes on Escape (the focus back on the ?) or a click elsewhere.
/** the lineage rank control */
export function RankControl() {
  const [noteOpen, setNoteOpen] = useState(false),
    question = useRef<HTMLButtonElement>(null),
    note = useRef<HTMLSpanElement>(null);
  const close = useRef(() => setNoteOpen(false)).current,
    inside = useRef([note]).current;
  useDropdown(noteOpen, close, question, inside);
  const rank = fields.rank.value;
  return (
    <>
      <span class="glab">
        <span id="rank-l">Assumed lineage rank</span>
        <button
          type="button"
          class="qinfo"
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
          <span aria-hidden="true">?</span>
        </button>
        <RankNote open={noteOpen} at={note} />
      </span>
      <RadioPills
        class="seg rankseg"
        id="rank"
        labelledBy="rank-l"
        dataValue={rank}
        pressed={false}
        pills={RANK_PILLS}
        value={rank}
        onPick={setRank}
      />
    </>
  );
}
