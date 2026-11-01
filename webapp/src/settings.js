// ---------- saved settings: one object, localStorage['llp:settings'] ----------
// { v, era: 'global' | 'gm', common: {targets, rank, ...}, global: {roster, outfits, cardsOff}, gm: {...} }
// Owned umas, outfits and cards are kept per data set; everything else is shared.
// Game data: Global today, or JP data cut at L'Arc's launch (24 Aug 2023), the Grand Masters era. Switching
// reloads the page with the other data set.
const SETTINGS_KEY = 'llp:settings',
  SETTINGS_VERSION = 2;
export const PER_ERA = ['roster', 'outfits', 'cardsOff'];
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
];
export const SETTINGS = [...COMMON, ...PER_ERA]; // exported and imported (older exports may carry more keys; those are ignored)
// anything else saved under the key (an older format) starts from the defaults
function loadSaved() {
  try {
    const saved = JSON.parse(localStorage.getItem(SETTINGS_KEY));
    if (saved && saved.v === SETTINGS_VERSION) return saved;
  } catch {
    // storage blocked or the saved settings unreadable: the defaults
  }
  return { v: SETTINGS_VERSION, era: 'global' };
}
export const SAVED = loadSaved();
for (const part of ['common', 'global', 'gm']) SAVED[part] = SAVED[part] || {};
export const ERA = SAVED.era === 'gm' ? 'gm' : 'global';
// A write replaces only what this tab owns over what is stored now: the shared settings, its era's part (and `parts`,
// an import's other era), and the era when this tab switches it. Another tab's era survives this tab's write.
/** @param {string[]} [parts] */
export function persist(parts = []) {
  const stored = loadSaved();
  for (const part of ['common', ERA, ...parts]) stored[part] = SAVED[part];
  if (SAVED.era !== ERA) stored.era = SAVED.era;
  try {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(stored));
  } catch {
    // storage blocked or full: the settings last for this visit only
  }
}
