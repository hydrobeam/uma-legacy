//! Career selection for greedy, loop, trace, exact and page-index policies, with rotation and index-run records.
//! Draws, path execution and exact audit comparisons live in the sibling modules.

use crate::career::plan;
use crate::career::plan::ScoringContext;
use crate::career::pool::Pool;
use crate::career::sim::draw::Keys;
use crate::career::sim::play::PathState;
use crate::career::sim::{Cur, DeckRequest, PAGE_TOP, Sim, SurrogatePick, leg_json};
use crate::career::{Career, Lineage, run_career};
use serde_json::{Value, json};
use std::rc::Rc;

/// Loop rotation: the next member's position and each member's latest rotation veteran
#[derive(Clone, Default)]
pub(super) struct Rotation {
    /// Careers run so far; the next member is this modulo the member count.
    pub(super) position: usize,
    /// Per member, her rotation veteran's legacy index.
    pub(super) latest: Vec<Option<usize>>,
}

/// `trace`: a builder's best career and what it does for the trainee built for (A, best odds p).
#[derive(Clone, Copy)]
struct BuilderCareer {
    /// lam (1/p - 1/p1): the expected fall in A's careers needed, per career run.
    efficiency: f64,
    pick: SurrogatePick,
    /// lam: the chance her veteran raises A's best odds.
    lift_chance: f64,
    /// p1: A's mean odds when raised (p when never).
    lifted_odds: f64,
}

/// `index`, this run: what its plan jobs did.
#[derive(Default)]
pub(super) struct IndexRunCounts {
    /// Plan jobs run.
    pub(super) plans: usize,
    /// Each plan job's seconds, rounded to the millisecond.
    pub(super) plan_seconds: Vec<f64>,
    /// Per role of the chosen option (`next.options[].role`), how many picks had it.
    pub(super) roles: std::collections::BTreeMap<String, usize>,
    /// Picks that were the best-odds career (`best_odds`).
    pub(super) best_odds_picks: usize,
}
thread_local! { pub(super) static IX: std::cell::RefCell<IndexRunCounts> = std::cell::RefCell::new(IndexRunCounts::default()); }

