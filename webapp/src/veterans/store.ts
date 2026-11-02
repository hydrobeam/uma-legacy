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
import { computed } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { ERA } from '../saved.ts';
import { browserStorage, readJson, updateJson } from '../storage.ts';
import type { Backing } from '../storage.ts';
import { gradesOfList, isGrades } from '../model/grades.ts';
import type { Grades } from '../gen/solver.ts';
import { jobLegacy, knownPinks } from './records.ts';
import { must } from '../must.ts';
import type { JobLegacy } from './records.ts';
import { store } from '../store.ts';
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
  apt?: Grades | null;
}
/** The trainer who lists a veteran found on uma.moe (umadump.ts `parseListing`): kept on this device, left out of
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
/** A parent place's record: her parent, or null where the record she came from lacks that parent (absent: a game
 *  export's veteran, umadump.ts), whom the solver reads as no one. A parent with no uma is one not entered yet
 *  (complete.ts). */
export type ParentNode = LineageNode | null;
/** a veteran's two parents, in their places */
export type Parents = [ParentNode, ParentNode];
/** A veteran of the pool: a lineage node with an identity, her source and her two parents. */
export interface Veteran extends LineageNode {
  id: string;
  /** where she came from: 'import' | 'tree' (uma.moe's lineage planner) | 'umamoe' (uma.moe's search) | 'hand' |
   *  'sample' | 'entered' (older saves: 'simulated') */
  src: string;
  borrowed: boolean;
  parents: Parents;
  lender?: Lender;
  /** her rank score (her career-rank badge), when the import or uma.moe gives it */
  rank?: number | undefined;
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
/** the career Use this chose with its deck (career/job.ts `chosenSetup`, `chosenApplies`), saved with the places: the
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
  storage: () => Backing;
  /** the page is hidden: a save goes out at once */
  hidden: () => boolean;
  /** run `f` once the next frame has been drawn */
  afterFrame: (f: () => void) => void;
}
const browserHost: VetsHost = {
  storage: browserStorage,
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
// what is stored, in this version (nothing, unreadable or the storage blocked: none)
// biome-ignore lint/suspicious/noExplicitAny: the stored object, unchecked until each part is read
function asAll(saved: any) {
  if (saved && saved.v === STORAGE_VERSION) return saved;
  if (saved && saved.v === 1) {
    // v1: a list of veterans per era
    const upgraded: Record<string, unknown> = { v: STORAGE_VERSION };
    for (const era of ['global', 'gm']) if (Array.isArray(saved[era])) upgraded[era] = { vets: saved[era] };
    return upgraded;
  }
  return { v: STORAGE_VERSION };
}
const loadAll = () => asAll(readJson(STORAGE_KEY, host.storage));
/** a stored or pasted object, before it is checked */
type Raw = Record<string, unknown>;
/** a record the page can show: an id and two parent places, each a node or null (absent); a damaged one is left out */
export const isVeteran = (v: unknown): boolean => {
  const r = v as Raw | null;
  return (
    !!r &&
    typeof r.id === 'string' &&
    Array.isArray(r.parents) &&
    r.parents.length === 2 &&
    r.parents.every((p: unknown) => p === null || (!!p && typeof p === 'object'))
  );
};
const isMap = (x: unknown) => !!x && typeof x === 'object' && !Array.isArray(x);
/** a stored or pasted lineage node with the fields the page draws, defaulted when missing or of the wrong kind (an
 *  older save, a hand-edited or cut export) */
const asNode = (n: Raw): LineageNode => ({
  ...n,
  // (grades saved as a list, before they had kinds, read as grades; damaged ones as none)
  ...(n.apt == null || isGrades(n.apt) ? {} : { apt: gradesOfList(n.apt) }),
  chara: Number.isInteger(n.chara) ? (n.chara as number) : null,
  sparks: isMap(n.sparks) ? (n.sparks as Sparks) : {},
  pinks: knownPinks(n.pinks).pinks, // (an aptitude the solver doesn't know is left out: records.ts)
  wins: Array.isArray(n.wins) ? n.wins : [],
});
// the sources of veterans entered here: a parent of theirs stored with no uma is one not entered (complete.ts: earlier
// pages made a hand-entered veteran's parents up so); any other source's is one the record she came from lacks, absent
const ENTERED_HERE = new Set(['hand', 'entered', 'simulated']);
/** a checked record (`isVeteran`) with its nodes' fields defaulted, each parent place a node or absent */
export const asVeteran = (v: unknown): Veteran => {
  const r = v as Raw,
    enteredHere = ENTERED_HERE.has(r.src as string),
    [first, second] = r.parents as [Raw | null, Raw | null]; // (two places: isVeteran)
  const parent = (p: Raw | null): ParentNode => {
    if (p === null) return null;
    const node = asNode(p);
    return node.chara === null && !enteredHere ? null : node;
  };
  return { ...r, ...asNode(r), parents: [parent(first), parent(second)] } as Veteran;
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
// the list with each id once, the first kept (logged): an id is how every edit and Remove finds her, and a malformed
// file or save can hold one twice
function onceEach<T extends { id: string }>(list: readonly T[]): T[] {
  const seen = new Set<string>(),
    twice: string[] = [];
  const once = list.filter((v) => {
    if (seen.has(v.id)) {
      twice.push(v.id);
      return false;
    }
    seen.add(v.id);
    return true;
  });
  if (twice.length) console.warn(`veterans: ids given twice, the first kept: ${twice.join(', ')}`);
  return once;
}
function loaded(): VetsState {
  const mine = loadAll()[ERA] || {};
  return freezeFresh({
    vets: onceEach((mine.vets || []).filter(isVeteran).map(asVeteran)),
    plan: mine.plan || { trainee: null, parents: [null, null] },
    chosen: isChosen(mine.chosen) ? mine.chosen : null,
  });
}
const [record, setRecord] = store<VetsState>(loaded());
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
  setRecord(loaded());
}

// ---------- saving
let persistDue = false;
function write(state: VetsState) {
  persistDue = false;
  // (uma.moe's listings come back from its kept answer; one chosen as a parent is saved, so the place keeps her). Only
  // this era is written over what is stored now: another tab's write to the other era survives this tab's. (Storage
  // blocked or full: the edit lasts for this visit only.)
  updateJson(
    STORAGE_KEY,
    (stored) => ({
      ...asAll(stored),
      [ERA]: {
        vets: state.vets.filter((v) => v.src !== 'umamoe' || state.plan.parents.includes(v.id)),
        plan: state.plan,
        chosen: state.chosen,
      },
    }),
    host.storage,
  );
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
  setRecord(next);
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
// each veteran as the job takes her, made once per record (null: her record is not whole: records.ts `jobLegacy`, which
// agrees with complete.ts `lacks`)
const jobOf = new WeakMap<VetRecord, JobLegacy | null>();
function jobOfVet(v: VetRecord): JobLegacy | null {
  let job = jobOf.get(v);
  if (job === undefined) {
    job = jobLegacy(v);
    jobOf.set(v, job);
  }
  return job;
}
// the veterans a job takes, in the pool's order, and each as the job takes her
const jobPool = computed(() => {
  const vetsIn = vets.value.filter((v) => jobOfVet(v) !== null);
  return { vets: vetsIn, legacies: vetsIn.map((v) => must(jobOfVet(v), 'a veteran the job takes is whole')) };
});
/** the complete veterans (complete.ts): those a job takes, in the pool's order, legacy index i the i-th; the same
 *  array until the veterans change */
export const readyVets: ReadonlySignal<readonly VetRecord[]> = computed(() => jobPool.value.vets);
/** every complete veteran as the job takes her: the same array until the veterans change (callers don't change it) */
export const legaciesJob = () => jobPool.value.legacies;

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
  const vets = onceEach(list);
  commit({ vets, plan: placesFor(vets, plan) });
}
/** veterans from an uma.moe tree, or the samples: added after the others (the numbers you have keep) */
export function addVets(list: readonly Veteran[]) {
  commitList(appended(record.peek().vets, list));
}
/** A veteran entered here, one of yours, after the others: `src` 'hand' (the parent picker's A new one: entry.ts) or
 *  'entered' (Try a run's Add to veterans: her record as entered, her parents the run's). Returns her record. */
export function newVet(her: LineageNode & { parents: Parents }, src: 'hand' | 'entered'): VetRecord {
  const v: Veteran = {
    ...her,
    id: `h${Date.now().toString(36)}${Math.random().toString(36).slice(2, 5)}`,
    src,
    borrowed: false,
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
  // the borrow list's veterans by their id (umadump.ts: a lender's viewer id after "tr"), not by the Yours/Borrowed
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
    if (!next.some((old) => old.id === v.id)) return false;
    next = next.map((old) => (old.id === v.id ? { ...old, ...v, borrowed: old.borrowed } : old));
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
/** uma.moe's listings for the targets (umadump.ts `parseListing`): borrowed veterans the planner weighs like any other,
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
// her record edited by `f` (none here, or `f` hands her back as she is: nothing is written)
function editAt(id: string, f: (v: VetRecord) => VetRecord) {
  const list = record.peek().vets,
    next = list.map((v) => (v.id === id ? f(v) : v));
  if (next.some((v, i) => v !== list[i])) commit({ vets: next });
}
/** a veteran's own fields (an edit of one no longer here does nothing) */
export function editVet(id: string, f: (now: VetRecord) => NodePatch & { borrowed?: boolean }) {
  editAt(id, (v) => patched(v, f(v)));
}
/** a parent's place: a lineage node's parents are a pair (isVeteran checks a stored veteran's) */
export type ParentPlace = 0 | 1;
/** a node's parents with their places, in order, leaving out an absent one (none for a node shown without her
 *  parents) */
export const parentsPlaced = <T>(parents: readonly (T | null)[]): [T, ParentPlace][] =>
  ([0, 1] as const).flatMap((j): [T, ParentPlace][] => {
    const parent = parents[j];
    return parent === undefined || parent === null ? [] : [[parent, j]];
  });
/** her parent j's fields (an absent one has none to edit) */
function editParent(id: string, j: ParentPlace, f: (now: NodeRecord) => NodePatch) {
  editAt(id, (v) => {
    const [first, second] = v.parents,
      parent = j === 0 ? first : second;
    if (!parent) return v;
    const edited = patched(parent, f(parent));
    return { ...v, parents: j === 0 ? [edited, second] : [first, edited] };
  });
}
/** a veteran of the pool, for an editor */
export const vetRef = (id: string): NodeRef => ({ node: () => vetById(id), edit: (f) => editVet(id, f) });
/** her parent j, for an editor (absent: no node) */
export const parentRef = (id: string, j: ParentPlace): NodeRef => ({
  node: () => vetById(id)?.parents[j] ?? undefined,
  edit: (f) => editParent(id, j, f),
});

// ---------- the veteran analysis' draft
// The veteran a roll of the career on show leaves (career/analysis): not saved, and apart from the record above, so an
// edit of her wakes no veterans' effect. Her editors address her, and her parents, by where she is (`draftRef`), and
// only while she is on show: each new draft (`setDraft`: a new career, a roll, Add, none) starts a generation, and an
// editor opened on an earlier one (a G1 wins window, the "+" list or the colour picker left open while an answer
// brings another career) sees no node and edits nothing.
/** the draft: a lineage node with her two parents, and what the roll knows of her */
export interface Draft extends LineageNode {
  borrowed: boolean;
  parents: LineageNode[];
  /** the skills she bought (group -> how), from the roll */
  bought?: Record<number, string>;
}
const [draftNow, putDraft] = store<Draft | null>(null);
let draftGen = 0; // the draft on show's generation (a staleness token: a number, never her object)
/** the draft on show (null: none) */
export const draft: ReadonlySignal<DeepReadonly<Draft> | null> = draftNow;
/** a new draft (a roll, a new career on show, Add), or none: editors opened on the last one go stale. The store keeps
 *  its own frozen copy (the caller's object stays the caller's, unfrozen) */
export function setDraft(next: Draft | DeepReadonly<Draft> | null) {
  draftGen++;
  putDraft(next && freezeFresh(structuredClone(next) as Draft));
}
// the draft on show for an editor opened now: `at` reads her (or her parent), `next` her edited record
function draftAt(
  at: (d: DeepReadonly<Draft>) => NodeRecord | undefined,
  next: (d: DeepReadonly<Draft>, f: Parameters<NodeEdit>[0]) => Draft,
): NodeRef {
  const gen = draftGen,
    now = () => (gen === draftGen ? draftNow.peek() : null);
  return {
    node: () => {
      const d = gen === draftGen ? draft.value : null;
      return d ? at(d) : undefined;
    },
    edit: (f) => {
      const d = now();
      if (d) putDraft(freezeFresh(next(d, f)));
    },
  };
}
/** her parents' records as they are now (the same draft, the same editors: they edit her own fields) */
export function setDraftParents(parents: readonly LineageNode[] | DeepReadonly<LineageNode[]>) {
  const d = draftNow.peek();
  if (d) putDraft(freezeFresh({ ...d, parents: structuredClone(parents) } as Draft));
}
/** the draft on show, for an editor opened now (an edit after she is replaced, or with no draft, does nothing) */
export const draftRef = (): NodeRef =>
  draftAt(
    (d) => d,
    (d, f) => patched(d, f(d)) as Draft,
  );
/** her parent j, for an editor opened now */
export const draftParentRef = (j: ParentPlace): NodeRef =>
  draftAt(
    (d) => d.parents[j],
    (d, f) => ({ ...d, parents: d.parents.map((p, i) => (i === j ? patched(p, f(p)) : p)) }) as Draft,
  );

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
