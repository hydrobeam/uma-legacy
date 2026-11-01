// samecode: is the page's code in one tree the same code as in another, moved or renamed but not changed?
//   node test/samecode.mjs [BASE [HEAD]] [--effects]   (./build.sh samecode; BASE defaults to HEAD, HEAD to the working tree)
// Each tree's three bundles (the two pages' entries and the solver's worker) are built by esbuild unminified, without
// minifying names, the path comments dropped, and compared twice:
// - the multiset of code lines: the same lines, possibly in another order. A move, a rename to .ts with annotations or a
//   split keeps it; any changed line breaks it. (A minified hash can't serve: esbuild's minified names follow the
//   input's character counts, so a type annotation can rename them, and a split moves code earlier in the bundle.)
// - the ordered list of load-time effects: the top-level statements that do something when the bundle loads (not a
//   declaration of a function, class or plain value). Modules run in import order, so a move that changes the order of
//   storage reads or document listeners breaks it.
// A module order change also swaps esbuild's collision suffixes (GAP, GAP2), which shows as changed lines: evaluation
// order did change, so it is reported. --effects lists each bundle's effects. Exits 1 on any difference.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import * as esbuild from 'esbuild';

const W = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
const ENTRIES = ['src/main.js', 'src/career.js', 'worker.template.js'];
// what a bundle's build reads beside the sources: the tsconfig (JSX, import elision) and the package's module type
const TREE = ['webapp/src', 'webapp/worker.template.js', 'webapp/tsconfig.json', 'webapp/package.json'];

// The bundle of `entry` in the webapp folder `dir`, as build.py makes it but unminified, as lines; the packages resolve
// from this checkout's node_modules
const bundle = (dir, entry) =>
  esbuild.buildSync({
    entryPoints: [path.join(dir, entry)],
    absWorkingDir: dir,
    nodePaths: [path.join(W, 'node_modules')],
    bundle: true,
    format: 'iife',
    charset: 'utf8',
    jsx: 'automatic',
    jsxImportSource: 'preact',
    legalComments: 'none',
    logLevel: 'warning',
    write: false,
  }).outputFiles[0].text;

// The code lines of a bundle: blank lines and esbuild's `// path/file.js` comments dropped
export const codeLines = (text) =>
  text.split('\n').filter((line) => line.trim() && !/^\s*\/\/ \S+\.(m?[jt]sx?|json)$/.test(line));

