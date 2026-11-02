// The career job: built from the settings and the veterans store's record (`buildJob`, values in: career/current.ts
// passes the stores' as they stand), the Use this choice it carries, the parent rules, and the solver's career job and
// reports, typed here. A model module: it reads no store (.dependency-cruiser.mjs checks it).
import { gameData } from '../data.ts';
import { stateKey } from '../engine.ts';
import { deckPart, isOwned, skillsPart } from '../job.ts';
import { starsOf, umaName } from '../veterans/names.ts';
import { lacks, lacksSaid } from '../veterans/complete.ts';
import { assert, must } from '../must.ts';
import type { SolverAnswer } from '../solver.ts';
import type * as Gen from '../gen/solver.ts';
import type { SettingsView } from '../settings.ts';
import type { ChosenCareer, LineageNode, PlacesRecord, VetRecord } from '../veterans/store.ts';
import type { JobLegacy } from '../veterans/records.ts';
import type { DeepReadonly } from '../store.ts';

// the places as a key: the trainee and the two parents (not how they were set, `fromTable`)
export const placesKey = (p: PlacesRecord): string => JSON.stringify({ trainee: p.trainee, parents: p.parents });
/** The career job (`buildJob`): the solver's `Request` for `career` (its `skills`, `deck` and `career` parts;
 *  generated), its veterans as the veterans store holds them, one array that every job made from the same veterans
 *  shares (read-only); a plan job adds `career.next` and `career.no_reports`, a comparison `career.compare`. Each part
 *  has the fields `buildJob` always writes (the solver has a default for each). */
export type CareerJob = Omit<SolverCareerJob, 'skills' | 'deck' | 'career'> & {
  skills: Gen.SkillJob & Required<Pick<Gen.SkillJob, 'rank_class' | 'exclude' | 'exclude_outfits' | 'extra'>>;
  deck: Gen.DeckJob & Required<Pick<Gen.DeckJob, 'borrow' | 'scenario'>>;
  career: Omit<Gen.CareerJob, 'legacies'> &
    Required<Pick<Gen.CareerJob, 'fix' | 'per_trainee' | 'top'>> & { legacies: readonly JobLegacy[] };
};
type SolverCareerJob = Extract<Gen.Request, { cmd: 'career' }>;
// ---------- the solver's reports, generated from its Rust structs (src/gen/solver.ts), with what the page adds ----------
/** A career's trainee as the page shows her: the solver's, and what the page sets on her (`oneOutfit`) */
export type CareerTrainee = Omit<Gen.CareerTrainee, 'outfits'> & {
  /** her outfits that cover a target; the page puts the one it names first (`oneOutfit`) */
  outfits: readonly Gen.OutfitLabel[];
  /** set by the page: no outfit covers a target, any would do */
  anyOutfit?: boolean;
  /** set by the page: what her popup says about the outfit */
  outfitNote?: string;
};
/** One career in a report (career/outcomes.rs `report`), her trainee as the page shows her, each target's buying read
 *  as one record (`CareerBuy`) */
export type CareerReport = Omit<Gen.CareerReport, 'trainee' | 'targets'> & {
  trainee: CareerTrainee;
  targets: CareerTarget[];
};
export type { CareerScheduleRow, CareerSlot, PairSearch } from '../gen/solver.ts';
/** A target of a career, its buying read as one record */
export type CareerTarget = Omit<Gen.CareerTarget, 'buy'> & { buy: CareerBuy };
/** How she buys a target and lands it, as the page reads it: either of the solver's `TargetBuy`s (`kind` "sourced", an
 *  uncertain target's, with its sources; "scenario", a scenario gold's) as one record, each field absent where it does
 *  not apply (either is assignable to it) */
export type CareerBuy = Pick<Gen.TargetBuy, 'kind' | 'gold' | 'white' | 'land_gold' | 'land_white'> &
  Partial<Omit<Gen.SourcedBuy, 'gold' | 'white' | 'land_gold' | 'land_white'>>;
/** A plan option's own fields (career/plan/output.rs: its place in the plan, beside its career, `report`), and the
 *  pair search's recommendation's, which the page sets on the row it makes of it (`pairSearchRow`) */
export type PlanOptionFields = Omit<Gen.PlanOption, 'report'> & {
  /** the pair search's recommendation (`pairSearchRow`): run as it is, its deck kept */
  pair_search?: boolean;
  /** its Saves per run in the pair search */
  pair_search_score?: number | null | undefined;
};
/** One career of the plan's table, as the table reads it: its career report's fields beside its place in the plan
 *  (`current.ts` `runJob` makes these rows of the solver's options) */
