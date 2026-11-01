// ---------- the comparison's run machine: Compare and Reroll, and how they follow the career runs ----------
// `createComparison(deps)` returns what the page and the career run machine (career/run.ts, its `analysis` deps) call:
// the runs' hooks (outdate, stop, show, clear, keepBaseline) and the popup's actions (compare, roll). A comparison is
// one of two async chains, `quickCompare` (a roll's) and `runComparison` (the exact one). Each takes a token on entry
// (`comparisonId`, closure state with the baseline cache: nothing draws them) and checks it after every await and in
// its 0.8 s timers; every hook that makes it stale bumps the token synchronously (a new run: `outdate`; an edit:
// `stop`; a newer comparison). What the popup draws is the analysis store's (career/analysis/state.ts), written only
// through its transitions, one per phase. Everything else of the page (the solver, the clock, the answer store, the
// career on show, the roll) comes in `deps`, so the node tests run the machine on fakes. It reads no store itself (a
// model module: .dependency-cruiser.mjs checks it).
import { batch } from '@preact/signals';
import type { DeepReadonly } from '../../store.ts';
import type * as AnswerStore from '../state.ts';
import type { PlanDone } from '../state.ts';
import type * as AnalysisStore from './state.ts';
import type { CareerAnswer, CareerJob, CareerReport, PlanReport } from '../job.ts';
import type { Draft, NodeRecord } from '../../veterans/store.ts';
import { keyOf } from '../keys.ts';
import { PLAN_NEXT } from '../run.ts';

/** what the comparison needs of the page, built once by the entry (career/main.tsx) */
export interface ComparisonDeps {
  /** the solver (career/job.ts `runJob`, engine.ts) */
  engine: {
    run: (job: CareerJob) => Promise<CareerAnswer>;
    /** kill the worker (a job in flight then never answers) and start a fresh one */
    restart: () => void;
    /** keep the worker's work for the next page load */
    saveState: () => void;
  };
  clock: { setTimer: (run: () => void, ms: number) => unknown };
  /** the answer store: the career on show's search and plan, and where Compare keeps a refit plan */
  answers: Pick<typeof AnswerStore, 'answer' | 'keepComparisonPlan'>;
  /** the analysis store's record and transitions */
  store: typeof AnalysisStore;
  /** the page's state as a comparison reads it */
  page: {
    /** the career on show (career/job.ts `shown`) */
    shown: () => DeepReadonly<CareerReport> | null;
    /** a run is at work (its answer replaces the career on show) */
    searching: () => boolean;
    /** career/job.ts `baseKey`: the job of today's best career over everything */
    baseKey: (job: DeepReadonly<CareerJob>) => string;
    /** the target skills, in the job's order */
    targets: () => readonly { group: number }[];
    /** she lands every target (career/job.ts `isFull`) */
    isFull: (nd: NodeRecord) => boolean;
    /** a run's plan job's `top` (career/job.ts `ROLL_K`) */
    planTop: number;
    /** a fresh draft of career `r`: a random result of it, on her parents' records now (a new career, a roll); null
     *  when a parent has left the pool under the answer on show */
    draftOf: (r: DeepReadonly<CareerReport>) => Draft | null;
    /** her draft on show (veterans/store.ts `draft`; null: none) */
    draft: () => DeepReadonly<Draft> | null;
  };
}

/** how long a roll's quick figures stand before the exact comparison checks them, ms */
export const STAND_MS = 800;

// the draft as a legacy of the comparison's jobs (a veteran's job record: career/job.ts `jobNode`)
function draftJob(v: DeepReadonly<Draft>) {
  const pairs = (o: DeepReadonly<Record<number, number>>) =>
    Object.entries(o).map(([g, s]): [number, number] => [+g, s]);
  const node = (x: NodeRecord) => ({
    chara: x.chara || 0,
    sparks: pairs(x.sparks),
    pinks: pairs(x.pinks),
    wins: [...x.wins],
  });
  return { ...node(v), borrowed: false, parents: v.parents.map(node) };
}

