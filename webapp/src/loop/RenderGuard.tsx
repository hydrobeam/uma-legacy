// ---------- a render error in one of the loop page's views: reported, and said on the status line ----------
// Every view of the page draws inside one (main.tsx, and inside each dialog). A throw while drawing (a malformed report,
// a bug) would otherwise escape in a microtask, after the search has settled, and leave the status line saying the
// search is done over a half-updated view. Here it is reported as an uncaught error is (errorlog.ts logs it), the status
// line says what could not be drawn (as the search's own errors do), and the view shows `fallback` until what it draws
// (`input`) is replaced, when it draws again.
import type { ReadonlySignal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { useErrorBoundary, useRef } from 'preact/hooks';
import { settle } from './state.ts';

interface GuardProps {
  /** what the view draws (its record): a new one is drawn again after a failure */
  input: ReadonlySignal<unknown>;
  /** the view's name on the status line ("The result"); none for the status line's own parts, which cannot say it */
  what?: string;
  /** shown in the view's place while it cannot be drawn */
  fallback?: ComponentChildren;
  children: ComponentChildren;
}
const NOT_FAILED = Symbol('not failed');
export function RenderGuard({ input, what, fallback = null, children }: GuardProps) {
  // the record it failed on: a new one (the store replaced it) is tried again
  const failedOn = useRef<unknown>(NOT_FAILED);
  const [error] = useErrorBoundary((thrown: unknown) => {
    failedOn.current = input.peek();
    reportError(thrown);
    if (what) settle({ kind: 'error', text: `${what} could not be drawn: ${errorText(thrown)}` });
  });
  return error && input.value === failedOn.current ? fallback : children;
}
const errorText = (thrown: unknown) => (thrown instanceof Error ? thrown.message : String(thrown));
