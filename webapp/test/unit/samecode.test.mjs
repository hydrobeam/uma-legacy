// samecode's parts (test/samecode.mjs): its readers on small bundles shaped as esbuild writes them, and whole
// comparisons of snippets bundled as samecode bundles them: node --test 'test/unit/*.test.*' (in webapp)
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { after, test } from 'node:test';
import { bundle, compare, effects, listDiff, loadOrder, units } from '../samecode.mjs';

const iife = (...body) => ['(() => {', ...body, '})();', ''].join('\n');
const unitTexts = (text) => units(text).map((unit) => unit.map((t) => t.text).join(' '));

test('declarations are not effects; calls, listeners and storage reads are', () => {
  const text = iife(
    '  // src/a.js',
    '  var ENTITY = { "&": "&amp;" };',
    '  var seen = new Set();',
    '  var add = (x) => {',
    '    seen.add(x);',
    '  };',
    '  var twice = (x) => add(x), half = (x) => x / 2;',
    '  var api = { get(k) {',
    '    return read(k);',
    '  }, put: (k) => write(k) };',
    '  function esc(t) {',
    '    return t;',
    '  }',
    '  var SAVED = loadSaved();',
    '  document.addEventListener("click", (e) => {',
    '    add(e);',
    '  });',
    '  var LIMIT = 3;',
    '  var list = [',
    '    1',
    '  ];',
  );
  assert.deepEqual(effects(text), ['var SAVED=loadSaved();', 'document.addEventListener("click",(e)=>{add(e);});']);
});

test('an initialiser that calls at load is an effect, however it is wrapped', () => {
  for (const line of [
    'var A = (() => read())();',
    'var A = { v: read() };',
    'var A = [read()];',
    'var A = "a" + read();',
    'var A = new Thing();',
    'var A = tag`x`;',
    // biome-ignore lint/suspicious/noTemplateCurlyInString: the source text of a template literal
    'var A = `${read()}`;',
    'var A = (x) => x, B = read();',
    'var A = i++;',
  ])
    assert.deepEqual(effects(iife(`  ${line}`)).length, 1, line);
});

test('lines inside a template literal, a string or a comment are not units', () => {
  const text = iife(
    '  var html = `',
    '  <div>${f(`',
    '  inner`)}</div>',
    '  run();`;',
    '  /*',
    '  stop();',
    '  */',
    '  var re = /[/]x/g;',
    '  var half = 1 / 2;',
    '  go();',
  );
  assert.deepEqual(
    effects(text).map((e) => e.slice(0, 10)),
    ['var html=`', 'go();'],
  );
  assert.equal(units(text).length, 5); // html, re, half, go, the frame
});

test('a unit holds its body; `++` and `--` are one token', () => {
  const text = iife('  function f(x) {', '    if (x) {', '      a();', '    } else {', '      b++;', '    }', '  }');
  assert.deepEqual(unitTexts(text)[0], 'function f ( x ) { if ( x ) { a ( ) ; } else { b ++ ; } }');
});

test('a reordered effect changes the order; a moved declaration does not', () => {
  const a = iife('  var x = 1;', '  read();', '  listen();', '  function f() {', '  }');
  const b = iife('  function f() {', '  }', '  var x = 1;', '  listen();', '  read();');
  const result = compare(a, b);
  assert.deepEqual([result.gone, result.added], [[], []]);
  assert.equal(result.sameOrder, false);
});

test('path comments and line breaks are dropped; a changed unit is found once per excess', () => {
  const a = units(iife('  // src/x.js', '  go(1, 2);', '  go(1, 2);'));
  const b = units(iife('  // src/y/x.ts', '  go(', '    1,', '    2', '  );', '  stop(a - -b);'));
  const text = (us) => us.slice(0, -1).map((u) => u.map((t) => t.text).join(''));
  assert.deepEqual(listDiff(text(a), text(b)), [['go(1,2);'], ['stop(a--b);']]);
});

