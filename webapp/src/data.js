// The data build.py embeds in the page, for the chosen era.
import { $ } from './dom.js';
import { ERA } from './settings.js';
const parse = (id) => JSON.parse($(id).textContent);
// The active era's data, set by loadData() before the page starts. Global's is inline in the page; the GM era's is
// fetched (data-gm.json) only when that era is selected (switching era reloads the page).
export let CARDS;   // [{id,name,rar,pal,links,hf,whites,ev:[[group,level,link,p]]}]
export let UMAS;    // [{id,name,outfits:[{id,label,rel}]}]
export let HINTS;   // {trainee:[[id,group,p]], deck:[[card,group,level,p]]}
export let CAT;     // [[group, white, gold|null]]
export let CARD_BY_ID, SK, NAME;
function setData(d) {
  CARDS = d.cards; UMAS = d.umas; HINTS = d.hints; CAT = d.skills;
  CARD_BY_ID = new Map(CARDS.map(c => [c.id, c]));
  SK = new Map(CAT.map(([g, w, gd]) => [g, { w, gd }]));
  NAME = new Map(UMAS.map(u => [u.id, u.name]));
}
export async function loadData() {
  if (ERA !== 'gm') { setData({ cards: parse('data-cards'), umas: parse('data-umas'), hints: parse('data-hints'), skills: parse('data-skills') }); return; }
  let r; try { r = await fetch('data-gm.json'); } catch { r = null; }
  if (!r || !r.ok) throw new Error('Could not load data-gm.json (the Grand Masters-era data). Open the page through a web server (./build.sh serve).');
  setData(await r.json());
}
export const EXAMPLE = parse('data-example');       // precomputed default result
export const SHIPPED = new Set(parse('data-img'));  // images in dist/img: 'uma:1001' | 'outfit:100102' | 'card:30116' | ...
export const SKILLICON = parse('data-skillicon');   // skill group -> icon id
export const RACEIDS = parse('data-raceids');       // race name -> race id (banner key)
export const PRESET = [
  { group: 20159, guaranteed: 0 }, { group: 20049, guaranteed: 2 }, { group: 20166, guaranteed: 0 }, { group: 20046, guaranteed: 2 },
  { group: 20161, guaranteed: 0 }, { group: 21010, guaranteed: 0 }, { group: 20165, guaranteed: 0 }, { group: 20203, guaranteed: 0 },
];
