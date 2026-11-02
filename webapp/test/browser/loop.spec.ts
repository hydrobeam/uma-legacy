// What needs a real browser on the planner pages: the keyboard and focus through each ARIA pattern (against the
// WAI-ARIA Authoring Practices pattern, https://www.w3.org/WAI/ARIA/apg/patterns/, as far as the page implements it:
// where it falls short, the check holds it to what it does and says what it lacks), the page as served (its files,
// fonts, HTTP/2, the worker script), its layout at a desktop's width, its live region, and a reload that shows the saved
// result from the first frame. What the pages draw from a result and the run machine are node tests
// (test/unit/loopresults.test.tsx, loop.test.ts, engine.test.ts).
import type { Locator, Page } from 'playwright/test';
import { expect, test } from './fixtures.ts';
import { careerDone, closeEditor, loopSettled, openEditor, openLoop } from './pages.ts';
import { quiet } from './solver.ts';

// the loop page on loop-small (three umas, two targets, fourteen cards); the career page's flow on career-small
test.use({ scenario: 'loop-small' });

/** what this file's init scripts and steps keep on the page's window */
interface LiveRegion {
  node: Element | null;
  region: string;
  texts: string[];
}
declare global {
  interface Window {
    /** the status line's live region as watched from load (`watchPage`) */
    llpLive?: LiveRegion;
    /** the odds each frame drew for the first seconds after load */
    llpSeen?: string[];
    /** the status line's button the focus was on */
    llpActButton?: Element | null;
  }
}

/** the focused element, for a failure's message */
const focusName = (page: Page) =>
  page.evaluate(() => {
    const a = document.activeElement;
    return a ? `${a.tagName.toLowerCase()}#${a.id} "${(a.textContent ?? '').trim().slice(0, 30)}"` : '';
  });
const dialog = (page: Page, id: string) => page.locator(`#${id}`);

// Dialog (modal): Enter on its button opens it with the focus inside; Escape closes it and the focus returns to the
// button. For a dialog opened by showModal() both are the browser's: Escape fails when the dialog is not modal or a page
// key handler takes the Escape first, and the focus return fails when a page handler moves the focus away.
async function dialogKeys(page: Page, name: string, button: Locator, id: string) {
  await test.step(`dialog ${name}`, async () => {
    await button.focus();
    await page.keyboard.press('Enter');
    await expect(dialog(page, id), 'Enter on its button opens it').toHaveAttribute('open');
    await expect(page.locator(`#${id}:focus-within`), 'the focus inside').toHaveCount(1);
    await page.keyboard.press('Escape');
    await expect(dialog(page, id), 'Escape closes it').not.toHaveAttribute('open');
    await expect(button, 'the focus back on its button').toBeFocused();
  });
}
// Switch (a checkbox with role switch): Space toggles it
async function switchKeys(page: Page, name: string, sw: Locator) {
  await test.step(`switch ${name}`, async () => {
    await sw.focus();
    const before = await sw.isChecked();
    await page.keyboard.press('Space');
    await expect(sw, 'Space toggles it').toBeChecked({ checked: !before });
  });
}
// Radio group: the checked radio is the group's Tab stop; ArrowRight checks and focuses the next, ArrowLeft goes back
async function radioKeys(page: Page, name: string, group: Locator) {
  await test.step(`radio group ${name}`, async () => {
    const checked = group.locator('[role="radio"][aria-checked="true"]');
    await checked.focus();
    const first = await checked.innerText();
    await page.keyboard.press('ArrowRight');
    await expect(checked, 'ArrowRight checks the next radio').not.toHaveText(first, { useInnerText: true });
    await expect(checked, 'and focuses it').toBeFocused();
    await page.keyboard.press('ArrowLeft');
    await expect(checked, 'ArrowLeft checks the one before').toHaveText(first, { useInnerText: true });
    await expect(checked, 'and focuses it').toBeFocused();
    await expect(checked, 'one radio checked').toHaveCount(1);
  });
}
// Menu button: Enter opens the menu with the focus on its checked item; `move` moves the focus to another item; Escape
// closes the menu and returns the focus to the button
async function menuKeys(page: Page, name: string, button: Locator, menu: Locator, move = 'ArrowDown') {
  await test.step(`menu ${name}`, async () => {
    await button.focus();
    await page.keyboard.press('Enter');
    await expect(menu, 'Enter opens it').toBeVisible();
    await expect(menu.locator('[aria-checked="true"]'), 'on the checked item').toBeFocused();
    const from = await focusName(page);
    await page.keyboard.press(move);
    expect(await focusName(page), `${move} moves the focus to another item`).not.toBe(from);
    expect(await menu.evaluate((m) => m.contains(document.activeElement)), 'within the menu').toBe(true);
    await page.keyboard.press('Escape');
    await expect(menu, 'Escape closes it').toBeHidden();
    await expect(button, 'the focus back on the button').toBeFocused();
  });
}

