// ---------- search: scheduling, and the engine (a Web Worker running the WebAssembly solver) ----------
// The search runs by itself: any change to a setting schedules one (after a short pause, so a run of clicks is one
// search); a change during a search kills the worker and starts again (a fresh worker is ready in ~60 ms).
import { $, spinHTML } from './dom.js';
import { ERA } from './settings.js';
import { UMAS, ASSETS, EXAMPLE } from './data.js';
import { run, killEngine } from './engine.js';
import { save, onSave } from './save.js';
import { EXTRA, jobTargets, ownedCards, rcardsOff } from './job.js';
import { state, ui, DEFAULTS } from './state.js';
import { noOutfit, reqOutfits, isRequired } from './roster.js';
import { renderDeck } from './results.js';
let searching = false, searchT0 = 0, searchTick = null, pauseTimer = null, searchId = 0;
// 'check': the fast result is on screen and the exact check is running
let phase = 'search';
// The status line heading the results (and its copy in the phone's floating button): while a search runs, its time and
// a Stop button; otherwise ui.status: {kind: 'done' | 'example' | 'error' | 'empty' | 'stopped' | 'loading', text,
// act: {label, click: id of the button it presses} or {label, run: function}}. Whenever the results on screen are not those of the current
// settings (an error, an empty state, a stopped or running search) they stay, dimmed, instead of being wiped.
let actKey = null;
function setAct(key, label, fn) {   // the button beside the status line, rebuilt only when it changes (keeps focus)
  if (key === actKey) return; actKey = key;
  const box = $('r-act'); box.textContent = '';
  if (!key) return;
  const b = document.createElement('button'); b.type = 'button'; b.className = 'btn'; b.textContent = label; b.onclick = fn; box.appendChild(b);
}
export function paintRunNote() {
  const n = $('run-note'), res = $('results'), st = ui.status, shown = !!res.querySelector('.loop');
  n.classList.toggle('busy', searching);
  $('rhead').dataset.kind = searching ? 'busy' : st.kind;
  res.setAttribute('aria-busy', String(searching));
  res.classList.toggle('stale', !searching && ['error', 'empty', 'stopped', 'pending'].includes(st.kind));
  let pill;
  if (searching) {
    const sec = Math.floor((performance.now() - searchT0) / 1000), t = sec >= 1 ? ` ${sec} s` : '';
    // the seconds are hidden from screen readers: the live region would read them out every second
    const what = phase === 'check' ? 'Checking the exact odds…' : 'Searching…';
    n.innerHTML = spinHTML() + '<span>' + what + '<span aria-hidden="true">' + t + '</span>' +
      (phase === 'check' ? (t ? '. ' : ' ') + 'The result below may still change.' : sec >= 1 && shown ? '. The results below are from before your change.' : '') + '</span>';
    setAct('stop', 'Stop', stopSearch); pill = what + t;
  } else {
    n.textContent = st.text;
    if (st.act) setAct(st.act.click || st.act.label, st.act.label, st.act.run || (() => $(st.act.click).click())); else setAct(null);
    pill = st.kind === 'done' || st.kind === 'example' ? (ui.headline ? ui.headline + ' · see result ↑' : st.text) : st.kind === 'loading' ? st.text : 'Not searched · see why ↑';
  }
  const p = $('run-pill'); p.hidden = !n.textContent; p.textContent = pill;
}
// Stop: the running search (or the one about to start) is dropped and its worker killed; the results on screen stay
function stopSearch() {
  if (!searching) return;
  searchId++; clearTimeout(pauseTimer); if (busy) stopEngine();
  const checking = phase === 'check';
  searching = false; phase = 'search'; clearInterval(searchTick);
  ui.status = checking
    ? { kind: 'done', text: 'Stopped the exact check: the result below is ranked by odds with targets treated as independent.', act: { label: 'Search again', run: doSearch } }
    : { kind: 'stopped', text: 'Stopped. The results below are from before your change.', act: { label: 'Search again', run: doSearch } };
  paintRunNote();
}
// auto-search on (default): a change searches after a 0.55 s pause (a change during the pause or the search restarts it);
// off: the results stay, marked out of date, until the Search button
function scheduleSearch() {
  clearTimeout(pauseTimer);
  if (state.auto === false) { if (searching) stopSearch(); ui.status = { kind: 'pending', text: 'Settings changed: press Search.', act: { label: 'Search', run: doSearch } }; paintRunNote(); return; }
  if (busy) { searchId++; stopEngine(); }   // a search for the old settings is pointless: stop it now, not when the next starts
  pauseTimer = setTimeout(doSearch, 550); phase = 'search';
  // "Searching…" during the pause, but the seconds count only once the search runs (doSearch starts the clock)
  clearInterval(searchTick); searching = true; searchT0 = performance.now(); paintRunNote();
}
// every setting change: save it and search again
onSave(scheduleSearch);
export { save };

let busy = false;   // a search is running in the engine
function stopEngine() { killEngine(); busy = false; }

