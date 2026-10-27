//! The pair search's per-trainee bound (`robust`'s `CheckWork::pair_search`, docs/SOLVER.md "Next career"): the pairs
//! of one trainee it scores, in either phase. A trainee with at most `PAIR_SEARCH_PAIRS` runnable pairs has all of them
//! scored, in the order the pair search always took (`CareerSetup::ranked_pairs`); past it, the `PAIR_SEARCH_PAIRS`
//! best by the pair's P(all) ceiling, ties by the parents' identities (`Legacy::identity`), so the pairs scored are a
//! function of the veterans, not of their order in the pool.
use crate::career::plan::ScoringPool;
use crate::career::{Env, Legacy, Lineage, RecordIdentity};

/// The most pairs the pair search scores for one trainee, in phase 1 and in phase 2. The model's best pair for a
/// trainee sat at ceiling rank 2 to 11 of her best 40 on the pools the pair sweep measured (p10, p16, gm16_nu8,
/// 2026-10-26; nothing measured past rank 40); the 13 val/plan pools have at most 108 runnable pairs per trainee, so
/// the bound changes nothing there. At 116 legacies the tied Tamamo Cross has 3,791 (21 s unbounded). It bounds the
/// pairs scored, not their scores: a pair past it is not scored, whatever it would have saved
pub(super) const PAIR_SEARCH_PAIRS: usize = 128;

/// A runnable pair as the bound ranks it (`bounded_pairs`): its parents (legacy indices, as `ranked_pairs` gave
/// them), its P(all) ceiling with the parents in identity order, and their identity ranks (`identity_ranks`),
/// ascending.
#[derive(Clone, Copy)]
pub(super) struct BoundPair {
    pub(super) first_parent: usize,
    pub(super) second_parent: usize,
    pub(super) ceiling: f64,
    pub(super) ranks: [usize; 2],
}

/// One trainee's pairs the pair search scores (`trainee_bounded_pairs`), and how many she could run.
pub(super) struct BoundedPairs {
    pub(super) pairs: Vec<(usize, usize)>,
    pub(super) runnable: usize,
}

impl BoundedPairs {
    /// Whether the bound left some of her runnable pairs unscored.
    pub(super) const fn truncated(&self) -> bool {
        self.runnable > self.pairs.len()
    }
}

/// Each legacy's place among the distinct identities of `identities` (`Legacy::identity`: by hash, then by the
/// record's text), equal records sharing one: a total order on the records, whatever their positions
pub(super) fn identity_ranks(identities: &[RecordIdentity]) -> Vec<usize> {
    let mut distinct: Vec<&RecordIdentity> = identities.iter().collect();
    distinct.sort_unstable();
    distinct.dedup();
    identities.iter().map(|identity| distinct.binary_search(&identity).unwrap()).collect()
}

/// The pairs `bound` keeps of `pairs`: all of them, in their order, where they fit; else the `bound` best by ceiling,
/// equal ceilings by the parents' identity ranks (each pair's ascending, compared in turn). Past the bound, the pairs
/// kept and their order are a function of the pairs' ceilings and identities, not of the order `pairs` came in (two
/// pairs equal in both are pairs of the same two records, interchangeable)
pub(super) fn bounded_pairs(mut pairs: Vec<BoundPair>, bound: usize) -> Vec<BoundPair> {
    if pairs.len() <= bound {
        return pairs;
    }
    pairs.sort_by(|first, second| second.ceiling.total_cmp(&first.ceiling).then(first.ranks.cmp(&second.ranks)));
    pairs.truncate(bound);
    pairs
}

