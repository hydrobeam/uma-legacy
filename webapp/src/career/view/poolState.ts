// The veterans dialog's state (Pool.tsx draws it): the tab on show, the one chosen beside the list, the list's order,
// whether the dialog has opened and whether it is open, and each tab's veterans in that order. Apart from the
// component, so modules that are not views (io.ts's imports switch the tab) take it without importing a view.
import { batch, computed, signal } from '@preact/signals-core';
import type { ReadonlySignal } from '@preact/signals-core';
import { settings } from '../../settings.ts';
import { vets } from '../../veterans/store.ts';
import type { VetRecord } from '../../veterans/store.ts';

export type Tab = 'own' | 'bor';
const tab = signal<Tab>('own'),
  chosen = signal<string | null>(null),
  dialogOpened = signal(false),
  dialogOpen = signal(false),
  ordered = signal({ by: 'tg', reversed: false });
/** the tab on show */
export const poolTab: ReadonlySignal<Tab> = tab;
/** the dialog has opened (its cards are drawn from then on) */
export const opened: ReadonlySignal<boolean> = dialogOpened;
/** the dialog is open (the pane beside the lists is drawn only then) */
export const poolOpen: ReadonlySignal<boolean> = dialogOpen;
/** the list's order: VSORTS' key, reversed or not */
export const order: ReadonlySignal<{ by: string; reversed: boolean }> = ordered;
/** her tab: Guests (borrowed) or yours */
export const inTab = (v: VetRecord, t: Tab = tab.peek()) => !!v.borrowed === (t === 'bor');
// the list's orders: the order added (uma.moe's listings in uma.moe's order), how many targets she carries (the
// default; ties in the order added), her rank; each starts as the menu says and picking the chosen one again reverses
// it (as the umas' and cards' sort menus)
export const VSORTS: Record<string, [string, string, (v: VetRecord, i: number) => number]> = {
  no: ['Added', 'In the order they were added to this page (uma.moe’s: in its own order)', (_v, i) => -i],
  tg: [
    'Targets',
    'The most targets she carries first',
    (v) => settings.value.targets.filter((t) => (v.sparks as Record<number, number>)?.[t.group] > 0).length,
  ],
  rank: ['Rank', 'The highest career rank first', (v) => v.rank || 0],
};
// each tab's veterans in the order chosen, with their numbers
const listOf = (t: Tab) => {
  const { by, reversed } = order.value,
    score = VSORTS[by][2],
    sign = reversed ? -1 : 1;
  return vets.value
    .map((v, i) => [v, i] as const)
    .filter(([v]) => inTab(v, t))
    .sort(([a, i], [b, j]) => sign * (score(b, j) - score(a, i)));
};
export const lists = { own: computed(() => listOf('own')), bor: computed(() => listOf('bor')) };
/** the one beside the list: the one chosen while she is in this tab, else the first shown */
export const selected = computed(() => {
  const list = lists[tab.value].value;
  return list.some(([v]) => v.id === chosen.value) ? chosen.value : (list[0]?.[0].id ?? null);
});
/** the tab on show: the one chosen there stays chosen (as the list drawn on a switch chose her) */
export function setVTab(t: Tab) {
  tab.value = t;
  chosen.value = selected.peek();
}
/** the dialog opens: its cards are drawn (they stay, and follow every change, from then on: a tab switch or a new
 *  opening builds none), and the pane beside them */
export function openPool() {
  batch(() => {
    dialogOpened.value = true;
    dialogOpen.value = true;
  });
}
/** the dialog has closed: the pane draws nothing (an edit elsewhere redraws no closed pane) */
export function poolClosed() {
  dialogOpen.value = false;
}
const note = signal('');
/** what the dialog says last (an import, a removal, a veteran added: its status line) */
export const vetsNote: ReadonlySignal<string> = note;
export const sayVets = (text: string) => {
  note.value = text;
};
/** her, beside the list */
export function choose(id: string) {
  chosen.value = id;
}
/** an order of the menu: the one in use again reverses it; the one chosen stays, while on show */
export function setOrder(by: string) {
  const now = ordered.peek();
  ordered.value = { by, reversed: by === now.by && !now.reversed };
  chosen.value = selected.peek();
}
