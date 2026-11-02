// ---------- the game's race calendar (Scheduled Races): the schedule window's and the G1 wins editor's ----------
// Three year columns of half-month slots, Junior's from July (the career starts in Junior Year, early July). On the
// planner pages (html.game) all three years side by side, each 4 x 6 slots, Junior's first three rows the game's grey
// cells for April to June (labelled as the game labels them, each half-month twice); stacked on a narrow screen, the
// grey cells left out. model/agenda.ts holds the logic. A slot is the game's white frame (its ::before), its race or
// "+" over the frame and the turn under it; what it holds, data-slot's kinds (pre, on, opt, many, free), reaches its
// parts through the theme's slot-pre:, slot-opt: and slot-many:. The checks find a year by data-year, its grid by
// data-grid, a slot by data-slot, a race by data-race (a win's button: data-ix), its banner by data-banner, a turn's
// bar by data-turn.
import type { ComponentChildren, Ref, VNode } from 'preact';
import { useEffect, useLayoutEffect, useMemo, useRef, useState } from 'preact/hooks';
import { MONTHS, YEARS, bannerSrc, shownP, slotLabel, turnName, why } from '../model/agenda.ts';
import type { WinInfo } from '../model/agenda.ts';
import type { ScheduleCell } from '../solver/solver.ts';
import { must } from '../must.ts';
import { Button } from './Button.tsx';
import { popover } from './Popover.tsx';

// April to June before the career, 4 cells a month (Early, Early, Late, Late)
const PRE = Array.from({ length: 12 }, (_, i) => (i % 4 < 2 ? 'Early ' : 'Late ') + MONTHS[3 + (i >> 2)]);
const SLOTS = Array.from({ length: 24 }, (_, i) => i);
/** the calendar: `slot(year, slot index 0-23, turn)` draws each slot of the career (a keyed `AgendaSlot`) */
export function AgendaGrid({ slot }: { slot: (year: number, slotIndex: number, turn: number) => VNode }) {
  const game = document.documentElement.classList.contains('game');
  return (
    <div class="grid grid-cols-3 gap-2.5 max-xl:grid-cols-1">
      {YEARS.map((year, y) => (
        <section key={year} class="bg-gm-pane rounded-xl px-2 pb-2" data-year={y}>
          <h4 class="art-bright inset-shadow-rim text-shadow-outline text-body -mx-2 mb-2 rounded-t-xl p-1.25 text-center font-bold tracking-wide text-white">{`${year} Year`}</h4>
          {/* (the same 4 x 6 grid every year, sized to the window so the whole calendar shows without scrolling) */}
          <div
            class="grid h-[max(360px,calc(100dvh-262px))] grid-cols-4 grid-rows-6 gap-1.5 max-xl:h-auto max-xl:auto-rows-[84px] max-xl:grid-rows-none"
            data-grid=""
          >
            {game &&
              y === 0 &&
              PRE.map((label, i) => (
                // biome-ignore lint/suspicious/noArrayIndexKey: the twelve grey cells, fixed (each label twice)
                <AgendaSlot key={`pre${i}`} pre>
                  <div class="contents" data-turn="">
                    <span class={TURN}>{label}</span>
                  </div>
                </AgendaSlot>
              ))}
            {/* Junior January to June, before the career: off html.game, one note fills it */}
            {!game && y === 0 && (
              <div class="text-ui text-muted border-line col-span-full row-span-3 grid place-items-center rounded-md border border-dashed p-2 text-center max-xl:hidden">
                The career starts in Junior Year, early July
              </div>
            )}
            {SLOTS.filter((s) => y > 0 || s >= 12).map((s) => slot(y, s, y * 24 + s))}
          </div>
        </section>
      ))}
    </div>
  );
}
// a slot: the game's white frame (a grey one, faint, before the career, left out when the years stack), the race or
// "+" drawn over it in its first row, the turn under it
const SLOT = {
  pre: 'before:nine-slot-empty relative grid min-h-0 grid-rows-[minmax(0,1fr)_auto] before:col-start-1 before:row-start-1 before:min-h-0 before:opacity-(--alpha-slot-empty) max-xl:hidden',
  career:
    'before:nine-slot-frame relative grid min-h-0 grid-rows-[minmax(0,1fr)_auto] before:col-start-1 before:row-start-1 before:min-h-0',
} as const;
/** a slot of the calendar: `pre`, a grey cell before the career; `on`, it holds a race (`opt`: an optional one, `many`:
 *  two wins); `free`, an empty slot where a G1 is held */
export function AgendaSlot(props: {
  pre?: boolean;
  on?: boolean;
  opt?: boolean;
  many?: boolean;
  free?: boolean;
  children: ComponentChildren;
}) {
  const { pre = false, children } = props,
    kinds = (['pre', 'on', 'opt', 'many', 'free'] as const).filter((k) => props[k]);
  return (
    <div class={SLOT[pre ? 'pre' : 'career']} data-slot={kinds.join(' ')}>
      {children}
    </div>
  );
}
// a slot's turn ("Early Apr") under its frame, faded in the grey cells
const TURN =
  'text-caption text-gm-ink slot-pre:text-gm-off-ink slot-pre:opacity-70 row-start-2 pt-1.5 text-center leading-none font-bold';
