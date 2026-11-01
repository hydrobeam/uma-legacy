// Keyboard, by role: one check per ARIA pattern instance on both planner pages, each against the keyboard contract of
// its WAI-ARIA Authoring Practices pattern (https://www.w3.org/WAI/ARIA/apg/patterns/) as far as the page implements it.
// A split or a rewrite can drop a handler while the role stays; these fail then. Where the page falls short of the APG
// pattern, the check holds it to what it does, and the comment says what it lacks.
// No check here sends the solver a job beyond the pages' own first ones (this group replays recorded replies): the
// loop page's settings change with Auto-search off, and the career page's controls opened here change nothing.
import { expect } from 'playwright/test';
import { check, open, openEditor, closeEditor, careerPage } from './helpers.mjs';

// the focused element, for a failure's detail
const focusName = (pg) =>
  pg.evaluate(() => {
    const a = document.activeElement;
    return a ? `${a.tagName.toLowerCase()}#${a.id}.${a.className} "${(a.textContent || '').trim().slice(0, 30)}"` : '';
  });
const opens = (pg, id) =>
  pg.waitForSelector(`#${id}[open]`, { timeout: 3000 }).then(
    () => true,
    () => false,
  );
// dialog `id` is open (`open`), or closed (its close lands on the render after the click or key that closes it)
const isOpen = (pg, id, name, open = true) =>
  open
    ? expect.soft(pg.locator(`#${id}`), name).toHaveAttribute('open')
    : expect.soft(pg.locator(`#${id}`), name).not.toHaveAttribute('open');
// a click on `button`, then whether dialog `id` opened (a failed open fails the checks that need it, not the group)
const clickOpens = async (pg, button, id) => {
  await button.click();
  return opens(pg, id);
};

// Dialog (modal): Enter on its button opens it with the focus inside; Escape closes it and the focus returns to the
// button (`back`: the element it returns to, when another stands in for the button). For a dialog opened by
// showModal() both of the last two are the browser's: Escape fails when the dialog is not modal (Odds opened by show()
// failed it) or a page key handler takes the Escape first (a capture-phase keydown, say), and the focus returns to the button with the page's own close handler removed (Odds,
// Deck and the trainee's popup passed without theirs in Chromium), so that clause fails when a page handler moves the
// focus away (Deck's close focusing another button failed it), not when one is missing.
async function dialogKeys(pg, name, button, id, back = button) {
  const title = `keyboard, dialog ${name}`;
  await button.focus();
  await pg.keyboard.press('Enter');
  await isOpen(pg, id, `${title}: Enter on its button opens it`);
  await expect.soft(pg.locator(`#${id}:focus-within`), `${title}: … the focus inside`).toHaveCount(1);
  if ((await pg.locator(`#${id}`).getAttribute('open')) !== null) await pg.keyboard.press('Escape');
  await isOpen(pg, id, `${title}: Escape closes it`, false);
  await expect
    .soft(back, `${title}: … the focus back on ${back === button ? 'the button' : 'its opener'}`)
    .toBeFocused();
}
// Switch (a checkbox with role switch): Space toggles it, and the page takes the change (`took`: whether it did, given
// the new state)
async function switchKeys(pg, name, sw, took) {
  await sw.focus();
  const before = await sw.isChecked();
  await pg.keyboard.press('Space');
  await expect.soft(sw, `keyboard, switch ${name}: Space toggles it`).toBeChecked({ checked: !before });
  check(`keyboard, switch ${name}: … and the page takes the change`, await took(!before), `${before} -> ${!before}`);
}
// Radio group: the checked radio is the group's Tab stop; ArrowRight checks and focuses the next, ArrowLeft goes back
async function radioKeys(pg, name, group) {
  const title = `keyboard, radio group ${name}`,
    checked = group.locator('[role="radio"][aria-checked="true"]');
  await checked.focus();
  const firstText = await checked.innerText();
  await pg.keyboard.press('ArrowRight');
  await expect
    .soft(checked, `${title}: ArrowRight checks the next radio`)
    .not.toHaveText(firstText, { useInnerText: true });
  await expect.soft(checked, `${title}: … and focuses it`).toBeFocused();
  await pg.keyboard.press('ArrowLeft');
  await expect.soft(checked, `${title}: ArrowLeft checks the one before`).toHaveText(firstText, { useInnerText: true });
  await expect.soft(checked, `${title}: … and focuses it`).toBeFocused();
  await expect.soft(checked, `${title}: one radio checked`).toHaveCount(1);
}
// Menu button: Enter opens the menu with the focus on its checked item; `move` (ArrowDown, or ArrowRight in the scenario
// grid) moves the focus to another item; Escape closes the menu and returns the focus to the button
async function menuKeys(pg, name, button, menu, move = 'ArrowDown') {
  const title = `keyboard, menu ${name}`;
  await button.focus();
  await pg.keyboard.press('Enter');
  await expect.soft(menu, `${title}: Enter opens it`).toBeVisible();
  await expect.soft(menu.locator('[aria-checked="true"]'), `${title}: … on the checked item`).toBeFocused();
  const from = await focusName(pg);
  await pg.keyboard.press(move);
  check(
    `${title}: ${move} moves the focus to another item`,
    (await focusName(pg)) !== from && (await menu.evaluate((m) => m.contains(document.activeElement))),
    `from ${from} to ${await focusName(pg)}`,
  );
  await pg.keyboard.press('Escape');
  await expect.soft(menu, `${title}: Escape closes it`).toBeHidden();
  await expect.soft(button, `${title}: … back to the button`).toBeFocused();
}

