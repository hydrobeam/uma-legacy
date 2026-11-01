// ---------- the component twin of details.js's inspView: the game's Inspiration view of a veteran ----------
// "Sparks", her sparks as the game's spark bars, then "Legacy Origin": her parents' (each a row with her portrait). The
// markup inspView draws; inspView stays for the veteran analysis (step 9). Her parents' sparks are editable when
// `parentKey` names their controls.
import { umaName } from '../vets.js';
import type { LineageNode } from '../vets.js';
import { UmaAv } from './Images.tsx';
import { Sparks } from './Sparks.tsx';

interface InspViewProps {
  /** the veteran shown */
  nd: LineageNode;
  /** her parents (none: her row alone) */
  parents: LineageNode[];
  /** her sparks' control names */
  k: string;
  onEdit: () => void;
  /** parent j's sparks' control names (null: her parents' read-only) */
  parentKey?: ((j: number) => string) | null;
  /** nothing editable (a veteran listed on uma.moe: another player's) */
  readOnly?: boolean;
}
export function InspView({ nd, parents, k, onEdit, parentKey = null, readOnly = false }: InspViewProps) {
  return (
    <div class="udinsp">
      <div class={`udlin${parents.length ? '' : ' solo'}`}>
        <div class="udrow me">
          <UmaAv uma={nd.chara || 0} outfit={nd.outfit} size={64} cls="udface" />
          {/* biome-ignore lint/a11y/useSemanticElements: today's markup (details.js inspView); a fieldset's legend and border would need the spark grid restyled */}
          <div class="simskills" role="group" aria-label="Her sparks">
            <Sparks nd={nd} k={k} onEdit={onEdit} readOnly={readOnly} />
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
              uma={g.chara || 0}
              outfit={g.outfit}
              size={64}
              cls="udface"
              label={`Her parent: ${umaName(g.chara)}`}
            />
            <div class="udpsp">
              {parentKey && !readOnly ? (
                <Sparks nd={g} k={parentKey(j)} onEdit={onEdit} />
              ) : (
                <Sparks nd={g} k={`simp${j}`} onEdit={onEdit} readOnly />
              )}
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}
