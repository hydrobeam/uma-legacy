// The shared setup editors (ui/Presets, ui/Targets) on a fake document (fakedom.mjs), bundled with esbuild as the page
// bundles them: what each edit stores. Presets by name: Load, Overwrite, a rename, a rename onto a taken name, Delete
// after a rename (review-11a F1); a target's removal and where the focus goes after it; the rank's ? keeps its click
// (F2); a sort menu over a stored value it doesn't list (F3); two presets of one name in storage (F4); the
// editors redraw for the settings they show only (F6).
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

const WEBAPP = fileURLToPath(new URL('../..', import.meta.url));
const built = await esbuild.build({
  stdin: {
    contents: `
      import '@preact/signals';
      export * as settings from './src/settings.ts';
      export * as presets from './src/presets.js';
      export { PresetDialogs, openPresets } from './src/ui/Presets.tsx';
      export { mountTargets } from './src/ui/Targets.tsx';
      export { mountRank } from './src/ui/Scenario.tsx';
      export { SettingsSort, SortMenu } from './src/ui/SortMenu.tsx';
      export { mountRoster } from './src/ui/Roster.tsx';
      export { h, options, render } from 'preact';
      export { gameData, loadData } from './src/data.js';`,
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
const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'llp-setup-')),
  file = path.join(dir, 'setup.mjs');
fs.writeFileSync(file, built.outputFiles[0].text);
const page = await import(pathToFileURL(file).href);
fs.rmSync(dir, { recursive: true });
const {
  settings: S,
  presets: P,
  PresetDialogs,
  openPresets,
  mountTargets,
  mountRank,
  SettingsSort,
  SortMenu,
  mountRoster,
  h,
  options,
  render,
  gameData,
  loadData,
} = page;
// the outfits' labels the Umas panel shows (the fake page's outfits have none)
for (const u of globalThis.__LLP_DATA['data-umas']) for (const o of u.outfits || []) o.label = `[Outfit ${o.id}]`;
await loadData();

const KEY = 'llp:settings';
// the store loaded from a new storage holding these shared settings; returns what is stored, read afresh each call
const fresh = (common) => {
  const map = new Map([[KEY, JSON.stringify({ v: 2, era: 'global', common, global: {} })]]);
  S.initStore(() => ({ getItem: (k) => map.get(k) ?? null, setItem: (k, v) => void map.set(k, v) }));
  return () => JSON.parse(map.get(KEY)).common;
};
const settle = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r, 0));
};
const descendants = (root) => {
  const out = [];
  const walk = (n) => {
    for (const c of n.childNodes)
      if (c.nodeType === 1) {
        out.push(c);
        walk(c);
      }
  };
  walk(root);
  return out;
};
const byId = (root, id) => descendants(root).find((e) => e.id === id || e.attributes.id === id);
const text = (e) => e.textContent.trim();
// typing into a box: its value, then the input event Preact listens for
const type = (box, value) => {
  box.value = value;
  for (const f of box.listeners.input || []) f.call(box, { type: 'input', target: box, currentTarget: box });
};

test('presets by name: Load, a rename onto a taken name (refused), a rename, Overwrite and Delete of the renamed one', async () => {
  const stored = fresh({
    presets: [
      { name: 'A', groups: [20049] },
      { name: 'B', groups: [20046, 20166] },
    ],
    targets: [{ group: 20159 }],
  });
  const root = document.createElement('div');
  render(h(PresetDialogs, {}), root);
  openPresets();
  await settle();
  const rows = () => root.querySelectorAll('div.prow');
  const row = (name) => rows().find((r) => r.attributes['aria-label'] === name);
  const button = (name, label) =>
    row(name)
      .querySelectorAll('button.btn')
      .find((b) => text(b) === label);
  assert.deepEqual(
    rows().map((r) => r.attributes['aria-label']),
    ['A', 'B'],
  );
  // Load B: its skills become the targets, the presets stay as they were
  button('B', 'Load').click();
  await settle();
  assert.deepEqual(stored().targets, [{ group: 20046 }, { group: 20166 }]);
  assert.deepEqual(
    stored().presets.map((p) => p.name),
    ['A', 'B'],
  );
  // A's view: a rename onto B is refused and stores nothing; a rename to "  C " stores C (trimmed) in A's place
  openPresets();
  await settle();
  row('A').querySelector('button.pview').click();
  await settle();
  assert.equal(text(byId(root, 'prv-title')), 'A');
  byId(root, 'prv-rename').click();
  await settle();
  type(byId(root, 'prv-name'), 'B');
  await settle();
  byId(root, 'prv-rename').click(); // Save name
  await settle();
  assert.match(text(byId(root, 'prv-msg')), /“B” is taken/);
  assert.deepEqual(
    stored().presets.map((p) => p.name),
    ['A', 'B'],
  );
  type(byId(root, 'prv-name'), '  C ');
  await settle();
  byId(root, 'prv-rename').click();
  await settle();
  assert.equal(text(byId(root, 'prv-title')), 'C', 'the view follows the new name');
  assert.deepEqual(stored().presets, [
    { name: 'C', groups: [20049] },
    { name: 'B', groups: [20046, 20166] },
  ]);
  // the view closed, C's row (keyed by its new name) overwrites C alone, then its trash deletes C alone
  byId(root, 'pr-view').querySelector('button.dclose').click();
  await settle();
  button('C', 'Overwrite').click();
  await settle();
  assert.deepEqual(stored().presets, [
    { name: 'C', groups: [20046, 20166] },
    { name: 'B', groups: [20046, 20166] },
  ]);
  assert.match(text(byId(root, 'pr-msg')), /Overwrote “C” with your 2 skills\./);
  row('C').querySelector('button.ptrash').click();
  await settle();
  assert.deepEqual(stored().presets, [{ name: 'B', groups: [20046, 20166] }]);
  assert.match(text(byId(root, 'pr-msg')), /Deleted “C”\./);
  render(null, root);
});

