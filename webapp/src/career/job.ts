// The career job: built from the settings, the veterans and the places (`buildJob`), the Use this choice it carries,
// what reads the job, the places and the career on show, and its run on the solver (`runJob`). The solver's career job
// and its reports are typed here.
import { computed } from '@preact/signals-core';
import { $ } from '../dom.ts';
import { careerTables, gameData } from '../data.ts';
import { settings } from '../settings.ts';
import { run as engineRun, stateKey } from '../engine.ts';
import { goldGiven, isOwned, jobTargets, ownedFields, scenarioFields, starDist } from '../job.ts';
import { srcFor } from '../images.ts';
import { vets, places, chosen, setChosen, vetById, addVets, legaciesJob } from '../veterans/store.ts';
import { umaName, targetGroups } from '../veterans/names.ts';
import { answer, answerCareer, reference } from './state.ts';
import { must, orNull } from '../must.ts';
import { planKey } from './keys.ts';
import { pickOutfit, outfitNote } from '../roster.ts';
import { unprovenWhy } from '../proofs.ts';
import type { ScheduleRow } from '../agenda.ts';
import type { ResultDeck } from '../loop/types.ts';
import type { SettingsView } from '../settings.ts';
import type { PlanProgress, SolverAnswer } from '../solver.ts';
import type { JobLegacy, LineageNode, PlacesRecord, Sparks, Veteran, VetRecord } from '../veterans/store.ts';
import type { DeepReadonly } from '../store.ts';
import type { Status } from './state.ts';

// the places as a key: the trainee and the two parents (not how they were set, `fromTable`)
export const placesKey = (p: PlacesRecord = places.value): string =>
  JSON.stringify({ trainee: p.trainee, parents: p.parents });
/** The career job (`buildJob`; the solver reads it as `SkillJob` + `DeckJob` + `CareerJob`, web.rs); a plan job adds
 *  `next` and `no_reports`, a comparison `compare`. (A type, not an interface: it is passed as a SolverJob, whose index
 *  signature an interface does not meet.) */
export type CareerJob = {
  cmd: 'career';
  targets: { group: number }[];
  /** umas not owned (by character id) */
  exclude: number[];
  /** outfits not owned */
  exclude_outfits: number[];
  extra: number[][];
  owned_cards: number[];
  borrow: boolean;
  /** the lineage rank's chances of 1, 2 and 3 stars */
  star_dist: number[];
  scenario: string;
  /** R cards (not pals) left out */
  exclude_cards?: number[];
  /** every veteran, by legacy index (`legaciesJob`) */
  legacies: readonly JobLegacy[];
  /** the trainee chosen (none: the planner picks) */
  trainees: number[];
  /** the parents chosen, by legacy index */
  fix: number[];
  per_trainee: boolean;
  /** careers to return */
  top: number;
  /** never set by `buildJob`, and the solver reads no such field: a leftover of the rollout planner that `baseOf`,
   *  `planKey` and the plan job still clear */
  rollout?: undefined;
  /** likewise */
  p_now?: undefined;
  /** left out at 'allow' */
  borrowed_parent?: 'later' | 'never';
  /** a plan job: its settings (`roll`: a roll's spark stars, per target; `fit`: another plan's continuations, taken as
   *  given instead of fitted to this pool; `robust`: the next step of the kept plan's robustness check, `checkPhase`;
   *  `pair_search`: that check ends with the within-trainee pair search, sent on the check's jobs only) */
  next?: { roll?: number[]; fit?: PlanFit; robust?: 'next'; pair_search?: boolean };
  /** a plan job: no career reports, `next` only */
  no_reports?: boolean;
  /** a comparison's job */
  compare?: boolean;
  /** one career reported as it is, no deck search (career/given.rs: its `career` id, on the trainee and both parents
   *  fixed): Use this on a builder's other career or the pair search's (`chosen`) */
  career?: string;
};
/** One career in a report (career/outcomes.rs `report`): the trainee and her parents, by legacy index. */
export interface CareerReport {
  trainee: CareerTrainee;
  parents: [number, number];
  /** the career's identity (`kept::career_id`: her uma, parents in order and deck) */
  career?: string;
  /** her six legacies */
  slots: CareerSlot[];
  /** per target, in the job's order */
  targets: CareerTarget[];
  /** the chance she lands every target */
  p_all: number;
  /** the chance she lands all but at most one */
  p_minus1: number;
  /** the targets she lands, expected */
  expected: number;
  bound: number;
  deck: ResultDeck;
  /** per pink the six legacies carry: the chance at least one of the two inheritance events passes it on, and its
   *  carriers */
  pinks: { group: number; q: number; carriers: number }[];
  /** her aptitude grades (codes 1..8 = G..S) and her veteran's pink spark (pink.ts); `fire`: each legacy pink that can
   *  fire at an April inspiration, its aptitude and chance (outcomes.rs `fire_json`) */
  aptitudes: {
    base: number[];
    start: number[];
    fire: { group: number; apt: number; p: number; slot: number; stars: number }[];
    pink_gen: { group: number; p: number }[];
  };
  /** every G1 she could enter, in turn order (the search's best career and the plan's pick only) */
  schedule: CareerScheduleRow[] | null;
  /** [mask]: the chance of landing exactly the targets in mask (as `schedule`, up to 12 targets) */
  outcome_sets: number[] | null;
  /** her G1 wins, expected (as `schedule`) */
  g1_wins: number | null;
  /** a plan option: both parents are full veterans */
  looped?: boolean;
}
/** A row of a career's schedule (career/schedule.rs): the loop schedule's row, one cell (hers), with the race's turn.
 */
