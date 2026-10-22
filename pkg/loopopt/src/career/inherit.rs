//! Legacy slots, affinity and inheritance chances.
use crate::career::pool::{Pool, Who, shared};
use crate::career::{CareerSetup, Lineage, SearchState};
use crate::deck::joint::{JointExt, joint_fast};
use crate::deck::sources::TargetSources;
use crate::deck::space::SelectedDeck;
/// The inheritance model's per-slot terms (MODEL.md; /about states the constants), shared by every copy of the
/// inheritance chance (`inherit`, `CareerSetup::lineage_qc`, `CareerSetup::gen_cands`, `Evaluator::fast_ub`, the
/// planner's her-slot setups).
/// A slot's affinity factor 1 + A / 100, A its relation part plus its race part
#[inline(always)]
pub(crate) fn aff_factor(relation_points: f64, race: f64) -> f64 {
    1.0 + (relation_points + race) / 100.0
}

/// one inspiration's miss of a spark at `stars` stars from a slot of affinity factor `affinity_factor`
#[inline(always)]
pub(crate) fn insp_miss(stars: f64, affinity_factor: f64) -> f64 {
    1.0 - (crate::skills::INSP_PER_STAR * stars * affinity_factor).min(1.0)
}

/// both inspirations' miss of that spark
#[inline(always)]
pub(crate) fn insp_miss2(stars: f64, affinity_factor: f64) -> f64 {
    let miss = insp_miss(stars, affinity_factor);
    miss * miss
}

/// a random-star slot's average squared miss over its star mix w (1, 2, 3 stars)
#[inline(always)]
pub(crate) fn mix_term(star_mix: [f64; 3], affinity_factor: f64) -> f64 {
    (0..3).map(|i| star_mix[i] * insp_miss2(i as f64 + 1.0, affinity_factor)).sum::<f64>()
}

/// the inheritance chance from the product of the fixed-star slots' misses and of the random-star slots' mix terms
#[inline(always)]
pub(crate) fn inherit_q(miss: f64, random_star_misses: f64) -> f64 {
    1.0 - miss * miss * random_star_misses
}

/// The spark-generation scale of a target n of the six legacy slots carry: 1.1^(n - 6) (black_box: the runtime powi
/// every caller computes, never constant-folded)
#[inline(always)]
pub(crate) fn spark_scale(n: i32) -> f64 {
    1.1f64.powi(std::hint::black_box(n - 6))
}

/// `CareerSetup::rise`'s factor for one outfit and uncertain target: how far the landing chance can grow from
/// inheritance chance `base_chance` to q, with her own event hint chance `hint_chance` and her outfit's coverage
/// `outfit_stars` of it (1 where it cannot grow). The one form behind `solve`'s Ref and ruled prunes and `outcome_hi`'s
/// anchors (`rise_leaves`, `rise_factors`, `AnchorBound::root_min`)
#[inline(always)]
pub(crate) fn rise_factor(outfit_stars: u8, hint_chance: f64, q: f64, base_chance: f64) -> f64 {
    if outfit_stars > 0 || q <= base_chance {
        1.0
    } else {
        (1.0 - (1.0 - hint_chance) * (1.0 - q)) / (1.0 - (1.0 - hint_chance) * (1.0 - base_chance))
    }
}

/// A slot's affinity part summed as every copy sums it: `first`, then each part in order
#[inline(always)]
pub(crate) fn part_sum(first: f64, parts: impl IntoIterator<Item = f64>) -> f64 {
    parts.into_iter().fold(first, |sum, part| sum + part)
}

/// The trainee's relation points a slot walk (`CareerSetup::walk_slots`) reads: with legacy `legacy` of the pool (`pt`)
/// and with her grandparent slot `grandparent_slot` (`gt`)
pub(crate) trait RelPts {
    fn pt(&self, legacy: usize) -> f64;
    fn gt(&self, legacy: usize, grandparent_slot: usize) -> f64;
}

/// trainee t's, from the relation tables (`CareerSetup::pair`, `CareerSetup::tri`)
pub(crate) struct RelOf<'setup, 'data> {
    pub(crate) setup: &'setup CareerSetup<'data>,
    pub(crate) pool: &'setup Pool,
    pub(crate) trainee: crate::uma::UmaIndex,
}

