// ---------- the solver's kept work across page loads (IndexedDB; the career planner) ----------
// One record per key (the solver's and the era data's fingerprints: a release starts afresh). Every failure (no
// IndexedDB, private mode, quota, a corrupt record) reads as "nothing stored": the page then plans cold.
// A second store keeps the last answer the page drew (`answerSave`), so a reload with nothing changed shows it at once.
const DB_NAME = 'llp-solver',
  STORE = 'state',
  ANSWER = 'answer';
let opening = null; // the open database (a promise); a failed open is forgotten, so the next use tries again
function database() {
  if (!opening)
    opening = new Promise((resolve, reject) => {
      try {
        const request = indexedDB.open(DB_NAME, 2);
        request.onupgradeneeded = () => {
          const db = request.result;
          for (const name of [STORE, ANSWER]) if (!db.objectStoreNames.contains(name)) db.createObjectStore(name);
        };
        request.onsuccess = () => resolve(request.result);
        request.onerror = () => reject(request.error);
        request.onblocked = () => reject(new Error('blocked'));
      } catch (e) {
        reject(e);
      }
    });
  opening.catch(() => {
    opening = null;
  });
  return opening;
}
// one transaction on the store: `use` makes its request; the promise resolves with that request's result once the
// transaction completes
function transact(mode, use, store = STORE) {
  return database().then(
    (db) =>
      new Promise((resolve, reject) => {
        const transaction = db.transaction(store, mode),
          request = use(transaction.objectStore(store));
        transaction.oncomplete = () => resolve(request?.result);
        transaction.onerror = () => reject(transaction.error);
        transaction.onabort = () => reject(transaction.error);
      }),
  );
}
// what was stored under key (null when nothing is, or anything fails); other keys are dropped (a past release's)
/** @param {string} key */
export async function stateLoad(key) {
  try {
    const keys = await transact('readonly', (store) => store.getAllKeys());
    for (const old of keys || []) if (old !== key) await transact('readwrite', (store) => store.delete(old));
    return (await transact('readonly', (store) => store.get(key))) || null;
  } catch {
    return null;
  }
}
/** @param {string} key @param {unknown} value */
export async function stateSave(key, value) {
  try {
    await transact('readwrite', (store) => store.put(value, key));
  } catch {
    /* quota, private mode: plan cold next time */
  }
}
/** @param {string} key */
export async function stateDrop(key) {
  try {
    await transact('readwrite', (store) => store.delete(key));
  } catch {
    /* nothing to drop */
  }
}
// the last answer drawn, with the key it was drawn for (the solver's and data's fingerprints, the job and the places):
// null when none is stored or anything fails
/** @returns {Promise<{key: string, value: any}|null>} */
export async function answerLoad() {
  try {
    return (await transact('readonly', (store) => store.get('last'), ANSWER)) || null;
  } catch {
    return null;
  }
}
/** @param {string} key @param {unknown} value */
export async function answerSave(key, value) {
  try {
    await transact('readwrite', (store) => store.put({ key, value }, 'last'), ANSWER);
  } catch {
    /* quota, private mode, an answer that can't be cloned: the next load plans */
  }
}
