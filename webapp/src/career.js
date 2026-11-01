// Career planner page: entry point. build.py bundles this and the modules it imports into dist/assets/career.<hash>.js,
// which dist/index.html loads.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (vets.js) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
// Here: the run machine (status line, job, search and plan), the answer, the pedigree, the career sections, the pool,
// settings and start. Its parts in career/:
//   roll.mjs      a roll: a random result of a career (no page state; the suite runs it in node)
//   analysis.js   the veteran analysis ("Try a run"): her draft, her comparison, and the state they keep
//   options.js    the options table ("Other careers": the plan's careers, Use this) and its Saves popup
import './errorlog.ts'; // first: it listens before any other module runs
import { $, $dialog, $input, $textarea, backdropCloses, el, fmtIn } from './dom.js';
import { ERA, SAVED } from './settings.js';
import { UMAS, SK, CAREER, RANKS, loadData } from './data.js';
import { state, goldGiven, writeSaved } from './state.js';
import {
  run as engineRun,
  restartEngine,
  engineBusy,
  enginePersist,
  engineSaveState,
  nextKill,
  stateKey,
} from './engine.js';
import { answerLoad, answerSave } from './statestore.js';
import { onSave, save } from './save.js';
import { EXTRA, jobTargets, ownedCards, rcardsOff } from './job.js';
import { umaAv, skillIcon, cardAv, runnerHTML, setRunner, scenLogo, srcFor } from './images.js';
import { initScenario, eraScenario, scenarioOf } from './scenario.js';
import { initRoster, renderRoster, noOutfit, pickOutfit, outfitNote } from './roster.js';
import { initCards, cardTitle, borLast, chainEndNote } from './cards.js';
import { initOwnedImport } from './owned.js';
import { initPopover, markOwn } from './popover.js';
import { openSched, tableCost, tableChance } from './agenda.js';
import { initSetup } from './setup.js';
import './gmtip.js'; // the game-style tooltips (both planner pages)
import { affEl, affTable as affTableOf } from './afftable.js';
import { pct, deckHint } from './odds.js';
import { initTargets, renderTargets } from './targets.js';
import {
  vets,
  plan,
  chosen,
  onVetsChange,
  changed,
  persist,
  vetById,
  umaName,
  sparksEl,
  winsBtn,
  winsText,
  changeUma,
  askSkills,
  sampleVets,
  newVet,
  importVets,
  isVeteran,
  asVeteran,
  addVets,
  removeVet,
  clearVets,
  dropMissing,
  legaciesJob,
  openUmaPicker,
  drawUmas,
  filterUmas,
} from './vets.js';
import { vetCell } from './vetcell.js';
import { wireMenu } from './sortmenu.js';
import { parseExport, parseTree } from './umadump.js';
import { initUmamoe, syncListings, umamoeButton } from './umamoesetup.js';
import { inspView, fitSparks, deckCardAt, openSkill } from './details.js';
import {
  renderSimulation,
  outdateComparison,
  stopComparison,
  showAnalysis,
  clearAnalysis,
  keepBaseline,
  keptPlan,
} from './career/analysis.js';
import { optionsEl } from './career/options.js';
import { unprovenWhy, searchCaveat, planCaveat } from './proofs.mjs';

const tName = (g) => {
  const s = SK.get(g) || { w: `Skill ${g}`, gd: null };
  return goldGiven(g) && s.gd ? s.gd : s.w;
};
const owned = (u) => state.roster[u.id] !== 'out' && !noOutfit(u);
export const btn = (label, cls, fn) => {
  const b = el('button', `btn${cls ? ` ${cls}` : ''}`, label);
  b.type = 'button';
  b.onclick = fn;
  return b;
};

// ---------- status line ----------
// One run per edit, one answer per run. A run has phases: the search (the best careers by odds), then, with the
// trainee left to the planner, the plan (its pick among them and every other owned uma). The status line names the
// phase at work; the answer (career, tree places, veteran analysis) is drawn once, when the run ends. Until then the
// previous answer stays on screen, dimmed, its actions off. The run machine's state is this one record, written only by
// the functions that drive a run (`schedule`, `search`, `planPhase`, `endRun`); everything else reads it.
export const run = {
  id: 0, // the run under way or last made; a newer edit's run makes an older one's work stale
  searching: false, // a run is at work (from the edit, through its pause, to its answer)
  started: false, // a job of this run is under way (the pause before it dims nothing: an edit that ends where it started runs none)
  phase: 'search', // 'search' | 'plan': the phase at work
  phaseStart: 0, // when the phase began (performance.now())
  lead: '', // the plan phase's lead: what the search found ("Weighed N careers in X s.")
  progress: '', // the plan's progress, as the solver reports it from inside the plan (`progText`)
  inflight: null, // the run under way: its job and places (`runKey`); null when none is
  tick: null, // the status line's one-second redraw while a job runs (setInterval)
  timer: null, // the pause between an edit and its search (setTimeout)
};
// a job's key: its JSON with the legacies' apart, kept with the array `legaciesJob` returns (the same until an edit)
let legaciesJson = { legacies: null, json: '' };
const jobKey = (job) => {
  if (legaciesJson.legacies !== job.legacies)
    legaciesJson = { legacies: job.legacies, json: JSON.stringify(job.legacies) };
  return `${JSON.stringify([ERA, { ...job, legacies: 0 }])}\n${legaciesJson.json}`;
};
// the places as a key: the trainee and the two parents (not how they were set, `fromTable`)
/** @type {(p?: LineagePlan) => string} */
export const placesKey = (p = plan) => JSON.stringify({ trainee: p.trainee, parents: p.parents });
const runKey = (job) => `${jobKey(job)}\n${placesKey()}`;
// the plan's phase reports (career/plan/mod.rs `index_rule`: phase, k of n) in words
const progText = ({ ph, k, n }) =>
  ph === 1
    ? 'searching the candidates'
    : ph === 2
      ? `scoring ${k + 1} of ${n} umas`
      : ph === 3
        ? 'careers on two full veteran parents'
        : ph === 4
          ? `weighing ${n} careers`
          : '';
/**
 * The status line's message once no run is at work.
 * @typedef {object} Status
 * @property {'loading'|'done'|'empty'|'error'} kind
 * @property {string} text
 * @property {{label: string, run: () => void}|null} [act]  a button after the text (e.g. Try sample veterans)
 *
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
 * @property {import('./vets.js').JobLegacy[]} legacies  every veteran, by legacy index (`legaciesJob`)
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
 * @property {import('./results.js').ResultDeck} deck
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
 * @typedef {import('./agenda.js').ScheduleRow & CareerScheduleFields} CareerScheduleRow
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
 * @property {{plan: string, pending: number, stable: boolean, contenders: string[], pick_varies?: boolean, pairs?: PairSearch|null, scoring_complete?: boolean}} [robust]
 *   the robustness check on the pick (plan/robust.rs), set by the page from its follow-up jobs (`checkPhase`): the pick
 *   it is of, the variants left, whether every tested variant scores the pick highest (`stable`), the careers (`keyOf`)
 *   the variants score highest (`contenders`), whether they disagree (`pick_varies`: the pick varies under these
 *   variants, a sensitivity, not a tie); the within-trainee pair search once it has run (`pairs`: null before, absent
 *   from a solver without it)
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
 * The career search's report (career/run.rs `search_output`), or its refusal (`error`).
 * @typedef {object} CareerSearch
 * @property {string} [error]
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
 *
 * The answer on show: the search's result for a job and the places it ran with (the plan's is `planState`).
 * @typedef {object} Answer
 * @property {CareerSearch} r  the solver's search report
 * @property {CareerJob} job  the job it answers
 * @property {string} key  `jobKey(job)`
 * @property {string} plan  the places, as JSON (`plan` when it ran)
 *
 * The answer's plan: none (the trainee chosen, or no run), done (`r`: the solver's `next` report; `job`: the plan job
 * it ran), or failed.
 * @typedef {{state: 'none', r?: null}} PlanNone
 * @typedef {{state: 'done', r: PlanReport, ms: number, job: CareerJob}} PlanDone
 * @typedef {{state: 'error', text: string}} PlanFailed
 * @typedef {PlanNone | PlanDone | PlanFailed} PlanState
 */
/** @import { LineagePlan, LineageNode } from './vets.js' */
/** @type {Status} */
let status = { kind: 'loading', text: 'Loading…' };
/** @type {Answer|null} */
export let last = null;
function paint() {
  const note = $('run-note');
  setRunner(plan.trainee || shown()?.trainee?.id); // the runner: the trainee you chose, else the career on show
  $('rhead').dataset.kind = run.searching ? 'busy' : status.kind;
  const dimmed = run.searching && run.started && !!shown(); // the previous answer, on screen until this run's
  $('results').classList.toggle('stale', dimmed || (!run.searching && ['error', 'empty'].includes(status.kind)));
  $('tree').classList.toggle('stale', dimmed);
  $('results').setAttribute('aria-busy', String(run.searching));
  if (run.searching) paintBusy(note);
  else {
    note.textContent = status.text + (status.kind === 'done' ? planNote() : '');
    // its actions, e.g. Import veterans and Try sample veterans (the last is #run-act)
    const acts = status.act ? [status.act].flat() : [];
    acts.forEach((a, i) => {
      const b = btn(a.label, i === acts.length - 1 ? 'primary' : '', a.run);
      if (i === acts.length - 1) b.id = 'run-act';
      note.append(' ', b);
    });
  }
}
// the status line during a run: the search's lead (in the plan phase), the phase at work, the plan's progress and the
// phase's seconds so far
function paintBusy(note) {
  const seconds = Math.floor((performance.now() - run.phaseStart) / 1000);
  note.textContent = run.phase === 'plan' && run.lead ? `${run.lead} ` : '';
  const line = el('span');
  line.innerHTML = runnerHTML();
  line.append(
    run.phase === 'plan' ? 'Planning your next career…' : 'Searching careers…',
    el('span', '', seconds >= 1 ? ` ${seconds} s` : ''),
  );
  /** @type {HTMLElement} */
  (line.lastChild).setAttribute('aria-hidden', 'true');
  // (the plan's progress, before the seconds; like them, not read out at every step)
  if (run.phase === 'plan' && run.progress) {
    const pg = el('span', 'runprog', ` (${run.progress})`);
    pg.setAttribute('aria-hidden', 'true');
    line.insertBefore(pg, line.lastChild);
  }
  note.appendChild(line);
}
// the plan's line: its time and the named assumptions its searches rest on (the job's, and its proven rows' own) that
// the search's line does not state (`planCaveat`), or why it failed
function planNote() {
  if (planState.state === 'error') return ` Plan not made: ${planState.text}`;
  if (planState.state !== 'done') return '';
  const plan = planState.r,
    assumes = [
      ...(plan.job_assumes || []),
      ...plan.options.filter((o) => o.exact && o.proven !== false).flatMap((o) => o.assumes || []),
    ];
  return ` Planned in ${(planState.ms / 1000).toFixed(1)} s.${planCaveat(assumes, last?.r || {})}`;
}
// every change: redraw now, search again after a short pause (a change during a search stops it)
function schedule() {
  // an edit that leaves the job as it is (a preset saved or renamed, a setting changed and back) asks for the answer
  // already coming or on show: a run under way goes on, and an answer on show stands (with her comparison, if one is
  // running); a job that can't be built (`Blocked`) goes on as any other edit
  let key = null;
  try {
    key = runKey(buildJob());
  } catch {
    /* the search says why */
  }
  if (
    key !== null &&
    (run.inflight
      ? run.inflight === key
      : !run.searching && last && `${last.key}\n${last.plan}` === key && planState.state !== 'error')
  )
    return;
  clearTimeout(run.timer);
  run.id++;
  run.inflight = null;
  // only a job in flight is cut short: an idle worker keeps what it holds for the next jobs (the last search, every
  // uma's best career, the proofs); during the pause before a search nothing runs, so a second edit kills nothing
  if (engineBusy()) restartEngine();
  // a comparison in flight is stopped with it: its panel goes back to idle, or a roll's quick figures stand unchecked
  // (an answer that stands keeps her)
  stopComparison();
  run.searching = true;
  run.started = false;
  run.phase = 'search';
  run.lead = '';
  run.phaseStart = performance.now();
  paint();
  run.timer = setTimeout(search, 600);
}

