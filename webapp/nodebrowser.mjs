// Browser regression suite for the built page (webapp/dist, served on a local port; LLP_URL tests another address).
// Node + Playwright (local or global install). LLP_SHOTS=dir saves pictures of the scenario button and picker there.
//   node webapp/nodebrowser.mjs                 every test, Chromium and Firefox
//   node webapp/nodebrowser.mjs chromium roster only the named browsers / tests (any order)
//   node webapp/nodebrowser.mjs quick           the fast tier (./build.sh testq): each page's core path, Chromium only
// Both browsers run at once, each with LLP_JOBS test groups in parallel (default 4; LLP_JOBS=1 runs them one by one);
// a group's lines print together when it ends. Exit code 1 if any check fails. Expectations come from the page's own embedded data where possible, so a data
// update does not break the suite; the one fixed number is the Global default search (1 in 119, R cards off, Unity Cup).
import { createRequire } from 'module';
import { AsyncLocalStorage } from 'async_hooks';
import { serveDist } from './serve.mjs';
import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

const local = process.env.LLP_URL ? null : await serveDist();
const URL = process.env.LLP_URL || local.url;
process.env.PLAYWRIGHT_BROWSERS_PATH ??= '0';   // the browsers live in webapp/node_modules (./build.sh browsers)
let pw;
// the scenario tiles' "Hint rate ×" lines as the solver's hint_base gives them (a multiple of URA Finale's)
function hintRates() {
  const src = fs.readFileSync(path.join(path.dirname(fileURLToPath(import.meta.url)), '..', 'pkg', 'loopopt', 'src', 'deck', 'hints.rs'), 'utf8');
  const body = /fn hint_base[^{]*\{[^]*?match scenario \{([^}]*)\}/.exec(src)[1];
  const v = Object.fromEntries([...body.matchAll(/"(\w+)" => ([\d.]+)/g)].map(m => [m[1], +m[2]])), dflt = +/_ => ([\d.]+)/.exec(body)[1];
  const of = s => v[s] ?? dflt;
  return Object.fromEntries(['unity', 'gc', 'ura', 'trackblazer', 'grandmasters'].map(s => [s, `Hint rate ×${(of(s) / of('ura')).toFixed(2)}`]));
}
try { pw = createRequire(import.meta.url)('playwright'); }
catch { console.error('Playwright is missing: run ./build.sh browsers (or cd webapp && npm ci)'); process.exit(1); }

const argv = process.argv.slice(2);
// the fast tier (`quick`; ./build.sh testq): each page's core path, Chromium only unless a browser is named: the planner
// loads and its search returns (load), the result's dialogs (popups), the phone layout (phone), the contrast audit in
// light mode (contrast), /about, and the career page up to its one-run answer (career stops there in this tier)
const QUICK = argv.includes('quick'), QUICK_GROUPS = ['career', 'load', 'popups', 'phone', 'contrast', 'about'];
const BROWSERS = ['chromium', 'firefox'].filter(b => argv.some(a => ['chromium', 'firefox'].includes(a)) ? argv.includes(b) : !QUICK || b === 'chromium');
let failed = 0, passed = 0;
const LOG = new AsyncLocalStorage();   // the running group's buffered lines (groups run in parallel)
const say = (line) => { const buf = LOG.getStore(); if (buf) buf.push(line); else console.log(line); };
const check = (name, ok, detail = '') => { if (ok) passed++; else failed++; say(`${ok ? '  ok  ' : '  FAIL'} ${name}${detail ? ' · ' + detail : ''}`); };
// Whether the build carries the game art: a clone without game files builds with initials in place of pictures, so
// checks of the art itself are skipped and the contrast audits leave out the initials that stand in for card art.
const ASSETS = path.join(path.dirname(fileURLToPath(import.meta.url)), 'dist', 'assets');
const ART = !!process.env.LLP_URL || (() => { try { const d = fs.readdirSync(ASSETS).find(n => n.startsWith('img.')); return !!d && fs.readdirSync(path.join(ASSETS, d, 'uma')).length > 0; } catch { return false; } })();   // the character icons `images` extracts
let skipped = 0;
const artCheck = (name, ok, detail = '') => ART ? check(name, ok, detail) : (skipped++, say(`  skip ${name} · no game art in this build`));
const artless = (fails) => ART ? fails : fails.filter(x => !/^span\.cav\b/.test(x));
// LLP_PROF=file: where each group's time goes (new contexts and page loads, waits for searches and plans, fixed sleeps,
// contrast audits, and the solver's own time per job), one JSON line per group appended to the file
const PROF = process.env.LLP_PROF;
const profOf = () => LOG.getStore()?.prof;
const timed = (key, p, st = profOf()) => { if (!st) return p; const t0 = Date.now();
  return Promise.resolve(p).finally(() => { st[key] = (st[key] || 0) + Date.now() - t0; st[key + '#'] = (st[key + '#'] || 0) + 1; }); };
async function profile(ctx, pg) {   // the solver's time per job (the worker reports it with each result), fixed sleeps, audits
  const st = profOf(); if (!st) return;
  await ctx.exposeBinding('__llpSolved', (_, kind, ms) => { st['solver:' + kind] = (st['solver:' + kind] || 0) + ms; st['solver:' + kind + '#'] = (st['solver:' + kind + '#'] || 0) + 1; (st.solverLog ||= []).push(`${kind} ${ms}`); });
  await ctx.addInitScript(() => { const W = window.Worker; window.Worker = function (...a) { const w = new W(...a), kinds = new Map(), pm = w.postMessage.bind(w);
    w.postMessage = (m, ...r) => { if (m && m.type === 'run') kinds.set(m.id, m.job.cmd ? m.job.cmd + (m.job.next ? '.next' : '') + (m.job.compare ? '.compare' : '') : 'loop'); return pm(m, ...r); };
    w.addEventListener('message', (e) => { if (e.data?.type === 'result') window.__llpSolved(kinds.get(e.data.id) || '?', Math.round(e.data.ms)); }); return w; }; });
  const sleep = pg.waitForTimeout.bind(pg); pg.waitForTimeout = (ms) => timed('sleep', sleep(ms), st);
  const ev = pg.evaluate.bind(pg); pg.evaluate = (fn, ...a) => fn === AUDIT ? timed('audit', ev(fn, ...a), st) : ev(fn, ...a);
}

// ---------- helpers
// before(pg): runs before the page loads (routes, request listeners)
// The settings editors (Target skills, Umas, Support cards; on the career page also your veterans) are dialogs the setup
// panel's buttons open. editorAware: an action on a control inside one of them opens its dialog first, and an action on
// the page outside any open dialog shuts them, so the checks of the editors' behaviour read as on a page that shows them
// (the buttons and dialogs themselves are checked on their own). Page actions and every locator's actions go through it.
const ACTS = ['click', 'fill', 'focus', 'press', 'hover', 'type', 'check', 'uncheck', 'dblclick', 'selectOption', 'tap', 'innerText', 'boundingBox', 'scrollIntoViewIfNeeded'];
function ensureEditor(el) {
  const d = el.closest('dialog.setdlg'), open = [...document.querySelectorAll('dialog.setdlg[open]')];
  if (d) { if (!d.open) { open.forEach(x => x.close()); d.showModal(); } }
  else if (!el.closest('dialog[open]')) open.forEach(x => x.close());
}
function editorAware(pg) {
  const before = (loc) => loc.first().evaluate(ensureEditor, null, { timeout: 1500 }).catch(() => {});
  const wrap = (loc) => new Proxy(loc, { get(t, k) { const v = t[k]; if (typeof v !== 'function') return v;
    if (ACTS.includes(k)) return async (...a) => { await before(t); return v.apply(t, a); };
    return (...a) => { const r = v.apply(t, a); return r && r.constructor && /Locator$/.test(r.constructor.name) ? wrap(r) : r; }; } });
  const L = pg.locator.bind(pg); pg.locator = (...a) => wrap(L(...a));
  for (const k of ['getByRole', 'getByText', 'getByLabel']) { const f = pg[k].bind(pg); pg[k] = (...a) => wrap(f(...a)); }
  for (const k of ACTS) { if (!pg[k]) continue; const f = pg[k].bind(pg); pg[k] = async (sel, ...a) => { await before(L(sel)); return f(sel, ...a); }; }
  pg.editor = (sel) => before(L(sel));   // open the dialog holding sel (for a layout read)
  return pg;
}
async function open(browser, { w = 1440, h = 900, mobile = false, init, before, scheme = 'light' } = {}) {
  const ctx = await timed('ctx', browser.newContext({ viewport: { width: w, height: h }, colorScheme: scheme, ...(mobile && browser.browserType().name() === 'chromium' ? { isMobile: true, hasTouch: true } : {}) }));
  await ctx.addInitScript(() => {   // count worker creations/terminations
    const W = window.Worker; window.__made = 0; window.__killed = 0;
    window.__runs = [];   // the jobs sent to the engine, in order
    window.Worker = function (...a) { const w = new W(...a); window.__made++; const t = w.terminate.bind(w); w.terminate = () => { window.__killed++; t(); };
      const pm = w.postMessage.bind(w); w.postMessage = (m, ...r) => { if (m && m.type === 'run') window.__runs.push(m.job); return pm(m, ...r); };
      // window.__hold: the worker's replies wait (a search stays running however fast it is) until window.__release()
      let h = null, q = [], dead = false; Object.defineProperty(w, 'onmessage', { get: () => h, set: (f) => { h = f; } });
      w.addEventListener('message', (e) => { if (window.__hold) q.push(e); else if (h) h(e); });
      const kill = w.terminate; w.terminate = () => { dead = true; q = []; kill(); };
      window.__flush.push(() => { const qq = q; q = []; if (!dead) qq.forEach((e) => h && h(e)); }); return w; };
    window.__hold = false; window.__flush = []; window.__release = () => { window.__hold = false; window.__flush.forEach((f) => f()); };
  });
  if (init) await ctx.addInitScript(init);
  const pg = editorAware(await ctx.newPage()); pg.errors = [];
  await profile(ctx, pg);
  pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));   // web-font fetch aborts are network noise
  if (before) await before(pg);
  await timed('goto', pg.goto(URL)); await timed('settle.load', settle(pg));
  return pg;
}
const busy = (pg) => pg.evaluate(() => document.getElementById('run-note').classList.contains('busy'));
// wait for the current (auto) search to finish. A setting change marks the status line busy at once (the search
// itself starts after a short pause), so no fixed sleep is needed; after a page load, the page's first job must have
// been sent (pages opened without open()'s worker hook have no __runs and skip that part)
const settle = (pg, t) => timed('settle', settle0(pg, t));
async function settle0(pg, t = 180000) {
  await pg.waitForFunction(() => { const n = document.getElementById('run-note');
    // a search has run, or (no job sent) the saved result of these settings shows as done (a reload's)
    return (window.__runs === undefined || window.__runs.length > 0 || /^Checked [\d,]+ decks/.test(n.textContent)) && !n.classList.contains('busy') && /Checked [\d,]+ decks|example/i.test(n.textContent); }, null, { timeout: t });
}
const jobSent = (pg, fn, arg) => pg.waitForFunction(fn, arg);   // a search's job has gone to the engine (its result is not awaited)
const lastJob = (pg, scen) => jobSent(pg, (s) => window.__runs.length > 0 && window.__runs.at(-1).scenario === s, scen);
const kind = (pg, k) => pg.waitForFunction((k) => document.getElementById('rhead').dataset.kind === k, k);   // the status line's state
const dialog = (pg, id) => pg.waitForSelector(`#${id}[open]`);
// a condition the page reaches on its own (an observer, a smooth scroll, a resize handler): true once it holds, false
// if it never does within the time
const until = (pg, fn, arg, timeout = 3000) => pg.waitForFunction(fn, arg, { timeout }).then(() => true, () => false);
async function navClick(pg, sel) { await Promise.all([pg.waitForEvent('load'), pg.click(sel)]); }   // a click that reloads the page
const note = (pg) => pg.locator('#run-note').innerText();
const odds = (pg) => pg.evaluate(() => (document.querySelector('#results .loop .odds') || {}).textContent || '');
// the page's data by its old element id: the shared lists come from build.py's data scripts (self.__LLP_DATA), the rest from JSON elements
const json = (pg, id) => pg.evaluate((id) => self.__LLP_DATA && id in self.__LLP_DATA ? self.__LLP_DATA[id] : JSON.parse(document.getElementById(id).textContent), id);
const scen = (pg) => pg.getAttribute('#x-scen', 'data-value');   // the chosen scenario
async function pickScen(pg, v) { await pg.click('#x-scen'); await pg.click(`#scen-menu .stile[data-v="${v}"]`); }

// WCAG 2 contrast of every visible text run in the open dialog (or the page)
const AUDIT = () => {
  const game = document.documentElement.classList.contains('game');
  const parse = (c) => { const m = c.match(/rgba?\(([^)]+)\)/); if (!m) return null; const p = m[1].split(/[ ,/]+/).filter(Boolean).map(Number); return { r: p[0], g: p[1], b: p[2], a: p.length > 3 ? p[3] : 1 }; };
  const over = (f, b) => ({ r: f.r * f.a + b.r * (1 - f.a), g: f.g * f.a + b.g * (1 - f.a), b: f.b * f.a + b.b * (1 - f.a), a: 1 });
  const lum = (c) => { const ch = (v) => { v /= 255; return v <= 0.03928 ? v / 12.92 : ((v + 0.055) / 1.055) ** 2.4; }; return 0.2126 * ch(c.r) + 0.7152 * ch(c.g) + 0.0722 * ch(c.b); };
  const ratio = (a, b) => { const x = lum(a), y = lum(b); return (Math.max(x, y) + 0.05) / (Math.min(x, y) + 0.05); };
  const bgOf = (el) => { const layers = []; let e = el, img = false;
    for (; e; e = e.parentElement) { const cs = getComputedStyle(e); if (cs.backgroundImage !== 'none' && !cs.backgroundImage.startsWith('linear')) img = true;
      const c = parse(cs.backgroundColor); if (c && c.a > 0) { layers.push(c); if (c.a >= 1) break; } }
    let bg = { r: 255, g: 255, b: 255, a: 1 }; for (let i = layers.length - 1; i >= 0; i--) bg = over(layers[i], bg); return { bg, img }; };
  const opac = (el) => { let o = 1; for (let e = el; e; e = e.parentElement) o *= +getComputedStyle(e).opacity; return o; };
  const out = []; const dlg = [...document.querySelectorAll('dialog[open]')]; const scope = dlg.length ? dlg[dlg.length - 1] : document.body;
  for (const el of scope.querySelectorAll('*')) {
    const txt = [...el.childNodes].filter(n => n.nodeType === 3).map(n => n.textContent).join('').trim(); if (!txt) continue;
    const r = el.getBoundingClientRect(); if (!r.width || !r.height) continue;
    const cs = getComputedStyle(el); if (cs.visibility === 'hidden' || cs.display === 'none' || el.closest('[hidden]') || el.closest('select')) continue;
    const { bg, img } = bgOf(el); if (img) continue;
    if (el.closest('h1.brand') && /^b[uma]$/.test(el.className)) continue;   // the brand's coloured "uma": the logo's colours by choice
    // the planner pages' recreation of the game's UI (html.game): the game's colours by choice over AAA
    if (game && el.closest('.ud, #simulation > h3, .rbsec > h3, .btn.primary, .seg [aria-pressed="true"], .vsk.pink, .pflab, .simhdr, .sethdr, .otag.pick, .lp .swhead, .lptgt h2, .afpick [aria-pressed="true"], .rankseg [aria-checked="true"], .udsk, .spark.g, .tgt, .agenda.game h4, .ag-slot.pre, .agenda.game .ag-when b')) continue;   // also text on the game's sprites (skill tiles), which the audit can't see behind it
    if (el.closest('.ptabs [aria-current="page"]')) continue;   // every page's header: the game's green tab with white outlined text
    if (el.closest('.gmdlg')) continue;
    if (game && el.closest('dialog > .dlg > .dhead, dialog .btn.primary')) continue;   // the planner pages' game-style dialogs
    const fg0 = parse(cs.color); const fg = over({ ...fg0, a: fg0.a * opac(el) }, bg);
    const px = parseFloat(cs.fontSize), large = px >= 24 || (+cs.fontWeight >= 700 && px >= 18.66), need = game ? 3 : large ? 4.5 : 7;   // AAA (/about); the planner pages keep the game's colours, held only to a 3:1 floor
    const rt = ratio(fg, bg); if (rt < need) out.push(`${el.tagName.toLowerCase()}.${el.className} "${txt.slice(0, 30)}" ${rt.toFixed(2)}<${need}`);
  }
  return out;
};

// every visible pill track (the segmented controls and the veteran card's tabs): its segments fill it flush, the full
// inner height, no gap at its ends or between them, the track a pill that clips them to its own curve, none of them drawn
// with the game's pill sprite (whose fixed-width caps don't fit other heights). Returns what is wrong.
const PILLS = () => {
  const bad = [], near = (a, b) => Math.abs(a - b) <= 0.6;
  for (const t of document.querySelectorAll('.seg, .udtabs')) {
    const r = t.getBoundingClientRect(), cs = getComputedStyle(t); if (!r.width || !r.height || cs.visibility === 'hidden') continue;
    const id = t.id ? '#' + t.id : t.getAttribute('aria-label') || t.getAttribute('aria-labelledby') || t.className;
    const inner = { l: r.left + parseFloat(cs.borderLeftWidth), r: r.right - parseFloat(cs.borderRightWidth), t: r.top + parseFloat(cs.borderTopWidth), b: r.bottom - parseFloat(cs.borderBottomWidth) };
    if (cs.overflowX !== 'hidden' && cs.overflowX !== 'clip') bad.push(`${id}: does not clip its segments`);
    if (parseFloat(cs.borderTopLeftRadius) < r.height / 2 - 0.5) bad.push(`${id}: not a pill`);
    const segs = [...t.children].filter(c => c.getBoundingClientRect().width > 0);
    segs.forEach((s, i) => { const q = s.getBoundingClientRect();
      if (!near(q.top, inner.t) || !near(q.bottom, inner.b)) bad.push(`${id} #${i}: ${(q.top - inner.t).toFixed(1)} / ${(inner.b - q.bottom).toFixed(1)} px short of top / bottom`);
      if (i === 0 && !near(q.left, inner.l)) bad.push(`${id}: ${(q.left - inner.l).toFixed(1)} px gap at its start`);
      if (i === segs.length - 1 && !near(q.right, inner.r)) bad.push(`${id}: ${(inner.r - q.right).toFixed(1)} px gap at its end`);
      if (i > 0 && !near(q.left, segs[i - 1].getBoundingClientRect().right)) bad.push(`${id}: a gap before #${i}`);
      if (getComputedStyle(s).borderImageSource !== 'none') bad.push(`${id} #${i}: drawn with a border-image`); });
  }
  return bad;
};
// what scrolls sideways: the page, an open dialog's body, or an open dialog that leaves the screen
const SIDEWAYS = () => {
  const d = document.documentElement, out = [];
  if (d.scrollWidth > d.clientWidth) out.push(`page ${d.scrollWidth} > ${d.clientWidth}`);
  for (const g of document.querySelectorAll('dialog[open]')) { const r = g.getBoundingClientRect();
    if (r.left < -0.5 || r.right > d.clientWidth + 0.5) out.push(`#${g.id} leaves the screen`);
    for (const s of g.querySelectorAll('.dlg, .gmdbody')) if (s.scrollWidth > s.clientWidth + 1) out.push(`#${g.id} scrolls sideways (${s.scrollWidth} > ${s.clientWidth})`); }
  return out;
};
// no transition or finite animation still running (a fade, a theme change): what a fixed short sleep waited for
const still = (pg) => until(pg, () => document.getAnimations().every(a => a.playState !== 'running' || a.effect?.getTiming().iterations === Infinity));

