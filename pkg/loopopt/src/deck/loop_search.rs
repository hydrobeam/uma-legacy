//! Loop-search inputs, keys, shared state and setup for per-uma or shared decks.
//! Ceiling tables, deck scans, search passes and reporting live in the child modules.
use self::shared::SharedResult;
use self::tables::{
    ByBound, ExactCacheEntry, ExactCeiling, Grid, NG, class_clusters, classes, full_clusters, pair_table,
};
use crate::Instant;
use crate::data::{Choice, Data, NT, Sched, Uma};
use crate::deck::dominance::{Cl, Name, PDeck};
use crate::deck::sources::{SourceEntry, TargetSources};
use crate::deck::space::DeckSpace;
use crate::deck::{Card, CardsRaw, DeckJob};
use crate::optimize::{Res, members};
use crate::skills::{MemberLineage, Model, SkillJob, check_sizes, lins};
use crate::uma::{UmaIndex, UmaTable};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
mod bands;
mod report;
mod shared;
mod tables;

/// Immutable resources for the loop search. All indices retain their original roster/deck-space provenance.
struct LoopJobInputs<'data> {
    data: &'data Data,
    cards: &'data CardsRaw,
    model: &'data Model<'data>,
    skill_job: &'data SkillJob,
    deck_job: &'data DeckJob,
    /// Pareto decks from the initial enumeration, in its retained order.
    skyline: &'data [PDeck],
    deck_space: &'data DeckSpace<'data>,
}

/// Per-uma decks: each trainee runs her own best legal deck, the same one in all the lineages she heads (three in a
/// four-uma loop, one in a three-uma rotation).
///   value(f) = fixed/L * sum_t max over decks D legal for f[t] of sum over her lineages l of T(f[t], q_l, D), L = 12 or 3
/// The skyline bounds product values without enforcing each trainee's card restrictions. Legal skyline decks
/// seed exact scoring; `DeckSpace` searches the remaining legal decks under their assumed structure-class caps.
pub(super) fn run_multi(
    data: &Data,
    card_data: &CardsRaw,
    model: &Model,
    skill_job: &SkillJob,
    deck_job: &DeckJob,
    skyline: &[PDeck],
    stats: Value,
    space: &DeckSpace,
) -> Value {
    let search = match Multi::new(LoopJobInputs {
        data,
        cards: card_data,
        model,
        skill_job,
        deck_job,
        skyline,
        deck_space: space,
    }) {
        Ok(m) => m,
        Err(e) => return e,
    };
    let (pass, cert) = search.certify();
    if pass.solved.is_empty() {
        return json!({"error": "No loop contains all the required umas."});
    }
    search.debug_summary(&pass, cert);
    search.report(pass, cert, stats)
}

/// A memo key of up to 12 chances (a loop's lineages): their bits and count, a fixed array (equal exactly when the
/// chance lists are bit for bit)
#[derive(PartialEq, Eq, Hash)]
struct ChanceKey {
    /// Bit patterns preserve exact equality, including signed zero.
    bits: [u64; 12],
    count: u8,
}
/// The reported deck attaining a structure class's highest exact/product ratio.
struct RatioExample {
    trainee_id: u32,
    cards: Vec<Name>,
}

/// Borrowed or rebuilt source entries and the remaining filler slots of one detached deck.
struct DeckSources<'data> {
    entries: Vec<SourceEntry<Cow<'data, TargetSources>>>,
    filler_count: usize,
}

/// An entry's card identity, optional early end, and reward variant.
#[derive(PartialEq, Eq, Hash)]
struct CardEntryKey {
    card_id: u32,
    end_link: u8,
    variant: u32,
}

/// A per-trainee exact query; chance order and bit patterns remain part of its identity.
#[derive(PartialEq, Eq, Hash)]
struct ExactQueryKey {
    trainee: UmaIndex,
    chances: ChanceKey,
}

/// A shared-deck exact query, retaining member and lineage order.
#[derive(PartialEq, Eq, Hash)]
struct SharedQueryKey {
    members: [UmaIndex; 4],
    chances: ChanceKey,
}

/// A checked selection in this job's fixed deck space.
#[derive(PartialEq, Eq, Hash)]
struct SelectionKey {
    entries: Vec<usize>,
    plain_pals: usize,
    filler_cards: usize,
}
/// A deck's identity in the shared-deck results: (card ids, end links, variants, plain pals)
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SharedDeckKey {
    card_ids: Vec<u32>,
    end_links: Vec<u8>,
    variants: Vec<u32>,
    plain_pals: usize,
}

