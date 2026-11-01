// ---------- a node's sparks as elements: the chips, the coloured bars and their picker ----------
import { lazyDialog } from '../dom.js';
import { careerTables, gameData } from '../data.js';
import { skillIcon } from '../images.js';
import { must } from '../must.ts';
import { NEW_STARS, inPlaceRef } from './store.ts';
import { skillName, pinkName, blueName, uniqueName, targetGroups } from './names.js';
import { openAdd } from './pickers.js';
/** @import { LineageNode, NodeRecord, NodeRef } from './store.ts' */

// A spark's stars: filled then empty, out of 3 (two spans: the career page's game-style card colours them apart), hidden
// from the screen reader (the control's label says the count).
function starsEl(stars) {
  const box = document.createElement('span');
  box.className = 'stars';
  box.setAttribute('aria-hidden', 'true');
  const filled = document.createElement('span');
  filled.className = 'sf';
  filled.textContent = '★'.repeat(stars);
  filled.style.setProperty('--n', String(stars));
  const empty = document.createElement('span');
  empty.className = 'se';
  empty.textContent = '☆'.repeat(3 - stars);
  empty.style.setProperty('--n', String(3 - stars));
  box.append(filled, empty);
  return box;
}
/**
 * One spark as a chip: a target skill (click: stars 1 -> 2 -> 3 -> 1; the corner x removes), a pink or a unique.
 * @typedef {object} ChipSpec
 * @property {string} label  the spark's name
 * @property {Node|null} icon  the skill's icon, if any
 * @property {number} stars  1-3
 * @property {boolean} pink  an aptitude (pink) spark
 * @property {() => void} onCycle  a click on the chip
 * @property {() => void} onRemove  a click on its corner x
 * @property {string} key  names the controls (data-k), so the page can put the keyboard focus back on the same one after
 *   a redraw
 * @property {boolean} [readOnly]  shown, not editable: no x, no click
 * @property {boolean} [other]  not one of the targets, greyed (in a list that shows other skills too)
 */
