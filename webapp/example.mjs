// Precompute the example shown before the first search: deck + loop search, Unity Cup targets, everything owned,
// R cards off (the page default; R pals stay).
import fs from 'node:fs';
import { initSync, Solver } from './src/gen/loopopt.js';
const DATA = '../pkg/loopopt/data/global/';
// the solver through its glue (src/worker.ts): no progress to show, the example is computed offline
globalThis.solverProgress = () => {
  // nothing to show
};
initSync({ module: fs.readFileSync('loopopt.opt.wasm') });
const readJson = (file) => JSON.parse(fs.readFileSync(DATA + file));
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
// what the data holds ahead of its start (export/upcoming.py: dated after the export): left out, as the page leaves it
// out until then. Taken as of the export rather than today, so the example changes only with the data or the solver.
const later = (kind) => readJson('upcoming.json')[kind].map(([id]) => id);
const cards = readJson('web_cards.json').cards;
// the page's defaults (settings.ts PRESET and DEFAULTS, job.ts EXTRA, jobTargets, skillsPart and deckPart)
const r = call({
  cmd: 'deck',
  skills: {
    targets: [
      { group: 20159 },
      { group: 20049, guaranteed: 2 },
      { group: 20166 },
      { group: 20046, guaranteed: 2 },
      { group: 20161 },
      { group: 21010 },
      { group: 20165 },
      { group: 20203 },
    ],
    rank_class: 'below_ss',
    exclude: later('umas'),
    exclude_outfits: later('outfits'),
    extra: [
      [1003, 20190, 1],
      [1078, 20165, 1],
    ],
  },
  deck: {
    owned_cards: cards.map((c) => c.id),
    borrow: true,
    top_decks: 1,
    per_uma: true,
    exclude_cards: [...cards.filter((c) => c.rarity === 1 && !c.pal).map((c) => c.id), ...later('cards')],
  },
});
fs.writeFileSync('example.json', JSON.stringify(r));
console.log(
  r.kind === 'refusal' ? r.error : '',
  r.top.map((t) => Math.round(1 / t.loop.p_all)),
);
