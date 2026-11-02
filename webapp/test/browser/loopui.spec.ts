// The loop page's setup and results where only a browser can tell: lengths at real widths (the phone at 375 px, the
// result's popups, the scenario tiles, the card list), the panel beside its card, the game tooltip over a modal dialog,
// the friend frame of a borrowed card, the era switch's reload into the Grand Masters data, what every job sent leaves
// out by the page's clock (engine.ts `gate`), and a page whose scripts stop as they load. What the components draw from
// the settings and a result is test/unit/loopui*.test.tsx's.
import fs from 'node:fs';
import path from 'node:path';
import type { Page } from 'playwright/test';
import type { Card, Uma } from '../../src/data.ts';
import { expect, test } from './fixtures.ts';
import { SIDEWAYS, closeEditor, loopSettled, openEditor, openLoop } from './pages.ts';

test.describe.configure({ timeout: 60_000 });
// (on loop-small: three umas, two targets, fourteen cards)
test.use({ scenario: 'loop-small' });

const REPO = path.resolve(import.meta.dirname, '../../..');
const PAGE: { cards: Card[]; umas: Uma[] } = JSON.parse(
  fs.readFileSync(path.join(REPO, 'pkg/loopopt/data/global/page.json'), 'utf8'),
);
// Grand Masters' tile line: research/results.json's pooled fit as a multiple of URA Finale's, each rounded to 3
// decimals as pkg/loopopt/build.rs reads them
const VALUES: Record<string, number> = JSON.parse(
  fs.readFileSync(path.join(REPO, 'research/results.json'), 'utf8'),
).values;
const fitted = (key: string) => +(VALUES[key] ?? Number.NaN).toFixed(3);
const GM_RATE = `Hint rate ×${(fitted('7.one_base_and_one_d_base') / fitted('7.specialty_base_ura_finale')).toFixed(2)}`;
// whether this build carries the game's art (a clone without the game files shows initials)
const ART = fs.existsSync(path.join(REPO, 'webapp/img/uma'));

// (runs in the page) every visible pill track (the segmented controls): its segments fill it flush, the full inner
// height, no gap at its ends or between them, the track a pill that clips them, none drawn with the game's pill sprite.
// What is wrong, one line per fault; distances in CSS px, 0.6 px tolerance.
const PILLS = (): string[] => {
  const bad: string[] = [];
  const near = (a: number, b: number) => Math.abs(a - b) <= 0.6;
  for (const t of document.querySelectorAll<HTMLElement>('[data-seg]:not([data-seg="apart"])')) {
    const r = t.getBoundingClientRect();
    const cs = getComputedStyle(t);
    if (!r.width || !r.height || cs.visibility === 'hidden') continue;
    const id = t.id ? `#${t.id}` : t.getAttribute('aria-label') || t.className;
    const inner = {
      l: r.left + parseFloat(cs.borderLeftWidth),
      r: r.right - parseFloat(cs.borderRightWidth),
      t: r.top + parseFloat(cs.borderTopWidth),
      b: r.bottom - parseFloat(cs.borderBottomWidth),
    };
    if (cs.overflowX !== 'hidden' && cs.overflowX !== 'clip') bad.push(`${id}: does not clip its segments`);
    if (parseFloat(cs.borderTopLeftRadius) < r.height / 2 - 0.5) bad.push(`${id}: not a pill`);
    const segs = [...t.children].filter((c) => c.getBoundingClientRect().width > 0);
    segs.forEach((s, i) => {
      const q = s.getBoundingClientRect();
      if (!near(q.top, inner.t) || !near(q.bottom, inner.b)) bad.push(`${id} #${i}: short of top / bottom`);
      if (i === 0 && !near(q.left, inner.l)) bad.push(`${id}: a gap at its start`);
      if (i === segs.length - 1 && !near(q.right, inner.r)) bad.push(`${id}: a gap at its end`);
      const before = segs[i - 1];
      if (before && !near(q.left, before.getBoundingClientRect().right)) bad.push(`${id}: a gap before #${i}`);
      if (getComputedStyle(s).borderImageSource !== 'none') bad.push(`${id} #${i}: drawn with a border-image`);
    });
  }
  return bad;
};
// (runs in the page) the card list's pictures: their widths (one, loaded or not) and how many overflow their tile
const CARD_ART = () => {
  const sizes = new Set<number>();
  let over = 0;
  for (const t of document.querySelectorAll('#cards button[aria-pressed]')) {
    const c = t.querySelector('[data-card]')?.getBoundingClientRect();
    if (!c) continue;
    sizes.add(Math.round(c.width));
    if (c.width > t.getBoundingClientRect().width + 0.5) over++;
  }
  return { sizes: [...sizes], over };
};
// (runs in the page) the scenario tiles: their sizes and the height of their hint-rate line
const TILES = () =>
  [...document.querySelectorAll('#scen-menu [role="menuitemradio"]')].map((t) => {
    const r = t.getBoundingClientRect();
    return {
      v: t instanceof HTMLElement ? (t.dataset.v ?? '') : '',
      size: `${Math.round(r.width)}x${Math.round(r.height)}`,
      line: Math.round(t.lastElementChild?.getBoundingClientRect().height ?? 0),
      x: Math.round(r.left + r.width / 2),
    };
  });
