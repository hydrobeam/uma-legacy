import { expect } from 'playwright/test';
import { check, open, openEditor, settle, pickScen, CARD_ART, json } from './helpers.mjs';

export default async function cards(b) {
  const pg = await open(b);
  await expect
    .soft(pg.locator('#cards .cgroup h3'), 'card sections SSR, SR, R in that order')
    .toHaveText(['SSR', 'SR', 'R']);
  const notes = await pg.evaluate(() =>
    [...document.querySelectorAll('#cards .gv')].map((g) => [g.querySelector('small').textContent, g.title]),
  );
  check(
    'card hint notes in plain words, each with a tooltip',
    notes.length > 0 &&
      notes.every(([n, t]) => /^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n) && t.length > 0),
    JSON.stringify(notes.filter(([n]) => !/^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n)).slice(0, 3)),
  );
  const art = await pg.evaluate(CARD_ART);
  check(
    'card list: every card picture is the same size and inside its tile, loaded or not',
    art.sizes.length === 1 && !art.over,
    JSON.stringify(art),
  );
  check(
    'card names carry no rarity (the frame and section show it)',
    await pg.evaluate(
      () => ![...document.querySelectorAll('#cards .ct .cn')].some((e) => / (SSR|SR|R)$/.test(e.textContent)),
    ),
  );
  // the game bans Team Sirius from Unity Cup and Grand Concert decks: the picker says so and no result deck holds it
  const sirius = pg.locator('#cards .ct', { hasText: 'Team Sirius' });
  await expect.soft(sirius, 'Team Sirius: in the card picker').toHaveCount(1);
  await expect
    .soft(sirius.locator('.cban'), 'Team Sirius: "Not allowed in Unity Cup" in the card picker')
    .toHaveText('Not allowed in Unity Cup');
  await expect.soft(pg.locator('#results .dc[data-cid="30081"]'), '… and in no Unity Cup result deck').toHaveCount(0);
  await pickScen(pg, 'ura'); // the picker follows at once; the search is not awaited
  await expect.soft(sirius.locator('.cban'), 'Team Sirius allowed again in URA Finale').toHaveCount(0);
  await pickScen(pg, 'unity');
  await settle(pg);
  await openEditor(pg, 'cards');
  const sep = await pg.evaluate(() => {
    const c = document.getElementById('cards'),
      r = c.previousElementSibling.getBoundingClientRect(),
      cs = getComputedStyle(c);
    return { gap: Math.round(c.getBoundingClientRect().top - r.bottom), pad: parseFloat(cs.paddingTop) };
  });
  check(
    "the card list scrolls apart from its search row (gap like the roster's), its tiles' rings and focus outline inside it",
    sep.gap >= 10 && sep.pad >= 5,
    JSON.stringify(sep),
  );
  await expect.soft(pg.locator('#cs'), 'default card sort: release, newest first').toHaveText('Release ↓');
  const tile = pg.locator('#cards .ct').first();
  const img = await tile.evaluate((t) => {
    t.__img = t.querySelector('img');
    return !!t.__img;
  });
  await tile.click();
  await expect.soft(tile, 'toggling a card: not owned').toHaveAttribute('aria-pressed', 'false');
  check(
    'toggling a card keeps its image element (no flicker)',
    img && (await tile.evaluate((t) => t.querySelector('img') === t.__img)),
  );
  await tile.click();
  // R cards are left out (no setting): the R section holds only the R pals
  const rpals = (await json(pg, 'data-cards')).filter((c) => c.rar === 1 && c.pal).length;
  await expect.soft(pg.locator('#x-rcards'), 'R cards are left out, with no switch').toHaveCount(0);
  await expect
    .soft(
      pg
        .locator('#cards .cgroup')
        .filter({ has: pg.locator('h3', { hasText: /^R$/ }) })
        .locator('.ct'),
      '… the R section keeps the R pals',
    )
    .toHaveCount(rpals);
  check(
    'every card image carries its type icon',
    (await pg.locator('#cards .ct [data-card="art"]').count()) ===
      (await pg.locator('#cards .ct [data-ctype]').count()),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