// Per line of `text`, whether it starts outside any string, template or comment and at bracket depth `depth`: a scan of
// the JS lexical grammar sufficient for esbuild's output (a `/` is a regular expression after an operator, an opening
// bracket or a keyword, else a division)
export function lineStarts(text, depth) {
  const starts = [true];
  const stack = []; // open brackets, and '`' for a template literal whose `${` is open
  let i = 0;
  let prev = ''; // the last significant character or word outside strings and comments
  const regexBefore =
    /^$|[(,=:[!&|?{};+\-*%<>~^]$|^(return|typeof|case|do|else|in|of|new|delete|void|throw|yield|await)$/;
  const skipString = (quote) => {
    for (i++; i < text.length && text[i] !== quote; i++) if (text[i] === '\\') i++;
  };
  const skipRegex = () => {
    // from its opening '/' to its closing one, a '/' in a character class or escaped not closing it
    let inClass = false;
    for (i++; i < text.length && (inClass || text[i] !== '/'); i++) {
      if (text[i] === '\\') i++;
      else if (text[i] === '[') inClass = true;
      else if (text[i] === ']') inClass = false;
    }
  };
  const skipTemplate = () => {
    // from after a '`' or a template's closing '}': to its end, or into a `${` (pushed)
    for (; i < text.length; i++) {
      if (text[i] === '\\') i++;
      else if (text[i] === '\n') starts.push(false);
      else if (text[i] === '`') return;
      else if (text[i] === '$' && text[i + 1] === '{') {
        stack.push('`');
        i++;
        return;
      }
    }
  };
  for (; i < text.length; i++) {
    const ch = text[i];
    if (ch === '\n') {
      starts.push(stack.length === depth);
      continue;
    }
    if (ch === ' ' || ch === '\t' || ch === '\r') continue;
    if (ch === '/' && text[i + 1] === '/') {
      while (i + 1 < text.length && text[i + 1] !== '\n') i++;
      continue;
    }
    if (ch === '/' && text[i + 1] === '*') {
      for (i += 2; i < text.length && !(text[i] === '*' && text[i + 1] === '/'); i++)
        if (text[i] === '\n') starts.push(false);
      i++;
      continue;
    }
    if (ch === '"' || ch === "'") skipString(ch);
    else if (ch === '`') {
      i++;
      skipTemplate();
    } else if (ch === '/' && regexBefore.test(prev)) {
      skipRegex();
    } else if ('([{'.includes(ch)) stack.push(ch);
    else if (')]}'.includes(ch)) {
      if (stack.pop() === '`') {
        i++;
        skipTemplate();
      }
    } else if (/[\w$]/.test(ch)) {
      let j = i;
      while (j + 1 < text.length && /[\w$]/.test(text[j + 1])) j++;
      prev = text.slice(i, j + 1);
      i = j;
      continue;
    }
    prev = ch;
  }
  return starts;
}

// a top-level statement that declares and does nothing else: a function or class, or a value made without a call
const DECLARATION = /^(async function|function\*?|class) /;
const PURE_VALUE =
  /^(var|let|const) [\w$]+ = (\(|async \(|[\w$]+ =>|\[|\{|function|class|[0-9"'`-]|null|true|false|void 0|\/|new (Map|Set|WeakMap)\(\))/;
// The load-time effects of a bundle: the first line of each top-level statement of its IIFE (two spaces in, outside any
// string or bracket but the IIFE's own) that is not a declaration (a `var x = f()` is: the call runs at load)
export function effects(text) {
  const lines = text.split('\n');
  const starts = lineStarts(text, 2);
  return lines
    .map((line, k) => (starts[k] && /^ {2}\S/.test(line) ? line.slice(2) : null))
    .filter((s) => s !== null && !/^([)\]}]|\/\/|\/\*)/.test(s))
    .filter((s) => !DECLARATION.test(s) && !(/^(var|let|const) /.test(s) && (PURE_VALUE.test(s) || !s.includes('('))));
}

// The difference of two bundles' code: [lines only in a, lines only in b] (each as often as it is in excess)
export function lineDiff(a, b) {
  const count = new Map();
  for (const l of a) count.set(l, (count.get(l) || 0) + 1);
  for (const l of b) count.set(l, (count.get(l) || 0) - 1);
  const only = (sign) => [...count].flatMap(([l, n]) => (Math.sign(n) === sign ? Array(Math.abs(n)).fill(l) : []));
  return [only(1), only(-1)];
}

// A webapp folder holding `rev`'s page sources (null: this working tree)
function tree(rev) {
  if (!rev) return W;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'samecode-'));
  const archive = spawnSync('git', ['-C', path.join(W, '..'), 'archive', '--format=tar', rev, ...TREE], {
    maxBuffer: 1 << 28,
  });
  if (archive.status !== 0) throw new Error(`git archive ${rev}: ${archive.stderr}`);
  const untar = spawnSync('tar', ['-x', '-C', dir], { input: archive.stdout });
  if (untar.status !== 0) throw new Error(`tar: ${untar.stderr}`);
  return path.join(dir, 'webapp');
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const listEffects = args.includes('--effects');
  const [baseRev = 'HEAD', headRev = null] = args.filter((a) => a !== '--effects');
  const base = tree(baseRev);
  const head = tree(headRev);
  let same = true;
  for (const entry of ENTRIES) {
    const [a, b] = [bundle(base, entry), bundle(head, entry)];
    const [onlyA, onlyB] = lineDiff(codeLines(a), codeLines(b));
    const [ea, eb] = [effects(a), effects(b)];
    const sameOrder = ea.length === eb.length && ea.every((e, k) => e === eb[k]);
    console.log(
      `${entry}: ${codeLines(b).length} code lines, ${onlyA.length + onlyB.length ? `${onlyA.length} gone and ${onlyB.length} new` : 'the same'}; ` +
        `${eb.length} load-time effects, ${sameOrder ? 'in the same order' : `ORDER CHANGED (${ea.length} before)`}`,
    );
    for (const l of onlyA.slice(0, 8)) console.log(`  - ${l.slice(0, 150)}`);
    for (const l of onlyB.slice(0, 8)) console.log(`  + ${l.slice(0, 150)}`);
    if (onlyA.length + onlyB.length > 16) console.log(`  (${onlyA.length + onlyB.length} differing lines in all)`);
    if (!sameOrder) {
      const k = ea.findIndex((e, j) => e !== eb[j]);
      const at = k < 0 ? ea.length : k;
      console.log(
        `  first divergence, effect ${at + 1}:\n    before: ${ea[at] ?? '(none)'}\n    after:  ${eb[at] ?? '(none)'}`,
      );
    }
    if (listEffects) for (const e of eb) console.log(`    ${e.slice(0, 150)}`);
    same &&= sameOrder && onlyA.length + onlyB.length === 0;
  }
  for (const dir of [base, head]) if (dir !== W) fs.rmSync(path.dirname(dir), { recursive: true });
  console.log(same ? 'samecode: the same code' : 'samecode: the code differs (above)');
  process.exit(same ? 0 : 1);
}
