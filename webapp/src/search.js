// ---------- search: scheduling, and the engine (a Web Worker running the WebAssembly solver) ----------
// The search runs by itself: any change to a setting schedules one (after a short pause, so a run of clicks is one
// search); a change during a search kills the worker and starts again (a fresh worker is ready in ~60 ms).
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { CARDS, UMAS, HINTS, ASSETS, EXAMPLE } from './data.js';
import { state, ui, writeSaved, DEFAULTS } from './state.js';
import { noOutfit, reqOutfits, isRequired } from './roster.js';
import { renderDeck } from './results.js';
let searching = false, searchT0 = 0, searchTick = null, pauseTimer = null, searchId = 0;
export function paintRunNote() {
  const n = $('run-note'); n.classList.toggle('busy', searching);
  if (!searching) { n.textContent = ui.runNote; return; }
  const sec = Math.floor((performance.now() - searchT0) / 1000);
  n.innerHTML = '<span class="spin" aria-hidden="true"></span>' + 'Searching…' + (sec >= 1 ? ` ${sec} s` : '') + (sec >= 2 && ui.lastRender ? ' · the results shown are from before your change' : '');
}
function scheduleSearch() {
  clearTimeout(pauseTimer); pauseTimer = setTimeout(doSearch, 450);
  if (!searching) { searching = true; searchT0 = performance.now(); paintRunNote(); }
}
// every setting change: save it and search again
export const save = () => { scheduleSearch(); writeSaved(); };
// extra job fields from the event options
function eventFields(deck) {
  const r = +state.rand, f = { scenario: state.scen || 'unity' };
  if (r > 0) {
    f.trainee_hints = HINTS.trainee.map(([id, g, p]) => [id, g, p * r]);
    if (deck) f.odds_events = HINTS.deck.map(([c, g, lv, p]) => [c, g, lv, p * r]);
  }
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
      .catch(() => { throw new Error('Could not load the solver data. Open the page through a web server (./build.sh serve), not as a file.'); });
    engineData.catch(() => { engineData = null; });
  }
  return engineData;
}
function compiled() {
  if (!wasmModule) wasmModule = (async () => {
    let bytes;
    try { const r = await fetch(ASSETS.wasm); if (!r.ok) throw 0; bytes = await r.arrayBuffer(); }
    catch { throw new Error('Could not load loopopt.wasm. Open the page through a web server (./build.sh serve), not as a file.'); }
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

// The search job for the current settings (throws when there is no target). Used by the search and by firstResult.
function buildJob() {
  if (!state.targets.length) throw new Error('Add at least one target skill.');
  const exclude = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).map(u => u.id);
  const exclude_outfits = Object.entries(state.outfits).filter(([, v]) => v === false).map(([k]) => +k);
  // must include per outfit: the uma is required, and her outfits that aren't marked are left out
  for (const u of UMAS) { const r = reqOutfits(u); if (r.length) for (const o of u.outfits) if (!r.includes(o) && !exclude_outfits.includes(o.id)) exclude_outfits.push(o.id); }
  const require = UMAS.filter(isRequired).map(u => u.id);
  // Tokai Teio's Head-On (her Triple Crown event; all three races are her career goals) is always counted
  const extra = [[1003, 20190, 1]]; if (state.zephyr) extra.push([1078, 20165, 1]);
  const targets = state.targets.map(t => ({ group: t.group, guaranteed: t.guaranteed || 0 }));
  const offN = CARDS.filter(c => state.cardsOff[c.id] === false).length;
  const owned_cards = offN ? CARDS.filter(c => state.cardsOff[c.id] !== false).map(c => c.id) : [];
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
    if (c && c.key === key) { shown = { key, r: JSON.stringify(c.r), example: false }; return { r: c.r, ms: c.ms, targets: b.targets }; } } catch {}
  if (ERA === 'global' && key === defaultKey()) { shown = { key, r: null, example: true }; return { r: EXAMPLE, example: true, targets: b.targets }; }
  return null;
}
export async function doSearch() {
  const my = ++searchId;
  if (busy) killEngine();   // a search is still running: stop it and start over
  searching = true; searchT0 = performance.now(); paintRunNote(); clearInterval(searchTick); searchTick = setInterval(paintRunNote, 1000);
  const box = $('results');
  try {
    const { job, targets } = buildJob();
    busy = true; const m = await run(job); if (my !== searchId) return;   // superseded: a newer search owns the page
    busy = false; ui.lastRender = () => renderDeck(m.r, { ms: m.ms, targets, rank: state.rank }); ui.lastResult = { kind: 'deck', r: m.r, meta: {} };
    const key = jobKey(job), str = JSON.stringify(m.r);
    if (shown.example || shown.key !== key || shown.r !== str) ui.lastRender();   // an identical result already on screen stays
    shown = { key, r: str, example: false };
    try { localStorage.setItem(LAST(), JSON.stringify({ key, ms: m.ms, r: m.r })); } catch {}
  } catch (e) { if (my !== searchId) return; busy = false; box.innerHTML = ''; const p = document.createElement('p'); p.className = 'err'; p.textContent = e.message; box.appendChild(p); ui.runNote = ''; }
  searching = false; clearInterval(searchTick); paintRunNote();
}
