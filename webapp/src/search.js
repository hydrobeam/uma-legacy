// ---------- search: scheduling, and the engine (a Web Worker running the WebAssembly solver) ----------
// The search runs by itself: any change to a setting schedules one (after a short pause, so a run of clicks is one
// search); a change during a search kills the worker and starts again (a fresh worker is ready in ~60 ms).
import { $ } from './dom.js';
import { runnerHTML } from './images.js';
import { ERA } from './settings.js';
import { UMAS, ASSETS, EXAMPLE, LATER } from './data.js';
import { run, restartEngine } from './engine.js';
import { save, onSave } from './save.js';
import { EXTRA, jobTargets, ownedCards, rcardsOff } from './job.js';
import { state, ui, DEFAULTS } from './state.js';
import { noOutfit } from './roster.js';
import { renderDeck } from './results.js';
import { loadExample } from './presets.js';
// The search's state. searching: a search runs or is about to (the status line shows it); searchT0: when it started
// (performance.now); searchTick: the timer redrawing the status line each second; pauseTimer: the pause before an
// automatic search; searchId: the newest search's number (an older one's answer is dropped); busy: a search is running
// in the engine (a kill would cut it short).
let searching = false, searchT0 = 0, searchTick = null, pauseTimer = null, searchId = 0, busy = false;
// 'check': the fast result is on screen and the exact check is running
let phase = 'search';
// the result on screen: its job's key, its loops and decks (JSON), whether it is the precomputed example
let shown = { key: null, r: null, example: false };
// the finished search of the result on show: its job's key and its status line (an automatic search of the same job
// shows that again instead of running)
let doneKey = null, doneStatus = null;
// The status line heading the results (and its copy in the phone's floating button): while a search runs, its time and
// a Stop button; otherwise ui.status (state.js Status). Whenever the results on screen are not those of the current
// settings (an error, an empty state, a stopped or running search) they stay, dimmed, instead of being wiped.
let actKey = null;
function setAct(key, label, onClick) {   // the button beside the status line, rebuilt only when it changes (keeps focus)
  if (key === actKey) return; actKey = key;
  const box = $('r-act'); box.textContent = '';
  if (!key) return;
  const b = document.createElement('button'); b.type = 'button'; b.className = 'btn'; b.textContent = label; b.onclick = onClick; box.appendChild(b);
}
export function paintRunNote() {
  const note = $('run-note'), results = $('results'), status = ui.status, hasResults = !!results.querySelector('.loop');
  note.classList.toggle('busy', searching);
  $('rhead').dataset.kind = searching ? 'busy' : status.kind;
  results.setAttribute('aria-busy', String(searching));
  results.classList.toggle('stale', !searching && ['error', 'empty', 'stopped', 'pending'].includes(status.kind));
  let pill;
  if (searching) {
    const sec = Math.floor((performance.now() - searchT0) / 1000), time = sec >= 1 ? ` ${sec} s` : '';
    // the seconds are hidden from screen readers: the live region would read them out every second
    const what = phase === 'check' ? 'Checking the exact odds…' : 'Searching…';
    // one line beside the spinner: the exact check's caveat is its tooltip (and read to screen readers)
    const caveat = phase === 'check' ? '<span class="vh"> The result below may still change.</span>' : sec >= 1 && hasResults ? '. The results below are from before your change.' : '';
    note.innerHTML = `${runnerHTML()}<span>${what}<span aria-hidden="true">${time}</span>${caveat}</span>`;
    note.title = phase === 'check' ? 'The result below is ranked by quick odds; the exact check can still reorder it' : '';
    setAct('stop', 'Stop', stopSearch); pill = what + time;
  } else {
    note.textContent = status.text; note.title = '';
    if (status.act) setAct(status.act.click || status.act.label, status.act.label, status.act.run || (() => $(status.act.click).click())); else setAct(null);
    const answered = status.kind === 'done' || status.kind === 'example';
    pill = answered ? (ui.headline ? `${ui.headline} · see result ↑` : status.text) : status.kind === 'loading' ? status.text : 'Not searched · see why ↑';
  }
  const pillButton = $('run-pill'); pillButton.hidden = !note.textContent; pillButton.textContent = pill;
}
// the status line after a search ends without running (or is dropped): no timer, the given status
function endSearch(status) { searching = false; clearInterval(searchTick); ui.status = status; paintRunNote(); }
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
const PAUSE_MS = 550;   // the pause after a change before the automatic search
// auto-search on (default): a change searches after a 0.55 s pause (a change during the pause or the search restarts it);
// off: the results stay, marked out of date, until the Search button
function scheduleSearch() {
  clearTimeout(pauseTimer);
  if (state.auto === false) {
    if (searching) stopSearch();
    ui.status = { kind: 'pending', text: 'Settings changed: press Search.', act: { label: 'Search', run: doSearch } }; paintRunNote();
    return;
  }
  if (busy) { searchId++; stopEngine(); }   // a search for the old settings is pointless: stop it now, not when the next starts
  pauseTimer = setTimeout(() => doSearch(true), PAUSE_MS); phase = 'search';
  // "Searching…" during the pause, but the seconds count only once the search runs (doSearch starts the clock)
  clearInterval(searchTick); searching = true; searchT0 = performance.now(); paintRunNote();
}
// every setting change: save it and search again
onSave(scheduleSearch);
export { save };

