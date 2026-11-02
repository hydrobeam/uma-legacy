// @vitest-environment happy-dom
// The career page's views on happy-dom's document, compiled as the page's build compiles them: a view that
// throws while drawing (review-8c 3: the render guard, ui/RenderGuard.tsx under career/view/Guard.tsx); the veterans
// dialog's lists and pane (review-10 2-4).
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import '@preact/signals';
import { h, options, render } from 'preact';
import * as vs from '../../src/veterans/store.ts';
import { NodeDialog } from '../../src/career/view/NodeDialog.tsx';
import { closeNode, nodeOpen, openNode, pickerClosed, pickerFor } from '../../src/career/view/dialogs.ts';
import { on } from '../../src/store.ts';
import { sayRenderErrors } from '../../src/ui/RenderGuard.tsx';
import { answer, loadFails } from '../../src/career/state.ts';
import { PoolBody, PoolHead } from '../../src/career/view/Pool.tsx';
import { choose, openPool, poolClosed } from '../../src/career/view/poolState.ts';
import { OddsDialog, openOdds } from '../../src/ui/OddsDialog.tsx';
import { ScheduleDialog, openSched } from '../../src/ui/Schedule.tsx';
import { loadData } from '../../src/data.ts';
import { memory } from './fakes.ts';
import { within } from '@testing-library/dom';
import { must } from '../../src/must.ts';
import type { Veteran } from '../../src/veterans/store.ts';
import { lineageNode } from './reports.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

document.documentElement.className = 'game'; // (the planner pages' class)
await loadData({ blues: {}, pinks: {}, uniques: {}, races: [], factors: {}, saddles: {} });

const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};
const node = (chara: number) => lineageNode(chara);

test('a node popup that throws while drawing: the dialog stays, the status line says so, a new subject draws (review-8c 3)', async () => {
  const frames: (() => void)[] = [];
  vs.initVets({ storage: () => memory(), hidden: () => false, afterFrame: (f) => frames.push(f) });
  // a malformed record (a parent's G1 wins not a list, as a hand-edited file could make it) and a good one
  // (as such a file's text reads: JSON.parse hands it over unchecked, as it would hand the page a damaged save)
  const bad: Veteran = JSON.parse(
    JSON.stringify({
      id: 'bad',
      src: 'hand',
      borrowed: false,
      ...node(1001),
      parents: [node(1002), { ...node(1004), wins: null }],
    }),
  );
  vs.addVets([bad, { id: 'good', src: 'hand', borrowed: false, ...node(1003), parents: [node(1002), node(1004)] }]);
  // the page's own status write (main.tsx), which must not draw the view again (a few at most here: a loop would not end)
  const said: string[] = [],
    reported: unknown[] = [];
  sayRenderErrors((text) => {
    said.push(text);
    if (said.length < 4) loadFails(text);
  });
  globalThis.reportError = (e: unknown) => {
    reported.push(e);
  };
  const root = document.body.appendChild(document.createElement('div'));
  render(h(NodeDialog, null), root);
  openNode({ kind: 'v', id: 'bad' });
  await settle();
  const dlg = root.querySelector('dialog');
  assert.ok(dlg !== null, 'the popup is still there');
  assert.equal(dlg.open, true, 'and open');
  assert.equal(
    [...root.querySelectorAll('button')].filter((b) => b.textContent === 'Close').length,
    1,
    'with its Close button',
  );
  assert.equal(reported.length, 1, 'the throw is reported');
  assert.match(said.join(' | '), /^The node popup could not be drawn: /, 'the status line says what');
  assert.equal(said.length, 1, 'once: the status write draws nothing again');
  assert.match(answer.peek().status.text, /^The node popup could not be drawn: /, 'on the status line');
  // another subject draws (and closing still works)
  openNode({ kind: 'v', id: 'good' });
  await settle();
  assert.match(root.querySelector('#node-body')?.textContent ?? '', /Uma C/, 'the next veteran draws');
  closeNode();
  await settle();
  assert.equal(dlg.open, false, 'Close closes it');
  for (const f of frames.splice(0)) f();
});

test('the node popup closes before the parent picker opens (two writes), and Remove is one (review-8c 9)', async () => {
  const frames: (() => void)[] = [];
  vs.initVets({ storage: () => memory(), hidden: () => false, afterFrame: (f) => frames.push(f) });
  vs.addVets(
    ['a', 'b'].map((id, i) => ({
      id,
      src: 'hand',
      borrowed: false,
      ...node(1001 + i),
      parents: [node(1003), node(1004)],
    })),
  );
  const root = document.body.appendChild(document.createElement('div'));
  render(h(NodeDialog, null), root);
  const button = (text: string) =>
    must(
      [...root.querySelectorAll('button')].find((b) => b.textContent === text),
      `a ${text} button`,
    );
  // Choose parent: the popup closes, then the picker opens (a batch would run the picker's effects first: the picker
  // would open over the popup and give the focus back to the popup's button, which is gone by then)
  openNode({ kind: 'p', k: 0 });
  await settle();
  let runs: unknown[][] = [];
  const stopPicker = on([nodeOpen, pickerFor], (spec, k) => runs.push([spec, k]));
  button('Choose parent').click();
  stopPicker();
  assert.deepEqual(
    runs,
    [
      [null, null],
      [null, 0],
    ],
    'closed, then the picker open',
  );
  pickerClosed();
  // Remove: the popup closes and she goes, as one change
  openNode({ kind: 'v', id: 'a' });
  await settle();
  runs = [];
  const stopRemove = on([nodeOpen, vs.vets], (spec, list) => runs.push([spec, list.map((v) => v.id)]));
  button('Remove').click();
  stopRemove();
  assert.deepEqual(runs, [[null, ['b']]], 'closed and removed, in one run');
  await settle();
  for (const f of frames.splice(0)) f();
});

