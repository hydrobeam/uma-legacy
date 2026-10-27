//! The worker's saved searches and their validation.

use crate::career::decks::DeckSearchCache;
use crate::career::plan::kept::Bests;
use crate::career::plan::roll::RollKeep;
use crate::career::pool::Pool;
use crate::career::search::{Assumes, CareerSearchResult, Found, Ref, Unproven};
use crate::career::{CareerSetup, ClassFlags};
use crate::deck::space::{MAX_DECK, SelectedDeck};
use serde_json::{Value, json};

/// Work retained between jobs. The worker owns it; career and planner operations borrow it explicitly.
#[derive(Default)]
pub(crate) struct KeptState {
    /// Best careers and proofs per plan, oldest first.
    pub(crate) bests: Vec<Bests>,
    /// Fully checked decks per (setup, trainees, pool) key (`CareerBuild::seen_fingerprint`) and deck class, oldest key
    /// first. Each Ref's chances and exact value depend on the setup and the trainees (their order sets the deck space's
    /// plain-pal order); they are kept per pool too, as `certified` could otherwise name another career of equal value
    /// after a job on another pool.
    pub(crate) seen: Vec<KeptSeen>,
    /// Main searches and their product proofs, oldest first, keyed by the job without `next` and `no_reports`.
    /// Reuse restores the product proofs a combined search/plan job would have left for its plan.
    pub(crate) searches: Vec<KeptSolve>,
    /// Class flags (`ClassFlags`, those with a flag set) by class content (`CareerSetup::class_key`: the setup and the
    /// class's deck problem, whatever the trainees, pool or class index): every job unions them in and keeps its own
    /// (`keep_flags`), so a breach or an assumption in any job holds for every later one with a class of that content,
    /// whichever search or plan it reuses. Never evicted (a few bytes a class), so no kept certificate outlives its
    /// flags; a kept plan also carries its own (`Bests::flags`).
    pub(crate) flags: std::collections::BTreeMap<u64, ClassFlags>,
    /// Roll scorings for recent plans, oldest first.
    pub(crate) rolls: Vec<RollKeep>,
    /// Recent plans' step caps (`plan::caps`), oldest first: what a later plan of a pool holding theirs reads.
    pub(crate) step_caps: Vec<crate::career::plan::caps::KeptCaps>,
    /// Recent plans' check states (`next.robust`), oldest first; not serialized.
    pub(crate) checks: Vec<crate::career::plan::CheckKeep>,
    /// Deck outcomes and corner tables per setup and trainee order; not serialized.
    pub(crate) decks: Vec<(u64, DeckSearchCache)>,
}

thread_local! {
    /// Native and wasm entry points retain one record per worker thread. Search code receives `&mut KeptState`;
    /// recursive simulator jobs use that borrow directly rather than borrowing this cell again.
    pub(crate) static KEPT: std::cell::RefCell<KeptState> = std::cell::RefCell::default();
}

/// Replace all kept work, including roll states, when isolating a simulated index run.
pub(crate) const fn kept_swap(kept: &mut KeptState, replacement: KeptState) -> KeptState {
    std::mem::replace(kept, replacement)
}

/// A job's setup fingerprint: everything but the pool and the places (targets, roster, cards, scenario, ...), the
/// key of what one job's search proves that holds for the next (p_bar, fully checked decks)
pub(crate) fn setup_fp(value: &Value) -> u64 {
    // (also the page's flags that change no deck value: `compare`, the plan's own fields, so the plan and compare
    // jobs share what the search jobs proved, and `borrowed_parent`, a rule on the pairs as `fix` is)
    hash_text(&json_without(
        value,
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

/// The JSON text of `value` with its top-level keys `drop` left out, and of its `next` object's keys `next_drop`: the text of a
/// clone with those keys removed, byte for byte, without the clone. serde_json's `Map` is a `BTreeMap` here (no
/// `preserve_order`), so the kept keys are written in the same order, by the same serializer (`Value`'s `Display` is
/// `to_writer`'s compact form), and an object left empty is "{}" either way
pub(crate) fn json_without(value: &Value, drop: &[&str], next_drop: &[&str]) -> String {
    struct Strip<'val> {
        value: &'val Value,
        drop: &'val [&'val str],
        next_drop: &'val [&'val str],
    }
    impl serde::Serialize for Strip<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            match self.value {
                Value::Object(map) => serializer.collect_map(
                    map.iter().filter(|(key, _)| !self.drop.contains(&key.as_str())).map(|(key, field_value)| {
                        (
                            key,
                            Strip {
                                value: field_value,
                                drop: if key == "next" { self.next_drop } else { &[] },
                                next_drop: &[],
                            },
                        )
                    }),
                ),
                field_value => field_value.serialize(serializer),
            }
        }
    }
    serde_json::to_string(&Strip { value, drop, next_drop }).unwrap()
}

