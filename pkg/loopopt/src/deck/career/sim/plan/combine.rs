//! Continuation fits and candidate combinations.
use super::{
    CEIL_SHARE, Cont, PlanCandidates, PlanInputs, PlanRow, RowOrigin, ScoreBase, Trio, legal_pair, lift_eff,
    loop_ceiling, val,
};
use crate::deck::career::sim::{Env, Legacy, Pool, Sim, Value};

/// A and B index `PlanCandidates::trainees`; the finisher indexes `Data::umas` directly.
/// Each side uses either an existing full veteran (a free step) or a newly built one.
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim::plan) struct PlanCombination {
    pub(in crate::deck::career::sim::plan) candidate_a_index: usize,
    pub(in crate::deck::career::sim::plan) candidate_b_index: usize,
    /// Expected careers for the finisher, after A and B are full veterans.
    pub(in crate::deck::career::sim::plan) finisher_cost: f64,
    pub(in crate::deck::career::sim::plan) finisher_trainee: usize,
    pub(in crate::deck::career::sim::plan) a_uses_existing: bool,
    pub(in crate::deck::career::sim::plan) b_uses_existing: bool,
}

impl PlanCombination {
    pub(in crate::deck::career::sim::plan) fn total_cost(&self, stage_costs: &[f64]) -> f64 {
        let a_cost = if self.a_uses_existing { 0.0 } else { stage_costs[self.candidate_a_index] };
        let b_cost = if self.b_uses_existing { 0.0 } else { stage_costs[self.candidate_b_index] };
        a_cost + b_cost + self.finisher_cost
    }

    pub(in crate::deck::career::sim::plan) const fn as_trio(&self) -> Trio {
        (self.candidate_a_index, self.candidate_b_index, self.finisher_cost, self.a_uses_existing, self.b_uses_existing)
    }
}

/// Continuation and trio values fitted together for scoring the plan's rows.
/// Per-candidate buffers use the fixed order in `PlanCandidates::trainees`.
pub(in crate::deck::career::sim::plan) struct PlanFit {
    pub(in crate::deck::career::sim::plan) continuations: Vec<Cont>,
    pub(in crate::deck::career::sim::plan) step_costs: Vec<f64>,
    pub(in crate::deck::career::sim::plan) trios: Vec<Trio>,
    pub(in crate::deck::career::sim::plan) sides: Vec<Vec<(bool, Legacy)>>,
    pub(in crate::deck::career::sim::plan) current_value: f64,
}

impl PlanFit {
    pub(in crate::deck::career::sim::plan) fn score_base<'fit>(
        &'fit self,
        candidates: &'fit PlanCandidates,
    ) -> ScoreBase<'fit> {
        ScoreBase {
            cands: &candidates.trainees,
            c1: &self.step_costs,
            m23: &candidates.zeros,
            v0: self.current_value,
            pnow: &candidates.pnow,
            cont: &self.continuations,
            combos: &self.trios,
            sides: &self.sides,
            all: &candidates.allowed_trainees,
        }
    }
}

/// Pass 2's completed fit and reporting results, used by the proxy check and output assembly.
pub(in crate::deck::career::sim::plan) struct PlanScores {
    pub(in crate::deck::career::sim::plan) fit: PlanFit,
    pub(in crate::deck::career::sim::plan) best_combo: Option<PlanCombination>,
    pub(in crate::deck::career::sim::plan) any_scored: bool,
    pub(in crate::deck::career::sim::plan) combo_ms: f64,
    pub(in crate::deck::career::sim::plan) index_ms: f64,
    pub(in crate::deck::career::sim::plan) work: Value,
}

pub(in crate::deck::career::sim::plan) fn plan_continuations(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    runs: &[PlanRow],
    wins_by_trainee: &[Vec<usize>],
    buf: &mut Vec<f64>,
) -> Vec<Cont> {
    let PlanInputs { d, pool, pj, .. } = *input;
    let n = candidates.trainees.len();

    let wt = |x: usize| wins_by_trainee[x].clone();
    let mut cont = vec![Cont::default(); n];
    for ui in 0..n {
        if candidates.pnow[ui] <= 0.0 || runs.is_empty() {
            continue;
        }
        let u = candidates.trainees[ui];
        let p = candidates.pnow[ui];
        let mut eff: Vec<(f64, usize, f64, f64)> = runs
            .iter()
            .filter(|r| {
                r.origin == RowOrigin::Base
                    && r.exact
                    && d.umas[r.trainee].id != d.umas[u].id
                    && r.lift_stats[ui].0 > 0.0
            })
            .map(|r| {
                let (pl, sp) = r.lift_stats[ui];
                (lift_eff(pl, sp, p), r.trainee, pl, sp / pl)
            })
            .collect();
        eff.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let Some(&(_, b1, lam, p1)) = eff.first() else { continue };
        let b2 = eff.iter().skip(1).map(|e| e.1).find(|&b| d.umas[b].id != d.umas[b1].id);
        let pinf = match b2 {
            Some(b2) => (CEIL_SHARE * loop_ceiling(s, d, pool, u, b1, b2, &wt, buf)).max(p1),
            None => p1,
        };
        let phi = if pinf > p && p1 > p { ((p1 / p).ln() / (pinf / p).ln()).clamp(0.0, 1.0) } else { 0.0 };
        cont[ui] = Cont::new(lam, phi, pinf);
    }
    // a given fit replaces the pool's (`PlanJob::fit`)
    if let Some(f) = &pj.fit {
        for ui in 0..n {
            if let Some(&(_, lam, phi, pinf)) = f.iter().find(|x| d.umas[candidates.trainees[ui]].id == x.0) {
                cont[ui] = Cont::new(lam, phi, pinf);
            }
        }
    }
    cont
}

