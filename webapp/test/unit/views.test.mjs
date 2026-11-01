// The veterans' views and editors on a fake document (fakedom.mjs), bundled with esbuild as the page bundles them: the
// spark editor (ui/Sparks under ui/InspView) shows each edit of the store's draft, a new record each (review-8c 1); change uma commits the uma at the pick, saved and
// seen by the page's effects once the skills prompt is gone (the search held while it is open, review-10 1), and the
// prompt dismissed keeps her sparks (review-8c 7).
// node --test 'test/unit/*.test.*' (in webapp)
import './fakepage.mjs';
import { dispatchToDocument } from './fakedom.mjs';
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
      export { openUmaPicker, umaPickClosed } from './src/veterans/pickers.ts';
      export { changeUma, searchHeld, openWins, winsClosed, askSkills, askClosed, openAdd, addOpen } from './src/veterans/pickers.ts';
      export { VetEditors, AddList } from './src/veterans/Editors.tsx';
      export { WinsEditor } from './src/ui/Agenda.tsx';
      export { openColour } from './src/veterans/sparks.ts';
      export { InspView } from './src/ui/InspView.tsx';
      export { Details, legaciesOf } from './src/career/analysis/Details.tsx';
      export { h, render } from 'preact';
      export { loadData } from './src/data.ts';`,
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
const { store: vs, on, changeUma, searchHeld, VetEditors, InspView, Details, legaciesOf, h, render, loadData } = page;
const { openWins, winsClosed, askSkills, askClosed, openAdd, addOpen, AddList, WinsEditor, openColour } = page;
const { openUmaPicker, umaPickClosed } = page;
// the career tables the spark bars read (blue, pink and unique names; no races)
globalThis.__LLP_DATA['data-career'] = {
  blues: {},
  pinks: { 11: 'Turf', 12: 'Dirt' },
  uniques: { 30001: 'Her Own' },
  races: ['Arima Kinen'],
  race_turns: [[47, 71]],
};
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
    return d && h(InspView, { nd: d, parents: d.parents, k: 'sim', at: vs.draftRef(), targets: [T] });
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

test('her unique spark added back, or a first pink set: the bar is the same button, so the focus stays on it', async () => {
  vs.setDraft({ ...node(1001), outfit: 30001, borrowed: false, parents: [] });
  const Card = () => {
    const d = vs.draft.value;
    return d && h(InspView, { nd: d, parents: d.parents, k: 'sim', at: vs.draftRef(), targets: [T] });
  };
  const root = document.createElement('div');
  render(h(Card, null), root);
  await settle();
  const bar = (k) => root.querySelectorAll('button').find((b) => b.attributes['data-k'] === k);
  const attached = (n) => {
    while (n && n !== root) n = n.parentNode;
    return n === root;
  };
  const none = bar('sim:green');
  assert.match(none.textContent, /\+ Unique spark/);
  none.focus();
  none.click(); // her unique at 2 stars
  await settle();
  assert.match(bar('sim:green').attributes['aria-label'], /\(unique\), 2 stars/);
  // (identity as a boolean: an assertion message inspecting two Preact-owned nodes walks their whole graphs)
  assert.ok(bar('sim:green') === none, "the unique's button is the one that had the focus");
  assert.ok(attached(document.activeElement), 'the focus is still in the card');
  // a first pink (the colour picker's pick lands; its close gives the focus back to its opener, which must be there)
  const pink = bar('sim:pink');
  vs.draftRef().edit(() => ({ pinks: { 11: 1 } }));
  await settle();
  assert.ok(bar('sim:pink') === pink, "the pink's opener is the same button");
});

// an element of the drawn editors by its id
const byId = (root, id) => {
  for (const c of root.childNodes) {
    const found = c.nodeType === 1 && (c.attributes.id === id ? c : byId(c, id));
    if (found) return found;
  }
  return undefined;
};
// (an uma's button: her portrait, then her name)
const button = (root, text) => root.querySelectorAll('button').find((b) => b.textContent.endsWith(text));
test('change uma: the uma is hers at the pick (saved, the page told once the prompt is gone); the prompt dismissed keeps her sparks', async () => {
  const storage = memory(),
    frames = [];
  vs.initVets({ storage: () => storage, hidden: () => false, afterFrame: (f) => frames.push(f) });
  vs.addVets([
    { id: 'a', src: 'hand', borrowed: false, ...node(1001, { [T]: 2 }), outfit: 100101, parents: [node(1), node(2)] },
  ]);
  for (const f of frames.splice(0)) f();
  // the page's veterans effect, with its hold (career/main.tsx)
  let woke = 0;
  const stop = on([vs.vets, searchHeld], (_list, held) => {
    if (!held) woke++;
  });
  const editors = document.createElement('div');
  render(h(VetEditors, null), editors);
  changeUma(vs.vetRef('a'), 'Veteran #1');
  await settle();
  assert.equal(byId(editors, 'pick-dlg').open, true, 'the uma picker is open');
  button(byId(editors, 'pick-list'), 'Uma B').click();
  await settle();
  const ask = byId(editors, 'ask-dlg');
  assert.equal(ask.open, true, 'the skills prompt asks');
  ask.close(); // Escape: no choice
  for (const f of frames.splice(0)) f();
  const v = vs.vetById('a');
  assert.deepEqual([v.chara, 'outfit' in v, v.sparks], [1002, false, { [T]: 2 }], 'her uma changed, her sparks kept');
  assert.equal(JSON.parse(storage.getItem('llp:career')).global.vets[0].chara, 1002, 'saved'); // (node's era: global)
  assert.equal(woke, 1, "the page's effect saw it (its search and redraw follow)");
  // a choice on the prompt sets her sparks as a second edit, which the page sees with the uma, once
  changeUma(vs.vetRef('a'), 'Veteran #1');
  await settle();
  button(byId(editors, 'pick-list'), 'Uma C').click();
  await settle();
  button(byId(editors, 'ask-btns'), 'No skills').click();
  assert.deepEqual([vs.vetById('a').chara, vs.vetById('a').sparks, woke], [1003, {}, 2]);
  stop();
});

test("Try a run's legacies: a place no veteran holds any more (the list changed under the answer) is left out (review-9 F6)", async () => {
  const career = { parents: [0, 4], trainee: { id: 1001, outfits: [] }, targets: [], deck: null };
  const list = [{ ...node(1002), id: 'a', borrowed: false, parents: [node(1), node(2)] }];
  const legacies = legaciesOf(
    career,
    (i) => list[i],
    () => undefined,
  );
  assert.deepEqual(
    legacies.map((x) => x.label),
    ['#1'],
    'the veteran at #5 is gone: her box goes',
  );
  vs.setDraft({ ...node(1001), borrowed: false, parents: [node(1002), node(1003)] });
  const spec = { d: vs.draft.peek(), r: career, races: [], raceName: String, targets: [T], legacies };
  const root = document.createElement('div');
  render(h(Details, { spec, actions: null, winsButton: null }), root);
  await settle();
  assert.equal(root.querySelectorAll('div.udleg').length, 1, 'her card draws, with the one legacy');
});

test("the search is held while a pool veteran's G1 wins window or a skills prompt is open (not the draft's window)", () => {
  openWins(vs.vetRef('a'), 'Veteran #1');
  assert.equal(searchHeld.peek(), true, "a pool veteran's wins window holds it");
  winsClosed();
  assert.equal(searchHeld.peek(), false, 'closed: released');
  let back = 0;
  openWins(vs.draftRef(), 'her', { onClose: () => back++ });
  assert.equal(searchHeld.peek(), false, "the draft's window (its own close) holds nothing");
  winsClosed();
  assert.equal(back, 1, 'its close called once');
  askSkills('Uma B', false, () => undefined);
  assert.equal(searchHeld.peek(), true, 'a skills prompt holds it');
  askClosed();
  assert.equal(searchHeld.peek(), false, 'dismissed: released');
});

test('a second "+" pressed while a "+" list is open moves the list there (review-11b 1)', async () => {
  const host = document.createElement('div'),
    first = document.createElement('button'),
    second = document.createElement('button');
  render(h(AddList, { host }), host);
  openAdd(first, vs.vetRef('a'));
  await settle(); // drawn, its listeners on
  // the second "+": its own handler, then the click reaches the document
  openAdd(second, vs.vetRef('a'));
  dispatchToDocument('click', { target: second });
  await settle();
  assert.ok(addOpen.peek()?.btn === second, 'the list is on the second "+"');
  assert.equal(host.hidden, false, 'and shown');
  // a press elsewhere closes it
  dispatchToDocument('click', { target: document.createElement('div') });
  await settle();
  assert.ok(addOpen.peek() === null, 'closed by a press outside');
});

// review-11b 2: the editors' writes and releases that no check covered (sabotages M1, M3, M6)
test("Escape on a pool veteran's G1 wins window releases the search hold (M1)", async () => {
  const editors = document.createElement('div');
  render(h(VetEditors, null), editors);
  openWins(vs.vetRef('a'), 'Veteran #1');
  await settle();
  const wins = byId(editors, 'wins-dlg');
  assert.equal(wins.open, true, 'open');
  assert.equal(searchHeld.peek(), true, 'the search held');
  wins.close(); // Escape (the backdrop closes it the same way)
  await settle();
  assert.equal(searchHeld.peek(), false, 'released');
});
test('a win moved to another turn of its race ("moves here") is written once, at the new turn (M3)', async () => {
  const edits = [],
    root = document.createElement('div');
  render(
    h(WinsEditor, {
      races: [{ ix: 0, name: 'Arima Kinen', turns: [47, 71] }],
      wins: [0],
      winAt: { 0: 47 },
      onWins: (wins, winAt) => edits.push({ wins, winAt }),
    }),
    root,
  );
  await settle();
  root
    .querySelectorAll('button')
    .find((b) => b.attributes['data-k'] === 'add:71')
    .click();
  await settle();
  root.querySelectorAll('button.wpick')[0].click();
  assert.deepEqual(edits, [{ wins: [0], winAt: { 0: 71 } }]);
});
test('a colour bar picked keeps her spark of that colour at its stars (M6)', async () => {
  vs.editVet('a', () => ({ pinks: { 11: 3 } }));
  const editors = document.createElement('div');
  render(h(VetEditors, null), editors);
  openColour(vs.vetRef('a'), 'pink');
  await settle();
  button(byId(editors, 'col-dlg'), 'Dirt').click();
  assert.deepEqual(vs.vetById('a').pinks, { 12: 3 }, 'Dirt, at 3 stars as Turf was');
});

test('a throw while the wins window or the skills prompt is open: it goes, the hold is released, a search runs (review-11b 3)', async () => {
  const editors = document.createElement('div'),
    reported = [];
  globalThis.reportError = (e) => reported.push(e);
  render(h(VetEditors, null), editors);
  vs.editVet('a', () => ({ wins: [0], winAt: { 0: 47 } }));
  // the page's veterans effect, with its hold (career/main.tsx)
  let woke = 0;
  const stop = on([vs.vets, searchHeld], (_list, held) => {
    if (!held) woke++;
  });
  // her schedule says she runs it then, but gives no figures: her win's button throws inside the open window
  openWins(vs.vetRef('a'), 'Veteran #1', { info: () => ({ turn: 47, parent: false }) });
  await settle();
  assert.equal(reported.length, 1, 'the throw reported');
  assert.equal(searchHeld.peek(), false, 'the window gone, the hold released');
  assert.equal(woke, 1, 'the page searches');
  // the prompt throwing as it draws
  const unnamed = {
    toString: () => {
      throw new Error('a name that cannot be drawn');
    },
  };
  askSkills(unnamed, false, () => undefined);
  await settle();
  assert.equal(searchHeld.peek(), false, 'the prompt gone, the hold released');
  assert.equal(woke, 2, 'the page searches');
  // the other editors still draw
  openWins(vs.vetRef('a'), 'Veteran #1');
  await settle();
  assert.equal(byId(editors, 'wins-dlg')?.open, true, 'the wins window opens again');
  winsClosed();
  stop();
});

// review-11b 6: a closed editor draws nothing, and one whose node is gone closes, the wins window as the colour picker
test('the uma picker closed draws no list; a G1 wins window whose draft is replaced closes', async () => {
  const editors = document.createElement('div');
  render(h(VetEditors, null), editors);
  openUmaPicker('Which uma?', () => undefined);
  await settle();
  assert.ok(byId(editors, 'pick-list').querySelectorAll('button.upick').length > 0, 'open: every uma');
  umaPickClosed();
  await settle();
  assert.equal(byId(editors, 'pick-list')?.querySelectorAll('button.upick').length ?? 0, 0, 'closed: none');
  vs.setDraft({ ...node(1001), borrowed: false, parents: [node(1002), node(1003)] });
  let back = 0;
  openWins(vs.draftRef(), 'her', { onClose: () => back++ });
  await settle();
  assert.equal(byId(editors, 'wins-dlg').open, true, 'open on her draft');
  vs.setDraft({ ...node(1002), borrowed: false, parents: [node(1001), node(1003)] }); // (a roll: another draft)
  await settle();
  assert.equal(byId(editors, 'wins-dlg').open, false, 'her draft gone: the window closes, as the colour picker does');
  assert.equal(back, 1, 'its close called');
});