test('the veterans dialog: a selection draws two cards, one list is #vets, each tab names its own list, closed no pane', async () => {
  const frames: (() => void)[] = [];
  vs.initVets({ storage: () => memory(), hidden: () => false, afterFrame: (f) => frames.push(f) });
  vs.addVets(
    ['a', 'b', 'c', 'd', 'e'].map((id, i) => ({
      id,
      src: 'hand',
      borrowed: i === 4,
      ...node(1001 + (i % 3)),
      parents: [node(1002), node(1003)],
    })),
  );
  const tabs = document.body.appendChild(document.createElement('div')),
    body = document.body.appendChild(document.createElement('div'));
  render(h(PoolHead, null), tabs);
  render(h(PoolBody, null), body);
  openPool();
  await settle();
  // the cards drawn on a selection (each card reads whether she is the one chosen through a computed of her own)
  let cards = 0;
  const diffed = options.diffed;
  options.diffed = (vnode) => {
    if (typeof vnode.type === 'function' && vnode.type.name === 'PoolCard') cards++;
    diffed?.(vnode);
  };
  choose('c');
  await settle();
  if (diffed) options.diffed = diffed;
  else delete options.diffed;
  assert.equal(cards, 2, 'the one chosen before and the one chosen now');
  const withId = (id: string) => body.querySelectorAll(`#${id}`);
  assert.equal(withId('vets').length, 1, 'one list is #vets');
  assert.equal(withId('vets-bor').length, 1, "the Guests list, hidden, by its tab's name");
  assert.deepEqual(
    [...tabs.querySelectorAll('[role="tab"]')].map((b) => [b.id, b.getAttribute('aria-controls')]),
    [
      ['v-tab-own', 'vets'],
      ['v-tab-bor', 'vets-bor'],
    ],
    'each tab names its own list',
  );
  const pane = () => must(withId('v-pane')[0], 'the pane');
  assert.ok(pane().childNodes.length > 0, 'open: the pane draws the one chosen');
  poolClosed();
  await settle();
  assert.equal(pane().childNodes.length, 0, 'closed: the pane draws nothing');
  openPool();
  await settle();
  assert.ok(pane().childNodes.length > 0, 'open again: drawn');
  for (const f of frames.splice(0)) f();
});

test('a popup whose content throws reports it once per opening: its close draws nothing again (review-11b 4)', async () => {
  const reported = [];
  globalThis.reportError = (e: unknown) => {
    reported.push(e);
  };
  const root = document.body.appendChild(document.createElement('div'));
  render(h(OddsDialog, null), root);
  // (a malformed spec, no chances: JSON.parse hands it over unchecked)
  openOdds(JSON.parse(JSON.stringify({ title: 'Odds', n: 2, ex: null, pAll: 0.1, expected: 1 })));
  await settle();
  assert.equal(reported.length, 1, 'reported on opening');
  const dialog = must(root.querySelector('dialog'), 'the dialog');
  dialog.close();
  await settle();
  assert.equal(reported.length, 1, 'not again on its close');
  openOdds({ title: 'Odds', n: 1, ex: [0.5, 0.5], pAll: 0.5, expected: 0.5 });
  await settle();
  assert.match(within(root).getByRole('table', { name: 'Odds' }).textContent, /1\/1/, 'the next odds draw');
});

test('the race schedule, after it failed to draw, draws on its next opening (review-11b 5)', async () => {
  const reported = [];
  globalThis.reportError = (e: unknown) => {
    reported.push(e);
  };
  sayRenderErrors(() => undefined);
  // as either page places it (the dialog's own guard draws it again for a new opening: nothing to wire)
  const root = document.body.appendChild(document.createElement('div'));
  render(h(ScheduleDialog, null), root);
  const member = { k: 0, id: 1001, name: 'Uma A', outfits: [] };
  // (malformed, no schedule: JSON.parse hands it over unchecked)
  openSched(JSON.parse(JSON.stringify({ members: [member], schedule: null })));
  await settle();
  const dialog = must(root.querySelector('dialog'), 'the dialog');
  assert.equal(reported.length, 1, 'the throw reported');
  const close = [...dialog.querySelectorAll('button')].find((b) => b.textContent === 'Close');
  assert.ok(close !== undefined, 'its heading and Close stay');
  close.click();
  await settle();
  assert.equal(dialog.open, false, 'closed');
  openSched({ members: [member], schedule: [] });
  await settle();
  assert.equal(dialog.open, true, 'opened again: open');
  assert.equal(within(dialog).getAllByRole('tab').length, 1, 'and drawn (her tab)');
  assert.equal(reported.length, 1, 'no new failure');
});
