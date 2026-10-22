//! The worker's saved searches and their validation.
use super::sim;
use super::{Env, Found, Pool, Ref, Solved};
use crate::deck::{MAX_DECK, SelectedDeck, Value, json};

/// A job's setup fingerprint: everything but the pool and the places (targets, roster, cards, scenario, ...), the
/// key of what one job's search proves that holds for the next (p_bar, fully checked decks)
pub(crate) fn setup_fp(v: &Value) -> u64 {
    // (also the page's flags that change no deck value: `compare`, the plan's own fields, so the plan and compare
    // jobs share what the search jobs proved, and `borrowed_parent`, a rule on the pairs as `fix` is)
    setup_hash(&json_without(
        v,
        &[
            "legacies",
            "trainees",
            "fix",
            "top",
            "per_trainee",
            "cmd",
            "mode",
            "compare",
            "next",
            "no_reports",
            "borrowed_parent",
        ],
        &[],
    ))
}

/// The JSON text of `v` with its top-level keys `drop` left out, and of its `next` object's keys `next_drop`: the text of a
/// clone with those keys removed, byte for byte, without the clone. serde_json's `Map` is a `BTreeMap` here (no
/// `preserve_order`), so the kept keys are written in the same order, by the same serializer (`Value`'s `Display` is
/// `to_writer`'s compact form), and an object left empty is "{}" either way
pub(crate) fn json_without(v: &Value, drop: &[&str], next_drop: &[&str]) -> String {
    struct Strip<'val> {
        v: &'val Value,
        drop: &'val [&'val str],
        next_drop: &'val [&'val str],
    }
    impl serde::Serialize for Strip<'_> {
        fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
            match self.v {
                Value::Object(m) => {
                    s.collect_map(m.iter().filter(|(k, _)| !self.drop.contains(&k.as_str())).map(|(k, x)| {
                        (k, Strip { v: x, drop: if k == "next" { self.next_drop } else { &[] }, next_drop: &[] })
                    }))
                }
                x => x.serialize(s),
            }
        }
    }
    serde_json::to_string(&Strip { v, drop, next_drop }).unwrap()
}

pub(crate) fn setup_hash(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}

