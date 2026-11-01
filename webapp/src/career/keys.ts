// The keys a career job, a plan job and a career are known by (the answers kept, the plan Compare kept, a career's
// identity, a comparison's baseline, the last plan's base). It reads no store and no game data.
import { ERA } from '../saved.ts';
import type { DeepReadonly } from '../store.ts';
import type { CareerJob } from './job.ts';

/** what the keys read of a job: they never write it */
type Job = DeepReadonly<CareerJob>;

// a job's key: its JSON with the legacies' apart, kept with the array `legaciesJob` returns (the same until an edit)
let legaciesJson: { legacies: Job['career']['legacies'] | null; json: string } = { legacies: null, json: '' };
export const jobKey = (job: Job): string => {
  const legacies = job.career.legacies;
  if (legaciesJson.legacies !== legacies) legaciesJson = { legacies, json: JSON.stringify(legacies) };
  return `${JSON.stringify([ERA, { ...job, career: { ...job.career, legacies: 0 } }])}\n${legaciesJson.json}`;
};
// a career's identity: the solver's `career` (her uma, her parents in order and her deck: two decks on the same parents
// are two careers), else (an answer from before it) her uma and parents
export const keyOf = (r: {
  career?: string | undefined;
  trainee: { id: number };
  parents: readonly number[];
}): string => r.career ?? `${r.trainee.id}:${r.parents.join(',')}`;
// x with every object's keys sorted and its undefined fields left out (JSON that two equal jobs share)
const canon = (x: unknown): unknown =>
  Array.isArray(x)
    ? x.map(canon)
    : x && typeof x === 'object'
      ? Object.fromEntries(
          Object.keys(x)
            .sort()
            .filter((k) => (x as Record<string, unknown>)[k] !== undefined)
            .map((k) => [k, canon((x as Record<string, unknown>)[k])]),
        )
      : x;
// a plan job's key (Compare's kept plan, the reference's base): its canonical JSON, the comparison's flag aside (run.ts
// `PLAN_NEXT`)
export const planKey = (job: Job): string => {
  const { compare: _c, ...career } = job.career;
  return JSON.stringify(canon({ ...job, career }));
};
// the job of today's best career over everything (no place set, one result): the comparison's baseline. `notOwned`:
// the umas not owned (job.ts `umasNotOwned`; the job's own `exclude` leaves out the trainee)
const baseOf = (job: Job, notOwned: readonly number[]): Job => {
  const { given: _given, ...search } = job.career;
  return {
    ...job,
    skills: { ...job.skills, exclude: notOwned },
    career: { ...search, trainees: [], fix: [], per_trainee: false, top: 1 },
  };
};
/** the key of `job`'s baseline (`baseOf`) */
export const baseKey = (job: Job, notOwned: readonly number[]): string => JSON.stringify(baseOf(job, notOwned));
// the last plan made (career/state.ts `reference`): while the career on show is one you chose on the same setup and
// veterans (its job without the places), its table stays, so you can weigh the plan's other careers or go back to its pick
export const refBase = (job: Job, notOwned: readonly number[]): string => {
  const { given: _given, ...search } = job.career;
  return planKey({
    ...job,
    skills: { ...job.skills, exclude: notOwned },
    career: { ...search, trainees: [], fix: [], top: 0 },
  });
};
