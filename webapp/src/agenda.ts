// ---------- the race calendar's logic: turns, the community win table, what a race's figure means ----------
// The schedule window and the G1 wins editor draw it (ui/Agenda.tsx, ui/Schedule.tsx); the career page's sections read
// the community table (`tableCost`, `tableChance`).
import { RACEIDS } from './data.js';
import { srcFor } from './images.js';
import { must } from './must.ts';

/** One member's attempt at a scheduled race (the solver's schedule report). */
export interface ScheduleCell {
  /** her win chance, in percent */
  p: number;
  /** the aptitude cost (distance and surface grades), in percentage points */
  apt?: number;
  /** the career planner's low-aptitude cost (a race at the lowest aptitudes wins less than the table says), in
   *  percentage points; absent from the loop planner's cells */
  low?: number;
  /** the penalty for racing several turns in a row, in percentage points (a goal race pays the previous position's) */
  pen?: number;
  /** which race in a row this is (1 = the first) */
  row?: number;
  /** a goal race of her scenario */
  goal: boolean;
  /** the career page: her win chance on the community table, on her own grades (`shownP` shows it in place of `p`, the
   *  planner's, which counts her pink sparks) */
  tbl?: number;
  /** the table's aptitude cost on her own grades, in percentage points */
  tapt?: number;
  /** those grades, as "Mile C, Turf A" */
  grades?: string;
}
/** One scheduled G1, one cell per member (null: she doesn't run it). */
export interface ScheduleRow {
  /** 0 Junior, 1 Classic, 2 Senior */
  year: number;
  /** the turn, as "Cl E-Apr" (year, Early or Late, month) */
  when: string;
  /** its name; "A/B" when either of two races runs at the same odds */
  race: string;
  /** two or more members run it (it builds affinity); one member: a parent won it */
  shared: boolean;
  cells: (ScheduleCell | null)[];
  /** one member: which of her parents won it */
  parents?: boolean[];
  /** how many members run it */
  runners: number;
  /** the other race of an "A/B" pair */
  alts?: string[];
}
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
  const m = /^(Jr|Cl|Sr) ([EL])-(\w+)$/.exec(when);
  return m ? { late: m[2] === 'L', month: m[3] } : null;
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
// the figure a cell shows: the community table's on the career page, else the planner's
export const shownP = (cell: ScheduleCell): number => cell.tbl ?? cell.p;
// the community table (110, minus each aptitude's cost, minus the k-th race in a row's penalty, at most 100): what the
// career page shows, and the Win table panel
const GRADE = ['G', 'F', 'E', 'D', 'C', 'B', 'A', 'S'],
  GRADE_COST = [90, 60, 40, 30, 20, 10, 0, 0],
  ROW_PEN = [0, 0, 5, 20, 30, 50];
/** her grades' cost for a race (`cat` 0-3 sprint to long, `ground` 1 turf, 2 dirt) on aptitude codes 1..8 in data.json's
 *  `apt` order (sprint, mile, medium, long, turf, dirt, ...) */
export function tableCost(apt: readonly number[], cat: number, ground: number): { cost: number; grades: string } {
  const d = apt[cat],
    s = apt[ground === 1 ? 4 : 5];
  return {
    cost: GRADE_COST[d - 1] + GRADE_COST[s - 1],
    grades: `${['Sprint', 'Mile', 'Medium', 'Long'][cat]} ${GRADE[d - 1]}, ${ground === 1 ? 'Turf' : 'Dirt'} ${GRADE[s - 1]}`,
  };
}
export const tableChance = (cost: number, row: number): number =>
  Math.max(0, Math.min(100, 110 - cost - ROW_PEN[Math.min(row, 6) - 1]));
// the community table as the schedule window's Win table panel shows it (its own layout): a column per aptitude pair (A
// with each grade, then each grade with G; drawn in the game's grade letters), a row per races in a row, each cell 110
// minus the pair's cost and the row's penalty as the table prints it (not capped at 100)
export const TABLE_PAIRS = ['AA', 'AB', 'AC', 'AD', 'AE', 'AF', 'AG', 'BG', 'CG', 'DG', 'EG', 'FG', 'GG'];
/** a grade letter's code (1 G .. 7 A) */
export const gradeCode = (letter: string): number => 'GFEDCBA'.indexOf(letter) + 1;
/** the table's figure for a pair of grade letters ("AB") and k races in a row */
export const tableFigure = (pair: string, k: number): number =>
  Math.max(0, 110 - GRADE_COST[gradeCode(pair[0]) - 1] - GRADE_COST[gradeCode(pair[1]) - 1] - ROW_PEN[k - 1]);
// why a race isn't a sure win, a line each: the career page's community-table figure (110 minus her grades' cost, minus
// the k-th race in a row's), then the planner's; elsewhere 110 minus the aptitude cost (distance and surface grades),
// minus the career planner's low-aptitude cost, minus the penalty for the k-th race in a row (a goal race pays the
// (k-1)-th's), capped at 100% ('' when the cell doesn't say)
const ORD = ['', '1st', '2nd', '3rd', '4th', '5th', '6th or later'];
export const why = (cell: ScheduleCell): string => {
  if (cell.tbl !== undefined) {
    // (career/view/parts.js tableRows sets the table's figure with its cost and grades, on a cell the solver gave its row)
    const row = must(cell.row, 'a scheduled race has its place in a row'),
      tapt = must(cell.tapt, 'a table figure comes with its cost on her grades');
    const pen = ROW_PEN[Math.min(row, 6) - 1],
      parts: string[] = [];
    if (tapt > 0) parts.push(`${tapt} for her aptitudes (${cell.grades})`);
    if (pen > 0) parts.push(`${pen} for the ${ORD[Math.min(row, 6)]} race in a row`);
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
  if (pen > 0)
    parts.push(`${ORD[Math.min(row, 6)]} race in a row${cell.goal ? ' (a goal race: one less)' : ''} −${pen}`);
  const costs = parts.length ? `, ${parts.join(', ')}` : '',
    capped = 110 - cell.apt - low - pen > 100 ? ', capped at 100' : '';
  return `${cell.p}% win: 110${costs}${capped}`;
};
