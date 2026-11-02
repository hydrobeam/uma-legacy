// A veteran is complete when she has exactly one pink and each parent place holds her parent (an uma and one pink) or
// is absent (veterans/complete.ts). An incomplete one is kept and shown, never sent: the job's legacies are the complete
// veterans alone (veterans/store.ts `readyVets`), and a parent place holding her blocks the job (career/job.ts
// `buildJob`). Parents an earlier page made up for a hand-entered veteran (no uma, her sparks copied) are flagged, not
// discarded; a parent the game's export lacks is absent, neither made up nor a lack. The parent picker's "A new one"
// (veterans/entry.ts) asks for her sparks and pink, then each parent's uma, sparks and pink.
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import { loadData } from '../../src/data.ts';
import { parseExport } from '../../src/umadump.ts';
import { initStore, settings } from '../../src/settings.ts';
import * as vs from '../../src/veterans/store.ts';
import { isComplete, lacks, lacksSaid } from '../../src/veterans/complete.ts';
import { enterVeteran } from '../../src/veterans/entry.ts';
import { colourClosed, colourEditing } from '../../src/veterans/sparks.ts';
import { skillsAsked, skillsChosen, umaPicked, umaPicking } from '../../src/veterans/pickers.ts';
import { Blocked, buildJob } from '../../src/career/job.ts';
import { ERA } from '../../src/saved.ts';
import { memory } from './fakes.mjs';

vi.mock('#page-data', () => import('./fakepage.mjs'));
await loadData({ blues: {}, pinks: {}, uniques: {}, races: [], mobs: [], sourced: [], factors: {}, saddles: {} });
initStore(() => memory());

const TURF_1 = { 11: 1 };
const node = (chara, pinks = TURF_1, sparks = {}) => ({ chara, sparks, pinks, blues: {}, greens: {}, wins: [] });
const vet = (id, extra = {}) => ({
  id,
  src: 'import',
  borrowed: false,
  ...node(1000 + id.charCodeAt(0)),
  parents: [node(1), node(2)],
  ...extra,
});
// the store loaded from a storage holding `list` (saves at once)
function load(list) {
  const storage = memory();
  storage.setItem(
    'llp:career',
    JSON.stringify({ v: 2, [ERA]: { vets: list, plan: { trainee: null, parents: [null, null] } } }),
  );
  vs.initVets({ storage: () => storage, hidden: () => true, afterFrame: (f) => f() });
}
const readyIds = () => vs.readyVets.peek().map((v) => v.id);

test('without exactly one pink she is incomplete: kept and shown, left out of the job until she has one', () => {
  load([vet('a'), vet('b', { pinks: {} }), vet('c', { pinks: { 11: 1, 33: 2 } }), vet('d')]);
  assert.deepEqual(
    vs.vets.peek().map((v) => v.id),
    ['a', 'b', 'c', 'd'],
    'every veteran kept',
  );
  assert.deepEqual(lacks(vs.vetById('b')), [{ kind: 'pink' }], 'no pink: her pink lacks');
  assert.deepEqual(lacks(vs.vetById('c')), [{ kind: 'pink' }], 'two pinks: not the one the game gives');
  assert.deepEqual(readyIds(), ['a', 'd'], 'the complete ones are the ready ones');
  assert.equal(vs.legaciesJob().length, 2, 'the job takes the complete ones alone');
  vs.vetRef('b').edit(() => ({ pinks: { 32: 2 } }));
  assert.ok(isComplete(vs.vetById('b')), 'her pink entered, she is complete');
  assert.deepEqual(readyIds(), ['a', 'b', 'd'], 'and joins the job in her place in the pool');
});

test('a parent without her pink, or not entered, makes her incomplete', () => {
  load([vet('a', { parents: [node(1, {}), node(2)] })]);
  assert.deepEqual(lacks(vs.vetById('a')), [{ kind: 'parent pink', j: 0 }]);
  assert.equal(lacksSaid(lacks(vs.vetById('a'))), "parent 1's pink");
});

test('parents made up by an earlier page are flagged incomplete, never discarded, until entered', () => {
  // (the old "A new one": her parents with no uma and copies of her sparks)
  const sparks = { 20159: 2, 21010: 2 },
    phantom = { chara: null, sparks: { ...sparks }, pinks: {}, blues: {}, greens: {}, wins: [] };
  load([vet('a'), vet('b'), { ...vet('h', { src: 'hand', sparks }), parents: [phantom, { ...phantom }] }]);
  const her = vs.vetById('h');
  assert.ok(her, 'she is kept');
  assert.deepEqual(her.parents[0].sparks, sparks, 'as stored');
  assert.deepEqual(
    lacks(her),
    [
      { kind: 'parent', j: 0 },
      { kind: 'parent', j: 1 },
    ],
    'both parents not entered',
  );
  assert.deepEqual(readyIds(), ['a', 'b'], 'not sent');
  for (const j of [0, 1]) vs.parentRef('h', j).edit(() => ({ chara: 1010 + j, pinks: TURF_1 }));
  assert.deepEqual(readyIds(), ['a', 'b', 'h'], 'her parents entered, she is sent');
});