// the career planner (/career) in a fresh context, its first (empty) run done: the job and result hooks, the record of
// what the page shows, and the helpers the career groups share (career, careerpool, careervets, careergen and careerrun
// run in parallel, each from its own fresh page)
async function careerPage(b) {
  const ctx = await b.newContext({ viewport: { width: 1440, height: 1000 } });
  await ctx.addInitScript(() => {
    const W = window.Worker; window.__careerJobs = []; window.__careerResults = [];
    window.Worker = function (...args) { const w = new W(...args), post = w.postMessage.bind(w);
      w.addEventListener('message', (e) => { if (e.data?.type === 'result') window.__careerResults.push(e.data.r); });
      w.postMessage = (m, ...rest) => { if (m?.type === 'run') window.__careerJobs.push(m.job); return post(m, ...rest); }; return w; };
    // what the page shows, each time it changes: busy or not, the phase named, the trainee on show, the veteran you
    // got (her sparks), whether the old answer is dimmed
    window.__seq = [];
    const snap = () => { const n = document.getElementById('run-note'); if (!n) return;
      const s = { busy: !!n.querySelector('.spin, .runner'), phase: /Planning your next career…/.test(n.textContent) ? 'plan' : /Searching careers…/.test(n.textContent) ? 'search' : '',
        who: document.querySelector('#tree .pn.trainee .nm')?.textContent || '', draft: (document.querySelector('#simulation .ud') ? 'v:' + [...document.querySelectorAll('#simulation .simskills .vskb')].map(b => b.getAttribute('aria-label')).join('|') : ''),
        stale: !!document.querySelector('#results.stale') };
      const k = JSON.stringify(s); if (window.__seq.at(-1)?.k !== k) window.__seq.push({ ...s, k }); };
    new MutationObserver(snap).observe(document, { subtree: true, childList: true, characterData: true, attributes: true, attributeFilter: ['class'] });
  });
  // one answer per run: while the run is busy the page shows the answer it had before it (`pre`, none at first), dimmed
  // if there is one, and names the phase; the answer it ends on is the only other one drawn
  const oneAnswer = async (pre) => {
    const seq = await pg.evaluate(() => window.__seq), busy = seq.filter(s => s.busy), end = seq.at(-1);
    const shows = (s) => s.who + '/' + s.draft, phases = busy.map(s => s.phase).filter((p, i, a) => p && p !== a[i - 1]);
    // dimmed from when its job starts (the pause before it dims nothing) to the answer
    const dim = busy.findIndex(s => s.stale);
    const ok = busy.length > 0 && !end.busy && busy.every(s => shows(s) === pre) && (!busy.some(s => s.draft) || dim >= 0 && busy.slice(dim).every(s => !s.draft || s.stale)) && end.draft !== '' && !end.stale
      && seq.slice(seq.indexOf(busy.at(-1)) + 1).every(s => shows(s) === shows(end));
    return { ok, phases, detail: `phases ${phases.join(' > ')} · before ${pre.slice(0, 60)} · busy shows ${[...new Set(busy.map(shows))].map(x => x.slice(0, 40)).join(' | ')} · end ${shows(end).slice(0, 60)}` };
  };
  const showsNow = () => pg.evaluate(() => (document.querySelector('#tree .pn.trainee .nm')?.textContent || '') + '/' + (document.querySelector('#simulation .ud') ? 'v:' + [...document.querySelectorAll('#simulation .simskills .vskb')].map(b => b.getAttribute('aria-label')).join('|') : ''));
  const pg = editorAware(await ctx.newPage()); pg.errors = [];
  await profile(ctx, pg);
  pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));
  const note = () => pg.textContent('#run-note');
  // the run an action started has ended. A change puts the spinner up at once (the search itself starts after a short
  // pause), so no sleep past the pause is needed; an action that starts no run returns at once
  const done = () => timed('done', pg.waitForFunction(() => { const n = document.getElementById('run-note'); return !n.querySelector('.spin, .runner') && n.textContent.length > 0; }, null, { timeout: 180000 }));
  const planned = () => timed('done', pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 }));
  const ask = async (label) => { await dialog(pg, 'ask-dlg'); await pg.locator('#ask-btns button', { hasText: label }).click(); };
  // the tree is portraits; a portrait opens its node popup (sparks, G1 wins, choose or hand back the place)
  // your veterans live in the Veteran Umamusume dialog: open it to reach them, shut it to reach the page again
  const vetsOpen = async () => { if (!(await pg.locator('#set-vets[open]').count())) { await pg.click('#v-toggle'); await dialog(pg, 'set-vets'); } };
  const vetsShut = async () => { if (await pg.locator('#set-vets[open]').count()) await pg.evaluate(() => document.getElementById('set-vets').close()); };
  const vetsSeen = async () => { if (!(await pg.locator('#set-vets[open]').count())) { await vetsOpen(); await vetsShut(); } };
  const node = async (key) => { await vetsShut(); await pg.click(`#tree [data-k="${key}"]`); await dialog(pg, 'node-dlg'); };
  const inNode = (name) => pg.locator('#node-dlg').getByRole('button', { name });
  const closeNode = async () => { if (await pg.locator('#node-dlg[open]').count()) await pg.keyboard.press('Escape'); };
  // the veteran you got: Uma Stan at n stars (added first if she lacks it)
  const simStars = async (n) => {
    const loc = pg.locator('[data-k="sim:s20159"]');
    if (!(await loc.count())) { await pg.locator('#simulation .vskadd').click(); await pg.locator('#add-pop .addsk', { hasText: 'Uma Stan' }).click(); }
    for (let i = 0; i < 3; i++) { if (+(await loc.getAttribute('aria-label')).match(/(\d) star/)[1] === n) return; await loc.click(); }
  };
  // the umadump test file's 12 veterans
  const importDump = async () => { await pg.locator('#v-file').setInputFiles(new globalThis.URL('../val/umadump/U_trained_chara_data.json', import.meta.url).pathname);
    await pg.waitForFunction(() => document.getElementById('v-msg').textContent); };   // the file is read asynchronously
  await pg.goto(URL.replace(/\/?$/, '/') + 'career'); await done();
  return { ctx, pg, note, done, planned, ask, oneAnswer, showsNow, vetsOpen, vetsShut, vetsSeen, node, inNode, closeNode, simStars, importDump };
}