export type CareerScheduleRow = ScheduleRow & CareerScheduleFields;
export interface CareerScheduleFields {
  /** 0..71 */
  turn: number;
  /** the race's full name (`race`: as the chart shows it) */
  name: string;
  /** she runs it (false: she skips it) */
  run: boolean;
  skipped: boolean;
  /** what she does that turn instead (a race's name, or "rest") */
  instead: string;
  /** her win chance (a fraction; null when she skips it) */
  p: number | null;
  /** a goal race of her scenario */
  goal: boolean;
}
export interface CareerTrainee {
  id: number;
  name: string;
  /** her outfits that cover a target; the page puts the one it names first (`oneOutfit`) */
  outfits: { id: number; label: string }[];
  covers: { group: number; level: number }[];
  /** set by the page: no outfit covers a target, any would do */
  anyOutfit?: boolean;
  /** set by the page: what her popup says about the outfit */
  outfitNote?: string;
}
export interface CareerSlot {
  kind: 'parent' | 'grandparent';
  /** the parent's legacy index */
  legacy: number;
  /** a grandparent: which of the parent's two */
  gp: number | null;
  chara: number;
  name: string;
  /** relation points */
  rel: number;
  /** shared G1 wins, as points */
  race: number;
  /** affinity: rel + race */
  aff: number;
}
export interface CareerTarget {
  group: number;
  /** the chance inheritance gives it */
  q: number;
  /** legacies with its spark */
  carriers: number;
  /** the chance she lands it */
  p: number;
  /** a gold the scenario hands out (`buy.scenario`) */
  gold: boolean;
  buy: CareerBuy;
}
/** How she buys a target and lands it as a spark (outcomes.rs `report`): the chance she buys it as gold and as white,
 *  lands each, and the sources; a gold the scenario hands out has `scenario` and the first four only. */