/// `trainee`'s runnable pairs of `pool` (its pick pool's rules and fixed parents) kept by `keep`, at most `bound`
/// (`bounded_pairs`; the pair search's is `PAIR_SEARCH_PAIRS`). Past the bound each pair's ceiling is taken again with
/// its parents in identity order (`ranked_pairs` takes them in position order, and the slots' factors multiply in
/// parent order, so its ceilings can differ by rounding when the pool is reordered; two parents of one identity are the
/// same record, either order alike). Within `aside`; it leaves the search state's folded buffer at some pair's chances,
/// as `ranked_pairs` does (every reader sets it first)
pub(super) fn trainee_bounded_pairs(
    env: &mut Env,
    pool: ScoringPool,
    trainee: crate::uma::UmaIndex,
    keep: impl Fn(usize, usize) -> bool,
    bound: usize,
) -> BoundedPairs {
    let pairs: Vec<(usize, usize)> = env
        .setup
        .ranked_pairs(&mut env.state, pool.pick_pool, pool.fixed_parents, trainee, keep)
        .map(|pair| (pair.first_parent, pair.second_parent))
        .collect();
    let runnable = pairs.len();
    if runnable <= bound {
        return BoundedPairs { pairs, runnable };
    }
    let identities: Vec<RecordIdentity> = pool.legacies.iter().map(Legacy::identity).collect();
    let ranks = identity_ranks(&identities);
    let mut chances = vec![];
    let ranked: Vec<BoundPair> = pairs
        .iter()
        .map(|&(first_parent, second_parent)| {
            let in_identity_order = ranks[first_parent] <= ranks[second_parent];
            let (low, high) =
                if in_identity_order { (first_parent, second_parent) } else { (second_parent, first_parent) };
            let lineage = Lineage { trainee, first_parent: low, second_parent: high };
            let spark_scale = env.setup.lineage_into(pool.pick_pool, lineage, &mut chances);
            // (`ranked_pairs` yields pairs only for a trainee with legal deck odds, where `ceiling` is Some)
            let ceiling = env.setup.ceiling(&mut env.state, trainee, &chances, spark_scale).unwrap_or(0.0);
            BoundPair { first_parent, second_parent, ceiling, ranks: [ranks[low], ranks[high]] }
        })
        .collect();
    let pairs = bounded_pairs(ranked, bound).into_iter().map(|pair| (pair.first_parent, pair.second_parent)).collect();
    BoundedPairs { pairs, runnable }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity(hash: u64, text: &str) -> RecordIdentity {
        RecordIdentity { hash, text: text.to_string() }
    }

    /// Ranks are a total order on the records: a hash collision between two different records falls to their text
    /// (each its own rank, in text order), and equal records share one, whatever the order they come in
    #[test]
    fn identity_ranks_break_a_hash_collision_by_the_record() {
        let pool = [identity(7, "b"), identity(7, "a"), identity(3, "z"), identity(7, "b")];
        assert_eq!(identity_ranks(&pool), [2, 1, 0, 2]);
        let mut reversed = pool.clone();
        reversed.reverse();
        assert_eq!(identity_ranks(&reversed), [2, 0, 1, 2]);
        // (colliding records with equal ceilings: the bound keeps the same records' pairs in either order)
        let kept = |pool: &[RecordIdentity]| {
            let ranks = identity_ranks(pool);
            let mut pairs = vec![];
            for first in 0..pool.len() {
                for second in first + 1..pool.len() {
                    let mut pair_ranks = [ranks[first], ranks[second]];
                    pair_ranks.sort_unstable();
                    pairs.push(BoundPair {
                        first_parent: first,
                        second_parent: second,
                        ceiling: 1.0,
                        ranks: pair_ranks,
                    });
                }
            }
            let mut texts: Vec<[String; 2]> = bounded_pairs(pairs, 2)
                .iter()
                .map(|pair| [pool[pair.first_parent].text.clone(), pool[pair.second_parent].text.clone()])
                .map(|mut texts| {
                    texts.sort_unstable();
                    texts
                })
                .collect();
            texts.sort_unstable();
            texts
        };
        assert_eq!(kept(&pool), kept(&reversed));
        assert_eq!(kept(&pool), [["a".to_string(), "z".to_string()], ["b".to_string(), "z".to_string()]]);
    }

    /// Past the bound the pairs kept are the same veterans' pairs whatever the pool's order: legacy records whose
    /// identities break exact ties in the ceiling, the pool reversed and rotated, the kept pairs compared by record
    #[test]
    fn the_bound_keeps_the_same_pairs_in_any_pool_order() {
        let records: Vec<Legacy> = (0..9u32)
            .map(|k| serde_json::from_value(serde_json::json!({"chara": 1001 + k, "sparks": [[20000 + k, 1 + k % 3]]})))
            .collect::<Result<_, _>>()
            .unwrap();
        // a ceiling of the two records alone, with many exact ties (it reads only the sparks' stars)
        let stars = |legacy: &Legacy| f64::from(legacy.sparks[0].1);
        let kept = |pool: &[Legacy]| {
            let identities: Vec<RecordIdentity> = pool.iter().map(Legacy::identity).collect();
            let ranks = identity_ranks(&identities);
            let mut pairs = vec![];
            for first in 0..pool.len() {
                for second in first + 1..pool.len() {
                    let mut pair_ranks = [ranks[first], ranks[second]];
                    pair_ranks.sort_unstable();
                    let ceiling = stars(&pool[first]) * stars(&pool[second]);
                    pairs.push(BoundPair { first_parent: first, second_parent: second, ceiling, ranks: pair_ranks });
                }
            }
            let mut kept: Vec<[u32; 2]> = bounded_pairs(pairs, 10)
                .iter()
                .map(|pair| {
                    let (first, second) = (pool[pair.first_parent].character_id, pool[pair.second_parent].character_id);
                    [first.min(second), first.max(second)]
                })
                .collect();
            kept.sort_unstable();
            kept
        };
        let in_order = kept(&records);
        assert_eq!(in_order.len(), 10);
        let mut reversed = records.clone();
        reversed.reverse();
        let mut rotated = records;
        rotated.rotate_left(4);
        assert_eq!(kept(&reversed), in_order);
        assert_eq!(kept(&rotated), in_order);
        // (within the bound every pair is kept, in the order given)
        let few = vec![
            BoundPair { first_parent: 0, second_parent: 1, ceiling: 0.1, ranks: [5, 6] },
            BoundPair { first_parent: 0, second_parent: 2, ceiling: 0.3, ranks: [5, 7] },
        ];
        let order: Vec<usize> = bounded_pairs(few, 2).iter().map(|pair| pair.second_parent).collect();
        assert_eq!(order, [1, 2]);
    }

    /// With the real ceilings (`trainee_bounded_pairs` on val/plan p16's legacies, its job's setup): every trainee with
    /// more runnable pairs than a bound of 20 keeps the same 20 pairs, by record, in the pool's order and reversed
    #[test]
    fn the_real_ceilings_keep_the_same_pairs_in_either_pool_order() {
        crate::career::tests::with_env(Box::new(|env: &mut Env, _: &crate::career::pool::Pool| {
            let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p16.json");
            let job: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            let legacies: Vec<Legacy> = serde_json::from_value(job["legacies"].clone()).unwrap();
            let mut reversed = legacies.clone();
            reversed.reverse();
            let trainees = env.setup.owned.clone();
            let kept = |env: &mut Env, legacies: &[Legacy], trainee: crate::uma::UmaIndex| {
                let pool = env.setup.pool(legacies);
                let scoring = ScoringPool { legacies, pool: &pool, pick_pool: &pool, fixed_parents: &[] };
                let bounded = trainee_bounded_pairs(env, scoring, trainee, |_, _| true, 20);
                let mut kept: Vec<[String; 2]> = bounded
                    .pairs
                    .iter()
                    .map(|&(first, second)| {
                        let mut texts = [legacies[first].identity().text, legacies[second].identity().text];
                        texts.sort_unstable();
                        texts
                    })
                    .collect();
                kept.sort_unstable();
                (bounded.truncated(), kept)
            };
            let mut truncated = 0;
            for trainee in trainees {
                let in_order = kept(env, &legacies, trainee);
                assert_eq!(in_order, kept(env, &reversed, trainee), "trainee {trainee}");
                truncated += usize::from(in_order.0);
            }
            assert!(truncated > 0, "no trainee past the bound");
        }));
    }
}
