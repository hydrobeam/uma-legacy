// What the browser suite's groups share (Playwright Test runs them: playwright.config.mjs, test/groups.spec.mjs): the page
// under test, the checks, the time profile, opening a page with the worker hooks, waits on the page's own state, the
// in-page audits (pill tracks, sideways scroll), the career page's fixture, and the data the checks read from
// the repo.
import fs from 'node:fs';
import { fileURLToPath } from 'node:url';
import { expect, test } from 'playwright/test';
import { imageCatalog } from '../vite/images.mjs';
import { SOLVER, installSolver } from './solverreplay.mjs';

// ---------- the page under test and the run's mode
/** The planner's address: LLP_URL, or webapp/dist served on a free local port (playwright.config.mjs sets both). */
export const PAGE_URL = process.env.LLP_PAGE;
/** The loop planner's address (the career planner is the front page, PAGE_URL itself). */
export const LOOP_URL = `${PAGE_URL.replace(/\/?$/, '/')}loop`;
/** The fast tier (LLP_QUICK=1; ./build.sh testq): career stops at its one-run answer. */
export const QUICK = !!process.env.LLP_QUICK;
const REPO_ROOT = new URL('../../', import.meta.url);
/** Every test context: the local server's certificate is self-signed (vite preview, HTTP/2 over TLS as the host). */
export const CONTEXT = { ignoreHTTPSErrors: true };
/** A file of the repository (data the page was built from, the solver's sources, validation outputs) as a file URL. */
export const repoUrl = (relative) => new URL(relative, REPO_ROOT);

// ---------- the checks: each a step of its group's test, so the report lists it; a failed one is a soft failure (the
// group goes on, the test fails at its end)
/** Where the check was called from: the first stack frame outside this file (the step's place in the report). */
function caller() {
  const here = new URL(import.meta.url).pathname;
  for (const line of new Error().stack.split('\n').slice(1)) {
    const m = line.match(/(?:file:\/\/)?(\/[^():]+):(\d+):(\d+)\)?$/);
    if (m && m[1] !== here) return { file: m[1], line: +m[2], column: +m[3] };
  }
}
export const check = (name, ok, detail = '') =>
  test.step(
    name,
    () => {
      expect.soft(ok, `${name}${detail ? ' · ' + detail : ''}`).toBeTruthy();
    },
    { location: caller() },
  );
// Whether the build carries the game art: a clone without game files builds with initials in place of pictures, so
// checks of the art itself are skipped.
const ASSETS = new URL('../dist/assets/', import.meta.url);
export const ART =
  !!process.env.LLP_URL ||
  (() => {
    // the character icons `images` extracts
    try {
      const images = fs.readdirSync(ASSETS).find((n) => n.startsWith('img.'));
      return !!images && fs.readdirSync(new URL(`${images}/uma`, ASSETS)).length > 0;
    } catch {
      return false;
    }
  })();
/** A check that did not run, with why: a skipped step, a `skipped check` annotation on its test, and a line on stdout. */
export function skip(name, why) {
  test.info().annotations.push({ type: 'skipped check', description: `${name} · ${why}` });
  console.log(`  skip ${name} · ${why}`);
  return test.step(
    name,
    (step) => {
      step.skip(true, why);
    },
    { location: caller() },
  );
}
export function artCheck(name, ok, detail = '') {
  if (ART) check(name, ok, detail);
  else skip(name, 'no game art in this build');
}

// ---------- LLP_PROF=file: where each group's time goes (new contexts and page loads, waits for searches and plans, fixed
// sleeps, and the solver's own time per job), one JSON line per group appended to the file
export const PROF = process.env.LLP_PROF;
/** @type {Record<string, number|string[]>|null} the running group's profile (LLP_PROF), else null: milliseconds per key
 * (`timed`'s keys, `solver:<job kind>`) and each key's count under `<key>#`; `solverLog` lists each job's time. A
 * worker runs one group at a time. */
let groupProf = null;
/**
 * Runs one group (groups.spec.mjs): its checks are steps of the running test; a throw ends the group and fails its
 * test. Every context it makes reaches the solver as `solver` says (solverreplay.mjs: replayed or live, recorded under
 * LLP_SOLVER=live); the test's `solver` annotation counts the replies replayed and recorded, and a job with no stored
 * reply fails the test with the job named. With LLP_PROF, appends the group's profile line: browser, group, wall and
 * start (ms from the run's start).
 * @param {string} group @param {(browser: object) => Promise<void>} run @param {object} browser
 * @param {import('./solverreplay.mjs').SolverMode} solver
 */