export interface CareerBuy {
  gold: number;
  white: number;
  land_gold: number;
  land_white: number;
  /** the deck hints its gold */
  deck_gold?: number;
  /** the deck hints its white, given it hints no gold */
  deck_white?: number;
  /** her own story events give it */
  own_event?: number;
  /** inspiration gives it */
  inherit?: number;
  /** her outfit's own skill: none, the white, the gold */
  outfit?: 0 | 1 | 2;
  scenario?: boolean;
}
/** One career of the plan's table (output.rs): a career report with its place in the plan. */
export type PlanOption = CareerReport & PlanOptionFields;
export interface PlanOptionFields {
  /** her place in today's estimate (`roleText`) */
  role: 'plan' | 'finish' | 'loop' | 'candidate' | 'builder';
  /** the search's best career */
  best_odds: boolean;
  /** the plan's pick */
  chosen: boolean;
  /** both parents are full veterans */
  looped: boolean;
  /** scored from her own career search */
  exact: boolean;
  /** false: her career search was cut short, or her deck class breached its assumed cap */
  proven?: boolean;
  /** the named assumptions her proven career rests on (the status line states the plan job's, `planCaveat`; the table
   *  adds none) */
  assumes?: string[];
  stage_cost: number | null;
  /** the careers she saves toward a fully looped parent, net */
  index: number | null;
  saving: number | null;
  /** the chance her result lowers the careers to a fully looped parent */
  lift: number | null;
  /** over her results that lower it, the stage candidate (uma id) whose side the route taken lifts most often and that
   *  chance, and the chance the route has her own full veteran as a side (`roleText`); null where the row was not
   *  scored */
  lifts_side?: { uma: number | null; chance: number; own: number } | null;
  /** the table's order */
  score: number | null;
  /** the pair search's recommendation (`pairSearchRow`): run as it is, its deck kept */
  pair_search?: boolean;
  /** its Saves per run in the pair search */
  pair_search_score?: number | null;
}
/** A row of the Career choices table: a plan option, or a career of the search (without the plan's fields). */
export type OptionRow = CareerReport & Partial<PlanOptionFields>;
/** A parent on the plan's route (`trio_report` in plan/output.rs): her step to an all-targets veteran. */
export interface RouteMember {
  /** her name */
  trainee: string | null;
  /** the chance of landing every target today of the career her fresh side is from */
  p: number;
  /** that career's parents, by legacy index (null for a real full veteran) */
  parents: [number, number] | null;
  /** that career is another of her top careers, not her best today (`fresh_lineages`) */
  alternative: boolean;
  /** a search with both its parents fixed proved that career's odds */
  proven: boolean;
  /** the odds her continuation is anchored at */
  p0: number;
  /** she already has a full veteran (a poorer parent than a fresh one) */
  has_full_veteran: boolean;
  /** the route uses her existing full veteran: her step costs no careers */
  real: boolean;
  /** her effective ceiling, the odds her lifts approach (null when lam is 0: no building) */
  p_inf: number | null;
  /** the chance a building career lifts her odds, before it falls with the remaining gap */
  lam: number;
  /** the share of the remaining log-odds gap to p_inf that each lift closes */
  phi: number;
  /** the expected careers she spends building lifts (each building career lifts her odds only by chance) before
   *  rerunning her best career; 0 when she does not build */
  building_careers: number;
  /** her odds once building stops */
  p_switch: number;
  /** the career id of the row her lifts are fitted from (null where she grinds, her continuation was given, or her step
   *  is her cap's) */
  builder?: string | null;
  /** her step's expected careers (0 when `real`) */
  careers: number;
}
/** The plan's report (the solver's `next`, career/plan/output.rs), or its refusal (`error`). */
export interface PlanReport {
  error?: string;
  /** the plan's pick */
  choice: CareerReport | null;
  /** by score, best first */
  options: PlanOption[];
  /** expected careers to a fully looped parent with the veterans now */
  v0: number | null;
  /** today's cheapest route: its steps' trainees by name, their careers, and A's and B's sides (`side_key` in
   *  career/plan/output.rs: "real:i" her veteran i, "current:i,j" her best career now, on veterans i and j, "alt:i,j"
   *  another career of hers, on veterans i and j) */
  v0_plan: { a: string | null; b: string | null; c: string | null; careers: number[]; sides?: string[] } | null;
  /** the cheapest routes, cheapest first; `near`: within 2% of V's (assumed), so about as cheap */
  trios?: {
    a: string | null;
    b: string | null;
    c: string | null;
    careers: number[];
    sides?: string[];
    value: number;
    near: boolean;
  }[];
  /** the route's A and B, in its order (empty without a route) */
  members?: RouteMember[];
  /** the career a roll is of (`next.roll`) */
  outcomes: { trainee: number; parents: [number, number]; career?: string; uncv: number[]; fixed: number[] } | null;
  /** a roll's saving over the plan's routes (`rollReq`) */
  roll?: { y: number };
  /** set by the page: the robustness check could not finish (`checkPhase`) */
  check_unavailable?: boolean;
  /** false: the plan's memory budget left out a route no batch could hold or a partner setup (career/plan/output.rs),
   *  so its savings are lower bounds and its pick may not be the model's */
  scoring_complete?: boolean;
  /** the robustness check on the pick (plan/robust.rs), set by the page from its follow-up jobs (`checkPhase`): the
   *  pick it is of, the variants left, whether every tested variant scores the pick highest (`stable`), the careers
   *  (`keyOf`) the variants score highest (`contenders`), whether they disagree (`pick_varies`: the pick varies under
   *  these variants, a sensitivity, not a tie); the within-trainee pair search once it has run (`pairs`: null before,
   *  absent from a solver without it); `error` and `unavailable`: the worker kept no plan to check (robust.rs) */
  robust?: {
    plan: string;
    pending: number;
    stable: boolean;
    contenders: string[];
    pick_varies?: boolean;
    pairs?: PairSearch | null;
    scoring_complete?: boolean;
    error?: string;
    unavailable?: boolean;
  };
  /** the stage candidates and their continuations the plan was fitted to: a comparison passes it on unchanged
   *  (`next.fit`), so the pool with her is priced the same way */
  fit?: PlanFit;
  /** set by the page: why some of its career searches proved nothing (`unprovenWhy`) */
  unproven_why?: string[];
  /** set by the page: the named assumptions any search of the plan job rests on (the job's `job_assumes`, kept with the
   *  plan; `planCaveat`) */
  job_assumes?: string[];
}
/** The within-trainee pair search (plan/robust.rs `pair_search`), one entry per trainee it ran for: the pick's, then
 *  each one a per-uma scenario scores highest. `shown`: the career she is weighed against (`keyOf`): for the pick's
 *  trainee the pick, for another her best-scoring shown career; `best_parents`: the legacy indices of her best pair by
 *  its best deck found so far; `score`: that pair's Saves per run after its deck was searched exactly;
 *  `margin_over_shown`: that less her shown career's; `margin_over_other_pairs`: that less the best other pair
 *  evaluated (null where there is none; neither margin certifies the best pair); `corner_margins`: the margin over her
 *  shown career under each building-curve corner; `scale_margins`: at her own odds x0.76 and x1.32 (both careers
 *  alike); `pairs_scored`: the pairs she was scored on, the best among them; `pairs_runnable`: the pairs she can run;
 *  `pairs_truncated`: the bound (`pair_bound` per uma) left some of them unscored, the rest by their all-target
 *  ceiling. `reason`: why she was searched ("pick", "scenario": a per-uma scenario's pick, "top": among the 4 best by
 *  score); `better_parents`: her best beats her shown career on another pair. `shared_coverage`: the umas without their
 *  own search the best pairs were tried for, and the careers so scored. `best`: the best (trainee, pair) by score over
 *  both, with its margin over the pick's trainee's shown career and, where it is not the pick, `versus_pick`: its
 *  margins over the pick (`career`) as fitted and in the two per-uma scenarios, and the scenarios that reverse the
 *  fitted margin's sign (`varies_under`: a sensitivity, deciding nothing). `recommendation`: the career to run, the
 *  higher-scoring of the pick and `best` as fitted (`source` "cross_trainee", "pick_trainee_pair" or "plan_pick"), with
 *  its pair-search `score` and `report` (a career report, its deck with it: the table's row for it, `pairSearchRow`).
 */