// ---------- the job ----------
const idxOf = (id) => vets.findIndex((v) => v.id === id);
class Blocked extends Error {
  constructor(kind, text, act) {
    super(text);
    this.kind = kind;
    this.act = act;
  }
}
// the borrowed-parent choice (Career setup): 'allow' (one per career, now and later), 'later' (none in this career; the
// plan's later careers still borrow), 'never'; the job's `borrowed_parent`, left out at 'allow' (the solver's default)
export const BORROW_CHOICES = { allow: 'Allowed', later: 'None left today', never: 'Never' };
const borrowChoice = () => (Object.hasOwn(BORROW_CHOICES, state.borrowPar) ? state.borrowPar : 'allow');
export function buildJob() {
  if (!state.targets.length) throw new Blocked('empty', 'No target skills: add them with Target skills above.');
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
    const k = v ? pairBlock(trainee, v, other) : '';
    if (k === 'same')
      throw new Blocked('error', `Both parents are ${umaName(v.chara)}: a career takes two different umas.`);
    if (k === 'both') throw new Blocked('error', 'Both parents are borrowed: a career can borrow one.');
    if (k === 'setting')
      throw new Blocked(
        'error',
        `${umaName(v.chara)} is borrowed, and Borrowed parent is set to ${BORROW_CHOICES[borrowChoice()]}: choose one of your veterans, or allow a borrowed parent.`,
      );
    if (k === 'trainee')
      throw new Blocked('error', `${umaName(plan.trainee)} can't be her own parent: choose another trainee or parent.`);
  }
  const exclude = UMAS.filter((u) => !owned(u) && u.id !== plan.trainee).map((u) => u.id);
  if (!plan.trainee && exclude.length >= UMAS.length)
    throw new Blocked('empty', 'No uma is owned: mark yours with Umas above.');
  const exclude_outfits = Object.entries(state.outfits)
    .filter(([, v]) => v === false)
    .map(([k]) => +k);
  const allPlaced = plan.trainee && fixed.length === 2;
  // a place left open: the K best careers by odds (their trainees are among the plan's stage candidates; the plan
  // scores every owned uma, `planPhase`)
  /** @type {CareerJob} */
  const job = {
    cmd: 'career',
    targets: jobTargets(),
    exclude,
    exclude_outfits,
    extra: EXTRA,
    owned_cards: ownedCards(),
    borrow: true,
    star_dist: state.rank.split(',').map(Number),
    scenario: state.scen || 'unity',
    ...rcardsOff(),
    legacies: legaciesJob(),
    trainees: plan.trainee ? [plan.trainee] : [],
    fix: plan.parents.filter(Boolean).map(idxOf),
    per_trainee: false,
    top: allPlaced ? 1 : ROLL_K,
  };
  if (borrowChoice() !== 'allow') job.borrowed_parent = borrowChoice();
  if (allPlaced && chosenApplies(job)) job.career = chosen.career.id;
  return job;
}
// the career Use this chose with its deck (vets.js `chosen`, saved with the plan): a builder's other career (an option
// with `alternative`: a search on its places would give her best deck there instead, but for the pick, searched on its
// pair before it is shown; listed with the plan's `builder_careers` above 1, its default 1, which the page keeps: none by
// default) or the pair search's recommendation (`pair_search`). It is sent only while it applies (`chosenApplies`): its
// places, the setup it was chosen on (`chosenSetup`: the job's settings, its targets and cards among them, its two
// parents' records and the solver's and data's fingerprints, engine.js `stateKey`) and its parents' places in the veteran list (the id names them by index) all as they were. One
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
let chosenDropped = null;
// a choice made with Use this: the dropped one's note goes
export function forgetDroppedChoice() {
  chosenDropped = null;
}
function dropStaleChosen() {
  if (!chosen.career) return;
  let applies = false;
  try {
    applies = buildJob().career === chosen.career.id;
  } catch {
    // settings that make no job (Blocked): the chosen career no longer applies
  }
  if (applies) return;
  chosenDropped = {
    places: chosen.career.places,
    text: 'The deck you chose with Use this before the reload no longer fits (its trainee, parents, their records, the targets or the card settings changed), so this is the deck the search finds.',
  };
  chosen.career = null;
  persist();
}
// a full veteran: a spark of every target (with full veteran parents: a fully looped parent)
export const isFull = (nd) => !!nd && state.targets.length > 0 && state.targets.every((t) => nd.sparks?.[t.group] > 0);
// a career on two full veteran parents: her own full veteran there is a fully looped parent (the plan says so; a search's row is read off the veterans)
export const looped = (r) => r.looped ?? r.parents.every((i) => isFull(vets[i]));
export const ROLL_K = 6; // careers a search returns with a place open (the plan's candidates; `buildJob`)
// the job of today's best career over everything (no place set, one result): the comparison's baseline
const baseOf = (job) => ({
  ...job,
  career: undefined,
  exclude: UMAS.filter((u) => !owned(u)).map((u) => u.id),
  trainees: [],
  fix: [],
  per_trainee: false,
  top: 1,
  rollout: undefined,
  p_now: undefined,
});
export const baseKey = (job) => JSON.stringify(baseOf(job));
// the answer's plan (the planner's candidates and its pick; none with the trainee chosen), and the entered veteran
// with its comparison (on demand)
/** @type {PlanState} */
export let planState = { state: 'none', r: null };
// a career's identity: the solver's `career` (her uma, her parents in order and her deck: two decks on the same parents
// are two careers), else (an answer from before it) her uma and parents
export const keyOf = (r) => r.career ?? `${r.trainee.id}:${r.parents.join(',')}`;
// the last plan made: while the career on show is one you chose on the same setup and veterans (its job without the
// places), its table stays, so you can weigh the plan's other careers or go back to its pick
/** @type {{base: string, places: LineagePlan, result: PlanReport}|null} base: `refBase` of its job; places: as it ran with
 *  (LineagePlan: vets.js); result: its `next` report */
let refPlan = null;
const refBase = (job) =>
  planKey({
    ...job,
    exclude: UMAS.filter((u) => !owned(u)).map((u) => u.id),
    trainees: [],
    fix: [],
    top: 0,
    career: undefined,
  });
export const refFor = () =>
  refPlan && last && planState.state !== 'done' && refPlan.base === refBase(last.job) ? refPlan : null;
// the last few answers, by job and places: going back to one draws it again without a run
const answers = new Map(),
  ANSWERS = 4;
// the answer the last visit drew, when it was for this setup (the solver's and data's fingerprints, the job, the places):
// put among the answers, so the first search draws it at once instead of planning (the solver is deterministic for its job)
async function preloadAnswer() {
  let job;
  try {
    job = buildJob();
  } catch {
    return;
  }
  const key = `${jobKey(job)}\n${placesKey()}`,
    saved = await answerLoad(ERA);
  if (saved?.key === `${stateKey()}\n${key}` && saved.value?.last) answers.set(key, saved.value);
}
const canon = (x) =>
  Array.isArray(x)
    ? x.map(canon)
    : x && typeof x === 'object'
      ? Object.fromEntries(
          Object.keys(x)
            .sort()
            .filter((k) => x[k] !== undefined)
            .map((k) => [k, canon(x[k])]),
        )
      : x;
