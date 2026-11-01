// ---------- a scrolling box's fade: data-more while there is more to scroll to below (its `data-more:fade-bottom`) ----------
// Kept current on a scroll, a change in the box (a filter hides tiles) and a resize (also when its dialog opens: its
// size goes from 0). The loop page's roster and card boxes have it.
import type { RefObject } from 'preact';
import { useLayoutEffect } from 'preact/hooks';

export function useMoreFade(box: RefObject<HTMLElement | null>, on: boolean) {
  useLayoutEffect(() => {
    const element = box.current;
    if (!on || !element) return;
    const update = () =>
      element.toggleAttribute('data-more', element.scrollTop + element.clientHeight < element.scrollHeight - 4); // 4 px: rounding slack
    update();
    element.addEventListener('scroll', update, { passive: true });
    const changes = new MutationObserver(update);
    changes.observe(element, { childList: true, subtree: true, attributes: true, attributeFilter: ['hidden'] });
    const sizes = window.ResizeObserver ? new ResizeObserver(update) : null;
    sizes?.observe(element);
    return () => {
      element.removeEventListener('scroll', update);
      changes.disconnect();
      sizes?.disconnect();
    };
  }, [box, on]);
}
