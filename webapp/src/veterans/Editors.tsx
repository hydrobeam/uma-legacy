// ---------- the veterans' editors as the game draws them ----------
// The uma picker, the skills prompt, the G1 wins window and the colour picker: one root (#vet-editors, `VetEditors`),
// their dialogs in that order, so a picker closing as the prompt opens gives the focus up first. The "+" list
// (`AddList`) draws into #add-pop, which it places by the "+" that opened it, inside that "+"'s open dialog (the page
// around a modal dialog is inert). What each shows, and its openers, are pickers.ts's and sparks.ts's. The checks find
// an uma's button as a button in #pick-list (or #vp-umas), a choice of the colour picker as its [data-bar] in #col-dlg,
// its stars by their names ("3 stars"), the "+" list's skills as the buttons in #add-pop.
import { useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import { createPortal } from 'preact';
import type { Ref } from 'preact';
import { gameData } from '../model/data.ts';
import type { Uma } from '../model/data.ts';
import { Dialog, DialogBody, DialogClose, DialogFoot, DialogHead, DialogPanel, DialogTitle } from '../ui/Dialog.tsx';
import { RenderGuard } from '../ui/RenderGuard.tsx';
import { UmaAv } from '../ui/Images.tsx';
import { WinsEditor } from '../ui/Agenda.tsx';
import { useDropdown } from '../ui/dropdown.ts';
import { AddPop } from '../setup/Shell.tsx';
import { NEW_STARS } from './store.ts';
import type { NodeRecord, NodeRef } from './store.ts';
import { settings } from '../state/settings.ts';
import { skillName, targetGroups } from './names.ts';
import { KIND, colourClosed, colourEditing, currentSpark } from './sparks.ts';
import type { ColourEdit } from './sparks.ts';
import {
  addOpen,
  allRaces,
  askClosed,
  closeAdd,
  skillsAsked,
  skillsChosen,
  umaPickClosed,
  umaPicked,
  umaPicking,
  winsClosed,
  winsEditing,
  winsText,
} from './pickers.ts';
import type { SkillsAsk, UmaPick, WinsEdit } from './pickers.ts';
import { Button } from '../ui/Button.tsx';
import { field } from '../ui/Field.ts';
import { SPARK_CELL, SPARK_NAME, sparkBar } from '../ui/Sparks.tsx';

/** the editors' dialogs (their requests read here, so one render opens one and closes another in this order), each
 *  in a render guard of its own: one that throws is gone, says so on the status line, and gives up its request (a
 *  search held for it runs), while the others still draw */
export function VetEditors() {
  return (
    <>
      <RenderGuard input={umaPicking} what="The uma picker" onFail={umaPickClosed}>
        <UmaPicker pick={umaPicking.value} />
      </RenderGuard>
      <RenderGuard input={skillsAsked} what="The skills prompt" onFail={askClosed}>
        <AskSkills asked={skillsAsked.value} />
      </RenderGuard>
      <RenderGuard input={winsEditing} what="The G1 wins window" onFail={winsClosed}>
        <WinsDialog edit={winsEditing.value} />
      </RenderGuard>
      <RenderGuard input={colourEditing} what="The colour picker" onFail={colourClosed}>
        <ColourDialog edit={colourEditing.value} />
      </RenderGuard>
    </>
  );
}

// ---------- the uma picker: every uma (or those its opener lets through), filterable; the filter is focused
function UmaPicker({ pick }: { pick: UmaPick | null }) {
  const filter = useRef<HTMLInputElement>(null);
  // (after the dialog's own focus, on each opening)
  useLayoutEffect(() => {
    if (pick?.open) filter.current?.focus();
  }, [pick]);
  return (
    <Dialog size="xl" id="pick-dlg" labelledBy="pick-title" open={!!pick?.open} onClose={umaPickClosed}>
      <DialogPanel>
        <DialogHead id="pick-title" title={pick ? pick.title : 'Which uma?'} close="Close" onClose={umaPickClosed} />
        {/* (closed, no list: an editor closed draws nothing) */}
        {pick?.open && <PickBody key={pick.n} only={pick.only} filterRef={filter} />}
      </DialogPanel>
    </Dialog>
  );
}
// an opening's filter and list (each opening starts with an empty filter)
function PickBody({ only, filterRef }: { only: UmaPick['only']; filterRef: Ref<HTMLInputElement> }) {
  const [filter, setFilter] = useState('');
  return (
    <>
      <input
        ref={filterRef}
        type="search"
        id="pick-filter"
        class={field.tap}
        placeholder="Filter by name"
        aria-label="Filter umas by name"
        value={filter}
        onInput={(e) => setFilter(e.currentTarget.value)}
      />
      <div class="grid grid-cols-[repeat(auto-fill,minmax(190px,1fr))] gap-1.5" id="pick-list">
        <UmaList filter={filter} onPick={umaPicked} only={only} />
      </div>
    </>
  );
}
// an uma's button: her portrait and her name, on the panel in a thin line (the accent's under the pointer)
const UMA =
  'border-line bg-panel text-ink text-ui hover:border-accent flex cursor-pointer items-center gap-2 rounded-md border py-1 pr-2 pl-1 text-left';
/** every uma `only` lets through, by name, the filter applied by hiding (typing builds nothing); onPick(chara id) */
export function UmaList(props: {
  filter: string;
  onPick: (chara: number) => void;
  only?: ((u: Uma) => boolean) | null;
}) {
  const { filter, onPick, only = null } = props;
  const umas = useMemo(
    () => [...gameData().umas].sort((a, b) => a.name.localeCompare(b.name)).filter((u) => !only || only(u)),
    [only],
  );
  const f = filter.trim().toLowerCase(),
    shown = (u: Uma) => !f || u.name.toLowerCase().includes(f);
  return (
    <>
      {umas.map((u) => (
        <button key={u.id} type="button" class={UMA} hidden={!shown(u)} onClick={() => onPick(u.id)}>
          <UmaAv uma={u.id} outfit={null} size={40} />
          <span>{u.name}</span>
        </button>
      ))}
      <p class="text-note text-gm-ink2" hidden={umas.some(shown)}>
        No uma matches.
      </p>
    </>
  );
}

// ---------- the skills prompt: whether to give her the target skills ("No skills" first, the default: a new uma
// carries nothing until you say so)
const allTargetsAt = (stars: number) => Object.fromEntries(targetGroups(settings.value.targets).map((g) => [g, stars]));
function AskSkills({ asked }: { asked: SkillsAsk | null }) {
  const first = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => {
    if (asked?.open) first.current?.focus();
  }, [asked]);
  const n = targetGroups(settings.value.targets).length,
    shown = asked?.open ? asked : null; // (closed, it draws nothing of the last question)
  const choices = [
    { label: 'No skills', sparks: () => ({}), primary: true },
    { label: `All ${n} at 2★`, sparks: () => allTargetsAt(2) },
    { label: `All ${n} at 3★`, sparks: () => allTargetsAt(3) },
    { label: `All ${n} at 1★`, sparks: () => allTargetsAt(1) },
    ...(shown?.hasSparks ? [{ label: 'Keep her current skills', sparks: () => null }] : []),
  ];
  return (
    <Dialog size="xl" id="ask-dlg" labelledBy="ask-title" open={!!asked?.open} onClose={askClosed}>
      <DialogPanel>
        <h3 class="text-heading font-black text-balance" id="ask-title">
          {shown ? `Add skills to ${shown.name}?` : 'Add skills?'}
        </h3>
        <p class="max-w-[75ch]" id="ask-text">
          {shown
            ? `Her sparks of your ${n} target skills decide what she passes on. A fully looped parent has all of them. You can change the stars of each afterwards.`
            : ''}
        </p>
        <div class="flex flex-wrap items-center gap-2" id="ask-btns">
          {shown &&
            choices.map(({ label, sparks, primary }) => (
              <Button
                key={label}
                ref={primary ? first : null}
                variant={primary ? 'primary' : 'plain'}
                onClick={() => skillsChosen(sparks())}
              >
                {label}
              </Button>
            ))}
        </div>
      </DialogPanel>
    </Dialog>
  );
}

