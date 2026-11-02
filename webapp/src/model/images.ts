// ---------- character icons, card art and skill icons: files in the page's image folder (model/data.ts IMG_BASE; from the game's own files) ----------
// Their paths, the runner's pick and what a portrait or a card tile shows without art; ui/Images.tsx draws them.
import { SHIPPED, IMG_EXT, IMG_BASE, SKILLICON, SKILLICON_GOLD } from './data.ts';
import { must } from '../must.ts';
// kind: the image folder ('uma', 'outfit', 'card', 'skill', 'run', ...)
export function srcFor(kind: string, id: number | string): string | null {
  return SHIPPED.has(`${kind}:${id}`) ? `${IMG_BASE}${kind}/${id}.${IMG_EXT[kind]}` : null;
}
/** a skill's icon (its gold's: `gold`), or null: the data has none for it, or the page doesn't ship it */
export function skillIconSrc(group: number, gold: boolean): string | null {
  const icon = (gold ? SKILLICON_GOLD : SKILLICON)[group];
  return icon === undefined ? null : srcFor('skill', icon);
}
// the runner in place of a spinner: an uma's mini model running (img/run/<chara>: a strip of 18 frames of the game's
// mini run, stepped by CSS, webapp/chibi/chibi.py). Who runs: the uma being planned (setRunner: the career page's
// trainee; setRunnerFrom: one of the loop's umas at random, kept while the loop on show has her); before either, a random
// uma with a strip. Her strip is fetched as soon as she is picked, so she is drawn when the next search shows her; a
// build without strips keeps the spinner. The negative delay keeps her stride when a status line is redrawn.
let runner: number | null = null;
export const RUN_CYCLE_MS = 600; // one loop of the strip (the CSS animation's duration)
const hasRun = (id: number) => SHIPPED.has(`run:${id}`);
// id: a character id
export function setRunner(id: number | null | undefined) {
  if (!id || !hasRun(id) || id === runner) return;
  runner = id;
  new Image().src = must(srcFor('run', id), 'hasRun: her strip is shipped');
}
// (the one running stays while she is among them: a new strip is fetched and decoded only when the loop changes)
// ids: character ids
export function setRunnerFrom(ids: number[]) {
  const withStrip = ids.filter(hasRun);
  if (withStrip.length && (runner === null || !withStrip.includes(runner)))
    // (none picked yet: pick one)
    setRunner(withStrip[Math.floor(Math.random() * withStrip.length)]);
}
// the runner's strip now (one picked at random the first time), or null: none shipped, a spinner instead
export function runnerSrc() {
  if (runner == null) setRunnerFrom([...SHIPPED].filter((key) => key.startsWith('run:')).map((key) => +key.slice(4)));
  return runner != null ? srcFor('run', runner) : null;
}
// two letters for a portrait without art: the first letters of her first two words, else the first two of her one word
export function initials(name: string) {
  const words = name
    .replace(/[^A-Za-z .]/g, '')
    .split(/[ .]+/)
    .filter(Boolean);
  const [first = '', second = ''] = words;
  return ((first || '?').slice(0, 1) + (second ? second.slice(0, 1) : first.slice(1, 2))).toUpperCase();
}
// a card's type as the text shown without its art (Card `kind`)
export const KIND_ABBR: Record<string, string> = {
  speed: 'SPD',
  stamina: 'STA',
  power: 'POW',
  guts: 'GUT',
  wit: 'WIT',
  pal: 'PAL',
  group: 'GRP',
  other: '·',
};
export const RARITY_CLASS = { 3: 'ssr', 2: 'sr', 1: 'r' };
