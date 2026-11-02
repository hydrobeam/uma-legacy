// @vitest-environment happy-dom
// The career page's options table ("Other careers", career/view/Options.tsx over career/options.ts) and its (i) popup
// ("What these mean"), drawn from plans as a run leaves them in the answer store: each row's extra careers against the
// pick (– the pick, ≈0 within 0.25, else signed), its tags (the pick, n/n parents, pick varies), the popup's route and
// explanation, and its notes (no career saves its own run, with the borrowed step None left today keeps out; a check cut
// short for memory; a check that did not finish), with nothing under the table; the focus kept on the (i) while the
// check writes its report. npx vitest run (in webapp)
import { expect, test, vi } from 'vitest';
import { act, fireEvent, render, screen, within } from '@testing-library/preact';
import { loadData } from '../../src/model/data.ts';
import { initStore } from '../../src/state/settings.ts';
import { setPlanCheck } from '../../src/career/state.ts';
import { keyOf } from '../../src/career/keys.ts';
import { Results, SavesPopup } from '../../src/career/view/Options.tsx';
import type { PlanOption } from '../../src/career/job.ts';
import { career, careerJob, check, completeVet, loadVets, member, option, plan, showPlan } from './careerplans.ts';
import { memory } from './fakes.ts';

vi.mock('#page-data', () => import('./fakepage.ts'));

document.documentElement.className = 'game'; // (the planner pages' class)
await loadData({ factors: {}, saddles: {}, pinks: {}, races: [] });
initStore(() => memory());
const POOL = ['a', 'b', 'c'];

// the pick (Uma C on #1 + #2), a row 0.1 below it and one 0.8 below, every score under 0
const rows = (over: Partial<PlanOption> = {}): PlanOption[] => [
  option(career(1003, 'Uma C', [0, 1]), -0.4, { chosen: true, ...over }),
  option(career(1005, 'Uma E', [0, 1]), -0.5),
  option(career(1002, 'Uma B', [0, 2]), -1.2),
];
function draw() {
  render(
    <>
      <div id="results">
        <Results />
      </div>
      <SavesPopup />
    </>,
  );
  return screen.getByRole('button', { name: 'What these numbers mean' });
}
const table = () => {
  const section = document.querySelector('section[data-sec="options"]');
  if (!(section instanceof HTMLElement)) throw new Error('the options table is not drawn');
  return section;
};
const extras = () => [...table().querySelectorAll('tbody [data-extra]')].map((x) => x.textContent);
const tags = (tag: string) => table().querySelectorAll(`tbody [data-tag="${tag}"]`).length;
// the (i) popup's notes, opened by the (i)
async function notes(info: HTMLElement): Promise<string[]> {
  await act(() => {
    fireEvent.click(info);
  });
  const dialog = document.getElementById('save-dlg');
  if (!(dialog instanceof HTMLDialogElement)) throw new Error('no Saves popup');
  expect(dialog.open).toBe(true);
  return [...dialog.querySelectorAll('[data-note]')].map((p) => p.textContent ?? '');
}

test('no career saves its own run: the pick shows –, a row within 0.25 ≈0 (no tag), and the (i) popup says it in one line, nothing under the table', async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  showPlan(plan(rows()), careerJob(), POOL);
  const info = draw();
  expect(extras()).toEqual(['–', '≈0', '+0.8']);
  expect(tags('pick')).toBe(1);
  expect(tags('tie')).toBe(0);
  expect(table().querySelectorAll('p[data-note], p, [data-route]').length).toBe(0);
  expect(await notes(info)).toEqual(['No career saves its own run today: the pick costs least.']);
});

test("under None left today, where the estimate's next step needs a borrowed parent, the (i) popup names her", async () => {
  loadVets([completeVet('a', 1001, { borrowed: true }), completeVet('b', 1002), completeVet('c', 1004)]);
  const report = plan(rows(), { members: [member('Uma C', [0, 1]), member('Uma E', [1, 2])], v0: 250 });
  showPlan(report, careerJob({ borrowed_parent: 'later' }), POOL);
  expect(await notes(draw())).toEqual([
    'No career saves its own run today: the pick costs least. Its next step needs a borrowed parent (#1 Uma A), kept out by None left today.',
  ]);
});

