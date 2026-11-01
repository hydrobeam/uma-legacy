// ---------- the career page's veterans and the next career's places: one record in a signal ----------
// `vets`, `places` and `chosen` hand the record's parts out read-only (each a computed: a write that leaves a part alone
// leaves its identity, and its readers, alone). Every write is a mutator here that builds the next record copy-on-write
// (only what it changes is new: an edited veteran is a new record, the others stay the same objects), saves it, then
// assigns it: effects run inside the assignment, so one that throws cannot lose the write. A record is frozen as it
// comes in. An editor addresses a node by where it is (`NodeRef`: a veteran by id, or her parent j), never by an object
// it holds: an open dialog's second edit then lands on the record the first one made.
// Saved per data era in localStorage['llp:career'] = {v: 2, global: {vets, plan, chosen}, gm: {...}}: an edit is saved
// once after the next frame (edits in between share it), at once while the page is hidden, and a save still due goes
// out when the page is hidden or left (the entry listens: `flushPersist`).
import { computed, signal } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';
import { ERA } from '../saved.ts';
import type { DeepReadonly } from '../store.ts';

/** a spark map: group -> stars (1-3); keys are object keys, so strings once read back with Object.keys / entries */
export type Sparks = Record<number, number>;
/** One uma of a lineage: a veteran's parent, or the fields a veteran shares with one. */
export interface LineageNode {
  /** her character id (null: not known yet) */
  chara: number | null;
  /** her outfit (card) id, when known */
  outfit?: number;
  /** white sparks: skill group -> stars */
  sparks: Sparks;
  /** pink sparks: aptitude group -> stars */
  pinks: Sparks;
  /** blue sparks: stat group (1 Speed .. 5 Wit) -> stars */
  blues?: Sparks;
  /** unique spark: outfit card id -> stars */
  greens?: Sparks;
  /** G1 wins, as race indices into careerTables().races */
  wins: number[];
  /** race index -> the turn its win shows at (display only; the solver reads `wins`) */
  winAt?: Record<number, number>;
  /** her end-of-career aptitude grades (the veteran analysis' draft) */
  apt?: number[] | null;
}
/** The trainer who lists a veteran found on uma.moe (umadump.js `parseListing`): kept on this device, left out of
 *  Export. */
export interface Lender {
  name: string;
  id: string;
  followers: number;
  updated: string;
  affinity?: number;
  wins?: number;
  sparkStars?: number;
}
/** A veteran of the pool: a lineage node with an identity, her source and her two parents. */
export interface Veteran extends LineageNode {
  id: string;
  /** where she came from: 'import' | 'tree' (uma.moe's lineage planner) | 'umamoe' (uma.moe's search) | 'hand' |
   *  'sample' | 'entered' (older saves: 'simulated') */
  src: string;
  borrowed: boolean;
  parents: LineageNode[];
  lender?: Lender;
  /** her rank score (her career-rank badge), when the import or uma.moe gives it */
  rank?: number;
}
/** The places of the next career: each is chosen here or left to the planner (null). */
export interface LineagePlan {
  /** the trainee's character id */
  trainee: number | null;
  /** the two parent places, by veteran id */
  parents: (string | null)[];
  /** the places came from the other careers' table (Use this): a change of setup hands them back to the planner */
  fromTable?: boolean;
}
/** the career Use this chose with its deck (career/job.js `chosenSetup`, `chosenApplies`), saved with the places: the
 *  page checks it against the setup it meets before it sends it */
export interface ChosenCareer {
  places: string;
  id: string;
  setup: string;
}
/** the records as the store hands them out */
export type NodeRecord = DeepReadonly<LineageNode>;
export type VetRecord = DeepReadonly<Veteran>;
export type PlacesRecord = DeepReadonly<LineagePlan>;
interface VetsState {
  vets: readonly VetRecord[];
  plan: PlacesRecord;
  chosen: DeepReadonly<ChosenCareer> | null;
}
/** where the store saves, and when (the page's: localStorage, after the next frame; a test passes its own) */
export interface VetsHost {
  storage: () => Pick<Storage, 'getItem' | 'setItem'>;
  /** the page is hidden: a save goes out at once */
  hidden: () => boolean;
  /** run `f` once the next frame has been drawn */
  afterFrame: (f: () => void) => void;
}
const browserHost: VetsHost = {
  storage: () => localStorage,
  hidden: () => document.visibilityState === 'hidden',
  afterFrame: (f) => requestAnimationFrame(() => setTimeout(f)),
};
let host = browserHost;

