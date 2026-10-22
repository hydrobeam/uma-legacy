//! Simulation paths: initial pools, veteran admission and pruning, survival accumulation and run summaries.
//! Policies choose the careers; sibling draw and audit modules supply samples and exact comparisons.

use crate::career::cache::Cache;
use crate::career::pool::Pool;
use crate::career::sim::audit::{AuditKind, RankedCareer};
use crate::career::sim::draw::Keys;
use crate::career::sim::policies::{IX, IndexRunCounts, Rotation};
use crate::career::sim::{
    Cur, KeptState, Sim, SimJob, SimulationOptions, SimulationRoster, SurrogatePick, family, kept_swap,
};
use crate::career::{Env, JobInputs, Legacy, Lineage};
use serde_json::{Value, json};
use std::rc::Rc;

/// A veteran's stars per job target and her distinct G1 wins, sorted: what dominance pruning compares.
#[derive(Clone)]
struct VeteranResult {
    stars: Vec<u8>,
    wins: Vec<usize>,
}
/// One path's state: veterans, the solver's pool, owned veterans per family (dominance), the surrogate's best
#[derive(Clone)]
pub(super) struct PathState {
    pub(super) legacies: Vec<Legacy>,
    pub(super) pool: Pool,
    /// Per family (`family`), the results of the owned veterans in it.
    families: crate::FxMap<String, Vec<VeteranResult>>,
    pub(super) best: Option<SurrogatePick>,
    /// The career the policy runs next, kept until the state changes.
    pub(super) current: Option<Rc<Cur>>,
    pub(super) rotation: Rotation,
    /// `trace`: each loop member's best career (by member index)
    pub(super) member_best: Vec<Option<SurrogatePick>>,
}

#[derive(Clone, Copy, PartialEq)]
enum Base {
    Greedy,
    Loop,
    Trace,
    Index,
}

fn tq975(df: f64) -> f64 {
    let z = 1.959_964_f64;
    let (z3, z5, z7) = (z.powi(3), z.powi(5), z.powi(7));
    z + (z3 + z) / (4.0 * df)
        + (5.0 * z5 + 16.0 * z3 + 3.0 * z) / (96.0 * df * df)
        + (3.0 * z7 + 19.0 * z5 + 17.0 * z3 - 15.0 * z) / (384.0 * df.powi(3))
}
fn stats(xs: &[f64]) -> Value {
    let n = xs.len() as f64;
    if xs.is_empty() {
        return Value::Null;
    }
    let mean = xs.iter().sum::<f64>() / n;
    let sd = if xs.len() > 1 { (xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (n - 1.0)).sqrt() } else { 0.0 };
    let half = if xs.len() > 1 { tq975(n - 1.0) * sd / n.sqrt() } else { 0.0 };
    json!({"n": xs.len(), "mean": mean, "sd": sd, "ci95": [mean - half, mean + half], "min": xs.iter().copied().fold(f64::INFINITY, f64::min), "max": xs.iter().copied().fold(f64::NEG_INFINITY, f64::max)})
}

