import { check, open, json, repoUrl } from './helpers.mjs';

export default async function roster(b) {
  const pg = await open(b);
  // as the page shows them: outfits dated after now hidden, an uma whose every outfit is (data.js `gameData().later`)
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
  // umadump's owned outfits and cards (made-up: val/umadump/U_card_data.json, U_support_card_data.json), both files
  // through the Umas dialog's Import: what they list owned, everything else not owned, ids the data lacks skipped
  const dump = (file) => repoUrl(`val/umadump/${file}`).pathname,
    importOwned = async (files) => {
      await pg.evaluate(() => {
        document.getElementById('r-msg').textContent = '';
      });
      await pg.locator('#r-file').setInputFiles(files.map(dump));
      await pg.waitForFunction(() => document.getElementById('r-msg').textContent);
      return pg.locator('#r-msg').innerText();
    },
    saved = () => pg.evaluate(() => JSON.parse(localStorage.getItem('llp:settings')).global);
  const msg = await importOwned(['U_card_data.json', 'U_support_card_data.json']),
    after = await saved(),
    cards = (await json(pg, 'data-cards')).filter((c) => c.rel <= Date.now() / 1000),
    outfitIds = umas.flatMap((u) => u.outfits.map((o) => o.id)),
    fileOutfits = [100101, 100102, 199901],
    fileCards = [30001, 30002, 10001, 39999],
    knownOutfits = fileOutfits.filter((id) => outfitIds.includes(id)).length,
    knownCards = fileCards.filter((id) => cards.some((c) => c.id === id)).length;
  check(
    "umadump import: the file's outfits owned, every other not",
    umas.every((u) => (after.roster[u.id] === 'out') === (!u.outfits.length && u.id !== 1001)) &&
      outfitIds.every((id) => (after.outfits[id] === false) === !fileOutfits.includes(id)),
    msg,
  );
  check(
    "umadump import: the file's cards owned, every other not",
    cards.every((c) => (after.cardsOff[c.id] === false) === !fileCards.includes(c.id)),
    msg,
  );
  check(
    'umadump import: says what it set and what it skipped',
    msg ===
      `${knownOutfits} outfits owned (${fileOutfits.length - knownOutfits} not in the data). ${knownCards} cards owned (${fileCards.length - knownCards} not in the data).`,
    msg,
  );
  const wrong = await importOwned(['U_trained_chara_data.json']);
  check(
    'umadump import: a veterans file is refused, nothing changed',
    /veterans file/.test(wrong) && JSON.stringify(await saved()) === JSON.stringify(after),
    wrong,
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
