// @vitest-environment happy-dom
// Where a veteran came from, as her popup and her card say it: one from an uma.moe tree is "From uma.moe", one added
// from Try a run "Entered here", and her card in the veterans dialog names neither. One veteran left, the page asks
// for parents (job.ts). A G1 win removed by a click and added back at the Senior turn of her race (results/Agenda.tsx).
// npx vitest run (in webapp)
import { render, screen } from '@testing-library/preact';
import { WinsEditor } from '../../src/results/Agenda.tsx';
import { expect, test, vi } from 'vitest';
import { loadData } from '../../src/model/data.ts';
import { Blocked, buildJob } from '../../src/career/job.ts';
import { initStore, settings } from '../../src/state/settings.ts';
import { NodeDialog } from '../../src/career/view/NodeDialog.tsx';
import { openNode } from '../../src/career/view/dialogs.ts';
import { PoolBody } from '../../src/career/view/Pool.tsx';
import { openPool } from '../../src/career/view/poolState.ts';
import * as vs from '../../src/veterans/store.ts';
import type { LineageNode, Veteran } from '../../src/veterans/store.ts';
import { CAREER } from '../browser/listings.ts';
import { memory } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));
document.documentElement.className = 'game';
await loadData(CAREER);
initStore(() => memory());

const node = (chara: number): LineageNode => ({ chara, sparks: {}, pinks: { 11: 1 }, blues: {}, greens: {}, wins: [] });
const vet = (id: string, src: string): Veteran => ({
  id,
  src,
  borrowed: false,
  ...node(1001),
  parents: [node(1002), node(1003)],
});
const settle = async () => {
  for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
};

test('her popup says where she came from: an uma.moe tree, or entered here; her card in the list says neither', async () => {
  vs.initVets({ storage: () => memory(), hidden: () => false, afterFrame: () => undefined });
  vs.addVets([vet('t', 'tree'), vet('e', 'entered')]);
  const { container } = render(<NodeDialog />);
  const said = async (id: string) => {
    openNode({ kind: 'v', id });
    await settle();
    return container.querySelector('#node-dlg [data-src]')?.textContent;
  };
  expect(await said('t')).toBe('From uma.moe');
  expect(container.querySelector('#node-dlg [data-src]')?.getAttribute('data-src')).toBe('umamoe');
  expect(await said('e')).toBe('Entered here');
  expect(container.querySelector('#node-dlg [data-src]')?.getAttribute('data-src')).toBe('here');
  const list = render(<PoolBody />);
  openPool();
  await settle();
  expect(list.container.querySelectorAll('#vets [data-vet]')).toHaveLength(2);
  expect(list.container.querySelectorAll('#vets [data-src]')).toHaveLength(0);
  expect(list.container.querySelector('#vets')?.textContent).not.toMatch(/From uma\.moe|Entered here/);
});

test('down to one veteran: the page asks for both parents, or an import (no search)', () => {
  vs.initVets({ storage: () => memory(), hidden: () => false, afterFrame: () => undefined });
  vs.addVets([vet('a', 'import')]);
  const from = {
    settings: settings.peek(),
    vets: vs.vets.peek(),
    ready: vs.readyVets.peek(),
    legacies: vs.legaciesJob(),
    places: { trainee: null, parents: [null, null] },
    chosen: null,
  };
  const acts = { importVets: () => undefined, trySamples: () => undefined };
  expect(() => buildJob(from, acts)).toThrow(Blocked);
  expect(() => buildJob(from, acts)).toThrow(/^Choose both parents, or import/);
});

test("a veteran's G1 wins: a click removes Arima Kinen, the Senior Late Dec menu adds it back there", async () => {
  const edits: { wins: readonly number[]; winAt: Readonly<Record<number, number>> }[] = [];
  const races = [{ ix: 22, name: 'Arima Kinen', turns: [47, 71] }];
  const onWins = (wins: readonly number[], winAt: Readonly<Record<number, number>>) => void edits.push({ wins, winAt });
  const { container, rerender } = render(<WinsEditor races={races} wins={[22]} winAt={{ 22: 47 }} onWins={onWins} />);
  await settle();
  const win = container.querySelector<HTMLElement>('button[data-ix="22"]');
  expect(win).not.toBeNull();
  win?.click();
  await settle();
  expect(edits.at(-1)?.wins).toEqual([]);
  rerender(<WinsEditor races={races} wins={[]} winAt={{}} onWins={onWins} />);
  await settle();
  expect(container.querySelector('button[data-ix="22"]')).toBeNull();
  screen.getByRole('button', { name: 'Senior Year, Late Dec: add a G1 win' }).click();
  await settle();
  screen
    .getByRole('dialog', { name: /G1s held$/ })
    .querySelector<HTMLElement>('button')
    ?.click();
  await settle();
  expect(edits.at(-1)).toEqual({ wins: [22], winAt: { 22: 71 } });
});
