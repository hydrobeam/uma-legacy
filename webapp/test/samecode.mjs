// samecode: is the page's code in one tree the same code as in another, moved or renamed but not changed?
//   node test/samecode.mjs [BASE [HEAD]] [--effects]   (BASE defaults to HEAD, HEAD to the working tree)
// Each tree's three bundles (the two pages' entries and the solver's worker) are built by esbuild without minifying,
// read as tokens (comments dropped, so esbuild's `// path/file.js` lines go; whitespace dropped, so a line the formatter
// reflowed reads the same) and cut into top-level units: the statements of the bundle's IIFE, each whole with its body.
// Two trees hold the same code when:
// - each bundle has the same multiset of units, possibly in another order. A move between modules or a rename to .ts
//   with annotations keeps every unit's text; a change inside a function (statements reordered, a branch swapped, a
//   statement moved into or out of a block or to another function) changes the unit that holds it.
// - its load-time steps run in the same order: the units that do something when the bundle loads (anything but a
//   function declaration or a declaration whose value is made without a call), and the declarations that read another
//   top-level variable at load (a snapshot: `var V = o.v`, `{...state}`). Modules run in import order, so a move that
//   changes the order of storage reads or document listeners, or moves a snapshot past an effect, breaks it.
// And, in the HEAD tree alone, no bundle reads a top-level variable at load before its declaration has run
// (`loadOrder`). esbuild prints top-level `const`, `let` and `class` as `var`, so such a read finds undefined (NaN, a
// "not a function" TypeError later), never a TDZ error; and with an import cycle a move between modules can put a
// declaration after its reader. Read at load: in a top-level statement outside the functions it defines, or in a
// function such a statement calls by name, as far as is visible without running it.
// A module order change also reshuffles esbuild's collision suffixes (GAP, GAP2), and a new library's short names push
// the page's own to new ones: before comparing, the after-bundle's names are renamed one to one within each suffix
// family to the pairing that leaves the fewest units differing (`pairSuffixes`; printed as a note). Every word naming
// a binding is renamed (properties are not; a shorthand property keeps its key), so the renamed bundle runs as the
// after-bundle does. A nested local esbuild suffixed in some functions and not others can't be paired so and still
// shows. Not compared: what a function called at load reads through a method, a
// constructor or a callback; a getter's or a function's own reads against the effects (only declarations are
// ordered); the templates, the CSS and build.py's flags.
// --effects lists each bundle's load-time steps. Exits 1 on any difference or misordered read.
import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import * as esbuild from 'esbuild';

const W = path.join(path.dirname(new URL(import.meta.url).pathname), '..');
// each bundle's entry by the names it has had, newest first: a tree's entry is the first name it holds, so a move or a
// rename to .ts compares the bundle across it
const ENTRIES = [
  ['src/loop/main.tsx', 'src/loop/main.js', 'src/main.js'],
  ['src/career.js'],
  ['src/worker.ts', 'worker.template.js'],
];
// what a bundle's build reads beside the sources: the tsconfig (JSX, import elision) and the package's module type; a
// path a revision lacks (an entry's older name) is left out of its tree
const TREE = ['webapp/src', 'webapp/worker.template.js', 'webapp/tsconfig.json', 'webapp/package.json'];

// A JSDoc type or satisfies comment. esbuild keeps such a comment in its output, and with it the parentheses of a cast
// `/** @type {T} */ (x)`; with the comment gone it prints only the parentheses the expression needs, as for `x as T`.
// It is matched in the raw source, strings included: a string holding `/** @type` would lose that text (none does).
const CAST_COMMENT = /\/\*\*\s*@(?:type|satisfies)\b(?:[^*]|\*(?!\/))*\*\//g;
export const dropCastComments = (source) => source.replace(CAST_COMMENT, '');

