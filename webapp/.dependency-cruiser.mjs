// The page's module layers and folders, checked by dependency-cruiser (`./build.sh lint`; `npx depcruise src`).
// Layers. Each module of src/ belongs to one layer: the types, io and model layers are the logic, which takes the
// settings and records as arguments (`import type` only), the stores hold them, and the views and helpers both pages
// share sit above, each page's own modules on top. A value import (static, dynamic or a re-export; an `import type`
// reads nothing at load and subscribes nothing) points to its own layer or one below, and nothing outside a page
// imports into it. So a logic module's value imports stay among logic modules, and none reaches a store's values
// through any chain of imports: nothing it reads can subscribe an effect. The model layer holds the rule's logic
// (frontend.md: the run machines, buildJob, uma.moe sync, io): model/ but data.ts (the io layer: the engine reads it)
// and the types grades.ts and eras.ts, both run machines (their store's transitions and the settings come in their
// deps), career/job.ts (its buildJob takes the settings and the veterans store's record as values; career/current.ts, a
// page module, passes the stores' own) and the uma.moe and umadump readers in io/ (uma.moe's sync: the targets and the
// pool's setListings come in through connectUmamoe). Outside it: loop/io.ts, the loop page's Export and Import, an
// editor that imports shared modules reading the settings; io/owned.ts, which saves the owned marks it reads. A page's
// layer is its folder, less the modules named below it (veterans/ is the career page's).
// Folders (`FOLDERS`): what each of src/'s folders imports nothing from, on top of the layers' rule, by what it is:
// model/ pure logic, ui/ the primitives, state/ the saved state and settings, solver/ the worker and its engine, io/
// the import readers, each page its own; dom/'s listeners only the entries install.
// Components stay views: a module that is not a component (.ts) imports no .tsx module, a type import included.
// Swc parses (dependency-cruiser reads TypeScript through TypeScript's JavaScript API, which TypeScript 7 no longer has).
// (biome.json's overrides name the stores layer's modules: they make their records with state/store.ts's store())
export const LAYERS = [
  [
    'types',
    [
      'globals.d',
      'must',
      'gen/(solver|constants)',
      'solver/solver',
      'state/store',
      'loop/types',
      'model/(grades|eras)',
    ],
  ],
  // (gen/loopopt: wasm-bindgen's glue, which the worker imports; JavaScript, the one module of src/ that is)
  ['io', ['gen/loopopt', 'model/data', 'solver/(engine|worker)', 'state/(saved|storage|statestore)', 'dom/errorlog']],
  [
    'model',
    ['model/(?!(data|grades|eras)\\.)[^/]+', 'io/(umadump|umamoe|umamoesetup)', 'loop/(job|keys|members|run)'].concat([
      'career/(roll|job|reply|keys|run)',
      'career/analysis/compare',
      'veterans/(names|records|complete)',
    ]),
  ],
  ['stores', ['state/settings', 'loop/state', 'veterans/store', 'career/state', 'career/analysis/state']],
  ['shared', null], // every module of src/ in no other layer
  ['page:loop', { folders: ['loop'] }],
  ['page:career', { folders: ['career', 'veterans'] }],
];
const modules = (names) => `^src/(${names.join('|')})\\.(tsx?|js)$`;
const named = LAYERS.flatMap(([, names]) => (Array.isArray(names) ? names : []));
const NAMED = modules(named);
const PAGES = `^src/(${LAYERS.flatMap(([, x]) => x?.folders ?? []).join('|')})/`;
const layer = ([, names]) => {
  if (Array.isArray(names)) return { path: modules(names) };
  if (names) return { path: `^src/(${names.folders.join('|')})/`, pathNot: NAMED };
  return { path: '^src/', pathNot: `(${NAMED})|(${PAGES})` };
};
const VALUE = { dependencyTypesNot: ['type-only'] };
// a rule for each layer and each layer above it, or another page's
const against = LAYERS.flatMap((from, i) =>
  LAYERS.filter((to, j) => j > i || (j !== i && to[0].startsWith('page:'))).map((to) => ({
    name: `${from[0]}-imports-no-${to[0]}`,
    comment: `the ${from[0]} layer imports its own layer or one below; nothing outside a page imports into it`,
    severity: 'error',
    from: layer(from),
    to: { ...layer(to), ...VALUE },
  })),
);
const folders = (names) => `^src/(${names.join('|')})/`;
const VIEWS = ['ui', 'setup', 'results', 'career', 'loop', 'veterans'];
// Each folder and what it imports nothing from (a value import; `types`: an `import type` too). `except`: the one
// module of the folder that may import one module of those, and why; it has a rule of its own, which lets that one
// through and no other.
const FOLDERS = [
  {
    from: 'model',
    not: ['state', 'solver', 'io', 'dom', ...VIEWS],
    why: 'pure game and planner logic: no storage, no solver, no DOM, no view',
    // the era this page load opened with: the one stored value the game data rests on (which era's data to read)
    except: { from: 'model/data.ts', to: 'state/saved.ts' },
  },
  {
    from: 'ui',
    not: ['state', 'solver', 'io', 'dom', ...VIEWS.filter((v) => v !== 'ui')],
    why: 'the primitives: no store, feature or page',
    // a deck card's panel opened from inside a dialog shows inside it (the page behind a modal is out of reach)
    except: { from: 'ui/Dialog.tsx', to: 'results/CardPop.tsx' },
  },
  { from: 'state', not: ['solver', 'io', 'dom', ...VIEWS], why: 'saved state and settings: no solver, reader or view' },
  { from: 'solver', not: ['io', 'dom', ...VIEWS], why: "the solver's side of the page: no reader or view" },
  { from: 'io', not: ['solver', 'dom', ...VIEWS], why: 'the import readers: no solver or view' },
  { from: 'loop', not: ['career', 'veterans'], types: true, why: "the loop page: nothing of the career page's" },
  { from: 'career', not: ['loop'], types: true, why: "the career page: nothing of the loop page's" },
  { from: 'veterans', not: ['loop'], types: true, why: "the career page's veterans: nothing of the loop page's" },
].flatMap(({ from, not, why, types, except }) => {
  const file = (path) => `^src/${path.replaceAll('.', '\\.')}$`;
  const rule = (name, fromPart, toPart) => ({
    name,
    comment: `${from}/: ${why}`,
    severity: 'error',
    from: fromPart,
    to: { path: folders(not), ...toPart, ...(types ? {} : VALUE) },
  });
  const name = `${from}-imports-no-${not.join('-')}`;
  if (!except) return [rule(name, { path: `^src/${from}/` }, {})];
  return [
    rule(name, { path: `^src/${from}/`, pathNot: file(except.from) }, {}),
    rule(`${name}-but-${except.to.replace(/\W+/g, '-')}`, { path: file(except.from) }, { pathNot: file(except.to) }),
  ];
});

export default {
  forbidden: [
    ...against,
    ...FOLDERS,
    {
      name: 'dom-imported-by-the-entries-only',
      comment: "dom/'s modules install page-wide listeners as they load: only the two entries import them",
      severity: 'error',
      from: { pathNot: '^src/(career|loop)/main\\.tsx$' },
      to: { path: '^src/dom/' },
    },
    {
      name: 'logic-module-imports-no-component',
      comment:
        'logic takes values; components import logic (a .ts module imports no .tsx one, an `import type` included)',
      severity: 'error',
      from: { path: '^src/.*\\.ts$' },
      to: { path: '^src/.*\\.tsx$' },
    },
  ],
  options: {
    parser: 'swc',
    includeOnly: '^src/',
    exclude: { path: '\\.d\\.ts$' },
    tsConfig: { fileName: 'tsconfig.json' },
  },
};