/// Combinations in candidate-pair order, then fresh/existing side order.
pub(in crate::deck::career::sim::plan) fn plan_combinations(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    real8: &[Vec<usize>],
    c1: &[f64],
    buf: &mut Vec<f64>,
) -> Vec<PlanCombination> {
    let PlanInputs { d, legs, pool, .. } = *input;
    let n = candidates.trainees.len();

    let hyp8: Vec<Option<Legacy>> = (0..n)
        .map(|ui| {
            if c1[ui].is_finite() {
                candidates.careers[ui].as_ref().map(|f| {
                    let wp = s.wins(pool, candidates.trainees[ui], f.a, f.b);
                    s.env.veteran(legs, candidates.trainees[ui], f.a, f.b, wp.to_vec())
                })
            } else {
                None
            }
        })
        .collect();
    let mut combos: Vec<PlanCombination> = vec![];
    for ia in 0..n {
        for ib in ia + 1..n {
            let (ua, ub) = (candidates.trainees[ia], candidates.trainees[ib]);
            if d.umas[ua].id == d.umas[ub].id {
                continue;
            }
            // (`Env::pool` of the legacies and the fresh 8/8s: the working copy of `pool` with them pushed, A's first,
            // popped below)
            let mut p8 = s.take_pool(pool);
            // per side: (real, legacy index)
            let side = |s: &mut Sim, p8: &mut Pool, ui: usize| -> Vec<(bool, usize)> {
                let mut v: Vec<(bool, usize)> = real8[ui].iter().map(|&i| (true, i)).collect();
                if let Some(h) = &hyp8[ui] {
                    s.env.pool_push(p8, h);
                    v.push((false, p8.me.len() - 1));
                }
                v
            };
            let (xs, ys) = (side(s, &mut p8, ia), side(s, &mut p8, ib));
            // per side choice (fresh/real for A, fresh/real for B): the best C
            let mut bc = [(0.0f64, usize::MAX); 4];
            for &c in &candidates.allowed_trainees {
                if d.umas[c].id == d.umas[ua].id || d.umas[c].id == d.umas[ub].id {
                    continue;
                }
                for &(ra, x) in &xs {
                    for &(rb, y) in &ys {
                        if legal_pair(d, &p8, c, x, y) {
                            let k = usize::from(ra) * 2 + usize::from(rb);
                            let v = val(s, &p8, c, x, y, buf);
                            if v > bc[k].0 {
                                bc[k] = (v, c);
                            }
                        }
                    }
                }
            }
            for (k, &(v, cu)) in bc.iter().enumerate() {
                if cu != usize::MAX {
                    combos.push(PlanCombination {
                        candidate_a_index: ia,
                        candidate_b_index: ib,
                        finisher_cost: 1.0 / v,
                        finisher_trainee: cu,
                        a_uses_existing: k >= 2,
                        b_uses_existing: k % 2 == 1,
                    });
                }
            }
            while p8.me.len() > pool.me.len() {
                Env::pool_pop(&mut p8);
            }
            s.put_pool(pool, p8);
        }
    }
    combos
}

// the trios' sides per candidate (her real 8/8s, then the fresh 8/8 her best career now would leave), for `her_trios`
pub(in crate::deck::career::sim::plan) fn plan_sides(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    real8: &[Vec<usize>],
    c1: &[f64],
) -> Vec<Vec<(bool, Legacy)>> {
    let PlanInputs { legs, pool, .. } = *input;
    let n = candidates.trainees.len();

    (0..n)
        .map(|ui| {
            let mut v: Vec<(bool, Legacy)> = real8[ui].iter().map(|&i| (true, legs[i].clone())).collect();
            if c1[ui].is_finite()
                && let Some(f) = candidates.careers[ui].as_ref()
            {
                let wp = s.wins(pool, candidates.trainees[ui], f.a, f.b);
                v.push((false, s.env.veteran(legs, candidates.trainees[ui], f.a, f.b, wp.to_vec())));
            }
            v
        })
        .collect()
}
