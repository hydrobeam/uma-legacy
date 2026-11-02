// ---------- the race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all ----------
// One member (the career page): no "All three" / "All four" tab; `shared` then means a parent won the race (it builds
// affinity when she is a parent herself). The popup (#sched-dlg) is ui/Dialog's on both pages; Close, Escape and the
// backdrop close it. Each opening starts on the first member, the Win table folded. The checks find the key by
// data-sched-key, the All table's box by data-tables, a member's column head by data-k, a turn's cell by data-when.
import { batch, signal, untracked } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import {
  YEARS,
  ROW_PLACES,
  TABLE_PAIRS,
  cellAt,
  shownP,
  slotOf,
  tableFigure,
  turnLabel,
  why,
} from '../model/agenda.ts';
import { Grade } from '../gen/solver.ts';
import type { ScheduleCell, ScheduleRow } from '../solver/solver.ts';
import { pickOutfit } from '../state/roster.ts';
import { must } from '../must.ts';
import type { DeepReadonly } from '../state/store.ts';
import { AgendaGrid, AgendaSlot, RACE_BOX, RaceFace, WhenBar } from './Agenda.tsx';
import { Dialog, DialogHead, DialogPanel } from './Dialog.tsx';
import { SegmentedControl } from './SegmentedControl.tsx';
import { RenderGuard } from './RenderGuard.tsx';
import { GradeBadge, UmaAv } from './Images.tsx';
import { Button } from './Button.tsx';
import { cx } from 'tailwind-variants/lite';

/** a member as the popup shows her: her place `k` (the index of her cell in each schedule row), her uma and her
 *  outfits that cover a target (her portrait's: loop/members.ts `Member`) */
export interface SchedMember {
  k: number;
  id: number;
  name: string;
  short?: string;
  outfits: readonly { id: number; label: string }[];
}
/** what the popup shows: the members (the career page: her alone) and their schedule */
export interface SchedSpec {
  members: readonly SchedMember[];
  schedule: DeepReadonly<ScheduleRow[]>;
}
// the schedule on show and its opening (each opening draws it afresh), and whether it is open
const shown = signal<{ spec: SchedSpec | null; n: number }>({ spec: null, n: 0 });
const isOpen = signal(false);
/** the schedule on show and its opening, which only `openSched` writes: what its render guards draw again for, so
 *  opening it again after a failure draws it, and nothing else does */
