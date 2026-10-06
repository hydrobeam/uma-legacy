// ---------- search: scheduling, and the engine (a Web Worker running the WebAssembly solver) ----------
// The search runs by itself: any change to a setting schedules one (after a short pause, so a run of clicks is one
// search); a change during a search kills the worker and starts again (a fresh worker is ready in ~60 ms).
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { CARDS, UMAS, ASSETS, EXAMPLE, SCEN_GOLDS, loadFail } from './data.js';
import { state, ui, writeSaved, DEFAULTS } from './state.js';
import { noOutfit, reqOutfits, isRequired } from './roster.js';
import { renderDeck } from './results.js';
let searching = false, searchT0 = 0, searchTick = null, pauseTimer = null, searchId = 0;
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
  res.classList.toggle('stale', !searching && ['error', 'empty', 'stopped'].includes(st.kind));
  let pill;
  if (searching) {
    const sec = Math.floor((performance.now() - searchT0) / 1000), t = sec >= 1 ? ` ${sec} s` : '';
    // the seconds are hidden from screen readers: the live region would read them out every second
    n.innerHTML = '<span class="spin" aria-hidden="true"></span><span>Searching…<span aria-hidden="true">' + t + '</span>' +
      (sec >= 1 && shown ? '. The results below are from before your change.' : '') + '</span>';
    setAct('stop', 'Stop', stopSearch); pill = 'Searching…' + t;
  } else {
    n.textContent = st.text;
    if (st.act) setAct(st.act.click || st.act.label, st.act.label, st.act.run || (() => $(st.act.click).click())); else setAct(null);
    pill = st.kind === 'done' || st.kind === 'example' ? (ui.headline ? ui.headline + ' · see result ↑' : st.text) : st.kind === 'loading' ? st.text : 'Not searched · see why ↑';
  }
  const p = $('run-pill'); p.hidden = !n.textContent; p.textContent = pill;
}
// Stop: the running search (or the one about to start) is dropped and its worker killed; the results on screen stay
export function stopSearch() {
  if (!searching) return;
  searchId++; clearTimeout(pauseTimer); if (busy) killEngine();
  searching = false; clearInterval(searchTick);
  ui.status = { kind: 'stopped', text: 'Stopped. The results below are from before your change.', act: { label: 'Search again', run: doSearch } };
  paintRunNote();
}
function scheduleSearch() {
  clearTimeout(pauseTimer); pauseTimer = setTimeout(doSearch, 450);
  if (!searching) { searching = true; searchT0 = performance.now(); paintRunNote(); }
}
// every setting change: save it and search again
export const save = () => { scheduleSearch(); writeSaved(); };
// extra job fields: the scenario, and R cards left out unless asked for
function eventFields(deck) {
  const f = { scenario: state.scen || 'unity' };
  if (deck && !state.rcards) f.exclude_cards = CARDS.filter(c => c.rar === 1 && !c.pal).map(c => c.id);
  return f;
}