thread_local! {
    /// the last few jobs' main searches (the most recent last), each keyed by the whole job without `next` and
    /// `no_reports` (the page's plan job repeats the job it follows), with what the search's product searches proved
    /// (`Env::ruled` as the search left it: a job that takes a kept search starts its plan from the Env state a combined
    /// job's plan starts from); kept across page reloads (`state_out`)
    pub(crate) static LAST_SOLVE: std::cell::RefCell<Vec<KeptSolve>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// One kept main search (`LAST_SOLVE`).
pub(crate) struct KeptSolve {
    /// The job's key (`CareerRun::search_key`).
    pub(crate) key: u64,
    pub(crate) solved: Solved,
    /// What its product searches proved (`Env::ruled` as the search left it).
    pub(crate) ruled: Ruled,
}

/// per deck class, the product searches' proofs (`Env::ruled`)
pub(crate) type Ruled = crate::UmaTable<Vec<ProductProof>>;

/// What one product search proved (`best_deck` pass 1): every legal deck's product value at chances q (fixed factor
/// included, before the spark scale) is at most S.
#[derive(Clone)]
pub(crate) struct ProductProof {
    /// q: the inheritance chances the search ran at, per job target.
    pub(crate) inheritance_chances: Vec<f64>,
    /// S: the best product value at q.
    pub(crate) product_bound: f64,
}

/// main searches kept (`LAST_SOLVE`): the page's run, and one it goes back to (a setting changed and back, a Compare or
/// Reroll plan in between)
pub(crate) const SOLVE_KEEP: usize = 3;

thread_local! {
    /// per setup fingerprint (the last few), every fully checked deck found so far (`Env::seen`): what the page's
    /// earlier jobs proved bounds the next job's searches. A Ref holds a trainee's chances and her best exact value
    /// there, which depend on the setup only (the job's trainees change no deck value: they only order plain pals).
    pub(crate) static SEEN: std::cell::RefCell<Vec<(u64, crate::UmaTable<Vec<Ref>>)>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Fully checked decks kept per trainee across jobs (the most recent): the bound loop over them stays cheap
pub(crate) const SEEN_KEEP: usize = 256;

// ---------- the worker's kept work across page loads ----------
// What a worker keeps between jobs (the kept plans, `sim::plan::BESTS`; the proofs, `SEEN`) leaves as `state_out` and comes
// back as `state_in`, so the first plan after a page reload reuses the work of the last session. Every float travels as
// its bit pattern in hex (decimal JSON does not round-trip here), so what comes back is what left, bit for bit; the page
// keys what it stores by the solver and data fingerprints, so a release starts afresh (the keys inside, `setup_fp` and
// `bests_key`, hash with std's DefaultHasher, fixed within a build). A store that does not parse is refused whole.
pub(crate) const STATE_VERSION: u64 = 4;

/// What a stored state may hold, checked as it is decoded (`state_in`, before anything is installed): a stored count (the
/// umas or deck classes a list covers) at most STATE_MAX_N, so nothing is allocated
/// from a count past it; every index within the list it indexes; no float NaN (the bounds compare them); chances,
/// odds and deck hint chances within [0, 1]; a deck at most `MAX_DECK` cards. What a store says of a setup (deck
/// entries, chances per target) is checked where it meets that setup (`Env::seen_fits`, `solve_kept`).
pub(crate) const STATE_MAX_N: usize = 1 << 10;

pub(crate) fn hx(x: f64) -> Value {
    Value::String(format!("{:016x}", x.to_bits()))
}

pub(crate) fn ux(x: u64) -> Value {
    Value::String(format!("{x:016x}"))
}

pub(crate) fn fx(v: &Value) -> Option<f64> {
    Some(f64::from_bits(u64::from_str_radix(v.as_str()?, 16).ok()?)).filter(|x| !x.is_nan())
}

/// a stored chance: in [0, 1]
fn px(v: &Value) -> Option<f64> {
    fx(v).filter(|x| (0.0..=1.0).contains(x))
}

pub(crate) fn uxr(v: &Value) -> Option<u64> {
    u64::from_str_radix(v.as_str()?, 16).ok()
}

pub(crate) fn fvec(v: &[f64]) -> Value {
    Value::Array(v.iter().map(|&x| hx(x)).collect())
}

pub(crate) fn fvec_r(v: &Value) -> Option<Vec<f64>> {
    v.as_array()?.iter().map(fx).collect()
}

/// a stored vector of chances (`px`)
pub(crate) fn pvec_r(v: &Value) -> Option<Vec<f64>> {
    v.as_array()?.iter().map(px).collect()
}

pub(crate) fn usz(v: &Value) -> Option<usize> {
    usize::try_from(v.as_u64()?).ok()
}

/// a stored count (`STATE_MAX_N`)
pub(crate) fn len_in(v: &Value) -> Option<usize> {
    usz(v).filter(|&n| n <= STATE_MAX_N)
}

pub(crate) fn sel_out(s: &SelectedDeck) -> Value {
    json!([s.entries, s.plain_pals, s.filler_cards, fvec(&s.white_hint_chances), fvec(&s.gold_hint_chances)])
}

pub(crate) fn sel_in(v: &Value) -> Option<SelectedDeck> {
    let a = v.as_array()?;
    let ents: Vec<usize> = a.first()?.as_array()?.iter().map(usz).collect::<Option<_>>()?;
    let (plain, filler) = (usz(a.get(1)?)?, usz(a.get(2)?)?);
    let (dw, dg) = (pvec_r(a.get(3)?)?, pvec_r(a.get(4)?)?);
    // (a deck: at most MAX_DECK cards, a white and a gold chance per target)
    let cards = ents.len().checked_add(plain)?.checked_add(filler)?;
    (cards <= MAX_DECK && dw.len() == dg.len()).then_some(SelectedDeck {
        entries: ents,
        plain_pals: plain,
        filler_cards: filler,
        white_hint_chances: dw,
        gold_hint_chances: dg,
    })
}

fn ref_out(r: &Ref) -> Value {
    json!([fvec(&r.q), hx(r.val), sel_out(&r.deck)])
}

fn ref_in(v: &Value) -> Option<Ref> {
    let a = v.as_array()?;
    Some(Ref { q: pvec_r(a.first()?)?, val: fx(a.get(1)?)?, deck: sel_in(a.get(2)?)? })
}

pub(crate) fn found_out(f: &Found) -> Value {
    json!([f.trainee, f.first_parent, f.second_parent, hx(f.value), hx(f.ceiling), sel_out(&f.deck)])
}

pub(crate) fn found_in(v: &Value) -> Option<Found> {
    let a = v.as_array()?;
    Some(Found {
        trainee: crate::UmaIndex(usz(a.first()?)?),
        first_parent: usz(a.get(1)?)?,
        second_parent: usz(a.get(2)?)?,
        value: fx(a.get(3)?)?,
        ceiling: fx(a.get(4)?)?,
        deck: sel_in(a.get(5)?)?,
    })
}

/// SEEN as [fingerprint, classes, [[class, [refs]], ...]] per setup (only the classes holding proofs)
fn seen_out() -> Value {
    SEEN.with(|c| {
        Value::Array(
            c.borrow()
                .iter()
                .map(|(fp, vv)| {
                    json!([
                        ux(*fp),
                        vv.len(),
                        vv.iter()
                            .enumerate()
                            .filter(|(_, rs)| !rs.is_empty())
                            .map(|(i, rs)| json!([i, rs.iter().map(ref_out).collect::<Vec<_>>()]))
                            .collect::<Vec<_>>()
                    ])
                })
                .collect(),
        )
    })
}

fn seen_in(v: &Value) -> Option<Vec<(u64, crate::UmaTable<Vec<Ref>>)>> {
    v.as_array()?
        .iter()
        .map(|e| {
            let a = e.as_array()?;
            let (fp, n) = (uxr(a.first()?)?, len_in(a.get(1)?)?);
            let mut vv = crate::UmaTable(vec![vec![]; n]);
            for c in a.get(2)?.as_array()? {
                let ca = c.as_array()?;
                let i = usz(ca.first()?)?;
                if i >= n {
                    return None;
                }
                vv[crate::UmaIndex(i)] = ca.get(1)?.as_array()?.iter().map(ref_in).collect::<Option<_>>()?;
            }
            Some((fp, vv))
        })
        .collect()
}

/// per-uma (or per-class) lists as [length, [[index, [items]], ...]] (only the non-empty ones)
fn lists_out<T>(vv: &crate::UmaTable<Vec<T>>, f: impl Fn(&T) -> Value) -> Value {
    json!([
        vv.len(),
        vv.iter()
            .enumerate()
            .filter(|(_, rs)| !rs.is_empty())
            .map(|(i, rs)| json!([i, rs.iter().map(&f).collect::<Vec<_>>()]))
            .collect::<Vec<_>>()
    ])
}

fn lists_in<T: Clone>(v: &Value, f: impl Fn(&Value) -> Option<T>) -> Option<crate::UmaTable<Vec<T>>> {
    let a = v.as_array()?;
    let n = len_in(a.first()?)?;
    let mut vv = crate::UmaTable(vec![vec![]; n]);
    for c in a.get(1)?.as_array()? {
        let ca = c.as_array()?;
        let i = usz(ca.first()?)?;
        if i >= n {
            return None;
        }
        vv[crate::UmaIndex(i)] = ca.get(1)?.as_array()?.iter().map(&f).collect::<Option<_>>()?;
    }
    Some(vv)
}

/// LAST_SOLVE as [key, search, proofs] per kept search: every field of `Solved`, floats as bit patterns
fn solve_out() -> Value {
    LAST_SOLVE.with(|c| Value::Array(c.borrow().iter().map(|KeptSolve { key: k, solved: s, ruled: r }| json!([ux(*k), {
        "found": s.found.iter().map(found_out).collect::<Vec<_>>(), "refs": lists_out(&s.refs, ref_out),
        "n": [s.work, s.cands, s.solved, s.bounded, s.scored, s.skipped], "next_bound": hx(s.next_bound), "complete": s.complete, "proven": s.proven, "cert": hx(s.cert)
    }, lists_out(r, |proof| json!([fvec(&proof.inheritance_chances), hx(proof.product_bound)]))])).collect()))
}

fn solve_in(v: &Value) -> Option<Vec<KeptSolve>> {
    v.as_array()?
        .iter()
        .map(|e| {
            let a = e.as_array()?;
            let s = a.get(1)?;
            let n: Vec<usize> = s["n"].as_array()?.iter().map(usz).collect::<Option<_>>()?;
            if n.len() != 6 {
                return None;
            }
            let sv = Solved {
                found: s["found"].as_array()?.iter().map(found_in).collect::<Option<_>>()?,
                refs: lists_in(&s["refs"], ref_in)?,
                work: n[0],
                cands: n[1],
                solved: n[2],
                bounded: n[3],
                scored: n[4],
                skipped: n[5],
                next_bound: fx(&s["next_bound"])?,
                complete: s["complete"].as_bool()?,
                proven: s["proven"].as_bool()?,
                cert: fx(&s["cert"])?,
            };
            let ruled = lists_in(a.get(2)?, |x| {
                let p = x.as_array()?;
                Some(ProductProof { inheritance_chances: pvec_r(p.first()?)?, product_bound: fx(p.get(1)?)? })
            })?;
            Some(KeptSolve { key: uxr(a.first()?)?, solved: sv, ruled })
        })
        .collect()
}

pub(crate) fn state_out() -> Value {
    json!({"ok": true, "state": {"v": STATE_VERSION, "bests": sim::plan::bests_out(), "seen": seen_out(), "solve": solve_out(), "rolls": sim::plan::rolls_out()}})
}

/// replaces the kept plans, proofs, main searches and plans' roll states with the page's stored ones (all or nothing: a
/// store that does not parse is refused)
pub(crate) fn state_in(v: &Value) -> Value {
    if v["v"].as_u64() != Some(STATE_VERSION) {
        return json!({"error": "state: another version"});
    }
    // Decode every store before replacing any live state. (The kept plans' per-uma count is checked here, before
    // `bests_in` allocates from it.)
    if !v["bests"].as_array().is_none_or(|a| a.iter().all(|e| len_in(&e["n"]).is_some())) {
        return json!({"error": "state: does not parse"});
    }
    let (Some(bests), Some(seen), Some(searches), Some(rolls)) = (
        sim::plan::bests_in(&v["bests"]),
        seen_in(&v["seen"]),
        solve_in(&v["solve"]),
        sim::plan::rolls_in(&v["rolls"]),
    ) else {
        return json!({"error": "state: does not parse"});
    };
    let (nb, ns, nm, nr) = (bests.len(), seen.len(), searches.len(), rolls.len());
    sim::plan::bests_set(bests);
    SEEN.with(|cache| *cache.borrow_mut() = seen);
    LAST_SOLVE.with(|cache| *cache.borrow_mut() = searches);
    sim::plan::rolls_set(rolls);
    json!({"ok": true, "bests": nb, "seen": ns, "solve": nm, "rolls": nr})
}

impl Env<'_> {
    // ---- what a stored state (`state_in`) says of a setup, checked where it meets this one ----
    /// a stored deck: entries of this job's deck space, within its deck size, a chance per uncertain target
    fn sel_fits(&self, s: &SelectedDeck) -> bool {
        s.entries.iter().all(|&j| j < self.space.card_indices.len())
            && s.entries.len() + s.plain_pals + s.filler_cards <= self.space.deck_job.deck_size
            && s.white_hint_chances.len() == self.uncertain_count
    }
    /// a stored fully checked deck: a chance per job target, and its deck
    fn ref_fits(&self, r: &Ref) -> bool {
        r.q.len() == self.target_count && self.sel_fits(&r.deck)
    }
    /// stored proofs (`SEEN`): per deck class (one per uma) the fully checked decks
    pub(crate) fn seen_fits(&self, seen: &crate::UmaTable<Vec<Ref>>) -> bool {
        seen.len() == self.data.umas.len() && seen.iter().flatten().all(|r| self.ref_fits(r))
    }
    /// a stored main search (`LAST_SOLVE`) of pool `p`: per uma its refs and product proofs, careers of its trainees and
    /// veterans
    pub(crate) fn solved_fits(&self, sv: &Solved, ruled: &Ruled, p: &Pool) -> bool {
        let n = self.data.umas.len();
        ruled.len() == n
            && ruled.iter().flatten().all(|proof| proof.inheritance_chances.len() == self.target_count)
            && sv.refs.len() == n
            && sv.refs.iter().flatten().all(|r| self.ref_fits(r))
            && sv.found.iter().all(|f| {
                f.trainee.0 < n
                    && f.first_parent < p.veterans.len()
                    && f.second_parent < p.veterans.len()
                    && self.sel_fits(&f.deck)
            })
    }
}
