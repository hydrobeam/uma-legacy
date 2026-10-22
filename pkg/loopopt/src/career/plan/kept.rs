//! Kept plans and cached career searches.
use crate::career::Env;
use crate::career::plan::ParentPool;
use crate::career::search::{Found, Ref};
use crate::career::sim::Sim;
use crate::career::state::hash_text;
use crate::uma::{UmaIndex, UmaTable};
use serde_json::{Value, json};
/// A cached result may be available from searching, certification or reuse. A known empty result is
/// reusable too; an unsearched entry still needs work. Completeness is tracked separately.
#[derive(Clone)]
pub(crate) enum CachedSearch {
    Unsearched,
    Known(Option<Found>),
}

impl CachedSearch {
    pub(crate) const fn is_unsearched(&self) -> bool {
        matches!(self, Self::Unsearched)
    }

    /// The found career, if any. Callers deciding whether to search must use `is_unsearched` instead.
    pub(crate) const fn found(&self) -> Option<&Found> {
        match self {
            Self::Unsearched => None,
            Self::Known(found) => found.as_ref(),
        }
    }
}

/// Preserve the stored distinction: unsearched entries are absent, searched empty entries are explicit nulls.
fn cached_searches_out(searches: &UmaTable<CachedSearch>) -> Vec<Value> {
    searches
        .iter()
        .enumerate()
        .filter_map(|(trainee, search)| match search {
            CachedSearch::Unsearched => None,
            CachedSearch::Known(found) => Some(json!([trainee, found.as_ref().map(crate::career::state::found_out)])),
        })
        .collect()
}

/// Each trainee's best career (`CareerSetup::solve`, one trainee, top 1) and the decks that search checked, per plan:
/// kept for the next plan of the same setup and places whose pool is this one plus one veteran at the end (the page's
/// Compare and Reroll: your veterans and her). Every career of the larger pool is one of this pool's or has her as a
/// parent, so the best is the better of the kept one and the best with her as a parent (a search with her fixed):
/// exact, the same value; on an exact tie the kept career stands.
/// `CachedSearch` distinguishes an unsearched trainee from a search that returned no career.
/// Search completeness is separate, in `base_incomplete` / `looped_incomplete`. Every vector is indexed by uma
/// (`Data::umas`).
pub(crate) struct Bests {
    /// The plan's setup and the pool it was planned on (`bests_key`).
    pub(crate) key: (u64, u64),
    /// Her best career.
    pub(crate) best_careers: UmaTable<CachedSearch>,
    /// Per trainee, the decks her best career's search checked fully (refs to absorb into the next search).
    pub(crate) proof_refs: UmaTable<UmaTable<Vec<Ref>>>,
    /// her best looped career (`index_rule`), where searched: kept the same way (a pool plus one veteran adds looped pairs
    /// only when she is a full veteran)
    pub(crate) looped_careers: UmaTable<CachedSearch>,
    /// whether a search behind her kept best (and her looped best) was not complete (`CareerSearchResult::complete` false: it ran
    /// out of work, a band overflowed, or a scored deck passed its class's cap)
    pub(crate) base_incomplete: UmaTable<bool>,
    pub(crate) looped_incomplete: UmaTable<bool>,
}

/// the kept plans for the page to store (`state_out`): per plan its key and every uma's kept best (her decks' refs are
/// not kept: nothing reads them back)
pub(crate) fn bests_out(bests: &[Bests]) -> Value {
    Value::Array(bests.iter().map(|b| json!({"k": [crate::career::state::ux(b.key.0), crate::career::state::ux(b.key.1)], "n": b.best_careers.len(),
        "b": cached_searches_out(&b.best_careers),
        "l": cached_searches_out(&b.looped_careers),
        "u": b.base_incomplete.indices().filter(|&u| b.base_incomplete[u]).collect::<Vec<_>>(),
        "lu": b.looped_incomplete.indices().filter(|&u| b.looped_incomplete[u]).collect::<Vec<_>>()})).collect())
}

