// A roll of the career planner (career/roll.ts) and her pink spark (pink.ts `drawPink`) against the solver's reports
// they are drawn from, exactly: each draw maps one uniform in [0, 1) to an outcome, so a scripted random source (the
// uniforms in order) puts u just below and just above each cut point and the outcome changes there, on a reference
// career (val/ref__global__career_U.out, the solver's output validation pins). The interval between two cut points is
// each outcome's chance, so these show every piece drawn with the chance the report gives it, and one whole roll shows
// which piece reads which uniform. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import fs from 'node:fs';
import { test } from 'vitest';
import { drawPink, pinkAmong } from '../../src/pink.ts';
import { rolledBlueGreen, rolledBought, rolledOutcome, scheduledRaces, starDraw } from '../../src/career/roll.ts';
import type { RollSetup } from '../../src/career/roll.ts';
import type { CareerReport } from '../../src/career/job.ts';
import { gradeOf, withGrade } from '../../src/grades.ts';
import { SOLVER } from '../../src/gen/constants.ts';
import { Grade } from '../../src/gen/solver.ts';
import type { Aptitude, Grades, RankClass } from '../../src/gen/solver.ts';

// how far from a cut point the scripted uniforms sit: well under any chance the reports give, well over rounding
const EPS = 1e-9;
/** A scripted random source: `us` in order, then `rest` for every further draw (none when it is left out: a draw past
 *  the script fails); `used()` counts the draws taken. */
function script(us: readonly number[], rest?: number) {
  let taken = 0;
  const rnd = () => {
    const u = us[taken] ?? rest;
    taken++;
    assert.ok(u !== undefined, `draw ${taken} is past the script`);
    return u;
  };
  return Object.assign(rnd, { used: () => taken });
}
const close = (a: number | undefined, b: number, what: string) => assert.ok(Math.abs((a ?? NaN) - b) < 1e-12, what);

// the first two careers of a reference output: one with the landed targets' sets (`outcome_sets`) and a schedule, and
// one over each target's own odds (the solver reports sets up to 12 targets; this pins the other branch)
const [withSets, perTarget] = (
  JSON.parse(fs.readFileSync(new URL('../../../val/ref__global__career_U.out', import.meta.url), 'utf8')) as {
    results: CareerReport[];
  }
).results;
assert.ok(withSets && perTarget && Array.isArray(withSets.outcome_sets) && !perTarget.outcome_sets);
const rank: RankClass = 'ss_and_up';
const [one = 0, two = 0] = SOLVER.star_mix[rank];
// a uniform drawing 1, 2 or 3 stars from the rank's star mix
const STAR = { 1: one / 2, 2: one + two / 2, 3: 1 - EPS };
const setupOf = (r: CareerReport, races: readonly string[] = []): RollSetup => {
  const outfit = r.trainee.outfits[0]?.id;
  return { career: { races, uniques: outfit ? { [outfit]: true } : {} }, targets: r.targets, rank };
};

test('the star mix and the blue stat: stars cut at the rank mix, the stat uniform over five', () => {
  // a uniform just below and just above each cut point: 1|2 stars at the 1-star share, 2|3 at the 1- and 2-star shares
  const stars = [one - EPS, one + EPS, one + two - EPS, one + two + EPS].map((u) => starDraw(rank, () => u)());
  assert.deepEqual(stars, [1, 2, 2, 3]);
  // the blue stat: stat k + 1 on [k/5, (k+1)/5), then its stars
  for (const [k, cut] of [0.2, 0.4, 0.6, 0.8].entries()) {
    const below = rolledBlueGreen(withSets, setupOf(withSets), script([cut - EPS, STAR[1], STAR[3]]));
    const above = rolledBlueGreen(withSets, setupOf(withSets), script([cut + EPS, STAR[2], STAR[3]]));
    assert.deepEqual([below.blues, above.blues], [{ [k + 1]: 1 }, { [k + 2]: 2 }], `blue stat at ${cut}`);
  }
});

