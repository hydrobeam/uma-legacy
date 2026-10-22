//! Career candidates, bounds and the best careers.
use crate::career::clock::{now_ms, prof};
use crate::career::inherit::{aff_factor, inherit_q, insp_miss, mix_term, part_sum, spark_scale};
use crate::career::pool::{Pool, holds_fix, shared};
use crate::career::state::{KeptSolve, KeptState, ProductProof, SOLVE_KEEP};
use crate::career::{CareerSetup, Env, Lineage, SearchState};
use crate::deck::space::SelectedDeck;
use crate::uma::{UmaIndex, UmaTable};
/// How a bound proves a career out (`CareerSetup::ceiling_out`, `refs_out`, `ruled_out`): `Below(th)`, `solve`'s prune,
/// the bound below the bar with its slack s, b (1 + s) < th (a computed bound a hair under the true one still proves
/// it); or `AtMost(v)`, `certified`'s, the bound at most v up to rounding, b <= v (1 + 1e-12): the career can't beat v,
/// and a tie proves value equality, all a certificate claims. The 1e-12 is rounding only: v is a proxy's value through
/// its deck's corner table and the bound the same deck's exact value, the same number computed two ways.
#[derive(Clone, Copy)]
pub(crate) enum Bar {
    Below(f64),
    AtMost(f64),
}

impl Bar {
    #[inline(always)]
    fn out(self, bound: f64, slack: f64) -> bool {
        match self {
            Self::Below(threshold) => bound * (1.0 + slack) < threshold,
            Self::AtMost(limit) => bound <= limit * (1.0 + 1e-12),
        }
    }
}

/// `CareerSetup::ranked_pairs`' pairs, sorted lazily as `solve`'s candidates are: `gen_cands` emits one trainee's pairs
/// in (a, b) order, so the stable sort by ceiling (descending, `partial_cmp`: 0 and -0 tie) is the total order (ceiling
/// descending, a, b), unique per pair; each step past the ordered prefix `v[..upto]` selects the next chunk (everything
/// before it orders before everything after it) and sorts only the chunk, so the sequence is the full sort's, element
/// for element, and a caller that stops early sorts little
pub(crate) struct Ranked {
    candidates: Vec<RankedPair>,
    sorted_until: usize,
    next_index: usize,
}

/// One of a trainee's legal parent pairs with its ceiling (`CareerSetup::ranked_pairs`).
#[derive(Clone, Copy)]
pub(crate) struct RankedPair {
    /// The pair's ceiling for the trainee.
    pub(crate) ceiling: f64,
    /// The first parent (legacy index).
    pub(crate) first_parent: usize,
    /// The second parent (legacy index).
    pub(crate) second_parent: usize,
}

impl Iterator for Ranked {
    type Item = RankedPair;
    fn next(&mut self) -> Option<Self::Item> {
        if self.next_index >= self.candidates.len() {
            return None;
        }
        if self.next_index >= self.sorted_until {
            let order = |first: &RankedPair, second: &RankedPair| {
                second
                    .ceiling
                    .partial_cmp(&first.ceiling)
                    .unwrap()
                    .then(first.first_parent.cmp(&second.first_parent))
                    .then(first.second_parent.cmp(&second.second_parent))
            };
            let rest = &mut self.candidates[self.sorted_until..];
            // (chunks of 256, then 4x what is sorted: each selection scans the whole remainder)
            let chunk = (4 * self.sorted_until).max(256).min(rest.len());
            if chunk < rest.len() {
                rest.select_nth_unstable_by(chunk - 1, order);
            }
            rest[..chunk].sort_unstable_by(order);
            self.sorted_until += chunk;
        }
        let pair = self.candidates[self.next_index];
        self.next_index += 1;
        Some(pair)
    }
}

/// A candidate the search solved: trainee, parents (legacy indices), value C x E, its ceiling, its best deck.
#[derive(Clone)]
pub(crate) struct Found {
    /// The trainee (uma index).
    pub(crate) trainee: UmaIndex,
    /// The first parent (legacy index).
    pub(crate) first_parent: usize,
    /// The second parent (legacy index).
    pub(crate) second_parent: usize,
    /// The career's value C x E: the spark scale times her best deck's exact value.
    pub(crate) value: f64,
    /// The ceiling the search bounded the career with before solving it.
    pub(crate) ceiling: f64,
    /// Her best deck.
    pub(crate) deck: SelectedDeck,
}

impl Found {
    /// Her trainee and parents, in the order recorded.
    pub(crate) const fn lineage(&self) -> Lineage {
        Lineage { trainee: self.trainee, first_parent: self.first_parent, second_parent: self.second_parent }
    }
}

/// A trainee's fully checked deck: the chances it was found at (job-target order), its exact value there before the
/// spark scale (the best of every legal deck, under the EXACT_RHO certificate, or cap-free after a rerun), the deck. Its
/// value bounds every deck at other chances through `rise`.
#[derive(Clone)]
pub(crate) struct Ref {
    pub(crate) inheritance_chances: Vec<f64>,
    pub(crate) value: f64,
    pub(crate) deck: SelectedDeck,
}

#[derive(Clone)]
pub(crate) enum DeckRes {
    NoDeck,
    Bounded,
    Done(f64, SelectedDeck, bool),
}

#[derive(Clone)]
/// (`cert`: the lowest cap a deck search of it was certified under, INFINITY when none was lowered; see
/// `DeckSearchAudit::cert`)
pub(crate) struct CareerSearchResult {
    pub(crate) found: Vec<Found>,
    pub(crate) refs: UmaTable<Vec<Ref>>,
    pub(crate) work: usize,
    pub(crate) candidates: usize,
    pub(crate) solved: usize,
    pub(crate) bounded: usize,
    pub(crate) scored: usize,
    pub(crate) skipped: usize,
    pub(crate) next_bound: f64,
    pub(crate) complete: bool,
    pub(crate) proven: bool,
    pub(crate) certified_cap: f64,
}

