// The settings store (src/settings.ts) and the effect helper (src/store.ts `on`), on a Map standing in for localStorage:
// each setter copies on write and persists before it assigns; an effect wakes only for the signals it names; what is
// stored loads back, old shapes included. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test } from 'vitest';
import { ERA } from '../../src/saved.ts';
import {
  DEFAULTS,
  fields,
  importSettings,
  importToEra,
  initStore,
  jobSetup,
  setAuto,
  setBorrowPar,
  setDecksTop,
  setOwned,
  setPerUma,
  setPresets,
  setRank,
  setScenario,
  setSort,
  setTargets,
  settings,
  switchEra,
} from '../../src/settings.ts';
import { on } from '../../src/store.ts';

const KEY = 'llp:settings',
  OTHER = ERA === 'gm' ? 'global' : 'gm';
// a storage holding `saved` (an object, stored as JSON, or a string, stored as it is)
const memory = (saved) => {
  const map = new Map(saved === undefined ? [] : [[KEY, typeof saved === 'string' ? saved : JSON.stringify(saved)]]);
  return { getItem: (key) => map.get(key) ?? null, setItem: (key, value) => void map.set(key, value) };
};
const stored = (storage) => JSON.parse(storage.getItem(KEY));
// the store loaded from a new storage holding `saved`; returns that storage
const fresh = (saved) => {
  const storage = memory(saved);
  initStore(() => storage);
  return storage;
};
const { rcards: _, ...SAVED_DEFAULTS } = DEFAULTS; // rcards is never saved, so never loaded

// each setter: what it is called with, and the fields it replaces
const SETTERS = [
  ['setTargets', () => setTargets([{ group: 1 }]), ['targets']],
  ['setPresets', () => setPresets([{ name: 'a', groups: [1, 2] }]), ['presets']],
  ['setRank', () => setRank('0.20,0.70,0.10'), ['rank']],
  ['setScenario', () => setScenario({ scen: 'grandmasters', rank: '0.20,0.70,0.10' }), ['scen', 'rank']],
  ['setAuto', () => setAuto(false), ['auto']],
  ['setDecksTop', () => setDecksTop('3'), ['decksTop']],
  ['setPerUma', () => setPerUma(false), ['perUma']],
  ['setBorrowPar', () => setBorrowPar('never'), ['borrowPar']],
  ['setSort (umas)', () => setSort({ prefix: 'r', sort: 'name', dir: 'asc' }), ['rsort', 'rdir']],
  ['setSort (cards)', () => setSort({ prefix: 'c', sort: 'name', dir: 'asc' }), ['csort', 'cdir']],
  ['setOwned (cards)', () => setOwned({ cardsOff: { 30116: false } }), ['cardsOff']],
  ['setOwned (umas)', () => setOwned({ roster: { 1001: 'out' }, outfits: { 100201: false } }), ['roster', 'outfits']],
  // an import replaces what it carries, and the umas' and outfits' marks (it drops "must include")
  [
    'importSettings',
    () => importSettings({ targets: [{ group: 2 }], roster: { 1001: 'req' } }),
    ['targets', 'roster', 'outfits'],
  ],
];
const JOB = ['targets', 'rank', 'scen', 'roster', 'outfits', 'cardsOff', 'decksTop', 'perUma', 'borrowPar'];

test('each setter copies on write: the old record is unchanged, the new one shares every field it did not replace', () => {
  for (const [name, call, replaced] of SETTERS) {
    const storage = fresh();
    const before = settings.value,
      copy = structuredClone(before);
    call();
    const after = settings.value;
    assert.deepEqual(before, copy, `${name}: the old record is unchanged`);
    assert.notEqual(after, before, `${name}: a new record`);
    assert.deepEqual(Object.keys(after), Object.keys(before), `${name}: the same settings, in the same order`);
    for (const key of Object.keys(before))
      if (replaced.includes(key)) assert.notEqual(after[key], before[key], `${name}: ${key} replaced`);
      else assert.equal(after[key], before[key], `${name}: ${key} is the same object`);
    // and it is saved: the shared settings and this era's marks, as the record has them
    const saved = stored(storage);
    for (const key of replaced)
      assert.deepEqual(saved[key in saved.common ? 'common' : ERA][key], after[key], `${name}: ${key} saved`);
  }
});

test('a write is persisted before it is assigned: an effect that throws keeps it, and every effect still runs', () => {
  const storage = fresh();
  let others = 0;
  const stops = [
    on([fields.targets], () => {
      throw new Error('boom');
    }),
    on([fields.targets], () => others++),
  ];
  assert.throws(() => setTargets([{ group: 7 }]), /boom/);
  assert.deepEqual(stored(storage).common.targets, [{ group: 7 }], 'saved');
  assert.deepEqual(settings.value.targets, [{ group: 7 }], 'assigned');
  assert.equal(others, 1, 'the other effect ran');
  assert.throws(() => setTargets([{ group: 8 }]), /boom/, 'the throwing effect is still live');
  assert.equal(others, 2);
  for (const stop of stops) stop();
});

