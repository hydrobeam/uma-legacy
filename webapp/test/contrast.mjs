// The contrast audit of the loop planner (Chromium: one engine is enough for colours), in light mode and then, switched
// by the header button, in dark mode (`dark` checks the switching itself): the page, the scenario picker, the rank popup,
// a card's popup, the Race schedule and its last tab, Affinity, Odds. The fast tier audits light mode only.
import { QUICK, open, settle, dialog, audit, contrastCheck, still } from './helpers.mjs';

export default async function contrast(b) {
  if (b.browserType().name() !== 'chromium') return;
  const pg = await open(b);
  await pg.locator('#roster .ut .up').nth(1).click();
  await pg.locator('#cards .ct').nth(1).click();
  await settle(pg);
  for (const theme of QUICK ? ['light'] : ['light', 'dark']) {
    if (theme === 'dark') await pg.click('#theme-btn');
    await still(pg); // the results' fade back from the searching dim (0.15 s), the theme's colour transitions
    const audits = [await audit(pg)];
    await pg.click('#x-scen');
    audits.push(await audit(pg));
    await pg.keyboard.press('Escape');
    await pg.click('#rank-q');
    audits.push(await audit(pg, (x) => /rtab|rfoot|gpop/.test(x)));
    await pg.keyboard.press('Escape');
    await pg.locator('.deck .dc').first().click();
    audits.push(await audit(pg, (x) => /cpop|sw|gtl|ctt/.test(x)));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click();
    await dialog(pg, 'sched-dlg');
    audits.push(await audit(pg));
    await pg.locator('#sched-dlg .stab').last().click();
    audits.push(await audit(pg));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click();
    await dialog(pg, 'why-dlg');
    audits.push(await audit(pg));
    await pg.keyboard.press('Escape');
    await pg.click('#results .loop .lacts button:has-text("Odds")'); // (a game-style popup, .gmdlg)
    await dialog(pg, 'odds-dlg');
    audits.push(await audit(pg));
    await pg.keyboard.press('Escape');
    contrastCheck(
      `contrast: ${theme} mode (page, scenario picker, rank popup, card popup, schedule, affinity, odds)`,
      audits,
    );
  }
  await pg.context().close();
}
