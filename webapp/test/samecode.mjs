// samecode: is the page's code in one tree the same code as in another, moved or renamed but not changed?
//   node test/samecode.mjs [BASE [HEAD]] [--effects]   (BASE defaults to HEAD, HEAD to the working tree)
// Each tree's three bundles (the two pages' entries and the solver's worker) are built by esbuild without minifying,
// read as tokens (comments dropped, so esbuild's `// path/file.js` lines go; whitespace dropped, so a line the formatter
// reflowed reads the same) and cut into top-level units: the statements of the bundle's IIFE, each whole with its body.
// Two trees hold the same code when:
// - each bundle has the same multiset of units, possibly in another order. A move between modules or a rename to .ts
//   with annotations keeps every unit's text; a change inside a function (statements reordered, a branch swapped, a
//   statement moved into or out of a block or to another function) changes the unit that holds it.
// - its load-time effects (the units that do something when the bundle loads: anything but a function declaration or
//   a declaration whose value is made without a call) run in the same order. Modules run in import order, so a move
//   that changes the order of storage reads or document listeners breaks it.
// A module order change also swaps esbuild's collision suffixes (GAP, GAP2), which shows as changed units: evaluation
// order did change, so it is reported. Not compared: a declaration's place among the effects (a value declared after
// an effect that reads it at load reads undefined), the templates, the CSS and build.py's flags.
// --effects lists each bundle's effects. Exits 1 on any difference.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import * as esbuild from 'esbuild';

const W = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
// each bundle's entry by the names it has had, newest first: a tree's entry is the first name it holds, so a move or a
// rename to .ts compares the bundle across it
const ENTRIES = [['src/main.js'], ['src/career.js'], ['src/worker.ts', 'worker.template.js']];
// what a bundle's build reads beside the sources: the tsconfig (JSX, import elision) and the package's module type; a
// path a revision lacks (an entry's older name) is left out of its tree
const TREE = ['webapp/src', 'webapp/worker.template.js', 'webapp/tsconfig.json', 'webapp/package.json'];

// A JSDoc type or satisfies comment. esbuild keeps such a comment in its output, and with it the parentheses of a cast
// `/** @type {T} */ (x)`; with the comment gone it prints only the parentheses the expression needs, as for `x as T`
const CAST_COMMENT = /\/\*\*\s*@(?:type|satisfies)\b(?:[^*]|\*(?!\/))*\*\//g;
export const dropCastComments = (source) => source.replace(CAST_COMMENT, '');

// the page's sources read without their JSDoc type comments (the packages as they are)
const castsDropped = {
  name: 'casts-dropped',
  setup(build) {
    build.onLoad({ filter: /\.[jt]sx?$/ }, (args) => {
      if (args.path.includes(`${path.sep}node_modules${path.sep}`)) return undefined;
      const source = fs.readFileSync(args.path, 'utf8');
      return { contents: dropCastComments(source), loader: path.extname(args.path).slice(1) };
    });
  },
};

// The bundle of `entry` in the webapp folder `dir`, as build.py makes it but unminified and without JSDoc type
// comments; the packages resolve from this checkout's node_modules
export const bundle = async (dir, entry) =>
  (
    await esbuild.build({
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
      plugins: [castsDropped],
      write: false,
    })
  ).outputFiles[0].text;

const WORD = /[\w$]/;
// the punctuators longer than one character, longest first, so `a++ + b` and `a + ++b` read apart
const PUNCTUATORS = [
  ...['>>>=', '...', '===', '!==', '**=', '<<=', '>>=', '>>>', '&&=', '||=', '??='],
  ...['=>', '==', '!=', '<=', '>=', '&&', '||', '??', '?.', '++', '--', '**', '<<', '>>'],
  ...['+=', '-=', '*=', '/=', '%=', '&=', '|=', '^='],
];
const REGEX_AFTER_WORD = /^(return|typeof|case|do|else|in|of|new|delete|void|throw|yield|await|instanceof)$/;
// whether a `/` after `prev` starts a regular expression (after an operator, an opening bracket or a keyword) rather
// than dividing (after a value)
const regexAfter = (prev) =>
  !prev ||
  (prev.kind === 'punct' && !/^([)\]]|\+\+|--)$/.test(prev.text)) ||
  (prev.kind === 'word' && REGEX_AFTER_WORD.test(prev.text));

