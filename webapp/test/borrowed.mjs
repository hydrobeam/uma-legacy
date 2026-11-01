import { check, open, openEditor, settle } from './helpers.mjs';

export default async function borrowed(b) {
  const pg = await open(b);
  await openEditor(pg, 'cards');
  for (const nm of ['Light Hello', 'Sasami Anshinzawa'])
    await pg.locator('#cards [role="group"]').first().locator('button[aria-pressed]', { hasText: nm }).first().click(); // the SSR section
  await settle(pg);
  const rows = await pg.evaluate(() =>
    [...document.querySelectorAll('#results [data-deck-cards]')]
      .map((r) => [...r.querySelectorAll('button[data-cid]')])
      .filter((r) => r.some((x) => x.dataset.tag === 'borrowed'))
      .map((r) => r.findIndex((x) => x.dataset.tag === 'borrowed') === r.length - 1),
  );
  check('a deck borrows a card when the pals are not owned', rows.length > 0);
  check('the borrowed card is last in its row', rows.every(Boolean));
  check(
    'the borrowed card is named as borrowed',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#results button[data-tag="borrowed"]')].every((d) =>
        /borrowed/.test(d.getAttribute('aria-label')),
      ),
    ),
  );
  check(
    'the borrowed card has the friend frame',
    await pg.evaluate(() => {
      const d = document.querySelector('#results button[data-tag="borrowed"]');
      return !!d && getComputedStyle(d, '::after').borderImageSource.startsWith('url');
    }),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