pub(crate) fn hash_text(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

/// One key's kept fully checked decks (`KeptState::seen`).
pub(crate) struct KeptSeen {
    /// `CareerBuild::seen_fingerprint` of the jobs that made them.
    pub(crate) key: u64,
    /// Per deck class, its fully checked decks.
    pub(crate) refs: crate::uma::UmaTable<Vec<Ref>>,
}

/// One kept main search (`KeptState::searches`).
pub(crate) struct KeptSolve {
    /// The job's key (`CareerRun::search_key`).
    pub(crate) key: u64,
    pub(crate) solved: CareerSearchResult,
    /// What its product searches proved (`SearchState::ruled` as the search left it).
    pub(crate) ruled: Ruled,
    /// Per deck class, its flags as the search left them (`SearchState::class_flags`): unioned into a job that reuses
    /// the search.
    pub(crate) class_flags: crate::uma::UmaTable<ClassFlags>,
    /// The deck classes of its trainees, ascending: the classes its answer rests on (a stored search that says it is
    /// `proven` with one of them breached is refused, `state_in`).
    pub(crate) classes: Vec<crate::uma::UmaIndex>,
}

/// per deck class, the product searches' proofs (`SearchState::ruled`)
pub(crate) type Ruled = crate::uma::UmaTable<Vec<ProductProof>>;

/// What one product search proved (`best_deck` pass 1): every legal deck's product value at chances q (fixed factor
/// included, before the spark scale) is at most S.
#[derive(Clone)]
pub(crate) struct ProductProof {
    /// q: the inheritance chances the search ran at, per job target.
    pub(crate) inheritance_chances: Vec<f64>,
    /// S: the best product value at q.
    pub(crate) product_bound: f64,
}

/// main searches kept (`KeptState::searches`): the page's run, and one it goes back to (a setting changed and back, a Compare or
/// Reroll plan in between)
pub(crate) const SOLVE_KEEP: usize = 3;

/// Fully checked decks kept per trainee across jobs (the most recent): the bound loop over them stays cheap
pub(crate) const SEEN_KEEP: usize = 256;

// ---------- the worker's kept work across page loads ----------
// What a worker keeps between jobs (the kept plans, `KeptState::bests`; the proofs, `KeptState::seen`) leaves as `state_out` and comes
// back as `state_in`, so the first plan after a page reload reuses the work of the last session. Every float travels as
// its bit pattern in hex (decimal JSON does not round-trip here), so what comes back is what left, bit for bit; the page
// keys what it stores by the solver and data fingerprints, so a release starts afresh (the keys inside, `setup_fp` and
// `bests_key`, hash with std's DefaultHasher, fixed within a build). A store that does not parse is refused whole.
// (5: a roll state's Conts carry their anchor odds, its scoring the lifted costs, its base no trios; 6: a kept search
// holds its `unproven` reasons, `assumes` (names), its class flags and its trainees' classes; the class flags by class
// content, and each kept plan's own; 7: a roll state's routes carry their C while a lifted side builds, and its Conts
// price odds above p0 off their lattice; 8: the plans' step caps, and a roll state's base its candidates' caps; 9: a
// step cap is its kept fit's curve, lam, phi, pinf, p0 and decay, and her odds then; 10: a roll state keeps no bound's
// inputs, the roll's y_hi gone: no found-career anchors, no flags of which candidates' odds are proven)
pub(crate) const STATE_VERSION: u64 = 10;

/// What a stored state may hold, checked as it is decoded (`state_in`, before anything is installed): a stored count (the
/// umas or deck classes a list covers) at most STATE_MAX_N, so nothing is allocated
/// from a count past it; every index within the list it indexes; no float NaN (the bounds compare them); chances,
/// odds, deck hint chances and a fully checked deck's value within [0, 1], a product proof's bound finite and not
/// negative (it may be a seed above 1: the bar over a small spark scale), as anything else is corrupt and the bounds
/// would prune with it; a deck at most `MAX_DECK` cards. What a store says of a setup (deck
/// entries, chances per target) is checked where it meets that setup (`CareerSetup::seen_fits`, `solve_kept`).
pub(crate) const STATE_MAX_N: usize = 1 << 10;

pub(crate) fn hx(value: f64) -> Value {
    Value::String(format!("{:016x}", value.to_bits()))
}

pub(crate) fn ux(value: u64) -> Value {
    Value::String(format!("{value:016x}"))
}

pub(crate) fn fx(value: &Value) -> Option<f64> {
    Some(f64::from_bits(u64::from_str_radix(value.as_str()?, 16).ok()?)).filter(|float| !float.is_nan())
}

/// a stored chance: in [0, 1]
fn px(value: &Value) -> Option<f64> {
    fx(value).filter(|chance| (0.0..=1.0).contains(chance))
}

pub(crate) fn uxr(value: &Value) -> Option<u64> {
    u64::from_str_radix(value.as_str()?, 16).ok()
}

pub(crate) fn fvec(value: &[f64]) -> Value {
    Value::Array(value.iter().map(|&float| hx(float)).collect())
}

pub(crate) fn fvec_r(value: &Value) -> Option<Vec<f64>> {
    value.as_array()?.iter().map(fx).collect()
}

/// a stored vector of chances (`px`)
pub(crate) fn pvec_r(value: &Value) -> Option<Vec<f64>> {
    value.as_array()?.iter().map(px).collect()
}

pub(crate) fn usz(value: &Value) -> Option<usize> {
    usize::try_from(value.as_u64()?).ok()
}

/// a stored count (`STATE_MAX_N`)
pub(crate) fn len_in(value: &Value) -> Option<usize> {
    usz(value).filter(|&n| n <= STATE_MAX_N)
}

pub(crate) fn sel_out(deck: &SelectedDeck) -> Value {
    json!([
        deck.entries,
        deck.plain_pals,
        deck.filler_cards,
        fvec(&deck.white_hint_chances),
        fvec(&deck.gold_hint_chances)
    ])
}

pub(crate) fn sel_in(value: &Value) -> Option<SelectedDeck> {
    let fields = value.as_array()?;
    let entries: Vec<usize> = fields.first()?.as_array()?.iter().map(usz).collect::<Option<_>>()?;
    let (plain, filler) = (usz(fields.get(1)?)?, usz(fields.get(2)?)?);
    let (white_hint_chances, gold_hint_chances) = (pvec_r(fields.get(3)?)?, pvec_r(fields.get(4)?)?);
    // (a deck: at most MAX_DECK cards, a white and a gold chance per target)
    let cards = entries.len().checked_add(plain)?.checked_add(filler)?;
    (cards <= MAX_DECK && white_hint_chances.len() == gold_hint_chances.len()).then_some(SelectedDeck {
        entries,
        plain_pals: plain,
        filler_cards: filler,
        white_hint_chances,
        gold_hint_chances,
    })
}

fn ref_out(reference: &Ref) -> Value {
    json!([fvec(&reference.inheritance_chances), hx(reference.value), sel_out(&reference.deck)])
}

fn ref_in(value: &Value) -> Option<Ref> {
    let fields = value.as_array()?;
    Some(Ref {
        inheritance_chances: pvec_r(fields.first()?)?,
        // (a P(all) before the spark scale: a chance)
        value: px(fields.get(1)?)?,
        deck: sel_in(fields.get(2)?)?,
    })
}

pub(crate) fn found_out(found: &Found) -> Value {
    json!([
        found.trainee,
        found.first_parent,
        found.second_parent,
        hx(found.value),
        hx(found.ceiling),
        sel_out(&found.deck)
    ])
}

pub(crate) fn found_in(value: &Value) -> Option<Found> {
    let fields = value.as_array()?;
    Some(Found {
        trainee: crate::uma::UmaIndex(usz(fields.first()?)?),
        first_parent: usz(fields.get(1)?)?,
        second_parent: usz(fields.get(2)?)?,
        value: fx(fields.get(3)?)?,
        ceiling: fx(fields.get(4)?)?,
        deck: sel_in(fields.get(5)?)?,
    })
}

/// Kept proofs as [fingerprint, classes, [[class, [refs]], ...]] per key (only the classes holding proofs)
fn seen_out(kept: &KeptState) -> Value {
    Value::Array(
        kept.seen
            .iter()
            .map(|KeptSeen { key, refs: classes }| {
                json!([
                    ux(*key),
                    classes.len(),
                    classes
                        .iter()
                        .enumerate()
                        .filter(|(_, refs)| !refs.is_empty())
                        .map(|(i, refs)| json!([i, refs.iter().map(ref_out).collect::<Vec<_>>()]))
                        .collect::<Vec<_>>()
                ])
            })
            .collect(),
    )
}

/// Class flags as [class count, [breached classes], [classes assuming two_feeder_cap], [classes assuming
/// multi_feeder_cap]]
fn flags_out(flags: &crate::uma::UmaTable<ClassFlags>) -> Value {
    let with = |set: fn(ClassFlags) -> bool| {
        flags.indices().filter(|&class| set(flags[class])).map(|class| class.0).collect::<Vec<_>>()
    };
    json!([flags.len(), with(|f| f.breached), with(|f| f.assumes.two_feeder), with(|f| f.assumes.multi_feeder)])
}

/// Class flags from [class count, [breached], [two_feeder_cap], [multi_feeder_cap]] (`flags_out`): every class within
/// the count
fn flags_in(value: &Value) -> Option<crate::uma::UmaTable<ClassFlags>> {
    let fields = value.as_array()?;
    let class_count = len_in(fields.first()?)?;
    let mut flags = crate::uma::UmaTable(vec![ClassFlags::default(); class_count]);
    let setters: [fn(&mut ClassFlags); 3] =
        [|f| f.breached = true, |f| f.assumes.two_feeder = true, |f| f.assumes.multi_feeder = true];
    for (field, set) in fields.get(1..4)?.iter().zip(setters) {
        for class in field.as_array()? {
            set(flags.0.get_mut(usz(class)?)?);
        }
    }
    Some(flags)
}

/// Class flags by content key as [[key, bits], ...] (`ClassFlags::bits`), keys ascending
pub(crate) fn keyed_flags_out<'flags>(flags: impl Iterator<Item = (&'flags u64, &'flags ClassFlags)>) -> Value {
    Value::Array(flags.map(|(&key, flags)| json!([ux(key), flags.bits()])).collect())
}

