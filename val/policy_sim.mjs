// Policy simulator: the expected number of careers to the first career that lands every target, for a decision policy
// on a fixed veteran pool, under the solver's model. See val/policy/README.md.
//
//   node val/policy_sim.mjs --pool POOL.json --policy planner,greedy[,repeat] --runs N --out DIR
//        [--first R] [--steps H] [--settle K] [--eps E] [--prune 1] [--progress N] [--seed S] [-j P] [--k 6] [--bin PATH]
//   node val/policy_sim.mjs report DIR [--steps H] [--settle K] [--eps E]   (re-summarise saved runs at a smaller H, other K or E)
//
// One run of a policy. Before career t the policy picks a career from the pool (a solver call); with chance p_t = its
// P(all) that career lands everything and the process ends. That ending is counted analytically, so the simulated path
// continues only through failures and E = sum_t S_t with S_t = prod_{j<t} (1 - p_j) (derived: E[N] = sum_t P(N > t)).
// A failed career's result is drawn from its exact landed-set distribution with the all-landed set removed
// (`outcome_sets`), each landed spark's stars from the job's `star_dist`, her G1 wins from her schedule's win chances
// (per race, at least one win of the rows she runs); the new veteran (the trainee, those sparks and wins, no pinks, the
// career's two parents as her parents) joins the pool.
//
// The path ends at the first of: settled (the same pick and the same pool-best career for K decisions running; K = 0
// never), H decisions, or survival S_t below eps. The tail is S_t / p_t: that career repeated forever (assumed).
// With K = 2 and a small H this is the "frozen" estimate; with K = 0, a large H and a small eps it follows the policy
// until the remaining survival is negligible (the long-horizon estimate).
//
// Pruning (default on): a new veteran dominated by one already in the pool (same character, same two parents, owned,
// at least her stars on every target, every G1 she won) is not added, and the policy is not asked again (the pool is
// unchanged, the solver deterministic). For P(all) this is exact: any career with her as a parent does no better than
// the same career with the dominating veteran (stars and wins only raise inheritance chances; two parents of one
// character cannot pair). The planner's top-K shortlist and her value as a later grandparent are not covered (see the
// README).
//
// Draws are keyed by (seed, run index, career index, purpose, item): two policies on the same run index face the same
// uniforms (common random numbers), so their per-run difference has far less noise than the difference of two means.

import fs from 'fs';
import path from 'path';
import crypto from 'crypto';
import { execFile } from 'child_process';
import { fileURLToPath } from 'url';
import { POLICIES } from './policy/policies.mjs';

const REPO = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
const SOLVER_DIR = path.join(REPO, 'pkg/loopopt');
const DEFAULT_STAR_DIST = [0.5, 0.45, 0.05];
const NUM_ARGS = ['progress', 'runs', 'first', 'steps', 'settle', 'seed', 'j', 'k', 'eps', 'prune'];

// ---- arguments ----
function parseArgs(argv) {
  const o = { policy: 'planner,greedy', runs: 48, first: 0, steps: 10, settle: 2, eps: 0, prune: 1, progress: 0, seed: 1, j: 4, k: undefined,
              bin: path.join(SOLVER_DIR, 'target/release/loopopt'), data: 'data.json', given: new Set() };
  const pos = [];
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (!a.startsWith('-')) { pos.push(a); continue; }
    const key = a.replace(/^-+/, ''), v = argv[++i];
    if (v === undefined) throw new Error(`missing value for ${a}`);
    o[key] = NUM_ARGS.includes(key) ? Number(v) : v; o.given.add(key);
  }
  o.pos = pos;
  return o;
}

