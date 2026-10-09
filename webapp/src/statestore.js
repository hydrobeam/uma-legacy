// ---------- the solver's kept work across page loads (IndexedDB; the career planner) ----------
// One record per key (the solver's and the era data's fingerprints: a release starts afresh). Every failure (no
// IndexedDB, private mode, quota, a corrupt record) reads as "nothing stored": the page then plans cold, as before.
const DB = 'llp-solver', STORE = 'state';
let dbp = null;
function db() {
  if (!dbp) dbp = new Promise((res, rej) => {
    try {
      const rq = indexedDB.open(DB, 1);
      rq.onupgradeneeded = () => { rq.result.createObjectStore(STORE); };
      rq.onsuccess = () => res(rq.result); rq.onerror = () => rej(rq.error); rq.onblocked = () => rej(new Error('blocked'));
    } catch (e) { rej(e); }
  });
  dbp.catch(() => { dbp = null; });
  return dbp;
}
function tx(mode, f) {
  return db().then(d => new Promise((res, rej) => {
    const t = d.transaction(STORE, mode), rq = f(t.objectStore(STORE));
    t.oncomplete = () => res(rq && rq.result); t.onerror = () => rej(t.error); t.onabort = () => rej(t.error);
  }));
}
// what was stored under key (null when nothing is, or anything fails); other keys are dropped (a past release's)
export async function stateLoad(key) {
  try {
    const keys = await tx('readonly', s => s.getAllKeys());
    for (const k of keys || []) if (k !== key) await tx('readwrite', s => s.delete(k));
    return (await tx('readonly', s => s.get(key))) || null;
  } catch { return null; }
}
export async function stateSave(key, v) { try { await tx('readwrite', s => s.put(v, key)); } catch { /* quota, private mode: plan cold next time */ } }
export async function stateDrop(key) { try { await tx('readwrite', s => s.delete(key)); } catch { /* nothing to drop */ } }
