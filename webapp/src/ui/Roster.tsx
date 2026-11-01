// ---------- the Umas panel: All / None owned, Import, the count; the filter and the order; one tile per outfit ----------
// A tile's look follows her ownership (roster.ts tileState) and the filter, each bound to its own computed, so a click
// or a keystroke changes the attributes it moves and redraws no tile; the list redraws only when its order changes.
// Drawn into the template's #umas-panel (`mountRoster`).
import { computed, signal, useComputed } from '@preact/signals';
import { Fragment } from 'preact';
import { renderGuarded } from './RenderGuard.tsx';
import { useRef } from 'preact/hooks';
import { $ } from '../dom.ts';
import { gameData } from '../data.ts';
import type { Outfit, Uma } from '../data.ts';
import { fields, settings } from '../settings.ts';
import { ownAllUmas, ownNoUmas, rosterCounts, tileOutfits, tileState, toggleTile } from '../roster.ts';
import { UmaAv } from './Images.tsx';
import { OwnedImport, useSaid } from './OwnedImport.tsx';
import { SettingsSort } from './SortMenu.tsx';
import { useMoreFade } from './moreFade.ts';

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

// her tile: the portrait is the toggle (its name says owned or not owned; the tile's look shows it), her name and the
// outfit (two lines at most, by CSS; the hover has it whole)
function RosterTile({ u, o, eager }: { u: Uma; o: Outfit | null; eager: boolean }) {
  const who = u.name + (o ? ` ${o.label}` : ''),
    name = u.name.toLowerCase(),
    outfit = o ? o.label.toLowerCase() : '';
  // (the computeds read props: right because each tile is keyed by her outfit, so its props never change)
  const mark = useComputed(() => tileState(u, o, { roster: fields.roster.value, outfits: fields.outfits.value })),
    label = useComputed(() => `${who}: ${STATE_LABEL[mark.value]}`),
    hidden = useComputed(() => !!query.value && !name.includes(query.value) && !outfit.includes(query.value));
  return (
    <div class="ut" data-uma={u.id} data-s={mark} hidden={hidden}>
      <button class="up" type="button" aria-label={label} title={who} onClick={() => toggleTile(u, o)}>
        <UmaAv uma={u.id} outfit={o ? o.id : null} size={96} eager={eager} />
      </button>
      <div class="un">{u.name}</div>
      {o && (
        <div class="uol" title={o.label}>
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
    <div class="roster" id="roster" ref={box}>
      {tiles.value.map((t, i) => (
        <RosterTile key={t.key} u={t.u} o={t.o} eager={i < EAGER} />
      ))}
    </div>
  );
}
function RosterPanel({ fadeMore }: { fadeMore: boolean }) {
  const [said, say] = useSaid();
  return (
    <>
      <div class="row between">
        <h2 id="h-roster">Umas</h2>
        <div class="row">
          <button class="btn" id="r-all" type="button" onClick={ownAllUmas}>
            All owned
          </button>
          <button class="btn" id="r-none" type="button" onClick={ownNoUmas}>
            None owned
          </button>
          <OwnedImport id="r-file" onSaid={say} />
          <span class="hint cnt" id="r-count" role="status">
            {countText}
          </span>
        </div>
      </div>
      <div class="row rtools">
        <input
          type="search"
          id="r-filter"
          aria-label="Filter umas"
          placeholder="Filter by name or outfit"
          onInput={(e) => {
            query.value = e.currentTarget.value.trim().toLowerCase();
          }}
        />
        <SettingsSort prefix="r" id="rs" what="sort umas" />
      </div>
      <p class="hint">
        {'Tap a portrait to mark it owned or not owned. '}
        <span id="r-msg" role="status">
          <Fragment key={said.n}>{said.text}</Fragment>
        </span>
      </p>
      <RosterList fadeMore={fadeMore} />
    </>
  );
}
/** the Umas panel into its mount point; `fadeMore`: the list's bottom edge fades while there is more below */
export function mountRoster(fadeMore: boolean) {
  renderGuarded(<RosterPanel fadeMore={fadeMore} />, $('umas-panel'), 'The Umas panel', settings);
}
