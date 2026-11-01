// ---------- the veterans (pool) ----------
// Your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog. Two tabs, as the game's
// Legacy Select: Veteran Umamusume (yours) and Guests (the ones you can borrow: your borrow list, then uma.moe's
// listings); a veteran keeps her number (her place in the whole list) in either. A wide dialog shows the chosen one
// beside the list; a narrow one opens her popup. The cards are made when the dialog first opens, then kept: an edit
// re-renders only the cards whose veteran, number, parent tab or the targets changed (300 veterans are about 1,900
// elements).
// No store yet (frontend.md step 10): the veterans are edited in place, so the page says when they, their parent
// places or the targets change (`poolChanged`), and a card compares a signature of what it shows.
import { Component, Fragment, render } from 'preact';
import type { ComponentChildren } from 'preact';
import { useLayoutEffect } from 'preact/hooks';
import { batch, computed, signal } from '@preact/signals';
import { $, $dialog, el } from '../../dom.js';
import { must } from '../../must.ts';
import { rebuild } from '../../ui/rebuild.ts';
import { wireMenu } from '../../sortmenu.js';
import { VetCell } from '../../vetcell.tsx';
import { umaName } from '../../vets.js';
import type { LineagePlan, Veteran } from '../../vets.js';

// what career.js hands the pool: the records it shows, her popup, and her view beside the list (el()-built, career.js
// `veteranView`, ported in step 8; '' when she is gone)
export type PoolProps = {
  vets: Veteran[];
  plan: LineagePlan;
  targets: () => readonly { group: number }[];
  openNode: (spec: { kind: 'v'; id: string }) => void;
  veteranView: (id: string, box: HTMLElement) => string;
};
let pool: PoolProps;
type Tab = 'own' | 'bor';
const rev = signal(0),
  vTab = signal<Tab>('own'),
  vSel = signal<string | null>(null),
  vSort = signal('tg'),
  vRev = signal(false);
// the veterans, their parent places or the targets changed
export const poolChanged = () => {
  rev.value++;
};
export const vetsTab = () => vTab.peek();
// a tab shown (its click, or an import showing whose veterans came in): its first veteran chosen, as the one chosen was
// in the other tab
export function setVTab(t: Tab) {
  for (const x of ['own', 'bor']) $(`v-tab-${x}`).setAttribute('aria-selected', String(x === t));
  $('vets').setAttribute('aria-labelledby', `v-tab-${t}`);
  batch(() => {
    vTab.value = t;
    vSel.value = null;
  });
}
const wideVets = () => matchMedia('(min-width: 760px)').matches;

// the list's orders: the order added (uma.moe's listings in uma.moe's order), how many targets she carries (the
// default; ties in the order added), her rank; each starts as the menu says and picking the chosen one again reverses
// it (as the umas' and cards' sort menus)
const VSORTS: Record<string, [string, string, (v: Veteran, i: number) => number]> = {
  no: ['Added', 'In the order they were added to this page (uma.moe’s: in its own order)', (_v, i) => -i],
  tg: [
    'Targets',
    'The most targets she carries first',
    (v) => pool.targets().filter((t) => (v.sparks?.[t.group] ?? 0) > 0).length,
  ],
  rank: ['Rank', 'The highest career rank first', (v) => v.rank || 0],
};
function paintVSort() {
  const arrow = vRev.peek() ? '↑' : '↓',
    b = $('vs'),
    [name, hint] = must(VSORTS[vSort.peek()], 'the order is one of VSORTS');
  b.textContent = `${name} ${arrow}`;
  b.title = hint + (vRev.peek() ? ', reversed' : '');
  b.setAttribute('aria-label', `${b.textContent}, order veterans`);
  for (const x of $('vs-menu').querySelectorAll('button')) {
    const on = x.dataset.k === vSort.peek();
    x.setAttribute('aria-checked', String(on));
    const [itemName, itemHint] = must(VSORTS[must(x.dataset.k, 'a menu item names its key')], 'an item of VSORTS');
    x.textContent = itemName + (on ? ` ${arrow}` : '');
    x.title = on ? 'Click to reverse' : itemHint;
  }
}

// the tab's veterans with their numbers, in the chosen order
const shown = computed(() => {
  void rev.value;
  const by = must(VSORTS[vSort.value], 'the order is one of VSORTS')[2],
    sign = vRev.value ? -1 : 1,
    bor = vTab.value === 'bor';
  return pool.vets
    .map((v, i): [Veteran, number] => [v, i])
    .filter(([v]) => !!v.borrowed === bor)
    .sort(([a, i], [b, j]) => sign * (by(b, j) - by(a, i)));
});
// the one beside the list: the chosen one while she is in this tab, else the first shown
const selId = computed(() => {
  const list = shown.value,
    id = vSel.value;
  return list.some(([v]) => v.id === id) ? id : (list[0]?.[0].id ?? null);
});

