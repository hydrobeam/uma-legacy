// Solver worker: the Rust solver compiled to WebAssembly.
let ex = null;
const enc = new TextEncoder(), dec = new TextDecoder();
function callRaw(text) {
  const b = enc.encode(text);
  const p = ex.alloc(b.length);
  new Uint8Array(ex.memory.buffer, p, b.length).set(b);
  const r = ex.call(p, b.length);
  const n = ex.result_len();
  const out = dec.decode(new Uint8Array(ex.memory.buffer, r, n).slice());
  ex.dealloc(p, b.length);
  return JSON.parse(out);
}
self.onmessage = async (e) => {
  const m = e.data;
  try {
    if (m.type === 'init') {
      try {
        if (m.module) ex = (await WebAssembly.instantiate(m.module, {})).exports;   // compiled once by the page
        else {
          const bin = atob(m.wasm); const bytes = new Uint8Array(bin.length);
          for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
          ex = (await WebAssembly.instantiate(bytes, {})).instance.exports;
        }
      } catch (err) {
        throw new Error('This browser or the site hosting the page refused to run WebAssembly, which the solver needs. Open the page file directly in a current browser.');
      }
      const r = callRaw('{"cmd":"init",' + m.data.trim().slice(1));
      if (r.error) throw new Error(r.error);
      self.postMessage({ type: 'ready' });
    } else if (m.type === 'run') {
      const t = performance.now();
      const r = callRaw(JSON.stringify(m.job));
      self.postMessage({ type: 'result', id: m.id, r, ms: performance.now() - t });
    }
  } catch (err) { self.postMessage({ type: 'error', message: String(err && err.message || err) }); }
};
