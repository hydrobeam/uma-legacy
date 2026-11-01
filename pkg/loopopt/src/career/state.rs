//! The worker's saved searches and their validation.

use crate::career::decks::DeckSearchCache;
use crate::career::plan::kept::Bests;
use crate::career::plan::roll::RollKeep;
use crate::career::pool::Pool;
use crate::career::reply::CareerReply;
use crate::career::search::{Assumes, CareerSearchResult, Found, Ref, Unproven};
use crate::career::{CareerSetup, ClassFlags};
use crate::deck::space::{MAX_DECK, SelectedDeck};
use crate::hex::{Bits, Key, bits};
use crate::refusal::Refusal;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

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

/// Replace all kept work, including roll states, when isolating a simulated index run.
pub(crate) const fn kept_swap(kept: &mut KeptState, replacement: KeptState) -> KeptState {
    std::mem::replace(kept, replacement)
}

/// The hash of a text (a legacy's identity, `Legacy::identity`).
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
// What a worker keeps between jobs (the kept plans, `KeptState::bests`; the proofs, `KeptState::seen`; its main
// searches, roll states and step caps) leaves as `state_out` and comes back as `state_in`, so the first plan after a page
// reload reuses the work of the last session. It is plain serde (`StoredState`), each float as its bits and each key in
// hex (`crate::hex`): the state passes through the page's JavaScript, whose JSON writes an infinite float (a search's
// `next_bound` or `cert`) as null and whose numbers cannot hold a 64-bit key, and what comes back is what left, bit for
// bit, as the kept bounds and proofs prune later searches by it. The page keys what it stores by the solver and data
// fingerprints, so a release starts afresh (the keys inside, `career::keys` and `bests_key`, hash with std's DefaultHasher,
// fixed within a build). A store of another version, or one that does not parse, is refused whole (the page then
// drops it and plans cold).
// (12: keys are the jobs' typed content, `crate::key`, no longer their JSON text; every earlier store is refused)
pub(crate) const STATE_VERSION: u64 = 12;

/// What a stored state may hold, checked as it is decoded (`state_in`, before anything is installed): a stored count (the
/// umas or deck classes a list covers) at most STATE_MAX_N, so nothing is allocated
/// from a count past it; every index within the list it indexes; no float NaN (the bounds compare them); chances,
/// odds, deck hint chances and a fully checked deck's value within [0, 1], a product proof's bound finite and not
/// negative (it may be a seed above 1: the bar over a small spark scale), as anything else is corrupt and the bounds
/// would prune with it; a deck at most `MAX_DECK` cards. What a store says of a setup (deck
/// entries, chances per target) is checked where it meets that setup (`CareerSetup::seen_fits`, `solve_kept`).
pub(crate) const STATE_MAX_N: usize = 1 << 10;

/// The worker's kept work as the page stores it (`state_out`; back with `state_in`).
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredState {
    /// `STATE_VERSION`: a store of another is refused
    pub(crate) v: u64,
    pub(crate) bests: Vec<crate::career::plan::kept::StoredBests>,
    pub(crate) seen: Vec<StoredSeen>,
    pub(crate) solve: Vec<StoredSolve>,
    /// class flags by class content key (`ClassFlags::bits`), keys ascending
    pub(crate) flags: Vec<(Key, u8)>,
    pub(crate) rolls: Vec<crate::career::plan::roll::StoredRoll>,
    pub(crate) caps: Vec<crate::career::plan::caps::StoredCaps>,
}

/// A stored float, any but NaN (the bounds compare them).
pub(crate) const fn number(value: Bits) -> Option<f64> {
    if value.0.is_nan() { None } else { Some(value.0) }
}

/// Stored floats (`number`).
pub(crate) fn numbers(values: &[Bits]) -> Option<Vec<f64>> {
    values.iter().map(|&value| number(value)).collect()
}

/// A stored chance: in [0, 1].
pub(crate) fn chance(value: Bits) -> Option<f64> {
    Some(value.0).filter(|chance| (0.0..=1.0).contains(chance))
}

/// Stored chances (`chance`).
pub(crate) fn chances(values: &[Bits]) -> Option<Vec<f64>> {
    values.iter().map(|&value| chance(value)).collect()
}

/// A stored count: at most `STATE_MAX_N`.
pub(crate) const fn count(stored: usize) -> Option<usize> {
    if stored <= STATE_MAX_N { Some(stored) } else { None }
}