// ---------- tests: each gets a fresh page (fresh storage) unless noted
const TESTS = {
  async load(b) {
    const pg = await open(b);
    check('first search runs by itself, and its status is just the count: checked N decks / M loops in T s (a caveat only when the exact check ran out of time)', /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s( · .+)?$/.test(await note(pg)), await note(pg));
    // the result's wording: the headline odds, each member's own odds as trainee
    const head = await pg.evaluate(() => { const l = document.querySelector('#results .loop'); return { odds: l.querySelector('.odds').innerText, sub: !!l.querySelector('.sub'), rank: !!l.querySelector('.rank'),
      own: [...l.querySelectorAll('.mem .own')].map(e => e.innerText), font: getComputedStyle(l.querySelector('.odds')).fontFamily }; });
    check('headline: "1 in 107 runs to land all 8" alone (no 90% line), in the game font; no rank with one result',
      /^1 in 107 runs to land all 8$/.test(head.odds) && !head.sub && !head.rank && /Zen Maru/.test(head.font), JSON.stringify(head));
    // Odds opens the career planner's table for the loop: every count, 8/8 the headline's (1 in 107), 7/8 and 8/8 together
    // the solver's 7-or-better (1 in 12.6), focus back on the button when it closes
    await pg.click('#results .loop .lacts button:has-text("Odds")'); await dialog(pg, 'odds-dlg');
    const lrows = await pg.$$eval('#odds-dlg .oddtab tbody tr', l => l.map(r => [...r.children].map(c => c.textContent)));
    const pv = (s) => parseFloat(s) / 100;
    await pg.keyboard.press('Escape');
    check('loop odds: a popup with every count 8/8 .. 0/8, all 8 at the headline odds, 7 or better at 1 in 12.6, focus back on Odds',
      lrows.length === 9 && lrows[0][0] === '8/8' && lrows[8][0] === '0/8' && Math.abs(1 / pv(lrows[0][1]) - 107) < 2 && Math.abs(1 / (pv(lrows[0][1]) + pv(lrows[1][1])) - 12.6) < 0.3
      && await pg.evaluate(() => document.activeElement?.textContent === 'Odds'), JSON.stringify(lrows.slice(0, 3)));
    // the default result is a three-uma rotation: three members in three columns, a deck for each
    check('members: three (a rotation), "as trainee: 1 in …", in three columns', head.own.length === 3 && head.own.every(x => /^as trainee: 1 in [\d,.]+$/.test(x))
      && await pg.evaluate(() => { const m = document.querySelector('#results .loop .members'); return m.classList.contains('n3') && getComputedStyle(m).gridTemplateColumns.split(' ').length === 3; }), head.own.join(' | '));
    // every member names one outfit, never "any outfit" (her popup says when any would do)
    const ofs = await pg.$$eval('#results .loop .mem', l => l.map(m => [m.querySelector('.of').textContent, m.dataset.outfitNote || '']));
    check('members: each names one outfit (never "any outfit"; her popup carries any note)', ofs.length > 0 && ofs.every(([o]) => o && !/any outfit/i.test(o))
      && !/any outfit/i.test(await pg.textContent('#results')), JSON.stringify(ofs));
    // a pal whose credited event choice needs a priority says so, derived from the event data (Anshinzawa SSR's finale)
    const anshin = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .dc[data-cid="30080"] i')].map(i => i.textContent));
    check('deck: Sasami Anshinzawa SSR says "prioritise Risky Business"', anshin.includes('prioritise Risky Business'), JSON.stringify(anshin));
    // the deck's odds per target, each with its skill icon
    const dd = await pg.evaluate(() => { const l = document.querySelector('#results .loop .dodds'); return { n: l.querySelectorAll('.dsk').length, icons: l.querySelectorAll('.dsk img.sic').length }; });
    artCheck('deck odds: a skill icon on every entry', dd.n > 0 && dd.icons === dd.n, JSON.stringify(dd));
    // the odds sit in a frame the (i) button opens; a swap row names its uma by her character icon
    const oi = pg.locator('#results .loop .deck .oinfo').first();
    check('deck odds start closed', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
    await oi.click();
    check('(i) opens the deck odds (aria-expanded, names and percentages)', (await oi.getAttribute('aria-expanded')) === 'true' && await pg.evaluate(() => { const o = document.querySelector('#results .loop .dodds'); return !o.hidden && o.querySelectorAll('.dnm').length > 0 && /\d+%/.test(o.innerText); }));
    await oi.click();
    check('(i) closes them again', await pg.evaluate(() => document.querySelector('#results .loop .dodds').hidden));
    const sw = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap .swho')].map(w => ({ av: w.querySelectorAll('.av').length, label: w.querySelector('.vh').textContent, tips: [...w.querySelectorAll('.facebtn')].map(a => a.title), shown: [...w.childNodes].filter(n => !(n.classList && (n.classList.contains('vh') || n.classList.contains('facebtn')))).map(n => n.textContent).join('').trim() })));   // .vh: screen-reader text
    const swd = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap')].map(d => [...d.querySelectorAll('.swd1')].map(x => x.textContent.trim())));
    check('each swap row says what it changes ("Skill a% → b%")', swd.length > 0 && swd.every(r => r.length > 0 && r.every(t => /\d+% → \d+%$/.test(t))), JSON.stringify(swd));
    // a target on the swapping uma's own skill list is no change for her (her deck drops its hints because she has it):
    // not listed (the default's Agnes Digital [Full-Color Fangirling] has Superstan, so no "Uma Stan 100% → 0%")
    const owned = await pg.evaluate(() => { const skl = self.__LLP_DATA['data-skills'], wOf = new Map(skl.filter(s => s[2]).map(s => [s[2], s[1]]));
      return [...document.querySelectorAll('#results .loop .deck.swap')].map(d => { const ids = [...d.querySelectorAll('.swho .facebtn')].map(f => f.dataset.uma);
        const per = ids.map(id => [...document.querySelectorAll(`#results .loop .mem[data-uma="${id}"] .sk .spark`)].flatMap(s => s.classList.contains('g') ? [s.textContent, wOf.get(s.textContent)] : [s.textContent]));
        return { mine: per.length ? per[0].filter(n => per.every(p => p.includes(n))) : [], swd: [...d.querySelectorAll('.swd1')].map(x => x.textContent.trim()) }; }); });
    check('swap rows leave out the targets their umas have on their own skill list', owned.some(r => r.mine.length) && owned.every(r => r.swd.every(t => !r.mine.some(n => t.startsWith(n + ' ')))), JSON.stringify(owned));
    const more = pg.locator('#results .loop .swd.more').first();
    if (await more.count()) { const n0 = await more.locator('.swd1:visible').count(); await more.click();
      check('clicking a swap\'s changes shows them all, again folds them', (await more.locator('.swd1:visible').count()) > n0 && (await more.getAttribute('aria-expanded')) === 'true'
        && (await more.click(), (await more.locator('.swd1:visible').count()) === n0)); }
    check('a label marks where the swap rows start', await pg.evaluate(() => { const h = document.querySelector('#results .loop .swhead'), s = document.querySelector('#results .loop .deck.swap'); return !!h && /swaps/i.test(h.textContent) && !!(h.compareDocumentPosition(s) & Node.DOCUMENT_POSITION_FOLLOWING); }));
    // each swap row: the card that goes greyed ➜ the card that comes in, every card's art on one line, the arrow at its middle
    const rows = await pg.evaluate(() => [...document.querySelectorAll('#results .loop .deck.swap')].map(d => { const art = [...d.querySelectorAll('.swc .dc .cav')].map(x => x.getBoundingClientRect()), a = d.querySelector('.swa svg').getBoundingClientRect();
      return { outN: d.querySelectorAll('.swout .dc').length, inN: d.querySelectorAll('.swin .dc').length, tops: art.map(x => Math.round(x.top)),
        mid: Math.round(Math.abs((a.top + a.bottom) / 2 - (art[0].top + art[0].bottom) / 2)), grey: getComputedStyle(d.querySelector('.swout .dc .cav')).filter }; }));
    check('swap rows: the card out greyed ➜ the card in, every card\'s art on one line, the arrow at its middle', rows.length > 0 && rows.every(r =>
      r.outN > 0 && r.outN === r.inN && /grayscale/.test(r.grey) && (!ART || new Set(r.tops).size === 1 && r.mid <= 1)), JSON.stringify(rows));   // the art's alignment only with art
    check('swap rows show the uma as a character icon, named for screen readers', sw.length > 0 && sw.every(x => x.av > 0 && /’s deck:$/.test(x.label) && x.tips.length === x.av && x.tips.every(t => t && !/ and |’s deck/.test(t)) && x.shown === ''), JSON.stringify(sw));
    // two columns: the Loop setup panel (the status row at its foot) beside the results, level at the top; the editors in dialogs
    check('desktop: Loop setup beside the results, their tops level, the whole page in one 900 px screen', await pg.evaluate(() => { const set = document.querySelector('.lp .rbbar').getBoundingClientRect(), res = document.querySelector('#results .loop').getBoundingClientRect();
      return set.right <= res.left && Math.abs(set.top - res.top) <= 1 && document.querySelector('.lpside').getBoundingClientRect().bottom <= innerHeight + 60; }));
    check('the status line and the Auto-search and Deck per uma switches sit at the foot of the setup panel, Results in the results\' corner', await pg.evaluate(() => { const t = document.querySelector('.lp .rbbar');
      return ['run-note', 'x-auto', 'c-peruma', 'rank'].every(id => t.contains(document.getElementById(id))) && document.getElementById('rcol').contains(document.getElementById('c-top')) && document.getElementById('c-peruma').getAttribute('role') === 'switch'; }));
    check('no Loop size control: the solver picks 3-uma rotations or 4-uma loops', !(await pg.locator('#c-size').count()) && await pg.evaluate(() => window.__runs.at(-1).sizes === undefined));
    // the scenario sets the training-hint rate: switching it searches again with the scenario in the job, and the
    // choice is kept (the default setup's odds barely move: its decks get the targets from events, not training hints)
    check('the first search is a Unity Cup job', await pg.evaluate(() => window.__runs.at(-1).scenario === 'unity'));
    await pickScen(pg, 'ura'); await lastJob(pg, 'ura');   // the check reads the job: its (slow) search is not awaited
    // Unity Cup's golds follow the scenario: off under URA, back on under Unity, and the 8/8 example takes them too
    check('URA drops the Unity golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
    await pg.click('#t-preset'); await pg.locator('#pr-list .prow').first().locator('button', { hasText: 'Load' }).click();
    check('8/8 example under URA: eight targets, no guaranteed golds', (await pg.locator('#targets .tgt').count()) === 8 && (await pg.getAttribute('#g-20049', 'data-g')) === '0');
    // a saved preset: kept across a reload, and an import adds its presets to yours instead of replacing them
    await pg.locator('#targets .tgt .x').first().click(); await pg.click('#t-preset'); await pg.click('#pr-save'); await pg.fill('#prn-name', 'Seven'); await pg.click('#prn-save');
    check('a saved preset is the current one', (await pg.locator('#pr-list .prow.cur .pname').textContent()) === 'Seven');
    await pg.click('#preset-dlg .dclose');
    await pg.reload(); await jobSent(pg, () => window.__runs.length > 0);
    check('scenario survives a reload', (await scen(pg)) === 'ura' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: URA Finale');
    await pg.click('#t-preset');
    check('presets survive a reload', (await pg.locator('#pr-list .pname').allTextContents()).join('|') === '8/8 example|Seven');
    await pg.click('#preset-dlg .dclose'); await pg.click('#ex-open');
    const ex = JSON.parse(await pg.inputValue('#ex-text')); ex.settings.presets = [{ name: 'Seven', groups: [20159] }, { name: 'Other', groups: [20049] }];
    await pg.fill('#ex-text', JSON.stringify(ex)); await pg.click('#ex-load'); await pg.click('#ex-close'); await pg.click('#t-preset');
    check('an import merges presets', (await pg.locator('#pr-list .pname').allTextContents()).join('|') === '8/8 example|Seven|Seven (imported)|Other');
    await pg.click('#preset-dlg .dclose');
    await pickScen(pg, 'unity');
    check('Unity Cup marks No Stopping Me! and It\'s On! as guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '2' && (await pg.getAttribute('#g-20046', 'data-g')) === '2');
    await settle(pg);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async scenario(b) {
    // the scenario picker: a button showing the chosen scenario's logo, opening a grid of the scenarios
    const pg = await open(b);
    await pg.click('#rank-q');
    check('lineage rank: the ? opens its explanation', !(await pg.locator('#rank-note').isHidden()) && (await pg.getAttribute('#rank-q', 'aria-expanded')) === 'true');
    await pg.click('#rank-q');
    const menu = pg.locator('#scen-menu'), tiles = pg.locator('#scen-menu .stile');
    const btnName = () => pg.getAttribute('#x-scen', 'aria-label');
    check('picker closed at first; the button is Unity Cup (logo only, named for assistive tech)', (await menu.isHidden()) && (await scen(pg)) === 'unity' && (await btnName()) === 'Scenario: Unity Cup'
      && (await pg.evaluate(() => self.__LLP_DATA['data-img'].keys.includes('scen:2'))
        ? (await pg.locator('#x-scen').innerText()).trim() === '' && (await pg.locator('#x-scen img').count()) === 1
        : (await pg.locator('#x-scen').innerText()).trim() === 'Unity Cup'));
    const shots = process.env.LLP_SHOTS, bn = b.browserType().name();   // pictures of the button and the open picker, for a look at the logos
    if (shots) await pg.locator('#x-scen').screenshot({ path: `${shots}/${bn}-scen-button.png` });
    await pg.click('#x-scen');
    const t = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect();
      const sub = t.querySelector('.ssub'), sr = sub.getBoundingClientRect(), pad = parseFloat(getComputedStyle(t).paddingBottom) + parseFloat(getComputedStyle(t).borderBottomWidth);
      return { v: t.dataset.v, sub: sub.innerText.replace(/\s+/g, ' ').trim(), w: Math.round(r.width), h: Math.round(r.height), gap: Math.round(r.bottom - pad - sr.bottom), subH: Math.round(sr.height) }; }));
    // hint rate as a multiple of URA Finale's, from the solver's hint_base (pkg/loopopt/src/deck/hints.rs)
    const want = hintRates();
    check('each tile has the same hover text as the button gives the chosen scenario', await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].every(t => t.title && t.title === t.getAttribute('aria-label')) && document.querySelector('#scen-menu .stile[data-v="unity"]').title === document.getElementById('x-scen').title));
    check('opens with four tiles, each with its hint rate against URA Finale', !(await menu.isHidden()) && t.length === 4 && t.every(x => x.sub === want[x.v]), t.map(x => x.v + ': ' + x.sub).join(' | '));
    check('the four tiles are the same size', new Set(t.map(x => x.w + 'x' + x.h)).size === 1, t.map(x => x.w + 'x' + x.h).join(' '));
    check('every tile is logo + one line', t.every(x => x.subH <= 22), t.map(x => `${x.v} line ${x.subH}px, blank ${x.gap}px`).join(' | '));
    // Unity Cup's guaranteed golds: their gold skill icons on the hint-rate line, named by tooltip and alt text
    const golds = await pg.evaluate(() => { const skl = self.__LLP_DATA['data-skills'], gi = self.__LLP_DATA['data-skillicon'].gold;
      const g = document.querySelector('#scen-menu .stile[data-v="unity"] .ssub .sgolds');
      return { want: [20049, 20046].map(x => ({ name: skl.find(s => s[0] === x)[2], src: `/skill/${gi[x]}.avif` })), group: g && g.getAttribute('aria-label'),
        got: g ? [...g.querySelectorAll('img')].map(i => ({ alt: i.alt, title: i.title, src: i.getAttribute('src'), h: Math.round(i.getBoundingClientRect().height) })) : [],
        text: g ? g.innerText.trim() : null, others: document.querySelectorAll('#scen-menu .stile:not([data-v="unity"]) .sgolds').length }; });
    check('Unity Cup tile: No Stopping Me! and It\'s On! as gold icons on the rate line, no visible names', golds.group === 'Guaranteed in every run' && golds.others === 0 && golds.text === ''
      && golds.got.length === 2 && golds.want.every((w, i) => golds.got[i].alt === w.name && golds.got[i].title === w.name && golds.got[i].src.endsWith(w.src) && golds.got[i].h <= 18), JSON.stringify(golds.got));
    check('logos keep their aspect (object-fit: contain)', await pg.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo img')].every(i => i.naturalWidth > 0 && getComputedStyle(i).objectFit === 'contain')));
    check('the open picker marks the chosen tile and focuses it', await pg.evaluate(() => { const t = document.activeElement; return t.classList.contains('stile') && t.dataset.v === 'unity' && t.getAttribute('aria-checked') === 'true'; })
      && (await pg.getAttribute('#x-scen', 'aria-expanded')) === 'true');
    if (shots) await menu.screenshot({ path: `${shots}/${bn}-scen-picker.png` });
    const r0 = await pg.evaluate(() => window.__runs.length);
    await tiles.nth(0).click();   // a change would show "Searching…" at once
    check('re-picking the chosen scenario closes the picker and does not search', (await menu.isHidden()) && (await pg.evaluate(() => window.__runs.length)) === r0 && !/Searching/.test(await note(pg)));
    await pg.keyboard.press('ArrowDown');   // focus is back on the button
    check('arrow key on the button opens the picker', !(await menu.isHidden()));
    await pg.keyboard.press('ArrowRight'); await pg.keyboard.press('ArrowDown');
    check('arrows move between tiles (two columns)', await pg.evaluate(() => document.activeElement.dataset.v) === 'trackblazer');
    await pg.keyboard.press('Escape');
    check('Escape closes it and returns focus to the button', (await menu.isHidden()) && await pg.evaluate(() => document.activeElement.id === 'x-scen'));
    await pg.click('#x-scen'); await pg.click('header.top h1');
    check('a click outside closes it', await menu.isHidden());
    // only the button and the grid count as inside: the label, the row's empty space and the rank control all close it
    const shut = [];
    await pg.click('#x-scen'); await pg.click('#x-scen-l'); shut.push(await menu.isHidden());
    await pg.click('#x-scen'); const bb = await pg.locator('#x-scen').boundingBox(); await pg.mouse.click(bb.x + bb.width + 8, bb.y + bb.height / 2); shut.push(await menu.isHidden());
    await pg.click('#x-scen'); await pg.click('.rankctl > span'); shut.push(await menu.isHidden());
    check('clicks on the label, beside the button, on the rank control close it', shut.every(Boolean), shut.join(','));
    await pg.click('#x-scen'); await pg.keyboard.press('End'); await pg.keyboard.press('Enter'); await lastJob(pg, 'trackblazer');
    check('Enter on a tile picks it and closes the picker', (await menu.isHidden()) && (await scen(pg)) === 'trackblazer' && (await btnName()) === 'Scenario: Trackblazer');
    // the lineage rank follows the scenario: Grand Concert SS, Unity Cup below SS
    await pickScen(pg, 'gc'); await lastJob(pg, 'gc');
    const sirius = await pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? (t.querySelector('.cban') || {}).textContent || '' : null; });
    check('Team Sirius: "Not allowed in Grand Concert" in the card picker', sirius === 'Not allowed in Grand Concert', String(sirius));
    check('picking Grand Concert sets the rank to SS', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    await pickScen(pg, 'unity'); await settle(pg);
    check('picking Unity Cup sets it back to below SS', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05');
    await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg); await pickScen(pg, 'unity');
    check('a rank changed by hand sticks (re-picking the scenario leaves it)', (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && await pg.evaluate(() => window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'));
    // the rank toggle is a radio group: game badges, arrow keys move the choice
    await pg.focus('#rank button[aria-checked="true"]'); await pg.keyboard.press('ArrowLeft');
    check('lineage rank: arrow keys switch it, aria-checked follows, badges shown', (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
      && (await pg.getAttribute('#rank button[data-v="0.50,0.45,0.05"]', 'aria-checked')) === 'true' && (await pg.locator('#rank .rk.img img').count()) === 3);
    await pg.keyboard.press('ArrowRight'); await settle(pg);
    await pg.click('#ex-open');
    check('export names the scenario', JSON.parse(await pg.inputValue('#ex-text')).readable.scenario === 'Unity Cup');
    const text = (await pg.inputValue('#ex-text')).replace('"scen": "unity"', '"scen": "gc"');
    await pg.fill('#ex-text', text); await pg.click('#ex-load'); await pg.click('#ex-close');
    check('import updates the button', (await scen(pg)) === 'gc' && (await btnName()) === 'Scenario: Grand Concert');   // its (slow) search is not awaited
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
    // without the logo files (images not built, or a failed fetch) the button and tiles show the name on a badge
    const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', r => r.abort()) });
    await until(p2, () => !document.querySelector('#scen-dd .slogo img')); await p2.click('#x-scen');
    const badges = await p2.evaluate(() => [...document.querySelectorAll('#scen-dd .slogo')].map(l => (l.querySelector('img') ? 'img' : '') + (l.querySelector('b') || {}).textContent).join(','));
    check('missing logos fall back to name badges', badges === 'Unity Cup,Unity Cup,Grand Concert,URA Finale,Trackblazer', badges);
    const sz = await p2.evaluate(() => new Set([...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); })).size);
    check('… and the tiles stay the same size', sz === 1);
    await p2.context().close();
  },
  async autosearch(b) {
    const pg = await open(b);
    // auto-search off: a change waits for the Search button; back on, it searches
    check('auto-search is on by default', await pg.locator('#x-auto').isChecked());
    await pg.click('#x-auto'); const n0 = await pg.evaluate(() => window.__runs.length);
    check('switching auto-search off leaves the results as they are (not out of date)', /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)) && !(await pg.evaluate(() => document.getElementById('results').classList.contains('stale'))));
    await pg.locator('#cards .ct').nth(6).click(); await pg.waitForTimeout(1300);
    check('auto-search off: a change does not search, the status says to press Search', (await pg.evaluate(() => window.__runs.length)) === n0 && /press Search/.test(await note(pg)) && (await pg.locator('#r-act button', { hasText: 'Search' }).count()) === 1);
    await pg.locator('#r-act button', { hasText: 'Search' }).click(); await settle(pg);
    check('… the Search button runs it', (await pg.evaluate(() => window.__runs.length)) === n0 + 1 && /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)));
    await pg.locator('#cards .ct').nth(6).click(); await pg.click('#x-auto'); await settle(pg);
    check('… turning it back on searches the change', (await pg.evaluate(() => window.__runs.length)) === n0 + 2);
    const made0 = await pg.evaluate(() => window.__made);
    await pg.evaluate(() => [...document.querySelectorAll('#cards .ct')].slice(0, 5).forEach(b => b.click()));   // five changes inside one pause
    check('a change shows "Searching…"', /Searching/.test(await note(pg)));
    // the fast pass is drawn first, with the status saying the exact check is still running
    const checking = await pg.waitForFunction(() => /Checking the exact odds/.test(document.getElementById('run-note').textContent), null, { timeout: 30000 }).then(() => true, () => false);
    check('the first pass shows while the exact check runs', checking && (await pg.locator('#results .loop').count()) > 0, await note(pg));
    await settle(pg);
    check('a burst of 5 changes is one search on the same worker', (await pg.evaluate(() => window.__made)) === made0);
    // a search changed mid-run (3 results, one shared deck; its replies held so it is still running however fast it
    // is): the worker is killed and the new search finishes
    const r0 = await pg.evaluate(() => { window.__hold = true; return window.__runs.length; });
    await pg.click('#c-top [data-v="3"]'); await pg.click('#c-peruma');
    await pg.waitForFunction((n) => window.__runs.length > n && window.__runs.at(-1).top_decks === 3 && window.__runs.at(-1).per_uma === false, r0);   // the slow search itself is running (not a quick job from the first change alone)
    const k0 = await pg.evaluate(() => window.__killed);
    check('old results stay visible while searching', (await pg.locator('#results .loop').count()) > 0);
    await pg.waitForFunction(() => /(Searching|Checking the exact odds)…\s*\d+ s/.test(document.getElementById('run-note').textContent), null, { timeout: 30000 });   // its clock is running
    await pg.click('#c-peruma'); const paused = await note(pg); await pg.evaluate(() => window.__release());
    check('a change mid-search clears the clock: it counts again only once the new search runs', /Searching/.test(paused) && !/\d+ s/.test(paused), paused);
    await pg.click('#c-top [data-v="1"]'); await settle(pg);
    check('a change mid-search restarts the worker', (await pg.evaluate(() => window.__killed)) > k0);
    check('the restarted search finishes', /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async roster(b) {
    const pg = await open(b);
    // as the page shows them: outfits dated after now hidden, an uma whose every outfit is (data.js LATER)
    const umas = (await json(pg, 'data-umas')).map(u => ({ ...u, outfits: u.outfits.filter(o => o.rel <= Date.now() / 1000), had: u.outfits.length }))
      .filter(u => u.outfits.length || !u.had);
    const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
    check('one tile per outfit', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
    const all = umas.flatMap(u => (u.outfits.length ? u.outfits : [{ label: '', rel: 0, id: 0 }]).map(o => ({ n: u.name + (o.label ? ' ' + o.label : ''), rel: o.rel, id: o.id })));
    const firstTile = async () => (await pg.locator('#roster .ut').first().innerText()).replace(/\n/g, ' ').trim();
    const newest = [...all].sort((a, c) => (c.rel - a.rel) || (c.id - a.id))[0].n, oldest = [...all].sort((a, c) => (a.rel - c.rel) || (a.id - c.id))[0].n;
    check('default sort: release, newest first', (await pg.locator('#rs').innerText()) === 'Release ↓' && await firstTile() === newest, await firstTile());
    await pg.click('#rs'); await pg.click('#rs-menu [data-k="rel"]');
    check('re-picking Release reverses it (oldest first)', (await pg.locator('#rs').innerText()) === 'Release ↑' && await firstTile() === oldest, await firstTile());
    check('the sort menu stays open after a pick', !(await pg.locator('#rs-menu').isHidden()));
    await pg.click('#rs-menu [data-k="name"]');
    const byName = [...all].sort((a, c) => a.n.localeCompare(c.n))[0].n;
    check('Name sorts A to Z', (await pg.locator('#rs').innerText()) === 'Name ↑' && await firstTile() === byName, await firstTile());
    await pg.keyboard.press('Escape'); check('Escape closes the menu', await pg.locator('#rs-menu').isHidden());
    // an uma with several outfits: a tap marks that outfit not owned, a second tap owned again
    const multi = umas.find(u => u.outfits.length > 1);
    const states = () => pg.evaluate((id) => [...document.querySelectorAll(`#roster .ut[data-uma="${id}"]`)].map(t => t.dataset.s).join(','), multi.id);
    const tile = pg.locator(`#roster .ut[data-uma="${multi.id}"] .up`).first();
    await tile.click(); const s1 = await states(); await tile.click(); const s2 = await states();
    check('tile toggle: that outfit not owned', s1.split(',').filter(x => x === 'out').length === 1, s1);
    check('tile toggle: back to owned', s2.split(',').every(x => x === 'own'), s2);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  // a build holds the next banner ahead of its start (pkg/loopopt/data/global/upcoming.json): hidden and left out of every search
  // until its start time, then there without a rebuild (the page's clock moved past it)
  async upcoming(b) {
    const up = JSON.parse((await import('fs')).readFileSync(new globalThis.URL('../pkg/loopopt/data/global/upcoming.json', import.meta.url), 'utf8'));
    const later = Object.fromEntries(Object.entries(up).map(([k, l]) => [k, l.filter(([, t]) => t > Date.now() / 1000)]));
    const all = Object.values(later).flat();
    if (!all.length) { check('upcoming: nothing in this build starts later (nothing to check)', true); return; }
    const at = Math.max(...all.map(([, t]) => t)) * 1000 + 60000;
    const look = async (init) => {
      const pg = await open(b, { init });
      const tiles = await pg.locator('#roster .ut').count(), cards = await pg.locator('#cards .ct').count();
      await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg);
      const job = await pg.evaluate(() => window.__runs.filter(j => j.cmd === 'deck').at(-1));
      const errs = pg.errors; await pg.context().close();
      return { tiles, cards, job, errs };
    };
    const ids = (k) => later[k].map(([id]) => id), has = (l, xs) => xs.every(x => (l || []).includes(x)), none = (l, xs) => !xs.some(x => (l || []).includes(x));
    const now = await look(null);
    check('upcoming: what starts later is left out of the search', now.job && has(now.job.exclude, ids('umas')) && has(now.job.exclude_outfits, ids('outfits')) && has(now.job.exclude_cards, ids('cards')),
      JSON.stringify(later));
    const then = await look(`{ const off = ${at} - Date.now(), n = Date.now; Date.now = () => n() + off; }`);
    // R cards stay out by default whatever their date (the page's R-cards switch)
    const rare = new Set(JSON.parse((await import('fs')).readFileSync(new globalThis.URL('../pkg/loopopt/data/global/web_cards.json', import.meta.url), 'utf8')).cards.filter(c => c.rarity > 1 || c.pal).map(c => c.id));
    check('upcoming: once started it is in the search', then.job && none(then.job.exclude, ids('umas')) && none(then.job.exclude_outfits, ids('outfits'))
      && none(then.job.exclude_cards, ids('cards').filter(id => rare.has(id))), JSON.stringify(then.job && then.job.exclude_cards.filter(id => ids('cards').includes(id))));
    check('upcoming: once started its outfits and cards are on the page', then.tiles - now.tiles === ids('outfits').length && then.cards > now.cards,
      `tiles ${now.tiles} -> ${then.tiles}, cards ${now.cards} -> ${then.cards}`);
    check('no page errors', !now.errs.length && !then.errs.length, [...now.errs, ...then.errs].join(' | '));
  },
  async cards(b) {
    const pg = await open(b);
    const secs = await pg.evaluate(() => [...document.querySelectorAll('#cards .cgroup h3')].map(h => h.textContent));
    check('card sections SSR, SR, R in that order', secs.join(',') === 'SSR,SR,R', secs.join(','));
    const notes = await pg.evaluate(() => [...document.querySelectorAll('#cards .gv')].map(g => [g.querySelector('small').textContent, g.title]));
    check('card hint notes in plain words, each with a tooltip', notes.length > 0 && notes.every(([n, t]) => /^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n) && t.length > 0), JSON.stringify(notes.filter(([n]) => !/^(hint|event|\d(st|nd|rd|th) event)( · \d+%)?$/.test(n)).slice(0, 3)));
    const art = await pg.evaluate(() => { const s = new Set(); let over = 0; for (const t of document.querySelectorAll('#cards .ct')) { const c = t.querySelector('.cav').getBoundingClientRect();
      s.add(Math.round(c.width)); if (c.width > t.getBoundingClientRect().width + 0.5) over++; } return { sizes: [...s], over }; });
    check('card list: every card picture is the same size and inside its tile, loaded or not', art.sizes.length === 1 && !art.over, JSON.stringify(art));
    check('card names carry no rarity (the frame and section show it)', await pg.evaluate(() => ![...document.querySelectorAll('#cards .ct .cn')].some(e => / (SSR|SR|R)$/.test(e.textContent))));
    // the game bans Team Sirius from Unity Cup and Grand Concert decks: the picker says so and no result deck holds it
    const sirius = () => pg.evaluate(() => { const t = [...document.querySelectorAll('#cards .ct')].find(x => /Team Sirius/.test(x.textContent)); return t ? (t.querySelector('.cban') || {}).textContent || '' : null; });
    check('Team Sirius: "Not allowed in Unity Cup" in the card picker', (await sirius()) === 'Not allowed in Unity Cup', String(await sirius()));
    check('… and in no Unity Cup result deck', await pg.evaluate(() => ![...document.querySelectorAll('#results .dc')].some(d => d.dataset.cid === '30081')));
    await pickScen(pg, 'ura');   // the picker follows at once; the search is not awaited
    check('Team Sirius allowed again in URA Finale', (await sirius()) === '');
    await pickScen(pg, 'unity'); await settle(pg);
    await pg.editor('#cards');
    const sep = await pg.evaluate(() => { const c = document.getElementById('cards'), r = c.previousElementSibling.getBoundingClientRect(), cs = getComputedStyle(c);
      return { gap: Math.round(c.getBoundingClientRect().top - r.bottom), pad: parseFloat(cs.paddingTop) }; });
    check('the card list scrolls apart from its search row (gap like the roster\'s), its tiles\' rings and focus outline inside it', sep.gap >= 10 && sep.pad >= 5, JSON.stringify(sep));
    check('default card sort: release, newest first', (await pg.locator('#cs').innerText()) === 'Release ↓');
    const tile = pg.locator('#cards .ct').first(); const img = await tile.evaluate(t => { t.__img = t.querySelector('img'); return !!t.__img; });
    await tile.click();
    check('toggling a card keeps its image element (no flicker)', img && await tile.evaluate(t => t.querySelector('img') === t.__img && t.getAttribute('aria-pressed') === 'false'));
    await tile.click();
    // R cards are left out (no setting): the R section holds only the R pals
    const rpals = await pg.evaluate(() => self.__LLP_DATA['data-cards'].filter(c => c.rar === 1 && c.pal).length);
    const rCount = () => pg.evaluate(() => { const g = [...document.querySelectorAll('#cards .cgroup')].find(x => x.querySelector('h3').textContent === 'R'); return g ? g.querySelectorAll('.ct').length : 0; });
    check('R cards are left out, with no switch; the R section keeps the R pals', (await pg.locator('#x-rcards').count()) === 0 && (await rCount()) === rpals, (await rCount()) + ' of ' + rpals);
    check('every card image carries its type icon', (await pg.locator('#cards .ct .cav.art').count()) === (await pg.locator('#cards .ct .ctype').count()));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async panels(b) {
    const pg = await open(b);
    const dc = pg.locator('.deck .dc').first(); await dc.click();
    const href = await pg.locator('.cpop .gtl').getAttribute('href');
    check('deck card panel: GameTora card link', /^https:\/\/gametora\.com\/umamusume\/supports\/\d+-[a-z0-9-]+$/.test(href || ''), href);
    const side = await pg.evaluate(() => { const c = document.querySelector('.deck .dc').getBoundingClientRect(), p = document.querySelector('.cpop').getBoundingClientRect(); return { beside: p.left >= c.right || p.right <= c.left, top: Math.abs(p.top - c.top) }; });
    check('deck card panel opens beside the card (desktop), not over the deck below', side.beside && side.top <= 2, JSON.stringify(side));
    const cid = await dc.getAttribute('data-cid');
    await pg.locator('.cpop .sw').click();
    check('deck card Owned switch updates the card picker', await pg.evaluate((id) => [...document.querySelectorAll('#cards .ct')].some(t => t.querySelector('img') && t.getAttribute('aria-pressed') === 'false'), cid));
    check('… and greys the card in the result', (await pg.locator('.deck .dc.off').count()) > 0);
    await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape');
    check('Escape closes the panel', await pg.evaluate(() => document.querySelector('.cpop').hidden));
    // an outfit skill under a member carries its skill icon
    const sp = pg.locator('.loop').first().locator('.mem .spark').first();
    check('outfit skill shows its skill icon', (await sp.locator('img.sic').count()) === 1);
    const mb = pg.locator('.loop').first().locator('.mem .memb').nth(1); await mb.click();
    await pg.keyboard.press('Escape');
    const fb = pg.locator('.loop').first().locator('.swho .facebtn').first();
    if (await fb.count()) { const id = await fb.getAttribute('data-uma'); await fb.click();
      check('a portrait in a swap row opens the same member panel', !(await pg.locator('.cpop').isHidden()) && (await pg.locator('.cpop input[data-act="uma"]').count()) === 1
        && (await pg.locator('.cpop b').first().innerText()) === await pg.locator(`.loop .mem[data-uma="${id}"] .nm`).first().innerText());
      await pg.keyboard.press('Escape'); await mb.click(); }
    check('member panel has one Owned switch', (await pg.locator('.cpop input[role=switch]').count()) === 1);
    const uh = await pg.locator('.cpop .gtl').getAttribute('href');
    check('member panel: GameTora character link', /^https:\/\/gametora\.com\/umamusume\/characters\/\d{6}-[a-z0-9-]+$/.test(uh || ''), uh);
    await pg.locator('.cpop .sw').click();
    check('member Owned switch updates the roster and greys her', (await pg.locator('#roster .ut[data-s="out"]').count()) === 1 && (await pg.locator('.mem.off').count()) > 0);
    await pg.locator('.cpop .sw').click(); await pg.keyboard.press('Escape'); await settle(pg);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async targets(b) {
    // the target list by keyboard: removing keeps the focus in the list, the add box takes Enter and the arrows
    const pg = await open(b);
    const skl = await json(pg, 'data-skills'), gold = (g) => skl.find(x => x[0] === g)[2];
    // the add box sits above the list, two targets per row
    await pg.editor('#targets');
    const lay = await pg.evaluate(() => { const t = [...document.querySelectorAll('#targets .tgt')].map(e => e.getBoundingClientRect());
      return { rows: new Set(t.map(r => Math.round(r.top))).size, n: t.length, addFirst: document.getElementById('t-add').getBoundingClientRect().bottom <= Math.min(...t.map(r => r.top)) }; });
    check('targets: add box first, two per row', lay.addFirst && lay.rows === Math.ceil(lay.n / 2), JSON.stringify(lay));
    check('a gold the scenario guarantees shows as the gold itself, in gold', (await pg.locator('#g-20049 .name').innerText()).trim() === gold(20049) && await pg.locator('#g-20049').evaluate(e => e.classList.contains('gold')), await pg.locator('#g-20049').innerText());
    await pg.locator('#targets .x').nth(2).focus(); await pg.keyboard.press('Enter');
    check('removing a target by keyboard moves the focus to the next remove button', (await pg.locator('#targets .tgt').count()) === 7
      && await pg.evaluate(() => document.activeElement.classList.contains('x') && [...document.querySelectorAll('#targets .x')].indexOf(document.activeElement) === 2));
    for (let i = 0; i < 7; i++) await pg.keyboard.press('Enter');
    check('… and to the add box after the last one', (await pg.locator('#targets .tgt').count()) === 0 && await pg.evaluate(() => document.activeElement.id === 't-add'));
    await pg.fill('#t-add', 'corner');
    const first = await pg.locator('#t-sugg button').first().innerText();
    await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('ArrowUp');
    const at = await pg.evaluate(() => [...document.querySelectorAll('#t-sugg button')].indexOf(document.activeElement));
    await pg.keyboard.press('ArrowUp'); const back = await pg.evaluate(() => document.activeElement.id);
    check('arrows walk the suggestions, Up from the first returns to the box', at === 0 && back === 't-add', at + ' ' + back);
    await pg.keyboard.press('ArrowDown'); await pg.keyboard.press('Escape');
    check('Escape on a suggestion closes the list and returns to the box', await pg.locator('#t-sugg').isHidden() && await pg.evaluate(() => document.activeElement.id === 't-add'));
    await pg.fill('#t-add', 'corner'); await pg.keyboard.press('Enter');
    check('Enter adds the first match and keeps the focus in the box', (await pg.locator('#targets .tgt').count()) === 1 && (await pg.locator('#targets .tgt').innerText()).includes(first.split(' → ')[0]) && await pg.evaluate(() => document.activeElement.id === 't-add'), first);
    await pg.fill('#t-add', first.split(' → ')[0].slice(0, 8));
    check('a skill already listed: "… is already a target"', (await pg.locator('#t-sugg .snote').allInnerTexts()).some(t => t === first.split(' → ')[0] + ' is already a target'));
    await pg.fill('#t-add', 'zzqx');
    check('no match: "No skill matches"', /^No skill matches/.test(await pg.locator('#t-sugg').innerText()));
    // a gold matched by its own name shows with its white
    const g = skl.find(x => x[2] && !x[1].toLowerCase().includes(x[2].toLowerCase().slice(0, 6)) && !skl.some(y => y !== x && (y[1] + (y[2] || '')).toLowerCase().includes(x[2].toLowerCase())));
    await pg.fill('#t-add', g[2]);
    check('a match on the gold\'s name shows "white → gold"', (await pg.locator('#t-sugg button').first().innerText()) === g[1] + ' → ' + g[2], g[1] + ' → ' + g[2]);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async keyboard(b) {
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
  },
  async states(b) {
    // what the status line says when the settings can't be searched, and the results on screen meanwhile (kept, dimmed)
    // (window.__trap: the next jobs reach the worker without their job, so they fail inside it, as a solver trap would)
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
      return L.map((l, i) => ({ rank: (l.querySelector('.rank') || {}).textContent, same: !!l.querySelector('.same'), mem: l.querySelectorAll('.mem').length,
        sameAs1: i > 0 && !l.querySelector('.same') && key(l) === key(L[0]) })); });
    check('three results: ranked #1, #2, #3', r.map(x => x.rank).join() === '#1,#2,#3', JSON.stringify(r));
    check('a result with #1\'s loop says "Same loop as #1" instead of repeating the umas', r.some(x => x.same) && r.every(x => x.same ? x.mem === 0 : x.mem === 3 || x.mem === 4) && !r.some(x => x.sameAs1), JSON.stringify(r));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  // three results per uma: the four-uma loops behind the rotation are drawn with four members, the "All four" tab and
  // the 12 careers
  async fourloop(b) {
    const pg = await open(b);
    await pg.click('#c-top [data-v="3"]'); await settle(pg);
    const sizes = await pg.evaluate(() => [...document.querySelectorAll('#results .loop')].map(l => l.querySelectorAll('.mem').length));
    check('three results: a rotation first, then four-uma loops', sizes.length === 3 && sizes[0] === 3 && sizes.slice(1).every(n => n === 4 || n === 0), JSON.stringify(sizes));
    const i = sizes.indexOf(4), loop = pg.locator('#results .loop').nth(i);
    check('a four-uma loop: four members in four columns', i > 0 && await loop.evaluate((l) => { const m = l.querySelector('.members'); return !m.classList.contains('n3') && getComputedStyle(m).gridTemplateColumns.split(' ').length === 4; }));
    await loop.locator('.lacts button', { hasText: 'Race schedule' }).click(); await dialog(pg, 'sched-dlg');
    check('its schedule: a tab per member plus "All four"', (await pg.locator('#sched-dlg .stab').count()) === 5 && (await pg.locator('#sched-dlg .stab').last().innerText()) === 'All four');
    await pg.locator('#sched-dlg .stab').last().click();
    check('All four: a column per member', (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 4);
    await pg.keyboard.press('Escape');
    await loop.locator('.lacts button', { hasText: 'Affinity' }).click(); await dialog(pg, 'why-dlg');
    // a four-uma loop's 12 careers: one pill per trainee (4), then her parents (3 pairs), never the same uma twice in a row
    const afT = await pg.evaluate(() => [...document.querySelectorAll('#why-dlg .afpick:not(.afpar) button')].map(b => b.textContent.replace(/\d+/g, '').trim()));
    check('its affinity: one pill per trainee, then her three parent pairs; six legacies each, no grandparent is the trainee', afT.length === 4 && new Set(afT).size === 4
      && await pg.locator('#why-dlg .afpar button').count() === 3 && await pg.locator('#why-dlg .afftab tbody tr').count() === 6
      && !(await pg.locator('#why-dlg .afftab .aft').count()), afT.join(', '));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async borrowed(b) {
    const pg = await open(b);
    for (const nm of ['Light Hello', 'Sasami Anshinzawa']) await pg.locator('#cards .cgroup').first().locator('.ct', { hasText: nm }).first().click();   // the SSR section
    await settle(pg);
    const rows = await pg.evaluate(() => [...document.querySelectorAll('.deck .dcards')].map(r => [...r.querySelectorAll('.dc')]).filter(r => r.some(x => x.classList.contains('bor'))).map(r => r.findIndex(x => x.classList.contains('bor')) === r.length - 1));
    check('a deck borrows a card when the pals are not owned', rows.length > 0);
    check('the borrowed card is last in its row', rows.every(Boolean));
    check('the borrowed card is named as borrowed', await pg.evaluate(() => [...document.querySelectorAll('.dc.bor')].every(d => /borrowed/.test(d.getAttribute('aria-label')))));
    check('the borrowed card has the friend frame', await pg.evaluate(() => { const d = document.querySelector('.dc.bor'); return !!d && getComputedStyle(d, '::after').borderImageSource.startsWith('url'); }));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async popups(b) {
    const pg = await open(b);
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
    check('schedule: a tab per member of the rotation plus "All three"', (await pg.locator('#sched-dlg .stab').count()) === 4 && (await pg.locator('#sched-dlg .stab').last().innerText()) === 'All three');
    const grids = await pg.evaluate(() => [...document.querySelectorAll('.ag-grid')].map(g => { const r = g.getBoundingClientRect(); return Math.round(r.width) + 'x' + Math.round(r.height); }));
    check('agenda: three identical year grids', grids.length === 3 && new Set(grids).size === 1, grids.join(' '));
    check('agenda: Junior Year opens with the game\'s grey cells for April to June', (await pg.locator('#sched-dlg .ag-year.y0 .ag-slot.pre').count()) === 12);
    check('agenda: fits the window without scrolling', await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); return d.scrollHeight <= d.clientHeight + 1; }));
    artCheck('agenda: every race has its banner', (await pg.locator('#sched-dlg .ag-race').count()) === (await pg.locator('#sched-dlg .ag-race img.banner').count()));
    check('agenda: a race\'s "x% win" sits in its turn bar, its tooltip says why', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .ag-slot.on .ag-when')].every(w => /^(Early|Late) [A-Z][a-z]{2}\d+% win$/.test(w.textContent) && /^\d+% win: 110/.test(w.querySelector('b').title || w.querySelector('b').dataset.tip))));   // (a hovered one's title is held in data-tip while its balloon shows)
    await pg.locator('#sched-dlg .stab').last().click(); check('All three shows the comparison table, a column per member', (await pg.locator('#sched-dlg table.sched').count()) > 0
      && (await pg.locator('#sched-dlg table.sched').first().locator('th.mh').count()) === 3);
    check('All three: turns written as on the member tabs ("Early Apr"), no shared/optional key', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg td.when')].every(t => /^(Early|Late) [A-Z][a-z]{2}$/.test(t.textContent)))
      && await pg.locator('#sched-dlg .agkey').isHidden());
    check('All three: each win % says why on hover, as its note promises', await pg.evaluate(() => { const c = [...document.querySelectorAll('#sched-dlg table.sched tbody td')].filter(t => /^\*?\d+$/.test(t.textContent));
      return c.length > 0 && c.every(t => /^\d+% win: 110/.test(t.title)) && /hover for why/.test(document.querySelector('#sched-dlg .tablewrap .hint').textContent); }));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
    // the career page's table: each parent, then her two parents (relation, G1 wins shared, affinity), a Total row
    const at = await pg.evaluate(() => { const t = document.querySelector('#why-dlg .afftab'), rows = [...t.tBodies].flatMap(b => [...b.rows]);
      const n = (x) => +x.replace(/\D/g, ''), aff = rows.map(r => n(r.cells[3].textContent));
      return { careers: document.querySelectorAll('#why-dlg .afpick:not(.afpar) button').length, rows: rows.length, own: rows.filter(r => /her own earlier veteran/.test(r.textContent)).map(r => n(r.cells[1].textContent)),
        sum: aff.reduce((a, x) => a + x, 0), total: n(t.tFoot.rows[0].cells[3].textContent), prose: document.querySelectorAll('#why-dlg p').length }; });
    check('affinity: a rotation has 3 careers; each shows six legacies and a Total that sums them, no prose', at.careers === 3 && at.rows === 6 && Math.abs(at.sum - at.total) <= 6 && at.prose === 0, JSON.stringify(at));
    check('affinity: in a rotation the trainee is her own grandparent twice, marked, with 0 relation points', at.own.length === 2 && at.own.every(x => x === 0), JSON.stringify(at.own));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async phone(b) {
    const pg = await open(b, { w: 375, h: 812, mobile: true });
    const small = await pg.evaluate(() => [...document.querySelectorAll('.lp .rbbar button, .lp .rbbar select, #rhead button, #rhead select, .loop .lacts button, .mem .memb')]
      .filter(e => e.offsetParent && !e.closest('.roster, .cards')).map(e => [e.id || e.className || e.tagName, Math.round(e.getBoundingClientRect().height)]).filter(([, h]) => h < 44));
    check('phone: buttons, selects and search boxes are at least 44 px high', !small.length, JSON.stringify(small.slice(0, 6)));
    // results first: above the settings, and first in the document (reading and Tab order), the result in the first screen
    const order = await pg.evaluate(() => { const c = document.getElementById('rcol'), r = document.querySelector('.lp .rbbar'), o = document.querySelector('#results .loop .odds').getBoundingClientRect();
      return { above: c.getBoundingClientRect().top < r.getBoundingClientRect().top, dom: !!(c.compareDocumentPosition(r) & Node.DOCUMENT_POSITION_FOLLOWING), odds: Math.round(o.bottom), h: innerHeight }; });
    check('phone: the results come before the settings (on screen and in the Tab order), the odds in the first screen', order.above && order.dom && order.odds < order.h, JSON.stringify(order));
    if (b.browserType().name() === 'chromium') { const cls = await pg.evaluate(() => new Promise(r => { let v = 0; new PerformanceObserver(l => { for (const e of l.getEntries()) if (!e.hadRecentInput) v += e.value; }).observe({ type: 'layout-shift', buffered: true }); setTimeout(() => r(v), 200); }));
      check('phone: the page barely shifts while loading (CLS < 0.05)', cls < 0.05, cls.toFixed(4)); }
    // the floating status button: shown only while the results are out of view, on screen wherever the page is scrolled
    const H = await pg.evaluate(() => document.documentElement.scrollHeight); const seen = [];
    for (const f of [0, 0.5, 1]) { await pg.evaluate((y) => scrollTo(0, y), f * H);   // the button follows an IntersectionObserver
      seen.push(await until(pg, () => { const p = document.getElementById('run-pill'), r = p.getBoundingClientRect(), c = document.getElementById('rcol').getBoundingClientRect();
        const resIn = c.bottom > 0 && c.top < innerHeight, shown = getComputedStyle(p).display !== 'none'; return shown === !resIn && (!shown || (r.top >= 0 && r.bottom <= innerHeight)); })); }
    check('phone: the status button shows (on screen) exactly when the results are out of view', seen.every(Boolean), seen.join(','));
    check('phone: it shows the headline odds', /^1 in 107 · see result/.test(await pg.locator('#run-pill').innerText()), await pg.locator('#run-pill').innerText());
    // edit a setting far down, then the button takes you back to the result
    const ct = pg.locator('#cards .ct').nth(2); await ct.scrollIntoViewIfNeeded(); await ct.click(); await settle(pg); await ct.click(); await settle(pg);
    await pg.click('#run-pill');   // a smooth scroll
    check('phone: after editing a card, the status button scrolls back to the results', await until(pg, () => { const r = document.getElementById('rcol').getBoundingClientRect(); return Math.abs(r.top) < 40 && document.activeElement.id === 'rcol'; }));
    // the roster and card boxes: shorter than the screen, their bottom edge fading while there is more below
    const boxes = []; for (const id of ['roster', 'cards']) { await pg.editor('#' + id); await pg.waitForTimeout(100);
      boxes.push(await pg.evaluate((id) => { const e = document.getElementById(id); return { h: e.clientHeight / innerHeight, more: e.classList.contains('more') }; }, id)); }
    const part = await pg.evaluate(() => { const s = new Set(); let over = 0; for (const t of document.querySelectorAll('#cards .ct')) { const c = t.querySelector('.cav').getBoundingClientRect();
      s.add(Math.round(c.width)); if (c.width > t.getBoundingClientRect().width + 0.5) over++; } return { sizes: [...s], over }; });
    check('phone: every card picture is the same size and inside its tile, loaded or not', part.sizes.length === 1 && !part.over, JSON.stringify(part));
    check('phone: the roster and card boxes fit in their dialogs and show that they scroll', boxes.every(x => x.h <= 0.8 && x.more), JSON.stringify(boxes));
    await pg.evaluate(() => { document.querySelectorAll('dialog[open]').forEach(d => d.close()); scrollTo(0, 0); });
    await pg.click('#x-scen');
    check('phone: the open scenario picker fits the screen', await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect();
      return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
    const ph = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(), s = t.querySelector('.ssub').getBoundingClientRect();
      return Math.round(r.width) + 'x' + Math.round(r.height) + ' line ' + Math.round(s.height); }));
    artCheck('phone: the scenario tiles are the same size, each logo + one line', new Set(ph.map(x => x.split(' ')[0])).size === 1 && ph.every(x => +x.split(' ').at(-1) <= 22), ph.join(' | '));
    await pg.keyboard.press('Escape');
    await pg.click('#rank-q');
    check('phone: the open lineage-rank popup fits the screen', await pg.evaluate(() => { const r = document.getElementById('rank-note').getBoundingClientRect();
      return r.width > 0 && r.left >= 0 && r.right <= innerWidth && document.documentElement.scrollWidth <= document.documentElement.clientWidth; }));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg');
    check('phone: agenda stacks without the grey Junior cells', (await pg.locator('#sched-dlg .ag-slot.pre:visible').count()) === 0);
    await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); d.scrollTo(0, d.scrollHeight); });
    const closeVis = () => pg.evaluate(() => { const b = [...document.querySelectorAll('dialog[open] .dhead button')].find(x => x.textContent === 'Close'), r = b.getBoundingClientRect(); return r.top >= 0 && r.bottom <= innerHeight; });
    check('phone: the schedule\'s Close stays on screen at the bottom of the dialog', await closeVis());
    await pg.locator('#sched-dlg .stab').last().click();
    check('phone: the All three table fits the 375 px screen', await pg.evaluate(() => [...document.querySelectorAll('#sched-dlg .tablewrap')].every(w => w.scrollWidth <= w.clientWidth)));
    await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg');
    await pg.evaluate(() => { const d = document.querySelector('#why-dlg .dlg'); d.scrollTo(0, d.scrollHeight); });
    check('phone: the affinity\'s Close stays on screen too', await closeVis());
    await pg.keyboard.press('Escape');
    await pg.click('#ex-open'); check('the export dialog\'s Close sits in the same header row as the others', await pg.evaluate(() => document.getElementById('ex-close').closest('.dhead') !== null)); await pg.keyboard.press('Escape');
    // the pill switches (Game data, lineage rank, Results): the chosen part fills its share of the track, flush
    await pg.evaluate(() => scrollTo(0, 0));
    const pills = await pg.evaluate(PILLS);
    check('phone: every pill track\'s segments fill it flush (full height, no gap at its ends or between them)', !pills.length && await pg.locator('.seg:visible').count() >= 3, pills.slice(0, 4).join(' | '));
    // nothing scrolls sideways with the editors and the result's popups open
    const side = [];
    for (const [sel, id, text] of [['#o-umas', 'set-umas'], ['#o-cards', 'set-cards'], ['.loop .lacts button', 'why-dlg', 'Affinity'], ['.loop .lacts button', 'odds-dlg', 'Odds']]) {
      const loc = text ? pg.locator(sel, { hasText: text }).first() : pg.locator(sel); await loc.scrollIntoViewIfNeeded(); await loc.click(); await dialog(pg, id);
      side.push(...(await pg.evaluate(SIDEWAYS)).map(x => id + ': ' + x)); await pg.evaluate(() => document.querySelectorAll('dialog[open]').forEach(d => d.close())); }
    check('phone: nothing scrolls sideways with Umas, Support cards, Affinity or Odds open', !side.length, side.slice(0, 4).join(' | '));
    await pg.setViewportSize({ width: 1280, height: 800 });   // the page reorders on its resize handler
    check('widened to desktop, the results go back beside (after) the settings', await until(pg, () => !!(document.querySelector('.lpside').compareDocumentPosition(document.getElementById('rcol')) & Node.DOCUMENT_POSITION_FOLLOWING)));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async careerphone(b) {
    // the career page on a 375 px phone (iPhone 13 mini) with the sample veterans' answer: nothing scrolls sideways (page or dialogs), the
    // setup rows inside their panel, the pill tracks flush, the pick in Options shaded as one card
    const chromium = b.browserType().name() === 'chromium';
    const ctx = await b.newContext({ viewport: { width: 375, height: 812 }, ...(chromium ? { isMobile: true, hasTouch: true } : {}) });
    const pg = await ctx.newPage(); pg.errors = [];
    pg.on('pageerror', e => pg.errors.push(String(e))); pg.on('console', m => m.type() === 'error' && !/downloadable font/.test(m.text()) && pg.errors.push(m.text()));
    await pg.goto(URL.replace(/\/?$/, '/') + 'career');
    await pg.waitForFunction(() => /Try sample veterans/.test(document.getElementById('run-act')?.textContent || ''), null, { timeout: 180000 });
    await pg.click('#run-act');
    await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent) && document.querySelector('.ud .udtabs'), null, { timeout: 300000 });
    const inside = await pg.evaluate(() => { const p = document.querySelector('.rbbar').getBoundingClientRect();
      return [...document.querySelectorAll('.rbbar .setlist > *, .rbbar .setlist .seg, .rbbar .setlist .scenbtn, .rbbar .lptgt .adder')].filter(e => e.getBoundingClientRect().width)
        .filter(e => { const r = e.getBoundingClientRect(); return r.left < p.left || r.right > p.right; }).map(e => e.id || e.className); });
    check('career phone: no sideways scroll; the setup rows and the add-a-skill box sit inside their panel', !(await pg.evaluate(SIDEWAYS)).length && !inside.length
      && (await pg.locator('.rbbar .lptgt .adder input').boundingBox()).width > 200, JSON.stringify([await pg.evaluate(SIDEWAYS), inside]));
    await pg.locator('.udtab').nth(1).click();
    const pills = await pg.evaluate(PILLS);
    check('career phone: the pill tracks (Game data, rank, her card\'s tabs) are filled flush by their segments', !pills.length && await pg.locator('.udtabs').count() === 1, pills.slice(0, 4).join(' | '));
    const pick = await pg.evaluate(() => { const tr = document.querySelector('.alts .optab tbody tr.on'); if (!tr) return null; const th = tr.querySelector('th');
      return { bg: getComputedStyle(tr).backgroundColor, cell: getComputedStyle(th).backgroundColor, wide: th.getBoundingClientRect().width >= tr.clientWidth - parseFloat(getComputedStyle(tr).paddingLeft) - parseFloat(getComputedStyle(tr).paddingRight) - 4 }; });   // the card's content width (it has inner padding)
    check('career phone: Career choices shades the planner\'s pick as one card, its career the card\'s full width', pick && pick.bg !== 'rgba(0, 0, 0, 0)' && pick.cell === 'rgba(0, 0, 0, 0)' && pick.wide, JSON.stringify(pick));
    const side = [];
    const shut = () => pg.evaluate(() => document.querySelectorAll('dialog[open]').forEach(d => d.close()));
    for (const [open, label] of [[() => pg.click('#v-toggle'), 'veterans'], [() => pg.click('#o-umas'), 'umas'], [() => pg.click('#o-cards'), 'cards'],
      [() => pg.locator('#tree [data-k]').first().click(), 'node'], [() => pg.locator('.udodds').first().click(), 'odds']]) {
      await open(); await pg.waitForSelector('dialog[open]'); await still(pg);
      side.push(...(await pg.evaluate(SIDEWAYS)).map(x => label + ': ' + x));
      if (label === 'veterans') side.push(...(await pg.evaluate(PILLS)).map(x => label + ': ' + x));
      await shut(); }
    check('career phone: nothing scrolls sideways in your veterans (their Yours / Borrowed pills flush), Umas, Support cards, the node popup or the odds', !side.length, side.slice(0, 4).join(' | '));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  async nonecards(b) {
    // "None owned" on support cards: no owned card reaches the solver (an empty owned list used to mean "all owned")
    const pg = await open(b); const all = await odds(pg);
    await pg.click('#c-none'); await settle(pg);
    check('None owned: the count at the top says "0 of N owned"', /^0 of \d+ owned$/.test(await pg.locator('#c-count').innerText()), await pg.locator('#c-count').innerText());
    const job = await pg.evaluate(() => window.__runs.at(-1));
    check('None owned: the job allows no owned card, only the one borrowed, and the odds drop', job.owned_cards.length === 1 && job.owned_cards[0] === 0 && job.borrow === true && (await odds(pg)) !== all, (await odds(pg)) + ' vs ' + all);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async reload(b) {
    // a reload with non-default settings shows their last result from the first frame (not the default example), and
    // the fresh identical result doesn't redraw it
    const pg = await open(b), ctx = pg.context(), def = await odds(pg);
    await pg.click('#rank button[data-v="0.20,0.70,0.10"]'); await settle(pg);   // rank SS: a different result, quick to search
    const mine = await odds(pg);
    await ctx.addInitScript(() => { window.__seen = []; const snap = () => { const o = document.querySelector('#results .loop .odds'); window.__seen.push(o ? o.textContent : '');
      if (o && !o.__mark) o.__mark = 1; if (performance.now() < 6000) requestAnimationFrame(snap); }; requestAnimationFrame(snap); });
    await pg.reload(); await settle(pg);
    // (the bundle is a cached script that may run after the first paint: a blank results area before it is allowed)
    const seen = [...new Set(await pg.evaluate(() => window.__seen))].filter(x => x !== '');
    check('reload: the first result drawn is the saved settings\' one, never the default example', seen.length === 1 && seen[0] === mine && mine !== '' && mine !== def, seen.join(' | ') + ' · default ' + def);
    check('reload: an identical fresh result is not redrawn', await pg.evaluate(() => document.querySelector('#results .loop .odds').__mark === 1));
    // the saved result of these settings counts as done: no search runs, the status is its own "Checked …" line
    await pg.waitForTimeout(1500);
    check('reload: the saved result is done, no search runs', await pg.evaluate(() => !window.__runs.some(j => j.cmd === 'deck')) && /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await pg.locator('#run-note').innerText()),
      await pg.locator('#run-note').innerText());
    await pg.click('#rank button[data-v="0.50,0.45,0.05"]'); await settle(pg);
    check('reload: … and the next edit searches', await pg.evaluate(() => window.__runs.some(j => j.cmd === 'deck')) && (await odds(pg)) === def);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  async delivery(b) {
    // self-hosted fonts (no third-party requests), a real favicon, the solver's files fingerprinted, the first roster
    // portraits eager, and accessible names that start with the visible text (voice control)
    const hosts = new Set(); const urls = [];
    const pg = await open(b, { before: (p) => p.on('request', r => { const u = new globalThis.URL(r.url()); if (u.protocol.startsWith('http')) { hosts.add(u.host); urls.push(u.pathname); } }) });
    check('no third-party requests (fonts self-hosted)', hosts.size === 1, [...hosts].join(' '));
    check('the solver data and binary come from fingerprinted files', urls.some(u => /^\/assets\/engine-global\.[0-9a-f]{10}\.json$/.test(u)) && urls.some(u => /^\/assets\/loopopt\.[0-9a-f]{10}\.wasm$/.test(u)), urls.filter(u => u.startsWith('/assets/') && !u.includes('/img.')).join(' '));
    check('page fonts are the self-hosted subsets', await pg.evaluate(async () => { await document.fonts.ready; return [...document.fonts].some(f => f.family.includes('Zen Kaku') && f.status === 'loaded'); }));
    const ico = await pg.request.get(new globalThis.URL('favicon.ico', URL).href);
    check('favicon.ico is an icon, not the page', ico.ok() && /image\//.test(ico.headers()['content-type'] || ''), ico.headers()['content-type']);
    check('the first roster portraits load eagerly', await pg.evaluate(() => { const im = [...document.querySelectorAll('#roster .av img')]; return im.length > 8 && im.slice(0, 8).every(i => i.loading === 'eager') && im.slice(8).every(i => i.loading === 'lazy'); }));
    check('sort buttons are named by their visible text first', await pg.evaluate(() => ['rs', 'cs'].every(id => { const e = document.getElementById(id); return e.getAttribute('aria-label').startsWith(e.textContent.trim()); })));
    check('card tiles take their name from their visible text', await pg.evaluate(() => [...document.querySelectorAll('#cards .ct')].every(e => !e.hasAttribute('aria-label') && e.hasAttribute('aria-pressed'))));
    await pg.context().close();
  },
  async gm(b) {
    // the era buttons: switching to the Grand Masters-era data (fetched, the status line says so meanwhile) opens on
    // Grand Masters (rank SS, no Unity golds) and searches with it; back on Global the scenario is Unity Cup again
    const pg = await open(b);
    await pg.route('**/assets/data-gm.*', async (r) => { await new Promise(x => setTimeout(x, 1500)); await r.continue(); });   // a slow fetch of the GM era's data
    await navClick(pg, '#era-gm');
    check('GM era: while its data loads, the status line says so', await until(pg, () => /Loading the JP Grand Masters-era data/.test(document.getElementById('run-note').textContent)), await note(pg));
    await settle(pg); await pg.unroute('**/assets/data-gm.*');
    check('switching to the GM era selects Grand Masters, at rank SS, and searches with it', (await scen(pg)) === 'grandmasters' && (await pg.getAttribute('#x-scen', 'aria-label')) === 'Scenario: Grand Masters'
      && (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10' && /^Checked [\d,]+ decks \/ [\d,]+ loops in [\d.]+ s/.test(await note(pg)) && (await pg.locator('#results .loop').count()) > 0
      && await pg.evaluate(() => window.__runs.at(-1).scenario === 'grandmasters' && window.__runs.at(-1).star_dist.join() === '0.2,0.7,0.1'), await odds(pg));
    check('Grand Masters gives no guaranteed golds', (await pg.getAttribute('#g-20049', 'data-g')) === '0' && (await pg.getAttribute('#g-20046', 'data-g')) === '0');
    const umas = (await pg.evaluate(() => fetch(JSON.parse(document.getElementById('data-assets').textContent).dataGm).then(r => r.json()))).umas; const nOut = umas.reduce((a, u) => a + Math.max(1, u.outfits.length), 0);
    check('GM era: roster from the GM data', (await pg.locator('#roster .ut').count()) === nOut, nOut + ' outfits');
    // the GM era offers Grand Masters as a fifth tile (same size as the others)
    await pg.click('#x-scen');
    const gt = await pg.evaluate(() => [...document.querySelectorAll('#scen-menu .stile')].map(t => { const r = t.getBoundingClientRect(); return { v: t.dataset.v, sub: t.querySelector('.ssub').innerText.replace(/\s+/g, ' ').trim(), size: Math.round(r.width) + 'x' + Math.round(r.height), x: Math.round(r.left + r.width / 2) }; }));
    const mid = await pg.evaluate(() => { const r = document.getElementById('scen-menu').getBoundingClientRect(); return Math.round(r.left + r.width / 2); });
    check('GM era: five equal tiles, Grand Masters last and centred', gt.length === 5 && gt[4].v === 'grandmasters' && gt[4].sub.replace(/ \((estimate|est\.)\)$/, '') === hintRates().grandmasters && / \((estimate|est\.)\)$/.test(gt[4].sub) && new Set(gt.map(t => t.size)).size === 1 && Math.abs(gt[4].x - mid) <= 1, JSON.stringify(gt));
    if (process.env.LLP_SHOTS) await pg.locator('#scen-menu').screenshot({ path: `${process.env.LLP_SHOTS}/${b.browserType().name()}-scen-picker-gm.png` });
    await pg.keyboard.press('Escape');
    await navClick(pg, '#era-global'); await settle(pg);
    check('back on Global: Unity Cup with its golds, rank below SS', (await scen(pg)) === 'unity' && (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05'
      && (await pg.getAttribute('#g-20049', 'data-g')) === '2'
      // (Global's result from before the switch is saved: it shows as done, no search; else the search is Unity Cup's)
      && await pg.evaluate(() => window.__runs.length === 0 ? /^Checked [\d,]+ decks/.test(document.getElementById('run-note').textContent) : window.__runs.at(-1).scenario === 'unity'));
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async settings(b) {
    // saved settings load: roster and cards per data set, the rest shared
    const pg = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      localStorage.setItem('llp:settings', JSON.stringify({ v: 2, era: 'global', common: { rank: '0.20,0.70,0.10' }, global: { roster: { 1001: 'out' }, cardsOff: { 30052: false } }, gm: { roster: { 1002: 'out' } } })); } catch {} } });
    check('saved settings load: the uma marked not owned shows not owned', await pg.evaluate(() => [...document.querySelectorAll('#roster .ut[data-uma="1001"]')].every(t => t.dataset.s === 'out')));
    check('… card ownership and rank too', await pg.evaluate(() => document.querySelectorAll('#cards .ct[aria-pressed="false"]').length === 1) && (await pg.getAttribute('#rank', 'data-value')) === '0.20,0.70,0.10');
    // changes persist across a reload
    await pg.locator('#roster .ut .up').nth(3).click(); await settle(pg); await pg.reload(); await settle(pg);
    check('a change survives a reload', (await pg.locator('#roster .ut[data-s="out"]').count()) >= 1 + (await pg.locator('#roster .ut[data-uma="1001"]').count()));
    // export, then import the same text
    await pg.click('#ex-open'); const text = await pg.inputValue('#ex-text');
    await pg.fill('#ex-text', text); await pg.click('#ex-load');
    check('export then import: settings load', (await pg.locator('#ex-msg').innerText()) === 'Settings loaded.' && JSON.parse(text).settings.era === 'global');
    // a setting of the wrong shape (a hand edit) is left out and named; the rest loads, and the page still loads after it
    // (the wrong value used to be saved first, and every later load failed on it)
    const bad = JSON.parse(text); bad.settings.targets = 5; bad.settings.rank = '0.50,0.45,0.05';
    await pg.fill('#ex-text', JSON.stringify(bad)); await pg.click('#ex-load');
    check('import: a malformed setting is named and left as it was, the others load', /Not understood, so left as they were: targets\.$/.test(await pg.locator('#ex-msg').innerText())
      && (await pg.locator('#targets .tgt').count()) === 8 && (await pg.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05', await pg.locator('#ex-msg').innerText());
    await pg.click('#ex-close'); await settle(pg); await pg.reload(); await settle(pg);
    check('… and the page loads after it', (await pg.locator('#targets .tgt').count()) === 8 && (await pg.locator('#results .loop').count()) > 0);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
    const p3 = await open(b, { init: () => { try { if (sessionStorage.getItem('seeded')) return; sessionStorage.setItem('seeded', '1'); localStorage.clear();
      localStorage.setItem('llp:settings', JSON.stringify({ v: 2, era: 'global', common: { scen: 'grandmasters', rank: '0.20,0.70,0.10', targets: [{ group: 20049 }, { group: 20046 }, { group: 20159 }] }, global: {}, gm: {} })); } catch {} } });
    check('Grand Masters saved, Global data active: Unity Cup is used (its golds and rank)', (await p3.getAttribute('#x-scen', 'data-value')) === 'unity'
      && await p3.evaluate(() => window.__runs.at(-1).scenario === 'unity') && (await p3.getAttribute('#rank', 'data-value')) === '0.50,0.45,0.05' && (await p3.getAttribute('#g-20049', 'data-g')) === '2');
    await p3.context().close();
  },
  async about(b) {
    // "How this works": a link in the header to its own page (/about), which links back
    const pg = await open(b);
    // the header is the same on every page: the three pages in one order, the page you are on marked
    const tabs = () => pg.evaluate(() => [...document.querySelectorAll('header.top .ptabs a')].map(a => a.textContent + (a.getAttribute('aria-current') === 'page' ? '*' : '')).join(','));
    const t0 = await tabs();
    await pg.locator('header.top .tnav a', { hasText: 'How this works' }).click(); await pg.waitForLoadState();
    check('header: the same page tabs everywhere, the current page marked', t0 === 'Loop planner*,Career planner,How this works' && await tabs() === 'Loop planner,Career planner,How this works*', `${t0} | ${await tabs()}`);
    check('header link opens /about', new globalThis.URL(pg.url()).pathname === '/about' && /How this works/i.test(await pg.locator('main.about h2').innerText()));
    const spec = pg.locator('main.about details.spec').first(), cspec = pg.locator('main.about details.spec').nth(1);
    check('/about: the detailed model starts folded and opens to the model text; the version shows', !(await spec.getAttribute('open') !== null) && !/Model\./.test(await pg.locator('main.about').innerText())
      && await (async () => { await spec.locator('summary').click(); return /Model\./.test(await spec.innerText()); })() && /Version v?\S+/.test(await pg.locator('main.about').innerText()));
    check('/about: the career planner in detail starts folded and opens to its steps', (await cspec.getAttribute('open')) === null
      && await (async () => { await cspec.locator('summary').click(); const t = await cspec.innerText(); return /C1\. Legal careers/.test(t) && /C8b\. Which careers are searched exactly/.test(t) && /The Comparison panel/.test(t); })());
    if (b.browserType().name() === 'chromium') { const f = artless(await pg.evaluate(AUDIT)); check('/about: every text run meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    await pg.locator('header.top .tnav a', { hasText: 'Loop planner' }).click(); await pg.waitForSelector('#targets');
    check('/about links back to the planner', new globalThis.URL(pg.url()).pathname === '/');
    await pg.context().close();
  },
  async dark(b) {
    // dark mode: the system's preference until the header button is used, then the visitor's choice, kept in this browser
    const pg = await open(b, { scheme: 'dark' });
    const theme = () => pg.evaluate(() => document.documentElement.getAttribute('data-theme'));
    check('dark: with no saved choice the page follows a dark system preference', (await theme()) === 'dark'
      && await pg.evaluate(() => getComputedStyle(document.body).backgroundColor !== 'rgb(243, 245, 248)' && document.getElementById('theme-btn').getAttribute('aria-pressed') === 'true'));
    await pg.emulateMedia({ colorScheme: 'light' });
    await pg.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'light', null, { timeout: 3000 }).catch(() => {});   // the change event arrives asynchronously
    check('dark: … and follows the system when it changes', (await theme()) === 'light');
    await pg.emulateMedia({ colorScheme: 'dark' });
    await pg.waitForFunction(() => document.documentElement.getAttribute('data-theme') === 'dark', null, { timeout: 3000 }).catch(() => {});
    if (b.browserType().name() === 'chromium') {
      await still(pg);   // the results' fade back from the searching dim
      const fails = [...await pg.evaluate(AUDIT)];
      await pg.click('#x-scen'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
      await pg.click('#rank-q'); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /rtab|rfoot|gpop/.test(x))); await pg.keyboard.press('Escape');
      await pg.locator('.deck .dc').first().click(); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /cpop|sw|gtl|ctt/.test(x))); await pg.keyboard.press('Escape');
      await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg'); fails.push(...await pg.evaluate(AUDIT));
      await pg.locator('#sched-dlg .stab').last().click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
      await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
      { const shown = artless(fails); check('dark: every text run meets WCAG AAA (page, scenario picker, rank popup, card popup, schedule, affinity)', !shown.length, shown.slice(0, 5).join(' | ')); }
    }
    await pg.click('#theme-btn');
    check('dark: the header button switches to light and remembers it', (await theme()) === 'light' && await pg.evaluate(() => localStorage.getItem('llp:theme') === 'light'));
    await pg.reload(); await settle(pg);
    check('dark: … the choice outlives a reload and beats the system preference', (await theme()) === 'light');
    await pg.emulateMedia({ colorScheme: 'light' }); await pg.emulateMedia({ colorScheme: 'dark' }); await pg.waitForTimeout(300);
    check('dark: … and a system change no longer overrides it', (await theme()) === 'light');
    // the status box repaints with the page, not after it (its colour transitions wait out the switch)
    const box = () => pg.evaluate(() => getComputedStyle(document.querySelector('.rhead')).backgroundColor);
    const bLight = await box(); await pg.click('#theme-btn'); const bNow = await box(); await pg.waitForTimeout(900); const bLater = await box();
    check('dark: the button switches back to dark', (await theme()) === 'dark');
    check('dark: … the status box takes its dark colour at once', bNow === bLater && bNow !== bLight, `${bLight} -> ${bNow} -> ${bLater}`);
    await pg.goto(URL.replace(/\/?$/, '/') + 'about'); await pg.waitForLoadState('load');
    check('dark: /about takes the same choice', (await theme()) === 'dark');
    if (b.browserType().name() === 'chromium') { const f = await pg.evaluate(AUDIT); check('dark: /about meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await pg.context().close();
  },
  async career(b) {
    // the career planner (/career): the lineage tree, the parent picker, the skills prompt, the settings dialogs,
    // umadump import, the veteran editor, the search and its results
    const { ctx, pg, note, done, planned, oneAnswer, showsNow, vetsOpen, vetsShut, vetsSeen, node, inNode, closeNode, simStars, importDump } = await careerPage(b);
    check('career: with no veterans it asks for parents or an import, and offers sample veterans', /Choose both parents, or import/.test(await note()) && /Try sample veterans/.test(await pg.textContent('#run-act')), await note());
    check('career: the setup panel shows the shared settings, the Enhance buttons, and Target skills in the open (the planner\'s editor)', await pg.locator('.rbbar #o-umas.enh .enhimg, .rbbar #o-cards.enh .enhimg').count() === 2
      && await pg.locator('.rbbar .lptgt #targets .tgt').count() === 8 && !(await pg.locator('#o-skills, #set-skills').count()));
    // the presets popup inside that panel lays out as on the planner: one column, its tiles the dialog's width, the buttons under them
    await pg.click('#t-preset'); await pg.locator('#pr-list .pview').first().click();
    const pv = await pg.evaluate(() => { const r = (s) => document.querySelector(s).getBoundingClientRect(), d = r('#pr-view .dlg'), t = r('#prv-tiles'), f = r('#pr-view .prfoot');
      return { cols: getComputedStyle(document.querySelector('#pr-view .dlg')).gridTemplateColumns.split(' ').length, wide: t.width > d.width - 60, below: f.top >= t.bottom }; });
    check('career: the preset popup is laid out as on the planner', pv.cols === 1 && pv.wide && pv.below, JSON.stringify(pv));
    await pg.locator('#pr-view .dclose').click(); await pg.locator('#preset-dlg .dclose').click();
    // the pink ribbon toggles it, its edge too (not the bare strip beside it); folded, the summary (the whole bar) opens it
    const hb = await pg.locator('#h-set').boundingBox(), bb = await pg.locator('.rbbar').boundingBox();
    await pg.locator('.rbbar').click({ position: { x: hb.x - bb.x + hb.width + 60, y: hb.y - bb.y + hb.height / 2 } });
    const stripStays = !(await pg.locator('.rbbar .setlist').isHidden());
    await pg.mouse.click(hb.x + 6, hb.y + 4);
    const foldSum = await pg.getAttribute('#set-sum', 'aria-label'), foldHid = await pg.locator('.rbbar .setlist').isHidden();
    await pg.click('#set-sum');
    const sumOpen = !(await pg.locator('.rbbar .setlist').isHidden());
    await pg.click('#set-fold'); const ribFold = await pg.locator('.rbbar .setlist').isHidden(); await pg.click('#set-fold');
    check('career: the Career setup ribbon (its edge too, not the strip beside it; folded, the summary) folds the panel to a one-line summary and back',
      stripStays && foldHid && sumOpen && ribFold && /^Global · .+ · rank (B–S\+|SS) · 8 target skills$/.test(foldSum)
      && !(await pg.locator('.rbbar .setlist').isHidden()) && (await pg.getAttribute('#set-fold', 'aria-expanded')) === 'true', foldSum);
    await pg.evaluate(() => { window.__seq = []; });
    await importDump();
    await vetsSeen();   // the veterans dialog draws its cards when open (and on opening)
    check('career: the umadump file imports 12 veterans', /Imported 12 veterans/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 12);
    // your veterans: the game's Veteran Umamusume button in the setup bar opens them in a dialog
    const vbtn = (await pg.getAttribute('#v-toggle', 'aria-label')) === 'Your veterans' && /^Veteran Umamusume$/.test((await pg.textContent('#v-toggle')).trim());
    await pg.click('#v-toggle'); await dialog(pg, 'set-vets');
    check('career: the Veteran Umamusume button opens your veterans with their import and export', vbtn && await pg.locator('#set-vets #vets .vet').count() === 12
      && await pg.locator('#set-vets #v-paste, #set-vets #v-export, #set-vets #v-clear, #set-vets #v-file').count() === 4);
    await done();
    const firstRun = await oneAnswer('Any uma/'), firstRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    check('career: one answer per run: while it searches, then plans, no career is drawn; the status names each phase; the answer is the plan\'s pick with the veteran drawn once',
      firstRun.ok && firstRun.phases.join() === 'search,plan' && firstRes && (await pg.textContent('#tree .pn.trainee .nm')) === firstRes.next.choice.trainee.name, firstRun.detail);
    if (QUICK) { check('no page errors', !pg.errors.length, pg.errors.join(' | ')); return ctx.close(); }   // the fast tier: the one-run answer
    // Export: a dialog with the veterans and the solver job as text; Copy puts it on the clipboard, Download saves it, Paste takes it back
    await vetsOpen(); await pg.click('#v-export'); await dialog(pg, 'vx-dlg');
    const exText = await pg.inputValue('#vx-text'), ex = JSON.parse(exText);
    const chromium = b.browserType().name() === 'chromium';
    if (chromium) await ctx.grantPermissions(['clipboard-read', 'clipboard-write']);
    await pg.click('#vx-copy'); await pg.waitForFunction(() => document.getElementById('vx-msg').textContent);
    const copied = (await pg.textContent('#vx-msg')) === 'Copied.' && (!chromium || await pg.evaluate(() => navigator.clipboard.readText()) === exText);
    const [dl] = await Promise.all([pg.waitForEvent('download'), pg.click('#vx-dl')]);
    const dlSame = (await import('fs')).readFileSync(await dl.path(), 'utf8') === exText;
    await pg.locator('#vx-dlg .dclose').click();
    // umadump's borrow list (rental_chara_data.json: the lender's viewer id as owner_viewer_id, trained_chara_ids that
    // can repeat yours) goes to the Borrowed tab beside your own veterans; a later borrow list replaces it; your own file
    // merges (a partial one removes nothing); Remove all takes only the open tab's
    const dump = JSON.parse(fs.readFileSync(new globalThis.URL('../val/umadump/U_trained_chara_data.json', import.meta.url), 'utf8'));
    const lend = (rows, id) => rows.map(r => ({ ...r, viewer_id: id, owner_viewer_id: id }));
    const paste = async (t) => { await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', t); await pg.click('#paste-load'); };
    const seen = async () => [await pg.textContent('#v-msg'), await pg.getAttribute('#v-tab-bor', 'aria-selected'), await pg.locator('#vets .vet').count(),
      (await pg.textContent('#v-tab-own')).trim(), (await pg.textContent('#v-tab-bor')).trim()].join(' | ');
    await paste(JSON.stringify(lend(dump.slice(0, 3), 900000000001))); const lent1 = await seen();
    await paste(JSON.stringify(lend(dump.slice(0, 2), 900000000002))); const lent2 = await seen();
    await paste(JSON.stringify(dump.slice(0, 5))); const part = await seen();
    check('career: a borrow list goes to the Borrowed tab and the next one replaces it; your partial file merges and removes nothing',
      lent1 === 'Replaced the borrow list with 3 borrowed veterans. | true | 3 | Yours · 12 | Borrowed · 3'
      && lent2 === 'Replaced the borrow list with 2 borrowed veterans (it had 3). | true | 2 | Yours · 12 | Borrowed · 2'
      && part === 'Imported 5 veterans of yours: 5 already here, updated. | false | 12 | Yours · 12 | Borrowed · 2', `${lent1} / ${lent2} / ${part}`);
    await pg.click('#v-tab-bor'); pg.once('dialog', d => d.accept()); await pg.click('#v-clear');
    const cleared = await seen(), emptyHint = await pg.textContent('#vets'); await pg.click('#v-tab-own');
    check('career: Remove all on the Borrowed tab removes only the borrowed, and the empty tab says how to fill it',
      /^Removed 2 borrowed veterans\. \| true \| 0 \| Yours · 12 \| Borrowed · 0$/.test(cleared) && /None borrowed yet/.test(emptyHint) && await pg.locator('#vets .vet').count() === 12, cleared);
    await vetsShut();
    await paste(exText);   // an Export replaces every import: the 12 again
    check('career: Export shows the veterans and the search job as text, Copy copies it, Download saves it, and Paste takes it back', ex.app === 'uma-legacy-veterans'
      && ex.veterans.length === 12 && ex.job && ex.job.cmd === 'career' && ex.job.legacies.length === 12 && copied && dlSame
      && /Imported 12 veterans from an export/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 12 && !(await pg.locator('#paste-dlg[open]').count()),
      `copy: ${await pg.textContent('#vx-msg')}, download same: ${dlSame}`);
    await done(); await planned();   // the pasted export's plan (the same pool)
    check('career: the planner weighs every career, checked on exact odds (only a check that ran out of time is mentioned)', /Weighed 3,618 careers/.test(await note()) && !/ran out of time|checked exact/.test(await note()), await note());
    check('career: the tree shows the planner\'s trainee and parents', await pg.locator('#tree .pn.ghost').count() === 3 && await pg.locator('#tree .pn.gp').count() === 4);
    const deckShut = !(await pg.locator('#tree-deck').count()) && (await pg.getAttribute('#deck-open', 'aria-expanded')) === 'false';
    await pg.click('#deck-open');
    // the odds in her card's header; a click opens every count (exactly k, k or better), all 8 and 7 or better as the solver's
    const oddsTxt = await pg.textContent('#odds-open');
    await pg.click('#odds-open'); await dialog(pg, 'odds-dlg');
    const oddRows = await pg.$$eval('#odds-dlg .oddtab tbody tr', l => l.map(r => [...r.children].map(c => c.textContent)));
    await pg.keyboard.press('Escape');
    check('career: the odds sit in the veteran card\'s header; a click lists the chance of landing exactly each count, in percent', /^1 in [\d,.]+ to land all 8/.test(oddsTxt)
      && oddRows.length === 9 && oddRows[0][0] === '8/8' && oddRows[1][0] === '7/8' && oddRows[8][0] === '0/8' && oddRows.every(r => r.length === 2 && /^[<\d.]+%$/.test(r[1]))
      && await pg.evaluate(() => document.activeElement?.id === 'odds-open'), JSON.stringify(oddRows.slice(0, 3)));
    check('career: the odds and eight target rows beside and under the tree; the six-card deck under the trainee, behind its button', /to land all 8/.test(oddsTxt)
      && deckShut && await pg.locator('#tree .pn.trainee #tree-deck .dc').count() === 6 && await pg.locator('.rtgt .cartab tbody tr').count() === 8);
    check('career: the six legacies carry their affinity as a heart and a number (no legacy numbers), grandparents labelled plainly', !(await pg.locator('#tree .gate').count())
      && await pg.locator('#tree .pn:not(.trainee) .aff').count() === 6 && await pg.locator('#tree .pn:not(.trainee) .aff .heart').count() === 6
      && (await pg.$$eval('#tree .pn .aff', l => l.every(x => /^Affinity \d+$/.test(x.getAttribute('aria-label')) && /^\d+$/.test(x.textContent))))
      && (await pg.$$eval('#tree .pn.gp .pnl', l => l.every(x => x.textContent === 'Grandparent'))));
    const affs = await pg.$$eval('#tree .pn .aff', l => l.map(x => +x.textContent.replace(/\D/g, '')));
    check('career: the trainee shows the affinity her six legacies hold (their sum), without a parents/grandparents line', !(await pg.locator('#tree .pn.trainee .tsum').count())
      && Math.abs(affs.slice(1).reduce((a, x) => a + x, 0) - affs[0]) <= 6, affs.join(','));
    await node('node:t');
    const affRows = await pg.$$eval('#node-dlg .afftab tbody tr', l => l.length), affTot = +(await pg.textContent('#node-dlg .afftab tfoot .aff')).replace(/\D/g, '');
    await closeNode();
    check('career: … her popup breaks it down legacy by legacy, summing to the same', affRows === 6 && Math.abs(affTot - affs[0]) <= 1, `${affRows} rows, ${affTot} vs ${affs[0]}`);
    // the planner's pick: the K best careers by odds, then the plan over them and a few more
    const lastJob = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career' && j.next && !j.compare).at(-1));
    const lastRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    check('career: with places open the planner picks the next career (the highest saving per run) after the six best by odds', lastJob && lastJob.top === 6 && lastJob.per_trainee === false && !lastJob.rollout
      && lastRes && lastRes.next.options.length >= 2 && /^index$/.test(lastRes.next.rule) && /Planned in/.test(await note()));
    check('career: the trainee names one outfit (never "any outfit")', !/any outfit/i.test(await pg.textContent('#tree')) && !/any outfit/i.test(await pg.textContent('#results')));
    check('career: … the career on show is its pick, with why under the odds',
      (await pg.textContent('#tree .pn.trainee .nm')) === lastRes.next.choice.trainee.name && await pg.locator('.alts .otag.pick').count() === 1, await pg.textContent('#tree .pn.trainee .nm'));
    // the deck: a card's panel, as in the planner; marking it not owned searches again without it
    const dc = pg.locator('#tree-deck .dc').first(), dcid = await dc.getAttribute('data-cid');
    await dc.click(); await pg.waitForSelector('.cpop:not([hidden]) [data-act="own"]');
    const preEdit = await showsNow(); await pg.evaluate(() => { window.__seq = []; });
    await pg.locator('.cpop [data-act="own"]').uncheck(); await done();
    const edit = await oneAnswer(preEdit);
    check('career: an edit keeps the old answer on screen, dimmed, through both phases (no interim career), then draws the new one', edit.ok && edit.phases.join() === 'search,plan', edit.detail);
    // an edit undone within the pause runs nothing and never dims the answer
    if (!(await pg.locator('.cpop:not([hidden]) [data-act="own"]').count())) await pg.locator('#tree-deck .dc').first().click();
    await pg.waitForSelector('.cpop:not([hidden]) [data-act="own"]');
    const pre3 = await showsNow(); await pg.evaluate(() => { window.__seq = []; });
    await pg.evaluate(() => { const c = document.querySelector('.cpop [data-act="own"]'); c.click(); c.click(); });
    await done();
    const seq3 = await pg.evaluate(() => window.__seq);
    check('career: an edit undone within the pause runs nothing and never dims the answer', seq3.length > 0 && !seq3.some(s => s.stale) && (await showsNow()) === pre3, JSON.stringify(seq3.map(s => [s.busy, s.stale])).slice(0, 120));
    check('career: a deck card\'s panel marks it not owned; the search drops it or borrows it', await pg.evaluate(id => JSON.parse(localStorage.getItem('llp:settings')).global.cardsOff[id] === false, dcid)
      && (!(await pg.locator(`#tree-deck .dc[data-cid="${dcid}"]`).count()) || await pg.locator(`#tree-deck .dc.bor[data-cid="${dcid}"]`).count() === 1));
    {   // with a card borrowed, every deck on the page (the tree's, Career Info's) lists it last: the sixth slot, as in the game
      const rows = await pg.$$eval('.dcards, .udcards', l => l.map(r => [...r.querySelectorAll(':scope > .dc')].map(e => e.classList.contains('bor'))).filter(k => k.length));
      const bad = rows.filter(k => k.some((b, i) => b && k.slice(i + 1).some(x => !x))).length, nb = rows.filter(k => k.includes(true)).length;
      check('career: decks list the borrowed card last', nb > 0 && bad === 0, `${rows.length} decks, ${nb} with a borrowed card, ${bad} out of order`);
    }
    await pg.click('#o-cards'); await dialog(pg, 'set-cards'); await pg.click('#c-all'); await pg.locator('#set-cards .dclose').click(); await done();
    // the schedule: the planner's window, one trainee, from a button beside the deck
    check('career: the race schedule is a button under the trainee, beside the deck\'s', await pg.locator('#tree .pn.trainee #sched-open').count() === 1
      && !(await pg.locator('#results h3', { hasText: 'Race schedule' }).count()));
    await pg.click('#sched-open'); await dialog(pg, 'sched-dlg');
    const nRun = +(await pg.getAttribute('#sched-open', 'title')).match(/She runs (\d+) G1s/)[1];
    check('career: her schedule opens in the planner\'s schedule window as the game\'s Scheduled Races: only the races she runs, the three years at once without scrolling, Junior\'s April to June grey, no plus (nothing to add there), the parent-won legend', nRun > 0
      && await pg.locator('#sched-dlg .ag-slot.on').count() === nRun && await pg.locator('#sched-dlg .ag-race').count() === nRun && !(await pg.locator('#sched-dlg .ag-race.wbtn').count())
      && /A parent won it/.test(await pg.textContent('#sched-dlg .agkey')) && await pg.locator('#sched-dlg .stab').count() === 1
      && await pg.locator('#sched-dlg .ag-year').count() === 3 && await pg.locator('#sched-dlg .ag-slot.pre').count() === 12 && !(await pg.locator('#sched-dlg .ag-plus, #sched-dlg .ag-add').count())
      && await pg.evaluate(() => { const d = document.querySelector('#sched-dlg .dlg'); return d.scrollHeight <= d.clientHeight + 1; }), `runs ${nRun}`);
    await pg.keyboard.press('Escape');
    // the options: per option this career's odds and its expected careers to land, the pick marked; no after-this-career section
    const opts = await pg.$$eval('.alts .optab tbody tr', ls => ls.map(l => [...l.children].map(c => c.textContent)));
    check('career: Options lists each option with "1 in X" for this career and its net saving per run, the plan to a fully looped parent above, the pick marked and shown above', opts.length >= 2
      && opts.every(c => /^1 in [\d,.]+$/.test(c[1]) && /^[+−]\d+\.\d$/.test(c[2])) && await pg.locator('#save-why').count() === 1 && await pg.locator('.alts tr.on', { hasText: 'shown above' }).count() === 1
      && await pg.locator('.alts .otag.pick').count() === 1 && !(await pg.locator('#ahead-sec').count()), opts.map(c => c.join(' / ')).join(' | '));
    // the trio: each step's line (her odds now and the lifts building pays for, or your 8/8 at no cost)
    await pg.click('#save-why'); await dialog(pg, 'save-dlg');
    const memTxt = await pg.textContent('#save-dlg .oloop').catch(() => ''); await pg.keyboard.press('Escape');
    check('career: the plan names its route and, per step, the odds now and what building pays for', !lastRes.next.members?.length
      || /(your \d+\/\d+ of .+ \(no careers\)|1 in [\d,]+)/.test(memTxt), memTxt || 'no trio');
    check('career: beside the tree, the veteran analysis with the odds in its header, then the target skills (no goal-step note, no prose; the deck is under the trainee)', await pg.evaluate(() => {
      const r = document.getElementById('results'), kids = [...r.children], at = (sel) => kids.findIndex(k => k.matches(sel)), sim = at('#sim-top');
      return document.getElementById('tree').nextElementSibling === r && at('#goal-note') === -1 && at('.deck') === -1 && at('.lhead') === -1
        && sim === 0 && !!kids[sim].querySelector('.ud .udtop #odds-open') && at('.rtgt') === sim + 1 && kids[sim].querySelector('#simulation > h3')?.textContent === 'Veteran analysis'
        && !!kids[sim].querySelector('#simulation #sim-compare'); }));
    // her card, as the game's Umamusume Details screen: portrait and outfit, aptitude grades, the tabs Skills / Inspiration
    // (her sparks beside her portrait, then her parents' under Legacy Origin) / Career Info (the G1 wins button); on a wide
    // screen and a phone, with no sideways scroll
    const simLayout = () => pg.evaluate(() => { const c = document.querySelector('#simulation .ud'); if (!c) return 'no card';
      const f = c.querySelector('.udtop .av'), tabs = [...c.querySelectorAll('[role="tab"]')], ins = c.querySelector('#udpane-insp'), who = ins.querySelector('.simwho'),
        s = who.querySelector('.simskills'), me = who.querySelector(':scope > .av'), fr = me.getBoundingClientRect(), sr = s.getBoundingClientRect();
      const apt = c.querySelector('.udapt');
      return f.getAttribute('role') === 'img' && /\S/.test(f.getAttribute('aria-label') || '') && Math.round(f.getBoundingClientRect().width) === 96
        && tabs.map(t => t.textContent).join('|') === 'Skills|Inspiration|Career Info' && tabs.filter(t => t.getAttribute('aria-selected') === 'true').length === 1
        && (!apt || (apt.querySelectorAll('[role="row"]').length === 3 && apt.querySelectorAll('[role="cell"]').length === 10))
        && fr.right <= sr.left && fr.top < sr.bottom && s.getAttribute('aria-label') === 'Her sparks' && ins.querySelectorAll('.udrow.par').length === 2
        && c.querySelector('#udpane-info').contains(document.getElementById('sim-wins')) && c.getBoundingClientRect().right <= innerWidth
        && document.documentElement.scrollWidth <= document.documentElement.clientWidth; });
    const wide = await simLayout(); await pg.setViewportSize({ width: 375, height: 812 }); await pg.waitForTimeout(200);
    const phone = await simLayout(); await pg.setViewportSize({ width: 1440, height: 1000 }); await pg.waitForTimeout(200);
    check('career: the veteran analysis is the game\'s details card: portrait (named for screen readers), aptitude grades, Skills / Inspiration / Career Info tabs, her sparks beside her portrait and her parents\' under Legacy Origin, also on a 375 px phone (iPhone 13 mini) with no sideways scroll', wide === true && phone === true, `wide ${wide}, phone ${phone}`);
    // Skills: her learned targets (a spark of one) and the scenario's golds, as skill tiles; the tabs switch by click and arrow keys
    await pg.click('#udtab-skills');
    // every target she could acquire, as the game's tiles (name only), acquired bright, others greyed; a spark means she
    // acquired it (at least as many acquired tiles as skill sparks); a tile opens the game-style skill popup
    const tiles = await pg.$$eval('#udpane-skills .udsk', l => l.map(t => [t.classList.contains('off'), t.getAttribute('aria-label') || '']));
    const sparkN = await pg.locator('#udpane-insp .simskills .vsk:not(.pink)').count();
    check('career: the Skills tab lists the target skills she could acquire as the game\'s tiles, acquired ones bright',
      !(await pg.locator('#udpane-skills').isHidden()) && await pg.locator('#udpane-insp').isHidden() && tiles.length > 0
      && tiles.every(t => /: (acquired|not acquired)/.test(t[1]) && t[0] === /: not acquired/.test(t[1])) && tiles.filter(t => !t[0]).length >= sparkN,
      JSON.stringify(tiles) + ' | sparks ' + sparkN);
    await pg.locator('#udpane-skills .udsk').first().click(); await dialog(pg, 'sk-dlg');
    const pop = await pg.textContent('#sk-dlg');
    await pg.keyboard.press('Escape');
    check('career: … a skill opens its details (acquired or not, where it comes from, how likely), and closing returns to the tile', /Skill Details/.test(pop)
      && /(Acquired|Not acquired)( \(gold\))? in this result/.test(pop) && /Where it comes from/.test(pop) && /Lands as a spark/.test(pop)
      && await pg.evaluate(() => document.activeElement?.matches('#udpane-skills .udsk')), pop.slice(0, 160));
    await pg.focus('#udtab-skills'); await pg.keyboard.press('ArrowRight');
    check('career: … an arrow key moves to the next tab', await pg.evaluate(() => document.activeElement.id === 'udtab-insp' && document.getElementById('udtab-insp').getAttribute('aria-selected') === 'true'));
    const draft = () => pg.$$eval('#simulation .simskills .vskb', l => l.map(b => b.getAttribute('aria-label')).join('|'));
    const policyJobs = () => pg.evaluate(() => window.__careerJobs.filter(j => j.compare).length);
    const d0 = await draft();
    check('career: the veteran you got starts as a random result (no default), and nothing is compared yet', d0.length > 0 && await policyJobs() === 0 && !(await pg.locator('#sim-verdict').count()));
    // her pink spark: her roll carries at most one (an aptitude she ends at A or S)
    const pinkChips = () => pg.$$eval('#simulation .simskills .vsk.pink .vskb', l => l.map(b => b.getAttribute('aria-label')));
    const pk0 = await pinkChips();
    check('career: her roll carries at most one pink spark', pk0.length <= 1 && pk0.every(x => / \(aptitude\), \d stars?/.test(x)), pk0.join(' / '));
    const original = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets);
    await simStars(3);
    check('career: editing her leaves your veterans untouched and runs nothing', await pg.evaluate(old => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old), original)
      && await policyJobs() === 0);
    await simStars(3);
    await pg.click('#sim-compare');
    await pg.waitForSelector('#sim-verdict', { timeout: 180000 });
    check('career: Compare says, exactly, what she changes for the next career and shows that career with her', /^(Saves [\d,]+ careers|No careers saved|Better odds|Parent of the next career|\d+\/\d+: step 1 done|Fully looped parent)/.test(await pg.textContent('#sim-verdict'))
      && /→/.test(await pg.textContent('#sim-result')) && !/Careers until one lands|Your best career today →/.test(await pg.textContent('#sim-result')));
    check('career: … the best career with her folded under "Best career with her": portraits (trainee, then her two parents, her marked) and its deck; its odds outside', await pg.locator('#sim-result details.nextdeck:not([open]) .nextc .cface').count() === 3
      && await pg.locator('#sim-result .nextc .cface .her').count() === 1 && await pg.locator('#sim-result details.nextdeck:not([open]) .deck .dc').count() === 6
      && await pg.locator('#sim-result > .simodds .tc').count() === 2
      && /^Best career with her$/.test(await pg.textContent('#sim-result details.nextdeck summary')));
    check('career: the comparison sends her fixed stars, race indices and her parents', await pg.evaluate(() => {
      const j = window.__careerJobs.filter(j => j.compare && !j.next).at(-1), v = j.legacies.at(-1);
      return j.compare === true && j.fix.length === 1 && j.fix[0] === j.legacies.length - 1 && v.sparks.some(([g, s]) => g === 20159 && s === 3)
        && v.wins.every(Number.isInteger) && v.parents.length === 2 && v.parents.every(p => p.chara > 0 && p.sparks.length > 0);
    }));
    if (chromium) {   // one engine is enough for colours
      for (const theme of ['light', 'dark']) {
        if (await pg.evaluate(() => document.documentElement.getAttribute('data-theme')) !== theme) await pg.click('#theme-btn');
        await still(pg);   // the theme's colour transitions
        const fails = await pg.evaluate(AUDIT);
        check(`career: ${theme} mode with the outcome table and a comparison meets WCAG AAA`, !fails.length, fails.slice(0, 5).join(' | '));
      }
      await pg.click('#theme-btn');
    }
    // a roll: a random result of this career, compared at once (stubbed random numbers: nothing lands, then everything at 3 stars)
    const roll = async (u) => { const before = await policyJobs(); await pg.evaluate(u => { const r = Math.random; Math.random = () => u; document.getElementById('sim-roll').click(); Math.random = r; }, u);
      await pg.waitForFunction(n => window.__careerJobs.filter(j => j.compare).length > n && document.querySelector('#sim-verdict'), before, { timeout: 180000 }); };
    await roll(0.999999999);
    const rolledAll = await pg.$$eval('#simulation .simskills .vsk:not(.pink) .vskb', l => l.map(b => b.getAttribute('aria-label')));
    check('career: "Reroll" draws from the career\'s odds and compares at once', rolledAll.length === 8 && rolledAll.every(x => /3 stars/.test(x))
      && await pg.evaluate(old => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old), original));
    await roll(0);
    check('career: … a low roll lands nothing', await pg.locator('#simulation .simskills .vsk:not(.pink)').count() === 0);
    // … and draws her pink: every legacy pink fires at both inspirations (+1 grade each), the first aptitude at A or S, 1 star
    const pkLow = await pinkChips();
    // … her blue spark: one stat, stars by the rank band; its bar opens the picker (the five stats, the stars)
    const blueLbl = await pg.getAttribute('#udpane-insp .simskills .vsb.blue:not(.none) .vskb', 'aria-label').catch(() => '');
    await pg.click('#udpane-insp .simskills .vsb.blue .vskb'); await dialog(pg, 'col-dlg');
    const blueOpts = await pg.$$eval('#col-dlg .colgrid .vskb', l => l.map(b => b.textContent));
    await pg.locator('#col-dlg .spstar').nth(2).click();
    const blue3 = await pg.getAttribute('#udpane-insp .simskills .vsb.blue .vskb', 'aria-label').catch(() => '');
    await pg.keyboard.press('Escape');
    check('career: … a roll draws her blue spark; its bar opens a picker of the five stats and the stars', /^(Speed|Stamina|Power|Guts|Wit) \(stat\), [123] stars?/.test(blueLbl)
      && blueOpts.join() === 'Speed,Stamina,Power,Guts,Wit' && /, 3 stars/.test(blue3), `${blueLbl} | ${blueOpts.join()} | ${blue3}`);
    check('career: … a roll draws at most one pink spark for her (stars from 20/70/10: a low draw is 1 star)', pkLow.length <= 1 && pkLow.every(x => /, 1 star\./.test(x)), pkLow.join(' / '));
    // rolls while a comparison runs: each cancels the one in flight (its worker goes), and only the last roll's runs and shows
    const policyResults = () => pg.evaluate(() => window.__careerResults.filter(r => r.compare).length);
    // a comparison is one job for the best career with her (no trainee set) plus, when she doesn't raise the best odds,
    // the planner's candidates (a `next` job when no plan ran) and one job per candidate
    const mainJobs = () => pg.evaluate(() => window.__careerJobs.filter(j => j.compare && !j.trainees.length && !j.next).length);
    const [rj0, rr0, rm0] = [await policyJobs(), await policyResults(), await mainJobs()];
    const usable = await pg.evaluate(() => { const r = Math.random, ok = [];
      for (const u of [0.999999999, 0.999999999, 0]) { const b = document.getElementById('sim-roll'); ok.push(!b.disabled); Math.random = () => u; b.click(); Math.random = r; }
      return ok.every(Boolean) && !!document.querySelector('#sim-result .spin, #sim-result .runner'); });
    await pg.waitForFunction(() => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin, #sim-result .runner'), null, { timeout: 180000 });
    await pg.waitForTimeout(1500);   // time for a stray comparison to answer, were one still running
    const [rj, rr, rm] = [await policyJobs() - rj0, await policyResults() - rr0, await mainJobs() - rm0];
    // an exact comparison in flight is killed (one runs); a quick one (the roll read from the plan's table) runs out with its
    // answer ignored, so the worker keeps its caches (one per roll): either way only the last roll's shows
    check('career: rolling again while a comparison runs supersedes it: only the last roll\'s comparison shows', usable && (rm === 1 || rm === 3) && rr === rj
      && await pg.locator('#simulation .simskills .vsk:not(.pink)').count() === 0 && await pg.evaluate(() => window.__careerJobs.filter(j => j.compare && !j.next).at(-1).legacies.at(-1).sparks.length === 0),
      `usable ${usable}, comparisons ${rm}, jobs ${rj}, results ${rr}`);
    check('career: there is no "back to the most likely result" button; Compare, Roll and Add sit in her card\'s header', !(await pg.locator('#sim-reset').count())
      && await pg.locator('#simulation .ud .udtop #sim-compare, #simulation .ud .udtop #sim-roll, #simulation .ud .udtop #sim-add').count() === 3);
    // an n/n veteran: Compare says step 1 of the goal is done (no odds verdict)
    for (let i = 0; i < 12; i++) {
      await pg.locator('#simulation .vskadd').click();
      const b = pg.locator('#add-pop .addsk').first();
      if (!(await b.count())) { await pg.locator('#simulation .vskadd').click(); break; }
      await b.click();
    }
    await pg.click('#sim-compare');
    await pg.waitForFunction(() => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin, #sim-result .runner'), null, { timeout: 180000 });
    check('career: an n/n veteran: Compare says step 1 of the goal is done (or, with n/n parents, a fully looped parent)', /^((\d+)\/\2: step 1 done|Fully looped parent)/.test(await pg.textContent('#sim-verdict')), await pg.textContent('#sim-verdict'));
    await roll(0);   // (no reset: a roll that lands nothing leaves her short of n/n)
    await simStars(3);
    await pg.click('#sim-compare'); await pg.waitForSelector('#sim-verdict', { timeout: 180000 });
    const nPolicy = await policyJobs();
    await simStars(1);
    check('career: an edit after Compare marks the comparison out of date without running another', /changed her since/.test(await pg.textContent('#sim-result')) && await policyJobs() === nPolicy);
    const baselineRuns = () => pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career' && !j.trainees.length && !j.fix.length && j.top === 1 && !j.next).length);
    const cachedRuns = await baselineRuns();
    await node('node:t'); await inNode(/^(Choose|Change) trainee$/).click(); await dialog(pg, 'pick-dlg');
    await pg.evaluate(() => { window.__seq = []; });
    await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick:visible').first().click(); await done();
    await pg.waitForSelector('#sim-compare:not([disabled])', { timeout: 180000 });
    const fixedRun = await oneAnswer('Agnes Digital/');
    check('career: with the trainee chosen the run is the search alone (no planning phase), its best career the one answer', fixedRun.ok && fixedRun.phases.join() === 'search', fixedRun.detail);
    // with the trainee chosen there is nothing for the plan to pick: Compare pressed twice restarts the comparison,
    // which finishes, and no plan job is sent
    const aheadJobs = () => pg.evaluate(() => window.__careerJobs.filter(j => j.next && !j.compare).length), la0 = await aheadJobs();
    await pg.evaluate(() => { document.getElementById('sim-compare').click(); document.getElementById('sim-compare').click(); });
    await pg.waitForFunction(() => document.querySelector('#sim-verdict') && !document.querySelector('#run-note .spin, #run-note .runner'), null, { timeout: 180000 });
    check('career: with the trainee chosen no plan runs, and Compare pressed twice finishes', await aheadJobs() === la0 && !/Planning|Planned in/.test(await note())
      && /^(Saves [\d,]+ careers|No careers saved|Better odds|Parent of the next career|\d+\/\d+: step 1 done|Fully looped parent)/.test(await pg.textContent('#sim-verdict')), await note());
    check('career: with the trainee fixed, Compare reuses today\'s best instead of searching it again', await baselineRuns() === cachedRuns);
    // her G1 wins: the G1 wins editor (each win in its slot; a click removes it, an empty slot's menu adds one); an edit
    // after Compare marks it out of date, nothing is saved to the pool, and the focus goes back to the wins button
    const winsN = async () => { const m = (await pg.textContent('#sim-wins')).match(/^(No|\d+) G1 win/); return m[1] === 'No' ? 0 : +m[1]; };
    const pj0 = await policyJobs(), w0 = await winsN();
    await pg.click('#udtab-info');
    const rec = await pg.$$eval('#udpane-info .udrace', l => l.map(r => r.classList.contains('won')));
    const recLine = await pg.textContent('#udpane-info .udrec'), racesShut = await pg.locator('#ud-races').isHidden();
    await pg.click('#udpane-info .udmore');
    check('career: the Career Info tab shows her support cards, her two legacies with their parents, and her record (the races behind a button) matching the wins button',
      await pg.locator('#udpane-info .udcard').count() >= 1 && await pg.locator('#udpane-info .udleg').count() === 2 && await pg.locator('#udpane-info .udleggx').count() === 4
      && rec.length > 0 && rec.filter(Boolean).length === w0 && new RegExp(`Wins: ${w0}\\b`).test(recLine) && racesShut && !(await pg.locator('#ud-races').isHidden()),
      `${rec.length} races, ${rec.filter(Boolean).length} won, button ${w0}, ${recLine}`);
    // a borrowed card there wears the friend frame within its own card (`all: unset` on these cards once dropped the
    // container the frame's 100cqw is measured on, and the frame grew to the page's width)
    if (ART) {
      const fr = await pg.evaluate(() => {
        const row = document.querySelector('#udpane-info .udcards'), c = row.querySelector('.udcard').cloneNode(true);
        c.classList.add('bor'); c.appendChild(Object.assign(document.createElement('span'), { className: 'udfr', textContent: 'Friends' })); row.appendChild(c);
        const r = c.getBoundingClientRect(), a = getComputedStyle(c, '::after'), out = [parseFloat(a.width), parseFloat(a.height), r.width, r.height,
          getComputedStyle(c).boxShadow, getComputedStyle(c.querySelector('.udfr')).display];
        c.remove(); return out;
      });
      // and wears it alone: the old pink ring and Friends tag (for cards without art) once framed the frame
      check('career: in Career Info a borrowed card\'s friend frame stays on its card, without the ring and tag', fr[0] <= fr[2] + 10 && fr[1] <= fr[3] + 20
        && fr[4] === 'none' && fr[5] === 'none', `frame ${fr[0]}x${fr[1]}, card ${fr[2]}x${fr[3]}, ring ${fr[4]}, tag ${fr[5]}`);
    }
    await pg.click('#sim-wins'); await dialog(pg, 'wins-dlg');
    const wb0 = pg.locator('#wins-dlg .ag-race.wbtn').first(), wname = (await wb0.getAttribute('aria-label')).replace(/, won: remove$/, '');
    const nShown = await pg.locator('#wins-dlg .ag-race.wbtn').count();
    await wb0.click();
    const freed = await pg.evaluate(() => document.activeElement.matches('#wins-dlg .ag-add') && document.activeElement.getAttribute('aria-label'));
    check('career: the entered veteran\'s G1 wins open on the calendar, one per slot; clicking one removes it, its slot offers to add one, the count follows', nShown === w0
      && await pg.locator('#wins-dlg .agenda .ag-year').count() === 3 && /G1 wins/.test(await pg.textContent('#wins-title')) && /: add a G1 win$/.test(freed || '')
      && await winsN() === w0 - 1 && (await pg.textContent('#wins-count')).startsWith(String(w0 - 1 || 'No')), `wins ${w0} -> ${await winsN()}, ${freed}`);
    await pg.keyboard.press('Enter');
    const picks = await pg.$$eval('#wins-dlg .wmenu:not([hidden]) .wpick', l => l.map(e => e.textContent));
    await pg.locator('#wins-dlg .wpick', { hasText: wname }).click();
    check('career: … the empty slot\'s menu lists the G1s held then (her schedule\'s first, with its odds); a pick puts the win back in that slot', picks[0]?.startsWith(wname) && /her schedule · \d+% win/.test(picks[0])
      && await winsN() === w0 && await pg.evaluate(n => document.activeElement.getAttribute('aria-label') === n + ', won: remove', wname), picks.join(' | '));
    check('career: … which marks the comparison out of date without running another, and saves nothing to your veterans', /changed her since/.test(await pg.textContent('#sim-result'))
      && await policyJobs() === pj0 && await pg.evaluate(old => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets) === JSON.stringify(old), original));
    await pg.keyboard.press('Escape');
    check('career: … closing it puts the focus back on the wins button', await pg.waitForFunction(() => document.activeElement?.id === 'sim-wins', null, { timeout: 2000 }).then(() => true, () => false));
    await pg.click('#udtab-insp');
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  // the career planner's veteran pool, edited: a veteran's sparks and G1 wins, new parents and grandparents, the trainee,
  // Use this, the settings, reloads, the uma.moe tree, generations saved, removals (from the 12 test veterans)
  async careervets(b) {
    const { ctx, pg, note, done, ask, vetsOpen, vetsShut, vetsSeen, node, inNode, closeNode, importDump } = await careerPage(b);
    await importDump(); await done();
    // a pool veteran: her card opens her popup, her sparks and her parents' as bars, editable in place; editing keeps the
    // result on screen while the search runs again
    const toteBefore = await pg.locator('#odds-open').count();
    await vetsOpen(); await pg.click('#vet-0 .vtile'); await dialog(pg, 'node-dlg');
    const chip = pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb').first(), ckey = await chip.getAttribute('data-k');
    const stars = async () => +(await pg.locator(`#node-dlg [data-k="${ckey}"]`).getAttribute('aria-label')).match(/(\d) star/)[1];
    const s0 = await stars(); await chip.click();
    check('career: a veteran\'s card opens her sparks and her parents\' as bars; editing one keeps the result on screen until the new one is ready',
      await pg.locator('#node-dlg .udrow.par').count() === 2 && toteBefore === 1 && await pg.locator('#odds-open').count() === 1 && await pg.locator('#tree .pn.ghost').count() === 3);
    await done();
    const s1 = await stars(); await pg.locator(`#node-dlg [data-k="${ckey}"]`).click(); const s2 = await stars(); await pg.locator(`#node-dlg [data-k="${ckey}"]`).click(); const s3 = await stars();
    check('career: clicking a spark cycles its stars back round (1 -> 2 -> 3 -> 1), in place', s1 === s0 % 3 + 1 && s2 === s1 % 3 + 1 && s3 === s2 % 3 + 1, [s0, s1, s2, s3].join(' '));
    check('career: … the focus stays on it, and the change is saved', await pg.evaluate(k => document.activeElement?.dataset?.k === k, ckey) && await pg.evaluate(() => /"sparks"/.test(localStorage.getItem('llp:career'))));
    const nsk = () => pg.locator('#node-dlg .udrow.me .vsk:not(.pink)').count(), n0 = await nsk();
    const first = await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb').first().getAttribute('aria-label');
    await pg.locator('#node-dlg .udrow.me .vsk:not(.pink)').first().hover();
    const vis = await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskx').first().evaluate(e => getComputedStyle(e).visibility);
    await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskx').first().click();
    check('career: the x shows on hover and removes the skill', vis === 'visible' && await nsk() === n0 - 1);
    await pg.locator('#node-dlg .udrow.me .vskadd').click();
    const offered = await pg.$$eval('#add-pop .addsk', els => els.map(e => e.textContent));
    check('career: "+" offers the missing target only, as a spark cell (her blue, pink and unique have their own bars)', offered.length === 1 && first.startsWith(offered[0])
      && !(await pg.locator('#add-pop .addpk').count()) && await pg.locator('#node-dlg .udrow.me .vsb.blue, #node-dlg .udrow.me .vsk.pink, #node-dlg .udrow.me .vsb.pink').count() >= 2, offered.join(', '));
    await pg.locator('#add-pop .addsk').first().click();
    check('career: … which comes back at 2 stars', await nsk() === n0 && /2 stars/.test(await pg.locator('#node-dlg .udrow.me .vskb', { hasText: offered[0] }).getAttribute('aria-label')));
    await done();
    // a pool veteran's G1 wins: the same editor, from her popup; Arima Kinen (Classic and Senior) is one race, moved from one turn to the other
    await pg.locator('#node-dlg .twho .vwins').first().click(); await dialog(pg, 'wins-dlg');
    const savedWins = () => pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets[0].wins);
    const arima = pg.locator('#wins-dlg .wbtn[data-ix="22"]'), had = (await savedWins()).includes(22), n0w = (await savedWins()).length;
    if (had) { await arima.click(); }
    const gone = !(await arima.count()) && (await savedWins()).length === n0w - (had ? 1 : 0);
    await pg.locator('#wins-dlg .ag-add[aria-label="Senior Year, Late Dec: add a G1 win"]').click();
    await pg.locator('#wins-dlg .wpick', { hasText: 'Arima Kinen' }).click();
    check('career: a veteran\'s G1 wins: a click removes Arima Kinen, the Senior Late Dec menu adds it back there, saved', gone && await arima.count() === 1
      && /Late Dec/.test(await arima.evaluate(e => e.closest('.ag-slot').textContent)) && await arima.evaluate(e => e.closest('.ag-year').classList.contains('y2'))
      && (await savedWins()).includes(22) && /\d+ G1 win/.test(await pg.textContent('#wins-count')), `had ${had}`);
    if (!had) await arima.click();
    await pg.locator('#wins-dlg .dclose').click(); await done();
    await closeNode();
    // parent 1: a new veteran, with the skills prompt
    await node('node:p0'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
    check('career: the parent picker lists your veterans', await pg.locator('#vp-list .vpt').count() === 12);
    await pg.click('#vp-tab-new'); await pg.fill('#vp-filter', 'Special Week'); await pg.locator('#vp-umas .upick:visible').first().click();
    await dialog(pg, 'ask-dlg');
    check('career: picking a new uma asks whether to add the skills', /Add skills to Special Week/.test(await pg.textContent('#ask-title')));
    await ask('All 8 at 2★'); await done();
    const p1 = pg.locator('.pf').nth(0).locator('.pn.parent');
    const sparksIn = async (key, n) => { await node(key); const ok = await pg.locator('#node-dlg .udrow.me .vsk:not(.pink) .vskb').evaluateAll((els, n) => els.length === 8 && els.every(e => new RegExp(n + ' stars').test(e.getAttribute('aria-label'))), n); await closeNode(); return ok; };
    check('career: … she is parent 1, with the eight targets at 2 stars (in her popup)', /Special Week/.test(await p1.textContent()) && !(await p1.getAttribute('class')).includes('ghost')
      && await sparksIn('node:p0', 2));
    await vetsSeen();   // the veterans dialog draws its cards when open (and on opening)
    check('career: … her parents are not known yet but carry the same eight sparks, and she joins your veterans', await pg.locator('.pf').nth(0).locator('.pn.gp', { hasText: 'Unknown' }).count() === 2
      && await sparksIn('node:p0g0', 2) && await sparksIn('node:p0g1', 2)
      && await pg.locator('#vets .vet').count() === 13 && /Parent 1/.test(await pg.locator('#vet-12 .vtop').textContent()));
    check('career: the planner fills the rest', /Weighed 804 careers/.test(await note()), await note());
    // a grandparent: pick her uma, then her skills
    await node('node:p0g0'); await pg.locator('#node-dlg .facebtn').click(); await dialog(pg, 'pick-dlg');
    await pg.fill('#pick-filter', 'Gold Ship'); await pg.locator('#pick-list .upick:visible').first().click(); await ask('All 8 at 3★'); await done();
    const g1 = pg.locator('.pf').nth(0).locator('.pn.gp').first();
    check('career: a grandparent takes an uma and the skills chosen for her', /Gold Ship/.test(await g1.textContent()) && await sparksIn('node:p0g0', 3));
    // the other parent can't be the same uma
    await node('node:p1'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
    const blocked = await pg.locator('#vp-list .vpt:disabled').count();
    await pg.locator('#vpick-dlg .dclose').click();
    check('career: veterans that can\'t be parent 2 are greyed out', blocked === 0 || await pg.locator('#vp-list .vpt:disabled .vptw').count() === blocked);
    // the trainee
    await node('node:t'); await inNode(/^(Choose|Change) trainee$/).click(); await dialog(pg, 'pick-dlg');
    await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick:visible').first().click(); await done();
    check('career: the chosen trainee is set and the picks list only parents', !(await pg.locator('#tree .pn.trainee').getAttribute('class')).includes('ghost')
      && (await pg.$$eval('.alts tbody .altw', ws => ws.every(w => /^Parents /.test(w.textContent.trim())))));
    await pg.locator('.alts tbody tr').filter({ has: pg.getByRole('button', { name: 'Use this' }) }).first().getByRole('button', { name: 'Use this' }).click(); await done();
    check('career: "Use this" sets the places; with all set the career is just scored', /Scored in/.test(await note())
      && await pg.locator('#tree .pn.ghost').count() === 0, await note());
    // the last plan's table stays (it once went with the plan, so its other careers were out of reach), her row marked
    const saves = await pg.$$eval('.alts tbody .onum', ns => ns.map(n => n.textContent.trim()));
    check('career: … the last plan\'s careers stay listed, with their savings, the planner\'s pick and hers marked', await pg.locator('.alts').count() === 1
      && /shown above/.test(await pg.locator('.alts tbody tr.on').textContent()) && await pg.locator('.alts .otag.pick').count() === 1
      && saves.some(x => /^[+−]\d/.test(x)), saves.join(' '));
    // the chosen parent's popup shows its actions: the footer (Close, Change parent, Let the planner pick) wraps on narrow
    // popups and once ran past the popup's bottom, hiding the only way to unselect a parent; desktop and phone
    await node('node:p1');
    const footIn = () => pg.evaluate(() => { const d = document.getElementById('node-dlg').getBoundingClientRect();
      return [...document.querySelectorAll('#node-dlg .gmdfoot button')].every(b => { const r = b.getBoundingClientRect(); return r.top >= d.top && r.bottom <= d.bottom + 0.5 && r.bottom <= innerHeight; }); });
    const vp0 = pg.viewportSize(), footWide = await footIn(); await pg.setViewportSize({ width: 375, height: 740 }); const footPhone = await footIn(); await pg.setViewportSize(vp0);
    check('career: a chosen parent\'s popup shows Change parent and Let the planner pick inside it (desktop and phone)', footWide && footPhone
      && await inNode('Let the planner pick').count() === 1, `desktop ${footWide}, phone ${footPhone}`);
    await inNode('Let the planner pick').click(); await closeNode(); await done();
    check('career: "Let the planner pick" hands a place back', await pg.locator('#tree .pn.ghost').count() === 1 && /Weighed/.test(await note()));
    // back to the plan's places, then her again: both answers were drawn before, so neither runs a job
    const jobsNow = () => pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career').length), jobs0 = await jobsNow();
    await pg.click('#opt-back'); await done();
    const backOk = await pg.locator('#tree .pn.ghost').count() === 2 && /Planned in/.test(await note()) && await jobsNow() === jobs0;
    await node('node:t'); await inNode(/^(Choose|Change) trainee$/).click(); await dialog(pg, 'pick-dlg');
    await pg.fill('#pick-filter', 'Agnes Digital'); await pg.locator('#pick-list .upick:visible').first().click(); await done();
    check('career: "Back to the planner\'s pick" restores the plan\'s places, and an answer seen before is drawn without a run', backOk
      && await pg.locator('#tree .pn.ghost').count() === 1 && await jobsNow() === jobs0, `${await note()}; jobs ${jobs0} -> ${await jobsNow()}`);
    // a settings change searches again
    await pg.click('#o-umas'); await dialog(pg, 'set-umas');
    const umasNow = () => pg.evaluate(() => { const g = JSON.parse(localStorage.getItem('llp:settings') || '{}').global || {}; return JSON.stringify([g.roster, g.outfits]); });   // (nothing saved before the first settings change)
    const before = await umasNow(), searches = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career').length);
    await pg.locator('#set-umas #roster .ut .up').nth(3).click(); await pg.locator('#set-umas .dclose').click(); await done();
    check('career: the Umas dialog changes the owned umas and searches again', (await umasNow()) !== before
      && await pg.evaluate(n => window.__careerJobs.filter(j => j.cmd === 'career').length > n, searches), before);
    await pg.reload(); await done();
    await vetsSeen();   // the veterans dialog draws its cards when open (and on opening)
    check('career: the veterans and the chosen places outlive a reload', await pg.locator('#vets .vet').count() === 13
      && /Special Week/.test(await pg.locator('.pf').nth(0).textContent()) && /Agnes Digital/.test(await pg.locator('#tree .pn.trainee').textContent()));
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', '{"x": 1}'); await pg.click('#paste-load');
    check('career: a paste that is not an export says so and changes nothing', /No veterans found/.test(await pg.textContent('#v-msg')) && await pg.locator('#vets .vet').count() === 13);
    await pg.keyboard.press('Escape');
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  // the career planner's pool grows: an uma.moe tree added, generations saved from the veteran you got (her parents the
  // next one's grandparents), removals (from the 12 test veterans, parent 1 fixed so that saving has places to clear)
  async careergen(b) {
    const { ctx, pg, note, done, vetsOpen, vetsShut, vetsSeen, node, inNode, simStars, importDump } = await careerPage(b);
    await importDump();   // (its run is cut short by the reload)
    await pg.evaluate(() => { const o = JSON.parse(localStorage.getItem('llp:career')); o.global.plan = { trainee: null, parents: [o.global.vets[0].id, null] }; localStorage.setItem('llp:career', JSON.stringify(o)); });
    await pg.reload(); await done();
    const nVets = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length);
    // an uma.moe lineage planner tree: one veteran with her parents, added after the others (factorId x 10 + level is the
    // game's factor id: Uma Stan 2, Racing Spirit: Stamina (factor group 21009, skill group 21010) 3, Dirt pink 1; saddle
    // 3 is a title (left out), 165 and 168 single-race G1s)
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', '{"version":1,"type":"lineage-planner","payload":[{"position":"p1","characterId":101901,"sparks":[{"factorId":201590,"level":2},{"factorId":210090,"level":3},{"factorId":120,"level":1}],"manualWinSaddleIds":[3,165,168]},{"position":"p1-1","characterId":102101,"sparks":[{"factorId":201650,"level":2}],"manualWinSaddleIds":[168]},{"position":"p1-2","characterId":102901,"sparks":[{"factorId":200490,"level":1}],"manualWinSaddleIds":[]}]}'); await pg.click('#paste-load'); await done();
    const fromTree = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1));
    check('career: an uma.moe tree adds her with her parents after your veterans, marked yours (her popup says she came from uma.moe)', await pg.locator('#vets .vet').count() === nVets + 1
      && fromTree.src === 'tree' && fromTree.chara === 1019 && !fromTree.borrowed && fromTree.sparks[20159] === 2 && fromTree.sparks[21010] === 3 && fromTree.pinks[12] === 1
      && fromTree.wins.length === 2 && fromTree.parents.map(p => p.chara).join() === '1021,1029' && fromTree.parents[0].sparks[20165] === 2 && fromTree.parents[0].wins.length === 1
      && !/From uma\.moe/.test(await pg.locator('#vets .vet').last().textContent()) && /uma\.moe tree/.test(await pg.textContent('#v-msg'))
      && await (async () => { await vetsOpen(); await pg.locator('#vets .vtile').last().click(); await dialog(pg, 'node-dlg'); const t = await pg.textContent('#node-dlg .srctag').catch(() => ''); await pg.keyboard.press('Escape'); return t === 'From uma.moe'; })(), JSON.stringify(fromTree));
    await pg.locator('#vets .vet').last().locator('.vdel').click(); await done(); await vetsShut();
    // Save one generation, then another: a veteran's parents become the next one's grandparents.
    const beforeSimulation = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.length);
    await simStars(3);
    // Compare first: it plans exactly as the run after Add will, so Add takes that plan instead of planning again
    await pg.click('#sim-compare');
    await pg.waitForFunction(() => document.querySelector('#sim-verdict') && !document.querySelector('#sim-result .spin, #sim-result .runner'), null, { timeout: 180000 });
    const planJobs = () => pg.evaluate(() => window.__careerJobs.filter(j => j.next && !j.compare).length);
    const pj0 = await planJobs(), cmpPick = await pg.evaluate(() => window.__careerResults.filter(r => r.next && r.next.options).at(-1)?.next.choice?.trainee.id);
    await pg.click('#sim-add'); await done();
    check('career: Add to veterans after Compare shows the plan Compare made, with no second plan', await planJobs() === pj0
      && /Planned in|Weighed/.test(await pg.textContent('#run-note')), `plan jobs ${pj0} -> ${await planJobs()}, compare's pick ${cmpPick}`);
    await vetsSeen();   // the veterans dialog draws its cards when open (and on opening)
    check('career: adding her saves a labelled veteran and opens the next career', await pg.evaluate(n => {
      const c = JSON.parse(localStorage.getItem('llp:career')).global;
      const v = c.vets.at(-1);
      return c.vets.length === n + 1 && v.src === 'entered' && v.sparks[20159] === 3
        && v.parents.length === 2 && v.parents.every(p => p.chara > 0)
        && c.plan.trainee === null && c.plan.parents.every(p => p === null);
    }, beforeSimulation) && !(await pg.locator('#vets .simtag').count()) && await (async () => { await vetsOpen();
      await pg.locator('#vets .vtile').last().click(); await dialog(pg, 'node-dlg'); const t = await pg.textContent('#node-dlg .simtag').catch(() => ''); await pg.keyboard.press('Escape'); return t === 'Entered here'; })());
    const simulated = await pg.evaluate(() => JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1));
    // Force this simulated veteran as a parent and complete another hypothetical career.
    await pg.evaluate(v => { const o = JSON.parse(localStorage.getItem('llp:career')); o.global.plan = { trainee: null, parents: [v.id, null] }; localStorage.setItem('llp:career', JSON.stringify(o)); }, simulated);
    await pg.reload(); await done();
    check('career: her ancestry survives reloading', await pg.evaluate(v => JSON.stringify(JSON.parse(localStorage.getItem('llp:career')).global.vets.at(-1)) === JSON.stringify(v), simulated));
    await pg.click('#sim-add'); await done();
    check('career: another generation keeps her as a grandparent', await pg.evaluate(v => {
      const o = JSON.parse(localStorage.getItem('llp:career')).global, next = o.vets.at(-1);
      return o.vets.length === v.count + 2 && next.parents.some(p => p.chara === v.chara && JSON.stringify(p.sparks) === JSON.stringify(v.sparks));
    }, { ...simulated, count: beforeSimulation }));

    await node('node:p0'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
    await pg.locator('#vp-list .vpt:not(:disabled)').last().click(); await done();
    const removal = await pg.evaluate(() => {
      const c = JSON.parse(localStorage.getItem('llp:career')).global;
      return { id: c.plan.parents.find(Boolean), count: c.vets.length };
    });
    const removalIndex = await pg.evaluate(id => JSON.parse(localStorage.getItem('llp:career')).global.vets.findIndex(v => v.id === id), removal.id);
    await vetsOpen(); await pg.locator(`#vet-${removalIndex} .vdel`).click();
    check('career: removing a selected veteran immediately redraws and clears the fixed parent', await pg.locator('#vets .vet').count() === removal.count - 1
      && await pg.evaluate(({ id, count }) => { const c = JSON.parse(localStorage.getItem('llp:career')).global;
        return c.vets.length === count - 1 && !c.vets.some(v => v.id === id) && !c.plan.parents.includes(id); }, removal));
    await done();
    await pg.reload(); await done();
    check('career: removed veterans stay removed after reload', await pg.evaluate(id => !JSON.parse(localStorage.getItem('llp:career')).global.vets.some(v => v.id === id), removal.id));
    await vetsOpen(); while (await pg.locator('#vets .vet').count() > 1) await pg.locator('#vets .vdel').last().click();
    await done();
    check('career: removing down to one veteran clears obsolete indexed results without a page error', await pg.locator('#vets .vet').count() === 1
      && /Choose both parents, or import/.test(await note()) && await pg.locator('#simulation').count() === 0);

    await pg.keyboard.press('Escape');
    if (b.browserType().name() === 'chromium') { await still(pg); const f = artless(await pg.evaluate(AUDIT)); check('career: every text run meets WCAG AAA', !f.length, f.slice(0, 5).join(' | ')); }
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  // the career planner on pasted pools: the goal's steps (8/8 veterans of two umas), a borrowed or owned fully looped
  // veteran, the trio path (from the 12 test veterans' export, changed)
  async careerpool(b) {
    const { ctx, pg, done, vetsOpen, vetsShut, importDump } = await careerPage(b);
    await importDump(); await done();
    await vetsOpen(); await pg.click('#v-export'); await dialog(pg, 'vx-dlg'); const exText = await pg.inputValue('#vx-text'); await pg.locator('#vx-dlg .dclose').click();
    // the goal's steps: 8/8 veterans of two different umas -> step 3 in the note; the planner still picks (nothing fixed),
    // pricing them as real 8/8 sides of its trio
    const goalEx = JSON.parse(exText), all8 = Object.fromEntries(goalEx.job.targets.map(t => [t.group, 2]));
    for (const v of goalEx.veterans) for (const g of v.parents) g.sparks = {};   // (the test file's parents carry every spark: they would make a looped parent at once)
    const two = [0, goalEx.veterans.findIndex(v => v.chara !== goalEx.veterans[0].chara)]; for (const k of two) goalEx.veterans[k].sparks = { ...all8 };
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', JSON.stringify(goalEx)); await pg.click('#paste-load'); await vetsShut();
    await done(); await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 });
    const goalJob = await pg.evaluate(() => window.__careerJobs.filter(j => j.cmd === 'career' && !j.next && !j.compare).at(-1));
    const goalRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    check('career: with 8/8 veterans of two umas nothing is fixed (no goal steps), and the planner prices them in its trio and picks', !(await pg.locator('#goal-note').count())
      && goalJob && goalJob.fix.length === 0 && goalRes && goalRes.next.mode === 'trio' && goalRes.next.members.length === 2 && goalRes.next.members.every(m => typeof m.real === 'boolean')
      && await pg.locator('.alts .otag.pick').count() === 1,
      `${JSON.stringify(goalJob && goalJob.fix)} | ${goalRes && JSON.stringify(goalRes.next.v0_plan)} | pick ${goalRes && JSON.stringify(goalRes.next.choice && [goalRes.next.choice.trainee.name, goalRes.next.choice.parents])}`);
    // each uma also gets her career on two of those 8/8s (only there is her own 8/8 a fully looped parent), the trio's last
    // run is one, and the table tags them
    const nTags = await pg.locator('.alts .otag', { hasText: /^\d+\/\d+ parents$/ }).count();
    check('career: with 8/8 veterans the plan offers careers on two of them, the route\'s last run among them, tagged in the table',
      goalRes && goalRes.next.options.some(o => o.looped) && goalRes.next.options.filter(o => o.role === 'finish').every(o => o.looped) && nTags >= 1,
      `${goalRes && goalRes.next.options.filter(o => o.looped).length} looped rows, ${nTags} tagged`);
    // a borrowed fully looped veteran is a parent to use, not the goal: the note stays at step 1 and the planner plans on
    const borEx = JSON.parse(exText);
    for (const v of borEx.veterans) for (const g of v.parents) g.sparks = {};
    Object.assign(borEx.veterans[0], { borrowed: true, sparks: { ...all8 } }); for (const g of borEx.veterans[0].parents) g.sparks = { ...all8 };
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', JSON.stringify(borEx)); await pg.click('#paste-load'); await vetsShut();
    await done(); await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 });
    const borRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    check('career: a borrowed fully looped veteran is a parent to use: the planner plans on, every career scored', borRes && borRes.next.mode === 'trio' && borRes.next.options.every(o => o.score != null),
      `${borRes && borRes.next.mode}`);
    // the same veteran as yours: the goal goes on (the next fully looped parent, any uma), every career still scored
    borEx.veterans[0].borrowed = false;
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', JSON.stringify(borEx)); await pg.click('#paste-load'); await vetsShut();
    await done(); await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 });
    const ownRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    check('career: with a fully looped parent of yours the planner goes for the next one and scores every career', ownRes && ownRes.next.mode === 'trio'
      && ownRes.next.options.every(o => o.score != null) && await pg.locator('.alts .otag.pick').count() === 1, `${ownRes && ownRes.next.mode}`);
    // the trio path: the same veterans without one target and with their parents' sparks cleared: every career scored
    // against the cheapest trio, each step named, the pick explained
    const trioEx = JSON.parse(exText), drop = String(trioEx.job.targets.find(x => !x.guaranteed).group);
    for (const v of trioEx.veterans) { delete v.sparks[drop]; for (const g of v.parents) g.sparks = {}; }
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', JSON.stringify(trioEx)); await pg.click('#paste-load'); await vetsShut();
    await done(); await pg.waitForFunction(() => /Planned in/.test(document.getElementById('run-note').textContent), null, { timeout: 300000 });
    const trioRes = await pg.evaluate(() => window.__careerResults.filter(r => r.next).at(-1));
    const trioCells = await pg.$$eval('.alts .optab tbody tr', ls => ls.map(l => l.children[2].textContent));
    // the plan behind the numbers sits in the "What do these mean?" popup: saves per run defined first, then the route and its formula
    await pg.click('#save-why'); await dialog(pg, 'save-dlg');
    const trioLoop = await pg.textContent('#save-dlg .oloop'), saveTxt = await pg.textContent('#save-dlg');
    await pg.mouse.click(5, 5);   // a click on the backdrop closes it
    check('career: with no looped parent the plan scores every career against the cheapest trio; its explanation popup names each step and says what saves per run means', !!trioRes && trioRes.next.mode === 'trio'
      && trioRes.next.members.length === 2 && /^First .+; then .+; then .+ with those two \d+\/\d+s as parents.*\. About [\d,]+ careers in all\.$/.test(trioLoop)
      && /1 in [\d,]+/.test(trioLoop) && /Saves per run: how much running this career next lowers the planner's estimate \(below\) of the careers you still need for a fully looped parent .+, counting the run itself\. .+ It is for comparing careers, not a forecast of careers you will save\./.test(saveTxt) && /The planner makes a rough estimate of how many careers you still need \(about [\d,]+ today\)/.test(saveTxt) && trioCells.length >= 2 && trioCells.every(c => /^[+−]\d+\.\d$/.test(c))
      && await pg.locator('.alts .otag.pick').count() === 1 && !(await pg.locator('#save-dlg[open]').count()) && !(await pg.locator('.alts p.hint, .alts .oloop').count()),
      `${trioRes && trioRes.next.mode} | ${trioLoop} | ${trioCells.join(' ')}`);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  // Borrowed parent (Career setup) on the 12 test veterans with the plan's first pick's parent made borrowed: Not this
  // career and Never send borrowed_parent ('later', 'never'; Allowed sends none), are saved with the settings, and leave
  // every career the planner offers without a borrowed parent; a borrowed parent you chose is blocked, also after a reload
  async careerborrow(b) {
    const { ctx, pg, note, done, planned, vetsOpen, vetsShut, node, inNode, importDump } = await careerPage(b);
    await importDump(); await done(); await planned();
    const lastPlan = () => pg.evaluate(() => ({ job: window.__careerJobs.filter(j => j.cmd === 'career' && j.next && !j.compare).at(-1),
      r: window.__careerResults.filter(r => r.next).at(-1), s: window.__careerResults.filter(r => !r.next && r.results?.length).at(-1) }));
    const withBor = (p) => [p.r.next.choice, ...p.r.next.options, ...p.s.results].filter(r => r && r.parents.some(i => p.job.legacies[i].borrowed)).length;
    const exported = async () => { await vetsOpen(); await pg.click('#v-export'); await dialog(pg, 'vx-dlg'); const x = JSON.parse(await pg.inputValue('#vx-text')); await pg.locator('#vx-dlg .dclose').click(); await vetsShut(); return x; };
    const bi = (await lastPlan()).r.next.choice.parents[0], ex = await exported();
    ex.veterans[bi].borrowed = true;
    await vetsOpen(); await pg.click('#v-paste'); await dialog(pg, 'paste-dlg'); await pg.fill('#paste-text', JSON.stringify(ex)); await pg.click('#paste-load'); await vetsShut();
    await done(); await planned();
    const a = await lastPlan();
    const pressed = () => pg.$$eval('#bp button', bs => bs.filter(b => b.getAttribute('aria-pressed') === 'true').map(b => b.dataset.v).join());
    const saved = () => pg.evaluate(() => JSON.parse(localStorage.getItem('llp:settings')).common.borrowPar);
    check('career: Borrowed parent starts at Allowed, its job without borrowed_parent, and the planner\'s careers may borrow (the premise)', await pressed() === 'allow'
      && !('borrowed_parent' in a.job) && withBor(a) > 0 && /limited per day/.test(await pg.textContent('#bp-hint')), `${await pressed()} ${a.job.borrowed_parent} ${withBor(a)}`);
    await pg.click('#bp button[data-v="later"]'); await done(); await planned();
    const l = await lastPlan();
    check('career: Not this career sends borrowed_parent "later", is saved, and no career the planner offers has a borrowed parent', l.job.borrowed_parent === 'later'
      && await pressed() === 'later' && await saved() === 'later' && withBor(l) === 0 && l.r.next.options.length > 0, `${l.job.borrowed_parent} ${await saved()} ${withBor(l)}`);
    await pg.click('#bp button[data-v="never"]'); await done(); await planned();
    const n = await lastPlan();
    check('career: Never sends borrowed_parent "never", and no career the planner offers has a borrowed parent', n.job.borrowed_parent === 'never' && await saved() === 'never' && withBor(n) === 0,
      `${n.job.borrowed_parent} ${withBor(n)}`);
    await pg.click('#bp button[data-v="allow"]'); await done();
    check('career: back at Allowed the job has no borrowed_parent', !('borrowed_parent' in (await exported()).job) && await saved() === 'allow');
    // the borrowed veteran chosen as parent 1, then borrowing turned off: blocked, and the picker greys her out (her place
    // in the list as the job has it: the paste may have moved her)
    const bj = a.job.legacies.findIndex(l => l.borrowed);
    await node('node:p0'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
    await pg.locator('#vp-list .vpt').nth(bj).click(); await done();
    await pg.click('#bp button[data-v="never"]'); await done();
    const blockedNote = /is borrowed, and Borrowed parent is set to Never: choose one of your veterans/;
    const n1 = await note();
    await node('node:p0'); await inNode(/^(Choose|Change) parent$/).click(); await dialog(pg, 'vpick-dlg');
    const greyed = await pg.locator('#vp-list .vpt').nth(bj).evaluate(b => b.disabled && /Can't: borrowed/.test(b.textContent));
    await pg.locator('#vpick-dlg .dclose').click();
    check('career: a borrowed parent you chose is blocked when borrowing is off, and the parent picker greys her out', blockedNote.test(n1) && greyed, n1);
    await pg.reload(); await done();
    check('career: … after a reload the choice and the block stand', await pressed() === 'never' && blockedNote.test(await note()), await note());
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  // the career planner's run (the 12 test veterans): an edit that leaves the job as it is lets the plan under way go on,
  // the status line says how far the plan has got, and after a reload the stored main search is taken (no search again)
  async careerrun(b) {
    const { ctx, pg, note, done, planned, importDump } = await careerPage(b);
    // the status line's texts, and the workers started (a stopped plan takes its worker with it)
    await pg.evaluate(() => { window.__notes = []; const n = document.getElementById('run-note');
      new MutationObserver(() => { if (window.__notes.at(-1) !== n.textContent) window.__notes.push(n.textContent); }).observe(n, { subtree: true, childList: true, characterData: true });
      const W = window.Worker; window.__workers = 0; window.Worker = function (...a) { window.__workers++; return new W(...a); }; });
    await importDump();
    await pg.waitForFunction(() => window.__careerJobs.some(j => j.next));   // the plan job is in the worker
    // a preset saved (a settings save that leaves the job as it is), all at once while the plan runs
    const edit = await pg.evaluate(() => { const plan = () => /Planning your next career/.test(document.getElementById('run-note').textContent), was = plan();
      const jobs = window.__careerJobs.length, workers = window.__workers;
      document.getElementById('t-preset').click(); document.getElementById('pr-save').click(); document.getElementById('prn-save').click(); document.getElementById('preset-dlg').close();
      return { was, now: plan(), jobs, workers, presets: JSON.parse(localStorage.getItem('llp:settings') || '{}').common?.presets?.length }; });
    await planned();
    const after = await pg.evaluate(() => ({ jobs: window.__careerJobs.filter(j => j.cmd === 'career').length, posted: window.__careerJobs.length, workers: window.__workers,
      plans: window.__careerResults.filter(r => r.next).length }));
    check('career: a preset saved during the plan (the job as it was) lets the plan go on: no new job, no new worker, one plan answered',
      edit.was && edit.now && after.posted === edit.jobs && after.workers === edit.workers && after.plans === 1 && edit.presets > 1, JSON.stringify({ edit, after }));
    const prog = await pg.evaluate(() => window.__notes.filter(t => /Planning your next career… \(.+\)/.test(t)).map(t => t.match(/\((.+?)\)/)[1]));
    check('career: the plan says how far it has got ("scoring k of n umas"), the counts rising', prog.some(t => /^scoring \d+ of \d+ umas$/.test(t))
      && prog.filter(t => /^scoring/.test(t)).map(t => +t.match(/\d+/)[0]).every((k, i, a) => !i || k >= a[i - 1]), [...new Set(prog)].slice(0, 8).join(' | '));
    // the worker's kept work, stored once the worker is idle after the plan; then a reload: the main search is the stored one
    // (an evaluate awaits the promise; a waitForFunction would take the promise itself as true)
    const stored = () => pg.evaluate(() => new Promise(res => { const rq = indexedDB.open('llp-solver');
      rq.onsuccess = () => { try { const g = rq.result.transaction('state').objectStore('state').getAll(); g.onsuccess = () => res(g.result.some(s => s.v === 3 && s.solve?.length > 0)); g.onerror = () => res(false); } catch { res(false); } };
      rq.onerror = () => res(false); }));
    let saved = false; for (let i = 0; i < 80 && !(saved = await stored()); i++) await pg.waitForTimeout(250);
    const before = await note();
    await pg.reload(); await done(); await planned();
    const re = await pg.evaluate(() => ({ restored: window.__careerResults.find(r => r.solve !== undefined), search: window.__careerResults.find(r => r.results) }));
    check('career: after a reload the stored main search is taken: the search job searches no deck', saved && re.restored?.solve > 0 && re.search?.work?.nodes === 0 && /Planned in/.test(await note()),
      `stored ${saved} | state_in ${JSON.stringify(re.restored)} | nodes ${re.search?.work?.nodes} | before: ${before} | after: ${await note()}`);
    check('no page errors', !pg.errors.length, pg.errors.join(' | '));
    await ctx.close();
  },
  async contrast(b) {
    if (b.browserType().name() !== 'chromium') return;   // one engine is enough for colours
    const pg = await open(b);
    await pg.locator('#roster .ut .up').nth(1).click(); await pg.locator('#cards .ct').nth(1).click(); await settle(pg);
    await still(pg);   // the results' fade back from the searching dim (0.15 s)
    const fails = [...await pg.evaluate(AUDIT)];
    await pg.click('#x-scen'); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.deck .dc').first().click(); fails.push(...(await pg.evaluate(AUDIT)).filter(x => /cpop|sw|gtl|ctt/.test(x))); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Race schedule' }).first().click(); await dialog(pg, 'sched-dlg'); fails.push(...await pg.evaluate(AUDIT));
    await pg.locator('#sched-dlg .stab').last().click(); fails.push(...await pg.evaluate(AUDIT)); await pg.keyboard.press('Escape');
    await pg.locator('.loop .lacts button', { hasText: 'Affinity' }).first().click(); await dialog(pg, 'why-dlg'); fails.push(...await pg.evaluate(AUDIT));
    const shown = artless(fails); check('contrast: every text run meets WCAG AAA', !shown.length, shown.slice(0, 5).join(' | '));
    await pg.context().close();
    const p2 = await open(b, { before: (p) => p.route('**/img.*/scen/**', r => r.abort()) });   // the first search done
    await still(p2);   // the results' fade back from the searching dim, as above
    await p2.click('#x-scen'); const bf = (await p2.evaluate(AUDIT)).filter(x => /^b\./.test(x) || /scen|stile|slogo|snm|ssub/.test(x));
    check('contrast: scenario badges (no logo files) meet WCAG AAA', !bf.length, bf.join(' | '));
    await p2.context().close();
  },
};

