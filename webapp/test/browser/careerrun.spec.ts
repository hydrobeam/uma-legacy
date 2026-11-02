// The career planner in a real browser, from career-small's plan (test/fixtures/solver: four made-up veterans): the worker's kept work stored in IndexedDB once
// the plan is in and restored into the next page's worker before its first job, the last answer drawn at once by a
// reload with nothing changed (career/run.ts and the run on the page are node-tested: careerplanrun.test.ts); and the
// page on a 375 px phone (iPhone 13 mini): nothing scrolls sideways, page or dialogs, the setup's rows in their panel,
// the pill tracks flush, Try a run's popup on the screen, the pick a card, the tree one column.
import type { Page } from 'playwright/test';
import { expect, test } from './fixtures.ts';
import { SIDEWAYS, SOLVER_WAIT, careerDone } from './pages.ts';
import { quiet } from './solver.ts';

test.describe.configure({ timeout: 60_000 });

test.use({ scenario: 'career-small' });
// the scenario's veterans, planned
async function seededPlan(page: Page) {
  await page.goto('/');
  await expect(page.locator('#run-note')).toHaveText(/Planned in/, { timeout: SOLVER_WAIT });
}
// what the page's IndexedDB holds of the worker's kept work: each stored state's length (the solver's own string, which
// the page stores untouched; the connection closed at once: an open one would block the page's own, which then saves
// nothing)
const storedStates = (page: Page) =>
  page.evaluate(
    () =>
      new Promise<number[]>((resolve) => {
        const rq = indexedDB.open('llp-solver');
        rq.onerror = () => resolve([]);
        rq.onsuccess = () => {
          const db = rq.result;
          try {
            const all = db.transaction('state').objectStore('state').getAll();
            all.onerror = () => resolve([]);
            all.onsuccess = () => resolve(all.result.map((s: unknown) => (typeof s === 'string' ? s.length : 0)));
          } catch {
            resolve([]);
          } finally {
            db.close();
          }
        };
      }),
  );
// the last answer kept to be drawn at once, forgotten (every era's): the next load plans, on the stored work
const forgetAnswer = (page: Page) =>
  page.evaluate(
    () =>
      new Promise<void>((resolve, reject) => {
        const rq = indexedDB.open('llp-solver');
        rq.onerror = () => reject(new Error('no llp-solver database'));
        rq.onsuccess = () => {
          const db = rq.result;
          const t = db.transaction('answer', 'readwrite');
          t.objectStore('answer').clear();
          t.oncomplete = () => {
            db.close();
            resolve();
          };
          t.onerror = () => reject(new Error('could not clear the kept answer'));
        };
      }),
  );
const trainee = (page: Page) => page.locator('#tree [data-node="trainee"] [data-nm]');

test('the kept work is stored after a plan and restored first by the next worker; a reload with nothing changed draws the answer at once @both', async ({
  page,
}) => {
  await seededPlan(page);
  // stored promptly once the worker is idle (engine.ts engineSaveState: 100 ms, behind the plan)
  await expect
    .poll(() => page.evaluate(() => window.__runs.filter((j) => j.cmd === 'state_out').length), { timeout: 1300 })
    .toBeGreaterThan(0);
  // (the store waits in the worker behind the plan's check, a background job)
  await expect
    .poll(async () => (await storedStates(page)).filter((length) => length > 0).length, {
      timeout: SOLVER_WAIT,
    })
    .toBeGreaterThan(0);
  const shown = await trainee(page).textContent();
  await forgetAnswer(page);
  await quiet(page);
  await page.reload();
  await careerDone(page);
  await expect(page.locator('#run-note')).toHaveText(/Planned in/, { timeout: SOLVER_WAIT });
  const restored = await page.evaluate(() => window.__runs.map((j) => j.cmd));
  expect(restored[0], "the new worker's first job restores the stored work").toBe('state_in');
  expect(restored).toContain('career');
  await expect(trainee(page)).toHaveText(shown ?? '');
  // nothing changed: the last answer at once, no search or plan
  await quiet(page);
  await page.reload();
  await careerDone(page);
  await expect(page.locator('#run-note')).toHaveText(/Planned in/);
  await expect(trainee(page)).toHaveText(shown ?? '');
  // (an unfinished check of the answer resumes in the background: `next.robust`, no search or plan)
  expect(
    await page.evaluate(() => window.__runs.filter((j) => j.cmd === 'career' && !j.career.next?.robust).length),
  ).toBe(0);
});

