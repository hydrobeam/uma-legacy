// Static checks on the page's source that no standard tool expresses, run by `./build.sh lint` (no browser, no build):
// what the browser suite can't see going stale. The rest of what the page's source must keep is checked by standard
// tools in lint: dead CSS by PurgeCSS (test/deadcss.mjs), dead exports, files and dependencies and imports of no file by
// knip (knip.jsonc) and tsc, import cycles by Biome (noImportCycles), the module layers, the logic that reaches no
// store and the components that stay views by dependency-cruiser (.dependency-cruiser.mjs), effects outside store.ts
// and exported mutable bindings by Biome (noRestrictedImports, lint/*.grit), `@ts-ignore` by Biome (noTsIgnore).
// - template ids: every id a module looks up by a literal name ($, $input, getElementById, a querySelector/closest of
//   '#id') exists on each page whose bundle holds the module (esbuild's metafile), in its page with its partials filled
//   in, in the build plugins' own markup or made by one of the bundle's modules. A module that moves into the other
//   page's bundle, or a page edit, would otherwise crash at load. A lookup that may miss on purpose is listed in
//   MAY_BE_MISSING, per page, with why. An exemption whose id the page has, or that no module of the page looks up,
//   fails too.
// - wiring: the career run machine's deps that no browser group checks are named as they must be (engine.key,
//   analysis.stop and outdate, view.drawCheck).
// - mount roots: no module of a page's bundle looks into a component's subtree but the module that draws it (MOUNTS),
//   the entry only at its mount points, a deliberate read listed with why (MOUNT_READS): by a selector or a class
//   lookup naming what the component draws, or by walking down from a mount point (its children, a query from it, a
//   write of its content), looked up or held in a variable; a write before any component draws there (MOUNT_WRITES).
// - dialogs and mounts: every modal dialog is ui/Dialog.tsx's (no `<dialog>` in a page, no showModal() anywhere
//   else; the ones not yet ported are listed, TEMPLATE_DIALOGS, until each goes), and only the two entries render into
//   a mount point (preact's render, RenderGuard's renderGuarded).
// - deferred writes and listeners at import: no requestAnimationFrame but the ones listed (RAF_KEPT: a measured DOM
//   write runs from the layout effect of the component that draws what it measures), and no listener added at a
//   module's top level outside the entries but the load-time effects listed (LOAD_LISTENERS); no view built from a
//   markup string (innerHTML, outerHTML, insertAdjacentHTML): every view is a component.
// Exits 1 with the list when any finds one.
// Reports, printed and never a failure: ids in the planner pages nothing names, partials no page uses, and the
// `@ts-expect-error` markers (tsc accepts them; the page has none).
import fs from 'node:fs';
import path from 'node:path';
import * as esbuild from 'esbuild';

const W = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
const read = (f) => fs.readFileSync(path.join(W, f), 'utf8');
// the files in `dir` whose names match `re`; `recursive`: in its subfolders too (src/career/)
const ls = (dir, re, recursive = false) =>
  fs
    .readdirSync(path.join(W, dir), { recursive })
    .filter((f) => re.test(f))
    .map((f) => path.join(dir, f));
// the pages, their partials and the build's plugins (vite/), which write markup of their own
const PAGE_FILES = ['index.html', 'loop.html', 'about.html'];
const BUILD_FILES = ['vite.config.mjs', ...ls('vite', /\.mjs$/)];
const sources = [
  ...ls('src', /\.(js|mjs|ts|tsx)$/, true),
  ...PAGE_FILES,
  ...ls('partials', /\.hbs$/),
  ...BUILD_FILES,
].map((f) => [f, read(f)]);
const problems = [];

