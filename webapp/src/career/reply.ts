// The career solver's replies, read: what a career job's answer holds (its search, its plan, a kept plan's roll or
// check), each narrowed from the reply's union by its `kind` (career/job.ts `CareerReply`). Types and pure functions
// only: it imports nothing at run time, so the run machine and its node tests read replies without the page's data.
import type * as Gen from '../gen/solver.ts';
import type { DeepReadonly } from '../state/store.ts';
import type { CareerReply, CareerSearch, PlanRefusal, PlanReport } from './job.ts';

type ReplyView = DeepReadonly<CareerReply>;
/** A reply's search: undefined for a follow-up (a roll or check answered from the kept plan) */
export function searchIn(r: CareerReply): CareerSearch | undefined;
export function searchIn(r: ReplyView): DeepReadonly<CareerSearch> | undefined;
export function searchIn(r: ReplyView) {
  return r.kind === 'search' ? r : undefined;
}
/** A reply's plan (`next`), or why the plan job made none (a refusal); undefined where it carries no plan */
export function planIn(r: CareerReply): PlanReport | PlanRefusal | undefined;
export function planIn(r: ReplyView): DeepReadonly<PlanReport | PlanRefusal> | undefined;
export function planIn(r: ReplyView) {
  return r.kind === 'search' ? r.next : undefined;
}
/** A plan job's roll: the kept plan's answer (`kind` "roll"), or the plan's run again (its `next.roll`); undefined
 *  where it has none */
export const rollIn = (r: ReplyView) =>
  r.kind === 'roll' ? r.roll : r.kind === 'search' && r.next?.kind === 'plan' ? r.next.roll : undefined;
/** A plan job's check, likewise (`kind` "check", or the plan's `next.robust`) */
export function checkIn(r: CareerReply): Gen.CheckReply | undefined;
export function checkIn(r: ReplyView): DeepReadonly<Gen.CheckReply> | undefined;
export function checkIn(r: ReplyView) {
  return r.kind === 'check' ? r.robust : r.kind === 'search' && r.next?.kind === 'plan' ? r.next.robust : undefined;
}