export default async function keyboard(b) {
  // ---------- the loop planner
  const pg = await open(b);
  await pg.keyboard.press('Tab');
  const skip = await pg.evaluate(() => {
    const a = document.activeElement,
      r = a.getBoundingClientRect();
    return { cls: a.className, top: r.top, text: a.textContent };
  });
  await pg.keyboard.press('Enter');
  check(
    'the first Tab stop is a visible "Skip to results" link',
    skip.cls === 'skip' && skip.top >= 0 && skip.text === 'Skip to results',
    JSON.stringify(skip),
  );
  await expect.soft(pg.locator('#rcol'), '… that moves the focus to the results').toBeFocused();
  // the deck card panel (a non-modal dialog): focus moves in on open; Tab past its end returns to the card
  await pg.locator('.deck .dc').first().focus();
  await pg.keyboard.press('Enter');
  const inPop = await pg.evaluate(
    () =>
      document.querySelector('.cpop').contains(document.activeElement) &&
      document.activeElement.getAttribute('role') === 'switch',
  );
  await pg.keyboard.press('Tab');
  await pg.keyboard.press('Tab');
  check('deck card panel: opening focuses its Owned switch', inPop);
  await expect.soft(pg.locator('.cpop').first(), 'deck card panel: tabbing out closes it').toBeHidden();
  await expect.soft(pg.locator('.dc:focus'), '… and returns to the card').toHaveCount(1);
  await pg.locator('.mem .memb').first().focus();
  await pg.keyboard.press('Enter');
  const memIn = await pg.evaluate(() => document.activeElement.getAttribute('role') === 'switch');
  await pg.keyboard.press('Shift+Tab');
  check('member panel: the same (opening focuses its Owned switch)', memIn);
  await expect.soft(pg.locator('.cpop').first(), 'member panel: Shift+Tab out closes it').toBeHidden();
  await expect.soft(pg.locator('.memb:focus'), '… and returns to her name').toHaveCount(1);
  // Tooltip (gmtip.ts): a control with a title shows it as the game's balloon on keyboard focus, and hides it when the
  // focus leaves it (APG: Escape would hide it too; the page does not). The focus leaves for nothing (a blur): moved to
  // another titled control, the balloon would show that one's title, and a check for this title gone would pass with
  // the hiding removed
  const auto = pg.getByRole('switch', { name: 'Auto-search' });
  await auto.focus();
  await pg.keyboard.press('Shift+Tab'); // (out and back by the keyboard: a focus by script may not be focus-visible)
  await pg.keyboard.press('Tab');
  const tipSeen = await pg.evaluate(() => {
    const t = document.querySelector('.gmtip'),
      a = document.activeElement;
    return `${a?.id} fv ${a?.matches(':focus-visible')} tip ${!!t} hidden ${t?.hidden} "${t?.textContent}"`;
  });
  const tip = / tip true hidden false "Automatically search/.test(tipSeen);
  await pg.evaluate(() => /** @type {HTMLElement} */ (document.activeElement).blur());
  check('keyboard, tooltip: keyboard focus on a titled control shows its balloon', tip, tipSeen);
  await expect.soft(pg.locator('.gmtip'), 'keyboard, tooltip: moving the focus away hides it').toBeHidden();
  // Auto-search off first, so the settings changed below send no job (the status line asks for a Search instead)
  const stored = () => pg.evaluate(() => localStorage.getItem('llp:settings') || '');
  await switchKeys(pg, 'Auto-search', auto, async (on) => !on && /"auto":false/.test(await stored()));
  await switchKeys(pg, 'Deck per uma', pg.getByRole('switch', { name: 'Deck per uma' }), async (on) =>
    pg.evaluate((on) => document.getElementById('rhead').dataset.kind === 'pending' && !on, on),
  );
  // Status line (role status): its Search button keeps the focus while further edits keep it the same button
  const search = pg.locator('#r-act button');
  const pending = await search.waitFor({ timeout: 3000 }).then(
    () => true,
    () => false,
  );
  if (pending) await search.focus();
  await pg.evaluate(() => {
    window.__actButton = document.activeElement;
    document.getElementById('c-peruma').click(); // an edit made elsewhere, the focus left where it is
  });
  const line = 'keyboard, status line: a pending change shows Search beside it (role status)';
  await expect.soft(pg.locator('#run-note'), line).toHaveAttribute('role', 'status');
  await expect.soft(search, `${line}: the button`).toHaveText('Search', { useInnerText: true });
  check(
    'keyboard, status line: … and another edit keeps the focus on that button',
    pending &&
      (await pg.evaluate(() => document.activeElement === window.__actButton && window.__actButton.isConnected)),
    await focusName(pg),
  );
  await radioKeys(pg, 'Results (1 or 3)', pg.locator('#c-top'));
  await radioKeys(pg, 'Rank', pg.locator('#rank'));
  // the sort menus are in the Umas and Support cards dialogs
  await openEditor(pg, 'umas');
  await menuKeys(pg, 'Sort umas', pg.locator('#rs'), pg.locator('#rs-menu'));
  await closeEditor(pg, 'umas');
  await openEditor(pg, 'cards');
  await menuKeys(pg, 'Sort cards', pg.locator('#cs'), pg.locator('#cs-menu'));
  await closeEditor(pg, 'cards');
  await menuKeys(pg, 'Scenario', pg.locator('#x-scen'), pg.locator('#scen-menu'), 'ArrowRight');
  await dialogKeys(pg, 'Umas', pg.locator('#o-umas'), 'set-umas');
  await dialogKeys(pg, 'Support cards', pg.locator('#o-cards'), 'set-cards');
  await dialogKeys(pg, 'Presets', pg.locator('#t-preset'), 'preset-dlg');
  // (a dialog over a dialog: Escape closes the top one only)
  const presets = await clickOpens(pg, pg.locator('#t-preset'), 'preset-dlg');
  await dialogKeys(pg, 'Save current skills (over Presets)', pg.locator('#pr-save'), 'pr-new');
  check('keyboard, dialog over a dialog: Presets opened', presets);
  await isOpen(pg, 'preset-dlg', 'keyboard, dialog over a dialog: Escape left Presets open');
  await pg.keyboard.press('Escape');
  await dialogKeys(pg, 'Export / import settings', pg.locator('#ex-open'), 'ex-dlg');
  await dialogKeys(pg, 'Odds', pg.locator('.loop .lacts button', { hasText: 'Odds' }).first(), 'odds-dlg');
  await dialogKeys(
    pg,
    'Race schedule',
    pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first(),
    'sched-dlg',
  );
  // Tabs (the schedule's members): ArrowRight selects and focuses the next tab; only the selected one is a Tab stop
  const sched = await clickOpens(
    pg,
    pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first(),
    'sched-dlg',
  );
  if (sched) await pg.locator('#sched-dlg').getByRole('tab').first().focus();
  await pg.keyboard.press('ArrowRight');
  const tabs = await pg.evaluate(() =>
    [...document.querySelectorAll('#sched-dlg [role="tab"]')].map((t) =>
      [t.getAttribute('aria-selected'), t.tabIndex, t.getAttribute('aria-controls')].join(':'),
    ),
  );
  check(
    'schedule tabs: ArrowRight selects the next tab, one tab in the Tab order, each controls the panel',
    sched && tabs[1] === 'true:0:sched-view' && tabs.filter((t) => t.startsWith('true')).length === 1,
    tabs.join(' '),
  );
  await expect.soft(pg.locator('#sched-dlg').getByRole('tab').nth(1), 'schedule tabs: … and focuses it').toBeFocused();
  await pg.keyboard.press('Escape');
  await dialogKeys(pg, 'Affinity', pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first(), 'why-dlg');
  // affinity: the careers are a radio group of pills; the arrows move and show one
  const why = await clickOpens(pg, pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first(), 'why-dlg');
  const cap0 = why ? await pg.locator('#why-dlg table caption').innerText() : '';
  if (why) await pg.locator('#why-dlg [aria-label="Trainee"] [aria-checked="true"]').focus();
  await pg.keyboard.press('ArrowRight');
  check('affinity: the dialog opened', why);
  await expect
    .soft(
      pg.locator('#why-dlg [role="radio"][aria-checked="true"]:focus'),
      'affinity: the careers are pill switches, the arrow moves the focus',
    )
    .toHaveCount(1);
  await expect
    .soft(pg.locator('#why-dlg table caption'), 'affinity: … an arrow key shows the next one')
    .not.toHaveText(cap0, { useInnerText: true });
  await pg.keyboard.press('Escape');
  check('no page errors (the loop planner)', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();

  // ---------- the career planner, with the umadump test file's 12 veterans
  const career = await careerPage(b);
  const cp = career.pg;
  await career.importDump();
  await career.done();
  await dialogKeys(cp, 'Veteran Umamusume', cp.locator('#v-toggle'), 'set-vets');
  const vets = await clickOpens(cp, cp.locator('#v-toggle'), 'set-vets');
  // Tabs (yours or guests): Enter selects a tab (APG: the arrows would move between the two, one Tab stop; here each is
  // a Tab stop and only Enter or Space selects)
  await cp.locator('#v-tab-bor').focus();
  await cp.keyboard.press('Enter');
  const guests = (await cp.getAttribute('#v-tab-bor', 'aria-selected')) === 'true';
  await cp.locator('#v-tab-own').focus();
  await cp.keyboard.press('Enter');
  const vtabs = 'keyboard, tabs Veteran Umamusume / Guests: Enter selects the focused tab';
  check(`${vtabs} (the dialog open, Guests selected first)`, vets && guests);
  await expect.soft(cp.locator('#v-tab-own'), vtabs).toHaveAttribute('aria-selected', 'true');
  await expect
    .soft(cp.locator('#v-tab-bor'), `${vtabs}, the other unselected`)
    .toHaveAttribute('aria-selected', 'false');
  await expect.soft(cp.locator('#v-tab-own'), `${vtabs}: … the focus stays on it`).toBeFocused();
  await menuKeys(cp, 'Sort veterans', cp.locator('#vs'), cp.locator('#vs-menu'));
  await isOpen(cp, 'set-vets', 'keyboard, a menu in a dialog: its Escape left Veteran Umamusume open');
  await dialogKeys(cp, 'Paste (over Veteran Umamusume)', cp.locator('#v-paste'), 'paste-dlg');
  await cp.keyboard.press('Escape');
  await dialogKeys(cp, 'Export veterans', cp.locator('#v-export'), 'vx-dlg');
  await dialogKeys(cp, 'uma.moe', cp.locator('#um-open'), 'um-dlg');
  await dialogKeys(cp, "the trainee's popup", cp.locator('#tree [data-k="node:t"]'), 'node-dlg');
  await dialogKeys(cp, 'Deck', cp.locator('#deck-open'), 'deck-dlg');
  await dialogKeys(
    cp,
    'Race schedule (career)',
    cp.getByRole('button', { name: 'Race schedule' }).first(),
    'sched-dlg',
  );
  await dialogKeys(
    cp,
    'a target skill',
    cp.getByRole('button', { name: /: lands .*Where it comes from/ }).first(),
    'sk-dlg',
  );
  // the parent picker, from a parent's popup (which closes first): its tabs as Veteran Umamusume's; Escape closes it
  const parent = await clickOpens(cp, cp.locator('#tree [data-k="node:p0"]'), 'node-dlg');
  const picker =
    parent &&
    (await clickOpens(
      cp,
      cp.locator('#node-dlg').getByRole('button', { name: /^(Change|Choose) parent$/ }),
      'vpick-dlg',
    ));
  if (picker) await cp.locator('#vp-tab-new').focus();
  await cp.keyboard.press('Enter');
  const pickNew =
    (await cp.getAttribute('#vp-tab-new', 'aria-selected')) === 'true' &&
    (await cp.getAttribute('#vp-tab-vets', 'aria-selected')) === 'false';
  await cp.keyboard.press('Escape');
  check('keyboard, tabs in the parent picker: Enter selects the focused tab', picker && pickNew);
  await isOpen(cp, 'vpick-dlg', 'keyboard, tabs in the parent picker: Escape closes the picker', false);
  // ---------- the focus comes back to what opened a popup (review-8c §6): by the keyboard, opened then closed
  // the node popup, closed by its Close button: back on the tree's parent place it opened from
  const place = cp.locator('#tree [data-k="node:p0"]');
  await place.focus();
  await cp.keyboard.press('Enter');
  const nodeIn = await opens(cp, 'node-dlg');
  await cp.locator('#node-dlg').getByRole('button', { name: 'Close', exact: true }).click();
  const byClose = 'focus: the node popup, closed by Close';
  check(`${byClose} (opened by Enter)`, nodeIn);
  await isOpen(cp, 'node-dlg', byClose, false);
  await expect.soft(place, `${byClose}, gives the focus back to the tree place that opened it`).toBeFocused();
  // the parent picker, from that place's popup (which closes as the picker opens): Escape gives it back to the place
  await place.focus();
  await cp.keyboard.press('Enter');
  const choose =
    (await opens(cp, 'node-dlg')) && cp.locator('#node-dlg').getByRole('button', { name: /^(Change|Choose) parent$/ });
  if (choose) await choose.focus();
  await cp.keyboard.press('Enter');
  const pickIn = !!choose && (await opens(cp, 'vpick-dlg'));
  await cp.keyboard.press('Escape');
  const pickEsc = 'focus: the parent picker opened from a node popup, closed by Escape';
  check(`${pickEsc} (it opened)`, pickIn);
  await isOpen(cp, 'vpick-dlg', pickEsc, false);
  await expect.soft(place, `${pickEsc}, gives the focus back to the tree place`).toBeFocused();
  // in Veteran Umamusume's pane: the colour picker from her blue bar, and the "+" list from her sparks' "+"
  const vetsIn = await clickOpens(cp, cp.locator('#v-toggle'), 'set-vets');
  const bar = cp.locator('#v-pane .vsb.blue button.vskb').first(),
    barKey = vetsIn ? await bar.getAttribute('data-k') : null;
  if (barKey) await bar.focus();
  await cp.keyboard.press('Enter');
  const colourIn = !!barKey && (await opens(cp, 'col-dlg'));
  await cp.keyboard.press('Escape');
  const colourEsc = 'focus: the colour picker, closed by Escape';
  check(`${colourEsc} (it opened)`, colourIn);
  await isOpen(cp, 'col-dlg', colourEsc, false);
  await expect
    .soft(cp.locator(`[data-k="${barKey}"]`), `${colourEsc}, gives the focus back to the spark bar that opened it`)
    .toBeFocused();
  await isOpen(cp, 'set-vets', `${colourEsc}: Veteran Umamusume stays open`);
  const plus = cp.locator('#v-pane .vskadd').first(),
    plusKey = vetsIn ? await plus.getAttribute('data-k') : null;
  if (plusKey) await plus.focus();
  await cp.keyboard.press('Enter');
  const listEsc = 'focus: the "+" list, closed by Escape';
  await expect.soft(cp.locator('#add-pop:focus-within'), `${listEsc} (it opened, the focus in it)`).toHaveCount(1);
  await cp.keyboard.press('Escape');
  await expect.soft(cp.locator('#add-pop'), listEsc).toBeHidden();
  await expect.soft(cp.locator(`[data-k="${plusKey}"]`), `${listEsc}, gives the focus back to its "+"`).toBeFocused();
  await isOpen(cp, 'set-vets', `${listEsc}: Veteran Umamusume stays open`);
  check('no page errors (the career planner)', !cp.errors.length, cp.errors.join(' | '));
  await career.ctx.close();
}
