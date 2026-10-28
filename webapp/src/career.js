// Career planner page: entry point. build.py bundles this and the modules it imports into dist/assets/career.<hash>.js,
// which dist/career.html loads.
// The lineage tree (trainee, two parents, their parents) is the page: each place is set by hand or left to the planner,
// which tries every owned uma and every pair of the player's veterans (vets.js) and fills the open places with its
// best pick (cmd "career"). The settings editors are the loop planner's (same modules, same saved settings).
import { $, $dialog, $input, $textarea, el, esc, fmtIn } from './dom.js';
import { ERA, SAVED } from './settings.js';
import { UMAS, SK, CAREER, loadData } from './data.js';
import { state, goldGiven, writeSaved } from './state.js';
import { run, restartEngine, engineBusy, enginePersist, engineSaveState, nextKill, stateKey } from './engine.js';
import { onSave, save } from './save.js';
import { EXTRA, jobTargets, ownedCards, rcardsOff } from './job.js';
import { umaAv, skillIcon, cardAv, runnerHTML, setRunner } from './images.js';
import { initScenario, eraScenario, scenarioOf } from './scenario.js';
import { initRoster, renderRoster, noOutfit, pickOutfit, outfitNote } from './roster.js';
import { initCards, cardTitle, borLast, chainEndNote } from './cards.js';
import { initPopover, markOwn } from './popover.js';
import { openSched } from './agenda.js';
import { initSetup } from './setup.js';
import './gmtip.js'; // the game-style tooltips (both planner pages)
import { affEl, affTable as affTableOf } from './afftable.js';
import { pct, openOdds as openOdds0 } from './odds.js';
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
  changeUma,
  askSkills,
  sampleVets,
  newVet,
  importVets,
  isVeteran,
  addVets,
  removeVet,
  clearVets,
  legaciesJob,
  openUmaPicker,
  drawUmas,
  filterUmas,
  openWins,
} from './vets.js';
import { parseExport, parseTree } from './umadump.js';
import { detailsEl, inspView, fitSparks, deckCardAt } from './details.js';
import { drawPink } from './pink.mjs';
import { unprovenWhy, whyText, searchCaveat, planCaveat } from './proofs.mjs';

const tName = (g) => {
  const s = SK.get(g) || { w: `Skill ${g}`, gd: null };
  return goldGiven(g) && s.gd ? s.gd : s.w;
};
const owned = (u) => state.roster[u.id] !== 'out' && !noOutfit(u);
const btn = (label, cls, fn) => {
  const b = el('button', `btn${cls ? ` ${cls}` : ''}`, label);
  b.type = 'button';
  b.onclick = fn;
  return b;
};

// ---------- status line ----------
// One run per edit, one answer per run. A run has phases: the search (the best careers by odds), then, with the
// trainee left to the planner, the plan (its pick among them and every other owned uma). The status line names the
// phase at work; the answer (career, tree places, veteran analysis) is drawn once, when the run ends. Until then the
// previous answer stays on screen, dimmed, its actions off.
let searching = false,
  phase = 'search',
  phaseStart = 0,
  secondsTick = null,
  searchTimer = null,
  runId = 0;
let started = false; // a job of this run is under way (the pause before it dims nothing: an edit that ends where it started runs none)
let phaseNote = ''; // the plan phase's lead: what the search found ("Weighed N careers in X s.")
let planProgress = ''; // the plan's progress, as the solver reports it from inside the plan (`progText`)
let inflight = null; // the run under way: its job and places (`runKey`); null when none is
// a job's key: its JSON with the legacies' apart, kept with the array `legaciesJob` returns (the same until an edit)
let legaciesJson = { legacies: null, json: '' };
const jobKey = (job) => {
  if (legaciesJson.legacies !== job.legacies)
    legaciesJson = { legacies: job.legacies, json: JSON.stringify(job.legacies) };
  return `${JSON.stringify([ERA, { ...job, legacies: 0 }])}\n${legaciesJson.json}`;
};
const runKey = (job) => `${jobKey(job)}\n${JSON.stringify(plan)}`;
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
 * @property {number} [deck_white]  the deck hints its white
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
let last = null;
function paint() {
  const note = $('run-note');
  setRunner(plan.trainee || shown()?.trainee?.id); // the runner: the trainee you chose, else the career on show
  $('rhead').dataset.kind = searching ? 'busy' : status.kind;
  const dimmed = searching && started && !!shown(); // the previous answer, on screen until this run's
  $('results').classList.toggle('stale', dimmed || (!searching && ['error', 'empty'].includes(status.kind)));
  $('tree').classList.toggle('stale', dimmed);
  $('results').setAttribute('aria-busy', String(searching));
  if (searching) paintBusy(note);
  else {
    note.textContent = status.text + (status.kind === 'done' ? planNote() : '');
    if (status.act && !searching) {
      const b = btn(status.act.label, 'primary', status.act.run);
      b.id = 'run-act';
      note.append(' ', b);
    } // e.g. Try sample veterans
  }
  // before the first answer: the analysis panel's frame, so the space beside the tree isn't blank while she is planned;
  // renderResults replaces it with the answer, and a run that ends without one takes it away
  const wait = $('results').querySelector(':scope > .simwait');
  if (searching && !shown() && !$('results').firstElementChild) $('results').append(waitEl());
  else if (!searching && wait) wait.remove();
}
// the status line during a run: the search's lead (in the plan phase), the phase at work, the plan's progress and the
// phase's seconds so far
function paintBusy(note) {
  const seconds = Math.floor((performance.now() - phaseStart) / 1000);
  note.textContent = phase === 'plan' && phaseNote ? `${phaseNote} ` : '';
  const line = el('span');
  line.innerHTML = runnerHTML();
  line.append(
    phase === 'plan' ? 'Planning your next career…' : 'Searching careers…',
    el('span', '', seconds >= 1 ? ` ${seconds} s` : ''),
  );
  /** @type {HTMLElement} */
  (line.lastChild).setAttribute('aria-hidden', 'true');
  // (the plan's progress, before the seconds; like them, not read out at every step)
  if (phase === 'plan' && planProgress) {
    const pg = el('span', 'runprog', ` (${planProgress})`);
    pg.setAttribute('aria-hidden', 'true');
    line.insertBefore(pg, line.lastChild);
  }
  note.appendChild(line);
}
function waitEl() {
  const w = el('div', 'simwait'),
    box = sectionEl('simulation', 'Veteran analysis'),
    body = el('div', 'ud simwbody');
  const p = el('p');
  p.innerHTML = runnerHTML(true);
  p.append('Warming up! The deck, sparks and comparison will be here as soon as the planner picks her career.');
  body.append(p);
  box.append(body);
  w.append(box);
  return w;
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
    (inflight
      ? inflight === key
      : !searching && last && `${last.key}\n${last.plan}` === key && planState.state !== 'error')
  )
    return;
  clearTimeout(searchTimer);
  runId++;
  inflight = null;
  // only a job in flight is cut short: an idle worker keeps what it holds for the next jobs (the last search, every
  // uma's best career, the proofs); during the pause before a search nothing runs, so a second edit kills nothing
  if (engineBusy()) restartEngine();
  // a comparison in flight is stopped with it: its panel goes back to idle, or a roll's quick figures stand unchecked
  // (an answer that stands keeps her)
  simulationId++;
  if (simulation.state === 'busy') Object.assign(simulation, { state: 'ready', r: null, kind: null });
  Object.assign(simulation, { refining: false, provisional: false, standing: false });
  searching = true;
  started = false;
  phase = 'search';
  phaseNote = '';
  phaseStart = performance.now();
  paint();
  searchTimer = setTimeout(search, 600);
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
const BORROW_CHOICES = { allow: 'Allowed', later: 'None left today', never: 'Never' };
const borrowChoice = () => (Object.hasOwn(BORROW_CHOICES, state.borrowPar) ? state.borrowPar : 'allow');
function buildJob() {
  if (!state.targets.length) throw new Blocked('empty', 'No target skills: add them with Target skills above.');
  const fixed = plan.parents.filter(Boolean).map(vetById);
  if (vets.length < 2)
    throw new Blocked(
      'empty',
      'Choose both parents, or import your veterans and let the planner pick.',
      !vets.length ? { label: 'Try sample veterans', run: () => addVets(sampleVets()) } : null,
    );
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
const chosenSetup = (job) =>
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
  if (!c || c.places !== JSON.stringify(plan) || c.setup !== chosenSetup(job)) return false;
  const named = (c.id.split(':')[1] || '').split(',').map(Number),
    fixed = job.fix;
  return named.length === 2 && fixed.length === 2 && named.every((i) => fixed.includes(i));
};
/** @type {{places: string, text: string}|null} a saved choice dropped on load, while its places stand */
let chosenDropped = null;
function dropStaleChosen() {
  if (!chosen.career) return;
  let applies = false;
  try {
    applies = buildJob().career === chosen.career.id;
  } catch {}
  if (applies) return;
  chosenDropped = {
    places: chosen.career.places,
    text: 'The deck you chose with Use this before the reload no longer fits (its trainee, parents, their records, the targets or the card settings changed), so this is the deck the search finds.',
  };
  chosen.career = null;
  persist();
}
// a full veteran: a spark of every target (with full veteran parents: a fully looped parent)
const isFull = (nd) => !!nd && state.targets.length > 0 && state.targets.every((t) => nd.sparks?.[t.group] > 0);
// a career on two full veteran parents: her own full veteran there is a fully looped parent (the plan says so; a search's row is read off the veterans)
const looped = (r) => r.looped ?? r.parents.every((i) => isFull(vets[i]));
const ROLL_K = 6; // careers a search returns with a place open (the plan's candidates; `buildJob`)
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
const baseKey = (job) => JSON.stringify(baseOf(job));
// the answer's plan (the planner's candidates and its pick; none with the trainee chosen), and the entered veteran
// with its comparison (on demand)
/** @type {PlanState} */
let planState = { state: 'none', r: null };
// a career's identity: the solver's `career` (her uma, her parents in order and her deck: two decks on the same parents
// are two careers), else (an answer from before it) her uma and parents
const keyOf = (r) => r.career ?? `${r.trainee.id}:${r.parents.join(',')}`;
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
const refFor = () =>
  refPlan && last && planState.state !== 'done' && refPlan.base === refBase(last.job) ? refPlan : null;
// the last few answers, by job and places: going back to one draws it again without a run
const answers = new Map(),
  ANSWERS = 4;