// ---- random numbers keyed by (seed, run, career, stream, item) ----
function fmix(h) { h ^= h >>> 16; h = Math.imul(h, 0x85ebca6b); h ^= h >>> 13; h = Math.imul(h, 0xc2b2ae35); h ^= h >>> 16; return h >>> 0; }
export function u01(...keys) {
  let a = 0x9e3779b9, b = 0x7f4a7c15;
  for (const k of keys) { a = fmix((a ^ (k >>> 0)) + 0x632be5ab); b = fmix((b + Math.imul(k >>> 0, 0x27d4eb2f)) ^ a); }
  return (a * 2 ** 21 + (b >>> 11)) / 2 ** 53;   // 53 bits in [0, 1)
}
const STREAM = { set: 1, star: 2, win: 3 };

// ---- solver ----
function makeSolver(o, tmpDir, tag) {
  const f = path.join(tmpDir, `${tag}.json`);   // one file per run: a run's solver calls are sequential
  let calls = 0, seconds = 0;
  const solve = (job) => new Promise((resolve, reject) => {
    fs.writeFileSync(f, JSON.stringify(job));
    const t0 = Date.now();
    execFile(o.bin, [o.data, f], { cwd: SOLVER_DIR, maxBuffer: 1 << 29 }, (err, stdout, stderr) => {
      calls++; seconds += (Date.now() - t0) / 1000;
      if (err) return reject(new Error(`solver failed (${tag}): ${err.message}\n${stderr}`));
      let out; try { out = JSON.parse(stdout); } catch (e) { return reject(new Error(`solver output not JSON (${tag}): ${stdout.slice(0, 300)}`)); }
      if (out.error) return reject(new Error(`solver error (${tag}): ${out.error}`));
      resolve(out);
    });
  });
  solve.stats = () => ({ calls, seconds });
  return solve;
}

// ---- the new veteran ----
const key = (c) => `${c.trainee.id}:${[...c.parents].sort((x, y) => x - y).join(',')}`;
const popcount = (m) => { let c = 0; while (m) { c += m & 1; m >>>= 1; } return c; };
const anc = (l) => ({ chara: l.chara, sparks: l.sparks, pinks: l.pinks ?? [], wins: l.wins ?? [] });
const canon = (a) => JSON.stringify([a.chara, [...a.sparks].map(String).sort(), [...(a.pinks ?? [])].map(String).sort(), [...(a.wins ?? [])].sort((x, y) => x - y)]);
const family = (l) => `${l.chara}|${l.parents.map(canon).sort().join('|')}`;

// Failed landed sets in coupling order: fewest targets first (so a high uniform means many landed for any policy).
function failOrder(n) {
  const full = (1 << n) - 1, v = [];
  for (let m = 0; m < full; m++) v.push(m);
  return v.sort((x, y) => popcount(x) - popcount(y) || x - y);
}

// The veteran a failed career leaves (module notes).
function drawVeteran(pick, legs, ctx, run, t) {
  const { targets, starDist, raceNames, order } = ctx;
  const os = pick.outcome_sets;
  if (!os) throw new Error('the picked career has no outcome_sets (more than 12 targets?)');
  let tot = 0; for (const m of order) tot += os[m];
  let u = u01(ctx.seed, run, t, STREAM.set, 0) * tot, mask = -1, last = -1;
  for (const m of order) { if (os[m] <= 0) continue; last = m; if (u < os[m]) { mask = m; break; } u -= os[m]; }
  if (mask < 0) mask = last;  // rounding at the top end
  const sparks = [], stars = [];
  targets.forEach((tg, i) => {
    if (!(mask >> i & 1)) { stars.push(0); return; }
    const x = u01(ctx.seed, run, t, STREAM.star, i);
    const s = x < starDist[0] ? 1 : x < starDist[0] + starDist[1] ? 2 : 3;
    sparks.push([tg.group, s]); stars.push(s);
  });
  // her chance per race index of at least one win (as the solver's `probs`): the rows she runs, each won with its p
  const miss = new Map();
  for (const r of pick.schedule ?? []) {
    if (r.run === false || r.p == null) continue;
    const ri = raceNames.indexOf(r.name);
    if (ri < 0) throw new Error(`schedule race not in data.json race_names: ${r.name}`);
    miss.set(ri, (miss.get(ri) ?? 1) * (1 - r.p));
  }
  const wins = [...miss].filter(([ri, m]) => u01(ctx.seed, run, t, STREAM.win, ri) < 1 - m).map(([ri]) => ri).sort((a, b) => a - b);
  const [a, b] = pick.parents;
  return { leg: { chara: pick.trainee.id, borrowed: false, sparks, pinks: [], wins, parents: [anc(legs[a]), anc(legs[b])] },
           info: { mask, landed: popcount(mask), stars: stars.join(''), wins: wins.length } };
}

