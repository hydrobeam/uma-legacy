//! Career candidates, bounds and the best careers.
use crate::career::clock::{now_ms, prof};
use crate::career::inherit::{aff_factor, inherit_q, insp_miss, mix_term, part_sum, spark_scale};
use crate::career::pool::{Pool, holds_fix, shared};
use crate::career::state::{KeptSolve, KeptState, ProductProof, SOLVE_KEEP};
use crate::career::{CareerSetup, ClassFlags, Env, Lineage, SearchState};
use crate::deck::space::SelectedDeck;
use crate::uma::{UmaIndex, UmaTable};
/// How a bound proves a career out (`CareerSetup::ceiling_out`, `refs_out`, `ruled_out`): `Below(threshold)`, `solve`'s
/// prune, the bound below the bar with its slack s, b (1 + s) < threshold (a computed bound a hair under the true one
/// still proves it); or `AtMost(limit)`, `certified`'s, the bound at most limit up to rounding, b <= limit (1 + 1e-12):
/// the career can't beat limit, and a tie proves value equality, all a certificate claims. The 1e-12 is rounding only:
/// limit is a proxy's value through its deck's corner table and the bound the same deck's exact value, the same number
/// computed two ways.
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
/// descending, a, b), unique per pair; each step past the ordered prefix `candidates[..sorted_until]` selects the next
/// chunk (everything before it orders before everything after it) and sorts only the chunk, so the sequence is the full
/// sort's, element for element, and a caller that stops early sorts little
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
    /// The pair's content rank (`pool::pair_rank`): ties in ceiling break by it before the legacy indices.
    pub(crate) content_rank: [u32; 2],
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
                    .then(first.content_rank.cmp(&second.content_rank))
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

/// Why a career search is not complete (`CareerSearchResult::complete` false), each reason that occurred.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Unproven {
    /// The work ran out: a deck search passed the work budget (it scored only some of its band), or the hard work cap
    /// stopped taking candidates.
    pub(crate) work_cap: bool,
    /// A deck band of over EXACT_CAP decks was certified at a lower cap only (`DeckSearchAudit::certified_cap`).
    pub(crate) band_overflow: bool,
    /// A capped deck search scored a deck past its class's cap (`DeckSearchAudit::capped_breaches`): the cap is wrong for
    /// that deck space, so what the search pruned through it (other deck searches of the class, its product proofs,
    /// ceilings that take EXACT_RHO) may have hidden a better career, whatever the cap-free rerun of that one deck search
    /// proved; or a rerun proved nothing, or none ran (`CAP_RERUN=0`).
    pub(crate) cap_breach: bool,
}

/// The named assumptions a proof rests on beyond the derived bounds and the measured class caps (docs/SOLVER.md, *What
/// "proven" rests on*), each that occurred.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug)]
pub(crate) struct Assumes {
    /// The class cap for decks of a pair of chain-varying targets fed by two or more chain cards
    /// (`DeckSearchAudit::assumed_searches`): two cards each feeding one target, a nested pair with another feeder.
    pub(crate) two_feeder: bool,
    /// The nested-chain bound for a nested chain beside other chain feeders or an uncertified group, their dependence
    /// left out (`DeckSearchAudit::multi_searches`, `caps::multi_feeder`).
    pub(crate) multi_feeder: bool,
}

impl Assumes {
    /// Both, each set where either has it.
    pub(crate) const fn union(self, other: Self) -> Self {
        Self { two_feeder: self.two_feeder || other.two_feeder, multi_feeder: self.multi_feeder || other.multi_feeder }
    }
    /// The assumptions' names, as the output lists them (`assumes`, `job_assumes`).
    pub(crate) fn names(self) -> Vec<&'static str> {
        [(self.two_feeder, "two_feeder_cap"), (self.multi_feeder, "multi_feeder_cap")]
            .into_iter()
            .filter_map(|(holds, name)| holds.then_some(name))
            .collect()
    }
    /// The assumptions from their names (`names`; absent: none); None for an unknown name.
    pub(crate) fn from_names(value: Option<&serde_json::Value>) -> Option<Self> {
        let mut assumes = Self::default();
        for name in value.map_or(Some(&vec![]), serde_json::Value::as_array)? {
            match name.as_str()? {
                "two_feeder_cap" => assumes.two_feeder = true,
                "multi_feeder_cap" => assumes.multi_feeder = true,
                _ => return None,
            }
        }
        Some(assumes)
    }
}

