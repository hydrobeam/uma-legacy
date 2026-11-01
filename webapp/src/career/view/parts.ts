// Small parts the career page's views share: a target's name, numbers, her schedule's rows.
import { gameData } from '../../data.ts';
import { settings } from '../../settings.ts';
import { goldGiven } from '../../job.ts';
import { tableCost, tableChance } from '../../agenda.ts';
import { must } from '../../must.ts';
import type { DeepReadonly } from '../../store.ts';
import type { CareerReport } from '../job.ts';

export const tName = (g: number): string => {
  const s = gameData().skillNames.get(g) || { w: `Skill ${g}`, gd: null };
  return goldGiven(settings.value.scen, g) && s.gd ? s.gd : s.w;
};
export const fmtN = (x: number): string => Math.round(x).toLocaleString();
/** a heading inside a popup's body (Skill Details, the Saves popup): bold, ruled off below */
export const popupHeading = 'border-gm-line border-b-2 pb-0.5 font-black';
// her race schedule: the schedule window (the game's calendar), from a button under the deck (the races she runs: the
// G1s she could enter but doesn't aren't shown)
// her schedule's rows, each race's figure the community table's on her own grades (agenda.ts `tableChance`; the
// planner's, which counts the pink sparks she may get, on hover)
export const tableRows = (r: DeepReadonly<CareerReport>) =>
  (r.schedule || []).map(({ chart, ...row }) => {
    // (the chart's row, with her turn, what she does and which parents won it beside it)
    const shown = { ...chart, ...row };
    const [c] = chart.cells,
      track = gameData().tracks.get(row.name),
      apt = r.aptitudes?.base;
    if (!c || !track || !apt) return shown;
    const { cost, grades } = tableCost(apt, track);
    const tbl = tableChance(cost, must(c.row, 'a scheduled race has its place in a row'));
    return { ...shown, cells: [{ ...c, tbl, tapt: cost, grades }] };
  });
