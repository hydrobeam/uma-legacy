// ---------- the settings store: the planner's settings (both pages), one record in a signal ----------
// `settings` hands the record out read-only. Each setter builds the next record copy-on-write (only the fields it
// changes are new objects), persists it (saved.ts), then assigns it: effects run inside the assignment, so one that
// throws cannot lose the write. Every record is frozen all the way down (`deepFreeze`), DEFAULTS too, and a record
// shares none of DEFAULTS' objects: a write past the read-only type throws. `fields` has a signal per setting, which changes only when that field is replaced;
// `jobSetup` changes when any setting a job reads is. Nothing here subscribes: the entries make the effects (store.ts
// `on`). Read `.value` in handlers and in what an entry's effect calls (`on` runs it untracked); logic modules take
// the values as arguments.
import { computed, signal } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import { COMMON, ERA, PER_ERA, SAVED, SETTINGS, browserStorage, loadSaved, writeSaved } from './saved.ts';
import type { Era, Saved, SavedPart, SettingsStorage } from './saved.ts';
import { deepFreeze } from './store.ts';
import type { DeepReadonly } from './store.ts';

export interface Target {
  group: number;
}
/** a named set of target skill groups */
export interface Preset {
  name: string;
  groups: number[];
}
// roster: uma id -> 'out' (not owned); outfits: id -> false (not owned); cardsOff: id -> false when not owned
export interface Settings {
  targets: Target[];
  presets: Preset[];
  /** the lineage's star mix, data.ts RANKS */
  rank: string;
  auto: boolean;
  decksTop: string;
  perUma: boolean;
  scen: string;
  rsort: string;
  rdir: string;
  csort: string;
  cdir: string;
  /** the career planner's borrowed parent: 'allow', 'later' (not this career), 'never' */
  borrowPar: string;
  roster: Partial<Record<number, 'out'>>;
  outfits: Partial<Record<number, false>>;
  cardsOff: Partial<Record<number, false>>;
  /** R cards (not pals) in the search: only in DEFAULTS (false), never saved; no setting turns them on */
  rcards?: boolean;
}
export type SettingsView = DeepReadonly<Settings>;

// the 8/8 example: the default eight targets, as skill groups (Unity Cup's two guaranteed golds among them: job.ts
// goldGiven)
export const PRESET = [20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203];
export const DEFAULTS: SettingsView = deepFreeze({
  targets: PRESET.map((group) => ({ group })),
  presets: [{ name: '8/8 example', groups: [...PRESET] }],
  rank: '0.50,0.45,0.05',
  auto: true,
  decksTop: '1',
  perUma: true,
  rcards: false, // R cards (not pals) stay out: their hint model is untested; no setting turns them on
  scen: 'unity',
  rsort: 'rel',
  rdir: 'desc',
  csort: 'rel',
  cdir: 'desc',
  roster: {},
  outfits: {},
  cardsOff: {},
  borrowPar: 'allow',
});
// The page once had a third mark, "must include" ('req' in roster and outfits); a saved or imported one reads as owned.
function dropRequired(settings: Record<string, unknown>) {
  const marked = (key: string) =>
    Object.fromEntries(Object.entries(settings[key] || {}).filter(([, mark]) => mark !== 'req'));
  return { ...settings, roster: marked('roster'), outfits: marked('outfits') } as unknown as SettingsView;
}
// Presets are edited by name (presets.ts): of two under one name, which only a hand-edited storage holds, the first
// is kept
const firstOfEachName = (presets: Preset[]) =>
  presets.filter((p, at) => presets.findIndex((q) => q.name === p.name) === at);
// the record from the stored object: each setting as saved (its era's part for the per-era ones), else a copy of its
// default
function fromSaved(saved: Saved) {
  const savedValue = (key: string) => ((PER_ERA as readonly string[]).includes(key) ? saved[ERA] : saved.common)?.[key];
  const value = (key: (typeof SETTINGS)[number]) =>
    savedValue(key) !== undefined ? savedValue(key) : structuredClone(DEFAULTS[key]);
  const loaded = Object.fromEntries(SETTINGS.map((key) => [key, value(key)]));
  if (Array.isArray(loaded.presets)) loaded.presets = firstOfEachName(loaded.presets);
  return deepFreeze(dropRequired(loaded));
}
const pick = (record: SettingsView, keys: readonly string[]): SavedPart =>
  Object.fromEntries(keys.map((key) => [key, record[key as keyof SettingsView]]));

