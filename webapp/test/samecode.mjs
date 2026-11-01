// samecode: is the page's code in one tree the same code as in another, moved or renamed but not changed?
//   node test/samecode.mjs [BASE [HEAD]] [--effects]   (BASE defaults to HEAD, HEAD to the working tree)
// Each tree's three bundles (the two pages' entries and the solver's worker) are built by esbuild without minifying,
// read as tokens (comments dropped, so esbuild's `// path/file.js` lines go; whitespace dropped, so a line the formatter
// reflowed reads the same) and compared twice:
// - the multiset of statements: the token runs between one `;`, `{` or `}` and the next, possibly in another order. A
//   move, a rename to .ts with annotations or a split keeps it; any changed statement breaks it. (A minified hash can't
//   serve: esbuild's minified names follow the input's character counts, so a type annotation can rename them, and a
//   split moves code earlier in the bundle.)
// - the ordered list of load-time effects: the top-level statements that do something when the bundle loads (not a
//   declaration of a function, class or plain value). Modules run in import order, so a move that changes the order of
//   storage reads or document listeners breaks it.
// A module order change also swaps esbuild's collision suffixes (GAP, GAP2), which shows as changed statements:
// evaluation order did change, so it is reported. --effects lists each bundle's effects. Exits 1 on any difference.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import * as esbuild from 'esbuild';

const W = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
const ENTRIES = ['src/main.js', 'src/career.js', 'worker.template.js'];
// what a bundle's build reads beside the sources: the tsconfig (JSX, import elision) and the package's module type
const TREE = ['webapp/src', 'webapp/worker.template.js', 'webapp/tsconfig.json', 'webapp/package.json'];

// The bundle of `entry` in the webapp folder `dir`, as build.py makes it but unminified; the packages resolve from this
// checkout's node_modules
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

const WORD = /[\w$]/;
// a `/` starts a regular expression after these (an operator, an opening bracket, a keyword), else it divides
const REGEX_AFTER =
  /^$|^[(,=:[!&|?{};+\-*%<>~^]$|^(return|typeof|case|do|else|in|of|new|delete|void|throw|yield|await)$/;

// The tokens of esbuild's output: {text, depth (open brackets before it), lineStart (the first token of its line),
// indent (its line's leading spaces)}. Strings, regular expressions and template literals (with what their `${}` hold)
// are one token each; comments and whitespace are dropped.
export function tokenize(text) {
  const tokens = [];
  let depth = 0;
  const casts = []; // per open parenthesis: whether it opens a JSDoc cast (dropped with its closing one)
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
  const push = (end) => {
    tokens.push({ text: text.slice(i, end), depth, lineStart, indent });
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
    } else if (ch === '"' || ch === "'" || ch === '`') push(quoteEnd(i));
    else if (ch === '/' && REGEX_AFTER.test(tokens.at(-1)?.text ?? '')) {
      let end = quoteEnd(i);
      while (end < text.length && /[a-z]/.test(text[end])) end++; // its flags
      push(end);
    } else if (WORD.test(ch)) {
      let end = i + 1;
      while (end < text.length && WORD.test(text[end])) end++;
      push(end);
    } else if (ch === '(' && /^\s*\/\*/.test(text.slice(i + 1, i + 200))) {
      // a JSDoc cast, `/** @type {T} */ (x)`: esbuild keeps the comment, so the parentheses around it; a type
      // annotation or a TS `as` leaves none
      casts.push(true);
      i++;
    } else if ('([{'.includes(ch)) {
      if (ch === '(') casts.push(false);
      push(i + 1);
      depth++;
    } else if (ch === ')' && casts.pop()) i++;
    else if (')]}'.includes(ch)) {
      depth--;
      push(i + 1);
    } else push(i + 1);
  }
  return tokens;
}

// tokens as text: a space only where two words would otherwise merge, or two signs (`a - -b`)
const join = (tokens) =>
  tokens
    .map((t, k) => {
      const before = tokens[k - 1]?.text.at(-1) ?? '';
      const glue = (WORD.test(before) && WORD.test(t.text[0])) || ('+-'.includes(before) && before === t.text[0]);
      return (k && glue ? ' ' : '') + t.text;
    })
    .join('');

// The statements of a bundle: its tokens cut after each `;`, `{` and `}`, as text
export function statements(text) {
  const out = [];
  let run = [];
  for (const token of tokenize(text)) {
    run.push(token);
    if (';{}'.includes(token.text)) {
      out.push(join(run));
      run = [];
    }
  }
  if (run.length) out.push(join(run));
  return out;
}

// a top-level statement that declares and does nothing else: a function or class, or a value made without a call
const DECLARATION = /^(async function|function)\b|^class\b/;
const PURE_VALUE =
  /^(var|let|const) [\w$]+=(\(|async\(|async [\w$]+=>|[\w$]+=>|\[|\{|function\b|class\b|[0-9"'`-]|null;|true;|false;|void 0;|\/|new (Map|Set|WeakMap)\(\);)/;
// The load-time effects of a bundle: the top-level statements of its IIFE (its own first tokens on a line two spaces
// in, inside the IIFE's two brackets) that are not declarations (a `var x = f()` is not: the call runs at load), each
// as its text up to its first `{` or `;`
export function effects(text) {
  const tokens = tokenize(text);
  const heads = [];
  for (let k = 0; k < tokens.length; k++) {
    const t = tokens[k];
    if (!(t.lineStart && t.depth === 2 && t.indent === 2) || ')]};'.includes(t.text)) continue;
    let end = k;
    while (end < tokens.length && !'{;'.includes(tokens[end].text)) end++;
    const head = join(tokens.slice(k, end + 1));
    const isDeclaration =
      DECLARATION.test(head) || (/^(var|let|const) /.test(head) && (PURE_VALUE.test(head) || !head.includes('(')));
    if (!isDeclaration) heads.push(head);
  }
  return heads;
}

// The difference of two lists: [entries only in a, entries only in b] (each as often as it is in excess)
export function listDiff(a, b) {
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
    const [sa, sb] = [statements(a), statements(b)];
    const [onlyA, onlyB] = listDiff(sa, sb);
    const [ea, eb] = [effects(a), effects(b)];
    const sameOrder = ea.length === eb.length && ea.every((e, k) => e === eb[k]);
    const changed = onlyA.length + onlyB.length;
    console.log(
      `${entry}: ${sb.length} statements, ${changed ? `${onlyA.length} gone and ${onlyB.length} new` : 'the same'}; ` +
        `${eb.length} load-time effects, ${sameOrder ? 'in the same order' : `ORDER CHANGED (${ea.length} before)`}`,
    );
    for (const s of onlyA.slice(0, 8)) console.log(`  - ${s.slice(0, 150)}`);
    for (const s of onlyB.slice(0, 8)) console.log(`  + ${s.slice(0, 150)}`);
    if (changed > 16) console.log(`  (${changed} differing statements in all)`);
    if (!sameOrder) {
      const k = ea.findIndex((e, j) => e !== eb[j]);
      const at = k < 0 ? ea.length : k;
      console.log(
        `  first divergence, effect ${at + 1}:\n    before: ${ea[at] ?? '(none)'}\n    after:  ${eb[at] ?? '(none)'}`,
      );
    }
    if (listEffects) for (const e of eb) console.log(`    ${e.slice(0, 150)}`);
    same &&= sameOrder && changed === 0;
  }
  for (const dir of [base, head]) if (dir !== W) fs.rmSync(path.dirname(dir), { recursive: true });
  console.log(same ? 'samecode: the same code' : 'samecode: the code differs (above)');
  process.exit(same ? 0 : 1);
}