/// A deck as stored: its entries, plain pals and filler cards, and its hint chances per uncertain target.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredDeck {
    pub(crate) entries: Vec<usize>,
    pub(crate) plain: usize,
    pub(crate) filler: usize,
    pub(crate) white: Vec<Bits>,
    pub(crate) gold: Vec<Bits>,
}

pub(crate) fn sel_out(deck: &SelectedDeck) -> StoredDeck {
    StoredDeck {
        entries: deck.entries.clone(),
        plain: deck.plain_pals,
        filler: deck.filler_cards,
        white: bits(&deck.white_hint_chances),
        gold: bits(&deck.gold_hint_chances),
    }
}

pub(crate) fn sel_in(stored: &StoredDeck) -> Option<SelectedDeck> {
    let (white_hint_chances, gold_hint_chances) = (chances(&stored.white)?, chances(&stored.gold)?);
    // (a deck: at most MAX_DECK cards, a white and a gold chance per target)
    let cards = stored.entries.len().checked_add(stored.plain)?.checked_add(stored.filler)?;
    (cards <= MAX_DECK && white_hint_chances.len() == gold_hint_chances.len()).then(|| SelectedDeck {
        entries: stored.entries.clone(),
        plain_pals: stored.plain,
        filler_cards: stored.filler,
        white_hint_chances,
        gold_hint_chances,
    })
}

/// A fully checked deck as stored: the inheritance chances it was checked at, its value and the deck.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredRef {
    pub(crate) q: Vec<Bits>,
    pub(crate) value: Bits,
    pub(crate) deck: StoredDeck,
}

fn ref_out(reference: &Ref) -> StoredRef {
    StoredRef { q: bits(&reference.inheritance_chances), value: Bits(reference.value), deck: sel_out(&reference.deck) }
}

fn ref_in(stored: &StoredRef) -> Option<Ref> {
    Some(Ref {
        inheritance_chances: chances(&stored.q)?,
        // (a P(all) before the spark scale: a chance)
        value: chance(stored.value)?,
        deck: sel_in(&stored.deck)?,
    })
}

/// A career as stored: the trainee (uma index), her parents (legacy indices), its value, ceiling and deck.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredFound {
    pub(crate) trainee: usize,
    pub(crate) parents: [usize; 2],
    pub(crate) value: Bits,
    pub(crate) ceiling: Bits,
    pub(crate) deck: StoredDeck,
}

pub(crate) fn found_out(found: &Found) -> StoredFound {
    StoredFound {
        trainee: found.trainee.0,
        parents: [found.first_parent, found.second_parent],
        value: Bits(found.value),
        ceiling: Bits(found.ceiling),
        deck: sel_out(&found.deck),
    }
}

pub(crate) fn found_in(stored: &StoredFound) -> Option<Found> {
    Some(Found {
        trainee: crate::uma::UmaIndex(stored.trainee),
        first_parent: stored.parents[0],
        second_parent: stored.parents[1],
        value: number(stored.value)?,
        ceiling: number(stored.ceiling)?,
        deck: sel_in(&stored.deck)?,
    })
}

/// Per-uma (or per-class) lists as stored: their count, and each non-empty one by its index.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredLists<Item> {
    pub(crate) count: usize,
    pub(crate) lists: Vec<(usize, Vec<Item>)>,
}

fn lists_out<Item, Stored>(
    lists: &crate::uma::UmaTable<Vec<Item>>,
    write_item: impl Fn(&Item) -> Stored,
) -> StoredLists<Stored> {
    StoredLists {
        count: lists.len(),
        lists: lists
            .iter()
            .enumerate()
            .filter(|(_, items)| !items.is_empty())
            .map(|(i, items)| (i, items.iter().map(&write_item).collect()))
            .collect(),
    }
}

/// The lists, each index within the count (`STATE_MAX_N` at most), each item read (`read_item`).
fn lists_in<Item: Clone, Stored>(
    stored: &StoredLists<Stored>,
    read_item: impl Fn(&Stored) -> Option<Item>,
) -> Option<crate::uma::UmaTable<Vec<Item>>> {
    let list_count = count(stored.count)?;
    let mut lists = crate::uma::UmaTable(vec![vec![]; list_count]);
    for (i, items) in &stored.lists {
        if *i >= list_count {
            return None;
        }
        lists[crate::uma::UmaIndex(*i)] = items.iter().map(&read_item).collect::<Option<_>>()?;
    }
    Some(lists)
}

/// One key's kept fully checked decks as stored (`KeptSeen`).
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredSeen {
    pub(crate) key: Key,
    /// per deck class, its fully checked decks
    pub(crate) refs: StoredLists<StoredRef>,
}