// the longest groups start first (each runs serially; one started last would end the run alone), the slowest first
const FIRST = ['careergen', 'careervets', 'career', 'careerpool', 'careerphone'];
const pick = (QUICK ? QUICK_GROUPS : Object.keys(TESTS)).filter(t => !argv.some(a => a in TESTS) || argv.includes(t))
  .sort((a, c) => (FIRST.includes(a) ? FIRST.indexOf(a) : FIRST.length) - (FIRST.includes(c) ? FIRST.indexOf(c) : FIRST.length));
const JOBS = Math.max(1, +process.env.LLP_JOBS || 4), T0 = Date.now();
await Promise.all(BROWSERS.map(async (name) => {
  const b = await pw[name].launch(), queue = [...pick];
  const worker = async () => { for (let t; (t = queue.shift());) {
    const buf = [], t0 = Date.now(); buf.prof = PROF ? {} : null;
    await LOG.run(buf, async () => { try { await TESTS[t](b); } catch (e) { failed++; say('  FAIL ' + t + ' threw: ' + String(e).split('\n').slice(0, 3).join(' ')); } });
    const wall = Date.now() - t0;
    console.log(`[${name}] ${t} (${(wall / 1000).toFixed(1)} s)\n` + buf.join('\n'));
    if (PROF) (await import('fs')).appendFileSync(PROF, JSON.stringify({ browser: name, group: t, wall, start: t0 - T0, ...buf.prof }) + '\n');
  } };
  await Promise.all(Array.from({ length: JOBS }, worker));
  await b.close();
}));
console.log(`\n${passed} passed, ${failed} failed${skipped ? `, ${skipped} skipped (no game art)` : ''} in ${((Date.now() - T0) / 1000).toFixed(1)} s`);
process.exit(failed ? 1 : 0);
