// The page's settings (state, saved) and what is on screen (ui, not saved).
import { PER_ERA, COMMON, SAVED, ERA, persist } from './settings.js';
import { PRESET, SCEN_GOLDS } from './data.js';
export const DEFAULTS = { targets: PRESET.map(group => ({ group })), presets: [{ name: 'Example targets', groups: [...PRESET] }], rank: '0.50,0.45,0.05', auto: true, decksTop: '1', perUma: true,
  rcards: false,   // R cards (not pals) stay out: their hint model is untested; no setting turns them on
  scen: 'unity', rsort: 'rel', rdir: 'desc', csort: 'rel', cdir: 'desc', roster: {}, outfits: {}, cardsOff: {},
  borrowPar: 'allow' };   // the career planner's borrowed parent: 'allow', 'later' (not this career), 'never'
const savedValue = (key) => (PER_ERA.includes(key) ? SAVED[ERA] : SAVED.common)[key];
// roster: uma id -> 'out' (not owned); outfits: id -> false (not owned); cardsOff: id -> false when not owned
export const state = Object.fromEntries([...COMMON, ...PER_ERA].map(key => [key, savedValue(key) !== undefined ? savedValue(key) : DEFAULTS[key]]));
// The page once had a third mark, "must include" ('req' in roster and outfits); a saved or imported one reads as owned.
export function dropRequired(settings) {
  for (const key of ['roster', 'outfits']) settings[key] = Object.fromEntries(Object.entries(settings[key] || {}).filter(([, mark]) => mark !== 'req'));
}
dropRequired(state);
// a target the scenario hands out as a gold whatever the deck (Unity Cup: No Stopping Me!, It's On!); nothing else is
// guaranteed outside the deck, so the player never sets it
/** @type {(group: number) => boolean} */
export const goldGiven = (group) => (SCEN_GOLDS[state.scen || 'unity'] || []).includes(group);
/**
 * The planner's status line when no search runs (search.js paintRunNote).
 * @typedef {object} Status
 * @property {'done' | 'example' | 'error' | 'empty' | 'stopped' | 'pending' | 'loading'} kind
 * @property {string} text
 * @property {{label: string, click?: string, run?: Function}} [act] the button beside it: `click` the id of the button it
 *   presses, else `run` called
 */
/**
 * What is on screen (not saved).
 * @typedef {object} UiState
 * @property {Status} status
 * @property {string} headline the top result's odds ("1 in 139"), for the phone's floating button
 * @property {(() => void) | null} lastRender redraws the results on screen
 * @property {{kind: string, r: import('./results.js').LoopReport, meta: {example?: boolean}} | null} lastResult the result on
 *   screen, for Export
 */
/** @type {UiState} */
export const ui = { status: { kind: 'loading', text: '' }, headline: '', lastRender: null, lastResult: null };
export function writeSaved() {
  SAVED.common = Object.fromEntries(COMMON.map(key => [key, state[key]]));
  SAVED[ERA] = Object.fromEntries(PER_ERA.map(key => [key, state[key]]));
  persist();
}