test('her unique spark: always for an outfit with one, by the star mix; none, and no draw, for an outfit without', () => {
  const outfit = withSets.trainee.outfits[0]?.id ?? 0;
  const rnd = script([0.5, STAR[1], STAR[3]]);
  assert.deepEqual(rolledBlueGreen(withSets, setupOf(withSets), rnd).greens, { [outfit]: 3 });
  assert.equal(rnd.used(), 3);
  const none = script([0.5, STAR[1]]);
  const noUnique = rolledBlueGreen(withSets, { ...setupOf(withSets), career: { races: [], uniques: {} } }, none);
  assert.deepEqual(noUnique.greens, {});
  assert.equal(none.used(), 2);
});

test("the landed targets: the set drawn by its chance in `outcome_sets`; over each target's odds, a target at its p", () => {
  const sets = withSets.outcome_sets ?? [];
  // the likeliest set: u lands it on [C(m-1), C(m)), C the cumulative chances, an interval of width sets[m]
  const m = sets.indexOf(Math.max(...sets));
  const before = sets.slice(0, m).reduce((a, b) => a + b, 0),
    after = before + (sets[m] ?? 0);
  const landed = (u: number) =>
    Object.keys(rolledOutcome(withSets, setupOf(withSets), script([u], 0.5)).sparks)
      .map(Number)
      .sort((a, b) => a - b);
  const groups = (mask: number) =>
    withSets.targets
      .filter((_, i) => mask & (1 << i))
      .map((t) => t.group)
      .sort((a, b) => a - b);
  assert.ok(sets[m - 1] && sets[m + 1], 'the sets either side have a chance');
  assert.deepEqual([before - EPS, before + EPS, after - EPS, after + EPS].map(landed), [
    groups(m - 1),
    groups(m),
    groups(m),
    groups(m + 1),
  ]);
  // over each target's own odds: the first target lands below its p and not above
  const [first] = perTarget.targets;
  assert.ok(first && first.p > 0 && first.p < 1);
  const lands = (u: number) => first.group in rolledOutcome(perTarget, setupOf(perTarget), script([u], 0.5)).sparks;
  assert.deepEqual([first.p - EPS, first.p + EPS].map(lands), [true, false]);
});

test("a target's buy: a landed one gold at land_gold / land; one that didn't land bought at (buy - land) / (1 - land), then gold at (gold - land_gold) / (buy - land), so bought at buy and gold at gold in all", () => {
  // the reference career's target bought gold or white whether or not it lands
  const target = withSets.targets.find((t) => t.group === 20165);
  assert.ok(target?.buy, 'target 20165 and its buy');
  const { gold, white, land_gold, land_white } = target.buy;
  const land = land_gold + land_white,
    buy = gold + white;
  const report = { ...withSets, targets: [target] };
  const bought = (sparks: Record<number, number>, us: number[]) => rolledBought(report, sparks, script(us))[20165];
  const landedGold = land_gold / land,
    unlandedBought = (buy - land) / (1 - land),
    unlandedGold = (gold - land_gold) / (buy - land);
  assert.ok(
    [landedGold, unlandedBought, unlandedGold].every((p) => p > 0.5 && p < 1),
    'cut points inside (0, 1)',
  );
  assert.deepEqual(
    [landedGold - EPS, landedGold + EPS].map((u) => bought({ 20165: 2 }, [u])),
    ['gold', 'white'],
  );
  assert.deepEqual(
    [
      bought({}, [unlandedBought - EPS, unlandedGold - EPS]),
      bought({}, [unlandedBought - EPS, unlandedGold + EPS]),
      bought({}, [unlandedBought + EPS]), // not bought: no draw for gold (the script has none)
    ],
    ['gold', 'white', undefined],
  );
  // a skill that never lands or is bought: no buy
  const never = { land_gold: 0, land_white: 0, gold: 0, white: 0 };
  const unlanded = rolledBought(
    { ...withSets, targets: [{ ...target, buy: { ...target.buy, ...never } }] },
    {},
    script([0.5]),
  );
  assert.deepEqual(unlanded, {});
});