// a plan job's `next`: nothing of its own; Compare's plan job of the same pool sends the same, so a run can take its
// answer (`planKept`). The check's follow-up jobs add `robust` and `pair_search` (`checkPhase`), which the solver leaves
// out of the key it finds the kept check by (`check_key`)
export const PLAN_NEXT = Object.freeze({});
export const planKey = (job) => {
  const { compare: _c, rollout: _r, p_now: _p, ...j } = job;
  return JSON.stringify(canon(j));
};
// every career a result names gets one outfit (never "any outfit"): its trainee's outfits are reordered with the chosen
// one first, filled with one of hers when none covers a target (`anyOutfit`); her popup says which
function oneOutfit(t) {
  if (!t || t.outfitNote != null) return;
  const f = pickOutfit(t.id, t.outfits || []);
  t.outfits = f.list;
  t.anyOutfit = f.any;
  t.outfitNote = outfitNote(f);
}
/** @type {(job: CareerJob, progress?: (p: import('./engine.js').PlanProgress) => void, background?: boolean) => Promise<{r: CareerSearch, ms: number}>} */
export const runJob = async (job, progress, background = false) => {
  const m = await engineRun(job, undefined, progress, background),
    r = m.r || {};
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
// (the loop members kept per setup until the plan stopped running the loop planner's search: the old store goes)
try {
  localStorage.removeItem('llp:loops');
} catch {
  // storage blocked: nothing was kept to remove
}
// A run: the search, then (with the trainee's place left to the planner) the plan, both on the one worker; the answer
// is drawn once, at the end. Two solver jobs: the plan's reuses the search's (the solver keeps its last search, so the
// plan job does none of that work again; a second job costs a few ms), and the page knows which phase is at work
// without the solver reporting it. A newer edit stops a run in either phase (`runId`); nothing of it is drawn.
async function search() {
  const id = ++run.id;
  outdateComparison();
  let job;
  try {
    job = buildJob();
  } catch (e) {
    endRun({
      status: { kind: e.kind || 'error', text: e.message, act: e.act },
      last: null,
      planState: { state: 'none' },
    });
    return;
  }
  // edits that end where they started (a setting changed and back, a star clicked twice): the job is the one whose
  // answer is on show (era, settings, veterans as the job carries them, and the same places), so it stands and nothing
  // runs (her comparison, if the edits cut one short, is hers to start again); a failed plan makes it run again
  const key = jobKey(job),
    places = placesKey(),
    answerKey = `${key}\n${places}`;
  if (last && last.key === key && last.plan === places && planState.state !== 'error') {
    run.searching = false;
    clearInterval(run.tick);
    renderAll();
    paint();
    resumeCheck(id);
    return;
  }
  const hit = answers.get(answerKey);
  if (hit) {
    answers.delete(answerKey);
    answers.set(answerKey, hit); // (the most recent last)
    ({ status, last, planState } = hit);
    const r = shown();
    showAnalysis(r);
    run.searching = false;
    clearInterval(run.tick);
    renderAll();
    paint();
    resumeCheck(id);
    return;
  }
  run.searching = true;
  run.started = true;
  run.inflight = answerKey;
  run.phase = 'search';
  run.lead = '';
  run.progress = '';
  run.phaseStart = performance.now();
  clearInterval(run.tick);
  run.tick = setInterval(paint, 1000);
  paint();
  const fail = (text) => {
    status = { kind: 'error', text };
    last = null;
    planState = { state: 'none' };
    clearAnalysis();
  };
  /** @type {PlanDone|null} the plan to check once it is shown (`checkPhase`) */
  let checkFor = null;
  try {
    const m = await runJob(job);
    if (id !== run.id) return;
    if (m.r.error) fail(`Not searched: ${m.r.error}`);
    else {
      const found = searchedText(m),
        searched = searchStatus(m, found);
      const best = m.r.results[0];
      const planned = best && !plan.trainee ? await planPhase(id, job, found) : { state: 'none' };
      if (planned === null) return; // a newer edit took over
      // the answer: the plan's pick (the best odds when no plan ran or it failed), and the veteran you got, drawn once
      // from it (a comparison on screen was for the old answer)
      status = searched;
      last = { r: m.r, job, key, plan: places };
      planState = planned;
      if (planned.state === 'done') {
        refPlan = { base: refBase(job), places: JSON.parse(places), result: planned.r };
        if (!planned.r.robust) checkFor = planned;
      }
      if (searched.kind === 'done' && planned.state !== 'error') {
        answers.set(answerKey, { status, last, planState });
        if (answers.size > ANSWERS) answers.delete(answers.keys().next().value);
        // kept across a reload too: the same setup draws it at once (`preloadAnswer`)
        answerSave(ERA, `${stateKey()}\n${answerKey}`, { status, last, planState });
      }
      const r = shown();
      showAnalysis(r);
      if (best && !job.trainees.length && !job.fix.length) keepBaseline(baseKey(job), best);
    }
  } catch (e) {
    if (id !== run.id) return;
    fail(e.message);
  }
  run.searching = false;
  run.inflight = null;
  clearInterval(run.tick);
  renderAll();
  paint();
  if (checkFor) checkPhase(id, checkFor);
}
// The robustness check on the pick, once it is shown: the plan job again with `next.robust: "next"`, one variant per
// job from the plan the worker keeps, as background jobs (an edit lets the variant under way finish and its job waits
// behind it, so the worker and what it keeps stay), until none is left, then one more for the within-trainee pair
// search (`next.pair_search`, sent on every check job: the report's `pairs`). Its report attaches only to the plan it was
// asked for: the same run and plan state, and the pick it names (`robust.plan`). Where the worker no longer keeps the
// plan (it was stopped or restarted: a stopped comparison, a solver error, a reload) the check is unavailable, and the
// table says so (`check_unavailable`).
/** @param {number} id  the run it belongs to @param {PlanDone} planned  its plan */
async function checkPhase(id, planned) {
  const current = () => id === run.id && planState === planned;
  // its report shows in the plan's table only: the tree and the veterans stay as they are (a redraw of the open
  // veterans dialog under a click in progress would lose the click, or land it on the dialog, which closes it)
  const redraw = () => {
    keepFocus(renderResults);
    paint();
  };
  const unavailable = () => {
    planned.r.check_unavailable = true;
    redraw();
  };
  for (;;) {
    let m = null;
    try {
      const job = {
        ...planned.job,
        next: { ...planned.job.next, robust: /** @type {'next'} */ ('next'), pair_search: true },
      };
      m = await Promise.race([runJob(job, undefined, true), nextKill().then(() => null)]);
    } catch {
      m = null;
    }
    if (!current()) return;
    // (a kill loses the check: the stored state keeps no check, so the next worker could not run it; REPORT N7)
    const check = m?.r.next?.robust;
    if (!check || check.unavailable || check.error) return unavailable();
    if (check.plan !== planned.r.outcomes?.career) return;
    const variantsWereDone = planned.r.robust?.pending === 0;
    planned.r.robust = check;
    if (check.pending > 0) continue;
    redraw();
    // (done once the pair search is in; a step after the last variant that brought none has nothing more to bring)
    if (checkComplete(check) || variantsWereDone) return;
  }
}
// a check that has nothing left to run: every variant done and the pair search in (`pairs`; absent where the solver has
// none)
/** @type {(check: PlanReport['robust']) => boolean} */
const checkComplete = (check) => check?.pending === 0 && check.pairs !== null;
// an unfinished check of the plan on show, resumed where its answer is shown again (`checkPhase`; a check cut short by
// an edit and its undoing, or by a newer answer): nothing where it is done or unavailable
function resumeCheck(id) {
  if (planState.state !== 'done' || planState.r.check_unavailable || checkComplete(planState.r.robust)) return;
  checkPhase(id, planState);
}
// a run that ends without searching (the job can't be built): its message, no answer
function endRun(answer) {
  run.searching = false;
  clearInterval(run.tick);
  ({ status, last, planState } = answer);
  clearAnalysis();
  renderAll();
  paint();
}
// the search's lead line: how many careers it weighed, in what time
function searchedText(m) {
  const n = m.r.candidates,
    seconds = (m.ms / 1000).toFixed(1);
  return n > 1 ? `Weighed ${n.toLocaleString()} careers in ${seconds} s.` : `Scored in ${seconds} s.`;
}
// the search's status: done, with a caveat (proofs.mjs `searchCaveat`) when the search could not prove the best odds'
// order (`proven_top`), or proved it only under a lower deck cap (`complete` false), saying what the report establishes
// of why, or proved it while a scored deck beat its class's assumed cap (the caps are assumptions the proofs rest on);
// or, when no career can land every target, which targets nothing can give (no veteran or parent carries their spark;
// no deck, outfit or event gives them either, or some career would have them)
/** @returns {Status} */
function searchStatus(m, found) {
  const cert = searchCaveat(m.r),
    dropped = chosenDropped?.places === placesKey() ? ` ${chosenDropped.text}` : '';
  if (m.r.results.length) return { kind: 'done', text: found + cert + dropped };
  const carried = new Set(
    vets.flatMap((v) => [v, ...v.parents]).flatMap((x) => Object.keys(x.sparks || {}).map(Number)),
  );
  const none = state.targets.filter((t) => !goldGiven(t.group) && !carried.has(t.group)).map((t) => tName(t.group));
  const why = none.length
    ? `: none of your veterans or their parents carries ${none.join(', ')}, and no deck or outfit gives ${none.length > 1 ? 'them' : 'it'}.`
    : ' with these veterans.';
  return { kind: 'error', text: `No career can land all ${state.targets.length} targets${why}${dropped}` };
}
// The plan (solver: career/plan/mod.rs `index_rule`, exact odds, no simulation): the same job with `next`, every owned uma's
// best career scored by the careers it saves toward a fully looped parent (the cheapest trio). The page reads `next`
// alone, so the solver skips the career reports. A trainee you chose leaves it nothing to pick (no plan phase).
// Returns the plan's state, or null when a newer edit (run `id` no longer current) took over.
async function planPhase(id, job, found) {
  run.phase = 'plan';
  run.lead = found;
  run.progress = '';
  run.phaseStart = performance.now();
  paint();
  const { rollout: _r, p_now: _p, ...plain } = job;
  let planned;
  try {
    // Compare may have run this very plan job already (`keptPlan`): take its answer instead of planning again
    const jp = { ...plain, next: { ...PLAN_NEXT }, no_reports: true },
      kept = keptPlan(planKey(jp));
    /** @type {{r: {next?: PlanReport, error?: string}, ms: number}} */
    const m2 = kept
      ? { r: { next: kept.r }, ms: kept.ms }
      : await runJob(jp, (p) => {
          if (id === run.id) {
            run.progress = progText(p);
            paint();
          }
        });
    if (id !== run.id) return null;
    const x = m2.r.next;
    planned =
      x && !x.error && x.options?.length
        ? { state: 'done', r: x, ms: m2.ms, job: jp }
        : { state: 'error', text: m2.r.error || x?.error || 'no option found.' };
  } catch (e) {
    if (id !== run.id) return null;
    planned = { state: 'error', text: e.message };
  }
  if (planned.state === 'done') engineSaveState(); // the worker's kept work, for the next page load
  return planned;
}

// ---------- the pedigree ----------
// the career on show: the top result while it belongs to the current plan
// (the plan's pick once it is made, else the highest odds)
export const shown = () =>
  last && last.plan === placesKey() && last.r.results[0]
    ? (planState.state === 'done' && planState.r.choice) || last.r.results[0]
    : null;
export const fmtN = (x) => Math.round(x).toLocaleString();
// which veteran sits in parent place k: the chosen one, else the planner's pick in the result on show
function parentAt(k, r) {
  const mine = plan.parents[k] && vetById(plan.parents[k]);
  if (mine) return { v: mine, pick: false };
  if (!r) return null;
  // the other place's chosen veteran (-1: none): the result's parent that isn't her fills this one
  const other = plan.parents[1 - k] ? idxOf(plan.parents[1 - k]) : -1;
  const i = other >= 0 ? r.parents.filter((x) => x !== other)[0] : r.parents[k];
  return i === undefined ? null : { v: vets[i], pick: true };
}
// a node of the pedigree: what it is (its header), then who; ghost: the planner's pick, not yours
function pnode(kind, label, ghost) {
  const n = el('div', `pn ${kind}${ghost ? ' ghost' : ''}`);
  const h = el('div', 'pnh');
  h.appendChild(el('span', 'pnl', label));
  n.appendChild(h);
  return n;
}
// The tree, as the game's legacy display: portraits only (the trainee; Legacy 1 blue and Legacy 2 pink, each the parent
// with her own two parents), each with its legacy number and affinity; a portrait opens the node popup (her sparks,
// G1 wins, choose or hand back the place, change who she is). The popup is refilled after every edit (`fillNode`).
// a click anywhere in a node's box opens it, as its portrait does (not on the box's own buttons or deck cards)
const wholeBox = (box, t) => {
  box.classList.add('pnclick');
  box.addEventListener('click', (e) => {
    if (!e.target.closest('button, a, .deck, input')) t.click();
  });
};
// a node's portrait button: her face, name and a line under it (sub, optional); key: its data-k
function tile(key, face, name, sub) {
  const b = el('button', 'ptile');
  b.type = 'button';
  b.dataset.k = key;
  b.setAttribute('aria-haspopup', 'dialog');
  b.appendChild(face);
  const nm = el('span', 'tnm');
  nm.appendChild(el('span', 'nm', name));
  if (sub) nm.appendChild(el('span', 'sub', sub));
  b.appendChild(nm);
  return b;
}
function traineeEl(r) {
  const id = plan.trainee || r?.trainee.id;
  const n = pnode('trainee', plan.trainee ? 'Your pick' : "Planner's pick", !plan.trainee);
  // the affinity she holds from her six legacies (its breakdown is in her popup)
  if (r && r.trainee.id === id) {
    const tot = legacyAff(r),
      a = affEl(tot, 'aff tot');
    a.title = `Affinity ${Math.round(tot)}, her six legacies: click her for the breakdown`;
    n.querySelector('.pnh').appendChild(a);
  }
  const sub = id && r && r.trainee.id === id ? r.trainee.outfits[0]?.label || null : null;
  const t = id
    ? tile('node:t', umaAv(id, r && r.trainee.id === id ? r.trainee.outfits[0]?.id : null, 96), umaName(id), sub)
    : tile('node:t', el('span', 'av empty'), 'Any uma', 'The planner tries every uma you own.');
  t.setAttribute('aria-label', `Trainee: ${id ? umaName(id) : 'the planner picks'}. Details`);
  t.onclick = () => openNode({ kind: 't' });
  n.appendChild(t);
  wholeBox(n, t);
  // her deck and her race schedule, each in its popup
  if (r && r.trainee.id === id) {
    const row = el('div', 'row tdeck');
    const db = btn('Deck', '', () => openDeck(r));
    db.id = 'deck-open';
    db.setAttribute('aria-haspopup', 'dialog');
    row.append(db, schedBtn(r));
    n.appendChild(row);
  }
  return n;
}
// her affinity, in all and legacy by legacy (afftable.js), from the report's six slots (career/inherit.rs `slots`);
// Legacy 1 and 2 follow the career's parents (a slot's legacy is her index among your veterans)
const legacyAff = (r) => r.slots.reduce((a, x) => a + x.aff, 0);
function affTable(r) {
  const nm = (x) => x.name || umaName(x.chara);
  return affTableOf(
    r.parents
      .map((ix) => r.slots.find((x) => x.kind === 'parent' && x.legacy === ix))
      .filter(Boolean)
      .map((p) => ({
        face: umaAv(p.chara || 0, null, 30),
        name: nm(p),
        rel: p.rel,
        race: p.race,
        aff: p.aff,
        gps: r.slots
          .filter((x) => x.kind === 'grandparent' && x.legacy === p.legacy)
          .sort((a, b) => a.gp - b.gp)
          .map((g) => ({ face: umaAv(g.chara || 0, null, 24), name: nm(g), rel: g.rel, race: g.race, aff: g.aff })),
      })),
  );
}
function familyEl(k, r) {
  const fam = el('div', 'pf');
  // the game's legacy box: "Legacy 1" blue, "Legacy 2" pink, the parent with her own two parents
  fam.appendChild(el('span', 'pflab', `Legacy ${k + 1}`));
  const at = parentAt(k, r);
  const n = el('div', `pn parent${at?.pick ? ' ghost' : ''}`);
  if (at?.v.borrowed) n.classList.add('bor');
  let t;
  if (at) {
    const v = at.v;
    t = tile(
      `node:p${k}`,
      umaAv(v.chara, v.outfit, 64),
      umaName(v.chara),
      v.lender
        ? `uma.moe · ${v.lender.name || 'trainer'}`
        : `Veteran #${idxOf(v.id) + 1}${v.borrowed ? ' · borrowed' : v.src === 'hand' ? ', made here' : ''}`,
    );
    t.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Details`);
  } else {
    t = tile(`node:p${k}`, el('span', 'av empty'), vets.length >= 2 ? 'The planner picks' : 'No parent yet', null);
    t.setAttribute('aria-label', `Parent ${k + 1}: none yet. Details`);
  }
  t.onclick = () => openNode({ kind: 'p', k });
  n.appendChild(t);
  wholeBox(n, t);
  fam.appendChild(n);
  // her own parents: the trainee's grandparents
  if (at) {
    const gk = el('div', 'pgs');
    at.v.parents.forEach((g, j) => {
      const gn = el('div', 'pn gp');
      const gt = tile(
        `node:p${k}g${j}`,
        umaAv(g.chara || 0, g.outfit, 36),
        g.chara ? umaName(g.chara) : 'Unknown: pick her',
        null,
      );
      gt.setAttribute('aria-label', `Grandparent: ${g.chara ? umaName(g.chara) : 'unknown'}. Details`);
      gt.onclick = () => openNode({ kind: 'g', k, j });
      gn.appendChild(gt);
      wholeBox(gn, gt);
      gk.appendChild(gn);
    });
    fam.appendChild(gk);
  }
  return fam;
}
// the node popup (game style): one dialog, refilled on every redraw while open
// nodeOpen, the node on show: {kind: 't'} the trainee, {kind: 'p', k} parent place k, {kind: 'g', k, j} parent k's
// parent j, {kind: 'v', id} a veteran of the pool
let nodeDlg = null,
  nodeOpen = null;
// the data-k of the tree's (or the pool's) button for a node
const nodeKey = (spec) =>
  spec.kind === 't'
    ? 'node:t'
    : spec.kind === 'p'
      ? `node:p${spec.k}`
      : spec.kind === 'v'
        ? `node:v${spec.id}`
        : `node:p${spec.k}g${spec.j}`;
export function openNode(spec) {
  if (!nodeDlg) {
    nodeDlg = el('dialog', 'gmdlg nodedlg');
    nodeDlg.id = 'node-dlg';
    nodeDlg.setAttribute('aria-labelledby', 'node-title');
    const h = el('h3', 'gmdhdr');
    h.id = 'node-title';
    const body = el('div', 'gmdbody');
    body.id = 'node-body';
    const f = el('div', 'gmdfoot');
    const c = el('button', 'btn gmclose', 'Close');
    c.type = 'button';
    c.onclick = () => nodeDlg.close();
    f.appendChild(c);
    nodeDlg.append(h, body, f);
    nodeDlg.addEventListener('close', () => {
      const sp = nodeOpen;
      nodeOpen = null;
      const key = sp && nodeKey(sp);
      const e = key && document.querySelector(`[data-k="${key}"]`);
      if (e && !document.querySelector('dialog[open]')) e.focus();
    });
    document.body.appendChild(nodeDlg);
  }
  nodeOpen = spec;
  fillNode();
  if (!nodeDlg.open) nodeDlg.showModal();
}
// a dialog of its own from the popup (the uma or parent picker): the popup closes first
const fromNode = (fn) => () => {
  if (nodeDlg?.open) nodeDlg.close();
  fn();
};
// Refill the popup for the node on show (`nodeOpen`): its title and body, and its actions in the footer. A node that is
// gone (a removed veteran, a grandparent whose parent place emptied) closes it instead.
function fillNode() {
  if (!nodeOpen || !nodeDlg) return;
  const r = shown(),
    body = $('node-body');
  body.textContent = '';
  const sp = nodeOpen;
  const acts = el('div', 'row tacts');
  const fill = { t: fillTraineeNode, p: fillParentNode, v: fillVeteranNode }[sp.kind] || fillGrandparentNode;
  if (!fill(sp, r, body, acts)) {
    nodeDlg.close();
    return;
  }
  // the popup's actions sit in its footer beside Close (no row of their own)
  const foot = nodeDlg.querySelector('.gmdfoot');
  for (const e of foot.querySelectorAll('.nact')) e.remove();
  for (const b of [...acts.children]) {
    b.classList.add('nact');
    foot.appendChild(b);
  }
  fitSparks(nodeDlg);
}
// Each node kind's popup fills the title, `body` (its content) and `acts` (its actions); false: the node is gone.
// The trainee: who she is, her outfit, her affinity's breakdown; choose her or hand her back to the planner.
function fillTraineeNode(_sp, r, body, acts) {
  $('node-title').textContent = plan.trainee ? 'Trainee' : "Trainee · planner's pick";
  const id = plan.trainee || r?.trainee.id;
  if (id) {
    const w = el('div', 'twho'),
      mine = r && r.trainee.id === id;
    const who = el('div');
    who.append(el('div', 'nm', umaName(id)));
    if (mine && r.trainee.outfits[0]) who.append(el('div', 'ctt', r.trainee.outfits[0].label));
    if (mine && r.trainee.outfitNote) who.append(el('div', 'onote', r.trainee.outfitNote));
    w.append(umaAv(id, mine ? r.trainee.outfits[0]?.id : null, 72), who);
    body.appendChild(w);
  } else body.appendChild(el('p', 'tempty', 'The planner tries every uma you own.'));
  if (id && r && r.trainee.id === id) body.appendChild(affTable(r));
  acts.appendChild(
    btn(
      plan.trainee ? 'Change trainee' : 'Choose trainee',
      'primary',
      fromNode(() =>
        openUmaPicker(
          'Trainee: which uma?',
          (u) => {
            plan.trainee = u;
            plan.fromTable = false; // (chosen by hand: it stays)
            changed();
          },
          owned,
        ),
      ),
    ),
  );
  if (plan.trainee)
    acts.appendChild(
      btn('Let the planner pick', '', () => {
        plan.trainee = null;
        changed();
      }),
    );
  return true;
}
// A parent place: the veteran in it (chosen or the planner's pick) with her sparks and her parents', all editable.
function fillParentNode({ k }, r, body, acts) {
  const at = parentAt(k, r);
  $('node-title').textContent = `Legacy ${k + 1} · parent ${k + 1}${at?.pick ? " · planner's pick" : ''}`;
  if (at) {
    const v = at.v,
      i = idxOf(v.id),
      w = el('div', 'twho');
    const pb = el('button', 'facebtn');
    pb.type = 'button';
    pb.append(umaAv(v.chara, v.outfit, 64));
    pb.title = 'Choose another parent';
    pb.setAttribute('aria-label', `Parent ${k + 1}: ${umaName(v.chara)}. Choose another parent`);
    pb.onclick = fromNode(() => openParentPicker(k));
    const nm = el('div', 'tnm');
    nm.append(
      nameCover(v),
      el(
        'div',
        'sub',
        v.lender
          ? `uma.moe · ${v.lender.name || 'trainer'}`
          : `Veteran #${i + 1}${v.src === 'hand' ? ', made here' : ''}${v.borrowed ? ', borrowed' : ''}`,
      ),
    );
    w.append(pb, nm, winsEl(v, umaName(v.chara), `p${k}`, !listed(v)));
    // borrowed from uma.moe: her trainer to follow in the game before the career starts
    if (v.lender) w.appendChild(copyIdBtn(v, 'sm'));
    body.append(w);
    // her Inspiration view, as the card's: her sparks, then her parents' (the trainee's grandparents), all editable
    const iv = el('div', 'ud udbare');
    iv.appendChild(
      inspView({
        d: v,
        key: `p${k}`,
        onEdit: changed,
        parentKey: (j) => `p${k}g${j}`,
        onEditParent: changed,
        header: false,
        readOnly: listed(v),
      }),
    );
    body.appendChild(iv);
  } else
    body.appendChild(el('p', 'tempty', vets.length >= 2 ? 'The planner picks from your veterans.' : 'No parent yet.'));
  acts.appendChild(
    btn(
      plan.parents[k] ? 'Change parent' : 'Choose parent',
      'primary big',
      fromNode(() => openParentPicker(k)),
    ),
  );
  if (plan.parents[k])
    acts.appendChild(
      btn('Let the planner pick', '', () => {
        plan.parents[k] = null;
        changed();
      }),
    );
  return true;
}
// an uma.moe listing (in the pool, not yours): shown in the popups, never edited
const listed = (v) => v.src === 'umamoe';
// her face: a button that changes who she is, or (a listed veteran) just her face
const faceEl = (nd, size, label, change) => {
  if (!change) return umaAv(nd.chara || 0, nd.outfit, size);
  const b = el('button', 'facebtn');
  b.type = 'button';
  b.append(umaAv(nd.chara || 0, nd.outfit, size));
  b.title = `${label}: ${umaName(nd.chara)}. Change who she is`;
  b.setAttribute('aria-label', b.title);
  b.onclick = fromNode(() => changeUma(nd, label));
  return b;
};
// her G1 wins: the button that edits them, or (a listed veteran) their count
const winsEl = (nd, who, key, edit) => (edit ? winsBtn(nd, who, key) : el('span', 'vwins', winsText(nd)));
// the trainer to follow in the game for an uma.moe veteran: a Copy trainer ID button
function copyIdBtn(v, cls = '') {
  const b = btn('Copy ID', cls, async () => {
    try {
      await navigator.clipboard.writeText(v.lender.id);
      b.textContent = 'Copied';
    } catch {
      b.textContent = v.lender.id;
    }
  });
  b.title = `${v.lender.name}: ${v.lender.id}`;
  return b;
}
// her name with how many of the targets her own sparks carry, "6/8" (an n/n veteran carries all n)
function nameCover(v) {
  const groups = state.targets.map((t) => t.group),
    x = groups.filter((g) => (v.sparks?.[g] || 0) > 0).length,
    n = el('div', 'nm', umaName(v.chara)),
    c = el('span', 'vxy', `${x}/${groups.length}`);
  c.title = `Her sparks carry ${x} of your ${groups.length} targets`;
  n.append(' ', c);
  return n;
}
// (her trainer's followers too: at 1,000 the veteran can't be borrowed by following)
const lenderText = (l) =>
  `uma.moe · ${l.name || 'trainer'} · ${l.followers >= 1000 ? 'max followers' : `${l.followers} follower${l.followers === 1 ? '' : 's'}`}`;