export type PairSearch = {
  trainees: {
    trainee: string;
    reason?: string;
    shown: string;
    best_parents: [number, number];
    score: number;
    margin_over_shown: number;
    better_parents: boolean;
    margin_over_other_pairs: number | null;
    corner_margins?: number[];
    scale_margins?: number[];
    pairs_scored: number;
    pairs_runnable?: number;
    pairs_truncated?: boolean;
  }[];
  shared_coverage?: { trainees: number; careers: number; trainees_truncated?: number };
  pair_bound?: number;
  best?: {
    trainee: string;
    career: string;
    best_parents: [number, number];
    score: number;
    margin_over_pick_trainee_shown: number | null;
    versus_pick?: {
      career: string;
      margins: { nominal: number; pick_down: number; pick_up: number };
      varies_under: string[];
    };
  } | null;
  recommendation?: { career: string; source: string; score?: number | null; report?: CareerReport };
};
/** A plan's continuation fit, one entry per stage candidate (`[chara, lam, phi, pinf, ...]`, as the solver writes it;
 *  the page never reads inside it). */
export type PlanFit = number[][];
/** The career search's report (career/run.rs `search_output`); a refusal is a SolverRefusal (solver.ts). */
export interface CareerSearch {
  /** the best careers, best first */
  results: CareerReport[];
  /** careers it weighed */
  candidates: number;
  /** the best odds' order is certified */
  proven_top: boolean;
  /** false: a search of the job was cut short, or certified at a lower deck cap only */
  complete?: boolean;
  /** deck bands too large to check in full (each then certified at a lower cap) */
  band_overflows?: number;
  /** per deck class past its assumed cap: [class, ratio, cap] */
  cap_exceeded?: [number, number, number][];
  /** the deck searches rerun without the cap after a breach: those that proved nothing, and breaches left unanswered */
  cap_rerun?: { searches: number; unproven: number; unanswered: number };
  /** why the main search is not complete (`work_cap`, `band_overflow`, `cap_breach`) */
  unproven?: string[];
  /** why any search of the job is not complete */
  job_unproven?: string[];
  /** the named assumptions the main search's proof rests on (`two_feeder_cap`, `multi_feeder_cap`): still a proof
   *  (proofs.ts `assumedText`) */
  assumes?: string[];
  /** the named assumptions any search of the job rests on */
  job_assumes?: string[];
  /** capped deck searches of the job that scored a deck past its class's cap */
  cap_breaches?: number;
  /** trainees (chara ids) whose deck class breached its cap, here or before */
  cap_breached?: number[];
  /** a plan job's plan */
  next?: PlanReport;
  /** set by the page: a quick comparison's stand-in while the search for her career runs */
  pending?: boolean;
}
// ---------- the job ----------
/** her place in the pool (-1: none) */
export const idxOf = (id: string | null): number => vets.value.findIndex((v) => v.id === id);
/** Settings, veterans and places that make no job: the status line's kind, its text and its button(s). */
export class Blocked extends Error {
  declare kind: 'empty' | 'error';
  declare act: Status['act'] | undefined;
  constructor(kind: 'empty' | 'error', text: string, act?: Status['act']) {
    super(text);
    this.kind = kind;
    this.act = act;
  }
}
// the borrowed-parent choice (Career setup): 'allow' (one per career, now and later), 'later' (none in this career; the
// plan's later careers still borrow), 'never'; the job's `borrowed_parent`, left out at 'allow' (the solver's default)
export const BORROW_CHOICES = { allow: 'Allowed', later: 'None left today', never: 'Never' };
export const borrowChoice = (s: SettingsView): keyof typeof BORROW_CHOICES =>
  Object.hasOwn(BORROW_CHOICES, s.borrowPar) ? (s.borrowPar as keyof typeof BORROW_CHOICES) : 'allow';
