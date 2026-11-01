// ---------- the game's race calendar (Scheduled Races): the schedule window's and the G1 wins editor's ----------
// Three year columns of half-month slots, Junior's from July (the career starts in Junior Year, early July). On the
// planner pages (html.game) all three years side by side, each 4 x 6 slots, Junior's first three rows the game's grey
// cells for April to June (labelled as the game labels them, each half-month twice). agenda.ts holds the logic.
import type { Ref, VNode } from 'preact';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import { MONTHS, YEARS, bannerSrc, shownP, slotLabel, turnName, why } from '../agenda.ts';
import type { WinInfo } from '../agenda.ts';
import type { ScheduleCell } from '../solver.ts';
import { must } from '../must.ts';
import { Button } from './Button.tsx';
import { cx } from 'tailwind-variants/lite';
// the three years' columns (y0, y1, y2: their colours)
const YEAR_CLASS = ['y0', 'y1', 'y2'];

// April to June before the career, 4 cells a month (Early, Early, Late, Late)
const PRE = Array.from({ length: 12 }, (_, i) => (i % 4 < 2 ? 'Early ' : 'Late ') + MONTHS[3 + (i >> 2)]);
const SLOTS = Array.from({ length: 24 }, (_, i) => i);
/** the calendar: `slot(year, slot index 0-23, turn)` draws each slot of the career (a keyed `div.ag-slot`) */
export function AgendaGrid({ slot }: { slot: (year: number, slotIndex: number, turn: number) => VNode }) {
  const game = document.documentElement.classList.contains('game');
  return (
    <div class={cx('agenda', game ? 'game' : '')}>
      {YEARS.map((year, y) => (
        <section key={year} class={cx('ag-year', YEAR_CLASS[y])}>
          <h4>{`${year} Year`}</h4>
          <div class="ag-grid">
            {game &&
              y === 0 &&
              PRE.map((label, i) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: the twelve grey cells, fixed (each label twice)
                <div key={`pre${i}`} class="ag-slot pre">
                  <div class="ag-when">
                    <span>{label}</span>
                  </div>
                </div>
              ))}
            {/* Junior January to June, before the career: off html.game, one note fills it */}
            {!game && y === 0 && <div class="ag-pre">The career starts in Junior Year, early July</div>}
            {SLOTS.filter((s) => y > 0 || s >= 12).map((s) => slot(y, s, y * 24 + s))}
          </div>
        </section>
      ))}
    </div>
  );
}
/** the bar at the foot of a slot: "Early Apr", and the win odds of the race run there */
export function WhenBar({ slotIndex, cell }: { slotIndex: number; cell: ScheduleCell | null }) {
  return (
    <div class="ag-when">
      <span>{slotLabel(slotIndex)}</span>
      {cell && <b title={why(cell)}>{`${shownP(cell)}% win`}</b>}
    </div>
  );
}
/** a race in its slot (the schedule window's cell, and a win in the G1 wins editor): banner (by `bannerName`), name,
 *  Goal badge; the element around them is the caller's (`div.ag-race` or `button.ag-race`) */
export const RaceFace = ({ bannerName, label, goal }: { bannerName: string; label: string; goal: boolean }) => {
  const src = bannerSrc(bannerName);
  return (
    <>
      {src && <img class="banner" alt="" loading="lazy" src={src} />}
      <div class="ag-nm">{label}</div>
      {goal && <span class="ag-goal">Goal</span>}
    </>
  );
};