/// Class flags by content key from `keyed_flags_out`'s form (a repeated key unioned); None for an unknown bit
pub(crate) fn keyed_flags_in(value: &Value) -> Option<std::collections::BTreeMap<u64, ClassFlags>> {
    let mut flags = std::collections::BTreeMap::new();
    for entry in value.as_array()? {
        let fields = entry.as_array()?;
        let kept = ClassFlags::from_bits(u8::try_from(fields.get(1)?.as_u64()?).ok()?)?;
        let slot: &mut ClassFlags = flags.entry(uxr(fields.first()?)?).or_default();
        *slot = slot.union(kept);
    }
    Some(flags)
}

fn seen_in(value: &Value) -> Option<Vec<KeptSeen>> {
    value
        .as_array()?
        .iter()
        .map(|setup_entry| {
            let fields = setup_entry.as_array()?;
            let (fingerprint, class_count) = (uxr(fields.first()?)?, len_in(fields.get(1)?)?);
            let mut classes = crate::uma::UmaTable(vec![vec![]; class_count]);
            for class_entry in fields.get(2)?.as_array()? {
                let class_fields = class_entry.as_array()?;
                let i = usz(class_fields.first()?)?;
                if i >= class_count {
                    return None;
                }
                classes[crate::uma::UmaIndex(i)] =
                    class_fields.get(1)?.as_array()?.iter().map(ref_in).collect::<Option<_>>()?;
            }
            Some(KeptSeen { key: fingerprint, refs: classes })
        })
        .collect()
}

