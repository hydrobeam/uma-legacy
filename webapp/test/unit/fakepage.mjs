// The page data for node tests of modules that read the game data (data.ts): a module shaped as the '#page-data' the
// build writes (vite/pagedata.mjs), for a small made-up game. A test file hands it to the page's modules with
// `vi.mock('#page-data', () => import('./fakepage.mjs'))`, then `await loadData(career)` as the page does; the lists
// are its arrays, so a test may add to them before `loadData`. Its released dates are in the past (FUTURE: one card not
// out yet), so what a job holds does not depend on the day the test runs.
const PAST = 1_600_000_000,
  FUTURE = 9_000_000_000;
const outfit = (id) => ({ id, rel: PAST });
export const UMAS = [
  { id: 1001, name: 'Uma A', outfits: [outfit(100101), outfit(100102)] },
  { id: 1002, name: 'Uma B', outfits: [outfit(100201)] },
  { id: 1003, name: 'Uma C', outfits: [outfit(100301)] },
  { id: 1004, name: 'Uma D' },
  { id: 1005, name: 'Uma E', outfits: [outfit(100501), outfit(100502)] },
];
const card = (id, rar, pal, rel = PAST) => ({ id, name: `[Card] ${id}`, rar, pal, kind: pal ? 'pal' : 'speed', rel });
export const CARDS = [
  card(30001, 3, false),
  card(30002, 2, false),
  card(30003, 1, false), // an R card, not a pal: out of every search
  card(30004, 1, true), // an R pal: in
  card(30005, 3, true),
  card(39999, 3, false, FUTURE), // not out yet: left out of the page's card list
];
// the made-up game's cards that are out
export const CARDS_OUT = CARDS.filter((c) => c.rel <= PAST).map((c) => c.id);
export const SKILLS = [
  [20049, 'Skill 20049', 'Gold 20049'],
  [20046, 'Skill 20046', 'Gold 20046'],
  [20159, 'Skill 20159', null],
  [20166, 'Skill 20166', null],
];
// the rest of what '#page-data' exports
export const assets = { wasm: 'solver.test.wasm', engine: { global: 'engine.global.test', gm: 'engine.gm.test' } };
export const img = { keys: [], base: 'img/', ext: {} };
export const skillIcons = { white: {}, gold: {} };
export const raceIds = {};
export const hintBase = { unity: 0.5, gc: 0.6, ura: 0.4, trackblazer: 0.45, grandmasters: 0.55 };
export { CARDS as cards, SKILLS as skills, UMAS as umas };