// every visible pill track (the segmented controls, the veteran card's tabs): its segments fill it flush, the full
// inner height, no gap at its ends or between them, the track a pill that clips them, none drawn with the game's pill
// sprite. CSS px, 0.6 px tolerance. (runs in the page)
const PILLS = (): string[] => {
  const bad: string[] = [];
  const near = (a: number, b: number) => Math.abs(a - b) <= 0.6;
  for (const t of document.querySelectorAll('[data-seg]:not([data-seg="apart"])')) {
    const r = t.getBoundingClientRect(),
      cs = getComputedStyle(t);
    if (!r.width || !r.height || cs.visibility === 'hidden') continue;
    const id = t.id ? `#${t.id}` : (t.getAttribute('aria-label') ?? t.className);
    const inner = {
      l: r.left + parseFloat(cs.borderLeftWidth),
      r: r.right - parseFloat(cs.borderRightWidth),
      t: r.top + parseFloat(cs.borderTopWidth),
      b: r.bottom - parseFloat(cs.borderBottomWidth),
    };
    if (cs.overflowX !== 'hidden' && cs.overflowX !== 'clip') bad.push(`${id}: does not clip its segments`);
    if (parseFloat(cs.borderTopLeftRadius) < r.height / 2 - 0.5) bad.push(`${id}: not a pill`);
    const segs = [...t.children].filter((c) => c.getBoundingClientRect().width > 0);
    segs.forEach((s, i) => {
      const q = s.getBoundingClientRect();
      if (!near(q.top, inner.t) || !near(q.bottom, inner.b)) bad.push(`${id} #${i}: short of top or bottom`);
      if (i === 0 && !near(q.left, inner.l)) bad.push(`${id}: a gap at its start`);
      if (i === segs.length - 1 && !near(q.right, inner.r)) bad.push(`${id}: a gap at its end`);
      const before = segs[i - 1];
      if (before && !near(q.left, before.getBoundingClientRect().right)) bad.push(`${id}: a gap before #${i}`);
      if (getComputedStyle(s).borderImageSource !== 'none') bad.push(`${id} #${i}: drawn with a border-image`);
    });
  }
  return bad;
};

