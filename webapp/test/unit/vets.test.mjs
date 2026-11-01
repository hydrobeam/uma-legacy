// The veterans store (src/veterans/store.ts) on a Map standing in for localStorage: every mutator copies on write (the
// records it leaves alone stay the same objects, the old ones are never changed), saves before it assigns (a throwing
// effect keeps the write, on the deferred path, the hidden path and the saves made at once), an editor addressing a
// node by where it is lands each of its edits on the record the last one made, and each part's effects wake only for
// that part. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test } from 'vitest';
import { batch } from '@preact/signals-core';
import { ERA } from '../../src/saved.ts';
import { on } from '../../src/store.ts';
import * as vs from '../../src/veterans/store.ts';
import { memory } from './fakes.mjs';

const KEY = 'llp:career',
  OTHER = ERA === 'gm' ? 'global' : 'gm';
const node = (chara, sparks = {}) => ({ chara, sparks, pinks: {}, blues: {}, greens: {}, wins: [] });
const vet = (id, extra = {}) => ({
  id,
  src: 'import',
  borrowed: false,
  ...node(1000 + id.length, { 7: 1 }),
  parents: [node(1, { 7: 2 }), node(2)],
  ...extra,
});
// the store loaded from a storage holding `mine` (this era's part) and another era's part; saves wait for `frame()`
// unless the page is `hidden`
function fresh(mine = { vets: [vet('a'), vet('b'), vet('c')], plan: { trainee: 9, parents: ['a', null] } }) {
  const storage = memory(),
    frames = [],
    host = {
      storage: () => storage,
      hidden: () => host.isHidden,
      afterFrame: (f) => frames.push(f),
      isHidden: false,
    };
  storage.setItem(KEY, JSON.stringify({ v: 2, [ERA]: mine, [OTHER]: { vets: [vet('z')] } }));
  vs.initVets(host);
  const frame = () => {
    for (const f of frames.splice(0)) f();
  };
  const saved = () => JSON.parse(storage.getItem(KEY));
  return { storage, host, frame, saved };
}
// what the store holds now, and a deep copy to check the old records against
const snapshot = () => {
  const now = { vets: vs.vets.peek(), plan: vs.places.peek(), chosen: vs.chosen.peek() };
  return { now, copy: structuredClone(now) };
};
const ids = () => vs.vets.peek().map((v) => v.id);

test('every mutator copies on write: old records unchanged, untouched veterans the same objects, new records frozen', () => {
  const cases = [
    ['addVets', () => vs.addVets([vet('d')]), []],
    ['newVet', () => vs.newVet(5, { 7: 2 }), []],
    ['editVet', () => vs.editVet('b', (v) => ({ sparks: { ...v.sparks, 8: 3 } })), ['b']],
    ['editVet borrowed', () => vs.editVet('b', () => ({ borrowed: true })), ['b']],
    ['parentRef edit', () => vs.parentRef('c', 1).edit((p) => ({ wins: [...p.wins, 4], winAt: { 4: 30 } })), ['c']],
    ['vetRef outfit gone', () => vs.vetRef('a').edit(() => ({ chara: 3, outfit: undefined })), ['a']],
    ['removeVet', () => vs.removeVet('b'), ['b']],
    ['removeVet placed', () => vs.removeVet('a'), ['a']],
    ['clearVets', () => vs.clearVets((v) => v.id !== 'a'), ['b', 'c']],
    ['importVets', () => vs.importVets([{ ...vet('b'), rank: 3 }, vet('e'), vet('tr1', { borrowed: true })]), ['b']],
    ['setListings', () => vs.setListings([vet('u1', { src: 'umamoe', borrowed: true })]), []],
    ['setTrainee', () => vs.setTrainee(4, true), []],
    ['setParent', () => vs.setParent(1, 'c', true), []],
    ['usePlaces', () => vs.usePlaces(4, ['b', 'c']), []],
    ['backToPick', () => vs.backToPick({ trainee: 2, parents: ['c', 'b'] }), []],
    ['clearPlaces', () => vs.clearPlaces(), []],
    ['handBackPlaces', () => vs.handBackPlaces(), []],
    ['setChosen', () => vs.setChosen({ places: 'p', id: 'x', setup: 's' }), []],
  ];
  for (const [name, run, touched] of cases) {
    fresh();
    const { now, copy } = snapshot();
    run();
    assert.deepEqual(now, copy, `${name}: the old record is unchanged`);
    const after = vs.vets.peek();
    for (const v of now.vets)
      if (!touched.includes(v.id) && after.some((x) => x.id === v.id))
        assert.equal(
          after.find((x) => x.id === v.id),
          v,
          `${name}: ${v.id} untouched is the same object`,
        );
    for (const id of touched) {
      const v = after.find((x) => x.id === id);
      if (v)
        assert.notEqual(
          v,
          now.vets.find((x) => x.id === id),
          `${name}: ${id} is a new record`,
        );
    }
    assert.ok(Object.isFrozen(after) && after.every((v) => Object.isFrozen(v) && Object.isFrozen(v.sparks)), name);
    assert.ok(Object.isFrozen(vs.places.peek()) && Object.isFrozen(vs.places.peek().parents), name);
  }
});