/// What the search's phases share: the job, the skyline with its per-class views and clusters, the ceiling tables,
/// the exact caches and the tally. Tables filled on demand sit in cells, so every phase works through `&self`.
struct Multi<'data> {
    inputs: LoopJobInputs<'data>,
    target_groups: Vec<u32>,
    target_count: usize,
    requested_target_count: usize,
    // (looked up only, never iterated: the hasher changes no order)
    card_of: crate::FxMap<u32, &'data Card>,
    pool: Vec<UmaIndex>,
    required_umas: Vec<UmaIndex>,
    allow_three: bool,
    allow_four: bool,
    fixed_schedules: Option<UmaTable<Option<Sched>>>,
    no_race: bool,
    /// pair ceilings (or the fixed schedules' pair values), by uma index
    pair_bounds: UmaTable<UmaTable<f64>>,
    fixed: f64,
    /// each uma's highest inheritance chance
    maximum_chances: UmaTable<f64>,
    grid: Grid,
    no_hint_chances: Vec<f64>,
    /// per class of umas with the same value coordinates (`ucoords`): its skyline decks and their clusters; each
    /// uma's class
    class_decks: Vec<Vec<usize>>,
    class_clusters: Vec<Vec<Cl>>,
    class_by_trainee: UmaTable<usize>,
    /// clusters of the whole skyline (empty with per-uma decks)
    skyline_clusters: Vec<Cl>,
    /// the clusters' bounds in reused buffers (`bb_max`, `sky_top`)
    cluster_bound_scratch: RefCell<Vec<ByBound>>,
    skyline_bound_scratch: RefCell<Vec<ByBound>>,
    /// B[u][i] = max over all skyline decks (legal for u or not) of T(u, grid[i], deck): no legal deck beats it
    relaxed_ceiling_cache: RefCell<UmaTable<[f64; NG]>>,
    /// exact ceiling table: X[u][i] = best exact value over every legal deck at chance grid[i] under the cap it was
    /// computed for (rises with the chance, so a lineage with chance q is bounded at grid[gidx(q)]; each lineage may
    /// take its own deck)
    exact_ceiling_cache: RefCell<UmaTable<[ExactCeiling; NG]>>,
    /// the deck space's entry per (card id, end link, variant)
    entry_by_card: crate::FxMap<CardEntryKey, usize>,
    exact_cache: RefCell<crate::FxMap<ExactQueryKey, ExactCacheEntry>>,
    shared_cache: RefCell<crate::FxMap<SharedQueryKey, SharedResult>>,
    /// Schedules found per structure (shared by the passes).
    schedule_cache: RefCell<crate::FxMap<[UmaIndex; 4], Res>>,
    tally: Tally,
    exact_caps: [f64; 4],
    maximum_exact_ratio: f64,
    shared: bool,
    loop_limit: usize,
    /// the pool by M_u (`rank`), highest first, and their M_u; the highest structure sum
    ranked_pool: Vec<UmaIndex>,
    ranked_ceilings: Vec<f64>,
    maximum_structure_sum: f64,
    started: Instant,
}

/// The exact search's counters, timers and work budget; `max_ratio` is per structure class (`cap_class`).
#[derive(Default)]
struct Tally {
    scored_decks: Cell<usize>,
    band_decks: Cell<usize>,
    bound_rejections: Cell<usize>,
    cap_hit: Cell<bool>,
    max_ratio: Cell<[f64; 4]>,
    max_deck: RefCell<[Option<RatioExample>; 4]>,
    band_seconds: Cell<f64>,
    score_seconds: Cell<f64>,
    seed_seconds: Cell<f64>,
    bound_seconds: Cell<f64>,
    work: Cell<usize>,
    budget: Cell<usize>,
    /// set when a pass runs out of budget (or a deck search out of room): that pass certifies nothing
    aborted: Cell<bool>,
}
impl Tally {
    fn spend(&self, w: usize) {
        self.work.set(self.work.get() + w);
    }
    fn ratio_at(&self, k: usize, r: f64, u: u32, x: &PDeck) {
        let mut m = self.max_ratio.get();
        if r > m[k] {
            m[k] = r;
            self.max_ratio.set(m);
            self.max_deck.borrow_mut()[k] = Some(RatioExample { trainee_id: u, cards: x.names.clone() });
        }
    }
}