impl RelPts for RelOf<'_, '_> {
    #[inline(always)]
    fn pt(&self, legacy: usize) -> f64 {
        self.setup.pair(Some(self.trainee), self.pool.veterans[legacy].uma_index)
    }
    #[inline(always)]
    fn gt(&self, legacy: usize, grandparent_slot: usize) -> f64 {
        self.setup.tri(
            Some(self.trainee),
            self.pool.veterans[legacy].uma_index,
            self.pool.parents[legacy][grandparent_slot].uma_index,
        )
    }
}

/// What a slot walk feeds: each of the six legacy slots in `slots`' order (parent a, her grandparent slots, then parent
/// b and hers) with its legacy, its grandparent slot (None: the parent herself), who fills it, and its relation and
/// race parts (a parent's: the sums over her and her grandparent slots)
pub(crate) trait SlotSink<'pool> {
    fn slot(
        &mut self,
        legacy: usize,
        grandparent_slot: Option<usize>,
        who: &'pool Who,
        relation_points: f64,
        race: f64,
    );
}

/// `inherit`'s accumulation, slot by slot (per target the same factors in slot order): the fixed-star slots' miss
/// product, the random-star slots' mix-term product and the carrier count. `skip`: a legacy whose parent slot is left
/// out (as if she carried nothing), its affinity factor kept in `her`
struct Inherit {
    miss: Vec<f64>,
    random_star_misses: Vec<f64>,
    carriers: Vec<i32>,
    skip: Option<usize>,
    skipped_affinity: Option<f64>,
}

impl Inherit {
    fn new(target_count: usize, skip: Option<usize>) -> Self {
        Self {
            miss: vec![1.0; target_count],
            random_star_misses: vec![1.0; target_count],
            carriers: vec![0; target_count],
            skip,
            skipped_affinity: None,
        }
    }
    #[inline(always)]
    fn add(&mut self, stars: &[u8], mix: Option<[f64; 3]>, affinity_factor: f64) {
        for k in 0..self.carriers.len() {
            let target_stars = stars[k];
            if target_stars == 0 {
                continue;
            }
            self.carriers[k] += 1;
            match mix {
                None => self.miss[k] *= insp_miss(f64::from(target_stars), affinity_factor),
                Some(star_mix) => self.random_star_misses[k] *= mix_term(star_mix, affinity_factor),
            }
        }
    }
    fn finish(self) -> Inheritance {
        Inheritance {
            chances: self
                .miss
                .iter()
                .zip(&self.random_star_misses)
                .map(|(&miss, &random_star_miss)| inherit_q(miss, random_star_miss))
                .collect(),
            carriers: self.carriers,
        }
    }
}

/// `Inherit` without a skipped slot, in fixed arrays (at most `LQN` job targets; `CareerSetup::lineage`'s): the same
/// factors multiplied in the same slot order
const LQN: usize = 16;

struct InheritA {
    target_count: usize,
    miss: [f64; LQN],
    random_star_misses: [f64; LQN],
    carriers: [i32; LQN],
}

impl SlotSink<'_> for InheritA {
    #[inline(always)]
    fn slot(&mut self, _: usize, _: Option<usize>, who: &Who, relation_points: f64, race: f64) {
        let affinity_factor = aff_factor(relation_points, race);
        for k in 0..self.target_count {
            let target_stars = who.stars[k];
            if target_stars == 0 {
                continue;
            }
            self.carriers[k] += 1;
            match who.star_mix {
                None => self.miss[k] *= insp_miss(f64::from(target_stars), affinity_factor),
                Some(star_mix) => self.random_star_misses[k] *= mix_term(star_mix, affinity_factor),
            }
        }
    }
}

impl SlotSink<'_> for Inherit {
    #[inline(always)]
    fn slot(&mut self, legacy: usize, grandparent_slot: Option<usize>, who: &Who, relation_points: f64, race: f64) {
        if grandparent_slot.is_none() && self.skip == Some(legacy) {
            self.skipped_affinity = Some(aff_factor(relation_points, race));
            return;
        }
        self.add(&who.stars, who.star_mix, aff_factor(relation_points, race));
    }
}

