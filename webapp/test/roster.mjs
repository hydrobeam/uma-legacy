import { check, open, json } from './helpers.mjs';

export default async function roster(b) {
  const pg = await open(b);
  // as the page shows them: outfits dated after now hidden, an uma whose every outfit is (data.js LATER)
  const umas = (await json(pg, 'data-umas'))
    .map((u) => ({ ...u, outfits: u.outfits.filter((o) => o.rel <= Date.now() / 1000), had: u.outfits.length }))
    .filter((u) => u.outfits.length || !u.had);
  const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
  check('one tile per outfit', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
  const all = umas.flatMap((u) =>
    (u.outfits.length ? u.outfits : [{ label: '', rel: 0, id: 0 }]).map((o) => ({
      n: u.name + (o.label ? ' ' + o.label : ''),
      rel: o.rel,
      id: o.id,
    })),
  );
  const firstTile = async () => (await pg.locator('#roster .ut').first().innerText()).replace(/\n/g, ' ').trim();
  const newest = [...all].sort((a, c) => c.rel - a.rel || c.id - a.id)[0].n,
    oldest = [...all].sort((a, c) => a.rel - c.rel || a.id - c.id)[0].n;
  check(
    'default sort: release, newest first',
    (await pg.locator('#rs').innerText()) === 'Release ↓' && (await firstTile()) === newest,
    await firstTile(),
  );
  await pg.click('#rs');
  await pg.click('#rs-menu [data-k="rel"]');
  check(
    're-picking Release reverses it (oldest first)',
    (await pg.locator('#rs').innerText()) === 'Release ↑' && (await firstTile()) === oldest,
    await firstTile(),
  );
  check('the sort menu stays open after a pick', !(await pg.locator('#rs-menu').isHidden()));
  await pg.click('#rs-menu [data-k="name"]');
  const byName = [...all].sort((a, c) => a.n.localeCompare(c.n))[0].n;
  check(
    'Name sorts A to Z',
    (await pg.locator('#rs').innerText()) === 'Name ↑' && (await firstTile()) === byName,
    await firstTile(),
  );
  await pg.keyboard.press('Escape');
  check('Escape closes the menu', await pg.locator('#rs-menu').isHidden());
  // an uma with several outfits: a tap marks that outfit not owned, a second tap owned again
  const multi = umas.find((u) => u.outfits.length > 1);
  const states = () =>
    pg.evaluate(
      (id) => [...document.querySelectorAll(`#roster .ut[data-uma="${id}"]`)].map((t) => t.dataset.s).join(','),
      multi.id,
    );
  const tile = pg.locator(`#roster .ut[data-uma="${multi.id}"] .up`).first();
  await tile.click();
  const s1 = await states();
  await tile.click();
  const s2 = await states();
  check('tile toggle: that outfit not owned', s1.split(',').filter((x) => x === 'out').length === 1, s1);
  check(
    'tile toggle: back to owned',
    s2.split(',').every((x) => x === 'own'),
    s2,
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
