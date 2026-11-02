// The career page's veterans in a real browser: the game's export through the file input, a veteran edited beside the
// list (stars cycle in place with the focus kept, the × on hover, "+" offering the missing target), places chosen and
// handed back, "Use this" and the planner's pick again, a setup change, and a reload; then uma.moe, its requests
// routed to two listings made from the test file: the key round trip, the Guests tab, Copy ID, the picker's heading and
// a chosen listing's tile, the popup's cards and order, its fit on a phone, a reload with no request, Remove all.
import type { Page } from 'playwright/test';
import type { Request } from '../../src/gen/solver.ts';
import { expect, test } from './fixtures.ts';
import { DUMP_PATH, RECORDS, listing } from './listings.ts';
import { quiet } from './solver.ts';
import { SOLVER_WAIT, careerDone, closeEditor, openEditor } from './pages.ts';

// the setup panel open (unchosen it opens folded once there are veterans)
test.use({ seed: { 'llp:setup-folded': '0' } });
// (each awaits the solver several times: recorded live, a run takes a minute or two)
test.describe.configure({ timeout: 4 * SOLVER_WAIT });

const careerJobs = (page: Page) =>
  page.evaluate(() => window.__runs.filter((j: Request) => j.cmd === 'career' && !j.career.next?.robust).length);
const stored = (page: Page) => page.evaluate(() => JSON.parse(localStorage.getItem('llp:career') ?? '{}').global);
async function importDump(page: Page) {
  await page.locator('#v-file').setInputFiles(DUMP_PATH);
  await expect(page.locator('#v-msg')).not.toBeEmpty();
  await careerDone(page);
}
async function openNode(page: Page, key: string) {
  await closeEditor(page, 'vets');
  await page.click(`#tree [data-k="${key}"]`);
  await expect(page.locator('#node-dlg')).toHaveAttribute('open');
}
const inNode = (page: Page, name: string | RegExp) => page.locator('#node-dlg').getByRole('button', { name });