// ---------- loading
const STORAGE_KEY = 'llp:career',
  STORAGE_VERSION = 2;
// stars of a skill added by hand: 2, the most common star count of a white spark at SS rank (70% at rank score 17,500+,
// 45% below it; hakuraku.moe)
export const NEW_STARS = 2;
function loadAll() {
  try {
    const saved = JSON.parse(host.storage().getItem(STORAGE_KEY) ?? 'null'); // (nothing stored yet: null)
    if (saved && saved.v === STORAGE_VERSION) return saved;
    if (saved && saved.v === 1) {
      // v1: a list of veterans per era
      const upgraded: Record<string, unknown> = { v: STORAGE_VERSION };
      for (const era of ['global', 'gm']) if (Array.isArray(saved[era])) upgraded[era] = { vets: saved[era] };
      return upgraded;
    }
  } catch {
    // storage blocked or the veterans unreadable: none
  }
  return { v: STORAGE_VERSION };
}
/** a stored or pasted object, before it is checked */
type Raw = Record<string, unknown>;
/** a record the page can show and the solver can read: an id and two parent nodes (a damaged one is left out) */
export const isVeteran = (v: unknown): boolean => {
  const r = v as Raw | null;
  return (
    !!r &&
    typeof r.id === 'string' &&
    Array.isArray(r.parents) &&
    r.parents.length === 2 &&
    r.parents.every((p: unknown) => !!p && typeof p === 'object')
  );
};
const isMap = (x: unknown) => !!x && typeof x === 'object' && !Array.isArray(x);
/** a stored or pasted lineage node with the fields the page draws, defaulted when missing or of the wrong kind (an
 *  older save, a hand-edited or cut export) */
const asNode = (n: Raw): LineageNode => ({
  ...n,
  chara: Number.isInteger(n.chara) ? (n.chara as number) : null,
  sparks: isMap(n.sparks) ? (n.sparks as Sparks) : {},
  pinks: isMap(n.pinks) ? (n.pinks as Sparks) : {},
  wins: Array.isArray(n.wins) ? n.wins : [],
});
/** a checked record (`isVeteran`) with its nodes' fields defaulted */
export const asVeteran = (v: unknown): Veteran => {
  const r = v as Raw;
  return { ...r, ...asNode(r), parents: (r.parents as Raw[]).map(asNode) } as Veteran;
};
const isChosen = (c: unknown) => {
  const r = c as Raw | null;
  return !!r && typeof r.places === 'string' && typeof r.id === 'string' && typeof r.setup === 'string';
};
// freezes what is new in a record, stopping at what is frozen already (a record's untouched parts were frozen when they
// came in): an edit costs the objects it made, not the whole pool
function freezeFresh<T>(value: T): T {
  if (value !== null && typeof value === 'object' && !Object.isFrozen(value)) {
    Object.freeze(value);
    for (const inner of Object.values(value)) freezeFresh(inner);
  }
  return value;
}
function loaded(): VetsState {
  const mine = loadAll()[ERA] || {};
  return freezeFresh({
    vets: (mine.vets || []).filter(isVeteran).map(asVeteran),
    plan: mine.plan || { trainee: null, parents: [null, null] },
    chosen: isChosen(mine.chosen) ? mine.chosen : null,
  });
}
const record = signal<VetsState>(loaded());
/** every veteran of this era; a veteran's number on the page is her index here plus one */
export const vets: ReadonlySignal<readonly VetRecord[]> = computed(() => record.value.vets);
/** the next career's places (saved as `plan`) */
export const places: ReadonlySignal<PlacesRecord> = computed(() => record.value.plan);
/** the career Use this chose (null: none) */
export const chosen: ReadonlySignal<DeepReadonly<ChosenCareer> | null> = computed(() => record.value.chosen);
// Load the veterans from `at`'s storage and save there from now on (the page's are localStorage's, read at load; a
// test passes its own)
export function initVets(at: VetsHost) {
  host = at;
  persistDue = false;
  record.value = loaded();
}