impl CareerSetup<'_> {
    /// The bound a fully checked deck search `reference` (a `Ref`: chances r.inheritance_chances, best value r.value
    /// before the spark scale) puts on every career of the trainee's deck class at chances q with spark scale c:
    /// c r.value rise(t, q, r.inheritance_chances) (`CareerSetup::rise`)
    #[inline]
    pub(crate) fn ref_bound(&self, trainee: UmaIndex, q: &[f64], career_spark_scale: f64, reference: &Ref) -> f64 {
        career_spark_scale * reference.value * self.rise(trainee, q, &reference.inheritance_chances)
    }
    /// The bound a product search's proof (`ruled`: chances rq = `proof_chances`, best product value s =
    /// `product_bound`) puts on it there: EXACT_RHO c s rise(t, q, rq) (`solve`'s derivation at its `ruled` test)
    #[inline]
    fn ruled_bound(
        &self,
        trainee: UmaIndex,
        q: &[f64],
        career_spark_scale: f64,
        proof_chances: &[f64],
        product_bound: f64,
    ) -> f64 {
        self.space.rho_top() * career_spark_scale * product_bound * self.rise(trainee, q, proof_chances)
    }
    /// Whether a career's ceiling (`gen_cands`) proves it out at `bar` (`Bar`; `solve`: ceiling (1 + 1e-9) < th)
    #[inline]
    pub(crate) fn ceiling_out(ceiling: f64, bar: Bar) -> bool {
        bar.out(ceiling, 1e-9)
    }
    /// Whether a fully checked deck of `refs` proves career (t, q, c) out at `bar` (`solve`: some `ref_bound` (1 + 1e-9) < th)
    #[inline]
    pub(crate) fn refs_out<'refs>(
        &self,
        trainee: UmaIndex,
        q: &[f64],
        career_spark_scale: f64,
        mut refs: impl Iterator<Item = &'refs Ref>,
        bar: Bar,
    ) -> bool {
        refs.any(|reference| bar.out(self.ref_bound(trainee, q, career_spark_scale, reference), 1e-9))
    }
    /// Whether a product proof of `ruled` does (`solve`: some `ruled_bound` (1 + 2e-9) < th; 2e-9: the rounding of the
    /// product value and of rise)
    #[inline]
    pub(crate) fn ruled_out(
        &self,
        trainee: UmaIndex,
        q: &[f64],
        career_spark_scale: f64,
        ruled: &[ProductProof],
        bar: Bar,
    ) -> bool {
        ruled.iter().any(|proof| {
            bar.out(
                self.ruled_bound(trainee, q, career_spark_scale, &proof.inheritance_chances, proof.product_bound),
                2e-9,
            )
        })
    }
    /// Trainee t's legal pairs of `p` holding `fix` and kept by `keep`, with their ceilings (`gen_cands`: `ceiling` at
    /// `lineage`'s chances, to the bit), best first, ties in pair order (the stable sort by ceiling every ranked scan
    /// took). The folded buffer after it: `ceiling` at the last pair's chances, as a loop of `ceiling` over the pairs left
    /// it (every reader of the folded buffer sets it first)
    pub(crate) fn ranked_pairs(
        &self,
        state: &mut SearchState,
        pool: &Pool,
        fixed_parents: &[usize],
        trainee: UmaIndex,
        keep: impl Fn(usize, usize) -> bool,
    ) -> Ranked {
        let pair = |ceiling, _, _, first_parent, second_parent| RankedPair { ceiling, first_parent, second_parent };
        Ranked {
            candidates: self.gen_cands(state, pool, fixed_parents, &[trainee], keep, pair),
            sorted_until: 0,
            next_index: 0,
        }
    }
    /// The ceiling of trainee t at chances q (her inheritance folded in here) with spark scale c: EXACT_RHO x C x the
    /// product value at each target's best possible deck odds (None: no legal deck odds for her).
    pub(crate) fn ceiling(
        &self,
        state: &mut SearchState,
        trainee: UmaIndex,
        q: &[f64],
        career_spark_scale: f64,
    ) -> Option<f64> {
        self.deck_odds_ceilings[trainee].as_ref()?;
        let (rho, fixed) = (self.space.rho_top(), self.model.fixed_factor());
        Some(self.ceiling_at(state, trainee, q, career_spark_scale, rho, fixed))
    }
    /// `ceiling` for a trainee with legal deck odds, EXACT_RHO and the fixed gold factor given (the same values)
    #[inline]
    fn ceiling_at(
        &self,
        state: &mut SearchState,
        trainee: UmaIndex,
        q: &[f64],
        career_spark_scale: f64,
        rho: f64,
        fixed: f64,
    ) -> f64 {
        self.set_trainee(state, trainee, q);
        let (white_hint_chances, gold_hint_chances) = self.deck_odds_ceilings[trainee].as_ref().unwrap();
        let chances = self.deck_chances(state);
        let product_value = (0..self.model.n_outfits(trainee))
            .map(|outfit| chances.outfit_fast(trainee, outfit, 0.0, white_hint_chances, gold_hint_chances))
            .fold(0.0f64, f64::max);
        rho * career_spark_scale * fixed * product_value
    }
    /// `solve`'s candidates: per trainee of `trainees` with legal deck odds (in order; `tp` her position), per legal pair
    /// a < b of `p` (all of `fix` among them, at most `p.max_borrowed` borrowed, neither her character), `mk(ceiling, tp, t, a, b)`,
    /// in that order. The numbers are `lineage` then `ceiling`'s, operation for operation, without the allocations and
    /// with what does not depend on the trainee computed once per pair (the parents' race parts and relation points
    /// between them, the carrier counts and so the spark scale c) or once per trainee and legacy (her relation points
    /// with it, its grandparent slots' miss factors). A slot without a spark of target k multiplies by 1 where
    /// `inherit` skips it (exact), and every folded buffer `ceiling` leaves (`set_trainee` at each candidate's q in
    /// turn) is the same.
    /// `keep`: a filter on the pairs (a, b) beside the rules (`solve`: every pair).
    pub(crate) fn gen_cands<C>(
        &self,
        state: &mut SearchState,
        pool: &Pool,
        fixed_parents: &[usize],
        trainees: &[UmaIndex],
        keep: impl Fn(usize, usize) -> bool,
        mut make_candidate: impl FnMut(f64, usize, UmaIndex, usize, usize) -> C,
    ) -> Vec<C> {
        let data = self.data;
        let legacy_count = pool.veterans.len();
        let target_count = self.target_count;
        // a slot's (miss, mix) factors of target k: `inherit`'s, 1 for the one it leaves alone (its affinity factor fa
        // computed once per slot)
        #[inline(always)]
        fn fac(stars: u8, mix: Option<[f64; 3]>, affinity_factor: f64) -> (f64, f64) {
            if stars == 0 {
                return (1.0, 1.0);
            }
            match mix {
                None => (insp_miss(f64::from(stars), affinity_factor), 1.0),
                Some(star_mix) => (1.0, mix_term(star_mix, affinity_factor)),
            }
        }
        // `spark_scale` per carrier count n
        let max_carriers = (0..legacy_count).map(|legacy| 1 + pool.parents[legacy].len()).max().unwrap_or(0) * 2;
        let spark_scales: Vec<f64> = (0..=max_carriers).map(|n| spark_scale(n as i32)).collect();
        struct PairPreparation {
            first_parent: usize,
            second_parent: usize,
            career_spark_scale: f64,
            race: [f64; 2],
            parent_pair_relations: [f64; 2],
        }
        let mut pairs: Vec<PairPreparation> = vec![];
        // per legacy and target, how many of her slots (her own, her parents') carry the spark
        let slot_carriers: Vec<usize> = (0..legacy_count)
            .flat_map(|legacy| {
                (0..target_count).map(move |k| {
                    std::iter::once(&pool.veterans[legacy])
                        .chain(&pool.parents[legacy])
                        .filter(|who| who.stars[k] != 0)
                        .count()
                })
            })
            .collect();
        let mut carriers = vec![0usize; target_count];
        // the pairs a < b in (a, b) order: with a fixed legacy f (`fixed_legacy`), only those holding it,
        // (a, f) for a < f then (f, b)
        let pair_list: Box<dyn Iterator<Item = (usize, usize)>> = match fixed_parents.first() {
            None => Box::new((0..legacy_count).flat_map(move |first_parent| {
                (first_parent + 1..legacy_count).map(move |second_parent| (first_parent, second_parent))
            })),
            Some(&fixed_legacy) if fixed_legacy < legacy_count => Box::new(
                (0..fixed_legacy)
                    .map(move |first_parent| (first_parent, fixed_legacy))
                    .chain((fixed_legacy + 1..legacy_count).map(move |second_parent| (fixed_legacy, second_parent))),
            ),
            Some(_) => Box::new(std::iter::empty()),
        };
        for (first_parent, second_parent) in pair_list {
            if !pool.parents_ok(first_parent, second_parent)
                || !holds_fix(fixed_parents, first_parent, second_parent)
                || !keep(first_parent, second_parent)
            {
                continue;
            }
            let shared_race_affinity = 3.0 * shared(&pool.veterans[first_parent], &pool.veterans[second_parent]);
            let race = [first_parent, second_parent]
                .map(|legacy| part_sum(shared_race_affinity, pool.grandparent_race_affinity[legacy].iter().copied()));
            let parent_pair_relations = [
                self.pair(pool.veterans[first_parent].uma_index, pool.veterans[second_parent].uma_index),
                self.pair(pool.veterans[second_parent].uma_index, pool.veterans[first_parent].uma_index),
            ];
            for k in 0..target_count {
                carriers[k] =
                    slot_carriers[first_parent * target_count + k] + slot_carriers[second_parent * target_count + k];
            }
            let career_spark_scale = self
                .uncertain_targets
                .iter()
                .map(|&i| spark_scales[carriers[i]])
                .chain(self.fixed_targets.iter().map(|&i| spark_scales[carriers[i]]))
                .product::<f64>();
            pairs.push(PairPreparation {
                first_parent,
                second_parent,
                career_spark_scale,
                race,
                parent_pair_relations,
            });
        }
        let mut candidates: Vec<C> = Vec::with_capacity(
            pairs.len() * trainees.iter().filter(|&&trainee| self.deck_odds_ceilings[trainee].is_some()).count(),
        );
        let (rho, fixed) = (self.space.rho_top(), self.model.fixed_factor());
        let mut q = vec![0.0f64; target_count];
        // per legacy x: her relation points with the trainee, and per grandparent slot its relation points and its
        // factors per target (flat, from `grandparent_offsets[x]`)
        let mut parent_relations = vec![0.0f64; legacy_count];
        let grandparent_offsets: Vec<usize> = std::iter::once(0)
            .chain((0..legacy_count).scan(0, |total, legacy| {
                *total += pool.parents[legacy].len();
                Some(*total)
            }))
            .collect();
        let mut grandparent_relations = vec![0.0f64; grandparent_offsets[legacy_count]];
        let mut grandparent_factors = vec![(1.0f64, 1.0f64); grandparent_offsets[legacy_count] * target_count];
        // per outfit and uncertain target, `outfit_fast`'s factor as Fixed (no chance in it) or Open(1 - e, 1 - dw, dg GOLD,
        // 1 - dg): the same subexpressions, each computed once for her
        enum OutfitFactor {
            Fixed(f64),
            Open(f64, f64, f64, f64),
        }
        // what one trainee's pair loop reads
        struct CandidateContext<'context> {
            pool: &'context Pool,
            pairs: &'context [PairPreparation],
            trainee_id: u32,
            target_count: usize,
            parent_relations: &'context [f64],
            grandparent_offsets: &'context [usize],
            grandparent_relations: &'context [f64],
            grandparent_factors: &'context [(f64, f64)],
            outfit_factors: &'context [Vec<OutfitFactor>],
            uncertain_targets: &'context [usize],
            white_spark_rate: f64,
            rho: f64,
            fixed: f64,
            trainee_position: usize,
            trainee: UmaIndex,
        }
        // One trainee's candidates, pair by pair; true when it made one (q then holds the last one's chances). Its own
        // function, entered once per trainee: a wasm call keeps the code it started in (V8 has no on-stack replacement
        // for wasm), so a loop over every trainee in one frame would run in baseline code throughout (docs/PERF.md).
        #[inline(never)]
        fn trainee_cands<C>(
            context: &CandidateContext,
            q: &mut [f64],
            uncertain_misses: &mut [f64],
            candidates: &mut Vec<C>,
            make_candidate: &mut impl FnMut(f64, usize, UmaIndex, usize, usize) -> C,
        ) -> bool {
            let (pool, grandparent_offsets, white_spark_rate, target_count) =
                (context.pool, context.grandparent_offsets, context.white_spark_rate, context.target_count);
            let mut last = false;
            for pair in context.pairs {
                let (first_parent, second_parent) = (pair.first_parent, pair.second_parent);
                if !pool.trainee_ok(context.trainee_id, first_parent, second_parent) {
                    continue;
                }
                // each parent slot's affinity factor, summed as `slots` sums it
                let affinity_factors = [0, 1].map(|side| {
                    let parent = if side == 0 { first_parent } else { second_parent };
                    aff_factor(
                        part_sum(
                            context.parent_relations[parent] + pair.parent_pair_relations[side],
                            context.grandparent_relations[grandparent_offsets[parent]..grandparent_offsets[parent + 1]]
                                .iter()
                                .copied(),
                        ),
                        pair.race[side],
                    )
                });
                for k in 0..target_count {
                    let (mut miss, mut mix_product) = (1.0f64, 1.0f64);
                    for (side, parent) in [first_parent, second_parent].into_iter().enumerate() {
                        let (slot_miss, slot_mix) =
                            fac(pool.veterans[parent].stars[k], pool.veterans[parent].star_mix, affinity_factors[side]);
                        miss *= slot_miss;
                        mix_product *= slot_mix;
                        for j in grandparent_offsets[parent]..grandparent_offsets[parent + 1] {
                            let (slot_miss, slot_mix) = context.grandparent_factors[j * target_count + k];
                            miss *= slot_miss;
                            mix_product *= slot_mix;
                        }
                    }
                    q[k] = inherit_q(miss, mix_product);
                }
                // `ceiling_at` without writing the folded buffer: per outfit `outfit_fast` at q = 0 on the hint chances
                // `set_trainee` would write (each factor (1 - q) = 1 dropped: x 1 is exact), the best outfit as its
                // fold (an open factor: `event_chance` = 1 - (1 - e)(1 - q_k),
                // white = GEN_WHITE (1 - (1 - dw)(1 - `event_chance`)), dg GOLD + (1 - dg) white)
                for k in 0..uncertain_misses.len() {
                    uncertain_misses[k] = 1.0 - q[context.uncertain_targets[k]];
                }
                let mut product_value = 0.0f64;
                for outfit in context.outfit_factors {
                    let mut outfit_value = 1.0f64;
                    for (k, factor) in outfit.iter().enumerate() {
                        outfit_value *= match *factor {
                            OutfitFactor::Fixed(fixed_factor) => fixed_factor,
                            OutfitFactor::Open(event_miss, white_miss, gold_factor, gold_miss) => {
                                let event_chance = 1.0 - event_miss * uncertain_misses[k];
                                gold_factor + gold_miss * (white_spark_rate * (1.0 - white_miss * (1.0 - event_chance)))
                            }
                        };
                    }
                    product_value = f64::max(product_value, outfit_value);
                }
                last = true;
                candidates.push(make_candidate(
                    context.rho * pair.career_spark_scale * context.fixed * product_value,
                    context.trainee_position,
                    context.trainee,
                    first_parent,
                    second_parent,
                ));
            }
            last
        }
        for (trainee_position, &trainee) in trainees.iter().enumerate() {
            if self.deck_odds_ceilings[trainee].is_none() {
                continue;
            }
            let (trainee_index, trainee_id) = (Some(trainee), data.umas[trainee].id);
            for legacy in 0..legacy_count {
                let veteran = &pool.veterans[legacy];
                parent_relations[legacy] = self.pair(trainee_index, veteran.uma_index);
                for (grandparent_slot, grandparent) in pool.parents[legacy].iter().enumerate() {
                    let j = grandparent_offsets[legacy] + grandparent_slot;
                    grandparent_relations[j] = self.tri(trainee_index, veteran.uma_index, grandparent.uma_index);
                    let affinity_factor =
                        aff_factor(grandparent_relations[j], pool.grandparent_race_affinity[legacy][grandparent_slot]);
                    for k in 0..target_count {
                        grandparent_factors[j * target_count + k] =
                            fac(grandparent.stars[k], grandparent.star_mix, affinity_factor);
                    }
                }
            }
            let (white_hint_chances, gold_hint_chances) = self.deck_odds_ceilings[trainee].as_ref().unwrap();
            debug_assert!(
                self.outfit_coverage[trainee].len() == self.model.n_outfits(trainee),
                "one skill table per outfit of trainee {trainee}"
            );
            let (gold_rate, white_spark_rate) = (crate::skills::GEN_GOLD, crate::skills::GEN_WHITE);
            let outfit_factors: Vec<Vec<OutfitFactor>> = self.outfit_coverage[trainee]
                .iter()
                .zip(&self.model.event_chances[trainee])
                .map(|(coverage, own_event_chances)| {
                    (0..coverage.len())
                        .map(|k| match coverage[k] {
                            2 => OutfitFactor::Fixed(gold_rate),
                            1 => OutfitFactor::Fixed(
                                gold_hint_chances[k] * gold_rate + (1.0 - gold_hint_chances[k]) * white_spark_rate,
                            ),
                            _ => OutfitFactor::Open(
                                1.0 - own_event_chances[k],
                                1.0 - white_hint_chances[k],
                                gold_hint_chances[k] * gold_rate,
                                1.0 - gold_hint_chances[k],
                            ),
                        })
                        .collect()
                })
                .collect();
            let mut uncertain_misses = vec![0.0f64; outfit_factors.iter().map(std::vec::Vec::len).max().unwrap_or(0)];
            let context = CandidateContext {
                pool,
                pairs: &pairs,
                trainee_id,
                target_count,
                parent_relations: &parent_relations,
                grandparent_offsets: &grandparent_offsets,
                grandparent_relations: &grandparent_relations,
                grandparent_factors: &grandparent_factors,
                outfit_factors: &outfit_factors,
                uncertain_targets: &self.uncertain_targets,
                white_spark_rate,
                rho,
                fixed,
                trainee_position,
                trainee,
            };
            let last = trainee_cands(&context, &mut q, &mut uncertain_misses, &mut candidates, &mut make_candidate);
            // the folded buffer the last candidate's `ceiling` left (q still holds its chances)
            if last {
                self.set_trainee(state, trainee, &q);
            }
        }
        candidates
    }
}

