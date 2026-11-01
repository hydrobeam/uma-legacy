import { expect } from 'playwright/test';
import { check, open, openEditor, closeEditor, busy, settle, kind, json } from './helpers.mjs';

export default async function states(b) {
  // what the status line says when the settings can't be searched, and the results on screen meanwhile (kept, dimmed)
  // (window.__trap: the next jobs reach the worker without their job, so they fail inside it, as a solver trap would)
  const pg = await open(b, {
    init: () => {
      window.__workerHooks.post.push((m) => (m?.type === 'run' && window.__trap ? { ...m, job: null } : m));
    },
  });
  const rhead = pg.locator('#rhead'),
    runNote = pg.locator('#run-note'),
    act = pg.locator('#r-act'),
    results = pg.locator('#results'),
    loops = pg.locator('#results article');
  const head = () =>
    pg.evaluate(() => ({
      kind: document.getElementById('rhead').dataset.kind,
      note: document.getElementById('run-note').textContent,
      act: document.getElementById('r-act').innerText,
      loops: document.querySelectorAll('#results article').length,
      stale: document.getElementById('results').hasAttribute('data-stale'),
    }));
  // a job that fails inside the worker: the search ends on the error (it used to spin for ever), the results stay, the
  // worker goes, and the next search runs on a fresh one
  const kt = await pg.evaluate(() => {
    window.__trap = true;
    return window.__killed;
  });
  await openEditor(pg, 'cards');
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  const trapped = 'a job that fails in the worker ends the search on its error, the results kept, the worker restarted';
  await expect.soft(rhead, trapped).toHaveAttribute('data-kind', 'error', { timeout: 15_000 });
  await expect.soft(runNote, `${trapped}: the status`).toHaveText(/^Not searched: the solver stopped with an error/);
  await expect.soft(loops, `${trapped}: the results kept`).not.toHaveCount(0);
  await expect.soft(results, `${trapped}: … dimmed`).toContainClass('stale');
  check(`${trapped}: the worker restarted`, (await pg.evaluate(() => window.__killed)) > kt);
  await pg.evaluate(() => {
    window.__trap = false;
  });
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  await settle(pg);
  await expect.soft(runNote, '… and the next search runs').toHaveText(/^Checked [\d,]+ decks/);
  await closeEditor(pg, 'cards');
  while (
    await pg
      .locator('#targets')
      .getByRole('button', { name: /^Remove / })
      .count()
  )
    await pg
      .locator('#targets')
      .getByRole('button', { name: /^Remove / })
      .first()
      .click();
  await kind(pg, 'empty');
  const noTargets =
    'no targets: a neutral status with a button that loads the example targets; the old result stays, dimmed';
  await expect.soft(runNote, noTargets).toHaveText(/Add a target skill/);
  await expect.soft(act, `${noTargets}: the button`).toHaveText(/8\/8 example/, { useInnerText: true });
  await expect.soft(loops, `${noTargets}: the old result stays`).not.toHaveCount(0);
  await expect.soft(results, `${noTargets}: … dimmed`).toContainClass('stale');
  // a preset is never emptied: with no targets, Overwrite is off on every preset
  await pg.click('#t-preset');
  const overwrite = await pg.$$eval('#pr-list [role="group"] button', (l) =>
    l.filter((x) => x.textContent === 'Overwrite').map((x) => x.disabled),
  );
  await pg.locator('#preset-dlg').getByRole('button', { name: 'Done', exact: true }).click();
  check(
    '… and Overwrite is off on every preset',
    overwrite.length > 0 && overwrite.every(Boolean),
    JSON.stringify(overwrite),
  );
  await pg.click('#r-act button');
  await settle(pg);
  const loaded = '… the button loads them and the search runs';
  await expect.soft(pg.locator('#targets > [data-g]'), loaded).toHaveCount(8);
  await expect.soft(rhead, `${loaded}: done`).toHaveAttribute('data-kind', 'done');
  await expect.soft(runNote, `${loaded}: the status`).toHaveText(/^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/);
  await expect.soft(results, `${loaded}: not dimmed`).not.toContainClass('stale');
  await openEditor(pg, 'umas');
  await pg.click('#r-none');
  await kind(pg, 'empty');
  const noUmas = 'no umas owned: a neutral status with "Mark all umas owned"';
  await expect.soft(runNote, noUmas).toHaveText(/at least 3 umas/);
  await expect.soft(act, `${noUmas}: the button`).toHaveText(/Mark all umas owned/, { useInnerText: true });
  await expect.soft(loops, `${noUmas}: the old result stays`).not.toHaveCount(0);
  await closeEditor(pg, 'umas');
  await pg.click('#r-act button');
  await settle(pg);
  // an old export's "must include" mark ('req', since removed) loads as owned
  const u = (await json(pg, 'data-umas')).find((x) => x.outfits.length > 1);
  await pg.click('#ex-open');
  const ex = JSON.parse(await pg.inputValue('#ex-text'));
  ex.settings.outfits = { [u.outfits[0].id]: 'req' };
  await pg.fill('#ex-text', JSON.stringify(ex));
  await pg.click('#ex-load');
  await pg.click('#ex-close');
  await settle(pg);
  const tiles = await pg.evaluate(
    (id) => [...document.querySelectorAll(`#roster [data-uma="${id}"]`)].map((t) => t.dataset.s),
    u.id,
  );
  const mustInclude = "an old export's must-include mark loads as owned, and the search runs";
  check(
    mustInclude,
    tiles.every((x) => x === 'own'),
    tiles.join(','),
  );
  await expect.soft(rhead, `${mustInclude}: done`).toHaveAttribute('data-kind', 'done');
  await expect.soft(runNote, `${mustInclude}: the status`).toHaveText(/^Checked [\d,]+ decks/);
  // Stop: a slow search (three results, one shared deck) is stopped; the results stay and the worker is killed;
  // "Search again" runs it to the end, and the three results are told apart
  const r0 = await pg.evaluate(() => window.__runs.length);
  await pg.click('#c-top [data-v="3"]');
  await pg.click('#c-peruma');
  await pg.waitForFunction(
    (n) =>
      window.__runs.length > n &&
      window.__runs.at(-1).deck.top_decks === 3 &&
      window.__runs.at(-1).deck.per_uma === false,
    r0,
  ); // the slow search itself is running (not a job from the first change alone)
  const searching = 'while searching: the results are marked busy and a Stop button shows';
  await expect.soft(results, searching).toHaveAttribute('aria-busy', 'true');
  await expect.soft(pg.locator('#r-act button'), `${searching}: Stop`).toHaveText('Stop', { useInnerText: true });
  const k0 = await pg.evaluate(() => window.__killed);
  await pg.click('#r-act button');
  const h = await head();
  // stopped before the first pass is drawn: the old results stay, dimmed; stopped during the exact check: the first
  // pass stays, current, and the status says it was not checked
  const stoppedOk = h.kind === 'stopped' || (h.kind === 'done' && /Stopped the exact check/.test(h.note) && !h.stale);
  check(
    'Stop kills the search and keeps the results',
    (await pg.evaluate(() => window.__killed)) > k0 && stoppedOk && h.loops > 0 && !(await busy(pg)),
    JSON.stringify(h),
  );
  await pg.click('#r-act button');
  await settle(pg);
  await expect
    .soft(runNote, '… and "Search again" runs it')
    .toHaveText(/^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/, { useInnerText: true });
  await expect.soft(loops, '… and "Search again" runs it: three results').toHaveCount(3);
  const r = await pg.evaluate(() => {
    const L = [...document.querySelectorAll('#results article')],
      key = (l) =>
        [...l.querySelectorAll('[data-member]')].map((m) => m.dataset.uma + ':' + m.dataset.outfit).join(' ');
    return L.map((l, i) => ({
      rank: l.querySelector('[data-rank]')?.textContent,
      same: !!l.querySelector('[data-same]'),
      mem: l.querySelectorAll('[data-member]').length,
      sameAs1: i > 0 && !l.querySelector('[data-same]') && key(l) === key(L[0]),
    }));
  });
  check('three results: ranked #1, #2, #3', r.map((x) => x.rank).join() === '#1,#2,#3', JSON.stringify(r));
  check(
    'a result with #1\'s loop says "Same loop as #1" instead of repeating the umas',
    r.some((x) => x.same) &&
      r.every((x) => (x.same ? x.mem === 0 : x.mem === 3 || x.mem === 4)) &&
      !r.some((x) => x.sameAs1),
    JSON.stringify(r),
  );
  // a result that throws while it is drawn (here the kept result with its first loop's outfits gone, as a malformed
  // report would be): the status line says so, as a failed search does, the column says why it is empty, the error is
  // logged; the next result draws
  await pg.evaluate(() => {
    const key = Object.keys(localStorage).find((k) => k.startsWith('llp:last:')) || '',
      kept = JSON.parse(localStorage.getItem(key) || 'null');
    kept.r.top[0].loop.outfits = [];
    localStorage.setItem(key, JSON.stringify(kept));
  });
  const errors0 = pg.errors.length;
  await pg.reload();
  const undrawn =
    'a result that throws while drawn: the status line says it could not be drawn, the column says so, the error is logged';
  await expect.soft(rhead, undrawn).toHaveAttribute('data-kind', 'error', { timeout: 10_000 });
  await expect.soft(runNote, `${undrawn}: the status`).toHaveText(/^The result could not be drawn: /);
  await expect.soft(loops, `${undrawn}: no result`).toHaveCount(0);
  await expect
    .soft(results, `${undrawn}: the column`)
    .toHaveText('The result could not be drawn.', { useInnerText: true });
  const drawErrors = pg.errors.splice(errors0);
  check(`${undrawn}: the error logged`, drawErrors.length > 0, JSON.stringify(drawErrors));
  await openEditor(pg, 'cards');
  await pg.locator('#cards button[aria-pressed]').nth(6).click();
  await settle(pg);
  const next = '… and the next result draws';
  await expect.soft(rhead, next).toHaveAttribute('data-kind', 'done');
  await expect.soft(loops, `${next}: results`).not.toHaveCount(0);
  await expect.soft(results, `${next}: not dimmed`).not.toContainClass('stale');
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
