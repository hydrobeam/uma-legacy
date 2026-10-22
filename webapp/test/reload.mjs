import { check, open, settle, odds } from './helpers.mjs';

export default async function reload(b) {
  // a reload with non-default settings shows their last result from the first frame (not the default example), and
  // the fresh identical result doesn't redraw it
  const pg = await open(b),
    ctx = pg.context(),
    def = await odds(pg);
  await pg.click('#rank button[data-v="0.20,0.70,0.10"]');
  await settle(pg); // rank SS: a different result, quick to search
  const mine = await odds(pg);
  await ctx.addInitScript(() => {
    window.__seen = [];
    const snap = () => {
      const o = document.querySelector('#results .loop .odds');
      window.__seen.push(o ? o.textContent : '');
      if (o && !o.__mark) o.__mark = 1;
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
  check(
    'reload: an identical fresh result is not redrawn',
    await pg.evaluate(() => document.querySelector('#results .loop .odds').__mark === 1),
  );
  // the saved result of these settings counts as done: no search runs, the status is its own "Checked …" line
  await pg.waitForTimeout(1500);
  check(
    'reload: the saved result is done, no search runs',
    (await pg.evaluate(() => !window.__runs.some((j) => j.cmd === 'deck'))) &&
      /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await pg.locator('#run-note').innerText()),
    await pg.locator('#run-note').innerText(),
  );
  await pg.click('#rank button[data-v="0.50,0.45,0.05"]');
  await settle(pg);
  check(
    'reload: … and the next edit searches',
    (await pg.evaluate(() => window.__runs.some((j) => j.cmd === 'deck'))) && (await odds(pg)) === def,
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await ctx.close();
}