// The tokens of esbuild's output: {text, kind (word, string, regex or punct), depth (open brackets before it; a closing
// bracket has its opener's), lineStart (the first token of its line), indent (its line's leading spaces)}. Strings,
// regular expressions and template literals (with what their `${}` hold) are one token each; comments and whitespace
// are dropped.
export function tokenize(text) {
  const tokens = [];
  let depth = 0;
  let i = 0;
  let lineStart = true;
  let indent = 0;
  // the end (exclusive) of the string, template or regular expression opening at `at`
  const quoteEnd = (at) => {
    const quote = text[at];
    let j = at + 1;
    let inClass = false; // in a regular expression's [...]
    for (; j < text.length; j++) {
      const ch = text[j];
      if (ch === '\\') j++;
      else if (quote === '/' && ch === '[') inClass = true;
      else if (quote === '/' && ch === ']') inClass = false;
      else if (ch === quote && !inClass) return j + 1;
      else if (quote === '`' && ch === '$' && text[j + 1] === '{') j = codeEnd(j + 2) - 1;
    }
    return j;
  };
  // the end (exclusive) of a template's `${ ... }`, from just inside it
  const codeEnd = (at) => {
    let open = 1;
    let j = at;
    for (; j < text.length && open > 0; j++) {
      const ch = text[j];
      if (ch === '"' || ch === "'" || ch === '`') j = quoteEnd(j) - 1;
      else if (ch === '{') open++;
      else if (ch === '}') open--;
    }
    return j;
  };
  const push = (end, kind) => {
    tokens.push({ text: text.slice(i, end), kind, depth, lineStart, indent });
    lineStart = false;
    i = end;
  };
  while (i < text.length) {
    const ch = text[i];
    if (ch === '\n') {
      lineStart = true;
      indent = text.slice(i + 1).match(/^ */)[0].length;
      i++;
    } else if (ch === ' ' || ch === '\t' || ch === '\r') i++;
    else if (ch === '/' && text[i + 1] === '/') while (i < text.length && text[i] !== '\n') i++;
    else if (ch === '/' && text[i + 1] === '*') {
      const close = text.indexOf('*/', i + 2);
      i = close < 0 ? text.length : close + 2;
    } else if (ch === '"' || ch === "'" || ch === '`') push(quoteEnd(i), 'string');
    else if (ch === '/' && regexAfter(tokens.at(-1))) {
      let end = quoteEnd(i);
      while (end < text.length && /[a-z]/.test(text[end])) end++; // its flags
      push(end, 'regex');
    } else if (WORD.test(ch)) {
      let end = i + 1;
      while (end < text.length && WORD.test(text[end])) end++;
      push(end, 'word');
    } else if ('([{'.includes(ch)) {
      push(i + 1, 'punct');
      depth++;
    } else if (')]}'.includes(ch)) {
      depth--;
      push(i + 1, 'punct');
    } else {
      const long = PUNCTUATORS.find((p) => text.startsWith(p, i));
      push(i + (long ? long.length : 1), 'punct');
    }
  }
  return tokens;
}

// tokens as text: a space only where two words would otherwise merge, or two signs (`a - -b`, `a++ +b`)
const join = (tokens) =>
  tokens
    .map((t, k) => {
      const before = tokens[k - 1]?.text.at(-1) ?? '';
      const glue = (WORD.test(before) && WORD.test(t.text[0])) || ('+-'.includes(before) && before === t.text[0]);
      return (k && glue ? ' ' : '') + t.text;
    })
    .join('');

// The top-level units of a bundle, as token lists: the statements of its IIFE, each whole with its body (a unit starts
// at its own first token on a line two spaces in, inside the IIFE's two brackets, as esbuild prints them), then the
// frame (the tokens outside the IIFE's body: "use strict", the IIFE's own brackets)
export function units(text) {
  const body = [];
  const frame = [];
  for (const t of tokenize(text)) {
    if (t.depth < 2) frame.push(t);
    else if (t.lineStart && t.depth === 2 && t.indent === 2 && !')]};'.includes(t.text)) body.push([t]);
    else if (body.length) body.at(-1).push(t);
    else frame.push(t);
  }
  return [...body, frame];
}

