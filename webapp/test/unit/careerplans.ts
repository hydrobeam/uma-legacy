// Typed plan fixtures for the career planner's node tests (careeroptions.test.tsx, careerplanrun.test.ts,
// careerborrow.test.tsx): a career job, a search report, a plan with its options, its robustness check and a route's
// members, with neutral defaults, each test overriding the fields it reads; the answer store and the veterans store set
// from them as a run leaves them. A wire-shape change in src/gen/solver.ts fails tsc here.
import { initVets } from '../../src/veterans/store.ts';
import { newPlanId, showAnswer } from '../../src/career/state.ts';
import { ERA } from '../../src/saved.ts';
import type { CareerJob, CareerReport, CareerSearch, PlanCheck, PlanOption, PlanReport } from '../../src/career/job.ts';
import type { RouteMember } from '../../src/gen/solver.ts';
import type { LineagePlan, Veteran } from '../../src/veterans/store.ts';
import { careerReport, lineageNode, veteran } from './reports.ts';
import { memory } from './fakes.ts';

/** a career job as buildJob makes it: one target, the places left to the planner, `over`'s career fields */
export const careerJob = (over: Partial<CareerJob['career']> = {}): CareerJob => ({
  cmd: 'career',
  skills: { targets: [{ group: 20049 }], rank_class: 'below_ss', exclude: [], exclude_outfits: [], extra: [] },
  deck: { owned_cards: [], borrow: false, scenario: 'unity' },
  career: { legacies: [], trainees: [], fix: [], per_trainee: false, top: 5, ...over },
});

/** a career of trainee `id` on parents `parents` (legacy indices), its id as the solver writes one */
export const career = (id: number, name: string, parents: [number, number], p = 0.01): CareerReport =>
  careerReport({
    trainee: { id, name, outfits: [], covers: [], schedule_outfit: 0 },
    career: `${id}:${parents.join(',')}:d`,
    parents,
    p_all: p,
  });

/** a plan's row: a career and its place in the plan, scored `score` */
export const option = (r: CareerReport, score: number | null, over: Partial<PlanOption> = {}): PlanOption => ({
  ...r,
  role: 'candidate',
  best_odds: false,
  chosen: false,
  looped: false,
  exact: true,
  stage_cost: null,
  index: score,
  saving: score,
  lift: null,
  lifts_side: null,
  score,
  ...over,
});

/** a route's member (A or B side): trainee `name`, her own career on `parents`, rerun at odds `p` */
export const member = (
  name: string,
  parents: [number, number] | null,
  over: Partial<RouteMember> = {},
): RouteMember => ({
  trainee: name,
  p: 0.01,
  parents,
  alternative: false,
  capped: false,
  proven: true,
  has_full_veteran: false,
  real: false,
  p_inf: null,
  lam: 0,
  phi: 0,
  p0: 0,
  building_careers: 0,
  p_switch: 0,
  builder: null,
  careers: 100,
  ...over,
});

/** a plan over `options` (the first chosen is the pick; its outcomes name the pick's career) */
export function plan(options: PlanOption[], over: Partial<PlanReport> = {}): PlanReport {
  const pick = options.find((o) => o.chosen) ?? null;
  return {
    kind: 'plan',
    choice: pick,
    rule: 'index',
    mode: 'trio',
    v0: 300,
    outcomes: pick
      ? { trainee: pick.trainee.id, parents: pick.parents, career: pick.career, uncv: [], fixed: [] }
      : null,
    members: [],
    trios: [],
    v0_plan: null,
    fit: [],
    pinf_builders: {},
    step_caps: {},
    loop_members: [],
    loop_note: null,
    hint: [],
    cut: [],
    side_only: [],
    umas_scored: options.length,
    umas_searched: options.length,
    umas_certified: options.length,
    umas_unproven: 0,
    work: {
      nodes: 0,
      exact: 0,
      max_ratio: 0,
      class_max_ratio: [0, 0, 0, 0],
      class_max_deck: [null, null, null, null],
      cap: 0,
      class_caps: [0, 0, 0, 0],
      prof: {},
    },
    sur: 'search',
    sur_diff: [],
    loop_ms: 0,
    search_ms: 0,
    combo_ms: 0,
    index_ms: 0,
    total_ms: 0,
    options,
    routes_dropped: 0,
    routes_dropped_uncertified: 0,
    scoring_complete: true,
    ...over,
  };
}

/** the robustness check's report on `pick` (a career id), every variant in */
export const check = (pick: string, over: Partial<PlanCheck> = {}): PlanCheck => ({
  kind: 'report',
  plan: pick,
  rows: [],
  variants: [],
  pending: 0,
  stable: true,
  contenders: [],
  pick_varies: false,
  check_ms: 0,
  pairs: null,
  scoring_complete: true,
  ...over,
});

/** the search's report with `results` */
export const search = (results: CareerReport[]): CareerSearch => ({
  kind: 'search',
  results,
  candidates: results.length,
  solved: results.length,
  bounded: 0,
  scored: results.length,
  skipped: 0,
  next_bound: null,
  complete: true,
  proven_top: true,
  cap: 0,
  class_caps: [0, 0, 0, 0],
  work: { nodes: 0, exact: 0 },
  certificates: {},
  timing_s: 0,
});

/** a complete veteran (one pink: Turf 1 star; her parents each with one) */
export const completeVet = (id: string, chara: number, over: Partial<Veteran> = {}): Veteran =>
  veteran(id, {
    ...lineageNode(chara, {}, { 11: 1 }),
    parents: [lineageNode(chara === 1002 ? 1003 : 1002, {}, { 11: 1 }), lineageNode(1004, {}, { 11: 1 })],
    ...over,
  });

/** the veterans store loaded from storage holding `vets` and `plan` (saves at once); returns the storage */
export function loadVets(vets: Veteran[], plan: LineagePlan = { trainee: null, parents: [null, null] }) {
  const storage = memory();
  storage.setItem('llp:career', JSON.stringify({ v: 2, [ERA]: { vets, plan } }));
  initVets({ storage: () => storage, hidden: () => true, afterFrame: (f) => f() });
  return storage;
}

/** the answer a run leaves: the search's results and the plan made on `job`, its id; the pool's veterans by id */
export function showPlan(planReport: PlanReport, job: CareerJob, pool: string[]): number {
  const id = newPlanId();
  showAnswer(
    {
      status: { kind: 'done', text: 'Planned in 1.0 s.' },
      last: { r: search(planReport.options), job, key: 'job', plan: '{"trainee":null,"parents":[null,null]}' },
      planState: { state: 'done', id, r: planReport, ms: 1000, job },
    },
    { base: 'base', places: { trainee: null, parents: [null, null] } },
    pool,
  );
  return id;
}
