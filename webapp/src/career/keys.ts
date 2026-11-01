// The keys a career job, a plan job and a career are known by (the answers kept, the plan Compare kept, a career's
// identity). It reads no store.
import { ERA } from '../saved.ts';
import type { DeepReadonly } from '../store.ts';
import type { CareerJob } from './job.js';

/** what the keys read of a job: they never write it */
type Job = DeepReadonly<CareerJob>;

// a job's key: its JSON with the legacies' apart, kept with the array `legaciesJob` returns (the same until an edit)
let legaciesJson: { legacies: Job['legacies'] | null; json: string } = { legacies: null, json: '' };
export const jobKey = (job: Job): string => {
  if (legaciesJson.legacies !== job.legacies)
    legaciesJson = { legacies: job.legacies, json: JSON.stringify(job.legacies) };
  return `${JSON.stringify([ERA, { ...job, legacies: 0 }])}\n${legaciesJson.json}`;
};
// a career's identity: the solver's `career` (her uma, her parents in order and her deck: two decks on the same parents
// are two careers), else (an answer from before it) her uma and parents
export const keyOf = (r: { career?: string; trainee: { id: number }; parents: readonly number[] }): string =>
  r.career ?? `${r.trainee.id}:${r.parents.join(',')}`;
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
// a plan job's key (Compare's kept plan, the reference's base): its canonical JSON, the comparison's flag and the
// rollout planner's leftovers aside (run.ts `PLAN_NEXT`)
export const planKey = (job: Job): string => {
  const { compare: _c, rollout: _r, p_now: _p, ...j } = job;
  return JSON.stringify(canon(j));
};