/** the bar at the foot of a slot: "Early Apr", and the win odds of the race run there (the game's pink badge over the
 *  frame's foot; an optional race's greyed, its words still white) */
export function WhenBar({ slotIndex, cell }: { slotIndex: number; cell: ScheduleCell | null }) {
  return (
    <div class="contents" data-turn="">
      <span class={TURN}>{slotLabel(slotIndex)}</span>
      {cell && (
        <b
          class="text-caption before:nine-sched-badge slot-opt:text-shadow-badge slot-opt:before:brightness-62 slot-opt:before:grayscale relative z-1 col-start-1 row-start-1 min-w-[64%] translate-y-[45%] self-end justify-self-center px-2.25 pb-0.5 text-center font-bold whitespace-nowrap text-white tabular-nums before:absolute before:inset-0 before:-z-1"
          title={why(cell)}
        >{`${shownP(cell)}% win`}</b>
      )}
    </div>
  );
}
/** a race's box in its slot, over the frame (the schedule window's; a win's button is WinButton's) */
export const RACE_BOX = 'relative col-start-1 row-start-1 block min-h-0 px-1 pt-1 pb-1.5';
/** a race in its slot (the schedule window's cell, and a win in the G1 wins editor): banner (by `bannerName`; none in
 *  a slot of two wins), name (for the screen reader), Goal badge (at the frame's corner; inside it on a phone); the
 *  element around them is the caller's (a div of RACE_BOX or WinButton) */
export const RaceFace = ({ bannerName, label, goal }: { bannerName: string; label: string; goal: boolean }) => {
  const src = bannerSrc(bannerName);
  return (
    <>
      {src && (
        <img
          class="slot-many:hidden block size-full min-h-0 object-contain"
          alt=""
          loading="lazy"
          src={src}
          data-banner=""
        />
      )}
      <div class="sr-only">{label}</div>
      {goal && (
        <span class="text-caption bg-gold text-panel absolute -top-1.25 -right-0.75 rounded-full px-1.25 font-bold uppercase max-md:top-0.5 max-md:right-0.5">
          Goal
        </span>
      )}
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
      slotRect = must(opener.closest('[data-slot]'), 'a "+" is in its slot').getBoundingClientRect(),
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
    <div class="relative" ref={box}>
      <AgendaGrid
        slot={(_y, slotIndex, turn) => {
          const here = byTurn.get(turn) || [],
            offered = here.length ? [] : (offer ? offer(turn) : heldAt.get(turn)) || [],
            [first, ...more] = here,
            only = first !== undefined && !more.length ? info(first) : null;
          return (
            <AgendaSlot key={turn} on={here.length > 0} many={here.length > 1} free={offered.length > 0}>
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
                  class="art-plus hover:art-plus-on aria-expanded:art-plus-on focus-visible:outline-accent col-start-1 row-start-1 grid min-h-0 w-full cursor-pointer place-items-center rounded-md focus-visible:outline-2 focus-visible:-outline-offset-2"
                  data-k={`add:${turn}`}
                  aria-label={`${turnName(turn)}: add a G1 win`}
                  title="Add a G1 win"
                  aria-haspopup="dialog"
                  aria-expanded={menuFor === turn}
                  onClick={() => setMenuFor(menuFor === turn ? null : turn)}
                >
                  <span class="hidden" aria-hidden="true">
                    +
                  </span>
                </button>
              )}
              <WhenBar slotIndex={slotIndex} cell={only && only.turn === turn ? (only.cell ?? null) : null} />
            </AgendaSlot>
          );
        }}
      />
      {offCalendar.length > 0 && (
        // a win with no turn on the calendar (not in the race data): listed, still removable
        <p class="text-note text-gm-ink2">
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
      class="text-ink hover:bg-accent-soft focus-visible:outline-accent slot-many:px-1 slot-many:py-0.5 relative col-start-1 row-start-1 block min-h-0 cursor-pointer rounded-sm px-1 pt-1 pb-1.5 focus-visible:outline-2 focus-visible:-outline-offset-2"
      data-race=""
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
      class={popover({
        frame: 'game',
        class:
          'text-ui absolute z-50 grid w-[min(492px,calc(100vw-48px))] grid-cols-[repeat(auto-fill,150px)] gap-2 p-2.5 focus:outline-none',
      })}
      data-pop="wins"
      hidden={turn === null}
      role="dialog"
      tabIndex={-1}
      aria-label={turn === null ? undefined : `${turnName(turn)}: G1s held`}
      onKeyDown={keys}
    >
      {turn !== null && (
        <>
          <p class="text-note text-gm-ink2 col-span-full">{`${turnName(turn)}: which G1 did she win?`}</p>
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
