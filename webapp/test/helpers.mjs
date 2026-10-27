// What the browser suite's groups share (webapp/nodebrowser.mjs runs them): the page under test, the check log and its
// counts, the time profile, opening a page with the worker hooks, waits on the page's own state, the in-page audits
// (contrast, pill tracks, sideways scroll), the career page's fixture, and the data the checks read from the repo.
import { AsyncLocalStorage } from 'node:async_hooks';
import fs from 'node:fs';
import { serveDist } from '../serve.mjs';

// ---------- the page under test and the run's mode
const local = process.env.LLP_URL ? null : await serveDist();
/** The planner's address: LLP_URL, or webapp/dist served on a free local port. */
export const PAGE_URL = process.env.LLP_URL || local.url;
/** The fast tier (`quick`; ./build.sh testq): the career group stops at its one-run answer. */
export const QUICK = process.argv.slice(2).includes('quick');
const REPO_ROOT = new URL('../../', import.meta.url);
/** A file of the repository (data the page was built from, the solver's sources, validation outputs) as a file URL. */
export const repoUrl = (relative) => new URL(relative, REPO_ROOT);

// ---------- the check log
/** The run's check counts, over every browser and group. */
export const tally = { passed: 0, failed: 0, skipped: 0 };
/**
 * @typedef {object} GroupLog  the running group's record (groups run in parallel; each prints when it ends)
 * @property {string[]} lines  its check lines, in order
 * @property {Record<string, number|string[]>|null} prof  where its time went (LLP_PROF), else null: milliseconds per
 *   key (`timed`'s keys, `solver:<job kind>`) and each key's count under `<key>#`; `solverLog` lists each job's time
 */
/** @type {AsyncLocalStorage<GroupLog>} */
export const groupLog = new AsyncLocalStorage();
export const say = (line) => {
  const log = groupLog.getStore();
  if (log) log.lines.push(line);
  else console.log(line);
};
export const check = (name, ok, detail = '') => {
  if (ok) tally.passed++;
  else tally.failed++;
  say(`${ok ? '  ok  ' : '  FAIL'} ${name}${detail ? ' · ' + detail : ''}`);
};
// Whether the build carries the game art: a clone without game files builds with initials in place of pictures, so
// checks of the art itself are skipped and the contrast audits leave out the initials that stand in for card art.
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
export function artCheck(name, ok, detail = '') {
  if (ART) {
    check(name, ok, detail);
    return;
  }
  tally.skipped++;
  say(`  skip ${name} · no game art in this build`);
}
export const artless = (fails) => (ART ? fails : fails.filter((x) => !/^span\.cav\b/.test(x)));

// ---------- LLP_PROF=file: where each group's time goes (new contexts and page loads, waits for searches and plans, fixed
// sleeps, contrast audits, and the solver's own time per job), one JSON line per group appended to the file
export const PROF = process.env.LLP_PROF;
const groupProfile = () => groupLog.getStore()?.prof;
/** Adds the time until `promise` settles to the group's profile under `key` (and counts it); returns the promise. */
const timed = (key, promise, prof = groupProfile()) => {
  if (!prof) return promise;
  const t0 = Date.now();
  return Promise.resolve(promise).finally(() => {
    prof[key] = (prof[key] || 0) + Date.now() - t0;
    prof[key + '#'] = (prof[key + '#'] || 0) + 1;
  });
};
async function profile(ctx, pg) {
  // the solver's time per job (the worker reports it with each result), fixed sleeps, audits
  const prof = groupProfile();
  if (!prof) return;
  await ctx.exposeBinding('__llpSolved', (_, kind, ms) => {
    prof['solver:' + kind] = (prof['solver:' + kind] || 0) + ms;
    prof['solver:' + kind + '#'] = (prof['solver:' + kind + '#'] || 0) + 1;
    prof.solverLog ||= [];
    prof.solverLog.push(`${kind} ${ms}`);
  });
  await ctx.addInitScript(() => {
    const W = window.Worker;
    // biome-ignore lint/complexity/useArrowFunction: the Worker stand-in is called with `new`, which an arrow function cannot be
    window.Worker = function (...a) {
      const w = new W(...a),
        kinds = new Map(),
        pm = w.postMessage.bind(w);
      w.postMessage = (m, ...r) => {
        if (m && m.type === 'run')
          kinds.set(
            m.id,
            m.job.cmd ? m.job.cmd + (m.job.next ? '.next' : '') + (m.job.compare ? '.compare' : '') : 'loop',
          );
        return pm(m, ...r);
      };
      w.addEventListener('message', (e) => {
        if (e.data?.type === 'result') window.__llpSolved(kinds.get(e.data.id) || '?', Math.round(e.data.ms));
      });
      return w;
    };
  });
  const sleep = pg.waitForTimeout.bind(pg);
  pg.waitForTimeout = (ms) => timed('sleep', sleep(ms), prof);
  const evaluate = pg.evaluate.bind(pg);
  pg.evaluate = (fn, ...a) => (fn === AUDIT ? timed('audit', evaluate(fn, ...a), prof) : evaluate(fn, ...a));
}