// where a veteran came from (entered from a career here, an uma.moe tree, an import, made here)
const VET_SOURCE = {
  entered: 'Entered here',
  simulated: 'Entered here',
  tree: 'From uma.moe',
  umamoe: 'From uma.moe',
  import: 'Imported',
  hand: 'Made here',
  sample: 'Sample',
};
// A veteran of yours (the pool, Career Info's legacies): her sparks and her parents', editable in place.
function fillVeteranNode({ id }, _r, body) {
  const title = veteranView(id, body);
  if (title) $('node-title').textContent = title;
  return !!title;
}
// her view (the popup's, and beside the list in Veteran Umamusume): her face, name, targets carried, wins, where she
// came from; Yours / Guest and Remove; her sparks and her parents'. Returns the title, '' when she is gone
function veteranView(id, body) {
  const i = idxOf(id),
    v = vets[i];
  if (!v) return '';
  const edit = !listed(v),
    w = el('div', 'twho');
  const src = VET_SOURCE[v.src];
  w.append(
    faceEl(v, 56, `Veteran #${i + 1}`, edit),
    nameCover(v),
    winsEl(v, `Veteran #${i + 1}, ${umaName(v.chara)}`, `v${v.id}`, edit),
  );
  if (src)
    w.appendChild(
      el('span', v.src === 'tree' || v.src === 'umamoe' ? 'srctag' : 'simtag', v.lender ? lenderText(v.lender) : src),
    );
  if (v.lender) w.appendChild(copyIdBtn(v, 'sm'));
  body.appendChild(w);
  // yours or borrowed (she moves to that tab of Veteran Umamusume), and Remove
  if (edit) {
    const row = el('div', 'row vctl'),
      seg = el('div', 'seg sm');
    seg.setAttribute('role', 'group');
    seg.setAttribute('aria-label', `Veteran #${i + 1}: yours or a guest`);
    for (const [b, l] of /** @type {[boolean, string][]} */ ([
      [false, 'Yours'],
      [true, 'Guest'],
    ])) {
      const x = el('button', '', l);
      x.type = 'button';
      x.setAttribute('aria-pressed', String(v.borrowed === b));
      x.dataset.k = `v${v.id}:own${b ? 'bor' : ''}`;
      x.onclick = () => {
        if (v.borrowed === b) return;
        v.borrowed = b;
        changed();
      };
      seg.appendChild(x);
    }
    row.append(
      seg,
      btn('Remove', 'vdel', () => {
        const index = idxOf(v.id);
        if (index < 0) return;
        if (nodeDlg?.open) nodeDlg.close();
        removeVet(index);
        $('v-msg').textContent = `Removed ${umaName(v.chara)} from your veterans.`;
      }),
    );
    body.appendChild(row);
  }
  const iv = el('div', 'ud udbare');
  iv.appendChild(
    inspView({
      d: v,
      key: `v${v.id}`,
      onEdit: changed,
      parentKey: (j) => `v${v.id}g${j}`,
      onEditParent: changed,
      header: false,
      readOnly: !edit,
    }),
  );
  body.appendChild(iv);
  // her parents: who they are and their G1 wins
  const pr = el('div', 'vpars');
  v.parents.forEach((g, j) => {
    const x = el('div', 'twho'),
      lab = `Parent ${j + 1} of #${i + 1}`;
    x.append(
      faceEl(g, 36, lab, edit),
      el('span', 'nm', g.chara ? umaName(g.chara) : 'Unknown parent'),
      winsEl(g, `${lab}, ${umaName(g.chara)}`, `v${v.id}g${j}`, edit),
    );
    pr.appendChild(x);
  });
  body.appendChild(pr);
  return v.lender ? `uma.moe · ${v.lender.name || 'trainer'}` : `Veteran #${i + 1}${v.borrowed ? ' · guest' : ''}`;
}
// A grandparent (parent k's parent j): who she is, her sparks and G1 wins.
function fillGrandparentNode({ k, j }, r, body) {
  const at = parentAt(k, r),
    g = at?.v.parents[j];
  $('node-title').textContent = `Legacy ${k + 1} · grandparent`;
  if (!g) return false;
  const edit = !listed(at.v),
    w = el('div', 'twho');
  w.append(
    faceEl(g, 56, 'Grandparent', edit),
    el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'),
    winsEl(g, umaName(g.chara), `p${k}g${j}`, edit),
  );
  const iv = el('div', 'ud udbare');
  iv.appendChild(
    inspView({ d: { ...g, parents: [] }, key: `p${k}g${j}`, onEdit: changed, header: false, readOnly: !edit }),
  );
  // (a copy without parents for the view; her sparks object is hers: edits land on her)
  if (edit) iv.querySelector('.simskills').replaceChildren(sparksEl(g, `p${k}g${j}`, changed));
  body.append(w, iv);
  return true;
}
// The next career as the player sets it up in the game, in the game's order: the odds, then whom to follow (a borrow
// from uma.moe), the trainee, her two legacies, the deck, and each target's chance to land
function renderTree() {
  const r = shown(),
    box = $('tree');
  box.textContent = '';
  $('lodds').replaceChildren(...(r ? verdictEl(r).childNodes : []));
  const t = sectionEl('ltrainee', 'Trainee');
  t.appendChild(traineeEl(r));
  box.appendChild(t);
  const lg = sectionEl('llegacies', 'Legacies'),
    k = el('div', 'pk');
  k.append(familyEl(0, r), familyEl(1, r));
  lg.appendChild(k);
  box.appendChild(lg);
  if (r && r.trainee.id === (plan.trainee || r.trainee.id)) {
    box.append(targetsEl(r));
    if (deckDlg?.open) fillDeck(r);
  }
  if (nodeDlg?.open) fillNode();
}
// the career's odds of landing every target and Try a run, beside the title
function verdictEl(r) {
  const v = el('div', 'lodds');
  // Try a run: a roll of the veteran this career leaves and her comparison, in a popup
  const tr = btn('Try a run', '', () => {
    renderSimulation();
    $dialog('sim-dlg').showModal();
  });
  tr.id = 'sim-try';
  tr.setAttribute('aria-haspopup', 'dialog');
  tr.title = 'Roll a random result of this career and see whether the veteran it leaves helps your loop';
  v.append(oddsBtn(r), tr);
  return v;
}