// the page's sources read without their JSDoc type comments (the packages as they are)
const castsDropped = {
  name: 'casts-dropped',
  setup(build) {
    build.onLoad({ filter: /\.[cm]?[jt]sx?$/ }, (args) => {
      if (args.path.includes(`${path.sep}node_modules${path.sep}`)) return undefined;
      const source = fs.readFileSync(args.path, 'utf8');
      // (esbuild's loader for .mjs and .cjs is js, for .mts and .cts ts)
      return { contents: dropCastComments(source), loader: path.extname(args.path).slice(1).replace(/^[cm]/, '') };
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

// the end (exclusive) of the string, template or regular expression opening at `text[at]`
function quoteEnd(text, at) {
  const quote = text[at];
  let j = at + 1;
  let inClass = false; // in a regular expression's [...]
  for (; j < text.length; j++) {
    const ch = text[j];
    if (ch === '\\') j++;
    else if (quote === '/' && ch === '[') inClass = true;
    else if (quote === '/' && ch === ']') inClass = false;
    else if (ch === quote && !inClass) return j + 1;
    else if (quote === '`' && ch === '$' && text[j + 1] === '{') j = codeEnd(text, j + 2) - 1;
  }
  return j;
}
// the end (exclusive, past its `}`) of a template's `${ ... }`, from just inside it
function codeEnd(text, at) {
  let open = 1;
  let j = at;
  for (; j < text.length && open > 0; j++) {
    const ch = text[j];
    if (ch === '"' || ch === "'" || ch === '`') j = quoteEnd(text, j) - 1;
    else if (ch === '{') open++;
    else if (ch === '}') open--;
  }
  return j;
}
// a template literal's text cut at its `${ }`s: [literal, code, literal, code, ..., literal]
function templatePieces(text) {
  const pieces = [];
  let from = 0;
  for (let j = 1; j < text.length - 1; j++) {
    if (text[j] === '\\') j++;
    else if (text[j] === '$' && text[j + 1] === '{') {
      const end = codeEnd(text, j + 2);
      pieces.push(text.slice(from, j + 2), text.slice(j + 2, end - 1));
      from = end - 1;
      j = end - 2;
    }
  }
  pieces.push(text.slice(from));
  return pieces;
}

// The tokens of esbuild's output: {text, kind (word, string, regex or punct), depth (open brackets before it; a closing
// bracket has its opener's), lineStart (the first token of its line), indent (its line's leading spaces), line (from
// 1), at (its place in the list)}. Strings, regular expressions and template literals (with what their `${}` hold) are
// one token each; comments and whitespace are dropped.
export function tokenize(text) {
  const tokens = [];
  let depth = 0;
  let i = 0;
  let lineStart = true;
  let indent = 0;
  let line = 1;
  const push = (end, kind) => {
    tokens.push({ text: text.slice(i, end), kind, depth, lineStart, indent, line, at: tokens.length });
    lineStart = false;
    for (let j = i; j < end; j++) if (text[j] === '\n') line++; // (a template's or a comment's lines)
    i = end;
  };
  while (i < text.length) {
    const ch = text[i];
    if (ch === '\n') {
      lineStart = true;
      indent = text.slice(i + 1).match(/^ */)[0].length;
      line++;
      i++;
    } else if (ch === ' ' || ch === '\t' || ch === '\r') i++;
    else if (ch === '/' && text[i + 1] === '/') while (i < text.length && text[i] !== '\n') i++;
    else if (ch === '/' && text[i + 1] === '*') {
      const close = text.indexOf('*/', i + 2);
      const end = close < 0 ? text.length : close + 2;
      for (let j = i; j < end; j++) if (text[j] === '\n') line++;
      i = end;
    } else if (ch === '"' || ch === "'" || ch === '`') push(quoteEnd(text, i), 'string');
    else if (ch === '/' && regexAfter(tokens.at(-1))) {
      let end = quoteEnd(text, i);
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

// ---------- what runs at load: the top-level bindings it reads

// words before a `(` that open a statement's head (`if (x) {` is a block; `f(x) {` is a method)
const CONTROL = /^(if|for|while|switch|catch|with)$/;
// whether the `{` at tokens[open] opens an object literal or a destructuring pattern, not a block
const objectBrace = (tokens, open) => !/^(\)|else|try|finally|do|=>|;|\{|\})$/.test(tokens[open - 1]?.text ?? ';');
// the bracket that holds tokens[k]: its index, or -1 at the outside
const holderOf = (tokens, k) => {
  let j = k - 1;
  while (j >= 0 && !(tokens[j].depth === tokens[k].depth - 1 && '([{'.includes(tokens[j].text))) j--;
  return j;
};
// whether the word at tokens[k] names a property rather than a binding: after `.` or `?.`, an object literal's key
// (`{ k: v }`) or a method's name (`k(x) { ... }`)
function isPropertyName(tokens, k) {
  const [prev, next] = [tokens[k - 1]?.text, tokens[k + 1]?.text];
  if (prev === '.' || prev === '?.') return true;
  if (next === ':' && (prev === '{' || prev === ',')) {
    const open = holderOf(tokens, k);
    return open >= 0 && tokens[open].text === '{' && objectBrace(tokens, open);
  }
  return next === '(' && prev !== 'function' && prev !== '*' && tokens[closeOf(tokens, k + 1)]?.text === '{';
}
// whether the function ending at tokens[end] (exclusive) is called where it stands: `(() => { ... })()`
const calledAt = (tokens, end) =>
  tokens[end]?.text === '(' || (tokens[end]?.text === ')' && tokens[end + 1]?.text === '(');

// Walks tokens[from, to) as they run when reached: the bodies of the functions, arrows and methods they define are
// skipped (they run when called), one called where it stands is walked, and so is the code in a template's `${}`.
// `visit(tokens, k, site)` gets each word that names a binding (not a property); `site`: the token whose place in the
// bundle the read has (the word itself, or the template, or the call that ran it, as the caller passes)
function walkLoad(tokens, from, to, visit, site = null) {
  for (let k = from; k < to; k++) {
    const t = tokens[k];
    if (t.text === '=>') {
      const end = arrowBodyEnd(tokens, k);
      if (!calledAt(tokens, end)) k = end - 1;
    } else if (t.kind === 'word' && t.text === 'function') {
      let open = k + 1;
      while (open < to && !(tokens[open].text === '{' && tokens[open].depth === t.depth)) open++;
      const end = closeOf(tokens, open);
      k = calledAt(tokens, end) ? open : end - 1; // (its parameters' defaults run when it is called)
    } else if (t.text === '(') {
      const after = closeOf(tokens, k);
      const prev = tokens[k - 1];
      if (tokens[after]?.text === '=>')
        k = after - 1; // an arrow's parameters
      else if (tokens[after]?.text === '{' && prev?.kind === 'word' && !CONTROL.test(prev.text))
        k = closeOf(tokens, after) - 1; // a method's parameters and body
    } else if (t.kind === 'string' && t.text[0] === '`') {
      const pieces = templatePieces(t.text);
      for (let j = 1; j < pieces.length; j += 2) {
        const inner = tokenize(pieces[j]);
        walkLoad(inner, 0, inner.length, visit, site ?? t);
      }
    } else if (t.kind === 'word' && !isPropertyName(tokens, k)) visit(tokens, k, site ?? t);
  }
}

// a destructuring pattern's or a declarator's names, from tokens[k] (its first token)
function declaredNames(tokens, k) {
  if (tokens[k]?.kind === 'word') return [tokens[k]];
  if (!'{['.includes(tokens[k]?.text)) return [];
  const names = [];
  for (let j = k + 1; j < closeOf(tokens, k) - 1; j++)
    if (tokens[j].kind === 'word' && /^[,}\]=]$/.test(tokens[j + 1]?.text) && tokens[j - 1]?.text !== '.')
      names.push(tokens[j]);
  return names;
}
// what calling the function made by tokens[k...] runs, as [tokens, from, to), or null when they make no function: a
// `function` expression, or an arrow with a block or an expression for its body
function functionBody(tokens, k) {
  let v = k;
  if (tokens[v]?.text === 'async') v++;
  if (tokens[v]?.text === 'function') {
    let open = v + 1;
    while (open < tokens.length && !(tokens[open].text === '{' && tokens[open].depth === tokens[v].depth)) open++;
    return [tokens, open + 1, closeOf(tokens, open) - 1];
  }
  let arrow = -1;
  if (tokens[v]?.kind === 'word' && tokens[v + 1]?.text === '=>') arrow = v + 1;
  else if (tokens[v]?.text === '(' && tokens[closeOf(tokens, v)]?.text === '=>') arrow = closeOf(tokens, v);
  if (arrow < 0) return null;
  if (tokens[arrow + 1]?.text === '{') return [tokens, arrow + 2, closeOf(tokens, arrow + 1) - 1];
  return [tokens, arrow + 1, arrowBodyEnd(tokens, arrow)];
}

// The top-level bindings of a bundle's units: {bindings: name -> {at, line, body}, names}. `at`: the token from which
// it holds its value (-1 for a function declaration, which is hoisted; for a variable or a class, the end of its
// declarator: esbuild prints top-level `const`, `let` and `class` as `var`, so a read before that finds undefined, not
// a TDZ error); `line`: where it is declared; `body`: what a call to it runs, for a function or a variable holding one.
// `names`: the tokens that name a binding where it is declared (not reads).
function bindingsOf(bodyUnits) {
  const bindings = new Map();
  const names = new Set();
  for (const unit of bodyUnits) {
    const [first, second] = unit;
    if (first.text === 'function' || (first.text === 'async' && second?.text === 'function')) {
      let k = first.text === 'async' ? 2 : 1;
      if (unit[k]?.text === '*') k++;
      const [, from, to] = functionBody(unit, first.text === 'async' ? 1 : 0);
      bindings.set(unit[k].text, { at: -1, line: unit[k].line, body: [unit, from, to] });
      names.add(unit[k]);
    } else if (first.text === 'class') {
      bindings.set(second.text, { at: unit.at(-1).at, line: second.line, body: null });
      names.add(second);
    } else if (/^(var|let|const)$/.test(first.text))
      for (let k = 1; k < unit.length; ) {
        let end = k; // the declarator's end: a `,` or `;` at the unit's depth, or the unit's end
        while (end < unit.length && !(unit[end].depth === first.depth && ',;'.includes(unit[end].text))) end++;
        const value = unit[k + 1]?.text === '=' ? functionBody(unit, k + 2) : null;
        for (const name of declaredNames(unit, k)) {
          bindings.set(name.text, { at: unit[Math.min(end, unit.length - 1)].at, line: name.line, body: value });
          names.add(name);
        }
        k = end + 1;
      }
  }
  return { bindings, names };
}

// the module of each line of a bundle (esbuild's `  // src/x.js` lines), indexed by line number
function modulesByLine(text) {
  let current = '(the bundle)';
  return [current, ...text.split('\n')].map((line) => {
    const m = line.match(/^ {2}\/\/ (\S+)$/);
    if (m) current = m[1];
    return current;
  });
}

// Each top-level variable a bundle reads at load before its declarator has run: `{name, readLine, readModule,
// declaredLine, declaredModule, via}`. Read at load: in a top-level statement outside the functions it defines, or in
// the body of a function such a statement calls by name (`via`: the calls, outermost first), as far as that is visible
// without running it (a function passed to another, a method or a constructor is not followed).
export function loadOrder(text) {
  const bodyUnits = units(text).slice(0, -1);
  const { bindings, names } = bindingsOf(bodyUnits);
  const moduleOf = modulesByLine(text);
  const problems = [];
  const followed = new Set(); // functions walked already, from an earlier site (a later one finds nothing new)
  const visitor = (via) => (tokens, k, site) => {
    const word = tokens[k];
    const binding = bindings.get(word.text);
    if (!binding || names.has(word)) return;
    if (binding.at > site.at)
      problems.push({
        name: word.text,
        readLine: site.line,
        readModule: moduleOf[site.line],
        declaredLine: binding.line,
        declaredModule: moduleOf[binding.line],
        via,
      });
    if (binding.body && tokens[k + 1]?.text === '(' && !followed.has(word.text)) {
      followed.add(word.text);
      const [body, from, to] = binding.body;
      walkLoad(body, from, to, visitor([...via, word.text]), site);
    }
  };
  for (const unit of bodyUnits) walkLoad(unit, 0, unit.length, visitor([]));
  return problems;
}

// one load-order problem as a line of text
export const loadOrderText = (p) =>
  `${p.name} is read at load on line ${p.readLine} (${p.readModule}${p.via.length ? `, in ${p.via.join('() -> ')}()` : ''}) ` +
  `before its declaration on line ${p.declaredLine} (${p.declaredModule}) has run`;

// The load-time steps of a bundle's units, in order: the units that do something when the bundle loads (not
// declarations), and the declarations that read another top-level variable at load (a snapshot, `var V = o.v` or
// `{...state}`: an effect that changes what it reads may not move past it)
function stepsOf(bodyUnits) {
  const { bindings, names } = bindingsOf(bodyUnits);
  const readsVariable = (unit) => {
    let found = false;
    walkLoad(unit, 0, unit.length, (tokens, k) => {
      const binding = bindings.get(tokens[k].text);
      if (binding && binding.at >= 0 && !names.has(tokens[k])) found = true;
    });
    return found;
  };
  return bodyUnits.filter((unit) => !isDeclaration(unit) || readsVariable(unit));
}
// The load-time steps of a bundle (`stepsOf`), in order, as text
export const effects = (text) => stepsOf(units(text).slice(0, -1)).map(join);

// ---------- esbuild's collision suffixes

// A name esbuild printed with a collision suffix: two modules' top-level `run` become `run` and `run2` in module
// order, and a nested `p` that a new top-level `p` would shadow becomes `p2`. A move that changes the module order, or
// a library's short names, reshuffles them.
const SUFFIXED = /^([A-Za-z_$][\w$]*?)([2-9]|[1-9]\d+)$/;
// the browser's own names: esbuild never suffixes a global, and renaming a reference to one would change what it names
const GLOBALS = new Set([
  ...Object.getOwnPropertyNames(globalThis),
  ...['window', 'document', 'self', 'top', 'parent', 'name', 'location', 'navigator', 'event', 'status', 'origin'],
  ...['history', 'screen', 'localStorage', 'sessionStorage', 'indexedDB'],
]);
const NO_RENAME = new Map();

// whether the word at tokens[k] is a shorthand property (`{ a }`, `{ a = 1 }` in a pattern): its key and its binding
const isShorthand = (tokens, k) => {
  const [prev, next] = [tokens[k - 1]?.text, tokens[k + 1]?.text];
  if (!(prev === '{' || prev === ',') || !(next === ',' || next === '}' || next === '=')) return false;
  const open = holderOf(tokens, k);
  return open >= 0 && tokens[open].text === '{' && objectBrace(tokens, open);
};
// a template literal's text, the code in its `${}` as tokens joined (with the bindings in `rename` renamed)
const templates = new Map(); // a template's text -> its text unrenamed
function templateText(text, rename) {
  const pieces = templatePieces(text);
  if (pieces.length === 1) return text;
  const renames = [...rename.keys()].some((name) => text.includes(name));
  if (!renames && templates.has(text)) return templates.get(text);
  const joined = pieces.map((piece, j) => (j % 2 ? textOf(tokenize(piece), rename) : piece)).join('');
  if (!renames) templates.set(text, joined);
  return joined;
}
// Tokens as text: a space only where two words would otherwise merge, or two signs (`a - -b`, `a++ +b`); a template's
// code joined the same way. `rename` (name -> name): the bindings to rename, wherever a word names one (inside a
// template's `${}` too; not a property, an object key or a method's name); a shorthand property keeps its key (`{ a }`
// with a renamed to b reads `{a:b}`).
function textOf(tokens, rename = NO_RENAME) {
  const texts = tokens.map((t, k) => {
    if (t.kind === 'string' && t.text[0] === '`') return templateText(t.text, rename);
    if (t.kind !== 'word' || !rename.has(t.text) || isPropertyName(tokens, k)) return t.text;
    return isShorthand(tokens, k) ? `${t.text}:${rename.get(t.text)}` : rename.get(t.text);
  });
  return texts
    .map((text, k) => {
      const before = texts[k - 1]?.at(-1) ?? '';
      const glue = (WORD.test(before) && WORD.test(text[0])) || ('+-'.includes(before) && before === text[0]);
      return (k && glue ? ' ' : '') + text;
    })
    .join('');
}
const join = (tokens) => textOf(tokens);

// The suffix families of two bundles' units: each stem's names (the stem, stem2, stem3, ...) where a word names a
// binding, in either bundle, when there are two or more
function suffixFamilies(unitLists) {
  const words = new Set();
  const collect = (tokens) =>
    tokens.forEach((t, k) => {
      if (t.kind === 'word' && !isPropertyName(tokens, k)) words.add(t.text);
      else if (t.kind === 'string' && t.text[0] === '`')
        for (const [j, piece] of templatePieces(t.text).entries()) if (j % 2) collect(tokenize(piece));
    });
  for (const unitList of unitLists) for (const unit of unitList) collect(unit);
  const families = new Map();
  for (const word of words) {
    const m = SUFFIXED.exec(word);
    if (!m || GLOBALS.has(word) || GLOBALS.has(m[1])) continue;
    if (!families.has(m[1])) families.set(m[1], new Set(words.has(m[1]) ? [m[1]] : []));
    families.get(m[1]).add(word);
  }
  return [...families].filter(([, names]) => names.size > 1).sort(([a], [b]) => (a < b ? -1 : 1));
}
// every order of `items`
const orders = (items) =>
  items.length < 2 ? [items] : items.flatMap((x, k) => orders(items.filter((_, j) => j !== k)).map((o) => [x, ...o]));

// The renaming of the after-bundle's suffixed names that leaves the fewest units differing: within each family, its
// names in another order (every order for up to five names, else swaps while one helps), family by family, twice (a
// unit can hold two families). A renaming within a family maps its names one to one and every word naming one of
// them, so the renamed bundle runs as the after-bundle does: a pure move reads the same, a change still differs.
// Returns after's name -> the name it reads as (only the names that moved).
function pairSuffixes(before, after) {
  const rename = new Map();
  const [unchanged, changed] = listDiff(before.map(join), after.map(join));
  if (unchanged.length + changed.length === 0) return rename; // (nothing to pair: the names already agree)
  for (let pass = 0; pass < 2; pass++)
    for (const [, family] of suffixFamilies([before, after])) {
      const names = [...family].sort();
      const pattern = new RegExp(`(?<![\\w$])(?:${names.map((n) => n.replace(/\$/g, '\\$')).join('|')})(?![\\w$])`);
      const touched = (units) => units.filter((unit) => unit.some((t) => pattern.test(t.text)));
      const [beforeTexts, afterUnits] = [touched(before).map(join), touched(after)];
      const withOrder = (order) => {
        const trial = new Map(rename);
        for (const [k, name] of names.entries()) {
          if (order[k] === name) trial.delete(name);
          else trial.set(name, order[k]);
        }
        return trial;
      };
      const cost = (order) => {
        const trial = withOrder(order);
        const [gone, added] = listDiff(
          beforeTexts,
          afterUnits.map((unit) => textOf(unit, trial)),
        );
        return gone.length + added.length;
      };
      let best = names.map((name) => rename.get(name) ?? name);
      let bestCost = cost(best);
      if (bestCost === 0) continue;
      if (names.length <= 5)
        for (const order of orders(names)) {
          const c = cost(order);
          if (c < bestCost) [best, bestCost] = [order, c];
        }
      else
        for (let improved = true; improved; ) {
          improved = false;
          for (let i = 0; i < best.length; i++)
            for (let j = i + 1; j < best.length; j++) {
              const order = [...best];
              [order[i], order[j]] = [order[j], order[i]];
              const c = cost(order);
              if (c < bestCost) [best, bestCost, improved] = [order, c, true];
            }
        }
      for (const [name, to] of withOrder(best)) if (family.has(name)) rename.set(name, to);
      for (const name of names) if (rename.get(name) === name) rename.delete(name);
    }
  return rename;
}

// The difference of two lists: [entries only in a, entries only in b] (each as often as it is in excess)
export function listDiff(a, b) {
  const count = new Map();
  for (const l of a) count.set(l, (count.get(l) || 0) + 1);
  for (const l of b) count.set(l, (count.get(l) || 0) - 1);
  const only = (sign) => [...count].flatMap(([l, n]) => (Math.sign(n) === sign ? Array(Math.abs(n)).fill(l) : []));
  return [only(1), only(-1)];
}

// Two bundles compared, after's suffixed names paired with before's (`pairSuffixes`): {unitCount, gone, added (units'
// text only in one), effectsBefore, effectsAfter (the load-time steps), sameOrder, renamed (after's names read as
// another: [name, as]), same}
export function compare(before, after) {
  const [unitsBefore, unitsAfter] = [units(before), units(after)];
  const rename = pairSuffixes(unitsBefore, unitsAfter);
  const [ua, ub] = [unitsBefore.map(join), unitsAfter.map((unit) => textOf(unit, rename))];
  const [gone, added] = listDiff(ua, ub);
  const effectsBefore = stepsOf(unitsBefore.slice(0, -1)).map(join);
  const effectsAfter = stepsOf(unitsAfter.slice(0, -1)).map((unit) => textOf(unit, rename));
  const sameOrder =
    effectsBefore.length === effectsAfter.length && effectsBefore.every((e, k) => e === effectsAfter[k]);
  return {
    unitCount: ub.length,
    gone,
    added,
    effectsBefore,
    effectsAfter,
    sameOrder,
    renamed: [...rename],
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
  try {
    const root = path.join(W, '..');
    const held = TREE.filter((p) => spawnSync('git', ['-C', root, 'cat-file', '-e', `${rev}:${p}`]).status === 0);
    const archive = spawnSync('git', ['-C', root, 'archive', '--format=tar', rev, ...held], {
      maxBuffer: 1 << 28,
    });
    if (archive.status !== 0) throw new Error(`git archive ${rev}: ${archive.stderr}`);
    const untar = spawnSync('tar', ['-x', '-C', dir], { input: archive.stdout });
    if (untar.status !== 0) throw new Error(`tar: ${untar.stderr}`);
    return path.join(dir, 'webapp');
  } catch (error) {
    fs.rmSync(dir, { recursive: true }); // (the folder made above: nothing of it is kept)
    throw error;
  }
}

// one bundle's comparison, printed: the changed units (a pair with the same first line shown where it first differs),
// then the first divergence of the effects
function report(entry, result, listEffects) {
  const { unitCount, gone, added, effectsBefore, effectsAfter, sameOrder } = result;
  const changed = gone.length + added.length;
  console.log(
    `${entry}: ${unitCount} top-level units, ${changed ? `${gone.length} gone and ${added.length} new` : 'the same'}; ` +
      `${effectsAfter.length} load-time steps, ${sameOrder ? 'in the same order' : `ORDER CHANGED (${effectsBefore.length} before)`}`,
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
      `  first divergence, step ${at + 1}:\n    before: ${show(effectsBefore[at])}\n    after:  ${show(effectsAfter[at])}`,
    );
  }
  if (result.renamed.length)
    console.log(
      `  note: suffixes paired (after's name read as): ${result.renamed.map(([a, b]) => `${a} as ${b}`).join(', ')}`,
    );
  if (listEffects) for (const e of effectsAfter) console.log(`    ${e.slice(0, 150)}`);
}

if (import.meta.main) {
  const args = process.argv.slice(2);
  const listEffects = args.includes('--effects');
  const [baseRev = 'HEAD', headRev = null] = args.filter((a) => a !== '--effects');
  const trees = []; // the folders made for revisions, removed however the comparison ends
  let same = true;
  let readBeforeDeclared = 0;
  // the entry's name in the tree `dir`
  const entryIn = (dir, names) => {
    const name = names.find((n) => fs.existsSync(path.join(dir, n)));
    if (!name) throw new Error(`no entry ${names.join(' or ')} in ${dir}`);
    return name;
  };
  try {
    trees.push(tree(baseRev));
    trees.push(tree(headRev));
    const [base, head] = trees;
    for (const names of ENTRIES) {
      const [fromName, toName] = [entryIn(base, names), entryIn(head, names)];
      const entry = fromName === toName ? toName : `${fromName} -> ${toName}`;
      const headText = await bundle(head, toName);
      const result = compare(await bundle(base, fromName), headText);
      report(entry, result, listEffects);
      same &&= result.same;
      const misordered = loadOrder(headText);
      for (const problem of misordered) console.log(`  READ BEFORE DECLARED: ${loadOrderText(problem)}`);
      readBeforeDeclared += misordered.length;
    }
  } finally {
    for (const dir of trees) if (dir !== W) fs.rmSync(path.dirname(dir), { recursive: true });
  }
  console.log(same ? 'samecode: the same code' : 'samecode: the code differs (above)');
  if (readBeforeDeclared) console.log(`samecode: ${readBeforeDeclared} read(s) at load before the declaration (above)`);
  process.exit(same && !readBeforeDeclared ? 0 : 1);
}