// the index just past the bracket that closes the one opening at tokens[at]
const closeOf = (tokens, at) => {
  let k = at + 1;
  while (k < tokens.length && tokens[k].depth > tokens[at].depth) k++;
  return k + 1;
};
// the index just past an arrow function's body, from its `=>`: a block, or an expression up to a `,` or `;` at the
// arrow's depth or a closing bracket below it
const arrowBodyEnd = (tokens, arrow) => {
  const depth = tokens[arrow].depth;
  if (tokens[arrow + 1]?.text === '{') return closeOf(tokens, arrow + 1);
  let k = arrow + 1;
  while (k < tokens.length && tokens[k].depth >= depth && !(tokens[k].depth === depth && ',;'.includes(tokens[k].text)))
    k++;
  return k;
};
// words before a `(` that make it a parenthesised expression, not a call
const NOT_A_CALLEE = /^(return|typeof|void|delete|in|of|instanceof|case|await|yield|async|new|else|do)$/;
const PLAIN_COLLECTION = /^(Map|Set|WeakMap|WeakSet)$/;

// Whether a declaration's tokens (after `var`, `let` or `const`; or a class) run anything at load: a call, a `new`
// other than an empty collection, a tagged template, a template with a call inside, `++`, `--`, `delete` or a compound
// assignment. The bodies of the functions, arrows and methods it defines are skipped: they run when called.
export function runsAtLoad(tokens) {
  for (let k = 0; k < tokens.length; k++) {
    const t = tokens[k];
    const prev = tokens[k - 1];
    if (t.text === '=>') {
      k = arrowBodyEnd(tokens, k) - 1;
    } else if (t.kind === 'word' && t.text === 'function') {
      let open = k + 1;
      while (open < tokens.length && !(tokens[open].text === '{' && tokens[open].depth === t.depth)) open++;
      k = closeOf(tokens, open) - 1;
    } else if (t.kind === 'word' && t.text === 'new') {
      const [name, open, close] = [tokens[k + 1], tokens[k + 2], tokens[k + 3]];
      if (!(PLAIN_COLLECTION.test(name?.text ?? '') && open?.text === '(' && close?.text === ')')) return true;
      k += 3;
    } else if (t.text === '(') {
      const after = closeOf(tokens, k);
      const next = tokens[after]?.text;
      if (next === '=>')
        k = after - 1; // an arrow's parameters (their defaults run when it is called)
      else if (next === '{')
        k = closeOf(tokens, after) - 1; // a method's parameters and body
      else if (prev && (')]'.includes(prev.text) || (prev.kind === 'word' && !NOT_A_CALLEE.test(prev.text))))
        return true;
    } else if (t.kind === 'string' && t.text[0] === '`') {
      if (prev && (prev.kind === 'word' || ')]'.includes(prev.text))) return true; // a tagged template
      if (t.text.includes('${') && t.text.includes('(')) return true;
    } else if (/^(\+\+|--|delete|[-+*/%&|^]=|\*\*=|<<=|>>>?=|&&=|\|\|=|\?\?=)$/.test(t.text)) return true;
  }
  return false;
}

// whether a top-level unit only declares: a function, or a variable or class whose value is made without running
// anything at load
const isDeclaration = (unit) => {
  const [first, second] = unit;
  if (first.text === 'function' || (first.text === 'async' && second?.text === 'function')) return true;
  if (/^(var|let|const)$/.test(first.text)) return !runsAtLoad(unit.slice(1));
  if (first.text === 'class') return !runsAtLoad(unit);
  return false;
};

// The load-time effects of a bundle, in order, as text: its top-level units that are not declarations
export const effects = (text) =>
  units(text)
    .slice(0, -1)
    .filter((unit) => !isDeclaration(unit))
    .map(join);

// The difference of two lists: [entries only in a, entries only in b] (each as often as it is in excess)
export function listDiff(a, b) {
  const count = new Map();
  for (const l of a) count.set(l, (count.get(l) || 0) + 1);
  for (const l of b) count.set(l, (count.get(l) || 0) - 1);
  const only = (sign) => [...count].flatMap(([l, n]) => (Math.sign(n) === sign ? Array(Math.abs(n)).fill(l) : []));
  return [only(1), only(-1)];
}

// Two bundles compared: {unitCount, gone, added (units' text only in one), effectsBefore, effectsAfter, sameOrder,
// same}
export function compare(before, after) {
  const [ua, ub] = [units(before).map(join), units(after).map(join)];
  const [gone, added] = listDiff(ua, ub);
  const [effectsBefore, effectsAfter] = [effects(before), effects(after)];
  const sameOrder =
    effectsBefore.length === effectsAfter.length && effectsBefore.every((e, k) => e === effectsAfter[k]);
  return {
    unitCount: ub.length,
    gone,
    added,
    effectsBefore,
    effectsAfter,
    sameOrder,
    same: sameOrder && gone.length + added.length === 0,
  };
}

