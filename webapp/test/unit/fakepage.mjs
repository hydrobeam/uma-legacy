// A page for node tests of modules that read the game data (data.ts): the data the build gives the page, keyed by the
// elements that once held it (self.__LLP_DATA; pagedata.mjs hands it to data.ts as '#page-data'), for a small made-up
// game, and a document that has no element (a test that draws views registers happy-dom's over it). Import it before
// any module of the page; then `await loadData(career)` as the page does. Its released dates are in the past (FUTURE:
// one card not out yet), so what a
// job holds does not depend on the day the test runs.
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
const SKILLS = [
  [20049, 'Skill 20049', 'Gold 20049'],
  [20046, 'Skill 20046', 'Gold 20046'],
  [20159, 'Skill 20159', null],
  [20166, 'Skill 20166', null],
];
globalThis.self = globalThis;
globalThis.__LLP_DATA = {
  'data-assets': { wasm: 'solver.test.wasm', engine: { global: 'engine.global.test', gm: 'engine.gm.test' } },
  'data-img': { keys: [], base: 'img/', ext: {} },
  'data-skillicon': { white: {}, gold: {} },
  'data-raceids': {},
  'data-hintbase': { unity: 0.5, gc: 0.6, ura: 0.4, trackblazer: 0.45, grandmasters: 0.55 },
  'data-cards': CARDS,
  'data-umas': UMAS,
  'data-skills': SKILLS,
};
globalThis.document = { getElementById: () => null };
