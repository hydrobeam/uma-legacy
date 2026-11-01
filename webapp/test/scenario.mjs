import { expect } from 'playwright/test';
import { check, json, open, settle, lastJob, pickScen, hintRates } from './helpers.mjs';

export default async function scenario(b) {
  // the scenario picker: a button showing the chosen scenario's logo, opening a grid of the scenarios
  const pg = await open(b);
  const rankQ = pg.locator('#rank-q');
  await rankQ.click();
  await expect.soft(pg.locator('#rank-note'), 'lineage rank: the ? opens its explanation').toBeVisible();
  await expect.soft(rankQ, 'lineage rank: the ? … expanded').toHaveAttribute('aria-expanded', 'true');
  await rankQ.click();
  const menu = pg.locator('#scen-menu'),
    tiles = pg.locator('#scen-menu .stile'),
    button = pg.locator('#x-scen'),
    rank = pg.locator('#rank');
  // the button: the chosen scenario (its data-value) and its name for assistive tech
  const chosen = async (v, name, what) => {
    await expect.soft(button, what).toHaveAttribute('data-value', v);
    await expect.soft(button, `${what}: named`).toHaveAttribute('aria-label', `Scenario: ${name}`);
  };
  // the job last sent asks for this lineage rank
  const jobRank = (dist) => pg.evaluate((d) => window.__runs.at(-1).star_dist.join() === d, dist);
  const first = 'picker closed at first; the button is Unity Cup (logo only, named for assistive tech)';
  await expect.soft(menu, first).toBeHidden();
  await chosen('unity', 'Unity Cup', first);
  if ((await json(pg, 'data-img')).keys.includes('scen:2')) {
    await expect.soft(button, `${first}: no text`).toHaveText('', { useInnerText: true });
    await expect.soft(button.locator('img'), `${first}: the logo`).toHaveCount(1);
  } else
    await expect.soft(button, `${first}: the name, without the logo`).toHaveText('Unity Cup', { useInnerText: true });
  const shots = process.env.LLP_SHOTS,
    bn = b.browserType().name(); // pictures of the button and the open picker, for a look at the logos
  if (shots) await pg.locator('#x-scen').screenshot({ path: `${shots}/${bn}-scen-button.png` });
  await pg.click('#x-scen');
  const t = await pg.evaluate(() =>
    [...document.querySelectorAll('#scen-menu .stile')].map((t) => {
      const r = t.getBoundingClientRect();
      const sub = t.querySelector('.ssub'),
        sr = sub.getBoundingClientRect(),
        pad = parseFloat(getComputedStyle(t).paddingBottom) + parseFloat(getComputedStyle(t).borderBottomWidth);
      return {
        v: t.dataset.v,
        sub: sub.innerText.replace(/\s+/g, ' ').trim(),
        w: Math.round(r.width),
        h: Math.round(r.height),
        gap: Math.round(r.bottom - pad - sr.bottom),
        subH: Math.round(sr.height),
      };
    }),
  );
  // hint rate as a multiple of URA Finale's, from the solver's hint_base (pkg/loopopt/src/deck/hints.rs)
  const want = hintRates();
  check(
    'each tile has the same hover text as the button gives the chosen scenario',
    await pg.evaluate(
      () =>
        [...document.querySelectorAll('#scen-menu .stile')].every(
          (t) => t.title && t.title === t.getAttribute('aria-label'),
        ) &&
        document.querySelector('#scen-menu .stile[data-v="unity"]').title === document.getElementById('x-scen').title,
    ),
  );
  const opens = 'opens with four tiles, each with its hint rate against URA Finale';
  await expect.soft(menu, opens).toBeVisible();
  await expect.soft(tiles, `${opens}: four`).toHaveCount(4);
  check(
    `${opens}: the rates`,
    t.length === 4 && t.every((x) => x.sub === want[x.v]),
    t.map((x) => x.v + ': ' + x.sub).join(' | '),
  );
  check(
    'the four tiles are the same size',
    new Set(t.map((x) => x.w + 'x' + x.h)).size === 1,
    t.map((x) => x.w + 'x' + x.h).join(' '),
  );
  check(
    'every tile is logo + one line',
    t.every((x) => x.subH <= 22),
    t.map((x) => `${x.v} line ${x.subH}px, blank ${x.gap}px`).join(' | '),
  );
  // Unity Cup's guaranteed golds: their gold skill icons on the hint-rate line, named by tooltip and alt text
  const golds = await pg.evaluate(
    ([skl, gi]) => {
      const g = document.querySelector('#scen-menu .stile[data-v="unity"] .ssub .sgolds');
      return {
        want: [20049, 20046].map((x) => ({ name: skl.find((s) => s[0] === x)[2], src: `/skill/${gi[x]}.avif` })),
        // biome-ignore lint/complexity/useOptionalChain: `?.` would turn null into undefined, which the stringified check compares
        group: g && g.getAttribute('aria-label'),
        got: g
          ? [...g.querySelectorAll('img')].map((i) => ({
              alt: i.alt,
              title: i.title,
              src: i.getAttribute('src'),
              h: Math.round(i.getBoundingClientRect().height),
            }))
          : [],
        text: g ? g.innerText.trim() : null,
        others: document.querySelectorAll('#scen-menu .stile:not([data-v="unity"]) .sgolds').length,
      };
    },
    [await json(pg, 'data-skills'), (await json(pg, 'data-skillicon')).gold],
  );
  check(
    "Unity Cup tile: No Stopping Me! and It's On! as gold icons on the rate line, no visible names",
    golds.group === 'Guaranteed in every run' &&
      golds.others === 0 &&
      golds.text === '' &&
      golds.got.length === 2 &&
      golds.want.every(
        (w, i) =>
          golds.got[i].alt === w.name &&
          golds.got[i].title === w.name &&
          golds.got[i].src.endsWith(w.src) &&
          golds.got[i].h <= 18,
      ),
    JSON.stringify(golds.got),
  );
  check(
    'logos keep their aspect (object-fit: contain)',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#scen-dd .slogo img')].every(
        (i) => i.naturalWidth > 0 && getComputedStyle(i).objectFit === 'contain',
      ),
    ),
  );
  const marks = 'the open picker marks the chosen tile and focuses it';
  const unity = menu.locator('.stile[data-v="unity"]');
  await expect.soft(unity, marks).toBeFocused();
  await expect.soft(unity, `${marks}: checked`).toHaveAttribute('aria-checked', 'true');
  await expect.soft(button, `${marks}: the button expanded`).toHaveAttribute('aria-expanded', 'true');
  if (shots) await menu.screenshot({ path: `${shots}/${bn}-scen-picker.png` });
  const r0 = await pg.evaluate(() => window.__runs.length);
  await tiles.nth(0).click(); // a change would show "Searching…" at once
  const repick = 're-picking the chosen scenario closes the picker and does not search';
  await expect.soft(menu, repick).toBeHidden();
  check(`${repick}: no job`, (await pg.evaluate(() => window.__runs.length)) === r0);
  await expect.soft(pg.locator('#run-note'), `${repick}: the status`).not.toHaveText(/Searching/);
  await pg.keyboard.press('ArrowDown'); // focus is back on the button
  await expect.soft(menu, 'arrow key on the button opens the picker').toBeVisible();
  await pg.keyboard.press('ArrowRight');
  await pg.keyboard.press('ArrowDown');
  await expect
    .soft(menu.locator('.stile[data-v="trackblazer"]'), 'arrows move between tiles (two columns)')
    .toBeFocused();
  await pg.keyboard.press('Escape');
  await expect.soft(menu, 'Escape closes it and returns focus to the button').toBeHidden();
  await expect.soft(button, 'Escape closes it and returns focus to the button: focus').toBeFocused();
  await button.click();
  await pg.getByRole('heading', { level: 1 }).click();
  await expect.soft(menu, 'a click outside closes it').toBeHidden();
  // only the button and the grid count as inside: the label, the row's empty space and the rank control all close it
  const shut = 'clicks on the label, beside the button, on the rank control close it';
  await button.click();
  await pg.click('#x-scen-l');
  await expect.soft(menu, `${shut}: the label`).toBeHidden();
  await button.click();
  const bb = await button.boundingBox();
  await pg.mouse.click(bb.x + bb.width + 8, bb.y + bb.height / 2);
  await expect.soft(menu, `${shut}: beside the button`).toBeHidden();
  await button.click();
  await pg.click('.rankctl > span');
  await expect.soft(menu, `${shut}: the rank control`).toBeHidden();
  await button.click();
  await pg.keyboard.press('End');
  await pg.keyboard.press('Enter');
  await lastJob(pg, 'trackblazer');
  const enter = 'Enter on a tile picks it and closes the picker';
  await expect.soft(menu, enter).toBeHidden();
  await chosen('trackblazer', 'Trackblazer', `${enter}: the button`);
  // the lineage rank follows the scenario: Grand Concert SS, Unity Cup below SS
  await pickScen(pg, 'gc');
  await lastJob(pg, 'gc');
  const sirius = await pg.evaluate(() => {
    const t = [...document.querySelectorAll('#cards .ct')].find((x) => /Team Sirius/.test(x.textContent));
    return t ? t.querySelector('.cban')?.textContent || '' : null;
  });
  check(
    'Team Sirius: "Not allowed in Grand Concert" in the card picker',
    sirius === 'Not allowed in Grand Concert',
    String(sirius),
  );
  const gc = 'picking Grand Concert sets the rank to SS';
  await expect.soft(rank, gc).toHaveAttribute('data-value', '0.20,0.70,0.10');
  check(`${gc}: the job`, await jobRank('0.2,0.7,0.1'));
  await pickScen(pg, 'unity');
  await settle(pg);
  await expect.soft(rank, 'picking Unity Cup sets it back to below SS').toHaveAttribute('data-value', '0.50,0.45,0.05');
  await pg.click('#rank button[data-v="0.20,0.70,0.10"]');
  await settle(pg);
  await pickScen(pg, 'unity');
  const sticks = 'a rank changed by hand sticks (re-picking the scenario leaves it)';
  await expect.soft(rank, sticks).toHaveAttribute('data-value', '0.20,0.70,0.10');
  check(`${sticks}: the job`, await jobRank('0.2,0.7,0.1'));
  // the rank toggle is a radio group: game badges, arrow keys move the choice
  await pg.focus('#rank button[aria-checked="true"]');
  await pg.keyboard.press('ArrowLeft');
  const arrows = 'lineage rank: arrow keys switch it, aria-checked follows, badges shown';
  await expect.soft(rank, arrows).toHaveAttribute('data-value', '0.50,0.45,0.05');
  await expect
    .soft(rank.locator('button[data-v="0.50,0.45,0.05"]'), `${arrows}: aria-checked`)
    .toHaveAttribute('aria-checked', 'true');
  await expect.soft(rank.locator('[data-r] img'), `${arrows}: badges`).toHaveCount(3);
  await pg.keyboard.press('ArrowRight');
  await settle(pg);
  await pg.click('#ex-open');
  check('export names the scenario', JSON.parse(await pg.inputValue('#ex-text')).readable.scenario === 'Unity Cup');
  const text = (await pg.inputValue('#ex-text')).replace('"scen": "unity"', '"scen": "gc"');
  await pg.fill('#ex-text', text);
  await pg.click('#ex-load');
  await pg.click('#ex-close');
  await chosen('gc', 'Grand Concert', 'import updates the button'); // its (slow) search is not awaited
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
  // without the logo files (images not built, or a failed fetch) the button and tiles show the name on a badge
  const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', (r) => r.abort()) });
  await p2.click('#x-scen');
  const fallback = 'missing logos fall back to name badges';
  await expect.soft(p2.locator('#scen-dd .slogo img'), `${fallback}: no picture`).toHaveCount(0);
  await expect
    .soft(p2.locator('#scen-dd .slogo b'), fallback)
    .toHaveText(['Unity Cup', 'Unity Cup', 'Grand Concert', 'URA Finale', 'Trackblazer']);
  const sz = await p2.evaluate(
    () =>
      new Set(
        [...document.querySelectorAll('#scen-menu .stile')].map((t) => {
          const r = t.getBoundingClientRect();
          return Math.round(r.width) + 'x' + Math.round(r.height);
        }),
      ).size,
  );
  check('… and the tiles stay the same size', sz === 1);
  await p2.context().close();
}
