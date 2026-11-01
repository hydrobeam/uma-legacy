// The lineage tree: the trainee and the two legacies with their parents (Tree, into #tree), and the career's odds with
// Try a run beside the title (Verdict, into #lodds). The game's legacy display: portraits only (the trainee; Legacy 1
// blue and Legacy 2 pink, each the parent with her own two parents), each with its legacy number and affinity; a
// portrait opens the node popup (her sparks, G1 wins, choose or hand back the place, change who she is). Both draw from
// the answer and the veterans store (veterans/store.ts).
import type { ComponentChildren } from 'preact';
import { useRef } from 'preact/hooks';
import { vets, places, parentsPlaced } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import { UmaAv } from '../../ui/Images.tsx';
import { AffHeart } from '../../ui/AffTable.tsx';
import { openTryRun } from '../analysis/view.tsx';
import { idxOf, shown, parentAt } from '../current.ts';
import type { ShownCareer as CareerReport } from './Sections.tsx';
import { openNode } from './dialogs.ts';
import type { NodeSpec } from './dialogs.ts';
import { Odds, DeckButton, SchedButton, Targets } from './Sections.tsx';
import { Button } from '../../ui/Button.tsx';
import { GameBar } from '../../ui/GameBar.tsx';
import { cx } from 'tailwind-variants/lite';

