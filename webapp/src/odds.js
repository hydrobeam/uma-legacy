// ---------- the odds popup (both planner pages): the chance of landing exactly k of the n targets ----------
import { el } from './dom.js';
// under 1%: two significant figures (0.0061%, 0.013%), so long odds that differ don't print the same
export const pct = (p) => { const x = p * 100; if (p === 0) return '0%';
  if (x < 1e-6) return '<0.000001%';
  return (x >= 10 ? x.toFixed(0) : x >= 1 ? x.toFixed(1) : x.toFixed(Math.min(6, 1 - Math.floor(Math.log10(x))))) + '%'; };
let dlg = null, from = null;
// ex[k]: the chance of exactly k (k = 0 .. n); pAll: the all-n figure shown (the solver's exact one); expected: the mean count
export function openOdds({ title, n, ex, pAll, expected }) {
  from = document.activeElement;
  if (!dlg) { dlg = el('dialog', 'gmdlg'); dlg.id = 'odds-dlg'; dlg.setAttribute('aria-labelledby', 'odds-title');
    dlg.addEventListener('close', () => { if (from && from.isConnected) from.focus(); }); document.body.appendChild(dlg); }
  dlg.textContent = '';
  const h = el('h3', 'gmdhdr', title); h.id = 'odds-title'; dlg.appendChild(h);
  const body = el('div', 'gmdbody');
  const tb = el('table', 'oddtab'); tb.innerHTML = '<thead><tr><th scope="col">Targets landed</th><th scope="col">Chance</th></tr></thead><tbody></tbody>';
  for (let k = n; k >= 0; k--) {
    const tr = el('tr', k === n ? 'all' : ''); const th = el('th', '', `${k}/${n}`); th.scope = 'row';
    tr.append(th, el('td', '', pct(k === n ? pAll : ex[k] || 0))); tb.tBodies[0].appendChild(tr);
  }
  body.append(tb, el('p', 'gmdstat', `${expected.toFixed(2)} of ${n} expected.`));
  dlg.appendChild(body);
  const f = el('div', 'gmdfoot'), c = el('button', 'btn gmclose', 'Close'); c.type = 'button'; c.onclick = () => dlg.close(); f.appendChild(c); dlg.appendChild(f);
  dlg.showModal(); c.focus();
}
