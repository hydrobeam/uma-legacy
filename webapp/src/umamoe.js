// ---------- uma.moe's veteran search (uma.moe/api/v3/search), called from the player's browser with their own key ----------
// The key is the player's (uma.moe: sign in, then Settings), kept in this browser under its own localStorage entry (never
// in llp:settings or llp:career, so never in an Export) and sent only as the X-API-Key header, to uma.moe only. A
// search is one request; its answer is kept for 24 h per data era, target set, threshold and page, so the page asks
// once per target set a day (maintainer, 2026-10-29: automatically, when the target set has no answer kept).
//
// Filters (measured 2026-10-23): a spark is the game's factor_id (factor group * 100 + stars). `main_white_factors`
// takes a comma list (any of them) and, repeated k times, asks for at least k of the list on the listed parent herself;
// `optional_white_sparks` (factor group * 10) ranks the results by how many of them the lineage carries. A skill can
// have two factor groups (Racing Spirit: Stamina, 21009 and 21015), so every factor group of a target is asked for.
// `max_follower_num` 999: trainers at 1,000 followers can't be followed (left out unless the player asks for full
// trainers too). Pages count from 0, 100 results each.
import { CAREER } from './data.js';

const KEY_STORE = 'llp:umamoe-key',
  CACHE_STORE = 'llp:umamoe',
  CACHE_MS = 24 * 3600 * 1000,
  CACHE_KEEP = 8,
  ENDPOINT = 'https://uma.moe/api/v3/search',
  PAGE_SIZE = 100,
  FOLLOWABLE = 999;

/** @returns {string} */
export function savedKey() {
  try {
    return localStorage.getItem(KEY_STORE) || '';
  } catch {
    return '';
  }
}
// full trainers (1,000 followers: they can't be followed) included in the search: the player's choice, kept here
export const fullTrainers = () => {
  try {
    return localStorage.getItem('llp:umamoe-full') === '1';
  } catch {
    return false;
  }
};
/** @param {boolean} on */
export function setFullTrainers(on) {
  try {
    localStorage.setItem('llp:umamoe-full', on ? '1' : '0');
  } catch {}
}
/** @param {string} key an empty key forgets it */
export function saveKey(key) {
  try {
    if (key) localStorage.setItem(KEY_STORE, key);
    else localStorage.removeItem(KEY_STORE);
  } catch {}
}

// the factor groups of each target skill group (CAREER.factors: factor group -> skill group); a target no white spark
// carries (none in the data) is left out of the filters
/** @param {number[]} groups @returns {number[][]} */
export function targetFactors(groups) {
  const of = new Map(),
    listed = CAREER.moe && new Set(CAREER.moe); // GM era: only the factors Global has
  for (const [factor, group] of Object.entries(CAREER.factors)) {
    if (listed && !listed.has(+factor)) continue;
    if (!of.has(group)) of.set(group, []);
    of.get(group).push(+factor);
  }
  return groups.map((g) => of.get(g) || []).filter((f) => f.length);
}

/**
 * One search: lineages whose listed parent carries at least `min` of the targets herself, best covered first.
 * @typedef {{factors: number[][], min: number, page: number, full?: boolean}} Query
 * @typedef {{items: any[], total: number, pages: number, cached: boolean}} Answer
 */
/** @param {Query} q @returns {string} */
function queryString({ factors, min, page, full = false }) {
  const p = new URLSearchParams({ search_type: 'inheritance', limit: String(PAGE_SIZE), page: String(page) });
  if (!full) p.set('max_follower_num', String(FOLLOWABLE));
  const any = factors.flatMap((fs) => fs.flatMap((f) => [1, 2, 3].map((stars) => f * 100 + stars))).join(',');
  for (let i = 0; i < min; i++) p.append('main_white_factors', any);
  p.append('optional_white_sparks', factors.flatMap((fs) => fs.map((f) => f * 10)).join(','));
  return p.toString();
}

function loadCache() {
  try {
    const c = JSON.parse(localStorage.getItem(CACHE_STORE));
    if (c && typeof c === 'object') return c;
  } catch {}
  return {};
}
function storeCache(cache) {
  const kept = Object.entries(cache)
    .filter(([, x]) => Date.now() - x.at < CACHE_MS)
    .sort((a, b) => b[1].at - a[1].at)
    .slice(0, CACHE_KEEP);
  try {
    localStorage.setItem(CACHE_STORE, JSON.stringify(Object.fromEntries(kept)));
  } catch {}
}

// the reason a search failed, in words for the page; `badKey`: uma.moe refused the key itself
export class SearchError extends Error {
  /** @param {string} message @param {boolean} [badKey] */
  constructor(message, badKey = false) {
    super(message);
    this.badKey = badKey;
  }
}

// the kept answer, when one is younger than 24 h
/** @param {string} era @param {Query} q @returns {(Answer & {at: number})|null} */
export function keptAnswer(era, q) {
  const hit = loadCache()[`${era}?${queryString(q)}`];
  return hit && Date.now() - hit.at < CACHE_MS ? { ...hit.answer, cached: true, at: hit.at } : null;
}
// the kept answer when one is younger than 24 h (unless `fresh`), else one request; throws SearchError
/** @param {string} key @param {string} era @param {Query} q @param {boolean} [fresh] @returns {Promise<Answer & {at: number}>} */
export async function search(key, era, q, fresh = false) {
  const kept = fresh ? null : keptAnswer(era, q);
  if (kept) return kept;
  const qs = queryString(q),
    id = `${era}?${qs}`,
    cache = loadCache();
  let res;
  try {
    res = await fetch(`${ENDPOINT}?${qs}`, { headers: { 'X-API-Key': key }, credentials: 'omit' });
  } catch {
    throw new SearchError("Can't reach uma.moe");
  }
  if (res.status === 401 || res.status === 403) throw new SearchError('Key rejected: paste a valid one', true);
  if (res.status === 429) throw new SearchError('Too many requests: try in a minute');
  if (!res.ok) throw new SearchError(`uma.moe error ${res.status}`);
  let body;
  try {
    body = await res.json();
  } catch {
    throw new SearchError('Unreadable answer');
  }
  const items = Array.isArray(body?.items) ? body.items.filter((x) => x?.inheritance?.inheritance_id) : [];
  const answer = { items, total: +body?.total || items.length, pages: +body?.total_pages || 1 },
    at = Date.now();
  cache[id] = { at, answer };
  storeCache(cache);
  return { ...answer, cached: false, at };
}