pub(crate) fn bests_in(v: &Value) -> Option<Vec<Bests>> {
    use crate::career::state::{found_in, len_in, usz, uxr};
    v.as_array()?
        .iter()
        .map(|e| {
            let k = e["k"].as_array()?;
            // (checked conversions: on wasm32 usize is 32 bits, and a stored count or index must not wrap into range)
            let n = len_in(&e["n"])?;
            let per_uma = |key: &str| -> Option<UmaTable<CachedSearch>> {
                let mut v = UmaTable(vec![CachedSearch::Unsearched; n]);
                for x in e[key].as_array()? {
                    let a = x.as_array()?;
                    let u = usz(a.first()?)?;
                    if u >= n {
                        return None;
                    }
                    v[UmaIndex(u)] =
                        CachedSearch::Known(if a.get(1)?.is_null() { None } else { Some(found_in(a.get(1)?)?) });
                }
                Some(v)
            };
            // Older stored states omit these flags; those searches are treated as proven.
            let flags = |key: &str| -> UmaTable<bool> {
                let mut flags = UmaTable(vec![false; n]);
                for entry in e[key].as_array().into_iter().flatten() {
                    if let Some(trainee) = usz(entry)
                        && trainee < n
                    {
                        flags[UmaIndex(trainee)] = true;
                    }
                }
                flags
            };
            Some(Bests {
                key: (uxr(k.first()?)?, uxr(k.get(1)?)?),
                best_careers: per_uma("b")?,
                proof_refs: UmaTable(vec![UmaTable(vec![]); n]),
                looped_careers: per_uma("l")?,
                base_incomplete: flags("u"),
                looped_incomplete: flags("lu"),
            })
        })
        .collect()
}

/// Remove one kept plan while `index_rule` owns it; the plan puts retained entries back when it finishes.
pub(crate) fn take_kept_plan(kept: &mut crate::career::state::KeptState, key: (u64, u64)) -> Option<Bests> {
    let index = kept.bests.iter().position(|plan| plan.key == key)?;
    Some(kept.bests.remove(index))
}

/// A kept plan's key: (setup and places, the first `legacy_count` legacies). The borrowed-parent rule is in the first
/// unless "allow": `setup_fp` leaves it out, and a plan under "later" or "never" keeps other looped careers, under
/// "never" other best careers too
pub(crate) fn bests_key(job: &Value, fixed_parents: &[usize], legacy_count: usize) -> (u64, u64) {
    // (the slice's text as the array's: the same serializer, no copy)
    let legacies_text = job
        .get("legacies")
        .and_then(|l| l.as_array())
        .map(|a| serde_json::to_string(&a[..legacy_count.min(a.len())]).unwrap())
        .unwrap_or_default();
    let borrowing_rule = job
        .get("borrowed_parent")
        .and_then(|b| b.as_str())
        .filter(|&b| b != "allow")
        .map_or(0, |b| hash_text(b).rotate_left(2));
    (
        crate::career::state::setup_fp(job) ^ hash_text(&format!("{fixed_parents:?}")).rotate_left(1) ^ borrowing_rule,
        hash_text(&legacies_text),
    )
}

/// The kept plans a career search may reuse (`Bests`), taken out of `KeptState::bests` while the plan runs.
#[derive(Clone, Copy)]
pub(crate) struct KeptPlans<'kept> {
    /// A kept plan of this very pool.
    pub(crate) same_pool: Option<&'kept Bests>,
    /// A kept plan of this pool less its last veteran.
    pub(crate) previous_pool: Option<&'kept Bests>,
    /// The pool's recorded veterans; the last of them, index `legacy_count - 1`, is the one `previous_pool` lacks.
    pub(crate) legacy_count: usize,
}

/// One trainee's best career search.
pub(crate) struct BestCareer {
    /// Her best career, if she has a legal one.
    pub(crate) found: Option<Found>,
    /// Per trainee, the decks the search checked fully: the refs to absorb.
    pub(crate) proof_refs: UmaTable<Vec<Ref>>,
    /// Every search behind it was complete (`CareerSearchResult::complete`).
    pub(crate) complete: bool,
}

