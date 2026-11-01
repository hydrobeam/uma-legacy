// Static checks on the page's source, run by `./build.sh lint` (no browser, no build): what the browser suite can't see
// going stale.
// - dead CSS: a class or id in style.css's selectors that no page script, template or build.py names (as a whole word,
//   HTML entities aside). A name the code builds from parts is listed in BUILT with where it is built.
// - dead exports: an export of src/ that no other file (src/, test/ and its folders) names.
// - stale storage writes: `localStorage.setItem(key, JSON.stringify(x))` where `x` was not read or built in the same
//   function after its last await. A copy read earlier (at load, or before a request) overwrites what another tab or
//   call stored meanwhile; read what is stored, change your part, write it.
// - template ids: every id a module looks up by a literal name ($, $dialog, $input, $textarea, getElementById, a
//   querySelector/closest of '#id') exists on each page whose bundle holds the module (esbuild's metafile), in its
//   template with the panels filled in, in build.py's own markup or made by one of the bundle's modules. A module that
//   moves into the other page's bundle, or a template edit, would otherwise crash at load. A lookup that may miss on
//   purpose is listed in MAY_BE_MISSING, per page, with why An exemption whose id the page has, or that no module of the page looks
//   up, fails too.
// - store reads: a logic module (the types, io and model layers of LAYERS) reaches no store's values through any chain
//   of imports (esbuild's metafiles: static, dynamic and re-exports, either quote): it takes the settings and records
//   as arguments (`import type` only), so nothing it reads can subscribe an effect.
// - effects: only store.ts calls signals' `effect` (its `on` helper, which names an effect's signals and runs its body
//   untracked); anywhere else an effect would subscribe to whatever its body happens to read. A signal's `.subscribe()`
//   counts as one, and so does importing or re-exporting `effect`.
// - import cycles: the page's modules import one another without a cycle (Biome's noImportCycles too): in a cycle a
//   module can run before a declaration it reads at load.
// - layers: an import points to its own layer or one below (LAYERS), and nothing outside a page imports into it;
//   the imports that broke it when it became a failure are listed (KNOWN_AGAINST) until their module is split.
// - wiring: the career run machine's deps that no browser group checks are named as they must be (engine.key,
//   analysis.stop and outdate, view.drawCheck).
// - read before declared: no bundle reads a top-level variable at load before its declaration has run (samecode's
//   `loadOrder`, on the bundles as samecode builds them).
// - mount roots: no module of a page's bundle looks into a component's subtree but the module that draws it (MOUNTS),
//   the entry only at its mount points, a deliberate read listed with why (MOUNT_READS): by a selector or a class
//   lookup naming what the component draws, or by walking down from a mount point (its children, a query from it, a
//   write of its content), looked up or held in a variable.
// - components stay views: no .ts or .js module imports a .tsx one, by any import esbuild resolves (each bundle's
//   import graph) or a type import it erases (a relative path in the module that resolves to a .tsx file).
// Exits 1 with the list when any finds one; delete the dead code, or list a built name.
// Reports, printed and never a failure: ids in the planner templates nothing names, panels no page uses, the known
// imports against the layer order, and the `@ts-expect-error` markers left by renames to .ts.
import fs from 'node:fs';
import path from 'node:path';
import * as esbuild from 'esbuild';
import { bundle, loadOrder, loadOrderText } from './samecode.mjs';

const W = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
const read = (f) => fs.readFileSync(path.join(W, f), 'utf8');
// the files in `dir` whose names match `re`; `recursive`: in its subfolders too (src/career/)
const ls = (dir, re, recursive = false) =>
  fs
    .readdirSync(path.join(W, dir), { recursive })
    .filter((f) => re.test(f))
    .map((f) => path.join(dir, f));