// the refit plan Compare ran for the pool with her, when today's plan had no V or no fit to price her under
// (`runComparison`):
// the same job a run sends once she is added (keys sorted, the comparison's flag aside), so Add to veterans takes it as
// it is instead of planning again (the solver is deterministic for its job). A plan under today's fit is never kept
// here: its job carries `next.fit`, which a run's does not
let planKept = null;
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
const PLAN_NEXT = Object.freeze({});
const planKey = (job) => {
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
const runJob = async (job, progress, background = false) => {
  const m = await run(job, undefined, progress, background),
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
} catch {}
/**
 * The veteran analysis: the veteran this career leaves (the draft) and her comparison.
 * @typedef {object} Simulation
 * @property {'none'|'ready'|'busy'|'done'|'error'} state  none: no career on show; ready: no comparison yet
 * @property {Draft|null} [draft]  the entered veteran
 * @property {CareerSearch|null} [r]  the comparison's search: the best career with her as a parent (`pending` while the
 *   quick comparison looks for it)
 * @property {CareerReport|null} [baseline]  today's best career, without her
 * @property {string|null} [error]
 * @property {string} [forKey]  `keyOf` the career she came from
 * @property {'quick'|null} [kind]  the running comparison is a roll's quick one
 * @property {boolean} [stale]  edited since the figures on show
 * @property {boolean} [refining]  an exact comparison runs behind a roll's quick figures
 * @property {boolean} [provisional]  the quick figures say no saving, but their bound allows one: checking
 * @property {boolean} [standing]  a roll's quick figures say it saves careers: the exact comparison starts once the
 *   roll has stood 0.8 s (shown as checking meanwhile)
 * @property {boolean} [quick]  the figures on show are the quick ones
 * @property {boolean} [full]  she lands every target
 * @property {{now: number|null, withHer: number|null}} [v0]  expected careers to a fully looped parent with your
 *   veterans now, and with her too
 * @property {boolean} [fixedFit]  with her is priced under today's plan's fit (Add to veterans then fits the plan to
 *   the pool with her, so its figures can differ); false: the refit itself (today's plan had no V or no fit)
 * @property {PlanOption|null} [pickA]  the planner's next career once she is added
 * @property {boolean} [samePick]  it is the same as without her
 * @property {boolean} [herParent]  she is one of its parents
 * @property {string} [where]  where her saving lands (`savedWhere`)
 *
 * The veteran a career leaves, as entered (a Veteran without id), with `apt` (her end grades) and `bought` (skill group
 * -> 'gold' | 'white') from her roll.
 * @typedef {LineageNode & {borrowed: boolean, parents: LineageNode[], apt?: number[], bought?: Object<number, string>}} Draft
 */
/** @type {Simulation} */
let simulation = { state: 'none', draft: null, r: null, baseline: null, error: null };
let simulationId = 0; // the comparison under way; a newer one (or a run) makes it stale
let baselineCache = null; // {key: baseKey(job), result}: today's best career, the comparisons' baseline
function comparisonJob() {
  return JSON.parse(baseKey(last.job));
}
// A run: the search, then (with the trainee's place left to the planner) the plan, both on the one worker; the answer
// is drawn once, at the end. Two solver jobs: the plan's reuses the search's (the solver keeps its last search, so the
// plan job does none of that work again; a second job costs a few ms), and the page knows which phase is at work
// without the solver reporting it. A newer edit stops a run in either phase (`runId`); nothing of it is drawn.
async function search() {
  const id = ++runId;
  simulationId++;
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
    places = JSON.stringify(plan),
    answerKey = `${key}\n${places}`;
  if (last && last.key === key && last.plan === places && planState.state !== 'error') {
    searching = false;
    clearInterval(secondsTick);
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
    simulation = r ? readySimulation(r) : { state: 'none' };
    searching = false;
    clearInterval(secondsTick);
    renderAll();
    paint();
    resumeCheck(id);
    return;
  }
  searching = true;
  started = true;
  inflight = answerKey;
  phase = 'search';
  phaseNote = '';
  planProgress = '';
  phaseStart = performance.now();
  clearInterval(secondsTick);
  secondsTick = setInterval(paint, 1000);
  paint();
  const fail = (text) => {
    status = { kind: 'error', text };
    last = null;
    planState = { state: 'none' };
    simulation = { state: 'none' };
  };
  /** @type {PlanDone|null} the plan to check once it is shown (`checkPhase`) */
  let checkFor = null;
  try {
    const m = await runJob(job);
    if (id !== runId) return;
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
      }
      const r = shown();
      simulation = r ? readySimulation(r) : { state: 'none' };
      if (best && !job.trainees.length && !job.fix.length) baselineCache = { key: baseKey(job), result: best };
    }
  } catch (e) {
    if (id !== runId) return;
    fail(e.message);
  }
  searching = false;
  inflight = null;
  clearInterval(secondsTick);
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
  const current = () => id === runId && planState === planned;
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
  searching = false;
  clearInterval(secondsTick);
  ({ status, last, planState } = answer);
  simulation = { state: 'none' };
  renderAll();
  paint();
}
// the veteran analysis, ready for the career on show: she starts as a random result of it
/** @type {(r: CareerReport) => Simulation} */
const readySimulation = (r) => ({
  state: 'ready',
  draft: draftOf(r),
  r: null,
  baseline: null,
  error: null,
  forKey: keyOf(r),
});
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
    dropped = chosenDropped?.places === JSON.stringify(plan) ? ` ${chosenDropped.text}` : '';
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
  phase = 'plan';
  phaseNote = found;
  planProgress = '';
  phaseStart = performance.now();
  paint();
  const { rollout: _r, p_now: _p, ...plain } = job;
  let planned;
  try {
    // Compare may have run this very plan job already (`planKept`): take its answer instead of planning again
    const jp = { ...plain, next: { ...PLAN_NEXT }, no_reports: true },
      kept = planKept && planKept.key === planKey(jp) ? planKept : null;
    /** @type {{r: {next?: PlanReport, error?: string}, ms: number}} */
    const m2 = kept
      ? { r: { next: kept.r }, ms: kept.ms }
      : await runJob(jp, (p) => {
          if (id === runId) {
            planProgress = progText(p);
            paint();
          }
        });
    if (id !== runId) return null;
    const x = m2.r.next;
    planned =
      x && !x.error && x.options?.length
        ? { state: 'done', r: x, ms: m2.ms, job: jp }
        : { state: 'error', text: m2.r.error || x?.error || 'no option found.' };
  } catch (e) {
    if (id !== runId) return null;
    planned = { state: 'error', text: e.message };
  }
  if (planned.state === 'done') engineSaveState(); // the worker's kept work, for the next page load
  return planned;
}

// ---------- the pedigree ----------
// the career on show: the top result while it belongs to the current plan
// (the plan's pick once it is made, else the highest odds)
const shown = () =>
  last && last.plan === JSON.stringify(plan) && last.r.results[0]
    ? (planState.state === 'done' && planState.r.choice) || last.r.results[0]
    : null;
