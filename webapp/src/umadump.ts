// ---------- veteran import: umadump's trained_chara_data.json and rental_chara_data.json (github.com/Werseter/umadump) ----------
// Per veteran: card_id (outfit; character = card_id / 100), factor_info_array (sparks: factor_id = factor group * 100 +
// stars; white skill sparks map to a skill group through careerTables().factors), win_saddle_id_array (single-race G1 saddles
// map to a race index through careerTables().saddles; titles are left out) and succession_chara_array (her parents at
// position_id 10 and 20, same fields). A veteran counts as borrowed when owner_viewer_id is set: it is 0 on your own
// veterans and the lender's viewer_id (= viewer_id) on the ones offered for borrowing (rental_chara_data.json, the
// career start screen's borrow list); use_type is 0 on both.
import { careerTables } from './data.ts';
import type { LineageNode, Sparks, Veteran } from './veterans/store.ts';

// The files are the game's JSON (or uma.moe's), typed below by the fields read: their shape is trusted once a record
// is found (`records`, `parseTree`'s type check, uma.moe's `inheritance_id`).
/** a spark as the game lists it */
interface Factor {
  factor_id: number;
}
/** a veteran or a parent in umadump's export */
interface ExportNode {
  card_id: number;
  factor_info_array?: readonly Factor[];
  win_saddle_id_array?: readonly number[];
}
/** a veteran in umadump's export */
interface ExportVeteran extends ExportNode {
  owner_viewer_id?: number;
  /** read as `+x || undefined`: one missing reads as none */
  rank_score: number;
  trained_chara_id?: number;
  succession_chara_array?: readonly (ExportNode & { position_id: number })[];
}
/** a node of uma.moe's lineage planner export */
interface TreeEntry {
  position: string;
  characterId: number;
  sparks?: readonly { factorId: number; level: number }[];
  manualWinSaddleIds?: readonly number[];
}
/** a lineage's three members on uma.moe: the listed parent and her two */
type Side = 'main' | 'left' | 'right';
/** a listing's lineage (uma.moe's `inheritance`): per member, her white factors (a list), her blue, pink and unique
 *  factor (one each) and her win saddles */
type Inheritance = {
  inheritance_id: number;
  main_parent_id: number;
  parent_left_id: number;
  parent_right_id: number;
  affinity_score: number;
  win_count: number;
  white_stars_sum: number;
  parent_rank: number;
} & { [K in `${Side}_white_factors` | `${Side}_win_saddles`]?: readonly number[] } & {
  [K in `${Side}_${'blue' | 'pink' | 'green'}_factors`]?: number;
};
/** One result of uma.moe's search (api/v3/search, `search_type=inheritance`). Its numbers are read as `+x || 0`, so one
 *  missing reads as 0. */
export interface UmamoeListing {
  account_id?: string | number;
  trainer_name?: string;
  follower_num: number;
  last_updated?: string;
  inheritance: Inheritance;
}

// factor_id = factor group * 100 + stars
const factorGroup = (factor: Factor): number => Math.floor(factor.factor_id / 100),
  factorStars = (factor: Factor): number => factor.factor_id % 100;
// sparks as {key: stars}, the most stars per key: `keyOf` gives a factor's key, undefined for a factor of another kind
function bestStars(factors: readonly Factor[] | undefined, keyOf: (group: number) => number | undefined): Sparks {
  const out: Sparks = {};
  for (const factor of factors || []) {
    const key = keyOf(factorGroup(factor));
    if (key !== undefined) out[key] = Math.max(out[key] || 0, factorStars(factor));
  }
  return out;
}
const kept =
  (keep: (group: number) => boolean) =>
  (group: number): number | undefined =>
    keep(group) ? group : undefined;
// white skill sparks, by skill group
const sparksOf = (factors: readonly Factor[] | undefined): Sparks =>
  bestStars(factors, (group) => careerTables().factors[group]);