// a component re-rendered only when one of its props changes, for one that reads no signal (@preact/signals spares
// those that do); preact/compat's memo would bring in all of compat
function memo<P extends object>(view: (props: P) => ComponentChildren) {
  return class extends Component<P> {
    override shouldComponentUpdate(next: P) {
      for (const k in next) if (next[k] !== this.props[k]) return true;
      return false;
    }
    override render(props: P) {
      return view(props);
    }
  };
}
// her card: her number (an uma.moe one: her trainer) under her, a "Parent 1" tab when she is one of the next career's
// parents; a click shows her beside the list (a wide dialog) or in her popup. `sig` is everything it shows: she is
// edited in place, so her record's identity says nothing
type CardProps = { v: Veteran; i: number; k: number; tg: number[]; sig: string; sel: boolean; wide: boolean };
const PoolCard = memo(({ v, i, k, tg, sel, wide }: CardProps) => (
  <VetCell
    v={v}
    id={`vet-${i}`}
    k={`node:v${v.id}`}
    sel={sel}
    current={sel && wide}
    name={v.lender ? v.lender.name || 'uma.moe' : `#${i + 1}`}
    label={`Veteran #${i + 1}: ${umaName(v.chara)}, ${tg.filter((g) => v.sparks?.[g]).length} of ${tg.length} target skills. Her sparks`}
    onClick={() => {
      if (wideVets()) vSel.value = v.id;
      else pool.openNode({ kind: 'v', id: v.id });
    }}
    tab={k >= 0 ? `Parent ${k + 1}` : undefined}
  />
));
const EMPTY = {
  own: "None yet. Import umadump's trained_chara_data.json, or make one with Choose parent.",
  bor: "None yet. Import umadump's rental_chara_data.json, mark a veteran Guest, or Borrow from uma.moe beside Next career.",
};
function PoolList() {
  const list = shown.value,
    sel = selId.value,
    tab = vTab.value,
    wide = wideVets(),
    targets = pool.targets(),
    tg = targets.map((t) => t.group);
  if (!list.length) return <p class="hint">{EMPTY[tab]}</p>;
  // Guests: your borrow list, then uma.moe's listings, each under its heading
  const groups: [string, [Veteran, number][]][] =
    tab === 'own'
      ? [['', list]]
      : [
          ['Borrow list', list.filter(([v]) => v.src !== 'umamoe')],
          ['uma.moe', list.filter(([v]) => v.src === 'umamoe')],
        ];
  return groups
    .filter(([, l]) => l.length)
    .map(([head, l]) => (
      <Fragment key={head}>
        {head ? <p class="vsec">{head}</p> : null}
        {l.map(([v, i]) => {
          const k = pool.plan.parents.indexOf(v.id);
          return (
            <PoolCard
              key={v.id}
              v={v}
              i={i}
              k={k}
              tg={tg}
              sig={JSON.stringify([v, i, k, targets])}
              sel={v.id === sel}
              wide={wide}
            />
          );
        })}
      </Fragment>
    ));
}
// the chosen one beside the list (a wide dialog): her view, rebuilt in the pane the template keeps
function Pane({ host }: { host: HTMLElement }) {
  const id = selId.value;
  void rev.value;
  useLayoutEffect(() =>
    rebuild(host, () => {
      host.textContent = '';
      if (id && wideVets()) {
        const h = el('h4', 'vptitle');
        host.appendChild(h);
        h.textContent = pool.veteranView(id, host);
      }
    }),
  );
  return null;
}

// The pool's wiring, from the career page's start: the tabs, the order menu, and the cards made on the first open
export function initPool(props: PoolProps) {
  pool = props;
  const dialog = $dialog('set-vets'),
    list = $('vets'),
    pane = $('v-pane');
  for (const t of ['own', 'bor'] as const) $(`v-tab-${t}`).onclick = () => setVTab(t);
  for (const k of Object.keys(VSORTS)) {
    const x = el('button', '', must(VSORTS[k], 'a key of VSORTS')[0]);
    x.type = 'button';
    x.setAttribute('role', 'menuitemradio');
    x.dataset.k = k;
    $('vs-menu').appendChild(x);
  }
  wireMenu('vs', (k) => {
    batch(() => {
      vRev.value = k === vSort.peek() && !vRev.peek();
      vSort.value = k;
    });
    paintVSort();
  });
  paintVSort();
  let made = false;
  $('v-toggle').onclick = () => {
    if (!made) {
      made = true;
      render(<PoolList />, list);
      render(<Pane host={pane} />, pane);
    }
    dialog.showModal();
  };
}
