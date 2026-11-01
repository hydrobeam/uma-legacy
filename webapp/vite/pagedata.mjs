// The page's data, from each era's exports (pkg/loopopt/data; export/export_page.py writes page.json and
// page_raceids.json, and checks what it writes). As modules: '#page-data', the planner pages' shared data (the Global
// era's lists, the image catalog, skill icons, race ids, the scenarios' hint bases, the asset table), and
// '#career-data', the Global era's career tables, which only the career page imports. As fingerprinted files: each
// era's solver data and the GM era's lists, fetched only when that era is selected. Each page with the solver gets the
// asset table in its head too (`data-assets`), where its first script preloads the solver's files, and every page
// preloads the UI font's 400 and 700.
import { createHash } from 'node:crypto';
import fs from 'node:fs';
import path from 'node:path';
import { hintBases } from './research.mjs';

const REPO = path.join(import.meta.dirname, '..', '..');
const DATA = path.join(REPO, 'pkg', 'loopopt', 'data');
const PAGE_DATA = '#page-data',
  CAREER_DATA = '#career-data';
const sha256 = (file) => createHash('sha256').update(fs.readFileSync(file)).digest('hex');
const readJson = (file) => JSON.parse(fs.readFileSync(file, 'utf8'));
// a module export of a JSON value, parsed when the module loads (faster than an object literal for large data)
const jsonExport = (name, value) => `export const ${name} = JSON.parse(${JSON.stringify(JSON.stringify(value))});`;

// The solver's downloads, started while the page is still being parsed: the wasm and the active era's data (the era as
// the settings keep it), from the asset table just before it (`data-assets`)
const PRELOAD = `(function () {
  var A = JSON.parse(document.getElementById('data-assets').textContent), gm = false;
  try { var o = JSON.parse(localStorage.getItem('llp:settings') || 'null'); gm = !!o && o.era === 'gm'; } catch (e) {}
  [A.wasm].concat(gm ? [A.engine.gm, A.dataGm] : [A.engine.global]).forEach(function (h) {
    var l = document.createElement('link'); l.rel = 'preload'; l.as = 'fetch'; l.crossOrigin = 'anonymous'; l.href = h; document.head.appendChild(l);
  });
})();`;

/** The era's page.json: the card picker's cards, the roster, the skill catalog and the career page's tables. Its
 * `inputs` (each file it was made from, with its sha256) must match the files now, or the build stops; one under game/
 * that is absent is skipped, so a checkout without the game files builds from the exported lists. */
function pageLists(dir) {
  const page = readJson(path.join(DATA, dir, 'page.json'));
  for (const [input, digest] of Object.entries(page.inputs)) {
    const file = path.join(REPO, input);
    if (input.startsWith('game/') && !fs.existsSync(file)) continue;
    if (!fs.existsSync(file) || sha256(file) !== digest)
      throw new Error(`${input} changed since pkg/loopopt/data/${dir}/page.json was made: run ./build.sh data`);
  }
  return page;
}

/** An era's data: its page lists (the career tables' sample veterans cut to those with a portrait) and the solver's. */
function era(dir, imageKeys) {
  const page = pageLists(dir);
  const exported = (name) => readJson(path.join(DATA, dir, name));
  page.career.mobs = page.career.mobs.filter(([id]) => imageKeys.has(`mob:${id}`));
  const engine = {
    data: exported('data.json'),
    skills: exported('web_skills.json'),
    cards: exported('web_cards.json'),
    events: exported('card_events.json'),
  };
  return { page, engine: JSON.stringify(engine) };
}

/** The plugin. `catalog`: the images' (images.mjs `imageCatalog`); `imgDir`: webapp/img; `wasm`: the solver's path
 * from the root, as Vite imports it; `solverPages`: the pages that start the solver (their file names). */
export function pageData({ catalog, imgDir, wasm, solverPages }) {
  const files = {};
  return {
    name: 'llp-page-data',
    enforce: 'pre', // its specifiers before Vite's resolver, which would follow package.json's imports to the tests' data
    resolveId(id) {
      return id === PAGE_DATA || id === CAREER_DATA ? `\0${id}` : null;
    },
    buildStart() {
      const keys = new Set(catalog.keys);
      const global = era('global', keys),
        gm = era('jp', keys); // GM era: pal first-year date events left out of its card_events.json, as in Global
      const { cards, umas, skills, career } = gm.page;
      const emit = (name, source) => this.emitFile({ type: 'asset', name, source });
      files.engineGlobal = emit('engine-global.json', global.engine);
      files.engineGm = emit('engine-gm.json', gm.engine);
      files.dataGm = emit('data-gm.json', JSON.stringify({ cards, umas, skills, career }));
      files.global = global.page;
    },
    load(id) {
      if (id === `\0${CAREER_DATA}`)
        return `export default JSON.parse(${JSON.stringify(JSON.stringify(files.global.career))});`;
      if (id !== `\0${PAGE_DATA}`) return null;
      // skill group -> icon id of its white and of its gold version (images.py; none in a build without the art)
      const icons = (name) => (fs.existsSync(path.join(imgDir, name)) ? readJson(path.join(imgDir, name)) : {});
      const { cards, umas, skills } = files.global;
      return [
        `import wasm from ${JSON.stringify(`${wasm}?url`)};`,
        'export const assets = {',
        '  wasm,',
        `  engine: { global: import.meta.ROLLUP_FILE_URL_${files.engineGlobal}, gm: import.meta.ROLLUP_FILE_URL_${files.engineGm} },`,
        `  dataGm: import.meta.ROLLUP_FILE_URL_${files.dataGm},`,
        '};',
        jsonExport('cards', cards),
        jsonExport('umas', umas),
        jsonExport('skills', skills),
        jsonExport('img', { ext: catalog.ext, keys: catalog.keys, base: catalog.base }),
        jsonExport('skillIcons', { white: icons('skill_icons.json'), gold: icons('skill_icons_gold.json') }),
        jsonExport('raceIds', readJson(path.join(DATA, 'page_raceids.json'))),
        jsonExport('hintBase', hintBases()),
      ].join('\n');
    },
    transformIndexHtml: {
      order: 'post',
      handler(html, { bundle, filename }) {
        const named = (name) => {
          const file = Object.values(bundle).find((f) => f.type === 'asset' && f.names.includes(name));
          if (!file) throw new Error(`the build has no ${name}`);
          return `/${file.fileName}`;
        };
        const preload = (weight) => ({
          tag: 'link',
          attrs: {
            rel: 'preload',
            href: named(`zen-kaku-gothic-new-latin-${weight}-normal.woff2`),
            as: 'font',
            type: 'font/woff2',
            crossorigin: true,
          },
          injectTo: 'head',
        });
        const tags = [preload(400), preload(700)];
        if (solverPages.includes(path.basename(filename))) {
          const assets = {
            wasm: named(path.basename(wasm)),
            engine: { global: named('engine-global.json'), gm: named('engine-gm.json') },
            dataGm: named('data-gm.json'),
          };
          const attrs = { type: 'application/json', id: 'data-assets' };
          tags.push({ tag: 'script', attrs, children: JSON.stringify(assets), injectTo: 'head' });
          tags.push({ tag: 'script', children: PRELOAD, injectTo: 'head' });
        }
        return { html, tags };
      },
    },
  };
}
