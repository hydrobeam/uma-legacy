// The career page's render guard (ui/RenderGuard.tsx): a view that throws while drawing shows nothing, the status line
// says what could not be drawn, and the view draws again once what the page draws changes: the career on show or the
// plan, the veterans, the places or the settings, and the guard's own input (`also`: a popup's subject). The status
// line's write changes none of these, so saying the failure redraws nothing.
import { computed, useComputed } from '@preact/signals';
import type { ReadonlySignal } from '@preact/signals';
import type { ComponentChildren } from 'preact';
import { settings } from '../../settings.ts';
import { places, vets } from '../../veterans/store.ts';
import { RenderGuard } from '../../ui/RenderGuard.tsx';
import { answer } from '../state.ts';

// (each part a computed of its own: a status-only write of the answer keeps `last` and `planState`, so neither moves)
const careerShown = computed(() => answer.value.last),
  planShown = computed(() => answer.value.planState);
const pageDrawn = computed(() => [careerShown.value, planShown.value, vets.value, places.value, settings.value]);
const NONE: ReadonlySignal<null> = computed(() => null);
interface GuardProps {
  /** the view's name on the status line ("The node popup"); none for the status line itself */
  what?: string | undefined;
  /** what else the view draws from (a popup's subject): a new value draws it again too */
  also?: ReadonlySignal<unknown> | undefined;
  children: ComponentChildren;
}
export function Guard({ what, also = NONE, children }: GuardProps) {
  const input = useComputed(() => [pageDrawn.value, also.value]);
  return (
    <RenderGuard input={input} what={what}>
      {children}
    </RenderGuard>
  );
}