impl Env<'_> {
    /// The best careers: trainees from `trainees` (uma indices), two legacies of `p` as parents (all of `fix` among
    /// them, at most one borrowed), each with her best deck; `top` of them, one per trainee when `per_trainee`.
    /// Candidates are kept as (ceiling, trainee, parents) only (a pool of 400 veterans has millions); their chances
    /// are recomputed when one is solved (the same numbers, so the same results). The search is `complete` unless a
    /// deck search was not `full`, a cap breach went unanswered, or the hard work cap stopped candidate evaluation.
    pub(crate) fn solve(&mut self, request: SearchRequest) -> CareerSearchResult {
        let SearchRequest { pool, fixed_parents, trainees, per_trainee, top } = request;
        let data = self.setup.data;
        self.state.work = 0; // each search has its own work budget
        let previous_certificate = std::mem::replace(&mut self.audit.certified_cap, f64::INFINITY);
        // (`tp`: the trainee's position in `trainees`; generated in (tp, a, b) order)
        // (packed as u16, not a `Lineage`: a pool of 400 veterans has millions of candidates)
        struct Candidate {
            ceiling: f64,
            trainee: u16,
            trainee_position: u16,
            first_parent: u16,
            second_parent: u16,
        }
        impl Candidate {
            const fn lineage(&self) -> Lineage {
                Lineage {
                    trainee: UmaIndex(self.trainee as usize),
                    first_parent: self.first_parent as usize,
                    second_parent: self.second_parent as usize,
                }
            }
        }
        let generation_start = now_ms();
        let legacy_count = pool.veterans.len();
        assert!(
            legacy_count <= u16::MAX as usize
                && data.umas.len() <= u16::MAX as usize
                && trainees.len() <= u16::MAX as usize,
            "legacies, umas and trainees are packed as u16"
        );
        let mut candidates: Vec<Candidate> = self.setup.gen_cands(
            &mut self.state,
            pool,
            fixed_parents,
            trainees,
            |_, _| true,
            |ceiling, trainee_position, trainee, first_parent, second_parent| Candidate {
                ceiling,
                trainee: trainee.0 as u16,
                trainee_position: trainee_position as u16,
                first_parent: first_parent as u16,
                second_parent: second_parent as u16,
            },
        );
        // Best ceiling first, ties in generation order: the order a stable sort by ceiling gives (partial_cmp, so 0 and
        // -0 tie as there), as a total order on (ceiling descending, tp, a, b), unique per candidate. Sorted lazily
        // (`upto`): the loop below reads a short prefix, so each step selects the next chunk (everything before it is
        // ordered before everything after it) and sorts only the chunk; the prefix is the full sort's, element for element.
        let candidate_order = |first: &Candidate, second: &Candidate| {
            second
                .ceiling
                .partial_cmp(&first.ceiling)
                .unwrap()
                .then(first.trainee_position.cmp(&second.trainee_position))
                .then(first.first_parent.cmp(&second.first_parent))
                .then(first.second_parent.cmp(&second.second_parent))
        };
        let mut sorted_until = 0usize;
        // cands[..upto] in final order; makes it reach past i
        let sort_to = |candidates: &mut [Candidate], i: usize, sorted_until: &mut usize| {
            if i < *sorted_until || *sorted_until >= candidates.len() {
                return;
            }
            let sort_start = now_ms();
            while i >= *sorted_until && *sorted_until < candidates.len() {
                let rest = &mut candidates[*sorted_until..];
                // (chunks of 1024, then 4x what is sorted: each selection scans the whole remainder)
                let chunk = (4 * *sorted_until).max(1024).min(rest.len());
                if chunk < rest.len() {
                    rest.select_nth_unstable_by(chunk - 1, candidate_order);
                }
                rest[..chunk].sort_unstable_by(candidate_order);
                *sorted_until += chunk;
            }
            prof("gen_ms", now_ms() - sort_start);
        };
        prof("gen_ms", now_ms() - generation_start);
        prof("gen_cands", candidates.len() as f64);
        prof("solves", 1.0);
        let top = top.max(1);
        let mut complete = true;
        // a breach no cap-free search answered (`CAP_RERUN=0`): the search may have pruned a better deck. A rerun that
        // proves nothing returns a deck search that is not full, so `complete` falls below as for any such search
        let unhandled_before = self.audit.cap_free_stats.unhandled;
        let mut done: Vec<(f64, usize, SelectedDeck)> = vec![];
        let floor = self.state.floor;
        let kth =
            |done: &[(f64, usize, SelectedDeck)]| if done.len() < top { floor } else { done[top - 1].0.max(floor) };
        let (mut n_solved, mut n_bounded, mut n_scored) = (0usize, 0usize, 0usize);
        let mut next = 0;
        // one result per trainee: a candidate only matters if it beats the k-th best and its trainee's best so far
        let threshold_for = |done: &[(f64, usize, SelectedDeck)], candidates: &[Candidate], trainee: UmaIndex| {
            if per_trainee {
                done.iter()
                    .find(|result| UmaIndex(candidates[result.1].trainee as usize) == trainee)
                    .map_or_else(|| kth(done), |result| result.0.max(kth(done)))
            } else {
                kth(done)
            }
        };
        // per trainee, her fully checked candidates (`Ref`: chances, exact best value before the spark scale, deck)
        let mut refs = UmaTable(vec![vec![]; data.umas.len()]);
        // per trainee, where this search's own entries in `ruled` start
        let prior_proof_counts: UmaTable<usize> = self.state.ruled.iter().map(std::vec::Vec::len).collect();
        // dominated veterans (`dominance_lists`), and the candidates this search took and did not solve (bounded, no
        // deck)
        let dominance = pool.dominance_lists();
        let prune = dominance.iter().any(|dominators| !dominators.is_empty());
        let mut unsolved: crate::FxSet<(u16, u16, u16)> = crate::FxSet::default();
        // the deck problems searched so far (`Twin`)
        let mut twins: crate::FxMap<TwinKey, Twin> = crate::FxMap::default();
        // a candidate whose ceiling is 0 cannot land every target (a target no legacy carries and nothing else gives):
        // nothing after it can either (sorted), and with every value 0 the bar would never rise, so every career would
        // be solved without a positive pruning bar; stop there, leaving such careers out
        while next < candidates.len()
            && {
                sort_to(&mut candidates, next, &mut sorted_until);
                candidates[next].ceiling > 0.0
            }
            && candidates[next].ceiling * (1.0 + 1e-9) >= kth(&done)
        {
            if self.state.hard_work_cap > 0 && self.state.work > self.state.hard_work_cap {
                complete = false;
                break;
            }
            let candidate_index = next;
            next += 1;
            let lineage = candidates[candidate_index].lineage();
            let Lineage { trainee, first_parent, second_parent } = lineage;
            let threshold = threshold_for(&done, &candidates, trainee);
            if prune {
                unsolved.insert((
                    candidates[candidate_index].trainee,
                    candidates[candidate_index].first_parent,
                    candidates[candidate_index].second_parent,
                ));
            }
            if CareerSetup::ceiling_out(candidates[candidate_index].ceiling, Bar::Below(threshold)) {
                n_bounded += 1;
                continue;
            }
            // A candidate whose parent has a dominating veteran x, where (t, x, other parent) was taken earlier and not
            // solved, would end unsolved too: that one was ruled out by its ceiling, a Ref's or a product search's bound,
            // a search that found no deck clearing the bar, or (in turn) this test, at a bar no higher than now (the bar
            // only rises), and each of those bounds is at most as high for this candidate (`dominates`: q, C and every
            // deck's value no higher), so it fails the same test (no deck at all: none here either). So no Ref, no
            // result, the same counters bar `work`; exact for any `top` and either `per_trainee`, and it uses nothing but
            // this search's own candidates. A taken-and-solved dominator proves nothing of the kind (this one could still
            // come out solved, a Ref the later searches start from), so only unsolved ones count.
            if prune {
                let key = |parent: usize, partner: usize| {
                    (trainee.0 as u16, parent.min(partner) as u16, parent.max(partner) as u16)
                };
                if [(first_parent, second_parent), (second_parent, first_parent)].iter().any(|&(parent, partner)| {
                    dominance[parent]
                        .iter()
                        .any(|&dominator| dominator != partner && unsolved.contains(&key(dominator, partner)))
                }) {
                    n_bounded += 1;
                    continue;
                }
            }
            let (q, career_spark_scale) = self.setup.lineage_q(pool, lineage);
            // (the ceiling this candidate was ranked and stopped by is `ceiling` at these chances, to the bit:
            // `gen_cands`' claim; only the trainee's reusable folded buffer changes)
            debug_assert!(
                {
                    let ceiling = self.setup.ceiling(&mut self.state, trainee, &q, career_spark_scale);
                    ceiling.map(f64::to_bits) == Some(candidates[candidate_index].ceiling.to_bits())
                },
                "gen_cands' ceiling of ({trainee}, {first_parent}, {second_parent}) is not ceiling(lineage)"
            );
            let deck_class = self.setup.deck_class[trainee];
            let seen: &[Ref] = if self.state.use_seen { &self.state.seen[deck_class] } else { &[] };
            // (the lowest of these bounds below th, as any one of them below it: the minimum is one of the bounds, and
            // rounding x (1 + 1e-9) is monotone; a NaN bound neither lowers the minimum nor passes the test)
            if self.setup.refs_out(
                trainee,
                &q,
                career_spark_scale,
                refs[trainee].iter().chain(seen),
                Bar::Below(threshold),
            ) {
                n_bounded += 1;
                continue;
            }
            // what her earlier product searches proved (`ruled`). Derived: with her chances folded in, target k's factor
            // in the product value of one outfit and one deck (odds dw, dg) is f(n) = A - B n, n = (1 - e)(1 - q_k),
            // A = dg G + (1 - dg) W, B = (1 - dg) W (1 - dw) <= A (`outfit_fast` at q = 0; a target the outfit has is
            // q-free). From q to q' with q'_k > q_k: n' < n, and A >= B gives f(n') / f(n) <= (1 - n') / (1 - n), the
            // factor `rise` takes ((A - B n')(1 - n) <= (A - B n)(1 - n') <=> A (n' - n) <= B (n' - n)); with q'_k <= q_k
            // the factor does not grow. So per outfit the product at q' is at most the product at q times that outfit's
            // ratio product, the best outfit at q' at most S x rise(t, q', q) for every deck, and the fixed gold factor
            // and the spark scale c' multiply both sides alike. Then the search below would end `Bounded` on either path:
            // no deck above its seed th / (EXACT_RHO c' (1 + 1e-9)), or its best deck pv' with EXACT_RHO c' pv' (1 + 1e-9)
            // < th. The slack 2e-9 covers the float rounding of pv' and rise (about 1e-15 each). A skipped candidate is
            // one that would have been `Bounded`: no Ref, no result, the same counters bar `work` (less of it, so a search
            // that ran out of its budget before can now get further). A search whose
            // decks seed the simulator's cache (`use_seen` off) takes only the proofs of its own candidates, so its work does not
            // depend on what earlier searches did either.
            let proof_start = if self.state.use_seen { 0 } else { prior_proof_counts[deck_class] };
            if self.setup.ruled_out(
                trainee,
                &q,
                career_spark_scale,
                &self.state.ruled[deck_class][proof_start..],
                Bar::Below(threshold),
            ) {
                n_bounded += 1;
                continue;
            }
            self.setup.set_trainee(&mut self.state, trainee, &q);
            let result = self.twin_deck_search(
                &mut twins,
                TwinSearch {
                    trainee,
                    chances: &q,
                    spark_scale: career_spark_scale,
                    bar: threshold,
                    starts: &refs[trainee],
                },
                &mut n_scored,
            );
            let (value, deck, full) = match result {
                DeckRes::NoDeck => continue,
                DeckRes::Bounded => {
                    n_bounded += 1;
                    continue;
                }
                DeckRes::Done(found_value, deck, full) => (found_value, deck, full),
            };
            n_solved += 1;
            if prune {
                unsolved.remove(&(
                    candidates[candidate_index].trainee,
                    candidates[candidate_index].first_parent,
                    candidates[candidate_index].second_parent,
                ));
            }
            if full {
                refs[trainee].push(Ref { inheritance_chances: q, value, deck: deck.clone() });
            } else {
                complete = false;
            }
            let value = career_spark_scale * value;
            if per_trainee
                && let Some(i) =
                    done.iter().position(|result| UmaIndex(candidates[result.1].trainee as usize) == trainee)
            {
                if done[i].0 >= value {
                    continue;
                }
                done.remove(i);
            }
            let pos = done.iter().position(|result| result.0 < value).unwrap_or(done.len());
            done.insert(pos, (value, candidate_index, deck));
        }
        self.setup.merge_seen(&mut self.state, &refs);
        let next_bound = if next < candidates.len() {
            sort_to(&mut candidates, next, &mut sorted_until);
            candidates[next].ceiling
        } else {
            0.0
        };
        if self.audit.cap_free_stats.unhandled > unhandled_before {
            complete = false;
        }
        let proven = complete
            && !done.is_empty()
            && (done.len() >= top.min(candidates.len()) || next >= candidates.len())
            && next_bound < done[0].0;
        let candidate_count = candidates.len();
        let certified_cap = self.audit.certified_cap;
        self.audit.certified_cap = previous_certificate.min(certified_cap);
        CareerSearchResult {
            found: done
                .into_iter()
                .take(top)
                .map(|(value, candidate_index, deck)| Found {
                    trainee: UmaIndex(candidates[candidate_index].trainee as usize),
                    first_parent: candidates[candidate_index].first_parent as usize,
                    second_parent: candidates[candidate_index].second_parent as usize,
                    value,
                    ceiling: candidates[candidate_index].ceiling,
                    deck,
                })
                .collect(),
            refs,
            candidates: candidate_count,
            solved: n_solved,
            bounded: n_bounded,
            scored: n_scored,
            skipped: candidate_count - next,
            next_bound,
            complete,
            proven,
            work: self.state.work,
            certified_cap,
        }
    }
}

