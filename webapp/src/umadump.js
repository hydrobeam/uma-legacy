// ---------- veteran import: umadump's trained_chara_data.json and rental_chara_data.json (github.com/Werseter/umadump) ----------
// Per veteran: card_id (outfit; character = card_id / 100), factor_info_array (sparks: factor_id = factor group * 100 +
// stars; white skill sparks map to a skill group through CAREER.factors), win_saddle_id_array (single-race G1 saddles
// map to a race index through CAREER.saddles; titles are left out) and succession_chara_array (her parents at
// position_id 10 and 20, same fields). A veteran counts as borrowed when owner_viewer_id is set: it is 0 on your own
// veterans and the lender's viewer_id (= viewer_id) on the ones offered for borrowing (rental_chara_data.json, the
// career start screen's borrow list); use_type is 0 on both.
import { CAREER } from './data.js';

function sparksOf(fs) {
  const out = {};
  for (const f of fs || []) {
    const g = CAREER.factors[Math.floor(f.factor_id / 100)];
    if (g !== undefined) out[g] = Math.max(out[g] || 0, f.factor_id % 100);
  }
  return out;
}
function winsOf(ws) { return [...new Set((ws || []).map(w => CAREER.saddles[w]).filter(x => x !== undefined))].sort((a, b) => a - b); }
// aptitude (pink) sparks: factor_id = aptitude group * 100 + stars, the groups in CAREER.pinks
function pinksOf(fs) {
  const out = {};
  for (const f of fs || []) { const g = Math.floor(f.factor_id / 100); if (CAREER.pinks[g] !== undefined) out[g] = Math.max(out[g] || 0, f.factor_id % 100); }
  return out;
}
// blue (stat) sparks: factor groups 1..5; unique (green) sparks: the outfit's card id as the factor group
function kindOf(fs, keep) { const out = {}; for (const f of fs || []) { const g = Math.floor(f.factor_id / 100); if (keep(g)) out[g] = Math.max(out[g] || 0, f.factor_id % 100); } return out; }
const node = (x) => ({ chara: Math.floor(x.card_id / 100), outfit: x.card_id, sparks: sparksOf(x.factor_info_array), pinks: pinksOf(x.factor_info_array),
  blues: kindOf(x.factor_info_array, g => g >= 1 && g <= 5), greens: kindOf(x.factor_info_array, g => g >= 100000 && (CAREER.uniques || {})[g] !== undefined),
  wins: winsOf(x.win_saddle_id_array) });

// the list of veterans in an export: the file's array, or the first array of veteran records inside an object
function records(o) {
  const isVet = (x) => x && typeof x === 'object' && 'card_id' in x && 'factor_info_array' in x;
  if (Array.isArray(o)) return o.filter(isVet);
  if (o && typeof o === 'object') for (const v of Object.values(o)) { const r = records(v); if (r.length) return r; }
  return [];
}
// veterans from export text; throws an Error with a message for the page when it is not an export
// uma.moe's lineage planner export ({type: 'lineage-planner', payload: [{position, characterId, sparks, manualWinSaddleIds}]}):
// a veteran per top position ("p1", "p2", ...) with her parents at "p1-1" and "p1-2". A spark's factorId x 10 + its level
// is the game's factor_id (master.mdb succession_factor); saddle ids are the game's win saddles. null when the text is not
// one; its veterans count as yours (the export doesn't say)
export function parseTree(text) {
  let o; try { o = JSON.parse(text); } catch { return null; }
  if (!o || o.type !== 'lineage-planner' || !Array.isArray(o.payload)) return null;
  const at = new Map(o.payload.map(x => [x.position, x]));
  const nd = (x) => node({ card_id: x.characterId, factor_info_array: (x.sparks || []).map(f => ({ factor_id: f.factorId * 10 + f.level })), win_saddle_id_array: x.manualWinSaddleIds });
  const empty = () => ({ chara: null, sparks: {}, pinks: {}, wins: [] });
  return o.payload.filter(x => /^p\d+$/.test(x.position)).map(x => ({
    ...nd(x), id: 't' + Date.now().toString(36) + Math.random().toString(36).slice(2, 5), src: 'tree', borrowed: false,
    parents: [1, 2].map(j => { const p = at.get(`${x.position}-${j}`); return p ? nd(p) : empty(); }),
  }));
}
export function parseExport(text) {
  let o; try { o = JSON.parse(text); } catch { throw new Error('That is not JSON. Pick umadump\'s trained_chara_data.json or rental_chara_data.json.'); }
  const rs = records(o);
  if (!rs.length) throw new Error('No veterans found. Pick umadump\'s trained_chara_data.json or rental_chara_data.json.');
  return rs.map(r => ({
    ...node(r), src: 'import', borrowed: !!r.owner_viewer_id,
    // trained_chara_id counts per player: a lender's id is kept apart from yours by her viewer id
    id: (r.owner_viewer_id ? 'tr' + r.owner_viewer_id + '-' : 'tc') + (r.trained_chara_id ?? Math.random().toString(36).slice(2)),
    parents: [10, 20].map(pos => { const p = (r.succession_chara_array || []).find(x => x.position_id === pos); return p ? node(p) : { chara: null, sparks: {}, pinks: {}, wins: [] }; }),
  }));
}