// an option's role, in plain words: one of the two parents of today's estimate (the route the (i) popup walks through)
// or its last run, else what her run does for it: the side her veteran's routes lift most often where she lowers it,
// with that chance per run, or her own n/n as a side (`lifts_side`); a loop member says so first
const roleText = (o) => {
  const n = state.targets.length;
  if (o.role === 'plan') return "One of the two parents in today's estimate";
  if (o.role === 'finish') return "The last run in today's estimate (with the two full veteran parents)";
  const loop = o.role === 'loop' ? 'In your loop. ' : '',
    side = o.lifts_side;
  if (!side) return `${loop}Not scored against today's estimate`;
  if (side.uma != null && side.chance >= side.own)
    return `${loop}Lifts ${umaName(side.uma)}'s side (${pct(side.chance)} per run)`;
  if (side.own > 0) return `${loop}Her own ${n}/${n} as a side (${pct(side.own)} per run)`;
  return `${loop}No result of hers lowers today's estimate`;
};
// δ, the per-decision tolerance (AGENTS.md, The career planner's goal and its bar): a row whose Saves per run is within
// it of the pick's is shown as about equal to the pick. Display only: the pick stays the strict top score
const TIE_CAREERS = 0.25;
// a career's parents, by legacy index, from its id (`kept::career_id`: "trainee:first,second:deck")
const careerParents = (id) => id.split(':')[1].split(',').map(Number);
// the first step of today's estimate (`members`, A then B, as planval's `route_first_step` reads them): the first side
// without a real full veteran of hers, her builder's career where she builds (`builder`) or else her own (`parents`);
// with both sides real, C's run on those two. Its parents by legacy index, and what it is; null without a route, or
// where it builds and names no builder (her Cont given)
/** @type {(planReport: PlanReport) => {what: string, parents: number[]}|null} */
function routeFirstStep(planReport) {
  const members = planReport.members || [];
  if (members.length < 2 || planReport.v0 == null) return null;
  for (const m of members) {
    if (m.real) continue;
    if (m.building_careers > 0)
      return m.builder ? { what: `a builder for ${m.trainee}`, parents: careerParents(m.builder) } : null;
    return m.parents ? { what: `${m.trainee}'s run`, parents: m.parents } : null;
  }
  const real = (planReport.v0_plan?.sides || []).map((side) => /^real:(\d+)$/.exec(side)).filter(Boolean);
  return real.length === 2 ? { what: `${planReport.v0_plan.c}'s run`, parents: real.map((m) => +m[1]) } : null;
}
// an option's net saving: the careers her veteran saves toward a fully looped parent on average, minus the one it costs
const netText = (x) => (x >= 0 ? '+' : '−') + Math.abs(x).toFixed(1);
// an option's Saves per run: the plan's `score` (her net saving; null when the plan scored nothing), else `index`
const scoreOf = (o) => o.score ?? o.index;
const fmtN = (x) => Math.round(x).toLocaleString();
// which veteran sits in parent place k: the chosen one, else the planner's pick in the result on show
function parentAt(k, r) {
  if (plan.parents[k]) return { v: vetById(plan.parents[k]), pick: false };
  if (!r) return null;
  // the other place's chosen veteran (-1: none): the result's parent that isn't her fills this one
  const other = plan.parents[1 - k] ? idxOf(plan.parents[1 - k]) : -1;
  const i = other >= 0 ? r.parents.filter((x) => x !== other)[0] : r.parents[k];
  return i === undefined ? null : { v: vets[i], pick: true };
}
// a legacy's affinity in the career on show (slot of veteran i, or of her parent j)
const affOf = (r, i, j = null) => {
  if (!r) return null;
  const slot = r.slots.find((x) => x.legacy === i && (j === null ? x.kind === 'parent' : x.gp === j));
  return slot ? slot.aff : null;
};
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
  const n = pnode('trainee', plan.trainee ? 'Trainee' : "Trainee · planner's pick", !plan.trainee);
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
  // her career's deck (opens here, under her) and race schedule (its window), behind buttons
  if (r && r.trainee.id === id) {
    const row = el('div', 'row tdeck');
    const db = btn(deckOpen ? 'Hide the deck' : 'Deck', '', () => {
      deckOpen = !deckOpen;
      renderTree();
      $('deck-open')?.focus();
    });
    db.id = 'deck-open';
    db.dataset.k = 'deck-open';
    db.setAttribute('aria-expanded', String(deckOpen));
    db.setAttribute('aria-controls', 'tree-deck');
    row.append(db, schedBtn(r));
    n.appendChild(row);
    if (deckOpen) {
      const dk = deckEl(r, 'Deck');
      dk.id = 'tree-deck';
      n.appendChild(dk);
    }
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
  const n = pnode(
    'parent',
    (at?.pick ? "Planner's pick" : at ? 'Your pick' : `Parent ${k + 1}`) + (at?.v.borrowed ? ' · guest' : ''),
    at?.pick,
  );
  if (at?.v.borrowed) n.classList.add('bor');
  let t;
  if (at) {
    const v = at.v,
      i = idxOf(v.id),
      a = affOf(r, i);
    if (a !== null) n.querySelector('.pnh').appendChild(affEl(a));
    t = tile(
      `node:p${k}`,
      umaAv(v.chara, v.outfit, 72),
      umaName(v.chara),
      `Veteran #${i + 1}${v.src === 'hand' ? ', made here' : ''}`,
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
    const gk = el('div', 'pk');
    at.v.parents.forEach((g, j) => {
      const gn = pnode('gp', 'Grandparent', false);
      const a = affOf(r, idxOf(at.v.id), j);
      if (a !== null) gn.querySelector('.pnh').appendChild(affEl(a));
      const gt = tile(
        `node:p${k}g${j}`,
        umaAv(g.chara || 0, g.outfit, 48),
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
let deckOpen = false; // the deck under the trainee: closed at first
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
function openNode(spec) {
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
      el('div', 'nm', umaName(v.chara)),
      el('div', 'sub', `Veteran #${i + 1}${v.src === 'hand' ? ', made here' : ''}${v.borrowed ? ', borrowed' : ''}`),
    );
    w.append(pb, nm, winsBtn(v, umaName(v.chara), `p${k}`));
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
// where a veteran came from (entered from a career here, an uma.moe tree, an import, made here)
const VET_SOURCE = {
  entered: 'Entered here',
  simulated: 'Entered here',
  tree: 'From uma.moe',
  import: 'Imported',
  hand: 'Made here',
  sample: 'Sample',
};
// A veteran of yours (the pool, Career Info's legacies): her sparks and her parents', editable in place.
function fillVeteranNode({ id }, _r, body) {
  const i = idxOf(id),
    v = vets[i];
  if (!v) return false;
  $('node-title').textContent = `Veteran #${i + 1}${v.borrowed ? ' · borrowed' : ''}`;
  const w = el('div', 'twho'),
    b = el('button', 'facebtn');
  b.type = 'button';
  b.append(umaAv(v.chara, v.outfit, 56));
  b.title = 'Change who she is';
  b.setAttribute('aria-label', `Veteran #${i + 1}: ${umaName(v.chara)}. Change who she is`);
  b.onclick = fromNode(() => changeUma(v, `Veteran #${i + 1}`));
  const src = VET_SOURCE[v.src];
  w.append(b, el('div', 'nm', umaName(v.chara)), winsBtn(v, `Veteran #${i + 1}, ${umaName(v.chara)}`, `v${v.id}`));
  if (src) w.appendChild(el('span', v.src === 'tree' ? 'srctag' : 'simtag', src));
  body.appendChild(w);
  const iv = el('div', 'ud udbare');
  iv.appendChild(
    inspView({
      d: v,
      key: `v${v.id}`,
      onEdit: changed,
      parentKey: (j) => `v${v.id}g${j}`,
      onEditParent: changed,
      header: false,
    }),
  );
  body.appendChild(iv);
  // her parents: who they are and their G1 wins
  const pr = el('div', 'vpars');
  v.parents.forEach((g, j) => {
    const x = el('div', 'twho'),
      gb = el('button', 'facebtn');
    gb.type = 'button';
    gb.append(umaAv(g.chara || 0, g.outfit, 36));
    const lab = `Parent ${j + 1} of #${i + 1}`;
    gb.title = `${lab}: ${umaName(g.chara)}. Change who she is`;
    gb.setAttribute('aria-label', gb.title);
    gb.onclick = fromNode(() => changeUma(g, lab));
    x.append(
      gb,
      el('span', 'nm', g.chara ? umaName(g.chara) : 'Unknown parent'),
      winsBtn(g, `${lab}, ${umaName(g.chara)}`, `v${v.id}g${j}`),
    );
    pr.appendChild(x);
  });
  body.appendChild(pr);
  return true;
}
// A grandparent (parent k's parent j): who she is, her sparks and G1 wins.
function fillGrandparentNode({ k, j }, r, body) {
  const at = parentAt(k, r),
    g = at?.v.parents[j];
  $('node-title').textContent = `Legacy ${k + 1} · grandparent`;
  if (!g) return false;
  const w = el('div', 'twho'),
    b = el('button', 'facebtn');
  b.type = 'button';
  b.append(umaAv(g.chara || 0, g.outfit, 56));
  b.title = 'Change who she is';
  b.setAttribute('aria-label', `Grandparent: ${umaName(g.chara)}. Change who she is`);
  b.onclick = fromNode(() => changeUma(g, 'Grandparent'));
  w.append(
    b,
    el('div', 'nm', g.chara ? umaName(g.chara) : 'Unknown: pick her'),
    winsBtn(g, umaName(g.chara), `p${k}g${j}`),
  );
  const iv = el('div', 'ud udbare');
  iv.appendChild(inspView({ d: { ...g, parents: [] }, key: `p${k}g${j}`, onEdit: changed, header: false }));
  // (a copy without parents for the view; her sparks object is hers: edits land on her)
  iv.querySelector('.simskills').replaceChildren(sparksEl(g, `p${k}g${j}`, changed));
  body.append(w, iv);
  return true;
}
function renderTree() {
  const r = shown(),
    box = $('tree');
  box.textContent = '';
  const t = el('div', 'pt');
  t.appendChild(traineeEl(r));
  box.appendChild(t);
  const k = el('div', 'pk');
  k.append(familyEl(0, r), familyEl(1, r));
  box.appendChild(k);
  if (nodeDlg?.open) fillNode();
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
  vets.forEach((v, i) => {
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
      el('div', 'sub', `#${i + 1}${v.borrowed ? ', borrowed' : ''}${cur ? ', chosen' : ''}`),
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
const sectionEl = (cls, title) => {
  const s = el('section', `rbsec ${cls}`);
  s.appendChild(el('h3', '', title));
  return s;
};
// the career's odds in the veteran card's header: all n; a click opens the odds popup (exactly each count)
function oddsBtn(r) {
  const n = state.targets.length,
    b = el('button', 'udodds');
  b.type = 'button';
  b.id = 'odds-open';
  b.dataset.k = 'odds-open';
  b.setAttribute('aria-haspopup', 'dialog');
  b.append(el('b', '', fmtIn(r.p_all)), el('span', '', ` to land all ${n}`));
  b.setAttribute('aria-label', `${fmtIn(r.p_all)} runs to land all ${n}. The odds of every count`);
  b.onclick = () => openOdds(r);
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
function openOdds(r) {
  openOdds0({
    title: 'Odds of this career',
    n: state.targets.length,
    ex: landedCounts(r),
    pAll: r.p_all,
    expected: r.expected,
  });
}
// the deck as the loop planner shows it: card art and name; a click opens the card's panel (owned or not, GameTora)
function deckEl(r, title = 'Deck') {
  const d = r.deck;
  const dk = el('div', 'deck');
  dk.innerHTML = '<div class="dtitle"><b></b></div><div class="dcards"></div>';
  dk.querySelector('.dtitle b').textContent = title;
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
    b.append(cardAv(c || null, 96), el('span', 'dn', full));
    if (pick) b.appendChild(el('i', '', `prioritise ${pick}`));
    if (endAt) b.appendChild(el('i', '', chainEndNote(c, endAt)));
    box.appendChild(b);
  });
  return dk;
}
function targetsEl(r) {
  const s = sectionEl('rtgt', 'Target skills');
  const tb = el('table', 'cartab');
  tb.innerHTML =
    '<thead><tr><th scope="col">Skill</th><th scope="col" title="Chance the six legacies pass the skill on (two inheritance events)">Inherited</th><th scope="col" title="Legacies (of six) with a spark of it">Sparks in lineage</th><th scope="col" title="Chance the new veteran gets its spark">Lands as a spark</th></tr></thead><tbody></tbody>';
  for (const t of r.targets) {
    // the skill as the game's tile (the card's Skills tab), gold for a gold target
    const row = el('tr'),
      c0 = el('th');
    c0.scope = 'row';
    const tile = el('div', `udsk${t.gold ? ' gold' : ''}`),
      ic = skillIcon(t.group, t.gold);
    if (ic) tile.appendChild(ic);
    tile.appendChild(el('span', 'udskn', tName(t.group)));
    c0.appendChild(tile);
    row.appendChild(c0);
    for (const v of [t.gold ? 'scenario' : pct(t.q), `${t.carriers} of 6`, pct(t.p)]) row.appendChild(el('td', '', v));
    tb.tBodies[0].appendChild(row);
  }
  s.appendChild(tb);
  return s;
}
// her race schedule: the loop planner's schedule window, from a button beside the deck
// (the races she runs: the G1s she could enter but doesn't aren't shown)
const schedOf = (r) => ({
  members: [{ id: r.trainee.id, name: r.trainee.name, short: r.trainee.name }],
  outfits: [{ outfits: r.trainee.outfits }],
  schedule: (r.schedule || []).filter((x) => x.run !== false),
});
function schedBtn(r) {
  const ran = schedOf(r).schedule.length;
  const b = btn('Race schedule', '', () => openSched(schedOf(r)));
  b.id = 'sched-open';
  b.title = ran ? `She runs ${ran} G1s: ${r.g1_wins.toFixed(1)} wins expected` : 'No G1s';
  return b;
}
// the veteran analysis' G1 wins: the G1 wins editor of the pool (a win in its slot, click to remove; an empty slot lists the
// G1s she could enter then). Her schedule's races keep their win odds in the turn bar; a win shows at the turn picked for
// it, else the turn her schedule runs it. A race held in Classic and Senior is one win saddle (one race index).
function openDraftWins(d, r) {
  // the G1s she could enter, by turn (heldAt), and those her schedule runs, by race (runs: the first turn and its cell)
  const rows = r.schedule || [],
    runs = new Map(),
    heldAt = new Map();
  for (const w of rows) {
    const ix = CAREER.races.indexOf(w.name || w.race);
    if (ix < 0) continue;
    if (!heldAt.has(w.turn)) heldAt.set(w.turn, []);
    heldAt.get(w.turn).push(ix);
    if (w.run !== false && !runs.has(ix)) runs.set(ix, { turn: w.turn, cell: w.cells[0] });
  }
  const parentWon = (ix) => d.parents.some((p) => p.wins.includes(ix));
  openWins(d, umaName(d.chara), {
    offer: rows.length ? (t) => heldAt.get(t) || [] : null,
    info: (ix) => {
      const x = runs.get(ix);
      return x || parentWon(ix) ? { ...(x || {}), parent: parentWon(ix) } : null;
    },
    onEdit: editDraft,
    onClose: () => $('sim-wins')?.focus(),
  }); // the button was redrawn by the edits
}
// a career as portraits: trainee (outfit), then her two parents; parents: [{chara, outfit, label, her}]
function facesEl(tid, outfit, parents, outfitLabel = null) {
  const box = el('span', 'cfaces');
  const t = el('span', 'cface tr');
  t.append(umaAv(tid, outfit, 32), umaName(tid) + (outfitLabel ? ` ${outfitLabel}` : ''));
  box.appendChild(t);
  box.appendChild(el('span', 'cwith', 'with'));
  parents.forEach((p, j) => {
    const f = el('span', 'cface');
    f.append(umaAv(p.chara, p.outfit, 26), p.label);
    if (p.her) f.appendChild(el('span', 'her', 'her'));
    box.appendChild(f);
    if (!j) box.appendChild(el('span', 'cwith', '+'));
  });
  return box;
}
// a parent of a future career by legacy index (index = vets.length: the veteran she leaves, `who`)
const parentFace = (i, who) =>
  i >= vets.length
    ? { chara: who.chara, outfit: who.outfit, label: umaName(who.chara), her: true }
    : { chara: vets[i].chara, outfit: vets[i].outfit, label: `#${i + 1} ${umaName(vets[i].chara)}` };
// ---------- the veteran analysis (the veteran you got) ----------
// The decision is per veteran: enter the one you got (she starts as a random result, drawn as Reroll draws) and Compare runs both
// choices, running this career again or using her in the next one, on the plan's terms (expected careers to land).
const copyNode = (v) => ({
  chara: v.chara,
  ...(v.outfit ? { outfit: v.outfit } : {}),
  sparks: { ...v.sparks },
  pinks: { ...v.pinks },
  blues: { ...(v.blues || {}) },
  greens: { ...(v.greens || {}) },
  wins: [...v.wins],
});
function scheduledRaces(r) {
  const races = new Map();
  for (const w of (r.schedule || []).filter((w) => w.run !== false)) {
    const ix = CAREER.races.indexOf(w.name || w.race);
    if (ix < 0) continue;
    const prev = races.get(ix); // run at another turn too: p, the chance she wins it at least once
    races.set(ix, {
      ix,
      name: w.name || w.race,
      p: 1 - (1 - (prev?.p || 0)) * (1 - w.p),
      turns: [...(prev?.turns || []), w.turn],
    });
  }
  return [...races.values()];
}
// the skills she bought in a roll, given the sparks it landed (the report's per-target buying and landing chances):
// a landed one is bought, gold with chance land_gold / land; one that didn't land was bought anyway with chance
// (buy - land) / (1 - land), gold with chance (gold - land_gold) / (buy - land)
function rolledBought(r, sparks) {
  const bought = {};
  for (const t of r.targets) {
    const b = t.buy;
    if (!b) continue;
    const land = b.land_gold + b.land_white,
      buy = b.gold + b.white; // P(lands as a spark), P(bought)
    if (sparks[t.group]) bought[t.group] = Math.random() < (land > 0 ? b.land_gold / land : 0) ? 'gold' : 'white';
    else if (land < 1 && Math.random() < Math.max(0, buy - land) / (1 - land)) {
      const goldGivenUnlanded = buy - land > 0 ? Math.max(0, b.gold - b.land_gold) / (buy - land) : 0;
      bought[t.group] = Math.random() < goldGivenUnlanded ? 'gold' : 'white';
    }
  }
  return bought;
}
// her pink spark drawn (`drawPink`)
const rolledPink = (r) => drawPink(r.aptitudes);
// a spark's stars, drawn from the rank band's star mix (state.rank: the chances of 1, 2 and 3 stars)
function starDraw() {
  const mix = state.rank.split(',').map(Number);
  return () => {
    const u = Math.random();
    return u < mix[0] ? 1 : u < mix[0] + mix[1] ? 2 : 3;
  };
}
// a random result of this career: the set of landed targets drawn from its exact odds, each spark's stars from the rank,
// each G1 she runs won with its chance (it differs every time; the career's first showing is one of these too)
function rolledOutcome(r) {
  const star = starDraw();
  const sparks = {};
  if (Array.isArray(r.outcome_sets)) {
    // the landed set (a mask over the targets) drawn by its chance; the last if rounding leaves u past them all
    const u = Math.random();
    let cumulative = 0,
      mask = r.outcome_sets.length - 1;
    for (let i = 0; i < r.outcome_sets.length; i++) {
      cumulative += r.outcome_sets[i];
      if (u < cumulative) {
        mask = i;
        break;
      }
    }
    state.targets.forEach((t, i) => {
      if (mask & (1 << i)) sparks[t.group] = star();
    });
  } else
    r.targets.forEach((t) => {
      if (Math.random() < t.p) sparks[t.group] = star();
    }); // over 12 targets: each on its own odds
  const wins = scheduledRaces(r)
    .filter((x) => Math.random() < x.p)
    .map((x) => x.ix)
    .sort((a, b) => a - b);
  const bought = rolledBought(r, sparks),
    pink = rolledPink(r);
  return { sparks, bought, wins, pinks: pink.pinks, apt: pink.apt, ...rolledBlueGreen(r) };
}
// her blue (stat) spark: one of the five stats, near-equally likely (hakuraku: 20% each, a slight lean to her stronger
// stats); its stars follow that stat's value, which the model doesn't have: the rank band's star mix stands in. Her unique
// (green) spark: always (GameTora: guaranteed for an outfit at 3 stars or more; the page assumes yours are), stars by the
// rank band (hakuraku: as white sparks, not by the skill's level)
function rolledBlueGreen(r) {
  const star = starDraw();
  const blues = { [1 + Math.min(4, Math.floor(Math.random() * 5))]: star() };
  const out = r.trainee.outfits[0]?.id,
    greens = {};
  if (out && CAREER.uniques?.[out]) greens[out] = star();
  return { blues, greens };
}
function rollDraft() {
  const r = shown();
  if (!r || searching) return;
  Object.assign(simulation.draft, rolledOutcome(r), { winAt: {} });
  simulation.r = null;
  simulation.stale = false;
  const request = rollReq(simulation.draft, r);
  if (request == null) {
    compare();
    return;
  }
  cancelCompare();
  quickCompare(request);
}
// A roll's careers saved, asked of the plan for the career on show (`roll`: the plan job and her stars per target; the
// solver evaluates the plan's pick at that one outcome from what it kept of the plan, career/plan/roll.rs `roll_kept`, so no
// plan runs again unless the worker kept none): y, exact for her sparks, her G1 wins at their chances, over the plan's
// routes. It is no verdict: a route the plan has not (another lineage of a candidate's
// whose last career she improves as a grandparent, without lifting the candidate's odds) can save careers too, so
// every roll is confirmed by the exact comparison, its quick figure shown meanwhile as provisional. null when the plan
// isn't for this career (then Reroll compares exactly).
function rollReq(d, r) {
  if (planState.state !== 'done') return null;
  const o = planState.r?.outcomes;
  if (!o || !planState.job || (o.career ?? `${o.trainee}:${o.parents.join(',')}`) !== keyOf(r)) return null;
  const sp = d.sparks || {};
  return { job: planState.job, stars: state.targets.map((t) => sp[t.group] || 0) };
}
// the reroll's comparison, in two solver calls: the roll's saving from the plan the worker keeps (`next.roll`, about a
// millisecond), shown at once as Careers to loop; then the best career with her as a parent (one quick search, most of
// a second), which fills in her odds and the verdict's "Better odds". A reroll meanwhile waits behind that search (the
// worker keeps it and its caches; its answer is ignored)
async function quickCompare(request) {
  const id = ++simulationId,
    source = runId,
    gone = () => id !== simulationId || source !== runId;
  simulation.state = 'busy';
  simulation.kind = 'quick';
  simulation.error = null;
  simulation.stale = false;
  simulation.provisional = false;
  simulation.standing = false;
  renderSimulation();
  let y = null; // the roll's saving (careers), from the plan's routes (`rollReq`)
  // her legacy index is after every veteran's
  const base = comparisonJob(),
    legaciesWithHer = [...base.legacies, draftJob(simulation.draft)],
    herIndex = base.legacies.length;
  try {
    if (!simulation.baseline) simulation.baseline = (last.r.results || [])[0] || null;
    if (!simulation.baseline) throw new Error('No next career found.');
    const m = await runJob({ ...request.job, next: { ...request.job.next, roll: request.stars } });
    if (gone()) return;
    // (no saving from the plan: the exact comparison answers instead)
    const roll = m.r.next?.roll;
    if (m.r.error || typeof roll?.y !== 'number') {
      simulation.kind = null;
      runComparison();
      return;
    }
    y = roll.y;
    const v0 = /** @type {PlanDone} */ (planState).r.v0; // done: `request` came from its `rollReq`
    simulation.r = /** @type {CareerSearch} */ ({ results: [], pending: true });
    simulation.full = isFull(simulation.draft);
    simulation.v0 = { now: v0, withHer: v0 != null ? Math.max(0, v0 - y) : null }; // r: a stand-in, no search figures
    simulation.fixedFit = true; // (y is the plan's own scoring, under its fit)
    simulation.pickA = null;
    simulation.samePick = false;
    simulation.herParent = false;
    simulation.where = '';
    simulation.quick = true;
    simulation.state = 'done';
  } catch (e) {
    if (gone()) return;
    simulation.error = e.message;
    simulation.state = 'error';
  }
  simulation.kind = null;
  renderSimulation();
  if (simulation.state !== 'done' || gone()) return;
  // a roll that saves careers goes to the exact comparison (which finds the best career with her too) once it has stood
  // 0.8 s, as a zero does (rolling on starts none, so no running check is stopped, the worker with it): the plan's
  // saving can be off either way (it prices her in its own routes only: a few partners, one deck per lifted side, one C
  // per route), so the exact count replaces it, shown as checking meanwhile; a zero is checked too, after the best
  // career with her
  if (y >= 0.5) {
    simulation.standing = true;
    renderSimulation();
    setTimeout(() => {
      if (id === simulationId && source === runId && simulation.standing && !simulation.refining) {
        simulation.standing = false;
        runComparison(true);
      }
    }, 800);
    return;
  }
  try {
    const m = await runJob({ ...base, fix: [herIndex], legacies: legaciesWithHer, top: 1, compare: true });
    if (gone()) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    simulation.r = m.r;
  } catch (e) {
    if (gone()) return;
    simulation.error = e.message;
    simulation.state = 'error';
  }
  renderSimulation();
  // a zero is checked by the exact comparison too (the plan's routes are not every route a replan can take), shown
  // meanwhile as provisional
  // (after the roll has stood a moment: rolling on starts none; a running check would be stopped, the worker with it)
  if (simulation.state === 'done' && !gone()) {
    simulation.provisional = true;
    renderSimulation();
    setTimeout(() => {
      if (id === simulationId && source === runId && simulation.provisional && !simulation.refining)
        runComparison(true);
    }, 800);
  }
}
// A comparison in flight is for a veteran that is going (a roll, a reset, Compare pressed again): stop it. A wasm call
// can't be interrupted, so the worker goes (a fresh one starts loading at once). (No run is in flight: they wait for the answer.)
function cancelCompare() {
  if (simulation.refining) {
    simulationId++;
    simulation.refining = false;
    simulation.provisional = false;
    simulation.standing = false;
    restartEngine();
    return;
  }
  if (simulation.state !== 'busy') return;
  simulationId++;
  simulation.state = 'ready';
  if (simulation.kind === 'quick') return; // one quick search: it finishes, its answer ignored (the worker and its caches stay)
  restartEngine();
}
// where a comparison's saving lands: the next career when she is one of its parents; else a later career, named (the
// step of V's route, A, B or C, whose careers fell most) only where both plans' cheapest routes are the same trio, on
// the same sides (`sides`: her real veteran, or her best career now or another career of hers, each on its parents: a
// side whose parents changed is another route). The
// cheapest trios are often near-tied (`trios`, `near`), so between two different ones a per-uma step comparison names
// an arbitrary uma; what falls then is V, which the panel shows. planNow / planWithHer: the plans' `next` reports;
// pickA: the planner's next career with her; samePick: it is the one without her
function savedWhere({ planNow, planWithHer, pickA, samePick, herIndex }) {
  if (!pickA) return '';
  if (pickA.parents.includes(herIndex)) return `Improves the next career (${pickA.trainee.name}).`;
  const trioNow = planNow?.v0_plan,
    trioWithHer = planWithHer?.v0_plan;
  // (A and B are ordered by their careers, so the same trio may list them swapped; each side by its uma and its veteran,
  // `sides`: the same names on other veterans, or on other parents, are another route, whose per-step careers are not
  // comparable)
  /** @type {(v: NonNullable<PlanReport['v0_plan']>) => string} */
  const sidesOf = (v) =>
    [v.a, v.b]
      .map((trainee, i) => `${trainee}|${v.sides?.[i] ?? '?'}`)
      .sort()
      .join();
  const sameTrio =
    !!trioNow &&
    !!trioWithHer &&
    !!trioNow.sides &&
    !!trioWithHer.sides &&
    trioNow.c === trioWithHer.c &&
    sidesOf(trioNow) === sidesOf(trioWithHer);
  // the route's steps: [the uma of the step, its careers]
  /** @type {(v: NonNullable<PlanReport['v0_plan']>) => [string|null, number][]} */
  const steps = (v) => [v.a, v.b, v.c].map((trainee, i) => [trainee, v.careers[i]]);
  let best = null,
    gain = 0;
  if (sameTrio) {
    const careersNow = new Map(steps(trioNow));
    for (const [trainee, careers] of steps(trioWithHer)) {
      const fell = (careersNow.get(trainee) ?? careers) - careers;
      if (trainee && fell > gain) {
        gain = fell;
        best = trainee;
      }
    }
  }
  return `Improves a later ${best ? `${best} ` : ''}career; the next career ${samePick ? 'stays' : 'is'} ${pickA.trainee.name}.`;
}
function draftOf(r) {
  return {
    chara: r.trainee.id,
    outfit: r.trainee.outfits[0]?.id,
    borrowed: false,
    pinks: {},
    ...rolledOutcome(r),
    winAt: {},
    parents: r.parents.map((i) => copyNode(vets[i])),
  };
}
function draftJob(v) {
  const pairs = (o) => Object.entries(o).map(([g, s]) => [+g, s]);
  const node = (v) => ({ chara: v.chara || 0, sparks: pairs(v.sparks), pinks: pairs(v.pinks), wins: [...v.wins] });
  return { ...node(v), borrowed: false, parents: v.parents.map(node) };
}
// an edit to the entered veteran: the comparison on screen is for the old one
function editDraft() {
  if (simulation.r) simulation.stale = true;
  renderSimulation();
}
// Compare (or a roll): a press while a comparison runs restarts it with the veteran as she is now
function compare() {
  if (searching || !shown()) return;
  cancelCompare();
  runComparison();
}
// refine: a roll's quick result stays on screen (an improvement shown at once, its figures held) while this works out
// the exact count; otherwise the panel waits on it
async function runComparison(refine = false) {
  const id = ++simulationId,
    source = runId,
    gone = () => id !== simulationId || source !== runId;
  const base = comparisonJob(),
    key = JSON.stringify(base);
  if (refine) simulation.refining = true;
  else {
    simulation.state = 'busy';
    simulation.refining = false;
    simulation.provisional = false;
    simulation.standing = false;
  }
  simulation.error = null;
  simulation.stale = false;
  renderSimulation();
  try {
    // today's best career (without her): the run's, when it searched without places, else cached or searched now
    if (!simulation.baseline) {
      const open = !last.job.trainees.length && !last.job.fix.length;
      const cached = baselineCache?.key === key ? baselineCache.result : null;
      /** @type {{results: CareerReport[], error?: string}} */
      const before = cached ? { results: [cached] } : open ? last.r : (await runJob(base)).r;
      if (gone()) return;
      if (before.error || !before.results?.length)
        throw new Error(before.error || 'No career found in the current roster.');
      simulation.baseline = before.results[0];
      baselineCache = { key, result: simulation.baseline };
    }
    // the planner's question (exact odds, estimated careers): the best career with her as a parent, and the planner's
    // expected careers to a fully looped parent (each plan's `v0`, V) from the veterans now, without and with her
    const legaciesWithHer = [...base.legacies, draftJob(simulation.draft)],
      herIndex = base.legacies.length;
    const m = await runJob({ ...base, fix: [herIndex], legacies: legaciesWithHer, top: 1, compare: true });
    if (gone()) return;
    if (m.r.error || !m.r.results?.length) throw new Error(m.r.error || 'No next career found.');
    const full = isFull(simulation.draft);
    const planOf = (legacies) => ({
      ...base,
      top: ROLL_K,
      legacies,
      next: { ...PLAN_NEXT },
      compare: true,
      no_reports: true,
    }); // as a run's plan job, once she is added
    let planNow = planState.state === 'done' && !last.job.fix.length ? planState.r : null;
    if (!planNow) {
      const mb = await runJob(planOf(base.legacies));
      if (gone()) return;
      planNow = mb.r.next || null;
    }
    // with her: her pool priced under today's plan's fit (`next.fit`, passed on as the plan wrote it), so the two Vs
    // differ only by what she changes, not by Cont fitted again to the pool with her (a refit moves V by hundreds of
    // careers on a weak pool, in either direction). The plan Add to veterans shows is that refit, run on Add. Only when
    // today's plan has no V to compare with (no route: `v0` null, though its `fit` may still list candidates) or no fit
    // is the refit run here, and kept so that Add takes it as it is
    const fit = Number.isFinite(planNow?.v0) && planNow.fit?.length ? planNow.fit : null,
      jobWithHer = fit ? { ...planOf(legaciesWithHer), next: { fit } } : planOf(legaciesWithHer),
      ma = await runJob(jobWithHer);
    if (gone()) return;
    const planWithHer = ma.r.next || null;
    if (!fit && planWithHer && !planWithHer.error) planKept = { key: planKey(jobWithHer), r: planWithHer, ms: ma.ms };
    engineSaveState();
    const careersToLoop = (next) => (next && next.v0 != null ? next.v0 : null); // careers to a fully looped parent with the pool as it is
    const careersNow = careersToLoop(planNow),
      pickA = planWithHer?.options?.find((o) => o.chosen) || null,
      pickB = planNow?.options?.find((o) => o.chosen) || null;
    // not clamped: in the game a veteran never costs careers (you can leave her out), but V can rise with her even
    // under the same fit: each fresh side of V's route comes from one of a candidate's `fresh_lineages` best careers
    // now (her best and her next two by cached decks, each searched on its pair), so her arrival can push the lineage
    // the route used out of them, and a pair's search that runs out of budget can stop below the pair's best. The panel
    // shows such a rise as it is, with that said (`comparisonResult`)
    const careersWithHer = careersToLoop(planWithHer);
    // the planner's next career keeps its trainee and parents (her legacy index is after them, so they keep theirs)
    const samePick = !!(pickA && pickB && keyOf(pickA) === keyOf(pickB));
    simulation.r = m.r;
    simulation.full = full;
    simulation.v0 = { now: careersNow, withHer: careersWithHer };
    simulation.fixedFit = !!fit;
    simulation.pickA = pickA;
    simulation.samePick = samePick;
    simulation.herParent = !!pickA?.parents.includes(herIndex);
    simulation.where = savedWhere({ planNow, planWithHer, pickA, samePick, herIndex });
    simulation.quick = false;
    simulation.refining = false;
    simulation.provisional = false;
    simulation.standing = false;
    simulation.state = 'done';
  } catch (e) {
    if (gone()) return;
    if (simulation.refining) {
      simulation.refining = false;
      simulation.provisional = false;
      simulation.standing = false;
    } else {
      simulation.error = e.message;
      simulation.state = 'error';
    }
  }
  renderSimulation();
}
function addDraft() {
  if (searching || !simulation.draft) return;
  const d = structuredClone(simulation.draft);
  delete d.apt;
  delete d.bought;
  const v = newVet(d.chara, d.sparks);
  Object.assign(v, d, { src: 'entered', borrowed: false });
  plan.trainee = null;
  plan.parents = [null, null];
  $('v-msg').textContent =
    `Added ${umaName(v.chara)} to your veterans. The planner now picks the next career from all of them.`;
  changed();
}
// The veteran analysis: her card (details.js; she starts as a random result of this career) with its actions, and the
// comparison panel beside it.
function simulationBody() {
  const box = sectionEl('simulation', 'Veteran analysis');
  box.id = 'simulation';
  const d = simulation.draft,
    r = shown();
  if (!d || !r) return box;
  box.appendChild(analysisCard(d, r));
  const side = el('div', 'simcol');
  side.appendChild(comparisonPanel(d, r));
  box.appendChild(side);
  return box;
}
// her card, as the game's Umamusume Details screen: aptitudes at the end of this result, Skills / Inspiration (her
// sparks, editable) / Career Info (her G1s, the wins editor); in its header the career's odds and her actions
function analysisCard(d, r) {
  const wb = btn(
    `${d.wins.length ? d.wins.length : 'No'} G1 win${d.wins.length === 1 ? '' : 's'} · edit`,
    'vwins',
    () => openDraftWins(d, r),
  );
  wb.id = 'sim-wins';
  wb.dataset.k = 'sim:wins';
  wb.setAttribute('aria-haspopup', 'dialog');
  const card = detailsEl({
    d,
    outfit: r.trainee.outfits[0]?.label || null,
    apt: d.apt || r.aptitudes?.start || null,
    races: scheduledRaces(r),
    raceName: (ix) => CAREER.races[ix] || `G1 ${ix}`,
    onEdit: editDraft,
    winsBtn: wb,
    targets: r.targets,
    deck: r.deck,
    legacies: r.parents.map((i) => ({
      v: vets[i],
      label: `#${i + 1}`,
      open: () => openNode({ kind: 'v', id: vets[i].id }),
    })),
    parentLabel: (j) => `#${r.parents[j] + 1}`,
  });
  const actions = el('div', 'row');
  // Compare and a roll stay usable while a comparison runs: either one restarts it (not during a run, whose
  // answer replaces the career on show)
  const go = btn('Compare', 'primary', compare);
  go.id = 'sim-compare';
  go.dataset.k = 'sim:compare';
  go.disabled = searching;
  const roll = btn('Reroll', '', rollDraft);
  roll.id = 'sim-roll';
  roll.dataset.k = 'sim:roll';
  roll.disabled = searching;
  const add = btn('Add to veterans', '', addDraft);
  add.id = 'sim-add';
  add.disabled = searching;
  actions.append(go, roll, add);
  const mid = el('div', 'udmid');
  mid.append(oddsBtn(r), actions);
  card.querySelector('.udtop')?.insertBefore(mid, card.querySelector('.udname'));
  return card;
}
// the panel beside her card: a mini header carrying the chance a roll of this career improves your loop (the plan's
// `lift`: her outcomes that lower V), then the comparison (before any, a line saying what goes here)
function comparisonPanel(d, r) {
  const busy = simulation.state === 'busy';
  const result = el('div', `simresult${simulation.stale ? ' stale' : ''}`);
  result.id = 'sim-result';
  result.setAttribute('role', 'status');
  const hdr = el('div', 'simhdr');
  hdr.append(el('span', '', 'Comparison'));
  const opt = planState.state === 'done' ? planState.r.options?.find((o) => keyOf(o) === keyOf(r)) : null;
  if (opt && opt.lift != null) {
    const odds = el(
      'span',
      'simchance',
      opt.lift > 0 ? `1/${Math.max(1, Math.round(1 / opt.lift)).toLocaleString()} to improve` : 'none improve',
    );
    odds.id = 'sim-chance';
    odds.title = `The planner's chance that a roll of this career lowers the careers to a fully looped parent (${pct(opt.lift)})`;
    hdr.append(odds);
  }
  result.appendChild(hdr);
  if (!busy && !simulation.error && !simulation.r)
    result.appendChild(el('p', 'simidle', 'Compare or reroll to see if she improves your loop.'));
  if (busy) {
    const p = el('p', 'runnote');
    p.innerHTML = `${runnerHTML()}<span>Comparing…</span>`;
    result.appendChild(p);
  } else if (simulation.error) result.appendChild(el('p', 'note', simulation.error));
  else if (simulation.r) comparisonResult(result, d);
  return result;
}
// what she does for the loop if you enter her: one short line (`saves`: the careers she saves, null: not known). A
// roll's quick zero is the plan's routes' only (`rollReq`), always provisional until the exact comparison answers: it
// reads "no saving found yet"
function verdictText(d, next, saves) {
  const n = state.targets.length,
    fullyLooped = simulation.full && d.parents.length === 2 && d.parents.every(isFull),
    none = simulation.quick ? 'saving found yet' : 'careers saved';
  return fullyLooped
    ? 'Fully looped parent'
    : simulation.full
      ? `${n}/${n}: step 1 done`
      : (simulation.refining || simulation.standing) && !simulation.provisional
        ? 'Checking…'
        : saves != null && saves >= 0.5
          ? `Saves ${saves.toFixed(1)} careers`
          : simulation.herParent
            ? 'Parent of the next career'
            : next && next.p_all > simulation.baseline.p_all * (1 + 1e-9)
              ? `Better odds, no ${none}`
              : `No ${none}`;
}
// One figure per row: its label, then now → with her, green when she makes it better, red when worse (`direction`).
function figureEl({ label, now, withHer, direction, title }) {
  const c = el('div', 'tc');
  c.title = title;
  c.append(el('span', 'tl', label), el('b', `tv ${direction}`, `${now} → ${withHer}`));
  return c;
}
// a figure's direction: 'up' (better with her), 'down' (worse) or 'same' (within the estimate's noise, or unknown)
function direction(now, withHer, lowerIsBetter) {
  if (now == null || withHer == null) return 'same';
  const gain = lowerIsBetter ? now - withHer : withHer - now;
  return Math.abs(gain) < (lowerIsBetter ? 0.5 : 1e-9 * now) ? 'same' : gain > 0 ? 'up' : 'down';
}
// the comparison on show: the verdict in one line, then the two figures it rests on (now -> with her), then the best
// career with her
function comparisonResult(result, d) {
  if (simulation.stale)
    result.appendChild(
      el('p', 'note', 'These figures are for her result before your last edit. Compare to update them.'),
    );
  const before = simulation.baseline,
    n = state.targets.length,
    { now: careersNow, withHer: careersWithHer } = simulation.v0 || { now: null, withHer: null };
  const next = simulation.r.results[0]; // (none yet while the quick search for the best career with her runs)
  const saves = careersNow != null && careersWithHer != null ? careersNow - careersWithHer : null;
  const verdict = el('p', 'simverdict');
  verdict.id = 'sim-verdict';
  const head = verdictText(d, next, saves),
    rose = saves != null && saves <= -0.5;
  verdict.append(el('strong', '', head));
  if (/^Saves /.test(head) && simulation.where) verdict.append(simulation.where);
  // a rise (never from a roll's quick figures, which can't rise): shown as it is, and said to be the planner's
  if (rose)
    verdict.append(
      `The planner's estimate rises by ${(-saves).toFixed(1)} careers with her, a limit of its route (it offers each uma's few best careers as sides, and she can push the one it used out of them); leaving her out costs nothing.`,
    );
  result.appendChild(verdict);
  if (simulation.refining || simulation.standing || simulation.provisional) {
    const p = el('p', 'runnote simrefine');
    p.innerHTML =
      runnerHTML() +
      (simulation.provisional ? '<span>Checking…</span>' : '<span>Working out how many careers it saves…</span>');
    result.appendChild(p);
  }
  const figs = el('div', `simodds${simulation.refining || simulation.standing ? ' held' : ''}`);
  const nextWithHer = simulation.pickA
    ? `. With her the planner's next career is ${simulation.pickA.trainee.name} (${simulation.pickA.parents.map((i) => `#${i + 1}`).join(' + ')}).`
    : '.';
  const fromTable = simulation.quick
    ? " From the planner's table for this career (her sparks exact, her G1 wins at their chances; only the routes the plan kept); Compare checks this exact roll."
    : '';
  figs.appendChild(
    figureEl({
      label: 'Careers to loop',
      now: careersNow != null ? `about ${fmtN(careersNow)}` : '?',
      withHer: careersWithHer != null ? fmtN(careersWithHer) : '?',
      direction: rose ? 'same' : direction(careersNow, careersWithHer, true),
      title: `Expected careers to a fully looped parent: with your veterans now → with her too${nextWithHer}${fromTable}`,
    }),
  );
  // with her priced under today's fit (`fixedFit`): the plan after Add is fitted to the pool with her, so it can show
  // other figures (more careers or fewer), which this one line says
  const fitNote =
    simulation.fixedFit && careersNow != null
      ? el(
          'p',
          'simfit',
          "Counted with today's plan; after Add the planner refits it to your veterans with her, so its figures can differ.",
        )
      : null;
  // the best career with her as a parent: its odds against today's best, then folded its portraits and deck
  if (!next) {
    result.appendChild(figs);
    if (fitNote) result.appendChild(fitNote);
    const p = el('p', 'runnote simbest');
    p.innerHTML = `${runnerHTML()}<span>Finding the best career with her…</span>`;
    result.appendChild(p);
    return;
  }
  const dd = el('details', 'nextdeck');
  dd.appendChild(el('summary', '', 'Best career with her'));
  const nx = el('div', 'nextc');
  nx.append(
    facesEl(
      next.trainee.id,
      next.trainee.outfits[0]?.id,
      next.parents.map((i) => parentFace(i, d)),
      next.trainee.outfits[0]?.label || '',
    ),
  );
  figs.appendChild(
    figureEl({
      label: `All ${n} odds`,
      now: fmtIn(before.p_all),
      withHer: fmtIn(next.p_all),
      direction: direction(before.p_all, next.p_all, false),
      title: `All ${n}: today's best career → the best career with her as a parent (${pct(before.p_all)} → ${pct(next.p_all)}; ${n - 1} or more: ${fmtIn(before.p_minus1)} → ${fmtIn(next.p_minus1)})`,
    }),
  );
  result.appendChild(figs);
  if (fitNote) result.appendChild(fitNote);
  dd.append(nx, deckEl(next, 'Deck'));
  result.appendChild(dd);
}
function renderSimulation() {
  const ae = /** @type {HTMLElement} */ (document.activeElement),
    k = ae?.dataset?.k,
    top = $('sim-top');
  if (top) {
    top.replaceChildren(shown() ? simulationBody() : '');
    fitSparks(top);
  }
  if (k?.startsWith('sim:') && !ae.isConnected) document.querySelector(`[data-k="${CSS.escape(k)}"]`)?.focus();
}
// who an option places: only the places left open, the trainee and the parents (places: the places the table's careers
// fill: yours, or the plan's when the table is the last plan's)
function pickWhat(r, places = plan) {
  const what = el('div', 'altw');
  if (!places.trainee) {
    what.append(umaAv(r.trainee.id, r.trainee.outfits[0]?.id, 32), ` ${r.trainee.name}`);
  }
  const open = r.parents.filter((i) => !places.parents.includes(vets[i].id));
  if (open.length) {
    if (!places.trainee) what.append(', parents ');
    else what.append('Parents ');
    open.forEach((i, j) => {
      if (j) what.append(' + ');
      what.append(umaAv(vets[i].chara, vets[i].outfit, 24), ` #${i + 1} ${umaName(vets[i].chara)}`);
    });
  }
  return what;
}
function useBtn(r, places = plan) {
  const b = btn('Use this', '', () => {
    // her trainee, and the career's parents in the places left open (in the career's order)
    plan.trainee = r.trainee.id;
    const parents = places.parents.slice();
    const unplaced = r.parents.filter((i) => !parents.includes(vets[i].id));
    for (let j = 0; j < 2; j++) if (!parents[j]) parents[j] = vets[unplaced.shift()].id;
    plan.parents = parents;
    // a builder's other career, or the pair search's recommendation, keeps its deck: the job then reports that career
    // (`career`) instead of searching, while the setup it was chosen on stands (`chosenApplies`)
    chosen.career = null;
    chosenDropped = null;
    if ((r.alternative || r.pair_search) && r.career) {
      try {
        chosen.career = { places: JSON.stringify(plan), id: r.career, setup: chosenSetup(buildJob()) };
      } catch {}
    }
    changed();
  });
  b.disabled = searching; // (an old answer's option, while a run works on the next)
  return b;
}
const OPT_SHOW = 8;
const pairText = (o) => o.parents.map((i) => `#${i + 1}`).join(' + ');
const sameParents = (a, b) => a.parents.length === b.parents.length && a.parents.every((i) => b.parents.includes(i));
// the plan's pick and the rows about equal to it (`keyOf`): their Saves per run within TIE_CAREERS of the pick's. Only
// the plan's own rows and scores (a pair-search row not among them is weighed by its own note)
/** @type {(planReport: PlanReport|null) => {pick: PlanOption|null, tied: Set<string>}} */
function tiedRows(planReport) {
  const pick = planReport?.options.find((o) => o.chosen) || null,
    pickScore = pick ? scoreOf(pick) : null;
  if (pickScore == null) return { pick, tied: new Set() };
  const near = planReport.options.filter(
    (o) => o !== pick && scoreOf(o) != null && Math.abs(pickScore - scoreOf(o)) <= TIE_CAREERS,
  );
  return { pick, tied: new Set(near.map(keyOf)) };
}
// the note under the table on the rows about equal to the pick: up to TIE_NAMED of them by name with their Saves per
// run, else how many and how many the table lists (tagged); and those on the pick's parents
const TIE_NAMED = 4;
function tieText(planReport, pick, tied, nameOf, listed) {
  const rows = planReport.options.filter((o) => tied.has(keyOf(o))),
    same = rows.filter((o) => sameParents(o, pick)),
    named = rows.length <= TIE_NAMED,
    onPick = `on the pick's parents (${pairText(pick)})`;
  const who = named
    ? rows.map((o) => `${nameOf(o)} on ${pairText(o)} (${netText(scoreOf(o))})`).join(', ')
    : `${rows.length} careers, ${rows.filter((o) => listed.has(keyOf(o))).length} of them in the table (tagged)`;
  const shared = !same.length
    ? ''
    : same.length === rows.length
      ? ` ${rows.length === 1 ? 'It is' : 'All are'} ${onPick}.`
      : named
        ? ` ${same.map(nameOf).join(', ')} ${same.length === 1 ? 'is' : 'are'} ${onPick}.`
        : ` ${same.length} of them are ${onPick}.`;
  return `About equal to the pick (within ${TIE_CAREERS} careers per run, the planner's tolerance): ${who}.${shared} The pick, ${nameOf(pick)} on ${pairText(pick)} (${netText(scoreOf(pick))}), has the top score; running one of these instead costs at most ${TIE_CAREERS} careers by the planner's estimate.`;
}
// where no row the plan lists (every one a career you can run now) saves more than its own run (top Saves per run
// below 0): the line above the table, with the pick's net; under None left today, where the estimate's first step
// (`routeFirstStep`) takes a borrowed parent, it says so. Null otherwise
function negativeTopText(planReport, pick) {
  if (!planReport || !pick) return null;
  const top = Math.max(...planReport.options.map((o) => scoreOf(o) ?? Number.NEGATIVE_INFINITY));
  if (!(Number.isFinite(top) && top < 0)) return null;
  const text = `No career you can run now saves more than the run itself on today's estimate: the pick is the one that costs least (${netText(scoreOf(pick))} per run).`;
  const job = planState.state === 'done' ? planState.job : last?.job,
    step = job?.borrowed_parent === 'later' ? routeFirstStep(planReport) : null,
    borrowed = step ? step.parents.filter((i) => vets[i]?.borrowed) : [];
  if (!borrowed.length) return text;
  const who = borrowed.map((i) => `#${i + 1} ${umaName(vets[i].chara)}`).join(' and ');
  return `${text} The estimate's next step, ${step.what} on ${step.parents.map((i) => `#${i + 1}`).join(' + ')}, needs a borrowed parent (${who}), which ${BORROW_CHOICES.later} keeps out of this career.`;
}
function optionsEl() {
  const box = sectionEl('alts', 'Career choices');
  // (today's best by odds outside the places you chose is in the plan's output, not offered here)
  // with the plan: the 8 highest savings (every uma is scored), the best odds, the pick and the career on show; with
  // every place chosen, the last plan's table on the same setup and veterans, with your career in it (from the search
  // when the plan didn't list it: no saving), and a way back to the places it ran with
  const on = shown(),
    ref = refFor(),
    mineOnly = !!plan.trainee && plan.parents.every(Boolean);
  const planReport = planState.state === 'done' ? planState.r : mineOnly && ref ? ref.result : null,
    places = planState.state !== 'done' && mineOnly && ref ? ref.places : plan;
  const { pick, tied } = tiedRows(planReport);
  /** @type {OptionRow[]} the plan's rows, or the search's careers (no plan fields) */
  const list = planReport
    ? planReport.options.filter(
        (o, i) =>
          i < OPT_SHOW ||
          o.best_odds ||
          o.chosen ||
          unsettled(planReport).has(keyOf(o)) ||
          (on && keyOf(o) === keyOf(on)),
      )
    : last.r.results.slice();
  // the pair search's recommendation where the plan's rows don't hold it: a row run as it is, its deck kept (`useBtn`)
  // (a row the table already has gets its flag, pair-search score and id merged in, as a copy: the plan's own rows stay)
  const recommended = pairSearchRow(planReport);
  if (recommended) {
    const at = list.findIndex((o) => keyOf(o) === keyOf(recommended));
    if (at < 0) list.push(recommended);
    else
      list[at] = {
        ...list[at],
        pair_search: true,
        pair_search_score: recommended.pair_search_score,
        career: recommended.career,
      };
  }
  const mine = planReport && on && !list.some((o) => keyOf(o) === keyOf(on)) ? on : null;
  if (mine) list.unshift(mine);
  const tb = el('table', 'cartab optab');
  tb.innerHTML =
    '<thead><tr><th scope="col">Career</th><th scope="col"></th><th scope="col"></th><th scope="col"><span class="sr">Choose</span></th></tr></thead><tbody></tbody>';
  // both columns explain themselves on hover or keyboard focus (gmtip.js); the (i) popup says more about Saves per run
  const n = state.targets.length,
    ths = tb.tHead.rows[0].cells;
  const t = el('span', 'thtip', 'This career');
  t.title = `Her odds of landing all ${n} targets in this one run: 1 in N means about N runs of this career for one ${n}/${n}.`;
  t.tabIndex = 0;
  const sp = el('span', 'thtip', 'Saves per run');
  sp.title =
    "How much this career lowers the planner's estimate of the careers you still need, minus the run itself. For comparing careers, not a forecast.";
  sp.tabIndex = 0;
  ths[1].appendChild(t);
  ths[2].appendChild(sp);
  // "Saves per run": what it means, and why the best odds aren't the loop's best, with the plan behind the numbers
  // (the popup behind a small (i) right after the section's title, in its header bar)
  if (planReport) {
    const q = el('button', 'oinfo savewhy', 'i');
    q.type = 'button';
    q.id = 'save-why';
    q.dataset.k = 'save-why';
    q.setAttribute('aria-haspopup', 'dialog');
    q.setAttribute('aria-label', 'What these numbers mean');
    q.title = 'What these numbers mean';
    q.onclick = () => openSaveWhy(planReport);
    box.querySelector(':scope > h3').append(q);
  }
  const tieOf = (r) => (pick && tied.has(keyOf(r)) ? { sameParents: sameParents(r, pick) } : null);
  for (const r of list) tb.tBodies[0].appendChild(optionRow(r, { on, mine, planReport, places, tie: tieOf(r) }));
  const nameOf = (o) => `${o.trainee.name}${o.trainee.outfits[0] ? ` ${o.trainee.outfits[0].label}` : ''}`;
  // no row saves more than its own run on today's estimate: said above the table, with the pick's net
  const negative = negativeTopText(planReport, pick);
  if (negative) box.appendChild(el('p', 'hint optnote negtop', negative));
  const wrap = el('div', 'optwrap');
  wrap.appendChild(tb);
  box.appendChild(wrap);
  if (pick && tied.size) {
    const listed = new Set(list.map(keyOf));
    box.appendChild(el('p', 'hint optnote tienote', tieText(planReport, pick, tied, nameOf, listed)));
  }
  // the robustness check found the rule's pick changes with the tested variants (`robust`): every career it selects is in
  // the table; the pick stays the planner's rule's
  if (planReport && unsettled(planReport).size) {
    const names = list.filter((o) => unsettled(planReport).has(keyOf(o))).map(nameOf);
    box.appendChild(
      el(
        'p',
        'hint optnote',
        `The tested variants of the planner’s model (its building curve at the ends of its fitted range, every all-target chance at the ends of its calibration interval, and per-uma scenarios: the top-scoring uma’s all-target chance lower and a rival uma’s higher, by a split inferred from the spread measured between umas) score different careers highest, among the careers shown: ${names.join(', ')}. The pick is the rule’s under the model as fitted.`,
      ),
    );
  }
  for (const text of pairSearchNotes(planReport, nameOf)) box.appendChild(el('p', 'hint optnote', text));
  if (recommended) box.appendChild(deckEl(recommended, `Deck of the pair search's career: ${nameOf(recommended)}`));
  if (planReport?.check_unavailable)
    box.appendChild(
      el(
        'p',
        'hint optnote',
        'The check of this pick under tested variants of the model did not finish (the solver was stopped or restarted before it was done, or a variant needed more memory than it may take).',
      ),
    );
  // the plan's scorings, or its check's (its variants and pair search: `robust.scoring_complete`), left something out for
  // memory (false; absent from a solver without it): every saving shown, or the check's, may be low, so the pick or the
  // recommendation may not be the model's
  if (planReport?.scoring_complete === false || planReport?.robust?.scoring_complete === false)
    box.appendChild(
      el(
        'p',
        'hint optnote incomplete',
        'The planner ran out of memory before it could weigh every route or partner for these careers: their Saves per run may be too low, and the pick may not be the best.',
      ),
    );
  // a row whose career search was not complete (`proven: false`: its work cap, a band overflow, or her deck class
  // breached its cap, in this job or an earlier one, whatever the cap-free rerun found): the best it found, maybe not
  // her best. The rows don't say which
  // cause was theirs; the plan job's report names the causes it saw in any of its searches (`unprovenWhy`)
  if (planReport && list.some((o) => o.proven === false))
    box.appendChild(
      el(
        'p',
        'hint optnote',
        `The search could not prove the best career for some of these umas (${whyText(planReport.unproven_why || [])}): a better one may exist for them.`,
      ),
    );
  // (a row proven under a named assumption is proven: the run note names the job's assumptions, the plan's searches
  // included, `searchCaveat`, so the table adds no note for them)
  if (ref && JSON.stringify(ref.places) !== JSON.stringify(plan)) {
    const back = btn("Back to the planner's pick", '', () => {
      plan.trainee = ref.places.trainee;
      plan.parents = ref.places.parents.slice();
      changed();
    });
    back.id = 'opt-back';
    back.disabled = searching;
    const f = el('div', 'optfoot');
    f.appendChild(back);
    box.appendChild(f);
  }
  return box;
}
// the pair search's recommendation as a row of the table (`pairs.recommendation.report`, with its pair-search score and
// `pair_search`, so Use this keeps its deck): where it is not the plan's pick, once the check of this pick is complete
/** @type {(planReport: PlanReport|null) => OptionRow|null} */
const pairSearchRow = (planReport) => {
  const recommendation = planReport && checkDone(planReport)?.pairs?.recommendation;
  if (!recommendation?.report || recommendation.source === 'plan_pick') return null;
  const score = recommendation.score ?? null;
  return { ...recommendation.report, score, pair_search: true, pair_search_score: score };
};
// the careers the tested variants score highest where they disagree (`robust.contenders`, `pick_varies`): none until
// every variant is in, where they agree, or where the check is of another pick
const checkDone = (planReport) => {
  const check = planReport.robust;
  return check && check.pending === 0 && check.plan === planReport.outcomes?.career ? check : null;
};
const unsettled = (planReport) => {
  const check = checkDone(planReport);
  return new Set(check?.pick_varies ? check.contenders : []);
};
// the pair search's findings (`robust.pairs`): one line per trainee it ran for as the pick's or a per-uma scenario's
// (`reason` "pick" or "scenario"): her better parents (`better_parents`), by how much they beat her shown career (the pick,
// for the pick's trainee), or that none did; then, where the best (trainee, pair) it found by score (`pairs.best`) is
// another trainee's, a line naming it against the pick (`versus_pick`): better where it scores above the pick as fitted
// (the solver's `recommendation` then), with the per-uma scenarios that reverse that (`varies_under`) named as a
// sensitivity, else not better. Nothing until the check is complete, or where it is of another pick. nameOf: an
// option's trainee as the table names her
/** @type {(planReport: PlanReport|null, nameOf: (o: PlanOption) => string) => string[]} */
function pairSearchNotes(planReport, nameOf) {
  const check = planReport && checkDone(planReport);
  if (!check?.pairs) return [];
  const vetName = (i) => (vets[i] ? `#${i + 1} ${umaName(vets[i].chara)}` : `#${i + 1}`);
  const shownOf = (found) => planReport.options.find((o) => keyOf(o) === found.shown);
  // (a bound on the pairs scored per uma, `pair_bound`: past it, her pairs with the highest all-target ceilings)
  const bound = check.pairs.pair_bound;
  const truncatedText = (found) =>
    found.pairs_truncated
      ? `; of her ${found.pairs_runnable} pairs only the ${found.pairs_scored} with the highest all-target ceilings were checked`
      : '';
  const lines = check.pairs.trainees
    .filter((found) => (found.reason ?? 'pick') !== 'top')
    .map((found) => {
      const shown = shownOf(found),
        who = shown ? nameOf(shown) : found.trainee,
        others = Math.max(0, found.pairs_scored - 1);
      // (a gain on her shown parents themselves is a better deck, not better parents: the solver's `better_parents`)
      if (!found.better_parents)
        return `No better parents found for ${who} among the ${found.pairs_scored} pairs checked (each with the best deck found so far for it${truncatedText(found)}).`;
      const varies = (found.corner_margins || []).some((margin) => !(margin > 0))
        ? '; not better under every tested building curve'
        : '';
      // (her own odds read high or low by the measured per-uma spread: both careers scaled alike, `scale_margins`)
      const scaleVaries = (found.scale_margins || []).some((margin) => !(margin > 0))
        ? '; not better if her odds are off by the measured spread'
        : '';
      const overOthers =
        found.margin_over_other_pairs == null
          ? ''
          : ` and ${netText(found.margin_over_other_pairs)} vs the best of the ${others} other pairs checked`;
      return `Better parents found for ${who}: ${found.best_parents.map(vetName).join(' + ')} (Saves ${netText(found.margin_over_shown)} per run vs the shown parents${overOthers}; not certified best${varies}${scaleVaries}${truncatedText(found)}).`;
    });
  const best = check.pairs.best,
    pickFound = check.pairs.trainees[0],
    versus = best?.versus_pick;
  if (best && pickFound && versus && best.trainee !== pickFound.trainee) {
    const bestFound = check.pairs.trainees.find((found) => found.trainee === best.trainee),
      bestShown = bestFound && shownOf(bestFound),
      pickShown = shownOf(pickFound),
      who = bestShown ? nameOf(bestShown) : best.trainee,
      pickWho = pickShown ? nameOf(pickShown) : pickFound.trainee,
      side = `the pick (${pickWho})`,
      shared = check.pairs.shared_coverage,
      truncated = check.pairs.trainees.filter((found) => found.pairs_truncated).length,
      searched = truncated
        ? `${check.pairs.trainees.length} umas on their pairs (${truncated} on only their ${bound} with the highest all-target ceilings)`
        : `${check.pairs.trainees.length} umas on all their pairs`,
      coverage = shared ? `${searched}, ${shared.trainees} more on the best pairs found` : searched,
      where = `${who} on ${best.best_parents.map(vetName).join(' + ')}`,
      { nominal, pick_down: pickDown, pick_up: pickUp } = versus.margins;
    // (the per-uma scenarios that reverse the margin: a sensitivity of the choice, named, deciding nothing)
    const varies = versus.varies_under?.length
      ? ` The choice varies under the per-uma scenarios: ${netText(pickDown)} vs the pick with ${pickWho}'s all-target chance ×0.76 and hers ×1.32, ${netText(pickUp)} the reverse.`
      : '';
    if (nominal > 0)
      lines.push(
        `Better career found by the pair search (${coverage}): ${where} (Saves ${netText(best.score)} per run, ${netText(nominal)} vs ${side}; not certified best).${varies}`,
      );
    else
      lines.push(
        `No other uma's career found by the pair search (${coverage}) beats ${side}: the best, ${where}, saves ${netText(best.score)} per run, ${netText(nominal)} vs it.`,
      );
  }
  return lines;
}
// One career of the table: who it places, its tags, its odds now, its Saves per run, and Use this. on: the career on
// show; mine: your career when the plan didn't list it; planReport: the plan's report (null: no Saves per run); places:
// the places the table's careers fill; tie: about equal to the pick (`tiedRows`), with whether on the pick's parents
function optionRow(r, { on, mine, planReport, places, tie }) {
  const n = state.targets.length;
  const cur = on && keyOf(r) === keyOf(on),
    row = el('tr', cur ? 'on' : ''),
    c0 = el('th');
  c0.scope = 'row';
  c0.appendChild(pickWhat(r, places));
  const tags = el('span', 'otags');
  if (r === mine) tags.appendChild(el('span', 'otag', 'your choice'));
  if (planReport && r.chosen) tags.appendChild(el('span', 'otag pick', "planner's pick"));
  if (planReport && r.best_odds) tags.appendChild(el('span', 'otag', 'best odds now'));
  if (tie) {
    const a = el('span', 'otag tie', tie.sameParents ? 'about equal, same parents' : 'about equal');
    a.title = `About equal to the pick (within ${TIE_CAREERS} careers per run, the planner's tolerance)${tie.sameParents ? ", on the pick's parents" : ''}`;
    tags.appendChild(a);
  }
  if (planReport && r.pair_search) {
    const a = el('span', 'otag', 'pair search');
    const saves =
      r.pair_search_score == null ? '' : ` (Saves ${netText(r.pair_search_score)} per run in the pair search)`;
    a.title = `The pair search’s recommendation${saves}, see below: Use this runs it with this deck`;
    tags.appendChild(a);
  }
  if (planReport && unsettled(planReport).has(keyOf(r))) {
    const a = el('span', 'otag', 'pick varies');
    a.title =
      'At least one tested variant of the planner’s model scores this career highest (its building curve, the all-target chances’ calibration, or one uma’s calibration against another’s), and the variants disagree';
    tags.appendChild(a);
  }
  if (looped(r)) {
    const a = el('span', 'otag', `${n}/${n} parents`);
    a.title = `Both parents are ${n}/${n}s: if she lands all ${n}, her veteran is a fully looped parent`;
    tags.appendChild(a);
  }
  if (tags.childElementCount) c0.appendChild(tags);
  // the number carries the tooltip (its balloon points at the number, not the middle of the cell)
  const sv = el('td', 'orole'),
    num = el('span', 'onum', planReport ? (scoreOf(r) == null ? '–' : netText(scoreOf(r))) : '–');
  sv.append(num);
  if (planReport && r.index != null) {
    num.tabIndex = 0;
    num.title = `${roleText(r)}${r.lift > 0 ? `. ${pct(r.lift)} of her possible results lower the estimate` : ''}`;
  }
  row.append(c0, el('td', '', fmtIn(r.p_all)), sv);
  const act = el('td');
  act.appendChild(cur ? el('span', 'hint', 'shown above') : useBtn(r, places));
  row.appendChild(act);
  return row;
}
// "Saves per run" explained, in plain words, with the plan the numbers come from
let saveDlg = null;
function openSaveWhy(planReport) {
  if (!saveDlg) {
    saveDlg = el('dialog', 'gmdlg');
    saveDlg.id = 'save-dlg';
    saveDlg.setAttribute('aria-labelledby', 'save-title');
    saveDlg.addEventListener('close', () => $('save-why')?.focus());
    document.body.appendChild(saveDlg);
  }
  saveDlg.textContent = '';
  const n = state.targets.length;
  const h = el('h3', 'gmdhdr', 'What these mean');
  h.id = 'save-title';
  saveDlg.appendChild(h);
  const body = el('div', 'gmdbody savebody');
  // what the number is, the yardstick (the plan's route from today's veterans, step by step, with its careers) and how a
  // row is measured against it (run this career once first, then follow the route), with a worked example from the plan
  const routePlan = planReport.v0_plan,
    routeMembers = planReport.members || [],
    careersToday = planReport.v0 != null ? fmtN(planReport.v0) : null;
  const routeStep = (m, i) => {
    const careers = routePlan?.careers?.[i];
    const c = careers != null ? ` (about ${fmtN(careers)} careers)` : '';
    if (m.real) return `your ${n}/${n} of ${m.trainee} (no careers)`;
    const who = m.has_full_veteran ? `a fresh ${n}/${n} of ${m.trainee} (yours is the poorer parent): ` : '';
    // the career her fresh side is from: her best today, or (`alternative`) another of her top careers, which the
    // cheapest route takes for its parents
    const on = m.parents ? ` on parents ${m.parents.map((j) => `#${j + 1}`).join(' + ')}` : '',
      career = m.alternative
        ? `another of ${m.trainee}'s careers${on} (not her best today: the cheapest route takes it)`
        : `${m.trainee}'s best career${on}`;
    return m.building_careers > 0
      ? `${who}lift ${m.trainee}'s odds from ${career} (${fmtIn(m.p)} now) with about ${fmtN(m.building_careers)} building careers, to about ${fmtIn(m.p_switch)}, then rerun her until she lands ${n}/${n}${c}`
      : `${who}rerun ${career} (${fmtIn(m.p)}) until she lands ${n}/${n}${c}`;
  };
  const route =
    routePlan && routeMembers.length >= 2 && careersToday
      ? `First ${routeStep(routeMembers[0], 0)}; then ${routeStep(routeMembers[1], 1)}; then ${routePlan.c} with those two ${n}/${n}s as parents${routePlan.careers?.[2] != null ? ` (about ${fmtN(routePlan.careers[2])} careers)` : ''}. About ${careersToday} careers in all.`
      : 'No estimate: no route from the veterans you have now.';
  // the cheapest routes (`trios`, at most three, today's first) and how many are within the assumed 2% band of it
  // (`near`): too close for the estimate to say which is cheapest
  const trios = planReport.trios || [],
    nearCount = trios.filter((t) => t.near).length;
  const trioText =
    trios.length > 1
      ? `The ${trios.length} cheapest routes: ${trios.map((t) => `${t.a} and ${t.b}, then ${t.c} (about ${fmtN(t.value)} careers)`).join('; ')}. ${
          nearCount > 1
            ? `The first ${nearCount} are within 2% of each other (a band assumed for the estimate's error between routes): too close for it to say which is cheapest.`
            : `The others are more than 2% dearer (a band assumed for the estimate's error between routes).`
        }`
      : '';
  // the example: the route's first step of an uma without a full veteran yet, her rerun odds (after its building careers, if any) lifted by
  // 6% (one helpful parent)
  const example = routeMembers.find((m) => !m.real && (m.building_careers > 0 ? m.p_switch : m.p) > 0),
    exampleOdds = example && (example.building_careers > 0 ? example.p_switch : example.p);
  const exampleText = example
    ? `Say her veteran, as a parent, lifts ${esc(example.trainee)}'s odds for her reruns from ${fmtIn(exampleOdds)} to ${fmtIn(exampleOdds * 1.06)}. Those reruns then take <b>about ${(1 / exampleOdds - 1 / (exampleOdds * 1.06)).toFixed(1)} fewer careers</b>. Most of her possible results lift nothing, so the planner averages over all of them, weighted by how likely each is.`
    : '';
  // for anyone: the two columns in a sentence each, the key points in bold and italics, how the saving is worked out in
  // three plain sentences; today's estimate and a worked example behind dropdowns (no formula)
  const para = (html, cls = '') => {
    const x = el('p', cls);
    x.innerHTML = html;
    return x;
  };
  const drop = (title, ...kids) => {
    const d = el('details', 'savemore');
    d.append(el('summary', '', title), ...kids);
    return d;
  };
  body.append(
    para(`<b>This career</b>: her chance of landing all ${n} targets in this one run.`),
    para(
      `<b>Saves per run</b>: how much running this career next lowers the planner's estimate (below) of the careers you still need for a <i>fully looped parent</i> (an ${n}/${n} whose parents are both ${n}/${n}), <i>counting the run itself</i>. <b>+5</b> means about five careers fewer by that estimate. It is for <i>comparing careers</i>, not a forecast of careers you will save.`,
    ),
    para(
      `The best odds aren't always the best choice: a career can be worth more for <b>the veteran she leaves behind</b>, who becomes a parent for your next runs.`,
    ),
    para(
      `The pick is the career with the <b>most Saves per run</b>. Better odds don't break a near-tie: the Saves per run already count the runs where she lands every target.`,
    ),
    el('div', 'gmdsub', 'How it is worked out'),
    para(
      `The planner makes a <b>rough estimate</b> of how many careers you still need${careersToday ? ` (<b>about ${careersToday}</b> today)` : ''}. Then, for every way this career could turn out, it adds her veteran to your parents and estimates again. <i>How much the estimate drops</i>, minus the run itself, is the saving.`,
    ),
    drop(
      "Today's estimate, step by step",
      el('p', 'oloop', route),
      ...(trioText ? [el('p', 'oloop', trioText)] : []),
      para(
        `A <i>yardstick</i>, not advice: it assumes the simplest road (${n}/${n}s of two umas, then a career of a third with them as parents), and only its total is used.`,
      ),
    ),
  );
  if (exampleText) body.append(drop('An example', para(exampleText)));
  body.append(
    para(`The whole method: <a href="about#career-planner">How this works, the career planner in detail</a>.`, 'hint'),
  );
  saveDlg.appendChild(body);
  const f = el('div', 'gmdfoot'),
    c = el('button', 'btn gmclose', 'Close');
  c.type = 'button';
  c.onclick = () => saveDlg.close();
  f.appendChild(c);
  saveDlg.appendChild(f);
  saveDlg.showModal();
  c.focus();
}
function renderResults() {
  const box = $('results');
  box.textContent = '';
  const r = shown();
  if (!r) {
    renderSimulation();
    return;
  }
  const top = el('div');
  top.id = 'sim-top';
  box.append(top, targetsEl(r));
  // the table: the plan's careers, or the search's with a place open; with every place chosen, the last plan's (refFor)
  const open = !plan.trainee || plan.parents.some((p) => !p);
  if ((open && (planState.state === 'done' ? planState.r.options : last.r.results).length > 1) || refFor())
    box.appendChild(optionsEl());
  renderSimulation();
  markOwn();
}

// ---------- the veterans (pool) ----------
function vetCardEl(v, i) {
  const card = el('article', `vet${v.borrowed ? ' bor' : ''}`);
  card.id = `vet-${i}`;
  const top = el('div', 'vtop');
  top.appendChild(el('span', 'vno', `#${i + 1}`));
  const seg = el('div', 'seg sm');
  seg.setAttribute('role', 'group');
  seg.setAttribute('aria-label', `Veteran #${i + 1}: yours or borrowed`);
  for (const [b, l] of /** @type {[boolean, string][]} */ ([
    [false, 'Yours'],
    [true, 'Borrowed'],
  ])) {
    const x = el('button', '', l);
    x.type = 'button';
    x.setAttribute('aria-pressed', String(v.borrowed === b));
    x.onclick = () => {
      if (v.borrowed !== b) {
        v.borrowed = b;
        $('v-msg').textContent = `Moved ${umaName(v.chara)} (#${i + 1}) to ${l}.`;
        changed();
      }
    };
    seg.appendChild(x); // she moves to that tab
  }
  top.appendChild(seg);
  const k = plan.parents.indexOf(v.id);
  if (k >= 0) top.appendChild(el('span', 'vin', `Parent ${k + 1}`));
  top
    .appendChild(
      btn('Remove', 'vdel', () => {
        const index = idxOf(v.id);
        if (index < 0) return;
        removeVet(index);
        $('v-msg').textContent = `Removed ${umaName(v.chara)} from your veterans.`;
      }),
    )
    .setAttribute('aria-label', `Remove veteran #${i + 1}`);
  card.appendChild(top);
  // her portrait, name, targets and G1 wins, her parents' faces; a click opens her popup (sparks editable in place)
  const tg = state.targets.map((t) => t.group),
    have = tg.filter((g) => v.sparks?.[g]).length;
  const t = el('button', 'vtile');
  t.type = 'button';
  t.dataset.k = `node:v${v.id}`;
  t.setAttribute('aria-haspopup', 'dialog');
  t.setAttribute(
    'aria-label',
    `Veteran #${i + 1}: ${umaName(v.chara)}, ${have} of ${tg.length} target skills. Her sparks`,
  );
  const nm = el('span', 'tnm');
  nm.append(
    el('span', 'nm', umaName(v.chara)),
    el(
      'span',
      'sub',
      `${have}/${tg.length} targets · ${v.wins.length ? v.wins.length : 'no'} G1 win${v.wins.length === 1 ? '' : 's'}`,
    ),
  );
  const ps = el('span', 'vtps');
  for (const g of v.parents) ps.appendChild(umaAv(g.chara || 0, g.outfit, 26));
  t.append(umaAv(v.chara, v.outfit, 48), nm, ps);
  t.onclick = () => openNode({ kind: 'v', id: v.id });
  card.appendChild(t);
  return card;
}
// your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog
// (the cards are drawn only while the dialog is open, and when it opens: every edit and result redraws the page)
// Two tabs over the one list, yours and borrowed; a veteran keeps her number (her place in the whole list) in either.
let poolStale = true,
  vTab = 'own';
const inTab = (v) => !!v.borrowed === (vTab === 'bor');
function setVTab(t) {
  vTab = t;
  for (const x of ['own', 'bor']) $(`v-tab-${x}`).setAttribute('aria-selected', String(x === t));
  $('vets').setAttribute('aria-labelledby', `v-tab-${t}`);
  renderPool();
}
function renderPool() {
  if (nodeDlg?.open && nodeOpen?.kind === 'v') fillNode();
  const nb = vets.filter((v) => v.borrowed).length;
  $('v-count').textContent = vets.length
    ? `${vets.length} veteran${vets.length > 1 ? 's' : ''}${nb ? `, ${nb} borrowed` : ''}.`
    : '';
  $('v-tab-own').textContent = `Yours · ${vets.length - nb}`;
  $('v-tab-bor').textContent = `Borrowed · ${nb}`;
  if (!$dialog('set-vets').open) {
    poolStale = true;
    return;
  }
  drawPool();
}
function drawPool() {
  const box = $('vets');
  box.textContent = '';
  poolStale = false;
  vets.forEach((v, i) => {
    if (inTab(v)) box.appendChild(vetCardEl(v, i));
  });
  if (!box.children.length)
    box.innerHTML =
      vTab === 'own'
        ? '<p class="hint">No veterans of yours yet. Import umadump\'s trained_chara_data.json, or make one with Choose parent.</p>'
        : '<p class="hint">None borrowed yet. Import umadump\'s rental_chara_data.json (the borrow list on the career start screen), or mark a veteran Borrowed.</p>';
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
  initTargets();
  initBorrow();
  renderRoster();
  renderTargets();
  enginePersist(); // the solver's kept work survives a page reload
  initSetup();
  initFold();
  // a settings change (targets, umas, cards, scenario): the tree shows the new targets, and the search runs again
  onSave(() => {
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
  const rank = state.rank === '0.20,0.70,0.10' ? 'SS' : 'B–S+',
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
    } catch {}
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
  } catch {}
  const data = { app: EXPORT_APP, version: 1, era: ERA, exported: new Date().toISOString(), veterans: vets, plan, job };
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
    } catch {}
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
    const damaged = own.veterans.filter((v) => !isVeteran(v)).length;
    if (damaged) {
      $('v-msg').textContent =
        `Not imported: ${damaged} veteran${damaged === 1 ? '' : 's'} in that export ${damaged === 1 ? 'is' : 'are'} damaged (no id or two parents).`;
      return true;
    }
    // each keeps her source (one made by hand stays hand-made); an export from before sources were saved: imported
    importVets(
      own.veterans.map((v) => ({ pinks: {}, wins: [], src: 'import', ...v })),
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
    $('v-msg').textContent =
      `Added ${tree.map((v) => umaName(v.chara)).join(', ')} from an uma.moe tree, with her parents, as #${vets.length - tree.length + 1}${tree.length > 1 ? `-#${vets.length}` : ''} (marked yours: switch to Borrowed if she is a friend's).`;
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
  () => {
    initSettings();
    initPopover();
    dropStaleChosen();
    // A result names parents by their place in the veteran list: adding, removing or importing veterans changes the
    // places, so the answer on screen goes; an edit to a veteran keeps it on screen, dimmed, until the new one is ready.
    const vetsSig = () => vets.map((v) => v.id).join(',');
    let sig = vetsSig();
    onVetsChange(() => {
      if (vetsSig() !== sig) {
        sig = vetsSig();
        last = null;
        simulation = { state: 'none' };
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
      const n = vets.filter(inTab).length,
        what = vTab === 'own' ? `${n} of your veterans` : `${n} borrowed veteran${n === 1 ? '' : 's'}`;
      if (n && confirm(`Remove all ${what}?`)) {
        clearVets(inTab);
        $('v-msg').textContent = `Removed ${what}.`;
      }
    };
    for (const x of ['own', 'bor']) $(`v-tab-${x}`).onclick = () => setVTab(x);
    renderAll();
    search();
  },
  (e) => {
    status = { kind: 'error', text: e.message };
    paint();
  },
);
