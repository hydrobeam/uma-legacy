// Where the solver's replies are kept for the browser flows (solver.ts): one directory of cache/solver-replies/ per
// solver and data, named from what the solver's answers are a function of, read from the tree with nothing built: the
// solver's sources (pkg/loopopt/src, its Cargo files and build.rs, and research/results.json, which build.rs compiles
// in), the worker that runs it (src/worker.ts and wasm-bindgen's glue, src/gen/loopopt.js) and each era's engine data
// as the page build writes it (no solver build needed). A page change (a view, the stylesheet, the bundler's file names)
// keeps the store; a solver or data change starts a new directory, which a recording run fills (LLP_SOLVER=live, with
// the built solver), and `./build.sh prune-replies` removes the old ones.
// Run as a script (node test/browser/store.ts), it prints the directory for this tree and the page in webapp/dist.
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

export const STORE_ROOT = fileURLToPath(new URL('../../../cache/solver-replies/', import.meta.url));
const REPO = fileURLToPath(new URL('../../../', import.meta.url));
const DIST = fileURLToPath(new URL('../../dist/', import.meta.url));
// the solver's sources and the worker's, from the repository's root: directories whole
const SOURCES = [
  'pkg/loopopt/src',
  'pkg/loopopt/Cargo.toml',
  'pkg/loopopt/Cargo.lock',
  'pkg/loopopt/build.rs',
  'research/results.json',
  'webapp/src/worker.ts',
  'webapp/src/gen/loopopt.js',
];

/** The page's solver files, as its head names them (data-assets: vite/pagedata.mjs). */
export interface PageAssets {
  wasm: string;
  engine: { global: string; gm: string };
}

/** A served file's bytes, by its path on the site (`/assets/…`): from webapp/dist, or from LLP_URL's host. */
export async function served(site: string): Promise<Uint8Array> {
  if (process.env.LLP_URL) {
    const response = await fetch(new URL(site, process.env.LLP_URL));
    if (!response.ok) throw new Error(`${site}: ${response.status}`);
    return new Uint8Array(await response.arrayBuffer());
  }
  const file = path.resolve(DIST, `.${site}`);
  if (!file.startsWith(DIST)) throw new Error(`${site} is outside webapp/dist`);
  return fs.promises.readFile(file);
}

const sha256 = (bytes: Uint8Array | string): string => createHash('sha256').update(bytes).digest('hex');

/** Every file under a path of the repository (itself, if a file), as paths from the root, sorted. */
function files(relative: string): string[] {
  const full = path.join(REPO, relative);
  if (!fs.statSync(full).isDirectory()) return [relative];
  return fs
    .readdirSync(full, { recursive: true, withFileTypes: true })
    .filter((d) => d.isFile())
    .map((d) => path.relative(REPO, path.join(d.parentPath, d.name)))
    .sort();
}

/** The solver's and worker's sources, digested: each file's path and bytes. */
function sourcesDigest(): string {
  const hash = createHash('sha256');
  for (const file of SOURCES.flatMap(files)) hash.update(`${file}\n`).update(fs.readFileSync(path.join(REPO, file)));
  return hash.digest('hex');
}

/** The store directory's name: the first 16 hex digits of the sources' digest and of each era's engine data's. */
export async function storeName(assets: PageAssets): Promise<string> {
  const engine = async (site: string) => sha256(await served(site)).slice(0, 16);
  return [sourcesDigest().slice(0, 16), await engine(assets.engine.global), await engine(assets.engine.gm)].join('-');
}

/** The solver assets of the page in webapp/dist. */
export function distAssets(): PageAssets {
  const html = fs.readFileSync(path.join(DIST, 'index.html'), 'utf8');
  const json = /<script type="application\/json" id="data-assets">([^<]*)<\/script>/.exec(html)?.[1];
  if (!json) throw new Error('webapp/dist/index.html has no data-assets script');
  return JSON.parse(json);
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  console.log(path.join(STORE_ROOT, await storeName(distAssets())));
}