export async function runGroup(group, run, browser, solver) {
  groupProf = PROF ? {} : null;
  const t0 = Date.now();
  // the browser is the worker's, shared by the groups it runs: its newContext is wrapped for this group only
  const newContext = browser.newContext.bind(browser),
    /** @type {import('./solverreplay.mjs').SolverTally} */
    tally = { replayed: 0, recorded: 0, differs: 0, missing: [], held: [], stopped: [] };
  browser.newContext = async (options) => {
    const ctx = await newContext(options);
    await installSolver(ctx, solver, tally);
    return ctx;
  };
  try {
    await run(browser);
  } finally {
    browser.newContext = newContext;
    const how = SOLVER === 'live' ? 'live, recorded' : solver.replay ? 'replayed' : 'live';
    test.info().annotations.push({
      type: 'solver',
      description: `${how}: ${tally.replayed} replies replayed, ${tally.recorded} recorded${tally.differs ? `, ${tally.differs} differing from the stored reply` : ''}${tally.stopped.length ? `; jobs the page stopped, never recorded: ${tally.stopped.join(' | ')}` : ''}${tally.held.length ? `; background jobs left unanswered (not recorded): ${tally.held.join(' | ')}` : ''}`,
    });
    if (PROF) {
      const line = {
        browser: browser.browserType().name(),
        group,
        wall: Date.now() - t0,
        start: t0 - +process.env.LLP_T0,
      };
      fs.appendFileSync(PROF, JSON.stringify({ ...line, ...groupProf }) + '\n');
    }
  }
  // (after the group, so its checks still report: the first missing job is usually the cause of the failed ones)
  if (tally.missing.length)
    throw new Error(
      `${tally.missing.length} solver jobs have no stored reply (a solver, data or page change: LLP_SOLVER=live records them):\n${tally.missing.slice(0, 5).join('\n')}`,
    );
}
/** Adds the time until `promise` settles to the group's profile under `key` (and counts it); returns the promise. */
const timed = (key, promise, prof = groupProf) => {
  if (!prof) return promise;
  const t0 = Date.now();
  return Promise.resolve(promise).finally(() => {
    prof[key] = (prof[key] || 0) + Date.now() - t0;
    prof[key + '#'] = (prof[key + '#'] || 0) + 1;
  });
};
async function profile(ctx, pg) {
  // the solver's time per job (the worker reports it with each result), fixed sleeps
  const prof = groupProf;
  if (!prof) return;
  await ctx.exposeBinding('__llpSolved', (_, kind, ms) => {
    prof['solver:' + kind] = (prof['solver:' + kind] || 0) + ms;
    prof['solver:' + kind + '#'] = (prof['solver:' + kind + '#'] || 0) + 1;
    prof.solverLog ||= [];
    prof.solverLog.push(`${kind} ${ms}`);
  });
  await ctx.addInitScript(() => {
    const kinds = new WeakMap(); // worker -> its jobs' kinds by job id
    const hooks = window.__workerHooks;
    hooks.made.push((w) => kinds.set(w, new Map()));
    hooks.post.push((m, w) => {
      if (m?.type === 'run')
        kinds
          .get(w)
          .set(m.id, m.job.cmd ? m.job.cmd + (m.job.next ? '.next' : '') + (m.job.compare ? '.compare' : '') : 'loop');
    });
    hooks.message.push((d, w) => {
      if (d?.type === 'result') window.__llpSolved(kinds.get(w).get(d.id) || '?', Math.round(d.ms));
    });
  });
  const sleep = pg.waitForTimeout.bind(pg);
  pg.waitForTimeout = (ms) => timed('sleep', sleep(ms), prof);
}

