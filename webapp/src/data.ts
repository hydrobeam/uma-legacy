// The page's data, for the chosen era: the Global era's comes with the page's scripts ('#page-data', built in by
// vite/pagedata.mjs; a node test mocks it with test/unit/fakepage.ts), the GM era's is fetched.
import * as page from '#page-data';
import { ERA } from './saved.ts';
import { assert, must } from './must.ts';
import { Distance, Surface } from './gen/solver.ts';
import type { Grades } from './gen/solver.ts';
/** Fingerprinted files of the build (vite/pagedata.mjs): the wasm, each era's solver data, the GM era's page data. */
export interface Assets {
  wasm: string;
  /** each era's solver data */
  engine: { global: string; gm: string };
  dataGm: string;
}
export const ASSETS: Assets = page.assets;
/** A support card (pkg/loopopt/export/export_page.py, its `cards`). */
export interface Card {
  id: number;
  /** "[Title] Character" */
  name: string;
  /** rarity: R, SR, SSR */
  rar: 1 | 2 | 3;
  /** a friend (pal) or group card */
  pal: boolean;
  /** 'speed', 'stamina', ..., 'pal', 'group', 'other', or '' when unknown (drawn as 'other') */
  kind: string;
  /** release time, Unix seconds */
  rel: number;
  /** GameTora's url_name for its page */
  gt: string | null;
  /** skill groups it hints when training with it */
  whites: number[];
  /** skills its events give: [skill group, level (1 white, 2 gold), story event link (0: not a chained event), chance the
   * event gives it] */
  ev: [number, number, number, number][];
  /** skills to prioritise in IT so the event choice the model credits is taken */
  pri?: string[];
  /** its chain-ending choices, one per story-event link that has one */
  end?: ChainEnd[];
  /** scenarios that do not allow it */
  ban?: string[];
}
/** A story event's choice that ends the card's chain (export_page.py `chain_ends`). IT takes it only when a skill on it
 * is prioritised. */
export interface ChainEnd {
  /** the story-event link */
  link: number;
  /** skills on the ending choice: [skill group, level (1 white, 2 gold)] */
  end: [number, number][];
  /** skills on the event's other choices, the same way */
  go: [number, number][];
}
export interface Outfit {
  /** outfit (card) id: character id * 100 + 1, 2, ... */
  id: number;
  label: string;
  /** release time, Unix seconds */
  rel: number;
}
export interface Uma {
  /** character id */
  id: number;
  name: string;
  /** GameTora's slug for her page */
  gt: string | null;
  /** her base aptitude grades (the first outfit's), when known */
  apt?: Grades;
  outfits: Outfit[];
}
/** a G1's distance and surface, from the career tables' `race_cat` and `race_ground` (the solver's codes) */
export interface RaceTrack {
  cat: Distance;
  ground: Surface;
}
// the solver's codes, checked on a value read from the data
const isDistance = (x: unknown): x is Distance => typeof x === 'number' && Distance[x] !== undefined,
  isSurface = (x: unknown): x is Surface => typeof x === 'number' && Surface[x] !== undefined;
// each G1's track by name. export_page.py stops on a race without a distance and a ground (`check_race_tracks`); the check here
// repeats it for a data file the build did not make (stale or hand-edited), and fails the data load (the status line
// says so) rather than give a race a wrong figure
function tracksOf(career: CareerTables | null): Map<string, RaceTrack> {
  const cats = career?.race_cat;
  if (!career || !cats) return new Map();
  const grounds = must(career.race_ground, 'export_page.py writes race_ground with race_cat');
  return new Map(
    career.races.map((name, ix): [string, RaceTrack] => {
      const cat = cats[ix],
        ground = grounds[ix];
      assert(isDistance(cat) && isSurface(ground), `export_page.py checks race ${ix}'s distance and ground`);
      return [name, { cat, ground }];
    }),
  );
}
/** a skill group's white name and its gold's (null: none) */
export type SkillNames = { w: string; gd: string | null };
/** The career page's tables (export_page.py `career`, its mobs cut to those with a portrait by the build; the planner
 * page has them only in the GM era). */
