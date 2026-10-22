import { check, open, dialog } from './helpers.mjs';

export default async function keyboard(b) {
  const pg = await open(b);
  const act = () => pg.evaluate(() => { const a = document.activeElement; return a.id || a.className || a.tagName; });
  await pg.keyboard.press('Tab');
  const skip = await pg.evaluate(() => { const a = document.activeElement, r = a.getBoundingClientRect(); return { cls: a.className, top: r.top, text: a.textContent }; });
  await pg.keyboard.press('Enter');
  check('the first Tab stop is a visible "Skip to results" link that moves the focus to the results', skip.cls === 'skip' && skip.top >= 0 && skip.text === 'Skip to results' && (await act()) === 'rcol', JSON.stringify(skip));
  // the deck card panel: focus moves in on open; Tab past its end returns to the card
  await pg.locator('.deck .dc').first().focus(); await pg.keyboard.press('Enter');
  const inPop = await pg.evaluate(() => document.querySelector('.cpop').contains(document.activeElement) && document.activeElement.getAttribute('role') === 'switch');
  await pg.keyboard.press('Tab'); await pg.keyboard.press('Tab');
  check('deck card panel: opening focuses its Owned switch; tabbing out closes it and returns to the card', inPop && await pg.evaluate(() => document.querySelector('.cpop').hidden && document.activeElement.classList.contains('dc')));
  await pg.locator('.mem .memb').first().focus(); await pg.keyboard.press('Enter');
  const memIn = await pg.evaluate(() => document.activeElement.getAttribute('role') === 'switch'); await pg.keyboard.press('Shift+Tab');
  check('member panel: the same (Shift+Tab out returns to her name)', memIn && await pg.evaluate(() => document.querySelector('.cpop').hidden && document.activeElement.classList.contains('memb')));
  // sort menu: the checked item takes the focus, arrows move, Escape returns
  await pg.focus('#rs'); await pg.keyboard.press('Enter');
  const checked = await pg.evaluate(() => document.activeElement.getAttribute('aria-checked'));
  await pg.keyboard.press('ArrowDown'); const moved = await pg.evaluate(() => document.activeElement.dataset.k);
  await pg.keyboard.press('Escape');
  check('sort menu: opens on the checked item, arrows move, Escape closes back to the button', checked === 'true' && moved === 'name' && await pg.locator('#rs-menu').isHidden() && (await act()) === 'rs', checked + ' ' + moved);
  // schedule tabs: arrows move and select
  await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
  await pg.locator('#sched-dlg .stab').first().focus(); await pg.keyboard.press('ArrowRight');
  const tabs = await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .stab')].map(t => [t.getAttribute('aria-selected'), t.tabIndex, t.getAttribute('aria-controls')].join(':')));
  check('schedule tabs: ArrowRight selects and focuses the next tab, one tab in the Tab order, each controls the panel', tabs[1] === 'true:0:sched-view' && tabs.filter(t => t.startsWith('true')).length === 1
    && await pg.evaluate(() => document.activeElement === document.querySelectorAll('#sched-dlg .stab')[1]), tabs.join(' '));
  await pg.keyboard.press('Escape');
  // affinity: the careers are a radio group of pills; the arrows move and show one
  await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
  const cap0 = await pg.locator('#why-dlg .afftab caption').innerText();
  await pg.locator('#why-dlg .afpick:not(.afpar) [aria-checked="true"]').focus(); await pg.keyboard.press('ArrowRight');
  check('affinity: the careers are pill switches; an arrow key shows the next one', await pg.evaluate(() => document.activeElement.getAttribute('aria-checked') === 'true' && document.activeElement.closest('.afpick') !== null)
    && (await pg.locator('#why-dlg .afftab caption').innerText()) !== cap0);
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
