// ---------- the engine: a Web Worker running the WebAssembly solver (shared by the planner and the career page) ----------
import { $ } from './dom.js';
import { ERA } from './settings.js';
import { ASSETS, loadFail } from './data.js';
import { stateLoad, stateSave, stateDrop } from './statestore.js';
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
// whether a job has been posted and not answered yet (a kill would cut it short; with none, the worker is idle)
export const engineBusy = () => pending.size > 0;
export function killEngine() { engineGen++; if (worker) worker.terminate(); worker = null; ready = null; pending.clear(); }
function startEngine() {
  if (ready) return ready;
  const gen = engineGen;
  ready = Promise.all([compiled(), eraEngineData()]).then(([{ module, bytes }, data]) => new Promise((resolve, reject) => {
    if (gen !== engineGen) { reject(new Error('superseded')); return; }
    // the worker's script as a blob URL, released once the worker has loaded it (one per worker start otherwise stays)
    let url = null;
    try {
      const src = $('engine-worker').textContent;
      url = URL.createObjectURL(new Blob([src], { type: 'text/javascript' }));
      worker = new Worker(url);
    } catch (e) { if (url) URL.revokeObjectURL(url); reject(new Error('This browser blocked the background worker the solver runs in.')); return; }
    worker.onmessage = (e) => {
      const m = e.data;
      if (m.type === 'ready') { URL.revokeObjectURL(url); resolve(); }
      else if (m.type === 'error') { reject(new Error(m.message)); }
      else if (m.type === 'interim') { const p = pending.get(m.id); p && p.interim && p.interim(m); }
      else if (m.type === 'result') { const p = pending.get(m.id); pending.delete(m.id); p && p.done(m); }
    };
    worker.onerror = (e) => reject(new Error('The solver failed to start: ' + (e.message || 'unknown error')));
    try { if (!module) throw 0; worker.postMessage({ type: 'init', module, data }); }
    catch { worker.postMessage({ type: 'init', bytes, data }); }   // module not transferable here
  }));
  ready = ready.then(() => restore().catch(() => {}));   // the last session's kept work, before the first job
  ready.catch(() => { if (gen === engineGen) ready = null; });   // a failed start is retried by the next search
  return ready;
}
// The career planner's kept work (each uma's best career per pool, the proofs) survives a page reload: a new worker gets
// what the last one held before its first job (`state_in`), and the page stores it after each plan (`state_out`, in the
// worker once it is idle). Only the page that asks for it (`enginePersist`) does this; its key is the solver's and the
// era data's fingerprints, so a release starts afresh. A stored state the solver refuses is dropped.
let persistOn = false, saveTimer = 0;
const stateKey = () => ASSETS.wasm + '|' + ASSETS.engine[ERA === 'gm' ? 'gm' : 'global'];
export function enginePersist() { persistOn = true; }
function post(job) { return new Promise(res => { const id = ++seq; pending.set(id, { done: res }); worker.postMessage({ type: 'run', id, job }); }); }
async function restore() {
  if (!persistOn) return;
  const key = stateKey(), st = await stateLoad(key);
  if (!st || !worker) return;
  const m = await post({ cmd: 'state_in', state: st });
  if (!m.r || m.r.error) stateDrop(key);
}
// after a plan: store what the worker holds, once it is idle (a job in flight comes first; a killed worker saves nothing)
export function engineSaveState() {
  if (!persistOn) return;
  clearTimeout(saveTimer);
  saveTimer = setTimeout(async () => {
    if (!worker || !ready || pending.size) { engineSaveState(); return; }
    try { await ready; const m = await post({ cmd: 'state_out' }); if (m.r && m.r.state) stateSave(stateKey(), m.r.state); } catch { /* the worker went: nothing to save */ }
  }, 1500);
}
// interim: called with the fast first pass of a deck search, before the exact one resolves the promise
export function run(job, interim) { return startEngine().then(() => new Promise(res => { const id = ++seq; pending.set(id, { done: res, interim }); worker.postMessage({ type: 'run', id, job }); })); }