// ---------- saving
let persistDue = false;
function write(state: VetsState) {
  persistDue = false;
  // (uma.moe's listings come back from its kept answer; one chosen as a parent is saved, so the place keeps her). Only
  // this era is written over what is stored now: another tab's write to the other era survives this tab's.
  const all = loadAll();
  all[ERA] = {
    vets: state.vets.filter((v) => v.src !== 'umamoe' || state.plan.parents.includes(v.id)),
    plan: state.plan,
    chosen: state.chosen,
  };
  try {
    host.storage().setItem(STORAGE_KEY, JSON.stringify(all));
  } catch {
    // storage blocked or full: the edit lasts for this visit only
  }
}
/** save the record now (the era switch, before its reload) */
export const persist = () => write(record.peek());
/** a save still due goes out now (the page hidden or left) */
export function flushPersist() {
  if (persistDue) persist();
}
// the next record, saved (or its save scheduled: it reads the record when it runs), then assigned
function commit(patch: Partial<VetsState>, now = false) {
  const next = freezeFresh({ ...record.peek(), ...patch });
  if (now || host.hidden()) write(next);
  else if (!persistDue) {
    persistDue = true;
    host.afterFrame(flushPersist);
  }
  record.value = next;
}

// ---------- reading
/** her record (undefined: no veteran has that id) */
export const vetById = (id: string) => vets.value.find((v) => v.id === id);
// the places with every parent place naming a veteran no longer in `list` handed back to the planner (the same record
// when none does)
function placesFor(list: readonly VetRecord[], plan: PlacesRecord): PlacesRecord {
  const parents = plan.parents.map((id) => (id && list.some((v) => v.id === id) ? id : null));
  return parents.every((id, k) => id === plan.parents[k]) ? plan : { ...plan, parents };
}
/**
 * A veteran as the solver's job takes her (sparks as [group, stars] pairs; an unknown parent's character 0: no relation
 * points).
 */
export interface JobNode {
  chara: number;
  sparks: [number, number][];
  pinks: [number, number][];
  wins: readonly number[];
}
export interface JobLegacy extends JobNode {
  borrowed: boolean;
  parents: JobNode[];
}
const pairs = (sparks: DeepReadonly<Sparks> | undefined): [number, number][] =>
  Object.entries(sparks || {}).map(([group, stars]) => [+group, stars]);
const jobNode = (n: NodeRecord): JobNode => ({
  chara: n.chara || 0,
  sparks: pairs(n.sparks),
  pinks: pairs(n.pinks),
  wins: n.wins,
});
// each veteran as the job takes her, made once per record
const jobOf = new WeakMap<VetRecord, JobLegacy>();
function jobLegacy(v: VetRecord) {
  let job = jobOf.get(v);
  if (!job) {
    job = { ...jobNode(v), borrowed: !!v.borrowed, parents: v.parents.map(jobNode) };
    jobOf.set(v, job);
  }
  return job;
}
const legacies = computed(() => vets.value.map(jobLegacy));
/** every veteran as the job takes her: the same array until the veterans change (callers don't change it) */
export const legaciesJob = () => legacies.value;

// ---------- the list
// `list` with item `at` replaced
const replaced = <T>(list: readonly T[], at: number, item: T) => list.map((x, i) => (i === at ? item : x));
// veterans join the pool before uma.moe's listings, which a new answer replaces at the end, so their numbers keep
function appended(list: readonly VetRecord[], added: readonly DeepReadonly<Veteran>[]) {
  const at = list.findIndex((v) => v.src === 'umamoe'),
    next = [...list];
  next.splice(at < 0 ? next.length : at, 0, ...added);
  return next;
}
// the list replaced, and the places that named a veteran no longer in it handed back
function commitList(list: readonly VetRecord[], plan = record.peek().plan) {
  commit({ vets: list, plan: placesFor(list, plan) });
}
/** veterans from an uma.moe tree, or the samples: added after the others (the numbers you have keep) */
export function addVets(list: readonly Veteran[]) {
  commitList(appended(record.peek().vets, list));
}
/** A new veteran by hand for a parent place: she gets `sparks`, and so do her parents (a looped parent's own parents
 *  carry the same sparks), whose umas are not known yet; `extra` over that (a veteran entered from the analysis).
 *  Returns her record. */
