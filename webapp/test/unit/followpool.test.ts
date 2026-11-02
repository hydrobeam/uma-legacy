// The career page's search follows the veterans and places (career/current.ts `followPool`, which the entry calls with
// its run machine and comparison): an edit searches again, a change of who is in the pool drops the answer on show
// and clears the comparison, and while a pool veteran's G1 wins window or a skills prompt is open nothing searches
// until it closes (review-10 1, 2). npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import { loadData } from '../../src/model/data.ts';
import * as vs from '../../src/veterans/store.ts';
import { askClosed, askSkills, openWins, winsClosed } from '../../src/veterans/pickers.ts';
import { followPool } from '../../src/career/current.ts';
import { answerPool, showAnswer } from '../../src/career/state.ts';
import { memory } from './fakes.ts';
import { lineageNode, veteran } from './reports.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

await loadData(null);

// (each with the one pink a complete veteran has: the job takes her)
const node = (chara: number) => lineageNode(chara, {}, { 11: 1 });
const vet = (id: string) => veteran(id, { ...node(1001), parents: [node(1002), node(1003)] });

test('an edit searches again; a new pool drops the answer; a held search waits for its window to close', () => {
  vs.initVets({ storage: () => memory(), hidden: () => true, afterFrame: (f) => f() });
  vs.addVets([vet('a')]);
  const seen = { schedules: 0, clears: 0 };
  const stop = followPool({ schedule: () => seen.schedules++ }, { clear: () => seen.clears++ });
  // an edit of a veteran: a search, the answer kept on show (dimmed) until the new one
  showAnswer({ status: { kind: 'done', text: 'found' }, last: null, planState: { state: 'none', r: null } }, null, [
    'a',
  ]);
  vs.editVet('a', () => ({ wins: [1] }));
  assert.equal(seen.schedules, 1);
  assert.equal(seen.clears, 0);
  assert.deepEqual(answerPool.peek(), ['a'], 'the answer stays');
  // a veteran added: the pool's places change, so the answer and the comparison go, and it searches
  vs.addVets([vet('b')]);
  assert.equal(seen.schedules, 2);
  assert.equal(seen.clears, 1);
  assert.deepEqual(answerPool.peek(), [], 'the answer goes');
  // a pool veteran's G1 wins window open: her edits and the pool's wait; its close searches once
  openWins(vs.vetRef('a'), 'A');
  vs.addVets([vet('c')]);
  assert.equal(seen.schedules, 2, 'held while the window is open');
  winsClosed();
  assert.equal(seen.schedules, 3);
  assert.equal(seen.clears, 2, 'the pool changed under the window');
  // a skills prompt holds it too
  askSkills('A', false, () => undefined);
  vs.addVets([vet('d')]);
  assert.equal(seen.schedules, 3, 'held while the prompt is open');
  askClosed();
  assert.equal(seen.schedules, 4);
  stop();
});
