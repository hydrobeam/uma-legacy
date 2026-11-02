// @vitest-environment happy-dom
// The career setup panel's fold (career/view/Setup.tsx): with no fold chosen, the first veterans fold it (a return
// visit with veterans opens folded, the plan's answer first); a fold chosen (here: opened) is kept in this browser and
// holds after a reload. Each "reload" imports the module afresh over the same localStorage. npx vitest run (in webapp)
import { expect, test, vi } from 'vitest';

vi.mock('#page-data', () => import('./fakepage.ts'));

const KEY = 'llp:setup-folded';
const load = async () => {
  vi.resetModules();
  return import('../../src/career/view/Setup.tsx');
};

test('no fold chosen: veterans fold the setup; opened by its ribbon, it stays open after a reload', async () => {
  localStorage.removeItem(KEY);
  let setup = await load();
  expect(setup.folded.value).toBe(false);
  setup.sawVeterans();
  expect(setup.folded.value).toBe(true);
  // a reload with veterans and no fold chosen: folded again
  setup = await load();
  setup.sawVeterans();
  expect(setup.folded.value).toBe(true);
  // folded, a click anywhere on the bar opens it, and the choice is kept
  const bar = document.body.appendChild(document.createElement('div'));
  bar.id = 'h-set';
  const { foldClick } = setup;
  bar.addEventListener('click', foldClick);
  bar.click();
  expect(setup.folded.value).toBe(false);
  expect(localStorage.getItem(KEY)).toBe('0');
  bar.removeEventListener('click', foldClick);
  setup = await load();
  setup.sawVeterans();
  expect(setup.folded.value).toBe(false);
});
