// The career job: built from the settings, the veterans and the places (`buildJob`), the Use this choice it carries,
// what reads the job, the places and the career on show, and its run on the solver (`runJob`). The solver's career job
// and its reports are typed here.
import { $ } from '../dom.js';
import { gameData } from '../data.js';
import { settings } from '../settings.ts';
import { run as engineRun, stateKey } from '../engine.ts';
import { isOwned, jobTargets, ownedFields, scenarioFields, starDist } from '../job.js';
import { vets, plan, chosen, persist, vetById, umaName, sampleVets, addVets, legaciesJob } from '../vets.js';
import { answer, answerCareer, reference } from './state.ts';
import { must, orNull } from '../must.ts';
import { jobKey, planKey } from './keys.js';
import { pickOutfit, outfitNote } from '../roster.js';
import { unprovenWhy } from '../proofs.mjs';
/** @import { LineagePlan } from '../vets.js' */
/** @import { DeepReadonly } from '../store.ts' */

// the places as a key: the trainee and the two parents (not how they were set, `fromTable`)
/** @type {(p?: DeepReadonly<LineagePlan>) => string} */
export const placesKey = (p = plan) => JSON.stringify({ trainee: p.trainee, parents: p.parents });
export const runKey = (job) => `${jobKey(job)}\n${placesKey()}`;
/**
 * The career job (`buildJob`; the solver reads it as `SkillJob` + `DeckJob` + `CareerJob`, web.rs); a plan job adds
 * `next` and `no_reports`, a comparison `compare`.
 * @typedef {object} CareerJob
 * @property {'career'} cmd
 * @property {{group: number}[]} targets
 * @property {number[]} exclude  umas not owned (by character id)
 * @property {number[]} exclude_outfits  outfits not owned
 * @property {number[][]} extra
 * @property {number[]} owned_cards
 * @property {boolean} borrow
 * @property {number[]} star_dist  the lineage rank's chances of 1, 2 and 3 stars
 * @property {string} scenario
 * @property {number[]} [exclude_cards]  R cards (not pals) left out
 * @property {import('../vets.js').JobLegacy[]} legacies  every veteran, by legacy index (`legaciesJob`)
 * @property {number[]} trainees  the trainee chosen (none: the planner picks)
 * @property {number[]} fix  the parents chosen, by legacy index
 * @property {boolean} per_trainee
 * @property {number} top  careers to return
 * @property {undefined} [rollout]  never set by `buildJob`, and the solver reads no such field: a leftover of the
 *   rollout planner that `baseOf`, `planKey` and the plan job still clear
 * @property {undefined} [p_now]  likewise
 * @property {'later'|'never'} [borrowed_parent]  left out at 'allow'
 * @property {{roll?: number[], fit?: PlanFit, robust?: 'next', pair_search?: boolean}} [next]  a plan job: its settings
 *   (`roll`: a roll's spark stars, per target; `fit`: another plan's continuations, taken as given instead of fitted to
 *   this pool; `robust`: the next step of the kept plan's robustness check, `checkPhase`; `pair_search`: that check
 *   ends with the within-trainee pair search, sent on the check's jobs only)
 * @property {boolean} [no_reports]  a plan job: no career reports, `next` only
 * @property {boolean} [compare]  a comparison's job
 * @property {string} [career]  one career reported as it is, no deck search (career/given.rs: its `career` id, on the
 *   trainee and both parents fixed): Use this on a builder's other career or the pair search's (`chosen`)
 *
 * One career in a report (career/outcomes.rs `report`): the trainee and her parents, by legacy index.
 * @typedef {object} CareerReport
 * @property {CareerTrainee} trainee
 * @property {[number, number]} parents
 * @property {string} [career]  the career's identity (`kept::career_id`: her uma, parents in order and deck)
 * @property {CareerSlot[]} slots  her six legacies
 * @property {CareerTarget[]} targets  per target, in the job's order
 * @property {number} p_all  the chance she lands every target
 * @property {number} p_minus1  the chance she lands all but at most one
 * @property {number} expected  the targets she lands, expected
 * @property {number} bound
 * @property {import('../loop/types.ts').ResultDeck} deck
 * @property {{group: number, q: number, carriers: number}[]} pinks  per pink the six legacies carry: the chance at least
 *   one of the two inheritance events passes it on, and its carriers
 * @property {{base: number[], start: number[], fire: object[], pink_gen: {group: number, p: number}[]}} aptitudes  her
 *   aptitude grades (codes 1..8 = G..S) and her veteran's pink spark (pink.mjs)
 * @property {CareerScheduleRow[]|null} schedule  every G1 she could enter, in turn order (the search's
 *   best career and the plan's pick only)
 * @property {number[]|null} outcome_sets  [mask]: the chance of landing exactly the targets in mask (as
 *   `schedule`, up to 12 targets)
 * @property {number|null} g1_wins  her G1 wins, expected (as `schedule`)
 * @property {boolean} [looped]  a plan option: both parents are full veterans
 *
 * A row of a career's schedule (career/schedule.rs): the loop schedule's row, one cell (hers), with the race's turn.
 * @typedef {import('../agenda.js').ScheduleRow & CareerScheduleFields} CareerScheduleRow
 * @typedef {object} CareerScheduleFields
 * @property {number} turn  0..71
 * @property {string} name  the race's full name (`race`: as the chart shows it)
 * @property {boolean} run  she runs it (false: she skips it)
 * @property {boolean} skipped
 * @property {string} instead  what she does that turn instead (a race's name, or "rest")
 * @property {number|null} p  her win chance (a fraction; null when she skips it)
 * @property {boolean} goal  a goal race of her scenario
 *
 * @typedef {object} CareerTrainee
 * @property {number} id
 * @property {string} name
 * @property {{id: number, label: string}[]} outfits  her outfits that cover a target; the page puts the one it names
 *   first (`oneOutfit`)
 * @property {{group: number, level: number}[]} covers
 * @property {boolean} [anyOutfit]  set by the page: no outfit covers a target, any would do
 * @property {string} [outfitNote]  set by the page: what her popup says about the outfit
 *
 * @typedef {object} CareerSlot
 * @property {'parent'|'grandparent'} kind
 * @property {number} legacy  the parent's legacy index
 * @property {number|null} gp  a grandparent: which of the parent's two
 * @property {number} chara
 * @property {string} name
 * @property {number} rel  relation points
 * @property {number} race  shared G1 wins, as points
 * @property {number} aff  affinity: rel + race
 *
 * @typedef {object} CareerTarget
 * @property {number} group
 * @property {number} q  the chance inheritance gives it
 * @property {number} carriers  legacies with its spark
 * @property {number} p  the chance she lands it
 * @property {boolean} gold  a gold the scenario hands out (`buy.scenario`)
 * @property {CareerBuy} buy
 *
 * How she buys a target and lands it as a spark (outcomes.rs `report`): the chance she buys it as gold and as white,
 * lands each, and the sources; a gold the scenario hands out has `scenario` and the first four only.
 * @typedef {object} CareerBuy
 * @property {number} gold
 * @property {number} white
 * @property {number} land_gold
 * @property {number} land_white
 * @property {number} [deck_gold]  the deck hints its gold
 * @property {number} [deck_white]  the deck hints its white, given it hints no gold
 * @property {number} [own_event]  her own story events give it
 * @property {number} [inherit]  inspiration gives it
 * @property {0|1|2} [outfit]  her outfit's own skill: none, the white, the gold
 * @property {boolean} [scenario]
 *
 * One career of the plan's table (output.rs): a career report with its place in the plan.
 * @typedef {CareerReport & PlanOptionFields} PlanOption
 * @typedef {object} PlanOptionFields
 * @property {'plan'|'finish'|'loop'|'candidate'|'builder'} role  her place in today's estimate (`roleText`)
 * @property {boolean} best_odds  the search's best career
 * @property {boolean} chosen  the plan's pick
 * @property {boolean} looped  both parents are full veterans
 * @property {boolean} exact  scored from her own career search
 * @property {boolean} [proven]  false: her career search was cut short, or her deck class breached its assumed cap
 * @property {string[]} [assumes]  the named assumptions her proven career rests on (the status line states the plan
 *   job's, `planCaveat`; the table adds none)
 * @property {number|null} stage_cost
 * @property {number|null} index  the careers she saves toward a fully looped parent, net
 * @property {number|null} saving
 * @property {number|null} lift  the chance her result lowers the careers to a fully looped parent
 * @property {{uma: number|null, chance: number, own: number}|null} [lifts_side]  over her results that lower it, the
 *   stage candidate (uma id) whose side the route taken lifts most often and that chance, and the chance the route
 *   has her own full veteran as a side (`roleText`); null where the row was not scored
 * @property {number|null} score  the table's order
 * @property {boolean} [pair_search]  the pair search's recommendation (`pairSearchRow`): run as it is, its deck kept
 * @property {number|null} [pair_search_score]  its Saves per run in the pair search
 *
 * A row of the Career choices table: a plan option, or a career of the search (without the plan's fields).
 * @typedef {CareerReport & Partial<PlanOptionFields>} OptionRow
 *
 * A parent on the plan's route (`trio_report` in plan/output.rs): her step to an all-targets veteran.
 * @typedef {object} RouteMember
 * @property {string|null} trainee  her name
 * @property {number} p  the chance of landing every target today of the career her fresh side is from
 * @property {[number, number]|null} parents  that career's parents, by legacy index (null for a real full veteran)
 * @property {boolean} alternative  that career is another of her top careers, not her best today (`fresh_lineages`)
 * @property {boolean} proven  a search with both its parents fixed proved that career's odds
 * @property {number} p0  the odds her continuation is anchored at
 * @property {boolean} has_full_veteran  she already has a full veteran (a poorer parent than a fresh one)
 * @property {boolean} real  the route uses her existing full veteran: her step costs no careers
 * @property {number|null} p_inf  her effective ceiling, the odds her lifts approach (null when lam is 0: no building)
 * @property {number} lam  the chance a building career lifts her odds, before it falls with the remaining gap
 * @property {number} phi  the share of the remaining log-odds gap to p_inf that each lift closes
 * @property {number} building_careers  the expected careers she spends building lifts (each building career lifts
 *   her odds only by chance) before rerunning her best career; 0 when she does not build
 * @property {number} p_switch  her odds once building stops
 * @property {string|null} [builder]  the career id of the row her lifts are fitted from (null where she grinds, her
 *   continuation was given, or her step is her cap's)
 * @property {number} careers  her step's expected careers (0 when `real`)
 *
 * The plan's report (the solver's `next`, career/plan/output.rs), or its refusal (`error`).
 * @typedef {object} PlanReport
 * @property {string} [error]
 * @property {CareerReport|null} choice  the plan's pick
 * @property {PlanOption[]} options  by score, best first
 * @property {number|null} v0  expected careers to a fully looped parent with the veterans now
 * @property {{a: string|null, b: string|null, c: string|null, careers: number[], sides?: string[]}|null} v0_plan  today's
 *   cheapest route: its steps' trainees by name, their careers, and A's and B's sides (`side_key` in
 *   career/plan/output.rs: "real:i" her veteran i, "current:i,j" her best career now, on veterans i and j, "alt:i,j"
 *   another career of hers, on veterans i and j)
 * @property {{a: string|null, b: string|null, c: string|null, careers: number[], sides?: string[], value: number, near: boolean}[]} [trios]
 *   the cheapest routes, cheapest first; `near`: within 2% of V's (assumed), so about as cheap
 * @property {RouteMember[]} [members]  the route's A and B, in its order (empty without a route)
 * @property {{trainee: number, parents: [number, number], career?: string, uncv: number[], fixed: number[]}|null} outcomes  the career a
 *   roll is of (`next.roll`)
 * @property {{y: number}} [roll]  a roll's saving over the plan's routes (`rollReq`)
 * @property {boolean} [check_unavailable]  set by the page: the robustness check could not finish (`checkPhase`)
 * @property {boolean} [scoring_complete]  false: the plan's memory budget left out a route no batch could hold or a
 *   partner setup (career/plan/output.rs), so its savings are lower bounds and its pick may not be the model's
 * @property {{plan: string, pending: number, stable: boolean, contenders: string[], pick_varies?: boolean, pairs?: PairSearch|null, scoring_complete?: boolean, error?: string, unavailable?: boolean}} [robust]
 *   the robustness check on the pick (plan/robust.rs), set by the page from its follow-up jobs (`checkPhase`): the pick
 *   it is of, the variants left, whether every tested variant scores the pick highest (`stable`), the careers (`keyOf`)
 *   the variants score highest (`contenders`), whether they disagree (`pick_varies`: the pick varies under these
 *   variants, a sensitivity, not a tie); the within-trainee pair search once it has run (`pairs`: null before, absent
 *   from a solver without it); `error` and `unavailable`: the worker kept no plan to check (robust.rs)
 * @property {PlanFit} [fit]  the stage candidates and their continuations the plan was fitted to: a comparison passes
 *   it on unchanged (`next.fit`), so the pool with her is priced the same way
 * @property {string[]} [unproven_why]  set by the page: why some of its career searches proved nothing (`unprovenWhy`)
 * @property {string[]} [job_assumes]  set by the page: the named assumptions any search of the plan job rests on (the
 *   job's `job_assumes`, kept with the plan; `planCaveat`)
 *
 * The within-trainee pair search (plan/robust.rs `pair_search`), one entry per trainee it ran for: the pick's, then each
 * one a per-uma scenario scores highest. `shown`: the career she is weighed against (`keyOf`): for the pick's trainee the
 * pick, for another her best-scoring shown career; `best_parents`: the legacy
 * indices of her best pair by its best deck found so far; `score`: that pair's Saves per run after its deck was searched
 * exactly; `margin_over_shown`: that less her shown career's; `margin_over_other_pairs`: that less the best other pair
 * evaluated (null where there is none; neither margin certifies the best pair); `corner_margins`: the margin over her
 * shown career under each building-curve corner; `scale_margins`: at her own odds x0.76 and x1.32 (both careers alike);
 * `pairs_scored`: the pairs she was scored on, the best among them; `pairs_runnable`: the pairs she can run;
 * `pairs_truncated`: the bound (`pair_bound` per uma) left some of them unscored, the rest by their all-target ceiling.
 * `reason`: why she was searched ("pick", "scenario": a per-uma scenario's pick, "top": among the 4 best by score);
 * `better_parents`: her best beats her shown career on another pair. `shared_coverage`: the umas without their own search
 * the best pairs were tried for, and the careers so scored. `best`: the best (trainee, pair) by score over both, with its
 * margin over the pick's trainee's shown career and, where it is not the pick, `versus_pick`: its margins over the pick
 * (`career`) as fitted and in the two per-uma scenarios, and the scenarios that reverse the fitted margin's sign
 * (`varies_under`: a sensitivity, deciding nothing). `recommendation`: the career to run, the higher-scoring of the pick
 * and `best` as fitted (`source` "cross_trainee", "pick_trainee_pair" or "plan_pick"), with its pair-search `score` and
 * `report` (a career report, its deck with it: the table's row for it, `pairSearchRow`).
 * @typedef {{trainees: {trainee: string, reason?: string, shown: string, best_parents: [number, number], score: number, margin_over_shown: number, better_parents: boolean, margin_over_other_pairs: number|null, corner_margins?: number[], scale_margins?: number[], pairs_scored: number, pairs_runnable?: number, pairs_truncated?: boolean}[], shared_coverage?: {trainees: number, careers: number, trainees_truncated?: number}, pair_bound?: number, best?: {trainee: string, career: string, best_parents: [number, number], score: number, margin_over_pick_trainee_shown: number|null, versus_pick?: {career: string, margins: {nominal: number, pick_down: number, pick_up: number}, varies_under: string[]}}|null, recommendation?: {career: string, source: string, score?: number|null, report?: CareerReport}}} PairSearch
 *
 * A plan's continuation fit, one entry per stage candidate (`[chara, lam, phi, pinf, ...]`, as the solver writes it;
 * the page never reads inside it).
 * @typedef {number[][]} PlanFit
 *
 * The career search's report (career/run.rs `search_output`); a refusal is a SolverRefusal (solver.ts).
 * @typedef {object} CareerSearch
 * @property {CareerReport[]} results  the best careers, best first
 * @property {number} candidates  careers it weighed
 * @property {boolean} proven_top  the best odds' order is certified
 * @property {boolean} [complete]  false: a search of the job was cut short, or certified at a lower deck cap only
 * @property {number} [band_overflows]  deck bands too large to check in full (each then certified at a lower cap)
 * @property {[number, number, number][]} [cap_exceeded]  per deck class past its assumed cap: [class, ratio, cap]
 * @property {{searches: number, unproven: number, unanswered: number}} [cap_rerun]  the deck searches rerun without the
 *   cap after a breach: those that proved nothing, and breaches left unanswered
 * @property {string[]} [unproven]  why the main search is not complete (`work_cap`, `band_overflow`, `cap_breach`)
 * @property {string[]} [job_unproven]  why any search of the job is not complete
 * @property {string[]} [assumes]  the named assumptions the main search's proof rests on (`two_feeder_cap`,
 *   `multi_feeder_cap`): still a proof (proofs.mjs `assumedText`)
 * @property {string[]} [job_assumes]  the named assumptions any search of the job rests on
 * @property {number} [cap_breaches]  capped deck searches of the job that scored a deck past its class's cap
 * @property {number[]} [cap_breached]  trainees (chara ids) whose deck class breached its cap, here or before
 * @property {PlanReport} [next]  a plan job's plan
 * @property {boolean} [pending]  set by the page: a quick comparison's stand-in while the search for her career runs
 */
