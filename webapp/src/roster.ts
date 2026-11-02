// ---------- roster: what you own of the umas, per outfit (solver: exclude + exclude_outfits) ----------
// Her ownership and a tile's, a tile's click, All / None owned and the counts; the outfit a result names for her. The
// Umas panel that draws them is ui/Roster.tsx.
import { computed } from '@preact/signals';
import { gameData } from './model/data.ts';
import { fields, settings, setOwned } from './settings.ts';
import { isOwned } from './model/job.ts';
import type { Uma, Outfit } from './model/data.ts';
import type { SettingsView } from './settings.ts';
// her ownership: 'out' when marked not owned or every outfit of hers is, else her roster mark ('own' by default)
export function umaState(u: Uma): 'own' | 'out' {
  const outfits = u.outfits || [],
    mark = settings.value.roster[u.id] || 'own';
  return mark === 'own' && outfits.length && outfits.every((o) => settings.value.outfits[o.id] === false)
    ? 'out'
    : mark;
}
// one tile's ownership under the marks `owned`: the uma's mark, else the outfit's (o null: an uma without outfits in
// the data)
export const tileState = (u: Uma, o: Outfit | null, owned: Pick<SettingsView, 'roster' | 'outfits'>): 'own' | 'out' =>
  owned.roster[u.id] === 'out' || (o && owned.outfits[o.id] === false) ? 'out' : 'own';
// her tiles' outfits: one tile per outfit, or a single tile (null) when the data lists none
export const tileOutfits = (u: Uma): (Outfit | null)[] => (u.outfits?.length ? u.outfits : [null]);
// The outfit a result names for uma `id`: one, never "any outfit". `offered`: the solver's outfits for her (each covers a
// target the same way; empty when none of hers covers one). Among the offered, the first; with none offered, among your
// owned outfits her original (<id>01), else the first.
// Returns { list: the chosen first, then the other offered ones (equally good), any: no outfit of hers covers a target }
export type OutfitPick = { list: readonly { id: number; label: string }[]; any: boolean };
export function pickOutfit(id: number, offered: readonly { id: number; label: string }[] = []): OutfitPick {
  const u = gameData().umas.find((x) => x.id === id),
    mine = (u?.outfits || []).filter((o) => settings.value.outfits[o.id] !== false);
  if (offered.length) return { list: offered, any: false };
  const chosen = mine.find((o) => o.id === id * 100 + 1) || mine[0];
  return { list: chosen ? [{ id: chosen.id, label: chosen.label }] : [], any: true };
}
// what her popup says about the outfit chosen: that any of hers would do, or the equally good others
export const outfitNote = (pick: OutfitPick): string =>
  pick.any
    ? 'Any outfit works: none of hers covers a target.'
    : pick.list.length > 1
      ? `Also works: ${pick.list
          .slice(1)
          .map((o) => o.label)
          .join(', ')}.`
      : '';
// what is owned, counted: umas and outfits (a tile each: an uma without outfits counts as one), owned and in all
export const rosterCounts = computed(() => {
  const s = { roster: fields.roster.value, outfits: fields.outfits.value },
    umas = gameData().umas;
  let outfits = 0,
    outfitsOwned = 0;
  for (const u of umas)
    for (const o of tileOutfits(u)) {
      outfits++;
      if (tileState(u, o, s) === 'own') outfitsOwned++;
    }
  return { umasOwned: umas.filter((u) => isOwned(u, s)).length, umas: umas.length, outfitsOwned, outfits };
});
// a click on a tile: owned <-> not owned, for its outfit (or the uma, on a tile without one); saved
export function toggleTile(u: Uma, o: Outfit | null) {
  const roster = { ...settings.value.roster },
    outfits = { ...settings.value.outfits };
  if (tileState(u, o, settings.value) === 'own') {
    if (o) outfits[o.id] = false;
    else roster[u.id] = 'out';
  } else if (!o) delete roster[u.id];
  else {
    // a whole-uma 'not owned' becomes per outfit, so only this tile changes
    if (roster[u.id] === 'out') {
      delete roster[u.id];
      for (const x of u.outfits) if (outfits[x.id] === undefined) outfits[x.id] = false;
    }
    delete outfits[o.id];
  }
  setOwned({ roster, outfits });
}
export const ownAllUmas = () => setOwned({ roster: {}, outfits: {} });
export const ownNoUmas = () =>
  setOwned({
    roster: {
      ...settings.value.roster,
      ...Object.fromEntries(gameData().umas.map((u) => [u.id, 'out' as const])),
    },
  });