// the career job for settings `s` and the veterans and places (throws Blocked when they make none)
export function buildJob(s: SettingsView): CareerJob {
  if (!s.targets.length) throw new Blocked('empty', 'No target skills: add them with Target skills above.');
  const plan = places.value,
    pool = vets.value;
  const fixed = (plan.parents.filter(Boolean) as string[]).map(vetById);
  if (pool.length < 2 || !pool.some((v) => !v.borrowed)) {
    const importVets = { label: 'Import veterans', run: () => $('v-toggle').click() };
    throw new Blocked(
      'empty',
      'Choose both parents, or import your veterans and let the planner pick.',
      !pool.some((v) => !listed(v))
        ? [importVets, { label: 'Try sample veterans', run: () => addVets(sampleVets()) }]
        : importVets,
    );
  }
  // the parent places' rules (`pairBlock`): the pair's first, each parent in turn, then the trainee's
  const [first, second] = fixed;
  const checks = [
    { trainee: 0, v: first, other: second },
    { trainee: 0, v: second, other: first },
    { trainee: plan.trainee, v: first },
    { trainee: plan.trainee, v: second },
  ];
  for (const { trainee, v, other } of checks) {
    if (!v) continue; // (no parent in that place: nothing to check)
    const k = pairBlock(trainee, v, other, s);
    if (k === 'same')
      throw new Blocked('error', `Both parents are ${umaName(v.chara)}: a career takes two different umas.`);
    if (k === 'both') throw new Blocked('error', 'Both parents are borrowed: a career can borrow one.');
    if (k === 'setting')
      throw new Blocked(
        'error',
        `${umaName(v.chara)} is borrowed, and Borrowed parent is set to ${BORROW_CHOICES[borrowChoice(s)]}: choose one of your veterans, or allow a borrowed parent.`,
      );
    if (k === 'trainee')
      throw new Blocked('error', `${umaName(plan.trainee)} can't be her own parent: choose another trainee or parent.`);
  }
  const exclude = gameData()
    .umas.filter((u) => !isOwned(u, s) && u.id !== plan.trainee)
    .map((u) => u.id);
  if (!plan.trainee && exclude.length >= gameData().umas.length)
    throw new Blocked('empty', 'No uma is owned: mark yours with Umas above.');
  const allPlaced = plan.trainee && fixed.length === 2;
  // a place left open: the K best careers by odds (their trainees are among the plan's stage candidates; the plan
  // scores every owned uma, `planPhase`)
  const job: CareerJob = {
    cmd: 'career',
    targets: jobTargets(s),
    ...ownedFields(s, exclude),
    star_dist: starDist(s),
    ...scenarioFields(s),
    legacies: legaciesJob(),
    trainees: plan.trainee ? [plan.trainee] : [],
    fix: plan.parents.filter(Boolean).map(idxOf),
    per_trainee: false,
    top: allPlaced ? 1 : ROLL_K,
  };
  const borrowed = borrowChoice(s);
  if (borrowed !== 'allow') job.borrowed_parent = borrowed;
  if (allPlaced && chosenApplies(job)) job.career = must(chosen.value, 'chosenApplies: a choice is saved').id;
  return job;
}
// the career Use this chose with its deck (veterans/store.ts `chosen`, saved with the plan): a builder's other career (an option
// with `alternative`: a search on its places would give her best deck there instead, but for the pick, searched on its
// pair before it is shown; listed with the plan's `builder_careers` above 1, its default 1, which the page keeps: none by
// default) or the pair search's recommendation (`pair_search`). It is sent only while it applies (`chosenApplies`): its
// places, the setup it was chosen on (`chosenSetup`: the job's settings, its targets and cards among them, its two
// parents' records and the solver's and data's fingerprints, engine.ts `stateKey`) and its parents' places in the veteran list (the id names them by index) all as they were. One
// saved before a reload that no longer applies is dropped, and the search's status says so (`chosenDropped`)
export const chosenSetup = (job: DeepReadonly<CareerJob>): string =>
  JSON.stringify({
    engine: stateKey(),
    ...job,
    legacies: job.fix.map((i) => job.legacies[i]),
    career: undefined,
    top: undefined,
  });
