// Times the planner's deck search in Node on the browser's wasm: node webapp/nodebench.mjs WASM ERA JOB... (run from
// any folder: WASM is relative to the current folder, the data and jobs are found from this script's own place; ERA
// global or gm; each JOB a val/jobs/<JOB>.json; PU=1 searches a deck per uma). Prints per job its name, milliseconds
// and the top loop's "1 in N".
import fs from 'node:fs';
import { initSync, Solver } from './src/gen/loopopt.js';
const [wasmPath, era, ...jobs] = process.argv.slice(2);
const REPO = new URL('../', import.meta.url); // this script lives in webapp/
const DATA = new URL(era === 'gm' ? 'pkg/loopopt/data/jp/' : 'pkg/loopopt/data/global/', REPO);
// the solver through its glue (src/worker.ts): no progress to show, the benchmark times the whole job
globalThis.solverProgress = () => {
  // nothing to show
};
initSync({ module: fs.readFileSync(wasmPath) });
const readJson = (file) => JSON.parse(fs.readFileSync(new URL(file, DATA)));
const solver = new Solver(
  JSON.stringify({
    data: readJson('data.json'),
    skills: readJson('web_skills.json'),
    cards: readJson('web_cards.json'),
    events: readJson('card_events.json'),
  }),
);
// one solver call: the job's JSON in, its reply's back
const call = (job) => JSON.parse(solver.call(JSON.stringify(job)));
for (const name of jobs) {
  // (a job file holds the job beside the data files it names, which are the era's: the CLI's JobFile)
  const { job } = JSON.parse(fs.readFileSync(new URL(`val/jobs/${name}.json`, REPO)));
  const t0 = Date.now();
  const r = call(process.env.PU ? { ...job, deck: { ...job.deck, per_uma: true } } : job);
  console.log(name, Date.now() - t0, Math.round(1 / r.top[0].loop.p_all));
}
