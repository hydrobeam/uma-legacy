import { check, open, settle } from './helpers.mjs';

export default async function borrowed(b) {
  const pg = await open(b);
  for (const nm of ['Light Hello', 'Sasami Anshinzawa'])
    await pg.locator('#cards .cgroup').first().locator('.ct', { hasText: nm }).first().click(); // the SSR section
  await settle(pg);
  const rows = await pg.evaluate(() =>
    [...document.querySelectorAll('.deck .dcards')]
      .map((r) => [...r.querySelectorAll('.dc')])
      .filter((r) => r.some((x) => x.classList.contains('bor')))
      .map((r) => r.findIndex((x) => x.classList.contains('bor')) === r.length - 1),
  );
  check('a deck borrows a card when the pals are not owned', rows.length > 0);
  check('the borrowed card is last in its row', rows.every(Boolean));
  check(
    'the borrowed card is named as borrowed',
    await pg.evaluate(() =>
      [...document.querySelectorAll('.dc.bor')].every((d) => /borrowed/.test(d.getAttribute('aria-label'))),
    ),
  );
  check(
    'the borrowed card has the friend frame',
    await pg.evaluate(() => {
      const d = document.querySelector('.dc.bor');
      return !!d && getComputedStyle(d, '::after').borderImageSource.startsWith('url');
    }),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
