// ---------- the veterans' editors: which is open, and for what (veterans/Editors.tsx draws them) ----------
// The uma picker, the skills prompt, the G1 wins window and the "+" list (the colour picker is sparks.js's). An opener
// writes what its editor shows; a choice lands on the record as it is then (`at`, a NodeRef). While a pool veteran's
// G1 wins window or a skills prompt is open the page holds its search (`searchHeld`): the edits made meanwhile reach
// one search when it closes.
import { batch, computed, signal } from '@preact/signals-core';
import { careerTables } from '../data.js';
import { umaName } from './names.js';
/** @import { ReadonlySignal } from '@preact/signals-core' */
/** @import { NodeRef, Sparks } from './store.ts' */
/** @import { Uma } from '../data.js' */
/** @import { WinInfo } from '../agenda.js' */

// ---------- the uma picker: every uma (or those `only` lets through), filterable; onPick(chara id)
/**
 * @typedef {object} UmaPick
 * @property {string} title
 * @property {(chara: number) => void} onPick
 * @property {((u: Uma) => boolean) | null} only
 * @property {boolean} open
 * @property {number} n  its opening (each opening starts with an empty filter)
 */
const umaPick = signal(/** @type {UmaPick | null} */ (null));
/** the uma picker's request (null: never opened) */
export const umaPicking = /** @type {ReadonlySignal<UmaPick | null>} */ (umaPick);
/** @param {string} title @param {(chara: number) => void} onPick @param {((u: Uma) => boolean) | null} [only] */
export function openUmaPicker(title, onPick, only = null) {
  umaPick.value = { title, onPick, only, open: true, n: (umaPick.peek()?.n ?? 0) + 1 };
}
/** the uma picker has closed (Escape, Close, the backdrop) */
export function umaPickClosed() {
  const pick = umaPick.peek();
  if (pick?.open) umaPick.value = { ...pick, open: false };
}
/** an uma picked: the picker closes, then her choice lands (it may open the skills prompt) @param {number} id */
export function umaPicked(id) {
  const pick = umaPick.peek();
  batch(() => {
    umaPickClosed();
    pick?.onPick(id);
  });
}

// ---------- the skills prompt: after an uma is picked for a node that carries sparks, whether to give her the target
// skills; onChoice(sparks | null: keep what she has). Dismissed (Escape), it chooses nothing
/**
 * @typedef {object} SkillsAsk
 * @property {string} name  her uma's name
 * @property {boolean} hasSparks  she has sparks to keep ("Keep her current skills" is offered)
 * @property {(sparks: Sparks | null) => void} onChoice
 * @property {boolean} withParents  the same choice goes on her two parents
 * @property {boolean} open
 * @property {number} n
 */
const ask = signal(/** @type {SkillsAsk | null} */ (null));
/** the skills prompt's question (null: never asked) */
export const skillsAsked = /** @type {ReadonlySignal<SkillsAsk | null>} */ (ask);
/** @param {string} name @param {boolean} hasSparks @param {(sparks: Sparks|null) => void} onChoice @param {boolean} [withParents] */
export function askSkills(name, hasSparks, onChoice, withParents = false) {
  ask.value = { name, hasSparks, onChoice, withParents, open: true, n: (ask.peek()?.n ?? 0) + 1 };
}
/** the prompt is dismissed (Escape): nothing chosen */
export function askClosed() {
  const now = ask.peek();
  if (now?.open) ask.value = { ...now, open: false };
}
/** a choice: it lands and the prompt closes in one write, so a held search runs once, after both
 *  @param {Sparks | null} sparks */
export function skillsChosen(sparks) {
  const now = ask.peek();
  batch(() => {
    now?.onChoice(sparks);
    askClosed();
  });
}

// ---------- the G1 wins of one node, on the schedule window's calendar (ui/Agenda.tsx WinsEditor): each win in its
// slot, click it to remove it; an empty slot where a G1 is held lists the races held then. Each edit lands on her record
// as it is then (`at`). A pool veteran's wins are saved on each edit, and the page searches again when the window
// closes. opts (the veteran analysis' draft): {offer, info, onClose} (see WinsEditor). winAt {race index: turn} is
// where a win shows, for display only (the solver reads `wins`).
/**
 * @typedef {object} WinsOpts
 * @property {((turn: number) => number[] | null) | null} [offer]
 * @property {((ix: number) => WinInfo | null) | null} [info]
 * @property {() => void} [onClose]  the window's own close (the draft's): the page holds no search for it
 *
 * @typedef {object} WinsEdit
 * @property {NodeRef} at
 * @property {string} who
 * @property {WinsOpts} opts
 * @property {boolean} open
 * @property {number} n
 */
const wins = signal(/** @type {WinsEdit | null} */ (null));
/** the G1 wins window's node (null: never opened) */
export const winsEditing = /** @type {ReadonlySignal<WinsEdit | null>} */ (wins);
/** every G1: its race index, name and the turns it is held */
export const allRaces = () =>
  careerTables().races.map((name, ix) => ({ ix, name, turns: (careerTables().race_turns || [])[ix] || [] }));
/** @param {NodeRef} at @param {string} who @param {WinsOpts} [opts] */
export function openWins(at, who, opts = {}) {
  wins.value = { at, who, opts, open: true, n: (wins.peek()?.n ?? 0) + 1 };
}
/** the window has closed (Done, Escape, the backdrop) */
export function winsClosed() {
  const now = wins.peek();
  if (!now?.open) return;
  wins.value = { ...now, open: false };
  now.opts.onClose?.();
}
/** @type {(nd: {wins: readonly number[]}) => string} */
export const winsText = (nd) =>
  nd.wins.length ? `${nd.wins.length} G1 win${nd.wins.length > 1 ? 's' : ''}` : 'No G1 wins';
// a pool veteran's G1 wins window is open (the draft's has its own close)
const poolWinsOpen = computed(() => !!wins.value?.open && !wins.value.opts.onClose);
/** the page holds its search: a pool veteran's G1 wins window or a skills prompt is open */
export const searchHeld = computed(() => poolWinsOpen.value || !!ask.value?.open);

// ---------- "+" beside a node's sparks: the target skills she lacks (Editors.tsx AddList draws them)
const add = signal(/** @type {{btn: HTMLElement, at: NodeRef} | null} */ (null));
/** the "+" list's opener and node (null: closed) */
export const addOpen = /** @type {ReadonlySignal<{btn: HTMLElement, at: NodeRef} | null>} */ (add);
/** the "+" whose list is open (null: none): it draws its aria-expanded from this */
export const addOpener = computed(() => add.value?.btn ?? null);
/** "+" pressed: its list opens, or closes when it is open @param {HTMLElement} btn @param {NodeRef} at */
export function openAdd(btn, at) {
  add.value = add.peek()?.btn === btn ? null : { btn, at };
}
export function closeAdd() {
  add.value = null;
}

// ---------- change who a node is: pick the uma (saved, and the page searches again once the prompt is gone), then
// whether to add the target skills (none chosen, as Escape on the prompt: she keeps her sparks)
/** @param {NodeRef} at @param {string} label */
export function changeUma(at, label) {
  openUmaPicker(`${label}: which uma?`, (id) => {
    at.edit(() => ({ chara: id, outfit: undefined }));
    const sparks = at.node()?.sparks || {};
    askSkills(umaName(id), Object.keys(sparks).length > 0, (sp) => {
      if (sp) at.edit(() => ({ sparks: sp }));
    });
  });
}
