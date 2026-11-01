import { expect } from 'playwright/test';
import { check, open, openEditor, closeEditor, settle, note } from './helpers.mjs';

// The status line's live region (#run-note, role status): the node the parser made, its text then (empty: the region
// exists before its first message, so that message is a change a screen reader announces) and every text it shows after
const watchLiveRegion = () => {
  window.__live = null;
  new MutationObserver((_, seen) => {
    const note = document.getElementById('run-note');
    if (!note) return;
    seen.disconnect();
    const live = {
      node: note,
      region: `${note.getAttribute('role')} ${note.getAttribute('aria-live')}`,
      texts: [note.textContent],
    };
    window.__live = live;
    new MutationObserver(() => {
      if (live.texts.at(-1) !== note.textContent) live.texts.push(note.textContent);
    }).observe(note, { childList: true, characterData: true, subtree: true });
  }).observe(document, { childList: true, subtree: true });
};
const liveRegion = (pg) =>
  pg.evaluate(() => ({
    region: window.__live?.region,
    same: window.__live?.node === document.getElementById('run-note'),
    texts: window.__live?.texts.map((t) => t.slice(0, 40)),
  }));

export default async function autosearch(b) {
  const pg = await open(b, { init: watchLiveRegion });
  const runNote = pg.locator('#run-note'),
    loops = pg.locator('#results .loop');
  // the status line's text as drawn
  const noteIs = (text, what, timeout) => expect.soft(runNote, what).toHaveText(text, { useInnerText: true, timeout });
  const done = /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/;
  const first = await liveRegion(pg);
  check(
    'the status line is a live region from load: empty before the first search, whose "Searching…" and result are changes to it',
    first.region === 'status polite' &&
      first.same &&
      first.texts[0] === '' &&
      first.texts.some((t) => /^Searching…/.test(t)) &&
      /^Checked [\d,]+ decks/.test(first.texts.at(-1)),
    JSON.stringify(first),
  );
  // auto-search off: a change waits for the Search button; back on, it searches
  await expect.soft(pg.locator('#x-auto'), 'auto-search is on by default').toBeChecked();
  await pg.click('#x-auto');
  const n0 = await pg.evaluate(() => window.__runs.length);
  const off = 'switching auto-search off leaves the results as they are (not out of date)';
  await noteIs(done, off);
  await expect.soft(pg.locator('#results'), `${off}: not dimmed`).not.toContainClass('stale');
  // P5: a card un-owned in the picker greys its card in the result on show at once, the same element (auto-search
  // off: that result stays); owned again, it is not greyed
  const tile = await pg.evaluate(() => {
    const chip = document.querySelector('#results .deck .dc:not(.bor)'),
      name = chip.dataset.name;
    window.__chip = chip;
    return [...document.querySelectorAll('#cards button[aria-pressed]')].findIndex((t) => {
      const shown = t.querySelector('[data-card] + span').textContent;
      return name === shown || name.startsWith(`${shown} `);
    });
  });
  await openEditor(pg, 'cards');
  await pg.locator('#cards button[aria-pressed]').nth(tile).click();
  const greyed = await pg.evaluate(() => window.__chip.isConnected && window.__chip.classList.contains('off'));
  await pg.locator('#cards button[aria-pressed]').nth(tile).click();
  check(
    'P5: un-owning a card in the picker greys it in the result on screen at once (the same element), owning it ungreys it',
    tile >= 0 &&
      greyed &&
      (await pg.evaluate(() => window.__chip.isConnected && !window.__chip.classList.contains('off'))),
    `tile ${tile}, greyed ${greyed}`,
  );
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  await pg.waitForTimeout(1300); // past the pause a change would search after
  await closeEditor(pg, 'cards');
  const waits = 'auto-search off: a change does not search, the status says to press Search';
  check(waits, (await pg.evaluate(() => window.__runs.length)) === n0);
  await noteIs(/press Search/, `${waits}: the status`);
  await expect.soft(pg.locator('#r-act button', { hasText: 'Search' }), `${waits}: the button`).toHaveCount(1);
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
  check('… the Search button runs it', (await pg.evaluate(() => window.__runs.length)) === n0 + 1);
  await noteIs(done, '… the Search button runs it: the status');
  await openEditor(pg, 'cards');
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  await closeEditor(pg, 'cards');
  await pg.click('#x-auto');
  await settle(pg);
  check('… turning it back on searches the change', (await pg.evaluate(() => window.__runs.length)) === n0 + 2);
  const made0 = await pg.evaluate(() => window.__made);
  const shown0 = await pg.evaluate(() => window.__live.texts.length);
  await pg.evaluate(() =>
    [...document.querySelectorAll('#cards button[aria-pressed]')].slice(0, 5).forEach((b) => {
      b.click();
    }),
  ); // five changes inside one pause
  await noteIs(/Searching/, 'a change shows "Searching…"');
  // the fast pass is drawn first, with the status saying the exact check is still running: read from every text the
  // live region showed (watchLiveRegion), as the check may last less than a retrying assertion's interval between reads
  const firstPass = 'the first pass shows while the exact check runs';
  const checking = await pg
    .waitForFunction((from) => window.__live.texts.slice(from).some((t) => /Checking the exact odds/.test(t)), shown0, {
      timeout: 30_000,
    })
    .then(
      () => true,
      () => false,
    );
  check(firstPass, checking, JSON.stringify(await pg.evaluate((from) => window.__live.texts.slice(from), shown0)));
  await expect.soft(loops, `${firstPass}: the results`).not.toHaveCount(0);
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
  await expect.soft(loops, 'old results stay visible while searching').not.toHaveCount(0);
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
  await noteIs(done, 'the restarted search finishes');
  // the focus on a deck card panel's Owned switch when the card is un-owned: its search draws a new result, which
  // closes the panel, and the focus goes to the results column (not the page top)
  await pg.locator('.deck .dc:not(.bor)').first().click();
  await pg.locator('[data-pop="card"] input[role=switch]').focus();
  await pg.keyboard.press('Space');
  await settle(pg);
  const refocus = 'a result drawn while the focus is in its panel: the focus goes to the results column';
  await expect.soft(pg.locator('#rcol'), refocus).toBeFocused();
  await expect.soft(pg.locator('[data-pop="card"]').first(), `${refocus}: the panel closed`).toBeHidden();
  // a reload shows the kept result as done at once: its line is a change to the region the page loaded with
  await pg.reload();
  await settle(pg);
  const kept = await liveRegion(pg);
  check(
    "… and a reload's kept result is announced: its done line is a change to the live region, empty from load",
    kept.region === 'status polite' &&
      kept.same &&
      kept.texts[0] === '' &&
      /^Checked [\d,]+ decks/.test(kept.texts.at(-1)) &&
      (await pg.evaluate(() => window.__runs.length)) === 0,
    JSON.stringify(kept),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