function stopEngine() { restartEngine(); busy = false; }

// Settings the search cannot start from: an empty state (nothing to search yet, with the button that fixes it) or an
// error (contradictory settings), said in the status line.
class Blocked extends Error { constructor(kind, text, act) { super(text); this.status = { kind, text, act }; } }
// The search job for the current settings (throws Blocked when they can't be searched). Used by the search and by firstResult.
function buildJob() {
  if (!state.targets.length) throw new Blocked('empty', 'Add a target skill to search.', { label: 'Load the 8/8 example targets', click: 't-preset', run: loadExample });
  const exclude = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).map(u => u.id);
  const exclude_outfits = Object.entries(state.outfits).filter(([, owned]) => owned === false).map(([id]) => +id);
  const owned = UMAS.length - exclude.length;
  if (owned < 3) throw new Blocked('empty', `Mark at least 3 umas as owned to search (${owned} owned now).`, { label: 'Mark all umas owned', click: 'r-all' });
  const targets = jobTargets();
  const job = { cmd: 'deck', targets, star_dist: state.rank.split(',').map(Number), exclude, exclude_outfits, extra: EXTRA,
    owned_cards: ownedCards(), borrow: true, top_decks: +state.decksTop, per_uma: state.perUma !== false, scenario: state.scen || 'unity',
    ...rcardsOff() };
  return { job, targets };
}
// The last result per era is kept in localStorage with the job that produced it (plus the solver's and data's
// fingerprints and what is still held back for a later start, data.js LATER), so a reload with unchanged settings shows
// it as done, without searching again, instead of the default example.
const lastResultKey = () => `llp:last:${ERA}`;
const jobKey = (job) => JSON.stringify([ASSETS.wasm, ASSETS.engine[ERA === 'gm' ? 'gm' : 'global'], LATER, job]);
function defaultKey() {   // the job of the default settings (the precomputed example's)
  const current = JSON.stringify(state); Object.assign(state, JSON.parse(JSON.stringify(DEFAULTS)));
  try { return jobKey(buildJob().job); } catch { return null; } finally { Object.assign(state, JSON.parse(current)); }
}
// What to show before the first search finishes: the remembered result of these exact settings, else the precomputed
// example when the settings are the defaults (Global), else nothing.
export function firstResult() {
  let built; try { built = buildJob(); } catch { return null; }
  const key = jobKey(built.job);
  try {
    const kept = JSON.parse(localStorage.getItem(lastResultKey()) || 'null');
    if (kept && kept.key === key && kept.r?.top?.length) {
      shown = { key, r: JSON.stringify(kept.r.top), example: false };
      doneKey = key; doneStatus = doneText(kept.r, kept.ms);   // the search of these settings, finished before: done, nothing to rerun
      return { r: kept.r, ms: kept.ms, targets: built.targets, status: doneStatus };
    }
  } catch {}
  if (ERA === 'global' && key === defaultKey()) { shown = { key, r: null, example: true }; return { r: EXAMPLE, example: true, targets: built.targets }; }
  return null;
}
// What the result is checked against (the solver's `exact`: rho = the cap it is certified under, rho_target the
// cap aimed for; see the model notes on /about): nothing when fully checked, else a short caveat
/** @param {{rho: number, rho_target: number}|null|undefined} exact */
export function certText(exact) {
  if (!exact || exact.rho >= exact.rho_target) return '';
  if (exact.rho > 1) return ` · exact check: decks within ${Math.round((1 - 1 / exact.rho) * 100)}% of it (ran out of time)`;
  return ' · not checked exactly (ran out of time)';
}
/** @type {(r: import('./results.js').LoopReport, ms: number) => import('./state.js').Status} */
const doneText = (r, ms) => ({ kind: 'done', text: `Checked ${r.decks_built.toLocaleString()} decks / ${r.foursomes.toLocaleString()} loops in ${(ms / 1000).toFixed(1)} s${certText(r.exact)}` });
// auto: the search the pause starts (not the Search buttons): edits that end where they started run nothing when the
// result on show is the finished search of the same job (its key holds the solver's and data's fingerprints, the era and
// every setting the job carries)
export async function doSearch(auto = false) {
  const thisSearch = ++searchId; clearTimeout(pauseTimer);
  let built;
  try { built = buildJob(); }
  catch (e) { if (busy) stopEngine(); endSearch(e.status || { kind: 'error', text: e.message }); return; }
  if (auto === true && !busy && doneKey && jobKey(built.job) === doneKey && shown.key === doneKey && !shown.example) { endSearch(doneStatus); return; }
  if (busy) stopEngine();   // a search is still running: stop it and start over
  searching = true; searchT0 = performance.now(); paintRunNote(); clearInterval(searchTick); searchTick = setInterval(paintRunNote, 1000);
  try {
    const { job, targets } = built;
    const key = jobKey(job);
    // a result is drawn unless the same loops and decks are already on screen
    const draw = (r) => {
      ui.lastRender = () => renderDeck(r, { targets }); ui.lastResult = { kind: 'deck', r, meta: {} };
      const loops = JSON.stringify(r.top);
      if (shown.example || shown.key !== key || shown.r !== loops) ui.lastRender();
      shown = { key, r: loops, example: false };
    };
    const hasLoops = (r) => !r.error && r.top && r.top.length;
    phase = 'search';
    busy = true;
    const answer = await run(job, (interim) => { if (thisSearch !== searchId || !hasLoops(interim.r)) return; draw(interim.r); phase = 'check'; paintRunNote(); });
    if (thisSearch !== searchId) return;   // superseded: a newer search owns the page
    busy = false; phase = 'search';
    if (!hasLoops(answer.r)) ui.status = { kind: 'error', text: answer.r.error ? `Not searched: ${answer.r.error}` : 'No result: no loop fits these settings.' };
    else {
      draw(answer.r);
      ui.status = doneText(answer.r, answer.ms);
      doneKey = key; doneStatus = ui.status;
      try { localStorage.setItem(lastResultKey(), JSON.stringify({ key, ms: answer.ms, r: answer.r })); } catch {}
    }
  } catch (e) { if (thisSearch !== searchId) return; busy = false; ui.status = { kind: 'error', text: e.message }; }
  searching = false; clearInterval(searchTick); paintRunNote();
}
