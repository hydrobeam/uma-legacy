// The lineage tree: the trainee and the two legacies with their parents (Tree, into #tree), and the career's odds with
// Try a run beside the title (Verdict, into #lodds). The game's legacy display: portraits only (the trainee; Legacy 1
// blue and Legacy 2 pink, each the parent with her own two parents), each with its legacy number and affinity; a
// portrait opens the node popup (her sparks, G1 wins, choose or hand back the place, change who she is). Both draw from
// the answer and the veterans (`vetsRev`: their records and the places are edited in place until step 10).
import type { ComponentChildren } from 'preact';
import { useRef } from 'preact/hooks';
import { $dialog } from '../../dom.js';
import { vets, vetsRev, plan, umaName } from '../../vets.js';
import { UmaAv } from '../../ui/Images.tsx';
import { AffHeart } from '../../ui/AffTable.tsx';
import { renderSimulation } from '../analysis.js';
import { idxOf, shown, parentAt } from '../job.js';
import type { ShownCareer as CareerReport } from './Sections.tsx';
import { openNode } from './dialogs.ts';
import type { NodeSpec } from './dialogs.ts';
import { Odds, DeckButton, SchedButton, Targets } from './Sections.tsx';

interface PlaceProps {
  /** the node it opens */
  spec: NodeSpec;
  /** the box's classes */
  cls: string;
  /** its data-k (the suite finds a node by it) */
  k: string;
  face: ComponentChildren;
  name: string;
  sub?: string | null;
  label: string;
  /** above the portrait (the trainee's head) */
  head?: ComponentChildren;
  /** under it (the trainee's deck and schedule) */
  children?: ComponentChildren;
}
// A node of the pedigree: its portrait button (her face, name and a line under it) in its box. A click anywhere in the
// box opens it, as the portrait does (not on the box's own buttons or deck cards); the popup gives the focus back to
// the portrait when it closes.
function Place({ spec, cls, k, face, name, sub, label, head, children }: PlaceProps) {
  const tile = useRef<HTMLButtonElement>(null),
    open = () => openNode(spec, () => tile.current?.focus());
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents lint/a11y/noStaticElementInteractions: the box widens the portrait's target for the pointer; the portrait is the keyboard's button
    <div
      class={`${cls} pnclick`}
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest('button, a, .deck, input')) open();
      }}
    >
      {head}
      <button
        ref={tile}
        type="button"
        class="ptile"
        data-k={k}
        aria-haspopup="dialog"
        aria-label={label}
        onClick={open}
      >
        {face}
        <span class="tnm">
          <span class="nm">{name}</span>
          {sub && <span class="sub">{sub}</span>}
        </span>
      </button>
      {children}
    </div>
  );
}
const empty = <span class="av empty" />;
// her affinity, in all, from the report's six slots (career/inherit.rs `slots`)
const legacyAff = (r: CareerReport) => r.slots.reduce((a, x) => a + x.aff, 0);
function Trainee({ r }: { r: CareerReport | null }) {
  const id = plan.trainee || r?.trainee.id,
    mine = !!r && r.trainee.id === id,
    outfit = mine ? r?.trainee.outfits[0] : undefined,
    tot = r && mine ? legacyAff(r) : null;
  return (
    <Place
      spec={{ kind: 't' }}
      cls={`pn trainee${plan.trainee ? '' : ' ghost'}`}
      k="node:t"
      face={id ? <UmaAv key={`${id}:${outfit?.id}`} uma={id} outfit={outfit?.id} size={96} /> : empty}
      name={id ? umaName(id) : 'Any uma'}
      sub={id ? outfit?.label || null : 'The planner tries every uma you own.'}
      label={`Trainee: ${id ? umaName(id) : 'the planner picks'}. Details`}
      head={
        <div class="pnh">
          <span class="pnl">{plan.trainee ? 'Your pick' : "Planner's pick"}</span>
          {tot !== null && (
            <AffHeart
              affinity={tot}
              cls="aff tot"
              title={`Affinity ${Math.round(tot)}, her six legacies: click her for the breakdown`}
            />
          )}
        </div>
      }
    >
      {r && mine && (
        <div class="row tdeck">
          <DeckButton />
          <SchedButton r={r} />
        </div>
      )}
    </Place>
  );
}
function Family({ k, r }: { k: number; r: CareerReport | null }) {
  const at = parentAt(k, r),
    v = at?.v;
  return (
    <div class="pf">
      <span class="pflab">{`Legacy ${k + 1}`}</span>
      {v ? (
        <Place
          spec={{ kind: 'p', k }}
          cls={`pn parent${at.pick ? ' ghost' : ''}${v.borrowed ? ' bor' : ''}`}
          k={`node:p${k}`}
          face={<UmaAv key={`${v.chara}:${v.outfit}`} uma={v.chara} outfit={v.outfit} size={64} />}
          name={umaName(v.chara)}
          sub={
            v.lender
              ? `uma.moe · ${v.lender.name || 'trainer'}`
              : `Veteran #${idxOf(v.id) + 1}${v.borrowed ? ' · borrowed' : v.src === 'hand' ? ', made here' : ''}`
          }
          label={`Parent ${k + 1}: ${umaName(v.chara)}. Details`}
        />
      ) : (
        <Place
          spec={{ kind: 'p', k }}
          cls="pn parent"
          k={`node:p${k}`}
          face={empty}
          name={vets.length >= 2 ? 'The planner picks' : 'No parent yet'}
          label={`Parent ${k + 1}: none yet. Details`}
        />
      )}
      {/* her own parents: the trainee's grandparents */}
      {v && (
        <div class="pgs">
          {v.parents.map((g: { chara: number | null; outfit?: number }, j: number) => (
            <Place
              // biome-ignore lint/suspicious/noArrayIndexKey: her two parents, in their fixed places
              key={j}
              spec={{ kind: 'g', k, j }}
              cls="pn gp"
              k={`node:p${k}g${j}`}
              face={<UmaAv key={`${g.chara}:${g.outfit}`} uma={g.chara || 0} outfit={g.outfit} size={36} />}
              name={g.chara ? umaName(g.chara) : 'Unknown: pick her'}
              label={`Grandparent: ${g.chara ? umaName(g.chara) : 'unknown'}. Details`}
            />
          ))}
        </div>
      )}
    </div>
  );
}
/** The next career as the player sets it up in the game, in the game's order: the trainee, her two legacies, and each
 *  target's chance to land */
export function Tree() {
  vetsRev.value;
  const r = shown();
  return (
    <>
      <section class="rbsec ltrainee">
        <h3>Trainee</h3>
        <Trainee r={r} />
      </section>
      <section class="rbsec llegacies">
        <h3>Legacies</h3>
        <div class="pk">
          <Family k={0} r={r} />
          <Family k={1} r={r} />
        </div>
      </section>
      {r && r.trainee.id === (plan.trainee || r.trainee.id) && <Targets r={r} />}
    </>
  );
}
/** the career's odds of landing every target, and Try a run (a roll of the veteran this career leaves and her
 *  comparison, in a popup) */
export function Verdict() {
  vetsRev.value;
  const r = shown();
  if (!r) return null;
  return (
    <>
      <Odds r={r} />
      <button
        type="button"
        class="btn"
        id="sim-try"
        aria-haspopup="dialog"
        title="Roll a random result of this career and see whether the veteran it leaves helps your loop"
        onClick={() => {
          renderSimulation();
          $dialog('sim-dlg').showModal();
        }}
      >
        Try a run
      </button>
    </>
  );
}
