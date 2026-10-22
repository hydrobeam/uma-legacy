//! Surrogate checks against exact career and deck searches, including audit rows and their ranking records.
//! Career selection and path execution live in the sibling modules.

use crate::career::inherit::{ExactOptions, LineageChances};
use crate::career::pool::Pool;
use crate::career::search::DeckRes;
use crate::career::sim::Sim;
use crate::career::sim::draw::Keys;
use crate::career::sim::play::PathState;
use crate::career::{Env, Lineage};
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};

impl Sim<'_, '_> {
    /// One audit row: a career valued by the surrogate's table (her cached decks, no refresh) against a fresh exact
    /// deck search there: [kind, table value, exact value, (exact - table) / exact, landed-set total variation between
    /// the two decks, search finished].
    pub(super) fn audit_row(&mut self, pool: &Pool, lineage: Lineage, kind: AuditKind) -> Value {
        let trainee = lineage.trainee;
        let LineageChances { chances: inheritance_chances, carriers, spark_scale } = self.env.lineage(pool, lineage);
        let mut interpolation_scratch = vec![];
        let (table_value, deck_index) =
            self.evaluator.tval(self.env, trainee, &inheritance_chances, spark_scale, &mut interpolation_scratch);
        let best = self.exact_best(trainee, &inheritance_chances, spark_scale);
        let landed_distance = if deck_index == usize::MAX {
            1.0
        } else {
            let cached = &self.evaluator.cache.decks[trainee][deck_index].sel;
            Self::landed_set_distance(self.env, trainee, &inheritance_chances, &carriers, cached, &best.deck)
        };
        best.audit_row(kind, table_value, landed_distance)
    }
    /// Total variation distance between two decks' landed-set distributions at the same inheritance chances.
    fn landed_set_distance(
        env: &mut Env,
        trainee: crate::uma::UmaIndex,
        inheritance_chances: &[f64],
        carriers: &[i32],
        first: &SelectedDeck,
        second: &SelectedDeck,
    ) -> f64 {
        let mut sets = |deck: &SelectedDeck| {
            env.set_trainee(trainee, inheritance_chances);
            let exact = env.exact(trainee, deck, carriers, ExactOptions { all_but_one: false, landed_sets: true });
            env.outcome_sets(exact.outfit, &exact.ext)
        };
        let first_sets = sets(first);
        let second_sets = sets(second);
        0.5 * first_sets.iter().zip(&second_sets).map(|(x, y)| (x - y).abs()).sum::<f64>()
    }
    /// The trainee's exact best deck at these inheritance chances (a full search), including the spark scale.
    pub(super) fn exact_best(
        &mut self,
        trainee: crate::uma::UmaIndex,
        inheritance_chances: &[f64],
        spark_scale: f64,
    ) -> ExactDeck {
        self.env.set_trainee(trainee, inheritance_chances);
        self.env.work = 0;
        let mut scored_decks = 0;
        let search_result = self.env.best_deck(trainee, spark_scale, 0.0, &[], &mut scored_decks);
        self.env.last_product_bound = None;
        match search_result {
            DeckRes::Done(value, deck, finished) => ExactDeck { value: spark_scale * value, deck, finished },
            _ => panic!("audit: no legal deck"),
        }
    }
    /// The greedy audit at a pick change: the exact career search's pick against the surrogate's, and audit rows for
    /// the pick and `audit_sample` random candidates with the newest veteran as a parent (ceiling at least 0.3 x B; every
    /// candidate of the pool at the start)
    pub(super) fn audit_greedy(&mut self, state: &mut PathState, keys: Keys, career_index: usize) -> Value {
        let career = self.greedy_cur(state);
        let trainees = self.trainees.clone();
        let search = self.env.solve(&state.pool, &[], &trainees, false, 1);
        let exact_pick = search.found.first().map(|found| {
            (
                found.value,
                format!(
                    "{}:{},{}",
                    self.d.umas[found.trainee].id,
                    found.first_parent.min(found.second_parent),
                    found.first_parent.max(found.second_parent)
                ),
            )
        });
        let surrogate_key = self.key(&career);
        let differs = exact_pick.as_ref().is_none_or(|(_, key)| *key != surrogate_key);
        let mut rows = vec![self.audit_row(&state.pool, career.lineage, AuditKind::Pick)];
        let best_value = state.best.map_or(0.0, |pick| pick.value);
        let pool_size = state.legacies.len();
        let newest_veteran = pool_size - 1;
        let all = career_index == 0;
        let picked = career.lineage.ordered();
        let mut candidates = vec![];
        for &trainee in &trainees {
            for first_parent in 0..pool_size {
                for second_parent in first_parent + 1..pool_size {
                    if !all && second_parent != newest_veteran {
                        continue;
                    }
                    let lineage = Lineage { trainee, first_parent, second_parent };
                    if !self.legal(state, lineage) || lineage == picked {
                        continue;
                    }
                    if self.evaluator.fast_ub(self.env, &state.pool, lineage) >= 0.3 * best_value {
                        candidates.push(lineage);
                    }
                }
            }
        }
        for i in 0..self.cfg.audit_sample.min(candidates.len()) {
            let j = i + (keys.uniform(career_index, 9, i) * (candidates.len() - i) as f64) as usize;
            let last = candidates.len() - 1;
            candidates.swap(i, j.min(last));
            rows.push(self.audit_row(&state.pool, candidates[i], AuditKind::Random));
        }
        json!({"t": career_index, "pool": pool_size, "sur": surrogate_key, "sur_p": career.p_all, "ex": exact_pick.as_ref().map(|(_, key)| key.clone()), "ex_p": exact_pick.as_ref().map(|(value, _)| *value),
               "differs": differs, "proven": search.proven, "rows": rows})
    }
    /// the surrogate against exact searches on this pool (the argmax career, and the newest veteran's lineages)
    pub(super) fn check(&mut self, state: &mut PathState, all: bool) -> Value {
        let career = self.greedy_cur(state);
        let trainees = self.trainees.clone();
        let search = self.env.solve(&state.pool, &[], &trainees, false, 1);
        let exact_pick = search.found.first().map(|found| {
            (found.value, format!("{}:{},{}", self.d.umas[found.trainee].id, found.first_parent, found.second_parent))
        });
        let argmax = json!({"pool": state.legacies.len(), "sur": self.key(&career), "sur_p": career.p_all, "sur_val": state.best.map(|pick| pick.value),
                            "ex": exact_pick.as_ref().map(|(_, key)| key.clone()), "ex_p": exact_pick.as_ref().map(|(value, _)| *value), "proven": search.proven});
        let mut rows = vec![];
        if self.cfg.check_lineages > 0 {
            let best_value = state.best.map_or(0.0, |pick| pick.value);
            let pool_size = state.legacies.len();
            let newest_veteran = pool_size - 1;
            // (the legal careers with her, or every one, `gen_cands`' ceilings, at least check_frac x the best)
            let mut candidates: Vec<RankedCareer> = self
                .env
                .gen_cands(
                    &state.pool,
                    &[],
                    &trainees,
                    |_, second_parent| all || second_parent == newest_veteran,
                    RankedCareer::new,
                )
                .into_iter()
                .filter(|candidate| candidate.ceiling >= self.cfg.check_frac * best_value)
                .collect();
            candidates.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
            candidates.truncate(self.cfg.check_lineages);
            if let Some(pick) = state.best
                && !candidates.iter().any(|candidate| candidate.lineage == pick.lineage)
            {
                let (inheritance_chances, spark_scale) = self.env.lineage_q(&state.pool, pick.lineage);
                candidates.push(RankedCareer {
                    ceiling: self.env.ceiling(pick.lineage.trainee, &inheritance_chances, spark_scale).unwrap(),
                    lineage: pick.lineage,
                });
            }
            let mut interpolation_scratch = vec![];
            for candidate in candidates {
                if let Some(row) = self.check_lineage(state, candidate, &mut interpolation_scratch) {
                    rows.push(row);
                }
            }
        }
        json!({"argmax": argmax, "lineages": rows})
    }
    fn check_lineage(
        &mut self,
        state: &PathState,
        candidate: RankedCareer,
        interpolation_scratch: &mut Vec<f64>,
    ) -> Option<Value> {
        let RankedCareer { ceiling, lineage } = candidate;
        let trainee = lineage.trainee;
        let LineageChances { chances: inheritance_chances, carriers, spark_scale } =
            self.env.lineage(&state.pool, lineage);
        let reference_bound = self.evaluator.cache.refs[trainee]
            .iter()
            .map(|reference| self.env.ref_bound(trainee, &inheritance_chances, spark_scale, reference))
            .fold(ceiling, f64::min);
        let (table_value, deck_index) =
            self.evaluator.tval(self.env, trainee, &inheritance_chances, spark_scale, interpolation_scratch);
        self.env.set_trainee(trainee, &inheritance_chances);
        self.env.work = 0;
        let mut scored_decks = 0;
        let search_result = self.env.best_deck(trainee, spark_scale, 0.0, &[], &mut scored_decks);
        self.env.last_product_bound = None;
        let DeckRes::Done(value, deck, finished) = search_result else { return None };
        let exact_value = spark_scale * value;
        // landed sets: the cached argmax deck against the exact best deck (total variation distance)
        let landed_distance = if deck_index == usize::MAX {
            1.0
        } else {
            let cached = &self.evaluator.cache.decks[trainee][deck_index].sel;
            Self::landed_set_distance(self.env, trainee, &inheritance_chances, &carriers, cached, &deck)
        };
        let pick = state.best.is_some_and(|pick| pick.lineage == lineage);
        Some(json!([
            ceiling,
            reference_bound,
            table_value,
            exact_value,
            (exact_value - table_value) / exact_value,
            reference_bound <= table_value * (1.0 + self.evaluator.margin),
            landed_distance,
            finished,
            pick
        ]))
    }
}

/// A legal career and its ceiling (`Env::gen_cands`' order).
#[derive(Clone, Copy)]
pub(super) struct RankedCareer {
    pub(super) ceiling: f64,
    pub(super) lineage: Lineage,
}
impl RankedCareer {
    /// Constructor for `Env::gen_cands`; the unused argument is the trainee position.
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

impl ExactDeck {
    fn audit_row(&self, kind: AuditKind, table_value: f64, landed_distance: f64) -> Value {
        let exact_value = self.value;
        json!([
            kind as u8,
            table_value,
            exact_value,
            if exact_value > 0.0 { (exact_value - table_value) / exact_value } else { 0.0 },
            landed_distance,
            self.finished
        ])
    }
}