// Whole comparisons: each tree is a folder of modules, bundled from `main.js` (or `main.ts`) as samecode does
const scratch = fs.mkdtempSync(path.join(os.tmpdir(), 'samecode-test-'));
after(() => fs.rmSync(scratch, { recursive: true }));
let trees = 0;
const bundleOf = async (files) => {
  const dir = path.join(scratch, String(trees++));
  for (const [name, source] of Object.entries(files)) {
    fs.mkdirSync(path.dirname(path.join(dir, name)), { recursive: true });
    fs.writeFileSync(path.join(dir, name), source);
  }
  return bundle(
    dir,
    Object.keys(files).find((name) => /^main\.[jt]s$/.test(name)),
  );
};
const sameCode = async (before, after) => compare(await bundleOf(before), await bundleOf(after)).same;

// each a behaviour change samecode must report (the review of frontend steps 0-2 found the first ten passed it)
const CHANGES = {
  'statements reordered in a function': [
    'function f(){ a(); b(); } f(); function a(){} function b(){}',
    'function f(){ b(); a(); } f(); function a(){} function b(){}',
  ],
  'if/else branches swapped': [
    'function f(x){ if (x) { a(); } else { b(); } } f(1); function a(){} function b(){}',
    'function f(x){ if (x) { b(); } else { a(); } } f(1); function a(){} function b(){}',
  ],
  'a statement moved out of an if': [
    'function f(x){ if (x) { a(); } b(); } f(1); function a(){} function b(){}',
    'function f(x){ if (x) { } a(); b(); } f(1); function a(){} function b(){}',
  ],
  'a statement moved into a loop': [
    'function f(xs){ for (const x of xs) { a(x); } done(); } f([1]); function a(){} function done(){}',
    'function f(xs){ for (const x of xs) { done(); } a(xs); } f([1]); function a(){} function done(){}',
  ],
  'a listener moved to another function': [
    'function g(el){ el.addEventListener("click", h); } function k(el){ } g(document); k(document); function h(){}',
    'function g(el){ } function k(el){ el.addEventListener("click", h); } g(document); k(document); function h(){}',
  ],
  'try and catch bodies swapped': [
    'function f(){ try { a(); } catch (e) { b(); } } f(); function a(){} function b(){}',
    'function f(){ try { b(); } catch (e) { a(); } } f(); function a(){} function b(){}',
  ],
  '++ moved to the other operand': ['let a = 1, b = 2; window.x = a++ + b;', 'let a = 1, b = 2; window.x = a + ++b;'],
  'a cast regrouped': [
    'let x = 2, y = 3, z = 4; window.r = x * (/** @type {number} */ (y) + z);',
    'let x = 2, y = 3, z = 4; window.r = x * /** @type {number} */ (y) + z;',
  ],
  'top-level initialisers with calls reordered': [
    'var A = (() => localStorage.getItem("a"))(); var B = (() => localStorage.setItem("a", "1"))(); window.v = [A, B];',
    'var B = (() => localStorage.setItem("a", "1"))(); var A = (() => localStorage.getItem("a"))(); window.v = [A, B];',
  ],
  'object-literal initialisers with calls reordered': [
    'var A = { v: read() }; var B = { v: write() }; window.v = [A, B]; function read(){} function write(){}',
    'var B = { v: write() }; var A = { v: read() }; window.v = [A, B]; function read(){} function write(){}',
  ],
  'a statement moved before a return, into a branch': [
    'function f(x){ if (x) { return 1; } g(); return 2; } window.f = f; function g(){}',
    'function f(x){ if (x) { g(); return 1; } return 2; } window.f = f; function g(){}',
  ],
  'a literal changed': [
    'window.t = setTimeout(f, 300); function f(){}',
    'window.t = setTimeout(f, 30); function f(){}',
  ],
  "a call's parentheses moved past a cast": [
    'window.r = g(/** @type {any} */ (a)).h(); function g(x){ return x; } var a = 1;',
    'window.r = g(/** @type {any} */ (a).h()); function g(x){ return x; } var a = 1;',
  ],
};
for (const [name, [before, after]] of Object.entries(CHANGES))
  test(`reported: ${name}`, async () => {
    assert.equal(await sameCode({ 'main.js': before }, { 'main.js': after }), false);
  });