export type PlanOption = CareerReport & PlanOptionFields;
/** A row of the Career choices table: a plan option, or a career of the search (without the plan's fields). */
export type OptionRow = CareerReport & Partial<PlanOptionFields>;
/** The robustness check's report on the pick (plan/robust.rs), as the page keeps it from its follow-up jobs */
export type PlanCheck = Extract<Gen.CheckReply, { kind: 'report' }>;
/** The plan (the solver's `next`, career/plan/output.rs), with what the page sets on it: its check (`checkPhase`), that
 *  the check could not finish, why its searches proved nothing (`unprovenWhy`), the plan job's named assumptions */
export type PlanReport = Omit<Gen.Plan, 'choice' | 'options' | 'robust'> & {
  kind: 'plan';
  /** the plan's pick */
  choice: CareerReport | null;
  /** by score, best first */
  options: PlanOption[];
  /** the robustness check on the pick, set by the page from its follow-up jobs (`checkPhase`) */
  robust?: PlanCheck;
  /** set by the page: the robustness check could not finish (`checkPhase`) */
  check_unavailable?: boolean;
  /** set by the page: why some of its career searches proved nothing (`unprovenWhy`) */
  unproven_why?: string[];
  /** set by the page: the named assumptions any search of the plan job rests on (the job's `job_assumes`) */
  job_assumes?: Gen.Assumption[];
};
/** A route's A, B and C steps: the expected careers of each (null: an infinite cost, a step that cannot land) */
export type RouteCareers = Gen.Route['careers'];
export type { RouteMember } from '../gen/solver.ts';
/** A plan's continuation fit, one entry per stage candidate (`[chara, lam, phi, pinf, p0]`, as the solver writes it;
 *  the page never reads inside it). */
export type PlanFit = Gen.Plan['fit'];
/** The career search's report (career/run.rs `search_output`), its careers as the page shows them, its plan (`next`)
 *  or the plan's refusal; `pending`: set by the page, a quick comparison's stand-in while the search runs */
export type CareerSearch = Omit<Gen.CareerSearch, 'results' | 'next'> & {
  kind: 'search';
  results: CareerReport[];
  next?: PlanReport | PlanRefusal;
  pending?: boolean;
};
/** Why the plan job made no plan (`next` of kind "refusal") */
export type PlanRefusal = Extract<Gen.PlanReply, { kind: 'refusal' }>;
/** What a career job the page sends is answered with, by `kind`: its search (with its plan when it asked for one), or,
 *  for a plan job's roll or check that the worker answers from the plan it keeps, that alone (career/reply.rs
 *  `CareerReply`); a refusal is a SolverRefusal */