fn seen_out(kept: &KeptState) -> Vec<StoredSeen> {
    kept.seen.iter().map(|seen| StoredSeen { key: Key(seen.key), refs: lists_out(&seen.refs, ref_out) }).collect()
}

fn seen_in(stored: &[StoredSeen]) -> Option<Vec<KeptSeen>> {
    stored.iter().map(|seen| Some(KeptSeen { key: seen.key.0, refs: lists_in(&seen.refs, ref_in)? })).collect()
}

/// Class flags as stored: the class count, and the breached classes and those assuming each named assumption.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredClassFlags {
    pub(crate) count: usize,
    pub(crate) breached: Vec<usize>,
    pub(crate) two_feeder: Vec<usize>,
    pub(crate) multi_feeder: Vec<usize>,
}

fn flags_out(flags: &crate::uma::UmaTable<ClassFlags>) -> StoredClassFlags {
    let with = |set: fn(ClassFlags) -> bool| {
        flags.indices().filter(|&class| set(flags[class])).map(|class| class.0).collect::<Vec<_>>()
    };
    StoredClassFlags {
        count: flags.len(),
        breached: with(|f| f.breached),
        two_feeder: with(|f| f.assumes.two_feeder),
        multi_feeder: with(|f| f.assumes.multi_feeder),
    }
}

/// Class flags from their stored form (`flags_out`): every class within the count
fn flags_in(stored: &StoredClassFlags) -> Option<crate::uma::UmaTable<ClassFlags>> {
    let mut flags = crate::uma::UmaTable(vec![ClassFlags::default(); count(stored.count)?]);
    for &class in &stored.breached {
        flags.0.get_mut(class)?.breached = true;
    }
    for &class in &stored.two_feeder {
        flags.0.get_mut(class)?.assumes.two_feeder = true;
    }
    for &class in &stored.multi_feeder {
        flags.0.get_mut(class)?.assumes.multi_feeder = true;
    }
    Some(flags)
}

/// Class flags by content key as stored ([key, bits], `ClassFlags::bits`), keys ascending
pub(crate) fn keyed_flags_out<'flags>(
    flags: impl Iterator<Item = (&'flags u64, &'flags ClassFlags)>,
) -> Vec<(Key, u8)> {
    flags.map(|(&key, flags)| (Key(key), flags.bits())).collect()
}

/// Class flags by content key from `keyed_flags_out`'s form (a repeated key unioned); None for an unknown bit
pub(crate) fn keyed_flags_in(stored: &[(Key, u8)]) -> Option<std::collections::BTreeMap<u64, ClassFlags>> {
    let mut flags = std::collections::BTreeMap::new();
    for &(key, bits) in stored {
        let kept = ClassFlags::from_bits(bits)?;
        let slot: &mut ClassFlags = flags.entry(key.0).or_default();
        *slot = slot.union(kept);
    }
    Some(flags)
}

/// A kept main search as stored (`KeptSolve`): every field of `CareerSearchResult`, its product proofs, class flags and
/// trainees' classes.
#[derive(Serialize, Deserialize, TS)]
pub(crate) struct StoredSolve {
    pub(crate) key: Key,
    pub(crate) found: Vec<StoredFound>,
    pub(crate) refs: StoredLists<StoredRef>,
    pub(crate) work: usize,
    pub(crate) candidates: usize,
    pub(crate) solved: usize,
    pub(crate) bounded: usize,
    pub(crate) scored: usize,
    pub(crate) skipped: usize,
    pub(crate) next_bound: Bits,
    pub(crate) complete: bool,
    pub(crate) unproven: Unproven,
    pub(crate) assumes: Assumes,
    pub(crate) proven: bool,
    pub(crate) cert: Bits,
    /// per deck class, its product proofs: [the chances searched at, the bound]
    pub(crate) ruled: StoredLists<(Vec<Bits>, Bits)>,
    pub(crate) class_flags: StoredClassFlags,
    pub(crate) classes: Vec<usize>,
}

fn solve_out(kept: &KeptState) -> Vec<StoredSolve> {
    kept.searches
        .iter()
        .map(|KeptSolve { key, solved, ruled, class_flags, classes }| StoredSolve {
            key: Key(*key),
            found: solved.found.iter().map(found_out).collect(),
            refs: lists_out(&solved.refs, ref_out),
            work: solved.work,
            candidates: solved.candidates,
            solved: solved.solved,
            bounded: solved.bounded,
            scored: solved.scored,
            skipped: solved.skipped,
            next_bound: Bits(solved.next_bound),
            complete: solved.complete,
            unproven: solved.unproven,
            assumes: solved.assumes,
            proven: solved.proven,
            cert: Bits(solved.certified_cap),
            ruled: lists_out(ruled, |proof| (bits(&proof.inheritance_chances), Bits(proof.product_bound))),
            class_flags: flags_out(class_flags),
            classes: classes.iter().map(|class| class.0).collect(),
        })
        .collect()
}