impl Env<'_> {
    /// `solve`'s deck search of a candidate at chances `search.chances` (folded in: `set_trainee`), or the outcome of
    /// a bit-identical problem it already searched (`Twin`); records the product search's proof in `ruled`
    fn twin_deck_search(
        &mut self,
        twins: &mut crate::FxMap<TwinKey, Twin>,
        search: TwinSearch,
        n_scored: &mut usize,
    ) -> DeckRes {
        let TwinSearch { trainee, chances: q, spark_scale: career_spark_scale, bar: threshold, starts } = search;
        let deck_class = self.setup.deck_class[trainee];
        let twin_key = (
            trainee.0 as u16,
            q.iter().map(|chance| chance.to_bits()).collect::<Vec<u64>>(),
            career_spark_scale.to_bits(),
        );
        match twins.get(&twin_key) {
            Some(Twin::Done(value, deck, product_bound)) => {
                if let Some(bound) = product_bound {
                    self.state.ruled[deck_class]
                        .push(ProductProof { inheritance_chances: q.to_vec(), product_bound: *bound });
                }
                DeckRes::Done(*value, deck.clone(), true)
            }
            Some(Twin::Bounded(bar)) if threshold >= *bar => DeckRes::Bounded,
            Some(Twin::NoDeck) => DeckRes::NoDeck,
            _ => {
                let starts: Vec<&SelectedDeck> = starts.iter().map(|reference| &reference.deck).collect();
                let result = self.best_deck(
                    crate::career::decks::DeckSearch { trainee, spark_scale: career_spark_scale, bar: threshold },
                    &starts,
                    n_scored,
                );
                let product_bound = self.state.last_product_bound.take();
                if let Some(bound) = product_bound {
                    self.state.ruled[deck_class]
                        .push(ProductProof { inheritance_chances: q.to_vec(), product_bound: bound });
                }
                match &result {
                    DeckRes::Done(value, deck, true) => {
                        twins.insert(twin_key, Twin::Done(*value, deck.clone(), product_bound));
                    }
                    DeckRes::Done(..) => {}
                    DeckRes::Bounded => {
                        twins.insert(twin_key, Twin::Bounded(threshold));
                    }
                    DeckRes::NoDeck => {
                        twins.insert(twin_key, Twin::NoDeck);
                    }
                }
                result
            }
        }
    }
}