export interface CareerTables {
  /** a white spark's factor group -> its skill group */
  factors: Record<number, number>;
  /** a saddle (a won title) -> its G1 race's index; titles of several races have none */
  saddles: Record<number, number>;
  /** an aptitude spark's group -> its name */
  pinks: Record<number, string>;
  /** a stat spark's group -> its name */
  blues?: Record<number, string>;
  /** an outfit with a unique skill -> the skill's name */
  uniques?: Record<number, string>;
  /** the G1 races by index */
  races: string[];
  /** each race's distance (the solver's `Distance`, checked at load: `tracksOf`) */
  race_cat?: number[];
  /** each race's ground (the solver's `Surface`, checked at load) */
  race_ground?: number[];
  /** each race's turns */
  race_turns?: number[][];
  /** the sample veterans: character id, name */
  mobs?: [number, string][];
  /** the skill groups a card hints or gives */
  sourced?: number[];
  /** the rank scores at which each badge starts */
  ranks?: number[];
  /** GM era: the factors Global has (uma.moe lists only those) */
  moe?: number[];
}
/** The active era's data, loaded once by loadData() before the page starts (Global's comes with the page's scripts;
 * the GM era's is fetched, ASSETS.dataGm, only when that era is selected: switching era reloads the page). */
export interface GameData {
  /** the cards started by now */
  cards: Card[];
  /** the umas started by now (an uma counts from her first outfit's start) */
  umas: Uma[];
  /** the skill catalog: [skill group, white name, gold name or null] */
  catalog: [number, string, string | null][];
  /** every card in the data, those not started yet included */
  cardById: Map<number, Card>;
  /** skill group -> its names */
  skillNames: Map<number, SkillNames>;
  /** character id -> name (the career page's sample veterans included) */
  names: Map<number, string>;
  /** the career page's tables (null on the planner page in the Global era) */
  career: CareerTables | null;
  /** each G1's distance and ground, by its name in the career tables (none without them, or where they lack them) */
  tracks: Map<string, RaceTrack>;
  /** what the data holds ahead of its start (a build from a master.mdb fetched ahead of its update holds the next week's
   * umas, outfits and cards, pkg/loopopt/export/upcoming.py): hidden here and left out of every search until its
   * start, so a banner turns on by itself. Fixed at page load. */
  later: { umas: number[]; outfits: number[]; cards: number[] };
}
/** an era's page data (export_page.py) as the build gives it */
export interface PageData {
  cards: Card[];
  umas: Uma[];
  skills: GameData['catalog'];
  career?: CareerTables | null;
}
let loaded: GameData | null = null;
/** The active era's data (once loaded: Rust's OnceLock); read before loadData() has run, it throws. */
export function gameData(): GameData {
  if (!loaded) throw new Error('the game data was read before loadData() ran');
  return loaded;
}
/** The career tables, on a page that has them (the career page; the planner page in the GM era); elsewhere it throws. */
export function careerTables(): CareerTables {
  const { career } = gameData();
  if (!career) throw new Error("the career tables were read on a page that doesn't load them");
  return career;
}
function setData(data: PageData) {
  const now = Date.now() / 1000;
  const later: GameData['later'] = {
    umas: [],
    outfits: [],
    cards: data.cards.filter((c) => c.rel > now).map((c) => c.id),
  };
  const umas = data.umas.map((u) => {
    const laterOutfits = (u.outfits || []).filter((o) => o.rel > now);
    if (!laterOutfits.length) return u;
    later.outfits.push(...laterOutfits.map((o) => o.id));
    const outfits = u.outfits.filter((o) => o.rel <= now);
    if (!outfits.length) later.umas.push(u.id);
    return { ...u, outfits };
  });
  const career: CareerTables | null = data.career || null;
  const names = new Map(data.umas.map((u) => [u.id, u.name]));
  for (const [id, name] of career?.mobs || []) names.set(id, name); // the career page's sample (mob) veterans
  loaded = {
    cards: data.cards.filter((c) => c.rel <= now),
    umas: umas.filter((u) => !later.umas.includes(u.id)),
    catalog: data.skills,
    cardById: new Map(data.cards.map((c) => [c.id, c])),
    skillNames: new Map(data.skills.map(([group, white, gold]) => [group, { w: white, gd: gold }])),
    names,
    career,
    tracks: tracksOf(career),
    later,
  };
}
// a download that failed: for visitors a connection problem; opened as a file (the page must be served), the fix for that
export const loadFail = (what: string): string => {
  const asFile =
    location.protocol === 'file:' ? ' The page has to be served, not opened as a file: ./build.sh serve.' : '';
  return `Could not load ${what}. Check your connection and reload the page.${asFile}`;
};
/** Loads the active era's data; `career`: the Global era's career tables, on a page that has them (the career page's
 * '#career-data'), else null. */
