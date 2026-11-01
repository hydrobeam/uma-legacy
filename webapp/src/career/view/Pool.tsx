// The veterans dialog (Veteran Umamusume): its head (PoolHead, into #v-head: the two tabs and the order menu,
// ui/SortMenu.tsx), the two tab lists and the pane beside them (PoolBody, into #v-body). Both lists stay mounted once the
// dialog has opened, the one not on show `hidden`, so a tab switch builds nothing; the list on show is `#vets` (its id
// moves with the tab); the pane is drawn only while the dialog is open (closed, an edit elsewhere redraws no pane). A
// card is drawn again only when her record, her number, her parent place or the targets change (her card reads the
// chosen one through a computed of its own, so a selection draws two cards).
import { useComputed } from '@preact/signals';
import { settings } from '../../settings.ts';
import type { Target } from '../../settings.ts';
import { vets, places } from '../../veterans/store.ts';
import type { VetRecord } from '../../veterans/store.ts';
import { umaName } from '../../veterans/names.ts';
import { SortMenu } from '../../ui/SortMenu.tsx';
import { VetCell } from '../../ui/VetCell.tsx';
import { idxOf } from '../job.ts';
import { openNode } from './dialogs.ts';
import { VetPane } from './NodeDialog.tsx';
import {
  VSORTS,
  choose,
  lists,
  opened,
  order,
  poolOpen,
  poolTab as tab,
  selected,
  setOrder,
  setVTab,
} from './poolState.ts';
import type { Tab } from './poolState.ts';

const wideVets = () => matchMedia('(min-width: 760px)').matches;

// ---------- the views
/** the dialog's head: the two tabs, each with its count, and the order menu */
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
const SORT_ITEMS = Object.entries(VSORTS).map(([k, [label, hint]]) => ({ k, label, hint }));
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
}
// her card, as the game's legacy list draws her: her number (an uma.moe one: her trainer) under her, a "Parent 1" tab
// when she is one of the next career's parents. A click shows her beside the list (a wide dialog) or opens her popup
// (her sparks editable, Yours / Guest, Remove). It reads the chosen one through its own computed, so it draws again
// only when its props change or it is chosen or no longer chosen.
function PoolCard({ v, i, k, targets }: CardProps) {
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
        if (!wideVets()) openNode({ kind: 'v', id }, () => document.getElementById(`vet-${idxOf(id)}`)?.focus());
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
    targets = settings.value.targets;
  const groups: [string, (readonly [VetRecord, number])[]][] =
    t === 'own'
      ? [['', list]]
      : [
          ['Borrow list', list.filter(([v]) => v.src !== 'umamoe')],
          ['uma.moe', list.filter(([v]) => v.src === 'umamoe')],
        ];
  return (
    <div class="umgrid vets" id={on ? 'vets' : `vets-${t}`} role="tabpanel" aria-labelledby={`v-tab-${t}`} hidden={!on}>
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
                  <PoolCard key={v.id} v={v} i={i} k={parents.indexOf(v.id)} targets={targets} />
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
/** the lists and the pane beside them (a wide dialog: the one chosen; nothing while the dialog is closed) */
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
