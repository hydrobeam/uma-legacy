// The career solver's replies, read: what a career job's answer holds (its search, its plan, a kept plan's roll or
// check), each narrowed from the reply's union (career/job.ts `CareerReply`). Types and pure functions only: it imports
// nothing at run time, so the run machine and its node tests read replies without the page's data.
import type * as Gen from '../gen/solver.ts';
import type { DeepReadonly } from '../store.ts';
import type { CareerReply, CareerSearch, PlanReport } from './job.ts';

type ReplyView = DeepReadonly<CareerReply>;
/** A reply's search: undefined for a follow-up (a roll or check answered from the kept plan) */
export function searchIn(r: CareerReply): CareerSearch | undefined;
export function searchIn(r: ReplyView): DeepReadonly<CareerSearch> | undefined;
export function searchIn(r: ReplyView) {
  return 'results' in r ? r : undefined;
}
/** A reply's plan (`next`), or why the plan job made none (a refusal); undefined where it carries no plan */
export function planIn(r: CareerReply): PlanReport | Gen.Refusal | undefined;
export function planIn(r: ReplyView): DeepReadonly<PlanReport | Gen.Refusal> | undefined;
export function planIn(r: ReplyView) {
  return 'results' in r ? r.next : undefined;
}
/** A plan job's roll (`next.roll`): the kept plan's answer, or the plan's run again; undefined where it has none */
export const rollIn = (r: ReplyView) => (r.next && 'roll' in r.next ? r.next.roll : undefined);
/** A plan job's check (`next.robust`), likewise */
export function checkIn(r: CareerReply): Gen.CheckReply | undefined;
export function checkIn(r: ReplyView): DeepReadonly<Gen.CheckReply> | undefined;
export function checkIn(r: ReplyView) {
  return r.next && 'robust' in r.next ? r.next.robust : undefined;
}
