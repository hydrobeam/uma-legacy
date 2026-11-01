import { check, open, settle, note } from './helpers.mjs';

export default async function autosearch(b) {
  const pg = await open(b);
  // auto-search off: a change waits for the Search button; back on, it searches
  check('auto-search is on by default', await pg.locator('#x-auto').isChecked());
  await pg.click('#x-auto');
  const n0 = await pg.evaluate(() => window.__runs.length);
  check(
    'switching auto-search off leaves the results as they are (not out of date)',
    /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)) &&
      !(await pg.evaluate(() => document.getElementById('results').classList.contains('stale'))),
  );
  // P5: a card un-owned in the picker greys its card in the result on show at once, the same element (auto-search
  // off: that result stays); owned again, it is not greyed
  const tile = await pg.evaluate(() => {
    const chip = document.querySelector('#results .deck .dc:not(.bor)'),
      name = chip.dataset.name;
    window.__chip = chip;
    return [...document.querySelectorAll('#cards .ct')].findIndex((t) => {
      const shown = t.querySelector('.cn').textContent;
      return name === shown || name.startsWith(`${shown} `);
    });
  });
  await pg.locator('#cards .ct').nth(tile).click();
  const greyed = await pg.evaluate(() => window.__chip.isConnected && window.__chip.classList.contains('off'));
  await pg.locator('#cards .ct').nth(tile).click();
  check(
    'P5: un-owning a card in the picker greys it in the result on screen at once (the same element), owning it ungreys it',
    tile >= 0 &&
      greyed &&
      (await pg.evaluate(() => window.__chip.isConnected && !window.__chip.classList.contains('off'))),
    `tile ${tile}, greyed ${greyed}`,
  );
  await pg.locator('#cards .ct').nth(6).click();
  await pg.waitForTimeout(1300);
  check(
    'auto-search off: a change does not search, the status says to press Search',
    (await pg.evaluate(() => window.__runs.length)) === n0 &&
      /press Search/.test(await note(pg)) &&
      (await pg.locator('#r-act button', { hasText: 'Search' }).count()) === 1,
  );
  // Export pairs these settings with the result on show, which is from before the change: it says so
  await pg.click('#ex-open');
  const exported = JSON.parse(await pg.inputValue('#ex-text')).results;
  await pg.click('#ex-close');
  check(
    '… and Export marks the result on show as from other settings',
    exported?.stale === true && exported.decks.length > 0,
    JSON.stringify({ stale: exported?.stale }),
  );
  await pg.locator('#r-act button', { hasText: 'Search' }).click();
  await settle(pg);
  check(
    '… the Search button runs it',
    (await pg.evaluate(() => window.__runs.length)) === n0 + 1 &&
      /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)),
  );
  await pg.locator('#cards .ct').nth(6).click();
  await pg.click('#x-auto');
  await settle(pg);
  check('… turning it back on searches the change', (await pg.evaluate(() => window.__runs.length)) === n0 + 2);
  const made0 = await pg.evaluate(() => window.__made);
  await pg.evaluate(() =>
    [...document.querySelectorAll('#cards .ct')].slice(0, 5).forEach((b) => {
      b.click();
    }),
  ); // five changes inside one pause
  check('a change shows "Searching…"', /Searching/.test(await note(pg)));
  // the fast pass is drawn first, with the status saying the exact check is still running
  const checking = await pg
    .waitForFunction(() => /Checking the exact odds/.test(document.getElementById('run-note').textContent), null, {
      timeout: 30000,
    })
    .then(
      () => true,
      () => false,
    );
  check(
    'the first pass shows while the exact check runs',
    checking && (await pg.locator('#results .loop').count()) > 0,
    await note(pg),
  );
  await settle(pg);
  check('a burst of 5 changes is one search on the same worker', (await pg.evaluate(() => window.__made)) === made0);
  // a search changed mid-run (3 results, one shared deck; its replies held so it is still running however fast it
  // is): the worker is killed and the new search finishes
  const r0 = await pg.evaluate(() => {
    window.__hold = true;
    return window.__runs.length;
  });
  await pg.click('#c-top [data-v="3"]');
  await pg.click('#c-peruma');
  await pg.waitForFunction(
    (n) => window.__runs.length > n && window.__runs.at(-1).top_decks === 3 && window.__runs.at(-1).per_uma === false,
    r0,
  ); // the slow search itself is running (not a quick job from the first change alone)
  const k0 = await pg.evaluate(() => window.__killed);
  check('old results stay visible while searching', (await pg.locator('#results .loop').count()) > 0);
  await pg.waitForFunction(
    () => /(Searching|Checking the exact odds)…\s*\d+ s/.test(document.getElementById('run-note').textContent),
    null,
    { timeout: 30000 },
  ); // its clock is running
  await pg.click('#c-peruma');
  const paused = await note(pg);
  await pg.evaluate(() => window.__release());
  check(
    'a change mid-search clears the clock: it counts again only once the new search runs',
    /Searching/.test(paused) && !/\d+ s/.test(paused),
    paused,
  );
  await pg.click('#c-top [data-v="1"]');
  await settle(pg);
  check('a change mid-search restarts the worker', (await pg.evaluate(() => window.__killed)) > k0);
  check('the restarted search finishes', /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
