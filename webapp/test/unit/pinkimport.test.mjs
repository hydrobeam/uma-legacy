// A pink spark of an aptitude the solver doesn't know never reaches a job: the game's export (umadump.ts `parseExport`)
// leaves it out of her record and counts it, a stored or pasted veteran's pink map keeps only the solver's aptitudes
// with 1 to 3 stars (veterans/records.ts `knownPinks`, read by veterans/store.ts `asVeteran`), and a job carries her
// pinks as the solver's records (`jobNode`). A stored veteran's grades saved as a list read as the solver's `Grades`.
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import { loadData } from '../../src/data.ts';
import { parseExport } from '../../src/umadump.ts';
import { jobNode, knownPinks, unknownPinksIn } from '../../src/veterans/records.ts';
import { asVeteran } from '../../src/veterans/store.ts';
import { Aptitude } from '../../src/gen/solver.ts';

vi.mock('#page-data', () => import('./fakepage.mjs'));
await loadData({ blues: {}, pinks: {}, uniques: {}, races: [], mobs: [], sourced: [], factors: {}, saddles: {} });

// factor_id = factor group x 100 + stars: 3202 a mile pink at 2 stars, 1503 a pink of group 15 (no such aptitude)
const MILE_2 = 3202,
  UNKNOWN_3 = 1503;
const exported = (factors) =>
  JSON.stringify([
    {
      card_id: 100101,
      owner_viewer_id: 0,
      rank_score: 1,
      trained_chara_id: 7,
      factor_info_array: factors.map((factor_id) => ({ factor_id })),
      win_saddle_id_array: [],
      succession_chara_array: [
        { position_id: 10, card_id: 100201, factor_info_array: [{ factor_id: UNKNOWN_3 }], win_saddle_id_array: [] },
      ],
    },
  ]);

test("the game's export leaves out a pink of an unknown aptitude and counts it", () => {
  const read = parseExport(exported([MILE_2, UNKNOWN_3]));
  assert.equal(read.veterans.length, 1, 'one veteran');
  const [her] = read.veterans;
  assert.deepEqual(her.pinks, { [Aptitude.Mile]: 2 }, 'her known pink kept');
  assert.deepEqual(her.parents[0].pinks, {}, "her parent's unknown pink left out");
  assert.equal(read.unknownPinks, 2, 'one of hers and one of her parent');
  assert.deepEqual(parseExport(exported([MILE_2])).unknownPinks, 1, "the parent's alone");
});

test('a stored or pasted pink map keeps the solver aptitudes with 1 to 3 stars', () => {
  assert.deepEqual(knownPinks({ [Aptitude.Turf]: 3, 15: 2, [Aptitude.Long]: 4, [Aptitude.Dirt]: 0 }), {
    pinks: { [Aptitude.Turf]: 3 },
    unknown: 3,
  });
  assert.deepEqual(knownPinks('not a map'), { pinks: {}, unknown: 0 }, 'no map: none');
  const raw = {
    id: 'h1',
    src: 'hand',
    borrowed: false,
    chara: 1001,
    sparks: {},
    pinks: { 15: 2, [Aptitude.Mile]: 1 },
    wins: [],
    // her grades as saves wrote them before grades had kinds: the distances, the surfaces, the styles
    apt: [7, 6, 5, 4, 8, 1, 2, 3, 4, 5],
    parents: [
      { chara: 1002, sparks: {}, pinks: { 99: 1 }, wins: [] },
      { chara: null, sparks: {}, pinks: {}, wins: [] },
    ],
  };
  assert.equal(unknownPinksIn(raw), 2, 'one of hers, one of a parent');
  const v = asVeteran(raw);
  assert.deepEqual(v.pinks, { [Aptitude.Mile]: 1 }, 'her known pink kept');
  assert.deepEqual(v.parents[0].pinks, {}, 'the unknown left out');
  assert.deepEqual(v.apt, { distance: [7, 6, 5, 4], surface: [8, 1], style: [2, 3, 4, 5] }, 'grades by kind');
  assert.deepEqual(jobNode(v).pinks, [{ aptitude: Aptitude.Mile, stars: 1 }], 'the job carries the solver record');
});