// ---------- opening a page
// The settings editors (Target skills, Umas, Support cards; on the career page also your veterans) are dialogs the setup
// panel's buttons open. editorAware: an action on a control inside one of them opens its dialog first, and an action on
// the page outside any open dialog shuts them, so the checks of the editors' behaviour read as on a page that shows them
// (the buttons and dialogs themselves are checked on their own). Page actions and every locator's actions go through it.
const ACTS = [
  'click',
  'fill',
  'focus',
  'press',
  'hover',
  'type',
  'check',
  'uncheck',
  'dblclick',
  'selectOption',
  'tap',
  'innerText',
  'boundingBox',
  'scrollIntoViewIfNeeded',
];
function ensureEditor(el) {
  // (runs in the page)
  const editor = el.closest('dialog.setdlg'),
    openEditors = [...document.querySelectorAll('dialog.setdlg[open]')];
  if (editor) {
    if (!editor.open) {
      openEditors.forEach((x) => {
        x.close();
      });
      editor.showModal();
    }
  } else if (!el.closest('dialog[open]'))
    openEditors.forEach((x) => {
      x.close();
    });
}
function editorAware(pg) {
  // no budget of its own: a short one that ran out under load (a busy renderer answers slowly) left the dialog shut and
  // the action then waited its 30 s for a tile it could never see. A locator that never appears fails in the action.
  const before = (loc) =>
    loc
      .first()
      .evaluate(ensureEditor)
      .catch(() => {});
  const wrap = (loc) =>
    new Proxy(loc, {
      get(t, k) {
        const v = t[k];
        if (typeof v !== 'function') return v;
        if (ACTS.includes(k))
          return async (...a) => {
            await before(t);
            return v.apply(t, a);
          };
        return (...a) => {
          const r = v.apply(t, a);
          return r?.constructor && /Locator$/.test(r.constructor.name) ? wrap(r) : r;
        };
      },
    });
  const locator = pg.locator.bind(pg);
  pg.locator = (...a) => wrap(locator(...a));
  for (const k of ['getByRole', 'getByText', 'getByLabel']) {
    const f = pg[k].bind(pg);
    pg[k] = (...a) => wrap(f(...a));
  }
  for (const k of ACTS) {
    if (!pg[k]) continue;
    const f = pg[k].bind(pg);
    pg[k] = async (sel, ...a) => {
      await before(locator(sel));
      return f(sel, ...a);
    };
  }
  pg.editor = (sel) => before(locator(sel)); // open the dialog holding sel (for a layout read)
  return pg;
}
// the page's errors and error-level console lines collect in pg.errors (web-font fetch aborts are network noise)
function collectErrors(pg) {
  pg.errors = [];
  pg.on('pageerror', (e) => pg.errors.push(String(e)));
  pg.on('console', (m) => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));
}
/**
 * The planner in a fresh context (fresh storage), its first search done, with the worker hooks: window.__made and
 * __killed count workers made and terminated, __runs lists the jobs sent, and while __hold is set the worker's replies
 * wait (a search stays running however fast it is) until __release().
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
      viewport: { width: w, height: h },
      colorScheme: scheme,
      ...(mobile && browser.browserType().name() === 'chromium' ? { isMobile: true, hasTouch: true } : {}),
    }),
  );
  await ctx.addInitScript(() => {
    const W = window.Worker;
    window.__made = 0;
    window.__killed = 0;
    window.__runs = []; // the jobs sent to the engine, in order
    // biome-ignore lint/complexity/useArrowFunction: the Worker stand-in is called with `new`, which an arrow function cannot be
    window.Worker = function (...a) {
      const w = new W(...a);
      window.__made++;
      const t = w.terminate.bind(w);
      w.terminate = () => {
        window.__killed++;
        t();
      };
      const pm = w.postMessage.bind(w);
      w.postMessage = (m, ...r) => {
        if (m && m.type === 'run') window.__runs.push(m.job);
        return pm(m, ...r);
      };
      let h = null,
        q = [],
        dead = false;
      Object.defineProperty(w, 'onmessage', {
        get: () => h,
        set: (f) => {
          h = f;
        },
      });
      w.addEventListener('message', (e) => {
        if (window.__hold) q.push(e);
        else if (h) h(e);
      });
      const kill = w.terminate;
      w.terminate = () => {
        dead = true;
        q = [];
        kill();
      };
      window.__flush.push(() => {
        const qq = q;
        q = [];
        if (!dead)
          qq.forEach((e) => {
            h?.(e);
          });
      });
      return w;
    };
    window.__hold = false;
    window.__flush = [];
    window.__release = () => {
      window.__hold = false;
      window.__flush.forEach((f) => {
        f();
      });
    };
  });
  if (init) await ctx.addInitScript(init);
  const pg = editorAware(await ctx.newPage());
  collectErrors(pg);
  await profile(ctx, pg);
  if (before) await before(pg);
  await timed('goto', pg.goto(PAGE_URL));
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
// the page's data by its id ('data-cards', ...): the shared lists come from build.py's data scripts (self.__LLP_DATA), the rest from JSON elements
export const json = (pg, id) =>
  pg.evaluate(
    (id) =>
      self.__LLP_DATA && id in self.__LLP_DATA
        ? self.__LLP_DATA[id]
        : JSON.parse(document.getElementById(id).textContent),
    id,
  );
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

// ---------- in-page audits (pg.evaluate(AUDIT) and so on): each returns what is wrong, one line per fault
// WCAG 2 contrast of every visible text run in the open dialog (or the page)
export const AUDIT = () => {
  const game = document.documentElement.classList.contains('game');
  const parse = (c) => {
    const m = c.match(/rgba?\(([^)]+)\)/);
    if (!m) return null;
    const p = m[1]
      .split(/[ ,/]+/)
      .filter(Boolean)
      .map(Number);
    return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 };
  };
  const over = (f, b) => ({
    r: f.r * f.a + b.r * (1 - f.a),
    g: f.g * f.a + b.g * (1 - f.a),
    b: f.b * f.a + b.b * (1 - f.a),
    a: 1,
  });
  const lum = (c) => {
    const ch = (v) => {
      v /= 255;
      return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4;
    };
    return 0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b);
  };
  const ratio = (a, b) => {
    const x = lum(a),
      y = lum(b);
    return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05);
  };
  // the colour behind el: its ancestors' background colours composited down to the first opaque one (over white);
  // img: a background picture lies behind it, which the audit can't see
  const bgOf = (el) => {
    const layers = [];
    let e = el,
      img = false;
    for (; e; e = e.parentElement) {
      const cs = getComputedStyle(e);
      if (cs.backgroundImage !== 'none' && !cs.backgroundImage.startsWith('linear')) img = true;
      const c = parse(cs.backgroundColor);
      if (c && c.a > 0) {
        layers.push(c);
        if (c.a >= 1) break;
      }
    }
    let bg = { r: 255, g: 255, b: 255, a: 1 };
    for (let i = layers.length - 1; i >= 0; i--) bg = over(layers[i], bg);
    return { bg, img };
  };
  const opac = (el) => {
    let o = 1;
    for (let e = el; e; e = e.parentElement) o *= +getComputedStyle(e).opacity;
    return o;
  };
  const out = [];
  const dlg = [...document.querySelectorAll('dialog[open]')];
  const scope = dlg.length ? dlg[dlg.length - 1] : document.body;
  for (const el of scope.querySelectorAll('*')) {
    const txt = [...el.childNodes]
      .filter((n) => n.nodeType === 3)
      .map((n) => n.textContent)
      .join('')
      .trim();
    if (!txt) continue;
    const r = el.getBoundingClientRect();
    if (!r.width || !r.height) continue;
    const cs = getComputedStyle(el);
    if (cs.visibility === 'hidden' || cs.display === 'none' || el.closest('[hidden]') || el.closest('select')) continue;
    const { bg, img } = bgOf(el);
    if (img) continue;
    if (el.closest('h1.brand') && /^b[uma]$/.test(el.className)) continue; // the brand's coloured "uma": the logo's colours by choice
    // the planner pages' recreation of the game's UI (html.game): the game's colours by choice over AAA
    if (
      game &&
      el.closest(
        '.ud, #simulation > h3, .rbsec > h3, .btn.primary, .seg [aria-pressed="true"], .vsk.pink, .pflab, .simhdr, .sethdr, .otag.pick, .lp .swhead, .lptgt h2, .afpick [aria-pressed="true"], .rankseg [aria-checked="true"], .udsk, .spark.g, .tgt, .agenda.game h4, .ag-slot.pre, .agenda.game .ag-when b',
      )
    )
      continue; // also text on the game's sprites (skill tiles), which the audit can't see behind it
    if (el.closest('.ptabs [aria-current="page"]')) continue; // every page's header: the game's green tab with white outlined text
    if (el.closest('.gmdlg')) continue;
    if (game && el.closest('dialog > .dlg > .dhead, dialog .btn.primary')) continue; // the planner pages' game-style dialogs
    const fg0 = parse(cs.color);
    const fg = over({ ...fg0, a: fg0.a * opac(el) }, bg);
    const px = parseFloat(cs.fontSize),
      large = px >= 24 || (+cs.fontWeight >= 700 && px >= 18.66),
      need = game ? 3 : large ? 4.5 : 7; // AAA (/about); the planner pages keep the game's colours, held only to a 3:1 floor
    const rt = ratio(fg, bg);
    if (rt < need)
      out.push(`${el.tagName.toLowerCase()}.${el.className} "${txt.slice(0, 30)}" ${rt.toFixed(2)}<${need}`);
  }
  return out;
};
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

// ---------- the career planner (/career)
/**
 * The career page with its run hooks and the helpers the career groups share.
 * @typedef {object} CareerPage
 * @property {object} ctx  the browser context (close it at the end)
 * @property {object} pg  the page (editor-aware; its errors in pg.errors)
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
 * @property {() => Promise<void>} saved  an edit's save has landed (vets.js persistSoon saves after the next frame)
 */