// Dominance index over the pool: per family (character + parents), the owned veterans' target stars and wins.
function makeFrontier(targets, legs) {
  const fam = new Map(), gi = new Map(targets.map((t, i) => [t.group, i]));
  const sig = (l) => { const st = new Array(targets.length).fill(0);
    for (const [g, s] of l.sparks) if (gi.has(g)) st[gi.get(g)] = Math.max(st[gi.get(g)], s);
    return { st, wins: new Set(l.wins ?? []) }; };
  const add = (l) => { if (l.borrowed) return; const k = family(l); if (!fam.has(k)) fam.set(k, []); fam.get(k).push(sig(l)); };
  legs.forEach(add);
  return {
    add,
    dominated: (l) => { const v = sig(l), list = fam.get(family(l)) ?? [];
      return list.some(L => L.st.every((s, i) => s >= v.st[i]) && [...v.wins].every(r => L.wins.has(r))); },
  };
}

// ---- expected careers from a recorded path ----
// A record's `segs`: one per decision the policy made, `len` decisions in a row with that pick (the extra ones are
// repeats after a pruned veteran); the last decision of the last segment is where the run applied its tail. Any H, K
// and eps no looser than the recording's give the same E as a run made with them (draws are keyed by career index).
// Only a run of decisions that pick the pool's best career settles: a pick made to breed a better veteran is never
// costed as if repeated forever (the policy repeats it only while the pool stays as it is).
const stops = (runlen, t, S, H, K, eps, onBest) => ({ settled: K > 0 && runlen >= K && onBest, small: S < eps, horizon: t >= H - 1 });
export function estimate(segs, H, K, eps) {
  let S = 1, E = 0, t = 0, run = 0, prev = null;
  for (let i = 0; i < segs.length; i++) {
    const g = segs[i], same = prev && prev.pick === g.pick && prev.best === g.best;
    for (let j = 0; j < g.len; j++, t++) {
      const st = stops((same ? run : 0) + j + 1, t, S, H, K, eps, g.pick === g.best), end = i === segs.length - 1 && j === g.len - 1;
      if (st.settled || st.small || st.horizon || end) {
        const small = !st.settled && st.small;
        return { E: E + S / g.p, careers: t, settled: st.settled, small, truncated: !st.settled && !small && !st.horizon, tail: S / g.p, final_p: g.p, final: g.pick };
      }
      E += S; S *= 1 - g.p;
    }
    run = (same ? run : 0) + g.len; prev = g;
  }
  throw new Error('empty path');
}

