// What every store shares: the read-only view of a record, and the one way an entry reacts to a store (`on`).
import { effect, untracked } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';

/** a record as a store hands it out: read-only all the way down (a write from outside is a compile error); an array
 *  or tuple becomes a readonly one (the mapped type keeps its shape), and a function in it stays callable */
export type DeepReadonly<T> = T extends (...args: never[]) => unknown
  ? T
  : T extends object
    ? { readonly [K in keyof T]: DeepReadonly<T[K]> }
    : T;
/** `value` frozen all the way down, as DeepReadonly says it is: a write past the type (a cast, `Object.assign`, which
 * TS lets through on a readonly target) then throws, modules being strict, instead of changing a store unseen */
export function deepFreeze<T>(value: T): DeepReadonly<T> {
  if (value !== null && typeof value === 'object') {
    Object.freeze(value);
    for (const inner of Object.values(value)) deepFreeze(inner);
  }
  return value as DeepReadonly<T>;
}
type Values<S extends readonly ReadonlySignal<unknown>[]> = {
  [K in keyof S]: S[K] extends ReadonlySignal<infer T> ? T : never;
};

// An effect on the signals listed, and on nothing else: `run` gets their values each time one of them changes (not
// when the effect is made: an entry draws the first state itself) and runs untracked, so what it reads, the store
// included, subscribes it to nothing. Effects run inside the write that triggers them; an error in `run` is thrown
// from that write (after the store has persisted it). Returns the effect's dispose.
export function on<const S extends readonly ReadonlySignal<unknown>[]>(
  sources: S,
  run: (...values: Values<S>) => void,
) {
  let made = false;
  return effect(() => {
    const values = sources.map((source) => source.value) as Values<S>;
    if (made) untracked(() => run(...values));
    made = true;
  });
}
