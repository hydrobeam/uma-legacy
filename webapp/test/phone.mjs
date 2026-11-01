import { expect } from 'playwright/test';
import {
  check,
  artCheck,
  open,
  openEditor,
  closeEditor,
  settle,
  dialog,
  until,
  PILLS,
  SIDEWAYS,
  CARD_ART,
} from './helpers.mjs';

export default async function phone(b) {
  const pg = await open(b, { w: 375, h: 812, mobile: true });
  const small = await pg.evaluate(() =>
    [
      ...document.querySelectorAll(
        '[aria-labelledby="h-set"] button, [aria-labelledby="h-set"] select, #rhead button, #rhead select, .loop .lacts button, .mem .memb',
      ),
    ]
      .filter((e) => e.offsetParent && !e.closest('.roster, .cards'))
      .map((e) => [e.id || e.className || e.tagName, Math.round(e.getBoundingClientRect().height)])
      .filter(([, h]) => h < 44),
  );
  check(
    'phone: buttons, selects and search boxes are at least 44 px high',
    !small.length,
    JSON.stringify(small.slice(0, 6)),
  );
  // results first: above the settings, and first in the document (reading and Tab order), the result in the first screen
  const order = await pg.evaluate(() => {
    const c = document.getElementById('rcol'),
      r = document.querySelector('[aria-labelledby="h-set"]'),
      o = document.querySelector('#results .loop .odds').getBoundingClientRect();
    return {
      above: c.getBoundingClientRect().top < r.getBoundingClientRect().top,
      dom: !!(c.compareDocumentPosition(r) & Node.DOCUMENT_POSITION_FOLLOWING),
      odds: Math.round(o.bottom),
      h: innerHeight,
    };
  });
  check(
    'phone: the results come before the settings (on screen and in the Tab order), the odds in the first screen',
    order.above && order.dom && order.odds < order.h,
    JSON.stringify(order),
  );
  // a swap row of three cards out and three in (5 of the 29 swap rows in the pinned val reports) still fits: the
  // example's 1/1 row with its chips cloned, then put back
  const swap33 = await pg.evaluate(() => {
    const row = document.querySelector('#results .deck.swap');
    if (!row) return null;
    const clones = [];
    for (const side of row.querySelectorAll('.swc'))
      for (let k = side.querySelectorAll('.dc').length; k < 3; k++) {
        const c = side.querySelector('.dc').cloneNode(true);
        side.append(c);
        clones.push(c);
      }
    const fits = document.documentElement.scrollWidth <= document.documentElement.clientWidth,
      card = row.getBoundingClientRect().right,
      cards = Math.round(Math.max(...[...row.querySelectorAll('.dc')].map((d) => d.getBoundingClientRect().right)));
    for (const c of clones) c.remove();
    return { fits, inside: cards <= card, cards, card: Math.round(card) };
  });
  check(
    'phone: a swap row of three cards out and three in fits the screen',
    !!swap33?.fits && swap33.inside,
    JSON.stringify(swap33),
  );
  if (b.browserType().name() === 'chromium') {
    const cls = await pg.evaluate(
      () =>
        new Promise((r) => {
          let v = 0;
          new PerformanceObserver((l) => {
            for (const e of l.getEntries()) if (!e.hadRecentInput) v += e.value;
          }).observe({ type: 'layout-shift', buffered: true });
          setTimeout(() => r(v), 200);
        }),
    );
    check('phone: the page barely shifts while loading (CLS < 0.05)', cls < 0.05, cls.toFixed(4));
  }
  // the floating status button: shown only while the results are out of view, on screen wherever the page is scrolled
  const H = await pg.evaluate(() => document.documentElement.scrollHeight);
  const seen = [];
  for (const f of [0, 0.5, 1]) {
    await pg.evaluate((y) => scrollTo(0, y), f * H); // the button follows an IntersectionObserver
    seen.push(
      await until(pg, () => {
        const p = document.getElementById('run-pill'),
          r = p.getBoundingClientRect(),
          c = document.getElementById('rcol').getBoundingClientRect();
        const resIn = c.bottom > 0 && c.top < innerHeight,
          shown = getComputedStyle(p).display !== 'none';
        return shown === !resIn && (!shown || (r.top >= 0 && r.bottom <= innerHeight));
      }),
    );
  }
  check(
    'phone: the status button shows (on screen) exactly when the results are out of view',
    seen.every(Boolean),
    seen.join(','),
  );
  await expect
    .soft(pg.locator('#run-pill'), 'phone: it shows the headline odds')
    .toHaveText(/^1 in 103 · see result/, { useInnerText: true });
  // edit a setting far down, then the button takes you back to the result
  await openEditor(pg, 'cards');
  const ct = pg.locator('#cards .ct').nth(2);
  await ct.scrollIntoViewIfNeeded();
  await ct.click();
  await settle(pg);
  await ct.click();
  await settle(pg);
  await closeEditor(pg, 'cards');
  await pg.click('#run-pill'); // a smooth scroll
  check(
    'phone: after editing a card, the status button scrolls back to the results',
    await until(pg, () => {
      const r = document.getElementById('rcol').getBoundingClientRect();
      return Math.abs(r.top) < 40 && document.activeElement.id === 'rcol';
    }),
  );
  // the roster and card boxes: shorter than the screen, their bottom edge fading while there is more below
  const boxes = [];
  for (const [id, editor] of [
    ['roster', 'umas'],
    ['cards', 'cards'],
  ]) {
    await closeEditor(pg, 'umas');
    await openEditor(pg, editor);
    await expect
      .soft(pg.locator(`#${id}`), `phone: the ${id} box shows that it scrolls (its bottom edge fades)`)
      .toHaveClass(/\bmore\b/);
    boxes.push(await pg.evaluate((id) => document.getElementById(id).clientHeight / innerHeight, id));
  }
  const part = await pg.evaluate(CARD_ART);
  check(
    'phone: every card picture is the same size and inside its tile, loaded or not',
    part.sizes.length === 1 && !part.over,
    JSON.stringify(part),
  );
  check(
    'phone: the roster and card boxes fit in their dialogs',
    boxes.every((h) => h <= 0.8),
    JSON.stringify(boxes),
  );
  await closeEditor(pg, 'cards');
  await pg.evaluate(() => scrollTo(0, 0));
  await pg.click('#x-scen');
  check(
    'phone: the open scenario picker fits the screen',
    await pg.evaluate(() => {
      const r = document.getElementById('scen-menu').getBoundingClientRect();
      return (
        r.width > 0 &&
        r.left >= 0 &&
        r.right <= innerWidth &&
        document.documentElement.scrollWidth <= document.documentElement.clientWidth
      );
    }),
  );
  const ph = await pg.evaluate(() =>
    [...document.querySelectorAll('#scen-menu .stile')].map((t) => {
      const r = t.getBoundingClientRect(),
        s = t.querySelector('.ssub').getBoundingClientRect();
      return Math.round(r.width) + 'x' + Math.round(r.height) + ' line ' + Math.round(s.height);
    }),
  );
  artCheck(
    'phone: the scenario tiles are the same size, each logo + one line',
    new Set(ph.map((x) => x.split(' ')[0])).size === 1 && ph.every((x) => +x.split(' ').at(-1) <= 22),
    ph.join(' | '),
  );
  await pg.keyboard.press('Escape');
  await pg.click('#rank-q');
  check(
    'phone: the open lineage-rank popup fits the screen',
    await pg.evaluate(() => {
      const r = document.getElementById('rank-note').getBoundingClientRect();
      return (
        r.width > 0 &&
        r.left >= 0 &&
        r.right <= innerWidth &&
        document.documentElement.scrollWidth <= document.documentElement.clientWidth
      );
    }),
  );
  await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click();
  await dialog(pg, 'sched-dlg');
  await expect
    .soft(pg.locator('#sched-dlg .ag-slot.pre:visible'), 'phone: agenda stacks without the grey Junior cells')
    .toHaveCount(0);
  await pg.evaluate(() => {
    const d = document.querySelector('#sched-dlg .dlg');
    d.scrollTo(0, d.scrollHeight);
  });
  const closeVis = () =>
    pg.evaluate(() => {
      const b = [...document.querySelectorAll('dialog[open] button')].find((x) => x.textContent === 'Close'),
        r = b.getBoundingClientRect();
      return r.top >= 0 && r.bottom <= innerHeight;
    });
  check("phone: the schedule's Close stays on screen at the bottom of the dialog", await closeVis());
  await pg.locator('#sched-dlg').getByRole('tab').last().click();
  check(
    'phone: the All three table fits the 375 px screen',
    await pg.evaluate(() =>
      [...document.querySelectorAll('#sched-dlg .tablewrap')].every((w) => w.scrollWidth <= w.clientWidth),
    ),
  );
  await pg.keyboard.press('Escape');
  await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click();
  await dialog(pg, 'why-dlg');
  await pg.evaluate(() => {
    const d = document.querySelector('#why-dlg .dlg');
    d.scrollTo(0, d.scrollHeight);
  });
  check("phone: the affinity's Close stays on screen too", await closeVis());
  await pg.keyboard.press('Escape');
  await pg.click('#ex-open');
  await expect
    .soft(
      pg.locator('#ex-dlg div:has(> h3) > #ex-close'),
      "the export dialog's Close sits in the same header row as the others",
    )
    .toHaveCount(1);
  await pg.keyboard.press('Escape');
  // the pill switches (Game data, lineage rank, Results): the chosen part fills its share of the track, flush
  await pg.evaluate(() => scrollTo(0, 0));
  const pills = await pg.evaluate(PILLS);
  check(
    "phone: every pill track's segments fill it flush (full height, no gap at its ends or between them)",
    !pills.length && (await pg.locator('[data-seg]:not([data-seg="apart"]):visible').count()) >= 3,
    pills.slice(0, 4).join(' | '),
  );
  // nothing scrolls sideways with the editors and the result's popups open
  const side = [];
  for (const editor of ['umas', 'cards']) {
    await openEditor(pg, editor);
    side.push(...(await pg.evaluate(SIDEWAYS)).map((x) => editor + ': ' + x));
    await closeEditor(pg, editor);
  }
  for (const [id, text] of [
    ['why-dlg', 'Affinity'],
    ['odds-dlg', 'Odds'],
  ]) {
    await pg.locator('.loop .lacts button', { hasText: text }).first().click();
    await dialog(pg, id);
    side.push(...(await pg.evaluate(SIDEWAYS)).map((x) => id + ': ' + x));
    await pg.keyboard.press('Escape');
    await expect(pg.locator(`#${id}`)).not.toHaveAttribute('open');
  }
  check(
    'phone: nothing scrolls sideways with Umas, Support cards, Affinity or Odds open',
    !side.length,
    side.slice(0, 4).join(' | '),
  );
  const swTouch = await pg.evaluate(() =>
    [...document.querySelectorAll('#rhead label:has([role="switch"])')].map((l) =>
      Math.round(l.getBoundingClientRect().height - 2 * parseFloat(getComputedStyle(l, '::before').top)),
    ),
  );
  check(
    'phone: the Auto-search and Deck per uma switches have a 44 px tall touch area',
    swTouch.length === 2 && swTouch.every((h) => h >= 44),
    swTouch.join(),
  );
  await pg.setViewportSize({ width: 1280, height: 800 }); // the page reorders on its resize handler
  check(
    'widened to desktop, the results go back beside (after) the settings',
    await until(
      pg,
      () =>
        !!(
          document
            .querySelector('[aria-labelledby="h-set"]')
            .parentElement.compareDocumentPosition(document.getElementById('rcol')) & Node.DOCUMENT_POSITION_FOLLOWING
        ),
    ),
  );
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