test('a G1 she runs: won below its p and not above; run twice, p = 1 - (1 - p1)(1 - p2)', () => {
  const row = (name: string) => {
    const found = withSets.schedule?.find((w) => w.name === name && w.run !== false);
    assert.ok(found, name);
    return found;
  };
  const sprinters = row('Sprinters Stakes'),
    tenno = row('Tenno Sho (Spring)');
  const twice = scheduledRaces(
    {
      schedule: [
        { ...sprinters, name: 'A', p: 0.5, turn: 1, run: true },
        { ...sprinters, name: 'A', p: 0.4, turn: 30, run: true },
        { ...sprinters, name: 'B', p: 0.9, turn: 2, run: false },
        { ...sprinters, name: 'C', p: 0.9, turn: 3, run: true },
      ],
    },
    ['B', 'A'],
  );
  assert.equal(twice.length, 1);
  assert.equal(twice[0]?.ix, 1);
  close(twice[0]?.p, 0.7, 'A run twice at 0.5 and 0.4: 0.7');
  assert.deepEqual(twice[0]?.turns, [1, 30]);
  // the reference career's Sprinters Stakes, run twice (turns 41 and 65), and its Tenno Sho (Spring), once
  const races = ['Sprinters Stakes', 'Tenno Sho (Spring)'];
  const [sp, te] = scheduledRaces(withSets, races);
  assert.deepEqual(sp?.turns, [41, 65]);
  close(sp?.p, 1 - (1 - (sprinters.p ?? 0)) ** 2, 'Sprinters Stakes');
  close(te?.p, tenno.p ?? 0, 'Tenno Sho (Spring)');
  // won when its uniform is below p: without targets or sets, the races are the roll's first draws
  const schedule = withSets.schedule?.filter((w) => races.includes(w.name)) ?? null;
  const bare = { ...withSets, targets: [], outcome_sets: null, schedule };
  const wins = (us: number[]) => rolledOutcome(bare, setupOf(bare, races), script(us, 0.5)).wins;
  for (const [ix, p] of [sp?.p ?? 0, te?.p ?? 0].entries()) {
    const other = 1 - EPS; // the other race lost
    const at = (u: number) => wins(ix === 0 ? [u, other] : [other, u]);
    assert.deepEqual([at(p - EPS), at(p + EPS)], [[ix], []], races[ix]);
  }
});

test("her pink spark: each aptitude's uniform interval is its `pink_gen` chance, a legacy pink fires at its p, stars 20/70/10", () => {
  const ap = withSets.aptitudes;
  assert.ok(ap?.start.kind === 'known');
  const { start, fire } = ap.start;
  // every pink that can fire raises an aptitude she starts at A or S, so the aptitudes she ends at A or S are those
  // she starts at, whatever fires: the draws below with none firing give each pink its whole chance
  assert.ok(
    fire.every((f) => ap.pink_gen.some((g) => g.aptitude === f.aptitude) && gradeAt(start, f.aptitude) >= Grade.A),
  );
  const noFire = fire.map(() => 1 - EPS).concat(fire.map(() => 1 - EPS)); // two April inspirations
  const pinkAt = (u: number) => Object.keys(drawPink(ap, script([...noFire, u, STAR[2]])).pinks).map(Number);
  // the aptitudes with a chance, in `pink_gen` order: aptitude k on [C(k-1), C(k)), C the cumulative chances
  const drawn = ap.pink_gen.filter((g) => g.p > 0);
  let cumulative = 0;
  assert.deepEqual(pinkAt(0), [drawn[0]?.aptitude]);
  drawn.forEach(({ aptitude, p }, k) => {
    cumulative += p;
    const next = drawn[k + 1];
    if (next) assert.deepEqual([pinkAt(cumulative - EPS), pinkAt(cumulative + EPS)], [[aptitude], [next.aptitude]]);
    else assert.deepEqual(pinkAt(1 - EPS), [aptitude]);
  });
  close(cumulative, 1, 'the chances add to 1');
  // a legacy pink fires (+1 grade) below its p and not above: the first, at the first inspiration
  const [first] = fire;
  assert.ok(first);
  const gradeAfter = (u: number) =>
    gradeAt(drawPink(ap, script([u, ...noFire.slice(1), 0.5, 0.5])).apt, first.aptitude);
  assert.deepEqual(
    [gradeAfter(first.p - EPS), gradeAfter(first.p + EPS)],
    [Math.min(Grade.S, gradeAt(start, first.aptitude) + 1), gradeAt(start, first.aptitude)],
  );
  // the stars: 1|2 at 0.2, 2|3 at 0.9
  const stars = [0.2 - EPS, 0.2 + EPS, 0.9 - EPS, 0.9 + EPS].map((u) => pinkAmong([31], script([0.5, u]))[31]);
  assert.deepEqual(stars, [1, 2, 2, 3]);
  // with several starts, the start is the first draw, by its chance: the first on [0, p1)
  const several = {
    ...ap,
    start: {
      kind: 'outcomes' as const,
      outcomes: [
        { p: 0.3, outcome: { start, fire: [] } },
        { p: 0.7, outcome: { start: withGrade(start, 32, Grade.S), fire: [] } },
      ],
    },
  };
  const startAt = (u: number) => gradeAt(drawPink(several, script([u, 0.5, 0.5])).apt, 32);
  assert.deepEqual([startAt(0.3 - EPS), startAt(0.3 + EPS)], [gradeAt(start, 32), Grade.S]);
});
function gradeAt(grades: Readonly<Grades> | null, aptitude: Aptitude): Grade {
  assert.ok(grades);
  return gradeOf(grades, aptitude);
}