/// A legacy slot of the planner's veteran-to-be (`CareerSetup::her_slot`): the trainee's inheritance with her slot left
/// out, and what her veteran in the slot adds.
pub(crate) struct HerSlot {
    /// q0: per job target, the trainee's inheritance chance with her slot left out.
    pub(crate) chances_without_her: Vec<f64>,
    /// x0: per uncertain target (`CareerSetup::uncertain_targets` order), 1 - q0.
    pub(crate) misses_without_her: Vec<f64>,
    /// xms: her miss factor squared per star count (1 at 0 stars).
    pub(crate) miss_by_stars: [f64; 4],
    /// pw: per job target, the spark scale without and with her spark (from the carriers without her, nc0).
    pub(crate) target_spark_scales: Vec<[f64; 2]>,
}

/// One legacy slot of a lineage: affinity parts and per-target stars.
#[derive(Clone)]
pub(crate) struct Slot {
    pub(crate) parent: bool,
    pub(crate) legacy: usize,
    pub(crate) grandparent_slot: Option<usize>,
    pub(crate) character_id: u32,
    pub(crate) relation_points: f64,
    pub(crate) race: f64,
    pub(crate) stars: std::rc::Rc<Vec<u8>>,
    pub(crate) pinks: std::rc::Rc<Vec<(u32, u8)>>,
    pub(crate) mix: Option<[f64; 3]>,
}

/// Fold inheritance into the trainee buffer `event_chances`, reading her immutable own-event chances
/// `own_event_chances`. Kept separate from `CareerSetup` so callers can keep the deck space borrowed.
pub(crate) fn set_ews(
    event_chances: &mut Vec<Vec<f64>>,
    own_event_chances: &[Vec<f64>],
    uncertain_targets: &[usize],
    q: &[f64],
) {
    // (in place when the shape is already this one: the same values)
    if event_chances.len() == own_event_chances.len()
        && event_chances
            .iter()
            .zip(own_event_chances)
            .all(|(chance, own)| chance.len() == own.len().min(uncertain_targets.len()))
    {
        for (chance, own) in event_chances.iter_mut().zip(own_event_chances) {
            for ((chance, e), &i) in chance.iter_mut().zip(own).zip(uncertain_targets) {
                *chance = 1.0 - (1.0 - e) * (1.0 - q[i]);
            }
        }
    } else {
        *event_chances = own_event_chances
            .iter()
            .map(|own| own.iter().zip(uncertain_targets).map(|(e, &i)| 1.0 - (1.0 - e) * (1.0 - q[i])).collect())
            .collect();
    }
}

/// What `CareerSetup::exact` computes beside P(all).
#[derive(Clone, Copy)]
pub(crate) struct ExactOptions {
    /// Also P(all but one) (else 0).
    pub(crate) all_but_one: bool,
    /// Also the chance of each set of uncertain targets landing (`JointExt::sets`, read by
    /// `CareerSetup::outcome_sets`).
    pub(crate) landed_sets: bool,
}

/// One career's exact numbers (`CareerSetup::exact`), at her best outfit.
pub(crate) struct CareerChances {
    pub(crate) p_all: f64,
    /// P(every target but at most one lands); 0 unless asked for.
    pub(crate) p_all_but_one: f64,
    /// The best outfit's index in her model options.
    pub(crate) outfit: usize,
    /// The per-target scales, marginals and landed-set chances (`JointExt`).
    pub(crate) extras: JointExt,
}

/// A lineage's inheritance, per job target.
pub(crate) struct Inheritance {
    /// The chance q_k that the trainee inherits the target's spark.
    pub(crate) chances: Vec<f64>,
    /// How many of the six legacy slots carry a spark of the target (n_k, the spark scale's count).
    pub(crate) carriers: Vec<i32>,
}

/// `Inheritance` with the spark scale C of its carrier counts (`CareerSetup::lineage`).
pub(crate) struct LineageChances {
    /// The chance q_k per job target.
    pub(crate) chances: Vec<f64>,
    /// Carrier count n_k per job target.
    pub(crate) carriers: Vec<i32>,
    /// C, the product of the targets' spark scales.
    pub(crate) spark_scale: f64,
}

/// Per job target: the inheritance chance and how many of the six carry the spark. A slot with random stars (`mix`)
/// enters as the average of its squared miss factor (its stars are independent of everything else).
pub(crate) fn inherit(slots: &[Slot], target_count: usize) -> Inheritance {
    let mut accumulator = Inherit::new(target_count, None);
    for slot in slots {
        accumulator.add(&slot.stars, slot.mix, aff_factor(slot.relation_points, slot.race));
    }
    accumulator.finish()
}