impl Unproven {
    /// The reasons from their names (`names`); None for an unknown name.
    pub(crate) fn from_names(value: &serde_json::Value) -> Option<Self> {
        let mut unproven = Self::default();
        for name in value.as_array()? {
            match name.as_str()? {
                "work_cap" => unproven.work_cap = true,
                "band_overflow" => unproven.band_overflow = true,
                "cap_breach" => unproven.cap_breach = true,
                _ => return None,
            }
        }
        Some(unproven)
    }
    /// Whether any reason occurred.
    pub(crate) const fn any(self) -> bool {
        self.work_cap || self.band_overflow || self.cap_breach
    }
    /// The reasons' names, as the output lists them (`unproven`).
    pub(crate) fn names(self) -> Vec<&'static str> {
        [(self.work_cap, "work_cap"), (self.band_overflow, "band_overflow"), (self.cap_breach, "cap_breach")]
            .into_iter()
            .filter_map(|(occurred, name)| occurred.then_some(name))
            .collect()
    }
}

#[derive(Clone)]
/// (`certified_cap`: the lowest cap a deck search of it was certified under, INFINITY when none was lowered; see
/// `DeckSearchAudit::certified_cap`; `complete`: no reason in `unproven`)
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
    pub(crate) unproven: Unproven,
    /// The named assumptions its deck searches pruned under (`Assumes`; the output's `assumes`): `proven` then holds
    /// under them.
    pub(crate) assumes: Assumes,
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
    /// `trainee`'s legal pairs of `pool` holding `fixed_parents` and kept by `keep`, with their ceilings (`gen_cands`:
    /// `ceiling` at `lineage`'s chances, to the bit), best first, ties by the pair's content rank
    /// (`pool::pair_rank`), then in pair order. The folded buffer after it: `ceiling` at the last pair's chances, as a loop of `ceiling` over
    /// the pairs left it (every reader of the folded buffer sets it first)
    pub(crate) fn ranked_pairs(
        &self,
        state: &mut SearchState,
        pool: &Pool,
        fixed_parents: &[usize],
        trainee: UmaIndex,
        keep: impl Fn(usize, usize) -> bool,
    ) -> Ranked {
        let ranks = pool.content_ranks();
        let pair = |ceiling, _, _, first_parent, second_parent| RankedPair {
            ceiling,
            first_parent,
            second_parent,
            content_rank: crate::career::pool::pair_rank(&ranks, first_parent, second_parent),
        };
        Ranked {
            candidates: self.gen_cands(state, pool, fixed_parents, &[trainee], keep, pair),
            sorted_until: 0,
            next_index: 0,
        }
    }
    /// The ceiling of trainee t at chances q (her inheritance folded in here) with spark scale c: her `ceiling_rho` x C x
    /// the product value at each target's best possible deck odds (None: no legal deck odds for her).
    pub(crate) fn ceiling(
        &self,
        state: &mut SearchState,
        trainee: UmaIndex,
        q: &[f64],
        career_spark_scale: f64,
    ) -> Option<f64> {
        self.deck_odds_ceilings[trainee].as_ref()?;
        let (rho, fixed) = (self.ceiling_rho[trainee], self.model.fixed_factor());
        Some(self.ceiling_at(state, trainee, q, career_spark_scale, rho, fixed))
    }
    /// `ceiling` with EXACT_RHO for every trainee (a bound too, at most EXACT_RHO times looser): for rankings across
    /// trainees by the ideal product value, which the per-trainee factor of `ceiling` would reorder
    pub(crate) fn ceiling_uniform(
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
    /// `ceiling` for a trainee with legal deck odds, her factor and the fixed gold factor given (the same values)
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
    /// `solve`'s candidates: per trainee of `trainees` with legal deck odds (in order; `trainee_position` her
    /// position), per legal pair a < b of `pool` (all of `fixed_parents` among them, at most `pool.max_borrowed`
    /// borrowed, neither her character), `make_candidate(ceiling, trainee_position, trainee, a, b)`, in that order. The
    /// numbers are `lineage` then `ceiling`'s, operation for operation, without the allocations and with what does not
    /// depend on the trainee computed once per pair (the parents' race parts and relation points between them, the
    /// carrier counts and so the spark scale c) or once per trainee and legacy (her relation points with it, its
    /// grandparent slots' miss factors). A slot without a spark of target k multiplies by 1 where `inherit` skips it
    /// (exact), and every folded buffer `ceiling` leaves (`set_trainee` at each candidate's q in turn) is the same.
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
            let race = [first_parent, second_parent].map(|legacy| {
                // (her grandparent slots in the pool's product order, `Pool::grandparent_order`, as `walk_slots`)
                let races = pool.grandparent_order(legacy).map(|slot| pool.grandparent_race_affinity[legacy][slot]);
                part_sum(shared_race_affinity, races)
            });
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
        let fixed = self.model.fixed_factor();
        let mut q = vec![0.0f64; target_count];
        // per legacy: her relation points with the trainee, and per grandparent slot its relation points and its
        // factors per target (flat, from `grandparent_offsets[legacy]`)
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
                            pool.grandparent_order(parent)
                                .map(|slot| context.grandparent_relations[grandparent_offsets[parent] + slot]),
                        ),
                        pair.race[side],
                    )
                });
                // (the parents in the pool's product order, `Pool::product_order`, as `walk_slots` takes them)
                let sides =
                    if pool.product_order(first_parent, second_parent).0 == first_parent { [0, 1] } else { [1, 0] };
                for k in 0..target_count {
                    let (mut miss, mut random_star_misses) = (1.0f64, 1.0f64);
                    for side in sides {
                        let parent = if side == 0 { first_parent } else { second_parent };
                        let (slot_miss, slot_mix) =
                            fac(pool.veterans[parent].stars[k], pool.veterans[parent].star_mix, affinity_factors[side]);
                        miss *= slot_miss;
                        random_star_misses *= slot_mix;
                        for slot in pool.grandparent_order(parent) {
                            let j = grandparent_offsets[parent] + slot;
                            let (slot_miss, slot_mix) = context.grandparent_factors[j * target_count + k];
                            miss *= slot_miss;
                            random_star_misses *= slot_mix;
                        }
                    }
                    q[k] = inherit_q(miss, random_star_misses);
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
                rho: self.ceiling_rho[trainee],
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
    /// The best careers: trainees from `trainees` (uma indices), two legacies of `pool` as parents (all of
    /// `fixed_parents` among them, at most one borrowed), each with her best deck; `top` of them, one per trainee when
    /// `per_trainee`. Candidates are kept as (ceiling, trainee, parents) only (a pool of 400 veterans has millions);
    /// their chances are recomputed when one is solved (the same numbers, so the same results). The search is
    /// `complete` unless a deck search was not `full` (the work budget, a band overflow, a cap-free rerun that proved
    /// nothing), a capped deck search breached its cap (answered or not), or the hard work cap stopped candidate
    /// evaluation (`Unproven`).
    pub(crate) fn solve(&mut self, request: SearchRequest) -> CareerSearchResult {
        let SearchRequest { pool, fixed_parents, trainees, per_trainee, top } = request;
        let data = self.setup.data;
        self.state.work = 0; // each search has its own work budget
        let previous_certificate = std::mem::replace(&mut self.audit.certified_cap, f64::INFINITY);
        // (`trainee_position`: the trainee's position in `trainees`; generated in (trainee_position, a, b) order)
        // (packed as u16, not a `Lineage`: a pool of 400 veterans has millions of candidates)
        struct Candidate {
            ceiling: f64,
            trainee: u16,
            trainee_position: u16,
            first_parent: u16,
            second_parent: u16,
            /// The pair's content rank (`pool::pair_rank`): ties in ceiling break by it before the legacy indices.
            content_rank: [u32; 2],
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
        let ranks = pool.content_ranks();
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
                content_rank: crate::career::pool::pair_rank(&ranks, first_parent, second_parent),
            },
        );
        // Best ceiling first, ties by the trainee's place, then the pair's content rank (`pool::pair_rank`, so the pool's
        // order chooses nothing among equal ceilings), then generation order (partial_cmp, so 0 and -0 tie as there):
        // a total order on (ceiling descending, trainee_position, pair key, a, b), unique per candidate.
        // Sorted lazily (`sorted_until`): the loop below reads a short prefix, so each step selects the next chunk
        // (everything before it is ordered before everything after it) and sorts only the chunk; the prefix is the full
        // sort's, element for element.
        let candidate_order = |first: &Candidate, second: &Candidate| {
            second
                .ceiling
                .partial_cmp(&first.ceiling)
                .unwrap()
                .then(first.trainee_position.cmp(&second.trainee_position))
                .then(first.content_rank.cmp(&second.content_rank))
                .then(first.first_parent.cmp(&second.first_parent))
                .then(first.second_parent.cmp(&second.second_parent))
        };
        let mut sorted_until = 0usize;
        // candidates[..sorted_until] in final order; makes it reach past i
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
        let mut unproven = Unproven::default();
        // a capped deck search that breached its cap, answered by a cap-free rerun or not (`CAP_RERUN=0`): see
        // `Unproven::cap_breach`. A rerun that proves nothing also returns a deck search that is not full (below)
        let (capped_breaches_before, assumed_before, multi_before) =
            (self.audit.capped_breaches, self.audit.assumed_searches, self.audit.multi_searches);
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
        // per trainee, her fully checked candidates (`Ref`: chances, exact best value before the spark scale, deck);
        // per trainee, the distinct decks among them, first occurrence first (`refs[t]` indices: her deck searches'
        // start decks, a deck scored once however many of her Refs hold it, since a repeat never beats the best it is
        // compared with); per deck class, every classmate's Refs ((trainee, `refs[t]` index): a Ref bounds the whole
        // class, `CareerSetup::deck_class`)
        let mut refs: UmaTable<Vec<Ref>> = UmaTable(vec![vec![]; data.umas.len()]);
        let mut start_decks: UmaTable<Vec<usize>> = UmaTable(vec![vec![]; data.umas.len()]);
        let mut class_refs: UmaTable<Vec<(UmaIndex, usize)>> = UmaTable(vec![vec![]; data.umas.len()]);
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
                unproven.work_cap = true;
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
            // (a class whose cap breached: its fully checked decks and product proofs bound every deck only under the
            // refuted cap, so none prunes; `Env::best_deck`)
            let breached = self.state.class_flags[deck_class].breached;
            let class_checked = class_refs[deck_class].iter().map(|&(classmate, i)| &refs[classmate][i]);
            if !breached
                && self.setup.refs_out(
                    trainee,
                    &q,
                    career_spark_scale,
                    class_checked.chain(seen),
                    Bar::Below(threshold),
                )
            {
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
            // and the spark scale c' multiply both sides alike. S is the covered product value (`SearchMode::BestCovered`:
            // max(P, B / EXACT_RHO), B a chain-shape bound, `deck::nested`): B is a maximum over outfits of an expectation
            // over chain states of products of such factors (A - B n per target and state), so termwise each rises by at
            // most rise's factor, and so do the expectation and the maximum. Then the search below would end `Bounded` on either path:
            // no deck above its seed th / (EXACT_RHO c' (1 + 1e-9)), or its best deck pv' with EXACT_RHO c' pv' (1 + 1e-9)
            // < th. The slack 2e-9 covers the float rounding of pv' and rise (about 1e-15 each). A skipped candidate is
            // one that would have been `Bounded`: no Ref, no result, the same counters bar `work` (less of it, so a search
            // that ran out of its budget before can now get further). A search whose
            // decks seed the simulator's cache (`use_seen` off) takes only the proofs of its own candidates, so its work does not
            // depend on what earlier searches did either.
            // (a breach in the class drops its proofs, `ClassFlags::breached`: then there are none past the start)
            let proof_start = if self.state.use_seen { 0 } else { prior_proof_counts[deck_class] };
            if !breached
                && self.setup.ruled_out(
                    trainee,
                    &q,
                    career_spark_scale,
                    self.state.ruled[deck_class].get(proof_start..).unwrap_or_default(),
                    Bar::Below(threshold),
                )
            {
                n_bounded += 1;
                continue;
            }
            self.setup.set_trainee(&mut self.state, trainee, &q);
            let starts: Vec<&SelectedDeck> = start_decks[trainee].iter().map(|&i| &refs[trainee][i].deck).collect();
            let (overflows_before, open_before) = (self.audit.overflows, self.audit.cap_free_stats.open);
            let TwinOutcome { result, repeated } = self.twin_deck_search(
                &mut twins,
                TwinSearch { trainee, chances: &q, spark_scale: career_spark_scale, bar: threshold, starts: &starts },
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
            if !full {
                // (why: a band overflow, a cap-free rerun that proved nothing, else the work budget)
                if self.audit.overflows > overflows_before {
                    unproven.band_overflow = true;
                } else if self.audit.cap_free_stats.open > open_before && !self.setup.space.deck_job.cap_free {
                    unproven.cap_breach = true;
                } else {
                    unproven.work_cap = true;
                }
            } else if !repeated && !self.state.class_flags[deck_class].breached {
                // (a Twin repeat's Ref is the first candidate's, already here: the same chances, value and deck; a breached
                // class keeps none, its searches proving nothing beyond their own answers)
                let index = refs[trainee].len();
                if !start_decks[trainee].iter().any(|&i| same_deck(&refs[trainee][i].deck, &deck)) {
                    start_decks[trainee].push(index);
                }
                class_refs[deck_class].push((trainee, index));
                refs[trainee].push(Ref { inheritance_chances: q, value, deck: deck.clone() });
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
        // (a class that breached during the search: its fully checked decks from before are dropped too)
        self.setup.drop_breached(&self.state, &mut refs);
        self.setup.merge_seen(&mut self.state, &refs);
        let next_bound = if next < candidates.len() {
            sort_to(&mut candidates, next, &mut sorted_until);
            candidates[next].ceiling
        } else {
            0.0
        };
        if self.audit.capped_breaches > capped_breaches_before {
            unproven.cap_breach = true;
        }
        let complete = !unproven.any();
        let proven = complete
            && !done.is_empty()
            && (done.len() >= top.min(candidates.len()) || next >= candidates.len())
            && next_bound < done[0].0;
        let candidate_count = candidates.len();
        let certified_cap = self.audit.certified_cap;
        self.audit.certified_cap = previous_certificate.min(certified_cap);
        let mut result = CareerSearchResult {
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
            unproven,
            assumes: Assumes {
                two_feeder: self.audit.assumed_searches > assumed_before,
                multi_feeder: self.audit.multi_searches > multi_before,
            },
            proven,
            work: self.state.work,
            certified_cap,
        };
        result.qualify(self.setup.trainee_flags(&self.state, trainees));
        result
    }
}

impl CareerSetup<'_> {
    /// The flags of the deck classes of `trainees` together (`ClassFlags::union`): what a search over them rests on.
    pub(crate) fn trainee_flags(&self, state: &SearchState, trainees: &[UmaIndex]) -> ClassFlags {
        trainees
            .iter()
            .fold(ClassFlags::default(), |flags, &trainee| flags.union(state.class_flags[self.deck_class[trainee]]))
    }
    /// The job's class flags by class content (`class_key`), those with any flag set: what the worker and a kept plan
    /// keep (`KeptState::flags`, `Bests::flags`), keys ascending.
    pub(crate) fn keyed_flags(&self, state: &SearchState) -> Vec<(u64, ClassFlags)> {
        let mut flags: Vec<(u64, ClassFlags)> = self
            .deck_class
            .indices()
            .filter(|&uma| self.deck_class[uma] == uma && state.class_flags[uma] != ClassFlags::default())
            .map(|class| (self.class_key[class], state.class_flags[class]))
            .collect();
        flags.sort_unstable_by_key(|&(key, _)| key);
        flags
    }
    /// Kept flags by class content unioned into the job's classes of the same content (never replacing a flag).
    pub(crate) fn union_keyed(&self, state: &mut SearchState, flags: impl IntoIterator<Item = (u64, ClassFlags)>) {
        let mut by_key: crate::FxMap<u64, ClassFlags> = crate::FxMap::default();
        for (key, kept) in flags {
            let entry = by_key.entry(key).or_default();
            *entry = entry.union(kept);
        }
        for class in self.deck_class.indices().filter(|&uma| self.deck_class[uma] == uma) {
            if let Some(&kept) = by_key.get(&self.class_key[class]) {
                state.class_flags[class] = state.class_flags[class].union(kept);
            }
        }
    }
    /// Per trainee fully checked decks `refs` without those of breached classes (`ClassFlags::breached`).
    pub(crate) fn drop_breached(&self, state: &SearchState, refs: &mut UmaTable<Vec<Ref>>) {
        for trainee in refs.indices() {
            if state.class_flags[self.deck_class[trainee]].breached {
                refs[trainee].clear();
            }
        }
    }
}

impl CareerSearchResult {
    /// The result with its trainees' class flags `flags` (`CareerSetup::trainee_flags`) as they stand: a breached class
    /// makes it unproven (`cap_breach`; then not `complete`, not `proven`), an assuming one makes what it proves rest
    /// on those assumptions (`assumes`). Flags are only ever set, so this applies again unchanged to a kept search
    /// reused after a later search flagged one of its classes.
    pub(crate) const fn qualify(&mut self, flags: ClassFlags) {
        self.unproven.cap_breach |= flags.breached;
        self.assumes = self.assumes.union(flags.assumes);
        self.complete = !self.unproven.any();
        self.proven &= self.complete;
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
    ) -> TwinOutcome {
        let TwinSearch { trainee, chances: q, spark_scale: career_spark_scale, bar: threshold, starts } = search;
        let deck_class = self.setup.deck_class[trainee];
        let twin_key = (
            trainee.0 as u16,
            q.iter().map(|chance| chance.to_bits()).collect::<Vec<u64>>(),
            career_spark_scale.to_bits(),
        );
        // (a repeat of a fully checked outcome adds no Ref and no `ruled` proof: the first candidate's are the same)
        let repeat = |result: DeckRes| TwinOutcome { result, repeated: true };
        match twins.get(&twin_key) {
            Some(Twin::Done(value, deck)) => return repeat(DeckRes::Done(*value, deck.clone(), true)),
            Some(Twin::Bounded(bar)) if threshold >= *bar => return repeat(DeckRes::Bounded),
            Some(Twin::NoDeck) => return repeat(DeckRes::NoDeck),
            _ => {}
        }
        let result = self.best_deck(
            crate::career::decks::DeckSearch { trainee, spark_scale: career_spark_scale, bar: threshold },
            starts,
            n_scored,
        );
        if let Some(bound) = self.state.last_product_bound.take() {
            self.state.ruled[deck_class].push(ProductProof { inheritance_chances: q.to_vec(), product_bound: bound });
        }
        match &result {
            DeckRes::Done(value, deck, true) => {
                twins.insert(twin_key, Twin::Done(*value, deck.clone()));
            }
            DeckRes::Done(..) => {}
            DeckRes::Bounded => {
                twins.insert(twin_key, Twin::Bounded(threshold));
            }
            DeckRes::NoDeck => {
                twins.insert(twin_key, Twin::NoDeck);
            }
        }
        TwinOutcome { result, repeated: false }
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
///   it goes into `done` exactly where R's own search would have put it. R's own Ref and proof would be F's again
///   (the same chances, value, deck and product bound), so R records neither: a repeat bounds and starts nothing a
///   first one does not.
/// - Bounded at th1: no deck above F's seed, or the best product deck below th1 / (rho c (1 + 1e-9)); a higher bar
///   gives a higher seed, so R is Bounded too (no `ruled` entry: R's would only repeat what F's proves).
/// - NoDeck (no legal deck at all, at any bar): R the same.
///
/// R's search would also have spent work, so a work budget can now reach further than before (the answer the
/// unbudgeted search gives); outputs are otherwise the same, only the work counters fall.
enum Twin {
    /// Fully checked: value, deck.
    Done(f64, SelectedDeck),
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
    /// Her distinct fully checked decks so far in this search: the deck search's start decks.
    starts: &'search [&'search SelectedDeck],
}

/// `twin_deck_search`'s answer: the deck search's outcome, and whether it repeats an earlier candidate's (`Twin`).
struct TwinOutcome {
    result: DeckRes,
    /// A repeat of an outcome already recorded: its Ref (when fully checked) and its product proof are the first's.
    repeated: bool,
}

/// Whether two of one trainee's selected decks are the same deck: entries, plain pals and fillers (her hint chances
/// follow from them).
fn same_deck(first: &SelectedDeck, second: &SelectedDeck) -> bool {
    first.entries == second.entries
        && first.plain_pals == second.plain_pals
        && first.filler_cards == second.filler_cards
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
/// (`SearchState::ruled` as a combined job's plan finds it), its class flags unioned into the job's (never replacing
/// them) and the result qualified by the flags as they now stand (`CareerSearchResult::qualify`: a class flagged by a
/// later search, the plan's included, unproves it; a breached class's proofs are dropped), else searched now and kept
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
        cache.last().map(|entry| (entry.solved.clone(), entry.ruled.clone(), entry.class_flags.clone()))
    })();
    match kept {
        Some((mut solved, ruled, class_flags))
            if env.setup.solved_fits(&solved, &ruled, &class_flags, request.pool) =>
        {
            env.state.ruled = ruled;
            env.state.union_flags(&class_flags);
            for class in env.state.class_flags.indices() {
                if env.state.class_flags[class].breached {
                    env.state.ruled[class].clear();
                }
            }
            env.setup.drop_breached(&env.state, &mut solved.refs);
            solved.qualify(env.setup.trainee_flags(&env.state, request.trainees));
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
                let mut classes: Vec<UmaIndex> =
                    request.trainees.iter().map(|&trainee| env.setup.deck_class[trainee]).collect();
                classes.sort_unstable();
                classes.dedup();
                cache.push(KeptSolve {
                    key,
                    solved: solved.clone(),
                    ruled: env.state.ruled.clone(),
                    class_flags: env.state.class_flags.clone(),
                    classes,
                });
                while cache.len() > SOLVE_KEEP {
                    cache.remove(0);
                }
            }
            solved
        }
    }
}