impl Sim<'_, '_> {
    /// a cached pick's career (her cached deck cloned out of the cache)
    fn pick_cur(&mut self, pool: &Pool, pick: SurrogatePick, rotation: bool) -> Cur {
        let deck = self.cache.decks[pick.lineage.trainee][pick.deck].sel.clone();
        self.make_cur(pool, Career { lineage: pick.lineage, deck: &deck }, rotation)
    }
    pub(super) fn greedy_cur(&mut self, st: &mut PathState) -> Rc<Cur> {
        if let Some(c) = &st.current {
            return Rc::clone(c);
        }
        let pick = st.best.expect("no legal career");
        let c = Rc::new(self.pick_cur(&st.pool, pick, false));
        st.current = Some(Rc::clone(&c));
        c
    }
    /// `trace`: building for loop_members[0] (A) with the others as builders. Each career: per builder her best career's
    /// exact chance of raising A's best odds (lam) and mean odds raised to (p1); the most efficient (lam (1/p - 1/p1)) runs,
    /// her veteran joins; recorded per career: A's odds before, the builder, lam, p1, and A's odds after
    pub(super) fn trace(&mut self, st0: &PathState) -> Value {
        let a = self.members[0];
        let nb = self.members.len();
        let mut out = vec![];
        for run in self.cfg.first..self.cfg.first + self.cfg.runs {
            let mut st = st0.clone();
            let keys = Keys::Main { seed: self.cfg.seed, run: run as u32 };
            let mut buf = vec![];
            let mut rows = vec![];
            let tm = std::time::Instant::now();
            for t in 0..self.cfg.max_careers {
                let Some(pka) = st.member_best[0] else { break };
                let pa = pka.value;
                // this career's scoring of the builders: A alone, at her best odds now, on this career's pool
                let one = [a];
                let pw = [pa];
                let base = plan::cont::ScoreBase {
                    candidates: &one,
                    step_costs: &[1.0],
                    later_stage_costs: &[0.0],
                    current_value: 0.0,
                    current_odds: &pw,
                    continuations: &[],
                    trios: &[],
                    trio_sides: &[],
                    finisher_umas: &[],
                };
                let scoring = ScoringContext { score_base: &base, legacies: &st.legacies, pool: &st.pool };
                let mut best_builder: Option<BuilderCareer> = None;
                for k in 1..nb {
                    let Some(pk) = st.member_best[k] else { continue };
                    let sel = self.cache.decks[pk.lineage.trainee][pk.deck].sel.clone();
                    let plan::score::CareerScore { lift_stats: stt, .. } = plan::score::score_career_of(
                        self,
                        &scoring,
                        Career { lineage: pk.lineage, deck: &sel },
                        plan::cont::Want::Stats,
                        &mut buf,
                    );
                    let plan::score::kernel::LiftStats { lift_chance: pl, lifted_odds_sum: sp } = stt[0];
                    let (e, p1) = if pl > 0.0 {
                        let p1 = sp / pl;
                        (pl * (1.0 / pa - 1.0 / p1), p1)
                    } else {
                        (0.0, pa)
                    };
                    if best_builder.is_none_or(|b| e > b.efficiency) {
                        best_builder =
                            Some(BuilderCareer { efficiency: e, pick: pk, lift_chance: pl, lifted_odds: p1 });
                    }
                }
                let Some(builder) = best_builder else { break };
                let cur = self.pick_cur(&st.pool, builder.pick, false);
                let vet = self.draw(&cur, &st.legacies, keys, t);
                self.add_vet(&mut st, vet, true, &mut buf);
                let after = st.member_best[0].map_or(pa, |p| p.value);
                rows.push(json!([
                    t,
                    pa,
                    self.d.umas[builder.pick.lineage.trainee].name,
                    builder.lift_chance,
                    builder.lifted_odds,
                    after
                ]));
            }
            eprintln!(
                "trace run {}: {} careers, A from {:.3e} to {:.3e}, {:.1} s",
                run,
                rows.len(),
                st0.member_best[0].map_or(0.0, |p| p.value),
                st.member_best[0].map_or(0.0, |p| p.value),
                tm.elapsed().as_secs_f64()
            );
            out.push(json!({"run": run, "rows": rows}));
        }
        json!({"trace": out, "trainee": self.d.umas[a].name, "builders": self.members[1..].iter().map(|&u| self.d.umas[u].name.clone()).collect::<Vec<_>>()})
    }
    /// the loop's next career (module notes: rotation and bootstrap)
    pub(super) fn loop_cur(&mut self, st: &mut PathState) -> Rc<Cur> {
        if let Some(c) = &st.current {
            return Rc::clone(c);
        }
        let nm = self.members.len();
        let pos = st.rotation.position % nm;
        let t = self.members[pos];
        let prev: Vec<usize> = [1, 2].iter().filter_map(|&k| st.rotation.latest[(pos + nm - k) % nm]).collect();
        let c = if prev.len() == 2 {
            let lineage =
                Lineage { trainee: t, first_parent: prev[0].min(prev[1]), second_parent: prev[0].max(prev[1]) };
            let (q, c) = self.env.lineage_q(&st.pool, lineage);
            let ub = self.env.ceiling(t, &q, c).unwrap_or(f64::INFINITY);
            let mut buf = vec![];
            let (_, di) = self
                .deck_for(
                    DeckRequest { trainee: t, inheritance_chances: &q, spark_scale: c, ceiling: ub, threshold: 0.0 },
                    &mut buf,
                )
                .expect("loop career without a deck");
            let sel = self.cache.decks[t][di].sel.clone();
            self.make_cur(&st.pool, Career { lineage, deck: &sel }, true)
        } else {
            self.env.use_seen = true;
            let sv = self.env.solve(&st.pool, &prev, &[t], false, 1);
            let f = sv.found.first().expect("loop bootstrap: no legal career for a member");
            self.cache.absorb(self.env, &sv.refs);
            self.make_cur(&st.pool, Career { lineage: f.lineage(), deck: &f.deck }, true)
        };
        let c = Rc::new(c);
        st.current = Some(Rc::clone(&c));
        c
    }
    /// The loop careers veteran v of member m is a parent of (members m + 1 and m + 2, the other parent their current
    /// rotation veteran; a term without one, or with her own character, is left out), in that order
    fn rotation_careers(&self, st: &PathState, m: usize, v: usize) -> Vec<Lineage> {
        let nm = self.members.len();
        let rotation = &st.rotation;
        let mut careers = vec![];
        for k in 1..=2 {
            let t = self.members[(m + k) % nm];
            let other = if k == 1 { rotation.latest[(m + nm - 1) % nm] } else { rotation.latest[(m + 1) % nm] };
            let Some(o) = other else { continue };
            if o == v || st.pool.veterans[o].chara == st.pool.veterans[v].chara {
                continue;
            }
            careers.push(Lineage { trainee: t, first_parent: v.min(o), second_parent: v.max(o) });
        }
        careers
    }
    /// The loop's rotation after a member's career (her veteran the pool's last): the member keeps the better of her
    /// rotation veteran and the new one, by the sum of the surrogate P(all) of the two loop careers she is a parent of
    /// (`rotation_careers`, for both); the first one she has is kept as is. Then the next member trains.
    pub(super) fn rot_update(&mut self, st: &mut PathState) {
        let nm = self.members.len();
        let m = st.rotation.position % nm;
        let vi = st.legacies.len() - 1;
        st.rotation.position += 1;
        st.current = None;
        let Some(old) = st.rotation.latest[m] else {
            st.rotation.latest[m] = Some(vi);
            return;
        };
        let mut buf = vec![];
        let mut surrogate = |s: &mut Self, v: usize| -> f64 {
            let mut tot = 0.0;
            for lineage in s.rotation_careers(st, m, v) {
                let t = lineage.trainee;
                let (q, c) = s.env.lineage_q(&st.pool, lineage);
                let ub = s.env.ceiling(t, &q, c).unwrap_or(f64::INFINITY);
                tot += s
                    .deck_for(
                        DeckRequest {
                            trainee: t,
                            inheritance_chances: &q,
                            spark_scale: c,
                            ceiling: ub,
                            threshold: 0.0,
                        },
                        &mut buf,
                    )
                    .map_or(0.0, |x| x.0);
            }
            tot
        };
        let (so, sn) = (surrogate(self, old), surrogate(self, vi));
        if self.cfg.audit && self.record_audits && st.rotation.position.is_multiple_of(self.cfg.audit_every) {
            // the keep-or-replace decision under exact values (a full deck search per term)
            let exact = |s: &mut Self, v: usize| -> f64 {
                let mut tot = 0.0;
                for lineage in s.rotation_careers(st, m, v) {
                    let (q, c) = s.env.lineage_q(&st.pool, lineage);
                    tot += s.exact_best(lineage.trainee, &q, c).value;
                }
                tot
            };
            let (eo, en) = (exact(self, old), exact(self, vi));
            self.audits.push(json!({"t": st.rotation.position, "rotation": [so, sn, eo, en, (sn > so) == (en > eo)]}));
        }
        if sn > so {
            st.rotation.latest[m] = Some(vi);
        }
    }
    /// Builds a runnable career from a selected internal solver result.
    /// The caller checks that `pick` is an object. Its required fields supply the
    /// exact probabilities; resolving the win chances also fills the schedule cache.
    fn cur_from_solver_pick(&mut self, pool: &Pool, pick: &Value) -> Cur {
        let trainee = self.env.uix(pick["trainee"]["id"].as_u64().unwrap() as u32).unwrap();
        let (parent_a, parent_b) =
            (pick["parents"][0].as_u64().unwrap() as usize, pick["parents"][1].as_u64().unwrap() as usize);
        let outcome_sets: Vec<f64> =
            pick["outcome_sets"].as_array().unwrap().iter().map(|x| x.as_f64().unwrap()).collect();
        let lineage = Lineage { trainee, first_parent: parent_a, second_parent: parent_b };
        let win_probs = self.wins(pool, lineage);
        Cur { lineage, p_all: pick["p_all"].as_f64().unwrap(), os: outcome_sets, wp: win_probs, rotation: false }
    }
    /// the in-process career search's pick (greedy_exact: the best career by P(all))
    pub(super) fn exact_cur(&mut self, st: &PathState) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        o.remove("simulate");
        o.insert("legacies".into(), Value::Array(st.legacies.iter().map(leg_json).collect()));
        o.insert("top".into(), json!(1));
        o.insert("per_trainee".into(), json!(false));
        let out = run_career(self.d, self.skills, self.cards, &job);
        let pick = out["results"][0].clone();
        assert!(pick.is_object(), "solver gave no pick: {}", out.to_string().chars().take(300).collect::<String>());
        self.cur_from_solver_pick(&st.pool, &pick)
    }
    /// `index`: the page's plan for this pool (the career job with the pool as legacies, no places, the page's `top`,
    /// `next`, `no_reports`: the plan job the page sends, docs/SOLVER.md "Next career"), in process; its `choice` runs.
    /// The job's `trainees` stay (the page sends none: every owned uma)
    pub(super) fn index_cur(&mut self, st: &PathState) -> Cur {
        let mut job = self.job.clone();
        let o = job.as_object_mut().unwrap();
        o.remove("simulate");
        o.insert("legacies".into(), Value::Array(st.legacies.iter().map(leg_json).collect()));
        o.insert("fix".into(), json!([]));
        o.insert("top".into(), json!(PAGE_TOP));
        o.insert("per_trainee".into(), json!(false));
        o.insert("next".into(), self.cfg.next.clone());
        o.insert("no_reports".into(), json!(true));
        let tm = std::time::Instant::now();
        let out = run_career(self.d, self.skills, self.cards, &job);
        let pick = out["next"]["choice"].clone();
        assert!(
            pick.is_object(),
            "index: the plan gave no pick: {}",
            out.to_string().chars().take(300).collect::<String>()
        );
        IX.with(|ix| {
            let mut ix = ix.borrow_mut();
            ix.plans += 1;
            ix.plan_seconds.push((tm.elapsed().as_secs_f64() * 1e3).round() / 1e3);
            if let Some(c) =
                out["next"]["options"].as_array().and_then(|v| v.iter().find(|x| x["chosen"].as_bool() == Some(true)))
            {
                *ix.roles.entry(c["role"].as_str().unwrap_or("?").to_string()).or_default() += 1;
                if c["best_odds"].as_bool() == Some(true) {
                    ix.best_odds_picks += 1;
                }
            }
        });
        self.cur_from_solver_pick(&st.pool, &pick)
    }
}