test('the mutators: what each writes', () => {
  fresh();
  vs.vetRef('a').edit(() => ({ chara: 3, outfit: undefined }));
  assert.ok(!('outfit' in vs.vetById('a')), 'an undefined field goes');
  vs.removeVet('a');
  assert.deepEqual(vs.places.peek().parents, [null, null], "a removed veteran's place goes back to the planner");
  fresh();
  vs.setListings([vet('u1', { src: 'umamoe' }), vet('u2', { src: 'umamoe' })]);
  vs.setParent(1, 'u1', true);
  vs.setListings([vet('u1', { src: 'umamoe', rank: 7 }), vet('u3', { src: 'umamoe' })]);
  assert.deepEqual(ids(), ['a', 'b', 'c', 'u1', 'u3'], 'a chosen listing stays, the others are replaced');
  assert.equal(vs.vetById('u1').rank, 7, 'a chosen listing listed again takes her new record');
  const r = vs.importVets([{ ...vet('b'), borrowed: true, rank: 2 }, vet('e')]);
  assert.deepEqual(ids(), ['a', 'b', 'c', 'e', 'u1', 'u3'], 'an import goes after your imports, before the listings');
  assert.equal(vs.vetById('b').borrowed, false, 'an updated veteran keeps her tab');
  assert.deepEqual(r, { added: 1, updated: 0, borrowed: 1, dropped: 0 }, 'a borrowed one in the file is borrowed');
  const v = vs.newVet(5, { 7: 2 }, { src: 'entered', outfit: 501 });
  assert.equal(ids().indexOf(v.id), 4, 'a new veteran goes before the listings');
  assert.deepEqual(
    Object.keys(v).slice(-2),
    ['parents', 'outfit'],
    'extra fields after hers, as Object.assign put them',
  );
});

test('saved before assigned: a throwing effect keeps the write, deferred, hidden and at once', () => {
  for (const path of ['deferred', 'hidden', 'handBackPlaces']) {
    const page = fresh();
    page.host.isHidden = path === 'hidden';
    let woken = 0;
    const stop = on([vs.vets, vs.places], () => {
      woken++;
      throw new Error('an effect fails');
    });
    const write = () =>
      path === 'handBackPlaces' ? vs.handBackPlaces() : vs.editVet('b', () => ({ sparks: { 7: 3 } }));
    assert.throws(write, /an effect fails/, path);
    stop();
    page.frame();
    const mine = page.saved()[ERA];
    if (path === 'handBackPlaces') {
      assert.deepEqual(vs.places.peek().parents, [null, null], path);
      assert.deepEqual(mine.plan.parents, [null, null], `${path}: saved`);
    } else {
      assert.deepEqual(vs.vetById('b').sparks, { 7: 3 }, path);
      assert.deepEqual(mine.vets[1].sparks, { 7: 3 }, `${path}: saved`);
    }
    assert.equal(woken, 1, path);
    assert.deepEqual(
      page.saved()[OTHER].vets.map((v) => v.id),
      ['z'],
      `${path}: the other era kept`,
    );
  }
});

