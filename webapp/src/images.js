// ---------- character icons, card art and skill icons: files in the page's image folder (data.js IMG_BASE; from the game's own files) ----------
// Portraits, card art and skill icons are lazy unless marked eager: the browser fetches one only when it scrolls near the
// view (loading is set before src).
import { SHIPPED, IMG_EXT, IMG_BASE, SKILLICON, SKILLICON_GOLD, NAME } from './data.js';
import { spinHTML } from './dom.js';
/** @param {string} kind the image folder ('uma', 'outfit', 'card', 'skill', 'run', ...) @param {number|string} id @returns {string|null} */
export function srcFor(kind, id) {
  return SHIPPED.has(`${kind}:${id}`) ? `${IMG_BASE}${kind}/${id}.${IMG_EXT[kind]}` : null;
}
// the runner in place of a spinner: an uma's mini model running (img/run/<chara>: a strip of 18 frames of the game's
// mini run, stepped by CSS, webapp/chibi/chibi.py). Who runs: the uma being planned (setRunner: the career page's
// trainee; setRunnerFrom: one of the loop's umas at random, kept while the loop on show has her); before either, a random
// uma with a strip. Her strip is fetched as soon as she is picked, so she is drawn when the next search shows her; a
// build without strips keeps the spinner. The negative delay keeps her stride when a status line is redrawn.
let runner = null;
const RUN_CYCLE_MS = 600; // one loop of the strip (the CSS animation's duration)
const hasRun = (id) => SHIPPED.has(`run:${id}`);
/** @param {number|null|undefined} id a character id */
export function setRunner(id) {
  if (!id || !hasRun(id) || id === runner) return;
  runner = id;
  new Image().src = srcFor('run', id);
}
// (the one running stays while she is among them: a new strip is fetched and decoded only when the loop changes)
/** @param {number[]} ids character ids */
export function setRunnerFrom(ids) {
  const withStrip = ids.filter(hasRun);
  if (withStrip.length && !withStrip.includes(runner))
    setRunner(withStrip[Math.floor(Math.random() * withStrip.length)]);
}
export function runnerHTML(big = false) {
  if (runner == null) setRunnerFrom([...SHIPPED].filter((key) => key.startsWith('run:')).map((key) => +key.slice(4)));
  const src = runner != null && srcFor('run', runner);
  if (!src) return spinHTML();
  const delay = Math.round(performance.now() % RUN_CYCLE_MS);
  return `<span class="runner${big ? ' big' : ''}" aria-hidden="true" style="background-image:url(${src});animation-delay:-${delay}ms"></span>`;
}
// the UI sprites (--btn-w, --spark-white, ...) are CSS variables the build writes into the stylesheet (build.py SPRITES)
/** @param {number} group a skill group @param {boolean} [gold] its gold's icon @returns {HTMLImageElement|null} */
export function skillIcon(group, gold = false) {
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
function initials(name) {
  const words = name
    .replace(/[^A-Za-z .]/g, '')
    .split(/[ .]+/)
    .filter(Boolean);
  const first = words[0] || '';
  return ((first || '?')[0] + (words[1] ? words[1][0] : first[1] || '')).toUpperCase();
}
// eager: an image in the first screenful (the roster's first rows): fetched at once, at high priority, not lazily
/** @param {number} charaId @param {number|null|undefined} outfitId her outfit's portrait, when known @param {number} size in px @param {boolean} [eager] @returns {HTMLSpanElement} */
export function umaAv(charaId, outfitId, size, eager = false) {
  const disc = document.createElement('span');
  disc.className = 'av';
  disc.style.width = disc.style.height = `${size}px`;
  disc.style.fontSize = `${Math.round(size * 0.38)}px`;
  disc.style.setProperty('--h', String((charaId * 47) % 360)); // her disc's hue, fixed per character
  disc.textContent = initials(NAME.get(charaId) || '?');
  const tries = [outfitId && srcFor('outfit', outfitId), srcFor('uma', charaId), srcFor('mob', charaId)].filter(
    Boolean,
  ); // a mob (sample veteran) has her own portrait
  // with a source, the disc is left blank (its colour) until the portrait loads; the initials show only when there is no
  // art or every source fails
  const next = () => {
    const src = tries.shift();
    disc.textContent = '';
    if (!src) {
      disc.textContent = initials(NAME.get(charaId) || '?');
      return;
    }
    const image = new Image();
    image.alt = '';
    image.loading = eager ? 'eager' : 'lazy';
    if (eager) image.fetchPriority = 'high';
    image.onerror = next;
    image.src = src;
    disc.appendChild(image);
  };
  if (tries.length) next();
  return disc;
}
// a card's type as the text shown without its art
const KIND_ABBR = {
  speed: 'SPD',
  stamina: 'STA',
  power: 'POW',
  guts: 'GUT',
  wit: 'WIT',
  pal: 'PAL',
  group: 'GRP',
  other: '·',
};
const RARITY_CLASS = { 3: 'ssr', 2: 'sr', 1: 'r' };
/** @param {import('./data.js').Card|null} c @param {number} size in px @returns {HTMLSpanElement} */
export function cardAv(c, size) {
  const tile = document.createElement('span');
  tile.className = 'cav';
  tile.style.width = tile.style.height = `${size}px`;
  tile.style.fontSize = `${Math.max(7, Math.round(size * 0.3))}px`;
  const kind = c?.kind || 'other';
  tile.style.setProperty('--k', `var(--k-${kind})`);
  tile.style.setProperty('--rr', c ? `var(--r-${RARITY_CLASS[c.rar] || 'r'})` : 'transparent');
  tile.textContent = KIND_ABBR[kind] || '·';
  tile.title = c ? c.name : '';
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
      i.title = kind;
      i.style.backgroundImage = `url(${typeIcon})`;
      tile.appendChild(i);
    }
  }
  return tile;
}
// scenario logo (game title art, img/scen/{id}; drawn whole inside its box, aspect kept); without it, or if it fails
// to load, a badge with the scenario's name on a light shade of its colour
/** @param {import('./data.js').Scenario} sc @returns {HTMLSpanElement} */
export function scenLogo(sc) {
  const logo = document.createElement('span');
  logo.className = 'slogo';
  logo.setAttribute('aria-hidden', 'true');
  const badge = () => {
    logo.classList.remove('art');
    logo.textContent = '';
    const name = document.createElement('b');
    name.textContent = sc.name;
    name.style.background = sc.tint;
    name.style.color = '#18202e'; // dark text on the light tint, in either theme
    logo.appendChild(name);
  };
  const src = srcFor('scen', sc.id);
  if (!src) {
    badge();
    return logo;
  }
  const image = new Image();
  image.alt = '';
  image.onerror = badge;
  image.src = src;
  logo.classList.add('art');
  logo.appendChild(image);
  return logo;
}
