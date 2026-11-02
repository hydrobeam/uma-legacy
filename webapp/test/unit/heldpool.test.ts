// @vitest-environment happy-dom
// A career names its parents by their place in the pool its job was built from (legacy indices). While a pool
// veteran's G1 wins window or a skills prompt holds the search, the pool can change under the answer on show (an uma.moe
// listing lands, say), so the page reads a parent by the id her place held when the job was built (career/state.ts
// `answerPool`), never by her place in the pool now: Try a run's draft (`draftOf`) and Use this (`takeCareer`) get the
// right veterans, or, where one has left the pool, none. The targets' count chart sizes itself from the report it draws,
// not from the settings, which can have changed under it (an answer stays on show, dimmed, until the next is ready).
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import '@preact/signals';
import { h, render } from 'preact';
import * as vs from '../../src/veterans/store.ts';
import { initStore, setTargets } from '../../src/settings.ts';
import { dropAnswer, showAnswer } from '../../src/career/state.ts';
import { takeCareer } from '../../src/career/options.ts';
import { draftOf } from '../../src/career/analysis/view.tsx';
import { legaciesOf } from '../../src/career/analysis/Details.tsx';
import { legacyNumber, legacyVet } from '../../src/career/current.ts';
import { Targets } from '../../src/career/view/Sections.tsx';
import { loadData } from '../../src/data.ts';
import { memory } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

document.documentElement.className = 'game'; // (the planner pages' class)
await loadData({ blues: {}, pinks: {}, uniques: {}, races: [], mobs: [], sourced: [] });
initStore(() => memory());

const node = (chara) => ({ chara, sparks: {}, pinks: {}, blues: {}, greens: {}, wins: [] });
const vet = (id, chara) => ({ id, src: 'hand', borrowed: false, ...node(chara), parents: [node(1), node(2)] });
// grades G at every aptitude
const allG = { distance: [1, 1, 1, 1], surface: [1, 1], style: [1, 1, 1, 1] };
// a career on the job's veterans #3 and #4 (legacy indices 2 and 3): what the report and the plan rows carry
const career = {
  trainee: { id: 1005, name: 'Uma E', outfits: [] },
  parents: [2, 3],
  slots: [],
  targets: [],
  p_all: 0.5,
  p_minus1: 0.5,
  expected: 0,
  bound: 0,
  deck: { cards: [], card_ids: [], odds: [] },
  pinks: [],
  aptitudes: { base: allG, start: { kind: 'known', start: allG, fire: [] }, pink_gen: [] },
  schedule: null,
  outcome_sets: null,
  g1_wins: null,
};
// the pool a, b, c, d; the answer on show was searched on it; then b leaves while the answer is held
function heldAnswer() {
  vs.initVets({ storage: () => memory(), hidden: () => true, afterFrame: (f) => f() });
  vs.addVets([vet('a', 1001), vet('b', 1002), vet('c', 1003), vet('d', 1004)]);
  const answer = { status: { kind: 'done', text: '' }, last: null, planState: { state: 'none' } };
  dropAnswer();
  showAnswer(answer, null, ['a', 'b', 'c', 'd']);
  vs.removeVet('b');
}

test("Try a run's draft takes the career's parents by the ids the job was built on, after the pool changed", () => {
  heldAnswer();
  const draft = draftOf(career);
  assert.deepEqual(
    draft?.parents.map((p) => p.chara),
    [1003, 1004],
    "#3 and #4 of the job's pool: c and d, wherever they are now",
  );
  vs.removeVet('c');
  assert.equal(draftOf(career), null, 'a parent left the pool: no draft');
});

test("Use this places the career's parents by the ids the job was built on, after the pool changed", () => {
  heldAnswer();
  takeCareer(career, { trainee: null, parents: [null, null] });
  assert.deepEqual(vs.places.peek().parents, ['c', 'd']);
  vs.clearPlaces();
  vs.removeVet('d');
  takeCareer(career, { trainee: null, parents: [null, null] });
  assert.deepEqual(vs.places.peek().parents, [null, null], 'a parent left the pool: nothing is placed');
});

test("a career's parents are numbered by their place in the pool now, as the pool list numbers them (review-nuia F7)", () => {
  heldAnswer();
  assert.deepEqual(
    legaciesOf(career, legacyVet, legacyNumber, () => undefined).map((x) => x.label),
    ['#2', '#3'],
    "legacy indices 2 and 3 (c and d): #2 and #3 of the pool now that b has left, not the job's #3 and #4",
  );
  assert.equal(legacyNumber(1), '#2', "b has left the pool: the job's number, the only one she has");
});

test("the targets' count chart is the report's: a target removed since leaves it whole", async () => {
  setTargets([{ group: 20049 }, { group: 20166 }]);
  const target = (group, p) => ({ group, p, buy: {} });
  const r = {
    ...career,
    targets: [target(20049, 0.5), target(20166, 0.5), target(20159, 0.5)],
    outcome_sets: [0.125, 0.125, 0.125, 0.125, 0.125, 0.125, 0.125, 0.125],
    expected: 1.5,
  };
  const root = document.body.appendChild(document.createElement('div'));
  render(h(Targets, { r }), root);
  await new Promise((resolve) => setTimeout(resolve, 0));
  // each column: its chance, then its count
  const columns = [...root.querySelectorAll('[data-counts] [data-count]')];
  assert.deepEqual(
    columns.map((s) => s.textContent),
    ['13%3/3', '38%2/3', '38%1/3', '13%0/3'],
  );
});
