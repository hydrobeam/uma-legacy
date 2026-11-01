// ---------- a render error in one of a page's views: reported, and said on the status line ----------
// Every view of both pages draws inside one (each view in its place in the page's layout, and inside each dialog, so a throw leaves the
// dialog itself open and closable). A throw while drawing (a malformed record, a bug) would otherwise escape in a
// microtask, after the run has settled, and leave the status line saying the run is done over a half-updated view. Here
// it is reported as an uncaught error is (errorlog.ts logs it), the status line says what could not be drawn (as a
// run's own errors do: each page names how, `sayRenderErrors`), and the view shows `fallback` until what it draws
// (`input`) is replaced, when it draws again.
import type { ReadonlySignal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useErrorBoundary, useRef } from 'preact/hooks';

interface GuardProps {
  /** what the view draws (its record): a new one is drawn again after a failure. The status line's write must not
   *  replace it, or the view would fail again on the record that write made */
  input: ReadonlySignal<unknown>;
  /** the view's name on the status line ("The result"); none for the status line's own parts, which cannot say it */
  what?: string | undefined;
  /** shown in the view's place while it cannot be drawn */
  fallback?: ComponentChildren;
  /** what a failure gives up (a popup's request: nothing then waits on a popup that is gone) */
  onFail?: () => void;
  children: ComponentChildren;
}
// the page's status line, told of a view that could not be drawn (each entry sets it once, before its first render)
let say: ((text: string) => void) | null = null;
/** how this page's status line says that a view could not be drawn */
export function sayRenderErrors(on: (text: string) => void) {
  say = on;
}
const NOT_FAILED = Symbol('not failed');
export function RenderGuard({ input, what, fallback = null, onFail, children }: GuardProps) {
  // the record it failed on: a new one (the store replaced it) is tried again
  const failedOn = useRef<unknown>(NOT_FAILED);
  const [error] = useErrorBoundary((thrown: unknown) => {
    failedOn.current = input.peek();
    reportError(thrown);
    if (what) say?.(`${what} could not be drawn: ${errorText(thrown)}`);
    onFail?.();
  });
  return error && input.value === failedOn.current ? fallback : children;
}
const errorText = (thrown: unknown) => (thrown instanceof Error ? thrown.message : String(thrown));
