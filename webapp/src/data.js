// The data build.py embeds in the page, for the chosen era.
import { $ } from './dom.js';
import { ERA } from './settings.js';
// the shared data comes from build.py's fingerprinted data scripts (self.__LLP_DATA, keyed 'data-cards', ...); what stays
// in the page (the asset table, the example) from its JSON elements of the same ids
const BUNDLED = self.__LLP_DATA || {};
const parse = (id) => id in BUNDLED ? BUNDLED[id] : JSON.parse($(id).textContent);
const has = (id) => id in BUNDLED || !!$(id);
// Fingerprinted files next to the page (build.py): the wasm, each era's solver data, the GM era's page data, the image folder.
export const ASSETS = parse('data-assets');
/**
 * A support card (build.py `ui_cards`).
 * @typedef {object} Card
 * @property {number} id
 * @property {string} name "[Title] Character"
 * @property {1 | 2 | 3} rar rarity: R, SR, SSR
 * @property {boolean} pal a friend (pal) or group card
 * @property {string} kind 'speed', 'stamina', ..., 'pal', 'group', 'other', or '' when unknown (drawn as 'other')
 * @property {number} rel release time, Unix seconds
 * @property {string | null} gt GameTora's url_name for its page
 * @property {number[]} whites skill groups it hints when training with it
 * @property {[number, number, number, number][]} ev skills its events give: [skill group, level (1 white, 2 gold), story
 *   event link (0: not a chained event), chance the event gives it]
 * @property {string[]} [pri] skills to prioritise in IT so the event choice the model credits is taken
 * @property {string[]} [ban] scenarios that do not allow it
 */
/**
 * @typedef {object} Outfit
 * @property {number} id outfit (card) id: character id * 100 + 1, 2, ...
 * @property {string} label
 * @property {number} rel release time, Unix seconds
 */
/**
 * @typedef {object} Uma
 * @property {number} id character id
 * @property {string} name
 * @property {string | null} gt GameTora's slug for her page
 * @property {number[]} [apt] her base aptitude grades (codes 1..8 = G..S, in the solver data's `apt` order), when known
 * @property {Outfit[]} outfits
 */
