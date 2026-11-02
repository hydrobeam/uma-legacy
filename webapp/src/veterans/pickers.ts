// ---------- the veterans' editors: which is open, and for what (veterans/Editors.tsx draws them) ----------
// The uma picker, the skills prompt, the G1 wins window and the "+" list (the colour picker is sparks.ts's). An opener
// writes what its editor shows; a choice lands on the record as it is then (`at`, a NodeRef). While a pool veteran's
// G1 wins window or a skills prompt is open the page holds its search (`searchHeld`): the edits made meanwhile reach
// one search when it closes.
import { batch, computed, signal } from '@preact/signals';
import { careerTables } from '../model/data.ts';
import { umaName } from './names.ts';
import type { ReadonlySignal } from '@preact/signals';
import type { NodeRef, Sparks } from './store.ts';
import type { Uma } from '../model/data.ts';
import type { WinInfo } from '../model/agenda.ts';

// ---------- the uma picker: every uma (or those `only` lets through), filterable; onPick(chara id)
export interface UmaPick {
  title: string;
  onPick: (chara: number) => void;
  only: ((u: Uma) => boolean) | null;
  open: boolean;
  /** its opening (each opening starts with an empty filter) */
  n: number;
}
const umaPick = signal<UmaPick | null>(null);
/** the uma picker's request (null: never opened) */
export const umaPicking: ReadonlySignal<UmaPick | null> = umaPick;
export function openUmaPicker(
  title: string,
  onPick: (chara: number) => void,
  only: ((u: Uma) => boolean) | null = null,
): void {
  umaPick.value = { title, onPick, only, open: true, n: (umaPick.peek()?.n ?? 0) + 1 };
}
/** the uma picker has closed (Escape, Close, the backdrop) */
export function umaPickClosed(): void {
  const pick = umaPick.peek();
  if (pick?.open) umaPick.value = { ...pick, open: false };
}
/** an uma picked: the picker closes, then her choice lands (it may open the skills prompt) */
export function umaPicked(id: number): void {
  const pick = umaPick.peek();
  batch(() => {
    umaPickClosed();
    pick?.onPick(id);
  });
}

// ---------- the skills prompt: after an uma is picked for a node that carries sparks, whether to give her the target
// skills; onChoice(sparks | null: keep what she has). Dismissed (Escape), it chooses nothing
export interface SkillsAsk {
  /** her uma's name */
  name: string;
  /** she has sparks to keep ("Keep her current skills" is offered) */
  hasSparks: boolean;
  onChoice: (sparks: Sparks | null) => void;
  open: boolean;
  n: number;
}
const ask = signal<SkillsAsk | null>(null);
/** the skills prompt's question (null: never asked) */
export const skillsAsked: ReadonlySignal<SkillsAsk | null> = ask;
export function askSkills(name: string, hasSparks: boolean, onChoice: (sparks: Sparks | null) => void): void {
  ask.value = { name, hasSparks, onChoice, open: true, n: (ask.peek()?.n ?? 0) + 1 };
}
/** the prompt is dismissed (Escape): nothing chosen */
export function askClosed(): void {
  const now = ask.peek();
  if (now?.open) ask.value = { ...now, open: false };
}
/** a choice: it lands while the prompt still holds the search, then the prompt closes and releases it (that order is what
 *  makes a held search run once, after both); the batch makes the two one notification */
export function skillsChosen(sparks: Sparks | null): void {
  const now = ask.peek();
  batch(() => {
    now?.onChoice(sparks);
    askClosed();
  });
}

// ---------- the G1 wins of one node, on the schedule window's calendar (results/Agenda.tsx WinsEditor): each win in its
// slot, click it to remove it; an empty slot where a G1 is held lists the races held then. Each edit lands on her record
// as it is then (`at`). A pool veteran's wins are saved on each edit, and the page searches again when the window
// closes. opts (the veteran analysis' draft): {offer, info, onClose} (see WinsEditor). winAt {race index: turn} is
// where a win shows, for display only (the solver reads `wins`).
interface WinsOpts {
  offer?: ((turn: number) => number[] | null) | null;
  info?: ((ix: number) => WinInfo | null) | null;
  /** the window's own close (the draft's): the page holds no search for it */
  onClose?: () => void;
}
export interface WinsEdit {
  at: NodeRef;
  who: string;
  opts: WinsOpts;
  open: boolean;
  n: number;
}
const wins = signal<WinsEdit | null>(null);
/** the G1 wins window's node (null: never opened) */
export const winsEditing: ReadonlySignal<WinsEdit | null> = wins;
/** every G1: its race index, name and the turns it is held */
export const allRaces = (): { ix: number; name: string; turns: number[] }[] =>
  careerTables().races.map((name, ix) => ({ ix, name, turns: (careerTables().race_turns || [])[ix] || [] }));
export function openWins(at: NodeRef, who: string, opts: WinsOpts = {}): void {
  wins.value = { at, who, opts, open: true, n: (wins.peek()?.n ?? 0) + 1 };
}
/** the window has closed (Done, Escape, the backdrop) */
export function winsClosed(): void {
  const now = wins.peek();
  if (!now?.open) return;
  wins.value = { ...now, open: false };
  now.opts.onClose?.();
}
export const winsText = (nd: { wins: readonly number[] }): string =>
  nd.wins.length ? `${nd.wins.length} G1 win${nd.wins.length > 1 ? 's' : ''}` : 'No G1 wins';
// a pool veteran's G1 wins window is open (the draft's has its own close)
const poolWinsOpen = computed(() => !!wins.value?.open && !wins.value.opts.onClose);
/** the page holds its search: a pool veteran's G1 wins window or a skills prompt is open */
export const searchHeld = computed(() => poolWinsOpen.value || !!ask.value?.open);

// ---------- "+" beside a node's sparks: the target skills she lacks (Editors.tsx AddList draws them)
const add = signal<{ btn: HTMLElement; at: NodeRef } | null>(null);
/** the "+" list's opener and node (null: closed) */
export const addOpen: ReadonlySignal<{ btn: HTMLElement; at: NodeRef } | null> = add;
/** the "+" whose list is open (null: none): it draws its aria-expanded from this */
export const addOpener = computed(() => add.value?.btn ?? null);
/** "+" pressed: its list opens, or closes when it is open */
export function openAdd(btn: HTMLElement, at: NodeRef): void {
  add.value = add.peek()?.btn === btn ? null : { btn, at };
}
export function closeAdd(): void {
  add.value = null;
}

// ---------- change who a node is: pick the uma (saved, and the page searches again once the prompt is gone), then
// whether to add the target skills (none chosen, as Escape on the prompt: she keeps her sparks)
export function changeUma(at: NodeRef, label: string): void {
  openUmaPicker(`${label}: which uma?`, (id) => {
    at.edit(() => ({ chara: id, outfit: undefined }));
    const sparks = at.node()?.sparks || {};
    askSkills(umaName(id), Object.keys(sparks).length > 0, (sp) => {
      if (sp) at.edit(() => ({ sparks: sp }));
    });
  });
}
