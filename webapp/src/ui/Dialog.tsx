// ---------- a modal dialog: the native <dialog>, shown and closed by its `open` prop ----------
// Preact has no modal `open` (the attribute opens a dialog non-modally: no backdrop, no inert page), so showModal() and
// close() are called from a layout effect, before the browser paints. The native close (Escape, or close() from the
// effect) reaches `onClose`, so whoever holds `open` hears every close. A click on the backdrop (the dialog element
// itself is the target) closes it. The browser returns the focus to the element that had it when the dialog opened.
import type { ComponentChildren } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { must } from '../must.ts';

interface DialogProps {
  id: string;
  /** the id of its heading */
  labelledBy: string;
  open: boolean;
  /** the dialog has closed (or is closing): set `open` false */
  onClose: () => void;
  children: ComponentChildren;
}
export function Dialog({ id, labelledBy, open, onClose, children }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);
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
      aria-labelledby={labelledBy}
      onClose={onClose}
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      {children}
    </dialog>
  );
}
