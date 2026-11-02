// The tree's node popup: the trainee, a parent place, a veteran (also the veterans dialog's pane: VetPane), a
// grandparent. It draws the node on show (dialogs.ts `nodeOpen`) from the answer and the veterans store, so an edit in
// it, or one behind it, redraws it in place; a node that is gone (a removed veteran, a grandparent whose parent place
// emptied) closes it. Its edits address the node by where it is (veterans/store.ts `vetRef`, `parentRef`).
import { batch } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { settings } from '../../state/settings.ts';
import { isOwned } from '../../model/job.ts';
import { fitSparks } from '../../results/details.ts';
import {
  vets,
  places,
  editVet,
  removeVet,
  setTrainee,
  setParent,
  vetRef,
  parentRef,
  parentsPlaced,
} from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import { lacks, lacksSaid } from '../../veterans/complete.ts';
import { openWins, winsText, changeUma, openUmaPicker } from '../../veterans/pickers.ts';
import type { NodeRecord, NodeRef, ParentPlace, VetRecord } from '../../veterans/store.ts';
import { Dialog, DialogBody, DialogClose, DialogFoot, DialogTitle, anyDialogOpen } from '../../ui/Dialog.tsx';
import { UmaAv } from '../../ui/Images.tsx';
import { AffTable } from '../../results/AffTable.tsx';
import { InspView } from '../../veterans/InspView.tsx';
import { listed } from '../job.ts';
import { idxOf, shown, parentAt } from '../current.ts';
import type { ShownCareer as CareerReport } from './Sections.tsx';
import { nodeOpen, nodeClosed, openParentPicker, closeNode } from './dialogs.ts';
import { focusChosen, sayVets } from './poolState.ts';
import type { NodeSpec } from './dialogs.ts';
import { Guard } from './Guard.tsx';
import { Button } from '../../ui/Button.tsx';
import { SegmentedControl } from '../../ui/SegmentedControl.tsx';

// her affinity, legacy by legacy, from the report's six slots (career/inherit.rs `slots`)
function Affinity({ r }: { r: CareerReport }) {
  const nm = (x: { name?: string; chara?: number | null }) => x.name || umaName(x.chara);
  const legs = r.parents
    .map((ix) => r.slots.find((x) => x.kind === 'parent' && x.legacy === ix))
    .filter((p) => !!p)
    .map((p) => ({
      face: <UmaAv key={p.chara} uma={p.chara || 0} outfit={null} size={30} />,
      name: nm(p),
      rel: p.rel,
      race: p.race,
      aff: p.aff,
      gps: r.slots
        .filter((x) => x.kind === 'grandparent' && x.legacy === p.legacy)
        .sort((a, b) => (a.gp ?? 0) - (b.gp ?? 0))
        .map((g) => ({
          face: <UmaAv key={g.chara} uma={g.chara || 0} outfit={null} size={24} />,
          name: nm(g),
          rel: g.rel,
          race: g.race,
          aff: g.aff,
        })),
    }));
  return <AffTable legs={legs} />;
}
// a dialog of its own from the popup (the uma or parent picker): the popup closes first. Two writes, not one batch: the
// popup must close (the browser gives the focus back to its opener) before the other opens and takes that as the place
// to give it back to; a batch runs its effects last written first, so the picker would open first (signals-core
// endBatch), and the focus would end on the page when it closes
const fromNode = (fn: () => void) => () => {
  closeNode();
  fn();
};
/** her face: a button that changes who she is, or (a listed veteran) just her face */
interface FaceProps {
  nd: NodeRecord;
  /** where a change of uma goes */
  at: NodeRef;
  size: number;
  label: string;
  change: boolean;
}
// a face that is a button (data-face: the checks count them), brighter under the pointer
const FACE =
  'focus-visible:outline-accent cursor-pointer rounded-md leading-0 hover:brightness-106 focus-visible:outline-2 focus-visible:outline-offset-2';