// ---------- opening a page
// The settings editors are dialogs the page's buttons open: Umas and Support cards (the setup panel's, both pages), and
// on the career page the Veteran Umamusume dialog. A group opens one by its button before it works the controls inside
// (openEditor), and closes it by its Done before it works the page behind it (closeEditor), as a visitor does.
const EDITORS = {
  umas: { button: '#o-umas', id: 'set-umas' },
  cards: { button: '#o-cards', id: 'set-cards' },
  vets: { button: '#v-toggle', id: 'set-vets' },
};
/** Opens the editor `which` ('umas', 'cards' or 'vets') by its button, unless it is open. */
export async function openEditor(pg, which) {
  const { button, id } = EDITORS[which],
    editor = pg.locator(`#${id}`);
  if ((await editor.getAttribute('open')) !== null) return;
  await pg.click(button);
  await expect(editor).toHaveAttribute('open');
}
/** Closes the editor `which` by its Done, if it is open. */
export async function closeEditor(pg, which) {
  const editor = pg.locator(`#${EDITORS[which].id}`);
  if ((await editor.getAttribute('open')) === null) return;
  await editor.locator('.dclose').click();
  await expect(editor).not.toHaveAttribute('open');
}
// the page's errors and error-level console lines collect in pg.errors (web-font fetch aborts are network noise)
function collectErrors(pg) {
  pg.errors = [];
  pg.on('pageerror', (e) => pg.errors.push(String(e)));
  pg.on('console', (m) => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));
}
/**
 * The planner in a fresh context (fresh storage), its first search done, with the worker hooks: window.__made and
 * __killed count workers made and terminated, __runs lists the jobs sent, __results their answers, and while __hold is
 * set the worker's replies wait (a search stays running however fast it is) until __release() (solverreplay.mjs's
 * stand-in holds them).
 * @param {object} [o]
 * @param {number} [o.w] @param {number} [o.h]  the viewport, CSS px
 * @param {boolean} [o.mobile]  a phone (touch, mobile viewport; Chromium only)
 * @param {Function} [o.init]  an init script of the test's own, after the hooks
 * @param {(pg: object) => Promise<void>|void} [o.before]  runs before the page loads (routes, request listeners)
 * @param {'light'|'dark'} [o.scheme]  the colour scheme the browser reports
 */
export async function open(browser, { w = 1440, h = 900, mobile = false, init, before, scheme = 'light' } = {}) {
  const ctx = await timed(
    'ctx',
    browser.newContext({
      ...CONTEXT,
      viewport: { width: w, height: h },
      colorScheme: scheme,
      ...(mobile && browser.browserType().name() === 'chromium' ? { isMobile: true, hasTouch: true } : {}),
    }),
  );
  await ctx.addInitScript(() => {
    window.__made = 0;
    window.__killed = 0;
    window.__runs = []; // the jobs sent to the engine, in order
    window.__results = []; // the engine's answers (each job's `r`), in order
    const hooks = window.__workerHooks;
    hooks.made.push(() => {
      window.__made++;
    });
    hooks.terminate.push(() => {
      window.__killed++;
    });
    hooks.post.push((m) => {
      if (m?.type === 'run') window.__runs.push(m.job);
    });
    hooks.message.push((d) => {
      if (d?.type === 'result') window.__results.push(d.r);
    });
  });
  if (init) await ctx.addInitScript(init);
  const pg = await ctx.newPage();
  collectErrors(pg);
  await profile(ctx, pg);
  if (before) await before(pg);
  await timed('goto', pg.goto(LOOP_URL));
  await timed('settle.load', settle(pg));
  return pg;
}

// ---------- waits and reads on the planner page
export const busy = (pg) => pg.evaluate(() => document.getElementById('run-note').classList.contains('busy'));
// wait for the current (auto) search to finish. A setting change marks the status line busy at once (the search
// itself starts after a short pause), so no fixed sleep is needed; after a page load, the page's first job must have
// been sent (pages opened without open()'s worker hook have no __runs and skip that part). timeout in ms.
export const settle = (pg, timeout) => timed('settle', searchSettled(pg, timeout));
async function searchSettled(pg, timeout = 180000) {
  await pg.waitForFunction(
    () => {
      const n = document.getElementById('run-note');
      // a search has run, or (no job sent) the saved result of these settings shows as done (a reload's)
      return (
        (window.__runs === undefined || window.__runs.length > 0 || /^Checked [\d,]+ decks/.test(n.textContent)) &&
        !n.classList.contains('busy') &&
        /Checked [\d,]+ decks|example/i.test(n.textContent)
      );
    },
    null,
    { timeout },
  );
}
export const jobSent = (pg, fn, arg) => pg.waitForFunction(fn, arg); // a search's job has gone to the engine (its result is not awaited)
export const lastJob = (pg, scenario) =>
  jobSent(pg, (s) => window.__runs.length > 0 && window.__runs.at(-1).scenario === s, scenario);
