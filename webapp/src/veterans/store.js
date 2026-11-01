// ---------- the career page's veterans and lineage plan: the records, their storage, the edits to the list ----------
/**
 * A spark map: group -> stars (1-3). Keys are object keys, so strings once read back with Object.keys / entries.
 * @typedef {Object<number, number>} Sparks
 *
 * One uma of a lineage: a veteran's parent, or the fields a veteran shares with one.
 * @typedef {object} LineageNode
 * @property {number|null} chara  her character id (null: not known yet)
 * @property {number} [outfit]  her outfit (card) id, when known
 * @property {Sparks} sparks  white sparks: skill group -> stars
 * @property {Sparks} pinks  pink sparks: aptitude group -> stars
 * @property {Sparks} [blues]  blue sparks: stat group (1 Speed .. 5 Wit) -> stars
 * @property {Sparks} [greens]  unique spark: outfit card id -> stars
 * @property {number[]} wins  G1 wins, as race indices into careerTables().races
 * @property {Object<number, number>} [winAt]  race index -> the turn its win shows at (display only; the solver reads `wins`)
 *
 * A veteran of the pool: a lineage node with an identity, her source and her two parents.
 * @typedef {LineageNode & {id: string, src: string, borrowed: boolean, parents: LineageNode[], lender?: Lender, rank?: number}} Veteran
 *   src: where she came from: 'import' | 'tree' (uma.moe's lineage planner) | 'umamoe' (uma.moe's search) | 'hand' |
 *   'sample' | 'entered' (older saves: 'simulated')
 *
 *   rank: her rank score (her career-rank badge), when the import or uma.moe gives it
 *
 * The trainer who lists a veteran found on uma.moe (umadump.js `parseListing`): kept on this device, left out of Export.
 * @typedef {{name: string, id: string, followers: number, updated: string, affinity?: number, wins?: number,
 *   sparkStars?: number}} Lender
 *
 * The places of the next career: each is chosen here or left to the planner (null).
 * @typedef {object} LineagePlan
 * @property {number|null} trainee  the trainee's character id
 * @property {(string|null)[]} parents  the two parent places, by veteran id
 * @property {boolean} [fromTable]  the places came from the other careers' table (Use this): a change of setup hands them
 *   back to the planner
 */
// Saved per data era in localStorage['llp:career'] = {v: 2, global: {vets, plan}, gm: {vets, plan}}.
import { signal } from '@preact/signals-core';
import { ERA } from '../saved.ts';
import { careerTables } from '../data.js';
import { settings } from '../settings.ts';
import { goldGiven } from '../job.js';
import { srcFor } from '../images.js';

const STORAGE_KEY = 'llp:career',
  STORAGE_VERSION = 2;
// stars of a skill added by hand: 2, the most common star count of a white spark at SS rank (70% at rank score 17,500+,
// 45% below it; hakuraku.moe)
export const NEW_STARS = 2;
function loadAll() {
  try {
    const saved = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null'); // (nothing stored yet: null)
    if (saved && saved.v === STORAGE_VERSION) return saved;
    if (saved && saved.v === 1) {
      // v1: a list of veterans per era
      const upgraded = { v: STORAGE_VERSION };
      for (const era of ['global', 'gm']) if (Array.isArray(saved[era])) upgraded[era] = { vets: saved[era] };
      return upgraded;
    }
  } catch {
    // storage blocked or the veterans unreadable: none
  }
  return { v: STORAGE_VERSION };
}
const MINE = loadAll()[ERA] || {};
/** a record the page can show and the solver can read: an id and two parent nodes (a damaged one is left out)
 * @param {any} v @returns {boolean} */
export const isVeteran = (v) =>
  !!v &&
  typeof v.id === 'string' &&
  Array.isArray(v.parents) &&
  v.parents.length === 2 &&
  v.parents.every((p) => !!p && typeof p === 'object');
const isMap = (x) => !!x && typeof x === 'object' && !Array.isArray(x);
/** a stored or pasted lineage node with the fields the page draws, defaulted when missing or of the wrong kind (an
 * older save, a hand-edited or cut export)
 * @param {any} n @returns {LineageNode} */
