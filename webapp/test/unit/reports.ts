// Typed fixtures the node tests share: whole solver reports and veteran records with neutral defaults (empty lists,
// zero chances, no schedule), each test overriding only the fields it reads, so a wire-shape change in src/gen/solver.ts
// fails tsc here, in one place.
import type {
  CareerJob,
  CareerReport,
  CareerSearch,
  CareerTarget,
  PlanCheck,
  PlanOption,
  PlanOptionFields,
  PlanReport,
} from '../../src/career/job.ts';
import type { ExactSummary, Grades, PairSearch, ResultDeck, ResultLoop } from '../../src/gen/solver.ts';
import type { LoopReport } from '../../src/loop/types.ts';
import type { LineageNode, Sparks, Veteran } from '../../src/veterans/store.ts';

/** grades G at every aptitude */
export const allG: Grades = { distance: [1, 1, 1, 1], surface: [1, 1], style: [1, 1, 1, 1] };

/** a result's deck: no cards, with `over`'s fields */
export const resultDeck = (over: Partial<ResultDeck> = {}): ResultDeck => ({
  cards: [],
  card_ids: [],
  pal_cards: [],
  end_chain_at: [],
  choices: [],
  odds: [],
  plain_ids: [],
  pals: 0,
  borrowed: null,
  ...over,
});

/** a career report: trainee 1001, parents 0 and 1, nothing else, with `over`'s fields */
export function careerReport(over: Partial<CareerReport> = {}): CareerReport {
  return {
    trainee: { id: 1001, name: 'Uma A', outfits: [], covers: [], schedule_outfit: 0 },
    career: '',
    parents: [0, 1],
    slots: [],
    targets: [],
    p_all: 0,
    p_minus1: 0,
    expected: 0,
    bound: 0,
    deck: resultDeck(),
    pinks: [],
    aptitudes: null,
    schedule: null,
    outcome_sets: null,
    g1_wins: null,
    ...over,
  };
}

/** a loop of a deck search's result: no members, odds `p_all`, with `over`'s fields */
export const resultLoop = (over: Partial<ResultLoop> = {}): ResultLoop => ({
  members: [],
  p_all: 0,
  p_minus1: 0,
  landed: null,
  rank_value: 0,
  outfits: [],
  per_trainee: [],
  mean_proc: 0,
  schedule: [],
  detail: { base: 0, gp_rate: 0, inspirations: 0, gen_white: 0, gen_gold: 0, pairs: [], lineages: [] },
  ...over,
});

/** a deck search's exact check: done, at no ratio, with `over`'s fields */
export const exactSummary = (over: Partial<ExactSummary> = {}): ExactSummary => ({
  rho: 0,
  rho_target: 0,
  class_caps: [0, 0, 0, 0],
  max_ratio: 0,
  class_max_ratio: [0, 0, 0, 0],
  class_max_deck: [null, null, null, null],
  complete: true,
  decks_scored: 0,
  deck_searches: 0,
  loops_solved: 0,
  schedule_rejects: 0,
  cap_hit: false,
  budget_out: false,
  bound_filtered: 0,
  ...over,
});

/** a deck search's report (`kind` "loops"): no loops, with `over`'s fields */
export const loopReport = (over: Partial<LoopReport> = {}): LoopReport => ({
  kind: 'loops',
  stats: {
    decks_built: 0,
    decks_on_frontier: 0,
    relevant_cards: 0,
    cards_dropped: 0,
    decks_swapped: 0,
    plain_pals: [],
  },
  top: [],
  per_uma: true,
  foursomes: 0,
  foursomes_ranked: 0,
  foursomes_scanned: 0,
  foursomes_solved: 0,
  next_bound: 0,
  proven: true,
  searches: 0,
  exact: exactSummary(),
  ...over,
});

