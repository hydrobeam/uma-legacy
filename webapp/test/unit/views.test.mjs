// The veterans' views and editors on a fake document (fakedom.mjs), bundled with esbuild as the page bundles them: the
// spark editor (ui/Sparks under ui/InspView) shows each edit of the store's draft, a new record each (review-8c 1); change uma commits the uma at the pick, saved and
// seen by the page's effects, and the skills prompt dismissed keeps her sparks (review-8c 7).
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
      export { on } from './src/store.ts';
      export { changeUma } from './src/veterans/pickers.js';
      export { InspView } from './src/ui/InspView.tsx';
      export { h, render } from 'preact';
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
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'llp-views-')),
  file = path.join(dir, 'views.mjs');
fs.writeFileSync(file, built.outputFiles[0].text);
const page = await import(pathToFileURL(file).href);
fs.rmSync(dir, { recursive: true });
const { store: vs, on, changeUma, InspView, h, render, loadData } = page;
// the career tables the spark bars read (blue, pink and unique names; no races)
globalThis.__LLP_DATA['data-career'] = { blues: {}, pinks: { 11: 'Turf' }, uniques: {}, races: [] };
await loadData();

const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};
const node = (chara, sparks = {}) => ({ chara, sparks, pinks: {}, blues: {}, greens: {}, wins: [] });
const T = 20159; // a target skill of the fake page's data
// her chip's label for target T in the drawn card
const chipLabel = (root) =>
  root.querySelectorAll('button').find((b) => b.attributes['data-k'] === `sim:s${T}`)?.attributes['aria-label'];

test("the spark editor shows each edit of the store's draft (a new record each)", async () => {
  // the store's draft: her card reads the signal and hands InspView her record
  vs.setDraft({ ...node(1001, { [T]: 1 }), borrowed: false, parents: [node(1002), node(1003)] });
  const Card = () => {
    const d = vs.draft.value;
    return d && h(InspView, { nd: d, parents: d.parents, k: 'sim', at: vs.draftRef, targets: [T] });
  };
  const root = document.createElement('div');
  render(h(Card, null), root);
  await settle();
  assert.match(chipLabel(root), /, 1 star\. Click for 2$/);
  root
    .querySelectorAll('button')
    .find((b) => b.attributes['data-k'] === `sim:s${T}`)
    .click();
  await settle();
  assert.match(chipLabel(root), /, 2 stars\. Click for 3$/, "the store's draft");
});

test('change uma: the uma is hers at the pick (saved, the page told); the prompt dismissed keeps her sparks', async () => {
  const storage = memory(),
    frames = [];
  vs.initVets({ storage: () => storage, hidden: () => false, afterFrame: (f) => frames.push(f) });
  vs.addVets([
    { id: 'a', src: 'hand', borrowed: false, ...node(1001, { [T]: 2 }), outfit: 100101, parents: [node(1), node(2)] },
  ]);
  for (const f of frames.splice(0)) f();
  let woke = 0;
  const stop = on([vs.vets], () => woke++);
  changeUma(vs.vetRef('a'), 'Veteran #1');
  const pick = document.getElementById('pick-list'),
    ask = document.getElementById('ask-dlg');
  assert.equal(document.getElementById('pick-dlg').open, true, 'the uma picker is open');
  pick
    .querySelectorAll('button.upick')
    .find((b) => b._nm === 'uma b')
    .click();
  assert.equal(ask.open, true, 'the skills prompt asks');
  ask.close(); // Escape: no choice
  for (const f of frames.splice(0)) f();
  const v = vs.vetById('a');
  assert.deepEqual([v.chara, 'outfit' in v, v.sparks], [1002, false, { [T]: 2 }], 'her uma changed, her sparks kept');
  assert.equal(JSON.parse(storage.getItem('llp:career')).global.vets[0].chara, 1002, 'saved'); // (node's era: global)
  assert.equal(woke, 1, "the page's effect saw it (its search and redraw follow)");
  // a choice on the prompt sets her sparks as a second edit
  changeUma(vs.vetRef('a'), 'Veteran #1');
  pick
    .querySelectorAll('button.upick')
    .find((b) => b._nm === 'uma c')
    .click();
  document
    .getElementById('ask-btns')
    .querySelectorAll('button')
    .find((b) => b.textContent === 'No skills')
    .click();
  assert.deepEqual([vs.vetById('a').chara, vs.vetById('a').sparks, woke], [1003, {}, 3]);
  stop();
});
