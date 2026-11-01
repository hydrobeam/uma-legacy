// ---------- saved settings: one object, localStorage['llp:settings'] ----------
// { v, era: 'global' | 'gm', common: {targets, rank, ...}, global: {roster, outfits, cardsOff}, gm: {...} }
// Owned umas, outfits and cards are kept per data set; everything else is shared.
// Game data: Global today, or JP data cut at L'Arc's launch (24 Aug 2023), the Grand Masters era. Switching
// reloads the page with the other data set. The settings themselves (the record, its setters) are settings.ts's.
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
/** where the settings are kept: the browser's localStorage, or a test's stand-in */
export type SettingsStorage = Pick<Storage, 'getItem' | 'setItem'>;
// the page's storage, reached inside each try: a blocked storage throws on access
export const browserStorage = (): SettingsStorage => localStorage;
// anything else saved under the key (an older format) starts from the defaults
export function loadSaved(storage = browserStorage): Saved {
  try {
    const saved = JSON.parse(storage().getItem(SETTINGS_KEY) ?? 'null'); // (nothing stored yet: null)
    if (saved && saved.v === SETTINGS_VERSION) return saved;
  } catch {
    // storage blocked or the saved settings unreadable: the defaults
  }
  return { v: SETTINGS_VERSION, era: 'global' };
}
export const SAVED = loadSaved();
for (const part of ['common', 'global', 'gm'] as const) SAVED[part] = SAVED[part] || {};
export const ERA: Era = SAVED.era === 'gm' ? 'gm' : 'global';
// A write replaces only the parts given over what is stored now (the shared settings, this tab's era's part, and on an
// import, the other era's), and the era when this tab switches it. Another tab's era survives this tab's write.
export function writeSaved(storage: () => SettingsStorage, parts: Partial<Pick<Saved, Era | 'common'>>, era?: Era) {
  const stored = loadSaved(storage);
  Object.assign(stored, parts);
  if (era) stored.era = era;
  try {
    storage().setItem(SETTINGS_KEY, JSON.stringify(stored));
  } catch {
    // storage blocked or full: the settings last for this visit only
  }
}
