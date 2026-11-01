// The career setup's components: the Borrowed parent choice (into #bp: three choices, saved with the other settings; a
// change runs the planner again: the entry's effect on the setup), and the panel's fold with its summary.
import { computed, signal } from '@preact/signals';
import { useLayoutEffect } from 'preact/hooks';
import { $ } from '../../dom.ts';
import { ERA } from '../../saved.ts';
import { RANKS } from '../../data.ts';
import { settings, setBorrowPar } from '../../settings.ts';
import { scenarioOf } from '../../scenario.ts';
import { must } from '../../must.ts';
import { BORROW_CHOICES, borrowChoice } from '../job.ts';

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
  const now = borrowChoice(settings.value);
  return (
    <>
      {CHOICES.map(([v, label, title]) => (
        <button
          key={v}
          type="button"
          data-v={v}
          aria-pressed={v === now}
          title={title}
          onClick={() => {
            if (v !== borrowChoice(settings.value)) setBorrowPar(v);
          }}
        >
          {label}
        </button>
      ))}
    </>
  );
}

// ---------- the setup panel's fold
// The pink ribbon folds the setup panel to its ribbon (click it) and a one-line summary (SetupSummary, into #set-sum);
// the choice is kept in this browser, and until one is made it folds once there are veterans (the plan's answer, not
// the settings, is what a return visit is for).
const FOLD_KEY = 'llp:setup-folded';
const foldChosen = signal<boolean | null>(
  (() => {
    try {
      const f = localStorage.getItem(FOLD_KEY);
      return f === null ? null : f === '1';
    } catch {
      return null; // storage blocked: as if no fold was chosen
    }
  })(),
);
// the first veterans fold it, and it stays folded (a latch: the entry sets it when veterans arrive, `sawVeterans`)
const sawVets = signal(false);
const folded = computed(() => foldChosen.value ?? sawVets.value);
/** veterans are here: the panel folds until a fold is chosen (and stays folded should they go) */
export function sawVeterans() {
  sawVets.value = true;
}
/** the ribbon toggles it (all of it, its edges included); folded, the whole bar (its summary included) */
export function initFold() {
  const bar = must(document.querySelector('.rbbar'), 'the career page has the setup bar');
  bar.addEventListener('click', (e) => {
    if (!folded.peek() && !(e.target as HTMLElement).closest('#h-set')) return;
    foldChosen.value = !folded.peek();
    try {
      localStorage.setItem(FOLD_KEY, foldChosen.value ? '1' : '0');
    } catch {
      // storage blocked: the fold holds for this visit only
    }
  });
}
/** folded, the setup's summary: each setting as its label, light, and its value, bold, as the panel's rows read (the
 *  whole line for the screen reader; the borrowed parent only when it isn't the default). It marks the bar and its
 *  ribbon button folded or not, and shows itself only folded. */
export function SetupSummary() {
  const isFolded = folded.value,
    s = settings.value,
    rank = s.rank === RANKS.ss ? 'SS' : 'B–S+',
    era = ERA === 'gm' ? 'JP, Grand Masters' : 'Global',
    sc = scenarioOf(s.scen).name,
    bp: [string, string][] = borrowChoice(s) === 'allow' ? [] : [['Borrowed parent', BORROW_CHOICES[borrowChoice(s)]]];
  const items: [string, string][] = [
    ['Game data', era],
    ['Scenario', sc],
    ['Rank', rank],
    ['Targets', String(s.targets.length)],
    ...bp,
  ];
  const label = [
    era,
    sc,
    `rank ${rank}`,
    `${s.targets.length} target skills`,
    ...bp.map(([k, v]) => `${k}: ${v}`),
  ].join(' · ');
  // (the bar, its ribbon and this summary's own box are the template's)
  useLayoutEffect(() => {
    must(document.querySelector('.rbbar'), 'the career page has the setup bar').classList.toggle('folded', isFolded);
    const b = $('set-fold'),
      sum = $('set-sum');
    b.setAttribute('aria-expanded', String(!isFolded));
    b.title = isFolded ? 'Show the career setup' : 'Fold the career setup';
    sum.hidden = !isFolded;
    sum.setAttribute('aria-label', label);
  });
  return (
    <>
      {items.map(([k, v]) => (
        <span key={k} class="sumkv" aria-hidden="true">
          <span class="sumk">{k}</span>
          <b>{v}</b>
        </span>
      ))}
    </>
  );
}
