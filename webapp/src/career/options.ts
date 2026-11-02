// The options table ("Other careers") under the career on show, as figures and words: the plan's careers by Saves per
// run (or the search's, with a place open), what each row's tags and numbers say, the table's notes, and Use this
// (`takeCareer`). view/Options.tsx draws it and its Saves popup. It reads the answer from the answer store
// (career/state.ts) and the career's helpers from job.ts, current.ts and keys.ts; its one write there is
// `forgetDroppedChoice`.
import { settings } from '../state/settings.ts';
import { pct } from '../model/odds.ts';
import { batch } from '@preact/signals';
import { places as yourPlaces, usePlaces, setChosen } from '../veterans/store.ts';
import { umaName } from '../veterans/names.ts';
import { whyText } from '../model/proofs.ts';
import { answer, forgetDroppedChoice } from './state.ts';
import { keyOf } from './keys.ts';
import { Blocked, chosenSetup, placesKey, BORROW_CHOICES } from './job.ts';
import { currentJob, legacyVet, poolNumber, refFor, shown } from './current.ts';
import { assert, must } from '../must.ts';
import type { OptionRow, PairSearch, PlanOption, PlanReport } from './job.ts';
import type { DeepReadonly } from '../state/store.ts';

/** a note under the table: `id` names it among the others (its kind; a pair-search note, its uma) */
export interface Note {
  id: string;
  text: string;
}
/** a row of the table, as the answer store hands it (read-only) */
type Row = DeepReadonly<OptionRow>;
/** the plan's report, as the answer store hands it */
type Plan = DeepReadonly<PlanReport>;
/** the pair search's entry for one trainee */
type PairFound = DeepReadonly<PairSearch['trainees'][number]>;
// an option's role, in plain words: one of the two parents of today's estimate (the route the (i) popup walks through)
// or its last run, else what her run does for it: the side her veteran's routes lift most often where she lowers it,
// with that chance per run, or her own n/n as a side (`lifts_side`); a loop member says so first
export const roleText = (o: Row): string => {
  const n = settings.value.targets.length;
  if (o.role === 'plan') return 'Makes one of the two parents your fully looped parent needs';
  if (o.role === 'finish') return 'The last run: both her parents are already full veterans';
  const loop = o.role === 'loop' ? 'Already in your loop. ' : '',
    side = o.lifts_side;
  if (!side) return `${loop}Not compared against the plan`;
  if (side.uma != null && side.chance >= side.own)
    return `${loop}Her veteran helps build ${umaName(side.uma)}'s parent: ${pct(side.chance)} of her runs do`;
  if (side.own > 0)
    return `${loop}Her veteran can be one of the two parents (an ${n}/${n}): ${pct(side.own)} of her runs`;
  return `${loop}No run of hers helps the plan`;
};
// δ, the per-decision tolerance (AGENTS.md, The career planner's goal and its bar): a row whose Saves per run is within
// it of the pick's is shown as about equal to the pick. Display only: the pick stays the strict top score
export const TIE_CAREERS = 0.25;
// a career's parents, by legacy index, from its id (`kept::career_id`: "trainee:first,second:deck")
const careerParents = (id: string): number[] => {
  const [, first, second] = /^\d+:(\d+),(\d+)/.exec(id) ?? [];
  assert(first !== undefined && second !== undefined, 'a career id names its two parents (kept::career_id)');
  return [Number(first), Number(second)];
};
// the first step of today's estimate (`members`, A then B, as planval's `route_first_step` reads them): the first side
// without a real full veteran of hers, her builder's career where she builds (`builder`) or else her own (`parents`);
// with both sides real, C's run on those two. Its parents by legacy index, and what it is; null without a route, or
// where it builds and names no builder (her Cont given)
function routeFirstStep(planReport: Plan): { what: string; parents: readonly number[] } | null {
  const members = planReport.members || [];
  if (members.length < 2 || planReport.v0 == null) return null;
  for (const m of members) {
    if (m.real) continue;
    if (m.building_careers > 0)
      return m.builder ? { what: `a builder for ${m.trainee}`, parents: careerParents(m.builder) } : null;
    return m.parents ? { what: `${m.trainee}'s run`, parents: m.parents } : null;
  }
  const route = planReport.v0_plan;
  // (her real veteran's side, "real:i": veteran i; the others are careers to run)
  const real = (route?.sides || []).flatMap((side) => {
    const [, i] = /^real:(\d+)$/.exec(side) ?? [];
    return i === undefined ? [] : [Number(i)];
  });
  return route && real.length === 2 ? { what: `${route.c}'s run`, parents: real } : null;
}
// an option's net saving: the careers her veteran saves toward a fully looped parent on average, minus the one it costs
export const netText = (x: number): string => (x >= 0 ? '+' : '−') + Math.abs(x).toFixed(1);
// an option's Saves per run: the plan's `score` (her net saving; null when the plan scored nothing), else `index`
const scoreOf = (o: Row): number | null | undefined => o.score ?? o.index;