export const kind = (pg, k) => pg.waitForFunction((k) => document.getElementById('rhead').dataset.kind === k, k); // the status line's state
export const dialog = (pg, id) => pg.waitForSelector(`#${id}[open]`);
// a condition the page reaches on its own (an observer, a smooth scroll, a resize handler): true once it holds, false
// if it never does within the time (ms)
export const until = (pg, fn, arg, timeout = 3000) =>
  pg.waitForFunction(fn, arg, { timeout }).then(
    () => true,
    () => false,
  );
export async function navClick(pg, sel) {
  await Promise.all([pg.waitForEvent('load'), pg.click(sel)]);
} // a click that reloads the page
export const note = (pg) => pg.locator('#run-note').innerText();
export const odds = (pg) => pg.evaluate(() => document.querySelector('#results .loop .odds')?.textContent || '');
// the page's Global-era data by its old element id ('data-cards', ...), read here from what the build reads
// (vite/pagedata.mjs '#page-data': page.json's lists, the image catalog, the skill icons), which a test may hand to the
// page as an argument
const readJson = (url) => JSON.parse(fs.readFileSync(url, 'utf8'));
const pageLists = readJson(repoUrl('pkg/loopopt/data/global/page.json'));
const imgFile = (name) => new URL(`../img/${name}`, import.meta.url);
const icons = (name) => (fs.existsSync(imgFile(name)) ? readJson(imgFile(name)) : {});
const PAGE_DATA = {
  'data-cards': pageLists.cards,
  'data-umas': pageLists.umas,
  'data-skills': pageLists.skills,
  'data-img': imageCatalog(fileURLToPath(imgFile(''))),
  'data-skillicon': { white: icons('skill_icons.json'), gold: icons('skill_icons_gold.json') },
};
export const json = async (_pg, id) => PAGE_DATA[id];
export const scen = (pg) => pg.getAttribute('#x-scen', 'data-value'); // the chosen scenario
export async function pickScen(pg, v) {
  await pg.click('#x-scen');
  await pg.click(`#scen-menu .stile[data-v="${v}"]`);
}
// no transition or finite animation still running (a fade, a theme change): what a fixed short sleep waited for
export const still = (pg) =>
  until(pg, () =>
    document.getAnimations().every((a) => a.playState !== 'running' || a.effect?.getTiming().iterations === Infinity),
  );

// ---------- in-page audits (pg.evaluate(PILLS) and so on): each returns what is wrong, one line per fault
// every visible pill track (the segmented controls and the veteran card's tabs): its segments fill it flush, the full
// inner height, no gap at its ends or between them, the track a pill that clips them to its own curve, none of them drawn
// with the game's pill sprite (whose fixed-width caps don't fit other heights). Distances in CSS px, 0.6 px tolerance.
export const PILLS = () => {
  const bad = [],
    near = (a, b) => Math.abs(a - b) <= 0.6;
  for (const t of document.querySelectorAll('.seg, .udtabs')) {
    const r = t.getBoundingClientRect(),
      cs = getComputedStyle(t);
    if (!r.width || !r.height || cs.visibility === 'hidden') continue;
    const id = t.id ? '#' + t.id : t.getAttribute('aria-label') || t.getAttribute('aria-labelledby') || t.className;
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
      if (!near(q.top, inner.t) || !near(q.bottom, inner.b))
        bad.push(
          `${id} #${i}: ${(q.top - inner.t).toFixed(1)} / ${(inner.b - q.bottom).toFixed(1)} px short of top / bottom`,
        );
      if (i === 0 && !near(q.left, inner.l)) bad.push(`${id}: ${(q.left - inner.l).toFixed(1)} px gap at its start`);
      if (i === segs.length - 1 && !near(q.right, inner.r))
        bad.push(`${id}: ${(inner.r - q.right).toFixed(1)} px gap at its end`);
      if (i > 0 && !near(q.left, segs[i - 1].getBoundingClientRect().right)) bad.push(`${id}: a gap before #${i}`);
      if (getComputedStyle(s).borderImageSource !== 'none') bad.push(`${id} #${i}: drawn with a border-image`);
    });
  }
  return bad;
};
// the card list's pictures: their widths (one, loaded or not) and how many overflow their tile
export const CARD_ART = () => {
  const s = new Set();
  let over = 0;
  for (const t of document.querySelectorAll('#cards .ct')) {
    const c = t.querySelector('.cav').getBoundingClientRect();
    s.add(Math.round(c.width));
    if (c.width > t.getBoundingClientRect().width + 0.5) over++;
  }
  return { sizes: [...s], over };
};
// what scrolls sideways: the page, an open dialog's body, or an open dialog that leaves the screen
export const SIDEWAYS = () => {
  const d = document.documentElement,
    out = [];
  if (d.scrollWidth > d.clientWidth) out.push(`page ${d.scrollWidth} > ${d.clientWidth}`);
  for (const g of document.querySelectorAll('dialog[open]')) {
    const r = g.getBoundingClientRect();
    if (r.left < -0.5 || r.right > d.clientWidth + 0.5) out.push(`#${g.id} leaves the screen`);
    for (const s of g.querySelectorAll('.dlg, .gmdbody'))
      if (s.scrollWidth > s.clientWidth + 1)
        out.push(`#${g.id} scrolls sideways (${s.scrollWidth} > ${s.clientWidth})`);
  }
  return out;
};