impl CareerSetup<'_> {
    /// A search's fully checked decks (`refs`, per trainee) into `seen` (per deck class), each new one once
    fn merge_seen(&self, state: &mut SearchState, refs: &UmaTable<Vec<Ref>>) {
        for (trainee, trainee_refs) in refs.indexed() {
            let deck_class = self.deck_class[trainee];
            for reference in trainee_refs {
                if !state.seen[deck_class].iter().any(|known| {
                    known.value == reference.value && known.inheritance_chances == reference.inheritance_chances
                }) {
                    state.seen[deck_class].push(reference.clone());
                }
            }
        }
    }
}

/// Identical deck problems, searched once (`Env::solve`): a candidate whose trainee, chances q and spark scale
/// c are bit-identical to an earlier candidate's of this search (interchangeable parents: veterans with the same stars
/// and no relation or race points between them, as every sample pool of mob veterans has; 15 pairs of 6 such
/// veterans are one problem per trainee) takes that candidate's outcome instead of a second `best_deck`.
/// Derived, per outcome of the first (F, at bar th1) for the later one (R, at bar th2 >= th1: the bar only rises):
/// - Done(v, D) fully checked: F's Ref (q, v, D) is in refs[t], so R reaches the deck search only with
///   th2 <= c v (1 + 1e-9) (the Ref bound above, rise = 1 at equal q). R's product search then starts below the
///   best product value pv (th2 / (rho c (1 + 1e-9)) <= v / rho <= pv under the cap every search assumes) and finds
///   F's product deck P, so the same `ruled` proof (q, pv); R's first exact pass scores P, then its starts, which
///   hold F's (D among them), so its best is at least v from the same decks in the same order up to D; its band
///   (decks with rho x product >= its best) is within F's, every deck of which F scored or ruled out against a
///   lower best, and the starts F did not have are decks legal at q, at most v (F's search is exact under the cap).
///   So R is Done(v, D) with the same floats (the same folded buffer, `set_trainee` at the same q); a tie or lower,
///   it goes into `done` exactly where R's own search would have put it.
/// - Bounded at th1: no deck above F's seed, or the best product deck below th1 / (rho c (1 + 1e-9)); a higher bar
///   gives a higher seed, so R is Bounded too (no `ruled` entry: R's would only repeat what F's proves).
/// - NoDeck (no legal deck at all, at any bar): R the same.
///
/// R's search would also have spent work, so a work budget can now reach further than before (the answer the
/// unbudgeted search gives); outputs are otherwise the same, only the work counters fall.
enum Twin {
    /// Fully checked: value, deck, the product search's proof.
    Done(f64, SelectedDeck, Option<f64>),
    /// Bounded at this bar.
    Bounded(f64),
    NoDeck,
}

