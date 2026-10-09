// The data build.py embeds in the page, for the chosen era.
import { $ } from './dom.js';
import { ERA } from './settings.js';
// the shared data comes from build.py's fingerprinted data scripts (self.__LLP_DATA, keyed by the ids of the elements that
// used to hold it); what stays in the page (the asset table, the example) from its JSON elements
const D = self.__LLP_DATA || {};
const parse = (id) => id in D ? D[id] : JSON.parse($(id).textContent);
const has = (id) => id in D || !!$(id);
// Fingerprinted files next to the page (build.py): the wasm, each era's solver data, the GM era's page data, the image folder.
export const ASSETS = parse('data-assets');
// The active era's data, set by loadData() before the page starts. Global's is inline in the page; the GM era's is
// fetched (ASSETS.dataGm) only when that era is selected (switching era reloads the page).
export let CARDS;   // [{id,name,rar,pal,kind,rel,gt,whites,ev:[[group,level,link,p]],ban?:[scenario]}]
export let UMAS;    // [{id,name,outfits:[{id,label,rel}]}]
export let CAT;     // [[group, white, gold|null]]
export let CARD_BY_ID, SK, NAME;
// the career page's veteran import tables ({factors, saddles, races}); null on the planner page
export let CAREER = null;
function setData(d) {
  CARDS = d.cards; UMAS = d.umas; CAT = d.skills; CAREER = d.career || null;
  CARD_BY_ID = new Map(CARDS.map(c => [c.id, c]));
  SK = new Map(CAT.map(([g, w, gd]) => [g, { w, gd }]));
  NAME = new Map(UMAS.map(u => [u.id, u.name]));
  for (const [id, name] of (CAREER && CAREER.mobs) || []) NAME.set(id, name);   // the career page's sample (mob) veterans
}
// a download that failed: for visitors a connection problem; opened as a file (the page must be served), the fix for that
export const loadFail = (what) => `Could not load ${what}. Check your connection and reload the page.` + (location.protocol === 'file:' ? ' The page has to be served, not opened as a file: ./build.sh serve.' : '');
export async function loadData() {
  if (ERA !== 'gm') { setData({ cards: parse('data-cards'), umas: parse('data-umas'), skills: parse('data-skills'), career: has('data-career') ? parse('data-career') : null }); return; }
  let r; try { r = await fetch(ASSETS.dataGm); } catch { r = null; }
  if (!r || !r.ok) throw new Error(loadFail('the Grand Masters-era data'));
  setData(await r.json());
}
export const EXAMPLE = $('data-example') ? parse('data-example') : null;       // precomputed default result (the planner page)
const IMG = parse('data-img');
export const SHIPPED = new Set(IMG.keys);   // images in dist/<IMG_BASE>: 'uma:1001' | 'outfit:100102' | 'card:30116' | ...
export const IMG_BASE = IMG.base;           // the fingerprinted image folder, e.g. 'assets/img.1a2b3c4d5e/'
export const IMG_EXT = IMG.ext;             // file extension per kind: avif, webp for the UI sprites
const SKI = parse('data-skillicon');
export const SKILLICON = SKI.white, SKILLICON_GOLD = SKI.gold;   // skill group -> icon id of its white / gold version
export const RACEIDS = parse('data-raceids');       // race name -> race id (banner key)
// target golds a scenario hands out in every Independent Training run (Unity Cup: No Stopping Me!, It's On!)
export const SCEN_GOLDS = { unity: [20049, 20046] };
// the scenario picker's tiles, in its order. id: Global single_mode_scenario id (logo img/scen/{id}); base: training
// hints per card per run at hint frequency 0 (pkg/loopopt/src/deck.rs hint_base), shown as a multiple of URA Finale's
// (the original scenario); est: no measured base (Grand Masters: a placeholder); tint: a light shade of the game's
// label colour (single_mode_scenario.label_bg_color) behind the name badge shown when the logo is missing; a tile also
// lists the scenario's guaranteed golds (SCEN_GOLDS). ss: picking the scenario sets the lineage rank to SS (else below
// SS): the share of uma-it runs whose rank score reaches 17,500 with every skill point spent is URA Finale 0%, Unity Cup
// 47% (median 17,428), Grand Concert 79% (median 18,094), Trackblazer 0% (median 12,386); Grand Masters has no data
// (assumed SS). era: offered only with that era's data (Grand Masters: the JP Grand Masters era).
export const SCENARIOS = [
  { v: 'unity', id: 2, name: 'Unity Cup', base: 2.70, ss: false, tint: '#c5ecff' },
  { v: 'gc', id: 3, name: 'Grand Concert', base: 2.03, ss: true, tint: '#ebe3ff' },
  { v: 'ura', id: 1, name: 'URA Finale', base: 1.60, ss: false, tint: '#e6f4c9' },
  { v: 'trackblazer', id: 4, name: 'Trackblazer', base: 1.81, ss: false, tint: '#faeebf' },
  { v: 'grandmasters', id: 5, name: 'Grand Masters', base: 2.30, est: true, ss: true, era: 'gm', tint: '#f3e1d2' },
];
// lineage rank: the star mix of the legacies' sparks (state.rank), below SS (rank score 6,500-17,499) or SS (17,500+)
export const RANKS = { below: '0.50,0.45,0.05', ss: '0.20,0.70,0.10' };
const URA_BASE = SCENARIOS.find(s => s.v === 'ura').base;
// mult: "Hint rate ×1.69" (the tile's line, which adds "(estimate)" itself); rate: with "(estimate)" (tooltips, labels)
for (const s of SCENARIOS) { s.mult = `Hint rate ×${(s.base / URA_BASE).toFixed(2)}`; s.rate = s.mult + (s.est ? ' (estimate)' : ''); }
// the 8/8 example: the default eight targets, as skill groups (what the scenario guarantees: state.js goldGiven)
export const PRESET = [20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203];