const chosenApplies = (job: DeepReadonly<CareerJob>): boolean => {
  const c = chosen.value;
  if (!c || c.places !== placesKey() || c.setup !== chosenSetup(job)) return false;
  const named = (c.id.split(':')[1] || '').split(',').map(Number),
    fixed = job.fix;
  return named.length === 2 && fixed.length === 2 && named.every((i) => fixed.includes(i));
};
/** a saved choice dropped on load, while its places stand */
export let chosenDropped: { places: string; text: string } | null = null;
// a choice made with Use this: the dropped one's note goes
export function forgetDroppedChoice(): void {
  chosenDropped = null;
}
export function dropStaleChosen(): void {
  const c = chosen.value;
  if (!c) return;
  // (settings that make no job: the chosen career no longer applies; a bug building it is reported, and the start goes on)
  if (orNull(() => buildJob(settings.value), Blocked)?.career === c.id) return;
  chosenDropped = {
    places: c.places,
    text: 'The deck you chose with Use this before the reload no longer fits (its trainee, parents, their records, the targets or the card settings changed), so this is the deck the search finds.',
  };
  setChosen(null, true);
}
// a full veteran: a spark of every target (with full veteran parents: a fully looped parent)
export const isFull = (nd: DeepReadonly<LineageNode> | null | undefined): boolean =>
  !!nd && settings.value.targets.length > 0 && settings.value.targets.every((t) => nd.sparks?.[t.group] > 0);
