// The page's module layers, checked by dependency-cruiser (`./build.sh lint`; `npx depcruise src`). Each module of src/
// belongs to one layer: the types, io and model layers are the logic, which takes the settings and records as arguments
// (`import type` only), the stores hold them, and the views and helpers both pages share sit above, each page's own
// modules on top. A value import (static, dynamic or a re-export; an `import type` reads nothing at load and subscribes
// nothing) points to its own layer or one below, and nothing outside a page imports into it. So a logic module's value
// imports stay among logic modules, and none reaches a store's values through any chain of imports: nothing it reads can
// subscribe an effect. The model layer holds the rule's logic (frontend.md: the run machines, buildJob, uma.moe sync, io):
// both run machines (their store's transitions and the settings come in their deps), career/job.ts (its buildJob takes
// the settings and the veterans store's record as values; career/current.ts, a page module, passes the stores' own) and
// io/umamoesetup.ts (uma.moe's sync: the targets and the pool's setListings come in through connectUmamoe). Outside it:
// loop/io.ts, the loop page's Export and Import, an editor that imports shared modules reading the settings.
// Components stay views: a module that is not a component (.ts) imports no .tsx module, a type import included.
// Swc parses (dependency-cruiser reads TypeScript through TypeScript's JavaScript API, which TypeScript 7 no longer has).
// (biome.json's overrides name the stores layer's modules: they make their records with store.ts's store())
export const LAYERS = [
  [
    'types',
    [
      'globals.d',
      'must',
      'model/grades',
      'solver/solver',
      'state/store',
      'loop/types',
      'model/eras',
      'gen/solver',
      'gen/constants',
    ],
  ],
  // (gen/loopopt: wasm-bindgen's glue, which the worker imports; JavaScript, the one module of src/ that is)
  [
    'io',
    [
      'model/data',
      'solver/engine',
      'state/statestore',
      'dom/errorlog',
      'solver/worker',
      'gen/loopopt',
      'state/saved',
      'state/storage',
    ],
  ],
  [
    'model',
    [
      'model/job',
      'model/odds',
      'model/pink',
      'model/proofs',
      'io/umadump',
      'io/umamoe',
      'career/roll',
      'career/job',
      'career/reply',
    ]
      .concat(['career/keys', 'career/run'])
      .concat(['career/analysis/compare', 'loop/job', 'loop/keys', 'loop/members', 'loop/run', 'io/umamoesetup'])
      .concat(['veterans/names', 'veterans/records', 'veterans/complete']),
  ],
  ['stores', ['state/settings', 'loop/state', 'veterans/store', 'career/state', 'career/analysis/state']],
  ['shared', null], // every module of src/ not named in another layer
  [
    'page:loop',
    ['loop/main', 'loop/Layout', 'loop/io', 'loop/RunNote', 'loop/Results', 'loop/Affinity', 'loop/Export'],
  ],
  [
    'page:career',
    [
      'career/main',
      'career/Layout',
      'career/analysis/view',
      'career/analysis/Details',
      'career/options',
      'career/current',
      'career/io',
    ]
      .concat(['career/view/parts', 'career/view/dialogs', 'career/view/Guard', 'career/view/SkillDetails'])
      .concat(['career/view/Options', 'career/view/Umamoe', 'career/view/RunNote', 'career/view/Tree'])
      .concat(['career/view/NodeDialog', 'career/view/ParentPicker', 'career/view/Sections', 'career/view/Results'])
      .concat(['career/view/Pool', 'career/view/Io', 'career/view/poolState', 'career/view/Setup']),
  ],
];
const modules = (names) => `^src/(${names.join('|')})\\.(tsx?|js)$`;
const NAMED = modules(LAYERS.flatMap(([, names]) => names ?? []));
const layer = ([, names]) => (names ? { path: modules(names) } : { path: '^src/', pathNot: NAMED });
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

export default {
  forbidden: [
    ...against,
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
