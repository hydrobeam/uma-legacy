// ---------- the Umas panel: All / None owned, Import, the count; the filter and the order; one tile per outfit ----------
// A tile's look follows her ownership (state/roster.ts tileState) and the filter, each bound to its own computed, so a
// click or a keystroke changes the attributes it moves and redraws no tile; the list redraws only when its order
// changes. Drawn in the Umas dialog (setup/Setup.tsx).
import { computed, signal, useComputed } from '@preact/signals';
import { Fragment } from 'preact';
import { useRef } from 'preact/hooks';
import { gameData } from '../model/data.ts';
import type { Outfit, Uma } from '../model/data.ts';
import { fields } from '../state/settings.ts';
import { ownAllUmas, ownNoUmas, rosterCounts, tileOutfits, tileState, toggleTile } from '../state/roster.ts';
import { UmaAv } from '../ui/Images.tsx';
import { OwnedImport, useSaid } from './OwnedImport.tsx';
import { SettingsSort } from './SettingsSort.tsx';
import { useMoreFade } from '../ui/moreFade.ts';
import { Button } from '../ui/Button.tsx';
import { field } from '../ui/Field.ts';
import { OWNED_PANEL } from './OwnedImport.tsx';
import { cx } from 'tailwind-variants/lite';

const STATE_LABEL = { own: 'owned', out: 'not owned' };
const EAGER = 8; // the first two rows are on screen when the dialog opens (phone and desktop): their portraits load at once
// the filter's text, trimmed and lower-cased: a tile hides unless her name or outfit holds it
const query = signal('');
// the tiles in the chosen order (release: the outfit's; name: hers, then the outfit's)
const tiles = computed(() => {
  const dir = fields.rdir.value === 'desc' ? -1 : 1,
    byName = fields.rsort.value === 'name';
  const list = gameData().umas.flatMap((u) =>
    tileOutfits(u).map((o) => ({ u, o, key: `${u.id}:${o?.id ?? 0}`, name: `${u.name} ${o ? o.label : ''}` })),
  );
  return list.sort(
    (a, b) =>
      dir *
      (byName ? a.name.localeCompare(b.name) : (a.o?.rel ?? 0) - (b.o?.rel ?? 0) || (a.o?.id ?? 0) - (b.o?.id ?? 0)),
  );
});
const countText = computed(() => {
  const n = rosterCounts.value;
  return `${n.umasOwned} of ${n.umas} umas · ${n.outfitsOwned} of ${n.outfits} outfits`;
});

// her tile: the portrait is the toggle (its name says owned or not owned; the tile's look shows it: data-s="out" greys
// her portrait and name, the theme's `unowned:`), her name and the outfit (two lines at most; the hover has it whole)
function RosterTile({ u, o, eager }: { u: Uma; o: Outfit | null; eager: boolean }) {
  const who = u.name + (o ? ` ${o.label}` : ''),
    name = u.name.toLowerCase(),
    outfit = o ? o.label.toLowerCase() : '';
  // (the computeds read props: right because each tile is keyed by her outfit, so its props never change)
  const mark = useComputed(() => tileState(u, o, { roster: fields.roster.value, outfits: fields.outfits.value })),
    label = useComputed(() => `${who}: ${STATE_LABEL[mark.value]}`),
    hidden = useComputed(() => !!query.value && !name.includes(query.value) && !outfit.includes(query.value));
  return (
    <div class="grid min-w-0 gap-1" data-uma={u.id} data-s={mark} hidden={hidden}>
      {/* its own <button>: an uma's tile in the picker, the portrait the toggle */}
      <button
        class="block rounded-xl p-0"
        type="button"
        aria-label={label}
        title={who}
        onClick={() => toggleTile(u, o)}
      >
        <UmaAv uma={u.id} outfit={o ? o.id : null} size={96} shape="tile" dim="unowned" eager={eager} />
      </button>
      <div class="text-note unowned:text-muted text-center leading-tight font-semibold wrap-anywhere">{u.name}</div>
      {o && (
        <div class="text-caption text-muted line-clamp-2 text-center leading-tight wrap-anywhere" title={o.label}>
          {o.label}
        </div>
      )}
    </div>
  );
}
function RosterList({ fadeMore }: { fadeMore: boolean }) {
  const box = useRef<HTMLDivElement>(null);
  useMoreFade(box, fadeMore);
  return (
    <div
      class="data-more:fade-bottom grid min-h-0 flex-auto [scrollbar-gutter:stable] grid-cols-[repeat(auto-fill,minmax(84px,1fr))] items-start gap-x-2 gap-y-2.5 overflow-auto py-1 pr-3 pl-1"
      id="roster"
      ref={box}
    >
      {tiles.value.map((t, i) => (
        <RosterTile key={t.key} u={t.u} o={t.o} eager={i < EAGER} />
      ))}
    </div>
  );
}
export function RosterPanel({ fadeMore }: { fadeMore: boolean }) {
  const [said, say] = useSaid();
  return (
    <>
      <div class={OWNED_PANEL.top}>
        {/* (the dialog's head shows the title: this one only names the panel) */}
        <h2 class="hidden" id="h-roster">
          Umas
        </h2>
        <div class={OWNED_PANEL.tools}>
          <Button id="r-all" onClick={ownAllUmas}>
            All owned
          </Button>
          <Button id="r-none" onClick={ownNoUmas}>
            None owned
          </Button>
          <OwnedImport id="r-file" onSaid={say} />
          <span class={OWNED_PANEL.count} id="r-count" role="status">
            {countText}
          </span>
        </div>
      </div>
      <div class={OWNED_PANEL.filters}>
        <input
          type="search"
          id="r-filter"
          class={cx(field.tap, 'flex-1')}
          aria-label="Filter umas"
          placeholder="Filter by name or outfit"
          onInput={(e) => {
            query.value = e.currentTarget.value.trim().toLowerCase();
          }}
        />
        <SettingsSort prefix="r" id="rs" what="sort umas" />
      </div>
      <p class={OWNED_PANEL.hint}>
        {'Tap a portrait to mark it owned or not owned. '}
        <span id="r-msg" role="status">
          <Fragment key={said.n}>{said.text}</Fragment>
        </span>
      </p>
      <RosterList fadeMore={fadeMore} />
    </>
  );
}
