// @vitest-environment happy-dom
// The loop page's result popups on happy-dom's document, drawn from the page's own example (webapp/example.json: the
// default search's reply, a three-uma rotation with a deck per uma) over the real Global data (realpage.ts): the deck
// card and member panels (their GameTora links, their Owned switches and what those change), the chain-end advice, the
// race schedule (a tab per member and "All three", the year grids, the win odds and why) and the affinity table.
// npx vitest run (in webapp)
import fs from 'node:fs';
import path from 'node:path';
import { createRef } from 'preact';
import { fireEvent, render, within } from '@testing-library/preact';
import { beforeEach, expect, test, vi } from 'vitest';
import { loadData } from '../../src/model/data.ts';
import * as S from '../../src/settings.ts';
import { initLoopState } from '../../src/loop/state.ts';
import { Results, drawResult } from '../../src/loop/Results.tsx';
import { AffinityDialog } from '../../src/loop/Affinity.tsx';
import { ScheduleDialog } from '../../src/ui/Schedule.tsx';
import { CardPop } from '../../src/ui/CardPop.tsx';
import { CardsPanel } from '../../src/ui/Cards.tsx';
import { closePop, initPopover } from '../../src/popover.ts';
import { jobTargets } from '../../src/model/job.ts';
import type { DrawnReport, LoopReport } from '../../src/loop/types.ts';
import { memory } from './fakes.ts';
import { REPO, skills } from './realpage.ts';

vi.mock('#page-data', () => import('./realpage.ts'));

document.documentElement.className = 'game';
await loadData(null);
initPopover();

const EXAMPLE: LoopReport = JSON.parse(fs.readFileSync(path.join(REPO, 'webapp/example.json'), 'utf8'));
// the example, its first deck's first card made Seiun Sky [Foolproof Plan] (30008) ending its story at link 1 (the
// example ends no chain)
function report(): DrawnReport {
  const copy: LoopReport = structuredClone(EXAMPLE);
  const [first, ...rest] = copy.top;
  if (!first) throw new Error('the example has a result');
  first.deck.card_ids[0] = 30008;
  first.deck.end_chain_at[0] = 1;
  return { ...copy, top: [first, ...rest] };
}
const settle = async () => {
  for (let i = 0; i < 6; i++) await new Promise((r) => setTimeout(r, 0));
};
const $ = <E extends Element = HTMLElement>(sel: string, root: ParentNode = document): E => {
  const found = root.querySelector<E>(sel);
  if (!found) throw new Error(`no ${sel}`);
  return found;
};
const $$ = <E extends Element = HTMLElement>(sel: string, root: ParentNode = document) => [
  ...root.querySelectorAll<E>(sel),
];
const first = <T,>(list: T[], what: string): T => {
  const [item] = list;
  if (item === undefined) throw new Error(`no ${what}`);
  return item;
};

beforeEach(async () => {
  closePop();
  S.initStore(memory);
  initLoopState();
  const column = createRef<HTMLElement>();
  render(
    <>
      <div id="results">
        <Results column={column} />
        <CardPop home={null} />
      </div>
      <div id="set-cards">
        <CardsPanel fadeMore={false} />
      </div>
      <ScheduleDialog />
      <AffinityDialog />
    </>,
  );
  drawResult(report(), jobTargets(S.settings.peek()));
  await settle();
});

const panel = () => $('[data-pop="card"]');
const owned = () => within(panel()).getByRole('switch', { name: 'Owned' });

test('a deck card: its panel links its GameTora page; its Owned switch changes the picker and greys it; Escape closes', async () => {
  const card = $('#results button[data-cid]');
  expect(card.dataset.cid).toBe('30008');
  fireEvent.click(card);
  await settle();
  expect(panel().hidden).toBe(false);
  expect(
    within(panel())
      .getByRole('link', { name: /GameTora/ })
      .getAttribute('href'),
  ).toMatch(/^https:\/\/gametora\.com\/umamusume\/supports\/\d+-[a-z0-9-]+$/);
  // the picker's tile, known by its picture's title (the card's full name)
  const tile = $$('#cards button[aria-pressed]').find(
    (t) => t.querySelector('[data-card]')?.getAttribute('title') === '[Foolproof Plan] Seiun Sky',
  );
  expect(tile?.getAttribute('aria-pressed')).toBe('true');
  fireEvent.click(owned());
  await settle();
  expect(S.settings.peek().cardsOff[30008]).toBe(false);
  expect(tile?.getAttribute('aria-pressed')).toBe('false');
  expect($$('#results button[data-cid][data-off]').length).toBeGreaterThan(0);
  fireEvent.click(owned());
  await settle();
  fireEvent.keyDown(panel(), { key: 'Escape' });
  await settle();
  expect(panel().hidden).toBe(true);
});