// where a comparison's saving lands: the next career when she is one of its parents; else a later career, named (the
// step of V's route, A, B or C, whose careers fell most) only where both plans' cheapest routes are the same trio, on
// the same sides (`sides`: her real veteran, or her best career now or another career of hers, each on its parents: a
// side whose parents changed is another route). The cheapest trios are often near-tied (`trios`, `near`), so between
// two different ones a per-uma step comparison names an arbitrary uma; what falls then is V, which the panel shows.
// planNow / planWithHer: the plans' `next` reports; pickA: the planner's next career with her; samePick: it is the one
// without her
export function savedWhere({
  planNow,
  planWithHer,
  pickA,
  samePick,
  herIndex,
}: {
  planNow: DeepReadonly<PlanReport> | null;
  planWithHer: DeepReadonly<PlanReport> | null;
  pickA: DeepReadonly<CareerReport> | null;
  samePick: boolean;
  herIndex: number;
}) {
  if (!pickA) return '';
  if (pickA.parents.includes(herIndex)) return `Improves the next career (${pickA.trainee.name}).`;
  const trioNow = planNow?.v0_plan,
    trioWithHer = planWithHer?.v0_plan;
  // (A and B are ordered by their careers, so the same trio may list them swapped; each side by its uma and its veteran,
  // `sides`: the same names on other veterans, or on other parents, are another route, whose per-step careers are not
  // comparable)
  type Trio = NonNullable<typeof trioNow>;
  const sidesOf = (v: Trio) =>
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
  const steps = (v: Trio) => {
    const [a, b, c] = v.careers;
    return [[v.a, a] as const, [v.b, b] as const, [v.c, c] as const];
  };
  let best: string | null = null,
    gain = 0;
  if (sameTrio && trioNow && trioWithHer) {
    const careersNow = new Map(steps(trioNow));
    for (const [trainee, careers] of steps(trioWithHer)) {
      const before = careersNow.get(trainee);
      // (a step not in today's route, or one that cannot land, its careers null: nothing fell)
      if (!trainee || before == null || careers == null) continue;
      const fell = before - careers;
      if (fell > gain) {
        gain = fell;
        best = trainee;
      }
    }
  }
  return `Improves a later ${best ? `${best} ` : ''}career; the next career ${samePick ? 'stays' : 'is'} ${pickA.trainee.name}.`;
}