const asNode = (n) => ({
  ...n,
  chara: Number.isInteger(n.chara) ? n.chara : null,
  sparks: isMap(n.sparks) ? n.sparks : {},
  pinks: isMap(n.pinks) ? n.pinks : {},
  wins: Array.isArray(n.wins) ? n.wins : [],
});
/** @param {any} v @returns {Veteran} */
export const asVeteran = (v) => ({ ...v, ...asNode(v), parents: v.parents.map(asNode) });
/** @type {Veteran[]} every veteran of this era; a veteran's number on the page is her index here plus one */
export const vets = (MINE.vets || []).filter(isVeteran).map(asVeteran);
/** @type {LineagePlan} */
export const plan = MINE.plan || { trainee: null, parents: [null, null] };
/** @typedef {{places: string, id: string, setup: string}} ChosenCareer */
/** @type {(c: any) => boolean} */
const isChosen = (c) => !!c && typeof c.places === 'string' && typeof c.id === 'string' && typeof c.setup === 'string';
// the career Use this chose with its deck (career/job.js `chosenSetup`, `chosenApplies`), saved with the plan: the page checks
// it against the setup it meets before it sends it (a damaged one is left out)
/** @type {{career: ChosenCareer|null}} */
export const chosen = { career: isChosen(MINE.chosen) ? MINE.chosen : null };
// Saving: `persist` writes now; an edit's `persistSoon` writes once after the next frame (edits in between share it), and
// a write still due goes out when the page is hidden or left (a reload or close loses nothing). `rev` counts the edits
// (each `persistSoon`, and a change of uma before its skills prompt answers): `legaciesJob` is rebuilt only after one.
let persistDue = false,
  rev = 0;
export function persist() {
  persistDue = false;
  // (uma.moe's listings come back from its kept answer; one chosen as a parent is saved, so the place keeps her). Only
  // this era is written over what is stored now: another tab's write to the other era survives this tab's.
  const all = loadAll();
  all[ERA] = {
    vets: vets.filter((v) => v.src !== 'umamoe' || plan.parents.includes(v.id)),
    plan,
    chosen: chosen.career,
  };
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(all));
  } catch {
    // storage blocked or full: the edit lasts for this visit only
  }
}
export function persistSoon() {
  rev++;
  if (document.visibilityState === 'hidden') {
    persist();
    return;
  }
  if (persistDue) return;
  persistDue = true;
  requestAnimationFrame(() =>
    setTimeout(() => {
      if (persistDue) persist();
    }),
  );
}
const flushPersist = () => {
  if (persistDue) persist();
};
addEventListener('pagehide', flushPersist);
document.addEventListener('visibilitychange', () => {
  if (document.visibilityState === 'hidden') flushPersist();
});

