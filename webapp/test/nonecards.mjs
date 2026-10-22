import { check, open, settle, odds } from './helpers.mjs';

export default async function nonecards(b) {
  // "None owned" on support cards: no owned card reaches the solver (an empty owned list used to mean "all owned")
  const pg = await open(b); const all = await odds(pg);
  await pg.click('#c-none'); await settle(pg);
  check('None owned: the count at the top says "0 of N owned"', /^0 of \d+ owned$/.test(await pg.locator('#c-count').innerText()), await pg.locator('#c-count').innerText());
  const job = await pg.evaluate(() => window.__runs.at(-1));
  check('None owned: the job allows no owned card, only the one borrowed, and the odds drop', job.owned_cards.length === 1 && job.owned_cards[0] === 0 && job.borrow === true && (await odds(pg)) !== all, (await odds(pg)) + ' vs ' + all);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
