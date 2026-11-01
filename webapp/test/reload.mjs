import { expect } from 'playwright/test';
import { check, open, openEditor, closeEditor, settle, odds } from './helpers.mjs';

export default async function reload(b) {
  // a reload with non-default settings shows their last result from the first frame (not the default example), and
  // a fresh identical result doesn't redraw it
  const pg = await open(b),
    ctx = pg.context(),
    def = await odds(pg);
  await pg.click('#rank button[data-v="ss_and_up"]');
  await settle(pg); // rank SS: a different result, quick to search
  const mine = await odds(pg);
  await ctx.addInitScript(() => {
    window.__seen = [];
    const snap = () => {
      const o = document.querySelector('#results [data-odds]');
      window.__seen.push(o ? o.textContent : '');
      if (performance.now() < 6000) requestAnimationFrame(snap);
    };
    requestAnimationFrame(snap);
  });
  await pg.reload();
  await settle(pg);
  // (the bundle is a cached script that may run after the first paint: a blank results area before it is allowed)
  const seen = [...new Set(await pg.evaluate(() => window.__seen))].filter((x) => x !== '');
  check(
    "reload: the first result drawn is the saved settings' one, never the default example",
    seen.length === 1 && seen[0] === mine && mine !== '' && mine !== def,
    seen.join(' | ') + ' · default ' + def,
  );
  // the saved result of these settings counts as done: no search runs, the status is its own "Checked …" line
  await pg.waitForTimeout(1500); // long enough for a search to have started
  const kept = 'reload: the saved result is done, no search runs';
  check(kept, await pg.evaluate(() => !window.__runs.some((j) => j.cmd === 'deck')));
  await expect
    .soft(pg.locator('#run-note'), `${kept}: the status`)
    .toHaveText(/^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/, { useInnerText: true });
  // the same search run again (auto-search off, an edit and its undoing, then Search: a button search always runs)
  // answers the result on show, which stays as drawn
  await pg.click('#x-auto');
  await pg.evaluate(() => {
    window.__drawn = document.querySelector('#results [data-odds]');
  });
  const runs = await pg.evaluate(() => window.__runs.length);
  await openEditor(pg, 'cards');
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  await closeEditor(pg, 'cards');
  await pg.locator('#r-act button', { hasText: 'Search' }).click();
  await settle(pg);
  check(
    'reload: … a fresh identical result is not redrawn',
    (await pg.evaluate(() => window.__runs.length)) === runs + 1 &&
      (await pg.evaluate(() => document.querySelector('#results [data-odds]') === window.__drawn)),
  );
  await pg.click('#x-auto');
  const runsBefore = await pg.evaluate(() => window.__runs.length);
  await pg.click('#rank button[data-v="below_ss"]');
  await settle(pg);
  check('reload: … and the next edit searches', (await pg.evaluate(() => window.__runs.length)) > runsBefore);
  await expect.soft(pg.locator('#results [data-odds]').first(), "reload: … the default's odds again").toHaveText(def);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