// ---------- the G1 wins window: her wins on the calendar (ui/Agenda.tsx WinsEditor), each edit landing on her record
// as it is then; drawn only while open. A node that is gone (a pool veteran removed, the draft replaced) closes it, as
// it does the colour picker
function WinsDialog({ edit }: { edit: WinsEdit | null }) {
  const races = useMemo(allRaces, []),
    nd = edit?.open ? edit.at.node() : undefined,
    open = !!nd;
  return (
    <Dialog size="full" id="wins-dlg" labelledBy="wins-title" open={open} onClose={winsClosed}>
      <DialogPanel>
        <DialogHead
          id="wins-title"
          title={nd && edit ? `G1 wins: ${edit.who}` : 'G1 wins'}
          close="Done"
          onClose={winsClosed}
        >
          <span class="text-ink mr-3 ml-auto font-bold whitespace-nowrap tabular-nums" id="wins-count">
            {nd ? winsText(nd) : ''}
          </span>
        </DialogHead>
        <div id="wins-list">
          {edit && nd && (
            <WinsEditor
              key={edit.n}
              races={races}
              wins={nd.wins}
              winAt={nd.winAt || {}}
              offer={edit.opts.offer}
              info={edit.opts.info}
              onWins={(wins, winAt) => edit.at.edit(() => ({ wins, winAt }))}
            />
          )}
        </div>
      </DialogPanel>
    </Dialog>
  );
}