// The career planner in a fresh context, its first (empty) run done. window.__careerJobs and __careerResults list the
// jobs sent and their results; window.__seq records what the page shows each time it changes; a test sets
// window.__rollPatch to stand in for a roll's saving, window.__planPatch for fields of a plan's report,
// window.__checkPatch for fields of its check's. Each career group (career, careerpool, careervets, careergen,
// careerrun, careerborrow) runs from its own fresh page, so they run in parallel.
/** @returns {Promise<CareerPage>} */
export async function careerPage(b) {
  const ctx = await b.newContext({ viewport: { width: 1440, height: 1000 } });
  await ctx.addInitScript(() => {
    // the setup chosen open, as these tests work its controls after reloads with veterans (unchosen, it would open folded
    // then: careerphone checks that)
    try {
      if (localStorage.getItem('llp:setup-folded') === null) localStorage.setItem('llp:setup-folded', '0');
    } catch {}
    const W = window.Worker;
    window.__careerJobs = [];
    window.__careerResults = [];
    // (the plan's robustness check, `next.robust` follow-up jobs, kept apart: the tests' plan jobs and results are the rest)
    window.__checkJobs = [];
    window.__checkResults = [];
    // biome-ignore lint/complexity/useArrowFunction: the Worker stand-in is called with `new`, which an arrow function cannot be
    window.Worker = function (...args) {
      const w = new W(...args),
        post = w.postMessage.bind(w);
      // (`__rollPatch`: a test's stand-in for a roll's saving, put over the solver's before the page reads it)
      w.addEventListener('message', (e) => {
        if (e.data?.type === 'result') {
          if (window.__rollPatch && e.data.r?.next?.roll) Object.assign(e.data.r.next.roll, window.__rollPatch);
          // (`__planPatch`: a test's stand-in for fields of a plan's report, put over the solver's the same way)
          if (window.__planPatch && e.data.r?.next?.options) Object.assign(e.data.r.next, window.__planPatch);
          if (window.__checkPatch && e.data.r?.next?.robust && !e.data.r.next.options)
            Object.assign(e.data.r.next.robust, window.__checkPatch);
          if (e.data.r?.next?.robust && !e.data.r.next.options) window.__checkResults.push(e.data.r);
          else window.__careerResults.push(e.data.r);
        }
      });
      w.postMessage = (m, ...rest) => {
        if (m?.type === 'run') (m.job?.next?.robust ? window.__checkJobs : window.__careerJobs).push(m.job);
        return post(m, ...rest);
      };
      return w;
    };
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
  const pg = editorAware(await ctx.newPage());
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
  const vetsOpen = async () => {
    if (!(await pg.locator('#set-vets[open]').count())) {
      await pg.click('#v-toggle');
      await dialog(pg, 'set-vets');
    }
  };
  const vetsShut = async () => {
    if (await pg.locator('#set-vets[open]').count())
      await pg.evaluate(() => document.getElementById('set-vets').close());
  };
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
  const simStars = async (n) => {
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
  // an edit's save lands after the next frame (vets.js persistSoon): what a check reads from storage straight after one
  const saved = () => pg.evaluate(() => new Promise((r) => requestAnimationFrame(() => setTimeout(r))));
  // the umadump test file's 12 veterans
  const importDump = async () => {
    await pg.locator('#v-file').setInputFiles(repoUrl('val/umadump/U_trained_chara_data.json').pathname);
    await pg.waitForFunction(() => document.getElementById('v-msg').textContent);
  }; // the file is read asynchronously
  await pg.goto(PAGE_URL.replace(/\/?$/, '/') + 'career');
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