test('the same code: a function moved between modules', async () => {
  const before = {
    'main.js': "import { a, b } from './a.js';\nwindow.go = () => b(a());\n",
    'a.js': 'export function a() { return 1; }\nexport function b(x) { return x + 1; }\n',
  };
  const after = {
    'main.js': "import { a } from './a.js';\nimport { b } from './b.js';\nwindow.go = () => b(a());\n",
    'a.js': 'export function a() { return 1; }\n',
    'b.js': 'export function b(x) { return x + 1; }\n',
  };
  assert.equal(await sameCode(before, after), true);
});

test('the same code: a module renamed to .ts, its JSDoc annotations and casts made TS types', async () => {
  const before = {
    'main.js': "import { byId, first } from './list.js';\nwindow.go = () => first([byId('x')]);\n",
    'list.js': [
      '/** @param {Element[]} xs */',
      'export function first(xs) {',
      '  return xs.indexOf(/** @type {Element} */ (document.activeElement));',
      '}',
      'export const byId = (id) => /** @type {HTMLElement} */ (document.getElementById(id));',
    ].join('\n'),
  };
  const after = {
    'main.js': "import { byId, first } from './list.ts';\nwindow.go = () => first([byId('x')]);\n",
    'list.ts': [
      'export function first(xs: Element[]): number {',
      '  return xs.indexOf(document.activeElement as Element);',
      '}',
      'export const byId = (id: string) => document.getElementById(id) as HTMLElement;',
    ].join('\n'),
  };
  assert.equal(await sameCode(before, after), true);
});

// Declaration order (the review of the gate fixes ran each in a vm: every after-tree behaves differently). A read at
// load of a variable declared later is a failure of the after-tree on its own (`loadOrder`); a snapshot moved past an
// effect that changes what it reads changes the order of the load-time steps (`compare`).
const one = (source) => ({ 'main.js': source });
const LOAD_ORDER = {
  'a declaration read by an earlier one': [
    one('const B = 1;\nconst A = B;\nwindow.a = A;'),
    one('const A = B;\nconst B = 1;\nwindow.a = A;'),
  ],
  'an arrow declared after the effect whose function calls it': [
    one('const helper = () => 1;\ninit();\nfunction init() { window.x = helper(); }'),
    one('init();\nconst helper = () => 1;\nfunction init() { window.x = helper(); }'),
  ],
  'a class declared after a load-time use (esbuild makes it a var: no TDZ error)': [
    one('class K {}\nwindow.k = new K();'),
    one('window.k = new K();\nclass K {}'),
  ],
  'a constant moved into a module that another module in a cycle with it reads at load': [
    {
      'main.js':
        "import { opt } from './options.js';\nimport { ROLL_K } from './consts.js';\nexport { ROLL_K };\nwindow.o = opt;",
      'options.js': "import { ROLL_K } from './consts.js';\nexport const opt = ROLL_K * 2;",
      'consts.js': 'export const ROLL_K = 6;',
    },
    {
      'main.js': "import { opt } from './options.js';\nexport const ROLL_K = 6;\nwindow.o = opt;",
      'options.js': "import { ROLL_K } from './main.js';\nexport const opt = ROLL_K * 2;",
    },
  ],
};
for (const [name, [before, after]] of Object.entries(LOAD_ORDER))
  test(`read before declared, a failure: ${name}`, async () => {
    assert.deepEqual(loadOrder(await bundleOf(before)), []);
    const problems = loadOrder(await bundleOf(after));
    assert.equal(problems.length, 1, JSON.stringify(problems));
  });