// ---------- the parent picker ----------
let pickK = 0;
function vpTab(t) {
  for (const x of ['vets', 'new', 'auto']) {
    $(`vp-tab-${x}`).setAttribute('aria-selected', String(x === t));
    $(`vp-${x}`).hidden = x !== t;
  }
  if (t === 'new') {
    $input('vp-filter').value = '';
    drawNew();
    $input('vp-filter').focus();
  }
}
const drawNew = () =>
  drawUmas($('vp-umas'), $input('vp-filter').value, (id) => {
    $dialog('vpick-dlg').close();
    askSkills(
      umaName(id),
      false,
      (sp) => {
        const v = newVet(id, sp || {});
        plan.parents[pickK] = v.id;
        changed();
      },
      true,
    );
  });
// The parent rules for veteran v beside the other parent o (none: no other) with trainee t (0: none), the page's form
// of the solver's `Pool::pair_ok` and the Borrowed parent setting: the first rule v breaks ('trainee': she is the
// trainee's uma; 'same': the other parent's; 'both': both borrowed; 'setting': borrowed while the setting allows none),
// or ''. The one place the parent picker (`blockedFor`) and the job (`buildJob`) read them
function pairBlock(t, v, o) {
  if (t && v.chara === t) return 'trainee';
  if (o && o.chara === v.chara) return 'same';
  if (o?.borrowed && v.borrowed) return 'both';
  if (v.borrowed && borrowChoice() !== 'allow') return 'setting';
  return '';
}
// why veteran v can't go in place k (or '')
const BLOCKED = {
  trainee: 'the trainee herself',
  same: 'same uma as the other parent',
  both: 'the other parent is borrowed',
};
function blockedFor(v, k) {
  const why = pairBlock(plan.trainee, v, plan.parents[1 - k] && vetById(plan.parents[1 - k]));
  return why === 'setting' ? `borrowed (Borrowed parent: ${BORROW_CHOICES[borrowChoice()]})` : BLOCKED[why] || '';
}
function openParentPicker(k) {
  pickK = k;
  $('vpick-title').textContent = `Choose parent ${k + 1}`;
  const box = $('vp-list');
  box.textContent = '';
  let headed = false;
  vets.forEach((v, i) => {
    if (v.src === 'umamoe' && !headed) {
      box.appendChild(el('h4', 'vpgh', 'uma.moe'));
      headed = true;
    }
    const why = blockedFor(v, k),
      cur = plan.parents[k] === v.id;
    const b = el('button', `vpt${cur ? ' cur' : ''}${v.borrowed ? ' bor' : ''}`);
    b.type = 'button';
    b.disabled = !!why;
    const top = el('div', 'vpth');
    top.append(umaAv(v.chara, v.outfit, 48));
    const nm = el('div');
    nm.append(
      el('div', 'nm', umaName(v.chara)),
      el(
        'div',
        'sub',
        v.lender
          ? `${v.lender.name}${cur ? ', chosen' : ''}`
          : `#${i + 1}${v.borrowed ? ', borrowed' : ''}${cur ? ', chosen' : ''}`,
      ),
    );
    top.appendChild(nm);
    b.appendChild(top);
    const tg = state.targets.map((t) => t.group),
      have = tg.filter((g) => v.sparks[g]),
      stars = have.reduce((a, g) => a + v.sparks[g], 0);
    b.appendChild(el('div', 'vpts', `${have.length} of ${tg.length} target skills, ${stars}★`));
    const gp = el('div', 'vptg');
    gp.append('Parents: ');
    for (const g of v.parents) {
      gp.append(umaAv(g.chara || 0, g.outfit, 22));
    }
    gp.append(` ${v.parents.map((g) => (g.chara ? umaName(g.chara) : 'unknown')).join(', ')}`);
    b.appendChild(gp);
    if (why) b.appendChild(el('div', 'vptw', `Can't: ${why}`));
    b.onclick = () => {
      plan.parents[k] = v.id;
      plan.fromTable = false; // (chosen by hand: it stays)
      $dialog('vpick-dlg').close();
      changed();
    };
    box.appendChild(b);
  });
  if (!vets.length)
    box.innerHTML = '<p class="hint">No veterans yet: import them in the paddock below, or make a new one.</p>';
  vpTab(vets.length ? 'vets' : 'new');
  $dialog('vpick-dlg').showModal();
}

