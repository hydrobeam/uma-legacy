// ---------- the veterans' editors as the game draws them ----------
// The uma picker, the skills prompt, the G1 wins window and the colour picker: one root (#vet-editors, `VetEditors`),
// their dialogs in that order, so a picker closing as the prompt opens gives the focus up first. The "+" list
// (`AddList`) draws into #add-pop, which it places by the "+" that opened it, inside that "+"'s open dialog (the page
// around a modal dialog is inert). What each shows, and its openers, are pickers.js's and sparks.js's.
import { useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import type { Ref } from 'preact';
import { gameData } from '../data.js';
import type { Uma } from '../data.js';
import { Dialog } from '../ui/Dialog.tsx';
import { RenderGuard } from '../ui/RenderGuard.tsx';
import { UmaAv } from '../ui/Images.tsx';
import { WinsEditor } from '../ui/Agenda.tsx';
import { useDropdown } from '../ui/SortMenu.tsx';
import { NEW_STARS } from './store.ts';
import type { NodeRecord, NodeRef } from './store.ts';
import { skillName, targetGroups } from './names.js';
import { KIND, colourClosed, colourEditing, currentSpark } from './sparks.js';
import type { ColourEdit } from './sparks.js';
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
} from './pickers.js';
import type { SkillsAsk, UmaPick, WinsEdit } from './pickers.js';

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
    <Dialog id="pick-dlg" labelledBy="pick-title" open={!!pick?.open} onClose={umaPickClosed}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="pick-title">{pick ? pick.title : 'Which uma?'}</h3>
          <button type="button" class="btn dclose" onClick={umaPickClosed}>
            Close
          </button>
        </div>
        <PickBody key={pick?.n ?? 0} only={pick?.only ?? null} filterRef={filter} />
      </div>
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
        placeholder="Filter by name"
        aria-label="Filter umas by name"
        value={filter}
        onInput={(e) => setFilter(e.currentTarget.value)}
      />
      <div class="umapick" id="pick-list">
        <UmaList filter={filter} onPick={umaPicked} only={only} />
      </div>
    </>
  );
}
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
        <button key={u.id} type="button" class="upick" hidden={!shown(u)} onClick={() => onPick(u.id)}>
          <UmaAv uma={u.id} outfit={null} size={40} />
          <span>{u.name}</span>
        </button>
      ))}
      <p class="hint" hidden={umas.some(shown)}>
        No uma matches.
      </p>
    </>
  );
}

// ---------- the skills prompt: whether to give her the target skills ("No skills" first, the default: a new uma
// carries nothing until you say so)
const allTargetsAt = (stars: number) => Object.fromEntries(targetGroups().map((g) => [g, stars]));
function AskSkills({ asked }: { asked: SkillsAsk | null }) {
  const first = useRef<HTMLButtonElement>(null);
  useLayoutEffect(() => {
    if (asked?.open) first.current?.focus();
  }, [asked]);
  const n = targetGroups().length,
    shown = asked?.open ? asked : null; // (closed, it draws nothing of the last question)
  const choices = [
    { label: 'No skills', sparks: () => ({}), primary: true },
    { label: `All ${n} at 2★`, sparks: () => allTargetsAt(2) },
    { label: `All ${n} at 3★`, sparks: () => allTargetsAt(3) },
    { label: `All ${n} at 1★`, sparks: () => allTargetsAt(1) },
    ...(shown?.hasSparks ? [{ label: 'Keep her current skills', sparks: () => null }] : []),
  ];
  return (
    <Dialog id="ask-dlg" labelledBy="ask-title" open={!!asked?.open} onClose={askClosed}>
      <div class="dlg">
        <h3 id="ask-title">{shown ? `Add skills to ${shown.name}?` : 'Add skills?'}</h3>
        <p id="ask-text">
          {shown
            ? `Her sparks of your ${n} target skills decide what she passes on. A fully looped parent has all of them${shown.withParents ? ', and so do her own two parents: the same choice goes on them.' : '.'} You can change the stars of each afterwards.`
            : ''}
        </p>
        <div class="row askrow" id="ask-btns">
          {shown &&
            choices.map(({ label, sparks, primary }) => (
              <button
                key={label}
                ref={primary ? first : undefined}
                type="button"
                class={`btn${primary ? ' primary' : ''}`}
                onClick={() => skillsChosen(sparks())}
              >
                {label}
              </button>
            ))}
        </div>
      </div>
    </Dialog>
  );
}

