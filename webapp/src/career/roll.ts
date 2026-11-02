// A roll: a random result of a career (the veteran it leaves), drawn from the solver's report of it. No page state:
// each function takes the report, what it reads of the settings and data (`RollSetup`) and the random source (`rnd`,
// Math.random on the page; test/unit/careerroll.test.ts passes scripted uniforms).
import { must } from '../must.ts';
import { drawPink } from '../model/pink.ts';
import { SOLVER } from '../gen/constants.ts';
import type { Grades, RankClass } from '../gen/solver.ts';
import type { DeepReadonly } from '../state/store.ts';
import type { CareerReport } from './job.ts';

/** What a roll reads besides the career's report. */
export interface RollSetup {
  /** the career tables (model/data.ts `careerTables()`): the G1 races by index, the outfits with a unique skill */
  career: { races: readonly string[]; uniques?: Readonly<Record<number, unknown>> };
  /** the target skills, in the job's order (the outcome sets' bit order) */
  targets: readonly { group: number }[];
  /** the lineage's rank band, whose star mix (the chances of 1, 2 and 3 stars) the solver has */
  rank: RankClass;
}
/** A roll: the sparks she landed (skill group -> stars), the skills she bought (group -> 'gold' | 'white'), the G1s she
 * won (race indices, ascending), her pink spark and end grades (model/pink.ts), her blue (stat -> stars) and unique
 * (outfit
 * -> stars) sparks. */
export interface Roll {
  sparks: Record<number, number>;
  bought: Record<number, string>;
  wins: number[];
  pinks: Record<number, number>;
  apt: Grades | null;
  blues: Record<number, number>;
  greens: Record<number, number>;
}
/** `r`, below: the career's report (career/job.ts `CareerReport`), read only */
type Report = DeepReadonly<CareerReport>;
/** a G1 she runs: its index in the career tables' races, its name, the chance she wins it at least once, her turns */
export interface RaceChance {
  ix: number;
  name: string;
  p: number;
  turns: number[];
}

// the G1s she runs (`races`: the career tables' race names, by index), each with p, the chance she wins it at least
// once, and the turns she runs it at
export function scheduledRaces(r: Pick<Report, 'schedule'>, races: readonly string[]): RaceChance[] {
  const byIndex = new Map<number, RaceChance>();
  for (const w of (r.schedule || []).filter((w) => w.run !== false)) {
    const ix = races.indexOf(w.name || w.chart.race);
    if (ix < 0) continue;
    // run at another turn too: p, the chance she wins it at least once, taking the two runs as independent. The solver
    // does too when her grades are certain; an April pink that may raise them links the runs (MODEL.md, E[p_C p_S]),
    // which this leaves out: under 0.1 points, since a firing is 1-5% per legacy and moves a run's chance by about 10
    const prev = byIndex.get(ix);
    byIndex.set(ix, {
      ix,
      name: w.name || w.chart.race,
      // (a row she runs has its chance: schedule.rs makes it null only on a skipped row, filtered out above)
      p: 1 - (1 - (prev?.p || 0)) * (1 - must(w.p, 'the win chance of a race she runs')),
      turns: [...(prev?.turns || []), w.turn],
    });
  }
  return [...byIndex.values()];
}
// the skills she bought in a roll, given the sparks it landed (the report's per-target buying and landing chances):
// a landed one is bought, gold with chance land_gold / land; one that didn't land was bought anyway with chance
// (buy - land) / (1 - land), gold with chance (gold - land_gold) / (buy - land)
export function rolledBought(
  r: Report,
  sparks: Readonly<Record<number, number>>,
  rnd: () => number = Math.random,
): Record<number, string> {
  const bought: Record<number, string> = {};
  for (const t of r.targets) {
    const b = t.buy;
    if (!b) continue;
    const land = b.land_gold + b.land_white,
      buy = b.gold + b.white; // P(lands as a spark), P(bought)
    if (sparks[t.group]) bought[t.group] = rnd() < (land > 0 ? b.land_gold / land : 0) ? 'gold' : 'white';
    else if (land < 1 && rnd() < Math.max(0, buy - land) / (1 - land)) {
      const goldGivenUnlanded = buy - land > 0 ? Math.max(0, b.gold - b.land_gold) / (buy - land) : 0;
      bought[t.group] = rnd() < goldGivenUnlanded ? 'gold' : 'white';
    }
  }
  return bought;
}
// a spark's stars, drawn from the rank band's star mix (the solver's: the chances of 1, 2 and 3 stars)
export function starDraw(rank: RankClass, rnd: () => number = Math.random): () => number {
  const [one, two] = SOLVER.star_mix[rank];
  return () => {
    const u = rnd();
    return u < one ? 1 : u < one + two ? 2 : 3;
  };
}
// a random result of this career: the set of landed targets drawn from its exact odds, each spark's stars from the rank,
// each G1 she runs won with its chance (it differs every time; the career's first showing is one of these too)
export function rolledOutcome(r: Report, setup: RollSetup, rnd: () => number = Math.random): Roll {
  const star = starDraw(setup.rank, rnd);
  const sparks: Record<number, number> = {};
  if (Array.isArray(r.outcome_sets)) {
    // the landed set (a mask over the targets) drawn by its chance; the last if rounding leaves u past them all
    const u = rnd();
    let cumulative = 0,
      mask = r.outcome_sets.length - 1;
    for (let i = 0; i < r.outcome_sets.length; i++) {
      cumulative += r.outcome_sets[i];
      if (u < cumulative) {
        mask = i;
        break;
      }
    }
    setup.targets.forEach((t, i) => {
      if (mask & (1 << i)) sparks[t.group] = star();
    });
  } else
    r.targets.forEach((t) => {
      if (rnd() < t.p) sparks[t.group] = star();
    }); // over 12 targets: each on its own odds
  const wins = scheduledRaces(r, setup.career.races)
    .filter((x) => rnd() < x.p)
    .map((x) => x.ix)
    .sort((a, b) => a - b);
  const bought = rolledBought(r, sparks, rnd),
    pink = drawPink(r.aptitudes, rnd); // her pink spark
  return { sparks, bought, wins, pinks: pink.pinks, apt: pink.apt, ...rolledBlueGreen(r, setup, rnd) };
}
// her blue (stat) spark: one of the five stats, near-equally likely (hakuraku: 20% each, a slight lean to her stronger
// stats); its stars follow that stat's value, which the model doesn't have: the rank band's star mix stands in. Her unique
// (green) spark: always (GameTora: guaranteed for an outfit at 3 stars or more; the page assumes yours are), stars by the
// rank band (hakuraku: as white sparks, not by the skill's level)
export function rolledBlueGreen(
  r: Report,
  setup: RollSetup,
  rnd: () => number = Math.random,
): Pick<Roll, 'blues' | 'greens'> {
  const star = starDraw(setup.rank, rnd);
  const blues = { [1 + Math.min(4, Math.floor(rnd() * 5))]: star() };
  const out = r.trainee.outfits[0]?.id,
    greens: Record<number, number> = {};
  if (out && setup.career.uniques?.[out]) greens[out] = star();
  return { blues, greens };
}
