// ---------- the race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all ----------
// One member (the career page): no "All three" / "All four" tab; `shared` then means a parent won the race (it builds
// affinity when she is a parent herself). The popup (#sched-dlg) is ui/Dialog's on both pages; Close, Escape and the
// backdrop close it. Each opening starts on the first member, the Win table folded.
import { batch, signal, untracked } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useState } from 'preact/hooks';
import { YEARS, ROW_PLACES, TABLE_PAIRS, cellAt, shownP, slotOf, tableFigure, turnLabel, why } from '../agenda.ts';
import { GRADE_LETTER } from '../grades.ts';
import type { ScheduleCell, ScheduleRow } from '../solver.ts';
import { pickOutfit } from '../roster.ts';
import { must } from '../must.ts';
import type { DeepReadonly } from '../store.ts';
import { AgendaGrid, RaceFace, WhenBar } from './Agenda.tsx';
import { Dialog } from './Dialog.tsx';
import { around } from './arrows.ts';
import { RenderGuard } from './RenderGuard.tsx';
import { Grade, UmaAv } from './Images.tsx';

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
/** the schedule on show and its opening, which only `openSched` writes: what its render guard draws again for (each
 *  entry's), so opening it again after a failure draws it, and nothing else does */
export const schedOpened: ReadonlySignal<{ spec: SchedSpec | null; n: number }> = shown;
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
/** the popup (each entry renders it into #sched-root) */
export function ScheduleDialog() {
  const { spec, n } = shown.value;
  // (a schedule that fails to draw leaves its heading and Close: the dialog is open by then)
  return (
    <Dialog id="sched-dlg" labelledBy="sched-title" open={isOpen.value} onClose={close}>
      <div class="dlg" id="sched-body">
        <RenderGuard input={schedOpened} what="The race schedule" fallback={<SchedHead close={close} />}>
          {spec && <Schedule key={n} spec={spec} close={close} />}
        </RenderGuard>
      </div>
    </Dialog>
  );
}
const SchedHead = ({ close, children }: { close: () => void; children?: ComponentChildren }) => (
  <div class="row between dhead">
    <h3 id="sched-title">Race schedule</h3>
    {children}
    <button class="btn" type="button" onClick={close}>
      Close
    </button>
  </div>
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
    tabs: Tab[] = [...members, ...(single ? [] : ['all' as const])];
  // the arrows (and Home/End) move between the tabs and select; one tab is in the Tab order (the selected)
  const tabKeys = (e: KeyboardEvent, i: number) => {
    const to = { ArrowRight: i + 1, ArrowLeft: i - 1, Home: 0, End: tabs.length - 1 }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    const next = around(tabs, to);
    setTab(next);
    must(document.getElementById(`stab-${tabKey(next)}`), 'each tab is drawn').focus();
  };
  // the career page's figures are the community table's: its contents behind a button
  const tabled = schedule.some((row) => row.cells.some((c) => c?.tbl !== undefined));
  return (
    <>
      <SchedHead close={close}>
        {tabled && (
          <button
            class="btn sm"
            type="button"
            title="How a race’s win chance is worked out"
            aria-expanded={tableOpen}
            aria-controls="sched-wintable"
            onClick={() => setTableOpen(!tableOpen)}
          >
            Win table
          </button>
        )}
      </SchedHead>
      <WinTable hidden={!tableOpen} />
      <div class="stabs" role="tablist">
        {tabs.map((t, i) => (
          <button
            key={tabKey(t)}
            type="button"
            class="stab"
            role="tab"
            id={`stab-${tabKey(t)}`}
            aria-controls="sched-view"
            aria-selected={tab === t ? 'true' : 'false'}
            tabIndex={tab === t ? 0 : -1}
            onClick={() => setTab(t)}
            onKeyDown={(e) => tabKeys(e, i)}
          >
            {t === 'all' ? null : <UmaAv key={`${t.id}`} uma={t.id} outfit={outfitOf(t)} size={28} />}
            {t === 'all' ? (members.length === 3 ? 'All three' : 'All four') : t.short || t.name}
          </button>
        ))}
        {/* (the "All" table says shared and optional in its own captions) */}
        <span class="agkey" hidden={tab === 'all'}>
          {single ? (
            <>
              <span>
                <i class="sh" />A parent won it: builds affinity
              </span>
              <span>
                <i class="op" />
                No parent won it
              </span>
            </>
          ) : (
            <>
              <span>
                <i class="sh" />
                Shared G1: builds affinity
              </span>
              <span>
                <i class="op" />
                Optional G1: runs alone, no affinity
              </span>
            </>
          )}
        </span>
      </div>
      <div class="sview" id="sched-view" role="tabpanel" aria-labelledby={`stab-${tabKey(tab)}`}>
        {tab === 'all' ? (
          <SchedTable members={members} schedule={schedule} outfitOf={outfitOf} />
        ) : (
          <MemberAgenda members={members} schedule={schedule} m={tab} />
        )}
      </div>
    </>
  );
}
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
          <div key={turn} class={`ag-slot${row ? ' on' : ''}${row && !row.shared ? ' opt' : ''}`}>
            {row && c && <RaceCell row={row} c={c} m={m} members={members} single={single} />}
            <WhenBar slotIndex={slotIndex} cell={c} />
          </div>
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
      class={`ag-race${row.shared ? '' : ' opt'}`}
      title={`${names.join(' or ')} · ${why(c) || `win ${shownP(c)}%`}${c.goal ? ' · goal' : ''}${company}`}
    >
      {/* (an "A/B" pair shows A's banner) */}
      <RaceFace bannerName={row.race.replace(/\/.*$/, '')} label={names.join(' or ')} goal={c.goal} />
    </div>
  );
}
// the community table, as the Win table panel shows it (13 columns: it scrolls sideways inside the panel on a phone)
function WinTable({ hidden }: { hidden: boolean }) {
  return (
    <div class="wintable" id="sched-wintable" hidden={hidden}>
      <div class="wtscroll">
        <table>
          <thead>
            <tr>
              <th
                scope="col"
                title="Her distance and surface grades (S counts as A) and the races she has run in a row"
              >
                Aptitude Combination
                <br />/ Consecutive Races
              </th>
              {TABLE_PAIRS.map(([a, b]) => (
                <th key={`${a}${b}`} scope="col" title={`${GRADE_LETTER[a]} and ${GRADE_LETTER[b]}`}>
                  <Grade g={a} />
                  <Grade g={b} />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {ROW_PLACES.map((k) => (
              <tr key={k}>
                <th scope="row">{k === 6 ? '6 or more Races' : `${k} Race${k > 1 ? 's' : ''}`}</th>
                {TABLE_PAIRS.map((pair) => (
                  <td key={pair.join('')}>{`${tableFigure(pair, k)}%`}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      </div>
    </div>
  );
}
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
        <th>G1</th>
        <th class="when">When</th>
        {members.map((x) => (
          <th key={x.id} class="mh" data-k={x.k}>
            <UmaAv key={`${x.id}`} uma={x.id} outfit={outfitOf(x)} size={22} />
            <span class="hn">{x.short || x.name}</span>
          </th>
        ))}
        <th>Runners</th>
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
              <tr key={`y${r.year}`} class="year">
                <td colSpan={members.length + 3}>{YEARS[r.year]}</td>
              </tr>,
            ]
          : []),
        <tr key={`${r.when}:${r.race}`}>
          <td>{r.race}</td>
          <td class="when">{turnLabel(r.when)}</td>
          {r.cells.map((c, k) => (
            // biome-ignore lint/suspicious/noArrayIndexKey: one cell per member, in the members' order
            <MemberCell key={k} c={c} />
          ))}
          <td>{`${r.runners}/${members.length}`}</td>
        </tr>,
      ];
    });
  };
  return (
    <div class="tablewrap">
      <table class="sched">
        <caption>Shared G1s (these build affinity)</caption>
        {head}
        <tbody>{rows(shared)}</tbody>
      </table>
      {optional.length > 0 && (
        <table class="sched">
          <caption>Optional G1s (one runner, no affinity; free to run, skip freely)</caption>
          {head}
          <tbody>{rows(optional)}</tbody>
        </table>
      )}
      <p class="hint">
        {`Number = win % at that attempt (aptitude and races in a row included; hover for why). * = goal race. · = not running it. Turns not listed are rest or a goal race that isn't a G1.${schedule.some((r) => r.alts?.length) ? ' A/B = either race at the same odds; every runner must pick the same one.' : ''}`}
      </p>
    </div>
  );
}
// a member's cell: her win %, * for a goal race, its reason on hover; · when she doesn't run it
const MemberCell = ({ c }: { c: ScheduleCell | null }) =>
  c ? (
    <td class={c.goal ? 'goal' : ''} title={why(c) || undefined}>
      {`${c.goal ? '*' : ''}${shownP(c)}`}
    </td>
  ) : (
    <td class="dot">·</td>
  );