impl CareerSetup<'_> {
    pub(crate) fn pair(&self, a: Option<crate::uma::UmaIndex>, b: Option<crate::uma::UmaIndex>) -> f64 {
        match (a, b) {
            (Some(a), Some(b)) => f64::from(self.data.pair_relations[a][b]),
            _ => 0.0,
        }
    }
    pub(crate) fn tri(
        &self,
        a: Option<crate::uma::UmaIndex>,
        b: Option<crate::uma::UmaIndex>,
        c: Option<crate::uma::UmaIndex>,
    ) -> f64 {
        match (a, b, c) {
            (Some(a), Some(b), Some(c)) => f64::from(self.data.trio_relations[a][b][c]),
            _ => 0.0,
        }
    }
    /// The six legacy slots with parents a = `first_parent` and b = `second_parent`, in order, into `sink`
    /// (`SlotSink`), the trainee's relation points from `relations`: per parent x (partner y) her relation part,
    /// pt(x) + pair(x, y) then her grandparent slots' gt(x, gi) in order, and her race part, 3 x shared(a, b) then
    /// theirs (`Pool::grandparent_race_affinity`) in order (`part_sum`); a grandparent slot's parts are its own. The
    /// one definition of the slots' affinity every copy reads (`slots`, `lineage`, `lineage_qc`, `her_slot`,
    /// `Evaluator::fast_ub`; `gen_cands` hoists the same sums)
    #[inline(always)]
    pub(crate) fn walk_slots<'pool, R: RelPts, S: SlotSink<'pool>>(
        &self,
        pool: &'pool Pool,
        first_parent: usize,
        second_parent: usize,
        relations: &R,
        sink: &mut S,
    ) {
        // (the parents' shared wins once: shared(a, b) and shared(b, a) are the same products summed in the same order)
        let shared_race = 3.0 * shared(&pool.veterans[first_parent], &pool.veterans[second_parent]);
        for (parent, partner) in [(first_parent, second_parent), (second_parent, first_parent)] {
            let veteran = &pool.veterans[parent];
            let grandparent_count = pool.parents[parent].len();
            let relation_points = part_sum(
                relations.pt(parent) + self.pair(veteran.uma_index, pool.veterans[partner].uma_index),
                (0..grandparent_count).map(|grandparent_slot| relations.gt(parent, grandparent_slot)),
            );
            let race = part_sum(shared_race, pool.grandparent_race_affinity[parent].iter().copied());
            sink.slot(parent, None, veteran, relation_points, race);
            for (grandparent_slot, grandparent) in pool.parents[parent].iter().enumerate() {
                sink.slot(
                    parent,
                    Some(grandparent_slot),
                    grandparent,
                    relations.gt(parent, grandparent_slot),
                    pool.grandparent_race_affinity[parent][grandparent_slot],
                );
            }
        }
    }
    /// the six legacy slots of the lineage's trainee with her parents, first parent's first
    pub(crate) fn slots(&self, pool: &Pool, lineage: Lineage) -> Vec<Slot> {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        struct V(Vec<Slot>);
        impl SlotSink<'_> for V {
            fn slot(
                &mut self,
                legacy: usize,
                grandparent_slot: Option<usize>,
                who: &Who,
                relation_points: f64,
                race: f64,
            ) {
                self.0.push(Slot {
                    parent: grandparent_slot.is_none(),
                    legacy,
                    grandparent_slot,
                    character_id: who.character_id,
                    relation_points,
                    race,
                    stars: std::rc::Rc::clone(&who.stars),
                    pinks: std::rc::Rc::clone(&who.pinks),
                    mix: who.star_mix,
                });
            }
        }
        let mut sink = V(Vec::with_capacity(2 + pool.parents[first_parent].len() + pool.parents[second_parent].len()));
        self.walk_slots(pool, first_parent, second_parent, &RelOf { setup: self, pool, trainee }, &mut sink);
        sink.0
    }
    /// Legacy `her`'s slot as a parent of the trainee beside `partner` (`HerSlot`; None when she is not one of the
    /// two); what `score_career`'s setups and `outcome_hi`'s partners take
    pub(crate) fn her_slot(&self, pool: &Pool, lineage: Lineage) -> Option<HerSlot> {
        let Lineage { trainee, first_parent: partner, second_parent: her } = lineage;
        let mut accumulator = Inherit::new(self.target_count, Some(her));
        self.walk_slots(pool, partner, her, &RelOf { setup: self, pool, trainee }, &mut accumulator);
        let affinity_factor = accumulator.skipped_affinity?;
        let Inheritance { chances: chances_without_her, carriers: carriers_without_her } = accumulator.finish();
        let miss_at = |stars: u8| insp_miss2(f64::from(stars), affinity_factor);
        Some(HerSlot {
            misses_without_her: self.uncertain_targets.iter().map(|&i| 1.0 - chances_without_her[i]).collect(),
            miss_by_stars: [1.0, miss_at(1), miss_at(2), miss_at(3)],
            target_spark_scales: carriers_without_her.iter().map(|&n| [spark_scale(n), spark_scale(n + 1)]).collect(),
            chances_without_her,
        })
    }
    pub(crate) fn scale(&self, carriers: &[i32]) -> crate::deck::joint::SparkScales {
        crate::deck::joint::SparkScales {
            uncertain: self.uncertain_targets.iter().map(|&i| spark_scale(carriers[i])).collect(),
            guaranteed: self.fixed_targets.iter().map(|&i| spark_scale(carriers[i])).collect(),
        }
    }
    /// The deck search's chances: the model with the search state's folded buffers.
    pub(crate) const fn deck_chances<'search>(
        &'search self,
        state: &'search SearchState,
    ) -> crate::skills::DeckChances<'search> {
        crate::skills::DeckChances { model: self.model, event_chances: &state.folded_events }
    }
    /// Trainee `trainee`'s exact-scoring query at her folded chances (`set_trainee`).
    pub(crate) fn joint_query<'search>(
        &'search self,
        state: &'search SearchState,
        trainee: crate::uma::UmaIndex,
    ) -> crate::deck::joint::JointQuery<'search> {
        crate::deck::joint::JointQuery {
            chain_events: self.space.deck_job.chain_events(),
            checks: crate::checks::get(),
            trainee,
            inheritance_chances: &[0.0],
            event_chances: &state.folded_events[trainee],
        }
    }
    /// trainee t's event hint chances with her lineage's inheritance chances folded in (the search then runs at q = 0)
    pub(crate) fn set_trainee(&self, state: &mut SearchState, trainee: crate::uma::UmaIndex, q: &[f64]) {
        set_ews(&mut state.folded_events[trainee], &self.model.event_chances[trainee], &self.uncertain_targets, q);
    }
    /// a deck's entries (sources borrowed from the deck space: `joint` only reads them)
    pub(crate) fn ents_of(&self, deck: &SelectedDeck) -> Vec<crate::deck::sources::SourceEntry<&TargetSources>> {
        deck.entries
            .iter()
            .map(|&j| crate::deck::sources::SourceEntry { sources: &self.space.sources[j], kind: self.space.kind[j] })
            .collect()
    }
    /// exact P(all) of trainee t (her inheritance already folded in) with deck `deck`, scaled per target by the
    /// carrier counts `carriers`; `options` says what else to compute
    pub(crate) fn exact(
        &self,
        state: &SearchState,
        trainee: crate::uma::UmaIndex,
        deck: &SelectedDeck,
        carriers: &[i32],
        options: ExactOptions,
    ) -> CareerChances {
        let scales = self.scale(carriers);
        let mut extras = JointExt {
            scales,
            marginals: vec![],
            sets: if options.landed_sets { vec![0.0] } else { vec![] },
            per_outfit: vec![],
        };
        let crate::deck::joint::JointChances { all: p_all, all_but_one: p_all_but_one, outfit } = joint_fast(
            self.model,
            self.joint_query(state, trainee),
            &self.ents_of(deck),
            deck.filler_cards,
            self.target_count,
            options.all_but_one,
            Some(&mut extras),
        )[0];
        CareerChances { p_all, p_all_but_one, outfit, extras }
    }

    /// How far trainee t's best exact value (before the spark scale) can rise from chances q0 (`base_chances`) to q. A
    /// target the outfit lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no
    /// hint, e: her own event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 -
    /// q0_k)) (n = 1, g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products
    /// of these factors, so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of it.
    pub(crate) fn rise(&self, trainee: crate::uma::UmaIndex, q: &[f64], base_chances: &[f64]) -> f64 {
        self.outfit_coverage[trainee]
            .iter()
            .zip(&self.model.event_chances[trainee])
            .map(|(outfit_stars, hint_chances)| {
                self.uncertain_targets
                    .iter()
                    .enumerate()
                    .map(|(k, &i)| rise_factor(outfit_stars[k], hint_chances[k], q[i], base_chances[i]))
                    .product::<f64>()
            })
            .fold(0.0f64, f64::max)
    }
    /// The lineage's inheritance chances, carriers and spark scale C.
    pub(crate) fn lineage(&self, pool: &Pool, lineage: Lineage) -> LineageChances {
        if self.target_count > LQN {
            let Lineage { trainee, first_parent, second_parent } = lineage;
            let mut accumulator = Inherit::new(self.target_count, None);
            self.walk_slots(pool, first_parent, second_parent, &RelOf { setup: self, pool, trainee }, &mut accumulator);
            let Inheritance { chances, carriers } = accumulator.finish();
            let spark_scale = self.c_of(&carriers);
            return LineageChances { chances, carriers, spark_scale };
        }
        let accumulator = self.lineage_acc(pool, lineage);
        LineageChances {
            chances: (0..accumulator.target_count)
                .map(|k| inherit_q(accumulator.miss[k], accumulator.random_star_misses[k]))
                .collect(),
            carriers: accumulator.carriers[..accumulator.target_count].to_vec(),
            spark_scale: self.c_of(&accumulator.carriers),
        }
    }
    /// `lineage`'s chances and spark scale C (the same numbers), without the carriers
    pub(crate) fn lineage_q(&self, pool: &Pool, lineage: Lineage) -> (Vec<f64>, f64) {
        let mut q = Vec::with_capacity(self.target_count);
        let spark_scale = self.lineage_into(pool, lineage, &mut q);
        (q, spark_scale)
    }
    /// `lineage`'s chances into `q` (cleared first) and its spark scale C, the same numbers
    pub(crate) fn lineage_into(&self, pool: &Pool, lineage: Lineage, q: &mut Vec<f64>) -> f64 {
        q.clear();
        if self.target_count > LQN {
            let chances = self.lineage(pool, lineage);
            q.extend_from_slice(&chances.chances);
            return chances.spark_scale;
        }
        let accumulator = self.lineage_acc(pool, lineage);
        q.extend(
            (0..accumulator.target_count).map(|k| inherit_q(accumulator.miss[k], accumulator.random_star_misses[k])),
        );
        self.c_of(&accumulator.carriers)
    }
    /// `inherit`'s accumulation over the lineage's six slots (ntar <= LQN)
    #[inline(always)]
    fn lineage_acc(&self, pool: &Pool, lineage: Lineage) -> InheritA {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        let mut accumulator = InheritA {
            target_count: self.target_count,
            miss: [1.0; LQN],
            random_star_misses: [1.0; LQN],
            carriers: [0; LQN],
        };
        self.walk_slots(pool, first_parent, second_parent, &RelOf { setup: self, pool, trainee }, &mut accumulator);
        accumulator
    }
    /// the spark scale C of carrier counts nc: `scale`'s factors multiplied from 1.0, uncertain targets then fixed ones
    #[inline(always)]
    fn c_of(&self, carriers: &[i32]) -> f64 {
        self.uncertain_targets.iter().chain(&self.fixed_targets).map(|&i| spark_scale(carriers[i])).product::<f64>()
    }
    /// `lineage`'s spark scale C, and its inheritance chances for the targets `target_indices` only (into q[i] for each
    /// i of them, the other entries of q left as they are), without allocating: the same slot walk (`walk_slots`) into
    /// `inherit`'s accumulation for those targets (each slot's factors multiplied in slot order, a slot without the
    /// spark skipped) and the carriers of every target, then `scale`'s product. More than four grandparent slots per
    /// parent, or more than QN targets: through `lineage` itself. `relation_cache`: the trainee's relation points with
    /// each legacy x of the pool (`pair`, at 5 x + 4) and with x's grandparent slots (`tri`, at 5 x + the slot), then
    /// `scale`'s factor per carrier count; built here when empty: the caller empties it for another trainee or pool.
    pub(crate) fn lineage_qc(
        &self,
        pool: &Pool,
        lineage: Lineage,
        target_indices: &[usize],
        q: &mut [f64],
        relation_cache: &mut Vec<f64>,
    ) -> f64 {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        const G: usize = 4;
        const QN: usize = 16;
        if pool.parents[first_parent].len() > G || pool.parents[second_parent].len() > G || self.target_count > QN {
            let chances = self.lineage(pool, lineage);
            for &i in target_indices {
                q[i] = chances.chances[i];
            }
            return chances.spark_scale;
        }
        let trainee_index = Some(trainee);
        let legacy_count = pool.veterans.len();
        if relation_cache.is_empty() {
            relation_cache.resize(legacy_count * (G + 1), 0.0);
            for legacy in 0..legacy_count {
                relation_cache[legacy * (G + 1) + G] = self.pair(trainee_index, pool.veterans[legacy].uma_index);
                for (grandparent_slot, grandparent) in pool.parents[legacy].iter().enumerate().take(G) {
                    relation_cache[legacy * (G + 1) + grandparent_slot] =
                        self.tri(trainee_index, pool.veterans[legacy].uma_index, grandparent.uma_index);
                }
            }
            // then `spark_scale` per carrier count n
            relation_cache.extend((0..=2 * (G + 1)).map(|n| spark_scale(n as i32)));
        }
        struct Tg<'tg>(&'tg [f64]);
        impl RelPts for Tg<'_> {
            #[inline(always)]
            fn pt(&self, legacy: usize) -> f64 {
                self.0[legacy * (G + 1) + G]
            }
            #[inline(always)]
            fn gt(&self, legacy: usize, grandparent_slot: usize) -> f64 {
                self.0[legacy * (G + 1) + grandparent_slot]
            }
        }
        // `Inherit` for the targets `target_indices` (and the carriers of every target), in fixed arrays
        struct Qc<'target_indices> {
            target_indices: &'target_indices [usize],
            target_count: usize,
            miss: [f64; QN],
            random_star_misses: [f64; QN],
            carriers: [usize; QN],
        }
        impl SlotSink<'_> for Qc<'_> {
            #[inline(always)]
            fn slot(&mut self, _: usize, _: Option<usize>, who: &Who, relation_points: f64, race: f64) {
                let affinity_factor = aff_factor(relation_points, race);
                for &k in self.target_indices {
                    let target_stars = who.stars[k];
                    if target_stars == 0 {
                        continue;
                    }
                    match who.star_mix {
                        None => self.miss[k] *= insp_miss(f64::from(target_stars), affinity_factor),
                        Some(star_mix) => self.random_star_misses[k] *= mix_term(star_mix, affinity_factor),
                    }
                }
                for k in 0..self.target_count {
                    if who.stars[k] != 0 {
                        self.carriers[k] += 1;
                    }
                }
            }
        }
        let mut accumulator = Qc {
            target_indices,
            target_count: self.target_count,
            miss: [1.0; QN],
            random_star_misses: [1.0; QN],
            carriers: [0; QN],
        };
        self.walk_slots(
            pool,
            first_parent,
            second_parent,
            &Tg(&relation_cache[..legacy_count * (G + 1)]),
            &mut accumulator,
        );
        for &k in target_indices {
            q[k] = inherit_q(accumulator.miss[k], accumulator.random_star_misses[k]);
        }
        let spark_scales = &relation_cache[legacy_count * (G + 1)..];
        self.uncertain_targets
            .iter()
            .chain(&self.fixed_targets)
            .map(|&i| spark_scales[accumulator.carriers[i]])
            .product::<f64>()
    }
}

#[cfg(test)]
mod tests {
    use super::set_ews;

    #[test]
    fn refolding_reuses_outfit_buffers() {
        let own = vec![vec![0.25, 0.5], vec![0.75, 0.0]];
        let mut folded = Vec::new();
        set_ews(&mut folded, &own, &[0, 2], &[0.5, 0.0, 1.0]);
        assert_eq!(folded, vec![vec![0.625, 1.0], vec![0.875, 1.0]]);
        let outer = folded.as_ptr();
        let outfits: Vec<_> = folded.iter().map(Vec::as_ptr).collect();
        set_ews(&mut folded, &own, &[0, 2], &[0.0, 1.0, 0.5]);
        assert_eq!(folded, vec![vec![0.25, 0.75], vec![0.75, 0.5]]);
        assert_eq!(folded.as_ptr(), outer);
        assert!(folded.iter().zip(outfits).all(|(row, pointer)| row.as_ptr() == pointer));
    }
}
