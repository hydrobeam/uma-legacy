// ---------- search: scheduling, and the engine (a Web Worker running the WebAssembly solver) ----------
// The search runs by itself: any change to a setting schedules one (after a short pause, so a run of clicks is one
// search); a change during a search kills the worker and starts again (a fresh worker is ready in ~60 ms).
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { CARDS, UMAS, HINTS } from './data.js';
import { state, ui, writeSaved } from './state.js';
import { noOutfit } from './roster.js';
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
  const r = +state.rand, f = {};
  if (r > 0) {
    f.trainee_hints = HINTS.trainee.map(([id, g, p]) => [id, g, p * r]);
    if (deck) f.odds_events = HINTS.deck.map(([c, g, lv, p]) => [c, g, lv, p * r]);
  }
  if (deck && !state.rcards) f.exclude_cards = CARDS.filter(c => c.rar === 1 && !c.pal).map(c => c.id);
  return f;
}

// A running wasm call cannot be interrupted, so a superseded search is stopped by terminating its worker.
let worker = null, ready = null, seq = 0, busy = false; const pending = new Map();
function killEngine() { if (worker) worker.terminate(); worker = null; ready = null; pending.clear(); busy = false; }
function startEngine() {
  if (ready) return ready;
  ready = new Promise((resolve, reject) => {
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
    worker.postMessage({ type: 'init', wasm: $('engine-wasm').textContent.trim(), data: $(ERA === 'gm' ? 'engine-data-gm' : 'engine-data').textContent });
  });
  return ready;
}
function run(job) { return startEngine().then(() => new Promise(res => { const id = ++seq; pending.set(id, res); worker.postMessage({ type: 'run', id, job }); })); }

export async function doSearch() {
  const my = ++searchId;
  if (busy) killEngine();   // a search is still running: stop it and start over
  searching = true; searchT0 = performance.now(); paintRunNote(); clearInterval(searchTick); searchTick = setInterval(paintRunNote, 1000);
  const box = $('results');
  const exclude = UMAS.filter(u => state.roster[u.id] === 'out' || noOutfit(u)).map(u => u.id);
  const exclude_outfits = Object.entries(state.outfits).filter(([, v]) => v === false).map(([k]) => +k);
  const require = Object.entries(state.roster).filter(([, v]) => v === 'req').map(([k]) => +k);
  try {
    if (!state.targets.length) throw new Error('Add at least one target skill.');
    const extra = []; if (state.teio) extra.push([1003, 20190, 1]); if (state.zephyr) extra.push([1078, 20165, 1]);
    const targets = state.targets.map(t => ({ group: t.group, guaranteed: t.guaranteed || 0 }));
    const offN = CARDS.filter(c => state.cardsOff[c.id] === false).length;
    const owned_cards = offN ? CARDS.filter(c => state.cardsOff[c.id] !== false).map(c => c.id) : [];
    const job = { cmd: 'deck', targets, star_dist: state.rank.split(',').map(Number), exclude, require, exclude_outfits, extra,
      owned_cards, borrow: state.borrow, top_decks: +state.decksTop, per_uma: state.perUma !== false, ...eventFields(true) };
    busy = true; const m = await run(job); if (my !== searchId) return;   // superseded: a newer search owns the page
    busy = false; ui.lastRender = () => renderDeck(m.r, { ms: m.ms, targets, rank: state.rank }); ui.lastRender(); ui.lastResult = { kind: 'deck', r: m.r, meta: {} };
  } catch (e) { if (my !== searchId) return; busy = false; box.innerHTML = ''; const p = document.createElement('p'); p.className = 'err'; p.textContent = e.message; box.appendChild(p); ui.runNote = ''; }
  searching = false; clearInterval(searchTick); paintRunNote();
}
