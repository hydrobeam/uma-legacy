// ---------- a node's colour sparks (blue, pink, unique): their kinds and their picker ----------
// (The sparks themselves are drawn by ui/Sparks.tsx.)
import { lazyDialog } from '../dom.js';
import { careerTables, gameData } from '../data.js';
import { must } from '../must.ts';
import { NEW_STARS } from './store.ts';
import { pinkName, blueName, uniqueName } from './names.js';
/** @import { NodeRecord, NodeRef } from './store.ts' */

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
// the picker (game style): the choices as bars of the spark's colour, its stars (click one), Done (no remove: every
// veteran has one blue and one pink). Each pick lands on her record as it is then (`at`). Closed, the focus goes back
// to the bar that opened it, while it is there.
/** @type {(() => HTMLElement | null) | null} */
let colBack = null;
const colourDialog = lazyDialog('gmdlg coldlg', (dialog) => {
  dialog.id = 'col-dlg';
  dialog.setAttribute('aria-labelledby', 'col-title');
  dialog.addEventListener('close', () => colBack?.()?.focus());
});
/** @param {NodeRef} at @param {'blue'|'pink'|'green'} kind */
export function openColour(at, kind) {
  const spec = KIND[kind];
  const colDlg = colourDialog.get(),
    opener = /** @type {HTMLElement | null} */ (document.activeElement);
  colBack = () => (opener?.isConnected ? opener : null);
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
