// samecode's parts (test/samecode.mjs) on small bundles shaped as esbuild writes them: node --test test/unit/
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { codeLines, effects, lineDiff } from '../samecode.mjs';

const iife = (...body) => ['(() => {', ...body, '})();', ''].join('\n');

test('declarations are not effects; calls, listeners and storage reads are', () => {
  const text = iife(
    '  // src/a.js',
    '  var ENTITY = { "&": "&amp;" };',
    '  var seen = new Set();',
    '  var add = (x) => {',
    '    seen.add(x);',
    '  };',
    '  function esc(t) {',
    '    return t;',
    '  }',
    '  var SAVED = loadSaved();',
    '  document.addEventListener("click", (e) => {',
    '    add(e);',
    '  });',
    '  var LIMIT = 3;',
  );
  assert.deepEqual(effects(text), ['var SAVED = loadSaved();', 'document.addEventListener("click", (e) => {']);
});

test('lines inside a template literal, a string or a comment are not statements', () => {
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
  assert.deepEqual(effects(text), ['go();']);
});

test('a reordered effect changes the list; a moved declaration does not', () => {
  const a = iife('  var x = 1;', '  read();', '  listen();', '  function f() {', '  }');
  const b = iife('  function f() {', '  }', '  var x = 1;', '  listen();', '  read();');
  assert.deepEqual(lineDiff(codeLines(a), codeLines(b)), [[], []]);
  assert.notDeepEqual(effects(a), effects(b));
});

test('path comments are dropped; a changed line is found once per excess', () => {
  const a = codeLines(iife('  // src/x.js', '  go();', '  go();'));
  const b = codeLines(iife('  // src/y/x.ts', '  go();', '  stop();'));
  assert.deepEqual(lineDiff(a, b), [['  go();'], ['  stop();']]);
});