const fits = (page: Page, id: string) =>
  page.evaluate((id) => {
    const r = document.getElementById(id)?.getBoundingClientRect();
    return (
      !!r &&
      r.width > 0 &&
      r.left >= 0 &&
      r.right <= innerWidth &&
      document.documentElement.scrollWidth <= document.documentElement.clientWidth
    );
  }, id);
const resultButton = (page: Page, name: string) =>
  page.locator('#results').getByRole('button', { name, exact: true }).first();
const closeShows = (page: Page) =>
  page.evaluate(() => {
    const r = [...document.querySelectorAll('dialog[open] button')]
      .find((x) => x.textContent === 'Close')
      ?.getBoundingClientRect();
    return !!r && r.top >= 0 && r.bottom <= innerHeight;
  });

// a phone: touch and a mobile viewport where the engine has them (Chromium; Firefox has no mobile emulation)
const phone = test.extend({
  isMobile: async ({ browserName }, use) => use(browserName === 'chromium'),
  hasTouch: async ({ browserName }, use) => use(browserName === 'chromium'),
});
phone.describe('phone @both', () => {
  phone.use({ viewport: { width: 375, height: 812 } });
  phone(
    '375 px: touch sizes, results first, the status button, the setup boxes, popups that fit, pills flush',
    async ({ page, browserName }) => {
      await openLoop(page);
      const small = await page.evaluate(() =>
        [
          ...document.querySelectorAll<HTMLElement>(
            '[aria-labelledby="h-set"] button, [aria-labelledby="h-set"] select, #rhead button, #rhead select, #results article > div:last-child button, #results [data-member] button',
          ),
        ]
          .filter((e) => e.offsetParent && !e.closest('#roster, #cards'))
          .map((e) => [e.id || e.className || e.tagName, Math.round(e.getBoundingClientRect().height)] as const)
          .filter(([, h]) => h < 44),
      );
      expect(small, 'buttons, selects and search boxes at least 44 px high').toEqual([]);
      // results first: above the settings and first in the document, the odds in the first screen
      const order = await page.evaluate(() => {
        const c = document.getElementById('rcol'),
          r = document.querySelector('[aria-labelledby="h-set"]'),
          o = document.querySelector('#results [data-odds]')?.getBoundingClientRect();
        return {
          above: !!c && !!r && c.getBoundingClientRect().top < r.getBoundingClientRect().top,
          dom: !!c && !!r && !!(c.compareDocumentPosition(r) & Node.DOCUMENT_POSITION_FOLLOWING),
          inFirstScreen: !!o && o.bottom < innerHeight,
        };
      });
      expect(order).toEqual({ above: true, dom: true, inFirstScreen: true });
      // a swap row of three cards out and three in still fits: the example's row with its chips cloned, then put back
      const swap33 = await page.evaluate(() => {
        const row = document.querySelector('#results [data-deck="swap"]');
        if (!row) return null;
        const clones: Element[] = [];
        for (const side of row.querySelectorAll('[data-side]'))
          for (let k = side.querySelectorAll('button[data-cid]').length; k < 3; k++) {
            const c = side.querySelector('button[data-cid]')?.cloneNode(true);
            if (c instanceof Element) {
              side.append(c);
              clones.push(c);
            }
          }
        const fit = document.documentElement.scrollWidth <= document.documentElement.clientWidth,
          right = row.getBoundingClientRect().right,
          cards = Math.max(
            ...[...row.querySelectorAll('button[data-cid]')].map((d) => d.getBoundingClientRect().right),
          );
        for (const c of clones) c.remove();
        return { fit, inside: cards <= right };
      });
      expect(swap33).toEqual({ fit: true, inside: true });
      if (browserName === 'chromium') {
        const cls = await page.evaluate(
          () =>
            new Promise<number>((resolve) => {
              let v = 0;
              new PerformanceObserver((l) => {
                for (const e of l.getEntries())
                  if ('hadRecentInput' in e && 'value' in e && !e.hadRecentInput && typeof e.value === 'number')
                    v += e.value;
              }).observe({ type: 'layout-shift', buffered: true });
              setTimeout(() => resolve(v), 200);
            }),
        );
        expect(cls, 'the page barely shifts while loading (CLS)').toBeLessThan(0.05);
      }
      // the floating status button: shown only while the results are out of view, then on screen
      const height = await page.evaluate(() => document.documentElement.scrollHeight);
      for (const f of [0, 0.5, 1]) {
        await page.evaluate((y) => scrollTo(0, y), f * height);
        await page.waitForFunction(() => {
          const p = document.getElementById('run-pill'),
            c = document.getElementById('rcol')?.getBoundingClientRect();
          if (!p || !c) return false;
          const r = p.getBoundingClientRect(),
            resultsIn = c.bottom > 0 && c.top < innerHeight,
            shown = getComputedStyle(p).display !== 'none';
          return shown === !resultsIn && (!shown || (r.top >= 0 && r.bottom <= innerHeight));
        });
      }
      await expect(page.locator('#run-pill')).toHaveText(/^1 in 103 · see result/, { useInnerText: true });
      // a card edited far down, then the button takes you back to the results
      await openEditor(page, 'cards');
      const card = page.locator('#cards button[aria-pressed]').nth(2);
      await card.click();
      await loopSettled(page);
      await card.click();
      await loopSettled(page);
      await closeEditor(page, 'cards');
      await page.click('#run-pill');
      await page.waitForFunction(() => {
        const r = document.getElementById('rcol')?.getBoundingClientRect();
        return !!r && Math.abs(r.top) < 40 && document.activeElement?.id === 'rcol';
      });
      // the roster and card boxes: shorter than the screen, their bottom edge fading while there is more below
      for (const [id, editor] of [
        ['roster', 'umas'],
        ['cards', 'cards'],
      ] as const) {
        await openEditor(page, editor);
        await expect(page.locator(`#${id}`)).toHaveAttribute('data-more', /.*/);
        expect(
          await page.evaluate((id) => (document.getElementById(id)?.clientHeight ?? 1e9) / innerHeight, id),
        ).toBeLessThanOrEqual(0.8);
        if (editor === 'cards') {
          const art = await page.evaluate(CARD_ART);
          expect(art.sizes.length, `card pictures one size: ${art.sizes}`).toBe(1);
          expect(art.over).toBe(0);
        }
        await closeEditor(page, editor);
      }
      await page.evaluate(() => scrollTo(0, 0));
      await page.click('#x-scen');
      expect(await fits(page, 'scen-menu'), 'the open scenario picker fits').toBe(true);
      const tiles = await page.evaluate(TILES);
      expect(new Set(tiles.map((t) => t.size)).size).toBe(1);
      for (const t of tiles) expect(t.line, `${t.v}: logo and one line`).toBeLessThanOrEqual(22);
      await page.keyboard.press('Escape');
      await page.click('#rank-q');
      expect(await fits(page, 'rank-note'), 'the open lineage-rank note fits').toBe(true);
      await page.keyboard.press('Escape');
      await resultButton(page, 'Race schedule').click();
      await expect(page.locator('#sched-dlg')).toHaveAttribute('open');
      await expect(page.locator('#sched-dlg [data-slot~="pre"]:visible'), 'no grey Junior cells').toHaveCount(0);
      await page.evaluate(() => {
        const d = document.getElementById('sched-body');
        d?.scrollTo(0, d.scrollHeight);
      });
      expect(await closeShows(page), "the schedule's Close stays on screen").toBe(true);
      await page.locator('#sched-dlg').getByRole('tab').last().click();
      expect(
        await page.evaluate(() =>
          [...document.querySelectorAll('#sched-dlg [data-tables]')].every((w) => w.scrollWidth <= w.clientWidth),
        ),
        'the All three table fits',
      ).toBe(true);
      await page.keyboard.press('Escape');
      await resultButton(page, 'Affinity').click();
      await expect(page.locator('#why-dlg')).toHaveAttribute('open');
      await page.evaluate(() => {
        const d = document.getElementById('why-body');
        d?.scrollTo(0, d.scrollHeight);
      });
      expect(await closeShows(page), "the affinity's Close stays on screen").toBe(true);
      await page.keyboard.press('Escape');
      await page.click('#ex-open');
      await expect(page.locator('#ex-dlg div:has(> h3) > #ex-close'), 'Close in the header row').toHaveCount(1);
      await page.keyboard.press('Escape');
      await page.evaluate(() => scrollTo(0, 0));
      expect(await page.evaluate(PILLS)).toEqual([]);
      expect(await page.locator('[data-seg]:not([data-seg="apart"]):visible').count()).toBeGreaterThanOrEqual(3);
      // nothing scrolls sideways with the editors and the result's popups open
      const sideways: string[] = [];
      for (const editor of ['umas', 'cards'] as const) {
        await openEditor(page, editor);
        sideways.push(...(await page.evaluate(SIDEWAYS)).map((x) => `${editor}: ${x}`));
        await closeEditor(page, editor);
      }
      for (const [id, name] of [
        ['why-dlg', 'Affinity'],
        ['odds-dlg', 'Odds'],
      ] as const) {
        await resultButton(page, name).click();
        await expect(page.locator(`#${id}`)).toHaveAttribute('open');
        sideways.push(...(await page.evaluate(SIDEWAYS)).map((x) => `${id}: ${x}`));
        await page.keyboard.press('Escape');
        await expect(page.locator(`#${id}`)).not.toHaveAttribute('open');
      }
      expect(sideways).toEqual([]);
      const touch = await page.evaluate(() =>
        [...document.querySelectorAll('#rhead label:has([role="switch"])')].map((l) =>
          Math.round(l.getBoundingClientRect().height - 2 * parseFloat(getComputedStyle(l, '::before').top)),
        ),
      );
      expect(touch.length).toBe(2);
      for (const h of touch)
        expect(h, 'the Auto-search and Deck per uma switches: 44 px touch areas').toBeGreaterThanOrEqual(44);
      // widened to desktop, the results go back beside (after) the settings
      await page.setViewportSize({ width: 1280, height: 800 });
      await page.waitForFunction(() => {
        const settings = document.querySelector('[aria-labelledby="h-set"]')?.parentElement,
          results = document.getElementById('rcol');
        return (
          !!settings && !!results && !!(settings.compareDocumentPosition(results) & Node.DOCUMENT_POSITION_FOLLOWING)
        );
      });
    },
  );
});