async function simulate(pool, policyName, run, o, ctx) {
  const policy = POLICIES[policyName];
  const job = structuredClone(pool), legs = job.legacies;
  const front = makeFrontier(pool.targets, legs);
  const solve = makeSolver(o, ctx.tmpDir, `${policyName}.${run}`);
  const t0 = Date.now(), segs = [];
  let t = 0, S = 1, E = 0, pruned = 0, runlen = 0, prev = null;
  for (;;) {
    const { pick, best, diag } = await policy(job, { solve, opt: o });
    const seg = { pick: key(pick), trainee: pick.trainee.name, p: pick.p_all, best: key(best), p_best: best.p_all, pool: legs.length, t, len: 0 };
    if (diag) seg.diag = diag;
    segs.push(seg);
    const base = prev && prev.pick === seg.pick && prev.best === seg.best ? runlen : 0;
    for (;;) {
      seg.len++; runlen = base + seg.len;
      const st = stops(runlen, t, S, o.steps, o.settle, o.eps, seg.pick === seg.best);
      if (st.settled || st.small || st.horizon) {
        const s = solve.stats(), check = estimate(segs, o.steps, o.settle, o.eps);
        if (Math.abs(check.E - (E + S / pick.p_all)) > 1e-9 * check.E) throw new Error('estimate disagrees with the simulation');
        return { policy: policyName, run, cfg: ctx.cfgHash, E: E + S / pick.p_all, H: o.steps, settled: st.settled, ended: st.settled ? 'settled' : st.small ? 'eps' : 'H', careers: t, tail: S / pick.p_all,
                 final: seg.pick, final_p: pick.p_all, solver_calls: s.calls, solver_s: s.seconds, pruned, pool: legs.length,
                 wall_s: (Date.now() - t0) / 1000, segs };
      }
      E += S; S *= 1 - pick.p_all;
      const v = drawVeteran(pick, legs, ctx, run, t); t++;
      if (o.prune && front.dominated(v.leg)) { pruned++; continue; }
      legs.push(v.leg); front.add(v.leg); seg.vet = v.info;
      if (o.progress && segs.length % o.progress === 0)
        console.error(`  ${policyName} run ${run}: ${segs.length} decisions, ${t} careers, pool ${legs.length}, S ${S.toFixed(4)}, p ${pick.p_all.toExponential(3)}, ${((Date.now() - t0) / 1000).toFixed(0)} s`);
      break;
    }
    prev = seg;
  }
}

// ---- statistics ----
function tq975(df) {   // Student t 0.975 quantile (Cornish-Fisher expansion; within 0.002 from df = 3)
  const z = 1.959964, z3 = z ** 3, z5 = z ** 5, z7 = z ** 7;
  return z + (z3 + z) / (4 * df) + (5 * z5 + 16 * z3 + 3 * z) / (96 * df ** 2) + (3 * z7 + 19 * z5 + 17 * z3 - 15 * z) / (384 * df ** 3);
}
export function stats(xs) {
  const n = xs.length, mean = xs.reduce((a, b) => a + b, 0) / n;
  const sd = n > 1 ? Math.sqrt(xs.reduce((a, b) => a + (b - mean) ** 2, 0) / (n - 1)) : 0;
  const half = n > 1 ? tq975(n - 1) * sd / Math.sqrt(n) : 0, s = [...xs].sort((a, b) => a - b);
  return { n, mean, sd, ci95: [mean - half, mean + half], half, median: n % 2 ? s[(n - 1) / 2] : (s[n / 2 - 1] + s[n / 2]) / 2, min: s[0], max: s[n - 1] };
}

function readRecords(dir) {
  const recs = {};
  for (const f of fs.readdirSync(dir).filter(f => f.endsWith('.jsonl'))) {
    recs[f.replace(/\.jsonl$/, '')] = fs.readFileSync(path.join(dir, f), 'utf8').split('\n').filter(Boolean).map(l => JSON.parse(l)).sort((a, b) => a.run - b.run);
  }
  return recs;
}

