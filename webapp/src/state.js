// What the planner page has on screen (ui, not saved). The settings are settings.ts's store.
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
 * @property {{kind: string, r: import('./loop/results.js').LoopReport, meta: {example?: boolean}} | null} lastResult the result on
 *   screen, for Export
 */
/** @type {UiState} */
export const ui = { status: { kind: 'loading', text: '' }, headline: '', lastRender: null, lastResult: null };
