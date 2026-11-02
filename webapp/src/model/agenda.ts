// ---------- the race calendar's logic: turns, the community win table, what a race's figure means ----------
// The schedule window and the G1 wins editor draw it (ui/Agenda.tsx, ui/Schedule.tsx); the career page's sections read
// the community table (`tableCost`, `tableChance`).
import { RACEIDS } from './data.ts';
import { srcFor } from './images.ts';
import { Distance, Grade, Surface } from '../gen/solver.ts';
import type { Grades } from '../gen/solver.ts';
import { SOLVER } from '../gen/constants.ts';
import { assert, must } from '../must.ts';
import type { RaceTrack } from './data.ts';
import type { ScheduleCell, ScheduleRow } from '../solver.ts';
import type { DeepReadonly } from '../state/store.ts';

/** A race in her schedule, for the G1 wins editor: its turn and its schedule cell with the win odds; `parent`: a parent
 *  won it too. */
export interface WinInfo {
  turn?: number;
  cell?: ScheduleCell | null;
  parent: boolean;
}

// a turn is one of 72 half-months: 24 per year (the year t / 24; its slot t % 24: month (t % 24) / 2, odd slots Late)
export const YEARS = ['Junior', 'Classic', 'Senior'];
export const MONTHS = ['Jan', 'Feb', 'Mar', 'Apr', 'May', 'Jun', 'Jul', 'Aug', 'Sep', 'Oct', 'Nov', 'Dec'];
// a schedule row's `when` ("Cl E-Apr") as {late, month}, or null when it isn't one
const parseWhen = (when: string): { late: boolean; month: string } | null => {
  const [, , half, month] = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(when) ?? [];
  return half && month ? { late: half === 'L', month } : null;
};
// its slot in the year (0-23), or -1
export const slotOf = (when: string): number => {
  const w = parseWhen(when);
  return w ? MONTHS.indexOf(w.month) * 2 + (w.late ? 1 : 0) : -1;
};
// a turn as the "All" table writes it ("Early Apr", the year being the group row)
export const turnLabel = (when: string): string => {
  const w = parseWhen(when);
  return w ? (w.late ? 'Late ' : 'Early ') + w.month : when;
};
// a slot's label in its date bar ("Early Apr")
export const slotLabel = (slot: number): string => (slot % 2 ? 'Late ' : 'Early ') + MONTHS[slot >> 1];
// a race's banner (by its full or chart name): its image's source, or null
export const bannerSrc = (name: string): string | null => {
  const id = (RACEIDS as Record<string, number>)[name];
  return (id && srcFor('race', id)) || null;
};
export const turnName = (turn: number): string =>
  `${YEARS[(turn / 24) | 0]} Year, ${turn % 2 ? 'Late' : 'Early'} ${MONTHS[(turn % 24) >> 1]}`;
/** member k's cell in a schedule row (null: she doesn't run that race): a row holds one per member */
export function cellAt(row: DeepReadonly<ScheduleRow>, k: number): DeepReadonly<ScheduleCell> | null {
  const cell = row.cells[k];
  assert(cell !== undefined, 'a schedule row holds a cell per member');
  return cell;
}
// the figure a cell shows: the community table's on the career page, else the planner's
export const shownP = (cell: ScheduleCell): number => cell.tbl ?? cell.p;
// the community table (110, minus each aptitude's cost, minus the k-th race in a row's penalty, at most 100): what the
// career page shows, and the Win table panel

/** a race's place in a row of races (1 = the first); the table counts the sixth and later alike */
export type RowPlace = 1 | 2 | 3 | 4 | 5 | 6;
export const ROW_PLACES: readonly RowPlace[] = [1, 2, 3, 4, 5, 6];
const ROW_PEN: Record<RowPlace, number> = { 1: 0, 2: 0, 3: 5, 4: 20, 5: 30, 6: 50 },
  ORD: Record<RowPlace, string> = { 1: '1st', 2: '2nd', 3: '3rd', 4: '4th', 5: '5th', 6: '6th or later' };
