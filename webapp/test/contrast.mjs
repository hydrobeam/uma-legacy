import { check, artless, open, settle, dialog, AUDIT, still } from './helpers.mjs';

export default async function contrast(b) {
  if (b.browserType().name() !== 'chromium') return;   // one engine is enough for colours
  const pg = await open(b);
  await pg.locator('#roster .ut .up').nth(1).click(); await pg.locator('#cards .ct').nth(1).click(); await settle(pg);
  await still(pg);   // the results' fade back from the searching dim (0.15 s)
  const fails = [...await pg.evaluate(AUDIT)];
  await pg.click('#x-scen'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
  await pg.locator('.deck .dc').first().click(); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /cpop|sw|gtl|ctt/.test(x))); await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg'); fails.push(...await pg.evaluate(AUDIT));
  await pg.locator('#sched-dlg .stab').last().click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg'); fails.push(...await pg.evaluate(AUDIT));
  const shown = artless(fails); check('contrast: every text run meets WCAG AAA', !shown.length, shown.slice(0, 5).join(' | '));
  await pg.context().close();
  const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', r => r.abort()) });   // the first search done
  await still(p2);   // the results' fade back from the searching dim, as above
  await p2.click('#x-scen'); const bf = (await p2.evaluate(AUDIT)).filter(x => /^b\./.test(x) || /scen|stile|slogo|snm|ssub/.test(x));
  check('contrast: scenario badges (no logo files) meet WCAG AAA', !bf.length, bf.join(' | '));
  await p2.context().close();
}