/** @typedef {{w: string, gd: string | null}} SkillNames a skill group's white name and its gold's (null: none) */
// The active era's data, set by loadData() before the page starts. Global's comes with the page's data scripts; the GM
// era's is fetched (ASSETS.dataGm) only when that era is selected (switching era reloads the page).
/** @type {Card[]} */
export let CARDS;
/** @type {Uma[]} */
export let UMAS;
/** @type {[number, string, string | null][]} the skill catalog: [skill group, white name, gold name or null] */
export let CAT;
/** @type {Map<number, Card>} every card in the data, those not started yet included */
export let CARD_BY_ID;
/** @type {Map<number, SkillNames>} skill group -> its names */
export let SK;
/** @type {Map<number, string>} character id -> name (the career page's sample veterans included) */
export let NAME;
// the career page's tables (build.py `career`: factors, saddles, races, mobs, ...); null on the planner page in the Global era
export let CAREER = null;
// what the data holds ahead of its start (a build from a master.mdb fetched ahead of its update holds the next week's
// umas, outfits and cards, pkg/loopopt/export/upcoming.py): hidden here and left out of every search until its start,
// so a banner turns on by itself. An uma counts from her first outfit's start. Fixed at page load.
export let LATER = { umas: [], outfits: [], cards: [] };
function setData(data) {
  const now = Date.now() / 1000;
  LATER = { umas: [], outfits: [], cards: data.cards.filter(c => c.rel > now).map(c => c.id) };
  const umas = data.umas.map(u => {
    const later = (u.outfits || []).filter(o => o.rel > now);
    if (!later.length) return u;
    LATER.outfits.push(...later.map(o => o.id));
    const outfits = u.outfits.filter(o => o.rel <= now);
    if (!outfits.length) LATER.umas.push(u.id);
    return { ...u, outfits };
  });
  CARDS = data.cards.filter(c => c.rel <= now); UMAS = umas.filter(u => !LATER.umas.includes(u.id));
  CAT = data.skills; CAREER = data.career || null;
  CARD_BY_ID = new Map(data.cards.map(c => [c.id, c]));
  SK = new Map(CAT.map(([group, white, gold]) => [group, { w: white, gd: gold }]));
  NAME = new Map(data.umas.map(u => [u.id, u.name]));
  for (const [id, name] of CAREER?.mobs || []) NAME.set(id, name);   // the career page's sample (mob) veterans
}
// a download that failed: for visitors a connection problem; opened as a file (the page must be served), the fix for that
export const loadFail = (what) => {
  const asFile = location.protocol === 'file:' ? ' The page has to be served, not opened as a file: ./build.sh serve.' : '';
  return `Could not load ${what}. Check your connection and reload the page.${asFile}`;
};
export async function loadData() {
  if (ERA !== 'gm') {
    setData({ cards: parse('data-cards'), umas: parse('data-umas'), skills: parse('data-skills'), career: has('data-career') ? parse('data-career') : null });
    return;
  }
  let response;
  try { response = await fetch(ASSETS.dataGm); } catch { response = null; }
  if (!response?.ok) throw new Error(loadFail('the Grand Masters-era data'));
  setData(await response.json());
}
export const EXAMPLE = $('data-example') ? parse('data-example') : null;       // precomputed default result (the planner page)
const IMAGES = parse('data-img');
export const SHIPPED = new Set(IMAGES.keys);   // images in dist/<IMG_BASE>: 'uma:1001' | 'outfit:100102' | 'card:30116' | ...
export const IMG_BASE = IMAGES.base;           // the fingerprinted image folder, e.g. 'assets/img.1a2b3c4d5e/'
export const IMG_EXT = IMAGES.ext;             // file extension per kind: avif, webp for the UI sprites
const SKILL_ICONS = parse('data-skillicon');
export const SKILLICON = SKILL_ICONS.white, SKILLICON_GOLD = SKILL_ICONS.gold;   // skill group -> icon id of its white / gold version
export const RACEIDS = parse('data-raceids');       // race name -> race id (banner key)
// target golds a scenario hands out in every Independent Training run (Unity Cup: No Stopping Me!, It's On!)
export const SCEN_GOLDS = { unity: [20049, 20046] };
// the scenario picker's tiles, in its order. id: Global single_mode_scenario id (logo img/scen/{id}); base: training
// hints per card per run at hint frequency 0 and specialty 0 (the solver's hint_base: research/results.json's fit,
// rounded to 3 decimals by pkg/loopopt/build.rs; the browser suite checks it equals that), shown as a multiple of URA
// Finale's (the original scenario); est: no measured base (Grand Masters: one base fitted over the other scenarios);
// tint: a light shade of the game's label colour (single_mode_scenario.label_bg_color) behind the name badge shown
// when the logo is missing; a tile also lists the scenario's guaranteed golds (SCEN_GOLDS).
// ss: picking the scenario sets the lineage rank to SS (else below SS), the band most of the scenario's careers reach
// with every skill point spent (docs/RESEARCH.md section 2, tested there; Grand Masters has no data and is assumed SS).
// era: offered only with that era's data (Grand Masters: the JP Grand Masters era).
/**
 * A scenario of the picker (the fields as above).
 * @typedef {object} Scenario
 * @property {string} v  its key (state.scen)
 * @property {number} id
 * @property {string} name
 * @property {number} base
 * @property {boolean} ss
 * @property {string} tint
 * @property {boolean} [est]
 * @property {'gm'} [era]
 * @property {string} [mult]  set below
 * @property {string} [rate]  set below
 */
/** @type {Scenario[]} */
export const SCENARIOS = [
  { v: 'unity', id: 2, name: 'Unity Cup', base: 2.395, ss: false, tint: '#c5ecff' },
  { v: 'gc', id: 3, name: 'Grand Concert', base: 1.999, ss: true, tint: '#ebe3ff' },
  { v: 'ura', id: 1, name: 'URA Finale', base: 1.380, ss: false, tint: '#e6f4c9' },
  { v: 'trackblazer', id: 4, name: 'Trackblazer', base: 1.702, ss: false, tint: '#faeebf' },
  { v: 'grandmasters', id: 5, name: 'Grand Masters', base: 1.991, est: true, ss: true, era: 'gm', tint: '#f3e1d2' },
];
// lineage rank: the star mix of the legacies' sparks (state.rank), below SS (rank score 6,500-17,499) or SS (17,500+)
export const RANKS = { below: '0.50,0.45,0.05', ss: '0.20,0.70,0.10' };
const URA_BASE = SCENARIOS.find(s => s.v === 'ura').base;
// mult: "Hint rate ×1.76" (the tile's line, which adds "(estimate)" itself); rate: with "(estimate)" (tooltips, labels)
for (const scenario of SCENARIOS) {
  scenario.mult = `Hint rate ×${(scenario.base / URA_BASE).toFixed(2)}`;
  scenario.rate = scenario.mult + (scenario.est ? ' (estimate)' : '');
}
// the 8/8 example: the default eight targets, as skill groups (Unity Cup's two guaranteed golds among them: state.js goldGiven)
export const PRESET = [20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203];