test('loop planner: the keyboard through every ARIA pattern; Auto-search back on searches the change it held @both', async ({
  page,
}) => {
  await openLoop(page);
  const results = page.locator('#results');
  await test.step('the first Tab stop is a visible "Skip to results" link, to the results', async () => {
    await page.keyboard.press('Tab');
    const skip = await page.evaluate(() => {
      const a = document.activeElement;
      return { href: a?.getAttribute('href'), top: a?.getBoundingClientRect().top ?? -1, text: a?.textContent };
    });
    expect(skip.href).toMatch(/^#\w/);
    expect(skip.top).toBeGreaterThanOrEqual(0);
    expect(skip.text).toBe('Skip to results');
    await page.keyboard.press('Enter');
    await expect(page.locator('#rcol')).toBeFocused();
  });
  await test.step('a deck card panel and a member panel (non-modal): focus in on open, out by Tab closes them', async () => {
    const pop = page.locator('[data-pop="card"]').first();
    await results.locator('button[data-cid]').first().focus();
    await page.keyboard.press('Enter');
    await expect(pop.locator('[role="switch"]:focus'), 'opening focuses its Owned switch').toHaveCount(1);
    await page.keyboard.press('Tab');
    await page.keyboard.press('Tab');
    await expect(pop, 'tabbing out closes it').toBeHidden();
    await expect(results.locator('button[data-cid]:focus'), 'and returns to the card').toHaveCount(1);
    await results.locator('[data-member] button').first().focus();
    await page.keyboard.press('Enter');
    await expect(pop.locator('[role="switch"]:focus'), 'a member panel: the same').toHaveCount(1);
    await page.keyboard.press('Shift+Tab');
    await expect(pop, 'Shift+Tab out closes it').toBeHidden();
    await expect(results.locator('[data-member] button:focus'), 'and returns to her name').toHaveCount(1);
  });
  const auto = page.getByRole('switch', { name: 'Auto-search' });
  await test.step('tooltip: keyboard focus on a titled control shows its balloon; the focus leaving hides it', async () => {
    // (out and back by the keyboard: a focus by script may not be focus-visible). APG: Escape would hide it too; the
    // page does not. The focus leaves for nothing (a blur): moved to another titled control, the balloon would show
    // that one's title, and a check for this title gone would pass with the hiding removed
    await auto.focus();
    await page.keyboard.press('Shift+Tab');
    await page.keyboard.press('Tab');
    const tip = page.locator('#gm-tip');
    await expect(tip).toBeVisible();
    await expect(tip).toHaveText(/^Automatically search/);
    await expect(tip, "the game's balloon: its green rim").toHaveCSS('border-top-width', '2px');
    await expect(tip, 'its own width').not.toHaveCSS('right', '0px');
    await page.evaluate(() => {
      const a = document.activeElement;
      if (a instanceof HTMLElement) a.blur();
    });
    await expect(tip).toBeHidden();
  });
  // Auto-search off first, so the settings changed below send no job (the status line asks for a Search instead)
  await switchKeys(page, 'Auto-search', auto);
  await expect.poll(() => page.evaluate(() => localStorage.getItem('llp:settings') ?? '')).toMatch(/"auto":false/);
  const perUma = page.getByRole('switch', { name: 'Deck per uma' });
  await switchKeys(page, 'Deck per uma', perUma);
  await expect(page.locator('#rhead'), 'a change waits for Search').toHaveAttribute('data-kind', 'pending');
  await test.step('status line: a pending change shows Search beside it; another edit keeps the focus on it', async () => {
    const search = page.locator('#r-act button');
    await expect(page.locator('#run-note')).toHaveAttribute('role', 'status');
    await expect(search).toHaveText('Search', { useInnerText: true });
    await search.focus();
    await page.evaluate(() => {
      window.llpActButton = document.activeElement;
      document.getElementById('c-peruma')?.click(); // an edit made elsewhere, the focus left where it is
    });
    expect(
      await page.evaluate(() => {
        const held = window.llpActButton;
        return !!held && document.activeElement === held && held.isConnected;
      }),
      await focusName(page),
    ).toBe(true);
  });
  await radioKeys(page, 'Results (1 or 3)', page.locator('#c-top'));
  await radioKeys(page, 'Rank', page.locator('#rank'));
  await openEditor(page, 'umas');
  await menuKeys(page, 'Sort umas', page.locator('#rs'), page.locator('#rs-menu'));
  await closeEditor(page, 'umas');
  await openEditor(page, 'cards');
  await menuKeys(page, 'Sort cards', page.locator('#cs'), page.locator('#cs-menu'));
  await closeEditor(page, 'cards');
  await menuKeys(page, 'Scenario', page.locator('#x-scen'), page.locator('#scen-menu'), 'ArrowRight');
  await dialogKeys(page, 'Umas', page.locator('#o-umas'), 'set-umas');
  await dialogKeys(page, 'Support cards', page.locator('#o-cards'), 'set-cards');
  await dialogKeys(page, 'Presets', page.locator('#t-preset'), 'preset-dlg');
  await test.step('a dialog over a dialog: Escape closes the top one only', async () => {
    await page.locator('#t-preset').click();
    await expect(dialog(page, 'preset-dlg')).toHaveAttribute('open');
    await dialogKeys(page, 'Save current skills (over Presets)', page.locator('#pr-save'), 'pr-new');
    await expect(dialog(page, 'preset-dlg'), 'Escape left Presets open').toHaveAttribute('open');
    await page.keyboard.press('Escape');
  });
  await dialogKeys(page, 'Export / import settings', page.locator('#ex-open'), 'ex-dlg');
  const action = (name: string) => results.getByRole('button', { name, exact: true }).first();
  await dialogKeys(page, 'Odds', action('Odds'), 'odds-dlg');
  await dialogKeys(page, 'Race schedule', action('Race schedule'), 'sched-dlg');
  await test.step('schedule tabs: ArrowRight selects and focuses the next tab, one tab in the Tab order', async () => {
    await action('Race schedule').click();
    const tabs = dialog(page, 'sched-dlg').getByRole('tab');
    await tabs.first().focus();
    await page.keyboard.press('ArrowRight');
    const states = await tabs.evaluateAll((all) =>
      all.map((t) =>
        [
          t.getAttribute('aria-selected'),
          t instanceof HTMLElement ? t.tabIndex : '',
          t.getAttribute('aria-controls'),
        ].join(':'),
      ),
    );
    expect(states[1]).toBe('true:0:sched-view');
    expect(states.filter((t) => t.startsWith('true'))).toHaveLength(1);
    await expect(tabs.nth(1)).toBeFocused();
    await page.keyboard.press('Escape');
  });
  await dialogKeys(page, 'Affinity', action('Affinity'), 'why-dlg');
  await test.step('affinity: the careers are a radio group of pills; an arrow moves and shows the next', async () => {
    await action('Affinity').click();
    const caption = dialog(page, 'why-dlg').locator('table caption');
    const first = await caption.innerText();
    await dialog(page, 'why-dlg').locator('[aria-label="Trainee"] [aria-checked="true"]').focus();
    await page.keyboard.press('ArrowRight');
    await expect(dialog(page, 'why-dlg').locator('[role="radio"][aria-checked="true"]:focus')).toHaveCount(1);
    await expect(caption).not.toHaveText(first, { useInnerText: true });
    await page.keyboard.press('Escape');
  });
  await test.step('Auto-search back on searches the change it held', async () => {
    await perUma.click(); // (Deck per uma off: a change from the result on show)
    const sent = await page.evaluate(() => window.__runs.length);
    await auto.click();
    await expect.poll(() => page.evaluate(() => window.__runs.length)).toBe(sent + 1);
    await loopSettled(page);
    await expect(page.locator('#rhead')).toHaveAttribute('data-kind', 'done');
  });
});

test.describe('career planner @both', () => {
  test.use({ scenario: 'career-small', seed: { 'llp:setup-folded': '0' } });

  test("the keyboard through its dialogs, tabs and pickers; the focus back on each popup's opener", async ({
    page,
  }) => {
    await page.goto('/');
    await careerDone(page);
    const vets = dialog(page, 'set-vets');
    await dialogKeys(page, 'Veteran Umamusume', page.locator('#v-toggle'), 'set-vets');
    await test.step('tabs Veteran Umamusume / Guests: Enter selects the focused tab (APG: arrows, one Tab stop; here each is a stop)', async () => {
      await page.locator('#v-toggle').click();
      await expect(vets).toHaveAttribute('open');
      await page.locator('#v-tab-bor').focus();
      await page.keyboard.press('Enter');
      await expect(page.locator('#v-tab-bor')).toHaveAttribute('aria-selected', 'true');
      await page.locator('#v-tab-own').focus();
      await page.keyboard.press('Enter');
      await expect(page.locator('#v-tab-own')).toHaveAttribute('aria-selected', 'true');
      await expect(page.locator('#v-tab-bor')).toHaveAttribute('aria-selected', 'false');
      await expect(page.locator('#v-tab-own')).toBeFocused();
    });
    await menuKeys(page, 'Sort veterans', page.locator('#vs'), page.locator('#vs-menu'));
    await expect(vets, "a menu's Escape in a dialog leaves the dialog open").toHaveAttribute('open');
    await dialogKeys(page, 'Paste (over Veteran Umamusume)', page.locator('#v-paste'), 'paste-dlg');
    await page.keyboard.press('Escape');
    await dialogKeys(page, 'Export veterans', page.locator('#v-export'), 'vx-dlg');
    await dialogKeys(page, 'uma.moe', page.locator('#um-open'), 'um-dlg');
    await dialogKeys(page, "the trainee's popup", page.locator('#tree [data-k="node:t"]'), 'node-dlg');
    await dialogKeys(page, 'Deck', page.locator('#deck-open'), 'deck-dlg');
    await dialogKeys(page, 'Race schedule', page.getByRole('button', { name: 'Race schedule' }).first(), 'sched-dlg');
    await dialogKeys(
      page,
      'a target skill',
      page.getByRole('button', { name: /: lands .*Where it comes from/ }).first(),
      'sk-dlg',
    );
    const place = page.locator('#tree [data-k="node:p0"]');
    const choose = dialog(page, 'node-dlg').getByRole('button', { name: /^(Change|Choose) parent$/ });
    await test.step("the parent picker: its tabs as Veteran Umamusume's; Escape closes it, the focus back on the tree place", async () => {
      await place.focus();
      await page.keyboard.press('Enter');
      await expect(page.locator('#node-dlg:focus-within')).toHaveCount(1);
      await choose.focus();
      await page.keyboard.press('Enter');
      // (the node popup closes as the picker opens: the focus lands in the picker before a tab takes it)
      await expect(page.locator('#vpick-dlg:focus-within')).toHaveCount(1);
      await page.locator('#vp-tab-new').focus();
      await expect(page.locator('#vp-tab-new')).toBeFocused();
      await page.keyboard.press('Enter');
      await expect(page.locator('#vp-tab-new')).toHaveAttribute('aria-selected', 'true');
      await expect(page.locator('#vp-tab-vets')).toHaveAttribute('aria-selected', 'false');
      await page.keyboard.press('Escape');
      await expect(dialog(page, 'vpick-dlg')).not.toHaveAttribute('open');
      await expect(place).toBeFocused();
    });
    await test.step('the node popup closed by its Close: the focus back on the tree place that opened it', async () => {
      await place.focus();
      await page.keyboard.press('Enter');
      await expect(dialog(page, 'node-dlg')).toHaveAttribute('open');
      await dialog(page, 'node-dlg').getByRole('button', { name: 'Close', exact: true }).click();
      await expect(dialog(page, 'node-dlg')).not.toHaveAttribute('open');
      await expect(place).toBeFocused();
    });
    await test.step('in Veteran Umamusume\'s pane: the colour picker and the "+" list give the focus back on Escape', async () => {
      await page.locator('#v-toggle').click();
      await expect(vets).toHaveAttribute('open');
      const bar = page.locator('#v-pane [data-spark="blue"] button[data-bar]').first();
      const barKey = await bar.getAttribute('data-k');
      await bar.focus();
      await page.keyboard.press('Enter');
      await expect(dialog(page, 'col-dlg')).toHaveAttribute('open');
      await page.keyboard.press('Escape');
      await expect(dialog(page, 'col-dlg')).not.toHaveAttribute('open');
      await expect(page.locator(`[data-k="${barKey}"]`)).toBeFocused();
      await expect(vets, 'Veteran Umamusume stays open').toHaveAttribute('open');
      const plus = page.locator('#v-pane [data-k$=":add"]').first();
      const plusKey = await plus.getAttribute('data-k');
      await plus.focus();
      await page.keyboard.press('Enter');
      await expect(page.locator('#add-pop:focus-within'), 'the list opened, the focus in it').toHaveCount(1);
      await page.keyboard.press('Escape');
      await expect(page.locator('#add-pop')).toBeHidden();
      await expect(page.locator(`[data-k="${plusKey}"]`)).toBeFocused();
      await expect(vets, 'Veteran Umamusume stays open').toHaveAttribute('open');
    });
  });
});

// (runs in the page) the status line's live region (#run-note, role status): the node the parser made, its text then
// (empty: the region exists before its first message, so that message is a change a screen reader announces) and every
// text it shows after; and the odds each frame draws for the first seconds (a reload's first result)
function watchPage() {
  const live: LiveRegion = { node: null, region: '', texts: [] };
  const seen: string[] = [];
  window.llpLive = live;
  window.llpSeen = seen;
  new MutationObserver((_, watcher) => {
    const note = document.getElementById('run-note');
    if (!note) return;
    watcher.disconnect();
    Object.assign(live, { node: note, region: `${note.getAttribute('role')} ${note.getAttribute('aria-live')}` });
    live.texts.push(note.textContent ?? '');
    new MutationObserver(() => {
      if (live.texts.at(-1) !== note.textContent) live.texts.push(note.textContent ?? '');
    }).observe(note, { childList: true, characterData: true, subtree: true });
  }).observe(document, { childList: true, subtree: true });
  const snap = () => {
    seen.push(document.querySelector('#results [data-odds]')?.textContent ?? '');
    if (performance.now() < 6000) requestAnimationFrame(snap);
  };
  requestAnimationFrame(snap);
}
const liveRegion = (page: Page) =>
  page.evaluate(() => {
    const live = window.llpLive;
    return live
      ? { region: live.region, same: live.node === document.getElementById('run-note'), texts: live.texts }
      : null;
  });

test('the loop page as served, laid out and announced; a reload shows the saved result from the first frame, searching nothing @both', async ({
  page,
}) => {
  const hosts = new Set<string>();
  const urls: string[] = [];
  page.on('request', (r) => {
    const u = new URL(r.url());
    if (u.protocol.startsWith('http')) {
      hosts.add(u.host);
      urls.push(u.pathname);
    }
  });
  await page.addInitScript(watchPage);
  await openLoop(page);
  await test.step('served: no third-party request, HTTP/2, fingerprinted solver files and worker, self-hosted fonts, a favicon', async () => {
    expect([...hosts]).toHaveLength(1);
    const protocols = await page.evaluate(() => [
      ...new Set(
        performance
          .getEntriesByType('resource')
          .map((e) => (e instanceof PerformanceResourceTiming ? e.nextHopProtocol : '')),
      ),
    ]);
    expect(protocols.length).toBeGreaterThan(0);
    for (const p of protocols) expect(p).toBe('h2');
    expect(urls.some((u) => /^\/assets\/engine-global-[\w-]{8}\.json$/.test(u))).toBe(true);
    expect(urls.some((u) => /^\/assets\/[\w.]+-[\w-]{8}\.wasm$/.test(u))).toBe(true);
    // (Firefox reports the worker's URL as the page wrote it, relative)
    const workers = page.workers().map((w) => new URL(w.url(), page.url()).pathname);
    expect(workers).toHaveLength(1);
    expect(workers[0]).toMatch(/^\/assets\/worker-[\w-]{8}\.js$/);
    expect(
      await page.evaluate(async () => {
        await document.fonts.ready;
        return [...document.fonts].some((f) => f.family.includes('Zen Kaku') && f.status === 'loaded');
      }),
    ).toBe(true);
    const ico = await page.request.get('/favicon.ico');
    expect(ico.ok()).toBe(true);
    expect(ico.headers()['content-type']).toMatch(/image\//);
  });
  await test.step('names and loading: the first 8 roster portraits eager; sort buttons and card tiles named by their visible text', async () => {
    expect(
      await page.evaluate(() => {
        const im = [...document.querySelectorAll('#roster [data-av] img')].filter((i) => i instanceof HTMLImageElement);
        return (
          im.length > 8 &&
          im.slice(0, 8).every((i) => i.loading === 'eager') &&
          im.slice(8).every((i) => i.loading === 'lazy')
        );
      }),
    ).toBe(true);
    for (const id of ['rs', 'cs']) {
      const sort = page.locator(`#${id}`);
      expect(await sort.getAttribute('aria-label')).toMatch(new RegExp(`^${(await sort.textContent())?.trim()}`));
    }
    expect(
      await page.$$eval('#cards button[aria-pressed]', (all) => all.every((e) => !e.hasAttribute('aria-label'))),
    ).toBe(true);
  });
  await test.step('the status line is a live region from load: empty first, "Searching…" and the result are changes to it', async () => {
    const first = await liveRegion(page);
    expect(first?.region).toBe('status polite');
    expect(first?.same).toBe(true);
    expect(first?.texts[0]).toBe('');
    expect(first?.texts.some((t) => /^Searching…/.test(t))).toBe(true);
    expect(first?.texts.at(-1)).toMatch(/^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s( · .+)?$/);
  });
  await test.step('desktop: Loop setup beside the results, tops level, one 900 px screen; the status and switches in its foot', async () => {
    const layout = await page.evaluate(() => {
      const panel = document.querySelector('[aria-labelledby="h-set"]');
      const set = panel?.getBoundingClientRect();
      const res = document.querySelector('#results article')?.getBoundingClientRect();
      const members = document.querySelector('#results [data-members]');
      const odds = document.querySelector('#results [data-odds]');
      return {
        beside: !!set && !!res && set.right <= res.left && Math.abs(set.top - res.top) <= 1,
        fits: (panel?.parentElement?.getBoundingClientRect().bottom ?? Infinity) <= innerHeight + 60,
        inPanel: ['run-note', 'x-auto', 'c-peruma', 'rank'].every((id) => panel?.contains(document.getElementById(id))),
        countInColumn: !!document.getElementById('rcol')?.contains(document.getElementById('c-top')),
        perUmaSwitch: document.getElementById('c-peruma')?.getAttribute('role'),
        noSize: !document.getElementById('c-size'),
        columns: members ? getComputedStyle(members).gridTemplateColumns.split(' ').length : 0,
        font: odds ? getComputedStyle(odds).fontFamily : '',
      };
    });
    expect(layout).toEqual({
      beside: true,
      fits: true,
      inPanel: true,
      countInColumn: true,
      perUmaSwitch: 'switch',
      noSize: true,
      columns: 3,
      font: expect.stringMatching(/Zen Maru/),
    });
  });
  await test.step("swap rows: the card out greyed ➜ the card in, every card's art on one line, the arrow at its middle", async () => {
    const rows = await page.$$eval('#results [data-deck="swap"]', (all) =>
      all.map((d) => {
        const art = [...d.querySelectorAll('[data-side] [data-card]')].map((x) => x.getBoundingClientRect());
        const a = d.querySelector('svg')?.getBoundingClientRect();
        const out = d.querySelector('[data-side="out"] [data-card]');
        const [first] = art;
        return {
          tops: new Set(art.map((x) => Math.round(x.top))).size,
          mid: a && first ? Math.round(Math.abs((a.top + a.bottom) / 2 - (first.top + first.bottom) / 2)) : -1,
          grey: out ? getComputedStyle(out).filter : '',
        };
      }),
    );
    expect(rows.length).toBeGreaterThan(0);
    for (const r of rows)
      expect({ ...r, mid: r.mid >= 0 && r.mid <= 1 }).toEqual({
        tops: 1,
        mid: true,
        grey: expect.stringMatching(/grayscale/),
      });
  });
  await test.step("a reload shows the saved settings' result from the first frame (never the example), and runs no search", async () => {
    await page.click('#rank button[data-v="ss_and_up"]');
    await expect.poll(() => page.evaluate(() => window.__runs.length)).toBe(2);
    await loopSettled(page);
    // (on loop-small the two answers' odds round alike, so the drawn text does not tell the example from the saved
    // result: the reload below shows that text from the first frame and runs no search)
    const mine = await page.locator('#results [data-odds]').first().textContent();
    await quiet(page);
    await page.clock.install(); // (the pause before a search is the page's own timer: run past it, not waited out)
    await page.reload();
    await loopSettled(page);
    // (the bundle is a cached script that may run after the first paint: a blank results area before it is allowed)
    const seen = [...new Set(await page.evaluate(() => window.llpSeen ?? []))].filter((x) => x !== '');
    expect(seen).toEqual([mine]);
    // (long enough for a search to have started: past the page's pause before one)
    await page.clock.runFor(1500);
    expect(await page.evaluate(() => window.__runs.length)).toBe(0);
    await expect(page.locator('#run-note')).toHaveText(/^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/, {
      useInnerText: true,
    });
    const kept = await liveRegion(page);
    expect(kept?.region).toBe('status polite');
    expect(kept?.same).toBe(true);
    expect(kept?.texts[0]).toBe('');
    expect(kept?.texts.at(-1)).toMatch(/^Checked [\d,]+ decks/);
  });
});