const SNAPSHOTS = {
  'a spread snapshot moved past an effect that changes its source': [
    one('const state = { x: 0 };\nconst SNAP = { ...state };\nstate.x = 1;\nwindow.s = SNAP;'),
    one('const state = { x: 0 };\nstate.x = 1;\nconst SNAP = { ...state };\nwindow.s = SNAP;'),
  ],
  'a getter read moved past the effect that sets it': [
    one('const o = { _v: 0, get v() { return this._v; } };\nconst V = o.v;\no._v = 5;\nwindow.v = V;'),
    one('const o = { _v: 0, get v() { return this._v; } };\no._v = 5;\nconst V = o.v;\nwindow.v = V;'),
  ],
};
for (const [name, [before, after]] of Object.entries(SNAPSHOTS))
  test(`reported: ${name}`, async () => {
    const [a, b] = [await bundleOf(before), await bundleOf(after)];
    assert.deepEqual([loadOrder(a), loadOrder(b)], [[], []]);
    const result = compare(a, b);
    assert.deepEqual([result.gone, result.added, result.sameOrder], [[], [], false]);
  });

test('a load-order problem names the binding, both lines and modules, and the calls that read it', async () => {
  const [problem] = loadOrder(
    await bundleOf({
      'main.js': "import { start } from './start.js';\nstart();\nexport const LIMIT = 3;",
      'start.js':
        "import { LIMIT } from './main.js';\nexport function start() { check(); }\nfunction check() { window.n = LIMIT; }",
    }),
  );
  assert.equal(problem.name, 'LIMIT');
  assert.deepEqual(
    [problem.readModule, problem.declaredModule, problem.via],
    ['main.js', 'main.js', ['start', 'check']],
  );
  assert.ok(problem.readLine < problem.declaredLine);
});

test('not read at load: a function body nobody calls at load, a listener, a property named like a variable', async () => {
  const text = await bundleOf(
    one(
      'document.addEventListener("click", () => { window.x = LATE; });\n' +
        'function later() { return LATE; }\nwindow.f = later;\nwindow.o = { LATE: 1 }.LATE;\nconst LATE = 2;',
    ),
  );
  assert.deepEqual(loadOrder(text), []);
});

// esbuild's collision suffixes: an entry's own `search` moved to a new module imported first takes the unsuffixed
// name, and the other module's `search` becomes `search2`. Paired back, the move reads the same; a change does not.
const suffixBase = {
  'main.js':
    "import { s } from './s.js';\nfunction search() { return 2; }\nwindow.go = () => s() + search();\n" +
    // biome-ignore lint/suspicious/noTemplateCurlyInString: a module's source, holding a template literal
    'window.t = () => `${search()} of ${s()}`;\nwindow.o = { search, s };',
  's.js': 'function search() { return 1; }\nexport function s() { return search(); }',
};
const suffixMoved = (searchBody, entryUses = 'search') => ({
  'main.js':
    `import { search } from './n.js';\nimport { s } from './s.js';\nwindow.go = () => s() + ${entryUses}();\n` +
    `window.t = () => \`\${${entryUses}()} of \${s()}\`;\nwindow.o = { search, s };`,
  's.js': suffixBase['s.js'],
  'n.js': `export function search() { ${searchBody} }`,
});
test('the same code: a move that swaps collision suffixes (calls, a template, a shorthand property)', async () => {
  const result = compare(await bundleOf(suffixBase), await bundleOf(suffixMoved('return 2;')));
  assert.deepEqual([result.gone, result.added, result.sameOrder], [[], [], true]);
  assert.deepEqual(result.renamed.sort(), [
    ['search', 'search2'],
    ['search2', 'search'],
  ]);
});
test('reported: a move that swaps collision suffixes and changes the moved function', async () => {
  assert.equal(await sameCode(suffixBase, suffixMoved('return 3;')), false);
});
test("reported: a call moved to another module's function of the same name", async () => {
  const before = {
    'main.js': "import { a, b } from './a.js';\nimport { c } from './c.js';\nwindow.go = () => [a(), b(), c()];",
    'a.js': 'function fmt(x) { return x + 1; }\nexport const a = () => fmt(1);\nexport const b = () => fmt(5);',
    'c.js': 'export function fmt(x) { return x * 2; }\nexport const c = () => fmt(2);',
  };
  const after = {
    ...before,
    'a.js':
      "import { fmt as other } from './c.js';\nfunction fmt(x) { return x + 1; }\n" +
      'export const a = () => other(1);\nexport const b = () => fmt(5);',
  };
  assert.equal(await sameCode(before, after), false);
});