let onChange = () => {
  // replaced by the career page's onVetsChange
};
export function onVetsChange(fn) {
  onChange = fn;
}
// the views that draw the veterans and the places read this count: every change (`changed`) moves it, as the records
// themselves are edited in place (copy-on-write in step 10)
const edits = signal(0);
export const vetsRev = /** @type {import('@preact/signals-core').ReadonlySignal<number>} */ (edits);
// a change of setup with places taken from the other careers' table: they go back to the planner (saved, and drawn)
export function handBackPlaces() {
  plan.trainee = null;
  plan.parents = [null, null];
  plan.fromTable = false;
  chosen.career = null;
  persist();
  edits.value++;
}
// every edit: saved, then the views redraw and the page searches again
export function changed() {
  persistSoon();
  edits.value++;
  onChange();
}
// a change of uma before its skills prompt answers (pickers.js `changeUma`): the job between takes her new uma
export function countEdit() {
  rev++;
}
export const targetGroups = () => settings.value.targets.map((t) => t.group);
/** @type {(id: string) => Veteran|undefined} */
export const vetById = (id) => vets.find((v) => v.id === id);
// a new veteran by hand for a parent place: she gets `sparks`, and so do her parents (a looped parent's own parents carry
// the same sparks), whose umas are not known yet
/** @param {number|null} chara @param {Sparks} sparks @returns {Veteran} */
export function newVet(chara, sparks) {
  const v = {
    id: `h${Date.now().toString(36)}${Math.random().toString(36).slice(2, 5)}`,
    src: 'hand',
    chara,
    borrowed: false,
    sparks,
    pinks: {},
    blues: {},
    greens: {},
    wins: [],
    parents: [0, 1].map(() => ({ chara: null, sparks: { ...sparks }, pinks: {}, blues: {}, greens: {}, wins: [] })),
  };
  append(v);
  return v;
}
// a veteran joins the pool before uma.moe's listings, which a new answer replaces at the end, so her number keeps
/** @param {...Veteran} list */
function append(...list) {
  const at = vets.findIndex((v) => v.src === 'umamoe');
  vets.splice(at < 0 ? vets.length : at, 0, ...list);
}
// an import never loses one of your veterans: yours merge by id (a veteran already here takes the file's version in
// her place, new ones go after your other imports), while borrowed ones replace the earlier borrow list (it is who
// lends today), after all of yours so your numbers keep. `all` (an Export of this page, holding both kinds): every
// earlier import goes. Your own veterans arriving retire the sample ones (they stood in until you had some). Hand-made
// ones stay (a plan slot holding a removed one goes back to the planner).
// Returns { added, updated } for yours and { borrowed, dropped } for the borrow list.
/** @param {Veteran[]} list @param {boolean} [all] @returns {{added: number, updated: number, borrowed: number, dropped: number}} */
export function importVets(list, all = false) {
  const isImport = (v) => v.src === 'import';
  // the borrow list's veterans by their id (umadump.js: a lender's viewer id after "tr"), not by the Yours/Borrowed
  // switch, which the player may have flipped
  const lent = (v) => isImport(v) && v.id.startsWith('tr');
  if (all) {
    for (let i = vets.length - 1; i >= 0; i--) if (isImport(vets[i])) vets.splice(i, 1);
  }
  const own = list.filter((v) => !v.borrowed),
    borrowed = list.filter((v) => v.borrowed),
    fresh = [];
  if (own.length) for (let i = vets.length - 1; i >= 0; i--) if (vets[i].src === 'sample') vets.splice(i, 1);
  let updated = 0,
    dropped = 0;
  // a veteran already here takes the file's version in her place, on the tab the player keeps her on
  const update = (old, v) => Object.assign(old, v, { borrowed: old.borrowed });
  for (const v of own) {
    const old = vetById(v.id);
    if (old) {
      update(old, v);
      updated++;
    } else fresh.push(v);
  }
  const afterOwnImports = vets.reduce((at, v, i) => (isImport(v) && !v.borrowed ? i + 1 : at), 0);
  vets.splice(afterOwnImports, 0, ...fresh);
  if (borrowed.length) {
    // the earlier borrow list goes, but not one the player moved to Yours
    for (let i = vets.length - 1; i >= 0; i--)
      if (lent(vets[i]) && vets[i].borrowed) {
        vets.splice(i, 1);
        dropped++;
      }
    for (const v of borrowed) {
      const old = vetById(v.id);
      if (old) update(old, v);
      else append(v);
    }
  }
  dropMissing();
  changed();
  return { added: fresh.length, updated, borrowed: borrowed.length, dropped };
}
// sample veterans to try the planner with (no veterans yet): 18 mob umas drawn at random from the game's (those with a
// portrait), six veterans with two mob parents each, the bare minimum to start a loop: each veteran carries, at 1 star,
// only the targets nothing but inheritance gives (no card hints or events them, no outfit has them, the scenario
// doesn't hand them out); her parents carry nothing. No relation points with anyone (the solver knows no mob), no G1
// wins. Marked as samples; Remove all clears them.
/** @returns {Veteran[]} */
export function sampleVets() {
  const all = careerTables().mobs || [],
    withArt = all.filter(([id]) => srcFor('mob', id)),
    pool = (withArt.length >= 18 ? withArt : all).slice();
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [pool[i], pool[j]] = [pool[j], pool[i]];
  }
  const sourced = new Set(careerTables().sourced || []),
    inherit = targetGroups().filter((g) => !sourced.has(g) && !goldGiven(settings.value.scen, g));
  const node = (id, sparks) => ({ chara: id, sparks, pinks: {}, blues: {}, greens: {}, wins: [] });
  const out = [];
  for (let i = 0; i + 2 < pool.length && out.length < 6; i += 3)
    out.push({
      id: `m${pool[i][0]}${Date.now().toString(36)}`,
      src: 'sample',
      borrowed: false,
      ...node(pool[i][0], Object.fromEntries(inherit.map((g) => [g, 1]))),
      parents: [node(pool[i + 1][0], {}), node(pool[i + 2][0], {})],
    });
  return out;
}
// uma.moe's listings for the targets (umadump.js `parseListing`): borrowed veterans the planner weighs like any other,
// after every other veteran. They replace the earlier listings, except one chosen as a parent (she stays while chosen,
// taking her new record when listed again). Not saved unless chosen
/** @param {Veteran[]} list */
export function setListings(list) {
  const fresh = new Map(list.map((v) => [v.id, v]));
  for (let i = vets.length - 1; i >= 0; i--) {
    const v = vets[i];
    if (v.src !== 'umamoe') continue;
    const again = fresh.get(v.id);
    if (plan.parents.includes(v.id)) {
      if (again) Object.assign(v, again);
      fresh.delete(v.id);
    } else vets.splice(i, 1);
  }
  vets.push(...fresh.values());
  dropMissing();
  changed();
}
// veterans from an uma.moe tree, or the samples: added after the others (the numbers you have keep)
/** @param {Veteran[]} list */
export function addVets(list) {
  append(...list);
  changed();
}
/** @param {number} i her index in `vets` */
export function removeVet(i) {
  vets.splice(i, 1);
  dropMissing();
  changed();
}
// every veteran, or those `drop` picks
/** @param {(vet: Veteran) => boolean} [drop] */
export function clearVets(drop = () => true) {
  for (let i = vets.length - 1; i >= 0; i--) if (drop(vets[i])) vets.splice(i, 1);
  dropMissing();
  changed();
}
// a parent place naming a veteran no longer here (removed; an uma.moe listing not back from its answer) goes back to the
// planner
export function dropMissing() {
  plan.parents = plan.parents.map((id) => (id && vetById(id) ? id : null));
}
// legaciesJob returns the same array until the next edit (`rev`): callers don't change it
/** @type {JobLegacy[] | null} */
let legaciesCache = null,
  legaciesRev = -1;
/**
 * A veteran as the solver's job takes her (sparks as [group, stars] pairs; an unknown parent's character 0: no relation
 * points).
 * @typedef {{chara: number, sparks: [number, number][], pinks: [number, number][], wins: number[]}} JobNode
 * @typedef {JobNode & {borrowed: boolean, parents: JobNode[]}} JobLegacy
 * @returns {JobLegacy[]}
 */
export function legaciesJob() {
  if (legaciesCache && legaciesRev === rev) return legaciesCache;
  legaciesRev = rev;
  legaciesCache = vets.map(jobLegacy);
  return legaciesCache;
}
/** @type {(sparks: Sparks|undefined) => [number, number][]} */
const pairs = (sparks) => Object.entries(sparks || {}).map(([group, stars]) => [+group, stars]);
/** @type {(n: LineageNode) => JobNode} */
const jobNode = (n) => ({ chara: n.chara || 0, sparks: pairs(n.sparks), pinks: pairs(n.pinks), wins: n.wins });
// one veteran as the job takes her
/** @param {Veteran} v @returns {JobLegacy} */
const jobLegacy = (v) => ({ ...jobNode(v), borrowed: !!v.borrowed, parents: v.parents.map(jobNode) });