// aptitude (pink) sparks: the groups in careerTables().pinks
const pinksOf = (factors: readonly Factor[] | undefined): Sparks =>
  bestStars(
    factors,
    kept((group) => careerTables().pinks[group] !== undefined),
  );
// blue (stat) sparks: factor groups 1..5; unique (green) sparks: the outfit's card id as the factor group
const isBlue = (group: number): boolean => group >= 1 && group <= 5,
  isGreen = (group: number): boolean => group >= 100000 && careerTables().uniques?.[group] !== undefined;
// single-race G1 wins as race indices, ascending (titles have no index and are left out)
function winsOf(saddles: readonly number[] | undefined): number[] {
  return [
    ...new Set((saddles || []).map((saddle) => careerTables().saddles[saddle]).filter((race) => race !== undefined)),
  ].sort((a, b) => a - b);
}
/** a veteran or one of her parents as the career page reads her */
const node = (record: ExportNode): LineageNode => ({
  chara: Math.floor(record.card_id / 100),
  outfit: record.card_id,
  sparks: sparksOf(record.factor_info_array),
  pinks: pinksOf(record.factor_info_array),
  blues: bestStars(record.factor_info_array, kept(isBlue)),
  greens: bestStars(record.factor_info_array, kept(isGreen)),
  wins: winsOf(record.win_saddle_id_array),
});
// a parent the export doesn't have
const unknownParent = (): LineageNode => ({ chara: null, sparks: {}, pinks: {}, wins: [] });

