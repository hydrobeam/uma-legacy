// ---------- the game's Inspiration view of a veteran ----------
// "Sparks" (Try a run's card: `header`), her sparks as the game's spark bars, then "Legacy Origin": her parents' (each a
// row with her portrait; an absent one has none). Her parents' sparks are editable when `parentKey` names their controls. It reads no store (as
// Sparks): it draws again whenever its parent does. Each row her portrait in the game's white and cream rings beside her
// sparks (48 px on a narrow phone, 64 elsewhere), a line joining hers to her parents' down the portraits' middle; the
// rows are data-row (me, parent).
import { umaName } from '../veterans/names.ts';
import { parentsPlaced } from '../veterans/store.ts';
import type { NodeRecord, NodeRef, ParentPlace } from '../veterans/store.ts';
import { UmaAv } from './Images.tsx';
import { Sparks } from './Sparks.tsx';
import { GameBar } from './GameBar.tsx';

interface InspViewProps {
  /** the veteran shown */
  nd: NodeRecord;
  /** her parents in their places, an absent one null (none: her row alone) */
  parents: readonly (NodeRecord | null)[];
  /** her sparks' control names */
  k: string;
  /** where her edits go */
  at: NodeRef;
  /** the target skills' groups */
  targets: readonly number[];
  /** parent j's sparks' control names (null: her parents' read-only) */
  parentKey?: ((j: ParentPlace) => string) | null;
  /** where parent j's edits go (with `parentKey`) */
  parentAt?: ((j: ParentPlace) => NodeRef) | null;
  /** nothing editable (a veteran listed on uma.moe: another player's) */
  readOnly?: boolean;
  /** the "Sparks" heading */
  header?: boolean;
  /** parent j's place, read out with her portrait ("#3"; null: "Her parent") */
  parentLabel?: ((j: ParentPlace) => string) | null;
}
// her row and each parent's: the portrait's column, then the sparks (the second parent's 10 px further down)
const ROW =
  'relative grid grid-cols-[64px_minmax(0,1fr)] items-start gap-2.5 max-sm:grid-cols-[48px_minmax(0,1fr)] max-sm:gap-1.5';
const ROW_NEXT =
  'relative mt-2.5 grid grid-cols-[64px_minmax(0,1fr)] items-start gap-2.5 max-sm:grid-cols-[48px_minmax(0,1fr)] max-sm:gap-1.5';
// the rows, and with parents under her the line joining them (down the portraits' middle, from under hers to the last's)
const LINEAGE = {
  solo: 'relative grid gap-2',
  joined:
    'before:bg-gm-edge relative grid gap-2 before:absolute before:top-17.5 before:bottom-8.5 before:left-7.5 before:w-1 before:rounded-full max-sm:before:top-13.5 max-sm:before:left-5.5',
} as const;
export function InspView({
  nd,
  parents,
  k,
  at,
  targets,
  parentKey = null,
  parentAt = null,
  readOnly = false,
  header = false,
  parentLabel = null,
}: InspViewProps) {
  return (
    <div class="grid gap-2">
      {header && <GameBar as="div">Sparks</GameBar>}
      <div class={parents.length ? LINEAGE.joined : LINEAGE.solo}>
        <div class={ROW} data-row="me">
          <UmaAv
            key={`${nd.chara}:${nd.outfit}`}
            uma={nd.chara || 0}
            outfit={nd.outfit}
            size={64}
            shape="insp"
            ring="insp"
            cls="relative"
          />
          {/* biome-ignore lint/a11y/useSemanticElements: today's markup (the el() view's before it); a fieldset's legend and border would need the spark grid restyled */}
          <div class="min-w-0" role="group" aria-label="Her sparks">
            <Sparks nd={nd} k={k} at={at} targets={targets} readOnly={readOnly} />
          </div>
        </div>
        {parents.length > 0 && (
          <div class="after:bg-gm-line flex items-center gap-2.5 pl-18.5 font-bold after:h-px after:flex-1 max-sm:pl-13.5">
            <span>Legacy Origin</span>
          </div>
        )}
        {parentsPlaced(parents).map(([g, j], n) => (
          <div key={j} class={n ? ROW_NEXT : ROW} data-row="parent">
            <UmaAv
              key={`${g.chara}:${g.outfit}`}
              uma={g.chara || 0}
              outfit={g.outfit}
              size={64}
              shape="insp"
              ring="insp"
              cls="relative"
              label={`${parentLabel ? `Parent ${parentLabel(j)}` : 'Her parent'}: ${g.chara ? umaName(g.chara) : 'not entered'}`}
            />
            <div>
              {parentKey && parentAt && !readOnly ? (
                <Sparks nd={g} k={parentKey(j)} at={parentAt(j)} targets={targets} parent />
              ) : (
                <Sparks nd={g} k={`simp${j}`} at={at} targets={targets} readOnly />
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