// Settings the search cannot start from: an empty state (nothing to search yet, with the button that fixes it) or an
// error (contradictory settings), said in the status line.
class Blocked extends Error { constructor(kind, text, act) { super(text); this.status = { kind, text, act }; } }
// The search job for the current settings (throws Blocked when they can't be searched). Used by the search and by firstResult.
function buildJob() {
  if (!state.targets.length) throw new Blocked('empty', 'Add a target skill to search.', { label: 'Load the 8/8 example targets', click: 't-preset' });
  const exclude = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).map(u => u.id);
  const exclude_outfits = Object.entries(state.outfits).filter(([, v]) => v === false).map(([k]) => +k);
  // must include per outfit: the uma is required, and her outfits that aren't marked are left out
  for (const u of UMAS) { const r = reqOutfits(u); if (r.length) for (const o of u.outfits) if (!r.includes(o) && !exclude_outfits.includes(o.id)) exclude_outfits.push(o.id); }
  // what the solver would refuse, said in the page's terms
  const req = UMAS.filter(isRequired), bad = req.filter(u => exclude.includes(u.id)), and = (l) => l.length > 1 ? l.slice(0, -1).join(', ') + ' and ' + l.at(-1) : l[0];
  if (bad.length) throw new Blocked('error', `Not searched: ${and(bad.map(u => u.name))} ${bad.length > 1 ? 'are' : 'is'} marked Must include but not owned.`);
  if (req.length > 4) throw new Blocked('error', `Not searched: a loop has at most four umas, and ${req.length} are marked Must include.`);
  const owned = UMAS.length - exclude.length;
  if (owned < 3) throw new Blocked('empty', `Mark at least 3 umas as owned to search (${owned} owned now).`, { label: 'Mark all umas owned', click: 'r-all' });
  const require = UMAS.filter(isRequired).map(u => u.id);
  const targets = jobTargets();
  const job = { cmd: 'deck', targets, star_dist: state.rank.split(',').map(Number), exclude, require, exclude_outfits, extra: EXTRA,
    owned_cards: ownedCards(), borrow: true, top_decks: +state.decksTop, per_uma: state.perUma !== false, scenario: state.scen || 'unity',
    ...rcardsOff() };
  return { job, targets };
}
// The last result per era is kept in localStorage with the job that produced it (plus the solver's and data's
// fingerprints), so a reload with unchanged settings shows it at once instead of the default example, and a fresh
// identical result isn't redrawn.
const LAST = () => 'llp:last:' + ERA;
const jobKey = (job) => JSON.stringify([ASSETS.wasm, ASSETS.engine[ERA === 'gm' ? 'gm' : 'global'], job]);
let shown = { key: null, r: null, example: false };   // the result on screen
function defaultKey() {   // the job of the default settings (the precomputed example's)
  const cur = JSON.stringify(state); Object.assign(state, JSON.parse(JSON.stringify(DEFAULTS)));
  try { return jobKey(buildJob().job); } catch { return null; } finally { Object.assign(state, JSON.parse(cur)); }
}
// What to show before the first search finishes: the remembered result of these exact settings, else the precomputed
// example when the settings are the defaults (Global), else nothing.
export function firstResult() {
  let b; try { b = buildJob(); } catch { return null; }
  const key = jobKey(b.job);
  try { const c = JSON.parse(localStorage.getItem(LAST()) || 'null');
    if (c && c.key === key && c.r && c.r.top && c.r.top.length) { shown = { key, r: JSON.stringify(c.r.top), example: false }; return { r: c.r, ms: c.ms, targets: b.targets, forced: b.job.require }; } } catch {}
  if (ERA === 'global' && key === defaultKey()) { shown = { key, r: null, example: true }; return { r: EXAMPLE, example: true, targets: b.targets }; }
  return null;
}
// What the result is checked against (the solver's `exact`: rho = the cap it is certified under, rho_target the
// cap aimed for; see the model notes on /about): nothing when fully checked, else a short caveat
export function certText(e) {
  if (!e || e.rho >= e.rho_target) return '';
  if (e.rho > 1) return ` · exact check: decks within ${Math.round((1 - 1 / e.rho) * 100)}% of it (ran out of time)`;
  return ' · not checked exactly (ran out of time)';
}
export async function doSearch() {
  const my = ++searchId; clearTimeout(pauseTimer);
  if (busy) stopEngine();   // a search is still running: stop it and start over
  let b;
  try { b = buildJob(); }
  catch (e) { searching = false; clearInterval(searchTick); ui.status = e.status || { kind: 'error', text: e.message }; paintRunNote(); return; }
  searching = true; searchT0 = performance.now(); paintRunNote(); clearInterval(searchTick); searchTick = setInterval(paintRunNote, 1000);
  try {
    const { job, targets } = b;
    const key = jobKey(job);
    // a result is drawn unless the same loops and decks are already on screen
    const draw = (r) => {
      ui.lastRender = () => renderDeck(r, { targets, forced: job.require }); ui.lastResult = { kind: 'deck', r, meta: {} };
      const str = JSON.stringify(r.top);
      if (shown.example || shown.key !== key || shown.r !== str) ui.lastRender();
      shown = { key, r: str, example: false };
    };
    const ok = (r) => !r.error && r.top && r.top.length;
    phase = 'search';
    busy = true;
    const m = await run(job, (mi) => { if (my !== searchId || !ok(mi.r)) return; draw(mi.r); phase = 'check'; paintRunNote(); });
    if (my !== searchId) return;   // superseded: a newer search owns the page
    busy = false; phase = 'search';
    if (!ok(m.r)) ui.status = { kind: 'error', text: m.r.error ? 'Not searched: ' + m.r.error : 'No result: no loop fits these settings.' };
    else {
      draw(m.r);
      ui.status = { kind: 'done', text: `Checked ${m.r.decks_built.toLocaleString()} decks / ${m.r.foursomes.toLocaleString()} loops in ${(m.ms / 1000).toFixed(1)} s` + certText(m.r.exact) };
      try { localStorage.setItem(LAST(), JSON.stringify({ key, ms: m.ms, r: m.r })); } catch {}
    }
  } catch (e) { if (my !== searchId) return; busy = false; ui.status = { kind: 'error', text: e.message }; }
  searching = false; clearInterval(searchTick); paintRunNote();
}
