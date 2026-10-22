// Precompute the example shown before the first search: deck + loop search, Unity Cup targets, everything owned,
// R cards off (the page default; R pals stay).
import fs from 'node:fs';
const DATA = '../pkg/loopopt/data/global/';
const { instance } = await WebAssembly.instantiate(fs.readFileSync('loopopt.opt.wasm'), { env: { clock_ms: () => performance.now(), progress: () => {} } });
const solver = instance.exports;
const encoder = new TextEncoder(), decoder = new TextDecoder();
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
// what the data holds ahead of its start (export/upcoming.py): left out, as the page leaves it out until then
const now = Date.now() / 1000, later = (kind) => readJson('upcoming.json')[kind].filter(([, start]) => start > now).map(([id]) => id);
// the page's defaults (data.js PRESET, job.js EXTRA and jobTargets, state.js DEFAULTS)
const r = call({ cmd: 'deck', targets: [{ group: 20159 }, { group: 20049, guaranteed: 2 }, { group: 20166 }, { group: 20046, guaranteed: 2 }, { group: 20161 }, { group: 21010 }, { group: 20165 }, { group: 20203 }],
  extra: [[1003, 20190, 1], [1078, 20165, 1]], star_dist: [0.5, 0.45, 0.05], borrow: true, top_decks: 1, per_uma: true,
  exclude: later('umas'), exclude_outfits: later('outfits'),
  exclude_cards: [...readJson('web_cards.json').cards.filter(c => c.rarity === 1 && !c.pal).map(c => c.id), ...later('cards')] });
fs.writeFileSync('example.json', JSON.stringify(r)); console.log(r.error || '', r.top.map(t => Math.round(1 / t.loop.p_all)));
