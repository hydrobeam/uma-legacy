// samecode's parts (test/samecode.mjs) on small bundles shaped as esbuild writes them: node --test test/unit/
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { effects, listDiff, statements } from '../samecode.mjs';

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
    '  var list = [',
    '    1',
    '  ];',
  );
  assert.deepEqual(effects(text), ['var SAVED=loadSaved();', 'document.addEventListener("click",(e)=>{']);
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
  assert.deepEqual(listDiff(statements(a), statements(b)), [[], []]);
  assert.notDeepEqual(effects(a), effects(b));
});

test('path comments and line breaks are dropped; a changed statement is found once per excess', () => {
  const a = statements(iife('  // src/x.js', '  go(1, 2);', '  go(1, 2);'));
  const b = statements(iife('  // src/y/x.ts', '  go(', '    1,', '    2', '  );', '  stop(a - -b);'));
  assert.deepEqual(listDiff(a, b), [['go(1,2);'], ['stop(a- -b);']]);
});

test("a JSDoc cast's parentheses read as none (esbuild keeps them with the comment)", () => {
  const cast = iife('  var $ = (id) => (', '    /** @type {HTMLElement} */', '    byId(id)', '  );', '  go();');
  const plain = iife('  var $ = (id) => byId(id);', '  go();');
  assert.deepEqual(listDiff(statements(cast), statements(plain)), [[], []]);
  assert.deepEqual(effects(cast), ['go();']);
});