impl Sim<'_, '_> {
    fn veteran_result(&self, l: &Legacy) -> VeteranResult {
        let stars = self
            .env
            .skill_job
            .targets
            .iter()
            .map(|t| l.sparks.iter().filter(|s| s.0 == t.group).map(|s| s.1).max().unwrap_or(0))
            .collect();
        let mut wins = l.wins.clone();
        wins.sort_unstable();
        wins.dedup();
        VeteranResult { stars, wins }
    }
    /// whether an owned veteran of her family has at least her stars on every target and every G1 she won
    fn dominated(&self, st: &PathState, l: &Legacy) -> bool {
        let new = self.veteran_result(l);
        st.families.get(&family(l)).is_some_and(|owned| {
            owned.iter().any(|old| {
                old.stars.iter().zip(&new.stars).all(|(x, y)| x >= y)
                    && new.wins.iter().all(|r| old.wins.binary_search(r).is_ok())
            })
        })
    }
    /// a veteran joins the pool; `eval`: the surrogate values every new candidate (she is one of its parents)
    pub(super) fn add_vet(&mut self, st: &mut PathState, l: Legacy, eval: bool, buf: &mut Vec<f64>) {
        // (her entry as `Env::pool` builds it, win chances and star mix kept)
        self.env.pool_push(&mut st.pool, &l);
        if !l.borrowed {
            let result = self.veteran_result(&l);
            st.families.entry(family(&l)).or_default().push(result);
        }
        st.legacies.push(l);
        self.n.veterans_added += 1;
        if !eval {
            return;
        }
        let vi = st.legacies.len() - 1;
        let trainees = self.trainees.clone();
        for &t in &trainees {
            if self.static_ceilings[t] * (1.0 + 1e-9) < self.bar(st, t) {
                continue;
            }
            for b in 0..vi {
                let lineage = Lineage { trainee: t, first_parent: b, second_parent: vi };
                if !self.legal(st, lineage) {
                    continue;
                }
                // the allocation-free ceiling first (`fast_ub`, the same bound to rounding)
                if self.fast_ub(st, lineage) * (1.0 + 1e-6) < self.bar(st, t) {
                    self.n.candidates += 1;
                    continue;
                }
                self.consider(st, lineage, buf);
            }
        }
    }
    /// the initial state: the job's pool, every candidate valued (best ceiling first)
    fn initial(&mut self, legs: &[Legacy], base: Base) -> PathState {
        let mut st = PathState {
            legacies: vec![],
            pool: self.env.pool(&[]),
            families: std::collections::HashMap::default(),
            best: None,
            current: None,
            rotation: Rotation { position: 0, latest: vec![None; self.members.len()] },
            member_best: vec![None; self.members.len()],
        };
        let mut buf = vec![];
        for l in legs {
            self.add_vet(&mut st, l.clone(), false, &mut buf);
        }
        if !matches!(base, Base::Loop | Base::Index) {
            // seed the cache with the career search's fully checked decks
            self.env.use_seen = false;
            let trainees = self.trainees.clone();
            // (on the path's pool, `Env::pool(legs)`: `add_vet` builds each entry as it does)
            let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
            self.env.use_seen = true;
            self.cache.absorb(self.env, &sv.refs);
            // (every legal career's ceiling, `gen_cands`: `ceiling` at `lineage`'s chances, to the bit; in (trainee,
            // pair) order, then best first)
            let mut candidates = self.env.gen_cands(&st.pool, &[], &trainees, |_, _| true, RankedCareer::new);
            candidates.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
            for candidate in candidates {
                let floor = st
                    .member_best
                    .iter()
                    .fold(st.best.map_or(0.0, |p| p.value), |m, p| m.min(p.map_or(0.0, |p| p.value)));
                if candidate.ceiling * (1.0 + 1e-9) < floor {
                    break;
                }
                if candidate.ceiling * (1.0 + 1e-9) < self.bar(&st, candidate.lineage.trainee) {
                    continue;
                }
                self.consider(&mut st, candidate.lineage, &mut buf);
            }
        }
        st
    }
    /// Follow `options.base` from state st (survival 1, career 0) until the survival S < eps or max_careers, recording
    /// pick changes, checks and audits into `trace`.
    fn play(&mut self, st: &mut PathState, options: PlayOptions, trace: &mut PathTrace) -> PathResult {
        let PlayOptions { base, keys, eps, run } = options;
        let (mut s, mut t) = (1.0f64, 0usize);
        let mut e = 0.0f64;
        let mut pruned = 0usize;
        let mut buf = vec![];
        let mut last_key = String::new();
        let exact = self.cfg.policy == "greedy_exact";
        // Exact and index policies keep their decision until the pool changes.
        let mut kept_decision: Option<Rc<Cur>> = None;
        let mut since_check = 0usize;
        if self.cfg.check > 0 && base == Base::Greedy {
            trace.checks.push(self.check(st, true));
        }
        loop {
            let cur: Rc<Cur> = if exact {
                if let Some(career) = &kept_decision {
                    Rc::clone(career)
                } else {
                    let career = Rc::new(self.exact_cur(st));
                    kept_decision = Some(Rc::clone(&career));
                    career
                }
            } else {
                match base {
                    Base::Greedy => self.greedy_cur(st),
                    Base::Loop => self.loop_cur(st),
                    Base::Trace => unreachable!("trace runs its own loop"),
                    Base::Index => {
                        if let Some(career) = &kept_decision {
                            Rc::clone(career)
                        } else {
                            let career = Rc::new(self.index_cur(st));
                            kept_decision = Some(Rc::clone(&career));
                            career
                        }
                    }
                }
            };
            let k = self.key(&cur);
            if k != last_key {
                trace.pick_changes.push(json!({"t": t, "pick": k, "trainee": self.d.umas[cur.lineage.trainee].name, "p": cur.p_all, "pool": st.legacies.len(), "s": s}));
                last_key = k;
                if self.cfg.audit && base == Base::Greedy && !exact {
                    let a = self.audit_greedy(st, keys, t);
                    self.audits.push(a);
                }
            }
            if self.cfg.audit && base == Base::Loop && t.is_multiple_of(self.cfg.audit_every) {
                let r = self.audit_row(&st.pool, cur.lineage, AuditKind::Loop);
                self.audits.push(json!({"t": t, "pool": st.legacies.len(), "rows": [r]}));
            }
            if self.cfg.progress > 0 && t.is_multiple_of(self.cfg.progress) && t > 0 {
                eprintln!(
                    "  {} run {}: {} careers, pool {}, S {:.4}, p {:.3e}",
                    self.cfg.policy,
                    run,
                    t,
                    st.legacies.len(),
                    s,
                    cur.p_all
                );
            }
            if s < eps || t >= self.cfg.max_careers {
                let tail = s / cur.p_all;
                return PathResult { expected_careers: e + tail, careers: t, tail, final_p_all: cur.p_all, pruned };
            }
            e += s;
            s *= 1.0 - cur.p_all;
            let vet = self.draw(&cur, &st.legacies, keys, t);
            t += 1;
            match base {
                Base::Loop => {
                    self.add_vet(st, vet, false, &mut buf);
                    if cur.rotation {
                        self.rot_update(st);
                    }
                    st.current = None;
                    kept_decision = None;
                }
                Base::Trace => unreachable!("trace runs its own loop"),
                // (index: a pruned veteran leaves the pool as it was, so the plan, and its pick, are kept)
                Base::Greedy | Base::Index => {
                    if self.cfg.prune && self.dominated(st, &vet) {
                        pruned += 1;
                        continue;
                    }
                    self.add_vet(st, vet, !exact && base == Base::Greedy, &mut buf);
                    kept_decision = None;
                    since_check += 1;
                    if base == Base::Greedy && self.cfg.check > 0 && since_check >= self.cfg.check {
                        since_check = 0;
                        trace.checks.push(self.check(st, false));
                    }
                }
            }
        }
    }
}