// Structures: four-uma loops (12 lineages) and three-uma rotations (3 lineages; `lins`), members as [UmaIndex; 4] with
// `NONE` last for three; a structure's value is the mean over its lineages, so both sizes rank together.
fn lin_of(f: &[UmaIndex; 4]) -> &'static [MemberLineage] {
    lins(members(f).len())
}
struct TrainingPool {
    eligible: Vec<UmaIndex>,
    required: Vec<UmaIndex>,
}

/// The umas that can train (owned or required, not excluded, with an outfit, not blocked by a locked card) and the
/// required ones
fn trainable(inputs: &LoopJobInputs, card_of: &crate::FxMap<u32, &Card>) -> TrainingPool {
    let id_ok = |id: u32| {
        (inputs.skill_job.owned.is_empty()
            || inputs.skill_job.owned.contains(&id)
            || inputs.skill_job.require.contains(&id))
            && !inputs.skill_job.exclude.contains(&id)
    };
    // a locked card goes in every deck, so a uma it blocks cannot train at all
    let lock_blocked = |u: UmaIndex| {
        inputs.deck_job.lock_cards.iter().any(|id| card_of.get(id).is_some_and(|c| c.blocks(inputs.data.umas[u].id)))
    };
    let pool: Vec<UmaIndex> = inputs
        .data
        .umas
        .indices()
        .filter(|&i| id_ok(inputs.data.umas[i].id) && !inputs.model.no_outfit[i] && !lock_blocked(i))
        .collect();
    let req: Vec<UmaIndex> = inputs
        .skill_job
        .require
        .iter()
        .filter_map(|&id| inputs.data.umas.iter().position(|u| u.id == id).map(UmaIndex))
        .collect();
    TrainingPool { eligible: pool, required: req }
}