// A node's look by its kind, each part's list whole: the trainee (her face over her name from 760 px, beside it under
// that), a parent (her face beside her name, in her legacy box), a grandparent (a face alone in a small panel, her name
// left to her label). The box takes a ring under the pointer; its portrait button none (it keeps its focus ring).
const NODE = {
  trainee: {
    box: 'hover:shadow-node-hover grid min-w-0 cursor-pointer justify-items-center gap-1 rounded-2xl p-1',
    tile: 'grid grid-cols-1 items-center justify-items-center gap-x-3 gap-y-0.5 text-center max-lg:grid-cols-[auto_minmax(0,1fr)] max-lg:justify-items-start max-lg:text-left',
    names: 'grid justify-items-center max-lg:justify-items-start',
    name: 'text-heading leading-tight font-black',
  },
  parent: {
    box: 'hover:shadow-node-hover grid min-w-0 cursor-pointer justify-items-stretch gap-1 rounded-2xl p-1',
    tile: 'grid grid-cols-[auto_minmax(0,1fr)] items-center justify-items-start gap-x-3 gap-y-0.5 justify-self-start text-left',
    names: 'grid justify-items-start',
    name: 'text-body leading-tight font-black',
  },
  gp: {
    box: 'bg-gm-panel border-gm-edge shadow-node hover:shadow-node-hover grid min-w-0 cursor-pointer justify-items-center gap-1 rounded-2xl border px-1.5 pt-1 pb-1.5',
    tile: 'grid justify-items-center gap-0.5 text-center',
    names: 'hidden',
    name: '',
  },
} as const;
interface PlaceProps {
  /** the node it opens */
  spec: NodeSpec;
  kind: keyof typeof NODE;
  /** a place the planner fills: dimmed while a run works on the next answer */
  ghost?: boolean;
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
// box opens it, as the portrait does (not on the box's own buttons); the popup gives the focus back to the portrait
// when it closes. data-node says its kind, data-ghost a place the planner fills (the checks read both).
function Place({ spec, kind, ghost = false, k, face, name, sub, label, head, children }: PlaceProps) {
  const tile = useRef<HTMLButtonElement>(null),
    open = () => openNode(spec, () => tile.current?.focus()),
    look = NODE[kind];
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents lint/a11y/noStaticElementInteractions: the box widens the portrait's target for the pointer; the portrait is the keyboard's button
    <div
      class={cx(look.box, ghost && 'stale:opacity-45')}
      data-node={kind}
      data-ghost={ghost ? '' : undefined}
      onClick={(e) => {
        if (!(e.target as HTMLElement).closest('button, a, input')) open();
      }}
    >
      {head}
      <button
        ref={tile}
        type="button"
        class={cx(
          look.tile,
          'focus-visible:outline-accent cursor-pointer rounded-lg px-1 py-0.5 focus-visible:outline-3 focus-visible:outline-offset-2',
        )}
        data-k={k}
        aria-haspopup="dialog"
        aria-label={label}
        onClick={open}
      >
        {face}
        <span class={look.names}>
          <span class={look.name} data-nm="">
            {name}
          </span>
          {sub && <span class="text-caption text-gm-ink2 leading-tight">{sub}</span>}
        </span>
      </button>
      {children}
    </div>
  );
}
// an empty place: a dashed circle
const empty = (
  <span class="border-gm-edge bg-gm-pane inline-block size-18 shrink-0 rounded-full border-2 border-dashed" />
);
// her affinity, in all, from the report's six slots (career/inherit.rs `slots`)
const legacyAff = (r: CareerReport) => r.slots.reduce((a, x) => a + x.aff, 0);
function Trainee({ r }: { r: CareerReport | null }) {
  const plan = places.value,
    id = plan.trainee || r?.trainee.id,
    mine = !!r && r.trainee.id === id,
    outfit = mine ? r?.trainee.outfits[0] : undefined,
    tot = r && mine ? legacyAff(r) : null;
  return (
    <Place
      spec={{ kind: 't' }}
      kind="trainee"
      ghost={!plan.trainee}
      k="node:t"
      face={
        id ? (
          <UmaAv key={`${id}:${outfit?.id}`} uma={id} outfit={outfit?.id} size={96} shape="trainee" ring="node" />
        ) : (
          empty
        )
      }
      name={id ? umaName(id) : 'Any uma'}
      sub={id ? outfit?.label || null : 'The planner tries every uma you own.'}
      label={`Trainee: ${id ? umaName(id) : 'the planner picks'}. Details`}
      head={
        <div class="text-gm-ink flex w-full flex-wrap items-center justify-center gap-1.5">
          <span class="text-caption text-gm-ink2 font-semibold tracking-wider uppercase">
            {plan.trainee ? 'Your pick' : "Planner's pick"}
          </span>
          {tot !== null && (
            <AffHeart
              affinity={tot}
              face="game"
              title={`Affinity ${Math.round(tot)}, her six legacies: click her for the breakdown`}
            />
          )}
        </div>
      }
    >
      {r && mine && (
        <div class="stale:opacity-45 mt-0.5 flex flex-wrap items-center justify-center gap-2">
          <DeckButton />
          <SchedButton r={r} />
        </div>
      )}
    </Place>
  );
}
// the game's legacy boxes, Legacy 1 blue and Legacy 2 pink: the box, and its label in the top left corner
const LEGACY = [
  { box: 'border-legacy-blue bg-legacy-blue-soft', label: 'bg-legacy-blue text-shadow-legacy-blue' },
  { box: 'border-legacy-pink bg-legacy-pink-soft', label: 'bg-legacy-pink text-shadow-legacy-pink' },
] as const;
function Family({ k, r }: { k: 0 | 1; r: CareerReport | null }) {
  const at = parentAt(k, r),
    v = at?.v,
    look = LEGACY[k];
  return (
    // data-legacy: its number (the checks find a box by it)
    <div
      class={cx(
        look.box,
        'relative grid min-w-0 grid-cols-[minmax(0,1fr)_auto] items-center gap-2 rounded-2xl border-3 px-2.5 pt-6.5 pb-2 max-md:px-1.25 max-md:pt-7 max-md:pb-1.5',
      )}
      data-legacy={k + 1}
    >
      <span
        class={cx(
          look.label,
          'text-body absolute top-0 left-0 z-1 rounded-tl-lg rounded-br-xl pt-0.5 pr-4 pb-0.75 pl-2.5 font-black text-white',
        )}
      >{`Legacy ${k + 1}`}</span>
      {v ? (
        <Place
          spec={{ kind: 'p', k }}
          kind="parent"
          ghost={at.pick}
          k={`node:p${k}`}
          face={<UmaAv key={`${v.chara}:${v.outfit}`} uma={v.chara} outfit={v.outfit} size={64} ring="node" />}
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
          kind="parent"
          k={`node:p${k}`}
          face={empty}
          name={vets.value.length >= 2 ? 'The planner picks' : 'No parent yet'}
          label={`Parent ${k + 1}: none yet. Details`}
        />
      )}
      {/* her own parents: the trainee's grandparents (on a phone, in her popup) */}
      {v && (
        <div class="flex gap-1 max-lg:hidden">
          {parentsPlaced(v.parents).map(([g, j]) => (
            <Place
              key={j}
              spec={{ kind: 'g', k, j }}
              kind="gp"
              k={`node:p${k}g${j}`}
              face={<UmaAv key={`${g.chara}:${g.outfit}`} uma={g.chara || 0} outfit={g.outfit} size={36} ring="node" />}
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
 *  target's chance to land (data-sec names each section for the checks) */
export function Tree() {
  const r = shown();
  return (
    <>
      <section class="grid min-w-0 gap-2" data-sec="trainee">
        <GameBar>Trainee</GameBar>
        <Trainee r={r} />
      </section>
      <section class="grid min-w-0 gap-2" data-sec="legacies">
        <GameBar>Legacies</GameBar>
        <div class="grid min-w-0 gap-2.5">
          <Family k={0} r={r} />
          <Family k={1} r={r} />
        </div>
      </section>
      {r && r.trainee.id === (places.value.trainee || r.trainee.id) && <Targets r={r} />}
    </>
  );
}
/** the career's odds of landing every target, and Try a run (a roll of the veteran this career leaves and her
 *  comparison, in a popup) */
export function Verdict() {
  const r = shown();
  if (!r) return null;
  return (
    <>
      <Odds r={r} />
      <Button
        id="sim-try"
        aria-haspopup="dialog"
        title="Roll a random result of this career and see whether the veteran it leaves helps your loop"
        onClick={openTryRun}
      >
        Try a run
      </Button>
    </>
  );
}
