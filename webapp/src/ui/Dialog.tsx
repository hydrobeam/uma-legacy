// ---------- a modal dialog: the native <dialog>, shown and closed by its `open` prop ----------
// Preact has no modal `open` (the attribute opens a dialog non-modally: no backdrop, no inert page), so showModal() and
// close() are called from a layout effect, before the browser paints. The native close (Escape, or close() from the
// effect) reaches `onClose`, so whoever holds `open` hears every close. A click on the backdrop (the dialog element
// itself is the target) closes it. The browser returns the focus to the element that had it when the dialog opened.
// A deck card's panel opened from inside it shows inside it (ui/CardPop.tsx: the page behind a modal is out of reach).
import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { must } from '../must.ts';
import { CardPop } from './CardPop.tsx';

interface DialogProps {
  id: string;
  cls?: string;
  /** the id of its heading */
  labelledBy?: string;
  /** its name, when no heading names it */
  label?: string;
  open: boolean;
  /** the dialog has closed (or is closing): set `open` false */
  onClose: () => void;
  children: ComponentChildren;
}
// how many dialogs are open now (by their `open` prop, which every close reaches through `onClose`)
let openDialogs = 0;
/** some dialog is open (a popup closing hands the focus back only onto a page with none open) */
export const anyDialogOpen = () => openDialogs > 0;
export function Dialog({ id, cls, labelledBy, label, open, onClose, children }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null),
    // (for its unmount: the props of its last render)
    last = useRef({ open, onClose });
  last.current = { open, onClose };
  // unmounted while open (a render guard's fallback took its place): it says it closed, so whoever holds `open` hears
  // it, as from a close
  useLayoutEffect(
    () => () => {
      if (last.current.open) last.current.onClose();
    },
    [],
  );
  useLayoutEffect(() => {
    if (!open) return;
    openDialogs++;
    return () => {
      openDialogs--;
    };
  }, [open]);
  useLayoutEffect(() => {
    const dialog = must(ref.current, 'the dialog is mounted');
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);
  return (
    // biome-ignore lint/a11y/useKeyWithClickEvents: the backdrop click is the pointer's way out; Escape (native) is the keyboard's
    <dialog
      ref={ref}
      id={id}
      class={cls}
      aria-labelledby={labelledBy}
      aria-label={label}
      onClose={(e) => {
        // (a close event is queued: one that arrives after the dialog was opened again is stale)
        if (!e.currentTarget.open) onClose();
      }}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {children}
      <CardPop home={id} />
    </dialog>
  );
}