// who a node is: her face, name and the rest in a wrapping row
const WHO = 'flex flex-wrap items-center gap-x-3 gap-y-2';
// her name in the popup (in the pool's pane, plain)
const NAME = 'text-title font-black';
function Face({ nd, at, size, label, change }: FaceProps) {
  const face = <UmaAv key={`${nd.chara}:${nd.outfit}`} uma={nd.chara || 0} outfit={nd.outfit} size={size} />;
  if (!change) return face;
  const title = `${label}: ${umaName(nd.chara)}. Change who she is`;
  return (
    <button
      type="button"
      class={FACE}
      data-face=""
      title={title}
      aria-label={title}
      onClick={fromNode(() => changeUma(at, label))}
    >
      {face}
    </button>
  );
}
/** her G1 wins: the button that opens their window (veterans/pickers.ts openWins), or (a listed veteran) their count */
interface WinsProps {
  nd: NodeRecord;
  /** where her edits go */
  at: NodeRef;
  who: string;
  k: string;
  edit: boolean;
}
function Wins({ nd, at, who, k, edit }: WinsProps) {
  if (!edit) return <span class="text-caption px-2 py-0.5">{winsText(nd)}</span>;
  return (
    <Button
      size="caption"
      data-k={`${k}:wins`}
      title="G1 wins (race affinity): click to edit"
      aria-haspopup="dialog"
      onClick={() => openWins(at, who)}
    >
      {winsText(nd)}
    </Button>
  );
}
/** the trainer to follow in the game for an uma.moe veteran: a Copy trainer ID button */
function CopyId({ v }: { v: VetRecord }) {
  const lender = v.lender as NonNullable<VetRecord['lender']>,
    [said, setSaid] = useState('Copy ID');
  return (
    <Button
      size="note"
      title={`${lender.name}: ${lender.id}`}
      onClick={async () => {
        try {
          await navigator.clipboard.writeText(lender.id);
          setSaid('Copied');
        } catch {
          setSaid(lender.id);
        }
      }}
    >
      {said}
    </Button>
  );
}
/** her name with how many of the targets her own sparks carry, "6/8" (an n/n veteran carries all n); `look`, the
 *  name's type */
function NameCover({ v, look }: { v: NodeRecord; look: string }) {
  const groups = settings.value.targets.map((t) => t.group),
    x = groups.filter((g) => ((v.sparks as Record<number, number>)?.[g] || 0) > 0).length;
  return (
    <div class={look}>
      {umaName(v.chara)}{' '}
      <span
        class="bg-carried text-note ml-1 rounded-full px-1.5 align-[2px] font-black text-white tabular-nums"
        title={`Her sparks carry ${x} of your ${groups.length} targets`}
      >
        {`${x}/${groups.length}`}
      </span>
    </div>
  );
}
/** her "Incomplete" tag, what she lacks in its tooltip (complete.ts); none when she is complete */
function Incomplete({ v }: { v: VetRecord }) {
  const lacking = lacks(v);
  if (!lacking.length) return null;
  return (
    <span
      class="border-out text-out font-data text-caption rounded-xs border px-1.75 py-0.75 font-bold"
      data-incomplete=""
      title={`Missing ${lacksSaid(lacking)}: the planner leaves her out`}
    >
      Incomplete
    </span>
  );
}
// (her trainer's followers too: at 1,000 the veteran can't be borrowed by following)
const lenderText = (l: NonNullable<VetRecord['lender']>) =>
  `uma.moe · ${l.name || 'trainer'} · ${l.followers >= 1000 ? 'max followers' : `${l.followers} follower${l.followers === 1 ? '' : 's'}`}`;
