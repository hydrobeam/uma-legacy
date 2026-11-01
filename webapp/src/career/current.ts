// The career job and the career on show as the stores stand: job.ts's `buildJob` over the settings and the veterans
// store now (`currentJob`), its run on the solver (`runJob`: every career it names gets one outfit), the career on show
// and the veteran in each parent place, and the Use this choice a reload no longer fits (`dropStaleChosen`). What a
// component calls here reads the stores with .value (it draws again when they change); what the run machine calls,
// with .peek().
import { computed } from '@preact/signals';
import { careerTables } from '../data.ts';
import { settings } from '../settings.ts';
import { run as engineRun } from '../engine.ts';
import { goldGiven } from '../job.ts';
import { srcFor } from '../images.ts';
import { vets, places, chosen, setChosen, vetById, addVets, legaciesJob } from '../veterans/store.ts';
import { targetGroups } from '../veterans/names.ts';
import { answer, answerCareer, answerPool, choiceDropped, reference } from './state.ts';
import { must, orNull } from '../must.ts';
import { pickOutfit, outfitNote } from '../roster.ts';
import { unprovenWhy } from '../proofs.ts';
import { refBase } from './keys.ts';
import { openPool } from './view/poolState.ts';
import { Blocked, buildJob, isFull, placesKey, umasNotOwned } from './job.ts';
import type { CareerAnswer, CareerJob, CareerReport, CareerTrainee, OptionRow, PoolActs } from './job.ts';
import type { PlanProgress } from '../solver.ts';
import type { LineageNode, Sparks, Veteran, VetRecord } from '../veterans/store.ts';
import type { DeepReadonly } from '../store.ts';

// ---------- the job
/** her place in the pool (-1: none) */
export const idxOf = (id: string | null): number => vets.value.findIndex((v) => v.id === id);
/** the veteran at legacy index `i` of the answer on show (a career's parent, a plan's side): the one whose id held that
 *  place when the answer's job was built (career/state.ts `answerPool`), wherever she is now; undefined when she has
 *  left the pool since (the page holds an answer while a pool veteran's G1 wins window is open, and drops it when the
 *  window closes on a changed pool: career/main.tsx) */
export const legacyVet = (i: number): VetRecord | undefined => legacyIn(answerPool.value, vets.value, i);
/** the same, read without subscribing (the run machine's) */
export const legacyVetNow = (i: number): VetRecord | undefined => legacyIn(answerPool.peek(), vets.peek(), i);
const legacyIn = (pool: readonly string[], list: readonly VetRecord[], i: number) => {
  const id = must(pool[i], "a career's legacy index is a place in its job's pool");
  return list.find((v) => v.id === id);
};
/** a veteran's number on the page: her place in the pool now ("#3"), as the pool list, the tree and her popup give it */
export const poolNumber = (v: VetRecord): string => `#${idxOf(v.id) + 1}`;
/** the number of the veteran at legacy index `i` of the answer on show: her `poolNumber`, which differs from `i + 1`
 *  once the pool has changed under a held answer; one who has left the pool since keeps the answer's, `#${i + 1}` */
export const legacyNumber = (i: number): string => {
  const v = legacyVet(i);
  return v ? poolNumber(v) : `#${i + 1}`;
};
/** the legacy index of veteran `id` in the answer on show (-1: none: she joined the pool since) */
const legacyOf = (id: string): number => answerPool.value.indexOf(id);
// the status line's buttons when the veterans make no job: the veterans dialog, the sample veterans
const POOL_ACTS: PoolActs = { importVets: openPool, trySamples: () => addVets(sampleVets()) };
/** the career job of the settings, veterans and places now (throws job.ts `Blocked` when they make none) */
export const currentJob = (): CareerJob =>
  buildJob(
    {
      settings: settings.peek(),
      vets: vets.peek(),
      legacies: legaciesJob(),
      places: places.peek(),
      chosen: chosen.peek(),
    },
    POOL_ACTS,
  );
/** A Use this choice saved before the reload that no longer applies is dropped, and the search's status says so
 *  (career/state.ts `dropped`) */