/** @param {ChipSpec} spec */
function chip({ label, icon, stars, pink, onCycle, onRemove, key, readOnly = false, other = false }) {
  const c = document.createElement('span');
  // a skill that isn't a target, among the targets: the game's grey (unobtained) look
  c.className = `vsk${pink ? ' pink' : ''}${other ? ' off' : ''}`;
  const b = document.createElement(readOnly ? 'span' : 'button');
  if (!readOnly) /** @type {HTMLButtonElement} */ (b).type = 'button';
  b.className = 'vskb';
  if (icon) b.appendChild(icon);
  const t = document.createElement('span');
  t.className = `vskn${label.length > 18 ? ' long' : ''}`; // (the game scales a long name down to fit)
  t.textContent = label;
  b.appendChild(t);
  b.appendChild(starsEl(stars));
  if (readOnly) {
    b.setAttribute(
      'aria-label',
      `${label}${pink ? ' (aptitude)' : ''}, ${stars} stars${other ? ', not a target' : ''}`,
    );
    c.appendChild(b);
    return c;
  }
  b.setAttribute(
    'aria-label',
    `${label}${pink ? ' (aptitude)' : ''}, ${stars} star${stars > 1 ? 's' : ''}. Click for ${(stars % 3) + 1}`,
  );
  b.onclick = onCycle;
  b.dataset.k = key;
  const x = document.createElement('button');
  x.type = 'button';
  x.className = 'vskx';
  x.textContent = '×';
  x.setAttribute('aria-label', `Remove ${label}`);
  x.title = `Remove ${label}`;
  x.onclick = onRemove;
  x.dataset.k = `${key}x`;
  c.append(b, x);
  return c;
}
// A veteran's sparks as the game's Inspiration cells: her blue (stat), pink (aptitude) and unique (green) sparks first,
// each a bar of its colour (a dashed one when she has none) that opens its picker; then her target skills' white sparks
// (click: stars 1 -> 2 -> 3 -> 1, the corner x removes), then "+" (a target skill she lacks). Sparks of skills that are
// not targets are kept but not shown (read-only: shown). key names the controls (data-k) for the focus after a redraw.
const noEdit = () => {
  // (read-only: nothing to follow)
};
// (Its one caller left is the veteran analysis' draft, edited in place: step 9 draws it with ui/InspView.)
/** @param {LineageNode} nd @param {string} [key] @param {() => void} [onEdit] @param {boolean} [readOnly] @returns {HTMLDivElement} */
export function sparksEl(nd, key = '', onEdit = noEdit, readOnly = false) {
  const at = inPlaceRef(nd, onEdit);
  const sp = document.createElement('div');
  sp.className = 'vsparks';
  for (const kind of ['blue', 'pink', 'green']) {
    const b = colourBar(at, kind, key, onEdit, readOnly);
    if (b) sp.appendChild(b);
  }
  // her target sparks (all she shows when editable); read-only, all her white sparks, the targets first, the rest grey
  const targets = new Set(targetGroups()),
    groups = readOnly
      ? Object.keys(nd.sparks)
          .map(Number)
          .sort((a, b) => Number(targets.has(b)) - Number(targets.has(a)))
      : targetGroups();
  for (const g of groups) {
    const stars = nd.sparks[g];
    if (!stars) continue;
    sp.appendChild(
      chip({
        label: skillName(g),
        icon: skillIcon(g, false),
        stars,
        pink: false,
        key: `${key}:s${g}`,
        readOnly,
        other: readOnly && !targets.has(g),
        onCycle: () => {
          nd.sparks[g] = (stars % 3) + 1;
          onEdit();
        },
        onRemove: () => {
          delete nd.sparks[g];
          onEdit();
        },
      }),
    );
  }
  if (readOnly) return sp;
  const add = document.createElement('button');
  add.type = 'button';
  add.className = 'vskadd';
  add.textContent = '+';
  add.setAttribute('aria-label', 'Add a target skill spark');
  add.title = 'Add a target skill spark';
  add.setAttribute('aria-haspopup', 'dialog');
  add.setAttribute('aria-expanded', 'false');
  add.onclick = () => openAdd(add, at);
  add.dataset.k = `${key}:add`;
  sp.appendChild(add);
  return sp;
}
/**
 * Her blue / pink / unique spark: one each (the game gives every veteran one blue and one pink; a unique spark is hers or
 * none).
 * @typedef {object} SparkKind
 * @property {'blues'|'pinks'|'greens'} field  the node's field: blues {stat group: stars}, pinks {aptitude group:
 *   stars}, greens {outfit card id: stars}
 * @property {string} title  the picker's title
 * @property {(group: number|string) => string|null} name  a group's name
 * @property {(nd: NodeRecord) => number[]} groups  the groups she can have one of
 */
