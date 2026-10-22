//! Surrogate checks against exact career and deck searches, including audit rows and their ranking records.
//! Career selection and path execution live in the sibling modules.

use crate::career::Lineage;
use crate::career::inherit::{ExactOptions, LineageChances};
use crate::career::pool::Pool;
use crate::career::search::DeckRes;
use crate::career::sim::Sim;
use crate::career::sim::draw::Keys;
use crate::career::sim::play::PathState;
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};

impl Sim<'_, '_> {
    /// One audit row: a career valued by the surrogate's table (her cached decks, no refresh) against a fresh exact
    /// deck search there: [kind, table value, exact value, (exact - table) / exact, landed-set total variation between
    /// the two decks, search finished].
    pub(super) fn audit_row(&mut self, pool: &Pool, lineage: Lineage, kind: AuditKind) -> Value {
        let t = lineage.trainee;
        let LineageChances { chances: q, carriers: nc, spark_scale: c } = self.env.lineage(pool, lineage);
        let mut buf = vec![];
        let (tv, di) = self.tval(t, &q, c, &mut buf);
        let best = self.exact_best(t, &q, c);
        let tvd = if di == usize::MAX {
            1.0
        } else {
            let cached = self.cache.decks[t][di].sel.clone();
            self.landed_set_distance(t, &q, &nc, &cached, &best.deck)
        };
        let exv = best.value;
        json!([kind as u8, tv, exv, if exv > 0.0 { (exv - tv) / exv } else { 0.0 }, tvd, best.finished])
    }
    /// total variation distance between the landed-set distributions of two decks of trainee t at chances q (carriers
    /// nc)
    fn landed_set_distance(
        &mut self,
        t: crate::uma::UmaIndex,
        q: &[f64],
        nc: &[i32],
        first: &SelectedDeck,
        second: &SelectedDeck,
    ) -> f64 {
        let mut sets = |sel: &SelectedDeck| {
            self.env.set_trainee(t, q);
            let exact = self.env.exact(t, sel, nc, ExactOptions { all_but_one: false, landed_sets: true });
            self.env.outcome_sets(exact.outfit, &exact.ext)
        };
        let s1 = sets(first);
        let s2 = sets(second);
        0.5 * s1.iter().zip(&s2).map(|(x, y)| (x - y).abs()).sum::<f64>()
    }
    /// trainee t's exact best deck at chances q (a full deck search), its value with spark scale c
    pub(super) fn exact_best(&mut self, t: crate::uma::UmaIndex, q: &[f64], c: f64) -> ExactDeck {
        self.env.set_trainee(t, q);
        self.env.work = 0;
        let mut sc = 0;
        let r = self.env.best_deck(t, c, 0.0, &[], &mut sc);
        self.env.last_product_bound = None;
        match r {
            DeckRes::Done(v, deck, finished) => ExactDeck { value: c * v, deck, finished },
            _ => panic!("audit: no legal deck"),
        }
    }
    /// The greedy audit at a pick change: the exact career search's pick against the surrogate's, and audit rows for
    /// the pick and `audit_sample` random candidates with the newest veteran as a parent (ceiling at least 0.3 x B; every
    /// candidate of the pool at the start)
    pub(super) fn audit_greedy(&mut self, st: &mut PathState, keys: Keys, t_career: usize) -> Value {
        let cur = self.greedy_cur(st);
        let trainees = self.trainees.clone();
        let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
        let ex = sv.found.first().map(|f| {
            (
                f.value,
                format!(
                    "{}:{},{}",
                    self.d.umas[f.trainee].id,
                    f.first_parent.min(f.second_parent),
                    f.first_parent.max(f.second_parent)
                ),
            )
        });
        let sk = self.key(&cur);
        let differs = ex.as_ref().is_none_or(|x| x.1 != sk);
        let mut rows = vec![self.audit_row(&st.pool, cur.lineage, AuditKind::Pick)];
        let bv = st.best.map_or(0.0, |p| p.value);
        let n = st.legacies.len();
        let vi = n - 1;
        let all = t_career == 0;
        let picked = cur.lineage.ordered();
        let mut cs = vec![];
        for &t in &trainees {
            for a in 0..n {
                for b in a + 1..n {
                    if !all && b != vi {
                        continue;
                    }
                    let lineage = Lineage { trainee: t, first_parent: a, second_parent: b };
                    if !self.legal(st, lineage) || lineage == picked {
                        continue;
                    }
                    if self.fast_ub(st, lineage) >= 0.3 * bv {
                        cs.push(lineage);
                    }
                }
            }
        }
        for i in 0..self.cfg.audit_sample.min(cs.len()) {
            let j = i + (keys.u(t_career, 9, i) * (cs.len() - i) as f64) as usize;
            let last = cs.len() - 1;
            cs.swap(i, j.min(last));
            rows.push(self.audit_row(&st.pool, cs[i], AuditKind::Random));
        }
        json!({"t": t_career, "pool": n, "sur": sk, "sur_p": cur.p_all, "ex": ex.as_ref().map(|x| x.1.clone()), "ex_p": ex.as_ref().map(|x| x.0),
               "differs": differs, "proven": sv.proven, "rows": rows})
    }
    /// the surrogate against exact searches on this pool (the argmax career, and the newest veteran's lineages)
    pub(super) fn check(&mut self, st: &mut PathState, all: bool) -> Value {
        let cur = self.greedy_cur(st);
        let trainees = self.trainees.clone();
        let sv = self.env.solve(&st.pool, &[], &trainees, false, 1);
        let ex = sv
            .found
            .first()
            .map(|f| (f.value, format!("{}:{},{}", self.d.umas[f.trainee].id, f.first_parent, f.second_parent)));
        let argmax = json!({"pool": st.legacies.len(), "sur": self.key(&cur), "sur_p": cur.p_all, "sur_val": st.best.map(|p| p.value),
                            "ex": ex.as_ref().map(|x| x.1.clone()), "ex_p": ex.as_ref().map(|x| x.0), "proven": sv.proven});
        let mut rows = vec![];
        if self.cfg.check_lineages > 0 {
            let bv = st.best.map_or(0.0, |p| p.value);
            let n = st.legacies.len();
            let vi = n - 1;
            // (the legal careers with her, or every one, `gen_cands`' ceilings, at least check_frac x the best)
            let mut cs: Vec<RankedCareer> = self
                .env
                .gen_cands(&st.pool, &[], &trainees, |_, b| all || b == vi, RankedCareer::new)
                .into_iter()
                .filter(|x| x.ceiling >= self.cfg.check_frac * bv)
                .collect();
            cs.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
            cs.truncate(self.cfg.check_lineages);
            if let Some(p) = st.best
                && !cs.iter().any(|x| x.lineage == p.lineage)
            {
                let (q, c) = self.env.lineage_q(&st.pool, p.lineage);
                cs.push(RankedCareer {
                    ceiling: self.env.ceiling(p.lineage.trainee, &q, c).unwrap(),
                    lineage: p.lineage,
                });
            }
            let mut buf = vec![];
            for RankedCareer { ceiling: ub, lineage } in cs {
                let t = lineage.trainee;
                let LineageChances { chances: q, carriers: nc, spark_scale: c } = self.env.lineage(&st.pool, lineage);
                let rb = self.cache.refs[t].iter().map(|r| self.env.ref_bound(t, &q, c, r)).fold(ub, f64::min);
                let (tv, di) = self.tval(t, &q, c, &mut buf);
                self.env.set_trainee(t, &q);
                self.env.work = 0;
                let mut sc = 0;
                let r = self.env.best_deck(t, c, 0.0, &[], &mut sc);
                self.env.last_product_bound = None;
                let DeckRes::Done(v, sel, full) = r else { continue };
                let exv = c * v;
                // landed sets: the cached argmax deck against the exact best deck (total variation distance)
                let tvd = if di == usize::MAX {
                    1.0
                } else {
                    let cached = self.cache.decks[t][di].sel.clone();
                    self.landed_set_distance(t, &q, &nc, &cached, &sel)
                };
                let pick = st.best.is_some_and(|p| p.lineage == lineage);
                rows.push(json!([
                    ub,
                    rb,
                    tv,
                    exv,
                    (exv - tv) / exv,
                    rb <= tv * (1.0 + self.cfg.margin),
                    tvd,
                    full,
                    pick
                ]));
            }
        }
        json!({"argmax": argmax, "lineages": rows})
    }
}

/// A legal career and its ceiling (`Env::gen_cands`' order).
#[derive(Clone, Copy)]
pub(super) struct RankedCareer {
    pub(super) ceiling: f64,
    pub(super) lineage: Lineage,
}
impl RankedCareer {
    /// `gen_cands`' constructor: (ceiling, trainee position, trainee, a, b)
    pub(super) const fn new(
        ceiling: f64,
        _: usize,
        trainee: crate::uma::UmaIndex,
        first_parent: usize,
        second_parent: usize,
    ) -> Self {
        Self { ceiling, lineage: Lineage { trainee, first_parent, second_parent } }
    }
}

/// What an audit row is of (`Sim::audit_row`; its number is the row's first entry).
#[derive(Clone, Copy)]
pub(super) enum AuditKind {
    /// The surrogate's pick at a pick change.
    Pick = 0,
    /// A random candidate with the newest veteran.
    Random = 1,
    /// A loop career.
    Loop = 3,
}

/// A trainee's exact best deck (`Sim::exact_best`).
pub(super) struct ExactDeck {
    /// Its exact value times the spark scale.
    pub(super) value: f64,
    deck: SelectedDeck,
    /// The deck search finished (proven best).
    finished: bool,
}
