import { check, open, settle, lastJob, until, note, scen, pickScen, hintRates } from './helpers.mjs';

export default async function scenario(b) {
  // the scenario picker: a button showing the chosen scenario's logo, opening a grid of the scenarios
  const pg = await open(b);
  await pg.click('#rank-q');
  check('lineage rank: the ? opens its explanation', !(await pg.locator('#rank-note').isHidden()) && (await pg.getAttribute('#rank-q', 'aria-expanded')) === 'true');
  await pg.click('#rank-q');
  const menu = pg.locator('#scen-menu'), tiles = pg.locator('#scen-menu .stile');
  const btnName = () => pg.getAttribute('#x-scen', 'aria-label');
  check('picker closed at first; the button is Unity Cup (logo only, named for assistive tech)', (await menu.isHidden()) && (await scen(pg)) === 'unity' && (await btnName()) === 'Scenario: Unity Cup'
    && (await pg.evaluate(() => self.__LLP_DATA['data-img'].keys.includes('scen:2'))
      ? (await pg.locator('#x-scen').innerText()).trim() === '' && (await pg.locator('#x-scen img').count()) === 1
      : (await pg.locator('#x-scen').innerText()).trim() === 'Unity Cup'));
  const shots = process.env.LLP_SHOTS, bn = b.browserType().name();   // pictures of the button and the open picker, for a look at the logos
  if (shots) await pg.locator('#x-scen').screenshot({ path: `${shots}/${bn}-scen-button.png` });
  await pg.click('#x-scen');
  const t = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect();
    const sub = t.querySelector('.ssub'), sr = sub.getBoundingClientRect(), pad = parseFloat(getComputedStyle(t).paddingBottom) + parseFloat(getComputedStyle(t).borderBottomWidth);
    return { v: t.dataset.v, sub: sub.innerText.replace(/\s+/g, ' ').trim(), w: Math.round(r.width), h: Math.round(r.height), gap: Math.round(r.bottom - pad - sr.bottom), subH: Math.round(sr.height) }; }));
  // hint rate as a multiple of URA Finale's, from the solver's hint_base (pkg/loopopt/src/deck/hints.rs)
  const want = hintRates();
  check('each tile has the same hover text as the button gives the chosen scenario', await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].every(t => t.title && t.title === t.getAttribute('aria-label')) && document.querySelector('#scen-menu .stile[data-v="unity"]').title === document.getElementById('x-scen').title));
  check('opens with four tiles, each with its hint rate against URA Finale', !(await menu.isHidden()) && t.length === 4 && t.every(x => x.sub === want[x.v]), t.map(x => x.v + ': ' + x.sub).join(' | '));
  check('the four tiles are the same size', new Set(t.map(x => x.w + 'x' + x.h)).size === 1, t.map(x => x.w + 'x' + x.h).join(' '));
  check('every tile is logo + one line', t.every(x => x.subH <= 22), t.map(x => `${x.v} line ${x.subH}px, blank ${x.gap}px`).join(' | '));
  // Unity Cup's guaranteed golds: their gold skill icons on the hint-rate line, named by tooltip and alt text
  const golds = await pg.evaluate(() => { const skl = self.__LLP_DATA['data-skills'], gi = self.__LLP_DATA['data-skillicon'].gold;
    const g = document.querySelector('#scen-menu .stile[data-v="unity"] .ssub .sgolds');
    return { want: [20049, 20046].map(x => ({ name: skl.find(s => s[0] === x)[2], src: `/skill/${gi[x]}.avif` })), group: g && g.getAttribute('aria-label'),
      got: g ? [...g.querySelectorAll('img')].map(i => ({ alt: i.alt, title: i.title, src: i.getAttribute('src'), h: Math.round(i.getBoundingClientRect().height) })) : [],
      text: g ? g.innerText.trim() : null, others: document.querySelectorAll('#scen-menu .stile:not([data-v="unity"]) .sgolds').length }; });
  check('Unity Cup tile: No Stopping Me! and It\'s On! as gold icons on the rate line, no visible names', golds.group === 'Guaranteed in every run' && golds.others === 0 && golds.text === ''
    && golds.got.length === 2 && golds.want.every((w, i) => golds.got[i].alt === w.name && golds.got[i].title === w.name && golds.got[i].src.endsWith(w.src) && golds.got[i].h <= 18), JSON.stringify(golds.got));
  check('logos keep their aspect (object-fit: contain)', await pg.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo img')].every(i => i.naturalWidth > 0 && getComputedStyle(i).objectFit === 'contain')));
  check('the open picker marks the chosen tile and focuses it', await pg.evaluate(() => { const t = document.activeElement; return t.classList.contains('stile') && t.dataset.v === 'unity' && t.getAttribute('aria-checked') === 'true'; })
    && (await pg.getAttribute('#x-scen', 'aria-expanded')) === 'true');
  if (shots) await menu.screenshot({ path: `${shots}/${bn}-scen-picker.png` });
  const r0 = await pg.evaluate(() => window.__runs.length);
  await tiles.nth(0).click();   // a change would show "Searching…" at once
  check('re-picking the chosen scenario closes the picker and does not search', (await menu.isHidden()) && (await pg.evaluate(() => window.__runs.length)) === r0 && !/Searching/.test(await note(pg)));
  await pg.keyboard.press('ArrowDown');   // focus is back on the button
  check('arrow key on the button opens the picker', !(await menu.isHidden()));
  await pg.keyboard.press('ArrowRight'); await pg.keyboard.press('ArrowDown');
  check('arrows move between tiles (two columns)', await pg.evaluate(() => document.activeElement.dataset.v) === 'trackblazer');
  await pg.keyboard.press('Escape');
  check('Escape closes it and returns focus to the button', (await menu.isHidden()) && await pg.evaluate(() => document.activeElement.id === 'x-scen'));
  await pg.click('#x-scen'); await pg.click('header.top h1');
  check('a click outside closes it', await menu.isHidden());
  // only the button and the grid count as inside: the label, the row's empty space and the rank control all close it
  const shut = [];
  await pg.click('#x-scen'); await pg.click('#x-scen-l'); shut.push(await menu.isHidden());
  await pg.click('#x-scen'); const bb = await pg.locator('#x-scen').boundingBox(); await pg.mouse.click(bb.x + bb.width + 8, bb.y + bb.height / 2); shut.push(await menu.isHidden());
  await pg.click('#x-scen'); await pg.click('.rankctl > span'); shut.push(await menu.isHidden());
  check('clicks on the label, beside the button, on the rank control close it', shut.every(Boolean), shut.join(','));
  await pg.click('#x-scen'); await pg.keyboard.press('End'); await pg.keyboard.press('Enter'); await lastJob(pg, 'trackblazer');
  check('Enter on a tile picks it and closes the picker', (await menu.isHidden()) && (await scen(pg)) === 'trackblazer' && (await btnName()) === 'Scenario: Trackblazer');
  // the lineage rank follows the scenario: Grand Concert SS, Unity Cup below SS
  await pickScen(pg, 'gc'); await lastJob(pg, 'gc');
  const sirius = await pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? t.querySelector('.cban')?.textContent || '' : null; });
  check('Team Sirius: "Not allowed in Grand Concert" in the card picker', sirius === 'Not allowed in Grand Concert', String(sirius));
  check('picking Grand Concert sets the rank to SS', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
  await pickScen(pg, 'unity'); await settle(pg);
  check('picking Unity Cup sets it back to below SS', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05');
  await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg); await pickScen(pg, 'unity');
  check('a rank changed by hand sticks (re-picking the scenario leaves it)', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
  // the rank toggle is a radio group: game badges, arrow keys move the choice
  await pg.focus('#rank button[aria-checked="true"]'); await pg.keyboard.press('ArrowLeft');
  check('lineage rank: arrow keys switch it, aria-checked follows, badges shown', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
    && (await pg.getAttribute('#rank button[data-v="0.50,0.45,0.05"]', 'aria-checked')) === 'true' && (await pg.locator('#rank .rk.img img').count()) === 3);
  await pg.keyboard.press('ArrowRight'); await settle(pg);
  await pg.click('#ex-open');
  check('export names the scenario', JSON.parse(await pg.inputValue('#ex-text')).readable.scenario === 'Unity Cup');
  const text = (await pg.inputValue('#ex-text')).replace('"scen": "unity"', '"scen": "gc"');
  await pg.fill('#ex-text', text); await pg.click('#ex-load'); await pg.click('#ex-close');
  check('import updates the button', (await scen(pg)) === 'gc' && (await btnName()) === 'Scenario: Grand Concert');   // its (slow) search is not awaited
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
  // without the logo files (images not built, or a failed fetch) the button and tiles show the name on a badge
  const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', r => r.abort()) });
  await until(p2, () => !document.querySelector('#scen-dd .slogo img')); await p2.click('#x-scen');
  const badges = await p2.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo')].map(l => (l.querySelector('img') ? 'img' : '') + l.querySelector('b')?.textContent).join(','));
  check('missing logos fall back to name badges', badges === 'Unity Cup,Unity Cup,Grand Concert,URA Finale,Trackblazer', badges);
  const sz = await p2.evaluate(() => new Set([...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); })).size);
  check('… and the tiles stay the same size', sz === 1);
  await p2.context().close();
}
