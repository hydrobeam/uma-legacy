import {
  CONTEXT,
  LOOP_URL,
  PAGE_URL,
  check,
  open,
  settle,
  until,
  navClick,
  note,
  odds,
  scen,
  hintRates,
} from './helpers.mjs';

export default async function gm(b) {
  // the era buttons: switching to the Grand Masters-era data (fetched, the status line says so meanwhile) opens on
  // Grand Masters (rank SS, no Unity golds) and searches with it; back on Global the scenario is Unity Cup again
  const pg = await open(b);
  await pg.route('**/assets/data-gm-*', async (r) => {
    await new Promise((x) => setTimeout(x, 1500));
    await r.continue();
  }); // a slow fetch of the GM era's data
  await navClick(pg, '#era-gm');
  check(
    'GM era: while its data loads, the status line says so',
    await until(pg, () =>
      /Loading the JP Grand Masters-era data/.test(document.getElementById('run-note').textContent),
    ),
    await note(pg),
  );
  await settle(pg);
  await pg.unroute('**/assets/data-gm-*');
  check(
    'switching to the GM era selects Grand Masters, at rank SS, and searches with it',
    (await scen(pg)) === 'grandmasters' &&
      (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: Grand Masters' &&
      (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' &&
      /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)) &&
      (await pg.locator('#results .loop').count()) > 0 &&
      (await pg.evaluate(
        () =>
          window.__runs.at(-1).scenario === 'grandmasters' && window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1',
      )),
    await odds(pg),
  );
  check(
    'Grand Masters gives no guaranteed golds',
    (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0',
  );
  const umas = (
    await pg.evaluate(() =>
      fetch(JSON.parse(document.getElementById('data-assets').textContent).dataGm).then((r) => r.json()),
    )
  ).umas;
  const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
  check('GM era: roster from the GM data', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
  // the GM era offers Grand Masters as a fifth tile (same size as the others)
  await pg.click('#x-scen');
  const gt = await pg.evaluate(() =>
    [...document.querySelectorAll('#scen-menu .stile')].map((t) => {
      const r = t.getBoundingClientRect();
      return {
        v: t.dataset.v,
        sub: t.querySelector('.ssub').innerText.replace(/\s+/g, ' ').trim(),
        size: Math.round(r.width) + 'x' + Math.round(r.height),
        x: Math.round(r.left + r.width / 2),
      };
    }),
  );
  const mid = await pg.evaluate(() => {
    const r = document.getElementById('scen-menu').getBoundingClientRect();
    return Math.round(r.left + r.width / 2);
  });
  check(
    'GM era: five equal tiles, Grand Masters last and centred',
    gt.length === 5 &&
      gt[4].v === 'grandmasters' &&
      gt[4].sub.replace(/ \((estimate|est\.)\)$/, '') === hintRates().grandmasters &&
      / \((estimate|est\.)\)$/.test(gt[4].sub) &&
      new Set(gt.map((t) => t.size)).size === 1 &&
      Math.abs(gt[4].x - mid) <= 1,
    JSON.stringify(gt),
  );
  if (process.env.LLP_SHOTS)
    await pg
      .locator('#scen-menu')
      .screenshot({ path: `${process.env.LLP_SHOTS}/${b.browserType().name()}-scen-picker-gm.png` });
  await pg.keyboard.press('Escape');
  await navClick(pg, '#era-global');
  await settle(pg);
  check(
    'back on Global: Unity Cup with its golds, rank below SS',
    (await scen(pg)) === 'unity' &&
      (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05' &&
      (await pg.getAttribute('#g-20049', 'data-g')) === '2' &&
      // (Global's result from before the switch is saved: it shows as done, no search; else the search is Unity Cup's)
      (await pg.evaluate(() =>
        window.__runs.length === 0
          ? /^Checked [\d,]+ decks/.test(document.getElementById('run-note').textContent)
          : window.__runs.at(-1).scenario === 'unity',
      )),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
  await stopsAtLoad(b);
}
// a page whose scripts stop while they load (here Unity Cup's hint base missing from the page's data, which the build
// refuses: data.ts reads it as it loads) says so on its status line, both pages, rather than stay blank (errorlog.ts)
async function stopsAtLoad(b) {
  const ctx = await b.newContext(CONTEXT),
    pg = await ctx.newPage(),
    errors = [];
  pg.on('pageerror', (e) => errors.push(String(e)));
  // the hint bases are one JSON text in the scripts both pages share (vite/pagedata.mjs '#page-data')
  await pg.route('**/assets/shared-*.js', async (r) => {
    const response = await r.fetch(),
      text = await response.text();
    if (text.split('{"unity":').length !== 2) throw new Error('the shared script holds no one hint-base table');
    await r.fulfill({ response, body: text.replace('{"unity":', '{"unity cup":') });
  });
  const said = [];
  for (const url of [PAGE_URL, LOOP_URL]) {
    await pg.goto(url);
    said.push(
      await pg.evaluate(() => [
        document.getElementById('run-note').textContent,
        document.getElementById('rhead').dataset.kind,
      ]),
    );
  }
  check(
    'a script that stops as the page loads: the status line says the page could not start, and why (both pages)',
    said.every(
      ([text, kind]) =>
        text === "The page could not start: failed: the build writes unity's hint base" && kind === 'error',
    ) && errors.length === 2,
    `${JSON.stringify(said)} | ${errors.join(' | ')}`,
  );
  await ctx.close();
}