// ---------- the career: odds, deck, targets, schedule, after this career, other picks ----------
// a section of the results, headed by its title
export const sectionEl = (cls, title) => {
  const s = el('section', `rbsec ${cls}`);
  s.appendChild(el('h3', '', title));
  return s;
};
// the career's odds of landing all n (each count is under the targets: countsEl)
function oddsBtn(r) {
  const n = state.targets.length,
    b = el('span', 'udodds');
  b.id = 'odds-open';
  b.append(el('b', '', fmtIn(r.p_all)), el('span', '', ` to land all ${n}`));
  b.title = `The chance this career lands all ${n} targets (each count under the targets)`;
  return b;
}
// the chance of landing exactly k of the n targets: exact over the landed sets (outcome_sets), else each target on its
// own odds (over 12 targets); the all-n and n-1-or-better figures are the solver's exact ones
const popCount = (mask) => {
  let c = 0;
  for (let x = mask; x; x &= x - 1) c++;
  return c;
};
function landedCounts(r) {
  const n = state.targets.length,
    exactly = new Array(n + 1).fill(0);
  if (Array.isArray(r.outcome_sets)) {
    // outcome_sets[mask]: the chance of landing exactly the targets in mask (bit i: target i)
    r.outcome_sets.forEach((p, mask) => {
      exactly[popCount(mask)] += p;
    });
    return exactly;
  }
  // each target on its own odds: the count's distribution built one target at a time (dist[c]: c landed so far)
  let dist = [1];
  for (const t of r.targets) {
    const next = new Array(dist.length + 1).fill(0);
    dist.forEach((x, c) => {
      next[c] += x * (1 - t.p);
      next[c + 1] += x * t.p;
    });
    dist = next;
  }
  dist.forEach((x, c) => {
    exactly[c] = x;
  });
  return exactly;
}
// the deck as the loop planner shows it: card art and name; a click opens the card's panel (owned or not, GameTora)
export function deckEl(r, title = '') {
  const d = r.deck;
  const dk = el('div', 'deck');
  dk.innerHTML = `${title ? '<div class="dtitle"><b></b></div>' : ''}<div class="dcards"></div>`;
  if (title) dk.querySelector('.dtitle b').textContent = title;
  const box = dk.querySelector('.dcards');
  borLast(d.cards).forEach((i) => {
    const { base, borrowed: bor, card: c } = deckCardAt(d, i);
    const endAt = (d.end_chain_at || [])[i] || 0,
      pick = (d.choices || [])[i] || (c?.pri ? c.pri.join(', ') : '');
    const t = c ? cardTitle(c) : { nm: base, ti: '' },
      tag = bor ? 'borrowed' : c?.pal ? 'pal' : '';
    const full =
      t.nm +
      (t.ti ? ` · ${t.ti}` : '') +
      (tag ? ` (${tag})` : '') +
      (pick ? ` · prioritise ${pick}` : '') +
      (endAt ? ` · ${chainEndNote(c, endAt)}` : '');
    const b = el('button', `dc${bor ? ' bor' : c?.pal ? ' pal' : ''}`);
    b.type = 'button';
    if (c) b.dataset.cid = String(c.id);
    b.dataset.name = t.nm;
    if (endAt) b.dataset.end = String(endAt);
    b.title = full;
    b.setAttribute('aria-label', full);
    b.setAttribute('aria-haspopup', 'dialog');
    b.setAttribute('aria-expanded', 'false');
    b.append(cardAv(c || null, 96, false), el('span', 'dn', full));
    if (pick) b.appendChild(el('i', '', `prioritise ${pick}`));
    if (endAt) b.appendChild(el('i', '', chainEndNote(c, endAt)));
    box.appendChild(b);
  });
  return dk;
}
// her deck in its popup (the game's dialog): the six cards with their priority and chain notes
let deckDlg = null;
function openDeck(r) {
  fillDeck(r);
  deckDlg.showModal();
}
// (refilled while open when a new answer comes: a card marked not owned there searches again)
function fillDeck(r) {
  // (only when the deck changed: a redraw with the same deck keeps a card's open panel)
  const key = `${keyOf(r)}|${JSON.stringify(r.deck.cards)}`;
  if (deckDlg && deckDlg._key === key && deckDlg.childElementCount) return;
  if (!deckDlg) {
    deckDlg = el('dialog', 'gmdlg deckdlg');
    deckDlg.id = 'deck-dlg';
    deckDlg.setAttribute('aria-labelledby', 'deck-title');
    deckDlg.addEventListener('close', () => $('deck-open')?.focus());
    backdropCloses(deckDlg);
    document.body.appendChild(deckDlg);
  }
  deckDlg._key = key;
  deckDlg.textContent = '';
  const h = el('h3', 'gmdhdr', 'Deck');
  h.id = 'deck-title';
  const body = el('div', 'gmdbody'),
    dk = deckEl(r);
  dk.id = 'tree-deck';
  body.appendChild(dk);
  const f = el('div', 'gmdfoot');
  f.appendChild(btn('Close', 'gmclose', () => deckDlg.close()));
  deckDlg.append(h, body, f);
}
// the deck cards that give target group g: by a hint, or by an event (a gold one, at its chain link)
function cardsGiving(r, g) {
  const out = [];
  for (let i = 0; i < r.deck.cards.length; i++) {
    const c = deckCardAt(r.deck, i).card;
    if (!c) continue;
    const ways = [];
    if ((c.whites || []).includes(g)) ways.push('hint');
    for (const [group, rarity, link] of c.ev || [])
      if (group === g) ways.push(`${rarity > 1 ? 'gold ' : ''}event${link ? ` at link ${link}` : ''}`);
    if (ways.length) out.push({ card: c, how: [...new Set(ways)].join(', ') });
  }
  return out;
}
// her six legacies (each parent, then her two parents), and whether each carries target group g
function legaciesCarrying(r, g) {
  const out = [];
  for (const ix of r.parents) {
    const v = vets[ix];
    if (!v) continue;
    out.push({ chara: v.chara, outfit: v.outfit, carries: v.sparks?.[g] > 0 });
    for (const p of v.parents) out.push({ chara: p.chara || 0, outfit: p.outfit, carries: p.sparks?.[g] > 0 });
  }
  return out;
}
// the targets, lowest first, one aligned row each: the game's skill tile with her chance to land its spark (tap: Skill
// Details, where it comes from with each chance); her chance to get the skill on the game's bond gauge, gold for its
// gold skill and white for the white alone (every bar the same length); the chance her legacies pass it on (the game's
// three uma figures); what else gives it (the deck's cards with their chance, her outfit, her own events, the
// scenario). Under them the chance of each count of targets, as columns.
function targetsEl(r) {
  const s = sectionEl('rtgt', 'Targets'),
    list = el('ul', 'ctgs'),
    ico = srcFor('ui', 'ico_legacies');
  for (const t of [...r.targets].sort((a, b) => a.p - b.p)) {
    const b = t.buy || {},
      gold = t.gold || (b.land_gold || 0) > (b.land_white || 0),
      li = el('li', 'ctg'),
      tile = el('button', 'udsk'),
      legacies = legaciesCarrying(r, t.group),
      cards = cardsGiving(r, t.group),
      deckP = deckHint(b.deck_gold || 0, b.deck_white || 0);
    tile.type = 'button';
    tile.setAttribute('aria-haspopup', 'dialog');
    tile.setAttribute('aria-label', `${tName(t.group)}: lands ${pct(t.p)}. Where it comes from`);
    const icon = skillIcon(t.group, false);
    if (icon) tile.appendChild(icon);
    const lands = el('b', 'tgp', pct(t.p));
    lands.title = `Lands as a spark: ${pct(t.p)}`;
    tile.append(el('span', 'udskn', tName(t.group)), lands);
    tile.onclick = () =>
      openSkill({ b, name: tName(t.group), icon: () => skillIcon(t.group, gold), gold, legacies, cards }, tile);
    // the bar: one pill as long as her chance to get it, gold for its gold skill, the rest white
    const bar = el('span', 'tgbar'),
      bg = b.gold || 0,
      bw = b.white || 0,
      fill = el('span', 'tgfill'),
      g = el('i', 'g');
    fill.style.width = `${Math.round((bg + bw) * 1000) / 10}%`;
    g.style.width = `${bg + bw > 0 ? Math.round((bg / (bg + bw)) * 1000) / 10 : 0}%`;
    fill.append(g, el('i', 'w'));
    bar.appendChild(fill);
    bar.title = `Acquired: ${pct(bg + bw)} (gold ${pct(bg)}, white only ${pct(bw)})`;
    const legs = el('span', 'tgfig');
    if (ico) legs.appendChild(Object.assign(el('img', 'tgico'), { src: ico, alt: '' }));
    legs.appendChild(el('b', '', String(Math.round(t.q * 100))));
    legs.title = `Inherited: ${pct(t.q)}`;
    // what else gives it: the deck's cards with their chance; her outfit, her own events and the scenario outright
    const deck = el('span', 'tgfig tgdk');
    if (cards.length) {
      for (const { card } of cards) deck.appendChild(cardAv(card, 20));
      deck.appendChild(el('b', '', String(Math.round(deckP * 100))));
      deck.title = `Deck: ${pct(deckP)}`;
    }
    if (b.outfit) {
      const o = umaAv(r.trainee.id, r.trainee.outfits[0]?.id, 20);
      o.title = `Her outfit gives its ${b.outfit === 2 ? 'gold skill' : 'white'}`;
      deck.appendChild(o);
    }
    if ((b.own_event || 0) > 0.005 && !b.outfit) {
      const o = umaAv(r.trainee.id, null, 20);
      o.title = `Her own events: ${pct(b.own_event)}`;
      deck.appendChild(o);
    }
    if (b.scenario) {
      const sc = scenLogo(scenarioOf(state.scen));
      sc.title = 'The scenario gives it';
      deck.appendChild(sc);
    }
    li.append(tile, bar, legs, deck);
    list.appendChild(li);
  }
  s.append(list, countsEl(r));
  return s;
}
// the chance of each count of targets (all n first), as columns in the game's gauge colours: the chance over a column
// as tall as its share (all n in gold), the count under it, "1 in X" on hover; under the axis, the game's limit arrow
// where the average roll lands (the expected count, between the columns it falls between)
function countsEl(r) {
  const n = state.targets.length,
    ex = landedCounts(r),
    top = Math.max(...ex),
    chart = el('div', 'tgchart'),
    cols = el('div', 'tgcounts');
  for (let k = n; k >= 0; k--) {
    const c = el('span', `tgc${k === n ? ' all' : ''}`),
      bar = el('span', 'tgcb'),
      col = el('i');
    col.style.height = `${top > 0 ? Math.max(2, Math.round((ex[k] / top) * 100)) : 0}%`;
    bar.appendChild(col);
    c.append(el('b', '', pct(ex[k])), bar, el('span', 'tgck', `${k}/${n}`));
    c.title = `Exactly ${k} of ${n}: ${fmtIn(ex[k])} (${pct(ex[k])})`;
    cols.appendChild(c);
  }
  chart.appendChild(cols);
  if (Number.isFinite(r.expected)) {
    const avg = el('span', 'tgavg'),
      arrow = srcFor('ui', 'arrow_avg');
    // column n - e from the left (all n first), at its centre: (n - e + 0.5) / (n + 1) of the width
    avg.style.left = `${((n - r.expected + 0.5) / (n + 1)) * 100}%`;
    if (arrow) avg.appendChild(Object.assign(el('img', ''), { src: arrow, alt: '' }));
    avg.appendChild(el('b', '', `Avg ${r.expected.toFixed(1)}`));
    avg.title = `The average roll lands ${r.expected.toFixed(2)} of ${n}`;
    chart.appendChild(avg);
  }
  return chart;
}
// her race schedule: the schedule window (the game's calendar), from a button under the deck (the races she runs: the
// G1s she could enter but doesn't aren't shown)
// her schedule's rows, each race's figure the community table's on her own grades (agenda.js `tableChance`; the
// planner's, which counts the pink sparks she may get, on hover)
export const tableRows = (r) =>
  (r.schedule || []).map((row) => {
    const c = row.cells[0],
      ix = CAREER.races.indexOf(row.name),
      apt = r.aptitudes?.base;
    if (!c || ix < 0 || !apt || !CAREER.race_cat) return row;
    const { cost, grades } = tableCost(apt, CAREER.race_cat[ix], CAREER.race_ground[ix]);
    return { ...row, cells: [{ ...c, tbl: tableChance(cost, c.row), tapt: cost, grades }] };
  });
