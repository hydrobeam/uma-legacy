// Times the planner's deck search in Node on the browser's wasm: node webapp/nodebench.mjs WASM ERA JOB... (run from
// webapp/; ERA global or gm; each JOB a val/jobs/<JOB>.json; PU=1 searches a deck per uma). Prints per job its name,
// milliseconds and the top loop's "1 in N".
import fs from 'node:fs';
const [wasmPath, era, ...jobs] = process.argv.slice(2);
const DATA = era === 'gm' ? '../pkg/loopopt/data/jp/' : '../pkg/loopopt/data/global/';
const encoder = new TextEncoder(), decoder = new TextDecoder();
const { instance } = await WebAssembly.instantiate(fs.readFileSync(wasmPath), { env: { clock_ms: () => performance.now(), progress: () => {} } });
const solver = instance.exports;
// one solver call: the request's JSON copied into the solver's memory, its JSON answer read back (worker.template.js)
function call(request) {
  const bytes = encoder.encode(JSON.stringify(request));
  const at = solver.alloc(bytes.length);
  new Uint8Array(solver.memory.buffer, at, bytes.length).set(bytes);
  const answerAt = solver.call(at, bytes.length), answerLength = solver.result_len();
  const answer = decoder.decode(new Uint8Array(solver.memory.buffer, answerAt, answerLength).slice());
  solver.dealloc(at, bytes.length);
  return JSON.parse(answer);
}
const readJson = (file) => JSON.parse(fs.readFileSync(DATA + file));
call({ cmd: 'init', data: readJson('data.json'), skills: readJson('web_skills.json'), cards: readJson('web_cards.json'), events: readJson('card_events.json') });
for (const name of jobs) {
  const job = JSON.parse(fs.readFileSync(`../val/jobs/${name}.json`));
  const t0 = Date.now(); const r = call({ ...job, cmd: 'deck', ...(process.env.PU ? { per_uma: true } : {}) });
  console.log(name, Date.now() - t0, Math.round(1 / r.top[0].loop.p_all));
}
