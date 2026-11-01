// ---------- the veteran analysis' store: Try a run's record in a signal ----------
// The comparison of the veteran this career leaves, one record handed out read-only. Each phase of the comparison
// (career/analysis/compare.ts `createComparison`, its run machine) replaces the record through a transition below, in
// one write, so a view never draws a phase half written. The comparison's staleness token and the baseline it caches
// are the machine's closure state: nothing draws them. Nothing here subscribes: the views read the signal, the machine
// reads it with peek(), and the entry tells it when her draft changes (`draftChanged`).
// The veteran herself as the player entered her (the draft) is the veterans store's (veterans/store.ts `draft`: a
// veteran-shaped record, frozen, edited by path through its `draftRef`); the transitions here set her with the record
// that goes with her (a new career on show, a roll, Add to veterans), in one batch.
// The folder: state.ts (this store), compare.ts (the comparison's run machine), view.tsx (Try a run's popup: her card's
// actions and the comparison panel), Details.tsx (her card, the game's Umamusume Details screen).
import { batch, signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { deepFreeze } from '../../store.ts';
import type { DeepReadonly } from '../../store.ts';
import { clearPlaces, draft, newVet, setDraft, setDraftParents } from '../../veterans/store.ts';
import type { Draft } from '../../veterans/store.ts';
import type { CareerReport, CareerSearch, PlanOption } from '../job.ts';

/** a stand-in while the quick comparison looks for the best career with her */
export interface PendingSearch {
  results: [];
  pending: true;
}
/** the figures a comparison ends on (both kinds) */
export interface Figures {
  /** she lands every target */
  full: boolean;
  /** expected careers to a fully looped parent with your veterans now, and with her too */
  v0: { now: number | null; withHer: number | null };
}
/** what the exact comparison adds to its figures */
export interface ExactFigures extends Figures {
  /** the best career with her as a parent */
  r: CareerSearch;
  /** with her is priced under today's plan's fit (Add to veterans then fits the plan to the pool with her, so its
   *  figures can differ); false: the refit itself (today's plan had no V or no fit) */
  fixedFit: boolean;
  /** the planner's next career once she is added */
  pickA: PlanOption | null;
  /** it is the same as without her */
  samePick: boolean;
  /** she is one of its parents */
  herParent: boolean;
  /** where her saving lands (compare.ts `savedWhere`) */
  where: string;
}
/** The veteran analysis: the comparison of the veteran this career leaves (the veterans store's `draft`). */
export interface Simulation extends Partial<Omit<ExactFigures, 'r'>> {
  /** none: no career on show; ready: no comparison yet */
  state: 'none' | 'ready' | 'busy' | 'done' | 'error';
  /** the comparison's search: the best career with her as a parent (`pending` while the quick comparison looks for it) */
  r?: CareerSearch | PendingSearch | null;
  /** today's best career, without her */
  baseline?: DeepReadonly<CareerReport> | null;
  error?: string | null;
  /** keys.ts `keyOf` the career she came from */
  forKey?: string;
  /** the running comparison is a roll's quick one */
  kind?: 'quick' | null;
  /** edited since the figures on show */
  stale?: boolean;
  /** an exact comparison runs behind a roll's quick figures */
  refining?: boolean;
  /** the quick figures say no saving, but their bound allows one: checking */
  provisional?: boolean;
  /** a roll's quick figures say it saves careers: the exact comparison starts once the roll has stood 0.8 s (shown as
   *  checking meanwhile) */
  standing?: boolean;
  /** the figures on show are the quick ones */
  quick?: boolean;
}
/** the record as the store hands it out */
export type SimulationView = DeepReadonly<Simulation>;

// ---------- the signal
const NONE: SimulationView = Object.freeze({ state: 'none' });
const record = signal<SimulationView>(NONE);
export const simulation: ReadonlySignal<SimulationView> = record;
// the next record: this one with `patch` written over it
const patch = (fields: Partial<SimulationView>) => {
  record.value = Object.freeze({ ...record.peek(), ...fields });
};

// ---------- the run machine's (career/run.ts, through the comparison's machine)
/** an answer shows a career: `next` is her fresh draft from it and its key (null: no career on show). The same career
 *  keeps the veteran entered for it (an edit elsewhere, or its undoing, leaves what the player typed), on her parents'
 *  records as they are now (an edit of one keeps the career); her comparison starts over on the new pool */
export function analysisShows(next: { draft: Draft; forKey: string } | null) {
  if (!next) {
    analysisClears();
    return;
  }
  const now = draft.peek(),
    keep = !!now && record.peek().forKey === next.forKey;
  batch(() => {
    record.value = Object.freeze({ state: 'ready', r: null, baseline: null, error: null, forKey: next.forKey });
    if (!keep) setDraft(next.draft);
    else if (JSON.stringify(now.parents) !== JSON.stringify(next.draft.parents)) setDraftParents(next.draft.parents);
  });
}
/** no answer: no career to analyse */
export function analysisClears() {
  batch(() => {
    record.value = NONE;
    setDraft(null);
  });
}
/** an edit elsewhere: a comparison in flight goes back to idle, or a roll's quick figures stand unchecked (an answer
 *  that stands keeps her); with nothing of the kind on show, no write */
export function comparisonStops() {
  const { state, refining, provisional, standing } = record.peek();
  if (state === 'busy')
    patch({ state: 'ready', r: null, kind: null, refining: false, provisional: false, standing: false });
  else if (refining || provisional || standing) patch({ refining: false, provisional: false, standing: false });
}

// ---------- the draft's
/** a roll redraws her: `next`, a fresh draft of the career on show (a random result of it, on her parents' records
 *  now); the figures on show go */
export function draftRolled(next: Draft) {
  if (!draft.peek()) throw new Error('a roll redraws the entered veteran');
  batch(() => {
    setDraft(next);
    patch({ r: null, stale: false });
  });
}
/** her draft changed (an edit by her editors, through the veterans store's `draftRef`; the entry's effect on `draft`
 *  calls this): figures on show are for the old one. A roll's or a new career's draft comes with the figures cleared
 *  in the same batch, so it marks nothing */
export function draftChanged() {
  const { r, stale } = record.peek();
  if (draft.peek() && r && !stale) patch({ stale: true });
}
/** she joins the veterans (Add to veterans): her record as entered, without what only her roll knows (`apt`, `bought`),
 *  the places back to the planner, and no draft (the next answer starts a new one even for the same career). Returns
 *  her uma (null: no draft) */
export function draftJoins() {
  const now = draft.peek();
  if (!now) return null;
  const { apt: _apt, bought: _bought, ...entered } = structuredClone(now) as Draft;
  batch(() => {
    setDraft(null);
    newVet(entered.chara, entered.sparks, { ...entered, src: 'entered', borrowed: false });
    clearPlaces();
  });
  return entered.chara;
}

// ---------- a comparison's phases (compare.ts)
/** a roll's quick comparison starts */
export const quickStarts = () =>
  patch({ state: 'busy', kind: 'quick', error: null, stale: false, provisional: false, standing: false });
/** the exact comparison starts: in place of the panel's figures, or (`refine`) behind a roll's quick figures, which
 *  stay on show (the roll's 0.8 s stand is over). It is not a quick one, whatever ran before it (a quick comparison
 *  cancelled leaves its `kind`; left, the next Compare would spare this job as a quick search) */
export const exactStarts = (refine: boolean) =>
  patch(
    refine
      ? { kind: null, refining: true, standing: false, error: null, stale: false }
      : { kind: null, state: 'busy', refining: false, provisional: false, standing: false, error: null, stale: false },
  );
/** today's best career, the comparisons' baseline, is known */
export const baselineFound = (baseline: DeepReadonly<CareerReport>) => patch({ baseline: deepFreeze(baseline) });
/** the plan had no saving for the roll: the exact comparison answers instead (it starts in the same batch) */
export const quickHandsOver = () => patch({ kind: null });
/** a roll's saving, from the plan's routes: its figures (no search figures yet); `standing`: it saves careers, so the
 *  exact comparison follows once the roll has stood */
export const quickFigures = (figures: Figures, standing: boolean) =>
  patch({
    ...deepFreeze(figures),
    r: Object.freeze({ results: [], pending: true }) as PendingSearch,
    fixedFit: true, // (y is the plan's own scoring, under its fit)
    pickA: null,
    samePick: false,
    herParent: false,
    where: '',
    quick: true,
    state: 'done',
    kind: null,
    standing,
  });
/** the quick comparison found the best career with her: its zero is checked by the exact comparison, provisional
 *  meanwhile */
export const bestFound = (r: CareerSearch) => patch({ r: deepFreeze(r), provisional: true });
/** the exact comparison's figures */
export const exactDone = (figures: ExactFigures) =>
  patch({
    ...deepFreeze(figures),
    quick: false,
    refining: false,
    provisional: false,
    standing: false,
    state: 'done',
  });
/** a comparison failed: `error` says why; `quick`: the roll's quick one (which is over) */
export const comparisonFails = (error: string, quick = false) =>
  patch(quick ? { error, state: 'error', kind: null } : { error, state: 'error' });
/** the check behind a roll's quick figures is over unanswered (it failed, or it is cancelled): the figures stand */
export const checkEnds = () => patch({ refining: false, provisional: false, standing: false });
/** a comparison in flight is cancelled: the panel back to idle */
export const comparisonCancelled = () => patch({ state: 'ready' });

// ---------- her card's tabs (Details.tsx): apart from the record, so a new draft or career keeps them, as a card's
// own state would not (it starts over whenever the card is drawn anew)
export type DetailsTab = 'skills' | 'insp' | 'info';
const tabOnShow = signal<DetailsTab>('insp'),
  racesOnShow = signal(false);
/** the tab on show (Inspiration first: it is where her sparks are entered) */
export const detailsTab: ReadonlySignal<DetailsTab> = tabOnShow;
/** Career Info's race-by-race record unfolded */
export const racesShown: ReadonlySignal<boolean> = racesOnShow;
export const showTab = (key: DetailsTab) => {
  tabOnShow.value = key;
};
export const toggleRaces = () => {
  racesOnShow.value = !racesOnShow.peek();
};