export type CareerReply = CareerSearch | Extract<Gen.CareerReply, { kind: 'roll' | 'check' }>;
// ---------- the job ----------
/** A status line's button: its label and what it does. */
export interface StatusAct {
  label: string;
  run: () => void;
}
/** Settings, veterans and places that make no job: the status line's kind, its text and its button(s). */
export class Blocked extends Error {
  declare kind: 'empty' | 'error';
  declare act: StatusAct | StatusAct[] | undefined;
  constructor(kind: 'empty' | 'error', text: string, act?: StatusAct | StatusAct[]) {
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
/** What a career job is made of: the settings and the veterans store's record (the veterans, the complete ones each as
 *  the job takes her, the places and the career chosen with Use this). */
export interface JobFrom {
  settings: SettingsView;
  /** every veteran (a place names any of them) */
  vets: readonly VetRecord[];
  /** the complete veterans (veterans/store.ts `readyVets`), in the order of the job's legacies */
  ready: readonly VetRecord[];
  /** each complete veteran as the job takes her (veterans/store.ts `legaciesJob`: the same array until the veterans
   *  change) */
  legacies: readonly JobLegacy[];
  places: PlacesRecord;
  chosen: DeepReadonly<ChosenCareer> | null;
}
/** What the status line's buttons do when the veterans make no job: open the veterans dialog, add the sample veterans */
export interface PoolActs {
  importVets: () => void;
  trySamples: () => void;
}
// the career job `from` the settings, veterans and places (throws Blocked when they make none; `acts`: its buttons).
// It is built on the complete veterans alone (veterans/complete.ts): a legacy index is a place among them
export function buildJob(from: JobFrom, acts: PoolActs): CareerJob {
  const { settings: s, ready: pool, places: plan } = from;
  assert(pool.length === from.legacies.length, 'the job takes each complete veteran once');
  if (!s.targets.length) throw new Blocked('empty', 'No target skills: add them with Target skills above.');
  // a place holding an incomplete veteran: no job until she is finished
  for (const [k, id] of plan.parents.entries()) {
    const v = id ? from.vets.find((x) => x.id === id) : undefined,
      lacking = v ? lacks(v) : [];
    if (v && lacking.length)
      throw new Blocked('error', `Parent ${k + 1} (${umaName(v.chara)}) is incomplete: missing ${lacksSaid(lacking)}.`);
  }
  const vetById = (id: string) => pool.find((v) => v.id === id);
  const fixed = (plan.parents.filter(Boolean) as string[]).map(vetById);
  if (pool.length < 2 || !pool.some((v) => !v.borrowed)) {
    const incomplete = from.vets.length - pool.length;
    if (incomplete)
      throw new Blocked(
        'empty',
        `${incomplete} veteran${incomplete === 1 ? ' is' : 's are'} incomplete: the planner needs two complete ones, one of them yours.`,
        { label: 'Veteran Umamusume', run: acts.importVets },
      );
    const importVets = { label: 'Import veterans', run: acts.importVets };
    throw new Blocked(
      'empty',
      'Choose both parents, or import your veterans and let the planner pick.',
      !pool.some((v) => !listed(v)) ? [importVets, { label: 'Try sample veterans', run: acts.trySamples }] : importVets,
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
  const { owned, notOwned } = umaRoster(s);
  const exclude = notOwned.filter((id) => id !== plan.trainee);
  if (!plan.trainee && !owned.length) throw new Blocked('empty', 'No uma is owned: mark yours with Umas above.');
  const allPlaced = plan.trainee && fixed.length === 2;
  // a place left open: the K best careers by odds of every owned uma (their trainees are among the plan's stage
  // candidates; the plan scores every owned uma, `planPhase`)
  const job: CareerJob = {
    cmd: 'career',
    skills: skillsPart(s, exclude),
    deck: deckPart(s),
    career: {
      legacies: from.legacies,
      trainees: plan.trainee ? [plan.trainee] : owned,
      fix: plan.parents.filter(Boolean).map((id) => pool.findIndex((v) => v.id === id)),
      per_trainee: false,
      top: allPlaced ? 1 : ROLL_K,
    },
  };
  const borrowed = borrowChoice(s);
  if (borrowed !== 'allow') job.career.borrowed_parent = borrowed;
  if (allPlaced && chosenApplies(job, from.chosen, plan))
    job.career.given = must(from.chosen, 'chosenApplies: a choice is saved').id;
  return job;
}
// the career Use this chose with its deck (veterans/store.ts `chosen`, saved with the plan): a builder's other career (an
// option with `alternative`: a search on its places would give her best deck there instead, but for the pick, searched
// on its pair before it is shown; listed with the plan's `builder_careers` above 1, its default 1, which the page keeps:
// none by default) or the pair search's recommendation (`pair_search`). It is sent only while it applies
// (`chosenApplies`): its places, the setup it was chosen on (`chosenSetup`: the job's settings, its targets and cards
// among them, its two parents' records and the solver's and data's fingerprints, engine.ts `stateKey`) and its parents'
// places in the veteran list (the id names them by index) all as they were. One saved before a reload that no longer
// applies is dropped, and the search's status says so (career/current.ts `dropStaleChosen`)
export const chosenSetup = (job: DeepReadonly<CareerJob>): string =>
  JSON.stringify({
    engine: stateKey(),
    ...job,
    career: {
      ...job.career,
      legacies: job.career.fix.map((i) => job.career.legacies[i]),
      given: undefined,
      top: undefined,
    },
  });
const chosenApplies = (
  job: DeepReadonly<CareerJob>,
  c: DeepReadonly<ChosenCareer> | null,
  places: PlacesRecord,
): boolean => {
  if (!c || c.places !== placesKey(places) || c.setup !== chosenSetup(job)) return false;
  const named = (c.id.split(':')[1] || '').split(',').map(Number),
    fixed = job.career.fix;
  return named.length === 2 && fixed.length === 2 && named.every((i) => fixed.includes(i));
};
// a full veteran: a spark of every target (with full veteran parents: a fully looped parent)
export const isFull = (nd: DeepReadonly<LineageNode> | null | undefined, targets: SettingsView['targets']): boolean =>
  !!nd && targets.length > 0 && targets.every((t) => starsOf(nd.sparks, t.group) > 0);
export const ROLL_K = 6; // careers a search returns with a place open (the plan's candidates; `buildJob`)
/** a roster's umas by id, in the data's order: those owned, those not */
export type Roster = { owned: number[]; notOwned: number[] };
/** the umas owned and not owned under settings `s`, by id (a baseline's or the last plan's roster: keys.ts `baseKey`) */
export const umaRoster = (s: SettingsView): Roster => ({
  owned: gameData()
    .umas.filter((u) => isOwned(u, s))
    .map((u) => u.id),
  notOwned: gameData()
    .umas.filter((u) => !isOwned(u, s))
    .map((u) => u.id),
});
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
/** The solver's answer to a career job: its CareerReply, or a refusal (`m.r.kind`). */
export type CareerAnswer = SolverAnswer<CareerReply>;
