// @vitest-environment happy-dom
// Borrowed parent (Career setup, career/view/Setup.tsx BorrowChoice): Allowed sends no borrowed_parent; None left
// today and Never send 'later' and 'never'; each choice is saved with the settings; a borrowed parent you chose is
// blocked when borrowing is off, the parent picker greys her out, and both stand after a reload (the stores read back
// from storage). npx vitest run (in webapp)
import { expect, test, vi } from 'vitest';
import { act, fireEvent, render } from '@testing-library/preact';
import { loadData } from '../../src/model/data.ts';
import { initStore } from '../../src/settings.ts';
import { initVets, setParent } from '../../src/veterans/store.ts';
import { currentJob } from '../../src/career/current.ts';
import { Blocked } from '../../src/career/job.ts';
import { BorrowChoice } from '../../src/career/view/Setup.tsx';
import { ParentPicker } from '../../src/career/view/ParentPicker.tsx';
import { openParentPicker } from '../../src/career/view/dialogs.ts';
import { completeVet, loadVets } from './careerplans.ts';
import { memory } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

document.documentElement.className = 'game';
await loadData({ factors: {}, saddles: {}, pinks: {}, races: [] });

const settingsStore = memory();
initStore(() => settingsStore);
const saved = (): unknown => JSON.parse(settingsStore.getItem('llp:settings') ?? '{}').common?.borrowPar;
// the job the settings, veterans and places make now, or why they make none
const jobOrBlock = () => {
  try {
    return currentJob();
  } catch (e) {
    if (e instanceof Blocked) return e.message;
    throw e;
  }
};
const BLOCKED = /^Uma B is borrowed, and Borrowed parent is set to Never: choose one of your veterans/;

test('Allowed, None left today, Never: what the job sends and what is saved; a borrowed parent chosen is blocked, greyed in the picker, and stands after a reload', async () => {
  const vetsStore = loadVets([
    completeVet('a', 1001),
    completeVet('b', 1002, { borrowed: true }),
    completeVet('c', 1004),
  ]);
  const { container } = render(<BorrowChoice />);
  const choice = (v: string) => {
    const b = container.querySelector(`#bp button[data-v="${v}"]`);
    if (!(b instanceof HTMLButtonElement)) throw new Error(`no ${v} choice`);
    return b;
  };
  const pressed = () =>
    [...container.querySelectorAll('#bp button[aria-pressed="true"]')].map((b) => b.getAttribute('data-v'));
  const sent = () => {
    const job = jobOrBlock();
    return typeof job === 'string' ? job : job.career.borrowed_parent;
  };
  expect(pressed()).toEqual(['allow']);
  expect(sent()).toBeUndefined();
  // a borrowed veteran is sent as one (the planner uses her as a parent, never as the goal)
  const first = jobOrBlock();
  if (typeof first === 'string') throw new Error(first);
  expect(first.career.legacies.map((l) => l.borrowed ?? false)).toEqual([false, true, false]);
  expect(first.career.fix).toEqual([]);
  for (const v of ['later', 'never'] as const) {
    await act(() => {
      fireEvent.click(choice(v));
    });
    expect(pressed()).toEqual([v]);
    expect(saved()).toBe(v);
    expect(sent()).toBe(v);
  }
  await act(() => {
    fireEvent.click(choice('allow'));
  });
  expect(saved()).toBe('allow');
  expect(sent()).toBeUndefined();
  // the borrowed veteran (#2) chosen as parent 1, then borrowing turned off
  setParent(0, 'b', true);
  expect(typeof jobOrBlock()).not.toBe('string');
  await act(() => {
    fireEvent.click(choice('never'));
  });
  expect(jobOrBlock()).toMatch(BLOCKED);
  render(<ParentPicker />);
  await act(() => openParentPicker(0));
  const list = document.getElementById('vp-list');
  const her = list?.querySelectorAll('button')[1];
  expect(her?.disabled).toBe(true);
  expect(her?.textContent).toMatch(/Can't: borrowed/);
  // a reload: both stores read back from storage
  initStore(() => settingsStore);
  initVets({ storage: () => vetsStore, hidden: () => true, afterFrame: (f) => f() });
  expect(saved()).toBe('never');
  expect(jobOrBlock()).toMatch(BLOCKED);
});
