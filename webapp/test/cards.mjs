import { check, open, settle, pickScen } from './helpers.mjs';

export default async function cards(b) {
  const pg = await open(b);
  const secs = await pg.evaluate(() => [...document.querySelectorAll('#cards .cgroup h3')].map(h => h.textContent));
  check('card sections SSR, SR, R in that order', secs.join(',') === 'SSR,SR,R', secs.join(','));
  const notes = await pg.evaluate(() => [...document.querySelectorAll('#cards .gv')].map(g => [g.querySelector('small').textContent, g.title]));
  check('card hint notes in plain words, each with a tooltip', notes.length > 0 && notes.every(([n, t]) => /^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n) && t.length > 0), JSON.stringify(notes.filter(([n]) => !/^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n)).slice(0, 3)));
  const art = await pg.evaluate(() => { const s = new Set(); let over = 0; for (const t of document.querySelectorAll('#cards .ct')) { const c = t.querySelector('.cav').getBoundingClientRect();
    s.add(Math.round(c.width)); if (c.width > t.getBoundingClientRect().width + 0.5) over++; } return { sizes: [...s], over }; });
  check('card list: every card picture is the same size and inside its tile, loaded or not', art.sizes.length === 1 && !art.over, JSON.stringify(art));
  check('card names carry no rarity (the frame and section show it)', await pg.evaluate(() => ![...document.querySelectorAll('#cards .ct .cn')].some(e => / (SSR|SR|R)$/.test(e.textContent))));
  // the game bans Team Sirius from Unity Cup and Grand Concert decks: the picker says so and no result deck holds it
  const sirius = () => pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? t.querySelector('.cban')?.textContent || '' : null; });
  check('Team Sirius: "Not allowed in Unity Cup" in the card picker', (await sirius()) === 'Not allowed in Unity Cup', String(await sirius()));
  check('… and in no Unity Cup result deck', await pg.evaluate(() => ![...document.querySelectorAll('#results .dc')].some(d => d.dataset.cid === '30081')));
  await pickScen(pg, 'ura');   // the picker follows at once; the search is not awaited
  check('Team Sirius allowed again in URA Finale', (await sirius()) === '');
  await pickScen(pg, 'unity'); await settle(pg);
  await pg.editor('#cards');
  const sep = await pg.evaluate(() => { const c = document.getElementById('cards'), r = c.previousElementSibling.getBoundingClientRect(), cs = getComputedStyle(c);
    return { gap: Math.round(c.getBoundingClientRect().top - r.bottom), pad: parseFloat(cs.paddingTop) }; });
  check('the card list scrolls apart from its search row (gap like the roster\'s), its tiles\' rings and focus outline inside it', sep.gap >= 10 && sep.pad >= 5, JSON.stringify(sep));
  check('default card sort: release, newest first', (await pg.locator('#cs').innerText()) === 'Release ↓');
  const tile = pg.locator('#cards .ct').first(); const img = await tile.evaluate(t => { t.__img = t.querySelector('img'); return !!t.__img; });
  await tile.click();
  check('toggling a card keeps its image element (no flicker)', img && await tile.evaluate(t => t.querySelector('img') === t.__img && t.getAttribute('aria-pressed') === 'false'));
  await tile.click();
  // R cards are left out (no setting): the R section holds only the R pals
  const rpals = await pg.evaluate(() => self.__LLP_DATA['data-cards'].filter(c => c.rar === 1 && c.pal).length);
  const rCount = () => pg.evaluate(() => { const g = [...document.querySelectorAll('#cards .cgroup')].find(x => x.querySelector('h3').textContent === 'R'); return g ? g.querySelectorAll('.ct').length : 0; });
  check('R cards are left out, with no switch; the R section keeps the R pals', (await pg.locator('#x-rcards').count()) === 0 && (await rCount()) === rpals, (await rCount()) + ' of ' + rpals);
  check('every card image carries its type icon', (await pg.locator('#cards .ct .cav.art').count()) === (await pg.locator('#cards .ct .ctype').count()));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
