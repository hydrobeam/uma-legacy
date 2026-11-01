// The veterans dialog (Veteran Umamusume, VetsDialog): its head (PoolHead, in #v-head: the two tabs and the order
// menu, ui/SortMenu.tsx), the two tab lists and the pane beside them (PoolBody, in #v-body), and its tools. Both lists
// stay mounted once the dialog has opened, the one not on show `hidden`, so a tab switch builds nothing; the list on
// show is `#vets` (its id moves with the tab); the pane is drawn only while the dialog is open (closed, an edit
// elsewhere redraws no pane). A card is drawn again only when her record, her number, her parent place or the targets
// change (her card reads the chosen one through a computed of its own, so a selection draws two cards).
import { useComputed } from '@preact/signals';
import type { RefObject } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { settings } from '../../settings.ts';
import type { Target } from '../../settings.ts';
import { vets, places, clearVets } from '../../veterans/store.ts';
import type { VetRecord } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import { SortMenu } from '../../ui/SortMenu.tsx';
import { VetCell } from '../../ui/VetCell.tsx';
import { idxOf } from '../current.ts';
import { openNode } from './dialogs.ts';
import { VetPane } from './NodeDialog.tsx';
import { Guard } from './Guard.tsx';
import { openPaste } from './Io.tsx';
import { importText } from '../io.ts';
import { Dialog } from '../../ui/Dialog.tsx';
import { must } from '../../must.ts';
import {
  VET_SORTS,
  VSORTS,
  choose,
  inTab,
  lists,
  opened,
  order,
  poolClosed,
  poolOpen,
  poolTab as tab,
  refocusAsked,
  sayVets,
  selected,
  setOrder,
  setVTab,
  vetsNote,
} from './poolState.ts';
import type { Tab } from './poolState.ts';

const wideVets = () => matchMedia('(min-width: 760px)').matches;

// ---------- the views
/** the dialog's head: the two tabs, each with its count, and the order menu (@public: test/unit/careerviews.test.mjs
 * draws it from an esbuild bundle written as a string, which knip does not read) */
export function PoolHead() {
  const { by, reversed } = order.value;
  return (
    <>
      <div class="seg" role="tablist" aria-label="Whose veterans" id="v-tabs">
        <PoolTabs />
      </div>
      <SortMenu
        id="vs"
        cls="btn sm"
        what="order veterans"
        items={SORT_ITEMS}
        value={by}
        arrow={reversed ? '↑' : '↓'}
        title={VSORTS[by][1] + (reversed ? ', reversed' : '')}
        onPick={setOrder}
      />
    </>
  );
}
const SORT_ITEMS = VET_SORTS.map((k) => ({ k, label: VSORTS[k][0], hint: VSORTS[k][1] }));
/** the two tabs, each with its count */
function PoolTabs() {
  const all = vets.value,
    guests = all.filter((v) => v.borrowed).length;
  const tabs = [
    ['own', `Veteran Umamusume · ${all.length - guests}`, undefined],
    ['bor', `Guests · ${guests}`, "Veterans you can borrow (one per career): your borrow list and uma.moe's"],
  ] as const;
  return (
    <>
      {tabs.map(([t, label, title]) => (
        <button
          key={t}
          type="button"
          role="tab"
          id={`v-tab-${t}`}
          aria-selected={tab.value === t}
          aria-controls={tab.value === t ? 'vets' : `vets-${t}`}
          title={title}
          onClick={() => setVTab(t)}
        >
          {label}
        </button>
      ))}
    </>
  );
}
interface CardProps {
  v: VetRecord;
  /** her number less one (her place in the pool) */
  i: number;
  /** her parent place (-1: none) */
  k: number;
  targets: readonly Target[];
  /** the list she is in: where her card is found again to take the focus back */
  grid: RefObject<HTMLDivElement | null>;
}
// her card, as the game's legacy list draws her: her number (an uma.moe one: her trainer) under her, a "Parent 1" tab
// when she is one of the next career's parents. A click shows her beside the list (a wide dialog) or opens her popup
// (her sparks editable, Yours / Guest, Remove). It reads the chosen one through its own computed, so it draws again
// only when its props change or it is chosen or no longer chosen.
function PoolCard({ v, i, k, targets, grid }: CardProps) {
  const sel = useComputed(() => selected.value === v.id).value,
    have = targets.filter((t) => (v.sparks as Record<number, number>)?.[t.group]).length,
    id = v.id;
  return (
    <VetCell
      v={v}
      id={`vet-${i}`}
      k={`node:v${id}`}
      sel={sel}
      current={sel && wideVets()}
      name={v.lender ? v.lender.name || 'uma.moe' : `#${i + 1}`}
      label={`Veteran #${i + 1}: ${umaName(v.chara)}, ${have} of ${targets.length} target skills. Her sparks`}
      tab={k >= 0 ? `Parent ${k + 1}` : undefined}
      onClick={() => {
        if (!wideVets())
          openNode({ kind: 'v', id }, () => grid.current?.querySelector<HTMLElement>(`#vet-${idxOf(id)}`)?.focus());
        else if (selected.peek() !== id) choose(id);
      }}
    />
  );
}
const NONE = {
  own: "None yet. Import umadump's trained_chara_data.json, or make one with Choose parent.",
  bor: "None yet. Import umadump's rental_chara_data.json, mark a veteran Guest, or Borrow from uma.moe beside Next career.",
};
/** one tab's list (no cards until the dialog has opened): Guests is your borrow list, then uma.moe's listings, each
 *  under its heading */