const schedOf = (r) => ({
  members: [{ id: r.trainee.id, name: r.trainee.name, short: r.trainee.name }],
  outfits: [{ outfits: r.trainee.outfits }],
  schedule: tableRows(r).filter((x) => x.run !== false),
});
function schedBtn(r) {
  const ran = schedOf(r).schedule.length;
  const b = btn('Race schedule', '', () => openSched(schedOf(r)));
  b.id = 'sched-open';
  b.title = ran ? `She runs ${ran} G1s: ${r.g1_wins.toFixed(1)} wins expected` : 'No G1s';
  return b;
}
function renderResults() {
  const box = $('results');
  box.textContent = '';
  const r = shown();
  if (!r) {
    renderSimulation();
    return;
  }
  // the table: the plan's careers, or the search's with a place open; with every place chosen, the last plan's (refFor)
  const open = !plan.trainee || plan.parents.some((p) => !p);
  if ((open && (planState.state === 'done' ? planState.r.options : last.r.results).length > 1) || refFor())
    box.appendChild(optionsEl());
  renderSimulation();
  markOwn();
}

// ---------- the veterans (pool) ----------
// a veteran as the game's legacy list draws her, her number (an uma.moe one: her trainer) under her, a "Parent 1" tab
// when she is one of the next career's parents; a click shows her beside the list (a wide dialog) or in her popup (her
// sparks editable in place, Yours / Guest, Remove)
function vetCardEl(v, i) {
  const k = plan.parents.indexOf(v.id),
    tg = state.targets.map((t) => t.group),
    have = tg.filter((g) => v.sparks?.[g]).length,
    c = vetCell(v, {
      name: v.lender ? v.lender.name || 'uma.moe' : `#${i + 1}`,
      label: `Veteran #${i + 1}: ${umaName(v.chara)}, ${have} of ${tg.length} target skills. Her sparks`,
      onClick: () => {
        if (!wideVets()) openNode({ kind: 'v', id: v.id });
        else if (vSel !== v.id) {
          vSel = v.id;
          markSel();
          drawPane();
        }
      },
      tab: k >= 0 ? `Parent ${k + 1}` : undefined,
    });
  c.id = `vet-${i}`;
  c.dataset.k = `node:v${v.id}`;
  return c;
}
// your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog
// (the cards are drawn only while the dialog is open, and when it opens: every edit and result redraws the page)
// Two tabs, as the game's Legacy Select: Veteran Umamusume (yours) and Guests (the ones you can borrow: your borrow list,
// then uma.moe's listings); a veteran keeps her number (her place in the whole list) in either. A wide dialog shows the
// chosen one beside the list (`vSel`); a narrow one opens her popup.
let poolStale = true,
  vTab = 'own',
  /** @type {string|null} */ vSel = null;
const wideVets = () => matchMedia('(min-width: 760px)').matches;
const inTab = (v) => !!v.borrowed === (vTab === 'bor');
function setVTab(t) {
  vTab = t;
  for (const x of ['own', 'bor']) $(`v-tab-${x}`).setAttribute('aria-selected', String(x === t));
  $('vets').setAttribute('aria-labelledby', `v-tab-${t}`);
  renderPool();
}
// the list's orders: the order added (uma.moe's listings in uma.moe's order), how many targets she carries (the
// default; ties in the order added), her rank; each starts as the menu says and picking the chosen one again reverses
// it (as the umas' and cards' sort menus)
/** @type {Record<string, [string, string, (v: import('./vets.js').Veteran, i: number) => number]>} */
const VSORTS = {
  no: ['Added', 'In the order they were added to this page (uma.moe’s: in its own order)', (_v, i) => -i],
  tg: [
    'Targets',
    'The most targets she carries first',
    (v) => state.targets.filter((t) => v.sparks?.[t.group] > 0).length,
  ],
  rank: ['Rank', 'The highest career rank first', (v) => v.rank || 0],
};
let vSort = 'tg',
  vRev = false;
function paintVSort() {
  const arrow = vRev ? '↑' : '↓',
    b = $('vs');
  b.textContent = `${VSORTS[vSort][0]} ${arrow}`;
  b.title = VSORTS[vSort][1] + (vRev ? ', reversed' : '');
  b.setAttribute('aria-label', `${b.textContent}, order veterans`);
  for (const x of $('vs-menu').querySelectorAll('button')) {
    const on = x.dataset.k === vSort;
    x.setAttribute('aria-checked', String(on));
    x.textContent = VSORTS[x.dataset.k][0] + (on ? ` ${arrow}` : '');
    x.title = on ? 'Click to reverse' : VSORTS[x.dataset.k][1];
  }
}
function initVSort() {
  for (const k of Object.keys(VSORTS)) {
    const x = el('button', '', VSORTS[k][0]);
    x.type = 'button';
    x.setAttribute('role', 'menuitemradio');
    x.dataset.k = k;
    $('vs-menu').appendChild(x);
  }
  wireMenu('vs', (k) => {
    vRev = k === vSort && !vRev;
    vSort = k;
    drawPool();
  });
}
function renderPool() {
  // the first veterans arriving fold the setup (as a reload with veterans does), unless you folded or opened it
  if (!folded && vets.length) {
    let chosen = null;
    try {
      chosen = localStorage.getItem('llp:setup-folded');
    } catch {
      // storage blocked: as if no fold was chosen
    }
    if (chosen === null) {
      folded = true;
      paintFold();
    }
  }
  umamoeButton();
  const guests = vets.filter((v) => v.borrowed).length;
  $('v-tab-own').textContent = `Veteran Umamusume · ${vets.length - guests}`;
  $('v-tab-bor').textContent = `Guests · ${guests}`;
  if (!$dialog('set-vets').open) {
    poolStale = true;
    return;
  }
  drawPool();
}
// the cards drawn last, by veteran: one whose record, number, parent tab and the targets are as they were is kept, so an
// edit or a tab switch builds only the cards that changed (300 veterans are about 1,900 elements)
/** @type {Map<string, {sig: string, card: HTMLElement}>} */
const poolCards = new Map();
/** @param {import('./vets.js').Veteran} v @param {number} i */
function poolCard(v, i) {
  const sig = JSON.stringify([v, i, plan.parents.indexOf(v.id), state.targets]),
    kept = poolCards.get(v.id);
  if (kept?.sig === sig) return kept.card;
  const card = vetCardEl(v, i);
  poolCards.set(v.id, { sig, card });
  return card;
}
// the chosen one's mark (beside the list on a wide dialog)
function markSel() {
  const selId = `vet-${idxOf(vSel)}`;
  for (const c of $('vets').querySelectorAll('.umcell')) {
    const on = c.id === selId;
    c.classList.toggle('sel', on);
    c.setAttribute('aria-current', String(on && wideVets()));
  }
}
function drawPane() {
  const pane = $('v-pane');
  pane.textContent = '';
  if (vSel && wideVets()) {
    const h = el('h4', 'vptitle');
    pane.appendChild(h);
    h.textContent = veteranView(vSel, pane) || '';
  }
}
function drawPool() {
  const box = $('vets');
  poolStale = false;
  paintVSort();
  const by = VSORTS[vSort][2],
    sign = vRev ? -1 : 1,
    shown = vets
      .map((v, i) => /** @type {[import('./vets.js').Veteran, number]} */ ([v, i]))
      .filter(([v]) => inTab(v))
      .sort(([a, i], [b, j]) => sign * (by(b, j) - by(a, i)));
  // Guests: your borrow list, then uma.moe's listings, each under its heading
  /** @type {[string, [import('./vets.js').Veteran, number][]][]} */
  const groups =
    vTab === 'own'
      ? [['', shown]]
      : [
          ['Borrow list', shown.filter(([v]) => v.src !== 'umamoe')],
          ['uma.moe', shown.filter(([v]) => v.src === 'umamoe')],
        ];
  const nodes = [];
  for (const [head, list] of groups) {
    if (!list.length) continue;
    if (head) nodes.push(el('p', 'vsec', head));
    for (const [v, i] of list) nodes.push(poolCard(v, i));
  }
  box.replaceChildren(...nodes);
  const ids = new Set(vets.map((v) => v.id));
  for (const id of poolCards.keys()) if (!ids.has(id)) poolCards.delete(id);
  if (!shown.length)
    box.innerHTML =
      vTab === 'own'
        ? '<p class="hint">None yet. Import umadump\'s trained_chara_data.json, or make one with Choose parent.</p>'
        : '<p class="hint">None yet. Import umadump\'s rental_chara_data.json, mark a veteran Guest, or Borrow from uma.moe beside Next career.</p>';
  // the one beside the list: the chosen one while she is in this tab, else the first shown
  if (!shown.some(([v]) => v.id === vSel)) vSel = shown[0]?.[0].id ?? null;
  markSel();
  drawPane();
}
// redraw everything, keeping the keyboard focus on the same control (every edit redraws)
function renderAll() {
  keepFocus(() => {
    renderTree();
    renderResults();
    renderPool();
  });
}
// a redraw that keeps the focus on the element of the same key (`data-k`) where it is drawn again
/** @param {() => void} draw */
function keepFocus(draw) {
  const ae = /** @type {HTMLElement} */ (document.activeElement),
    k = ae?.dataset ? ae.dataset.k : null;
  draw();
  if (k) {
    const e = document.querySelector(`[data-k="${CSS.escape(k)}"]`);
    if (e) e.focus();
  }
}

