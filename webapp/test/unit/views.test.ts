// @vitest-environment happy-dom
// The veterans' views and editors on happy-dom's document, compiled as the page's build compiles them: the
// spark editor (ui/Sparks under ui/InspView) shows each edit of the store's draft, a new record each (review-8c 1); change uma commits the uma at the pick, saved and
// seen by the page's effects once the skills prompt is gone (the search held while it is open, review-10 1), and the
// prompt dismissed keeps her sparks (review-8c 7).
// npx vitest run (in webapp)
import assert from 'node:assert/strict';
import { test, vi } from 'vitest';
import '@preact/signals';
import { h, render } from 'preact';
import * as vs from '../../src/veterans/store.ts';
import { on } from '../../src/store.ts';
import {
  addOpen,
  askClosed,
  askSkills,
  changeUma,
  openAdd,
  openUmaPicker,
  openWins,
  searchHeld,
  umaPickClosed,
  winsClosed,
} from '../../src/veterans/pickers.ts';
import { AddList, VetEditors } from '../../src/veterans/Editors.tsx';
import { WinsEditor } from '../../src/ui/Agenda.tsx';
import { openColour } from '../../src/veterans/sparks.ts';
import { InspView } from '../../src/ui/InspView.tsx';
import { Details, legaciesOf } from '../../src/career/analysis/Details.tsx';
import { loadData } from '../../src/model/data.ts';
import { memory } from './fakes.ts';
import { within } from '@testing-library/dom';
import { SegmentedControl } from '../../src/ui/SegmentedControl.tsx';
import { must } from '../../src/must.ts';
import type { Sparks } from '../../src/veterans/store.ts';
import { dialogAt, find, query, settle } from './dom.ts';
import { careerReport, lineageNode, veteran } from './reports.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

document.documentElement.className = 'game'; // (the planner pages' class)
// the career tables the spark bars read (blue, pink and unique names; no races)
await loadData({
  blues: {},
  pinks: { 11: 'Turf', 12: 'Dirt' },
  uniques: { 30001: 'Her Own' },
  races: ['Arima Kinen'],
  race_turns: [[47, 71]],
  factors: {},
  saddles: {},
});

const node = (chara: number, sparks: Sparks = {}) => lineageNode(chara, sparks);
const T = 20159; // a target skill of the fake page's data
// her chip's label for target T in the drawn card
const chipLabel = (root: ParentNode) =>
  root.querySelector(`button[data-k="sim:s${T}"]`)?.getAttribute('aria-label') ?? '';

test("the spark editor shows each edit of the store's draft (a new record each)", async () => {
  // the store's draft: her card reads the signal and hands InspView her record
  vs.setDraft({ ...node(1001, { [T]: 1 }), borrowed: false, parents: [node(1002), node(1003)] });
  const Card = () => {
    const d = vs.draft.value;
    return d && h(InspView, { nd: d, parents: d.parents, k: 'sim', at: vs.draftRef(), targets: [T] });
  };
  const root = document.body.appendChild(document.createElement('div'));
  render(h(Card, null), root);
  await settle();
  assert.match(chipLabel(root), /, 1 star\. Click for 2$/);
  find(root, `button[data-k="sim:s${T}"]`).click();
  await settle();
  assert.match(chipLabel(root), /, 2 stars\. Click for 3$/, "the store's draft");
});

test('her unique spark added back, or a first pink set: the bar is the same button, so the focus stays on it', async () => {
  vs.setDraft({ ...node(1001), outfit: 30001, borrowed: false, parents: [] });
  const Card = () => {
    const d = vs.draft.value;
    return d && h(InspView, { nd: d, parents: d.parents, k: 'sim', at: vs.draftRef(), targets: [T] });
  };
  const root = document.body.appendChild(document.createElement('div'));
  render(h(Card, null), root);
  await settle();
  const bar = (k: string) => find(root, `button[data-k="${k}"]`);
  const none = bar('sim:green');
  assert.match(none.textContent, /\+ Unique spark/);
  none.focus();
  none.click(); // her unique at 2 stars
  await settle();
  assert.match(bar('sim:green').getAttribute('aria-label') ?? '', /\(unique\), 2 stars/);
  // (identity as a boolean: an assertion message inspecting two Preact-owned nodes walks their whole graphs)
  assert.ok(bar('sim:green') === none, "the unique's button is the one that had the focus");
  assert.ok(root.contains(document.activeElement), 'the focus is still in the card');
  // a first pink (the colour picker's pick lands; its close gives the focus back to its opener, which must be there)
  const pink = bar('sim:pink');
  vs.draftRef().edit(() => ({ pinks: { 11: 1 } }));
  await settle();
  assert.ok(bar('sim:pink') === pink, "the pink's opener is the same button");
});

// an element of the drawn editors by its id
const byId = (root: ParentNode, id: string) => query(root, `#${id}`);
const dialogById = (root: ParentNode, id: string) => dialogAt(root, `#${id}`);
// (an uma's button: her portrait, then her name)
const button = (root: ParentNode | null, text: string) =>
  must(
    [...must(root, 'the list').querySelectorAll('button')].find((b) => b.textContent?.endsWith(text)),
    `a button ${text}`,
  );