// ---------- the career planner (the front page)
/**
 * The career page with its run hooks and the helpers the career groups share.
 * @typedef {object} CareerPage
 * @property {object} ctx  the browser context (close it at the end)
 * @property {object} pg  the page (its errors in pg.errors)
 * @property {() => Promise<string>} note  the status line's text
 * @property {() => Promise<void>} done  the run an action started has ended (returns at once if none started)
 * @property {() => Promise<void>} planned  the status line says the plan is done ("Planned in")
 * @property {(label: string) => Promise<void>} ask  answers the ask dialog with the button so labelled
 * @property {(pre: string) => Promise<{ok: boolean, phases: string[], detail: string}>} oneAnswer  whether the run
 *   just ended showed one answer (see its comment); `pre` is what showsNow() read before the run
 * @property {() => Promise<string>} showsNow  the answer on show: the trainee's name and the veteran you got's sparks
 * @property {() => Promise<void>} vetsOpen  opens the Veteran Umamusume dialog (where your veterans live)
 * @property {() => Promise<void>} vetsShut  shuts it, to reach the page again
 * @property {() => Promise<void>} vetsSeen  opens and shuts it once if it is shut (it draws its cards when open)
 * @property {(key: string) => Promise<void>} node  opens the popup of the tree's portrait `data-k=key`
 * @property {(name: string|RegExp) => object} inNode  a button in that popup, by its name
 * @property {() => Promise<void>} closeNode  shuts that popup if it is open
 * @property {(stars: number) => Promise<void>} simStars  the veteran you got's Uma Stan at so many stars (added first)
 * @property {() => Promise<void>} importDump  imports the umadump test file's 12 veterans
 * @property {() => Promise<void>} saved  an edit's save has landed (veterans/store.ts saves after the next frame)
 * @property {() => Promise<void>} simOpen  opens Try a run's popup (the veteran card) if it is shut
 * @property {() => Promise<void>} simShut  shuts it if it is open
 */