// ---------- template ids: every id a module looks up by name exists on each page whose bundle holds the module
// A page's markup is its page with the shared partials filled in (as the build fills them), the markup the build
// writes itself (the theme button) and what the bundle's modules make with an id. A lookup a module makes on purpose
// where the element may be missing is listed in MAY_BE_MISSING, under each page (its bundle's entry) where it may be,
// with why: an exemption on one page leaves the other page's lookup checked.
const MAY_BE_MISSING = {
  'src/loop/main.tsx': {},
  'src/career/main.tsx': {},
};
const PAGES = { 'src/loop/main.tsx': 'loop.html', 'src/career/main.tsx': 'index.html' };
const LOOKUP = /(?<![\w$.])(?:\$|\$input|document\.getElementById)\(\s*(['"`])([^'"`$]*?)\1\s*\)/g;
const DYNAMIC_LOOKUP = /(?<![\w$.])(?:\$|\$input|document\.getElementById)\(\s*(?!['"])[^)]/g;
const SELECTOR_ID = /\.(?:querySelector(?:All)?|closest|matches)\(\s*(['"])#([A-Za-z][\w-]*)\1\s*\)/g;
const MADE_ID = /(?:\.id\s*=\s*|\bid:\s*|setAttribute\(\s*'id',\s*)(['"`])([A-Za-z][\w-]*)\1|\bid="([A-Za-z][\w-]*)"/g;
// the partials (partials/*.hbs), each `{{> name ...}}` of a page filled with its file (vite.config.mjs, handlebars)
const PANELS = Object.fromEntries(ls('partials', /\.hbs$/).map((f) => [path.basename(f, '.hbs'), read(f)]));
const fillPanels = (page) => {
  let filled = page;
  let m = filled.match(/\{\{> (\w+)[^}]*\}\}/);
  for (; m; m = filled.match(/\{\{> (\w+)[^}]*\}\}/)) filled = filled.replace(m[0], PANELS[m[1]]);
  return filled;
};
const idsIn = (text) => [...text.matchAll(MADE_ID)].map((m) => m[2] || m[3]);
const srcText = Object.fromEntries(sources.filter(([f]) => f.startsWith('src')));
// each page's metafile (esbuild): its modules and their imports, paths as in `sources`
const metafiles = Object.fromEntries(
  Object.keys(PAGES).map((entry) => [
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
    ...BUILD_FILES.flatMap((f) => idsIn(read(f))),
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
      const message = `${file}: looks up #${m}, which ${template} (with its partials) and its bundle's modules don't have`;
      if (!ids.has(m) && !(m in MAY_BE_MISSING[entry]) && !problems.includes(message)) problems.push(message);
    }
  }
  // an exemption holds only while its id is missing from the page and a module of the page looks it up: one whose id
  // the page now has would hide the lookups the code guards with it (cards.ts's c-peruma guard)
  for (const id of Object.keys(MAY_BE_MISSING[entry])) {
    if (ids.has(id))
      problems.push(`MAY_BE_MISSING: #${id} is exempt on ${entry}'s page, which has it: drop the exemption`);
    if (!lookedHere.has(id))
      problems.push(`MAY_BE_MISSING: #${id} is exempt on ${entry}'s page, where no module looks it up: drop it`);
  }
}

// ---------- mount roots: what a component draws is looked into by its own module only
// Each mount point of a page, the module whose components draw into it, and what they draw (the ids, and the classes
// no other module of that page's bundle draws). Another module of the bundle that looks one up (`$`/getElementById of
// an id, or a querySelector/closest/matches selector naming the id or a class) would read or write a component's DOM
// behind its back: a write is undone, or kept against what the component would draw, by its next render. The entry
// renders into its mount points and marks them (attributes on the mount point itself), so it may look those up, but
// not walk down from one (TRAVERSAL: its children, a query from it, its content written), whether straight from the
// lookup or from a variable that holds it; nor may it name what is under one in a selector ('#results > *'). A
// lookup that stays on purpose is listed in MOUNT_READS with why, and a module that writes into a mount point before
// any component draws there in MOUNT_WRITES; an exemption no module makes fails too.
// the shared setup's editors, drawn on both pages (ui/Setup.tsx mounts them)
const SETUP_MOUNTS = [
  ['pop-root', 'src/ui/CardPop.tsx', [], ['cpop', 'pgives', 'pfoot', 'gtl', 'inpopup']],
  [
    'scen-dd',
    'src/ui/Scenario.tsx',
    ['x-scen-l', 'x-scen', 'scen-menu'],
    ['scenpick', 'stile', 'ssub', 'sgolds', 'sgold'],
  ],
  [
    'rank-ctl',
    'src/ui/Scenario.tsx',
    ['rank-l', 'rank-q', 'rank-note', 'rank'],
    ['rankseg', 'rk', 'rkdash', 'rtab', 'rfoot'],
  ],
  [
    'umas-panel',
    'src/ui/Roster.tsx',
    ['h-roster', 'r-all', 'r-none', 'r-file', 'r-count', 'r-filter', 'rs', 'rs-dd', 'rs-menu', 'r-msg', 'roster'],
    ['ut', 'uol'],
  ],
  [
    'cards-panel',
    'src/ui/Cards.tsx',
    ['h-deck', 'c-all', 'c-none', 'c-file', 'c-count', 'c-msg', 'c-filter', 'cs', 'cs-dd', 'cs-menu', 'cards'],
    ['cgroup', 'ctiles', 'ct', 'cban'],
  ],
  [
    'skills-panel',
    'src/ui/Targets.tsx',
    ['h-skills', 't-preset', 't-add', 't-sugg', 'targets', 'preset-dlg', 'pr-title', 'pr-list', 'pr-msg', 'pr-save']
      .concat(['pr-new', 'prn-title', 'prn-name', 'prn-msg', 'prn-save', 'pr-view', 'prv-title', 'prv-tiles'])
      .concat(['prv-ren', 'prv-name', 'prv-msg', 'prv-rename']),
    ['adder', 'sugg', 'snote', 'tgt', 'prlist', 'prow', 'pview', 'pname', 'pmeta', 'pcur', 'pbtns', 'ptrash'],
  ],
];
const MOUNTS = {
  'src/career/main.tsx': [
    ...SETUP_MOUNTS,
    ['run-note', 'src/career/view/RunNote.tsx', ['run-act'], ['runprog']],
    [
      'results',
      'src/career/view/Options.tsx',
      ['save-why', 'opt-back'],
      ['optwrap', 'optab', 'otags', 'otag', 'onum', 'orole', 'altw', 'thtip'],
    ],
    [
      'save-root',
      'src/career/view/Options.tsx',
      ['save-dlg', 'save-title'],
      ['savebody', 'savemore', 'oloop', 'optnote'],
    ],
    ['um-root', 'src/career/view/Umamoe.tsx', ['um-open'], []],
    [
      'um-dlg-root',
      'src/career/view/Umamoe.tsx',
      [
        'um-dlg',
        'um-title',
        'um-body',
        'um-keyrow',
        'um-key',
        'um-save',
        'um-err',
        'um-on',
        'um-msg',
        'um-again',
        'um-what',
        'um-full',
        'um-list',
      ].concat(['um-forget', 'um-sort', 'um-sort-menu', 'um-sort-dd']),
      ['umline', 'umtog', 'umerr'],
    ],
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
    [
      'vets-root',
      'src/career/view/Pool.tsx',
      [
        'set-vets',
        'h-vets',
        'v-head',
        'v-tabs',
        'v-tab-own',
        'v-tab-bor',
        'vs',
        'vs-menu',
        'vs-dd',
        'v-body',
        'vets',
      ].concat(['v-pane', 'v-file', 'v-paste', 'v-clear', 'v-msg']),
      ['umcell', 'vsec', 'vptitle', 'vctl', 'vpars', 'vtools', 'filebtn'],
    ],
    ['paste-root', 'src/career/view/Io.tsx', ['paste-dlg', 'paste-title', 'paste-text', 'paste-load'], []],
    ['vx-root', 'src/career/view/Io.tsx', ['vx-dlg', 'vx-title', 'vx-text', 'vx-copy', 'vx-dl', 'vx-msg'], []],
    [
      'node-root',
      'src/career/view/NodeDialog.tsx',
      ['node-dlg', 'node-title', 'node-body'],
      ['nodedlg', 'twho', 'vxy', 'srctag', 'simtag', 'vdel', 'tempty', 'onote', 'ctt', 'nact'],
    ],
    ['deck-root', 'src/career/view/Sections.tsx', ['deck-dlg', 'deck-title', 'tree-deck'], ['deckdlg']],
    [
      'sched-root',
      'src/ui/Schedule.tsx',
      ['sched-dlg', 'sched-body', 'sched-title', 'sched-wintable', 'sched-view'],
      ['wintable', 'wtscroll', 'stabs', 'stab', 'agkey', 'sview', 'tablewrap'],
    ],
    [
      'vet-editors',
      'src/veterans/Editors.tsx',
      ['pick-dlg', 'pick-title', 'pick-filter', 'pick-list', 'ask-dlg', 'ask-title', 'ask-text', 'ask-btns'].concat([
        'wins-dlg',
        'wins-title',
        'wins-count',
        'wins-list',
        'col-dlg',
        'col-title',
      ]),
      ['askrow', 'coldlg', 'colgrid', 'spstars', 'spstar', 'upick'],
    ],
    ['add-pop', 'src/veterans/Editors.tsx', [], ['addlist', 'addsk', 'addn']],
    ['skill-root', 'src/career/view/SkillDetails.tsx', ['sk-dlg', 'sk-title'], ['gmdfaces', 'gmdcard', 'gmdcards']],
    [
      'sim-root',
      'src/career/analysis/view.tsx',
      [
        'sim-dlg',
        'sim-title',
        'sim-top',
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
    ['odds-root', 'src/ui/OddsDialog.tsx', ['odds-dlg', 'odds-title'], ['oddtab']],
    [
      'sched-root',
      'src/ui/Schedule.tsx',
      ['sched-dlg', 'sched-body', 'sched-title', 'sched-wintable', 'sched-view'],
      ['wintable', 'wtscroll', 'stabs', 'stab', 'agkey', 'sview', 'tablewrap'],
    ],
  ],
};
// 'file mount point' -> why the module may write into it
const MOUNT_WRITES = {
  'src/errorlog.ts run-note':
    "the status line's text, only for an error before the entry has run to its end (`started`): it stopped the entry before it drew anything",
};
// (file, selector) -> why it may stay
const MOUNT_READS = {
  'src/popover.ts': {
    '.deck .dc': 'the click delegation: which opener was clicked, and its data-* for the panel (ui/CardPop.tsx)',
    '.mem': 'reads the clicked member block’s data-* for her panel, at the click',
    '.mem .memb': 'the click delegation: which opener was clicked',
    '.loop .facebtn': 'the click delegation: which opener was clicked',
    '.loop': "reads a portrait's result card for her member block, at the click",
    '.loop.first ${mem}': 'reads #1\'s member block for a "Same loop as #1" portrait, at the click',
    '.cpop': 'a click inside a panel (or a menu drawn as one) keeps the panel open',
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
        if (MOUNT_WRITES[`${file} ${root}`] !== undefined && ids.includes(root)) {
          used.add(`${file} ${root}`);
          continue;
        }
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
  for (const write of Object.keys(MOUNT_WRITES))
    if (!used.has(write)) problems.push(`MOUNT_WRITES: no longer made: '${write}': drop it`);
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
      /on\(\[vets, places, searchHeld\], \(list, _places, held\) => \{\s*if \(held\) return;/,
      "the veterans effect holds the search while a pool veteran's G1 wins window or a skills prompt is open " +
        '(pickers.ts searchHeld; review-10 1, 2)',
    ],
    [
      /mount\('sched-root', <ScheduleDialog \/>, 'The race schedule', schedOpened\);/,
      "the race schedule's guard draws it again for a new opening (Schedule.tsx schedOpened: review-11b 5)",
    ],
    [
      /drawCheck: paint,/,
      'view.drawCheck redraws the status line only (the results column follows the answer store; a redraw of the open ' +
        'veterans dialog under a click would lose it: run.ts RunDeps)',
    ],
  ];
  for (const [wire, what] of WIRES)
    if (!wire.test(srcText['src/career/main.tsx']))
      problems.push(`src/career/main.tsx: the run machine's wiring: ${what}`);
  if (
    !/renderGuarded\(<ScheduleDialog \/>, \$\('sched-root'\), 'The race schedule', schedOpened\);/.test(
      srcText['src/loop/main.tsx'],
    )
  )
    problems.push('src/loop/main.tsx: the race schedule draws inside a guard that draws it again for a new opening');
}

// ---------- dialogs and mounts: ui/Dialog.tsx is every modal dialog (it calls showModal() and close() from its `open`,
// restores the focus, closes on the backdrop); a render into a mount point starts a view's effects, which the entries
// alone decide (frontend.md §2 Rules). The template dialogs a module still opens by hand, until each is ported:
const TEMPLATE_DIALOGS = [];
const ENTRIES = ['src/loop/main.tsx', 'src/career/main.tsx'];
{
  const code = (text) => text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"`\\])\/\/.*$/gm, '$1');
  const templateDialogs = new Set(
    [...PAGE_FILES, ...ls('partials', /\.hbs$/), ...BUILD_FILES].flatMap((f) =>
      [...read(f).matchAll(/<dialog\b[^>]*\bid="([\w-]+)"/g)].map((m) => m[1]),
    ),
  );
  for (const id of templateDialogs)
    if (!TEMPLATE_DIALOGS.includes(id))
      problems.push(
        `#${id}: a <dialog> in a template; render it with ui/Dialog.tsx (its open prop shows and closes it)`,
      );
  for (const id of TEMPLATE_DIALOGS)
    if (!templateDialogs.has(id)) problems.push(`TEMPLATE_DIALOGS: #${id} is no template's any more: drop it`);
  for (const [file, text] of sources.filter(([f]) => f.startsWith('src') && /\.tsx?$/.test(f))) {
    const body = code(text);
    if (file !== path.join('src', 'ui', 'Dialog.tsx') && /\.showModal\(/.test(body) && !TEMPLATE_DIALOGS.length)
      problems.push(`${file}: calls showModal(); a dialog is ui/Dialog.tsx's, opened by its open prop`);
    if (
      !ENTRIES.includes(file) &&
      file !== path.join('src', 'ui', 'RenderGuard.tsx') &&
      /(^|[^.\w])(render|renderGuarded)\(/m.test(body)
    )
      problems.push(
        `${file}: renders into a mount point; the entries alone do (ui/Setup.tsx's setupViews hands them views)`,
      );
  }
}

// ---------- deferred writes, and listeners added at import
const RAF_KEPT = {
  'src/veterans/store.ts': "the veterans' save waits a frame (afterFrame): it writes storage, not the DOM",
};
const LOAD_LISTENERS = {
  'src/errorlog.ts': 'first of all: it logs every uncaught error',
  'src/gmtip.ts': 'the game-style tooltips listen to the whole document on both pages',
};
{
  const code = (text) => text.replace(/\/\*[\s\S]*?\*\//g, '').replace(/(^|[^:'"`\\])\/\/.*$/gm, '$1');
  for (const [file, text] of sources.filter(([f]) => f.startsWith('src') && /\.tsx?$/.test(f))) {
    const body = code(text);
    if (/\brequestAnimationFrame\(/.test(body) && !RAF_KEPT[file])
      problems.push(
        `${file}: requestAnimationFrame: a DOM write runs from the layout effect of the component it measures`,
      );
    if (/\.(innerHTML|outerHTML)\s*=|\.insertAdjacentHTML\(/.test(body))
      problems.push(`${file}: builds a view from a markup string; draw it as a component`);
    if (/^[\w.]*addEventListener\(/m.test(body) && !LOAD_LISTENERS[file] && !ENTRIES.includes(file))
      problems.push(
        `${file}: adds a listener at import; the entries make load-time effects (or list it in LOAD_LISTENERS)`,
      );
  }
  for (const file of [...Object.keys(RAF_KEPT), ...Object.keys(LOAD_LISTENERS)])
    if (!sources.some(([f]) => f === file)) problems.push(`RAF_KEPT/LOAD_LISTENERS: ${file} is gone: drop it`);
}

// ---------- reports (printed, never a failure)
const reports = [];
// ids in the templates that nothing names: no script, no style rule, no other markup (for=, aria-*, href="#")
{
  const templates = PAGE_FILES.filter((f) => f !== 'about.html');
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
        unnamed.push(`${file.replace('.html', '')}#${id}`);
  }
  if (unnamed.length) reports.push(`ids no script, style or markup names (${unnamed.length}): ${unnamed.join(', ')}`);
  const unused = Object.keys(PANELS).filter(
    (name) => ![...templates, ...ls('partials', /\.hbs$/)].some((f) => read(f).includes(`{{> ${name}`)),
  );
  if (unused.length) reports.push(`panels no page or panel uses (${unused.length}): ${unused.join(', ')}`);
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
  `static checks: ${looked.size} ids looked up, each on its pages ` + `(${dynamic} lookups by a computed id unchecked)`,
);