export function newVet(chara: number | null, sparks: Sparks, extra: Partial<Veteran> = {}): VetRecord {
  const v: Veteran = {
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
    ...extra,
  };
  addVets([v]);
  return v;
}
/**
 * An import never loses one of your veterans: yours merge by id (a veteran already here takes the file's version in
 * her place, new ones go after your other imports), while borrowed ones replace the earlier borrow list (it is who
 * lends today), after all of yours so your numbers keep. `all` (an Export of this page, holding both kinds): every
 * earlier import goes. Your own veterans arriving retire the sample ones (they stood in until you had some). Hand-made
 * ones stay (a place holding a removed one goes back to the planner).
 * Returns { added, updated } for yours and { borrowed, dropped } for the borrow list.
 */
export function importVets(list: readonly Veteran[], all = false) {
  const isImport = (v: VetRecord) => v.src === 'import';
  // the borrow list's veterans by their id (umadump.js: a lender's viewer id after "tr"), not by the Yours/Borrowed
  // switch, which the player may have flipped
  const lent = (v: VetRecord) => isImport(v) && v.id.startsWith('tr');
  let next = record.peek().vets.filter((v) => !all || !isImport(v));
  const own = list.filter((v) => !v.borrowed),
    borrowed = list.filter((v) => v.borrowed),
    fresh: Veteran[] = [];
  if (own.length) next = next.filter((v) => v.src !== 'sample');
  let updated = 0;
  // a veteran already here takes the file's version in her place, on the tab the player keeps her on (true when she is)
  const update = (v: Veteran) => {
    const at = next.findIndex((old) => old.id === v.id);
    if (at < 0) return false;
    next = replaced(next, at, { ...next[at], ...v, borrowed: next[at].borrowed });
    return true;
  };
  for (const v of own) {
    if (update(v)) updated++;
    else fresh.push(v);
  }
  const afterOwnImports = next.reduce((at, v, i) => (isImport(v) && !v.borrowed ? i + 1 : at), 0);
  next = [...next.slice(0, afterOwnImports), ...fresh, ...next.slice(afterOwnImports)];
  let dropped = 0;
  if (borrowed.length) {
    // the earlier borrow list goes, but not one the player moved to Yours
    const kept = next.filter((v) => !(lent(v) && v.borrowed));
    dropped = next.length - kept.length;
    next = kept;
    for (const v of borrowed) if (!update(v)) next = appended(next, [v]);
  }
  commitList(next);
  return { added: fresh.length, updated, borrowed: borrowed.length, dropped };
}
/** uma.moe's listings for the targets (umadump.js `parseListing`): borrowed veterans the planner weighs like any other,
 *  after every other veteran. They replace the earlier listings, except one chosen as a parent (she stays while chosen,
 *  taking her new record when listed again). Not saved unless chosen. */
export function setListings(list: readonly Veteran[]) {
  const fresh = new Map(list.map((v) => [v.id, v])),
    { vets: now, plan } = record.peek(),
    next: VetRecord[] = [];
  for (const v of now) {
    if (v.src !== 'umamoe') next.push(v);
    else if (plan.parents.includes(v.id)) {
      const again = fresh.get(v.id);
      next.push(again ? { ...v, ...again } : v);
      fresh.delete(v.id);
    }
  }
  commitList([...next, ...fresh.values()]);
}
/** she leaves the pool */
export function removeVet(id: string) {
  commitList(record.peek().vets.filter((v) => v.id !== id));
}
/** every veteran, or those `drop` picks */
export function clearVets(drop: (v: VetRecord) => boolean = () => true) {
  commitList(record.peek().vets.filter((v) => !drop(v)));
}
/** a parent place naming a veteran no longer here (removed; an uma.moe listing not back from its answer) goes back to
 *  the planner */