// where two texts first differ, as a short excerpt of each
const excerpts = (a, b) => {
  let k = 0;
  while (k < a.length && a[k] === b[k]) k++;
  const from = Math.max(0, k - 40);
  return [a.slice(from, k + 60), b.slice(from, k + 60)];
};

// A webapp folder holding `rev`'s page sources (null: this working tree)
function tree(rev) {
  if (!rev) return W;
  const dir = fs.mkdtempSync(path.join(os.tmpdir(), 'samecode-'));
  const root = path.join(W, '..');
  const held = TREE.filter((p) => spawnSync('git', ['-C', root, 'cat-file', '-e', `${rev}:${p}`]).status === 0);
  const archive = spawnSync('git', ['-C', root, 'archive', '--format=tar', rev, ...held], {
    maxBuffer: 1 << 28,
  });
  if (archive.status !== 0) throw new Error(`git archive ${rev}: ${archive.stderr}`);
  const untar = spawnSync('tar', ['-x', '-C', dir], { input: archive.stdout });
  if (untar.status !== 0) throw new Error(`tar: ${untar.stderr}`);
  return path.join(dir, 'webapp');
}

// one bundle's comparison, printed: the changed units (a pair with the same first line shown where it first differs),
// then the first divergence of the effects
function report(entry, result, listEffects) {
  const { unitCount, gone, added, effectsBefore, effectsAfter, sameOrder } = result;
  const changed = gone.length + added.length;
  console.log(
    `${entry}: ${unitCount} top-level units, ${changed ? `${gone.length} gone and ${added.length} new` : 'the same'}; ` +
      `${effectsAfter.length} load-time effects, ${sameOrder ? 'in the same order' : `ORDER CHANGED (${effectsBefore.length} before)`}`,
  );
  const head = (unit) => unit.split(/[{;]/)[0];
  const unpaired = [...added];
  for (const g of gone.slice(0, 8)) {
    const k = unpaired.findIndex((a) => head(a) === head(g));
    if (k < 0) {
      console.log(`  - ${g.slice(0, 150)}`);
      continue;
    }
    const [was, now] = excerpts(g, unpaired.splice(k, 1)[0]);
    console.log(`  ~ ${head(g).slice(0, 80)}\n      before: ...${was}\n      after:  ...${now}`);
  }
  for (const a of unpaired.slice(0, 8)) console.log(`  + ${a.slice(0, 150)}`);
  if (changed > 16) console.log(`  (${changed} differing units in all)`);
  if (!sameOrder) {
    const k = effectsBefore.findIndex((e, j) => e !== effectsAfter[j]);
    const at = k < 0 ? effectsBefore.length : k;
    const show = (e) => (e === undefined ? '(none)' : e.slice(0, 150));
    console.log(
      `  first divergence, effect ${at + 1}:\n    before: ${show(effectsBefore[at])}\n    after:  ${show(effectsAfter[at])}`,
    );
  }
  if (listEffects) for (const e of effectsAfter) console.log(`    ${e.slice(0, 150)}`);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const listEffects = args.includes('--effects');
  const [baseRev = 'HEAD', headRev = null] = args.filter((a) => a !== '--effects');
  const base = tree(baseRev);
  const head = tree(headRev);
  let same = true;
  // the entry's name in the tree `dir`
  const entryIn = (dir, names) => {
    const name = names.find((n) => fs.existsSync(path.join(dir, n)));
    if (!name) throw new Error(`no entry ${names.join(' or ')} in ${dir}`);
    return name;
  };
  for (const names of ENTRIES) {
    const [fromName, toName] = [entryIn(base, names), entryIn(head, names)];
    const entry = fromName === toName ? toName : `${fromName} -> ${toName}`;
    const result = compare(await bundle(base, fromName), await bundle(head, toName));
    report(entry, result, listEffects);
    same &&= result.same;
  }
  for (const dir of [base, head]) if (dir !== W) fs.rmSync(path.dirname(dir), { recursive: true });
  console.log(same ? 'samecode: the same code' : 'samecode: the code differs (above)');
  process.exit(same ? 0 : 1);
}
