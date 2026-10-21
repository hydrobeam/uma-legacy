// The page's settings (state, saved) and what is on screen (ui, not saved).
import { PER_ERA, COMMON, SAVED, ERA, persist } from './settings.js';
import { PRESET, SCEN_GOLDS } from './data.js';
export const DEFAULTS = { targets: PRESET.map(group => ({ group })), presets: [{ name: '8/8 example', groups: [...PRESET] }], rank: '0.50,0.45,0.05', auto: true, decksTop: '1', perUma: true,
  rcards: false,   // R cards (not pals) stay out: their hint model is untested; no setting turns them on
  scen: 'unity', rsort: 'rel', rdir: 'desc', csort: 'rel', cdir: 'desc', roster: {}, outfits: {}, cardsOff: {},
  borrowPar: 'allow',   // the career planner's borrowed parent: 'allow', 'later' (not this career), 'never'
  reroll: 'never' };    // the game's end-of-career spark reroll the plan assumes: 'never', 'all_bought', 'always'
const saved = (k) => (PER_ERA.includes(k) ? SAVED[ERA] : SAVED.common)[k];
// roster: uma id -> 'out' (not owned); outfits: id -> false (not owned); cardsOff: id -> false when not owned
export const state = Object.fromEntries([...COMMON, ...PER_ERA].map(k => [k, saved(k) !== undefined ? saved(k) : DEFAULTS[k]]));
// The page once had a third mark, "must include" ('req' in roster and outfits); a saved or imported one reads as owned.
export function dropRequired(s) { for (const k of ['roster', 'outfits']) s[k] = Object.fromEntries(Object.entries(s[k] || {}).filter(([, v]) => v !== 'req')); }
dropRequired(state);
// status: the status line when no search runs (search.js paintRunNote); headline: the top result's odds ("1 in 139"),
// for the phone's floating button; lastRender: redraws the results on screen; lastResult: {kind, r, meta} for export
// a target the scenario hands out as a gold whatever the deck (Unity Cup: No Stopping Me!, It's On!); nothing else is
// guaranteed outside the deck, so the player never sets it
export const goldGiven = (group) => (SCEN_GOLDS[state.scen || 'unity'] || []).includes(group);
export const ui = { status: { kind: 'loading', text: '' }, headline: '', lastRender: null, lastResult: null };
export function writeSaved() {
  SAVED.common = Object.fromEntries(COMMON.map(k => [k, state[k]])); SAVED[ERA] = Object.fromEntries(PER_ERA.map(k => [k, state[k]]));
  persist();
}