/// A deck problem's identity in `solve`: trainee, chances q and spark scale c, to the bit.
type TwinKey = (u16, Vec<u64>, u64);

/// One candidate's deck problem in `solve` (`CareerSetup::twin_deck_search`).
#[derive(Clone, Copy)]
struct TwinSearch<'search> {
    trainee: UmaIndex,
    /// Her inheritance chances q, already folded into the trainee buffer.
    chances: &'search [f64],
    /// C, the spark scale.
    spark_scale: f64,
    /// th, the value the career must beat.
    bar: f64,
    /// Her fully checked decks so far in this search: the deck search's start decks.
    starts: &'search [Ref],
}

/// What a career search covers (`Env::solve`'s arguments, as `solve_kept` takes them).
#[derive(Clone, Copy)]
pub(crate) struct SearchRequest<'request> {
    /// The legacies the parents come from.
    pub(crate) pool: &'request Pool,
    /// Legacy indices that must be among the two parents (the job's places).
    pub(crate) fixed_parents: &'request [usize],
    /// Trainees to search (uma indices).
    pub(crate) trainees: &'request [UmaIndex],
    /// One result per trainee (false: the best careers whoever trains).
    pub(crate) per_trainee: bool,
    /// How many results to keep.
    pub(crate) top: usize,
}

