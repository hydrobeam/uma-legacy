// The browser suite's groups as Playwright tests, one test per group (each a module of this folder named after it, its
// checks in order; each gets a fresh page, fresh storage, unless its module says otherwise). Expectations come from the
// page's own embedded data where possible, so a data update does not break the suite; the fixed numbers are the Global
// default search's odds (R cards off, Unity Cup): 1 in 103 for all 8, 1 in 12.1 for 7 or better.
import { test } from 'playwright/test';
import { runGroup } from './helpers.mjs';
import load from './load.mjs';
import scenario from './scenario.mjs';
import autosearch from './autosearch.mjs';
import roster from './roster.mjs';
import upcoming from './upcoming.mjs';
import cards from './cards.mjs';
import panels from './panels.mjs';
import targets from './targets.mjs';
import keyboard from './keyboard.mjs';
import states from './states.mjs';
import fourloop from './fourloop.mjs';
import borrowed from './borrowed.mjs';
import popups from './popups.mjs';
import phone from './phone.mjs';
import careerphone from './careerphone.mjs';
import nonecards from './nonecards.mjs';
import reload from './reload.mjs';
import delivery from './delivery.mjs';
import gm from './gm.mjs';
import settings from './settings.mjs';
import about from './about.mjs';
import dark from './dark.mjs';
import career from './career.mjs';
import careervets from './careervets.mjs';
import careergen from './careergen.mjs';
import careerpool from './careerpool.mjs';
import careerborrow from './careerborrow.mjs';
import careerumamoe from './careerumamoe.mjs';
import careerrun from './careerrun.mjs';
import contrast from './contrast.mjs';

// In the order they start: the longest first (a group runs serially; one started last would end the run alone).
const GROUPS = {
  careergen,
  careervets,
  career,
  careerpool,
  careerphone,
  load,
  scenario,
  autosearch,
  roster,
  upcoming,
  cards,
  panels,
  targets,
  keyboard,
  states,
  fourloop,
  borrowed,
  popups,
  phone,
  nonecards,
  reload,
  delivery,
  gm,
  settings,
  about,
  dark,
  careerborrow,
  careerumamoe,
  careerrun,
  contrast,
};
// the fast tier (tag @quick; ./build.sh testq, with LLP_QUICK=1): each page's core path: the planner loads and its
// search returns (load), the result's dialogs (popups), the phone layout (phone), the contrast audit in light mode
// (contrast), /about, and the career page up to its one-run answer (career stops there in this tier)
const QUICK = ['career', 'load', 'popups', 'phone', 'contrast', 'about'];

// the groups run on separate workers (each test is one group; the config keeps fullyParallel off for any other file)
test.describe.configure({ mode: 'parallel' });
for (const [name, run] of Object.entries(GROUPS))
  test(name, { tag: QUICK.includes(name) ? ['@quick'] : [] }, ({ browser }) => runGroup(name, run, browser));
