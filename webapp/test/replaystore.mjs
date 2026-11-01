// Where the recorded solver replies live (solverreplay.mjs): one directory of cache/solver-replies/ per solver and
// data, named by the SHA-256 of the bytes the page's solver runs on: the wasm, both eras' engine data and the worker
// script (which turns each job into a call of the wasm and its answer into the messages recorded). Only those files'
// contents name it, never an asset's name or the rest of the page, so a page change (a view, the stylesheet, the
// bundler's file names) keeps the store, a solver or data change starts a new directory, and `./build.sh prune-replies`
// removes the old ones. Run as a script, it prints the directory for the page in webapp/dist (./build.sh test and testq
// record into it when it is missing).
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const STORE_ROOT = fileURLToPath(new URL('../../cache/solver-replies/', import.meta.url));
const DIST = fileURLToPath(new URL('../dist/', import.meta.url));

/** A served file's bytes, by its path on the site (`/assets/…`): from webapp/dist, or from LLP_URL's host when the
 * suite runs on a deployed page.
 * @param {string} site @returns {Promise<Uint8Array>} */
async function served(site) {
  if (process.env.LLP_URL) {
    const response = await fetch(new URL(site, process.env.LLP_URL));
    if (!response.ok) throw new Error(`${site}: ${response.status}`);
    return new Uint8Array(await response.arrayBuffer());
  }
  const file = path.resolve(DIST, `.${site}`);
  if (!file.startsWith(DIST)) throw new Error(`${site} is outside webapp/dist`);
  return fs.promises.readFile(file);
}

const sha256 = (bytes) => createHash('sha256').update(bytes).digest('hex');

/**
 * The store directory's name for a solver: the first 16 hex digits of the SHA-256 of each of its files' bytes.
 * @param {{assets: {wasm: string, engine: {global: string, gm: string}}, worker: string}} solver  the page's data-assets
 *   (vite/pagedata.mjs) and its worker script's path on the site
 */
export async function storeName({ assets, worker }) {
  const digest = async (site) => sha256(await served(site)).slice(0, 16);
  const files = [assets.wasm, assets.engine.global, assets.engine.gm, worker];
  return (await Promise.all(files.map(digest))).join('-');
}

/** The solver of the page in webapp/dist: its data-assets and its one worker script. */
function distSolver() {
  const html = fs.readFileSync(path.join(DIST, 'index.html'), 'utf8');
  const json = /<script type="application\/json" id="data-assets">([^<]*)<\/script>/.exec(html)?.[1];
  if (!json) throw new Error('webapp/dist/index.html has no data-assets script');
  const workers = fs.readdirSync(path.join(DIST, 'assets')).filter((f) => /^worker-[\w-]+\.js$/.test(f));
  if (workers.length !== 1) throw new Error(`webapp/dist/assets has ${workers.length} worker scripts, not one`);
  return { assets: JSON.parse(json), worker: `/assets/${workers[0]}` };
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  console.log(path.join(STORE_ROOT, await storeName(distSolver())));
}