/// The main search of the job keyed `key` (`KeptState::searches`): the kept one, with what its product searches proved
/// (`SearchState::ruled` as a combined job's plan finds it), else searched now and kept
pub(crate) fn solve_kept(
    env: &mut Env,
    worker: &mut KeptState,
    key: u64,
    request: &SearchRequest,
) -> CareerSearchResult {
    let kept = (|| {
        let cache = &mut worker.searches;
        let index = cache.iter().position(|entry| entry.key == key)?;
        let entry = cache.remove(index);
        cache.push(entry);
        // The retained result and this job's working result must both remain available.
        cache.last().map(|entry| (entry.solved.clone(), entry.ruled.clone()))
    })();
    match kept {
        Some((solved, ruled)) if env.setup.solved_fits(&solved, &ruled, request.pool) => {
            env.state.ruled = ruled;
            solved
        }
        _ => {
            let solved = env.solve(crate::career::search::SearchRequest {
                pool: request.pool,
                fixed_parents: request.fixed_parents,
                trainees: request.trainees,
                per_trainee: request.per_trainee,
                top: request.top,
            });
            {
                let cache = &mut worker.searches;
                cache.retain(|entry| entry.key != key);
                cache.push(KeptSolve { key, solved: solved.clone(), ruled: env.state.ruled.clone() });
                while cache.len() > SOLVE_KEEP {
                    cache.remove(0);
                }
            }
            solved
        }
    }
}
