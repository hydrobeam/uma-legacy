// ---------- saved settings: one object, localStorage['llp:settings'] ----------
// { v, era: 'global' | 'gm', common: {targets, rank, ...}, global: {roster, outfits, cardsOff}, gm: {...} }
// Owned umas, outfits and cards are kept per data set; everything else is shared.
// Game data: Global today, or JP data cut at L'Arc's launch (24 Aug 2023), the Grand Masters era. Switching
// reloads the page with the other data set. The settings themselves (the record, its setters) are settings.ts's.
import { browserStorage, readJson, updateJson } from './storage.ts';
import type { Backing } from './storage.ts';

const SETTINGS_KEY = 'llp:settings',
  SETTINGS_VERSION = 2;
export const PER_ERA = ['roster', 'outfits', 'cardsOff'] as const;
export const COMMON = [
  'targets',
  'presets',
  'rank',
  'auto',
  'decksTop',
  'perUma',
  'scen',
  'rsort',
  'rdir',
  'csort',
  'cdir',
  'borrowPar',
] as const;
export const SETTINGS = [...COMMON, ...PER_ERA]; // exported and imported (older exports may carry more keys; those are ignored)
export type Era = 'global' | 'gm';
/** one part of the stored object: a setting's name -> its value as stored (unchecked: whatever was saved) */
export type SavedPart = Record<string, unknown>;
/** the stored object */
export interface Saved {
  v: number;
  era: string;
  common?: SavedPart;
  global?: SavedPart;
  gm?: SavedPart;
}
// anything else saved under the key (an older format, nothing, unreadable) starts from the defaults
const asSaved = (stored: unknown): Saved =>
  stored && typeof stored === 'object' && (stored as Saved).v === SETTINGS_VERSION
    ? (stored as Saved)
    : { v: SETTINGS_VERSION, era: 'global' };
/** the settings as stored now (`storage`: the page's localStorage, or a test's stand-in) */
export const loadSaved = (storage: () => Backing = browserStorage): Saved => asSaved(readJson(SETTINGS_KEY, storage));
export const SAVED = loadSaved();
for (const part of ['common', 'global', 'gm'] as const) SAVED[part] = SAVED[part] || {};
export const ERA: Era = SAVED.era === 'gm' ? 'gm' : 'global';
// A write replaces only the parts `parts` gives over what is stored now (the shared settings, this tab's era's part,
// and on an import, the other era's), and the era when this tab switches it. Another tab's era survives this tab's
// write. (Storage blocked or full: the settings last for this visit only.)
export function writeSaved(
  storage: () => Backing,
  parts: (stored: Saved) => Partial<Pick<Saved, Era | 'common'>>,
  era?: Era,
) {
  updateJson(
    SETTINGS_KEY,
    (raw) => {
      const stored = asSaved(raw);
      Object.assign(stored, parts(stored));
      if (era) stored.era = era;
      return stored;
    },
    storage,
  );
}
