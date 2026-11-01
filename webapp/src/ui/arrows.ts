// ---------- arrow keys over a list (a menu, a tab list, a pill switch): the entry a key moves to ----------
// A key's target is an index worked out from the current one (one on, one back, a row down); these take it into the
// list, round its ends or held at them, so a view never indexes with arithmetic itself.
import { must } from '../must.ts';

/** entry `to` of `list` taken round its ends (one past the last is the first, one before the first the last); `list`
 *  is not empty: it is what the key moves over */
export const around = <T>(list: readonly T[], to: number): T =>
  must(list[((to % list.length) + list.length) % list.length], 'a place taken round a list that is not empty is in it');
/** entry `to` of `list` held at its ends; `list` is not empty */
export const clamped = <T>(list: readonly T[], to: number): T =>
  must(list[Math.max(0, Math.min(list.length - 1, to))], 'a place held to a list that is not empty is in it');
