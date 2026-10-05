// The page's settings (state, saved) and what is on screen (ui, not saved).
import { PER_ERA, COMMON, SAVED, ERA, persist } from './settings.js';
import { PRESET } from './data.js';
export const DEFAULTS = { targets: PRESET.map(t => ({ ...t })), rank: '0.50,0.45,0.05', borrow: true, decksTop: '1', perUma: true,
  rcards: false, scen: 'unity', rsort: 'rel', rdir: 'desc', csort: 'rel', cdir: 'desc', roster: {}, outfits: {}, cardsOff: {} };
const saved = (k) => (PER_ERA.includes(k) ? SAVED[ERA] : SAVED.common)[k];
// roster: uma id -> 'out' | 'req' ('req' only for umas without separate outfits); outfits: id -> false (not owned) | 'req'
// (must include this outfit); cardsOff: id -> false when not owned
export const state = Object.fromEntries([...COMMON, ...PER_ERA].map(k => [k, saved(k) !== undefined ? saved(k) : DEFAULTS[k]]));
// status: the status line when no search runs (search.js paintRunNote); headline: the top result's odds ("1 in 139"),
// for the phone's floating button; lastRender: redraws the results on screen; lastResult: {kind, r, meta} for export
export const ui = { status: { kind: 'loading', text: '' }, headline: '', lastRender: null, lastResult: null };
export function writeSaved() {
  SAVED.common = Object.fromEntries(COMMON.map(k => [k, state[k]])); SAVED[ERA] = Object.fromEntries(PER_ERA.map(k => [k, state[k]]));
  persist();
}
