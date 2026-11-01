// Small parts the career page's views share: a target's name, numbers, her schedule's rows.
import { careerTables, gameData } from '../../data.js';
import { settings } from '../../settings.ts';
import { goldGiven } from '../../job.js';
import { tableCost, tableChance } from '../../agenda.js';
import { must } from '../../must.ts';
import type { DeepReadonly } from '../../store.ts';
import type { CareerReport } from '../job.js';

export const tName = (g: number): string => {
  const s = gameData().skillNames.get(g) || { w: `Skill ${g}`, gd: null };
  return goldGiven(settings.value.scen, g) && s.gd ? s.gd : s.w;
};
export const fmtN = (x: number): string => Math.round(x).toLocaleString();
// her race schedule: the schedule window (the game's calendar), from a button under the deck (the races she runs: the
// G1s she could enter but doesn't aren't shown)
// her schedule's rows, each race's figure the community table's on her own grades (agenda.js `tableChance`; the
// planner's, which counts the pink sparks she may get, on hover)
export const tableRows = (r: DeepReadonly<CareerReport>) =>
  (r.schedule || []).map((row) => {
    const tables = careerTables(),
      c = row.cells[0],
      ix = tables.races.indexOf(row.name),
      apt = r.aptitudes?.base;
    if (!c || ix < 0 || !apt || !tables.race_cat) return row;
    const ground = must(tables.race_ground, 'build.py writes race_ground with race_cat')[ix];
    const { cost, grades } = tableCost(apt, tables.race_cat[ix], ground);
    // @ts-expect-error a scheduled race has its place in a row (`row`): must() in the strict commit
    return { ...row, cells: [{ ...c, tbl: tableChance(cost, c.row), tapt: cost, grades }] };
  });