test.describe('on a 375 px phone @both', () => {
  test.use({
    viewport: { width: 375, height: 812 },
    // (touch and a mobile viewport: Chromium's emulation; Firefox has none)
    isMobile: async ({ browserName }, use) => use(browserName === 'chromium'),
    hasTouch: async ({ browserName }, use) => use(browserName === 'chromium'),
  });

  test('nothing scrolls sideways; the setup, the pills, Try a run, the pick and the tree fit the phone', async ({
    page,
  }) => {
    await seededPlan(page);
    // the first veterans fold the setup: the tree starts on the first screen
    await expect(page.locator('#set-fold')).toHaveAttribute('aria-expanded', 'false');
    await expect(page.locator('#set-sum')).toBeVisible();
    const treeTop = await page.evaluate(
      () => (document.getElementById('tree')?.getBoundingClientRect().top ?? 1e9) + scrollY,
    );
    expect(treeTop).toBeLessThan(812);
    await page.click('#set-fold');
    const outside = await page.evaluate(() => {
      const panel = document.querySelector('[aria-labelledby="h-set"]')?.getBoundingClientRect();
      if (!panel) return ['no setup panel'];
      return [
        ...document.querySelectorAll(
          '#set-body > :first-child > *, #set-body > :first-child [data-seg], #x-scen, #t-add',
        ),
      ]
        .filter((e) => {
          const r = e.getBoundingClientRect();
          return r.width > 0 && (r.left < panel.left || r.right > panel.right);
        })
        .map((e) => e.id || e.className);
    });
    expect(outside).toEqual([]);
    expect(await page.evaluate(SIDEWAYS)).toEqual([]);
    expect((await page.locator('#t-add').boundingBox())?.width).toBeGreaterThan(200);
    // Try a run's popup: her card's tabs flush, the popup on the screen, its body scrolling
    await page.click('#sim-try');
    const tabs = page.locator('[role="tablist"][aria-label="Her details"]');
    await expect(tabs).toHaveCount(1);
    await tabs.locator('[role="tab"]').nth(1).click();
    expect(await page.evaluate(PILLS)).toEqual([]);
    const sim = await page.evaluate(() => {
      const d = document.getElementById('sim-dlg')?.getBoundingClientRect(),
        body = document.getElementById('sim-top');
      return !!d && !!body && d.left >= 0 && d.right <= 375 && d.bottom <= 812 && body.scrollHeight > body.clientHeight;
    });
    expect(sim, "Try a run's popup fits the screen and scrolls").toBe(true);
    await page.locator('#sim-dlg').getByRole('button', { name: 'Close', exact: true }).click();
    // the pick in Other careers: one card, its career the card's full width
    const pick = await page.evaluate(() => {
      const tr = document.querySelector('[data-sec="options"] tbody tr[aria-current="true"]');
      const th = tr?.querySelector('th');
      if (!(tr instanceof HTMLElement) || !th) return null;
      const cs = getComputedStyle(tr);
      return {
        row: cs.backgroundColor,
        cell: getComputedStyle(th).backgroundColor,
        wide:
          th.getBoundingClientRect().width >=
          tr.clientWidth - parseFloat(cs.paddingLeft) - parseFloat(cs.paddingRight) - 4,
      };
    });
    expect(pick?.row).not.toBe('rgba(0, 0, 0, 0)');
    expect(pick?.cell).toBe('rgba(0, 0, 0, 0)');
    expect(pick?.wide).toBe(true);
    // the dialogs: your veterans (their tabs flush), Umas, Support cards, the node popup
    const opens: [string, () => Promise<void>][] = [
      ['veterans', () => page.click('#v-toggle')],
      ['umas', () => page.click('#o-umas')],
      ['cards', () => page.click('#o-cards')],
      ['node', () => page.locator('#tree [data-k]').first().click()],
    ];
    for (const [label, open] of opens) {
      await open();
      await page.waitForSelector('dialog[open]');
      await page.evaluate(() => Promise.all(document.getAnimations().map((a) => a.finished)));
      expect(await page.evaluate(SIDEWAYS), label).toEqual([]);
      if (label === 'veterans') expect(await page.evaluate(PILLS), label).toEqual([]);
      await page.evaluate(() => {
        for (const d of document.querySelectorAll('dialog[open]')) if (d instanceof HTMLDialogElement) d.close();
      });
    }
    // the next career in one column, top to bottom; each legacy box one compact row
    const tree = await page.evaluate(() => {
      const t = document.getElementById('tree');
      const kids = [...(t?.children ?? [])].map((k) => k.getBoundingClientRect());
      return {
        column: kids.every((r, i) => r.width >= 330 && (i === 0 || r.top >= (kids[i - 1]?.bottom ?? 0) - 1)),
        boxes: [...(t?.querySelectorAll('[data-legacy]') ?? [])].map((e) =>
          Math.round(e.getBoundingClientRect().height),
        ),
      };
    });
    expect(tree.column).toBe(true);
    expect(tree.boxes).toHaveLength(2);
    expect(tree.boxes.every((h) => h < 130)).toBe(true);
  });
});
