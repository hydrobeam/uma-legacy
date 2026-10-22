// Browser regression suite for the built page (webapp/dist, served on a local port; LLP_URL tests another address).
// Node + Playwright (webapp/node_modules). LLP_SHOTS=dir saves pictures of the scenario button and picker there.
//   node webapp/nodebrowser.mjs                 every test, Chromium and Firefox
//   node webapp/nodebrowser.mjs chromium roster only the named browsers / tests (any order)
//   node webapp/nodebrowser.mjs quick           the fast tier (./build.sh testq): each page's core path, Chromium only
// Both browsers run at once, each with LLP_JOBS test groups in parallel (default 4; LLP_JOBS=1 runs them one by one);
// a group's lines print together when it ends. Exit code 1 if any check fails. Expectations come from the page's own
// embedded data where possible, so a data update does not break the suite; the fixed numbers are the Global default
// search's odds (R cards off, Unity Cup): 1 in 107 for all 8, 1 in 12.6 for 7 or better.
// Each group is a module of webapp/test/ named after it (its checks, in order); webapp/test/helpers.mjs holds what they
// share. Each group gets a fresh page (fresh storage) unless its module says otherwise.
import { createRequire } from 'node:module';
import fs from 'node:fs';
import { tally, groupLog, say, PROF, QUICK } from './test/helpers.mjs';
import load from './test/load.mjs';
import scenario from './test/scenario.mjs';
import autosearch from './test/autosearch.mjs';
import roster from './test/roster.mjs';
import upcoming from './test/upcoming.mjs';
import cards from './test/cards.mjs';
import panels from './test/panels.mjs';
import targets from './test/targets.mjs';
import keyboard from './test/keyboard.mjs';
import states from './test/states.mjs';
import fourloop from './test/fourloop.mjs';
import borrowed from './test/borrowed.mjs';
import popups from './test/popups.mjs';
import phone from './test/phone.mjs';
import careerphone from './test/careerphone.mjs';
import nonecards from './test/nonecards.mjs';
import reload from './test/reload.mjs';
import delivery from './test/delivery.mjs';
import gm from './test/gm.mjs';
import settings from './test/settings.mjs';
import about from './test/about.mjs';
import dark from './test/dark.mjs';
import career from './test/career.mjs';
import careervets from './test/careervets.mjs';
import careergen from './test/careergen.mjs';
import careerpool from './test/careerpool.mjs';
import careerborrow from './test/careerborrow.mjs';
import careerrun from './test/careerrun.mjs';
import contrast from './test/contrast.mjs';

/** The groups by name, each an `async (browser) => void` that logs its checks. */
const TESTS = { load, scenario, autosearch, roster, upcoming, cards, panels, targets, keyboard, states, fourloop, borrowed, popups, phone, careerphone,
  nonecards, reload, delivery, gm, settings, about, dark, career, careervets, careergen, careerpool, careerborrow, careerrun, contrast };

process.env.PLAYWRIGHT_BROWSERS_PATH ??= '0';   // the browsers live in webapp/node_modules (./build.sh browsers)
let pw;
try { pw = createRequire(import.meta.url)('playwright'); }
catch { console.error('Playwright is missing: run ./build.sh browsers (or cd webapp && npm ci)'); process.exit(1); }

// ---------- what runs
const argv = process.argv.slice(2);
// the fast tier (`quick`; ./build.sh testq): each page's core path, Chromium only unless a browser is named: the planner
// loads and its search returns (load), the result's dialogs (popups), the phone layout (phone), the contrast audit in
// light mode (contrast), /about, and the career page up to its one-run answer (career stops there in this tier)
const QUICK_GROUPS = ['career', 'load', 'popups', 'phone', 'contrast', 'about'];
const BROWSER_NAMES = ['chromium', 'firefox'];
const BROWSERS = BROWSER_NAMES.filter(b => argv.some(a => BROWSER_NAMES.includes(a)) ? argv.includes(b) : !QUICK || b === 'chromium');
// the longest groups start first (each runs serially; one started last would end the run alone), the slowest first
const FIRST = ['careergen', 'careervets', 'career', 'careerpool', 'careerphone'];
const startRank = (group) => FIRST.includes(group) ? FIRST.indexOf(group) : FIRST.length;
const groups = (QUICK ? QUICK_GROUPS : Object.keys(TESTS)).filter(t => !argv.some(a => a in TESTS) || argv.includes(t))
  .sort((a, c) => startRank(a) - startRank(c));
const JOBS = Math.max(1, +process.env.LLP_JOBS || 4), T0 = Date.now();

// ---------- running them
// one group in one browser: its lines buffered and printed together when it ends (a throw counts as one failure)
async function runGroup(browserName, browser, group) {
  const log = { lines: [], prof: PROF ? {} : null }, t0 = Date.now();
  await groupLog.run(log, async () => { try { await TESTS[group](browser); } catch (e) { tally.failed++; say('  FAIL ' + group + ' threw: ' + String(e).split('\n').slice(0, 3).join(' ')); } });
  const wall = Date.now() - t0;
  console.log(`[${browserName}] ${group} (${(wall / 1000).toFixed(1)} s)\n` + log.lines.join('\n'));
  if (PROF) fs.appendFileSync(PROF, JSON.stringify({ browser: browserName, group, wall, start: t0 - T0, ...log.prof }) + '\n');
}
// every chosen group in one browser, JOBS at a time, in the order of `groups`
async function runBrowser(name) {
  const browser = await pw[name].launch(), queue = [...groups];
  const worker = async () => { while (queue.length > 0) await runGroup(name, browser, queue.shift()); };
  await Promise.all(Array.from({ length: JOBS }, worker));
  await browser.close();
}
await Promise.all(BROWSERS.map(runBrowser));
const { passed, failed, skipped } = tally;
console.log(`\n${passed} passed, ${failed} failed${skipped ? `, ${skipped} skipped (no game art)` : ''} in ${((Date.now() - T0) / 1000).toFixed(1)} s`);
process.exit(failed ? 1 : 0);
