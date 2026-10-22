//! Kept plans and cached career searches.
use crate::deck::career::sim::{Env, Found, Pool, Ref, Sim, Value, json};

/// A cached result may be available from searching, certification or reuse. A known empty result is
/// reusable too; an unsearched entry still needs work. Completeness is tracked separately.
#[derive(Clone)]
pub(in crate::deck::career::sim::plan) enum CachedSearch {
    Unsearched,
    Known(Option<Found>),
}

impl CachedSearch {
    pub(in crate::deck::career::sim::plan) const fn is_unsearched(&self) -> bool {
        matches!(self, Self::Unsearched)
    }

    /// The found career, if any. Callers deciding whether to search must use `is_unsearched` instead.
    pub(in crate::deck::career::sim::plan) const fn found(&self) -> Option<&Found> {
        match self {
            Self::Unsearched => None,
            Self::Known(found) => found.as_ref(),
        }
    }
}

/// Preserve the stored distinction: unsearched entries are absent, searched empty entries are explicit nulls.
fn cached_searches_out(searches: &[CachedSearch]) -> Vec<Value> {
    searches
        .iter()
        .enumerate()
        .filter_map(|(trainee, search)| match search {
            CachedSearch::Unsearched => None,
            CachedSearch::Known(found) => Some(json!([trainee, found.as_ref().map(crate::deck::career::found_out)])),
        })
        .collect()
}

/// Each trainee's best career (`Env::solve`, one trainee, top 1) and the decks that search checked, per plan: kept for
/// the next plan of the same setup and places whose pool is this one plus one veteran at the end (the page's Compare
/// and Reroll: your veterans and her). Every career of the larger pool is one of this pool's or has her as a parent, so
/// the best is the better of the kept one and the best with her as a parent (a search with her fixed): exact, the same
/// value; on an exact tie the kept career stands.
/// `CachedSearch` distinguishes an unsearched trainee from a search that returned no career.
/// Search completeness is separate, in `unp`/`lunp`.
pub(in crate::deck::career) struct Bests {
    pub(in crate::deck::career::sim::plan) key: (u64, u64),
    pub(in crate::deck::career::sim::plan) best: Vec<CachedSearch>,
    pub(in crate::deck::career::sim::plan) refs: Vec<Vec<Vec<Ref>>>,
    /// her best looped career (`index_rule`), where searched: kept the same way (a pool plus one veteran adds looped pairs
    /// only when she is an 8/8)
    pub(in crate::deck::career::sim::plan) looped: Vec<CachedSearch>,
    /// per uma whether a search behind her kept best (and her looped best) was not complete (`Solved::complete` false: it
    /// ran out of work, a band overflowed, or a scored deck passed its class's cap)
    pub(in crate::deck::career::sim::plan) unp: Vec<bool>,
    pub(in crate::deck::career::sim::plan) lunp: Vec<bool>,
}

thread_local! { pub(in crate::deck::career::sim::plan) static BESTS: std::cell::RefCell<Vec<Bests>> = const { std::cell::RefCell::new(Vec::new()) }; }

/// the kept plans for the page to store (`state_out`): per plan its key and every uma's kept best (her decks' refs are
/// not kept: nothing reads them back)
pub(in crate::deck::career) fn bests_out() -> Value {
    BESTS.with(|c| Value::Array(c.borrow().iter().map(|b| json!({"k": [crate::deck::career::ux(b.key.0), crate::deck::career::ux(b.key.1)], "n": b.best.len(),
        "b": cached_searches_out(&b.best),
        "l": cached_searches_out(&b.looped),
        "u": (0..b.unp.len()).filter(|&u| b.unp[u]).collect::<Vec<_>>(), "lu": (0..b.lunp.len()).filter(|&u| b.lunp[u]).collect::<Vec<_>>()})).collect()))
}

pub(in crate::deck::career) fn bests_in(v: &Value) -> Option<Vec<Bests>> {
    use crate::deck::career::{found_in, len_in, usz, uxr};
    v.as_array()?
        .iter()
        .map(|e| {
            let k = e["k"].as_array()?;
            // (checked conversions: on wasm32 usize is 32 bits, and a stored count or index must not wrap into range)
            let n = len_in(&e["n"])?;
            let per_uma = |key: &str| -> Option<Vec<CachedSearch>> {
                let mut v = vec![CachedSearch::Unsearched; n];
                for x in e[key].as_array()? {
                    let a = x.as_array()?;
                    let u = usz(a.first()?)?;
                    if u >= n {
                        return None;
                    }
                    v[u] = CachedSearch::Known(if a.get(1)?.is_null() { None } else { Some(found_in(a.get(1)?)?) });
                }
                Some(v)
            };
            // Older stored states omit these flags; those searches are treated as proven.
            let flags = |key: &str| -> Vec<bool> {
                let mut flags = vec![false; n];
                for entry in e[key].as_array().into_iter().flatten() {
                    if let Some(trainee) = usz(entry)
                        && trainee < n
                    {
                        flags[trainee] = true;
                    }
                }
                flags
            };
            Some(Bests {
                key: (uxr(k.first()?)?, uxr(k.get(1)?)?),
                best: per_uma("b")?,
                refs: vec![vec![]; n],
                looped: per_uma("l")?,
                unp: flags("u"),
                lunp: flags("lu"),
            })
        })
        .collect()
}