// ---------- the G1 wins window: her wins on the calendar (ui/Agenda.tsx WinsEditor), each edit landing on her record
// as it is then; drawn only while open
function WinsDialog({ edit }: { edit: WinsEdit | null }) {
  const races = useMemo(allRaces, []),
    open = !!edit?.open,
    nd: Pick<NodeRecord, 'wins' | 'winAt'> | null = open && edit ? edit.at.node() || { wins: [], winAt: {} } : null;
  return (
    <Dialog id="wins-dlg" labelledBy="wins-title" open={open} onClose={winsClosed}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="wins-title">{open && edit ? `G1 wins: ${edit.who}` : 'G1 wins'}</h3>
          <span class="num" id="wins-count">
            {nd ? winsText(nd) : ''}
          </span>
          <button type="button" class="btn dclose" onClick={winsClosed}>
            Done
          </button>
        </div>
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
      </div>
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
    <Dialog id="col-dlg" cls="gmdlg coldlg" labelledBy="col-title" open={!!nd} onClose={colourClosed}>
      <h3 class="gmdhdr" id="col-title">
        {edit ? KIND[edit.kind].title : ''}
      </h3>
      {edit && nd && <Colours nd={nd} at={edit.at} kind={edit.kind} />}
      <div class="gmdfoot">
        <button ref={done} type="button" class="btn gmclose" onClick={colourClosed}>
          Done
        </button>
      </div>
    </Dialog>
  );
}
function Colours({ nd, at, kind }: { nd: NodeRecord; at: NodeRef; kind: ColourEdit['kind'] }) {
  const spec = KIND[kind];
  // her spark now: its group (null: none yet, so no stars to pick) and stars (0: none)
  const cur = currentSpark(nd, kind),
    curGroup = cur ? +cur[0] : null,
    curStars = cur ? cur[1] : 0;
  return (
    <div class="gmdbody">
      <div class="ud udbare">
        <div class="colgrid vsparks">
          {spec.groups(nd).map((g) => (
            <span key={g} class={kind === 'pink' ? 'vsk pink' : `vsb ${kind}`}>
              <button
                type="button"
                class="vskb"
                aria-pressed={g === curGroup}
                onClick={() => at.edit(() => ({ [spec.field]: { [g]: curStars || NEW_STARS } }))}
              >
                <span class="vskn">{spec.name(g)}</span>
              </button>
            </span>
          ))}
        </div>
      </div>
      {/* biome-ignore lint/a11y/useSemanticElements: today's markup (the game's star row); a fieldset would need it restyled */}
      <div class="spstars" role="group" aria-label="Stars">
        {[1, 2, 3].map((k) => (
          <button
            key={k}
            type="button"
            class={`spstar${k <= curStars ? ' on' : ''}`}
            aria-label={`${k} star${k > 1 ? 's' : ''}`}
            aria-pressed={k === curStars}
            disabled={curGroup === null}
            onClick={() => curGroup !== null && at.edit(() => ({ [spec.field]: { [curGroup]: k } }))}
          />
        ))}
      </div>
      {kind === 'blue' && (
        <p class="gmdstat">A 3★ blue needs that stat at 600 or more (rank B) by the end of the career.</p>
      )}
      {kind === 'pink' && (
        <p class="gmdstat">
          {nd.apt
            ? 'The aptitudes she ends this career at A or S.'
            : "Her uma's aptitudes at A or S (a pink needs A or better)."}
        </p>
      )}
    </div>
  );
}

// ---------- "+" beside a node's sparks: the target skills she lacks, as the game's white spark cells (her blue, pink
// and unique sparks have bars of their own). Drawn into #add-pop (`host`), which it places under its "+": inside the
// "+"'s open dialog, placed against it. A dropdown (ui/SortMenu.tsx useDropdown): a press outside it and its "+"
// closes it (another "+" moves it there); Escape closes it, not the dialog around it, back to the "+".
export const AddList = ({ host }: { host: HTMLElement }) => <AddBody host={host} open={addOpen.value} />;
// the "+" the list is open on, as the dropdown's button: read when a click lands, so a second "+" pressed has the list
// before the click reaches the document; once closed, the last one (Escape gives it the focus back)
const opener = {
  last: null as HTMLElement | null,
  get current() {
    return addOpen.peek()?.btn ?? this.last;
  },
};
function AddBody({ host, open }: { host: HTMLElement; open: { btn: HTMLElement; at: NodeRef } | null }) {
  const inside = useRef([{ current: host }]).current;
  const sparks = (open?.at.node()?.sparks || {}) as Record<number, number>,
    missing = open ? targetGroups().filter((g) => !sparks[g]) : [];
  useLayoutEffect(() => {
    host.hidden = !open;
    if (!open) return;
    opener.last = open.btn;
    const r = open.btn.getBoundingClientRect(),
      dialog = open.btn.closest('dialog[open]');
    if ((dialog || document.body) !== host.parentElement) (dialog || document.body).appendChild(host);
    if (dialog) {
      const h = dialog.getBoundingClientRect();
      host.style.left = `${Math.max(4, Math.min(r.left - h.left, h.width - 300))}px`;
      host.style.top = `${r.bottom - h.top + dialog.scrollTop + 6}px`;
    } else {
      host.style.left = `${Math.max(8, Math.min(r.left + scrollX, scrollX + innerWidth - 320))}px`;
      host.style.top = `${r.bottom + scrollY + 6}px`;
    }
    host.focus();
  }, [open, host]);
  useDropdown(!!open, closeAdd, opener, inside);
  if (!open) return null;
  return (
    <>
      <p class="hint">{missing.length ? 'Target skills she lacks' : 'She has every target skill.'}</p>
      <div class="addlist">
        {missing.map((g) => (
          <button
            key={g}
            type="button"
            class="addsk"
            onClick={() => {
              closeAdd();
              open.at.edit((now) => ({ sparks: { ...now.sparks, [g]: NEW_STARS } }));
            }}
          >
            <span class="addn">{skillName(g)}</span>
          </button>
        ))}
      </div>
    </>
  );
}
