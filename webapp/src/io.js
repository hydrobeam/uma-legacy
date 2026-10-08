// ---------- export / import ----------
import { $ } from './dom.js';
import { paintTop } from './cards.js';
import { PER_ERA, SETTINGS, SAVED, ERA, persist } from './settings.js';
import { CARDS, UMAS, SK } from './data.js';
import { state, ui, goldGiven } from './state.js';
import { save } from './save.js';
import { noOutfit, renderRoster, reqOutfits, isRequired } from './roster.js';
import { renderTargets } from './targets.js';
import { fmtIn } from './results.js';
import { paintScen, paintRank, scenarioOf } from './scenario.js';
function exportText() {
  const nmG = (g, lv) => { const s = SK.get(g) || { w: String(g), gd: null }; return lv === 2 ? (s.gd || s.w) : s.w; };
  const settings = Object.fromEntries(SETTINGS.map(k => [k, state[k]]));
  const readable = {
    targets: state.targets.map(t => ({ skill: nmG(t.group, 1), gold: (SK.get(t.group) || {}).gd || null, guaranteed: goldGiven(t.group) ? 'gold' : 'no' })),
    umas_not_owned: UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).map(u => u.name),
    umas_required: UMAS.filter(isRequired).map(u => u.name + (reqOutfits(u).length ? ' ' + reqOutfits(u).map(o => o.label).join(' or ') : '')),
    cards_not_owned: CARDS.filter(c => state.cardsOff[c.id] === false).map(c => c.name),
    deck: state.perUma !== false ? 'find best deck per uma' : 'find best deck (one for the whole loop)',
    game_data: ERA === 'gm' ? 'JP, Grand Masters' : 'Global', scenario: scenarioOf(state.scen).name,
  };
  let results = null;
  if (ui.lastResult) {
    const { r, meta } = ui.lastResult;
    const loopSum = (l) => ({ all_land: fmtIn(l.p_all), at_least_n_minus_1: fmtIn(l.p_minus1),
      umas: l.members.map((m, k) => m.name + (l.outfits[k].outfits.length ? ' ' + l.outfits[k].outfits.map(o => o.label).join('/') : '')) });
    results = { example: !!meta.example, decks: (r.top || []).map(t => ({ ...loopSum(t.loop), deck: t.deck.cards, pals: t.deck.pals, borrowed: t.deck.borrowed,
      ...(t.decks ? { deck_per_uma: Object.fromEntries(t.loop.members.map((m, k) => [m.name, t.decks[k].cards])) } : {}),
      deck_odds: t.deck.odds.filter(o => o.white > 0.005 || o.gold > 0.005).map(o => ({ skill: nmG(o.group, 1), white: +o.white.toFixed(3), gold: +o.gold.toFixed(3) })) })),
      decks_searched: r.decks_built, proven: r.proven };
  }
  return JSON.stringify({ app: 'legacy-loop-planner', version: 1, readable, results, settings: { era: ERA, ...settings } }, null, 1);
}
export function initIO() {
  $('ex-open').onclick = () => { $('ex-text').value = exportText(); $('ex-msg').textContent = ''; $('ex-dlg').showModal(); };
  $('ex-close').onclick = () => $('ex-dlg').close();
  $('ex-dlg').addEventListener('click', (e) => { if (e.target === $('ex-dlg')) $('ex-dlg').close(); });
  $('ex-copy').onclick = () => {
    const t = $('ex-text'); const done = () => { $('ex-msg').textContent = 'Copied.'; };
    const fallback = () => { t.focus(); t.select(); $('ex-msg').textContent = 'Selected. Press Ctrl+C (or ⌘C) to copy.'; };
    try { navigator.clipboard.writeText(t.value).then(done, fallback); } catch { fallback(); }
  };
  $('ex-load').onclick = () => {
    try { const o = JSON.parse($('ex-text').value); if (!o || !o.settings) throw new Error('no settings');
      const era = o.settings.era === 'gm' ? 'gm' : 'global';
      if (era !== ERA) {   // settings for the other data set: store them under its keys, then switch
        for (const k of SETTINGS) if (k in o.settings) (PER_ERA.includes(k) ? SAVED[era] : SAVED.common)[k] = o.settings[k];
        SAVED.era = era; persist();
        location.reload(); return;
      }
      for (const k of SETTINGS) if (k in o.settings) state[k] = o.settings[k];
      save();
      paintRank(); paintTop(); $('c-peruma').checked = state.perUma !== false;
      $('x-auto').checked = state.auto !== false; paintScen();
      renderRoster(); renderTargets(); $('ex-msg').textContent = 'Settings loaded.';
    } catch { $('ex-msg').textContent = 'That text is not an export from this page.'; }
  };
}
