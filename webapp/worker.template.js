// Solver worker: WebAssembly first; if the page's security policy refuses to compile it, a pure-JS build of the same code.
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
      let engine = 'wasm';
      try {
        const bin = atob(m.wasm); const bytes = new Uint8Array(bin.length);
        for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
        const { instance } = await WebAssembly.instantiate(bytes, {});
        ex = instance.exports;
      } catch (err) { engine = 'js'; ex = loadFallback(); }
      const r = callRaw('{"cmd":"init",' + m.data.trim().slice(1));
      if (r.error) throw new Error(r.error);
      self.postMessage({ type: 'ready', engine });
    } else if (m.type === 'run') {
      const t = performance.now();
      const r = callRaw(JSON.stringify(m.job));
      self.postMessage({ type: 'result', id: m.id, r, ms: performance.now() - t });
    }
  } catch (err) { self.postMessage({ type: 'error', message: String(err && err.message || err) }); }
};
function loadFallback() {
/*FALLBACK*/
  return retasmFunc;
}