const words = (text) => new Set(text.replace(/&#?\w+;/g, ' ').match(/[A-Za-z_][\w-]*/g));

// class names the code builds from parts (a whole-word search can't find them): name -> where
const BUILT = {
  y0: 'agenda.js, the year rows: ag-year y + the year',
  y1: 'agenda.js, the year rows: ag-year y + the year',
  y2: 'agenda.js, the year rows: ag-year y + the year',
  leg0: 'ui/AffTable.tsx, the legacy columns: leg + the legacy index',
  leg1: 'ui/AffTable.tsx, the legacy columns: leg + the legacy index',
};

const sources = [...ls('src', /\.(js|mjs|ts|tsx)$/, true), ...ls('.', /\.template\.(html|js)$/), 'build.py'].map(
  (f) => [f, read(f)],
);
const problems = [];

// ---------- dead CSS
{
  const css = read('src/style.css').replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, ' '));
  const used = words(sources.map(([, t]) => t).join('\n'));
  const dead = new Map();
  // selector text: what stands before a '{' that opens a rule (not a declaration block's values)
  for (const m of css.matchAll(/([^{};]*)\{/g)) {
    const selector = m[1];
    if (/^\s*@/.test(selector)) continue;
    for (const [, name] of selector.matchAll(/[.#]([A-Za-z_][\w-]*)/g))
      if (!used.has(name) && !(name in BUILT)) {
        const line = css.slice(0, m.index + m[0].length).split('\n').length;
        dead.set(name, [...(dead.get(name) || []), line]);
      }
  }
  for (const [name, lines] of dead)
    problems.push(`style.css: .${name} (lines ${lines.join(', ')}) is named by no script, template or build.py`);
}

// ---------- dead exports
{
  const others = [...sources, ...ls('test', /\.(mjs|ts)$/, true).map((f) => [f, read(f)])];
  for (const [file, text] of sources.filter(([f]) => f.startsWith('src')))
    for (const [, name] of text.matchAll(/^export (?:async )?(?:const|let|function\*?|class) ([A-Za-z_$][\w$]*)/gm)) {
      const re = new RegExp(`(?<![\\w$])${name.replace(/\$/g, '\\$')}(?![\\w$])`);
      if (!others.some(([f, t]) => f !== file && re.test(t)))
        problems.push(`${file}: export ${name} is used by no other file (unexport it, or delete it)`);
    }
}

// ---------- stale storage writes
for (const [file, text] of sources.filter(([f]) => f.startsWith('src'))) {
  const lines = text.split('\n');
  lines.forEach((line, k) => {
    const m = line.match(/localStorage\.setItem\([^,]+,\s*JSON\.stringify\(([A-Za-z_$][\w$]*)\)\)/);
    if (!m) return;
    let start = k;
    while (start > 0 && !/^(export )?(async )?function |^(export )?const [\w$]+ = (async )?\(/.test(lines[start]))
      start--;
    const body = lines.slice(start, k).join('\n'),
      after = body.slice(body.lastIndexOf('await ') + 1),
      name = m[1].replace(/\$/g, '\\$');
    if (!new RegExp(`(?:const|let)\\s+${name}(?![\\w$])|(?<![\\w$.])${name}\\s*=(?!=)`).test(after))
      problems.push(
        `${file}:${k + 1}: stores ${m[1]}, a copy not read from storage in this function (after its last await)`,
      );
  });
}

// ---------- template ids: every id a module looks up by name exists on each page whose bundle holds the module
// A page's markup is its template with the shared panels filled in (as build.py fills them), the markup build.py
// writes itself (the theme button) and what the bundle's modules make with an id. A lookup a module makes on purpose
// where the element may be missing is listed in MAY_BE_MISSING, under each page (its bundle's entry) where it may be,
// with why: an exemption on one page leaves the other page's lookup checked.
const MAY_BE_MISSING = {
  'src/loop/main.tsx': {},
  'src/career/main.tsx': {
    'data-example': "data.js probes it: the loop page's precomputed example result; the career page has none",
    'c-peruma':
      "cards.js: the loop page's Deck per uma switch; the career page has none (cards.js returns before using it)",
  },
};
const PAGES = { 'src/loop/main.tsx': 'index.template.html', 'src/career/main.tsx': 'career.template.html' };
const WORKER = 'src/worker.ts';
const LOOKUP = /(?<![\w$.])(?:\$|\$dialog|\$input|\$textarea|document\.getElementById)\(\s*(['"`])([^'"`$]*?)\1\s*\)/g;
const DYNAMIC_LOOKUP = /(?<![\w$.])(?:\$|\$dialog|\$input|\$textarea|document\.getElementById)\(\s*(?!['"])[^)]/g;
const SELECTOR_ID = /\.(?:querySelector(?:All)?|closest|matches)\(\s*(['"])#([A-Za-z][\w-]*)\1\s*\)/g;
const MADE_ID = /(?:\.id\s*=\s*|\bid:\s*|setAttribute\(\s*'id',\s*)(['"`])([A-Za-z][\w-]*)\1|\bid="([A-Za-z][\w-]*)"/g;
const PANELS = Object.fromEntries(
  [...read('panels.template.html').matchAll(/<!--panel:(\w+)-->([\s\S]*?)<!--\/panel:\1-->/g)].map((m) => [m[1], m[2]]),
);
const fillPanels = (page) => {
  let filled = page;
  let m = filled.match(/__PANEL_([A-Z]+)__/);
  for (; m; m = filled.match(/__PANEL_([A-Z]+)__/)) filled = filled.replace(m[0], PANELS[m[1].toLowerCase()]);
  return filled;
};
const idsIn = (text) => [...text.matchAll(MADE_ID)].map((m) => m[2] || m[3]);
const srcText = Object.fromEntries(sources.filter(([f]) => f.startsWith('src')));
// each page's and the worker's metafile (esbuild): its modules and their imports, paths as in `sources`
const metafiles = Object.fromEntries(
  [...Object.keys(PAGES), WORKER].map((entry) => [
    entry,
    esbuild.buildSync({ entryPoints: [entry], bundle: true, write: false, metafile: true, logLevel: 'error' }).metafile,
  ]),
);
// the modules of each page's bundle
const bundled = Object.fromEntries(
  Object.keys(PAGES).map((entry) => [entry, Object.keys(metafiles[entry].inputs).filter((f) => f.startsWith('src/'))]),
);
const looked = new Set();
const dynamic = Object.values(srcText).reduce((n, text) => n + [...text.matchAll(DYNAMIC_LOOKUP)].length, 0);
for (const [entry, template] of Object.entries(PAGES)) {
  const modules = bundled[entry];
  const ids = new Set([
    ...idsIn(fillPanels(read(template))),
    ...idsIn(read('build.py')),
    ...modules.flatMap((f) => idsIn(srcText[f])),
  ]);
  const lookedHere = new Set();
  for (const file of modules) {
    const text = srcText[file];
    for (const m of [...text.matchAll(LOOKUP)]
      .map((m) => m[2])
      .concat([...text.matchAll(SELECTOR_ID)].map((m) => m[2]))) {
      looked.add(m);
      lookedHere.add(m);
      const message = `${file}: looks up #${m}, which ${template} (with its panels) and its bundle's modules don't have`;
      if (!ids.has(m) && !(m in MAY_BE_MISSING[entry]) && !problems.includes(message)) problems.push(message);
    }
  }
  // an exemption holds only while its id is missing from the page and a module of the page looks it up: one whose id
  // the page now has would hide the lookups the code guards with it (cards.js's c-peruma guard)
  for (const id of Object.keys(MAY_BE_MISSING[entry])) {
    if (ids.has(id))
      problems.push(`MAY_BE_MISSING: #${id} is exempt on ${entry}'s page, which has it: drop the exemption`);
    if (!lookedHere.has(id))
      problems.push(`MAY_BE_MISSING: #${id} is exempt on ${entry}'s page, where no module looks it up: drop it`);
  }
}

// ---------- import cycles: none
// Without a cycle, a module's imports run before it, so a move between modules can't put a declaration after a
// load-time reader; within one it can (samecode checks the order of such reads in the bundles, `loadOrder`).
const cycles = [];
{
  // the static imports among the page's modules, over the three bundles (a dynamic import runs later: no cycle)
  const edges = new Map();
  for (const meta of Object.values(metafiles))
    for (const [file, input] of Object.entries(meta.inputs))
      if (file.startsWith('src/'))
        edges.set(file, [
          ...new Set([
            ...(edges.get(file) || []),
            ...input.imports
              .filter((i) => i.kind === 'import-statement' && i.path.startsWith('src/'))
              .map((i) => i.path),
          ]),
        ]);
  // the strongly connected components (Tarjan's algorithm) of more than one module, or a module importing itself
  let counter = 0;
  const index = new Map(),
    low = new Map(),
    stack = [],
    onStack = new Set();
  const connect = (v) => {
    index.set(v, counter);
    low.set(v, counter++);
    stack.push(v);
    onStack.add(v);
    for (const w of edges.get(v) || []) {
      if (!index.has(w)) {
        connect(w);
        low.set(v, Math.min(low.get(v), low.get(w)));
      } else if (onStack.has(w)) low.set(v, Math.min(low.get(v), index.get(w)));
    }
    if (low.get(v) !== index.get(v)) return;
    const component = [];
    for (let w = stack.pop(); ; w = stack.pop()) {
      onStack.delete(w);
      component.push(w);
      if (w === v) break;
    }
    if (component.length > 1 || (edges.get(v) || []).includes(v)) cycles.push(component.sort());
  };
  for (const v of edges.keys()) if (!index.has(v)) connect(v);
  for (const component of cycles)
    problems.push(
      `import cycle: ${component.join(', ')} import each other (a module in a cycle can run before what it imports ` +
        'has been declared): break it',
    );
}

// ---------- mount roots: what a component draws is looked into by its own module only
// Each mount point of a page, the module whose components draw into it, and what they draw (the ids, and the classes
// no other module of that page's bundle draws). Another module of the bundle that looks one up (`$`/getElementById of
// an id, or a querySelector/closest/matches selector naming the id or a class) would read or write a component's DOM
// behind its back: a write is undone, or kept against what the component would draw, by its next render. The entry
// renders into its mount points and marks them (attributes on the mount point itself), so it may look those up, but
// not walk down from one (TRAVERSAL: its children, a query from it, its content written), whether straight from the
// lookup or from a variable that holds it; nor may it name what is under one in a selector ('#results > *'). A
// lookup that stays on purpose is listed in MOUNT_READS with why; an exemption no module makes fails too.
// the shared setup's editors, drawn on both pages (ui/Setup.tsx mounts them)
const SETUP_MOUNTS = [
  [
    'umas-panel',
    'src/ui/Roster.tsx',
    ['h-roster', 'r-all', 'r-none', 'r-file', 'r-count', 'r-filter', 'rs', 'rs-dd', 'rs-menu', 'r-msg', 'roster'],
    ['ut', 'uol'],
  ],
];
const MOUNTS = {
  'src/career/main.tsx': [
    ...SETUP_MOUNTS,
    ['run-note', 'src/career/view/RunNote.tsx', ['run-act'], ['runprog']],
    ['lodds', 'src/career/view/Tree.tsx', ['odds-open', 'sim-try'], ['udodds']],
    [
      'tree',
      'src/career/view/Tree.tsx',
      ['deck-open', 'sched-open'],
      ['pn', 'pnh', 'pnl', 'pnclick', 'ptile', 'tnm', 'pf', 'pflab', 'pk', 'pgs', 'tdeck', 'rtgt', 'ctgs']
        .concat(['ctg', 'tgp', 'tgbar', 'tgfill', 'tgfig', 'tgdk', 'tgico', 'tgchart', 'tgcounts', 'tgc'])
        .concat(['tgcb', 'tgck', 'tgavg']),
    ],
    ['bp', 'src/career/view/Setup.tsx', [], []],
    ['set-sum', 'src/career/view/Setup.tsx', [], ['sumkv', 'sumk']],
    ['v-tabs', 'src/career/view/Pool.tsx', ['v-tab-own', 'v-tab-bor'], []],
    ['v-body', 'src/career/view/Pool.tsx', ['vets', 'v-pane'], ['umcell', 'vsec', 'vptitle', 'vctl', 'vpars']],
    [
      'node-root',
      'src/career/view/NodeDialog.tsx',
      ['node-dlg', 'node-title', 'node-body'],
      ['nodedlg', 'twho', 'vxy', 'srctag', 'simtag', 'vdel', 'tempty', 'onote', 'ctt', 'nact'],
    ],
    ['deck-root', 'src/career/view/Sections.tsx', ['deck-dlg', 'deck-title', 'tree-deck'], ['deckdlg']],
    [
      'sim-top',
      'src/career/analysis/view.tsx',
      [
        'simulation',
        'sim-compare',
        'sim-roll',
        'sim-add',
        'sim-wins',
        'sim-result',
        'sim-chance',
        'sim-verdict',
      ].concat(['ud-races']),
      ['simcol', 'simresult', 'simhdr', 'simchance', 'simidle', 'simverdict', 'simrefine', 'simodds', 'simbest']
        .concat(['simfit', 'nextc', 'nextdeck', 'cfaces', 'cface', 'cwith', 'udtop', 'udpor', 'udmid', 'udname'])
        .concat(['udout', 'udnm', 'udapt', 'udarow', 'udal', 'udac', 'udan', 'udtabs', 'udtab', 'udpanes', 'udpane'])
        .concat(['udskills', 'udnone', 'udfoot', 'udcards', 'udcard', 'udfr', 'udlegs', 'udleg', 'udlegh', 'udlegt'])
        .concat(['udlegp', 'udlegn', 'udguest', 'udleggp', 'udleggx', 'udsub', 'udrec', 'udrecl', 'udstat', 'udmore'])
        .concat(['udraces', 'udrace', 'udrw', 'udgr', 'udrn', 'udres', 'udact']),
    ],
    [
      'vpick-root',
      'src/career/view/ParentPicker.tsx',
      ['vpick-dlg', 'vpick-title', 'vp-tab-vets', 'vp-tab-new', 'vp-tab-auto', 'vp-vets', 'vp-new', 'vp-auto'].concat([
        'vp-list',
        'vp-filter',
        'vp-umas',
        'vp-auto-go',
      ]),
      ['vpt', 'vpth', 'vpts', 'vptg', 'vptw', 'vpgh', 'vpgrid'],
    ],
  ],
  'src/loop/main.tsx': [
    ...SETUP_MOUNTS,
    ['run-note', 'src/loop/RunNote.tsx', [], []],
    ['r-act', 'src/loop/RunNote.tsx', [], []],
    ['rtop', 'src/loop/Results.tsx', ['c-top', 'c-top-l'], []],
    [
      'results',
      'src/loop/Results.tsx',
      [],
      ['loop', 'lhead', 'members', 'mem', 'memb', 'facebtn', 'deck', 'dtitle', 'dwho', 'dcards', 'dc', 'oinfo', 'dodds']
        .concat(['dsk', 'swap', 'swrow', 'swho', 'swcards', 'swc', 'swout', 'swin', 'swto', 'swa', 'swd', 'swd1'])
        .concat(['swmore', 'swhead', 'lacts', 'same', 'sk', 'dnm', 'dbar', 'dlab', 'n3']),
    ],
    [
      'why-root',
      'src/loop/Affinity.tsx',
      ['why-dlg', 'why-body', 'why-title'],
      ['afpick', 'afpar', 'aflab', 'afpn', 'afview'],
    ],
    [
      'ex-root',
      'src/loop/Export.tsx',
      ['ex-dlg', 'ex-title', 'ex-close', 'ex-text', 'ex-copy', 'ex-load', 'ex-msg'],
      [],
    ],
  ],
};
// (file, selector) -> why it may stay
const MOUNT_READS = {
  'src/popover.js': {
    '.deck .dc':
      "markOwn: the career page's el()-built decks; on the loop page it never runs (initPopover({drawn: true})); " +
      'and the click delegation, which reads the opener it was clicked on (the Popover port, step 11)',
    '.mem[data-uma]': "markOwn: the career page's member blocks (it has none); never run on the loop page",
    '.mem': 'reads the clicked member block’s data-* for her panel (the Popover port, step 11)',
    '.mem .memb': 'the click delegation, reading the opener (the Popover port, step 11)',
    '.loop .facebtn': 'the click delegation, reading the opener (the Popover port, step 11)',
    '.loop': "reads a portrait's result card for her member block (the Popover port, step 11)",
    '.loop.first ${mem}': 'reads #1\'s member block for a "Same loop as #1" portrait (the Popover port, step 11)',
  },
};
{
  const SELECTOR = /\.(?:querySelector(?:All)?|closest|matches)(?:<[^>]*>)?\(\s*(['"`])((?:(?!\1).)*)\1/g;
  // a class lookup, read as the selector of its classes
  const CLASS_LOOKUP = /\.getElementsByClassName\(\s*(['"`])((?:(?!\1).)*)\1/g;
  // a walk down from an element, or a write of its content
  const TRAVERSAL =
    /^\s*\)?\s*\??\.\s*(?:children|childNodes|(?:first|last)(?:Element)?Child|querySelector(?:All)?|getElementsBy\w+|inner(?:HTML|Text)|outerHTML|textContent|replaceChildren|append(?:Child)?|prepend|insert\w*|removeChild|replaceChild)\b/;
  // the variable a lookup is held in: `const results = $('results')`, `…, results = $('results')`
  const HELD = /(?:\b(?:const|let|var)|,)\s*([A-Za-z_$][\w$]*)\s*(?::[^=]+)?=\s*$/;
  const used = new Set();
  for (const [entry, mounts] of Object.entries(MOUNTS)) {
    for (const file of bundled[entry]) {
      const text = srcText[file];
      const lookups = [...text.matchAll(LOOKUP)];
      const ids = lookups.map((m) => m[2]);
      const selectors = [
        ...[...text.matchAll(SELECTOR)].map((m) => m[2]),
        ...[...text.matchAll(CLASS_LOOKUP)].map((m) => m[2].replace(/(^|\s+)(?=\S)/g, '.')),
      ];
      for (const [root, owner, ownIds, ownClasses] of mounts) {
        if (file === owner) continue;
        const named = (id) => (id === root ? file !== entry : ownIds.includes(id));
        for (const id of ids.filter(named))
          problems.push(`${file}: looks up #${id}, which ${owner} draws (mount point #${root})`);
        // walking down from the mount point: straight from its lookup, or from the variable that holds it
        for (const m of lookups.filter((lookup) => lookup[2] === root)) {
          const after = text.slice(m.index + m[0].length);
          const held = text.slice(Math.max(0, m.index - 80), m.index).match(HELD)?.[1];
          const walks =
            TRAVERSAL.test(after) ||
            (held !== undefined &&
              new RegExp(`(?<![\\w$.])${held.replace(/\$/g, '\\$')}${TRAVERSAL.source.slice(1)}`, 'm').test(after));
          if (walks) problems.push(`${file}: walks into #${root}, which ${owner} draws, from its mount point`);
        }
        for (const selector of selectors) {
          const hit =
            [...selector.matchAll(/#([\w-]+)/g)].some((m) => named(m[1]) || (m[1] === root && file !== entry)) ||
            new RegExp(`#${root}(?![\\w-])\\s*[\\s>~+]\\s*\\S`).test(selector) ||
            [...selector.matchAll(/\.([\w-]+)/g)].some((m) => ownClasses.includes(m[1]));
          if (!hit) continue;
          if (MOUNT_READS[file]?.[selector] !== undefined) used.add(`${file} ${selector}`);
          else problems.push(`${file}: looks into ${owner}'s components (#${root}) with '${selector}'`);
        }
      }
    }
  }
  for (const [file, reads] of Object.entries(MOUNT_READS))
    for (const selector of Object.keys(reads))
      if (!used.has(`${file} ${selector}`))
        problems.push(`MOUNT_READS: ${file} no longer looks up '${selector}': drop it`);
}

// ---------- the career run machine's wiring (career/main.tsx `createRun`): the deps no browser group notices when
// they are miswired (review-8ab finding 4), each named as it must be
{
  const WIRES = [
    [/\bkey: stateKey,/, "engine.key is engine.ts's stateKey (a constant would take another release's saved answer)"],
    [
      /\banalysis: comparison,/,
      "analysis is the comparison's machine (a no-op stop leaves a comparison running across an edit; a no-op outdate " +
        'leaves it fresh across a new run)',
    ],
    [
      /drawCheck: \(\) => \{\s*renderResults\(\);\s*paint\(\);\s*\}/,
      'view.drawCheck redraws the results column and the status line only (a redraw of the open veterans dialog under ' +
        'a click would lose it: run.ts RunDeps)',
    ],
  ];
  for (const [wire, what] of WIRES)
    if (!wire.test(srcText['src/career/main.tsx']))
      problems.push(`src/career/main.tsx: the run machine's wiring: ${what}`);
}

// ---------- components stay views: a module that is not one (.ts, .js) imports no .tsx module
// What runs: each bundle's import graph (esbuild's metafiles), so every import esbuild resolves counts (a dynamic
// import(), either quote, a path without its extension). What esbuild erases before it resolves (an `import type`, a
// JSDoc `@import` or `import('…')` type): any relative path written in the module that resolves to a .tsx file.
// A module kept only to give a component's mount an old name another lane's code still imports is listed in
// VIEW_IMPORTS_KEPT with why; one that no longer imports it fails too.
const VIEW_IMPORTS_KEPT = {
  'src/setup.js src/ui/Setup.tsx':
    "career/main.tsx imports mountSetup as setup.js's initSetup; part 2 of step 11 imports it from ui/Setup.tsx and " +
    'deletes setup.js',
};
{
  const isView = (file) => file.endsWith('.tsx');
  const viewImports = new Set();
  for (const meta of Object.values(metafiles))
    for (const [file, input] of Object.entries(meta.inputs))
      if (file.startsWith('src/') && !isView(file))
        for (const imported of input.imports) if (isView(imported.path)) viewImports.add(`${file} ${imported.path}`);
  const resolvesToView = (file, spec) => {
    const base = path.join(path.dirname(file), spec);
    return [base, `${base}.tsx`, path.join(base, 'index.tsx')].find((f) => isView(f) && fs.existsSync(path.join(W, f)));
  };
  for (const [file, text] of sources.filter(([f]) => f.startsWith('src') && !isView(f)))
    for (const [, , spec] of text.matchAll(/(['"`])(\.{1,2}\/[^'"`\s]*)\1/g)) {
      const view = resolvesToView(file, spec);
      if (view) viewImports.add(`${file} ${view}`);
    }
  for (const pair of Object.keys(VIEW_IMPORTS_KEPT))
    if (!viewImports.has(pair))
      problems.push(`VIEW_IMPORTS_KEPT: ${pair.replace(' ', ' no longer imports ')}: drop it`);
  for (const pair of viewImports) {
    if (VIEW_IMPORTS_KEPT[pair] !== undefined) continue;
    const [file, view] = pair.split(' ');
    problems.push(`${file}: imports the component module ${view}; logic takes values, components import logic`);
  }
}

// ---------- read before declared: each bundle as samecode reads it (`loadOrder`)
for (const entry of [...Object.keys(PAGES), WORKER])
  for (const problem of loadOrder(await bundle(W, entry))) problems.push(`${entry} bundle: ${loadOrderText(problem)}`);

// ---------- reports (printed, never a failure)
const reports = [];
// ids in the templates that nothing names: no script, no style rule, no other markup (for=, aria-*, href="#")
{
  const templates = ls('.', /\.template\.html$/).filter((f) => f !== 'about.template.html');
  const others = (file) =>
    [...sources.filter(([f]) => f !== file), ['src/style.css', read('src/style.css')]].map(([, t]) => t).join('\n');
  // an id the code builds from a name and a suffix (`${id}-menu`) is named when its name is
  const suffixes = new Set(sources.flatMap(([, t]) => [...t.matchAll(/\$\{[^}]+\}(-[\w-]+)/g)].map((m) => m[1])));
  const isNamed = (id, text) => {
    const word = (w) => new RegExp(`(?<![\\w-])${w}(?![\\w-])`).test(text);
    return word(id) || [...suffixes].some((x) => id.endsWith(x) && word(id.slice(0, -x.length)));
  };
  const unnamed = [];
  for (const file of templates) {
    const text = read(file);
    for (const id of new Set(idsIn(text)))
      if (!isNamed(id, others(file)) && !isNamed(id, text.split(`id="${id}"`).join('')))
        unnamed.push(`${file.replace('.template.html', '')}#${id}`);
  }
  if (unnamed.length) reports.push(`ids no script, style or markup names (${unnamed.length}): ${unnamed.join(', ')}`);
  const unused = Object.keys(PANELS).filter(
    (name) => !templates.some((f) => read(f).includes(`__PANEL_${name.toUpperCase()}__`)),
  );
  if (unused.length) reports.push(`panels no page or panel uses (${unused.length}): ${unused.join(', ')}`);
}
// layers: an import points to its own layer or one below; a page's modules are imported by nothing outside the page.
// Any other import against them fails; the known ones (KNOWN_AGAINST) are reported. A logic module (types, io, model) that reaches a store's values
// through any chain of imports, static, dynamic or re-exported, is a failure: a shared module may read a store, so a
// logic module's call into one could read it for it.
// The rule's other logic (frontend.md: run machines, uma.moe sync, io) lives today inside modules that are views too, so
// they stay outside these layers until each is split, reading the store only in handlers and in `on` bodies (untracked;
// the review of step 4 traced every such read): loop/run.ts (the loop page's run machine: it changes loop/state.ts
// through its transitions and reads it with .peek(); the settings come in its deps), umamoesetup.js (the uma.moe panel with its sync: `sigOf` and `query`
// read the targets), loop/io.js (the loop page's Export and Import: it writes the store by design, an editor), career/job.js
// (the career job, built from the veterans and settings). career/run.ts, the career run machine, is a model module: the
// answer store's transitions come in its deps.
const LAYERS = [
  ['types', ['globals.d', 'must', 'solver', 'store', 'loop/types']],
  ['io', ['dom', 'data', 'engine', 'statestore', 'errorlog', 'worker', 'saved']],
  [
    'model',
    [
      'job',
      'pink',
      'proofs',
      'umadump',
      'umamoe',
      'career/roll',
      'career/keys',
      'career/run',
      'career/analysis/compare',
      'loop/job',
      'loop/keys',
    ],
  ],
  ['stores', ['settings', 'loop/state', 'veterans/store', 'career/state', 'career/analysis/state']],
  ['shared', null], // every module not named elsewhere
  [
    'page:loop',
    [
      'loop/main',
      'loop/run',
      'loop/io',
      'loop/RunNote',
      'loop/Results',
      'loop/Affinity',
      'loop/Export',
      'loop/RenderGuard',
    ],
  ],
  [
    'page:career',
    [
      'career/main',
      'career/analysis/view',
      'career/analysis/Details',
      'career/options',
      'career/job',
      'career/io',
      'career/view/parts',
      'career/view/dialogs',
      'career/view/RunNote',
      'career/view/Tree',
      'career/view/NodeDialog',
      'career/view/ParentPicker',
      'career/view/Sections',
      'career/view/Results',
      'career/view/Pool',
      'career/view/poolState',
      'career/view/Setup',
    ],
  ],
];
// the imports against the layers that stand (none since the veterans' store was split, step 10)
const KNOWN_AGAINST = [];
{
  const moduleOf = (file) => file.replace(/^src\//, '').replace(/\.(js|mjs|ts|tsx)$/, '');
  const layerOf = (mod) => {
    const k = LAYERS.findIndex(([, mods]) => mods?.includes(mod));
    return k < 0 ? LAYERS.findIndex(([, mods]) => mods === null) : k;
  };
  const against = [];
  const logic = ['types', 'io', 'model'].map((name) => LAYERS.findIndex(([layer]) => layer === name)),
    stores = LAYERS.findIndex(([layer]) => layer === 'stores');
  // the value imports among the page's modules (esbuild's metafiles, which leave out `import type`: types are erased,
  // so they set no load order and read no store): every kind, a dynamic import() and a re-export included
  const imports = new Map();
  for (const meta of Object.values(metafiles))
    for (const [file, input] of Object.entries(meta.inputs))
      if (file.startsWith('src/'))
        imports.set(file, [
          ...new Set([
            ...(imports.get(file) || []),
            ...input.imports.map((i) => i.path).filter((f) => f.startsWith('src/')),
          ]),
        ]);
  for (const [file, targets] of imports) {
    const from = layerOf(moduleOf(file));
    for (const target of targets) {
      const to = layerOf(moduleOf(target));
      const pages = [from, to].map((k) => LAYERS[k][0]);
      if (to > from || (pages[0] !== pages[1] && pages[1].startsWith('page:')))
        against.push(`${moduleOf(file)} (${pages[0]}) -> ${moduleOf(target)} (${pages[1]})`);
    }
  }
  const known = against.filter((a) => KNOWN_AGAINST.includes(a));
  for (const a of against.filter((x) => !KNOWN_AGAINST.includes(x)))
    problems.push(`import against the layer order: ${a} (an import points to its own layer or one below)`);
  for (const k of KNOWN_AGAINST.filter((x) => !against.includes(x)))
    problems.push(`KNOWN_AGAINST: ${k} no longer stands: drop it`);
  if (known.length) reports.push(`known imports against the layer order (${known.length}): ${known.join(', ')}`);
  for (const file of imports.keys()) {
    if (!logic.includes(layerOf(moduleOf(file)))) continue;
    // the shortest import chain from `file` to a store, breadth first
    const via = new Map([[file, null]]);
    for (const queue = [file]; queue.length; ) {
      const at = queue.shift();
      if (layerOf(moduleOf(at)) === stores) {
        const chain = [];
        for (let f = at; f; f = via.get(f)) chain.unshift(moduleOf(f));
        problems.push(
          `${file}: reaches the store ${moduleOf(at)} (${chain.join(' -> ')}): a logic module takes the store's values ` +
            'as arguments (import type only)',
        );
        break;
      }
      for (const next of imports.get(at) || [])
        if (!via.has(next)) {
          via.set(next, at);
          queue.push(next);
        }
    }
  }
}
// effects: only store.ts makes one (`on`); comments are dropped first (they name effects). A signal's `.subscribe()`
// is an effect too, and so is a module importing or re-exporting signals' `effect` (either quote)
{
  const code = (text) => text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"`\\])\/\/.*$/gm, '$1');
  const MAKES_EFFECT = /\beffect\s*\(|\.subscribe\s*\(/,
    TAKES_EFFECT = /\b(?:import|export)\b[^;]*\beffect\b[^;]*from\s*['"]@preact\/signals/;
  for (const [file, text] of sources.filter(([f]) => f.startsWith('src') && f !== path.join('src', 'store.ts')))
    if (MAKES_EFFECT.test(code(text)) || TAKES_EFFECT.test(text))
      problems.push(
        `${file}: makes or passes on an effect outside store.ts's \`on\` (name its signals: on([a, b], (a, b) => ...))`,
      );
}
// the type checker's `@ts-expect-error` markers: each a line a module's rename to .ts left for its strict commit
// (a count of what the strict commits have left to remove)
{
  const markers = sources
    .filter(([f]) => f.startsWith('src'))
    .flatMap(([f, text]) =>
      text.split('\n').flatMap((line, k) => (line.includes('@ts-expect-error') ? [`${f}:${k + 1}`] : [])),
    );
  if (markers.length) reports.push(`@ts-expect-error markers (${markers.length}): ${markers.join(', ')}`);
}

if (problems.length) {
  console.log(problems.join('\n'));
  console.log(`${problems.length} static check failure${problems.length === 1 ? '' : 's'} (webapp/test/static.mjs)`);
  process.exit(1);
}
for (const report of reports) console.log(`report: ${report}`);
console.log(
  `static checks: no dead CSS, no dead exports, no stale storage writes; ${looked.size} ids looked up, each on its pages ` +
    `(${dynamic} lookups by a computed id unchecked)`,
);