test('change uma: the uma is hers at the pick (saved, the page told once the prompt is gone); the prompt dismissed keeps her sparks', async () => {
  const storage = memory(),
    frames: (() => void)[] = [];
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
  const editors = document.body.appendChild(document.createElement('div'));
  render(h(VetEditors, null), editors);
  changeUma(vs.vetRef('a'), 'Veteran #1');
  await settle();
  assert.equal(dialogById(editors, 'pick-dlg').open, true, 'the uma picker is open');
  button(byId(editors, 'pick-list'), 'Uma B').click();
  await settle();
  const ask = dialogById(editors, 'ask-dlg');
  assert.equal(ask.open, true, 'the skills prompt asks');
  ask.close(); // Escape: no choice
  await settle(); // (a browser fires the dialog's close as a task of its own)
  for (const f of frames.splice(0)) f();
  const v = must(vs.vetById('a'), 'veteran a');
  assert.deepEqual([v.chara, 'outfit' in v, v.sparks], [1002, false, { [T]: 2 }], 'her uma changed, her sparks kept');
  assert.equal(JSON.parse(storage.getItem('llp:career') ?? 'null').global.vets[0].chara, 1002, 'saved'); // (node's era: global)
  assert.equal(woke, 1, "the page's effect saw it (its search and redraw follow)");
  // a choice on the prompt sets her sparks as a second edit, which the page sees with the uma, once
  changeUma(vs.vetRef('a'), 'Veteran #1');
  await settle();
  button(byId(editors, 'pick-list'), 'Uma C').click();
  await settle();
  button(byId(editors, 'ask-btns'), 'No skills').click();
  assert.deepEqual([vs.vetById('a')?.chara, vs.vetById('a')?.sparks, woke], [1003, {}, 2]);
  stop();
});

test("Try a run's legacies: a place no veteran holds any more (the list changed under the answer) is left out (review-9 F6)", async () => {
  const career = careerReport({ parents: [0, 4] });
  const list = [veteran('a', { ...node(1002), parents: [node(1), node(2)] })];
  const numberOf = (i: number) => `#${i + 1}`;
  const legacies = legaciesOf(
    career,
    (i) => list[i],
    numberOf,
    () => undefined,
  );
  assert.deepEqual(
    legacies.map((x) => x.label),
    ['#1'],
    'the veteran at #5 is gone: her box goes',
  );
  vs.setDraft({ ...node(1001), borrowed: false, parents: [node(1002), node(1003)] });
  const spec = {
    d: must(vs.draft.peek(), 'the draft'),
    r: career,
    races: [],
    raceName: String,
    targets: [T],
    legacies,
    numberOf,
  };
  const root = document.body.appendChild(document.createElement('div'));
  render(h(Details, { spec, actions: null, winsButton: null }), root);
  await settle();
  assert.equal(root.querySelectorAll('button[aria-label^="Legacy "]').length, 1, 'her card draws, with the one legacy');
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
  const host = document.body.appendChild(document.createElement('div'));
  // a "+" opens the list on its click (ui/Sparks.tsx)
  const plus = () => {
    const btn = document.body.appendChild(document.createElement('button'));
    btn.addEventListener('click', () => openAdd(btn, vs.vetRef('a')));
    return btn;
  };
  const first = plus(),
    second = plus();
  render(h(AddList, { host }), host);
  first.click();
  await settle(); // drawn, its listeners on
  // the second "+": its own handler, then the click reaches the document
  second.click();
  await settle();
  assert.ok(addOpen.peek()?.btn === second, 'the list is on the second "+"');
  assert.equal(host.hidden, false, 'and shown');
  // a press elsewhere closes it
  document.body.appendChild(document.createElement('div')).click();
  await settle();
  assert.ok(addOpen.peek() === null, 'closed by a press outside');
});

// review-11b 2: the editors' writes and releases that no check covered (sabotages M1, M3, M6)
test("Escape on a pool veteran's G1 wins window releases the search hold (M1)", async () => {
  const editors = document.body.appendChild(document.createElement('div'));
  render(h(VetEditors, null), editors);
  openWins(vs.vetRef('a'), 'Veteran #1');
  await settle();
  const wins = dialogById(editors, 'wins-dlg');
  assert.equal(wins.open, true, 'open');
  assert.equal(searchHeld.peek(), true, 'the search held');
  wins.close(); // Escape (the backdrop closes it the same way)
  await settle();
  assert.equal(searchHeld.peek(), false, 'released');
});
test('a win moved to another turn of its race ("moves here") is written once, at the new turn (M3)', async () => {
  const edits: unknown[] = [],
    root = document.body.appendChild(document.createElement('div'));
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
  find(root, 'button[data-k="add:71"]').click();
  await settle();
  // (the menu's race: its button, named by its title)
  const menu = within(root).getByRole('dialog', { name: /G1s held$/ });
  within(menu)
    .getByRole('button', { name: /^Arima Kinen/ })
    .click();
  assert.deepEqual(edits, [{ wins: [0], winAt: { 0: 71 } }]);
});
test('a colour bar picked keeps her spark of that colour at its stars (M6)', async () => {
  vs.editVet('a', () => ({ pinks: { 11: 3 } }));
  const editors = document.body.appendChild(document.createElement('div'));
  render(h(VetEditors, null), editors);
  openColour(vs.vetRef('a'), 'pink');
  await settle();
  button(byId(editors, 'col-dlg'), 'Dirt').click();
  assert.deepEqual(vs.vetById('a')?.pinks, { 12: 3 }, 'Dirt, at 3 stars as Turf was');
});

