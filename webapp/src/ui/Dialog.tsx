// The native <dialog> as a component (frontend.md §3): shown and closed by its `open` prop, by calls (showModal and close:
// Preact has no modal `open`, and the `open` property would show it without its backdrop or the top layer), after the
// diff, so its content is in place when it opens; its close event (Escape, its Close, the page's backdrop click) goes to
// `onClose`.
import type { ComponentChildren, DialogHTMLAttributes } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { must } from '../must.ts';

type DialogProps = Omit<DialogHTMLAttributes<HTMLDialogElement>, 'open' | 'onClose'> & {
  open: boolean;
  onClose: () => void;
  children?: ComponentChildren;
};
export function Dialog({ open, onClose, children, ...attrs }: DialogProps) {
  const ref = useRef<HTMLDialogElement>(null);
  useLayoutEffect(() => {
    const dialog = must(ref.current, 'the dialog is rendered');
    if (open && !dialog.open) dialog.showModal();
    else if (!open && dialog.open) dialog.close();
  }, [open]);
  return (
    <dialog ref={ref} {...attrs} onClose={onClose}>
      {children}
    </dialog>
  );
}