test('the deferred save: one write per frame for the edits in between, of the record as it is then', () => {
  const page = fresh();
  let writes = 0;
  const setItem = page.storage.setItem;
  page.storage.setItem = (k, v) => {
    writes++;
    setItem(k, v);
  };
  vs.editVet('a', () => ({ sparks: { 7: 2 } }));
  vs.editVet('a', () => ({ sparks: { 7: 3 } }));
  vs.setTrainee(4, true);
  assert.equal(writes, 0, 'nothing written before the frame');
  page.frame();
  assert.equal(writes, 1);
  assert.equal(page.saved()[ERA].vets[0].sparks[7], 3);
  assert.equal(page.saved()[ERA].plan.trainee, 4);
  vs.flushPersist();
  assert.equal(writes, 1, 'a flush with nothing due writes nothing');
  vs.setListings([vet('u1', { src: 'umamoe' })]);
  page.frame();
  assert.deepEqual(
    page.saved()[ERA].vets.map((v) => v.id),
    ['a', 'b', 'c'],
    'listings are not saved unless chosen',
  );
});

test("an editor by path: an open dialog's second edit lands on the record its first made", () => {
  fresh();
  // the wins window: two edits in one session; the colour picker: a colour then its stars; change uma then her sparks
  const wins = vs.parentRef('b', 0);
  wins.edit((p) => ({ wins: [...p.wins, 3], winAt: { ...p.winAt, 3: 20 } }));
  wins.edit((p) => ({ wins: [...p.wins, 5], winAt: { ...p.winAt, 5: 40 } }));
  assert.deepEqual(vs.vetById('b').parents[0].wins, [3, 5]);
  assert.deepEqual(vs.vetById('b').parents[0].winAt, { 3: 20, 5: 40 });
  const colour = vs.vetRef('c');
  colour.edit(() => ({ blues: { 2: 2 } }));
  colour.edit((v) => ({ blues: { [Object.keys(v.blues)[0]]: 3 } }));
  assert.deepEqual(vs.vetById('c').blues, { 2: 3 });
  const grand = vs.parentRef('a', 1);
  grand.edit(() => ({ chara: 77, outfit: undefined }));
  vs.editVet('a', () => ({ borrowed: true })); // an edit of the same veteran from elsewhere, meanwhile
  grand.edit(() => ({ sparks: { 9: 1 } }));
  const a = vs.vetById('a');
  assert.deepEqual([a.borrowed, a.parents[1].chara, a.parents[1].sparks], [true, 77, { 9: 1 }]);
  assert.equal(grand.node(), a.parents[1], 'the ref reads her record as it is now');
  vs.removeVet('a');
  grand.edit(() => ({ sparks: {} }));
  assert.equal(grand.node(), undefined, 'an edit of one gone does nothing');
});

