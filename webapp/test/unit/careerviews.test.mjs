// The career page's views on a fake document (fakedom.mjs), bundled with esbuild as the page bundles them: a view that
// throws while drawing (review-8c 3: the render guard, ui/RenderGuard.tsx under career/view/Guard.tsx); the veterans
// dialog's lists and pane (review-10 2-4).
// node --test 'test/unit/*.test.*' (in webapp)
import './fakepage.mjs';
import './fakedom.mjs';
import assert from 'node:assert/strict';
import { test } from 'node:test';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import * as esbuild from 'esbuild';
import { memory } from './fakes.mjs';

const WEBAPP = fileURLToPath(new URL('../..', import.meta.url));
const built = await esbuild.build({
  stdin: {
    contents: `
      import '@preact/signals';
      export * as store from './src/veterans/store.ts';
      export { NodeDialog } from './src/career/view/NodeDialog.tsx';
      export { openNode, closeNode } from './src/career/view/dialogs.ts';
      export { sayRenderErrors } from './src/ui/RenderGuard.tsx';
      export { PoolBody, PoolTabs } from './src/career/view/Pool.tsx';
      export { openPool, poolClosed, choose } from './src/career/view/poolState.ts';
      export { h, render, options } from 'preact';
      export { loadData } from './src/data.js';`,
    resolveDir: WEBAPP,
    loader: 'js',
  },
  bundle: true,
  write: false,
  format: 'esm',
  jsx: 'automatic',
  jsxImportSource: 'preact',
  logLevel: 'silent',
});
// (imported from a file, so a failure's stack names its lines)
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'llp-careerviews-')),
  file = path.join(dir, 'careerviews.mjs');
fs.writeFileSync(file, built.outputFiles[0].text);
const page = await import(pathToFileURL(file).href);
fs.rmSync(dir, { recursive: true });
const { store: vs, NodeDialog, openNode, closeNode, sayRenderErrors, h, render, options, loadData } = page;
const { PoolBody, PoolTabs, openPool, poolClosed, choose } = page;
globalThis.__LLP_DATA['data-career'] = { blues: {}, pinks: {}, uniques: {}, races: [] };
await loadData();

const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};
const node = (chara, sparks = {}) => ({ chara, sparks, pinks: {}, blues: {}, greens: {}, wins: [] });

test('a node popup that throws while drawing: the dialog stays, the status line says so, a new subject draws (review-8c 3)', async () => {
  const frames = [];
  vs.initVets({ storage: () => memory(), hidden: () => false, afterFrame: (f) => frames.push(f) });
  // a malformed record (one parent missing, as a hand-edited file could make it) and a good one
  vs.addVets([
    { id: 'bad', src: 'hand', borrowed: false, ...node(1001), parents: [node(1002), null] },
    { id: 'good', src: 'hand', borrowed: false, ...node(1003), parents: [node(1002), node(1004)] },
  ]);
  const said = [],
    reported = [];
  sayRenderErrors((text) => said.push(text));
  globalThis.reportError = (e) => reported.push(e);
  const root = document.createElement('div');
  render(h(NodeDialog, null), root);
  openNode({ kind: 'v', id: 'bad' });
  await settle();
  const dlg = root.querySelector('dialog');
  assert.ok(dlg !== null, 'the popup is still there');
  assert.equal(dlg.open, true, 'and open');
  assert.equal(root.querySelectorAll('button.gmclose').length, 1, 'with its Close button');
  assert.equal(reported.length, 1, 'the throw is reported');
  assert.match(said.join(' | '), /^The node popup could not be drawn: /, 'the status line says what');
  // another subject draws (and closing still works)
  openNode({ kind: 'v', id: 'good' });
  await settle();
  assert.match(root.querySelector('div.gmdbody').textContent, /Uma C/, 'the next veteran draws');
  closeNode();
  await settle();
  assert.equal(dlg.open, false, 'Close closes it');
  for (const f of frames.splice(0)) f();
});

// every element under `root`, depth first
const elements = (root) => root.childNodes.filter((c) => c.nodeType === 1).flatMap((c) => [c, ...elements(c)]);
test('the veterans dialog: a selection draws two cards, one list is #vets, each tab names its own list, closed no pane', async () => {
  const frames = [];
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
  const tabs = document.createElement('div'),
    body = document.createElement('div');
  render(h(PoolTabs, null), tabs);
  render(h(PoolBody, null), body);
  openPool();
  await settle();
  // the cards drawn on a selection (each card reads whether she is the one chosen through a computed of her own)
  let cards = 0;
  const diffed = options.diffed;
  options.diffed = (vnode) => {
    if (vnode.type?.name === 'PoolCard') cards++;
    diffed?.(vnode);
  };
  choose('c');
  await settle();
  options.diffed = diffed;
  assert.equal(cards, 2, 'the one chosen before and the one chosen now');
  const withId = (id) => elements(body).filter((e) => e.attributes.id === id);
  assert.equal(withId('vets').length, 1, 'one list is #vets');
  assert.equal(withId('vets-bor').length, 1, "the Guests list, hidden, by its tab's name");
  assert.deepEqual(
    elements(tabs).map((b) => [b.attributes.id, b.attributes['aria-controls']]),
    [
      ['v-tab-own', 'vets'],
      ['v-tab-bor', 'vets-bor'],
    ],
    'each tab names its own list',
  );
  const pane = () => withId('v-pane')[0];
  assert.ok(pane().childNodes.length > 0, 'open: the pane draws the one chosen');
  poolClosed();
  await settle();
  assert.equal(pane().childNodes.length, 0, 'closed: the pane draws nothing');
  openPool();
  await settle();
  assert.ok(pane().childNodes.length > 0, 'open again: drawn');
  for (const f of frames.splice(0)) f();
});