function PoolList({ t }: { t: Tab }) {
  const on = tab.value === t,
    list = lists[t].value,
    parents = places.value.parents,
    targets = settings.value.targets,
    grid = useRef<HTMLDivElement>(null),
    ask = refocusAsked.value;
  // a removal asked for the focus: the card chosen now takes it (by her number, which her card's id carries)
  // biome-ignore lint/correctness/useExhaustiveDependencies: once per ask (`ask` is a module signal's value, which Biome reads as constant)
  useLayoutEffect(() => {
    const id = selected.peek();
    if (ask && on && id) grid.current?.querySelector<HTMLElement>(`#vet-${idxOf(id)}`)?.focus();
  }, [ask]);
  const groups: [string, (readonly [VetRecord, number])[]][] =
    t === 'own'
      ? [['', list]]
      : [
          ['Borrow list', list.filter(([v]) => v.src !== 'umamoe')],
          ['uma.moe', list.filter(([v]) => v.src === 'umamoe')],
        ];
  return (
    <div
      ref={grid}
      class="umgrid vets"
      id={on ? 'vets' : `vets-${t}`}
      role="tabpanel"
      aria-labelledby={`v-tab-${t}`}
      hidden={!on}
    >
      {!opened.value ? null : list.length ? (
        groups.flatMap(([head, rows]) =>
          rows.length
            ? [
                head && (
                  <p key={head} class="vsec">
                    {head}
                  </p>
                ),
                ...rows.map(([v, i]) => (
                  <PoolCard key={v.id} v={v} i={i} k={parents.indexOf(v.id)} targets={targets} grid={grid} />
                )),
              ]
            : [],
        )
      ) : (
        <p class="hint">{NONE[t]}</p>
      )}
    </div>
  );
}
/** the lists and the pane beside them (a wide dialog: the one chosen; nothing while the dialog is closed; @public: as
 * PoolHead) */
export function PoolBody() {
  const id = poolOpen.value ? selected.value : null;
  return (
    <>
      <PoolList t="own" />
      <PoolList t="bor" />
      <section class="vpane" id="v-pane" aria-label="The veteran chosen">
        {id && wideVets() && <VetPane id={id} />}
      </section>
    </>
  );
}
/** The veterans dialog (Veteran Umamusume, ui/Dialog: its button opens it, `openPool`): the head and the body, each
 *  behind its guard, and the tools under them: Import umadump file, Paste, Remove all and what the last of them did
 *  (`vetsNote`). The career entry renders it into #vets-root. */
export function VetsDialog() {
  return (
    <Dialog id="set-vets" cls="setdlg vetdlg" labelledBy="h-vets" open={poolOpen.value} onClose={poolClosed}>
      <div class="dlg">
        <div class="row between dhead">
          <h3 id="h-vets">Veteran Umamusume</h3>
          <button class="btn dclose" type="button" onClick={poolClosed}>
            Done
          </button>
        </div>
        <div class="vhead" id="v-head">
          <Guard what="The veterans' tabs" also={tab}>
            <PoolHead />
          </Guard>
        </div>
        <div class="vbody" id="v-body">
          <Guard what="The veterans" also={selected}>
            <PoolBody />
          </Guard>
        </div>
        <div class="row vtools">
          <label
            class="btn filebtn"
            title="umadump (github.com/Werseter/umadump) reads them from the game: trained_chara_data.json for yours, rental_chara_data.json for your borrow list"
          >
            Import umadump file
            <input type="file" id="v-file" accept=".json,application/json" onChange={importFile} />
          </label>
          <button class="btn" type="button" id="v-paste" onClick={openPaste}>
            Paste
          </button>
          <button class="btn" type="button" id="v-clear" onClick={removeAll}>
            Remove all
          </button>
          <span class="hint" id="v-msg" role="status">
            {vetsNote.value}
          </span>
        </div>
      </div>
    </Dialog>
  );
}
// a umadump file chosen: imported, and the input emptied (the same file can be chosen again)
async function importFile(e: Event) {
  const input = e.currentTarget as HTMLInputElement;
  const f = must(input.files, 'a file input has its file list')[0];
  if (f) importText(await f.text());
  input.value = '';
}
// Remove all: the open tab's veterans (uma.moe's listings stay, unless one is a chosen parent: they are its answer for
// the targets, not yours)
function removeAll() {
  const parents = places.value.parents,
    gone = (v: VetRecord) => inTab(v) && (v.src !== 'umamoe' || parents.includes(v.id)),
    n = vets.value.filter(gone).length,
    what = tab.peek() === 'own' ? `${n} of your veterans` : `${n} borrowed veteran${n === 1 ? '' : 's'}`;
  if (n && confirm(`Remove all ${what}?`)) {
    clearVets(gone);
    sayVets(`Removed ${what}.`);
  }
}
