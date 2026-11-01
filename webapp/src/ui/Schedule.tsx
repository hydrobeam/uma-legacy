// ---------- the race schedule popup: one tab per member (her races as banner cards, like the game's list), plus all ----------
// One member (the career page): no "All three" / "All four" tab; `shared` then means a parent won the race (it builds
// affinity when she is a parent herself). The popup is the template's #sched-dlg on both pages: this draws its contents
// into it (the dialog is its mount point) and opens it; Close, Escape and the backdrop close it. Each opening starts on
// the first member, the Win table folded.
import { signal, untracked } from '@preact/signals';
import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { YEARS, TABLE_PAIRS, gradeCode, shownP, slotOf, tableFigure, turnLabel, why } from '../agenda.js';
import type { ScheduleCell, ScheduleRow } from '../agenda.js';
import { pickOutfit } from '../roster.js';
import { must } from '../must.ts';
import { AgendaGrid, RaceFace, WhenBar } from './Agenda.tsx';
import { Grade, UmaAv } from './Images.tsx';

/** what the popup shows: the members (the career page: her alone), their outfits, their schedule */
export interface SchedSpec {
  members: readonly { id: number; name: string; short?: string }[];
  outfits: readonly { outfits: readonly { id: number; label: string }[] }[];
  schedule: readonly ScheduleRow[];
}
// the schedule on show, and its opening (each opening draws it afresh)
const shown = signal<{ spec: SchedSpec | null; n: number }>({ spec: null, n: 0 });
/** the race schedule window for a loop's members (the career page: her alone) */
export function openSched(spec: SchedSpec) {
  shown.value = { spec, n: shown.peek().n + 1 };
}
/** the popup's contents, rendered into the template's #sched-dlg */
export function ScheduleDialog() {
  const { spec, n } = shown.value,
    body = useRef<HTMLDivElement>(null);
  const dialog = () => must(body.current?.parentElement, 'the popup is drawn into its dialog') as HTMLDialogElement;
  // each opening (`n` moves) shows it
  useLayoutEffect(() => {
    const d = body.current?.parentElement as HTMLDialogElement | null | undefined;
    if (n && d && !d.open) d.showModal();
  }, [n]);
  return (
    <div class="dlg" id="sched-body" ref={body}>
      {spec && <Schedule key={n} spec={spec} close={() => dialog().close()} />}
    </div>
  );
}
type TabKey = number | 'all';
function Schedule({ spec: { members, outfits, schedule }, close }: { spec: SchedSpec; close: () => void }) {
  const [tab, setTab] = useState<TabKey>(0),
    [tableOpen, setTableOpen] = useState(false);
  // her outfit's portrait, picked as her result card picks it, from the settings as they are when the popup opens
  const outfitOf = (k: number) =>
    untracked(() => pickOutfit(members[k].id, [...outfits[k].outfits]).list[0]?.id ?? null);
  const single = members.length === 1,
    keys: TabKey[] = [...members.map((_, k) => k), ...(single ? [] : ['all' as const])];
  // the arrows (and Home/End) move between the tabs and select; one tab is in the Tab order (the selected)
  const tabKeys = (e: KeyboardEvent, i: number) => {
    const to = {
      ArrowRight: (i + 1) % keys.length,
      ArrowLeft: (i - 1 + keys.length) % keys.length,
      Home: 0,
      End: keys.length - 1,
    }[e.key];
    if (to === undefined) return;
    e.preventDefault();
    setTab(keys[to]);
    must(document.getElementById(`stab-${keys[to]}`), 'each tab is drawn').focus();
  };
  // the career page's figures are the community table's: its contents behind a button
  const tabled = schedule.some((row) => row.cells.some((c) => c?.tbl !== undefined));
  return (
    <>
      <div class="row between dhead">
        <h3 id="sched-title">Race schedule</h3>
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
        <button class="btn" type="button" onClick={close}>
          Close
        </button>
      </div>
      <WinTable hidden={!tableOpen} />
      <div class="stabs" role="tablist">
        {keys.map((key, i) => (
          <button
            key={key}
            type="button"
            class="stab"
            role="tab"
            id={`stab-${key}`}
            aria-controls="sched-view"
            aria-selected={tab === key ? 'true' : 'false'}
            tabIndex={tab === key ? 0 : -1}
            onClick={() => setTab(key)}
            onKeyDown={(e) => tabKeys(e, i)}
          >
            {key === 'all' ? null : (
              <UmaAv key={`${members[key].id}`} uma={members[key].id} outfit={outfitOf(key)} size={28} />
            )}
            {key === 'all'
              ? members.length === 3
                ? 'All three'
                : 'All four'
              : members[key].short || members[key].name}
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
      <div class="sview" id="sched-view" role="tabpanel" aria-labelledby={`stab-${tab}`}>
        {tab === 'all' ? (
          <SchedTable members={members} schedule={schedule} outfitOf={outfitOf} />
        ) : (
          <MemberAgenda members={members} schedule={schedule} k={tab} />
        )}
      </div>
    </>
  );
}
// member k's races on the calendar: the turn, and the win odds of the race run there (in the bar, not over the
// banner's lettering)
function MemberAgenda({
  members,
  schedule,
  k,
}: {
  members: SchedSpec['members'];
  schedule: SchedSpec['schedule'];
  k: number;
}) {
  const single = members.length === 1;
  return (
    <AgendaGrid
      slot={(y, slotIndex, turn) => {
        const row = schedule.find((x) => x.year === y && x.cells[k] && slotOf(x.when) === slotIndex);
        return (
          <div key={turn} class={`ag-slot${row ? ' on' : ''}${row && !row.shared ? ' opt' : ''}`}>
            {row && <RaceCell row={row} k={k} members={members} single={single} />}
            <WhenBar slotIndex={slotIndex} cell={row ? row.cells[k] : null} />
          </div>
        );
      }}
    />
  );
}
// one scheduled race in member k's slot, with whom she runs it (one member: which parent won it)
function RaceCell({
  row,
  k,
  members,
  single,
}: {
  row: ScheduleRow;
  k: number;
  members: SchedSpec['members'];
  single: boolean;
}) {
  const c = must(row.cells[k], 'a member who runs the race has its cell'),
    names = row.race.split('/');
  const others = members.map((_, j) => j).filter((j) => j !== k && row.cells[j]);
  const wonBy =
    single && row.parents ? row.parents.map((won, j) => (won ? `parent ${j + 1}` : '')).filter(Boolean) : [];
  const company = single
    ? wonBy.length
      ? ` · won by ${wonBy.join(' and ')}`
      : ' · no parent won it'
    : others.length
      ? ` · with ${others.map((j) => members[j].name).join(', ')}`
      : ' · alone (no affinity, optional)';
  return (
    <div
      class={`ag-race${row.shared ? '' : ' opt'}`}
      title={`${names.join(' or ')} · ${why(c) || `win ${shownP(c)}%`}${c.goal ? ' · goal' : ''}${company}`}
    >
      <RaceFace bannerName={names[0]} label={names.join(' or ')} goal={c.goal} />
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
              {TABLE_PAIRS.map((pair) => (
                <th key={pair} scope="col" title={`${pair[0]} and ${pair[1]}`}>
                  <Grade g={gradeCode(pair[0])} />
                  <Grade g={gradeCode(pair[1])} />
                </th>
              ))}
            </tr>
          </thead>
          <tbody>
            {[1, 2, 3, 4, 5, 6].map((k) => (
              <tr key={k}>
                <th scope="row">{k === 6 ? '6 or more Races' : `${k} Race${k > 1 ? 's' : ''}`}</th>
                {TABLE_PAIRS.map((pair) => (
                  <td key={pair}>{`${tableFigure(pair, k)}%`}</td>
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
  outfitOf: (k: number) => number | null;
}) {
  const { members, schedule, outfitOf } = props,
    shared = schedule.filter((r) => r.shared),
    optional = schedule.filter((r) => !r.shared);
  const head = (
    <thead>
      <tr>
        <th>G1</th>
        <th class="when">When</th>
        {members.map((x, k) => (
          <th key={x.id} class="mh" data-k={k}>
            <UmaAv key={`${x.id}`} uma={x.id} outfit={outfitOf(k)} size={22} />
            <span class="hn">{x.short || x.name}</span>
          </th>
        ))}
        <th>Runners</th>
      </tr>
    </thead>
  );
  // the rows, a year row before each year's first
  const rows = (list: readonly ScheduleRow[]) =>
    list.flatMap((r, i) => [
      ...(i === 0 || list[i - 1].year !== r.year
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
    ]);
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