/// per-uma (or per-class) lists as [length, [[index, [items]], ...]] (only the non-empty ones)
fn lists_out<T>(lists: &crate::uma::UmaTable<Vec<T>>, write_item: impl Fn(&T) -> Value) -> Value {
    json!([
        lists.len(),
        lists
            .iter()
            .enumerate()
            .filter(|(_, items)| !items.is_empty())
            .map(|(i, items)| json!([i, items.iter().map(&write_item).collect::<Vec<_>>()]))
            .collect::<Vec<_>>()
    ])
}

fn lists_in<T: Clone>(value: &Value, read_item: impl Fn(&Value) -> Option<T>) -> Option<crate::uma::UmaTable<Vec<T>>> {
    let fields = value.as_array()?;
    let list_count = len_in(fields.first()?)?;
    let mut lists = crate::uma::UmaTable(vec![vec![]; list_count]);
    for list_entry in fields.get(1)?.as_array()? {
        let list_fields = list_entry.as_array()?;
        let i = usz(list_fields.first()?)?;
        if i >= list_count {
            return None;
        }
        lists[crate::uma::UmaIndex(i)] =
            list_fields.get(1)?.as_array()?.iter().map(&read_item).collect::<Option<_>>()?;
    }
    Some(lists)
}

/// Kept searches as [key, search, proofs, class flags, classes] per kept search: every field of `CareerSearchResult`
/// (`unproven` as its reasons' names), floats as bit patterns
fn solve_out(kept: &KeptState) -> Value {
    Value::Array(kept.searches.iter().map(|KeptSolve { key, solved, ruled, class_flags, classes }| json!([ux(*key), {
        "found": solved.found.iter().map(found_out).collect::<Vec<_>>(), "refs": lists_out(&solved.refs, ref_out),
        "n": [solved.work, solved.candidates, solved.solved, solved.bounded, solved.scored, solved.skipped], "next_bound": hx(solved.next_bound), "complete": solved.complete, "unproven": solved.unproven.names(), "assumes": solved.assumes.names(), "proven": solved.proven, "cert": hx(solved.certified_cap)
    }, lists_out(ruled, |proof| json!([fvec(&proof.inheritance_chances), hx(proof.product_bound)])),
    flags_out(class_flags), classes.iter().map(|class| class.0).collect::<Vec<_>>()])).collect())
}