// A running wasm call cannot be interrupted, so a superseded search is stopped by terminating its worker.
let worker = null, ready = null, seq = 0, busy = false; const pending = new Map();
// The wasm (ASSETS.wasm, fingerprinted next to the page) is fetched and compiled once here and the compiled module handed to every
// worker, so a restart skips fetching and compiling it. Where a module can't be posted, the worker gets the bytes.
let wasmModule = null, engineGen = 0;   // engineGen: a kill during the compile wait cancels that start
// the solver's data for the active era (its own file, preloaded by the page head)
let engineData = null;
function eraEngineData() {
  if (!engineData) {
    engineData = fetch(ASSETS.engine[ERA === 'gm' ? 'gm' : 'global']).then(r => { if (!r.ok) throw 0; return r.text(); })
      .catch(() => { throw new Error(loadFail('the solver data')); });
    engineData.catch(() => { engineData = null; });
  }
  return engineData;
}
function compiled() {
  if (!wasmModule) wasmModule = (async () => {
    let bytes;
    try { const r = await fetch(ASSETS.wasm); if (!r.ok) throw 0; bytes = await r.arrayBuffer(); }
    catch { throw new Error(loadFail('the solver')); }
    let module = null; try { module = await WebAssembly.compile(bytes); } catch { module = null; }
    return { module, bytes };
  })();
  wasmModule.catch(() => { wasmModule = null; });   // a failed load is retried on the next search
  return wasmModule;
}
function killEngine() { engineGen++; if (worker) worker.terminate(); worker = null; ready = null; pending.clear(); busy = false; }
function startEngine() {
  if (ready) return ready;
  const gen = engineGen;
  ready = Promise.all([compiled(), eraEngineData()]).then(([{ module, bytes }, data]) => new Promise((resolve, reject) => {
    if (gen !== engineGen) { reject(new Error('superseded')); return; }
    try {
      const src = $('engine-worker').textContent;
      worker = new Worker(URL.createObjectURL(new Blob([src], { type: 'text/javascript' })));
    } catch (e) { reject(new Error('This browser blocked the background worker the solver runs in.')); return; }
    worker.onmessage = (e) => {
      const m = e.data;
      if (m.type === 'ready') resolve();
      else if (m.type === 'error') { reject(new Error(m.message)); }
      else if (m.type === 'result') { const p = pending.get(m.id); pending.delete(m.id); p && p(m); }
    };
    worker.onerror = (e) => reject(new Error('The solver failed to start: ' + (e.message || 'unknown error')));
    try { if (!module) throw 0; worker.postMessage({ type: 'init', module, data }); }
    catch { worker.postMessage({ type: 'init', bytes, data }); }   // module not transferable here
  }));
  ready.catch(() => { if (gen === engineGen) ready = null; });   // a failed start is retried by the next search
  return ready;
}
function run(job) { return startEngine().then(() => new Promise(res => { const id = ++seq; pending.set(id, res); worker.postMessage({ type: 'run', id, job }); })); }

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
  if (req.length > 4) throw new Blocked('error', `Not searched: a loop has four umas, and ${req.length} are marked Must include.`);
  const owned = UMAS.length - exclude.length;
  if (owned < 4) throw new Blocked('empty', `Mark at least 4 umas as owned to search (${owned} owned now).`, { label: 'Mark all umas owned', click: 'r-all' });
  const require = UMAS.filter(isRequired).map(u => u.id);
  // Tokai Teio's Head-On (her Triple Crown event; all three races are her career goals) is always counted
  // skills a career always gives an uma outside her outfit and the deck: Yamanin Zephyr's Slipstream comes with a goal
  // race (GameTora's event data lacks it)
  const extra = [[1003, 20190, 1], [1078, 20165, 1]];
  const sg = SCEN_GOLDS[state.scen || 'unity'] || [];   // guaranteed outside the deck: the scenario's golds only
  const targets = state.targets.map(t => ({ group: t.group, guaranteed: sg.includes(t.group) ? 2 : 0 }));
  const offN = CARDS.filter(c => state.cardsOff[c.id] === false).length;
  // the solver reads an empty list as "no restriction", so with every card not owned it gets [0], an id no card has
  const owned_cards = offN ? (offN === CARDS.length ? [0] : CARDS.filter(c => state.cardsOff[c.id] !== false).map(c => c.id)) : [];
  const job = { cmd: 'deck', targets, star_dist: state.rank.split(',').map(Number), exclude, require, exclude_outfits, extra,
    owned_cards, borrow: state.borrow, top_decks: +state.decksTop, per_uma: state.perUma !== false, ...eventFields(true) };
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
    if (c && c.key === key && c.r && c.r.top && c.r.top.length) { shown = { key, r: JSON.stringify(c.r), example: false }; return { r: c.r, ms: c.ms, targets: b.targets }; } } catch {}
  if (ERA === 'global' && key === defaultKey()) { shown = { key, r: null, example: true }; return { r: EXAMPLE, example: true, targets: b.targets }; }
  return null;
}
export async function doSearch() {
  const my = ++searchId; clearTimeout(pauseTimer);
  if (busy) killEngine();   // a search is still running: stop it and start over
  let b;
  try { b = buildJob(); }
  catch (e) { searching = false; clearInterval(searchTick); ui.status = e.status || { kind: 'error', text: e.message }; paintRunNote(); return; }
  searching = true; searchT0 = performance.now(); paintRunNote(); clearInterval(searchTick); searchTick = setInterval(paintRunNote, 1000);
  try {
    const { job, targets } = b;
    busy = true; const m = await run(job); if (my !== searchId) return;   // superseded: a newer search owns the page
    busy = false;
    if (m.r.error || !m.r.top || !m.r.top.length) ui.status = { kind: 'error', text: m.r.error ? 'Not searched: ' + m.r.error : 'No result: no loop fits these settings.' };
    else {
      ui.lastRender = () => renderDeck(m.r, { ms: m.ms, targets, rank: state.rank }); ui.lastResult = { kind: 'deck', r: m.r, meta: {} };
      const key = jobKey(job), str = JSON.stringify(m.r);
      if (shown.example || shown.key !== key || shown.r !== str) ui.lastRender();   // an identical result already on screen stays
      shown = { key, r: str, example: false };
      ui.status = { kind: 'done', text: `Searched ${m.r.decks_built.toLocaleString()} decks and ${m.r.foursomes.toLocaleString()} loops in ${(m.ms / 1000).toFixed(1)} s` };
      try { localStorage.setItem(LAST(), JSON.stringify({ key, ms: m.ms, r: m.r })); } catch {}
    }
  } catch (e) { if (my !== searchId) return; busy = false; ui.status = { kind: 'error', text: e.message }; }
  searching = false; clearInterval(searchTick); paintRunNote();
}