test.describe('desktop results @both', () => {
  // three results (a rotation, then four-uma loops), and a pal not owned, so a deck borrows one: loop-small with a
  // fourth uma, three decks shown and a pal left out
  test.use({ scenario: 'loop-four' });
  test('four-uma loops, a borrowed card, the panel beside its card, the schedule and affinity popups, a tooltip over them', async ({
    page,
  }) => {
    await openLoop(page);
    const sizes = await page.evaluate(() =>
      [...document.querySelectorAll('#results article')].map((l) => l.querySelectorAll('[data-member]').length),
    );
    expect(sizes.length).toBe(3);
    expect(sizes[0]).toBe(3);
    for (const n of sizes.slice(1)) expect([0, 4]).toContain(n);
    const four = sizes.indexOf(4);
    expect(four).toBeGreaterThan(0);
    const loop = page.locator('#results article').nth(four);
    expect(
      await loop.evaluate(
        (l) => getComputedStyle(l.querySelector('[data-members]') ?? l).gridTemplateColumns.split(' ').length,
      ),
      'a four-uma loop: four members in four columns',
    ).toBe(4);
    // a borrowed card: last in its row, named as borrowed, in the friend frame
    const rows = await page.evaluate(() =>
      [...document.querySelectorAll('#results [data-deck-cards]')]
        .map((r) => [...r.querySelectorAll<HTMLElement>('button[data-cid]')])
        .filter((r) => r.some((x) => x.dataset.tag === 'borrowed'))
        .map((r) => r.findIndex((x) => x.dataset.tag === 'borrowed') === r.length - 1),
    );
    expect(rows.length).toBeGreaterThan(0);
    expect(rows.every(Boolean)).toBe(true);
    expect(
      await page.evaluate(() =>
        [...document.querySelectorAll('#results button[data-tag="borrowed"]')].every((d) =>
          /borrowed/.test(d.getAttribute('aria-label') ?? ''),
        ),
      ),
    ).toBe(true);
    expect(
      await page.evaluate(() => {
        const d = document.querySelector('#results button[data-tag="borrowed"]');
        return !!d && getComputedStyle(d, '::after').borderImageSource.startsWith('url');
      }),
      'the friend frame',
    ).toBe(true);
    // the deck card panel opens beside its card, not over the deck below
    await page.locator('#results button[data-cid]').first().click();
    const side = await page.evaluate(() => {
      const c = document.querySelector('#results button[data-cid]')?.getBoundingClientRect(),
        p = document.querySelector('[data-pop="card"]')?.getBoundingClientRect();
      return !!c && !!p && (p.left >= c.right || p.right <= c.left) && Math.abs(p.top - c.top) <= 2;
    });
    expect(side).toBe(true);
    await page.keyboard.press('Escape');
    if (ART) {
      const spark = page.locator('#results article').first().locator('[data-spark]').first();
      await expect(spark.locator('img[data-icon="skill"]'), 'an outfit skill shows its icon').toHaveCount(1);
    }
    // the four-uma loop's schedule: a tab per member and "All four", its table a column per member
    await loop.getByRole('button', { name: 'Race schedule', exact: true }).click();
    const sched = page.locator('#sched-dlg');
    await expect(sched).toHaveAttribute('open');
    await expect(sched.getByRole('tab')).toHaveCount(5);
    await expect(sched.getByRole('tab').last()).toHaveText('All four');
    const grids = await page.evaluate(() =>
      [...document.querySelectorAll('#sched-dlg [data-grid]')].map((g) => {
        const r = g.getBoundingClientRect();
        return `${Math.round(r.width)}x${Math.round(r.height)}`;
      }),
    );
    expect(grids.length).toBe(3);
    expect(new Set(grids).size, 'three identical year grids').toBe(1);
    expect(
      await page.evaluate(() => {
        const d = document.getElementById('sched-body');
        return !!d && d.scrollHeight <= d.clientHeight + 1;
      }),
      'the agenda fits the window without scrolling',
    ).toBe(true);
    if (ART)
      expect(await sched.locator('[data-race]').count()).toBe(
        await sched.locator('[data-race] img[data-banner]').count(),
      );
    await sched.getByRole('tab').last().click();
    await expect(page.locator('#sched-view table').first().locator('th[data-k]')).toHaveCount(4);
    await page.keyboard.press('Escape');
    // its affinity: one pill per trainee (4), her three parent pairs, six legacies each, no grandparent the trainee
    await loop.getByRole('button', { name: 'Affinity', exact: true }).click();
    const why = page.locator('#why-dlg');
    await expect(why).toHaveAttribute('open');
    const trainees = await why.locator('[aria-label="Trainee"] [role="radio"]').allTextContents();
    expect(new Set(trainees.map((t) => t.replace(/\d+/g, '').trim())).size).toBe(4);
    expect(trainees.length).toBe(4);
    await expect(why.getByRole('radiogroup', { name: 'Parents' }).getByRole('radio')).toHaveCount(3);
    await expect(why.getByRole('table').locator('tbody tr')).toHaveCount(6);
    await expect(why.getByText('her own earlier veteran')).toHaveCount(0);
    // the game tooltip over a modal dialog: its balloon in the top layer outside the dialog, the title held while it
    // shows; a title drawn again meanwhile is kept when it goes
    await why.locator('[aria-label="Trainee"] [role="radio"]').first().hover();
    await page.waitForSelector('#gm-tip:not([hidden])');
    const tip = await page.evaluate(() => {
      const t = document.querySelector('#gm-tip'),
        held = document.querySelector('#why-dlg [aria-label="Trainee"] [role="radio"]');
      if (!(t instanceof HTMLElement) || !(held instanceof HTMLElement)) return null;
      const shown = {
        inBody: t.parentElement === document.body,
        top: t.matches(':popover-open'),
        text: (t.textContent ?? '').length > 0,
        held: !held.hasAttribute('title'),
      };
      held.title = 'Drawn again';
      return shown;
    });
    expect(tip).toEqual({ inBody: true, top: true, text: true, held: true });
    await page.mouse.move(0, 0);
    await expect(page.locator('#gm-tip')).toBeHidden();
    expect(
      await page.evaluate(() => ({
        title: document.querySelector<HTMLElement>('#why-dlg [aria-label="Trainee"] [role="radio"]')?.title,
        top: document.querySelector('#gm-tip')?.matches(':popover-open'),
      })),
    ).toEqual({ title: 'Drawn again', top: false });
  });
});