/// How a run's path is followed (`Sim::play`).
#[derive(Clone, Copy)]
struct PlayOptions {
    base: Base,
    keys: Keys,
    /// Stop once the survival S falls below this.
    eps: f64,
    /// The run index (progress lines).
    run: u32,
}

/// What a path records as it goes (`Sim::play`).
#[derive(Default)]
struct PathTrace {
    /// Each change of the career picked: career index, pick, trainee, P(all), pool size, survival.
    pick_changes: Vec<Value>,
    /// The surrogate's checks against exact searches (`Sim::check`).
    checks: Vec<Value>,
}

/// One run's result (`Sim::play`).
struct PathResult {
    /// E: the sum of the survivals S_t, the tail S / p included (module notes).
    expected_careers: f64,
    /// Careers run before the path stopped.
    careers: usize,
    /// The tail S / p added at the stop.
    tail: f64,
    /// P(all) of the career the path stopped at.
    final_p_all: f64,
    /// New veterans dropped as dominated.
    pruned: usize,
}

pub(in crate::career) fn simulate<'data>(
    env: &mut Env<'data>,
    input: &JobInputs<'data>,
    legs: &[Legacy],
    trainees: &[crate::uma::UmaIndex],
    sim: &Value,
) -> Value {
    let cfg: SimJob = match serde_json::from_value(sim.clone()) {
        Ok(x) => x,
        Err(e) => return json!({"error": format!("simulate: {e}")}),
    };
    let ntar = env.target_count;
    if ntar > 12 || env.uncertain_count > 10 {
        return json!({"error": "simulate takes at most 12 targets, 10 of them not guaranteed"});
    }
    let base = match cfg.policy.as_str() {
        "greedy" | "greedy_exact" => Base::Greedy,
        "loop" => Base::Loop,
        "trace" => Base::Trace,
        "index" => Base::Index,
        p => return json!({"error": format!("simulate: unknown policy {p}")}),
    };
    let members: Vec<crate::uma::UmaIndex> = cfg.loop_members.iter().filter_map(|&id| env.uix(id)).collect();
    if matches!(base, Base::Loop | Base::Trace)
        && (members.len() < 3
            || members.len() != cfg.loop_members.len()
            || members.iter().any(|m| !env.owned.contains(m)))
    {
        return json!({"error": "loop: loop_members must be at least three owned umas"});
    }
    let n = input.data.umas.len();
    let mut mi = crate::uma::UmaTable(vec![None; if base == Base::Trace { n } else { 0 }]);
    if base == Base::Trace {
        for (k, &m) in members.iter().enumerate() {
            mi[m] = Some(k);
        }
    }
    let mut s = Sim::new(
        env,
        input,
        cfg.clone(),
        SimulationRoster { trainees: trainees.to_vec(), members, member_by_trainee: mi },
        SimulationOptions { record_audits: true, fill_missing_decks: false },
    );
    let t_init = std::time::Instant::now();
    let st0 = s.initial(legs, base);
    if base == Base::Trace {
        return s.trace(&st0);
    }
    let init_s = t_init.elapsed().as_secs_f64();
    let mut runs = vec![];
    let mut es = vec![];
    let eps = cfg.eps;
    // index: the kept state before the simulation, put back after it (module notes)
    let kept_before_simulation = (base == Base::Index).then(|| kept_swap(KeptState::default()));
    for run in cfg.first..cfg.first + cfg.runs {
        if !cfg.share_cache && run > cfg.first {
            s.cache = Cache::new(n);
        }
        if base == Base::Index {
            kept_swap(KeptState::default());
            IX.with(|ix| *ix.borrow_mut() = IndexRunCounts::default());
        }
        let tm = std::time::Instant::now();
        let n0 = s.n;
        let mut st = st0.clone();
        let mut trace = PathTrace::default();
        let keys = Keys::Main { seed: cfg.seed, run: run as u32 };
        let PathResult { expected_careers: e, careers, tail, final_p_all: fp, pruned } =
            s.play(&mut st, PlayOptions { base, keys, eps, run: run as u32 }, &mut trace);
        let n1 = s.n;
        es.push(e);
        let PathTrace { pick_changes: segs, checks } = trace;
        let finalk = segs.last().map_or(Value::Null, |x| x["pick"].clone());
        let wall = tm.elapsed().as_secs_f64();
        eprintln!(
            "{} run {}: E {:.1}, {} careers, tail {:.2}, final p {:.3e}, pool {}, {:.1} s",
            cfg.policy,
            run,
            e,
            careers,
            tail,
            fp,
            st.legacies.len(),
            wall
        );
        runs.push(json!({"run": run, "E": e, "careers": careers, "tail": tail, "final_p": fp, "final": finalk, "pool": st.legacies.len(), "pruned": pruned,
            "wall_s": wall, "candidates": n1.candidates - n0.candidates, "ceiling_pass": n1.ceiling_passes - n0.ceiling_passes, "tables": n1.tables - n0.tables,
            "refreshes": n1.searches - n0.searches, "partial_searches": n1.partial_searches - n0.partial_searches, "segs": segs, "checks": checks, "audits": std::mem::take(&mut s.audits)}));
        if base == Base::Index {
            let ix = IX.with(|ix| std::mem::take(&mut *ix.borrow_mut()));
            runs.last_mut().unwrap()["index"] = json!({"plans": ix.plans, "plan_s": ix.plan_seconds, "roles": ix.roles, "best_odds": ix.best_odds_picks});
        }
    }
    if let Some(kept) = kept_before_simulation {
        kept_swap(kept);
    }
    json!({"simulate": sim, "policy": cfg.policy, "runs": runs, "E": stats(&es), "init_s": init_s, "decks": s.cache.n_decks(), "deck_tables": s.cache.tables,
           "refreshes": s.n.searches, "schedules": s.win_chances.len()})
}
