// A roll of the career planner (career/roll.ts) and her pink spark (pink.ts `drawPink`) against the solver's reports
// they are drawn from: on every reference career (val/ref__global__career_*.out, the solver's outputs validation pins),
// seeded draws land each figure as often as the report gives it, to 5 sigma (at least 1e-3). The draws are few enough
// to run in about a second each (10,000 pink draws and 5,000 rolls per career: 5 sigma is about ±0.025 and ±0.035 at
// p = 0.5), so they catch a figure drawn from the wrong field or bit, not a bias of a percent. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { test } from 'vitest';
import { drawPink } from '../../src/pink.ts';
import { rolledBlueGreen, rolledBought, rolledOutcome, scheduledRaces, starDraw } from '../../src/career/roll.ts';
import type { CareerReport } from '../../src/career/job.ts';
import { SOLVER } from '../../src/gen/constants.ts';
import type { RankClass } from '../../src/gen/solver.ts';

// a seeded random source (mulberry32)
function seeded(seed = 0x9e3779b9) {
  let s = seed;
  return () => {
    s = (s + 0x6d2b79f5) | 0;
    let t = Math.imul(s ^ (s >>> 15), 1 | s);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}
const VAL = new URL('../../../val/', import.meta.url);
// the reference careers' reports: [file, report]
const referenceCareers = (): [string, CareerReport][] =>
  fs
    .readdirSync(VAL)
    .filter((f) => /^ref__global__career_.*\.out$/.test(f))
    .sort()
    .flatMap((f) => {
      const out: { results?: CareerReport[] } = JSON.parse(fs.readFileSync(new URL(f, VAL), 'utf8'));
      return (out.results ?? []).map((r): [string, CareerReport] => [f, r]);
    });

/** Counts each figure's draws against its chance; the first one off by more than 5 sigma (at least 1e-3). */
function tally() {
  let figures = 0,
    worst = 0,
    bad = '';
  return {
    near(what: string, count: number, p: number, n: number) {
      const fr = count / n,
        sd = Math.sqrt((p * (1 - p)) / n);
      figures++;
      if (Math.abs(fr - p) > Math.max(5 * sd, 1e-3)) bad ||= `${what}: drawn ${fr.toFixed(4)} vs ${p.toFixed(4)}`;
      if (sd > 0) worst = Math.max(worst, Math.abs(fr - p) / sd);
    },
    fail(why: string) {
      bad ||= why;
    },
    check() {
      assert.ok(figures > 0, 'some figures compared');
      assert.equal(bad, '', `worst ${worst.toFixed(2)} sigma`);
    },
  };
}
const count = (m: Map<number, number>, k: number) => m.set(k, (m.get(k) ?? 0) + 1);

test("a roll's pink spark (`drawPink`) is drawn from the solver's pink model: each aptitude as often as its `pink_gen` chance", () => {
  const rnd = seeded(),
    N = 10000,
    t = tally();
  for (const [f, r] of referenceCareers()) {
    const ap = r.aptitudes;
    if (ap?.start.kind !== 'known' || !ap.start.fire.length) continue;
    const drawn = new Map<number, number>(); // pink aptitude -> draws that gave it
    for (let i = 0; i < N; i++) {
      const g = Object.keys(drawPink(ap, rnd).pinks)[0];
      if (g !== undefined) count(drawn, +g);
    }
    for (const { aptitude, p } of ap.pink_gen) t.near(`${f} aptitude ${aptitude}`, drawn.get(aptitude) ?? 0, p, N);
  }
  t.check();
});

test("a roll keeps its career's report: targets land, are bought and gold, G1s are won as the report says; stars, blue stat and unique as the model draws them", () => {
  const rnd = seeded(),
    N = 5000,
    rank: RankClass = 'ss_and_up',
    mix = SOLVER.star_mix[rank],
    t = tally();
  for (const [f, r] of referenceCareers()) {
    const races = [...new Set((r.schedule ?? []).map((w) => w.name || w.chart.race))],
      outfit = r.trainee.outfits[0]?.id,
      setup = { career: { races, uniques: outfit ? { [outfit]: true } : {} }, targets: r.targets, rank };
    const landed = new Map<number, number>(),
      bought = new Map<number, number>(),
      gold = new Map<number, number>(),
      won = new Map<number, number>(), // race index -> rolls she won it in
      stars = [0, 0, 0],
      blues = [0, 0, 0, 0, 0];
    let greens = 0;
    for (let i = 0; i < N; i++) {
      const roll = rolledOutcome(r, setup, rnd);
      for (const [g, s] of Object.entries(roll.sparks)) {
        count(landed, +g);
        stars[s - 1] = (stars[s - 1] ?? 0) + 1;
      }
      for (const [g, kind] of Object.entries(roll.bought)) {
        count(bought, +g);
        if (kind === 'gold') count(gold, +g);
      }
      for (const ix of roll.wins) count(won, ix);
      const blue = +(Object.keys(roll.blues)[0] ?? 0) - 1;
      blues[blue] = (blues[blue] ?? 0) + 1;
      greens += outfit && roll.greens[outfit] ? 1 : 0;
    }
    const sets = Array.isArray(r.outcome_sets) ? r.outcome_sets : null;
    r.targets.forEach((target, i) => {
      const p = sets ? sets.reduce((sum, q, mask) => sum + (mask & (1 << i) ? q : 0), 0) : target.p;
      t.near(`${f} ${r.career} target ${target.group} lands`, landed.get(target.group) ?? 0, p, N);
      if (!target.buy) return;
      const { gold: g, white } = target.buy;
      t.near(`${f} ${r.career} target ${target.group} bought`, bought.get(target.group) ?? 0, g + white, N);
      t.near(`${f} ${r.career} target ${target.group} gold`, gold.get(target.group) ?? 0, g, N);
    });
    for (const race of scheduledRaces(r, races))
      t.near(`${f} ${r.career} ${race.name} won`, won.get(race.ix) ?? 0, race.p, N);
    const sparks = stars.reduce((a, b) => a + b, 0);
    mix.forEach((p, k) => {
      t.near(`${f} ${r.career} ${k + 1}-star sparks`, stars[k] ?? 0, p, sparks);
    });
    blues.forEach((n, k) => {
      t.near(`${f} ${r.career} blue stat ${k + 1}`, n, 0.2, N);
    });
    // (a report without an outfit, as before outfits were reported: no unique)
    if (greens !== (outfit ? N : 0)) t.fail(`${f} ${r.career}: her unique in ${greens} of ${N} rolls`);
  }
  t.check();
});

test("a roll's pieces: a race run twice is won with 1 - (1 - p1)(1 - p2), the star mix's cut points, no bought draw for a skill that never lands, no unique for an outfit without one", () => {
  const [[, report] = []] = referenceCareers();
  assert.ok(report, 'a reference career');
  const [row] = report.schedule ?? [];
  const [target] = report.targets;
  const [outfit] = report.trainee.outfits;
  assert.ok(row && target && outfit, "the reference career's schedule, targets and outfit");
  const twice = scheduledRaces(
    {
      schedule: [
        { ...row, name: 'A', p: 0.5, turn: 1, run: true },
        { ...row, name: 'A', p: 0.4, turn: 30, run: true },
        { ...row, name: 'B', p: 0.9, turn: 2, run: false },
        { ...row, name: 'C', p: 0.9, turn: 3, run: true },
      ],
    },
    ['B', 'A'],
  );
  assert.equal(twice.length, 1);
  assert.equal(twice[0]?.ix, 1);
  assert.ok(Math.abs((twice[0]?.p ?? 0) - 0.7) < 1e-12);
  assert.deepEqual(twice[0]?.turns, [1, 30]);
  const rank: RankClass = 'ss_and_up';
  const [one = 0, two = 0] = SOLVER.star_mix[rank];
  // a uniform just below and just above each of the rank class's cut points (1|2 and 2|3 stars)
  const stars = [one - 1e-9, one + 1e-9, one + two - 1e-9, one + two + 1e-9].map((u) => starDraw(rank, () => u)());
  assert.deepEqual(stars, [1, 2, 2, 3]);
  const rnd = seeded();
  const never = { land_gold: 0, land_white: 0, gold: 0, white: 0 };
  const unlanded = rolledBought(
    { ...report, targets: [{ ...target, group: 7, buy: { ...target.buy, ...never } }] },
    {},
    rnd,
  );
  assert.deepEqual(unlanded, {});
  const noUnique = rolledBlueGreen(
    { ...report, trainee: { ...report.trainee, outfits: [{ ...outfit, id: 5 }] } },
    { career: { races: [] }, targets: [], rank },
    rnd,
  );
  assert.deepEqual(noUnique.greens, {});
});