// the list of veterans in an export: the file's array, or the first array of veteran records inside an object
function records(json: unknown): ExportVeteran[] {
  // a veteran's record (Array.isArray reads the file's array as any[]: the return type is where its records are typed)
  const isVet = (x: unknown) => x && typeof x === 'object' && 'card_id' in x && 'factor_info_array' in x;
  if (Array.isArray(json)) return json.filter(isVet);
  if (json && typeof json === 'object')
    for (const value of Object.values(json)) {
      const found = records(value);
      if (found.length) return found;
    }
  return [];
}
// uma.moe's lineage planner export ({type: 'lineage-planner', payload: [{position, characterId, sparks, manualWinSaddleIds}]}):
// a veteran per top position ("p1", "p2", ...) with her parents at "p1-1" and "p1-2". A spark's factorId x 10 + its level
// is the game's factor_id (master.mdb succession_factor); saddle ids are the game's win saddles. null when the text is not
// one; its veterans count as yours (the export doesn't say)
export function parseTree(text: string): Veteran[] | null {
  let json: { type?: unknown; payload?: readonly TreeEntry[] } | null;
  try {
    json = JSON.parse(text);
  } catch {
    return null;
  }
  if (json?.type !== 'lineage-planner' || !Array.isArray(json.payload)) return null;
  const at = new Map(json.payload.map((x) => [x.position, x]));
  const treeNode = (x: TreeEntry): LineageNode =>
    node({
      card_id: x.characterId,
      factor_info_array: (x.sparks || []).map((f) => ({ factor_id: f.factorId * 10 + f.level })),
      win_saddle_id_array: x.manualWinSaddleIds,
    });
  return json.payload
    .filter((x) => /^p\d+$/.test(x.position))
    .map((x) => ({
      ...treeNode(x),
      id: `t${Date.now().toString(36)}${Math.random().toString(36).slice(2, 5)}`,
      src: 'tree',
      borrowed: false,
      parents: [1, 2].map((j) => {
        const parent = at.get(`${x.position}-${j}`);
        return parent ? treeNode(parent) : unknownParent();
      }),
    }));
}
// veterans from umadump's export text; throws an Error with a message for the page when it is not an export
export function parseExport(text: string): Veteran[] {
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error("That is not JSON. Pick umadump's trained_chara_data.json or rental_chara_data.json.");
  }
  const found = records(json);
  if (!found.length)
    throw new Error("No veterans found. Pick umadump's trained_chara_data.json or rental_chara_data.json.");
  return found.map((r) => ({
    ...node(r),
    src: 'import',
    borrowed: !!r.owner_viewer_id,
    rank: +r.rank_score || undefined, // her rank score (the badge, careerTables().ranks)
    // trained_chara_id counts per player: a lender's id is kept apart from yours by her viewer id
    id:
      (r.owner_viewer_id ? `tr${r.owner_viewer_id}-` : 'tc') +
      (r.trained_chara_id ?? Math.random().toString(36).slice(2)),
    parents: [10, 20].map((position) => {
      const parent = (r.succession_chara_array || []).find((x) => x.position_id === position);
      return parent ? node(parent) : unknownParent();
    }),
  }));
}
// A lineage listed on uma.moe's search (api/v3/search, `inheritance`): the parent at main_*, her parents at left_* and
// right_*, each with the game's factor ids (white as a list, blue, pink and unique one each) and win saddles. A borrowed
// veteran keyed by the listing (`u` + inheritance_id), with its lender: the trainer to follow in the game
export function parseListing(item: UmamoeListing): Veteran {
  const h = item.inheritance;
  const side = (s: Side, card: number): LineageNode =>
    node({
      card_id: card,
      factor_info_array: (
        [
          ...(h[`${s}_white_factors` as const] || []),
          h[`${s}_blue_factors` as const],
          h[`${s}_pink_factors` as const],
          h[`${s}_green_factors` as const],
        ].filter(Boolean) as number[]
      ).map((factor_id) => ({ factor_id })),
      win_saddle_id_array: h[`${s}_win_saddles` as const],
    });
  return {
    ...side('main', h.main_parent_id),
    id: `u${h.inheritance_id}`,
    src: 'umamoe',
    borrowed: true,
    lender: {
      name: String(item.trainer_name ?? ''),
      id: String(item.account_id ?? ''),
      followers: +item.follower_num || 0,
      updated: String(item.last_updated ?? ''),
      // (uma.moe's own figures, for its sorts)
      affinity: +h.affinity_score || 0,
      wins: +h.win_count || 0,
      sparkStars: +h.white_stars_sum || 0,
    },
    rank: +h.parent_rank || 0, // her rank score (the badge, careerTables().ranks)
    parents: [side('left', h.parent_left_id), side('right', h.parent_right_id)],
  };
}
// ---------- owned umas and support cards: umadump's card_data.json and support_card_data.json ----------
// card_data.json lists the outfits you own, one record each: card_id (the outfit id), rarity (her stars), talent_level
// (her potential), create_time, skill_data_array. support_card_data.json lists the support cards you own:
// support_card_id, limit_break_count, exp, stock (spare copies), extra_data {level, max_level, ...}. The page keeps
// ownership only (no stars, levels or limit breaks), so only the ids are read.
const PICK_OWNED = "Pick umadump's card_data.json or support_card_data.json.";
/** what one file says you own: outfit or support card ids */
interface OwnedList {
  kind: 'outfits' | 'cards';
  ids: number[];
}
// what umadump's owned-umas or owned-cards file lists; throws an Error with a message for the page when it is neither
export function parseOwned(text: string): OwnedList {
  let json: unknown;
  try {
    json = JSON.parse(text);
  } catch {
    throw new Error(`That is not JSON. ${PICK_OWNED}`);
  }
  const list = Array.isArray(json) ? json : [];
  const every = (test: (x: { support_card_id?: unknown; card_id?: unknown }) => boolean): boolean =>
    list.length > 0 && list.every((x) => x && typeof x === 'object' && test(x));
  if (every((x) => Number.isInteger(x.support_card_id)))
    return { kind: 'cards', ids: list.map((x) => x.support_card_id) };
  // (a veteran has a card_id too, with her sparks)
  if (every((x) => Number.isInteger(x.card_id) && !('factor_info_array' in x)))
    return { kind: 'outfits', ids: list.map((x) => x.card_id) };
  if (records(json).length) throw new Error('That is a veterans file: import it in Veteran Umamusume (career page).');
  throw new Error(`No umas or cards found. ${PICK_OWNED}`);
}