// a career on two full veteran parents: her own full veteran there is a fully looped parent (the plan says so; a search's row is read off the veterans)
export const looped = (r: DeepReadonly<OptionRow>): boolean =>
  r.looped ?? r.parents.every((i) => isFull(vets.value[i]));
export const ROLL_K = 6; // careers a search returns with a place open (the plan's candidates; `buildJob`)
// the job of today's best career over everything (no place set, one result): the comparison's baseline
const baseOf = (job: DeepReadonly<CareerJob>): DeepReadonly<CareerJob> => ({
  ...job,
  career: undefined,
  exclude: gameData()
    .umas.filter((u) => !isOwned(u, settings.value))
    .map((u) => u.id),
  trainees: [],
  fix: [],
  per_trainee: false,
  top: 1,
  rollout: undefined,
  p_now: undefined,
});
export const baseKey = (job: DeepReadonly<CareerJob>): string => JSON.stringify(baseOf(job));
// the last plan made (career/state.ts `reference`): while the career on show is one you chose on the same setup and
// veterans (its job without the places), its table stays, so you can weigh the plan's other careers or go back to its pick
export const refBase = (job: DeepReadonly<CareerJob>): string =>
  planKey({
    ...job,
    exclude: gameData()
      .umas.filter((u) => !isOwned(u, settings.value))
      .map((u) => u.id),
    trainees: [],
    fix: [],
    top: 0,
    career: undefined,
  });
