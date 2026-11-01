// ---------- character icons, card art and skill icons: files in the page's image folder (data.ts IMG_BASE; from the game's own files) ----------
// Portraits, card art and skill icons are lazy unless marked eager: the browser fetches one only when it scrolls near the
// view (loading is set before src).
import { SHIPPED, IMG_EXT, IMG_BASE, SKILLICON, SKILLICON_GOLD, gameData } from './data.ts';
import { spinHTML } from './dom.ts';
import { must } from './must.ts';
import type { Card } from './data.ts';
// kind: the image folder ('uma', 'outfit', 'card', 'skill', 'run', ...)
export function srcFor(kind: string, id: number | string): string | null {
  return SHIPPED.has(`${kind}:${id}`) ? `${IMG_BASE}${kind}/${id}.${IMG_EXT[kind]}` : null;
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
// twin: ui/Images.tsx Runner (the same markup, as a component): an edit here goes there too, until step 8c
export function runnerHTML(big = false) {
  const src = runnerSrc();
  if (!src) return spinHTML();
  const delay = Math.round(performance.now() % RUN_CYCLE_MS);
  return `<span class="runner${big ? ' big' : ''}" aria-hidden="true" style="background-image:url(${src});animation-delay:-${delay}ms"></span>`;
}
// the UI sprites (--btn-w, --spark-white, ...) are CSS variables the build writes into the stylesheet (build.py SPRITES)
// twin: ui/Images.tsx SkillIcon (the same markup, as a component): an edit here goes there too, until step 8c
// group: a skill group; gold: its gold's icon
export function skillIcon(group: number, gold = false): HTMLImageElement | null {
  const src = srcFor('skill', (gold ? SKILLICON_GOLD : SKILLICON)[group]);
  if (!src) return null;
  const image = new Image();
  image.className = 'sic';
  image.alt = '';
  image.loading = 'lazy';
  image.src = src;
  return image;
}
// two letters for a portrait without art: the first letters of her first two words, else the first two of her one word
export function initials(name: string) {
  const words = name
    .replace(/[^A-Za-z .]/g, '')
    .split(/[ .]+/)
    .filter(Boolean);
  const first = words[0] || '';
  return ((first || '?')[0] + (words[1] ? words[1][0] : first[1] || '')).toUpperCase();
}
// twin: ui/Images.tsx UmaAv (the same markup, as a component; only the twin is ever eager, in the roster): an edit
// here goes there too, while both exist
// uma: her character id; null: an unknown uma (a parent nobody recorded, a damaged save), drawn as id 0 is: '?' on hue
// 0, no portrait. outfitId: her outfit's portrait, when known. size: in px
export function umaAv(uma: number | null, outfitId: number | null | undefined, size: number): HTMLSpanElement {
  const charaId = uma ?? 0;
  const disc = document.createElement('span');
  disc.className = 'av';
  // its size and its initials' (style.css .av reads them; a context resizes it with an ordinary rule)
  disc.style.setProperty('--s', `${size}px`);
  disc.style.setProperty('--fs', `${Math.round(size * 0.38)}px`);
  disc.style.setProperty('--h', String((charaId * 47) % 360)); // her disc's hue, fixed per character
  disc.textContent = initials(gameData().names.get(charaId) || '?');
  const tries = [outfitId && srcFor('outfit', outfitId), srcFor('uma', charaId), srcFor('mob', charaId)].filter(
    Boolean,
  ); // a mob (sample veteran) has her own portrait
  // with a source, the disc is left blank (its colour) until the portrait loads; the initials show only when there is no
  // art or every source fails
  const next = () => {
    const src = tries.shift();
    disc.textContent = '';
    if (!src) {
      disc.textContent = initials(gameData().names.get(charaId) || '?');
      return;
    }
    const image = new Image();
    image.alt = '';
    image.loading = 'lazy';
    image.onerror = next;
    image.src = src;
    disc.appendChild(image);
  };
  if (tries.length) next();
  return disc;
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
// twin: ui/Images.tsx CardAv (the same markup, as a component): an edit here goes there too, until step 8c
// `titled`: its own hover names (the card's name, its type); off inside a chip whose title says more
// size: in px
export function cardAv(c: Card | null, size: number, titled = true): HTMLSpanElement {
  const tile = document.createElement('span');
  tile.className = 'cav';
  tile.style.setProperty('--s', `${size}px`); // (style.css .cav, as umaAv's)
  tile.style.setProperty('--fs', `${Math.max(7, Math.round(size * 0.3))}px`);
  const kind = c?.kind || 'other';
  tile.style.setProperty('--k', `var(--k-${kind})`);
  tile.style.setProperty('--rr', c ? `var(--r-${RARITY_CLASS[c.rar] || 'r'})` : 'transparent');
  tile.textContent = KIND_ABBR[kind] || '·';
  if (titled) tile.title = c ? c.name : '';
  const src = c && srcFor('card', c.id);
  if (src) {
    const image = new Image();
    image.alt = '';
    image.loading = 'lazy';
    image.src = src;
    tile.textContent = '';
    tile.classList.add('art');
    tile.appendChild(image);
    image.onerror = () => {
      image.remove();
      tile.classList.remove('art');
      tile.textContent = KIND_ABBR[kind] || '·';
    };
    const typeIcon = srcFor('ui', `type_${kind}`);
    if (typeIcon) {
      const i = document.createElement('i');
      i.className = 'ctype';
      if (titled) i.title = kind;
      i.style.backgroundImage = `url(${typeIcon})`;
      tile.appendChild(i);
    }
  }
  return tile;
}