const schedOpened: ReadonlySignal<{ spec: SchedSpec | null; n: number }> = shown;
/** the race schedule window for a loop's members (the career page: her alone) */
export function openSched(spec: SchedSpec) {
  batch(() => {
    shown.value = { spec, n: shown.peek().n + 1 };
    isOpen.value = true;
  });
}
const close = () => {
  isOpen.value = false;
};
/** the popup (in #sched-root on both pages), inside its own guard: a new opening draws it again after a failure */
export const ScheduleDialog = () => (
  <RenderGuard input={schedOpened} what="The race schedule">
    <ScheduleBox />
  </RenderGuard>
);
function ScheduleBox() {
  const { spec, n } = shown.value;
  // (a schedule that fails to draw leaves its heading and Close: the dialog is open by then)
  return (
    <Dialog size="full" id="sched-dlg" labelledBy="sched-title" open={isOpen.value} onClose={close}>
      <DialogPanel id="sched-body">
        <RenderGuard input={schedOpened} what="The race schedule" fallback={<SchedHead close={close} />}>
          {spec && <Schedule key={n} spec={spec} close={close} />}
        </RenderGuard>
      </DialogPanel>
    </Dialog>
  );
}
const SchedHead = ({ close, children }: { close: () => void; children?: ComponentChildren }) => (
  <DialogHead id="sched-title" title="Race schedule" close="Close" onClose={close}>
    {children}
  </DialogHead>
);
/** a tab: a member's, or the table of all */
type Tab = SchedMember | 'all';
const tabKey = (tab: Tab) => (tab === 'all' ? 'all' : String(tab.k));
function Schedule({ spec: { members, schedule }, close }: { spec: SchedSpec; close: () => void }) {
  const [tab, setTab] = useState<Tab>(() => must(members[0], 'a schedule is of one member or more')),
    [tableOpen, setTableOpen] = useState(false);
  // her outfit's portrait, picked as her result card picks it, from the settings as they are when the popup opens
  const outfitOf = (m: SchedMember) => untracked(() => pickOutfit(m.id, m.outfits).list[0]?.id ?? null);
  const single = members.length === 1,
    tabs: Tab[] = [...members, ...(single ? [] : ['all' as const])],
    allName = members.length === 3 ? 'All three' : 'All four';
  // the career page's figures are the community table's: its contents behind a button
  const tabled = schedule.some((row) => row.cells.some((c) => c?.tbl !== undefined));
  return (
    <>
      <SchedHead close={close}>
        {tabled && (
          <Button
            size="note"
            title="How a race’s win chance is worked out"
            aria-expanded={tableOpen}
            aria-controls="sched-wintable"
            onClick={() => setTableOpen(!tableOpen)}
          >
            Win table
          </Button>
        )}
      </SchedHead>
      <WinTable hidden={!tableOpen} />
      <SegmentedControl
        kind="tab"
        shape="apart"
        value={tabKey(tab)}
        onPick={(key) =>
          setTab(
            must(
              tabs.find((t) => tabKey(t) === key),
              'a tab picked is one drawn',
            ),
          )
        }
        options={tabs.map((t) => ({
          v: tabKey(t),
          id: `stab-${tabKey(t)}`,
          controls: 'sched-view',
          body:
            t === 'all' ? (
              allName
            ) : (
              <>
                <UmaAv key={`${t.id}`} uma={t.id} outfit={outfitOf(t)} size={28} />
                {t.short || t.name}
              </>
            ),
        }))}
        trail={<SchedKey single={single} hidden={tab === 'all'} />}
      />
      <div class="grid gap-2.5" id="sched-view" role="tabpanel" aria-labelledby={`stab-${tabKey(tab)}`}>
        {tab === 'all' ? (
          <SchedTable members={members} schedule={schedule} outfitOf={outfitOf} />
        ) : (
          <MemberAgenda members={members} schedule={schedule} m={tab} />
        )}
      </div>
    </>
  );
}
// the key to a member's tab, after the tabs (the "All" table says shared and optional in its own captions): a dot in
// the badge's pink, one in its grey
const DOT = 'mr-1.25 inline-block size-3 rounded-full align-[-1px]';
const SchedKey = ({ single, hidden }: { single: boolean; hidden: boolean }) => (
  <span
    class="text-note text-ink ml-auto flex flex-wrap items-center gap-x-3.5 gap-y-1"
    hidden={hidden}
    data-sched-key=""
  >
    <span>
      <i class={cx(DOT, 'bg-sched-shared')} />
      {single ? 'A parent won it: builds affinity' : 'Shared G1: builds affinity'}
    </span>
    <span>
      <i class={cx(DOT, 'bg-sched-opt')} />
      {single ? 'No parent won it' : 'Optional G1: runs alone, no affinity'}
    </span>
  </span>
);
// member m's races on the calendar: the turn, and the win odds of the race run there (in the bar, not over the
// banner's lettering)
function MemberAgenda({
  members,
  schedule,
  m,
}: {
  members: SchedSpec['members'];
  schedule: SchedSpec['schedule'];
  m: SchedMember;
}) {
  const single = members.length === 1;
  return (
    <AgendaGrid
      slot={(y, slotIndex, turn) => {
        // (the race she runs then, and her cell in its row)
        const row = schedule.find((x) => x.year === y && cellAt(x, m.k) && slotOf(x.when) === slotIndex),
          c = row ? cellAt(row, m.k) : null;
        return (
          <AgendaSlot key={turn} on={!!row} opt={!!row && !row.shared}>
            {row && c && <RaceCell row={row} c={c} m={m} members={members} single={single} />}
            <WhenBar slotIndex={slotIndex} cell={c} />
          </AgendaSlot>
        );
      }}
    />
  );
}
// one scheduled race in member m's slot (`c`: her cell), with whom she runs it (one member: which parent won it)
function RaceCell({
  row,
  c,
  m,
  members,
  single,
}: {
  row: DeepReadonly<ScheduleRow>;
  c: DeepReadonly<ScheduleCell>;
  m: SchedMember;
  members: SchedSpec['members'];
  single: boolean;
}) {
  const names = row.race.split('/');
  const others = members.filter((o) => o !== m && row.cells[o.k]);
  const wonBy =
    single && row.parents ? row.parents.map((won, j) => (won ? `parent ${j + 1}` : '')).filter(Boolean) : [];
  const company = single
    ? wonBy.length
      ? ` · won by ${wonBy.join(' and ')}`
      : ' · no parent won it'
    : others.length
      ? ` · with ${others.map((o) => o.name).join(', ')}`
      : ' · alone (no affinity, optional)';
  return (
    <div
      class={RACE_BOX}
      data-race=""
      title={`${names.join(' or ')} · ${why(c) || `win ${shownP(c)}%`}${c.goal ? ' · goal' : ''}${company}`}
    >
      {/* (an "A/B" pair shows A's banner) */}
      <RaceFace bannerName={row.race.replace(/\/.*$/, '')} label={names.join(' or ')} goal={c.goal} />
    </div>
  );
}
// the community table as it is published, as the Win table panel shows it: a bordered grid, beige heads, the aptitude
// pairs in the game's grade letters (13 columns: it scrolls sideways inside the panel on a phone)
const WT_HEAD = 'border-gm-line bg-wt-head text-note border px-2.5 py-2 text-center font-bold whitespace-nowrap';
const WT_ROW_HEAD = 'border-gm-line bg-wt-head text-note border px-2.5 py-2 text-center font-normal whitespace-nowrap';
const WT_CELL = 'border-gm-line border px-2.5 py-2 text-center whitespace-nowrap';
function WinTable({ hidden }: { hidden: boolean }) {
  return (
    <div
      class="bg-gm-panel inset-ring-gm-edge text-note mb-2.5 w-fit max-w-full min-w-0 rounded-lg px-2.5 py-2 inset-ring"
      id="sched-wintable"
      hidden={hidden}
    >
      <div class="overflow-x-auto">
        <table class="bg-gm-cell text-gm-ink border-collapse tabular-nums">
          <thead>
            <tr>
              <th
                class={WT_HEAD}
                scope="col"
                title="Her distance and surface grades (S counts as A) and the races she has run in a row"
              >
                Aptitude Combination
                <br />/ Consecutive Races
              </th>
              {TABLE_PAIRS.map(([a, b]) => (
                <th key={`${a}${b}`} class={WT_HEAD} scope="col" title={`${Grade[a]} and ${Grade[b]}`}>
                  <GradeBadge g={a} size="head" />
                  <GradeBadge g={b} size="head" />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {ROW_PLACES.map((k) => (
              <tr key={k}>
                <th class={WT_ROW_HEAD} scope="row">
                  {k === 6 ? '6 or more Races' : `${k} Race${k > 1 ? 's' : ''}`}
                </th>
                {TABLE_PAIRS.map((pair) => (
                  <td key={pair.join('')} class={WT_CELL}>{`${tableFigure(pair, k)}%`}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
// the All table's cells, each kind's whole look (on a phone tighter, the Runners column dropped): the heads, the race
// and turn at the left, the figures at the right (a goal's in the accent, a dot where she doesn't run it in the muted
// ink), a year's row in small capitals
const CELL = {
  head: 'border-line border-b px-2 py-1 text-left whitespace-nowrap max-md:px-0.75',
  member: 'border-line border-b px-2 py-1 text-right whitespace-nowrap max-md:px-0.75',
  runnersHead: 'border-line border-b px-2 py-1 text-right whitespace-nowrap max-md:hidden',
  race: 'border-line border-b px-2 py-1 text-left max-md:px-0.75',
  when: 'border-line border-b px-2 py-1 text-left whitespace-nowrap max-md:px-0.75',
  figure: 'border-line border-b px-2 py-1 text-right whitespace-nowrap max-md:px-0.75',
  goal: 'border-line text-accent border-b px-2 py-1 text-right font-semibold whitespace-nowrap max-md:px-0.75',
  dot: 'border-line text-muted border-b px-2 py-1 text-right whitespace-nowrap max-md:px-0.75',
  runners: 'border-line border-b px-2 py-1 text-right whitespace-nowrap max-md:hidden',
  year: 'text-caption text-muted px-2 pt-2.5 pb-1 text-left font-bold tracking-widest uppercase max-md:px-0.75',
} as const;
// the "All three" / "All four" tab: every member's schedule as one table (turns written as on the member tabs, "Early
// Apr", the year being the group row; the members' columns headed by their portrait and short name, the name dropped to
// the screen reader on a narrow phone so the table fits)
function SchedTable(props: {
  members: SchedSpec['members'];
  schedule: SchedSpec['schedule'];
  outfitOf: (m: SchedMember) => number | null;
}) {
  const { members, schedule, outfitOf } = props,
    shared = schedule.filter((r) => r.shared),
    optional = schedule.filter((r) => !r.shared);
  const head = (
    <thead>
      <tr>
        <th class={CELL.head}>G1</th>
        <th class={CELL.head}>When</th>
        {members.map((x) => (
          <th key={x.id} class={CELL.member} data-k={x.k}>
            <span class="flex flex-col items-end gap-0.5 max-sm:items-center">
              <UmaAv key={`${x.id}`} uma={x.id} outfit={outfitOf(x)} size={22} />
              <span class="max-sm:sr-only">{x.short || x.name}</span>
            </span>
          </th>
        ))}
        <th class={CELL.runnersHead}>Runners</th>
      </tr>
    </thead>
  );
  // the rows, a year row before each year's first
  const rows = (list: DeepReadonly<ScheduleRow[]>) => {
    let year: number | null = null; // the year of the row before
    return list.flatMap((r) => {
      const opensYear = r.year !== year;
      year = r.year;
      return [
        ...(opensYear
          ? [
              <tr key={`y${r.year}`}>
                <td class={CELL.year} colSpan={members.length + 3}>
                  {YEARS[r.year]}
                </td>
              </tr>,
            ]
          : []),
        <tr key={`${r.when}:${r.race}`}>
          <td class={CELL.race}>{r.race}</td>
          <td class={CELL.when} data-when="">
            {turnLabel(r.when)}
          </td>
          {r.cells.map((c, k) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: one cell per member, in the members' order
            <MemberCell key={k} c={c} />
          ))}
          <td class={CELL.runners}>{`${r.runners}/${members.length}`}</td>
        </tr>,
      ];
    });
  };
  return (
    // (a shade at an edge with more table beyond it)
    <div class="art-scroll-shade mt-2 overflow-x-auto" data-tables="">
      <table class={TABLE}>
        <caption class={CAPTION}>Shared G1s (these build affinity)</caption>
        {head}
        <tbody>{rows(shared)}</tbody>
      </table>
      {optional.length > 0 && (
        <table class={TABLE}>
          <caption class={CAPTION}>Optional G1s (one runner, no affinity; free to run, skip freely)</caption>
          {head}
          <tbody>{rows(optional)}</tbody>
        </table>
      )}
      <p class="text-note text-gm-ink2">
        {`Number = win % at that attempt (aptitude and races in a row included; hover for why). * = goal race. · = not running it. Turns not listed are rest or a goal race that isn't a G1.${schedule.some((r) => r.alts?.length) ? ' A/B = either race at the same odds; every runner must pick the same one.' : ''}`}
      </p>
    </div>
  );
}
const TABLE = 'text-note max-md:text-caption w-full border-separate border-spacing-0 tabular-nums';
const CAPTION = 'caption-top py-1.5 text-left font-bold';
// a member's cell: her win %, * for a goal race, its reason on hover; · when she doesn't run it
const MemberCell = ({ c }: { c: ScheduleCell | null }) =>
  c ? (
    <td class={c.goal ? CELL.goal : CELL.figure} title={why(c) || undefined}>
      {`${c.goal ? '*' : ''}${shownP(c)}`}
    </td>
  ) : (
    <td class={CELL.dot}>·</td>
  );