// The career planner in a fresh context, its first (empty) run done. window.__careerJobs and __careerResults list the
// jobs sent and their results; window.__seq records what the page shows each time it changes; a test sets
// window.__rollPatch to stand in for a roll's saving, window.__planPatch for fields of a plan's report (or
// window.__planEdit, a function given the report to change in place), window.__checkPatch for fields of its check's.
// Each career group (career, careerpool, careervets, careergen, careerrun, careerborrow) runs from its own fresh page,
// so they run in parallel.
/** @returns {Promise<CareerPage>} */
export async function careerPage(b) {
  const ctx = await b.newContext({ ...CONTEXT, viewport: { width: 1440, height: 1000 } });
  await ctx.addInitScript(() => {
    // the setup chosen open, as these tests work its controls after reloads with veterans (unchosen, it would open folded
    // then: careerphone checks that)
    try {
      if (localStorage.getItem('llp:setup-folded') === null) localStorage.setItem('llp:setup-folded', '0');
    } catch {
      // storage blocked: the page opens with its own default
    }
    window.__careerJobs = [];
    window.__careerResults = [];
    // (the plan's robustness check, `next.robust` follow-up jobs, kept apart: the tests' plan jobs and results are the rest)
    window.__checkJobs = [];
    window.__checkResults = [];
    const hooks = window.__workerHooks;
    // (`__rollPatch`: a test's stand-in for a roll's saving, put over the solver's before the page reads it)
    hooks.message.push((d) => {
      if (d?.type !== 'result') return;
      if (window.__rollPatch && d.r?.next?.roll) Object.assign(d.r.next.roll, window.__rollPatch);
      // (`__planPatch`: a test's stand-in for fields of a plan's report, put over the solver's the same way)
      if (window.__planPatch && d.r?.next?.options) Object.assign(d.r.next, window.__planPatch);
      if (window.__planEdit && d.r?.next?.options) window.__planEdit(d.r.next);
      if (window.__checkPatch && d.r?.next?.robust && !d.r.next.options)
        Object.assign(d.r.next.robust, window.__checkPatch);
      if (d.r?.next?.robust && !d.r.next.options) window.__checkResults.push(d.r);
      else window.__careerResults.push(d.r);
    });
    hooks.post.push((m) => {
      if (m?.type === 'run') (m.job?.next?.robust ? window.__checkJobs : window.__careerJobs).push(m.job);
    });
    // what the page shows, each time it changes: busy or not, the phase named, the trainee on show, the veteran you
    // got (her sparks), whether the old answer is dimmed
    window.__seq = [];
    const snap = () => {
      const n = document.getElementById('run-note');
      if (!n) return;
      const s = {
        busy: !!n.querySelector('.spin, .runner'),
        phase: /Planning your next career…/.test(n.textContent)
          ? 'plan'
          : /Searching careers…/.test(n.textContent)
            ? 'search'
            : '',
        who: document.querySelector('#tree .pn.trainee .nm')?.textContent || '',
        draft: document.querySelector('#simulation .ud')
          ? 'v:' +
            [...document.querySelectorAll('#simulation .simskills .vskb')]
              .map((b) => b.getAttribute('aria-label'))
              .join('|')
          : '',
        stale: !!document.querySelector('#results.stale'),
      };
      const k = JSON.stringify(s);
      if (window.__seq.at(-1)?.k !== k) window.__seq.push({ ...s, k });
    };
    new MutationObserver(snap).observe(document, {
      subtree: true,
      childList: true,
      characterData: true,
      attributes: true,
      attributeFilter: ['class'],
    });
  });
  // one answer per run: while the run is busy the page shows the answer it had before it (`pre`, none at first), dimmed
  // if there is one, and names the phase; the answer it ends on is the only other one drawn
  const oneAnswer = async (pre) => {
    const seq = await pg.evaluate(() => window.__seq),
      busy = seq.filter((s) => s.busy),
      end = seq.at(-1);
    const shows = (s) => s.who + '/' + s.draft,
      phases = busy.map((s) => s.phase).filter((p, i, a) => p && p !== a[i - 1]);
    // dimmed from when its job starts (the pause before it dims nothing) to the answer
    const dim = busy.findIndex((s) => s.stale);
    const ok =
      busy.length > 0 &&
      !end.busy &&
      busy.every((s) => shows(s) === pre) &&
      (!busy.some((s) => s.draft) || (dim >= 0 && busy.slice(dim).every((s) => !s.draft || s.stale))) &&
      end.draft !== '' &&
      !end.stale &&
      seq.slice(seq.indexOf(busy.at(-1)) + 1).every((s) => shows(s) === shows(end));
    return {
      ok,
      phases,
      detail: `phases ${phases.join(' > ')} · before ${pre.slice(0, 60)} · busy shows ${[...new Set(busy.map(shows))].map((x) => x.slice(0, 40)).join(' | ')} · end ${shows(end).slice(0, 60)}`,
    };
  };
  const showsNow = () =>
    pg.evaluate(
      () =>
        (document.querySelector('#tree .pn.trainee .nm')?.textContent || '') +
        '/' +
        (document.querySelector('#simulation .ud')
          ? 'v:' +
            [...document.querySelectorAll('#simulation .simskills .vskb')]
              .map((b) => b.getAttribute('aria-label'))
              .join('|')
          : ''),
    );
  const pg = await ctx.newPage();
  collectErrors(pg);
  await profile(ctx, pg);
  const note = () => pg.textContent('#run-note');
  // the run an action started has ended. A change puts the spinner up at once (the search itself starts after a short
  // pause), so no sleep past the pause is needed; an action that starts no run returns at once
  const done = () =>
    timed(
      'done',
      pg.waitForFunction(
        () => {
          const n = document.getElementById('run-note');
          return !n.querySelector('.spin, .runner') && n.textContent.length > 0;
        },
        null,
        { timeout: 180000 },
      ),
    );
  const planned = () =>
    timed(
      'done',
      pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, {
        timeout: 300000,
      }),
    );
  const ask = async (label) => {
    await dialog(pg, 'ask-dlg');
    await pg.locator('#ask-btns button', { hasText: label }).click();
  };
  // the tree is portraits; a portrait opens its node popup (sparks, G1 wins, choose or hand back the place)
  // your veterans live in the Veteran Umamusume dialog: open it to reach them, shut it to reach the page again
  const vetsOpen = () => openEditor(pg, 'vets');
  const vetsShut = () => closeEditor(pg, 'vets');
  const vetsSeen = async () => {
    if (!(await pg.locator('#set-vets[open]').count())) {
      await vetsOpen();
      await vetsShut();
    }
  };
  const node = async (key) => {
    await vetsShut();
    await pg.click(`#tree [data-k="${key}"]`);
    await dialog(pg, 'node-dlg');
  };
  const inNode = (name) => pg.locator('#node-dlg').getByRole('button', { name });
  const closeNode = async () => {
    if (await pg.locator('#node-dlg[open]').count()) await pg.keyboard.press('Escape');
  };
  // the veteran you got: Uma Stan at n stars (added first if she lacks it)
  // Try a run's popup (the veteran card and her comparison)
  const simIsOpen = () => pg.evaluate(() => !!document.getElementById('sim-dlg')?.open);
  const simOpen = async () => {
    if (!(await simIsOpen())) await pg.click('#sim-try');
    await pg.waitForFunction(() => document.getElementById('sim-dlg').open);
  };
  const simShut = async () => {
    if (await simIsOpen()) await pg.locator('#sim-dlg').getByRole('button', { name: 'Close', exact: true }).click();
  };
  const simStars = async (n) => {
    await simOpen();
    const loc = pg.locator('[data-k="sim:s20159"]');
    if (!(await loc.count())) {
      await pg.locator('#simulation .vskadd').click();
      await pg.locator('#add-pop .addsk', { hasText: 'Uma Stan' }).click();
    }
    for (let i = 0; i < 3; i++) {
      if (+(await loc.getAttribute('aria-label')).match(/(\d) star/)[1] === n) return;
      await loc.click();
    }
  };
  // an edit's save lands after the next frame (veterans/store.ts): what a check reads from storage straight after one
  const saved = () => pg.evaluate(() => new Promise((r) => requestAnimationFrame(() => setTimeout(r))));
  // the umadump test file's 12 veterans
  const importDump = async () => {
    await pg.locator('#v-file').setInputFiles(repoUrl('val/umadump/U_trained_chara_data.json').pathname);
    await pg.waitForFunction(() => document.getElementById('v-msg').textContent);
  }; // the file is read asynchronously
  await pg.goto(PAGE_URL);
  await done();
  return {
    ctx,
    pg,
    note,
    done,
    planned,
    ask,
    oneAnswer,
    showsNow,
    vetsOpen,
    vetsShut,
    vetsSeen,
    node,
    inNode,
    closeNode,
    simStars,
    importDump,
    saved,
    simOpen,
    simShut,
  };
}

// ---------- data the checks read from the repo
// the scenario tiles' "Hint rate ×" lines as the solver's hint_base gives them (a multiple of URA Finale's), by scenario:
// research/results.json's section 7 fit rounded to 3 decimals, as pkg/loopopt/build.rs reads it (Grand Masters: the
// pooled fit; an unknown scenario reads as Unity Cup)
export function hintRates() {
  const values = JSON.parse(fs.readFileSync(repoUrl('research/results.json'), 'utf8')).values;
  const fitted = { unity: 'unity_cup', gc: 'grand_concert', ura: 'ura_finale', trackblazer: 'trackblazer' };
  const of = (s) =>
    +values[
      s === 'grandmasters' ? '7.one_base_and_one_d_base' : `7.specialty_base_${fitted[s] ?? 'unity_cup'}`
    ].toFixed(3);
  return Object.fromEntries(
    ['unity', 'gc', 'ura', 'trackblazer', 'grandmasters'].map((s) => [
      s,
      `Hint rate ×${(of(s) / of('ura')).toFixed(2)}`,
    ]),
  );
}