// ---------- the G1 wins editor (a pool veteran's, and the veteran analysis' draft), on the same calendar ----------
// Each win sits in one slot as the schedule window's cell; a click removes it and the slot is empty. An empty slot where
// a G1 is held is a button: a menu of the races held then, and a pick marks one won there. A race held in Classic and
// Senior is one race (one win saddle): it shows once, at the turn picked for it (`winAt`, kept by the page for display
// only), else the turn her schedule runs it (`info`), else its first turn free of another win (an imported win has no
// turn).
export interface WinsEditorProps {
  /** every G1: its race index, name and the turns it is held */
  races: readonly { ix: number; name: string; turns: readonly number[] }[];
  /** the race indices she won */
  wins: readonly number[];
  /** race index -> the turn its win shows at */
  winAt: Readonly<Record<number, number>>;
  /** the races the menu lists at a turn (none: every race held then) */
  offer?: ((turn: number) => number[] | null) | null | undefined;
  /** the race in her schedule */
  info?: ((ix: number) => WinInfo | null) | null | undefined;
  /** an edit: her wins (sorted, no repeats) and where each shows, both new */
  onWins: (wins: number[], winAt: Record<number, number>) => void;
}
export function WinsEditor({ races, wins, winAt, offer, info: infoOf, onWins }: WinsEditorProps) {
  const [menuFor, setMenuFor] = useState<number | null>(null), // the turn whose menu is open
    box = useRef<HTMLDivElement>(null),
    menu = useRef<HTMLDivElement>(null),
    focusKey = useRef<string | null>(null); // the control (data-k) to focus once an edit is drawn
  const { raceName, raceTurns, heldAt } = useMemo(() => racesBy(races), [races]);
  const info = (ix: number) => infoOf?.(ix) || null;
  // where each win shows: byTurn, turn -> [ix] (two wins on one turn only when the record says so; they share the
  // slot); offCalendar, the wins of races with no turn; shownAt, ix -> its turn
  const byTurn = new Map<number, number[]>(),
    offCalendar: number[] = [],
    shownAt = new Map<number, number>();
  // wins placed by the page first, then those her schedule runs, then the rest (each in race order)
  // (a race the page placed no win of has no `winAt`)
  const placed = (ix: number) => {
    const at = winAt[ix];
    return at !== undefined && (raceTurns.get(ix) || []).includes(at);
  };
  const rank = (ix: number) => (placed(ix) ? 0 : info(ix)?.turn !== undefined ? 1 : 2);
  for (const ix of [...wins].sort((a, b) => rank(a) - rank(b) || a - b)) {
    const turns = raceTurns.get(ix) || [];
    if (!turns.length) {
      offCalendar.push(ix);
      continue;
    }
    // (a race not placed by the page, or not in her schedule, has no turn there)
    const preferred = [winAt[ix], info(ix)?.turn, ...turns].flatMap((t) =>
      t !== undefined && turns.includes(t) ? [t] : [],
    );
    const t = preferred.find((at) => !byTurn.has(at)) ?? must(preferred[0], 'a race held on turns prefers one of them');
    byTurn.set(t, [...(byTurn.get(t) || []), ix]);
    shownAt.set(ix, t);
  }
  const edit = (next: number[], at: Record<number, number>) => {
    setMenuFor(null);
    onWins(
      [...new Set(next)].sort((a, b) => a - b),
      at,
    );
  };
  const remove = (ix: number, focus: string | null) => {
    const { [ix]: _, ...at } = winAt;
    focusKey.current = focus;
    edit(
      wins.filter((x) => x !== ix),
      at,
    );
  };
  const pick = (ix: number, turn: number) => {
    focusKey.current = `win:${ix}`;
    edit([...wins, ix], { ...winAt, [ix]: turn });
  };
  // after an edit, the control of its key: a removed win's slot "+", a picked race's win (else the first button)
  useLayoutEffect(() => {
    const key = focusKey.current;
    focusKey.current = null;
    if (key)
      (
        box.current?.querySelector<HTMLElement>(`[data-k="${CSS.escape(key)}"]`) || box.current?.querySelector('button')
      )?.focus();
  });
  // the menu opens by its slot, inside the editor (it scrolls with the calendar), clamped to the editor's width; below
  // the slot, or above it when it would run past the editor's foot (no scrolling to reach it); its first race focused
  useLayoutEffect(() => {
    const m = menu.current,
      b = box.current,
      opener = b && menuFor !== null ? b.querySelector<HTMLElement>(`[data-k="add:${menuFor}"]`) : null;
    if (!m || !b || !opener) return;
    const boxRect = b.getBoundingClientRect(),
      slotRect = must(opener.closest('.ag-slot'), 'a "+" is in its slot').getBoundingClientRect(),
      width = m.offsetWidth;
    m.style.left = `${Math.max(0, Math.min(slotRect.left - boxRect.left, b.clientWidth - width))}px`;
    const below = slotRect.bottom - boxRect.top + 4,
      above = slotRect.top - boxRect.top - m.offsetHeight - 4;
    m.style.top = `${below + m.offsetHeight > b.clientHeight && above >= 0 ? above : below}px`;
    m.querySelector<HTMLElement>('button')?.focus(); // (the first race: the menu holds its hint and the races)
  }, [menuFor]);
  // closed: a press outside the menu and its "+" closes it (the focus stays where the press puts it)
  useEffect(() => {
    if (menuFor === null) return;
    const outside = (e: PointerEvent) => {
      const opener = box.current?.querySelector(`[data-k="add:${menuFor}"]`),
        target = e.target as Node;
      if (!menu.current?.contains(target) && !opener?.contains(target)) setMenuFor(null);
    };
    document.addEventListener('pointerdown', outside, true);
    return () => document.removeEventListener('pointerdown', outside, true);
  }, [menuFor]);
  // closed with the keyboard: back to its "+"
  const closeBack = () => {
    focusKey.current = `add:${menuFor}`;
    setMenuFor(null);
  };
  return (
    <div class="winsed" ref={box}>
      <AgendaGrid
        slot={(_y, slotIndex, turn) => {
          const here = byTurn.get(turn) || [],
            offered = here.length ? [] : (offer ? offer(turn) : heldAt.get(turn)) || [],
            [first, ...more] = here,
            only = first !== undefined && !more.length ? info(first) : null;
          return (
            <div
              key={turn}
              class={cx(
                'ag-slot',
                here.length ? 'on' : '',
                here.length > 1 ? 'many' : '',
                offered.length ? 'free' : '',
              )}
            >
              {here.map((ix) => (
                <WinButton
                  key={ix}
                  ix={ix}
                  name={must(raceName.get(ix), 'a win is a race of the data')}
                  scheduled={info(ix)}
                  turn={turn}
                  onRemove={() => remove(ix, `add:${turn}`)}
                />
              ))}
              {offered.length > 0 && (
                <button
                  type="button"
                  class="ag-add"
                  data-k={`add:${turn}`}
                  aria-label={`${turnName(turn)}: add a G1 win`}
                  title="Add a G1 win"
                  aria-haspopup="dialog"
                  aria-expanded={menuFor === turn}
                  onClick={() => setMenuFor(menuFor === turn ? null : turn)}
                >
                  <span aria-hidden="true">+</span>
                </button>
              )}
              <WhenBar slotIndex={slotIndex} cell={only && only.turn === turn ? (only.cell ?? null) : null} />
            </div>
          );
        }}
      />
      {offCalendar.length > 0 && (
        // a win with no turn on the calendar (not in the race data): listed, still removable
        <p class="hint">
          Not on the calendar:{' '}
          {offCalendar.map((ix) => {
            const name = raceName.get(ix) || `Race ${ix}`;
            return (
              <Button
                key={ix}
                data-k={`win:${ix}`}
                aria-label={`${name}, won: remove`}
                onClick={() => remove(ix, null)}
              >
                {`${name} ×`}
              </Button>
            );
          })}
        </p>
      )}
      <WinsMenu
        menuRef={menu}
        turn={menuFor}
        offered={menuFor === null ? [] : (offer ? offer(menuFor) : heldAt.get(menuFor)) || []}
        raceName={raceName}
        info={info}
        shownAt={shownAt}
        onPick={pick}
        onBack={closeBack}
      />
    </div>
  );
}
// the races by index, and the races held at each turn
function racesBy(races: WinsEditorProps['races']) {
  const raceName = new Map(races.map((r) => [r.ix, r.name])),
    raceTurns = new Map(races.map((r) => [r.ix, r.turns])),
    heldAt = new Map<number, number[]>();
  for (const r of races) for (const t of r.turns) heldAt.set(t, [...(heldAt.get(t) || []), r.ix]);
  return { raceName, raceTurns, heldAt };
}
/** a win in its slot: a button that removes it */
function WinButton(props: { ix: number; name: string; scheduled: WinInfo | null; turn: number; onRemove: () => void }) {
  const { ix, name, scheduled, turn, onRemove } = props,
    cellRun = scheduled && scheduled.turn === turn ? must(scheduled.cell, 'a race in her schedule has its cell') : null,
    odds = cellRun ? ` · ${why(cellRun) || `${shownP(cellRun)}% win`}` : '';
  return (
    <button
      type="button"
      class="ag-race wbtn"
      data-k={`win:${ix}`}
      data-ix={ix}
      aria-label={`${name}, won: remove`}
      title={`${name} · won${odds}${scheduled?.parent ? ' · a parent won it too' : ''} · click to remove`}
      onClick={onRemove}
    >
      <RaceFace bannerName={name} label={name} goal={!!cellRun?.goal} />
    </button>
  );
}
interface MenuProps {
  menuRef: Ref<HTMLDivElement>;
  /** the turn it is open at (null: closed) */
  turn: number | null;
  offered: readonly number[];
  raceName: Map<number, string>;
  info: (ix: number) => WinInfo | null;
  shownAt: Map<number, number>;
  onPick: (ix: number, turn: number) => void;
  /** closed with the keyboard (Escape, or Tab past either end) */
  onBack: () => void;
}
/** the races held at the turn: her schedule's first, then by name; a race won at another turn moves here */
function WinsMenu({ menuRef, turn, offered, raceName, info, shownAt, onPick, onBack }: MenuProps) {
  const inSchedule = (ix: number) => turn !== null && info(ix)?.turn === turn,
    nameOf = (ix: number) => must(raceName.get(ix), 'an offered race is a race of the data');
  const sorted = [...offered].sort(
    (x, y) => Number(inSchedule(y)) - Number(inSchedule(x)) || nameOf(x).localeCompare(nameOf(y)),
  );
  // Escape closes the menu (not the dialog) and returns to its slot; Tab past either end does the same
  const keys = (e: KeyboardEvent) => {
    if (e.key === 'Escape') {
      e.preventDefault();
      e.stopPropagation();
      onBack();
      return;
    }
    if (e.key !== 'Tab') return;
    const buttons = [...(e.currentTarget as HTMLElement).querySelectorAll('button')],
      i = buttons.indexOf(document.activeElement as HTMLButtonElement);
    if (!buttons.length || (e.shiftKey ? i <= 0 : i === buttons.length - 1)) {
      e.preventDefault();
      onBack();
    }
  };
  return (
    <div
      ref={menuRef}
      class="cpop wmenu"
      hidden={turn === null}
      role="dialog"
      tabIndex={-1}
      aria-label={turn === null ? undefined : `${turnName(turn)}: G1s held`}
      onKeyDown={keys}
    >
      {turn !== null && (
        <>
          <p class="hint">{`${turnName(turn)}: which G1 did she win?`}</p>
          {sorted.map((ix) => (
            <WinPick
              key={ix}
              name={nameOf(ix)}
              sub={
                inSchedule(ix)
                  ? `her schedule · ${shownP(must(info(ix)?.cell, 'a race in her schedule has its cell'))}% win`
                  : shownAt.has(ix)
                    ? `won, shown in ${turnName(must(shownAt.get(ix), 'a win shown has its turn'))}: moves here`
                    : ''
              }
              onPick={() => onPick(ix, turn)}
            />
          ))}
        </>
      )}
    </div>
  );
}
// a race as its banner (the name stays for the screen reader and when the banner is missing)
function WinPick({ name, sub, onPick }: { name: string; sub: string; onPick: () => void }) {
  const src = bannerSrc(name);
  return (
    <Button
      variant={src ? 'slot' : 'plain'}
      class={src ? undefined : 'grid w-full justify-items-start text-left'}
      title={name}
      onClick={onPick}
    >
      {src && <img class="block aspect-2/1 w-full object-contain" alt="" loading="eager" src={src} />}
      <span class={src ? 'sr-only' : undefined}>{name}</span>
      {sub && (
        <small
          class={
            src
              ? 'text-caption text-muted text-center leading-tight font-medium'
              : 'text-muted text-caption font-medium'
          }
        >
          {sub}
        </small>
      )}
    </Button>
  );
}