test("a target's × removes its own place (one skill held twice too); the focus goes to the next ×, else the last", async () => {
  const stored = fresh({ targets: [{ group: 20049 }, { group: 20159 }, { group: 20049 }, { group: 20166 }] });
  mountTargets();
  await settle();
  const box = document.getElementById('skills-panel');
  const removes = () => box.querySelectorAll('button.x');
  const groups = () => stored().targets.map((t) => t.group);
  const labels = () => removes().map((b) => b.attributes['aria-label']);
  assert.deepEqual(labels(), ['Remove Skill 20049', 'Remove Skill 20159', 'Remove Skill 20049', 'Remove Skill 20166']);
  removes()[1].click(); // the second place
  await settle();
  assert.deepEqual(groups(), [20049, 20049, 20166]);
  assert.deepEqual(labels(), ['Remove Skill 20049', 'Remove Skill 20049', 'Remove Skill 20166']);
  assert.ok(document.activeElement === removes()[1], 'the focus on the next place');
  removes()[0].click();
  await settle();
  assert.deepEqual(groups(), [20049, 20166]);
  assert.ok(document.activeElement === removes()[0], 'the focus on the next place');
  removes()[1].click(); // the last
  await settle();
  assert.deepEqual(groups(), [20049]);
  assert.ok(document.activeElement === removes()[0], 'the focus on the last place left');
  render(null, box);
});

test("the lineage rank's ? opens its note and stops its click there (other popups' outside-click handlers never see it)", async () => {
  fresh({});
  mountRank();
  await settle();
  const box = document.getElementById('rank-ctl'),
    question = byId(box, 'rank-q');
  let stopped = false;
  const click = { type: 'click', target: question, currentTarget: question, preventDefault: () => undefined };
  for (const f of question.listeners.click || [])
    f.call(question, {
      ...click,
      stopPropagation: () => {
        stopped = true;
      },
    });
  await settle();
  assert.equal(question.attributes['aria-expanded'], 'true', 'the note is open');
  assert.ok(stopped, 'the click went no further than the ?');
  render(null, box);
});

test('a sort menu given a value it does not list (a hand-edited storage) shows its first item instead of throwing', async () => {
  fresh({ rsort: 'bogus', rdir: 'sideways' });
  const root = document.createElement('div');
  // the umas' menu: read as the roster orders by it (release, ascending)
  render(h(SettingsSort, { prefix: 'r', id: 'rs', what: 'sort umas' }), root);
  await settle();
  assert.equal(text(byId(root, 'rs')), 'Release ↑');
  assert.equal(byId(root, 'rs').attributes.title, 'Sorted oldest first');
  const items = [
    { k: 'a', label: 'First', hint: '' },
    { k: 'b', label: 'Second', hint: '' },
  ];
  const props = { id: 'vs', cls: 'btn', what: 'order', items, value: 'gone', arrow: '↓', title: '', onPick: () => 0 };
  render(h(SortMenu, props), root);
  await settle();
  assert.equal(text(byId(root, 'vs')), 'First ↓');
  render(null, root);
});

test('two presets of one name in storage (a hand-edit): the first is kept, so Overwrite and Delete hit one', () => {
  const stored = fresh({
    presets: [
      { name: 'X', groups: [20049] },
      { name: 'X', groups: [20046] },
      { name: 'Y', groups: [20166] },
    ],
    targets: [{ group: 20159 }],
  });
  assert.deepEqual(
    S.settings.value.presets.map((p) => [p.name, ...p.groups]),
    [
      ['X', 20049],
      ['Y', 20166],
    ],
  );
  P.overwritePreset('X');
  assert.deepEqual(stored().presets, [
    { name: 'X', groups: [20159] },
    { name: 'Y', groups: [20166] },
  ]);
  P.deletePreset('X');
  assert.deepEqual(stored().presets, [{ name: 'Y', groups: [20166] }]);
});

test('a commit of a setting the editors do not show redraws none of them and re-runs no roster tile', async () => {
  fresh({});
  mountRoster(false);
  mountTargets();
  await settle();
  type(byId(document.getElementById('skills-panel'), 't-add'), 'sk'); // the suggestions listed
  await settle();
  // the components drawn from here on (Preact's hook, chained to the signals' own), and each tile's reads of her id
  const drawn = [],
    chained = options.diffed;
  options.diffed = (vnode) => {
    if (typeof vnode.type === 'function') drawn.push(vnode.type.name);
    chained?.(vnode);
  };
  let idReads = 0,
    readsAfter = -1;
  for (const u of gameData().umas) {
    const id = u.id;
    Object.defineProperty(u, 'id', {
      get: () => {
        idReads++;
        return id;
      },
      configurable: true,
    });
  }
  try {
    S.setRank('0.20,0.70,0.10');
    await settle();
    readsAfter = idReads;
  } finally {
    options.diffed = chained;
    for (const u of gameData().umas) Object.defineProperty(u, 'id', { value: u.id, writable: true, enumerable: true });
  }
  assert.deepEqual(drawn, [], 'no editor redrawn');
  assert.equal(readsAfter, 0, "no tile's state re-read");
  render(null, document.getElementById('umas-panel'));
  render(null, document.getElementById('skills-panel'));
});