function comparisonOf({ engine, clock, answers, store, page }: ComparisonDeps) {
  // the comparison under way; a newer one makes it stale, and so does a run: the run machine calls `stop` or `outdate`
  // whenever a new run begins (career/run.ts, its node tests), so this one id covers both
  let comparisonId = 0;
  // key: baseKey(job); result: today's best career, the comparisons' baseline
  let baselineCache: { key: string; result: DeepReadonly<CareerReport> } | null = null;
  const now = () => store.simulation.peek();
  const lastShown = () => {
    const { last } = answers.answer.peek();
    if (!last) throw new Error('a comparison is of the career on show');
    return last;
  };
  const comparisonJob = (): CareerJob => JSON.parse(page.baseKey(lastShown().job));
  const draftNow = () => {
    const draft = page.draft();
    if (!draft) throw new Error('a comparison is of the entered veteran');
    return draft;
  };

  // today's best career without her, which a roll's figures and a comparison's both set hers against: the run's when it
  // searched without places (a fixed trainee or parent makes it the best of those), else cached or searched now
  async function baselineFor(base: CareerJob) {
    const shownNow = lastShown();
    const key = JSON.stringify(base),
      open = !shownNow.job.trainees.length && !shownNow.job.fix.length,
      cached = baselineCache?.key === key ? baselineCache.result : null;
    const before = cached ? { results: [cached] } : open ? shownNow.r : (await engine.run(base)).r;
    if ('error' in before) throw new Error(before.error);
    const [best] = before.results ?? [];
    if (!best) throw new Error('No career found in the current roster.');
    baselineCache = { key, result: best };
    return best;
  }

  // A roll's careers saved, asked of the plan for the career on show (`roll`: the plan job and her stars per target;
  // the solver evaluates the plan's pick at that one outcome from what it kept of the plan, career/plan/roll.rs
  // `roll_kept`, so no plan runs again unless the worker kept none): y, exact for her sparks, her G1 wins at their
  // chances, over the plan's routes. It is no verdict: a route the plan has not (another lineage of a candidate's whose
  // last career she improves as a grandparent, without lifting the candidate's odds) can save careers too, so every
  // roll is confirmed by the exact comparison, its quick figure shown meanwhile as provisional. null when the plan
  // isn't for this career (then Reroll compares exactly).
  function rollReq(d: DeepReadonly<Draft>, r: DeepReadonly<CareerReport>) {
    const { planState } = answers.answer.peek();
    if (planState.state !== 'done') return null;
    const o = planState.r?.outcomes;
    if (!o || !planState.job || (o.career ?? `${o.trainee}:${o.parents.join(',')}`) !== keyOf(r)) return null;
    const sp = d.sparks || {};
    return { job: planState.job, stars: page.targets().map((t) => sp[t.group] || 0) };
  }

  // the reroll's comparison, in two solver calls: the roll's saving from the plan the worker keeps (`next.roll`, about
  // a millisecond), shown at once as Careers to loop; then the best career with her as a parent (one quick search, most
  // of a second), which fills in her odds and the verdict's "Better odds". A reroll meanwhile waits behind that search
  // (the worker keeps it and its caches; its answer is ignored)
  async function quickCompare(request: NonNullable<ReturnType<typeof rollReq>>) {
    const id = ++comparisonId,
      gone = () => id !== comparisonId;
    store.quickStarts();
    /** the roll's saving (careers), from the plan's routes (`rollReq`) */
    let y: number | null = null;
    // her legacy index is after every veteran's
    const base = comparisonJob(),
      legaciesWithHer = [...base.legacies, draftJob(draftNow())],
      herIndex = base.legacies.length;
    try {
      if (!now().baseline) {
        const before = await baselineFor(base);
        if (gone()) return;
        store.baselineFound(before);
      }
      const m = await engine.run({ ...request.job, next: { ...request.job.next, roll: request.stars } } as CareerJob);
      if (gone()) return;
      // (no saving from the plan: the exact comparison answers instead)
      const roll = 'error' in m.r ? undefined : m.r.next?.roll;
      if (typeof roll?.y !== 'number') {
        batch(() => {
          store.quickHandsOver();
          runComparison();
        });
        return;
      }
      y = roll.y;
      const v0 = (answers.answer.peek().planState as DeepReadonly<PlanDone>).r.v0; // done: `request` came from its `rollReq`
      // a roll that saves careers goes to the exact comparison (which finds the best career with her too) once it has
      // stood 0.8 s, as a zero does (rolling on starts none, so no running check is stopped, the worker with it): the
      // plan's saving can be off either way (it prices her in its own routes only: a few partners, one deck per lifted
      // side, one C per route), so the exact count replaces it, shown as checking meanwhile; a zero is checked too,
      // after the best career with her
      store.quickFigures(
        { full: page.isFull(draftNow()), v0: { now: v0, withHer: v0 != null ? Math.max(0, v0 - y) : null } },
        y >= 0.5,
      );
    } catch (e) {
      if (gone()) return;
      store.comparisonFails((e as Error).message, true);
    }
    if (now().state !== 'done' || gone()) return;
    if (now().standing) {
      clock.setTimer(() => {
        if (id === comparisonId && now().standing && !now().refining) runComparison(true);
      }, STAND_MS);
      return;
    }
    try {
      const m = await engine.run({ ...base, fix: [herIndex], legacies: legaciesWithHer, top: 1, compare: true });
      if (gone()) return;
      if ('error' in m.r) throw new Error(m.r.error);
      if (!m.r.results?.length) throw new Error('No next career found.');
      // a zero is checked by the exact comparison too (the plan's routes are not every route a replan can take), shown
      // meanwhile as provisional (after the roll has stood a moment: rolling on starts none; a running check would be
      // stopped, the worker with it)
      store.bestFound(m.r);
    } catch (e) {
      if (gone()) return;
      store.comparisonFails((e as Error).message);
    }
    if (now().state === 'done' && !gone())
      clock.setTimer(() => {
        if (id === comparisonId && now().provisional && !now().refining) runComparison(true);
      }, STAND_MS);
  }

  // A comparison in flight is for a veteran that is going (a roll, a reset, Compare pressed again): stop it. A wasm
  // call can't be interrupted, so the worker goes (a fresh one starts loading at once). (No run is in flight: they wait
  // for the answer.)
  function cancelCompare() {
    const { refining, state, kind } = now();
    if (refining) {
      comparisonId++;
      store.checkEnds();
      engine.restart();
      return;
    }
    if (state !== 'busy') return;
    comparisonId++;
    store.comparisonCancelled();
    if (kind === 'quick') return; // one quick search: it finishes, its answer ignored (the worker and its caches stay)
    engine.restart();
  }

  // refine: a roll's quick result stays on screen (an improvement shown at once, its figures held) while this works out
  // the exact count; otherwise the panel waits on it
  async function runComparison(refine = false) {
    const id = ++comparisonId,
      gone = () => id !== comparisonId;
    const base = comparisonJob();
    store.exactStarts(refine);
    try {
      if (!now().baseline) {
        const before = await baselineFor(base);
        if (gone()) return;
        store.baselineFound(before);
      }
      // the planner's question (exact odds, estimated careers): the best career with her as a parent, and the planner's
      // expected careers to a fully looped parent (each plan's `v0`, V) from the veterans now, without and with her
      const legaciesWithHer = [...base.legacies, draftJob(draftNow())],
        herIndex = base.legacies.length;
      const m = await engine.run({ ...base, fix: [herIndex], legacies: legaciesWithHer, top: 1, compare: true });
      if (gone()) return;
      if ('error' in m.r) throw new Error(m.r.error);
      if (!m.r.results?.length) throw new Error('No next career found.');
      const full = page.isFull(draftNow());
      const planOf = (legacies: CareerJob['legacies']): CareerJob => ({
        ...base,
        top: page.planTop,
        legacies,
        next: { ...PLAN_NEXT },
        compare: true,
        no_reports: true,
      }); // as a run's plan job, once she is added
      const { planState } = answers.answer.peek();
      let planNow: DeepReadonly<PlanReport> | null =
        planState.state === 'done' && !lastShown().job.fix.length ? planState.r : null;
      if (!planNow) {
        const mb = await engine.run(planOf(base.legacies));
        if (gone()) return;
        planNow = ('error' in mb.r ? null : mb.r.next) || null;
      }
      // with her: her pool priced under today's plan's fit (`next.fit`, passed on as the plan wrote it), so the two Vs
      // differ only by what she changes, not by Cont fitted again to the pool with her (a refit moves V by hundreds of
      // careers on a weak pool, in either direction). The plan Add to veterans shows is that refit, run on Add. Only
      // when today's plan has no V to compare with (no route: `v0` null, though its `fit` may still list candidates) or
      // no fit is the refit run here, and kept so that Add takes it as it is
      const fit = planNow && Number.isFinite(planNow.v0) && planNow.fit?.length ? planNow.fit : null,
        jobWithHer = fit ? { ...planOf(legaciesWithHer), next: { fit } } : planOf(legaciesWithHer),
        ma = await engine.run(jobWithHer as CareerJob);
      if (gone()) return;
      const planWithHer = ('error' in ma.r ? null : ma.r.next) || null;
      if (!fit && planWithHer && !planWithHer.error) answers.keepComparisonPlan(jobWithHer, planWithHer, ma.ms);
      engine.saveState();
      // careers to a fully looped parent with the pool as it is
      const careersToLoop = (next: DeepReadonly<PlanReport> | null) => (next && next.v0 != null ? next.v0 : null);
      const careersNow = careersToLoop(planNow),
        pickA = planWithHer?.options?.find((o) => o.chosen) || null,
        pickB = planNow?.options?.find((o) => o.chosen) || null;
      // not clamped: in the game a veteran never costs careers (you can leave her out), but V can rise with her even
      // under the same fit: each fresh side of V's route comes from one of a candidate's `fresh_lineages` best careers
      // now (her best and her next two by cached decks, each searched on its pair), so her arrival can push the lineage
      // the route used out of them, and a pair's search that runs out of budget can stop below the pair's best. The
      // panel shows such a rise as it is, with that said (view.tsx `ComparisonResult`)
      const careersWithHer = careersToLoop(planWithHer);
      // the planner's next career keeps its trainee and parents (her legacy index is after them, so they keep theirs)
      const samePick = !!(pickA && pickB && keyOf(pickA) === keyOf(pickB));
      store.exactDone({
        r: m.r,
        full,
        v0: { now: careersNow, withHer: careersWithHer },
        fixedFit: !!fit,
        pickA,
        samePick,
        herParent: !!pickA?.parents.includes(herIndex),
        where: savedWhere({ planNow, planWithHer, pickA, samePick, herIndex }),
      });
    } catch (e) {
      if (gone()) return;
      if (now().refining) store.checkEnds();
      else store.comparisonFails((e as Error).message);
    }
  }

  // Compare (or a roll): a press while a comparison runs restarts it with the veteran as she is now
  function compare() {
    if (page.searching() || !page.shown()) return;
    batch(() => {
      cancelCompare();
      runComparison();
    });
  }
  // Reroll: a new random result of the career on show, and its quick comparison (the exact one when the plan isn't for
  // this career)
  function roll() {
    const r = page.shown();
    if (!r || page.searching()) return;
    const next = page.draftOf(r);
    if (!next) return; // (a parent has left the pool under the answer on show: it goes once the pool's window closes)
    batch(() => {
      store.draftRolled(next);
      const request = rollReq(draftNow(), r);
      if (request == null) {
        compare();
        return;
      }
      cancelCompare();
      quickCompare(request);
    });
  }

  return {
    // ---------- the run machine's hooks (career/run.ts RunDeps `analysis`)
    /** a new run: the comparison under way, if any, is stale (its answer is ignored) */
    outdate() {
      comparisonId++;
    },
    /** an edit: the comparison in flight stops, its panel back to idle, or a roll's quick figures stand unchecked (an
     *  answer that stands keeps her) */
    stop() {
      comparisonId++;
      store.comparisonStops();
    },
    /** an answer shows `r`: her analysis, ready (a fresh draft, or the one entered for the same career) */
    show(r: DeepReadonly<CareerReport> | null) {
      // (a parent who has left the pool under the answer on show: no draft, so nothing to analyse)
      const next = r && page.draftOf(r);
      store.analysisShows(r && next ? { draft: next, forKey: keyOf(r) } : null);
    },
    /** no answer: no career to analyse */
    clear: store.analysisClears,
    /** a run searched today's best career (no place set): the comparisons' baseline (`baselineFor`), under `key`
     *  (`baseKey`) */
    keepBaseline(key: string, result: DeepReadonly<CareerReport>) {
      baselineCache = { key, result };
    },
    // ---------- the popup's actions
    compare,
    roll,
  };
}
/** A comparison: what `createComparison` makes, and nothing else can be: its machine is a private field, which no object
 *  literal or spread of one has, so the run machine's `analysis` (career/run.ts RunDeps) is this machine and never a
 *  stand-in whose `stop` or `outdate` does nothing. */
export class Comparison {
  readonly #machine: ReturnType<typeof comparisonOf>;
  constructor(deps: ComparisonDeps) {
    this.#machine = comparisonOf(deps);
  }
  // (fields, not methods: each is handed out alone, as a click's handler)
  readonly outdate = () => this.#machine.outdate();
  readonly stop = () => this.#machine.stop();
  readonly show = (r: DeepReadonly<CareerReport> | null) => this.#machine.show(r);
  readonly clear = () => this.#machine.clear();
  readonly keepBaseline = (key: string, result: DeepReadonly<CareerReport>) => this.#machine.keepBaseline(key, result);
  readonly compare = () => this.#machine.compare();
  readonly roll = () => this.#machine.roll();
}
export const createComparison = (deps: ComparisonDeps) => new Comparison(deps);
