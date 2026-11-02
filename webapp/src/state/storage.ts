// The browser's localStorage, reached only here (Biome bans the global and a `.setItem` / `.removeItem` call
// everywhere else: biome.json, lint/storage-write.grit). The one way to write is `update`: it hands its change what is
// stored under the key now and stores what the change returns, so a write is made from what is there at that moment,
// never from a copy read earlier (at load, or before a request), which would erase what another tab or call stored
// meanwhile. A blocked or full storage throws on access: a read then finds nothing and a write is lost for this visit.
// Each function takes where to (`at`): the page's localStorage, or a test's stand-in.

/** where a key is kept: the browser's localStorage, or a test's stand-in */
export type Backing = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;
/** the page's (reached inside each try: a blocked storage throws on access) */
export const browserStorage = (): Backing => localStorage;

/** the text stored under `key` (null: nothing, or the storage is blocked) */
export function read(key: string, at: () => Pick<Backing, 'getItem'> = browserStorage): string | null {
  try {
    return at().getItem(key);
  } catch {
    return null;
  }
}
/** the value stored under `key` as JSON (null: nothing, unreadable, or the storage is blocked) */
export function readJson(key: string, at: () => Pick<Backing, 'getItem'> = browserStorage): unknown {
  try {
    return JSON.parse(read(key, at) ?? 'null');
  } catch {
    return null;
  }
}
/** The one write: `change` gets what is stored under `key` now and returns what to store there (null: remove the
 *  key). False when the storage is blocked or full (the change then lasts for this visit only). */
export function update(
  key: string,
  change: (stored: string | null) => string | null,
  at: () => Backing = browserStorage,
): boolean {
  try {
    const next = change(at().getItem(key));
    if (next === null) at().removeItem(key);
    else at().setItem(key, next);
    return true;
  } catch {
    return false;
  }
}
/** `update` of a JSON value: `change` gets the value stored now (null: none or unreadable) */
export const updateJson = (key: string, change: (stored: unknown) => unknown, at: () => Backing = browserStorage) =>
  update(
    key,
    (stored) => {
      let value: unknown = null;
      try {
        value = JSON.parse(stored ?? 'null');
      } catch {
        // unreadable: as if nothing were stored
      }
      const next = change(value);
      return next === null ? null : JSON.stringify(next);
    },
    at,
  );