// Use this: her trainee, and the career's parents in the places left open (in the career's order); one batch: the page
// reacts once, to the places and the choice together. places: the places the table's careers fill
/** `r`: a row (a builder's other career has `alternative`) */
export function takeCareer(
  r: Row & { alternative?: boolean },
  places: { trainee: number | null; parents: readonly (string | null)[] },
): void {
  // the career's parents by id (`legacyVet`); one who has left the pool under the answer on show: the row names a
  // veteran that is gone, so nothing is placed (the answer goes once the pool's window closes: career/main.tsx)
  const ids: string[] = [];
  for (const i of r.parents) {
    const v = legacyVet(i);
    if (!v) return;
    ids.push(v.id);
  }
  const parents = places.parents.slice();
  const unplaced = ids.filter((id) => !parents.includes(id));
  for (let j = 0; j < 2; j++)
    if (!parents[j]) parents[j] = must(unplaced.shift(), "a place left open has one of the career's parents left");
  batch(() => {
    // (a change of setup hands the places back to the planner: main.tsx initSettings)
    usePlaces(r.trainee.id, parents);
    // a builder's other career, or the pair search's recommendation, keeps its deck: the job then reports that
    // career (`career`) instead of searching, while the setup it was chosen on stands (`chosenApplies`); the job is
    // built on the places just written
    forgetDroppedChoice();
    if ((r.alternative || r.pair_search) && r.career) {
      try {
        setChosen({ places: placesKey(yourPlaces.value), id: r.career, setup: chosenSetup(currentJob()) });
      } catch (e) {
        if (!(e instanceof Blocked)) throw e;
        // settings that make no job: nothing is chosen
      }
    }
  });
}
const OPT_SHOW = 8;
// where no row the plan lists (every one a career you can run now) saves more than its own run (top Saves per run
// below 0): the line above the table, with the pick's net; under None left today, where the estimate's first step
// (`routeFirstStep`) takes a borrowed parent, it says so. Null otherwise
function negativeTopText(planReport: Plan | null, pick: Row | null): string | null {
  if (!planReport || !pick) return null;
  const top = Math.max(...planReport.options.map((o) => scoreOf(o) ?? Number.NEGATIVE_INFINITY));
  if (!(Number.isFinite(top) && top < 0)) return null;
  const text = 'No career saves its own run today: the pick costs least.';
  const { last, planState } = answer.value,
    job = planState.state === 'done' ? planState.job : last?.job,
    step = job?.career.borrowed_parent === 'later' ? routeFirstStep(planReport) : null;
  // the step's borrowed parents (one who has left the pool under the answer, `legacyVet`, is none of them)
  const borrowed: string[] = [];
  for (const i of step ? step.parents : []) {
    const v = legacyVet(i);
    if (v?.borrowed) borrowed.push(`${poolNumber(v)} ${umaName(v.chara)}`);
  }
  if (!borrowed.length) return text;
  const who = borrowed.join(' and ');
  return `${text} Its next step needs a borrowed parent (${who}), kept out by ${BORROW_CHOICES.later}.`;
}
// The table's rows and what goes with them. With the plan: the 8 highest savings (every uma is scored), the best odds,
// the pick and the career on show; with every place chosen, the last plan's table on the same setup and veterans, with
// your career in it (from the search when the plan didn't list it: no saving), and a way back to the places it ran with
// (`back`). (Today's best by odds outside the places you chose is in the plan's output, not offered here.) The notes
// go in the (i) popup, not under the table: no career saves its run; the variants, the pair search, a check that
// didn't finish, memory, an unproven search.
export function optionsTable() {
  const on = shown(),
    ref = refFor(),
    plan = yourPlaces.value,
    mineOnly = !!plan.trainee && plan.parents.every(Boolean),
    { last, planState } = answer.value;
  const planReport = planState.state === 'done' ? planState.r : mineOnly && ref ? ref.result : null,
    places = planState.state !== 'done' && mineOnly && ref ? ref.places : plan;
  const pick = planReport?.options.find((o) => o.chosen) || null;
  // the plan's rows, or the search's careers (no plan fields)
  const list: Row[] = planReport
    ? planReport.options.filter(
        (o, i) =>
          i < OPT_SHOW ||
          o.best_odds ||
          o.chosen ||
          unsettled(planReport).has(keyOf(o)) ||
          (on && keyOf(o) === keyOf(on)),
      )
    : must(last, 'the options table draws a career on show').r.results.slice();
  // the pair search's recommendation where the plan's rows don't hold it: a row run as it is, its deck kept (`takeCareer`)
  // (a row the table already has gets its flag, pair-search score and id merged in, as a copy: the plan's own rows stay)
  const recommended = pairSearchRow(planReport);
  if (recommended) {
    const at = list.findIndex((o) => keyOf(o) === keyOf(recommended)),
      listed = list[at]; // (none: the table doesn't hold it)
    if (!listed) list.push(recommended);
    else
      list[at] = {
        ...listed,
        pair_search: true,
        pair_search_score: recommended.pair_search_score,
        career: recommended.career,
      };
  }
  const mine = planReport && on && !list.some((o) => keyOf(o) === keyOf(on)) ? on : null;
  if (mine) list.unshift(mine);
  const nameOf = (o: Row): string => `${o.trainee.name}${o.trainee.outfits[0] ? ` ${o.trainee.outfits[0].label}` : ''}`;
  const notes: Note[] = [];
  const negative = negativeTopText(planReport, pick);
  if (negative) notes.push({ id: 'negative', text: negative });
  // the robustness check found the rule's pick changes with the tested variants (`robust`): every career it selects is in
  // the table; the pick stays the planner's rule's
  if (planReport && unsettled(planReport).size) {
    const names = list.filter((o) => unsettled(planReport).has(keyOf(o))).map(nameOf);
    notes.push({
      id: 'variants',
      text: `The tested variants of the planner’s model (its building curve at the ends of its fitted range, every all-target chance at the ends of its calibration interval, and per-uma scenarios: the top-scoring uma’s all-target chance lower and a rival uma’s higher, by a split inferred from the spread measured between umas) score different careers highest, among the careers shown: ${names.join(', ')}. The pick is the rule’s under the model as fitted.`,
    });
  }
  notes.push(...pairSearchNotes(planReport, nameOf));
  if (planReport?.check_unavailable)
    notes.push({
      id: 'unavailable',
      text: 'The check of this pick under tested variants of the model did not finish (the solver was stopped or restarted before it was done, or a variant needed more memory than it may take).',
    });
  // the plan's scorings, or its check's (its variants and pair search: `robust.scoring_complete`), left something out for
  // memory (false; absent from a solver without it): every saving shown, or the check's, may be low, so the pick or the
  // recommendation may not be the model's
  if (planReport?.scoring_complete === false || planReport?.robust?.scoring_complete === false)
    notes.push({
      id: 'memory',
      text: 'The planner ran out of memory before it could weigh every route or partner for these careers: their Saves per run may be too low, and the pick may not be the best.',
    });
  // a row whose career search was not complete (`proven: false`: its work cap, a band overflow, or her deck class
  // breached its cap, in this job or an earlier one, whatever the cap-free rerun found): the best it found, maybe not
  // her best. The rows don't say which
  // cause was theirs; the plan job's report names the causes it saw in any of its searches (`unprovenWhy`)
  if (planReport && list.some((o) => o.proven === false))
    notes.push({
      id: 'unproven',
      text: `The search could not prove the best career for some of these umas (${whyText(planReport.unproven_why || [])}): a better one may exist for them.`,
    });
  // (a row proven under a named assumption is proven: the run note names the job's assumptions, the plan's searches
  // included, `searchCaveat`, so the table adds no note for them)
  const back = ref && placesKey(ref.places) !== placesKey(yourPlaces.value) ? ref.places : null;
  return { on, planReport, places, pick, list, mine, recommended, notes, nameOf, back };
}
// the pair search's recommendation as a row of the table (`pairs.recommendation.report`, with its pair-search score and
// `pair_search`, so Use this keeps its deck): where it is not the plan's pick, once the check of this pick is complete
const pairSearchRow = (planReport: Plan | null): Row | null => {
  const recommendation = planReport && checkDone(planReport)?.pairs?.recommendation;
  if (!recommendation?.report || recommendation.source === 'plan_pick') return null;
  const score = recommendation.score ?? null;
  return { ...recommendation.report, score, pair_search: true, pair_search_score: score };
};
// the careers the tested variants score highest where they disagree (`robust.contenders`, `pick_varies`): none until
// every variant is in, where they agree, or where the check is of another pick
const checkDone = (planReport: Plan) => {
  const check = planReport.robust;
  return check && check.pending === 0 && check.plan === planReport.outcomes?.career ? check : null;
};
export const unsettled = (planReport: Plan): Set<string> => {
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
function pairSearchNotes(planReport: Plan | null, nameOf: (o: DeepReadonly<PlanOption>) => string): Note[] {
  if (!planReport) return [];
  const check = checkDone(planReport);
  if (!check?.pairs) return [];
  // (one who has left the pool under the answer, `legacyVet`: her number alone)
  const vetName = (i: number): string => {
    const v = legacyVet(i);
    return v ? `${poolNumber(v)} ${umaName(v.chara)}` : `#${i + 1}`;
  };
  const shownOf = (found: PairFound) => planReport.options.find((o) => keyOf(o) === found.shown);
  // (a bound on the pairs scored per uma, `pair_bound`: past it, her pairs with the highest all-target ceilings)
  const bound = check.pairs.pair_bound;
  const truncatedText = (found: PairFound): string =>
    found.pairs_truncated
      ? `; of her ${found.pairs_runnable} pairs only the ${found.pairs_scored} with the highest all-target ceilings were checked`
      : '';
  const lines = check.pairs.trainees
    .filter((found) => (found.reason ?? 'pick') !== 'top')
    .map((found): Note => {
      const id = `pair:${found.trainee}`;
      const shown = shownOf(found),
        who = shown ? nameOf(shown) : found.trainee,
        others = Math.max(0, found.pairs_scored - 1);
      // (a gain on her shown parents themselves is a better deck, not better parents: the solver's `better_parents`)
      if (!found.better_parents)
        return {
          id,
          text: `No better parents found for ${who} among the ${found.pairs_scored} pairs checked (each with the best deck found so far for it${truncatedText(found)}).`,
        };
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
      return {
        id,
        text: `Better parents found for ${who}: ${found.best_parents.map(vetName).join(' + ')} (Saves ${netText(found.margin_over_shown)} per run vs the shown parents${overOthers}; not certified best${varies}${scaleVaries}${truncatedText(found)}).`,
      };
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
      lines.push({
        id: 'pair:best',
        text: `Better career found by the pair search (${coverage}): ${where} (Saves ${netText(best.score)} per run, ${netText(nominal)} vs ${side}; not certified best).${varies}`,
      });
    else
      lines.push({
        id: 'pair:best',
        text: `No other uma's career found by the pair search (${coverage}) beats ${side}: the best, ${where}, saves ${netText(best.score)} per run, ${netText(nominal)} vs it.`,
      });
  }
  return lines;
}
// a row's extra careers against the pick (the pick's Saves per run minus the row's): "≈0" within the tie tolerance, "–"
// for the pick itself or where a score is missing
export function extraText(r: Row, pick: Row | null, planReport: Plan | null): string {
  if (!planReport || !pick || r === pick || keyOf(r) === keyOf(pick)) return '–';
  const a = scoreOf(pick),
    b = scoreOf(r);
  if (a == null || b == null) return '–';
  const extra = a - b;
  return Math.abs(extra) < TIE_CAREERS ? '≈0' : netText(extra);
}
