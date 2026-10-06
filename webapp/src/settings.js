// ---------- saved settings: one object, localStorage['llp:settings'] ----------
// { v, era: 'global' | 'gm', common: {targets, rank, ...}, global: {roster, outfits, cardsOff}, gm: {...} }
// Owned umas, outfits and cards are kept per data set; everything else is shared.
// Game data: Global today, or JP data cut at L'Arc's launch (24 Aug 2023), the Grand Masters era. Switching
// reloads the page with the other data set.
const SKEY = 'llp:settings', SV = 2;
export const PER_ERA = ['roster', 'outfits', 'cardsOff'];
export const COMMON = ['targets', 'rank', 'auto', 'borrow', 'decksTop', 'perUma', 'rcards', 'scen', 'rsort', 'rdir', 'csort', 'cdir'];
export const SETTINGS = [...COMMON, ...PER_ERA];   // exported and imported (older exports may carry more keys; those are ignored)
// anything else saved under the key (an older format) starts from the defaults
function loadSaved() { try { const o = JSON.parse(localStorage.getItem(SKEY)); if (o && o.v === SV) return o; } catch {} return { v: SV, era: 'global' }; }
export const SAVED = loadSaved();
for (const k of ['common', 'global', 'gm']) SAVED[k] = SAVED[k] || {};
export const ERA = SAVED.era === 'gm' ? 'gm' : 'global';
export function persist() { try { localStorage.setItem(SKEY, JSON.stringify(SAVED)); } catch {} }
