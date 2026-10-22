// three results per uma: the four-uma loops behind the rotation are drawn with four members, the "All four" tab and
// the 12 careers
import { check, open, settle, dialog } from './helpers.mjs';

export default async function fourloop(b) {
  const pg = await open(b);
  await pg.click('#c-top [data-v="3"]'); await settle(pg);
  const sizes = await pg.evaluate(() => [...document.querySelectorAll('#results .loop')].map(l => l.querySelectorAll('.mem').length));
  check('three results: a rotation first, then four-uma loops', sizes.length === 3 && sizes[0] === 3 && sizes.slice(1).every(n => n === 4 || n === 0), JSON.stringify(sizes));
  const i = sizes.indexOf(4), loop = pg.locator('#results .loop').nth(i);
  check('a four-uma loop: four members in four columns', i > 0 && await loop.evaluate((l) => { const m = l.querySelector('.members'); return !m.classList.contains('n3') && getComputedStyle(m).gridTemplateColumns.split(' ').length === 4; }));
  await loop.locator('.lacts button', { hasText: 'Race schedule' }).click(); await dialog(pg, 'sched-dlg');
  check('its schedule: a tab per member plus "All four"', (await pg.locator('#sched-dlg .stab').count()) === 5 && (await pg.locator('#sched-dlg .stab').last().innerText()) === 'All four');
  await pg.locator('#sched-dlg .stab').last().click();
  check('All four: a column per member', (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 4);
  await pg.keyboard.press('Escape');
  await loop.locator('.lacts button', { hasText: 'Affinity' }).click(); await dialog(pg, 'why-dlg');
  // a four-uma loop's 12 careers: one pill per trainee (4), then her parents (3 pairs), never the same uma twice in a row
  const afT = await pg.evaluate(() => [...document.querySelectorAll('#why-dlg .afpick:not(.afpar) button')].map(b => b.textContent.replace(/\d+/g, '').trim()));
  check('its affinity: one pill per trainee, then her three parent pairs; six legacies each, no grandparent is the trainee', afT.length === 4 && new Set(afT).size === 4
    && await pg.locator('#why-dlg .afpar button').count() === 3 && await pg.locator('#why-dlg .afftab tbody tr').count() === 6
    && !(await pg.locator('#why-dlg .afftab .aft').count()), afT.join(', '));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
