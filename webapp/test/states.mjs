import { check, open, busy, settle, kind, until, note, json, AUDIT } from './helpers.mjs';

export default async function states(b) {
  // what the status line says when the settings can't be searched, and the results on screen meanwhile (kept, dimmed)
  // (window.__trap: the next jobs reach the worker without their job, so they fail inside it, as a solver trap would)
  // biome-ignore lint/complexity/useArrowFunction: the Worker stand-in is called with `new`, which an arrow function cannot be
  const pg = await open(b, { init: () => { const W0 = window.Worker; window.Worker = function (...a) { const w = new W0(...a), pm = w.postMessage.bind(w);
    w.postMessage = (m, ...r) => pm(m && m.type === 'run' && window.__trap ? { ...m, job: null } : m, ...r); return w; }; } });
  const head = () => pg.evaluate(() => ({ kind: document.getElementById('rhead').dataset.kind, note: document.getElementById('run-note').textContent, act: document.getElementById('r-act').innerText,
    loops: document.querySelectorAll('#results .loop').length, stale: document.getElementById('results').classList.contains('stale') }));
  // a job that fails inside the worker: the search ends on the error (it used to spin for ever), the results stay, the
  // worker goes, and the next search runs on a fresh one
  const kt = await pg.evaluate(() => { window.__trap = true; return window.__killed; });
  await pg.locator('#cards .ct').nth(6).click();
  const trapped = await until(pg, () => document.getElementById('rhead').dataset.kind === 'error', null, 15000); let h = await head();
  check('a job that fails in the worker ends the search on its error, the results kept, the worker restarted', trapped && /^Not searched: the solver stopped with an error/.test(h.note) && h.loops > 0 && h.stale
    && (await pg.evaluate(() => window.__killed)) > kt, JSON.stringify(h));
  if (b.browserType().name() === 'chromium') { const cf = (await pg.evaluate(AUDIT)).filter(x => /^p\.runnote/.test(x)); check('contrast: the error status line meets WCAG AAA', !cf.length, cf.join(' | ')); }
  await pg.evaluate(() => { window.__trap = false; }); await pg.locator('#cards .ct').nth(6).click(); await settle(pg);
  check('… and the next search runs', /^Checked [\d,]+ decks/.test(await note(pg)));
  while (await pg.locator('#targets .x').count()) await pg.locator('#targets .x').first().click();
  await kind(pg, 'empty'); h = await head();
  check('no targets: a neutral status with a button that loads the example targets; the old result stays, dimmed', h.kind === 'empty' && /Add a target skill/.test(h.note) && /8\/8 example/.test(h.act) && h.loops > 0 && h.stale, JSON.stringify(h));
  await pg.click('#r-act button'); await settle(pg); h = await head();
  check('… the button loads them and the search runs', (await pg.locator('#targets .tgt').count()) === 8 && h.kind === 'done' && /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(h.note) && !h.stale, JSON.stringify(h));
  await pg.click('#r-none'); await kind(pg, 'empty'); h = await head();
  check('no umas owned: a neutral status with "Mark all umas owned"', h.kind === 'empty' && /at least 3 umas/.test(h.note) && /Mark all umas owned/.test(h.act) && h.loops > 0, JSON.stringify(h));
  await pg.click('#r-act button'); await settle(pg);
  // an old export's "must include" mark ('req', since removed) loads as owned
  const u = (await json(pg, 'data-umas')).find(x => x.outfits.length > 1);
  await pg.click('#ex-open'); const ex = JSON.parse(await pg.inputValue('#ex-text'));
  ex.settings.outfits = { [u.outfits[0].id]: 'req' };
  await pg.fill('#ex-text', JSON.stringify(ex)); await pg.click('#ex-load'); await pg.click('#ex-close'); await settle(pg); h = await head();
  const tiles = await pg.evaluate((id) => [...document.querySelectorAll(`#roster .ut[data-uma="${id}"]`)].map(t => t.dataset.s), u.id);
  check('an old export\'s must-include mark loads as owned, and the search runs', tiles.every(x => x === 'own') && h.kind === 'done' && /^Checked [\d,]+ decks/.test(h.note), tiles.join(',') + ' · ' + JSON.stringify(h));
  // Stop: a slow search (three results, one shared deck) is stopped; the results stay and the worker is killed;
  // "Search again" runs it to the end, and the three results are told apart
  const r0 = await pg.evaluate(() => window.__runs.length);
  await pg.click('#c-top [data-v="3"]'); await pg.click('#c-peruma');
  await pg.waitForFunction((n) => window.__runs.length > n && window.__runs.at(-1).top_decks === 3 && window.__runs.at(-1).per_uma === false, r0);   // the slow search itself is running (not a job from the first change alone)
  check('while searching: the results are marked busy and a Stop button shows', await pg.evaluate(() => document.getElementById('results').getAttribute('aria-busy') === 'true') && (await pg.locator('#r-act button').innerText()) === 'Stop');
  const k0 = await pg.evaluate(() => window.__killed); await pg.click('#r-act button'); h = await head();
  // stopped before the first pass is drawn: the old results stay, dimmed; stopped during the exact check: the first
  // pass stays, current, and the status says it was not checked
  const stoppedOk = h.kind === 'stopped' || (h.kind === 'done' && /Stopped the exact check/.test(h.note) && !h.stale);
  check('Stop kills the search and keeps the results', (await pg.evaluate(() => window.__killed)) > k0 && stoppedOk && h.loops > 0 && !(await busy(pg)), JSON.stringify(h));
  await pg.click('#r-act button'); await settle(pg);
  check('… and "Search again" runs it', /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)) && (await pg.locator('#results .loop').count()) === 3);
  const r = await pg.evaluate(() => { const L = [...document.querySelectorAll('#results .loop')], key = (l) => [...l.querySelectorAll('.mem')].map(m => m.dataset.uma + ':' + m.dataset.outfit).join(' ');
    return L.map((l, i) => ({ rank: l.querySelector('.rank')?.textContent, same: !!l.querySelector('.same'), mem: l.querySelectorAll('.mem').length,
      sameAs1: i > 0 && !l.querySelector('.same') && key(l) === key(L[0]) })); });
  check('three results: ranked #1, #2, #3', r.map(x => x.rank).join() === '#1,#2,#3', JSON.stringify(r));
  check('a result with #1\'s loop says "Same loop as #1" instead of repeating the umas', r.some(x => x.same) && r.every(x => x.same ? x.mem === 0 : x.mem === 3 || x.mem === 4) && !r.some(x => x.sameAs1), JSON.stringify(r));
  check('no page errors', !pg.errors.length, pg.errors.join(' | '));
  await pg.context().close();
}