fn solve_in(stored: &[StoredSolve]) -> Option<Vec<KeptSolve>> {
    stored
        .iter()
        .map(|stored| {
            let solved = CareerSearchResult {
                found: stored.found.iter().map(found_in).collect::<Option<_>>()?,
                refs: lists_in(&stored.refs, ref_in)?,
                work: stored.work,
                candidates: stored.candidates,
                solved: stored.solved,
                bounded: stored.bounded,
                scored: stored.scored,
                skipped: stored.skipped,
                next_bound: number(stored.next_bound)?,
                complete: stored.complete,
                unproven: stored.unproven,
                assumes: stored.assumes,
                proven: stored.proven,
                certified_cap: number(stored.cert)?,
            };
            // (complete exactly when no reason is listed; proven only when complete)
            if solved.complete == solved.unproven.any() || (solved.proven && !solved.complete) {
                return None;
            }
            let ruled = lists_in(&stored.ruled, |(inheritance_chances, bound)| {
                Some(ProductProof {
                    inheritance_chances: chances(inheritance_chances)?,
                    // (a product value, or the seed a search found nothing above: finite and not negative)
                    product_bound: Some(bound.0).filter(|bound| bound.is_finite() && *bound >= 0.0)?,
                })
            })?;
            let class_flags = flags_in(&stored.class_flags)?;
            let classes: Vec<crate::uma::UmaIndex> = stored
                .classes
                .iter()
                .map(|&class| (class < class_flags.len()).then_some(crate::uma::UmaIndex(class)))
                .collect::<Option<_>>()?;
            // (proven with a class of its trainees breached: inconsistent, `CareerSearchResult::qualify`)
            if solved.proven && classes.iter().any(|&class| !class_flags[class].proves()) {
                return None;
            }
            Some(KeptSolve { key: stored.key.0, solved, ruled, class_flags, classes })
        })
        .collect()
}

pub(crate) fn state_out(kept: &KeptState) -> StateOut {
    StateOut {
        state: StoredState {
            v: STATE_VERSION,
            bests: crate::career::plan::kept::bests_out(&kept.bests),
            seen: seen_out(kept),
            solve: solve_out(kept),
            flags: keyed_flags_out(kept.flags.iter()),
            rolls: crate::career::plan::roll::rolls_out(&kept.rolls),
            caps: crate::career::plan::caps::caps_out(&kept.step_caps),
        },
    }
}

/// replaces the kept plans, proofs, main searches, plans' roll states and step caps with the page's stored ones and unions its
/// class flags into the worker's (all or nothing: a store of another version, or that does not parse, is refused)
pub(crate) fn state_in(kept: &mut KeptState, stored: &StoredState) -> CareerReply {
    if stored.v != STATE_VERSION {
        return CareerReply::Refusal(Refusal::new("state: another version"));
    }
    // Decode every store before replacing any live state.
    let (Some(bests), Some(seen), Some(searches), Some(flags), Some(rolls), Some(step_caps)) = (
        crate::career::plan::kept::bests_in(&stored.bests),
        seen_in(&stored.seen),
        solve_in(&stored.solve),
        keyed_flags_in(&stored.flags),
        crate::career::plan::roll::rolls_in(&stored.rolls),
        crate::career::plan::caps::caps_in(&stored.caps),
    ) else {
        return CareerReply::Refusal(Refusal::new("state: does not parse"));
    };
    let counts = StateCounts {
        bests: bests.len(),
        seen: seen.len(),
        solve: searches.len(),
        rolls: rolls.len(),
        caps: step_caps.len(),
    };
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
    CareerReply::StateIn(counts)
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

/// `state_out`'s answer: the kept work to store.
#[derive(Serialize, TS)]
pub(crate) struct StateOut {
    pub(crate) state: StoredState,
}

/// What a store restored (`state_in`; a store it refuses is a refusal, which the page drops): the kept plans, proofs'
/// keys, main searches, roll states and step caps.
#[derive(Serialize, TS)]
pub(crate) struct StateCounts {
    pub(crate) bests: usize,
    pub(crate) seen: usize,
    pub(crate) solve: usize,
    pub(crate) rolls: usize,
    pub(crate) caps: usize,
}
