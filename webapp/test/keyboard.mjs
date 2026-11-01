// Keyboard, by role: one check per ARIA pattern instance on both planner pages, each against the keyboard contract of
// its WAI-ARIA Authoring Practices pattern (https://www.w3.org/WAI/ARIA/apg/patterns/) as far as the page implements it.
// A split or a rewrite can drop a handler while the role stays; these fail then. Where the page falls short of the APG
// pattern, the check holds it to what it does, and the comment says what it lacks.
// No check here sends the solver a job beyond the pages' own first ones (this group replays recorded replies): the
// loop page's settings change with Auto-search off, and the career page's controls opened here change nothing.
import { check, open, careerPage } from './helpers.mjs';

// whether the focused element matches `selector`
const focusOn = (pg, selector) => pg.evaluate((s) => !!document.activeElement?.matches(s), selector);
// the focused element, for a failure's detail
const focusName = (pg) =>
  pg.evaluate(() => {
    const a = document.activeElement;
    return a ? `${a.tagName.toLowerCase()}#${a.id}.${a.className} "${(a.textContent || '').trim().slice(0, 30)}"` : '';
  });
const isOpen = (pg, id) => pg.evaluate((id) => !!document.getElementById(id)?.open, id);
const opens = (pg, id) =>
  pg.waitForSelector(`#${id}[open]`, { timeout: 3000 }).then(
    () => true,
    () => false,
  );
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
  await button.focus();
  await pg.keyboard.press('Enter');
  const opened = await opens(pg, id);
  const inside =
    opened && (await pg.evaluate((id) => document.getElementById(id).contains(document.activeElement), id));
  if (opened) await pg.keyboard.press('Escape');
  const shut = !(await isOpen(pg, id));
  const returned = await back.evaluate((e) => e === document.activeElement).catch(() => false);
  check(
    `keyboard, dialog ${name}: Enter on its button opens it, the focus inside; Escape closes it, the focus back on ${back === button ? 'the button' : 'its opener'}`,
    opened && inside && shut && returned,
    `opened ${opened}, focus inside ${inside}, closed ${shut}, then on ${await focusName(pg)}`,
  );
}
// Switch (a checkbox with role switch): Space toggles it, and the page takes the change (`took`: whether it did, given
// the new state)
async function switchKeys(pg, name, sw, took) {
  await sw.focus();
  const before = await sw.isChecked();
  await pg.keyboard.press('Space');
  const after = await sw.isChecked();
  check(
    `keyboard, switch ${name}: Space toggles it and the page takes the change`,
    after === !before && (await took(after)),
    `${before} -> ${after}`,
  );
}
// Radio group: the checked radio is the group's Tab stop; ArrowRight checks and focuses the next, ArrowLeft goes back
async function radioKeys(pg, name, group) {
  const checked = () => group.locator('[role="radio"][aria-checked="true"]');
  await checked().focus();
  const first = await checked()
    .getAttribute('data-v')
    .catch(() => null);
  const firstText = await checked().innerText();
  await pg.keyboard.press('ArrowRight');
  const movedTo = await checked().innerText();
  const moved = movedTo !== firstText && (await checked().evaluate((e) => e === document.activeElement));
  await pg.keyboard.press('ArrowLeft');
  const backAgain =
    (await checked().innerText()) === firstText && (await checked().evaluate((e) => e === document.activeElement));
  check(
    `keyboard, radio group ${name}: ArrowRight checks and focuses the next radio, ArrowLeft the one before`,
    moved && backAgain && (await group.locator('[role="radio"][aria-checked="true"]').count()) === 1,
    `${first ?? firstText} -> ${movedTo}, back: ${backAgain}`,
  );
}
// Menu button: Enter opens the menu with the focus on its checked item; `move` (ArrowDown, or ArrowRight in the scenario
// grid) moves the focus to another item; Escape closes the menu and returns the focus to the button
async function menuKeys(pg, name, button, menu, move = 'ArrowDown') {
  await button.focus();
  await pg.keyboard.press('Enter');
  const shown = await menu.isVisible();
  const onChecked = await pg.evaluate(() => document.activeElement?.getAttribute('aria-checked') === 'true');
  const from = await focusName(pg);
  await pg.keyboard.press(move);
  const moved = (await focusName(pg)) !== from && (await menu.evaluate((m) => m.contains(document.activeElement)));
  await pg.keyboard.press('Escape');
  check(
    `keyboard, menu ${name}: Enter opens it on the checked item, ${move} moves, Escape closes it back to the button`,
    shown &&
      onChecked &&
      moved &&
      (await menu.isHidden()) &&
      (await button.evaluate((e) => e === document.activeElement)),
    `shown ${shown}, on the checked item ${onChecked}, moved ${moved}, then on ${await focusName(pg)}`,
  );
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
    'the first Tab stop is a visible "Skip to results" link that moves the focus to the results',
    skip.cls === 'skip' && skip.top >= 0 && skip.text === 'Skip to results' && (await focusOn(pg, '#rcol')),
    JSON.stringify(skip),
  );
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
  check(
    'deck card panel: opening focuses its Owned switch; tabbing out closes it and returns to the card',
    inPop &&
      (await pg.evaluate(
        () => document.querySelector('.cpop').hidden && document.activeElement.classList.contains('dc'),
      )),
  );
  await pg.locator('.mem .memb').first().focus();
  await pg.keyboard.press('Enter');
  const memIn = await pg.evaluate(() => document.activeElement.getAttribute('role') === 'switch');
  await pg.keyboard.press('Shift+Tab');
  check(
    'member panel: the same (Shift+Tab out returns to her name)',
    memIn &&
      (await pg.evaluate(
        () => document.querySelector('.cpop').hidden && document.activeElement.classList.contains('memb'),
      )),
  );
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
  check(
    'keyboard, tooltip: keyboard focus on a titled control shows its balloon; moving the focus away hides it',
    tip && (await pg.evaluate(() => document.querySelector('.gmtip').hidden)),
    tipSeen,
  );
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
  check(
    'keyboard, status line: a pending change shows Search beside it (role status), and another edit keeps the focus on that button',
    pending &&
      (await pg.getAttribute('#run-note', 'role')) === 'status' &&
      (await search.innerText()) === 'Search' &&
      (await pg.evaluate(() => document.activeElement === window.__actButton && window.__actButton.isConnected)),
    await focusName(pg),
  );
  await radioKeys(pg, 'Results (1 or 3)', pg.locator('#c-top'));
  await radioKeys(pg, 'Rank', pg.locator('#rank'));
  await menuKeys(pg, 'Sort umas', pg.locator('#rs'), pg.locator('#rs-menu'));
  await menuKeys(pg, 'Sort cards', pg.locator('#cs'), pg.locator('#cs-menu'));
  await menuKeys(pg, 'Scenario', pg.locator('#x-scen'), pg.locator('#scen-menu'), 'ArrowRight');
  await dialogKeys(pg, 'Umas', pg.locator('#o-umas'), 'set-umas');
  await dialogKeys(pg, 'Support cards', pg.locator('#o-cards'), 'set-cards');
  await dialogKeys(pg, 'Presets', pg.locator('#t-preset'), 'preset-dlg');
  // (a dialog over a dialog: Escape closes the top one only)
  const presets = await clickOpens(pg, pg.locator('#t-preset'), 'preset-dlg');
  await dialogKeys(pg, 'Save current skills (over Presets)', pg.locator('#pr-save'), 'pr-new');
  check('keyboard, dialog over a dialog: Escape left Presets open', presets && (await isOpen(pg, 'preset-dlg')));
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
  if (sched) await pg.locator('#sched-dlg .stab').first().focus();
  await pg.keyboard.press('ArrowRight');
  const tabs = await pg.evaluate(() =>
    [...document.querySelectorAll('#sched-dlg .stab')].map((t) =>
      [t.getAttribute('aria-selected'), t.tabIndex, t.getAttribute('aria-controls')].join(':'),
    ),
  );
  check(
    'schedule tabs: ArrowRight selects and focuses the next tab, one tab in the Tab order, each controls the panel',
    sched &&
      tabs[1] === 'true:0:sched-view' &&
      tabs.filter((t) => t.startsWith('true')).length === 1 &&
      (await pg.evaluate(() => document.activeElement === document.querySelectorAll('#sched-dlg .stab')[1])),
    tabs.join(' '),
  );
  await pg.keyboard.press('Escape');
  await dialogKeys(pg, 'Affinity', pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first(), 'why-dlg');
  // affinity: the careers are a radio group of pills; the arrows move and show one
  const why = await clickOpens(pg, pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first(), 'why-dlg');
  const cap0 = why ? await pg.locator('#why-dlg .afftab caption').innerText() : '';
  if (why) await pg.locator('#why-dlg .afpick:not(.afpar) [aria-checked="true"]').focus();
  await pg.keyboard.press('ArrowRight');
  check(
    'affinity: the careers are pill switches; an arrow key shows the next one',
    why &&
      (await pg.evaluate(
        () =>
          document.activeElement.getAttribute('aria-checked') === 'true' &&
          document.activeElement.closest('.afpick') !== null,
      )) &&
      (await pg.locator('#why-dlg .afftab caption').innerText()) !== cap0,
  );
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
  check(
    'keyboard, tabs Veteran Umamusume / Guests: Enter selects the focused tab, the other unselected',
    vets &&
      guests &&
      (await cp.getAttribute('#v-tab-own', 'aria-selected')) === 'true' &&
      (await cp.getAttribute('#v-tab-bor', 'aria-selected')) === 'false' &&
      (await focusOn(cp, '#v-tab-own')),
  );
  await menuKeys(cp, 'Sort veterans', cp.locator('#vs'), cp.locator('#vs-menu'));
  check('keyboard, a menu in a dialog: its Escape left Veteran Umamusume open', vets && (await isOpen(cp, 'set-vets')));
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
  check(
    'keyboard, tabs in the parent picker: Enter selects the focused tab; Escape closes the picker',
    picker && pickNew && !(await isOpen(cp, 'vpick-dlg')),
  );
  check('no page errors (the career planner)', !cp.errors.length, cp.errors.join(' | '));
  await career.ctx.close();
}