test('a throw while the wins window or the skills prompt is open: it goes, the hold is released, a search runs (review-11b 3)', async () => {
  const editors = document.body.appendChild(document.createElement('div')),
    reported: unknown[] = [];
  globalThis.reportError = (e: unknown) => {
    reported.push(e);
  };
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
  // (a name that is no string: Reflect.apply hands it over unchecked, as a damaged record's would reach the prompt)
  Reflect.apply(askSkills, undefined, [unnamed, false, () => undefined]);
  await settle();
  assert.equal(searchHeld.peek(), false, 'the prompt gone, the hold released');
  assert.equal(woke, 2, 'the page searches');
  // the other editors still draw
  openWins(vs.vetRef('a'), 'Veteran #1');
  await settle();
  assert.equal(dialogById(editors, 'wins-dlg').open, true, 'the wins window opens again');
  winsClosed();
  stop();
});

// review-11b 6: a closed editor draws nothing, and one whose node is gone closes, the wins window as the colour picker
test('the uma picker closed draws no list; a G1 wins window whose draft is replaced closes', async () => {
  const editors = document.body.appendChild(document.createElement('div'));
  render(h(VetEditors, null), editors);
  openUmaPicker('Which uma?', () => undefined);
  await settle();
  assert.ok(within(find(editors, '#pick-list')).getAllByRole('button').length > 0, 'open: every uma');
  umaPickClosed();
  await settle();
  assert.equal(byId(editors, 'pick-list'), null, 'closed: none');
  vs.setDraft({ ...node(1001), borrowed: false, parents: [node(1002), node(1003)] });
  let back = 0;
  openWins(vs.draftRef(), 'her', { onClose: () => back++ });
  await settle();
  assert.equal(dialogById(editors, 'wins-dlg').open, true, 'open on her draft');
  vs.setDraft({ ...node(1002), borrowed: false, parents: [node(1001), node(1003)] }); // (a roll: another draft)
  await settle();
  assert.equal(
    dialogById(editors, 'wins-dlg').open,
    false,
    'her draft gone: the window closes, as the colour picker does',
  );
  assert.equal(back, 1, 'its close called');
});
test('a pill switch: radios and tabs are one Tab stop, the arrows pick and focus; toggles each a stop', async () => {
  const root = document.body.appendChild(document.createElement('div'));
  const draw = (kind: 'radio' | 'tab' | 'toggle', value: string, onPick: (v: string | number | boolean) => void) =>
    render(
      h(SegmentedControl, {
        kind,
        label: 'Count',
        value,
        onPick,
        options: [
          { v: '1', body: 'One' },
          { v: '3', body: 'Three' },
        ],
      }),
      root,
    );
  const key = (el: HTMLElement, k: string) => el.dispatchEvent(new KeyboardEvent('keydown', { key: k, bubbles: true }));
  let picked: (string | number | boolean)[] = [];
  draw('radio', '1', (v) => {
    picked.push(v);
  });
  await settle();
  const group = within(root).getByRole('radiogroup', { name: 'Count' }),
    one = within(group).getByRole('radio', { name: 'One' }),
    three = within(group).getByRole('radio', { name: 'Three' });
  assert.equal(one.getAttribute('aria-checked'), 'true');
  assert.deepEqual([one.tabIndex, three.tabIndex], [0, -1], 'the chosen radio is the Tab stop');
  one.focus();
  key(one, 'ArrowRight');
  assert.deepEqual(picked, ['3'], 'ArrowRight picks the next');
  assert.equal(document.activeElement, three, '… and focuses it');
  three.click();
  assert.deepEqual(picked, ['3'], 'a radio picked again is no new pick');
  picked = [];
  draw('tab', '1', (v) => {
    picked.push(v);
  });
  await settle();
  key(within(root).getByRole('tab', { name: 'One' }), 'End');
  assert.deepEqual(picked, ['3'], 'End picks the last tab');
  draw('toggle', '3', () => undefined);
  await settle();
  const pressed = within(root).getByRole('button', { name: 'Three' });
  assert.equal(pressed.getAttribute('aria-pressed'), 'true', 'a toggle says pressed');
  assert.equal(pressed.getAttribute('tabindex'), null, 'and is a Tab stop of its own');
  render(null, root);
  root.remove();
});
