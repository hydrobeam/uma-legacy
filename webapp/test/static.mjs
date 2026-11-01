// Static checks on the page's source, run by `./build.sh lint` (no browser, no build): what the browser suite can't see
// going stale.
// - dead CSS: a class or id in style.css's selectors that no page script, template or build.py names (as a whole word,
//   HTML entities aside). A name the code builds from parts is listed in BUILT with where it is built.
// - dead exports: an export of src/ that no other file (src/, test/ and its folders) names.
// - stale storage writes: `localStorage.setItem(key, JSON.stringify(x))` where `x` was not read or built in the same
//   function after its last await. A copy read earlier (at load, or before a request) overwrites what another tab or
//   call stored meanwhile; read what is stored, change your part, write it.
// Exits 1 with the list when either finds one; delete the dead code, or list a built name.
import fs from 'node:fs';
import path from 'node:path';

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
  leg0: 'afftable.js, the legacy columns: leg + the legacy index',
  leg1: 'afftable.js, the legacy columns: leg + the legacy index',
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

if (problems.length) {
  console.log(problems.join('\n'));
  console.log(`${problems.length} static check failure${problems.length === 1 ? '' : 's'} (webapp/test/static.mjs)`);
  process.exit(1);
}
console.log('static checks: no dead CSS, no dead exports, no stale storage writes');