test.describe('desktop setup', () => {
  // the newest outfits and cards stand in for a banner not started yet: the page's clock a minute before their start
  const newest = Math.max(...PAGE.cards.map((c) => c.rel), ...PAGE.umas.flatMap((u) => u.outfits.map((o) => o.rel)));
  test('setup lengths, a job leaving out what has not started, the Grand Masters era, failed logos, a page that cannot start', async ({
    page,
    pageErrors,
  }) => {
    await page.clock.setFixedTime((newest - 60) * 1000);
    await openLoop(page);
    // a job sent leaves out what starts later (engine.ts `gate`)
    await page.click('#rank button[data-v="ss_and_up"]');
    await loopSettled(page);
    const sent = await page.evaluate(() => window.__runs.filter((j) => j.cmd === 'deck').at(-1));
    if (sent?.cmd !== 'deck') throw new Error('a deck job was sent');
    const later = {
      outfits: PAGE.umas
        .flatMap((u) => u.outfits)
        .filter((o) => o.rel > newest - 60)
        .map((o) => o.id),
      cards: PAGE.cards.filter((c) => c.rel > newest - 60).map((c) => c.id),
    };
    expect(later.outfits.length + later.cards.length).toBeGreaterThan(0);
    for (const id of later.outfits) expect(sent.skills.exclude_outfits ?? []).toContain(id);
    for (const id of later.cards) expect(sent.deck.exclude_cards ?? []).toContain(id);
    // the targets: the add box above the list, two per row
    const lay = await page.evaluate(() => {
      const t = [...document.querySelectorAll('#targets > [data-g]')].map((e) => e.getBoundingClientRect());
      return {
        rows: new Set(t.map((r) => Math.round(r.top))).size,
        n: t.length,
        addFirst:
          (document.getElementById('t-add')?.getBoundingClientRect().bottom ?? 1e9) <= Math.min(...t.map((r) => r.top)),
      };
    });
    expect(lay.addFirst).toBe(true);
    expect(lay.rows).toBe(Math.ceil(lay.n / 2));
    // the scenario tiles: one size, logo and one line; the logos keep their aspect; the golds' icons one line high
    await page.click('#x-scen');
    const tiles = await page.evaluate(TILES);
    expect(tiles.length).toBe(4);
    expect(new Set(tiles.map((t) => t.size)).size).toBe(1);
    for (const t of tiles) expect(t.line).toBeLessThanOrEqual(22);
    if (ART) {
      expect(
        await page.evaluate(() =>
          [...document.querySelectorAll<HTMLImageElement>('#scen-dd [data-scen-logo] img')].every(
            (i) => i.naturalWidth > 0 && getComputedStyle(i).objectFit === 'contain',
          ),
        ),
      ).toBe(true);
      const golds = await page.evaluate(() =>
        [...document.querySelectorAll('#scen-menu [data-v="unity"] [role="group"] img')].map((i) =>
          Math.round(i.getBoundingClientRect().height),
        ),
      );
      expect(golds.length).toBe(2);
      for (const h of golds) expect(h).toBeLessThanOrEqual(18);
    }
    await page.keyboard.press('Escape');
    // the card list scrolls apart from its search row, its tiles' rings and focus outline inside it; one picture size
    await openEditor(page, 'cards');
    const sep = await page.evaluate(() => {
      const c = document.getElementById('cards'),
        r = c?.previousElementSibling?.getBoundingClientRect();
      return c && r
        ? { gap: c.getBoundingClientRect().top - r.bottom, pad: parseFloat(getComputedStyle(c).paddingTop) }
        : null;
    });
    expect(sep?.gap).toBeGreaterThanOrEqual(10);
    expect(sep?.pad).toBeGreaterThanOrEqual(5);
    const art = await page.evaluate(CARD_ART);
    expect(art.sizes.length).toBe(1);
    expect(art.over).toBe(0);
    await closeEditor(page, 'cards');
    // the Grand Masters era: its data fetched (the status line says so meanwhile), Grand Masters at SS, searched
    await page.route('**/assets/data-gm-*', async (r) => {
      await new Promise((x) => setTimeout(x, 1500));
      await r.continue();
    });
    await page.click('#era-gm');
    await expect(page.locator('#run-note')).toHaveText(/Loading the JP Grand Masters-era data/);
    await loopSettled(page);
    await page.unroute('**/assets/data-gm-*');
    await expect(page.locator('#x-scen')).toHaveAttribute('aria-label', 'Scenario: Grand Masters');
    await expect(page.locator('#rank')).toHaveAttribute('data-value', 'ss_and_up');
    await expect(page.locator('#run-note')).toHaveText(/^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/);
    expect(
      await page.evaluate(() => {
        const j = window.__runs.at(-1);
        return j?.cmd === 'deck' && j.deck.scenario === 'grandmasters' && j.skills.rank_class === 'ss_and_up';
      }),
    ).toBe(true);
    await expect(page.locator('#g-20049')).toHaveAttribute('data-g', '0');
    await expect(page.locator('#g-20046')).toHaveAttribute('data-g', '0');
    const gmUmas: Uma[] = await page.evaluate(async () => {
      const assets = JSON.parse(document.getElementById('data-assets')?.textContent ?? '{}');
      return (await (await fetch(assets.dataGm)).json()).umas;
    });
    const now = (newest - 60) * 1000;
    const gmTiles = gmUmas
      .map((u) => u.outfits.filter((o) => o.rel * 1000 <= now).length || (u.outfits.length ? 0 : 1))
      .reduce((a, n) => a + n, 0);
    await expect(page.locator('#roster > [data-uma]'), 'the roster from the GM data').toHaveCount(gmTiles);
    // its fifth tile: Grand Masters, the same size, last and centred, its rate an estimate
    await page.click('#x-scen');
    const gm = await page.evaluate(TILES);
    const mid = await page.evaluate(() => {
      const r = document.getElementById('scen-menu')?.getBoundingClientRect();
      return r ? Math.round(r.left + r.width / 2) : -1;
    });
    expect(gm.length).toBe(5);
    expect(gm[4]?.v).toBe('grandmasters');
    expect(new Set(gm.map((t) => t.size)).size).toBe(1);
    expect(Math.abs((gm[4]?.x ?? 0) - mid)).toBeLessThanOrEqual(1);
    const gmLine = await page
      .locator('#scen-menu [data-v="grandmasters"] > :last-child')
      .evaluate((e) => (e instanceof HTMLElement ? e.innerText : '').replace(/\s+/g, ' ').trim());
    expect(gmLine).toMatch(/ \((estimate|est\.)\)$/);
    expect(gmLine.replace(/ \((estimate|est\.)\)$/, '')).toBe(GM_RATE);
    await page.keyboard.press('Escape');
    // back on Global: Unity Cup with its golds, rank below SS
    await page.click('#era-global');
    await loopSettled(page);
    await expect(page.locator('#x-scen')).toHaveAttribute('data-value', 'unity');
    await expect(page.locator('#rank')).toHaveAttribute('data-value', 'below_ss');
    await expect(page.locator('#g-20049')).toHaveAttribute('data-g', '2');
    // logos that fail to load: name badges, the tiles one size still
    await page.route('**/img.*/scen/**', (r) => r.abort());
    await page.reload();
    await loopSettled(page);
    await page.click('#x-scen');
    await expect(page.locator('#scen-dd [data-scen-logo] img')).toHaveCount(0);
    expect(new Set((await page.evaluate(TILES)).map((t) => t.size)).size).toBe(1);
    await page.keyboard.press('Escape');
    // a script that stops as the page loads (Unity Cup's hint base missing from the page's data): the status line
    // says the page could not start, and why, on both pages
    await page.route('**/assets/shared-*.js', async (r) => {
      const response = await r.fetch(),
        body = await response.text();
      if (body.split('{"unity":').length !== 2) throw new Error('the shared script holds no one hint-base table');
      await r.fulfill({ response, body: body.replace('{"unity":', '{"unity cup":') });
    });
    for (const url of ['/', '/loop']) {
      await page.goto(url);
      await expect(page.locator('#run-note')).toHaveText(
        "The page could not start: failed: the build writes unity's hint base",
      );
      await expect(page.locator('#rhead')).toHaveAttribute('data-kind', 'error');
    }
    expect(pageErrors.filter((e) => /hint base/.test(e)).length).toBeGreaterThanOrEqual(2);
    pageErrors.length = 0;
  });
});