// where a veteran came from (entered from a career here, an uma.moe tree, an import, made here)
const VET_SOURCE: Record<string, string> = {
  entered: 'Entered here',
  simulated: 'Entered here',
  tree: 'From uma.moe',
  umamoe: 'From uma.moe',
  import: 'Imported',
  hand: 'Made here',
  sample: 'Sample',
};
/** a veteran's title in the popup and the pane ('' when she is gone) */
function veteranTitle(id: string) {
  const i = idxOf(id),
    v = vets.value[i];
  if (!v) return '';
  return v.lender ? `uma.moe · ${v.lender.name || 'trainer'}` : `Veteran #${i + 1}${v.borrowed ? ' · guest' : ''}`;
}
/** A veteran of yours (the popup, the pane beside the pool's list: `pane`): her face, name, targets carried, wins,
 *  where she came from; Yours / Guest and Remove; her sparks and her parents'; her parents' faces and G1 wins. */
function VeteranView({ id, pane = false }: { id: string; pane?: boolean }) {
  const i = idxOf(id),
    v: VetRecord | undefined = vets.value[i];
  if (!v) return null;
  const at = vetRef(id),
    targets = settings.value.targets.map((t) => t.group),
    edit = !listed(v),
    src = VET_SOURCE[v.src],
    name = umaName(v.chara),
    nameLook = pane ? '' : NAME;
  return (
    <>
      {/* (data-who: the checks find her row by it) */}
      <div class={WHO} data-who="">
        <Face nd={v} at={at} size={56} label={`Veteran #${i + 1}`} change={edit} />
        <NameCover v={v} look={nameLook} />
        <Wins nd={v} at={at} who={`Veteran #${i + 1}, ${name}`} k={`v${v.id}`} edit={edit} />
        {/* where she came from (data-src: from uma.moe, or here) */}
        {src && (
          <span
            class="border-line font-data text-caption rounded-xs border px-1.75 py-0.75"
            data-src={v.src === 'tree' || v.src === 'umamoe' ? 'umamoe' : 'here'}
          >
            {v.lender ? lenderText(v.lender) : src}
          </span>
        )}
        {v.lender && <CopyId v={v} />}
        <Incomplete v={v} />
      </div>
      {edit && (
        <div class="mt-0.5 mb-2 flex flex-wrap items-center gap-2.5">
          <SegmentedControl<boolean>
            kind="toggle"
            size="sm"
            label={`Veteran #${i + 1}: yours or a guest`}
            value={v.borrowed}
            onPick={(b) => {
              if (v.borrowed !== b) editVet(id, () => ({ borrowed: b }));
            }}
            options={([false, true] as const).map((b) => ({
              v: b,
              dataK: `v${v.id}:own${b ? 'bor' : ''}`,
              body: b ? 'Guest' : 'Yours',
            }))}
          />
          <Button
            class="ml-auto"
            onClick={() => {
              batch(() => {
                closeNode();
                removeVet(id);
              });
              sayVets(`Removed ${name} from your veterans.`);
              focusChosen(); // (her card, which had the focus or opened this popup, is gone)
            }}
          >
            Remove
          </Button>
        </div>
      )}
      <InspView
        nd={v}
        parents={v.parents}
        k={`v${v.id}`}
        at={at}
        targets={targets}
        parentKey={(j) => `v${v.id}g${j}`}
        parentAt={(j) => parentRef(id, j)}
        readOnly={!edit}
      />
      <div class="grid gap-1.5">
        {parentsPlaced(v.parents).map(([g, j]) => {
          const lab = `Parent ${j + 1} of #${i + 1}`;
          return (
            <div key={j} class="flex items-center gap-2">
              <Face nd={g} at={parentRef(id, j)} size={36} label={lab} change={edit} />
              <span class={nameLook}>{g.chara ? umaName(g.chara) : 'Not entered'}</span>
              <Wins nd={g} at={parentRef(id, j)} who={`${lab}, ${umaName(g.chara)}`} k={`v${v.id}g${j}`} edit={edit} />
            </div>
          );
        })}
      </div>
    </>
  );
}
/** the pane beside the pool's list (wide): the veteran chosen there (Pool.tsx) */
export function VetPane({ id }: { id: string }) {
  return (
    <>
      <h4 class="text-title text-gm-ink mb-2 font-black">{veteranTitle(id)}</h4>
      <VeteranView key={id} id={id} pane />
    </>
  );
}
/** a node's popup: its title, its body, its actions (in the footer beside Close); null: the node is gone */
interface NodeShown {
  title: string;
  body: ComponentChildren;
  acts?: ComponentChildren;
}
// The trainee: who she is, her outfit, her affinity's breakdown; choose her or hand her back to the planner.
function traineeNode(r: CareerReport | null): NodeShown {
  const plan = places.value,
    id = plan.trainee || r?.trainee.id,
    mine = !!r && r.trainee.id === id;
  return {
    title: plan.trainee ? 'Trainee' : "Trainee · planner's pick",
    body: (
      <>
        {id ? (
          <div class={WHO}>
            <UmaAv
              key={`${id}:${mine ? r?.trainee.outfits[0]?.id : null}`}
              uma={id}
              outfit={mine ? r?.trainee.outfits[0]?.id : null}
              size={72}
            />
            <div>
              <div class={NAME}>{umaName(id)}</div>
              {mine && r?.trainee.outfits[0] && <div class="text-ui text-muted">{r.trainee.outfits[0].label}</div>}
              {mine && r?.trainee.outfitNote && <div class="text-note text-muted mt-0.5">{r.trainee.outfitNote}</div>}
            </div>
          </div>
        ) : (
          <p class="text-muted">The planner tries every uma you own.</p>
        )}
        {mine && r && <Affinity r={r} />}
      </>
    ),
    acts: (
      <>
        <Button
          variant="primary"
          onClick={fromNode(() =>
            openUmaPicker(
              'Trainee: which uma?',
              (u: number) => setTrainee(u, true), // (chosen by hand: it stays)
              (u: Parameters<typeof isOwned>[0]) => isOwned(u, settings.value),
            ),
          )}
        >
          {plan.trainee ? 'Change trainee' : 'Choose trainee'}
        </Button>
        {plan.trainee && <Button onClick={() => setTrainee(null, false)}>Let the planner pick</Button>}
      </>
    ),
  };
}
// A parent place: the veteran in it (chosen or the planner's pick) with her sparks and her parents', all editable.
function parentNode(k: number, r: CareerReport | null): NodeShown {
  const plan = places.value,
    at = parentAt(k, r),
    v: VetRecord | undefined = at?.v;
  const acts = (
    <>
      <Button variant="primary" size="body" onClick={fromNode(() => openParentPicker(k))}>
        {plan.parents[k] ? 'Change parent' : 'Choose parent'}
      </Button>
      {plan.parents[k] && <Button onClick={() => setParent(k, null, false)}>Let the planner pick</Button>}
    </>
  );
  const title = `Legacy ${k + 1} · parent ${k + 1}${at?.pick ? " · planner's pick" : ''}`;
  if (!v)
    return {
      title,
      body: (
        <p class="text-muted">{vets.value.length >= 2 ? 'The planner picks from your veterans.' : 'No parent yet.'}</p>
      ),
      acts,
    };
  const i = idxOf(v.id);
  return {
    title,
    body: (
      <>
        <div class={WHO}>
          <button
            type="button"
            class={FACE}
            data-face=""
            title="Choose another parent"
            aria-label={`Parent ${k + 1}: ${umaName(v.chara)}. Choose another parent`}
            onClick={fromNode(() => openParentPicker(k))}
          >
            <UmaAv key={`${v.chara}:${v.outfit}`} uma={v.chara} outfit={v.outfit} size={64} />
          </button>
          <div>
            <NameCover v={v} look="text-title leading-tight font-black" />
            <div class="text-ui text-gm-ink2">
              {v.lender
                ? `uma.moe · ${v.lender.name || 'trainer'}`
                : `Veteran #${i + 1}${v.src === 'hand' ? ', made here' : ''}${v.borrowed ? ', borrowed' : ''}`}
            </div>
          </div>
          <Wins nd={v} at={vetRef(v.id)} who={umaName(v.chara)} k={`p${k}`} edit={!listed(v)} />
          {v.lender && <CopyId key={v.id} v={v} />}
          <Incomplete v={v} />
        </div>
        <InspView
          nd={v}
          parents={v.parents}
          k={`p${k}`}
          at={vetRef(v.id)}
          targets={settings.value.targets.map((t) => t.group)}
          parentKey={(j) => `p${k}g${j}`}
          parentAt={(j) => parentRef(v.id, j)}
          readOnly={listed(v)}
        />
      </>
    ),
    acts,
  };
}
// A grandparent (parent k's parent j): who she is, her sparks and G1 wins.
function grandparentNode(k: number, j: ParentPlace, r: CareerReport | null): NodeShown | null {
  const at = parentAt(k, r),
    g: NodeRecord | null | undefined = at?.v.parents[j];
  if (!at || !g) return null; // (no parent place, or her parent is absent)
  const edit = !listed(at.v),
    gAt = parentRef(at.v.id, j);
  return {
    title: `Legacy ${k + 1} · grandparent`,
    body: (
      <>
        <div class={WHO}>
          <Face nd={g} at={gAt} size={56} label="Grandparent" change={edit} />
          <div class={NAME}>{g.chara ? umaName(g.chara) : 'Not entered'}</div>
          <Wins nd={g} at={gAt} who={umaName(g.chara)} k={`p${k}g${j}`} edit={edit} />
        </div>
        <InspView
          nd={g}
          parents={[]}
          k={`p${k}g${j}`}
          at={gAt}
          targets={settings.value.targets.map((t) => t.group)}
          readOnly={!edit}
        />
      </>
    ),
  };
}
function nodeShown(spec: NodeSpec, r: CareerReport | null): NodeShown | null {
  if (spec.kind === 't') return traineeNode(r);
  if (spec.kind === 'p') return parentNode(spec.k, r);
  if (spec.kind === 'g') return grandparentNode(spec.k, spec.j, r);
  const title = veteranTitle(spec.id);
  return title ? { title, body: <VeteranView key={spec.id} id={spec.id} /> } : null;
}
/** the node popup (game style): one dialog, drawn again on every change while open */
export function NodeDialog() {
  const spec = nodeOpen.value,
    body = useRef<HTMLDivElement>(null);
  // (closed, it draws nothing: an edit elsewhere redraws no closed popup)
  const on = spec && nodeShown(spec, shown());
  // a node that is gone closes the popup; on show, her spark names shrink to fit (results/details.ts fitSparks: this
  // runs after its Dialog's layout effect has opened it)
  useLayoutEffect(() => {
    if (spec && !on) closeNode();
    if (on && body.current) fitSparks(body.current);
  });
  // closed: the focus goes back to the button that opened it, as the browser does; one drawn again since (a pool card)
  // is found by its own view (`back`)
  const closed = () => {
    const back = nodeClosed();
    if (back)
      setTimeout(() => {
        if (document.activeElement === document.body && !anyDialogOpen()) back();
      });
  };
  return (
    <Dialog size="md" id="node-dlg" kind="popup" labelledBy="node-title" open={!!spec && !!on} onClose={closed}>
      <DialogTitle id="node-title">{on ? on.title : ''}</DialogTitle>
      {/* (the guards inside the dialog: a throw leaves it open, and Close still closes it) */}
      <DialogBody id="node-body" ref={body}>
        <Guard what="The node popup" also={nodeOpen}>
          {on?.body}
        </Guard>
      </DialogBody>
      <DialogFoot>
        <DialogClose onClick={closeNode} />
        <Guard what="The node popup's buttons" also={nodeOpen}>
          {on?.acts}
        </Guard>
      </DialogFoot>
    </Dialog>
  );
}
