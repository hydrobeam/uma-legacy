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
        // env.clock_ms: the solver's wall clock (the career plan's time budget)
        const imports = { env: { clock_ms: () => performance.now() } };
        if (m.module) ex = (await WebAssembly.instantiate(m.module, imports)).exports;   // compiled once by the page
        else ex = (await WebAssembly.instantiate(m.bytes, imports)).instance.exports;
      } catch (err) {
        throw new Error('This browser refused to run WebAssembly, which the solver needs. Use a current browser with WebAssembly enabled.');
      }
      const r = callRaw('{"cmd":"init",' + m.data.trim().slice(1));
      if (r.error) throw new Error(r.error);
      self.postMessage({ type: 'ready' });
    } else if (m.type === 'run') {
      const t = performance.now();
      // a deck search runs twice: first ranked by per-target odds (fast, posted at once), then by exact odds
      if ((m.job.cmd === 'deck' || m.job.mode === 'deck') && m.job.exact !== false) {
        const r0 = callRaw(JSON.stringify(Object.assign({}, m.job, { exact: false })));
        self.postMessage({ type: 'interim', id: m.id, r: r0, ms: performance.now() - t });
        if (r0.error) { self.postMessage({ type: 'result', id: m.id, r: r0, ms: performance.now() - t }); return; }
      }
      const r = callRaw(JSON.stringify(m.job));
      self.postMessage({ type: 'result', id: m.id, r, ms: performance.now() - t });
    }
  } catch (err) { self.postMessage({ type: 'error', message: String(err && err.message || err) }); }
};