export function dropStaleChosen(): void {
  const c = chosen.peek();
  if (!c) return;
  // (settings that make no job: the chosen career no longer applies; a bug building it is reported, and the start goes on)
  if (orNull(currentJob, Blocked)?.career === c.id) return;
  choiceDropped({
    places: c.places,
    text: 'The deck you chose with Use this before the reload no longer fits (its trainee, parents, their records, the targets or the card settings changed), so this is the deck the search finds.',
  });
  setChosen(null, true);
}
// a career on two full veteran parents: her own full veteran there is a fully looped parent (the plan says so; a
// search's row is read off the veterans)
export const looped = (r: DeepReadonly<OptionRow>): boolean =>
  r.looped ?? r.parents.every((i) => isFull(legacyVet(i), settings.value.targets));
/** the last plan made, while it stands for the career on show (keys.ts `refBase`) */
export const refFor = () => {
  const ref = reference.value,
    { last, planState } = answer.value;
  return ref && last && planState.state !== 'done' && ref.base === refBase(last.job, umasNotOwned(settings.value))
    ? ref
    : null;
};
// ---------- the pedigree ----------
// the career on show: the answer's (career/state.ts `answerCareer`: the plan's pick once it is made, else the highest
// odds) while it belongs to the places on screen
const shownCareer = computed(() => (answer.value.last?.plan === placesKey(places.value) ? answerCareer.value : null));
export const shown = () => shownCareer.value;
// which veteran sits in parent place k: the chosen one, else the planner's pick in the result on show (null: none, or
// the pick has left the pool under the answer on show, `legacyVet`)
export function parentAt(
  k: number,
  r: DeepReadonly<CareerReport> | null | undefined,
): { v: VetRecord; pick: boolean } | null {
  const plan = places.value,
    mine = plan.parents[k] && vetById(plan.parents[k]);
  if (mine) return { v: mine, pick: false };
  if (!r) return null;
  // the other place's chosen veteran, by legacy index (-1: none): the result's parent that isn't her fills this one
  const otherId = plan.parents[1 - k],
    other = otherId ? legacyOf(otherId) : -1,
    [first, second] = r.parents;
  const v = legacyVet(other >= 0 ? (first !== other ? first : second) : k === 0 ? first : second);
  return v ? { v, pick: true } : null;
}
// sample veterans to try the planner with (no veterans yet): 18 mob umas drawn at random from the game's (those with a
// portrait), six veterans with two mob parents each, the bare minimum to start a loop: each veteran carries, at 1 star,
// only the targets nothing but inheritance gives (no card hints or events them, no outfit has them, the scenario
// doesn't hand them out); her parents carry nothing. No relation points with anyone (the solver knows no mob), no G1
// wins. Marked as samples; Remove all clears them.
function sampleVets(): Veteran[] {
  const all = careerTables().mobs || [],
    withArt = all.filter(([id]) => srcFor('mob', id)),
    pool = shuffled(withArt.length >= 18 ? withArt : all);
  const sourced = new Set(careerTables().sourced || []),
    inherit = targetGroups(settings.peek().targets).filter(
      (g) => !sourced.has(g) && !goldGiven(settings.peek().scen, g),
    );
  const node = (id: number, sparks: Sparks): LineageNode => ({
    chara: id,
    sparks,
    pinks: {},
    blues: {},
    greens: {},
    wins: [],
  });
  const out: Veteran[] = [];
  for (let i = 0; out.length < 6; i += 3) {
    const [her, first, second] = pool.slice(i, i + 3);
    if (!her || !first || !second) break; // (fewer than three mobs left)
    out.push({
      id: `m${her[0]}${Date.now().toString(36)}`,
      src: 'sample',
      borrowed: false,
      ...node(her[0], Object.fromEntries(inherit.map((g) => [g, 1]))),
      parents: [node(first[0], {}), node(second[0], {})],
    });
  }
  return out;
}
// `list` in a random order: Fisher-Yates, its draws as they always were (the browser suite's seeded samples rest on them)
function shuffled<T extends {}>(list: readonly T[]): T[] {
  const out = list.slice();
  for (let i = out.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    const last = must(out[i], 'i < out.length'),
      drawn = must(out[j], '0 <= j <= i');
    out[i] = drawn;
    out[j] = last;
  }
  return out;
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