pub(in crate::deck::career) fn bests_set(b: Vec<Bests>) {
    BESTS.with(|c| *c.borrow_mut() = b);
}

/// the kept plans, taken out (the simulator's `index` policy starts each run without them, `sim::kept_swap`)
pub(in crate::deck::career) fn bests_take() -> Vec<Bests> {
    BESTS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}

/// Remove one kept plan while `index_rule` owns it; the plan puts retained entries back when it finishes.
pub(in crate::deck::career::sim::plan) fn take_kept_plan(key: (u64, u64)) -> Option<Bests> {
    BESTS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|plan| plan.key == key);
        index.map(|index| cache.remove(index))
    })
}

pub(in crate::deck::career::sim::plan) fn h64(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

/// (setup and places, the first `n` legacies; the borrowed-parent rule unless "allow": `setup_fp` leaves it out, and a
/// plan under "later" or "never" keeps other looped careers, under "never" other best careers too)
pub(in crate::deck::career::sim::plan) fn bests_key(job: &Value, fix: &[usize], n: usize) -> (u64, u64) {
    // (the slice's text as the array's: the same serializer, no copy)
    let ls = job
        .get("legacies")
        .and_then(|l| l.as_array())
        .map(|a| serde_json::to_string(&a[..n.min(a.len())]).unwrap())
        .unwrap_or_default();
    let bp = job
        .get("borrowed_parent")
        .and_then(|b| b.as_str())
        .filter(|&b| b != "allow")
        .map_or(0, |b| h64(b).rotate_left(2));
    (crate::deck::career::setup_fp(job) ^ h64(&format!("{fix:?}")).rotate_left(1) ^ bp, h64(&ls))
}

/// trainee u's best career in `pool` (and the refs to absorb, and whether every search behind it was complete,
/// `Solved::complete`), reusing a kept plan of the pool less its last veteran
pub(in crate::deck::career::sim::plan) fn best_of(
    env: &mut Env,
    pool: &Pool,
    fix: &[usize],
    u: usize,
    same: Option<&Bests>,
    prev: Option<&Bests>,
    nl: usize,
) -> (Option<Found>, Vec<Vec<Ref>>, bool) {
    // a kept plan of this very pool (the same job planned again: a reload, a setting changed and back): her best, as kept
    if let Some(same) = same
        && let Some(CachedSearch::Known(found)) = same.best.get(u)
    {
        return (found.clone(), vec![], !same.unp.get(u).copied().unwrap_or(false));
    }
    if let Some(previous) = prev
        && let Some(CachedSearch::Known(old)) = previous.best.get(u)
    {
        let mut refs = previous.refs.get(u).cloned().unwrap_or_default();
        let kept_ok = !previous.unp.get(u).copied().unwrap_or(false);
        if fix.len() >= 2 {
            return (old.clone(), refs, kept_ok);
        } // both parents fixed: she can't be one
        let mut fixed_with_new_veteran = fix.to_vec();
        fixed_with_new_veteran.push(nl - 1);
        // no career with her can beat the kept one (each one's ceiling, as `solve` takes them, is below it): kept, exact
        if let Some(o) = old {
            let her = nl - 1;
            // (her legal pairs, `gen_cands`' ceilings: `ceiling` at `lineage`'s chances)
            // (their maximum folded from 0 in generation order, as a fold over the collected ceilings; no Vec: `()` items)
            let mut ub = 0.0f64;
            env.gen_cands(
                pool,
                fix,
                &[u],
                |_, b| b == her,
                |x, _, _, _, _| {
                    ub = f64::max(ub, x);
                },
            );
            if ub * (1.0 + 1e-9) < o.val {
                return (Some(o.clone()), refs, kept_ok);
            }
        }
        // only a career above the kept one matters: the search's bar starts there (exact; a tie keeps the kept one)
        env.floor = old.as_ref().map_or(f64::NEG_INFINITY, |o| o.val);
        let searched = env.solve(pool, &fixed_with_new_veteran, &[u], false, 1);
        env.floor = f64::NEG_INFINITY;
        if refs.len() < searched.refs.len() {
            refs.resize(searched.refs.len(), vec![]);
        }
        for (trainee, checked) in searched.refs.into_iter().enumerate() {
            refs[trainee].extend(checked);
        }
        let ok = kept_ok && searched.complete;
        let new = searched.found.into_iter().next();
        let best = match (old, new) {
            (Some(old), Some(new)) => Some(if new.val > old.val { new } else { old.clone() }),
            (old, new) => old.clone().or(new),
        };
        return (best, refs, ok);
    }
    let sr = env.solve(pool, fix, &[u], false, 1);
    let ok = sr.complete;
    (sr.found.into_iter().next(), sr.refs, ok)
}

/// What `score_career` reads that grows while a plan runs: the deck cache's decks and fully checked decks (`Cache`,
/// per trainee, both only grow) summed over the trainees `us` (None: every trainee), and the deck searches run so far
/// (`deck_for`'s: the only work in a scoring that changes what a later one reads)
fn sver(s: &Sim, us: Option<&[usize]>) -> (usize, usize, usize) {
    let (decks, proofs) = match us {
        Some(trainees) => trainees.iter().fold((0, 0), |(decks, proofs), &trainee| {
            (decks + s.cache.decks[trainee].len(), proofs + s.cache.refs[trainee].len())
        }),
        None => (s.cache.n_decks(), s.cache.refs.iter().map(std::vec::Vec::len).sum()),
    };
    (decks, proofs, s.n.searches)
}

/// A scoring with its stamp: the cache it read (`sver` over `us`, the trainees whose decks it reads) when it ran no deck
/// search and added nothing, else None. A scoring of the same career against the same base, run when `sver` is again
/// the stamp, reads the same decks and tables, takes the same branches (whether `deck_for` searches is decided by those
/// alone) and gives the same f64s (`stamp_ok`), so it need not run
pub(in crate::deck::career::sim::plan) fn stamped<R>(
    s: &mut Sim,
    us: Option<&[usize]>,
    f: impl FnOnce(&mut Sim) -> R,
) -> (R, Option<(usize, usize)>) {
    let before = sver(s, us);
    let result = f(s);
    let after = sver(s, us);
    (result, (before == after).then_some((after.0, after.1)))
}

pub(in crate::deck::career::sim::plan) fn stamp_ok(
    s: &Sim,
    us: Option<&[usize]>,
    stamp: Option<(usize, usize)>,
) -> bool {
    let current = sver(s, us);
    stamp == Some((current.0, current.1))
}

/// the same career as `score_career` reads it: trainee, parents in order, every field of the deck (floats by bits). Not
/// its value (`Found::val`, nor `ub`): `score_career` never reads them, and a proxy's value (the surrogate's corner-table
/// interpolation) and the search's exact one differ in their last bits
pub(in crate::deck::career::sim::plan) fn same_career(f: &Found, g: &Found) -> bool {
    let bits = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
    f.t == g.t
        && f.a == g.a
        && f.b == g.b
        && f.sel.ents == g.sel.ents
        && f.sel.plain == g.sel.plain
        && f.sel.filler == g.sel.filler
        && bits(&f.sel.dw, &g.sel.dw)
        && bits(&f.sel.dg, &g.sel.dg)
}

#[cfg(test)]
mod cached_search_tests {
    use super::*;
    use crate::deck::Sel;

    #[test]
    fn stored_searches_distinguish_missing_empty_and_found() {
        let found = Found {
            t: 2,
            a: 0,
            b: 1,
            val: 0.25,
            ub: 0.5,
            sel: Sel { ents: vec![1], plain: 1, filler: 0, dw: vec![-0.0], dg: vec![0.5] },
        };
        let searches =
            vec![CachedSearch::Unsearched, CachedSearch::Known(None), CachedSearch::Known(Some(found.clone()))];
        let rows = json!(cached_searches_out(&searches));
        assert_eq!(rows, json!([[1, null], [2, crate::deck::career::found_out(&found)]]));

        let stored = json!([{
            "k": ["0000000000000001", "0000000000000002"], "n": 3,
            "b": rows, "l": rows, "u": [2], "lu": [1],
        }]);
        let decoded = bests_in(&stored).unwrap();
        let cached = &decoded[0];
        assert!(cached.best[0].is_unsearched());
        assert!(matches!(cached.best[1], CachedSearch::Known(None)));
        assert!(!cached.best[1].is_unsearched());
        assert!(cached.best[1].found().is_none());
        assert_eq!(cached.best[2].found().unwrap().sel.dw[0].to_bits(), (-0.0f64).to_bits());
        assert_eq!(json!(cached_searches_out(&cached.best)), stored[0]["b"]);
        assert_eq!(json!(cached_searches_out(&cached.looped)), stored[0]["l"]);
        assert_eq!(cached.unp, [false, false, true]);
        assert_eq!(cached.lunp, [false, true, false]);

        // counts and indices are converted checked (usize is 32 bits on wasm32): an oversized count is refused, an
        // index past u32::MAX is out of range (not wrapped into it), an unusable flag is skipped
        let wide = u64::from(u32::MAX) + 2;
        let refused =
            |n: Value, b: Value| bests_in(&json!([{"k": stored[0]["k"], "n": n, "b": b, "l": [], "u": [], "lu": []}]));
        assert!(refused(json!(u64::MAX), json!([])).is_none(), "a count beyond STATE_MAX_N is refused");
        assert!(refused(json!(3), json!([[wide, null]])).is_none(), "an index past u32::MAX is refused");
        let flagged = json!([{"k": stored[0]["k"], "n": 3, "b": [], "l": [], "u": [wide, 1], "lu": []}]);
        assert_eq!(bests_in(&flagged).unwrap()[0].unp, [false, true, false]);
    }
}