/// `trainee`'s best career in the pool, reusing a kept plan of this pool or of the pool less its last veteran
pub(super) fn best_of(env: &mut Env, parents: ParentPool, trainee: UmaIndex, kept: KeptPlans) -> BestCareer {
    let ParentPool { pool, fixed_parents } = parents;
    // a kept plan of this very pool (the same job planned again: a reload, a setting changed and back): her best, as kept
    if let Some(same) = kept.same_pool
        && let Some(CachedSearch::Known(found)) = same.best_careers.get(trainee)
    {
        return BestCareer {
            found: found.clone(),
            proof_refs: UmaTable(vec![]),
            complete: !same.base_incomplete.get(trainee).copied().unwrap_or(false),
        };
    }
    if let Some(previous) = kept.previous_pool
        && let Some(CachedSearch::Known(old)) = previous.best_careers.get(trainee)
    {
        let mut proof_refs = previous.proof_refs.get(trainee).cloned().unwrap_or_default();
        let kept_complete = !previous.base_incomplete.get(trainee).copied().unwrap_or(false);
        if fixed_parents.len() >= 2 {
            // both parents fixed: she can't be one
            return BestCareer { found: old.clone(), proof_refs, complete: kept_complete };
        }
        let mut fixed_with_new_veteran = fixed_parents.to_vec();
        fixed_with_new_veteran.push(kept.legacy_count - 1);
        // no career with her can beat the kept one (each one's ceiling, as `solve` takes them, is below it): kept, exact
        if let Some(old_career) = old {
            let new_veteran = kept.legacy_count - 1;
            // (her legal pairs, `gen_cands`' ceilings: `ceiling` at `lineage`'s chances)
            // (their maximum folded from 0 in generation order, as a fold over the collected ceilings; no Vec: `()` items)
            let mut best_ceiling = 0.0f64;
            env.setup.gen_cands(
                &mut env.state,
                pool,
                fixed_parents,
                &[trainee],
                |_, second_parent| second_parent == new_veteran,
                |ceiling, _, _, _, _| {
                    best_ceiling = f64::max(best_ceiling, ceiling);
                },
            );
            if best_ceiling * (1.0 + 1e-9) < old_career.value {
                return BestCareer { found: Some(old_career.clone()), proof_refs, complete: kept_complete };
            }
        }
        // only a career above the kept one matters: the search's bar starts there (exact; a tie keeps the kept one)
        env.state.floor = old.as_ref().map_or(f64::NEG_INFINITY, |old_career| old_career.value);
        let searched = env.setup.solve(
            &mut env.state,
            &mut env.audit,
            &mut env.memo,
            pool,
            &fixed_with_new_veteran,
            &[trainee],
            false,
            1,
        );
        env.state.floor = f64::NEG_INFINITY;
        if proof_refs.len() < searched.refs.len() {
            proof_refs.resize(searched.refs.len(), vec![]);
        }
        for (checked_trainee, checked) in searched.refs.into_iter().enumerate() {
            proof_refs[UmaIndex(checked_trainee)].extend(checked);
        }
        let complete = kept_complete && searched.complete;
        let new = searched.found.into_iter().next();
        let found = match (old, new) {
            (Some(old), Some(new)) => Some(if new.value > old.value { new } else { old.clone() }),
            (old, new) => old.clone().or(new),
        };
        return BestCareer { found, proof_refs, complete };
    }
    let searched =
        env.setup.solve(&mut env.state, &mut env.audit, &mut env.memo, pool, fixed_parents, &[trainee], false, 1);
    let complete = searched.complete;
    BestCareer { found: searched.found.into_iter().next(), proof_refs: searched.refs, complete }
}

/// The part of the deck cache a scoring reads: its decks and fully checked decks (`Cache`, per trainee, both only grow)
/// summed over the trainees it reads.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) struct CacheStamp {
    decks: usize,
    proofs: usize,
}

/// What `score_career` reads that grows while a plan runs: the cache's stamp, and the deck searches run so far
/// (`deck_for`'s: the only work in a scoring that changes what a later one reads)
#[derive(PartialEq, Eq)]
struct CacheVersion {
    stamp: CacheStamp,
    deck_searches: usize,
}

/// The cache's version summed over the trainees `trainees` (None: every trainee)
fn cache_version(s: &Sim, trainees: Option<&[UmaIndex]>) -> CacheVersion {
    let (decks, proofs) = match trainees {
        Some(trainees) => trainees.iter().fold((0, 0), |(decks, proofs), &trainee| {
            (decks + s.evaluator.cache.decks[trainee].len(), proofs + s.evaluator.cache.refs[trainee].len())
        }),
        None => (s.evaluator.cache.deck_count(), s.evaluator.cache.refs.iter().map(std::vec::Vec::len).sum()),
    };
    CacheVersion { stamp: CacheStamp { decks, proofs }, deck_searches: s.evaluator.n.searches }
}

