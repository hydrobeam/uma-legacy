// ---------- the odds popup (both planner pages): the chance of landing exactly k of the n targets ----------
import { el, backdropCloses } from './dom.js';
// under 1%: two significant figures (0.0061%, 0.013%), so long odds that differ don't print the same
/** @type {(p: number) => string} */
export const pct = (p) => {
  const percent = p * 100;
  if (p === 0) return '0%';
  if (percent < 1e-6) return '<0.000001%';
  const digits = percent >= 10 ? 0 : percent >= 1 ? 1 : Math.min(6, 1 - Math.floor(Math.log10(percent)));
  return `${percent.toFixed(digits)}%`;
};
// the dialog, made on first use; `opener`: the element focused before it opened, focused again when it closes
let dialog = null,
  opener = null;
// ex[k]: the chance of exactly k (k = 0 .. n); pAll: the all-n figure shown (the solver's exact one); expected: the mean count
/** @param {{title: string, n: number, ex: number[], pAll: number, expected: number}} spec */
export function openOdds({ title, n, ex, pAll, expected }) {
  opener = document.activeElement;
  if (!dialog) {
    dialog = el('dialog', 'gmdlg');
    dialog.id = 'odds-dlg';
    dialog.setAttribute('aria-labelledby', 'odds-title');
    dialog.addEventListener('close', () => {
      if (opener?.isConnected) opener.focus();
    });
    backdropCloses(dialog);
    document.body.appendChild(dialog);
  }
  dialog.textContent = '';
  const heading = el('h3', 'gmdhdr', title);
  heading.id = 'odds-title';
  dialog.appendChild(heading);
  const body = el('div', 'gmdbody');
  const table = el('table', 'oddtab');
  table.innerHTML =
    '<thead><tr><th scope="col">Targets landed</th><th scope="col">Chance</th></tr></thead><tbody></tbody>';
  for (let k = n; k >= 0; k--) {
    const row = el('tr', k === n ? 'all' : ''),
      count = el('th', '', `${k}/${n}`);
    count.scope = 'row';
    row.append(count, el('td', '', pct(k === n ? pAll : ex[k] || 0)));
    table.tBodies[0].appendChild(row);
  }
  body.append(table, el('p', 'gmdstat', `${expected.toFixed(2)} of ${n} expected.`));
  dialog.appendChild(body);
  const foot = el('div', 'gmdfoot'),
    close = el('button', 'btn gmclose', 'Close');
  close.type = 'button';
  close.onclick = () => dialog.close();
  foot.appendChild(close);
  dialog.appendChild(foot);
  dialog.showModal();
  close.focus();
}