test('chain end: the panel names the skill to prioritise and when IT may not take it; a rival target is named', async () => {
  const chip = () => $('#results button[data-cid="30008"][data-end="1"]');
  fireEvent.click(chip());
  await settle();
  const plain = panel().textContent ?? '';
  expect(plain).toContain('Prioritise Second Wind in IT: it takes the chain-ending choice at link 1.');
  expect(plain).toContain(
    "IT may not take it if Dodging Danger is also prioritised, or if Second Wind's hint is already Lv 5.",
  );
  fireEvent.click(chip());
  await settle();
  const dodging = first(
    skills.filter(([, white]) => white === 'Dodging Danger'),
    'Dodging Danger in the data',
  )[0];
  S.setTargets([...S.settings.peek().targets, { group: dodging }]);
  fireEvent.click(chip());
  await settle();
  expect(panel().textContent).toContain('Dodging Danger is also a target: prioritising both makes the end uncertain.');
});

test("a member's panel: one Owned switch, her GameTora page; the switch marks her not owned and greys her", async () => {
  // (the old group's: the first result's second member button)
  const button = $$<HTMLButtonElement>('#results article [data-member] button')[1];
  if (!button) throw new Error('a member button');
  fireEvent.click(button);
  await settle();
  expect(panel().querySelectorAll('input[role=switch]').length).toBe(1);
  expect(
    within(panel())
      .getByRole('link', { name: /GameTora/ })
      .getAttribute('href'),
  ).toMatch(/^https:\/\/gametora\.com\/umamusume\/characters\/\d{6}-[a-z0-9-]+$/);
  fireEvent.click(owned());
  await settle();
  const s = S.settings.peek();
  expect(Object.values(s.roster).filter((x) => x === 'out').length + Object.keys(s.outfits).length).toBe(1);
  expect($$('#results [data-member][data-off]').length).toBeGreaterThan(0);
});

test('a portrait in a swap row opens the same member panel', async () => {
  const face = first($$<HTMLButtonElement>('#results article [data-deck="swap"] button[data-uma]'), 'a swap portrait');
  fireEvent.click(face);
  await settle();
  expect(panel().hidden).toBe(false);
  expect(panel().querySelectorAll('input[data-act="uma"]').length).toBe(1);
  expect($('b', panel()).textContent).toBe(
    $(`#results [data-member][data-uma="${face.dataset.uma}"] [data-nm]`).textContent,
  );
});

test('schedule: a tab per member and "All three"; year grids, the Junior grey cells, each win and why', async () => {
  fireEvent.click(first(within($('#results')).getAllByRole('button', { name: 'Race schedule' }), 'the button'));
  await settle();
  const dlg = $<HTMLDialogElement>('#sched-dlg');
  expect(dlg.open).toBe(true);
  const tabs = within(dlg).getAllByRole('tab');
  expect(tabs.length).toBe(4);
  expect(tabs.at(-1)?.textContent).toBe('All three');
  expect($$('[data-grid]', dlg).length).toBe(3);
  expect($$('[data-year="0"] [data-slot~="pre"]', dlg).length).toBe(12);
  const turns = $$('[data-slot~="on"] [data-turn]', dlg);
  expect(turns.length).toBeGreaterThan(0);
  for (const w of turns) {
    expect(w.textContent).toMatch(/^(Early|Late) [A-Z][a-z]{2}\d+% win$/);
    expect($('b', w).title).toMatch(/^\d+% win: 110/);
  }
  fireEvent.click(tabs.at(-1) ?? dlg);
  await settle();
  expect($$('th[data-k]', $('#sched-view table')).length).toBe(3);
  const when = $$('td[data-when]', dlg);
  expect(when.length).toBeGreaterThan(0);
  for (const t of when) expect(t.textContent).toMatch(/^(Early|Late) [A-Z][a-z]{2}$/);
  expect($('[data-sched-key]', dlg).closest('[hidden]')).not.toBeNull();
  const odds = $$('#sched-view table tbody td').filter((t) => /^\*?\d+$/.test(t.textContent ?? ''));
  expect(odds.length).toBeGreaterThan(0);
  for (const t of odds) expect(t.title).toMatch(/^\d+% win: 110/);
  expect($('[data-tables] > p', dlg).textContent).toMatch(/hover for why/);
});

test('affinity: a rotation has 3 careers, six legacies each and a Total that sums them; the trainee is her own grandparent twice', async () => {
  fireEvent.click(first(within($('#results')).getAllByRole('button', { name: 'Affinity' }), 'the button'));
  await settle();
  const dlg = $<HTMLDialogElement>('#why-dlg');
  expect(dlg.open).toBe(true);
  const table = $('table', dlg);
  const rows = $$<HTMLTableRowElement>('tbody tr', table);
  const n = (x: string | null | undefined) => Number((x ?? '').replace(/\D/g, ''));
  const affinity = rows.map((r) => n(r.cells[3]?.textContent));
  expect($$('[aria-label="Trainee"] [role="radio"]', dlg).length).toBe(3);
  expect(rows.length).toBe(6);
  const total = n($<HTMLTableRowElement>('tfoot tr', table).cells[3]?.textContent);
  expect(Math.abs(affinity.reduce((a, x) => a + x, 0) - total)).toBeLessThanOrEqual(6);
  expect($$('p', dlg).length).toBe(0);
  const own = rows
    .filter((r) => /her own earlier veteran/.test(r.textContent ?? ''))
    .map((r) => n(r.cells[1]?.textContent));
  expect(own).toEqual([0, 0]);
});