impl<'data> Multi<'data> {
    /// Setup: the pool, its pair ceilings and chance grid, the skyline's classes and clusters, and the pool's order;
    /// Err with the job's error when the required umas cannot train or the sizes do not fit
    fn new(inputs: LoopJobInputs<'data>) -> Result<Self, Value> {
        let target_groups: Vec<u32> =
            inputs.model.uncertain.iter().map(|&i| inputs.skill_job.targets[i].group).collect();
        let target_count = target_groups.len();
        let card_of: crate::FxMap<u32, &Card> = inputs.cards.cards.iter().map(|c| (c.id, c)).collect();
        let n = inputs.data.umas.len();
        let TrainingPool { eligible: pool, required: required_umas } = trainable(&inputs, &card_of);
        if required_umas.iter().any(|r| !pool.contains(r)) {
            return Err(json!({"error": "A required uma cannot train with a locked card of her own."}));
        }
        if let Err(e) = check_sizes(&inputs.skill_job.sizes, pool.len(), required_umas.len()) {
            return Err(json!({"error": e}));
        }
        let (allow_three, allow_four) = (inputs.skill_job.sizes.contains(&3), inputs.skill_job.sizes.contains(&4));
        let started = Instant::now();
        let fixed_schedules: Option<UmaTable<Option<Sched>>> = inputs.deck_job.schedule.as_deref().map(|mode| {
            inputs
                .data
                .umas
                .indices()
                .map(|i| pool.contains(&i).then(|| prebuilt(inputs.data, &inputs.data.umas[i], mode)))
                .collect()
        });
        let no_race = inputs.deck_job.schedule.as_deref() == Some("none");
        let pair_bounds = pair_table(inputs.data, &pool, fixed_schedules.as_ref(), no_race);
        let fixed = inputs.model.fixed_factor();
        let maximum_chances: UmaTable<f64> =
            inputs
                .data
                .umas
                .indices()
                .map(|u| {
                    if pool.contains(&u) { inputs.model.q_ub_trainee(u, &pool, &pair_bounds, allow_three) } else { 0.0 }
                })
                .collect();
        let qlo = inputs.model.q_floor();
        let qhi = pool.iter().map(|&u| maximum_chances[u]).fold(qlo, f64::max);
        let grid = Grid::new(qlo, qhi);
        let (class_decks, class_by_trainee) = classes(inputs.model, &pool, inputs.skyline, target_count, n);
        // Clusters of each class's decks (recursive median splits on the widest odds coordinate) with their
        // componentwise maximum odds. T(u, q, deck) rises with every deck odds coordinate (for 0 <= q <= 1, since a
        // white is worth less than a gold), so T at a cluster's maximum bounds every member; maxima over a class are
        // found cluster by cluster, best bound first, skipping clusters whose bound (with 1e-9 relative slack for
        // rounding) is below the best value so far. The maximum of a set does not depend on visit order, so the
        // result is the same value bit for bit.
        let class_clusters = class_clusters(inputs.skyline, &class_decks, target_count);
        if std::env::var("DECK_DEBUG").is_ok() {
            eprintln!(
                "classes {} sizes {:?} ({:.2}s)",
                class_decks.len(),
                class_decks.iter().map(std::vec::Vec::len).collect::<Vec<_>>(),
                started.elapsed().as_secs_f64()
            );
        }
        // A deck's sources: the deck space's entry for (card, end link, variant) where it has one of that card (built
        // by the same `Sources::src` and `kind` on the same job and targets, so the same values), borrowed; else built
        let entry_by_card: crate::FxMap<CardEntryKey, usize> = (0..inputs.deck_space.card_indices.len())
            .map(|j| {
                (
                    CardEntryKey {
                        card_id: inputs.deck_space.cards[inputs.deck_space.card_indices[j]].id,
                        end_link: inputs.deck_space.end_links[j],
                        variant: inputs.deck_space.variants[j],
                    },
                    j,
                )
            })
            .collect();
        let exact_caps = inputs.deck_space.caps();
        let skyline_clusters = full_clusters(inputs.skyline, target_count, inputs.deck_job.per_uma);
        let mut m = Multi {
            target_groups,
            target_count,
            requested_target_count: inputs.skill_job.targets.len(),
            card_of,
            pool,
            required_umas,
            allow_three,
            allow_four,
            fixed_schedules,
            no_race,
            pair_bounds,
            fixed,
            maximum_chances,
            grid,
            no_hint_chances: vec![0.0f64; target_count],
            class_decks,
            class_clusters,
            class_by_trainee,
            skyline_clusters,
            cluster_bound_scratch: RefCell::default(),
            skyline_bound_scratch: RefCell::default(),
            relaxed_ceiling_cache: RefCell::new(UmaTable(vec![[f64::NAN; NG]; n])),
            exact_ceiling_cache: RefCell::new(UmaTable(vec![
                [ExactCeiling { value: f64::NAN, certified_cap: 0.0 }; NG];
                n
            ])),
            entry_by_card,
            exact_cache: RefCell::default(),
            shared_cache: RefCell::default(),
            schedule_cache: RefCell::default(),
            tally: Tally::default(),
            exact_caps,
            maximum_exact_ratio: exact_caps[3],
            shared: !inputs.deck_job.per_uma,
            loop_limit: inputs.deck_job.top_decks.max(1),
            ranked_pool: vec![],
            ranked_ceilings: vec![],
            maximum_structure_sum: 0.0,
            started,
            inputs,
        };
        m.rank();
        Ok(m)
    }
}
fn qkey(qs: &[f64]) -> ChanceKey {
    assert!(qs.len() <= 12, "more than 12 chances in a memo key");
    let mut k = [0u64; 12];
    for (x, q) in k.iter_mut().zip(qs) {
        *x = q.to_bits();
    }
    ChanceKey { bits: k, count: qs.len() as u8 }
}

/// Fixed schedule for one uma (see DeckJob::schedule).
fn prebuilt(d: &Data, u: &Uma, mode: &str) -> Sched {
    let solo = u.best_response(d.nr, &vec![1.0; d.nr], None).expect("infeasible goals").0;
    if mode != "goals" {
        return solo;
    }

    let mut schedule = solo;
    let mut covered_goals = 0u32;
    for turn in 0..NT {
        if let Choice::Opt(option) = schedule[turn] {
            let goal_bits = u.bits[turn][option as usize];
            if u.forced[turn].is_some() || goal_bits & !covered_goals != 0 {
                covered_goals |= goal_bits;
            } else {
                schedule[turn] = Choice::Rest;
            }
        }
    }
    if u.feasible(&schedule) { schedule } else { solo }
}