const fmt = (x) => Math.abs(x) >= 100 ? Math.round(x).toLocaleString('en-US') : x.toFixed(1);
export function summarise(dir, H, K, eps) {
  const cfg = JSON.parse(fs.readFileSync(path.join(dir, 'config.json'), 'utf8'));
  H = H ?? cfg.steps; K = K ?? cfg.settle; eps = eps ?? cfg.eps;
  const order = (n) => ['planner', 'greedy'].includes(n) ? ['planner', 'greedy'].indexOf(n) : 2;
  const recs = Object.fromEntries(Object.entries(readRecords(dir)).sort(([a], [b]) => order(a) - order(b) || a.localeCompare(b))), out = { pool: cfg.pool, seed: cfg.seed, steps: H, settle: K, eps, prune: cfg.prune, policies: {}, paired: [] };
  const lines = [`pool ${cfg.pool}  seed ${cfg.seed}  H ${H}  settle ${K}${K ? " (on the best career)" : " (never)"}  eps ${eps}  prune ${cfg.prune}` +
                 `${H !== cfg.steps || K !== cfg.settle || eps !== cfg.eps ? `  (recorded H ${cfg.steps}, settle ${cfg.settle}, eps ${cfg.eps})` : ''}`];
  const perRun = {};
  for (const [name, rs] of Object.entries(recs)) {
    if (name === 'repeat') {
      out.policies.repeat = { exact: true, E: rs[0].E, p: rs[0].final_p, final: rs[0].final };
      lines.push(`repeat   exact ${fmt(rs[0].E)}  (1/p*, p* = ${rs[0].final_p.toExponential(4)}, ${rs[0].final})`);
      continue;
    }
    const ests = rs.map(r => ({ run: r.run, ...estimate(r.segs, H, K, eps) }));
    perRun[name] = new Map(ests.map(e => [e.run, e.E]));
    const st = stats(ests.map(e => e.E)), careers = stats(ests.map(e => e.careers)), tail = stats(ests.map(e => e.tail));
    const unsettled = ests.filter(e => !e.settled && !e.small).length, truncated = ests.filter(e => e.truncated).length;
    const finals = {}; for (const e of ests) finals[e.final] = (finals[e.final] ?? 0) + 1;
    const calls = rs.reduce((a, r) => a + r.solver_calls, 0), secs = rs.reduce((a, r) => a + r.solver_s, 0);
    out.policies[name] = { ...st, unsettled, truncated, careers_mean: careers.mean, careers_max: careers.max, tail_mean: tail.mean,
                           solver_calls: calls, solver_s: secs, pruned: rs.reduce((a, r) => a + r.pruned, 0), finals };
    lines.push(`${name.padEnd(8)} n ${st.n}  mean ${fmt(st.mean)}  sd ${fmt(st.sd)}  95% CI ${fmt(st.ci95[0])} to ${fmt(st.ci95[1])}  median ${fmt(st.median)}  ` +
               `range ${fmt(st.min)} to ${fmt(st.max)}`);
    lines.push(`${''.padEnd(8)} careers simulated ${careers.mean.toFixed(1)} (max ${careers.max}), tail S/p ${fmt(tail.mean)}, ended at H unsettled ${unsettled}` +
               `${truncated ? ` (${truncated} cut short by the recording)` : ''}; solver ${calls} calls, ${fmt(secs / 60)} min; pruned ${out.policies[name].pruned}`);
  }
  const names = Object.keys(perRun);
  for (let i = 0; i < names.length; i++) for (let j = i + 1; j < names.length; j++) {
    const [A, B] = [names[i], names[j]], runs = [...perRun[A].keys()].filter(r => perRun[B].has(r));
    if (runs.length < 2) continue;
    const st = stats(runs.map(r => perRun[A].get(r) - perRun[B].get(r)));
    const lower = runs.filter(r => perRun[A].get(r) < perRun[B].get(r)).length, higher = runs.filter(r => perRun[A].get(r) > perRun[B].get(r)).length;
    out.paired.push({ a: A, b: B, ...st, a_lower: lower, a_higher: higher });
    lines.push(`${A} - ${B}, paired over ${st.n} runs: ${st.mean >= 0 ? '+' : ''}${fmt(st.mean)}  95% CI ${fmt(st.ci95[0])} to ${fmt(st.ci95[1])}  sd ${fmt(st.sd)}  (${A} lower in ${lower}, higher in ${higher})`);
  }
  // sensitivity to H: the same paths cut at smaller H (draws are keyed by career index, so a cut path is the path at that H)
  const hs = [2, 4, 6, 10, 20, 50, 100, 200, 500, 1000, 2000, 5000, 10000].filter(h => h < H && (H < 1e9 || h <= 10000));
  if (H < 1e9) hs.push(H);
  out.by_H = {};
  for (const [name, rs] of Object.entries(recs)) {
    if (name === 'repeat') continue;
    out.by_H[name] = hs.map(h => { const es = rs.map(r => estimate(r.segs, h, K, eps)); const s = stats(es.map(e => e.E));
      return { H: h, mean: s.mean, half: s.half, unsettled: es.filter(e => !e.settled && !e.small).length }; });
    lines.push(`${name.padEnd(8)} by H: ` + out.by_H[name].map(x => `${x.H}: ${fmt(x.mean)} ±${fmt(x.half)} (${x.unsettled} unsettled)`).join(';  '));
  }
  return { out, text: lines.join('\n') };
}