let storage = browserStorage;
const record = signal(fromSaved(SAVED));
export const settings: ReadonlySignal<SettingsView> = record;
// Load the settings from `storage` and keep them there from now on: the page's are localStorage's, read at load
// (saved.ts); a test passes its own
export function initStore(at: () => SettingsStorage) {
  storage = at;
  record.value = fromSaved(loadSaved(at));
}
export const fields = Object.fromEntries(SETTINGS.map((key) => [key, computed(() => record.value[key])])) as {
  readonly [K in (typeof SETTINGS)[number]]: ReadonlySignal<SettingsView[K]>;
};
// What a job reads, the loop search's or the career planner's (each page edits only its own; the career job's
// veterans and places are veterans/store.ts's): a change of any is a change of setup, and the page searches again
export const jobSetup = computed(() =>
  (['targets', 'rank', 'scen', 'roster', 'outfits', 'cardsOff', 'decksTop', 'perUma', 'borrowPar'] as const).map(
    (key) => fields[key].value,
  ),
);

// persist, then assign (the effects run in the assignment)
function commit(patch: Partial<SettingsView>) {
  const next = deepFreeze({ ...record.peek(), ...patch });
  writeSaved(storage, { common: pick(next, COMMON), [ERA]: pick(next, PER_ERA) });
  record.value = next;
}
export const setTargets = (targets: readonly Target[]) => commit({ targets });
export const setPresets = (presets: readonly DeepReadonly<Preset>[]) => commit({ presets });
export const setRank = (rank: string) => commit({ rank });
/** a scenario with its lineage rank (named: two strings in a row would swap unseen) */
export const setScenario = (choice: { scen: string; rank: string }) => commit({ scen: choice.scen, rank: choice.rank });
export const setAuto = (auto: boolean) => commit({ auto });
export const setDecksTop = (decksTop: string) => commit({ decksTop });
export const setPerUma = (perUma: boolean) => commit({ perUma });
export const setBorrowPar = (borrowPar: string) => commit({ borrowPar });
/** a sort menu's choice: `prefix` 'r' (umas: rsort, rdir) or 'c' (cards), the field it sorts by and the direction */
export interface SortChoice {
  prefix: 'r' | 'c';
  sort: string;
  dir: 'asc' | 'desc';
}
export const setSort = ({ prefix, sort, dir }: SortChoice) =>
  commit(prefix === 'r' ? { rsort: sort, rdir: dir } : { csort: sort, cdir: dir });
/** what is owned: the umas' marks, the outfits', the cards' (each one given replaces that one) */
export const setOwned = (owned: Partial<Pick<Settings, 'roster' | 'outfits' | 'cardsOff'>>) => commit(owned);
/** the settings an import carries (checked by io.ts), over these */
export const importSettings = (imported: Partial<Settings>) => commit(dropRequired({ ...record.peek(), ...imported }));

// The page reloads into `era`: what is stored is what it loads. These write storage only: the record on show goes with
// the page, and no effect runs for it.
/** this tab's settings with `patch` over them (a scenario the other era opens on), and the era */
export function switchEra(era: Era, patch: Partial<Settings>) {
  const next = { ...record.peek(), ...patch };
  writeSaved(storage, { common: pick(next, COMMON), [ERA]: pick(next, PER_ERA) }, era);
}
/** an import's settings for the other era: its shared settings over this tab's, its per-era ones over that era's as
 *  stored */
export function importToEra(era: Era, imported: Partial<Settings>) {
  const given = (keys: readonly string[]) =>
    Object.fromEntries(Object.entries(imported).filter(([key]) => keys.includes(key)));
  writeSaved(
    storage,
    {
      common: { ...pick(record.peek(), COMMON), ...given(COMMON) },
      [ERA]: pick(record.peek(), PER_ERA),
      [era]: { ...loadSaved(storage)[era], ...given(PER_ERA) },
    },
    era,
  );
}