export async function loadData(career: CareerTables | null) {
  if (ERA !== 'gm') {
    setData({ cards: page.cards, umas: page.umas, skills: page.skills, career });
    return;
  }
  let response: Response | null;
  try {
    response = await fetch(ASSETS.dataGm);
  } catch {
    response = null;
  }
  if (!response?.ok) throw new Error(loadFail('the Grand Masters-era data'));
  setData(await response.json());
}
const IMAGES = page.img;
export const SHIPPED = new Set(IMAGES.keys); // images in dist/<IMG_BASE>: 'uma:1001' | 'outfit:100102' | 'card:30116' | ...
export const IMG_BASE = IMAGES.base; // the fingerprinted image folder, e.g. 'assets/img.1a2b3c4d5e/'
export const IMG_EXT = IMAGES.ext; // file extension per kind: avif, webp for the UI sprites
export const SKILLICON = page.skillIcons.white,
  SKILLICON_GOLD = page.skillIcons.gold; // skill group -> icon id of its white / gold version
export const RACEIDS = page.raceIds; // race name -> race id (banner key)
// target golds a scenario hands out in every Independent Training run (Unity Cup: No Stopping Me!, It's On!)
export const SCEN_GOLDS: Record<string, number[]> = { unity: [20049, 20046] };
// each scenario's training-hint base by scenario key: research/results.json's fit rounded to 3 decimals, which the build
// reads with pkg/loopopt/build.rs's keys and rounding (vite/research.mjs), so it is the solver's hint_base
const HINT_BASE: Record<string, unknown> = page.hintBase;
// a scenario's hint base, checked where the page first reads it: the build writes one per scenario key (`hintBases`,
// which stops on a missing one). Read as this module loads, so a miss stops the page there (errorlog.ts says so on the
// status line)
const hintBase = (key: string): number => {
  const base = HINT_BASE[key];
  assert(typeof base === 'number', `the build writes ${key}'s hint base`);
  return base;
};
// the scenario picker's tiles, in its order. id: Global single_mode_scenario id (logo img/scen/{id}); base: training
// hints per card per run at hint frequency 0 and specialty 0 (HINT_BASE), shown as a multiple of URA Finale's (the
// original scenario); est: no measured base (Grand Masters: one base fitted over the other scenarios);
// tint: a light shade of the game's label colour (single_mode_scenario.label_bg_color) behind the name badge shown
// when the logo is missing; a tile also lists the scenario's guaranteed golds (SCEN_GOLDS).
// ss: picking the scenario sets the lineage rank to SS (else below SS), the band most of the scenario's careers reach
// with every skill point spent (docs/RESEARCH.md section 2, tested there; Grand Masters has no data and is assumed SS).
// era: offered only with that era's data (Grand Masters: the JP Grand Masters era).
/** A scenario of the picker (the fields as above). */
export interface Scenario {
  /** its key (state.scen) */
  v: string;
  id: number;
  name: string;
  base: number;
  ss: boolean;
  tint: string;
  est?: boolean;
  era?: 'gm';
  /** set below */
  mult?: string;
  /** set below */
  rate?: string;
}
export const SCENARIOS: readonly [Scenario, ...Scenario[]] = [
  { v: 'unity', id: 2, name: 'Unity Cup', base: hintBase('unity'), ss: false, tint: '#c5ecff' },
  { v: 'gc', id: 3, name: 'Grand Concert', base: hintBase('gc'), ss: true, tint: '#ebe3ff' },
  { v: 'ura', id: 1, name: 'URA Finale', base: hintBase('ura'), ss: false, tint: '#e6f4c9' },
  { v: 'trackblazer', id: 4, name: 'Trackblazer', base: hintBase('trackblazer'), ss: false, tint: '#faeebf' },
  {
    v: 'grandmasters',
    id: 5,
    name: 'Grand Masters',
    base: hintBase('grandmasters'),
    est: true,
    ss: true,
    era: 'gm',
    tint: '#f3e1d2',
  },
];
const URA_BASE = must(
  SCENARIOS.find((s) => s.v === 'ura'),
  'SCENARIOS lists URA',
).base;
// mult: "Hint rate ×1.76" (the tile's line, which adds "(estimate)" itself); rate: with "(estimate)" (tooltips, labels)
for (const scenario of SCENARIOS) {
  scenario.mult = `Hint rate ×${(scenario.base / URA_BASE).toFixed(2)}`;
  scenario.rate = scenario.mult + (scenario.est ? ' (estimate)' : '');
}
