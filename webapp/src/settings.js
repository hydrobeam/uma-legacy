// ---------- saved settings: one object, localStorage['llp:settings'] ----------
// { v, era: 'global' | 'gm', common: {targets, rank, ...}, global: {roster, outfits, cardsOff}, gm: {...} }
// Owned umas, outfits and cards are kept per data set; everything else is shared.
// Game data: Global today, or JP data cut at L'Arc's launch (24 Aug 2023), the Grand Masters era. Switching
// reloads the page with the other data set.
export const SKEY = 'llp:settings', SV = 1;
export const PER_ERA = ['roster', 'outfits', 'cardsOff'];
export const COMMON = ['targets', 'rank', 'zephyr', 'borrow', 'decksTop', 'perUma', 'rand', 'rcards', 'rsort', 'rdir', 'csort', 'cdir'];
export const SETTINGS = [...COMMON, ...PER_ERA];   // exported and imported (older exports may carry more keys; those are ignored)
function migrate() {   // from the per-key layout (llp:<key>, llp:gm:<key>, llp:era) and its one-time fixes
  const raw = (k) => { try { return localStorage.getItem(k); } catch { return null; } };
  const get = (k) => { const v = raw(k); if (v == null) return undefined; try { return JSON.parse(v); } catch { return undefined; } };
  const o = { v: SV, era: raw('llp:era') === 'gm' ? 'gm' : 'global', common: {}, global: {}, gm: {} };
  for (const k of COMMON) { const v = get('llp:' + k); if (v !== undefined) o.common[k] = v; }
  for (const k of PER_ERA) { const a = get('llp:' + k), b = get('llp:gm:' + k); if (a !== undefined) o.global[k] = a; if (b !== undefined) o.gm[k] = b; }
  const c = o.common;
  if (c.rank) c.rank = ({ '0.45,0.50,0.05': '0.50,0.45,0.05', '0.175,0.70,0.125': '0.20,0.70,0.10' })[c.rank] || c.rank;   // old star mixes
  if (Array.isArray(c.targets)) {
    // older Unity Cup presets marked See Ya Later! and Nothing Ventured as guaranteed golds; the deck search gives them now
    if (!raw('llp:targetsV2')) for (const t of c.targets) if ((t.group === 20166 || t.group === 20203) && t.guaranteed === 2) t.guaranteed = 0;
    c.targets = c.targets.map(t => ({ group: t.group, guaranteed: t.guaranteed || 0 }));   // hand-set deck odds are gone
  }
  if (c.rsort === 'rel' && c.rdir === 'asc' && !raw('llp:rsortv2')) c.rdir = 'desc';   // the roster default became newest first
  try { localStorage.setItem(SKEY, JSON.stringify(o));
    for (const k of Object.keys(localStorage)) if (k.startsWith('llp:') && k !== SKEY) localStorage.removeItem(k); } catch {}
  return o;
}
function loadSaved() { try { const o = JSON.parse(localStorage.getItem(SKEY)); if (o && o.v === SV) return o; } catch {} return migrate(); }
export const SAVED = loadSaved();
for (const k of ['common', 'global', 'gm']) SAVED[k] = SAVED[k] || {};
export const ERA = SAVED.era === 'gm' ? 'gm' : 'global';
export function persist() { try { localStorage.setItem(SKEY, JSON.stringify(SAVED)); } catch {} }
