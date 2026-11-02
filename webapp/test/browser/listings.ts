// uma.moe stood in for: listings made from career-small's made-up veterans as the game exports them
// (test/fixtures/solver/export-small.json), as uma.moe's search (api/v3/search, `inheritance`) answers them. Shared by the node test of the sync
// (test/unit/umamoe.test.ts) and the browser flow that routes uma.moe's requests (vets.spec.ts).
import { readFileSync } from 'node:fs';
import path from 'node:path';
import type { CareerTables } from '../../src/model/data.ts';
import type { UmamoeListing } from '../../src/io/umadump.ts';

// (a path joined, not `new URL(…, import.meta.url)`, which Vite's build would take for an asset of the page)
const repo = (relative: string) => path.join(import.meta.dirname, '../../..', relative);

/** The Global career tables as the page's data carries them (factor group -> skill group, saddles, sparks' names). */
export const CAREER: CareerTables = {
  races: [],
  ...JSON.parse(readFileSync(repo('pkg/loopopt/data/global/career.json'), 'utf8')),
};

/** A record of the game's export, as far as a listing reads it. */
interface ExportRecord {
  card_id: number;
  rank_score?: number;
  position_id?: number;
  factor_info_array: { factor_id: number }[];
  win_saddle_id_array?: number[];
  succession_chara_array?: ExportRecord[];
}
/** The export file's four veterans. */
export const RECORDS: ExportRecord[] = JSON.parse(
  readFileSync(repo('webapp/test/fixtures/solver/export-small.json'), 'utf8'),
);
/** The export file, for a page's file input. */
export const EXPORT_PATH = repo('webapp/test/fixtures/solver/export-small.json');

/** Veteran `rec` as uma.moe's k-th result: herself at main_*, her parents (positions 10, 20) at left_* and right_*.
 * The first listing also carries a white spark of a skill none of the file's veterans carry (not a target). */
export function listing(rec: ExportRecord, k: number): UmamoeListing {
  const parent = (position: number): ExportRecord => {
    const found = rec.succession_chara_array?.find((x) => x.position_id === position);
    if (!found) throw new Error(`test veteran ${rec.card_id} has no parent at ${position}`);
    return found;
  };
  const side = (r: ExportRecord) => {
    const ids = r.factor_info_array.map((f) => f.factor_id);
    const kind = (lo: number, hi: number) => ids.find((id) => id >= lo && id < hi);
    return {
      white: ids.filter((id) => id >= 1000000 && id < 10000000),
      blue: kind(100, 600),
      pink: kind(1100, 10000),
      green: kind(10000000, 1e9),
      wins: r.win_saddle_id_array ?? [],
    };
  };
  const main = side(rec);
  if (k === 1) {
    const carried = new Set(main.white.map((id) => CAREER.factors[Math.floor(id / 100)]));
    const other = Object.keys(CAREER.factors).find((fg) => !carried.has(CAREER.factors[+fg]));
    if (other === undefined) throw new Error('every skill is carried');
    main.white.push(+other * 100 + 1);
  }
  const left = side(parent(10));
  const right = side(parent(20));
  return {
    account_id: `10000000000${k}`,
    trainer_name: `Trainer ${k}`,
    follower_num: 12 + k,
    last_updated: new Date(Date.now() - 3 * 3600e3).toISOString(),
    inheritance: {
      inheritance_id: 900000 + k,
      main_parent_id: rec.card_id,
      parent_left_id: parent(10).card_id,
      parent_right_id: parent(20).card_id,
      parent_rank: rec.rank_score ?? 0,
      win_count: 10 * k,
      affinity_score: 0,
      white_stars_sum: 0,
      ...sideFields('main', main),
      ...sideFields('left', left),
      ...sideFields('right', right),
    },
  };
}
type Inheritance = UmamoeListing['inheritance'];
/** One member's factors and wins, under her side's field names. */
function sideFields(
  s: 'main' | 'left' | 'right',
  x: { white: number[]; blue: number | undefined; pink: number | undefined; green: number | undefined; wins: number[] },
): Partial<Inheritance> {
  const out: Partial<Inheritance> = {};
  out[`${s}_white_factors`] = x.white;
  out[`${s}_win_saddles`] = x.wins;
  if (x.blue !== undefined) out[`${s}_blue_factors`] = x.blue;
  if (x.pink !== undefined) out[`${s}_pink_factors`] = x.pink;
  if (x.green !== undefined) out[`${s}_green_factors`] = x.green;
  return out;
}