export function dropMissing() {
  const { vets: list, plan } = record.peek(),
    next = placesFor(list, plan);
  if (next !== plan) commit({ plan: next });
}

// ---------- a node's fields
/** what an edit gives a node: the fields it replaces (undefined: the field goes) */
export type NodePatch = { [K in keyof LineageNode]?: DeepReadonly<LineageNode[K]> | undefined };
/** an edit of one node, from her record as it is when the edit lands */
export type NodeEdit = (f: (now: NodeRecord) => NodePatch) => void;
/** a node an editor works on: her record now (undefined: gone), and how to edit her */
export interface NodeRef {
  node: () => NodeRecord | undefined;
  edit: NodeEdit;
}
function patched<T extends object>(node: T, patch: object): T {
  const next: Record<string, unknown> = { ...(node as Record<string, unknown>), ...patch };
  for (const [key, value] of Object.entries(patch)) if (value === undefined) delete next[key];
  return next as T;
}
function editAt(id: string, f: (v: VetRecord) => VetRecord) {
  const list = record.peek().vets,
    at = list.findIndex((v) => v.id === id);
  if (at >= 0) commit({ vets: replaced(list, at, f(list[at])) });
}
/** a veteran's own fields (an edit of one no longer here does nothing) */
export function editVet(id: string, f: (now: VetRecord) => NodePatch & { borrowed?: boolean }) {
  editAt(id, (v) => patched(v, f(v)));
}
/** her parent j's fields */
function editParent(id: string, j: number, f: (now: NodeRecord) => NodePatch) {
  editAt(id, (v) => ({ ...v, parents: replaced(v.parents, j, patched(v.parents[j], f(v.parents[j]))) }));
}
/** a veteran of the pool, for an editor */
export const vetRef = (id: string): NodeRef => ({ node: () => vetById(id), edit: (f) => editVet(id, f) });
/** her parent j, for an editor */
export const parentRef = (id: string, j: number): NodeRef => ({
  node: () => vetById(id)?.parents[j],
  edit: (f) => editParent(id, j, f),
});

// ---------- the places
/** the trainee (null: the planner picks); `byHand`: chosen here, so a change of setup keeps her */
export function setTrainee(trainee: number | null, byHand: boolean) {
  const plan = record.peek().plan;
  commit({ plan: { ...plan, trainee, ...(byHand ? { fromTable: false } : {}) } });
}
/** parent place k (null: the planner picks); `byHand`: chosen here, so a change of setup keeps her */
export function setParent(k: number, id: string | null, byHand: boolean) {
  const plan = record.peek().plan;
  commit({ plan: { ...plan, parents: replaced(plan.parents, k, id), ...(byHand ? { fromTable: false } : {}) } });
}
/** the places of a career of the other careers' table (Use this); what it chose goes (`setChosen` names the next) */
export function usePlaces(trainee: number, parents: readonly (string | null)[]) {
  commit({ plan: { ...record.peek().plan, trainee, parents, fromTable: true }, chosen: null });
}
/** the places as the plan picked them (Back to the planner's pick) */
export function backToPick(pick: PlacesRecord) {
  commit({ plan: { ...record.peek().plan, trainee: pick.trainee, parents: pick.parents.slice() } });
}
/** every place back to the planner */
export function clearPlaces() {
  commit({ plan: { ...record.peek().plan, trainee: null, parents: [null, null] } });
}
/** a change of setup with places taken from the other careers' table: they go back to the planner (saved at once) */
export function handBackPlaces() {
  const plan = record.peek().plan;
  commit({ plan: { ...plan, trainee: null, parents: [null, null], fromTable: false }, chosen: null }, true);
}
/** the career Use this chose (null: none); `now`: saved at once */
export function setChosen(career: ChosenCareer | null, now = false) {
  commit({ chosen: career }, now);
}