fn solve_in(value: &Value) -> Option<Vec<KeptSolve>> {
    value
        .as_array()?
        .iter()
        .map(|entry| {
            let fields = entry.as_array()?;
            let stored = fields.get(1)?;
            let counts: Vec<usize> = stored["n"].as_array()?.iter().map(usz).collect::<Option<_>>()?;
            if counts.len() != 6 {
                return None;
            }
            let solved = CareerSearchResult {
                found: stored["found"].as_array()?.iter().map(found_in).collect::<Option<_>>()?,
                refs: lists_in(&stored["refs"], ref_in)?,
                work: counts[0],
                candidates: counts[1],
                solved: counts[2],
                bounded: counts[3],
                scored: counts[4],
                skipped: counts[5],
                next_bound: fx(&stored["next_bound"])?,
                complete: stored["complete"].as_bool()?,
                unproven: Unproven::from_names(&stored["unproven"])?,
                assumes: Assumes::from_names(Some(&stored["assumes"]))?,
                proven: stored["proven"].as_bool()?,
                certified_cap: fx(&stored["cert"])?,
            };
            // (complete exactly when no reason is listed; proven only when complete)
            if solved.complete == solved.unproven.any() || (solved.proven && !solved.complete) {
                return None;
            }
            let ruled = lists_in(fields.get(2)?, |proof| {
                let proof_fields = proof.as_array()?;
                Some(ProductProof {
                    inheritance_chances: pvec_r(proof_fields.first()?)?,
                    // (a product value, or the seed a search found nothing above: finite and not negative)
                    product_bound: fx(proof_fields.get(1)?).filter(|bound| bound.is_finite() && *bound >= 0.0)?,
                })
            })?;
            let class_flags = flags_in(fields.get(3)?)?;
            let classes: Vec<crate::uma::UmaIndex> = fields
                .get(4)?
                .as_array()?
                .iter()
                .map(|class| usz(class).filter(|&class| class < class_flags.len()).map(crate::uma::UmaIndex))
                .collect::<Option<_>>()?;
            // (proven with a class of its trainees breached: inconsistent, `CareerSearchResult::qualify`)
            if solved.proven && classes.iter().any(|&class| !class_flags[class].proves()) {
                return None;
            }
            Some(KeptSolve { key: uxr(fields.first()?)?, solved, ruled, class_flags, classes })
        })
        .collect()
}

pub(crate) fn state_out(kept: &KeptState) -> Value {
    json!({"ok": true, "state": {"v": STATE_VERSION, "bests": crate::career::plan::kept::bests_out(&kept.bests), "seen": seen_out(kept), "solve": solve_out(kept), "flags": keyed_flags_out(kept.flags.iter()), "rolls": crate::career::plan::roll::rolls_out(&kept.rolls), "caps": crate::career::plan::caps::caps_out(&kept.step_caps)}})
}

