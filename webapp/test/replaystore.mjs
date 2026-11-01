// Where the recorded solver replies live (solverreplay.mjs): one directory of cache/solver-replies/ per solver and
// data, named by the fingerprints of the page's wasm and both eras' engine data, so a solver or data change starts a
// new directory and `./build.sh prune-replies` removes the old ones. Run as a script, it prints the directory for the
// page in webapp/dist (./build.sh test and testq record into it when it is missing).
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const STORE_ROOT = fileURLToPath(new URL('../../cache/solver-replies/', import.meta.url));

/** The store directory's name for a page's data-assets (build.py ASSETS): its wasm's and engines' fingerprints.
 * @param {{wasm: string, engine: {global: string, gm: string}}} assets */
export function storeName(assets) {
  const files = [assets.wasm, assets.engine.global, assets.engine.gm];
  const hashes = files.map((f) => /\.([0-9a-f]+)\.[a-z]+$/.exec(f)?.[1]);
  if (hashes.some((h) => !h)) throw new Error(`no fingerprint in the asset names ${files.join(', ')}`);
  return hashes.join('-');
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const html = fs.readFileSync(new URL('../dist/index.html', import.meta.url), 'utf8');
  const json = /<script type="application\/json" id="data-assets">([^<]*)<\/script>/.exec(html)?.[1];
  if (!json) throw new Error('webapp/dist/index.html has no data-assets script');
  console.log(path.join(STORE_ROOT, storeName(JSON.parse(json))));
}
