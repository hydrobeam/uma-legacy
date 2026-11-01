import { expect } from 'playwright/test';
import { check, open, openEditor, settle, odds } from './helpers.mjs';

export default async function nonecards(b) {
  // "None owned" on support cards: no owned card reaches the solver (an empty owned list used to mean "all owned")
  const pg = await open(b);
  const all = await odds(pg);
  await openEditor(pg, 'cards');
  await pg.click('#c-none');
  await settle(pg);
  await expect
    .soft(pg.locator('#c-count'), 'None owned: the count at the top says "0 of N owned"')
    .toHaveText(/^0 of \d+ owned$/, { useInnerText: true });
  const job = await pg.evaluate(() => window.__runs.at(-1));
  const none = 'None owned: the job allows no owned card, only the one borrowed, and the odds drop';
  check(
    none,
    job.deck.owned_cards.length === 1 && job.deck.owned_cards[0] === 0 && job.deck.borrow === true,
    JSON.stringify(job),
  );
  await expect.soft(pg.locator('#results [data-odds]').first(), `${none}: the odds`).not.toHaveText(all);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