// ---------- settings ----------
function initSettings() {
  $('era-global').setAttribute('aria-pressed', String(ERA === 'global'));
  $('era-gm').setAttribute('aria-pressed', String(ERA === 'gm'));
  const setEra = (e) => {
    if (e === ERA) return;
    eraScenario(e);
    SAVED.era = e;
    writeSaved();
    persist();
    location.reload();
  };
  $('era-global').onclick = () => setEra('global');
  $('era-gm').onclick = () => setEra('gm');
  initScenario();
  initRoster();
  initCards();
  initOwnedImport();
  initTargets();
  initBorrow();
  renderRoster();
  renderTargets();
  enginePersist(); // the solver's kept work survives a page reload
  initSetup();
  initFold();
  // a settings change (targets, umas, cards, scenario): the tree shows the new targets, and the search runs again
  onSave(() => {
    // a career taken from the other careers' table was for that setup: the planner picks again for this one
    if (plan.fromTable) {
      plan.trainee = null;
      plan.parents = [null, null];
      plan.fromTable = false;
      chosen.career = null;
      persist();
    }
    syncListings(); // a changed target set: its listings (kept, or one request)
    schedule();
    renderAll();
    paintFold();
  });
}
// Borrowed parent: three choices, saved with the other settings (a change runs the planner again, `onSave`)
function paintBorrow() {
  for (const b of $('bp').querySelectorAll('button'))
    b.setAttribute('aria-pressed', String(b.dataset.v === borrowChoice()));
}
function initBorrow() {
  for (const b of $('bp').querySelectorAll('button'))
    b.onclick = () => {
      if (b.dataset.v === borrowChoice()) return;
      state.borrowPar = b.dataset.v;
      paintBorrow();
      save();
    };
  paintBorrow();
}
// the setup panel folds to its ribbon (click it) and a one-line summary; the choice is kept in this browser, and until
// one is made it opens folded once there are veterans (the plan's answer, not the settings, is what a return visit is for)
let folded = (() => {
  try {
    const f = localStorage.getItem('llp:setup-folded');
    return f === null ? vets.length > 0 : f === '1';
  } catch {
    return vets.length > 0;
  }
})();
function paintFold() {
  const bar = document.querySelector('.rbbar'),
    b = $('set-fold'),
    sum = $('set-sum');
  bar.classList.toggle('folded', folded);
  b.setAttribute('aria-expanded', String(!folded));
  sum.hidden = !folded;
  b.title = folded ? 'Show the career setup' : 'Fold the career setup';
  const rank = state.rank === RANKS.ss ? 'SS' : 'B–S+',
    era = ERA === 'gm' ? 'JP, Grand Masters' : 'Global',
    sc = scenarioOf(state.scen).name;
  // each setting as its label, light, and its value, bold, as the panel's rows read (the whole line for the screen reader)
  // (the borrowed parent only when it isn't the default)
  const bp = borrowChoice() === 'allow' ? [] : [['Borrowed parent', BORROW_CHOICES[borrowChoice()]]];
  sum.setAttribute(
    'aria-label',
    [era, sc, `rank ${rank}`, `${state.targets.length} target skills`, ...bp.map(([k, v]) => `${k}: ${v}`)].join(' · '),
  );
  sum.replaceChildren(
    ...[['Game data', era], ['Scenario', sc], ['Rank', rank], ['Targets', String(state.targets.length)], ...bp].map(
      ([k, v]) => {
        const p = el('span', 'sumkv');
        p.setAttribute('aria-hidden', 'true');
        p.append(el('span', 'sumk', k), el('b', '', v));
        return p;
      },
    ),
  );
}
// the pink ribbon toggles it (all of it, its edges included); folded, the whole bar (its summary included)
function initFold() {
  const bar = document.querySelector('.rbbar');
  bar.addEventListener('click', (/** @type {TargetedEvent} */ e) => {
    if (!folded && !e.target.closest('#h-set')) return;
    folded = !folded;
    try {
      localStorage.setItem('llp:setup-folded', folded ? '1' : '0');
    } catch {
      // storage blocked: the fold holds for this visit only
    }
    paintFold();
  });
  paintFold();
}

// ---------- start ----------
// Export: the veterans as the page keeps them, the plan, and the solver job the planner sends for them (to replay a
// search outside the page); Import takes the file back
const EXPORT_APP = 'uma-legacy-veterans';
function exportVets() {
  let job = null;
  try {
    job = buildJob();
  } catch {
    // settings that make no job (Blocked): the export carries none
  }
  // (uma.moe's listings are uma.moe's, not yours: left out, their trainers with them)
  const veterans = vets.filter((v) => v.src !== 'umamoe');
  const data = { app: EXPORT_APP, version: 1, era: ERA, exported: new Date().toISOString(), veterans, plan, job };
  $textarea('vx-text').value = JSON.stringify(data);
  $('vx-msg').textContent = '';
  $dialog('vx-dlg').showModal();
}
// Copy: the clipboard API, else select the text and the old copy command, else leave it selected for Ctrl+C
function copyExport() {
  const t = $textarea('vx-text'),
    msg = (m) => {
      $('vx-msg').textContent = m;
    };
  const fallback = (why) => {
    t.focus();
    t.select();
    let ok = false;
    try {
      ok = document.execCommand('copy');
    } catch {
      // the old copy command refused: ok stays false and the text stays selected
    }
    msg(ok ? 'Copied.' : `Could not copy${why ? ` (${why})` : ''}. The text is selected: press Ctrl+C (or ⌘C).`);
  };
  if (!navigator.clipboard?.writeText) {
    fallback('no clipboard access here');
    return;
  }
  navigator.clipboard.writeText(t.value).then(
    () => msg('Copied.'),
    (e) => fallback(e?.message || 'clipboard refused'),
  );
}
function downloadExport() {
  const url = URL.createObjectURL(new Blob([$textarea('vx-text').value], { type: 'application/json' }));
  const a = document.createElement('a');
  a.href = url;
  a.download = `uma-legacy-veterans-${ERA}.json`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
  $('vx-msg').textContent = 'Downloaded.';
}
function ownExport(text) {
  try {
    const o = JSON.parse(text);
    return o && o.app === EXPORT_APP && Array.isArray(o.veterans) ? o : null;
  } catch {
    return null;
  }
}
function importText(text) {
  const own = ownExport(text);
  if (own) {
    if (own.era && own.era !== ERA) {
      $('v-msg').textContent = `Not imported: that export is of the ${$('era-' + own.era)?.textContent} data.`;
      return true;
    }
    const damaged = own.veterans.filter((v) => !isVeteran(v)).length;
    if (damaged) {
      $('v-msg').textContent =
        `Not imported: ${damaged} veteran${damaged === 1 ? '' : 's'} in that export ${damaged === 1 ? 'is' : 'are'} damaged (no id or two parents).`;
      return true;
    }
    // each keeps her source (one made by hand stays hand-made); an export from before sources were saved: imported
    importVets(
      own.veterans.map((v) => ({ src: 'import', ...asVeteran(v) })),
      true,
    );
    setVTab('own');
    $('v-msg').textContent =
      `Imported ${own.veterans.length} veteran${own.veterans.length === 1 ? '' : 's'} from an export of this page.`;
    return true;
  }
  const tree = parseTree(text);
  if (tree) {
    if (!tree.length) {
      $('v-msg').textContent = 'No veteran in that uma.moe tree.';
      return false;
    }
    addVets(tree);
    const first = vets.indexOf(tree[0]) + 1;
    $('v-msg').textContent =
      `Added ${tree.map((v) => umaName(v.chara)).join(', ')} from an uma.moe tree, with her parents, as #${first}${tree.length > 1 ? `-#${first + tree.length - 1}` : ''} (marked yours: switch to Borrowed if she is a friend's).`;
    return true;
  }
  try {
    const r = importVets(parseExport(text)),
      counted = (k, word) => `${k} ${word}${k === 1 ? '' : 's'}`,
      said = [];
    if (r.added || r.updated)
      said.push(
        `Imported ${counted(r.added + r.updated, 'veteran')} of yours: ${[r.added && `${r.added} new`, r.updated && `${r.updated} already here, updated`].filter(Boolean).join(', ')}`,
      );
    if (r.borrowed)
      said.push(
        `${said.length ? 'r' : 'R'}eplaced the borrow list with ${counted(r.borrowed, 'borrowed veteran')}${r.dropped ? ` (it had ${r.dropped})` : ''}`,
      );
    setVTab(r.added || r.updated ? 'own' : 'bor');
    $('v-msg').textContent = `${said.join('; ')}.`;
    return true;
  } catch (e) {
    $('v-msg').textContent = e.message;
    return false;
  }
}
document.addEventListener('click', (e) => {
  if (e.target instanceof HTMLDialogElement && e.target.open) e.target.close();
}); // a backdrop click closes
loadData().then(
  async () => {
    initSettings();
    initPopover();
    // uma.moe's listings for the targets, from the kept answer, before the first search (a request otherwise)
    initUmamoe({ open: (v) => openNode({ kind: 'v', id: v.id }) });
    syncListings();
    dropMissing(); // (a place naming a veteran not here: an uma.moe one whose answer is gone, say)
    // Try a run's popup: Close and a backdrop click shut it
    const simDlg = $dialog('sim-dlg');
    for (const x of simDlg.querySelectorAll('.dclose')) /** @type {HTMLElement} */ (x).onclick = () => simDlg.close();
    backdropCloses(simDlg);
    dropStaleChosen();
    // A result names parents by their place in the veteran list: adding, removing or importing veterans changes the
    // places, so the answer on screen goes; an edit to a veteran keeps it on screen, dimmed, until the new one is ready.
    const vetsSig = () => vets.map((v) => v.id).join(',');
    let sig = vetsSig();
    onVetsChange(() => {
      if (vetsSig() !== sig) {
        sig = vetsSig();
        last = null;
        clearAnalysis();
        planState = { state: 'none' };
      }
      schedule();
      renderAll();
    });
    for (const x of ['vets', 'new', 'auto']) $(`vp-tab-${x}`).onclick = () => vpTab(x);
    $input('vp-filter').oninput = () => filterUmas($('vp-umas'), $input('vp-filter').value);
    $('vp-auto-go').onclick = () => {
      plan.parents[pickK] = null;
      $dialog('vpick-dlg').close();
      changed();
    };
    $('v-file').onchange = async (/** @type {TargetedEvent<HTMLInputElement>} */ e) => {
      const f = e.target.files[0];
      if (f) importText(await f.text());
      e.target.value = '';
    };
    $('v-paste').onclick = () => {
      $textarea('paste-text').value = '';
      $dialog('paste-dlg').showModal();
    };
    $('v-export').onclick = exportVets;
    $('vx-copy').onclick = copyExport;
    $('vx-dl').onclick = downloadExport;
    $('paste-load').onclick = () => {
      if (importText($textarea('paste-text').value)) $dialog('paste-dlg').close();
    };
    $('v-toggle').onclick = () => {
      if (poolStale) drawPool();
      $dialog('set-vets').showModal();
    };
    // however the dialog opens, its cards are drawn before it paints (a mutation callback runs before the next frame)
    new MutationObserver(() => {
      if ($dialog('set-vets').open && poolStale) drawPool();
    }).observe($dialog('set-vets'), { attributes: true, attributeFilter: ['open'] });
    $('v-clear').onclick = () => {
      // the open tab's veterans
      // (uma.moe's listings stay, unless one is a chosen parent: they are its answer for the targets, not yours)
      const gone = (v) => inTab(v) && (v.src !== 'umamoe' || plan.parents.includes(v.id)),
        n = vets.filter(gone).length,
        what = vTab === 'own' ? `${n} of your veterans` : `${n} borrowed veteran${n === 1 ? '' : 's'}`;
      if (n && confirm(`Remove all ${what}?`)) {
        clearVets(gone);
        $('v-msg').textContent = `Removed ${what}.`;
      }
    };
    for (const x of ['own', 'bor']) $(`v-tab-${x}`).onclick = () => setVTab(x);
    initVSort();
    renderAll();
    await preloadAnswer(); // the last visit's answer, when nothing changed since: drawn at once
    search();
  },
  (e) => {
    status = { kind: 'error', text: e.message };
    paint();
  },
);
