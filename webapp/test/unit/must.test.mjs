// must.ts: `must` and `assert` throw naming their reason. `orNull`: an expected error gives null quietly; any other is
// reported through the page's error path (`reportError`, stood in for here) and gives null too, so the caller goes on.
// node --test 'test/unit/*.test.*'
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { assert as holds, must, orNull } from '../../src/must.ts';

class Expected extends Error {}
const reported = [];
globalThis.reportError = (error) => reported.push(error);

test('orNull: the value; null for an expected error, unreported; null for any other, reported', () => {
  reported.length = 0;
  assert.equal(
    orNull(() => 7, Expected),
    7,
  );
  assert.equal(
    orNull(() => {
      throw new Expected('no job');
    }, Expected),
    null,
  );
  assert.deepEqual(reported, []);
  const bug = new TypeError('vets[i] is undefined');
  assert.equal(
    orNull(() => {
      throw bug;
    }, Expected),
    null,
  );
  assert.deepEqual(reported, [bug]);
});

test('must: the value, or an error naming the reason; assert: nothing, or an error naming the reason', () => {
  assert.equal(must(0, 'zero is a value'), 0);
  assert.throws(() => must(undefined, 'the member has her deck'), /^Error: missing: the member has her deck$/);
  holds(true, 'never thrown');
  assert.throws(() => holds(false, 'one deck per member'), /^Error: failed: one deck per member$/);
});