test("a whole roll reads its uniforms in order: the landed set, each landed target's stars, the G1s, the buys, the pink's fires, aptitude and stars, the blue stat and stars, the unique's stars", () => {
  const sets = withSets.outcome_sets ?? [];
  const m = sets.indexOf(Math.max(...sets));
  const sprinters = withSets.schedule?.filter((w) => w.name === 'Sprinters Stakes') ?? [];
  const tenno = withSets.schedule?.find((w) => w.name === 'Tenno Sho (Spring)');
  assert.ok(sprinters.length === 2 && tenno);
  const races = ['Sprinters Stakes', 'Tenno Sho (Spring)'];
  const report = { ...withSets, schedule: [sprinters[0], tenno, sprinters[1]].flatMap((w) => (w ? [w] : [])) };
  const ap = withSets.aptitudes;
  assert.ok(ap?.start.kind === 'known');
  const groups = withSets.targets.map((t) => t.group);
  assert.deepEqual(groups, [20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203]);
  assert.equal(m, 0b11001111, 'the likeliest set: all but 20161 and 21010');
  const [fire0, ...fires] = ap.start.fire.map(() => 1 - EPS).concat(ap.start.fire.map(() => 1 - EPS));
  assert.ok(fire0 && ap.start.fire[0]?.aptitude === 32, 'the first pink that can fire is on aptitude 32');
  const rnd = script([
    sets.slice(0, m).reduce((a, b) => a + b, 0) + (sets[m] ?? 0) / 2, // the landed set: m
    ...[STAR[1], STAR[2], STAR[3], STAR[1], STAR[2], STAR[3]], // its targets' stars, in the targets' order
    0.7, // Sprinters Stakes (1 - 0.53^2 = 0.72): won
    0.5, // Tenno Sho (Spring) (0.17): lost
    ...[0.5, 0.5, 0.5, 0.5], // the four landed gold-only targets: gold
    0.5, // 20161, unlanded: bought (0.99)
    0.5, // and white (it is never gold)
    0.99, // 21010, unlanded: not bought (0.93)
    0.99, // 20165, landed: white (gold at 0.986)
    0.5, // 20203, landed: gold (0.857)
    0.1, // the pink on 32 fires at the first inspiration (0.1125): 32 to S
    ...fires, // the others don't
    0.5, // her pink: the fourth of the six she ends at A or S (32, 33, 11, 12, 22, 23)
    0.95, // at 3 stars
    0.45, // the blue stat: 3
    STAR[2], // at 2 stars
    STAR[1], // her unique, at 1 star
  ]);
  const roll = rolledOutcome(report, setupOf(report, races), rnd);
  assert.deepEqual(roll, {
    sparks: { 20159: 1, 20049: 2, 20166: 3, 20046: 1, 20165: 2, 20203: 3 },
    bought: {
      20159: 'gold',
      20049: 'gold',
      20166: 'gold',
      20046: 'gold',
      20161: 'white',
      20165: 'white',
      20203: 'gold',
    },
    wins: [0],
    pinks: { 12: 3 },
    apt: withGrade(ap.start.start, 32, Grade.S),
    blues: { 3: 2 },
    greens: { [withSets.trainee.outfits[0]?.id ?? 0]: 1 },
  });
  assert.equal(rnd.used(), 1 + 6 + 2 + 9 + 2 * ap.start.fire.length + 2 + 2 + 1, 'every scripted uniform read');
});