const isRowPlace = (x: number): x is RowPlace => Number.isInteger(x) && x >= 1 && x <= 6;
/** the place of the `row`-th race in a row as the table counts it (the solver's `row`: 1 for the first) */
const rowPlace = (row: number): RowPlace => {
  const place = Math.min(row, 6);
  assert(isRowPlace(place), 'a race in a row is the first or a later one');
  return place;
};
const DISTANCE: Record<Distance, string> = {
  [Distance.Short]: 'Sprint',
  [Distance.Mile]: 'Mile',
  [Distance.Medium]: 'Medium',
  [Distance.Long]: 'Long',
};
const SURFACE: Record<Surface, string> = { [Surface.Turf]: 'Turf', [Surface.Dirt]: 'Dirt' };
/** her grades' cost for a race on `track` (model/data.ts `RaceTrack`), on her aptitude grades (model/grades.ts) */
export function tableCost(apt: DeepReadonly<Grades>, track: RaceTrack): { cost: number; grades: string } {
  const d = apt.distance[track.cat],
    s = apt.surface[track.ground];
  return {
    cost: SOLVER.grade_cost[d] + SOLVER.grade_cost[s],
    grades: `${DISTANCE[track.cat]} ${Grade[d]}, ${SURFACE[track.ground]} ${Grade[s]}`,
  };
}
export const tableChance = (cost: number, row: number): number =>
  Math.max(0, Math.min(100, 110 - cost - ROW_PEN[rowPlace(row)]));
// the community table as the schedule window's Win table panel shows it (its own layout): a column per aptitude pair (A
// with each grade, then each grade with G; drawn in the game's grade letters), a row per races in a row, each cell 110
// minus the pair's cost and the row's penalty as the table prints it (not capped at 100)
export const TABLE_PAIRS: readonly (readonly [Grade, Grade])[] = [
  [Grade.A, Grade.A],
  [Grade.A, Grade.B],
  [Grade.A, Grade.C],
  [Grade.A, Grade.D],
  [Grade.A, Grade.E],
  [Grade.A, Grade.F],
  [Grade.A, Grade.G],
  [Grade.B, Grade.G],
  [Grade.C, Grade.G],
  [Grade.D, Grade.G],
  [Grade.E, Grade.G],
  [Grade.F, Grade.G],
  [Grade.G, Grade.G],
];
/** the table's figure for a pair of grades and the k-th race in a row */
export const tableFigure = ([a, b]: readonly [Grade, Grade], k: RowPlace): number =>
  Math.max(0, 110 - SOLVER.grade_cost[a] - SOLVER.grade_cost[b] - ROW_PEN[k]);
// why a race isn't a sure win, a line each: the career page's community-table figure (110 minus her grades' cost, minus
// the k-th race in a row's), then the planner's; elsewhere 110 minus the aptitude cost (distance and surface grades),
// minus the career planner's low-aptitude cost, minus the penalty for the k-th race in a row (a goal race pays the
// (k-1)-th's), capped at 100% ('' when the cell doesn't say)
export const why = (cell: ScheduleCell): string => {
  if (cell.tbl !== undefined) {
    // (career/view/parts.ts tableRows sets the table's figure with its cost and grades, on a cell the solver gave its row)
    const row = must(cell.row, 'a scheduled race has its place in a row'),
      tapt = must(cell.tapt, 'a table figure comes with its cost on her grades');
    const place = rowPlace(row),
      pen = ROW_PEN[place],
      parts: string[] = [];
    if (tapt > 0) parts.push(`${tapt} for her aptitudes (${cell.grades})`);
    if (pen > 0) parts.push(`${pen} for the ${ORD[place]} race in a row`);
    const capped = 110 - tapt - pen > 100 ? ', at most 100' : '';
    return [
      `${cell.tbl}% to win (community table)`,
      parts.length ? `110${parts.map((x) => ` − ${x}`).join('')}${capped}` : 'Nothing taken off: her aptitudes suit it',
      ...(cell.p === cell.tbl ? [] : [`The planner, counting the pink sparks she may get: ${cell.p}%`]),
    ].join('\n');
  }
  if (cell.apt === undefined) return '';
  // (the solver writes apt, row and pen together: career/schedule.rs, chart.rs)
  const row = must(cell.row, 'the solver writes row with apt'),
    pen = must(cell.pen, 'the solver writes pen with apt');
  const parts: string[] = [];
  if (cell.apt > 0) parts.push(`aptitude −${cell.apt}`);
  const low = cell.low ?? 0;
  if (low > 0) parts.push(`low-aptitude cost −${low}`);
  if (pen > 0) parts.push(`${ORD[rowPlace(row)]} race in a row${cell.goal ? ' (a goal race: one less)' : ''} −${pen}`);
  const costs = parts.length ? `, ${parts.join(', ')}` : '',
    capped = 110 - cell.apt - low - pen > 100 ? ', capped at 100' : '';
  return `${cell.p}% win: 110${costs}${capped}`;
};
