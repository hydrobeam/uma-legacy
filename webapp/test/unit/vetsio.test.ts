// @vitest-environment happy-dom
// The career page's veterans as imports and removals change them (career/io.ts, veterans/store.ts): an uma.moe tree
// adds her with her parents after your veterans, marked yours; a paste that is not an export says so and changes
// nothing; Export leaves uma.moe's listings out; removing a fixed parent hands her place back; a place naming a veteran
// not here goes back to the planner; a career taken from the table goes back to the planner, saved at once; a save
// still due goes out when the page is hidden or left; another tab's write to the other era survives this tab's, and a
// tab on the other era keeps the era chosen since.
// npx vitest run (in webapp)
import { expect, test, vi } from 'vitest';
import { loadData } from '../../src/model/data.ts';
import { ERA, loadSaved, writeSaved } from '../../src/state/saved.ts';
import { parseListing } from '../../src/umadump.ts';
import * as vs from '../../src/veterans/store.ts';
import type { LineageNode, Veteran } from '../../src/veterans/store.ts';
import { exportText, importText } from '../../src/career/io.ts';
import { vetsNote } from '../../src/career/view/poolState.ts';
import { draftJoins } from '../../src/career/analysis/state.ts';
import { CAREER, RECORDS, listing } from '../browser/listings.ts';
import { memory } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));
await loadData(CAREER);

const KEY = 'llp:career';
const OTHER = ERA === 'gm' ? 'global' : 'gm';
const node = (chara: number, sparks: Record<number, number> = {}): LineageNode => ({
  chara,
  sparks,
  pinks: { 11: 1 },
  blues: {},
  greens: {},
  wins: [],
});
const vet = (id: string, extra: Partial<Veteran> = {}): Veteran => ({
  id,
  src: 'import',
  borrowed: false,
  ...node(1001, { 20159: 1 }),
  parents: [node(1002), node(1003)],
  ...extra,
});
// the store on a fresh storage holding `mine` (this era's part) and another era's part; saves wait for `frame()`
function fresh(mine: object = { vets: [vet('a'), vet('b'), vet('c')], plan: { trainee: null, parents: ['a', null] } }) {
  const storage = memory();
  const frames: (() => void)[] = [];
  const host = { storage: () => storage, hidden: () => false, afterFrame: (f: () => void) => void frames.push(f) };
  storage.setItem(KEY, JSON.stringify({ v: 2, [ERA]: mine, [OTHER]: { vets: [vet('z')] } }));
  vs.initVets(host);
  const frame = () => {
    for (const f of frames.splice(0)) f();
  };
  const saved = () => JSON.parse(storage.getItem(KEY) ?? 'null');
  return { storage, frame, saved };
}
const ids = () => vs.vets.peek().map((v) => v.id);

// uma.moe's lineage planner export: one veteran with her parents (factorId x 10 + level is the game's factor id: Uma
// Stan 2, Racing Spirit: Stamina (factor group 21009, skill group 21010) 3, Dirt pink 1; saddle 3 is a title, left out;
// 165 and 168 single-race G1s)
const TREE = JSON.stringify({
  version: 1,
  type: 'lineage-planner',
  payload: [
    {
      position: 'p1',
      characterId: 101901,
      sparks: [
        { factorId: 201590, level: 2 },
        { factorId: 210090, level: 3 },
        { factorId: 120, level: 1 },
      ],
      manualWinSaddleIds: [3, 165, 168],
    },
    { position: 'p1-1', characterId: 102101, sparks: [{ factorId: 201650, level: 2 }], manualWinSaddleIds: [168] },
    { position: 'p1-2', characterId: 102901, sparks: [{ factorId: 200490, level: 1 }], manualWinSaddleIds: [] },
  ],
});

test('an uma.moe tree adds her with her parents after your veterans, marked yours, and the note says so', () => {
  const { frame, saved } = fresh();
  expect(importText(TREE)).toBe(true);
  frame();
  const all = saved()[ERA].vets;
  expect(all).toHaveLength(4);
  const her = all.at(-1);
  expect(her.src).toBe('tree');
  expect(her.chara).toBe(1019);
  expect(her.borrowed).toBe(false);
  expect(her.sparks[20159]).toBe(2);
  expect(her.sparks[21010]).toBe(3);
  expect(her.pinks[12]).toBe(1);
  expect(her.wins).toHaveLength(2);
  expect(her.parents.map((p: LineageNode) => p.chara)).toEqual([1021, 1029]);
  expect(her.parents[0].sparks[20165]).toBe(2);
  expect(her.parents[0].wins).toHaveLength(1);
  expect(vetsNote.peek()).toMatch(/uma\.moe tree/);
});

