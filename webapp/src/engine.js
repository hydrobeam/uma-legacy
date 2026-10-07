// ---------- the engine: a Web Worker running the WebAssembly solver (shared by the planner and the career page) ----------
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { ASSETS, loadFail } from './data.js';
// A running wasm call cannot be interrupted, so a superseded search is stopped by terminating its worker.
let worker = null, ready = null, seq = 0; const pending = new Map();
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
export function killEngine() { engineGen++; if (worker) worker.terminate(); worker = null; ready = null; pending.clear(); }
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
      else if (m.type === 'interim') { const p = pending.get(m.id); p && p.interim && p.interim(m); }
      else if (m.type === 'result') { const p = pending.get(m.id); pending.delete(m.id); p && p.done(m); }
    };
    worker.onerror = (e) => reject(new Error('The solver failed to start: ' + (e.message || 'unknown error')));
    try { if (!module) throw 0; worker.postMessage({ type: 'init', module, data }); }
    catch { worker.postMessage({ type: 'init', bytes, data }); }   // module not transferable here
  }));
  ready.catch(() => { if (gen === engineGen) ready = null; });   // a failed start is retried by the next search
  return ready;
}
// interim: called with the fast first pass of a deck search, before the exact one resolves the promise
export function run(job, interim) { return startEngine().then(() => new Promise(res => { const id = ++seq; pending.set(id, { done: res, interim }); worker.postMessage({ type: 'run', id, job }); })); }