test("effects wake for their part only; a places write keeps the veterans' identity and the job's legacies", () => {
  fresh();
  const woke = { vets: 0, places: 0, chosen: 0 };
  const parts = { vets: vs.vets, places: vs.places, chosen: vs.chosen },
    stops = Object.keys(woke).map((k) => on([parts[k]], () => woke[k]++));
  const expect = (name, run, want) => {
    for (const k of Object.keys(woke)) woke[k] = 0;
    run();
    assert.deepEqual(woke, want, name);
  };
  const legacies = vs.legaciesJob(),
    vetsBefore = vs.vets.peek();
  expect('setTrainee', () => vs.setTrainee(3, false), { vets: 0, places: 1, chosen: 0 });
  assert.equal(vs.vets.peek(), vetsBefore);
  assert.equal(vs.legaciesJob(), legacies, 'the job legacies are the same array');
  expect('setChosen', () => vs.setChosen({ places: 'p', id: 'x', setup: 's' }), { vets: 0, places: 0, chosen: 1 });
  expect('editVet', () => vs.editVet('b', () => ({ rank: 3 })), { vets: 1, places: 0, chosen: 0 });
  const after = vs.legaciesJob();
  assert.notEqual(after, legacies);
  assert.equal(after[0], legacies[0], "an edit keeps the other veterans' job records");
  assert.notEqual(after[1], legacies[1]);
  expect('removeVet unplaced', () => vs.removeVet('c'), { vets: 1, places: 0, chosen: 0 });
  expect('removeVet placed', () => vs.removeVet('a'), { vets: 1, places: 1, chosen: 0 });
  expect('usePlaces', () => vs.usePlaces(2, ['b', null]), { vets: 0, places: 1, chosen: 1 });
  expect('dropMissing with none missing', () => vs.dropMissing(), { vets: 0, places: 0, chosen: 0 });
  expect(
    'a batch: a new veteran and the places cleared',
    () =>
      batch(() => {
        vs.newVet(1, {});
        vs.clearPlaces();
      }),
    { vets: 1, places: 1, chosen: 0 },
  );
  for (const stop of stops) stop();
});

test('loading: a v1 save, a damaged veteran and a damaged choice left out, blocked storage', () => {
  const storage = memory(),
    host = { storage: () => storage, hidden: () => false, afterFrame: () => undefined };
  storage.setItem(KEY, JSON.stringify({ v: 1, [ERA]: [vet('a'), { id: 'x', parents: [1] }] }));
  vs.initVets(host);
  assert.deepEqual(ids(), ['a']);
  assert.deepEqual(vs.places.peek(), { trainee: null, parents: [null, null] });
  storage.setItem(KEY, JSON.stringify({ v: 2, [ERA]: { vets: [{ id: 'n', parents: [{}, {}] }], chosen: { id: 1 } } }));
  vs.initVets(host);
  assert.deepEqual(vs.vetById('n').parents[0], { chara: null, sparks: {}, pinks: {}, wins: [] });
  assert.equal(vs.chosen.peek(), null);
  vs.initVets({
    storage: () => {
      throw new Error('blocked');
    },
    hidden: () => true,
    afterFrame: () => undefined,
  });
  assert.deepEqual(ids(), []);
  vs.addVets([vet('a')]); // (a blocked save: the edit lasts for the visit)
  assert.deepEqual(ids(), ['a']);
});

test("the analysis' draft: her own signal, edits by path copy on write, the veterans' effects asleep", () => {
  fresh();
  const woke = { vets: 0, draft: 0 },
    stops = [on([vs.vets], () => woke.vets++), on([vs.draft], () => woke.draft++)];
  const d = { ...node(5, { 7: 1 }), borrowed: false, parents: [node(1), node(2, { 8: 2 })] };
  vs.setDraft(d);
  const first = vs.draft.peek();
  assert.ok(Object.isFrozen(first) && Object.isFrozen(first.parents[1].sparks), 'a draft set is frozen');
  vs.draftRef().edit((n) => ({ sparks: { ...n.sparks, 9: 3 } }));
  vs.draftRef().edit((n) => ({ wins: [...n.wins, 4] }));
  vs.draftParentRef(1).edit((p) => ({ sparks: { ...p.sparks, 8: 3 } }));
  vs.draftParentRef(1).edit(() => ({ chara: 33, outfit: undefined }));
  const now = vs.draft.peek();
  assert.deepEqual(
    [now.sparks, now.wins, now.parents[1].sparks, now.parents[1].chara],
    [{ 7: 1, 9: 3 }, [4], { 8: 3 }, 33],
  );
  assert.deepEqual(first.sparks, { 7: 1 }, 'the first record is unchanged');
  assert.equal(now.parents[0], first.parents[0], 'an untouched parent is the same object');
  assert.deepEqual(woke, { vets: 0, draft: 5 });
  vs.setDraft(null);
  vs.draftRef().edit(() => ({ sparks: {} }));
  assert.equal(vs.draft.peek(), null, 'an edit with no draft does nothing');
  for (const stop of stops) stop();
});