test('a paste that is not an export says so and changes nothing', () => {
  fresh();
  const before = vs.vets.peek();
  expect(importText('{"x": 1}')).toBe(false);
  expect(vetsNote.peek()).toMatch(/No veterans found/);
  expect(vs.vets.peek()).toBe(before);
});

test("Export leaves uma.moe's listings out, their trainers with them", () => {
  fresh();
  vs.setListings(RECORDS.slice(0, 2).map((r, i) => parseListing(listing(r, i + 1)).veteran));
  expect(ids()).toEqual(['a', 'b', 'c', 'u900001', 'u900002']);
  const out = JSON.parse(exportText());
  expect(out.veterans.map((v: Veteran) => v.id)).toEqual(['a', 'b', 'c']);
  expect(JSON.stringify(out.veterans)).not.toMatch(/Trainer [12]/);
});

test('removing a fixed parent hands her place back, and she stays removed in storage', () => {
  const { frame, saved } = fresh();
  vs.removeVet('a');
  frame();
  expect(ids()).toEqual(['b', 'c']);
  expect(vs.places.peek().parents).toEqual([null, null]);
  expect(saved()[ERA].vets.map((v: Veteran) => v.id)).toEqual(['b', 'c']);
  expect(saved()[ERA].plan.parents).toEqual([null, null]);
});

test('a parent place naming a veteran not here (an uma.moe one never saved) goes back to the planner on load', () => {
  fresh({ vets: [vet('a'), vet('b')], plan: { trainee: null, parents: ['u999999', null] } });
  vs.dropMissing();
  expect(vs.places.peek().parents).toEqual([null, null]);
});

test('a career taken from the table goes back to the planner on a change of setup, saved at once', () => {
  const { saved } = fresh();
  vs.usePlaces(1001, ['a', 'b']);
  vs.handBackPlaces();
  expect(saved()[ERA].plan.parents).toEqual([null, null]);
  expect(saved()[ERA].plan.trainee).toBeNull();
});

test('an edit followed at once by the page going: the save still due goes out then', () => {
  const { saved } = fresh();
  vs.removeVet('c');
  expect(saved()[ERA].vets.map((v: Veteran) => v.id)).toContain('c'); // (due after the next frame)
  vs.flushPersist();
  expect(saved()[ERA].vets.map((v: Veteran) => v.id)).not.toContain('c');
});

test("another tab's write to the other era survives this tab's saves", () => {
  const { storage, frame, saved } = fresh();
  const stored = saved();
  stored[OTHER] = { vets: [vet('z'), vet('y')] };
  storage.setItem(KEY, JSON.stringify(stored));
  vs.editVet('b', () => ({ sparks: { 20159: 3 } }));
  frame();
  expect(saved()[OTHER].vets.map((v: Veteran) => v.id)).toEqual(['z', 'y']);
});

test('a tab on the other era saving its settings leaves the era chosen since', () => {
  const storage = memory();
  storage.setItem('llp:settings', JSON.stringify({ v: 2, era: 'gm', common: {}, global: {}, gm: {} }));
  writeSaved(
    () => storage,
    () => ({ common: { rank: 'ss_and_up' }, global: { roster: {} } }),
  );
  const saved = loadSaved(() => storage);
  expect(saved.era).toBe('gm');
  expect(saved.common?.rank).toBe('ss_and_up');
});

test('Add to veterans saves her as entered here, the places back to the planner; a second generation keeps her as a grandparent, over a reload', () => {
  const { storage, frame, saved } = fresh();
  const her = { ...node(1004, { 20159: 3 }), borrowed: false, parents: [node(1002), node(1003)], winAt: {} };
  vs.setDraft(her);
  expect(draftJoins()).toBe(1004);
  frame();
  const first = saved()[ERA];
  const added = first.vets.at(-1);
  expect(first.vets).toHaveLength(4);
  expect(added.src).toBe('entered');
  expect(added.sparks[20159]).toBe(3);
  expect(added.parents.map((p: LineageNode) => p.chara)).toEqual([1002, 1003]);
  expect(first.plan).toMatchObject({ trainee: null, parents: [null, null] });
  // a reload keeps her as she was
  vs.initVets({ storage: () => storage, hidden: () => false, afterFrame: (f: () => void) => f() });
  expect(JSON.stringify(vs.vets.peek().at(-1))).toBe(JSON.stringify(added));
  // the next generation, her a parent of the next veteran
  const { id: _id, src: _src, borrowed: _b, parents: _p, ...asParent } = added;
  vs.setDraft({ ...node(1005), borrowed: false, parents: [asParent, node(1001)], winAt: {} });
  draftJoins();
  const next = saved()[ERA].vets.at(-1);
  expect(saved()[ERA].vets).toHaveLength(5);
  expect(
    next.parents.some(
      (p: LineageNode) => p.chara === 1004 && JSON.stringify(p.sparks) === JSON.stringify(added.sparks),
    ),
  ).toBe(true);
});