test('veterans: edited beside the list, placed, used and handed back, kept over a reload', async ({ page }) => {
  await page.goto('/');
  await careerDone(page);
  await importDump(page);
  const cards = page.locator('#vets [data-vet]');
  const ghosts = page.locator('#tree [data-ghost]');
  await expect(page.locator('#odds-open')).toHaveCount(1);
  await openEditor(page, 'vets');
  // (every card marked: one built again loses the mark)
  await page.evaluate(() => {
    for (const c of document.querySelectorAll('#vets [data-vet]')) c.setAttribute('data-test-kept', '');
  });
  await page.click('#vet-1');
  await page.click('#vet-0'); // (a wide dialog: she shows beside the list)
  await expect(page.locator('#v-pane [data-row="parent"]')).toHaveCount(2);
  const chip = page.locator('#v-pane [data-row="me"] [data-spark="white"] [data-bar]').first();
  const key = await chip.getAttribute('data-k');
  const bar = page.locator(`#v-pane [data-k="${key}"]`);
  const stars = async () => Number(/(\d) star/.exec((await bar.getAttribute('aria-label')) ?? '')?.[1]);
  const s0 = await stars();
  await bar.click();
  // the result stays on screen while the search runs again
  await expect(page.locator('#odds-open')).toHaveCount(1);
  await expect(ghosts).toHaveCount(3);
  await careerDone(page);
  await expect(cards).toHaveCount(12);
  expect(await page.locator('#vets [data-vet]:not([data-test-kept]):not(#vet-0)').count()).toBe(0);
  // (each edit's run ended before the next: a run cut short by an edit ends its worker, which the replay must match)
  const s1 = await stars();
  await bar.click();
  const s2 = await stars();
  await careerDone(page);
  await bar.click();
  const s3 = await stars();
  await careerDone(page);
  expect([s1, s2, s3]).toEqual([(s0 % 3) + 1, (s1 % 3) + 1, (s2 % 3) + 1]);
  await expect(bar).toBeFocused();
  expect(JSON.stringify(await stored(page))).toMatch(/"sparks"/);
  // the × shows on hover and removes the skill; "+" offers the missing target only, back at 2 stars
  const skills = page.locator('#v-pane [data-row="me"] [data-spark="white"]');
  const n0 = await skills.count();
  const first = (await chip.getAttribute('aria-label')) ?? '';
  await skills.first().hover();
  const x = skills.first().getByRole('button', { name: /^Remove / });
  await expect(x).toBeVisible();
  expect(await x.evaluate((e) => getComputedStyle(e).visibility)).toBe('visible');
  await x.click();
  await expect(skills).toHaveCount(n0 - 1);
  await careerDone(page);
  await page.locator('#v-pane [data-row="me"] [data-k$=":add"]').click();
  const offered = await page.locator('#add-pop button').allTextContents();
  expect(offered).toHaveLength(1);
  const [missing] = offered;
  if (missing === undefined) throw new Error('"+" offered nothing');
  expect(first.startsWith(missing)).toBe(true);
  expect(
    await page
      .locator('#v-pane [data-row="me"] [data-spark="blue"], #v-pane [data-row="me"] [data-spark="pink"]')
      .count(),
  ).toBeGreaterThanOrEqual(2);
  await page.locator('#add-pop button').first().click();
  await expect(skills).toHaveCount(n0);
  await expect(page.locator('#v-pane [data-row="me"] [data-bar]', { hasText: missing })).toHaveAttribute(
    'aria-label',
    /2 stars/,
  );
  await careerDone(page);

  // parent 2's picker greys out who can't be her, saying why
  await openNode(page, 'node:p1');
  await inNode(page, /^(Choose|Change) parent$/).click();
  await expect(page.locator('#vpick-dlg')).toHaveAttribute('open');
  await expect(page.locator('#vp-list button')).not.toHaveCount(0);
  const blocked = await page.locator('#vp-list button:disabled').count();
  expect(await page.locator('#vp-list button:disabled [data-why]').count()).toBe(blocked);
  await page.locator('#vp-list button:not(:disabled)').first().click();
  await careerDone(page);
  // (the closed picker and node popup hold no rows or node: an edit elsewhere redraws none of them)
  await expect(page.locator('#vp-list > *')).toHaveCount(0);
  await expect(page.locator('#node-body > *')).toHaveCount(0);
  // a chosen parent's popup shows its footer inside it, desktop and phone
  await openNode(page, 'node:p1');
  const footIn = () =>
    page.evaluate(() => {
      const dlg = document.getElementById('node-dlg');
      const d = dlg?.getBoundingClientRect();
      const buttons = [...(dlg?.lastElementChild?.querySelectorAll('button') ?? [])];
      return (
        !!d &&
        buttons.length > 0 &&
        buttons.every((b) => {
          const r = b.getBoundingClientRect();
          return r.top >= d.top && r.bottom <= d.bottom + 0.5 && r.bottom <= innerHeight;
        })
      );
    });
  expect(await footIn()).toBe(true);
  await page.setViewportSize({ width: 375, height: 740 });
  expect(await footIn()).toBe(true);
  await page.setViewportSize({ width: 1440, height: 900 });
  await inNode(page, 'Let the planner pick').click();
  await page.keyboard.press('Escape');
  await careerDone(page);
  await expect(ghosts).toHaveCount(3);
  await expect(page.locator('#run-note')).toHaveText(/Planned in|Weighed/);

  // the trainee chosen: the picks list parents only; "Use this" sets every place and the career is just scored, the
  // plan's table staying with the pick and hers marked
  await openNode(page, 'node:t');
  await inNode(page, /^(Choose|Change) trainee$/).click();
  await page.fill('#pick-filter', 'Agnes Digital');
  await page.locator('#pick-list button:visible').first().click();
  await careerDone(page);
  await expect(page.locator('#tree [data-node="trainee"]')).not.toHaveAttribute('data-ghost');
  const who = await page.locator('[data-sec="options"] tbody [data-who]').allTextContents();
  expect(who.length).toBeGreaterThan(0);
  expect(who.every((w) => /^Parents /.test(w.trim()))).toBe(true);
  const useThis = page.locator('[data-sec="options"] tbody').getByRole('button', { name: 'Use this' }).first();
  await useThis.click();
  await careerDone(page);
  await expect(page.locator('#run-note')).toHaveText(/Scored in/);
  await expect(ghosts).toHaveCount(0);
  await expect(page.locator('[data-sec="options"]')).toHaveCount(1);
  await expect(page.locator('[data-sec="options"] tbody tr[aria-current="true"]')).toHaveText(/shown above/);
  await expect(page.locator('[data-sec="options"] [data-tag="pick"]')).toHaveCount(1);
  const saves = await page.locator('[data-sec="options"] tbody [data-extra]').allTextContents();
  expect(saves.some((s) => /^[+−]\d/.test(s.trim()))).toBe(true);
  // back to the plan's places: an answer seen before is drawn with no run
  const jobs0 = await careerJobs(page);
  await page.click('#opt-back');
  await careerDone(page);
  await expect(page.locator('#run-note')).toHaveText(/Planned in/);
  await expect(ghosts).toHaveCount(2);
  expect(await careerJobs(page)).toBe(jobs0);
  // a setting changed: the Umas dialog searches again
  await openEditor(page, 'umas');
  await page.locator('#set-umas #roster [data-uma] button').nth(3).click();
  await closeEditor(page, 'umas');
  await careerDone(page);
  expect(await careerJobs(page)).toBeGreaterThan(jobs0);
  // the veterans and the places outlive a reload
  await quiet(page); // (the worker's kept work stored first, as a visitor's reload a moment later finds it)
  await page.reload();
  await careerDone(page);
  await openEditor(page, 'vets');
  await expect(cards).toHaveCount(12);
  await expect(page.locator('#tree [data-node="trainee"]')).toHaveText(/Agnes Digital/);
  await closeEditor(page, 'vets');
  // a career taken with "Use this" goes back to the planner when a setting changes
  await useThis.click();
  await careerDone(page);
  await page.click('#bp button[data-v="never"]');
  await careerDone(page);
  await expect(ghosts).toHaveCount(3);
  expect((await stored(page)).plan.parents).toEqual([null, null]);
  // Remove from her pane: her card had the focus and is gone, so the card chosen now takes it (review-8c §6)
  await openEditor(page, 'vets');
  await cards.last().click();
  await page.locator('#v-pane').getByRole('button', { name: 'Remove', exact: true }).click();
  await expect(page.locator('#vets [data-vet][data-sel]')).toBeFocused();
  await expect(cards).toHaveCount(11);
  await careerDone(page);
  // an edit, and the page going before its save is due: the page saves it as it goes (pagehide)
  const quick = await page.evaluate(async () => {
    const list: { id: string }[] = JSON.parse(localStorage.getItem('llp:career') ?? '{}').global.vets;
    const ids = () =>
      JSON.parse(localStorage.getItem('llp:career') ?? '{}').global.vets.map((v: { id: string }) => v.id);
    const card = [...document.querySelectorAll<HTMLElement>('#vets [data-vet]')].at(-1);
    const id = list[Number(card?.id.slice(4))]?.id;
    card?.click();
    await new Promise((r) => setTimeout(r)); // (the pane draws her after the click's task)
    [...document.querySelectorAll<HTMLElement>('#v-pane button')].find((b) => b.textContent === 'Remove')?.click();
    const due = ids().includes(id);
    dispatchEvent(new PageTransitionEvent('pagehide'));
    return { id, due, flushed: !ids().includes(id) };
  });
  expect(quick).toMatchObject({ due: true, flushed: true });
  await careerDone(page);
  await quiet(page);
  await page.reload();
  await careerDone(page);
  expect((await stored(page)).vets.some((v: { id: string }) => v.id === quick.id)).toBe(false);
});

