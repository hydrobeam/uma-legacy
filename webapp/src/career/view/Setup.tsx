// The career setup's components: the Borrowed parent choice (#bp: three choices, saved with the other settings; a
// change runs the planner again: the entry's effect on the setup), and the panel's fold with its summary.
import { computed, signal } from '@preact/signals';
import { ERA } from '../../state/saved.ts';
import { read, update } from '../../state/storage.ts';
import { settings, setBorrowPar } from '../../state/settings.ts';
import { scenarioOf } from '../../state/scenario.ts';
import { ERA_NAMES } from '../../model/eras.ts';
import { BORROW_CHOICES, borrowChoice } from '../job.ts';
import { SegmentedControl } from '../../ui/SegmentedControl.tsx';

const CHOICES = [
  ['allow', 'Allowed', 'This career may borrow one parent, and so may the careers after it'],
  [
    'later',
    'None left today',
    'Out of borrows for today: this career uses two of your veterans, and the plan still counts on borrowing in the careers after it',
  ],
  ['never', 'Never', "No borrowed parents, in this career or the plan's later ones"],
] as const;
export function BorrowChoice() {
  return (
    <SegmentedControl
      kind="toggle"
      shape="fill"
      id="bp"
      labelledBy="bp-l"
      value={borrowChoice(settings.value)}
      onPick={(v) => {
        if (v !== borrowChoice(settings.value)) setBorrowPar(v);
      }}
      options={CHOICES.map(([v, label, title]) => ({ v, dataV: v, title, body: label }))}
    />
  );
}

// ---------- the setup panel's fold
// The pink ribbon folds the setup panel to its ribbon (click it) and a one-line summary (SetupSummary, in #set-sum);
// the choice is kept in this browser, and until one is made it folds once there are veterans (the plan's answer, not
// the settings, is what a return visit is for). The layout (career/Layout.tsx) draws the bar, its ribbon and the
// summary's box from `folded` and `summaryLabel`.
const FOLD_KEY = 'llp:setup-folded';
const kept = read(FOLD_KEY); // (storage blocked: as if no fold was chosen)
const foldChosen = signal<boolean | null>(kept === null ? null : kept === '1');
// the first veterans fold it, and it stays folded (a latch: the entry sets it when veterans arrive, `sawVeterans`)
const sawVets = signal(false);
/** the setup panel is folded */
export const folded = computed(() => foldChosen.value ?? sawVets.value);
/** veterans are here: the panel folds until a fold is chosen (and stays folded should they go) */
export function sawVeterans() {
  sawVets.value = true;
}
/** a click on the setup bar: on its ribbon (all of it, its edges included) it toggles the fold; folded, anywhere on the
 *  bar (its summary included) unfolds it */
export function foldClick(e: MouseEvent) {
  if (!folded.peek() && !(e.target as HTMLElement).closest('#h-set')) return;
  foldChosen.value = !folded.peek();
  update(FOLD_KEY, () => (foldChosen.value ? '1' : '0')); // (storage blocked: the fold holds for this visit only)
}
// each setting as its label and its value, as the panel's rows read (the borrowed parent only when it isn't the default)
const summaryItems = computed((): [string, string][] => {
  const s = settings.value,
    rank = s.rank === 'ss_and_up' ? 'SS' : 'B–S+',
    bp: [string, string][] = borrowChoice(s) === 'allow' ? [] : [['Borrowed parent', BORROW_CHOICES[borrowChoice(s)]]];
  return [
    ['Game data', ERA_NAMES[ERA]],
    ['Scenario', scenarioOf(s.scen).name],
    ['Rank', rank],
    ['Targets', String(s.targets.length)],
    ...bp,
  ];
});
/** the summary, whole, for the screen reader */
export const summaryLabel = computed(() =>
  summaryItems.value
    .map(([k, v]) =>
      k === 'Game data' || k === 'Scenario'
        ? v
        : k === 'Rank'
          ? `rank ${v}`
          : k === 'Targets'
            ? `${v} target skills`
            : `${k}: ${v}`,
    )
    .join(' · '),
);
/** folded, the setup's summary: each setting as its label, light, and its value, bold */
export function SetupSummary() {
  return (
    <>
      {summaryItems.value.map(([k, v]) => (
        <span key={k} aria-hidden="true">
          <span class="text-caption text-gm-ink2 mr-1.5 font-medium">{k}</span>
          <b class="text-ui text-gm-ink font-bold">{v}</b>
        </span>
      ))}
    </>
  );
}