/** @type {{blue: SparkKind, pink: SparkKind, green: SparkKind}} */
export const KIND = {
  blue: {
    field: 'blues',
    title: 'Blue spark',
    name: blueName,
    groups: () => Object.keys(careerTables().blues || { 1: 1, 2: 1, 3: 1, 4: 1, 5: 1 }).map(Number),
  },
  pink: { field: 'pinks', title: 'Pink spark', name: pinkName, groups: (nd) => pinkChoices(nd) },
  green: {
    field: 'greens',
    title: 'Unique spark',
    name: uniqueName,
    groups: (nd) => (nd.outfit && uniqueName(nd.outfit) ? [nd.outfit] : []),
  },
};
// the aptitudes a pink can come from: those she ends at A or S (a pink is drawn from them only): her end-of-career
// grades when known (the veteran analysis), else her uma's base grades; the pink she has always stays a choice
const APT_OF = { 31: 0, 32: 1, 33: 2, 34: 3, 11: 4, 12: 5, 21: 6, 22: 7, 23: 8, 24: 9 };
function pinkChoices(nd) {
  const grades = nd.apt || gameData().umas.find((u) => u.id === nd.chara)?.apt;
  const all = Object.keys(careerTables().pinks).map(Number),
    have = Object.keys(nd.pinks || {}).map(Number);
  return grades ? all.filter((g) => grades[APT_OF[g]] >= 7 || have.includes(g)) : all;
}
// her spark of one kind, as [group (an object key: a string), stars], or undefined (none)
export const currentSpark = (nd, kind) => Object.entries(nd[KIND[kind].field] || {}).find(([, stars]) => stars > 0);
// her spark of one kind as its coloured bar: a chip for a unique or a pink she has, else a bar that opens the picker (a
// missing unique's bar adds it at NEW_STARS)
function colourBar(at, kind, key, onEdit, readOnly) {
  const nd = must(at.node(), 'the draft is there while drawn'),
    spec = KIND[kind],
    cur = currentSpark(nd, kind);
  if (!cur && (readOnly || !spec.groups(nd).length)) return null;
  const k = `${key}:${kind}`;
  if (kind === 'green') {
    if (cur) {
      const [group, stars] = cur;
      const c = chip({
        label: uniqueName(group) || 'Unique spark',
        icon: null,
        stars,
        pink: false,
        key: k,
        readOnly,
        onCycle: () => {
          nd.greens[group] = (stars % 3) + 1;
          onEdit();
        },
        onRemove: () => {
          delete nd.greens[group];
          onEdit();
        },
      });
      c.className = 'vsb green';
      const b = must(c.querySelector('.vskb'), 'chip() makes its face');
      b.setAttribute(
        'aria-label',
        must(b.getAttribute('aria-label'), 'chip() labels its face').replace(/^([^,]+),/, '$1 (unique),'),
      );
      return c;
    }
    const c = document.createElement('span');
    c.className = 'vsb green none';
    const b = document.createElement('button');
    b.type = 'button';
    b.className = 'vskb';
    b.dataset.k = k;
    b.setAttribute('aria-label', 'Unique spark: none. Add it at 2 stars');
    b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: '+ Unique spark' }));
    b.onclick = () => {
      nd.greens = { [nd.outfit]: NEW_STARS };
      onEdit();
    };
    c.appendChild(b);
    return c;
  }
  if (cur && kind === 'pink') {
    // (pinks keep the chip's look and label: "Mile (aptitude), 2 stars")
    const [group, stars] = cur;
    const c = chip({
      label: pinkName(group),
      icon: null,
      stars,
      pink: true,
      key: k,
      readOnly,
      onCycle: () => openColour(at, kind, k),
      onRemove: () => {
        delete nd.pinks[group];
        onEdit();
      },
    });
    const b = must(c.querySelector('.vskb'), 'chip() makes its face');
    if (!readOnly) {
      b.setAttribute('aria-haspopup', 'dialog');
      b.setAttribute('aria-label', `${pinkName(group)} (aptitude), ${stars} star${stars > 1 ? 's' : ''}. Change`);
    }
    c.querySelector('.vskx')?.remove();
    return c;
  }
  // a blue (she has one or not), or no pink yet
  const c = document.createElement('span');
  c.className = `vsb ${kind}${cur ? '' : ' none'}`;
  const b = document.createElement(readOnly ? 'span' : 'button');
  if (!readOnly) {
    /** @type {HTMLButtonElement} */ (b).type = 'button';
    b.dataset.k = k;
    b.setAttribute('aria-haspopup', 'dialog');
    b.onclick = () => openColour(at, kind, k);
  }
  b.className = 'vskb';
  const label = cur ? spec.name(cur[0]) || spec.title : `${spec.title}: none`;
  b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: label }));
  if (cur) b.appendChild(starsEl(cur[1]));
  b.setAttribute(
    'aria-label',
    cur
      ? `${label} (${kind === 'blue' ? 'stat' : 'unique'}), ${cur[1]} star${cur[1] > 1 ? 's' : ''}${readOnly ? '' : '. Change'}`
      : `${spec.title}: none. Set it`,
  );
  c.appendChild(b);
  return c;
}
// the picker (game style): the choices as bars of the spark's colour, its stars (click one), Done (no remove: every
// veteran has one blue and one pink). Each pick lands on her record as it is then (`at`). Closed, the focus goes back
// to the bar that opened it; `back` finds it when it was drawn again meanwhile (the el() draft's card: its key).
/** @type {(() => HTMLElement | null) | null} */
let colBack = null;
const colourDialog = lazyDialog('gmdlg coldlg', (dialog) => {
  dialog.id = 'col-dlg';
  dialog.setAttribute('aria-labelledby', 'col-title');
  dialog.addEventListener('close', () => colBack?.()?.focus());
});
/** @param {NodeRef} at @param {'blue'|'pink'|'green'} kind @param {string} [key] the opener's data-k (el() only) */
export function openColour(at, kind, key = '') {
  const spec = KIND[kind];
  const colDlg = colourDialog.get(),
    opener = /** @type {HTMLElement | null} */ (document.activeElement);
  colBack = () => (opener?.isConnected ? opener : key ? document.querySelector(`[data-k="${CSS.escape(key)}"]`) : null);
  const draw = () => {
    const nd = at.node();
    if (!nd) return;
    colDlg.textContent = '';
    // her spark now: its group (null: none yet, so no stars to pick) and stars (0: none)
    const cur = currentSpark(nd, kind),
      curGroup = cur ? +cur[0] : null,
      curStars = cur ? cur[1] : 0;
    const h = document.createElement('h3');
    h.className = 'gmdhdr';
    h.id = 'col-title';
    h.textContent = spec.title;
    colDlg.appendChild(h);
    const body = document.createElement('div');
    body.className = 'gmdbody';
    const wrap = document.createElement('div');
    wrap.className = 'ud udbare';
    body.appendChild(wrap);
    const grid = document.createElement('div');
    grid.className = 'colgrid vsparks';
    for (const g of spec.groups(nd)) {
      const c = document.createElement('span');
      c.className = kind === 'pink' ? 'vsk pink' : `vsb ${kind}`;
      const b = document.createElement('button');
      b.type = 'button';
      b.className = 'vskb';
      b.setAttribute('aria-pressed', String(g === curGroup));
      b.appendChild(Object.assign(document.createElement('span'), { className: 'vskn', textContent: spec.name(g) }));
      b.onclick = () => {
        at.edit(() => ({ [spec.field]: { [g]: curStars || NEW_STARS } }));
        draw();
      };
      c.appendChild(b);
      grid.appendChild(c);
    }
    wrap.appendChild(grid);
    const row = document.createElement('div');
    row.className = 'spstars';
    row.setAttribute('role', 'group');
    row.setAttribute('aria-label', 'Stars');
    for (let k = 1; k <= 3; k++) {
      const st = document.createElement('button');
      st.type = 'button';
      st.className = `spstar${k <= curStars ? ' on' : ''}`;
      st.setAttribute('aria-label', `${k} star${k > 1 ? 's' : ''}`);
      st.setAttribute('aria-pressed', String(k === curStars));
      st.disabled = curGroup === null;
      st.onclick = () => {
        at.edit(() => ({ [spec.field]: { [must(curGroup, 'a star is enabled once a colour is chosen')]: k } }));
        draw();
      };
      row.appendChild(st);
    }
    body.appendChild(row);
    if (kind === 'blue')
      body.appendChild(
        Object.assign(document.createElement('p'), {
          className: 'gmdstat',
          textContent: 'A 3★ blue needs that stat at 600 or more (rank B) by the end of the career.',
        }),
      );
    if (kind === 'pink')
      body.appendChild(
        Object.assign(document.createElement('p'), {
          className: 'gmdstat',
          textContent: nd.apt
            ? 'The aptitudes she ends this career at A or S.'
            : "Her uma's aptitudes at A or S (a pink needs A or better).",
        }),
      );
    colDlg.appendChild(body);
    const f = document.createElement('div');
    f.className = 'gmdfoot';
    const done = document.createElement('button');
    done.type = 'button';
    done.className = 'btn gmclose';
    done.textContent = 'Done';
    done.onclick = () => colDlg.close();
    f.appendChild(done);
    colDlg.appendChild(f);
  };
  draw();
  colDlg.showModal();
  colDlg.querySelector('.gmclose:last-child')?.focus();
}