// ---------- the colour picker (game style): the choices as bars of the spark's colour, its stars (click one), Done (no
// remove: every veteran has one blue and one pink); Done focused. A node that is gone (the draft replaced) closes it.
function ColourDialog({ edit }: { edit: ColourEdit | null }) {
  const done = useRef<HTMLButtonElement>(null),
    nd = edit?.open ? edit.at.node() : undefined;
  useLayoutEffect(() => {
    if (edit?.open) done.current?.focus();
  }, [edit]);
  return (
    <Dialog size="md" id="col-dlg" kind="popup" labelledBy="col-title" open={!!nd} onClose={colourClosed}>
      <DialogTitle id="col-title">
        {edit ? `${KIND[edit.kind].title}${edit.opts.who ? `: ${edit.opts.who}` : ''}` : ''}
      </DialogTitle>
      {edit && nd && <Colours nd={nd} at={edit.at} kind={edit.kind} />}
      <DialogFoot>
        <DialogClose ref={done} onClick={colourClosed} label="Done" />
      </DialogFoot>
    </Dialog>
  );
}
// a star of the picker's three, the game's, lit or unlit (a 44 px button)
const STAR = {
  on: 'art-star-on-lg drop-shadow-star-lg focus-visible:outline-accent size-11 rounded-full p-0 focus-visible:outline-3 focus-visible:outline-offset-2 enabled:cursor-pointer disabled:cursor-default disabled:opacity-50',
  off: 'art-star-off-lg drop-shadow-star-lg focus-visible:outline-accent size-11 rounded-full p-0 focus-visible:outline-3 focus-visible:outline-offset-2 enabled:cursor-pointer disabled:cursor-default disabled:opacity-50',
} as const;
// what a 3-star blue or an A pink takes, under the stars
const STAT = 'text-gm-ink2 font-bold';
function Colours({ nd, at, kind }: { nd: NodeRecord; at: NodeRef; kind: ColourEdit['kind'] }) {
  const spec = KIND[kind];
  // her spark now: its group (null: none yet, so no stars to pick) and stars (0: none)
  const cur = currentSpark(nd, kind),
    curGroup = cur ? +cur[0] : null,
    curStars = cur ? cur[1] : 0;
  return (
    <DialogBody>
      <div class="grid grid-cols-2 gap-x-2 gap-y-1.5">
        {spec.groups(nd).map((g) => (
          <span key={g} class={SPARK_CELL.pick} data-spark={kind}>
            <button
              type="button"
              class={sparkBar({ look: kind, use: 'pick' })}
              data-bar=""
              aria-pressed={g === curGroup}
              onClick={() => at.edit(() => ({ [spec.field]: { [g]: curStars || NEW_STARS } }))}
            >
              <span class={SPARK_NAME.short} data-spark-name="">
                {spec.name(g)}
              </span>
            </button>
          </span>
        ))}
      </div>
      {/* biome-ignore lint/a11y/useSemanticElements: today's markup (the game's star row); a fieldset would need it restyled */}
      <div class="mt-1.5 flex justify-center gap-2" role="group" aria-label="Stars">
        {[1, 2, 3].map((k) => (
          <button
            key={k}
            type="button"
            class={k <= curStars ? STAR.on : STAR.off}
            aria-label={`${k} star${k > 1 ? 's' : ''}`}
            aria-pressed={k === curStars}
            disabled={curGroup === null}
            onClick={() => curGroup !== null && at.edit(() => ({ [spec.field]: { [curGroup]: k } }))}
          />
        ))}
      </div>
      {kind === 'blue' && (
        <p class={STAT}>A 3★ blue needs that stat at 600 or more (rank B) by the end of the career.</p>
      )}
      {kind === 'pink' && (
        <p class={STAT}>
          {nd.apt ? 'The aptitudes she ends this career at A or S.' : 'An aptitude she ended at A or S.'}
        </p>
      )}
    </DialogBody>
  );
}

