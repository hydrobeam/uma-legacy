// ---------- a render root's error boundary: a view that throws is reported (errorlog.ts) and shows one line ----------
// Without it a render error, which runs in a microtask outside every caller's try, would leave the root half drawn and
// say nothing.
import type { ComponentChildren, VNode } from 'preact';
import { useErrorBoundary } from 'preact/hooks';

export function Boundary({ children }: { children: ComponentChildren }) {
  const [error] = useErrorBoundary((e) => reportError(e));
  return error ? <span class="hint">This part of the page could not be drawn.</span> : (children as VNode);
}