test('an effect runs only when a signal it names changes, never on an unrelated setter or for what its body reads', () => {
  const runs = {};
  const count = (name) => () => {
    runs[name] = (runs[name] ?? 0) + 1;
  };
  const effects = () => [
    ...Object.entries(fields).map(([key, field]) => on([field], count(key))),
    on([jobSetup], count('jobSetup')),
    // its body reads the whole record, which must not subscribe it to anything
    on([fields.presets], () => {
      count('reads everything')();
      void JSON.stringify(settings.value);
      for (const field of Object.values(fields)) void field.value;
    }),
  ];
  for (const [name, call, replaced] of SETTERS) {
    fresh();
    const stops = effects();
    assert.deepEqual(runs, {}, `${name}: no effect runs when it is made`);
    call();
    const expected = Object.fromEntries(replaced.map((key) => [key, 1]));
    if (replaced.some((key) => JOB.includes(key))) expected.jobSetup = 1;
    if (replaced.includes('presets')) expected['reads everything'] = 1;
    assert.deepEqual(runs, expected, name);
    for (const stop of stops) stop();
    for (const key of Object.keys(runs)) delete runs[key];
  }
  // a primitive set to the value it has is no change
  fresh();
  const stops = effects();
  setRank(settings.value.rank);
  assert.deepEqual(runs, {}, 'setRank to the same rank');
  // once the body that reads the whole record has run, setters of other fields still don't wake it: what it read
  // subscribed it to nothing (`on` runs it untracked)
  setPresets([{ name: 'a', groups: [1, 2] }]);
  setRank('0.20,0.70,0.10');
  setSort({ prefix: 'r', sort: 'name', dir: 'asc' });
  setAuto(!settings.value.auto);
  assert.equal(runs['reads everything'], 1, 'reads everything: after other setters');
  for (const stop of stops) stop();
});

test('an era switch and an import for the other era write storage only: the record and its effects stay', () => {
  const storage = fresh({
    v: 2,
    era: ERA,
    common: {},
    [ERA]: {},
    [OTHER]: { roster: { 1001: 'out' }, cardsOff: { 1: false } },
  });
  let runs = 0;
  const stop = on([settings], () => runs++);
  const before = settings.value;
  switchEra(OTHER, { scen: 'grandmasters' });
  assert.equal(stored(storage).era, OTHER);
  assert.equal(stored(storage).common.scen, 'grandmasters');
  importToEra(OTHER, { targets: [{ group: 5 }], cardsOff: { 2: false } });
  const saved = stored(storage);
  assert.deepEqual(saved.common.targets, [{ group: 5 }], 'its shared settings over these');
  assert.equal(saved.common.rank, before.rank, 'the shared settings it does not carry stay');
  assert.deepEqual(
    saved[OTHER],
    { roster: { 1001: 'out' }, cardsOff: { 2: false } },
    "its marks over that era's stored",
  );
  assert.deepEqual(saved[ERA], { roster: {}, outfits: {}, cardsOff: {} }, "this era's marks stay");
  assert.equal(settings.value, before);
  assert.equal(runs, 0);
  stop();
});

test('storage round trip: what is saved loads back; old and broken shapes load as the defaults', () => {
  fresh();
  assert.deepEqual(settings.value, SAVED_DEFAULTS, 'nothing stored: the defaults');
  fresh({ v: 1, common: { rank: 'x' } });
  assert.deepEqual(settings.value, SAVED_DEFAULTS, 'another version: the defaults');
  fresh('{not json');
  assert.deepEqual(settings.value, SAVED_DEFAULTS, 'unreadable: the defaults');
  initStore(() => {
    throw new Error('blocked');
  });
  assert.deepEqual(settings.value, SAVED_DEFAULTS, 'blocked storage: the defaults');
  setAuto(false);
  assert.equal(settings.value.auto, false, 'blocked storage: a setting lasts for the visit');

  // version 2 with "must include" marks and a setting missing
  const storage = fresh({
    v: 2,
    era: OTHER, // another tab switched the era: this tab's writes keep it
    common: { targets: [{ group: 3 }], rank: '0.20,0.70,0.10' },
    [ERA]: { roster: { 1001: 'req', 1002: 'out' }, outfits: { 100101: 'req', 100201: false } },
    [OTHER]: { cardsOff: { 9: false } },
  });
  assert.deepEqual(settings.value, {
    ...SAVED_DEFAULTS,
    targets: [{ group: 3 }],
    rank: '0.20,0.70,0.10',
    roster: { 1002: 'out' },
    outfits: { 100201: false },
  });
  for (const [, call] of SETTERS) call();
  const written = settings.value,
    saved = stored(storage);
  assert.equal(saved.era, OTHER, "another tab's era");
  assert.deepEqual(saved[OTHER], { cardsOff: { 9: false } }, "the other era's marks");
  initStore(() => storage);
  assert.deepEqual(settings.value, written, 'loads back as written');
});

test('every record is frozen all the way down, DEFAULTS too, and no record shares an object of DEFAULTS', () => {
  fresh(); // (nothing saved: every setting its default)
  for (const key of ['targets', 'presets', 'roster', 'outfits', 'cardsOff'])
    assert.notEqual(settings.value[key], DEFAULTS[key], `${key}: a copy of the default`);
  assert.notEqual(settings.value.presets[0].groups, DEFAULTS.presets[0].groups);
  // writes past the read-only type: Object.assign (TS lets it through on a readonly target) and what a cast allows
  assert.throws(() => Object.assign(settings.value.cardsOff, { 1: false }), TypeError);
  assert.throws(() => settings.value.targets.push({ group: 1 }), TypeError);
  assert.throws(() => {
    settings.value.presets[0].name = 'renamed';
  }, TypeError);
  assert.throws(() => DEFAULTS.targets.push({ group: 1 }), TypeError);
  assert.throws(() => {
    DEFAULTS.roster[1] = 'out';
  }, TypeError);
  // a setter's record, its new field included, and an import's
  setTargets([{ group: 7 }]);
  assert.throws(() => {
    settings.value.targets[0].group = 8;
  }, TypeError);
  importSettings({ roster: { 3: 'out' } });
  assert.throws(() => Object.assign(settings.value.roster, { 4: 'out' }), TypeError);
  assert.deepEqual(settings.value.roster, { 3: 'out' });
});