test('uma.moe: key, listings in the pool, the popup, a reload, Remove all on Guests', async ({
  page,
  context,
  pageErrors,
}) => {
  const [first, second, lender] = RECORDS;
  if (!first || !second || !lender) throw new Error('the test file has fewer than three veterans');
  const items = [listing(first, 1), listing(second, 2)];
  const requests: { url: URL; key: string | undefined; cookie: string | undefined }[] = [];
  await context.route('https://uma.moe/api/v3/search**', async (route) => {
    const req = route.request();
    const cors = {
      'access-control-allow-origin': '*',
      'access-control-allow-headers': 'x-api-key',
      'access-control-allow-methods': 'GET',
    };
    if (req.method() === 'OPTIONS') return route.fulfill({ status: 204, headers: cors });
    const key = req.headers()['x-api-key'];
    requests.push({ url: new URL(req.url()), key, cookie: req.headers().cookie });
    if (key !== 'test-key') return route.fulfill({ status: 401, headers: cors, body: '{}' });
    return route.fulfill({
      status: 200,
      headers: { ...cors, 'content-type': 'application/json' },
      body: JSON.stringify({ items, total: '2', page: 0, limit: 100, total_pages: 1 }),
    });
  });
  await page.goto('/');
  await careerDone(page);
  await expect(page.locator('#um-open')).toBeHidden(); // (no veteran of yours yet)
  await importDump(page);
  const label = page.locator('#um-open');
  await expect(label).toHaveText('Borrow from uma.moe');
  await expect(page.locator('#set-body #um-row')).toHaveCount(0);
  expect(requests).toHaveLength(0);
  await label.click();
  await page.fill('#um-key', 'bad-key');
  await page.click('#um-save');
  await expect(page.locator('#um-err')).toHaveText(/Key rejected/);
  await expect(label).toHaveText('uma.moe · key rejected');
  await expect(page.locator('#um-key')).toBeVisible();
  // (the browser logs the refused request: expected)
  pageErrors.splice(0, pageErrors.length, ...pageErrors.filter((e) => !/status of 401/.test(e)));
  await page.fill('#um-key', 'test-key');
  await page.press('#um-key', 'Enter');
  await expect(label).toHaveText('uma.moe · 2 veterans');
  expect(requests[1]?.key).toBe('test-key');
  expect(requests[1]?.cookie).toBeUndefined();
  await page.keyboard.press('Escape');
  await expect(page.locator('#um-dlg')).not.toHaveAttribute('open');
  await careerDone(page);
  // the Guests tab holds them under an uma.moe heading; Copy ID belongs to the listing on show
  await openEditor(page, 'vets');
  await page.click('#v-tab-bor');
  await expect(page.locator('#vets [data-vet]')).toHaveCount(2);
  await expect(page.locator('#vets h4')).toHaveText('uma.moe');
  await expect(page.locator('#v-tab-bor')).toHaveText('Guests · 2');
  await page.locator('#vets [data-vet]').nth(0).click();
  await page.locator('#v-pane button', { hasText: 'Copy ID' }).click();
  const copy = page.locator('#v-pane [data-who]').getByRole('button', { name: /^(Copy ID|Copied|\d+)$/ });
  await expect(copy).not.toHaveText('Copy ID');
  await page.locator('#vets [data-vet]').nth(1).click();
  await expect(copy).toHaveText('Copy ID');
  // Remove all on Guests takes the borrow list, not uma.moe's listings
  await page.click('#v-paste');
  await page.fill(
    '#paste-text',
    JSON.stringify([{ ...lender, viewer_id: 900000000003, owner_viewer_id: 900000000003 }]),
  );
  await page.click('#paste-load');
  await page.click('#v-tab-bor');
  await careerDone(page);
  const listed = await page.locator('#vets [data-vet]').count();
  let asked2 = '';
  page.once('dialog', (d) => {
    asked2 = d.message();
    void d.accept();
  });
  await page.click('#v-clear');
  expect(asked2).toBe('Remove all 1 borrowed veteran?');
  await expect(page.locator('#vets [data-vet]')).toHaveCount(listed - 1);
  await page.click('#v-tab-own');
  await careerDone(page);
  // one as parent 1, from the picker's uma.moe heading: her tile names her trainer, Copy ID in her popup
  await openNode(page, 'node:p0');
  await inNode(page, /^(Choose|Change) parent$/).click();
  await expect(page.locator('#vp-list h4')).toHaveText(['uma.moe']);
  await page.locator('#vp-list button').last().click();
  await careerDone(page);
  const tile = page.locator('#tree [data-node="parent"]').first();
  await expect(tile).toHaveText(/uma\.moe · Trainer 2/);
  await expect(tile.locator('button:not([data-k])')).toHaveCount(0);
  await openNode(page, 'node:p0');
  await expect(page.locator('#node-dlg button', { hasText: 'Copy ID' })).toHaveCount(1);
  await page.keyboard.press('Escape');
  // the popup: the game's legacy cards (its tag and star images), uma.moe's order or by wins; a card opens her read-only
  await label.click();
  const list = page.locator('#um-list [data-vet]');
  await expect(list).toHaveCount(2);
  const art = await list.evaluateAll((cs) =>
    cs.every((c) => {
      const tags = [...c.querySelectorAll('img[data-part="tag"]')];
      const starImgs = [...c.querySelectorAll('img[data-part="stars"]')];
      const rank = c.querySelector('img[data-part="rank"]');
      return (
        tags.length === 3 &&
        tags[2]?.getAttribute('alt') === 'UNIQ' &&
        tags.every((t) => /ftag_(blue_\d|pink_\d+|green)\./.test(t.getAttribute('src') ?? '')) &&
        starImgs.every((s) => /fstar_(blue|pink|green)_[123]\./.test(s.getAttribute('src') ?? '')) &&
        /vrank_\d+\./.test(rank?.getAttribute('src') ?? '')
      );
    }),
  );
  expect(art).toBe(true);
  const names = page.locator('#um-list [data-vet] [data-part="name"]');
  await expect(names).toHaveText(['Trainer 1', 'Trainer 2']);
  await page.click('#um-sort');
  await page.click('#um-sort-menu button[data-k="wins"]');
  await expect(names).toHaveText(['Trainer 2', 'Trainer 1']);
  await page.click('#um-sort-menu button[data-k="match"]');
  await page.keyboard.press('Escape');
  await list.first().click();
  await expect(page.locator('#node-dlg')).toHaveAttribute('open');
  await expect(page.locator('#node-dlg [data-src="umamoe"]')).toHaveText(/· 13 followers$/);
  const off = await page
    .locator('#node-dlg [data-row="me"] [data-spark="white"]')
    .evaluateAll((l) => l.map((c) => c.hasAttribute('data-off')));
  expect(off.length).toBeGreaterThan(1);
  expect(off.at(-1)).toBe(true);
  expect(off.filter(Boolean)).toHaveLength(1);
  await expect(page.locator('#node-dlg button', { hasText: 'Copy ID' })).toHaveCount(1);
  await expect(
    page.locator(
      '#node-dlg [data-face], #node-dlg button[data-k$=":wins"], #node-dlg [data-k$=":add"], #node-dlg button[data-bar]',
    ),
  ).toHaveCount(0);
  await page.keyboard.press('Escape');
  // her popup closed, back to it: Include max borrows asks again without the follower limit
  await expect(page.locator('#um-dlg')).toHaveAttribute('open');
  await page.check('#um-full');
  await expect(page.locator('#um-msg')).toHaveText(/veterans found/);
  await expect(page.locator('#um-again')).toBeEnabled();
  expect(requests.at(-1)?.url.searchParams.has('max_follower_num')).toBe(false);
  await page.uncheck('#um-full');
  // at 375 px the popup stays on screen, its long body scrolling inside it
  await page.setViewportSize({ width: 375, height: 812 });
  const fit = await page.evaluate(() => {
    const listEl = document.getElementById('um-list');
    const firstCard = listEl?.firstElementChild;
    if (!listEl || !firstCard) return null;
    for (let i = 0; i < 60; i++) listEl.appendChild(firstCard.cloneNode(true));
    const d = document.getElementById('um-dlg')?.getBoundingClientRect();
    const body = document.getElementById('um-body');
    if (!d || !body) return null;
    return {
      onScreen: d.left >= 0 && d.right <= 375 && d.bottom <= 812,
      scrolls: body.scrollHeight > body.clientHeight && getComputedStyle(body).overflowY === 'auto',
      sideways: document.documentElement.scrollWidth > 375,
    };
  });
  expect(fit).toEqual({ onScreen: true, scrolls: true, sideways: false });
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.keyboard.press('Escape');
  await careerDone(page);
  // a reload: the listings with no request and no solver job (the plan's answer kept), the chosen one saved once
  await expect(page.locator('#run-note')).toHaveText(/Planned in/, { timeout: SOLVER_WAIT });
  const asked = requests.length;
  await quiet(page); // (the worker's kept work stored first, as a visitor's reload a moment later finds it)
  await page.reload();
  await careerDone(page);
  await expect(label).toHaveText('uma.moe · 2 veterans');
  expect(requests).toHaveLength(asked);
  expect(await careerJobs(page)).toBe(0);
  expect((await stored(page)).vets).toHaveLength(13);
});