test("a draft set, or her parents set, is the store's own copy: the caller's object stays its own (review-10 F6)", () => {
  const mine = { ...node(5, { 7: 1 }), borrowed: false, parents: [node(1), node(2)] };
  vs.setDraft(mine);
  mine.sparks[7] = 3; // (frozen in place, this would throw)
  assert.equal(vs.draft.peek().sparks[7], 1, 'her draft is not the caller object');
  const parents = [node(3, { 8: 1 }), node(4)];
  vs.setDraftParents(parents);
  parents[0].sparks[8] = 2;
  assert.equal(vs.draft.peek().parents[0].sparks[8], 1);
  assert.ok(Object.isFrozen(vs.draft.peek().parents[0]));
  vs.setDraft(null);
});

// review-10 2's surviving sabotages: each of these fails when its store line is undone
test('saves: a chosen uma.moe listing is saved; the hidden page saves at once; an at-once save writes the choice it makes', () => {
  // a listing chosen as a parent is saved with the places (the place keeps her); the others are not
  const page = fresh();
  vs.setListings([vet('u1', { src: 'umamoe', borrowed: true }), vet('u2', { src: 'umamoe', borrowed: true })]);
  vs.setParent(1, 'u1', true);
  page.frame();
  assert.deepEqual(
    page.saved()[ERA].vets.map((v) => v.id),
    ['a', 'b', 'c', 'u1'],
    'the chosen listing saved, the other not',
  );
  // hidden (the tab in the background, a close coming): saved before any frame
  const hidden = fresh();
  hidden.host.isHidden = true;
  vs.editVet('b', () => ({ sparks: { 7: 3 } }));
  assert.deepEqual(hidden.saved()[ERA].vets[1].sparks, { 7: 3 }, 'hidden: saved at once, no frame');
  // an at-once save (the places handed back) writes the record it makes: the choice cleared, not the one before
  const chosen = fresh();
  vs.setChosen({ places: '{}', id: 'x', setup: '' });
  chosen.frame();
  assert.equal(chosen.saved()[ERA].chosen?.id, 'x', 'the choice saved');
  vs.handBackPlaces();
  assert.equal(chosen.saved()[ERA].chosen, null, 'handed back: the choice cleared in the save, at once');
});

test('ids given twice (a malformed file or save): the first kept, so an edit and Remove find one record (review-10 5)', () => {
  const warned = [],
    warn = console.warn;
  console.warn = (text) => warned.push(text);
  try {
    fresh({
      vets: [vet('a'), vet('a', { sparks: { 7: 3 } }), vet('b')],
      plan: { trainee: null, parents: [null, null] },
    });
    assert.deepEqual(ids(), ['a', 'b'], 'loaded once each');
    assert.equal(vs.vetById('a').sparks[7], 1, 'the first kept');
    vs.importVets([vet('n'), vet('n', { sparks: { 7: 2 } })]);
    assert.deepEqual(ids(), ['a', 'b', 'n'], 'an import holding one twice adds her once');
    vs.editVet('n', () => ({ sparks: { 7: 3 } }));
    vs.removeVet('a');
    assert.deepEqual(
      vs.vets.peek().map((v) => [v.id, v.sparks[7]]),
      [
        ['b', 1],
        ['n', 3],
      ],
    );
    assert.equal(warned.length, 2, 'each logged');
  } finally {
    console.warn = warn;
  }
});
