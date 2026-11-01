// ---------- the game's Inspiration view of a veteran ----------
// "Sparks" (Try a run's card: `header`), her sparks as the game's spark bars, then "Legacy Origin": her parents' (each a
// row with her portrait). Her parents' sparks are editable when `parentKey` names their controls. It reads no store (as
// Sparks): it draws again whenever its parent does.
import { umaName } from '../veterans/names.ts';
import type { NodeRecord, NodeRef } from '../veterans/store.ts';
import { UmaAv } from './Images.tsx';
import { Sparks } from './Sparks.tsx';

interface InspViewProps {
  /** the veteran shown */
  nd: NodeRecord;
  /** her parents (none: her row alone) */
  parents: readonly NodeRecord[];
  /** her sparks' control names */
  k: string;
  /** where her edits go */
  at: NodeRef;
  /** the target skills' groups */
  targets: readonly number[];
  /** parent j's sparks' control names (null: her parents' read-only) */
  parentKey?: ((j: number) => string) | null;
  /** where parent j's edits go (with `parentKey`) */
  parentAt?: ((j: number) => NodeRef) | null;
  /** nothing editable (a veteran listed on uma.moe: another player's) */
  readOnly?: boolean;
  /** the "Sparks" heading */
  header?: boolean;
  /** a class beside her row's own */
  meClass?: string;
  /** parent j's place, read out with her portrait ("#3"; null: "Her parent") */
  parentLabel?: ((j: number) => string) | null;
}
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
  meClass = '',
  parentLabel = null,
}: InspViewProps) {
  return (
    <div class="udinsp">
      {header && <div class="udhdr">Sparks</div>}
      <div class={`udlin${parents.length ? '' : ' solo'}`}>
        <div class={`udrow me${meClass ? ` ${meClass}` : ''}`}>
          <UmaAv key={`${nd.chara}:${nd.outfit}`} uma={nd.chara || 0} outfit={nd.outfit} size={64} cls="udface" />
          {/* biome-ignore lint/a11y/useSemanticElements: today's markup (the el() view's before it); a fieldset's legend and border would need the spark grid restyled */}
          <div class="simskills" role="group" aria-label="Her sparks">
            <Sparks nd={nd} k={k} at={at} targets={targets} readOnly={readOnly} />
          </div>
        </div>
        {parents.length > 0 && (
          <div class="udorigin">
            <span>Legacy Origin</span>
          </div>
        )}
        {parents.map((g, j) => (
          // biome-ignore lint/suspicious/noArrayIndexKey: her two parents, in their fixed places
          <div key={j} class="udrow par">
            <UmaAv
              key={`${g.chara}:${g.outfit}`}
              uma={g.chara || 0}
              outfit={g.outfit}
              size={64}
              cls="udface"
              label={`${parentLabel ? `Parent ${parentLabel(j)}` : 'Her parent'}: ${umaName(g.chara)}`}
            />
            <div class="udpsp">
              {parentKey && parentAt && !readOnly ? (
                <Sparks nd={g} k={parentKey(j)} at={parentAt(j)} targets={targets} />
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