// ---------- "+" beside a node's sparks: the target skills she lacks, as the game's white spark cells (her blue, pink
// and unique sparks have bars of their own). Its box (#add-pop, setup/Shell.tsx AddPop) sits in the page's layout while
// shut; open from a "+" inside a dialog, it is drawn inside that dialog (a portal: the page behind a modal dialog is out
// of reach), placed against the "+". A dropdown (ui/dropdown.ts useDropdown): a press outside it and its "+" closes it
// (another "+" moves it there); Escape closes it, not the dialog around it, back to the "+".
// a skill she lacks, as the game's white spark cell (its dot at the left)
const ADD_SKILL =
  'art-spark-add text-spark-ink inset-shadow-spark-add font-game before:art-add-dot focus-visible:outline-accent flex min-h-8.5 cursor-pointer items-center gap-2 rounded-md pr-3 pl-2 text-left font-bold before:size-4.5 before:flex-none before:rounded-full hover:brightness-105 focus-visible:outline-3 focus-visible:outline-offset-2';
export const AddList = () => <AddBody open={addOpen.value} />;
// the "+" the list is open on, as the dropdown's button: read when a click lands, so a second "+" pressed has the list
// before the click reaches the document; once closed, the last one (Escape gives it the focus back)
const opener = {
  last: null as HTMLElement | null,
  get current() {
    return addOpen.peek()?.btn ?? this.last;
  },
};
function AddBody({ open }: { open: { btn: HTMLElement; at: NodeRef } | null }) {
  const box = useRef<HTMLDivElement>(null),
    inside = useRef([box]).current;
  const sparks = (open?.at.node()?.sparks || {}) as Record<number, number>,
    missing = open ? targetGroups(settings.value.targets).filter((g) => !sparks[g]) : [],
    // (the dialog the "+" sits in, if any: where the box is drawn)
    dialog = open?.btn.closest('dialog[open]') ?? null;
  useLayoutEffect(() => {
    const host = box.current;
    if (!open || !host) return;
    opener.last = open.btn;
    const r = open.btn.getBoundingClientRect();
    if (dialog) {
      const h = dialog.getBoundingClientRect();
      host.style.left = `${Math.max(4, Math.min(r.left - h.left, h.width - 300))}px`;
      host.style.top = `${r.bottom - h.top + dialog.scrollTop + 6}px`;
    } else {
      host.style.left = `${Math.max(8, Math.min(r.left + scrollX, scrollX + innerWidth - 320))}px`;
      host.style.top = `${r.bottom + scrollY + 6}px`;
    }
    host.focus();
  }, [open, dialog]);
  useDropdown(!!open, closeAdd, opener, inside);
  const list = open && (
    <AddPop hidden={false} boxRef={box}>
      <p class="text-note text-gm-ink2">{missing.length ? 'Target skills she lacks' : 'She has every target skill.'}</p>
      <div class="grid gap-1.5">
        {missing.map((g) => (
          <button
            key={g}
            type="button"
            class={ADD_SKILL}
            onClick={() => {
              closeAdd();
              open.at.edit((now) => ({ sparks: { ...now.sparks, [g]: NEW_STARS } }));
            }}
          >
            <span>{skillName(g)}</span>
          </button>
        ))}
      </div>
    </AddPop>
  );
  // shut: the empty box in its place; open in a dialog: the box there, and none in its place (one #add-pop at a time)
  if (!list) return <AddPop boxRef={box} />;
  return dialog ? createPortal(list, dialog) : list;
}