// ---------- the job ----------
export const idxOf = (id) => vets.findIndex((v) => v.id === id);
export class Blocked extends Error {
  constructor(kind, text, act) {
    super(text);
    this.kind = kind;
    this.act = act;
  }
}
// the borrowed-parent choice (Career setup): 'allow' (one per career, now and later), 'later' (none in this career; the
// plan's later careers still borrow), 'never'; the job's `borrowed_parent`, left out at 'allow' (the solver's default)
export const BORROW_CHOICES = { allow: 'Allowed', later: 'None left today', never: 'Never' };
/** @type {(s: import('../settings.ts').SettingsView) => keyof typeof BORROW_CHOICES} */
export const borrowChoice = (s) =>
  Object.hasOwn(BORROW_CHOICES, s.borrowPar) ? /** @type {keyof typeof BORROW_CHOICES} */ (s.borrowPar) : 'allow';
// the career job for settings `s` and the veterans and places (throws Blocked when they make none)
/** @param {import('../settings.ts').SettingsView} s */
export function buildJob(s) {
  if (!s.targets.length) throw new Blocked('empty', 'No target skills: add them with Target skills above.');
  const fixed = plan.parents.filter(Boolean).map(vetById);
  if (vets.length < 2 || !vets.some((v) => !v.borrowed)) {
    const importVets = { label: 'Import veterans', run: () => $('v-toggle').click() };
    throw new Blocked(
      'empty',
      'Choose both parents, or import your veterans and let the planner pick.',
      !vets.some((v) => !listed(v))
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
  /** @type {CareerJob} */
  const job = {
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
  if (allPlaced && chosenApplies(job)) job.career = must(chosen.career, 'chosenApplies: a choice is saved').id;
  return job;
}
// the career Use this chose with its deck (vets.js `chosen`, saved with the plan): a builder's other career (an option
// with `alternative`: a search on its places would give her best deck there instead, but for the pick, searched on its
// pair before it is shown; listed with the plan's `builder_careers` above 1, its default 1, which the page keeps: none by
// default) or the pair search's recommendation (`pair_search`). It is sent only while it applies (`chosenApplies`): its
// places, the setup it was chosen on (`chosenSetup`: the job's settings, its targets and cards among them, its two
// parents' records and the solver's and data's fingerprints, engine.ts `stateKey`) and its parents' places in the veteran list (the id names them by index) all as they were. One
// saved before a reload that no longer applies is dropped, and the search's status says so (`chosenDropped`)
/** @type {(job: CareerJob) => string} */
export const chosenSetup = (job) =>
  JSON.stringify({
    engine: stateKey(),
    ...job,
    legacies: job.fix.map((i) => job.legacies[i]),
    career: undefined,
    top: undefined,
  });
/** @type {(job: CareerJob) => boolean} */
const chosenApplies = (job) => {
  const c = chosen.career;
  if (!c || c.places !== placesKey() || c.setup !== chosenSetup(job)) return false;
  const named = (c.id.split(':')[1] || '').split(',').map(Number),
    fixed = job.fix;
  return named.length === 2 && fixed.length === 2 && named.every((i) => fixed.includes(i));
};
/** @type {{places: string, text: string}|null} a saved choice dropped on load, while its places stand */
export let chosenDropped = null;
// a choice made with Use this: the dropped one's note goes
export function forgetDroppedChoice() {
  chosenDropped = null;
}
export function dropStaleChosen() {
  if (!chosen.career) return;
  // (settings that make no job: the chosen career no longer applies; a bug building it is reported, and the start goes on)
  if (orNull(() => buildJob(settings.value), Blocked)?.career === chosen.career.id) return;
  chosenDropped = {
    places: chosen.career.places,
    text: 'The deck you chose with Use this before the reload no longer fits (its trainee, parents, their records, the targets or the card settings changed), so this is the deck the search finds.',
  };
  chosen.career = null;
  persist();
}
// a full veteran: a spark of every target (with full veteran parents: a fully looped parent)
export const isFull = (nd) =>
  !!nd && settings.value.targets.length > 0 && settings.value.targets.every((t) => nd.sparks?.[t.group] > 0);
// a career on two full veteran parents: her own full veteran there is a fully looped parent (the plan says so; a search's row is read off the veterans)
export const looped = (r) => r.looped ?? r.parents.every((i) => isFull(vets[i]));
export const ROLL_K = 6; // careers a search returns with a place open (the plan's candidates; `buildJob`)
// the job of today's best career over everything (no place set, one result): the comparison's baseline
const baseOf = (job) => ({
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
export const baseKey = (job) => JSON.stringify(baseOf(job));
// the last plan made (career/state.ts `reference`): while the career on show is one you chose on the same setup and
// veterans (its job without the places), its table stays, so you can weigh the plan's other careers or go back to its pick
export const refBase = (job) =>
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
// odds) while it belongs to the places on screen (`plan`, not yet a signal, so not part of the computed)
export const shown = () => (answer.value.last?.plan === placesKey() ? answerCareer.value : null);
// which veteran sits in parent place k: the chosen one, else the planner's pick in the result on show
export function parentAt(k, r) {
  const mine = plan.parents[k] && vetById(plan.parents[k]);
  if (mine) return { v: mine, pick: false };
  if (!r) return null;
  // the other place's chosen veteran (-1: none): the result's parent that isn't her fills this one
  const other = plan.parents[1 - k] ? idxOf(plan.parents[1 - k]) : -1;
  const i = other >= 0 ? r.parents.filter((x) => x !== other)[0] : r.parents[k];
  return i === undefined ? null : { v: vets[i], pick: true };
}
// an uma.moe listing (in the pool, not yours): shown in the popups, never edited
export const listed = (v) => v.src === 'umamoe';
// The parent rules for veteran v beside the other parent o (none: no other) with trainee t (0: none), the page's form
// of the solver's `Pool::pair_ok` and the Borrowed parent setting: the first rule v breaks ('trainee': she is the
// trainee's uma; 'same': the other parent's; 'both': both borrowed; 'setting': borrowed while the setting allows none),
// or ''. The one place the parent picker (`blockedFor`) and the job (`buildJob`) read them
/** @param {import('../settings.ts').SettingsView} s the settings (Borrowed parent) */
export function pairBlock(t, v, o, s) {
  if (t && v.chara === t) return 'trainee';
  if (o && o.chara === v.chara) return 'same';
  if (o?.borrowed && v.borrowed) return 'both';
  if (v.borrowed && borrowChoice(s) !== 'allow') return 'setting';
  return '';
}

// ---------- the job on the solver
// every career a result names gets one outfit (never "any outfit"): its trainee's outfits are reordered with the chosen
// one first, filled with one of hers when none covers a target (`anyOutfit`); her popup says which
function oneOutfit(t) {
  if (!t || t.outfitNote != null) return;
  const f = pickOutfit(t.id, t.outfits || []);
  t.outfits = f.list;
  t.anyOutfit = f.any;
  t.outfitNote = outfitNote(f);
}
/** The solver's answer to a career job: its CareerSearch, or a refusal (`'error' in m.r`).
 * @typedef {import('../solver.ts').SolverAnswer<CareerSearch>} CareerAnswer */
/** @type {(job: CareerJob, progress?: (p: import('../solver.ts').PlanProgress) => void, background?: boolean) => Promise<CareerAnswer>} */
export const runJob = async (job, progress, background = false) => {
  // (a career job: its report is a CareerSearch)
  const m = /** @type {CareerAnswer} */ (await engineRun(job, undefined, progress, background));
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
