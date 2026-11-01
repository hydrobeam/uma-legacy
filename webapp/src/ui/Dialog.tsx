// ---------- a modal dialog: the native <dialog>, shown and closed by its `open` prop ----------
// Preact has no modal `open` (the attribute opens a dialog non-modally: no backdrop, no inert page), so showModal() and
// close() are called from a layout effect, before the browser paints. The native close (Escape, or close() from the
// effect) reaches `onClose`, so whoever holds `open` hears every close. A click on the backdrop (the dialog element
// itself is the target) closes it. The browser returns the focus to the element that had it when the dialog opened.
// A deck card's panel opened from inside it shows inside it (ui/CardPop.tsx: the page behind a modal is out of reach).
// Two kinds, both the game's: `popup`, a popup (a green title bar, a pale body that scrolls, a footer with Close and the
// actions: DialogTitle, DialogBody, DialogFoot, DialogClose), and `window`, a window (DialogPanel, a body that scrolls
// under its sticky green head, DialogHead, with the title and Close at the right). `size` is its width, one of the
// theme's five (the screen's less the browser's margin, on a phone); every dialog names its own.
import type { ComponentChildren, Ref } from 'preact';
import { useLayoutEffect, useRef } from 'preact/hooks';
import { must } from '../must.ts';
import { Button } from './Button.tsx';
import { CardPop } from './CardPop.tsx';
import { tv } from 'tailwind-variants/lite';
import type { VariantProps } from 'tailwind-variants/lite';

// the frame (the <dialog>): its width, its height, and a popup's column of title, body and footer
const frame = tv({
  base: 'bg-gm-panel font-game text-gm-ink shadow-dialog backdrop:bg-backdrop overflow-hidden rounded-2xl',
  variants: {
    kind: { popup: 'open:flex open:flex-col', window: '' },
    size: {
      sm: 'w-dialog-sm',
      md: 'w-dialog-md',
      lg: 'w-dialog-lg',
      xl: 'w-dialog-xl',
      full: 'w-dialog-full',
    },
    /** as tall as the screen allows less 24 px (else less 48) */
    tall: { true: 'max-h-[calc(100dvh-24px)]', false: 'max-h-[calc(100dvh-48px)]' },
  },
  defaultVariants: { kind: 'window', tall: false },
});
type Frame = VariantProps<typeof frame>;
interface DialogProps {
  id: string;
  /** a popup (title, body, footer) or a window (a panel under a sticky head) */
  kind?: Frame['kind'];
  /** its width: the theme's --container-dialog-* */
  size: NonNullable<Frame['size']>;
  tall?: boolean;
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
export function Dialog({ id, kind, size, tall, labelledBy, label, open, onClose, children }: DialogProps) {
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
      class={frame({ kind, size, tall })}
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

// ---------- a popup's parts ----------
/** its title, in the green bar */
export const DialogTitle = ({ id, children }: { id: string; children: ComponentChildren }) => (
  <h3 id={id} class="art-bar text-shadow-outline text-heading flex-none px-3 py-2 text-center font-black text-white">
    {children}
  </h3>
);
/** its body, which scrolls in the room the title and the footer leave */
export const DialogBody = ({
  id,
  ref = null,
  children,
}: {
  id?: string;
  ref?: Ref<HTMLDivElement>;
  children: ComponentChildren;
}) => (
  <div id={id} ref={ref} class="bg-gm-pane grid min-h-0 flex-auto gap-2.5 overflow-auto px-4 py-3.5">
    {children}
  </div>
);
/** its footer: Close and the dialog's actions, centred, wrapping */
export const DialogFoot = ({ children }: { children: ComponentChildren }) => (
  <div class="bg-gm-panel flex flex-none flex-wrap justify-center gap-2.5 p-3">{children}</div>
);
/** the footer's Close */
export const DialogClose = ({
  onClick,
  ref = null,
  label = 'Close',
}: {
  onClick: () => void;
  ref?: Ref<HTMLButtonElement>;
  label?: string;
}) => (
  <Button ref={ref} size="title" class="min-w-40" onClick={onClick}>
    {label}
  </Button>
);

// ---------- a window's parts ----------
// its panel: the head stays on top as the body scrolls (scroll; its one column the dialog's width, so a wide table
// scrolls inside its own box: Firefox sized an auto column to the table's full width, which the dialog then clipped),
// or nothing scrolls but its contents' own lists (fill), the dialog as tall as the screen allows whatever it holds
// (full)
const panel = tv({
  base: 'bg-gm-panel gap-3.5 p-4.5',
  variants: {
    fit: {
      scroll: 'grid max-h-[calc(100dvh-48px)] grid-cols-1 overflow-auto',
      fill: 'flex max-h-[calc(100dvh-48px)] flex-col overflow-hidden',
      full: 'flex h-[calc(100dvh-48px)] max-h-[calc(100dvh-48px)] flex-col overflow-hidden',
    },
  },
  defaultVariants: { fit: 'scroll' },
});
/** the window's body: its head first, then its contents */
export const DialogPanel = ({
  id,
  fit,
  children,
}: {
  id?: string;
  fit?: VariantProps<typeof panel>['fit'];
  children: ComponentChildren;
}) => (
  <div id={id} class={panel({ fit })}>
    {children}
  </div>
);
/** the window's head, which stays on top as the body scrolls: its title, what the dialog adds beside it, and Close */
export function DialogHead({
  id,
  title,
  tip,
  close,
  closeId,
  onClose,
  children,
}: {
  /** the title's id (the dialog's name, when it has no label of its own) */
  id?: string;
  title: ComponentChildren;
  /** the title's tooltip */
  tip?: string;
  /** Close's label ("Close", "Done"); none without */
  close?: string;
  closeId?: string;
  onClose?: () => void;
  children?: ComponentChildren;
}) {
  return (
    <div class="art-bar sticky -top-4.5 z-6 -mx-4.5 -mt-4.5 flex flex-nowrap items-center justify-between gap-2 px-4.5 pt-3 pb-2.5">
      <h3
        id={id}
        title={tip}
        class="text-shadow-outline text-heading flex-1 text-center font-black text-balance text-white"
      >
        {title}
      </h3>
      {children}
      {close && onClose && (
        <Button id={closeId} onClick={onClose}>
          {close}
        </Button>
      )}
    </div>
  );
}
