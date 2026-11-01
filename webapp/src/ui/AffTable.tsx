// ---------- an affinity heart, and a career's affinity table legacy by legacy (it says what the table holds) ----------
import type { ComponentChildren } from 'preact';
import { must } from '../must.ts';
import { cx, variants } from './variants.ts';

/** her affinity as the game's heart and the number (none without one) */
export function AffHeart({
  affinity,
  cls = 'aff',
  title,
}: {
  affinity: number | null | undefined;
  cls?: string;
  title?: string;
}) {
  if (affinity === null || affinity === undefined) return null;
  const shown = Math.round(affinity);
  return (
    // biome-ignore lint/a11y/useAriaPropsSupportedByRole: today's markup (the old affEl): its name reads the heart as a number; a role for it is the AffinityTable port's (step 11)
    <span class={cls} aria-label={`Affinity ${shown}`} title={title ?? `Affinity ${shown}`}>
      <i class="heart" />
      {String(shown)}
    </span>
  );
}
/** one legacy's row of the table */
export interface AffinityLeg {
  /** her portrait */
  face: ComponentChildren;
  name: string;
  tag?: string | null;
  /** relation points */
  rel: number;
  /** shared G1 wins, as points */
  race: number;
  aff: number;
  /** a parent's: her two parents */
  gps?: AffinityLeg[];
}
const whole = (x: number) => String(Math.round(x));
// The cells: ruled off below, numbers right-aligned; a phone's tighter. Each row kind's lists are whole: two lists that
// set one property never meet on a cell.
type Kind = 'parent' | 'second' | 'grandparent' | 'total';
// the first cell, who: her portrait, name, tag and legacy in a row (on a phone the portrait beside the other three,
// stacked: side by side they made the table wider than a 360 px dialog)
const stack =
  'flex items-center gap-2 max-sm:grid max-sm:grid-cols-[auto_minmax(0,1fr)] max-sm:justify-items-start max-sm:gap-x-1.5 max-sm:gap-y-0 max-sm:*:first:row-span-3';
const who = variants({
  base: 'text-left',
  variants: {
    kind: {
      /** Legacy 1's parent */
      parent: cx(stack, 'border-gm-line border-b px-2 py-1 font-bold max-sm:px-1.25'),
      /** Legacy 2's parent, ruled off from Legacy 1's rows */
      second: cx(stack, 'border-gm-line border-t-2 border-b px-2 py-1 font-bold max-sm:px-1.25'),
      grandparent: cx(stack, 'border-gm-line border-b py-1 pr-2 pl-6.5 font-medium max-sm:pr-1.25 max-sm:pl-3.5'),
      total: 'px-2 py-1 font-black max-sm:px-1.25',
    },
  },
  defaults: { kind: 'parent' },
});
// the number cells
const num = variants({
  base: 'px-2 py-1 text-right max-sm:px-1.25',
  variants: {
    kind: {
      parent: 'border-gm-line border-b',
      second: 'border-gm-line border-t-2 border-b',
      grandparent: 'border-gm-line border-b',
      total: 'font-black',
    },
  },
  defaults: { kind: 'parent' },
});
// one row: who (her face, name, tag, legacy number), then the legacy's relation, shared G1s and affinity
function Row({
  kind,
  name,
  legacy,
}: {
  kind: Kind;
  name: ComponentChildren;
  legacy: Pick<AffinityLeg, 'rel' | 'race' | 'aff'>;
}) {
  return (
    <tr>
      <th scope="row" class={who({ kind })}>
        {name}
      </th>
      <td class={num({ kind })}>{whole(legacy.rel)}</td>
      <td class={num({ kind })}>{whole(legacy.race)}</td>
      <td class={num({ kind })}>
        <AffHeart affinity={legacy.aff} cls="aff justify-end" />
      </td>
    </tr>
  );
}
// legacyIndex: the parent's place (Legacy 1, 2), in its legacy's colour; a grandparent's row has none
const Who = ({ legacy, legacyIndex }: { legacy: AffinityLeg; legacyIndex?: number }) => (
  <>
    {legacy.face}
    <span>{legacy.name}</span>
    {legacy.tag && <span class="text-2xs text-gm-ink2 font-medium">{legacy.tag}</span>}
    {legacyIndex != null && (
      <span
        class={cx(
          'text-2xs ml-auto font-bold max-sm:ml-0 max-sm:whitespace-nowrap',
          legacyIndex === 0 ? 'text-leg1' : 'text-leg2',
        )}
      >
        Legacy {legacyIndex + 1}
      </span>
    )}
  </>
);
const head =
  'border-gm-line border-b bg-gm-tab-low px-2 py-1 text-right font-bold text-gm-ink2 text-xs max-sm:px-1.25 max-sm:text-3xs';
/** the parents' rows (Legacy 1 first), each with her parents', and the total */
export function AffTable({ legs, caption = 'Affinity' }: { legs: AffinityLeg[]; caption?: string }) {
  const gps = (parent: AffinityLeg) => must(parent.gps, "a parent's row carries her parents");
  const everyone = legs.flatMap((parent) => [parent, ...gps(parent)]),
    sum = (field: (x: AffinityLeg) => number) => everyone.reduce((total, x) => total + field(x), 0);
  return (
    <table class="border-gm-line bg-gm-cell text-gm-ink text-md w-full border-separate border-spacing-0 overflow-hidden rounded-lg border tabular-nums">
      {caption && <caption class="text-gm-ink px-0.5 pb-1.5 text-left font-black">{caption}</caption>}
      <thead>
        <tr>
          <th scope="col" class={head} />
          <th scope="col" class={head}>
            Relation
          </th>
          <th scope="col" class={head}>
            From shared G1s
          </th>
          <th scope="col" class={head}>
            Affinity
          </th>
        </tr>
      </thead>
      <tfoot>
        <Row
          kind="total"
          name="Total"
          legacy={{ rel: sum((x) => x.rel), race: sum((x) => x.race), aff: sum((x) => x.aff) }}
        />
      </tfoot>
      {legs.map((parent, k) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: a legacy's place is its identity (Legacy 1, Legacy 2)
        <tbody key={k}>
          <Row kind={k === 0 ? 'parent' : 'second'} name={<Who legacy={parent} legacyIndex={k} />} legacy={parent} />
          {gps(parent).map((gp, j) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: a grandparent's place under her parent is her identity
            <Row key={j} kind="grandparent" name={<Who legacy={gp} />} legacy={gp} />
          ))}
        </tbody>
      ))}
    </table>
  );
}