test('a check whose variants disagree tags each career they pick; one cut short for memory says so once; one that could not finish says so', async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  const options = rows(),
    [pick, near] = options;
  if (!pick || !near) throw new Error('two rows');
  const robust = check(keyOf(pick), {
    stable: false,
    pick_varies: true,
    contenders: [keyOf(pick), keyOf(near)],
    scoring_complete: false,
  });
  showPlan(plan(options, { robust }), careerJob(), POOL);
  const said = await notes(draw());
  expect(tags('varies')).toBe(2);
  expect(said.filter((t) => /ran out of memory/.test(t))).toHaveLength(1);
  expect(said.some((t) => /did not finish/.test(t))).toBe(false);
});

test('a check that could not finish (the worker lost the plan) says so in the (i) popup', async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  showPlan(plan(rows(), { check_unavailable: true }), careerJob(), POOL);
  const said = await notes(draw());
  expect(said.filter((t) => /did not finish/.test(t))).toHaveLength(1);
  expect(tags('varies')).toBe(0);
});

test("the focus stays on the table's (i) while the check writes its report (review-8c 5)", async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  const options = rows(),
    [pick] = options;
  if (!pick) throw new Error('a pick');
  const id = showPlan(plan(options), careerJob(), POOL);
  const info = draw();
  info.focus();
  expect(document.activeElement).toBe(info);
  await act(() => {
    setPlanCheck(id, check(keyOf(pick), { pending: 3 }));
  });
  await act(() => {
    setPlanCheck(id, check(keyOf(pick), { pending: 0, scoring_complete: false }));
  });
  expect(info.isConnected).toBe(true);
  expect(document.activeElement).toBe(info);
});

test('careers on two full veterans are tagged n/n parents, the last run among them', () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  const options = [
    option(career(1003, 'Uma C', [0, 1]), 0.6, { chosen: true, looped: true, role: 'finish' }),
    option(career(1005, 'Uma E', [0, 1]), 0.2, { looped: true }),
    option(career(1002, 'Uma B', [0, 2]), 0.1),
  ];
  showPlan(plan(options), careerJob(), POOL);
  draw();
  expect(tags('pick')).toBe(1);
  const looped = [...table().querySelectorAll('tbody [data-tag="looped"]')].map((t) => t.textContent);
  expect(looped).toEqual(['8/8 parents', '8/8 parents']);
});

test('with no looped parent the (i) popup walks the cheapest trio step by step and says what Extra careers means; its backdrop closes it', async () => {
  loadVets([completeVet('a', 1001), completeVet('b', 1002), completeVet('c', 1004)]);
  const options = [
    option(career(1003, 'Uma C', [0, 1]), 0.9, { chosen: true }),
    option(career(1005, 'Uma E', [0, 1]), 0.8),
    option(career(1002, 'Uma B', [0, 2]), -0.3),
  ];
  const report = plan(options, {
    v0: 1234,
    members: [member('Uma C', [0, 1], { p: 0.01 }), member('Uma E', [1, 2], { p: 0.004 })],
    v0_plan: { a: 'Uma C', b: 'Uma E', c: 'Uma A', careers: [100, 250, 884], sides: ['a', 'b'] },
  });
  showPlan(report, careerJob(), POOL);
  const info = draw();
  expect(extras().every((c) => /^(–|≈0|[+−]\d+\.\d)$/.test(c ?? ''))).toBe(true);
  expect(extras()).toEqual(['–', '≈0', '+1.2']);
  expect(await notes(info)).toEqual([]);
  const dialog = document.getElementById('save-dlg');
  if (!(dialog instanceof HTMLDialogElement)) throw new Error('no Saves popup');
  const route = within(dialog).getAllByText(/^First /)[0]?.textContent ?? '';
  expect(route).toMatch(
    /^First .+; then .+; then .+ with those two \d+\/\d+s as parents.*\. About [\d,]+ careers in all\.$/,
  );
  expect(route).toMatch(/1 in [\d,]+/);
  const text = dialog.textContent ?? '';
  expect(text).toMatch(
    /Extra careers: how many more careers than the pick this one costs on the planner's estimate \(below\) of the careers you still need for a fully looped parent .+\. .+ It is for comparing careers, not a forecast\./,
  );
  expect(text).toMatch(/The planner makes a rough estimate of how many careers you still need \(about [\d,]+ today\)/);
  // a click on the dialog itself (its backdrop) closes it
  await act(() => {
    fireEvent.click(dialog);
  });
  expect(dialog.open).toBe(false);
});