// ---- main ----
async function main() {
  const o = parseArgs(process.argv.slice(2));
  if (o.pos[0] === 'report') {
    const dir = o.pos[1], g = (k) => o.given.has(k) ? o[k] : undefined;
    const { out, text } = summarise(dir, g('steps'), g('settle'), g('eps'));
    console.log(text);
    if (!['steps', 'settle', 'eps'].some(k => o.given.has(k))) fs.writeFileSync(path.join(dir, 'summary.json'), JSON.stringify(out, null, 1));
    return;
  }
  if (!o.pool || !o.out) throw new Error('usage: node val/policy_sim.mjs --pool POOL.json --policy planner,greedy --runs N --out DIR (see the header)');
  o.out = path.resolve(o.out); o.bin = path.resolve(o.bin);   // the solver runs in pkg/loopopt
  const pool = JSON.parse(fs.readFileSync(o.pool, 'utf8'));
  if (pool.mode !== 'career') throw new Error('the pool must be a career-mode job');
  if (pool.targets.length > 12) throw new Error('outcome_sets needs at most 12 targets');
  const names = String(o.policy).split(',');
  for (const n of names) if (!POLICIES[n]) throw new Error(`unknown policy ${n}; known: ${Object.keys(POLICIES).join(', ')}`);
  const data = JSON.parse(fs.readFileSync(path.join(SOLVER_DIR, o.data), 'utf8'));
  fs.mkdirSync(o.out, { recursive: true });
  const tmpDir = path.join(o.out, 'tmp'); fs.mkdirSync(tmpDir, { recursive: true });
  const sha = (b) => crypto.createHash('sha256').update(b).digest('hex').slice(0, 16);
  const cfg = { pool: path.relative(REPO, path.resolve(o.pool)), pool_sha: sha(JSON.stringify(pool)), seed: o.seed, steps: o.steps, settle: o.settle,
                eps: o.eps, prune: o.prune, k: o.k ?? pool.rollout ?? 6, bin_sha: sha(fs.readFileSync(o.bin)) };
  const cfgPath = path.join(o.out, 'config.json');
  if (fs.existsSync(cfgPath)) {
    const old = JSON.parse(fs.readFileSync(cfgPath, 'utf8'));
    for (const f of ['pool_sha', 'seed', 'settle', 'eps', 'prune', 'k']) if (old[f] !== cfg[f]) throw new Error(`${o.out} holds runs with ${f} = ${old[f]}, not ${cfg[f]}; use another --out`);
    if (cfg.steps < old.steps) throw new Error(`${o.out} holds runs with H = ${old.steps}; a smaller H is a report option (report DIR --steps H)`);
    if (cfg.steps > old.steps) {   // drop the runs that ended at the old H; the rest are the same paths at the new H
      console.error(`raising H from ${old.steps} to ${cfg.steps}: runs that ended at H are rerun`);
      for (const f of fs.readdirSync(o.out).filter(f => f.endsWith('.jsonl'))) {
        const keep = fs.readFileSync(path.join(o.out, f), 'utf8').split('\n').filter(Boolean).filter(l => JSON.parse(l).ended !== 'H');
        fs.writeFileSync(path.join(o.out, f), keep.map(l => l + '\n').join(''));
      }
      fs.writeFileSync(cfgPath, JSON.stringify(cfg, null, 1));
    }
    if (old.bin_sha !== cfg.bin_sha) console.error(`note: the solver binary differs from the one that wrote the earlier runs in ${o.out}`);
  } else fs.writeFileSync(cfgPath, JSON.stringify(cfg, null, 1));
  const cfgHash = [cfg.pool_sha, cfg.seed, cfg.settle, cfg.eps, cfg.prune, cfg.k].join(':');   // H apart: see `done`
  const ctx = { seed: o.seed >>> 0, targets: pool.targets, starDist: pool.star_dist ?? DEFAULT_STAR_DIST, raceNames: data.race_names,
                order: failOrder(pool.targets.length), tmpDir, cfgHash };

  const done = (n) => { const f = path.join(o.out, `${n}.jsonl`);
    return new Set(fs.existsSync(f) ? fs.readFileSync(f, 'utf8').split('\n').filter(Boolean).map(l => JSON.parse(l)).filter(r => r.cfg === cfgHash && (r.ended !== 'H' || r.H === o.steps)).map(r => r.run) : []); };
  const tasks = [];
  for (const n of names) {
    const have = done(n);
    if (POLICIES[n].exact) { if (!have.has(0)) tasks.push({ n, run: 0 }); continue; }
    for (let r = o.first; r < o.first + o.runs; r++) if (!have.has(r)) tasks.push({ n, run: r });
  }
  tasks.sort((a, b) => a.run - b.run || names.indexOf(a.n) - names.indexOf(b.n));   // paired runs progress together
  console.error(`${tasks.length} runs to do (${names.join(', ')}), ${o.j} at a time`);
  const t0 = Date.now(); let next = 0, finished = 0;
  const worker = async () => {
    while (next < tasks.length) {
      const { n, run } = tasks[next++];
      let rec;
      if (POLICIES[n].exact) {
        const solve = makeSolver(o, tmpDir, `${n}.${run}`);
        const { pick } = await POLICIES[n](structuredClone(pool), { solve, opt: o });
        rec = { policy: n, run, cfg: cfgHash, E: 1 / pick.p_all, settled: true, careers: 0, final: key(pick), final_p: pick.p_all,
                solver_calls: 1, solver_s: solve.stats().seconds, segs: [] };
      } else rec = await simulate(pool, n, run, o, ctx);
      fs.appendFileSync(path.join(o.out, `${n}.jsonl`), JSON.stringify(rec) + '\n');
      finished++;
      console.error(`[${((Date.now() - t0) / 1000).toFixed(0)} s] ${finished}/${tasks.length} ${n} run ${run}: E ${fmt(rec.E)}, ${rec.careers} careers, ` +
                    `${rec.solver_calls} solver calls (${fmt(rec.solver_s)} s), ended by ${rec.ended ?? 'exact'} on ${rec.final} (p ${rec.final_p.toExponential(3)})`);
    }
  };
  await Promise.all(Array.from({ length: Math.max(1, o.j) }, worker));
  fs.rmSync(tmpDir, { recursive: true, force: true });
  const { out, text } = summarise(o.out);
  fs.writeFileSync(path.join(o.out, 'summary.json'), JSON.stringify(out, null, 1));
  console.log(text);
  console.log(`wall ${((Date.now() - t0) / 60000).toFixed(1)} min`);
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  main().catch(e => { console.error(e.stack ?? e); process.exit(1); });
}
