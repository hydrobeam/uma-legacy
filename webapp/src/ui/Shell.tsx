// ---------- the planner pages' shell: what both pages' layouts (career/Layout.tsx, loop/Layout.tsx) draw alike ----------
// The layouts are drawn twice: at build time into each page's HTML (vite/prerender.mjs, renderToString), so the setup's
// markup is there before any script runs, and on load by the entry, which hydrates that markup (one Preact root per
// page). So these modules import nothing but Preact: no store, no game data, no module with an effect at load. What
// the entry knows and the build does not (the era, the counts, the handlers) comes in as props once the entry has
// hydrated (`useHydrated`): until then each layout draws exactly what the build drew.
import type { ComponentChildren, Ref } from 'preact';
import { useLayoutEffect, useState } from 'preact/hooks';
import { ERA_NAMES } from '../eras.ts';
import type { Era } from '../eras.ts';
import { SegmentedControl } from './SegmentedControl.tsx';

/** false for the first draw (the hydration, which must draw what the build drew), true from the next */
export function useHydrated() {
  const [hydrated, setHydrated] = useState(false);
  useLayoutEffect(() => setHydrated(true), []);
  return hydrated;
}

/** the game data's era switch: the era on show pressed (Global at build time); a press of the other reloads into it */
export function EraRow({
  era = 'global',
  compact = false,
  onPick,
}: {
  era?: Era | undefined;
  /** the loop page's smaller setup rows */
  compact?: boolean;
  onPick?: ((era: Era) => void) | undefined;
}) {
  return (
    <div class="gctl setrow">
      <span class="glab" id="era-l">
        Game data
      </span>
      <SegmentedControl<Era>
        kind="toggle"
        shape="fill"
        size={compact ? 'md' : 'lg'}
        labelledBy="era-l"
        value={era}
        onPick={(v) => onPick?.(v)}
        options={[
          { v: 'global', id: 'era-global', title: "Global, today's data", body: ERA_NAMES.global },
          {
            v: 'gm',
            id: 'era-gm',
            title: "JP data from the Grand Masters era (cut at L'Arc's launch, 24 Aug 2023)",
            body: ERA_NAMES.gm,
          },
        ]}
      />
    </div>
  );
}

/** a count on an Enhance button: its badge and its tooltip (none until the entry has counted) */
export interface Badge {
  n: string;
  title: string;
}
/** the Umas and Support cards buttons (ui/Setup.tsx: their dialogs and counts) */
export function EnhButtons({
  umas,
  cards,
  onUmas,
  onCards,
}: {
  umas?: Badge | undefined;
  cards?: Badge | undefined;
  onUmas?: (() => void) | undefined;
  onCards?: (() => void) | undefined;
}) {
  return (
    <>
      <EnhButton id="o-umas" label="Umas" badge={umas} onClick={onUmas} />
      <EnhButton id="o-cards" label="Support cards" badge={cards} onClick={onCards} />
    </>
  );
}
function EnhButton({
  id,
  label,
  badge,
  onClick,
}: {
  id: string;
  label: string;
  badge: Badge | undefined;
  onClick: (() => void) | undefined;
}) {
  return (
    <button
      class="enh"
      type="button"
      id={id}
      aria-haspopup="dialog"
      data-n={badge?.n}
      title={badge?.title}
      onClick={onClick}
    >
      <span class="enhimg" aria-hidden="true" />
      <span class="enhl">{label}</span>
    </button>
  );
}

/** the "+" list's box beside a node's sparks (veterans/Editors.tsx AddList fills it and places it; hidden while shut) */
export const AddPop = ({
  hidden = true,
  boxRef,
  children,
}: {
  hidden?: boolean;
  boxRef?: Ref<HTMLDivElement>;
  children?: ComponentChildren;
}) => (
  <div
    ref={boxRef ?? null}
    class="cpop addpop"
    id="add-pop"
    role="dialog"
    aria-label="Add a spark"
    tabIndex={-1}
    hidden={hidden}
  >
    {children}
  </div>
);

/** the footer: the page's own button and the version the build stamps */
export function Footer({ version, children }: { version: string; children: ComponentChildren }) {
  return (
    <footer class="foot">
      <p class="row">
        {children}
        <span class="hint" id="ver">{`Version ${version}`}</span>
      </p>
    </footer>
  );
}
