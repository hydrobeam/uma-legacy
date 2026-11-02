// The browser suite's groups as Playwright tests, one test per group (each a module of this folder named after it, its
// checks in order; each gets a fresh page, fresh storage, unless its module says otherwise). Expectations come from the
// page's own embedded data where possible, so a data update does not break the suite; the fixed numbers are the Global
// default search's odds (R cards off, Unity Cup): 1 in 103 for all 8, 1 in 12.1 for 7 or better.
import { test } from 'playwright/test';
import { runGroup } from './helpers.mjs';
import career from './career.mjs';
import careervets from './careervets.mjs';
import careergen from './careergen.mjs';
import careerumamoe from './careerumamoe.mjs';

// In the order they start: the longest first (a group runs serially; one started last would end the run alone).
const GROUPS = {
  careergen,
  careervets,
  career,
  careerumamoe,
};
// the fast tier (tag @quick; ./build.sh testq, with LLP_QUICK=1): each page's core path: the planner loads and its
// search returns (load), the result's dialogs (popups), the phone layout (phone), /about, and the career page up to
// its one-run answer (career stops there in this tier)
const QUICK = ['career'];
// the groups Firefox runs too (tag @both; playwright.config.mjs): where the engine matters, the worker, wasm, IndexedDB
// and timing (load, states, autosearch, reload, delivery, career, careerrun) or layout and focus (careerphone, phone,
// panels, keyboard, popups; Firefox rounds lengths differently). The rest test the page's logic, which Chromium covers.
const BOTH = ['career'];
// the groups whose solver replies are replayed (solverreplay.mjs): those that use the solver to put the page in a state,
// or check what the page draws from its replies (the replies were checked by the live run that recorded them). The rest
// run the real solver: they check its worker's live behaviour (load, autosearch, states: kills, restarts, a failing job,
// jobs in flight; career, careerrun: progress phases, races between rolls and comparisons, the background check, the
// kept work stored and restored; delivery: the wasm and worker as served), or the deck solver's own odds (nonecards), or
// use no solver (about). Under LLP_SOLVER=live every group runs the real solver and records its replies.
const REPLAY = ['careervets', 'careergen', 'careerumamoe'];

// the groups run on separate workers (each test is one group; the config keeps fullyParallel off for any other file)
test.describe.configure({ mode: 'parallel' });
for (const [name, run] of Object.entries(GROUPS))
  test(
    name,
    { tag: [...(QUICK.includes(name) ? ['@quick'] : []), ...(BOTH.includes(name) ? ['@both'] : [])] },
    ({ browser }) => runGroup(name, run, browser, { replay: REPLAY.includes(name) }),
  );
