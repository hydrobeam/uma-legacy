// ---------- an affinity heart, and a career's affinity table legacy by legacy (it says what the table holds) ----------
import type { ComponentChildren } from 'preact';
import { must } from '../must.ts';

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
// one row: who (her face, name, tag, legacy number), then the legacy's relation, shared G1s and affinity
function Row({
  cls,
  who,
  legacy,
}: {
  cls: string;
  who: ComponentChildren;
  legacy: Pick<AffinityLeg, 'rel' | 'race' | 'aff'>;
}) {
  return (
    <tr class={cls}>
      <th scope="row">{who}</th>
      <td>{whole(legacy.rel)}</td>
      <td>{whole(legacy.race)}</td>
      <td class="afv">
        <AffHeart affinity={legacy.aff} />
      </td>
    </tr>
  );
}
// legacyIndex: the parent's place (Legacy 1, 2); a grandparent's row has none
const Who = ({ legacy, legacyIndex }: { legacy: AffinityLeg; legacyIndex?: number }) => (
  <>
    {legacy.face}
    <span class="afn">{legacy.name}</span>
    {legacy.tag && <span class="aft">{legacy.tag}</span>}
    {legacyIndex != null && <span class="afk">Legacy {legacyIndex + 1}</span>}
  </>
);
/** the parents' rows (Legacy 1 first), each with her parents', and the total */
export function AffTable({ legs, caption = 'Affinity' }: { legs: AffinityLeg[]; caption?: string }) {
  const gps = (parent: AffinityLeg) => must(parent.gps, "a parent's row carries her parents");
  const everyone = legs.flatMap((parent) => [parent, ...gps(parent)]),
    sum = (field: (x: AffinityLeg) => number) => everyone.reduce((total, x) => total + field(x), 0);
  return (
    <table class="afftab">
      {caption && <caption>{caption}</caption>}
      <thead>
        <tr>
          <th scope="col" />
          <th scope="col">Relation</th>
          <th scope="col">From shared G1s</th>
          <th scope="col">Affinity</th>
        </tr>
      </thead>
      <tfoot>
        <Row
          cls="tot"
          who={<span class="afk">Total</span>}
          legacy={{ rel: sum((x) => x.rel), race: sum((x) => x.race), aff: sum((x) => x.aff) }}
        />
      </tfoot>
      {legs.map((parent, k) => (
        // biome-ignore lint/suspicious/noArrayIndexKey: a legacy's place is its identity (Legacy 1, Legacy 2)
        <tbody key={k} class={`leg${k}`}>
          <Row cls="par" who={<Who legacy={parent} legacyIndex={k} />} legacy={parent} />
          {gps(parent).map((gp, j) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: a grandparent's place under her parent is her identity
            <Row key={j} cls="gp" who={<Who legacy={gp} />} legacy={gp} />
          ))}
        </tbody>
      ))}
    </table>
  );
}
