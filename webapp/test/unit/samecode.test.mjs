// samecode's parts (test/samecode.mjs): its readers on small bundles shaped as esbuild writes them, and whole
// comparisons of snippets bundled as samecode bundles them: node --test test/unit/
import assert from 'node:assert/strict';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { after, test } from 'node:test';
import { bundle, compare, effects, listDiff, units } from '../samecode.mjs';

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