/// A scoring with its stamp: the cache it read (`cache_version` over `trainees`, those whose decks it reads) when it
/// ran no deck search and added nothing, else None. A scoring of the same career against the same base, run when the
/// cache is again at the stamp, reads the same decks and tables, takes the same branches (whether `deck_for` searches
/// is decided by those alone) and gives the same f64s (`stamp_ok`), so it need not run
pub(crate) fn stamped<R>(
    s: &mut Sim,
    trainees: Option<&[UmaIndex]>,
    scoring: impl FnOnce(&mut Sim) -> R,
) -> (R, Option<CacheStamp>) {
    let before = cache_version(s, trainees);
    let result = scoring(s);
    let after = cache_version(s, trainees);
    let unchanged = before == after;
    (result, unchanged.then_some(after.stamp))
}

pub(crate) fn stamp_ok(s: &Sim, trainees: Option<&[UmaIndex]>, stamp: Option<CacheStamp>) -> bool {
    stamp == Some(cache_version(s, trainees).stamp)
}

/// the same career as `score_career` reads it: trainee, parents in order, every field of the deck (floats by bits). Not
/// its value (`Found::value`, nor `ceiling`): `score_career` never reads them, and a proxy's value (the surrogate's
/// corner-table interpolation) and the search's exact one differ in their last bits
pub(crate) fn same_career(f: &Found, g: &Found) -> bool {
    let bits = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
    f.trainee == g.trainee
        && f.first_parent == g.first_parent
        && f.second_parent == g.second_parent
        && f.deck.entries == g.deck.entries
        && f.deck.plain_pals == g.deck.plain_pals
        && f.deck.filler_cards == g.deck.filler_cards
        && bits(&f.deck.white_hint_chances, &g.deck.white_hint_chances)
        && bits(&f.deck.gold_hint_chances, &g.deck.gold_hint_chances)
}

#[cfg(test)]
mod cached_search_tests {
    use super::*;
    use crate::deck::space::SelectedDeck;
    #[test]
    fn stored_searches_distinguish_missing_empty_and_found() {
        let found = Found {
            trainee: UmaIndex(2),
            first_parent: 0,
            second_parent: 1,
            value: 0.25,
            ceiling: 0.5,
            deck: SelectedDeck {
                entries: vec![1],
                plain_pals: 1,
                filler_cards: 0,
                white_hint_chances: vec![-0.0],
                gold_hint_chances: vec![0.5],
            },
        };
        let searches = UmaTable(vec![
            CachedSearch::Unsearched,
            CachedSearch::Known(None),
            CachedSearch::Known(Some(found.clone())),
        ]);
        let rows = json!(cached_searches_out(&searches));
        assert_eq!(rows, json!([[1, null], [2, crate::career::state::found_out(&found)]]));

        let stored = json!([{
            "k": ["0000000000000001", "0000000000000002"], "n": 3,
            "b": rows, "l": rows, "u": [2], "lu": [1],
        }]);
        let decoded = bests_in(&stored).unwrap();
        let cached = &decoded[0];
        assert!(cached.best_careers[UmaIndex(0)].is_unsearched());
        assert!(matches!(cached.best_careers[UmaIndex(1)], CachedSearch::Known(None)));
        assert!(!cached.best_careers[UmaIndex(1)].is_unsearched());
        assert!(cached.best_careers[UmaIndex(1)].found().is_none());
        assert_eq!(
            cached.best_careers[UmaIndex(2)].found().unwrap().deck.white_hint_chances[0].to_bits(),
            (-0.0f64).to_bits()
        );
        assert_eq!(json!(cached_searches_out(&cached.best_careers)), stored[0]["b"]);
        assert_eq!(json!(cached_searches_out(&cached.looped_careers)), stored[0]["l"]);
        assert_eq!(cached.base_incomplete.0, [false, false, true]);
        assert_eq!(cached.looped_incomplete.0, [false, true, false]);

        // counts and indices are converted checked (usize is 32 bits on wasm32): an oversized count is refused, an
        // index past u32::MAX is out of range (not wrapped into it), an unusable flag is skipped
        let wide = u64::from(u32::MAX) + 2;
        let refused =
            |n: Value, b: Value| bests_in(&json!([{"k": stored[0]["k"], "n": n, "b": b, "l": [], "u": [], "lu": []}]));
        assert!(refused(json!(u64::MAX), json!([])).is_none(), "a count beyond STATE_MAX_N is refused");
        assert!(refused(json!(3), json!([[wide, null]])).is_none(), "an index past u32::MAX is refused");
        let flagged = json!([{"k": stored[0]["k"], "n": 3, "b": [], "l": [], "u": [wide, 1], "lu": []}]);
        assert_eq!(bests_in(&flagged).unwrap()[0].base_incomplete.0, [false, true, false]);
    }
}
