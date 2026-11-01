// ---------- class lists: a component's look as one base list and a list per variant ----------
// Each list is whole and static: Tailwind finds a class only where it is written out, and Biome's Tailwind rules check
// the strings passed to `cx` and, in `variants`, `base` and every list under `variants` (biome.json `tailwind`: cva's
// shape; `defaults` holds names, not classes). Two lists that would set one property must never both apply: utilities
// don't override by their order in the attribute, only by Tailwind's own order in the sheet.

/** the classes given, skipping the empty ones */
export const cx = (...classes: (string | false | null | undefined)[]): string => classes.filter(Boolean).join(' ');

type Lists<V> = { [K in keyof V]: Record<string, string> };
type Picks<V extends Lists<V>> = { [K in keyof V]?: keyof V[K] | undefined };

/** a look: `base` always, and per variant the list its pick names (else its default's); `extra` places it */
export function variants<V extends Lists<V>>({
  base,
  variants: lists,
  defaults,
}: {
  base: string;
  variants: V;
  defaults: { [K in keyof V]: keyof V[K] };
}) {
  return (picks: Picks<V> = {}, extra?: string | undefined): string => {
    const listOf = <K extends keyof V>(k: K): V[K][keyof V[K]] => lists[k][picks[k] ?? defaults[k]];
    const chosen: string[] = [];
    for (const k in lists) chosen.push(listOf(k));
    return cx(base, ...chosen, extra);
  };
}