/** a plan's option: career `report` as a candidate, unscored */
export const planOption = (report: CareerReport, over: Partial<PlanOptionFields> = {}): PlanOption => ({
  ...report,
  role: 'candidate',
  best_odds: false,
  chosen: false,
  looped: false,
  exact: false,
  stage_cost: null,
  index: null,
  saving: null,
  lift: null,
  lifts_side: null,
  score: null,
  ...over,
});

/** a plan (the search's `next`): no pick, no options, nothing scored, with `over`'s fields */
export const planReport = (over: Partial<PlanReport> = {}): PlanReport => ({
  kind: 'plan',
  choice: null,
  rule: 'index',
  mode: 'trio',
  v0: null,
  outcomes: null,
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
  umas_scored: 0,
  umas_searched: 0,
  umas_certified: 0,
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
  options: [],
  routes_dropped: 0,
  routes_dropped_uncertified: 0,
  scoring_complete: false,
  ...over,
});

/** a career search's report: no careers, nothing searched, with `over`'s fields */
export const careerSearch = (over: Partial<CareerSearch> = {}): CareerSearch => ({
  kind: 'search',
  results: [],
  candidates: 0,
  solved: 0,
  bounded: 0,
  scored: 0,
  skipped: 0,
  next_bound: null,
  complete: false,
  proven_top: false,
  cap: 0,
  class_caps: [0, 0, 0, 0],
  work: { nodes: 0, exact: 0 },
  certificates: {},
  timing_s: 0,
  ...over,
});

/** the pair search of a plan's check: no pairs, with `over`'s fields */
export const pairSearch = (over: Partial<PairSearch> = {}): PairSearch => ({
  trainees: [],
  shared: [],
  shared_coverage: { trainees: 0, careers: 0, pairs_runnable: 0, trainees_truncated: 0 },
  best: null,
  recommendation: {
    career: '',
    trainee: '',
    trainee_id: 0,
    parents: [0, 0],
    source: '',
    score: null,
    report: {
      ...careerReport(),
      trainee: { id: 1001, name: 'Uma A', outfits: [], covers: [], schedule_outfit: 0 },
      targets: [],
    },
  },
  skipped_for_work: [],
  pair_bound: 0,
  pairs_ms: 0,
  scoring_complete: false,
  ...over,
});

/** a plan's robustness check report (`kind` "report"): no variants, nothing pending, with `over`'s fields */
export const checkReport = (over: Partial<PlanCheck> = {}): PlanCheck => ({
  kind: 'report',
  plan: '',
  rows: [],
  variants: [],
  pending: 0,
  stable: false,
  contenders: [],
  pick_varies: false,
  check_ms: 0,
  pairs: null,
  scoring_complete: false,
  ...over,
});

/** a career job: no targets, no veterans, the solver's defaults written out, `career`'s fields over them */
export const careerJob = (career: Partial<CareerJob['career']> = {}): CareerJob => ({
  cmd: 'career',
  skills: { targets: [], rank_class: 'below_ss', exclude: [], exclude_outfits: [], extra: [] },
  deck: { owned_cards: [], borrow: false, scenario: 'unity' },
  career: { legacies: [], trainees: [], fix: [], per_trainee: false, top: 0, ...career },
});

/** a target of a career, bought by its sources at no chance */
export const careerTarget = (group: number, p: number): CareerTarget => ({
  group,
  q: 0,
  carriers: 0,
  p,
  gold: false,
  buy: { kind: 'sourced', gold: 0, white: 0, land_gold: 0, land_white: 0 },
});

/** a lineage node: her uma and sparks, the rest empty */
export const lineageNode = (chara: number | null, sparks: Sparks = {}, pinks: Sparks = {}): LineageNode => ({
  chara,
  sparks,
  pinks,
  blues: {},
  greens: {},
  wins: [],
});

/** a veteran with two parents, `over`'s fields over the defaults */
export const veteran = (id: string, over: Partial<Veteran> = {}): Veteran => ({
  id,
  src: 'import',
  borrowed: false,
  ...lineageNode(1001),
  parents: [lineageNode(1002), lineageNode(1003)],
  ...over,
});
