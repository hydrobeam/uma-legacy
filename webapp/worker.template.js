// Solver worker: the Rust solver compiled to WebAssembly.
// The wasm's calling convention: the page's JSON request is copied into memory the solver allocates (`alloc`), `call`
// returns where its JSON answer starts and `result_len` its length, and the request's memory is freed (`dealloc`).
let solver = null, runId;   // solver: the instance's exports; runId: the job in the solver now (its progress reports name it)
const encoder = new TextEncoder(), decoder = new TextDecoder();
function callRaw(text) {
  const request = encoder.encode(text);
  const at = solver.alloc(request.length);
  new Uint8Array(solver.memory.buffer, at, request.length).set(request);
  const answerAt = solver.call(at, request.length);
  const answerLength = solver.result_len();
  const answer = decoder.decode(new Uint8Array(solver.memory.buffer, answerAt, answerLength));   // a view, no copy: the memory is not shared and nothing runs in between
  solver.dealloc(at, request.length);
  return JSON.parse(answer);
}
const post = (type, id, r, t0) => self.postMessage({ type, id, r, ms: performance.now() - t0 });
self.onmessage = async (e) => {
  const message = e.data;
  try {
    if (message.type === 'init') {
      try {
        // env.clock_ms: the solver's wall clock (the career plan's timings, reported with it); env.progress: the career
        // plan's phase (k of n), posted to the page from inside the call
        const imports = { env: { clock_ms: () => performance.now(), progress: (ph, k, n) => self.postMessage({ type: 'progress', id: runId, ph, k, n }) } };
        if (message.module) solver = (await WebAssembly.instantiate(message.module, imports)).exports;   // compiled once by the page
        else solver = (await WebAssembly.instantiate(message.bytes, imports)).instance.exports;
      } catch {
        throw new Error('This browser refused to run WebAssembly, which the solver needs. Use a current browser with WebAssembly enabled.');
      }
      // the init command with the era's data: its JSON object with "cmd" put first
      const r = callRaw(`{"cmd":"init",${message.data.trim().slice(1)}`);
      if (r.error) throw new Error(r.error);
      self.postMessage({ type: 'ready' });
    } else if (message.type === 'run') {
      const t0 = performance.now(), job = message.job; runId = message.id;
      // a deck search runs twice: first by the product of per-target odds (fast, posted at once), then by exact odds
      if ((job.cmd === 'deck' || job.mode === 'deck') && job.exact !== false) {
        const quick = callRaw(JSON.stringify(Object.assign({}, job, { exact: false })));
        post('interim', message.id, quick, t0);
        if (quick.error) { post('result', message.id, quick, t0); return; }
      }
      post('result', message.id, callRaw(JSON.stringify(job)), t0);
    }
  } catch (err) { self.postMessage({ type: 'error', id: message.type === 'run' ? message.id : undefined, message: String(err?.message || err) }); }
};