test("a parent the game's export lacks is absent: no one in the job, and no lack", () => {
  const read = parseExport(
    JSON.stringify([
      {
        card_id: 100101,
        trained_chara_id: 7,
        factor_info_array: [{ factor_id: 1101 }],
        win_saddle_id_array: [],
        succession_chara_array: [
          { position_id: 10, card_id: 100201, factor_info_array: [{ factor_id: 3302 }], win_saddle_id_array: [] },
        ],
      },
    ]),
  );
  const [her] = read.veterans;
  assert.equal(her.parents[1], null, 'her second parent is absent');
  assert.deepEqual(lacks(her), [], 'and she lacks nothing');
  load([her, vet('b')]);
  assert.equal(vs.vetById(her.id).parents[1], null, 'saved and loaded absent');
  const [job] = vs.legaciesJob();
  assert.deepEqual(job.parents[1], { chara: 0, sparks: [], pinks: [], wins: [] }, 'the job reads no one there');
  // a save from before: an imported veteran's missing parent stored as a node with no uma reads absent
  load([{ ...vet('a'), parents: [node(1), { chara: null, sparks: {}, pinks: {}, wins: [] }] }]);
  assert.equal(vs.vetById('a').parents[1], null);
  assert.ok(isComplete(vs.vetById('a')));
});

test('a parent place holding an incomplete veteran blocks the job; legacy indices count the complete ones', () => {
  load([vet('a', { pinks: {} }), vet('b'), vet('c')]);
  const from = (parents) => ({
    settings: { ...settings.peek(), targets: [{ group: 20159 }] },
    vets: vs.vets.peek(),
    ready: vs.readyVets.peek(),
    legacies: vs.legaciesJob(),
    places: { trainee: null, parents },
    chosen: null,
  });
  const acts = { importVets: () => undefined, trySamples: () => undefined };
  assert.throws(
    () => buildJob(from(['a', null]), acts),
    (e) => e instanceof Blocked && /Parent 1 .* is incomplete: missing pink/.test(e.message),
  );
  const job = buildJob(from([null, 'c']), acts);
  assert.equal(job.career.legacies.length, 2, 'her two complete veterans');
  assert.deepEqual(job.career.fix, [1], 'c is legacy 1: the second complete veteran');
});

test('"A new one": her sparks and pink, then each parent\'s uma, sparks and pink; dismissed, she stays incomplete', async () => {
  load([vet('a'), vet('b')]);
  const tick = () => new Promise((resolve) => setTimeout(resolve));
  enterVeteran(1001, 0);
  const id = vs.places.peek().parents[0],
    her = () => vs.vetById(id);
  assert.ok(her(), 'she joins the pool in the place she was entered for');
  assert.deepEqual(
    lacks(her()).map((x) => x.kind),
    ['pink', 'parent', 'parent'],
  );
  // her sparks, her pink
  assert.ok(skillsAsked.peek()?.open);
  skillsChosen({ 20159: 2 });
  assert.equal(colourEditing.peek()?.open, true, 'the colour picker opens for her pink');
  colourEditing.peek().at.edit(() => ({ pinks: { 33: 2 } }));
  colourClosed();
  await tick();
  // each parent: her uma (never hers, nor the other parent's), her sparks, her pink
  for (const [j, chara] of [
    [0, 1002],
    [1, 1003],
  ]) {
    const pick = umaPicking.peek();
    assert.ok(pick?.open, `parent ${j + 1}: the uma picker opens`);
    assert.ok(!pick.only({ id: 1001 }), "her own uma isn't offered");
    if (j === 1) assert.ok(!pick.only({ id: 1002 }), "nor the other parent's");
    umaPicked(chara);
    skillsChosen({ 20159: 3 });
    colourEditing.peek().at.edit(() => ({ pinks: { 11: 1 } }));
    colourClosed();
    await tick();
  }
  const done = her();
  assert.deepEqual(
    [done.sparks, done.pinks, done.parents.map((p) => [p.chara, p.sparks, p.pinks])],
    [
      { 20159: 2 },
      { 33: 2 },
      [
        [1002, { 20159: 3 }, { 11: 1 }],
        [1003, { 20159: 3 }, { 11: 1 }],
      ],
    ],
    'she and her parents as entered: no sparks made up',
  );
  assert.ok(isComplete(done) && readyIds().includes(id), 'complete, and sent');
  // dismissed part-way: the colour picker shut with no pink ends the entry
  enterVeteran(1004, 1);
  const second = vs.places.peek().parents[1];
  skillsChosen({});
  colourClosed();
  await tick();
  assert.equal(umaPicking.peek()?.open, false, 'no next step');
  assert.deepEqual(
    lacks(vs.vetById(second)).map((x) => x.kind),
    ['pink', 'parent', 'parent'],
    'she stays, incomplete',
  );
});
