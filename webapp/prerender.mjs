// The Global roster's tiles in default order (release, newest first; all owned), for build.py to write into the page:
// the same markup the page script builds (src/rostercore.js). Reads {umas, shipped, ext, base} as JSON on stdin.
import fs from 'fs';
import { rosterTiles, compareTiles, tileHTML, EAGER } from './src/rostercore.js';
const { umas, shipped, ext, base } = JSON.parse(fs.readFileSync(0, 'utf8'));
const S = new Set(shipped), src = (kind, id) => S.has(kind + ':' + id) ? `${base}${kind}/${id}.${ext[kind]}` : null;
const tiles = rosterTiles(umas).sort(compareTiles('rel', 'desc'));
process.stdout.write(tiles.map((t, i) => tileHTML(t, { src: (t.o && src('outfit', t.o.id)) || src('uma', t.u.id), eager: i < EAGER, state: 'own' })).join(''));