/// replaces the kept plans, proofs, main searches, plans' roll states and step caps with the page's stored ones and unions its
/// class flags into the worker's (all or nothing: a store that does not parse is refused)
pub(crate) fn state_in(kept: &mut KeptState, value: &Value) -> Value {
    if value["v"].as_u64() != Some(STATE_VERSION) {
        return json!({"error": "state: another version"});
    }
    // Decode every store before replacing any live state. (The kept plans' per-uma count is checked here, before
    // `bests_in` allocates from it.)
    if !value["bests"].as_array().is_none_or(|bests_list| bests_list.iter().all(|best| len_in(&best["n"]).is_some())) {
        return json!({"error": "state: does not parse"});
    }
    let (Some(bests), Some(seen), Some(searches), Some(flags), Some(rolls), Some(step_caps)) = (
        crate::career::plan::kept::bests_in(&value["bests"]),
        seen_in(&value["seen"]),
        solve_in(&value["solve"]),
        keyed_flags_in(&value["flags"]),
        crate::career::plan::roll::rolls_in(&value["rolls"]),
        crate::career::plan::caps::caps_in(&value["caps"]),
    ) else {
        return json!({"error": "state: does not parse"});
    };
    let (best_count, seen_count, search_count, roll_count, cap_count) =
        (bests.len(), seen.len(), searches.len(), rolls.len(), step_caps.len());
    // (the stored flags unioned into the worker's: a store never clears a flag)
    let mut flags = flags;
    for (key, live) in std::mem::take(&mut kept.flags) {
        let slot = flags.entry(key).or_default();
        *slot = slot.union(live);
    }
    // Deck memos are process-local and were never part of the stored format; retain them on import.
    *kept = KeptState {
        bests,
        seen,
        searches,
        rolls,
        step_caps,
        flags,
        checks: std::mem::take(&mut kept.checks),
        decks: std::mem::take(&mut kept.decks),
    };
    json!({"ok": true, "bests": best_count, "seen": seen_count, "solve": search_count, "rolls": roll_count, "caps": cap_count})
}

impl CareerSetup<'_> {
    // ---- what a stored state (`state_in`) says of a setup, checked where it meets this one ----
    /// a stored deck: entries of this job's deck space, within its deck size, a chance per uncertain target
    fn sel_fits(&self, deck: &SelectedDeck) -> bool {
        deck.entries.iter().all(|&j| j < self.space.card_indices.len())
            && deck.entries.len() + deck.plain_pals + deck.filler_cards <= self.space.deck_job.deck_size
            && deck.white_hint_chances.len() == self.uncertain_count
    }
    /// a stored fully checked deck: a chance per job target, and its deck
    fn ref_fits(&self, reference: &Ref) -> bool {
        reference.inheritance_chances.len() == self.target_count && self.sel_fits(&reference.deck)
    }
    /// stored proofs (`KeptState::seen`): per deck class (one per uma) the fully checked decks
    pub(crate) fn seen_fits(&self, seen: &crate::uma::UmaTable<Vec<Ref>>) -> bool {
        seen.len() == self.data.umas.len() && seen.iter().flatten().all(|reference| self.ref_fits(reference))
    }
    /// a stored main search (`KeptState::searches`) of `pool`: per uma its refs, product proofs and class flags,
    /// careers of its trainees and veterans
    pub(crate) fn solved_fits(
        &self,
        solved: &CareerSearchResult,
        ruled: &Ruled,
        class_flags: &crate::uma::UmaTable<ClassFlags>,
        pool: &Pool,
    ) -> bool {
        let uma_count = self.data.umas.len();
        ruled.len() == uma_count
            && class_flags.len() == uma_count
            && ruled.iter().flatten().all(|proof| proof.inheritance_chances.len() == self.target_count)
            && solved.refs.len() == uma_count
            && solved.refs.iter().flatten().all(|reference| self.ref_fits(reference))
            && solved.found.iter().all(|found_career| {
                found_career.trainee.0 < uma_count
                    && found_career.first_parent < pool.veterans.len()
                    && found_career.second_parent < pool.veterans.len()
                    && self.sel_fits(&found_career.deck)
            })
    }
}
