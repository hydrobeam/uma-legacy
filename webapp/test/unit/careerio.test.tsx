// @vitest-environment happy-dom
// The career page's imports and its Export (career/io.ts) as the veterans dialog shows them (view/Pool.tsx), on the
// Global era's data and umadump's test file (val/umadump/U_trained_chara_data.json, 12 veterans): the file and its
// message, a borrow list replacing the last, a partial file merging, Remove all on one tab, an export of the other era
// refused, a cut veteran drawn, and an Export taken back whole. npx vitest run (in webapp)
import assert from 'node:assert/strict';
import fs from 'node:fs';
import path from 'node:path';
import { test, vi } from 'vitest';
import { fireEvent, render, screen, within } from '@testing-library/preact';
import { loadData } from '../../src/data.ts';
import { initStore } from '../../src/settings.ts';
import * as vs from '../../src/veterans/store.ts';
import { exportText, importText } from '../../src/career/io.ts';
import { VetsDialog } from '../../src/career/view/Pool.tsx';
import { choose, openPool, setVTab, vetsNote } from '../../src/career/view/poolState.ts';
import { ERA } from '../../src/saved.ts';
import { memory } from './fakes.ts';
import { CAREER } from './globalpage.ts';

vi.mock('#page-data', () => import('./globalpage.ts'));
vi.mock('#career-data', async () => ({ default: (await import('./globalpage.ts')).CAREER }));

await loadData(CAREER);
const DUMP_FILE = path.resolve(import.meta.dirname, '../../../val/umadump/U_trained_chara_data.json');
const DUMP = fs.readFileSync(DUMP_FILE, 'utf8');
const rows: Record<string, unknown>[] = JSON.parse(DUMP);
// a borrow list: umadump's rental_chara_data.json, the lender's viewer id as owner_viewer_id
const lend = (n: number, id: number) =>
  JSON.stringify(rows.slice(0, n).map((r) => ({ ...r, viewer_id: id, owner_viewer_id: id })));
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};

/** A fresh page: settings and veterans in memory, the veterans dialog drawn and open. */
async function page() {
  const storage = memory();
  initStore(() => storage);
  vs.initVets({ storage: () => storage, hidden: () => true, afterFrame: (f) => f() });
  document.documentElement.className = 'game';
  render(<VetsDialog />);
  openPool();
  await settle();
  const cards = () => document.querySelectorAll('#vets [data-vet]').length;
  const tabs = () =>
    ['v-tab-own', 'v-tab-bor'].map((id) => (document.getElementById(id)?.textContent ?? '').trim()).join(' | ');
  const onBorrowed = () => document.getElementById('v-tab-bor')?.getAttribute('aria-selected') === 'true';
  return { storage, cards, tabs, onBorrowed };
}

test('the umadump file imports 12 veterans, and the sample veterans tried before it go', async () => {
  const p = await page();
  const [sample] = vs.vets.peek();
  assert.equal(sample, undefined, 'none at first');
  const node = { chara: 1001, sparks: {}, pinks: { 11: 1 }, blues: {}, greens: {}, wins: [] };
  vs.addVets([{ id: 'm1', src: 'sample', borrowed: false, ...node, parents: [node, node] }]);
  const input = document.getElementById('v-file');
  assert.ok(input instanceof HTMLInputElement, 'the file input');
  Object.defineProperty(input, 'files', { value: [new File([DUMP], 'U_trained_chara_data.json')] });
  fireEvent.change(input);
  await settle();
  assert.match(vetsNote.peek(), /Imported 12 veterans/);
  assert.equal(p.cards(), 12, '12 cards');
  assert.ok(!vs.vets.peek().some((v) => v.src === 'sample'), 'no sample stays');
  const tools = within(screen.getByRole('dialog', { hidden: true }));
  assert.ok(tools.getByText('Import umadump file', { exact: false }), 'with its import');
  assert.equal(document.querySelectorAll('#set-vets #v-paste, #set-vets #v-clear, #set-vets #v-file').length, 3);
});

test('a borrow list goes to the Borrowed tab and the next one replaces it; your partial file merges and removes nothing; Remove all on the Borrowed tab takes only the borrowed', async () => {
  const p = await page();
  importText(DUMP);
  await settle();
  const seen = () => `${vetsNote.peek()} | ${p.onBorrowed()} | ${p.cards()} | ${p.tabs()}`;
  importText(lend(3, 900000000001));
  await settle();
  assert.equal(
    seen(),
    'Replaced the borrow list with 3 borrowed veterans. | true | 3 | Veteran Umamusume · 12 | Guests · 3',
  );
  importText(lend(2, 900000000002));
  await settle();
  assert.equal(
    seen(),
    'Replaced the borrow list with 2 borrowed veterans (it had 3). | true | 2 | Veteran Umamusume · 12 | Guests · 2',
  );
  importText(JSON.stringify(rows.slice(0, 5)));
  await settle();
  assert.equal(
    seen(),
    'Imported 5 veterans of yours: 5 already here, updated. | false | 12 | Veteran Umamusume · 12 | Guests · 2',
  );
  setVTab('bor');
  await settle();
  Object.assign(globalThis, { confirm: () => true });
  fireEvent.click(screen.getByRole('button', { name: 'Remove all', hidden: true }));
  await settle();
  assert.equal(seen(), 'Removed 2 borrowed veterans. | true | 0 | Veteran Umamusume · 12 | Guests · 0');
  assert.match(document.getElementById('vets-bor')?.textContent ?? document.body.textContent ?? '', /None yet/);
  setVTab('own');
  await settle();
  assert.equal(p.cards(), 12, 'your 12 stay');
});

test('Export carries the veterans and the job; an export of the other era is refused; a cut veteran draws; Paste takes the export back', async () => {
  const p = await page();
  importText(DUMP);
  await settle();
  const text = exportText();
  const ex: {
    app: string;
    era: string;
    veterans: { id: string }[];
    job: { cmd: string; career?: { legacies: unknown[] } } | null;
  } = JSON.parse(text);
  assert.equal(ex.app, 'uma-legacy-veterans');
  assert.equal(ex.era, ERA);
  assert.equal(ex.veterans.length, 12);
  assert.equal(ex.job?.cmd, 'career');
  assert.equal(ex.job?.career?.legacies.length, 12, 'the job names the 12');
  const ids = () => vs.vets.peek().map((v) => v.id);
  const ids0 = ids();
  importText(JSON.stringify({ ...ex, era: ERA === 'gm' ? 'global' : 'gm', veterans: ex.veterans.slice(0, 3) }));
  await settle();
  assert.match(vetsNote.peek(), /^Not imported: that export is of the .+ data\.$/);
  assert.deepEqual(ids(), ids0, "this era's veterans stay");
  assert.equal(p.cards(), 12);
  // a veteran whose record lacks her sparks, wins and character (cut or edited by hand) draws, those taken as empty
  const reported: unknown[] = [];
  Object.assign(globalThis, { reportError: (e: unknown) => reported.push(e) });
  importText(JSON.stringify({ ...ex, veterans: [...ex.veterans, { id: 'x', parents: [{}, {}] }] }));
  choose('x');
  await settle();
  assert.equal(p.cards(), 13, '13 cards');
  assert.deepEqual(reported, [], 'drawn without errors');
  importText(text); // an Export replaces every import: the 12 again
  await settle();
  assert.match(vetsNote.peek(), /Imported 12 veterans from an export/);
  assert.deepEqual(
    ids(),
    ex.veterans.map((v) => v.id),
  );
  assert.equal(p.cards(), 12);
});