export const refFor = () => {
  const ref = reference.value,
    { last, planState } = answer.value;
  return ref && last && planState.state !== 'done' && ref.base === refBase(last.job) ? ref : null;
};
// ---------- the pedigree ----------
// the career on show: the answer's (career/state.ts `answerCareer`: the plan's pick once it is made, else the highest
// odds) while it belongs to the places on screen
const shownCareer = computed(() => (answer.value.last?.plan === placesKey(places.value) ? answerCareer.value : null));
export const shown = () => shownCareer.value;
// which veteran sits in parent place k: the chosen one, else the planner's pick in the result on show
export function parentAt(
  k: number,
  r: DeepReadonly<CareerReport> | null | undefined,
): { v: VetRecord; pick: boolean } | null {
  const plan = places.value,
    mine = plan.parents[k] && vetById(plan.parents[k]);
  if (mine) return { v: mine, pick: false };
  if (!r) return null;
  // the other place's chosen veteran (-1: none): the result's parent that isn't her fills this one
  const other = plan.parents[1 - k] ? idxOf(plan.parents[1 - k]) : -1;
  const i = other >= 0 ? r.parents.filter((x) => x !== other)[0] : r.parents[k];
  return i === undefined ? null : { v: vets.value[i], pick: true };
}
// sample veterans to try the planner with (no veterans yet): 18 mob umas drawn at random from the game's (those with a
// portrait), six veterans with two mob parents each, the bare minimum to start a loop: each veteran carries, at 1 star,
// only the targets nothing but inheritance gives (no card hints or events them, no outfit has them, the scenario
// doesn't hand them out); her parents carry nothing. No relation points with anyone (the solver knows no mob), no G1
// wins. Marked as samples; Remove all clears them.
function sampleVets(): Veteran[] {
  const all = careerTables().mobs || [],
    withArt = all.filter(([id]) => srcFor('mob', id)),
    pool = (withArt.length >= 18 ? withArt : all).slice();
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  const sourced = new Set(careerTables().sourced || []),
    inherit = targetGroups().filter((g) => !sourced.has(g) && !goldGiven(settings.value.scen, g));
  const node = (id: number, sparks: Sparks): LineageNode => ({
    chara: id,
    sparks,
    pinks: {},
    blues: {},
    greens: {},
    wins: [],
  });
  const out: Veteran[] = [];
  for (let i = 0; i + 2 < pool.length && out.length < 6; i += 3)
    out.push({
      id: `m${pool[i][0]}${Date.now().toString(36)}`,
      src: 'sample',
      borrowed: false,
      ...node(pool[i][0], Object.fromEntries(inherit.map((g) => [g, 1]))),
      parents: [node(pool[i + 1][0], {}), node(pool[i + 2][0], {})],
    });
  return out;
}
// an uma.moe listing (in the pool, not yours): shown in the popups, never edited
export const listed = (v: { src: string }): boolean => v.src === 'umamoe';
// The parent rules for veteran v beside the other parent o (none: no other) with trainee t (0: none), the page's form
// of the solver's `Pool::pair_ok` and the Borrowed parent setting: the first rule v breaks ('trainee': she is the
// trainee's uma; 'same': the other parent's; 'both': both borrowed; 'setting': borrowed while the setting allows none),
// or ''. The one place the parent picker (`blockedFor`) and the job (`buildJob`) read them
/** `s`: the settings (Borrowed parent) */
export function pairBlock(
  t: number | null,
  v: Pick<VetRecord, 'chara' | 'borrowed'>,
  o: Pick<VetRecord, 'chara' | 'borrowed'> | null | undefined,
  s: SettingsView,
): '' | 'trainee' | 'same' | 'both' | 'setting' {
  if (t && v.chara === t) return 'trainee';
  if (o && o.chara === v.chara) return 'same';
  if (o?.borrowed && v.borrowed) return 'both';
  if (v.borrowed && borrowChoice(s) !== 'allow') return 'setting';
  return '';
}

// ---------- the job on the solver
// every career a result names gets one outfit (never "any outfit"): its trainee's outfits are reordered with the chosen
// one first, filled with one of hers when none covers a target (`anyOutfit`); her popup says which
function oneOutfit(t: CareerTrainee | undefined): void {
  if (!t || t.outfitNote != null) return;
  const f = pickOutfit(t.id, t.outfits || []);
  t.outfits = f.list;
  t.anyOutfit = f.any;
  t.outfitNote = outfitNote(f);
}
/** The solver's answer to a career job: its CareerSearch, or a refusal (`'error' in m.r`). */
export type CareerAnswer = SolverAnswer<CareerSearch>;
export const runJob = async (
  job: CareerJob,
  progress?: (p: PlanProgress) => void,
  background = false,
): Promise<CareerAnswer> => {
  // (a career job: its report is a CareerSearch)
  const m = (await engineRun(job, undefined, progress, background)) as CareerAnswer;
  if ('error' in m.r) return m; // (a refusal: nothing to fill in)
  const r = m.r;
  for (const x of [...(r.results || []), r.next?.choice, ...(r.next?.options || [])]) oneOutfit(x?.trainee);
  // (a plan's rows carry `proven` alone; why, the solver reports for the whole job: kept with the plan, so a plan
  // taken again from `planKept` or `answers` says the same)
  if (r.next?.options) {
    r.next.unproven_why = unprovenWhy(r);
    // (and the named assumptions any search of the job rests on: the status line's plan caveat, `planNote`)
    r.next.job_assumes = r.job_assumes || [];
  }
  return m;
};
