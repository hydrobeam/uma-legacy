//! The state-conditioned bounds of one configuration (`StateBound`): the band pass's (`plain`) and the cap-free band's,
//! its cheap bound (`cheap`) tried before the per-state one (`full`); and the search-wide ratio tables the cap-free
//! bounds read (`CapFreeRatios`). Also the state-conditioned value they bound, at a whole deck (`state_value`,
//! `state_value_cf`), and the configuration tables they read (`StateBoundConfig`).
use crate::deck::chains::{ChainKind, ChainState, KeyLevel, chain_states, event_pmf_job, key_level};
use crate::deck::joint::{ChainEntry, LandPre, MissFactors, land_factor, src_levels};
use crate::deck::sources::{SourceEntry, TargetSources};
use crate::deck::space::{EntryGroup, GroupSlots};
use crate::skills::{Model, OutfitOption};
use std::cell::RefCell;
use std::rc::Rc;
/// Landing factors and entry misses used by both state-conditioned bounds.
#[derive(Clone, Copy)]
pub(super) struct LandingBoundTables<'table> {
    /// Outfit-major factors, one block of target_count per outfit.
    pub(super) factors: &'table [LandPre],
    /// Entry-major optional rows, then chain level and target.
    pub(super) entry_rows: &'table [Option<Vec<MissFactors>>],
    pub(super) target_count: usize,
    pub(super) outfit_count: usize,
}

#[derive(Clone, Copy)]
pub(super) struct BoundOptions {
    pub(super) cap_free: bool,
    pub(super) trainee: crate::uma::UmaIndex,
    pub(super) fixed_factor: f64,
}

/// Slots fill in configuration-group order; chain_position is absent for a level-free slot.
#[derive(Clone, Copy)]
struct BoundSlot {
    group: usize,
    chain_position: usize,
}

/// A filled chain slot uses its chosen entry's miss rows at the state's level.
#[derive(Clone, Copy)]
struct FilledChainSlot<'row> {
    position: usize,
    rows: &'row [MissFactors],
}

/// An open chain slot relaxes over its group's entries, independently for misses and ratios.
#[derive(Clone, Copy)]
struct OpenChainSlot<'row> {
    position: usize,
    rows: &'row [MissFactors],
    ratios: &'row [f64],
    group: usize,
}

/// One chain slot's tables in the event-count bound, in deck pick order.
#[derive(Clone, Copy)]
struct ChainRatioSlot<'row> {
    position: usize,
    rows: &'row [MissFactors],
    filled: bool,
    ratios: &'row [f64],
    level_count: usize,
}

/// The cap-free band's ratio tables (the ratio bound of `StateBound::full` and `cheap`), one search's: per outfit of the
/// trainee its zero-base targets (`z`: factor 0 with no hint, whose ratios would be infinite, so they take the
/// per-target bound), and per group of `groups` (`group`), level and outfit (row l * outfit_count + o) the best ratio over its
/// entries: R(e, l) = prod over the other targets of the factor at e's misses at level l over the factor with no card;
/// per entry (`entry`) the same ratio of that entry alone. Both built on first use.
pub(crate) struct CapFreeRatios<'tab> {
    target_count: usize,
    outfit_count: usize,
    kind_count: usize,
    landing_factors: &'tab [LandPre],
    entry_rows: &'tab [Option<Vec<MissFactors>>],
    groups: &'tab [&'tab Vec<usize>],
    kinds: &'tab [ChainKind],
    pub(super) z: Vec<u32>,
    group_ratios: RefCell<Vec<Option<Rc<Vec<f64>>>>>,
    entry_ratios: RefCell<crate::FxMap<usize, Rc<Vec<f64>>>>,
}

impl<'tab> CapFreeRatios<'tab> {
    /// `outfit_count`: the trainee's outfit count in the cap-free band (`cap_free`), else 0 (no tables)
    pub(super) fn new(
        tables: LandingBoundTables<'tab>,
        cap_free: bool,
        kind_count: usize,
        groups: &'tab [&'tab Vec<usize>],
        kinds: &'tab [ChainKind],
    ) -> Self {
        let LandingBoundTables { factors: landing_factors, entry_rows, target_count, outfit_count } = tables;
        let z: Vec<u32> = (0..outfit_count)
            .map(|o| {
                (0..target_count)
                    .filter(|&k| !(landing_factors[o * target_count + k].f(1.0, 1.0) > 0.0))
                    .fold(0u32, |m, k| m | 1 << k)
            })
            .collect();
        let group_ratios = RefCell::new(vec![None; if cap_free { groups.len() } else { 0 }]);
        CapFreeRatios {
            target_count,
            outfit_count,
            kind_count,
            landing_factors,
            entry_rows,
            groups,
            kinds,
            z,
            group_ratios,
            entry_ratios: RefCell::default(),
        }
    }

    /// (outfit o's product over the targets outside its Z of the factors with no card: R's denominator)
    fn den(&self, o: usize) -> f64 {
        let (target_count, landing_factors, z) = (self.target_count, self.landing_factors, &self.z);
        (0..target_count)
            .filter(|&k| z[o] >> k & 1 == 0)
            .map(|k| landing_factors[o * target_count + k].f(1.0, 1.0))
            .product()
    }

    /// (outfit o's product over the targets outside its Z of the factors at an entry's level-l misses `row`, over `den`:
    /// R(e, l))
    fn ratio(&self, o: usize, row: &[MissFactors], den: f64) -> f64 {
        let (target_count, landing_factors, z) = (self.target_count, self.landing_factors, &self.z);
        (0..target_count)
            .filter(|&k| z[o] >> k & 1 == 0)
            .map(|k| landing_factors[o * target_count + k].f(row[k].no_gold, row[k].no_hint))
            .product::<f64>()
            / den
    }

    /// Group gi's best ratio per level (up to its kind's stop) and outfit, row l * outfit_count + o
    pub(super) fn group(&self, gi: usize) -> Rc<Vec<f64>> {
        if let Some(r) = &self.group_ratios.borrow()[gi] {
            return Rc::clone(r);
        }
        let (target_count, outfit_count) = (self.target_count, self.outfit_count);
        let stop = if gi < self.kind_count { self.kinds[gi].stop as usize } else { 0 };
        let mut m = vec![0.0f64; (stop + 1) * outfit_count];
        for o in 0..outfit_count {
            let den = self.den(o);
            for &j in self.groups[gi] {
                if let Some(rows) = &self.entry_rows[j] {
                    for l in 0..=stop.min(rows.len() / target_count - 1) {
                        let r = self.ratio(o, &rows[l * target_count..(l + 1) * target_count], den);
                        if !(r <= m[l * outfit_count + o]) {
                            m[l * outfit_count + o] = r;
                        }
                    }
                }
            }
        }
        let r = Rc::new(m);
        self.group_ratios.borrow_mut()[gi] = Some(Rc::clone(&r));
        r
    }

    /// Entry j's ratio per level and outfit, row l * outfit_count + o
    pub(super) fn entry(&self, j: usize) -> Rc<Vec<f64>> {
        if let Some(r) = self.entry_ratios.borrow().get(&j) {
            return Rc::clone(r);
        }
        let (target_count, outfit_count) = (self.target_count, self.outfit_count);
        let rows = self.entry_rows[j].as_ref().unwrap();
        let level_count = rows.len() / target_count;
        let mut m = vec![0.0f64; level_count * outfit_count];
        for o in 0..outfit_count {
            let den = self.den(o);
            for l in 0..level_count {
                m[l * outfit_count + o] = self.ratio(o, &rows[l * target_count..(l + 1) * target_count], den);
            }
        }
        let r = Rc::new(m);
        self.entry_ratios.borrow_mut().insert(j, Rc::clone(&r));
        r
    }
}

/// (`StateBound::full`'s cache, per number of filled slots and visiting order: each state's classes in the order the
/// states are visited, extended as far as a call has reached: `filled_classes` the class of its levels at the filled chain
/// positions (`filled_keys`), `representatives` the first state visited whose levels agree at the filled positions and, per group, up to
/// their order at its open positions (`completion_keys`))
struct StateClasses {
    filled_classes: Vec<u32>,
    representatives: Vec<u32>,
    filled_keys: crate::FxMap<u64, u32>,
    completion_keys: crate::FxMap<u64, u32>,
}

/// (Kept between calls of `full`, this configuration's: `state_classes` (`StateClasses`); per call (`generation`): each filled class's
/// misses and ratio-bound factor (`filled_factors`, `filled_stamps` its call), each visited state's terms.)
struct StateBoundCache {
    state_classes: Vec<Option<StateClasses>>,
    filled_stamps: Vec<u32>,
    filled_factors: Vec<f64>,
    state_terms: Vec<f64>,
    generation: u32,
}

/// Event-count bound workspace. Prefixes are extended only beyond the unchanged selected entries;
/// products keep their original target and slot order.
struct EventBoundCache {
    cached_entries: Vec<usize>,
    /// Filled slot's ratios, indexed by level and outfit.
    entry_ratios: Vec<Option<Rc<Vec<f64>>>>,
    /// Per depth and level cap, white/gold miss products of the slots already filled.
    level_prefix_misses: Vec<f64>,
    /// Per depth, misses of the filled slots with no chain level.
    plain_prefix_misses: Vec<f64>,
    level_misses: Vec<f64>,
    target_bounds: Vec<f64>,
    /// Last evaluated ratios per outfit, chain slot and level.
    cached_ratios: Vec<f64>,
    cached_level_counts: Vec<usize>,
    slot_ratios: Vec<f64>,
    /// Expectations per outfit, chain slot and event count; unchanged ratios reuse these values.
    expected_ratios: Vec<f64>,
    prefix_expectations: Vec<f64>,
}

/// One configuration's state-conditioned bounds at a partial deck `selection` (entries in pick order, filling the groups in
/// turn), over its tables `configuration` (`StateBoundConfig`; None when unused, and then no bound is asked for)
struct StateBoundContext<'cfg> {
    configuration: Option<Rc<StateBoundConfig>>,
    groups: &'cfg [EntryGroup<'cfg>],
    model: &'cfg Model<'cfg>,
    trainee: crate::uma::UmaIndex,
    entry_rows: &'cfg [Option<Vec<MissFactors>>],
    landing_factors: &'cfg [LandPre],
    zero_base_targets: &'cfg [u32],
    ratios: &'cfg CapFreeRatios<'cfg>,
    target_count: usize,
    cap_free_outfit_count: usize,
    fixed_factor: f64,
    cap_free: bool,
    // (each slot's group and chain position in pick order, the chain slots, each group's best ratios)
    slots: Vec<BoundSlot>,
    group_ratios: Vec<Rc<Vec<f64>>>,
    chain_slots: Vec<usize>,
    maximum_level: usize,
    key_levels: Option<Rc<KeyLevel>>,
    event_probabilities: Rc<Vec<f64>>,
    event_counts: Vec<usize>,
    remaining_event_mass: f64,
    all_zero_base_targets: u32,
}

pub(super) struct StateBound<'cfg> {
    context: StateBoundContext<'cfg>,
    state_cache: StateBoundCache,
    event_cache: EventBoundCache,
}

impl<'cfg> StateBound<'cfg> {
    /// `groups` supplies entries and slots; `configured_groups` supplies their search-wide group indices and slots.
    pub(super) fn new(
        configuration: Option<Rc<StateBoundConfig>>,
        groups: &'cfg [EntryGroup<'cfg>],
        configured_groups: &[GroupSlots],
        model: &'cfg Model<'cfg>,
        tables: LandingBoundTables<'cfg>,
        ratios: &'cfg CapFreeRatios<'cfg>,
        options: BoundOptions,
    ) -> Self {
        let LandingBoundTables {
            factors: landing_factors,
            entry_rows,
            target_count,
            outfit_count: cap_free_outfit_count,
        } = tables;
        let BoundOptions { cap_free, trainee, fixed_factor } = options;
        let group_count = groups.len();
        let zero_base_targets = &*ratios.z;
        let mut slots: Vec<BoundSlot> = vec![];
        let mut group_ratios: Vec<Rc<Vec<f64>>> = vec![];
        if cap_free && let Some(c) = &configuration {
            for g in 0..group_count {
                for t in 0..groups[g].slots {
                    slots.push(BoundSlot { group: g, chain_position: c.slots[g][t] });
                }
            }
            group_ratios = (0..group_count).map(|g| ratios.group(configured_groups[g].group)).collect();
        }
        let chain_slots: Vec<usize> = (0..slots.len()).filter(|&i| slots[i].chain_position != usize::MAX).collect();
        let maximum_level =
            configuration.as_ref().map_or(0, |c| c.kinds.iter().map(|k| k.stop as usize).max().unwrap_or(0));
        let state_cache = StateBoundCache {
            state_classes: vec![],
            filled_stamps: vec![],
            filled_factors: vec![],
            state_terms: vec![],
            generation: 0,
        };
        let key_levels: Option<Rc<KeyLevel>> = if cap_free {
            configuration.as_ref().and_then(|c| (!c.kinds.is_empty()).then(|| key_level(&c.kinds)))
        } else {
            None
        };
        let event_probabilities = if cap_free { event_pmf_job() } else { Rc::new(vec![]) };
        // (the event counts summed term by term, those of P(n) >= 1e-12; the rest's mass, each such term at most A
        // at every slot's most links done)
        let event_counts: Vec<usize> =
            (0..event_probabilities.len()).filter(|&n| event_probabilities[n] >= 1e-12).collect();
        let remaining_event_mass: f64 = (0..event_probabilities.len())
            .filter(|&n| event_probabilities[n] < 1e-12)
            .map(|n| event_probabilities[n])
            .sum::<f64>()
            * (1.0 + 1e-9)
            + 1e-300;
        let all_zero_base_targets: u32 = zero_base_targets.iter().fold(0u32, |m, &z| m | z);
        // (at most 16 chain slots, as outfits)
        assert!(chain_slots.len() <= 16, "{} chain slots: at most 16", chain_slots.len());
        let event_cache = {
            let (slot_count, level_cap_count, chain_count, event_count, on) =
                (slots.len(), maximum_level + 1, chain_slots.len(), event_counts.len(), usize::from(cap_free));
            EventBoundCache {
                cached_entries: vec![],
                entry_ratios: vec![None; on * slot_count],
                level_prefix_misses: vec![1.0f64; on * (slot_count + 1) * level_cap_count * target_count * 2],
                plain_prefix_misses: vec![1.0f64; on * (slot_count + 1) * target_count * 2],
                level_misses: vec![1.0f64; on * level_cap_count * target_count * 2],
                target_bounds: vec![0.0f64; on * level_cap_count],
                cached_ratios: vec![0.0f64; cap_free_outfit_count * chain_count * level_cap_count],
                cached_level_counts: vec![0usize; cap_free_outfit_count * chain_count],
                slot_ratios: vec![0.0f64; chain_count * level_cap_count],
                expected_ratios: vec![0.0f64; cap_free_outfit_count * chain_count * event_count],
                prefix_expectations: vec![0.0f64; cap_free_outfit_count * chain_count * event_count],
            }
        };
        StateBound {
            context: StateBoundContext {
                configuration,
                groups,
                model,
                trainee,
                entry_rows,
                landing_factors,
                zero_base_targets,
                ratios,
                target_count,
                cap_free_outfit_count,
                fixed_factor,
                cap_free,
                slots,
                group_ratios,
                chain_slots,
                maximum_level,
                key_levels,
                event_probabilities,
                event_counts,
                remaining_event_mass,
                all_zero_base_targets,
            },
            state_cache,
            event_cache,
        }
    }

    /// The configuration's root upper bound, used to prune against `sb_bar`; returns 0 when no bound is enabled.
    pub(super) fn root(&mut self, cfs: f64, sb_bar: f64, sbm: u8) -> f64 {
        if self.context.cap_free {
            let x = self.cheap(&[]);
            if cfs == 0.0 && x * (1.0 + 1e-9) < sb_bar {
                x
            } else {
                x.min(self.full(&[], if cfs == 0.0 { sb_bar } else { 0.0 }))
            }
        } else if sbm > 0 {
            self.bound(&[])
        } else {
            0.0
        }
    }

    /// The state-conditioned bound at `selection`: the cap-free band's per-state one (`full`, the full sum), else `plain`
    pub(super) fn bound(&mut self, selection: &[usize]) -> f64 {
        if self.context.cap_free { self.full(selection, 0.0) } else { self.plain(selection) }
    }

    pub(super) fn plain(&self, selection: &[usize]) -> f64 {
        self.context.plain(selection)
    }

    pub(super) fn full(&mut self, selection: &[usize], threshold: f64) -> f64 {
        self.context.full(&mut self.state_cache, selection, threshold)
    }

    pub(super) fn cheap(&mut self, selection: &[usize]) -> f64 {
        self.context.cheap(&mut self.event_cache, selection)
    }
}

impl StateBoundContext<'_> {
    /// The band pass's state-conditioned bound: per chain state, each target at its partial deck's misses and its open
    /// slots' groups' lowest, the state's weight times the product of the factors, summed; the best outfit
    fn plain(&self, selection: &[usize]) -> f64 {
        let context = self;
        let (target_count, fixed_factor, group_count) =
            (context.target_count, context.fixed_factor, context.groups.len());
        let configuration = context.configuration.as_ref().unwrap();
        let (mut base_white_misses, mut base_gold_misses) = ([1.0f64; 32], [1.0f64; 32]);
        let mut chain: Vec<FilledChainSlot<'_>> = vec![];
        let mut idx = 0usize;
        for g in 0..group_count {
            for t in 0..context.groups[g].slots {
                let rows: &[MissFactors] = if idx < selection.len() {
                    context.entry_rows[selection[idx]].as_deref().unwrap()
                } else {
                    &configuration.group_misses[g]
                };
                idx += 1;
                let p = configuration.slots[g][t];
                if p == usize::MAX {
                    for k in 0..target_count {
                        base_white_misses[k] *= rows[k].no_gold;
                        base_gold_misses[k] *= rows[k].no_hint;
                    }
                } else {
                    chain.push(FilledChainSlot { position: p, rows });
                }
            }
        }
        let opts = &context.model.opts[context.trainee];
        let mut acc = [0.0f64; 16];
        let (mut state_white_misses, mut state_gold_misses) = ([0.0f64; 32], [0.0f64; 32]);
        for ChainState { progress: x, probability: w } in &configuration.states {
            state_white_misses[..target_count].copy_from_slice(&base_white_misses[..target_count]);
            state_gold_misses[..target_count].copy_from_slice(&base_gold_misses[..target_count]);
            for &FilledChainSlot { position: p, rows } in &chain {
                let b = x[p] as usize * target_count;
                for k in 0..target_count {
                    state_white_misses[k] *= rows[b + k].no_gold;
                    state_gold_misses[k] *= rows[b + k].no_hint;
                }
            }
            for o in 0..opts.len() {
                let mut f = 1.0f64;
                for k in 0..target_count {
                    f *= context.landing_factors[o * target_count + k].f(state_white_misses[k], state_gold_misses[k]);
                }
                acc[o] += w * f;
            }
        }
        fixed_factor * acc[..opts.len()].iter().copied().fold(0.0f64, f64::max)
    }

    /// The cap-free band's bound (`cap_free`): per chain state x the lower of `plain`'s per-target bound A_x and a ratio
    /// bound B_x, summed over states, the best outfit. Derived: given x the deck's state value is a product over
    /// targets of phi_k at its misses given x (phi_k = G - (G - W) ng - W a nh, a = (1 - e)(1 - q)); its log gains
    /// less from a card the lower the misses already are (phi(ng y, nh y') / phi(ng, nh) rises with ng and with
    /// nh for y' <= y <= 1), so over the targets N with phi_k(1, 1) > 0 the completion's product is at most the
    /// partial deck's times each open slot's card's ratio from no card, at most the best ratio of its group at the
    /// slot's level (`CapFreeRatios::group`); the zero-base targets Z take A's factors. Every completion's state value
    /// is at most both, so their minimum bounds it state by state, and the sum over states (`state_value`'s argument,
    /// `TargetSources::certified_cap_free`) bounds its exact value.
    /// `cut` (> 0: the value a node must reach to pass): the states are taken heaviest first and the sum stops once
    /// it decides the test, returning its partial sum plus the weight left times every state's most (A with every
    /// slot at its most links done, at least any state's term): an upper bound either way, below `cut` exactly
    /// when the full sum's test would fail to the slack. 0: the full sum, in state order.
    fn full(&self, state_cache: &mut StateBoundCache, selection: &[usize], cut: f64) -> f64 {
        let context = self;
        let (target_count, cap_free_outfit_count, fixed_factor, maximum_level, group_count) = (
            context.target_count,
            context.cap_free_outfit_count,
            context.fixed_factor,
            context.maximum_level,
            context.groups.len(),
        );
        let configuration = context.configuration.as_ref().unwrap();
        // filled slots without a level (partial_white_misses, partial_gold_misses), open ones (open_white_misses, open_gold_misses: their groups' lowest misses; open_ratios: their best
        // ratios); filled chain slots (position, rows), open ones (position, lowest rows, ratio table, group);
        // every slot at its most links done (lowest_white_misses, lowest_gold_misses)
        let (mut partial_white_misses, mut partial_gold_misses, mut open_white_misses, mut open_gold_misses) =
            ([1.0f64; 32], [1.0f64; 32], [1.0f64; 32], [1.0f64; 32]);
        let (mut lowest_white_misses, mut lowest_gold_misses) = ([1.0f64; 32], [1.0f64; 32]);
        let mut open_ratios = [1.0f64; 16];
        let mut filled_slots = [FilledChainSlot { position: 0, rows: &[] }; 16];
        let mut filled_count = 0;
        let mut open_slots = [OpenChainSlot { position: 0, rows: &[], ratios: &[], group: 0 }; 16];
        let mut open_count = 0;
        let mut open = false;
        let mut idx = 0usize;
        for g in 0..group_count {
            for t in 0..context.groups[g].slots {
                let filled = idx < selection.len();
                let rows: &[MissFactors] = if filled {
                    context.entry_rows[selection[idx]].as_deref().unwrap()
                } else {
                    &configuration.group_misses[g]
                };
                idx += 1;
                let top = &rows[(rows.len() / target_count - 1) * target_count..];
                for k in 0..target_count {
                    lowest_white_misses[k] *= top[k].no_gold;
                    lowest_gold_misses[k] *= top[k].no_hint;
                }
                let p = configuration.slots[g][t];
                if filled {
                    if p == usize::MAX {
                        for k in 0..target_count {
                            partial_white_misses[k] *= rows[k].no_gold;
                            partial_gold_misses[k] *= rows[k].no_hint;
                        }
                    } else {
                        filled_slots[filled_count] = FilledChainSlot { position: p, rows };
                        filled_count += 1;
                    }
                } else {
                    open = true;
                    let rm = &context.group_ratios[g][..];
                    if p == usize::MAX {
                        for k in 0..target_count {
                            open_white_misses[k] *= rows[k].no_gold;
                            open_gold_misses[k] *= rows[k].no_hint;
                        }
                        for o in 0..cap_free_outfit_count {
                            open_ratios[o] *= rm[o];
                        }
                    } else {
                        open_slots[open_count] = OpenChainSlot { position: p, rows, ratios: rm, group: g };
                        open_count += 1;
                    }
                }
            }
        }
        let filled_slots = &filled_slots[..filled_count];
        let open_slots = &open_slots[..open_count];
        let opts = &context.model.opts[context.trainee];
        let outfit_count = opts.len();
        let early = cut > 0.0;
        let mut umax = [0.0f64; 16];
        if early {
            for o in 0..opts.len() {
                let mut a = 1.0f64;
                for k in 0..target_count {
                    a *= context.landing_factors[o * target_count + k].f(lowest_white_misses[k], lowest_gold_misses[k]);
                }
                umax[o] = a;
            }
        }
        let state_count = configuration.states.len();
        let (ord, tail) = if early {
            let x = configuration.order.get_or_init(|| {
                let mut o: Vec<u32> = (0..state_count as u32).collect();
                o.sort_by(|&a, &b| {
                    configuration.states[b as usize]
                        .probability
                        .partial_cmp(&configuration.states[a as usize].probability)
                        .unwrap()
                        .then(a.cmp(&b))
                });
                let mut t = vec![0.0f64; state_count + 1];
                for i in (0..state_count).rev() {
                    t[i] = t[i + 1] + configuration.states[o[i] as usize].probability;
                }
                WeightedStates { indices: o, suffix_weights: t }
            });
            (&*x.indices, &*x.suffix_weights)
        } else {
            (&[][..], &[][..])
        };
        let mut acc = [0.0f64; 16];
        let mut chk = true;
        let (mut state_white_misses, mut state_gold_misses) = ([0.0f64; 32], [0.0f64; 32]);
        // x*: state x's misses of the partial deck (its level-free filled slots', then its filled chain slots'
        // rows at x's levels)
        let xstar = |x: &[u8], state_white_misses: &mut [f64; 32], state_gold_misses: &mut [f64; 32]| {
            state_white_misses[..target_count].copy_from_slice(&partial_white_misses[..target_count]);
            state_gold_misses[..target_count].copy_from_slice(&partial_gold_misses[..target_count]);
            for &FilledChainSlot { position: p, rows } in filled_slots {
                let r = &rows[x[p] as usize * target_count..][..target_count];
                for ((y, z), rk) in state_white_misses[..target_count]
                    .iter_mut()
                    .zip(state_gold_misses[..target_count].iter_mut())
                    .zip(r)
                {
                    *y *= rk.no_gold;
                    *z *= rk.no_hint;
                }
            }
        };
        // per outfit, the ratio bound's factor of the targets outside Z at x*
        let bn_of = |o: usize, state_white_misses: &[f64], state_gold_misses: &[f64]| -> f64 {
            let mut bn = 1.0f64;
            for k in 0..target_count {
                if context.zero_base_targets[o] >> k & 1 == 0 {
                    bn *= context.landing_factors[o * target_count + k].f(state_white_misses[k], state_gold_misses[k]);
                }
            }
            bn
        };
        // the state's term per outfit from x* and its bn (open slots only): y*, every open slot at its group's
        // lowest, then A at y* (and Z's part of it, bz), with an open slot min(A, bz bn r) (r: the open slots' best
        // ratios at x's levels), else A
        let terms =
            |x: &[u8], state_white_misses: &[f64], state_gold_misses: &[f64], bn: &[f64; 16], f: &mut [f64; 16]| {
                let (mut yg, mut yh) = ([0.0f64; 32], [0.0f64; 32]);
                for k in 0..target_count {
                    yg[k] = state_white_misses[k] * open_white_misses[k];
                    yh[k] = state_gold_misses[k] * open_gold_misses[k];
                }
                for &OpenChainSlot { position: p, rows, .. } in open_slots {
                    let r = &rows[x[p] as usize * target_count..][..target_count];
                    for ((y, z), rk) in yg[..target_count].iter_mut().zip(yh[..target_count].iter_mut()).zip(r) {
                        *y *= rk.no_gold;
                        *z *= rk.no_hint;
                    }
                }
                for o in 0..outfit_count {
                    let (mut a, mut bz) = (1.0f64, 1.0f64);
                    for k in 0..target_count {
                        let fa = context.landing_factors[o * target_count + k].f(yg[k], yh[k]);
                        a *= fa;
                        if context.zero_base_targets[o] >> k & 1 != 0 {
                            bz *= fa;
                        }
                    }
                    f[o] = if open {
                        let mut r = open_ratios[o];
                        for &OpenChainSlot { position: p, ratios: rm, .. } in open_slots {
                            r *= rm[x[p] as usize * cap_free_outfit_count + o];
                        }
                        a.min(bz * bn[o] * r)
                    } else {
                        a
                    };
                }
            };
        let (mut ratio_products, mut outfit_values) = ([0.0f64; 16], [0.0f64; 16]);
        // With an open chain slot, the states share work. A state's filled part (its misses x* given x and, per
        // outfit, the ratio bound's factor of the targets outside Z at x*) depends only on its levels at the
        // filled positions, so it is computed once per class (the same expressions on the same f64s: the same
        // values). And the term of a state is a symmetric function of the levels at one group's open positions
        // (those slots share their group's lowest rows and best ratios, multiplied in), so the states that agree
        // up to that order have one term (exactly, in real arithmetic), taken from the first of them visited:
        // the sum is the same, up to the rounding of the products.
        let ucache = state_cache;
        let memo = !open_slots.is_empty() && maximum_level < 16;
        if memo {
            let uc = &mut *ucache;
            let mi = selection.len() * 2 + usize::from(early);
            if uc.state_classes.len() <= mi {
                uc.state_classes.resize_with(mi + 1, || None);
            }
            uc.generation = uc.generation.wrapping_add(1);
            if uc.generation == 0 {
                uc.filled_stamps.iter_mut().for_each(|x| *x = 0);
                uc.generation = 1;
            }
        }
        for i in 0..state_count {
            let ChainState { progress: x, probability: w } =
                if early { &configuration.states[ord[i] as usize] } else { &configuration.states[i] };
            if memo {
                let StateBoundCache { state_classes, filled_stamps, filled_factors, state_terms, generation } =
                    &mut *ucache;
                let mp = state_classes[selection.len() * 2 + usize::from(early)].get_or_insert_with(|| StateClasses {
                    filled_classes: vec![],
                    representatives: vec![],
                    filled_keys: crate::FxMap::default(),
                    completion_keys: crate::FxMap::default(),
                });
                if mp.representatives.len() <= i {
                    // (the state's keys: its levels at the filled positions, then per group its open positions'
                    // levels sorted; 4 bits each, at most 16 positions)
                    let mut fkey = 0u64;
                    for &FilledChainSlot { position: p, .. } in filled_slots {
                        fkey = fkey << 4 | u64::from(x[p]);
                    }
                    let mut ckey = fkey;
                    let mut s0 = 0usize;
                    while s0 < open_slots.len() {
                        let mut s1 = s0;
                        while s1 < open_slots.len() && open_slots[s1].group == open_slots[s0].group {
                            s1 += 1;
                        }
                        let mut lv = [0u8; 16];
                        for s in s0..s1 {
                            lv[s - s0] = x[open_slots[s].position];
                        }
                        lv[..s1 - s0].sort_unstable();
                        for &l in &lv[..s1 - s0] {
                            ckey = ckey << 4 | u64::from(l);
                        }
                        s0 = s1;
                    }
                    let nfk = mp.filled_keys.len() as u32;
                    let f = *mp.filled_keys.entry(fkey).or_insert(nfk);
                    let r = *mp.completion_keys.entry(ckey).or_insert(i as u32);
                    mp.filled_classes.push(f);
                    mp.representatives.push(r);
                }
                let r = mp.representatives[i] as usize;
                if state_terms.len() < (i + 1) * outfit_count {
                    state_terms.resize((i + 1) * outfit_count, 0.0);
                }
                if r == i {
                    let f = mp.filled_classes[i] as usize;
                    let fw = 2 * target_count + outfit_count;
                    if filled_stamps.len() <= f {
                        filled_stamps.resize(f + 1, 0);
                        filled_factors.resize((f + 1) * fw, 0.0);
                    }
                    let fxr = &mut filled_factors[f * fw..(f + 1) * fw];
                    if filled_stamps[f] != *generation {
                        filled_stamps[f] = *generation;
                        xstar(x, &mut state_white_misses, &mut state_gold_misses);
                        fxr[..target_count].copy_from_slice(&state_white_misses[..target_count]);
                        fxr[target_count..2 * target_count].copy_from_slice(&state_gold_misses[..target_count]);
                        for o in 0..outfit_count {
                            fxr[2 * target_count + o] = bn_of(o, &state_white_misses, &state_gold_misses);
                        }
                    }
                    ratio_products[..outfit_count]
                        .copy_from_slice(&fxr[2 * target_count..2 * target_count + outfit_count]);
                    terms(
                        x,
                        &fxr[..target_count],
                        &fxr[target_count..2 * target_count],
                        &ratio_products,
                        &mut outfit_values,
                    );
                    for o in 0..outfit_count {
                        state_terms[i * outfit_count + o] = outfit_values[o];
                        acc[o] += w * outfit_values[o];
                    }
                } else {
                    for o in 0..outfit_count {
                        acc[o] += w * state_terms[r * outfit_count + o];
                    }
                }
            } else {
                xstar(x, &mut state_white_misses, &mut state_gold_misses);
                if open {
                    for o in 0..outfit_count {
                        ratio_products[o] = bn_of(o, &state_white_misses, &state_gold_misses);
                    }
                }
                terms(x, &state_white_misses, &state_gold_misses, &ratio_products, &mut outfit_values);
                for o in 0..outfit_count {
                    acc[o] += w * outfit_values[o];
                }
            }
            // (every 4 states: a test decided by a partial sum is the full sum's, so how often it is tried
            // changes no outcome, only where the sum stops)
            if early && chk && (i & 3 == 3 || i + 1 == state_count) {
                // (the weight left with a hair of slack for its rounding)
                let rem = tail[i + 1] * (1.0 + 1e-12);
                let (mut hi, mut lo) = (0.0f64, 0.0f64);
                for o in 0..opts.len() {
                    hi = hi.max(acc[o] + rem * umax[o]);
                    lo = lo.max(acc[o]);
                }
                // An infinite or NaN term prevents an early stop; evaluate the full sum.
                if !hi.is_finite() {
                    chk = false;
                    continue;
                }
                if fixed_factor * hi * (1.0 + 1e-9) < cut || fixed_factor * lo * (1.0 + 1e-9) >= cut {
                    return fixed_factor * hi;
                }
            }
        }
        fixed_factor * acc[..opts.len()].iter().copied().fold(0.0f64, f64::max)
    }

    /// The cap-free band's cheap bound, tried before `full` (a node either fails is pruned): given N = n events
    /// only. Derived: per state the completion's value is at most prod_Z phi_k at its lowest misses (every slot
    /// at its most links done; the zero-base targets) times prod_N phi_k at the filled level-free slots' misses
    /// times, per chain slot, its card's ratio at its level over that base (a filled slot's own, an open slot's
    /// group's best from no card, `CapFreeRatios`) and each open level-free slot's best ratio: the log-submodularity
    /// of `full`, card by card. Each chain slot's ratio rises with its level (its misses only fall), so given
    /// N = n, where the configuration's progress vector is negatively upper-orthant dependent (`key_level`,
    /// checked exactly, the projection on these positions included), the expected product is at most the product
    /// of the slots' expected ratios at P(level | n); summed over P(n). Infinite (no prune) when the key fails
    /// the orthant check.
    fn cheap(&self, event_cache: &mut EventBoundCache, selection: &[usize]) -> f64 {
        let context = self;
        let (target_count, cap_free_outfit_count, fixed_factor, remaining_event_mass, all_zero_base_targets) = (
            context.target_count,
            context.cap_free_outfit_count,
            context.fixed_factor,
            context.remaining_event_mass,
            context.all_zero_base_targets,
        );
        let configuration = context.configuration.as_ref().unwrap();
        if context.key_levels.as_ref().is_some_and(|k| !k.nuod) {
            return f64::INFINITY;
        }
        // Borrow the independent hot buffers once: writes to one must not reload another's Vec metadata.
        let EventBoundCache {
            cached_entries,
            entry_ratios,
            level_prefix_misses,
            plain_prefix_misses,
            level_misses,
            target_bounds,
            cached_ratios,
            cached_level_counts,
            slot_ratios,
            expected_ratios,
            prefix_expectations,
        } = event_cache;
        let (slot_count, maximum_level, chain_count, event_count) =
            (context.slots.len(), context.maximum_level, context.chain_slots.len(), context.event_counts.len());
        let level_cap_count = maximum_level + 1;
        let d = selection.len();
        // the prefix rows: kept up to the first pick that differs from the last deck's, extended to this one's
        let mut shared_prefix = 0usize;
        while shared_prefix < d
            && shared_prefix < cached_entries.len()
            && cached_entries[shared_prefix] == selection[shared_prefix]
        {
            shared_prefix += 1;
        }
        cached_entries.truncate(shared_prefix);
        for i in shared_prefix..d {
            let rows: &[MissFactors] = context.entry_rows[selection[i]].as_deref().unwrap();
            let level_count = rows.len() / target_count;
            let (zlo, zhi) = level_prefix_misses.split_at_mut((i + 1) * level_cap_count * target_count * 2);
            let (zp, zc) =
                (&zlo[i * level_cap_count * target_count * 2..], &mut zhi[..level_cap_count * target_count * 2]);
            for cc in 0..level_cap_count {
                let r = &rows[cc.min(level_count - 1) * target_count..][..target_count];
                let b = cc * target_count * 2;
                for ((z2, p2), rk) in zc[b..b + 2 * target_count]
                    .chunks_exact_mut(2)
                    .zip(zp[b..b + 2 * target_count].chunks_exact(2))
                    .zip(r)
                {
                    z2[0] = p2[0] * rk.no_gold;
                    z2[1] = p2[1] * rk.no_hint;
                }
            }
            let (plo, phi) = plain_prefix_misses.split_at_mut((i + 1) * target_count * 2);
            let (pp0, pc) = (&plo[i * target_count * 2..], &mut phi[..target_count * 2]);
            if context.slots[i].chain_position == usize::MAX {
                for k in 0..target_count {
                    pc[2 * k] = pp0[2 * k] * rows[k].no_gold;
                    pc[2 * k + 1] = pp0[2 * k + 1] * rows[k].no_hint;
                }
            } else {
                pc.copy_from_slice(pp0);
                entry_ratios[i] = Some(context.ratios.entry(selection[i]));
            }
            cached_entries.push(selection[i]);
        }
        // filled level-free slots' misses (partial_white_misses, partial_gold_misses); per level cap c (every slot at most c links done, at most its
        // stop) each slot at its lowest misses (level_misses at c: the prefix's, then the open slots'); open level-free
        // slots' best ratios (open_ratios); every level-free slot's lowest misses (plain_white_misses, plain_gold_misses: the zero-base targets'
        // level-free part)
        let (mut partial_white_misses, mut partial_gold_misses) = ([1.0f64; 32], [1.0f64; 32]);
        for k in 0..target_count {
            partial_white_misses[k] = plain_prefix_misses[d * target_count * 2 + 2 * k];
            partial_gold_misses[k] = plain_prefix_misses[d * target_count * 2 + 2 * k + 1];
        }
        level_misses.copy_from_slice(
            &level_prefix_misses[d * level_cap_count * target_count * 2..(d + 1) * level_cap_count * target_count * 2],
        );
        let (mut plain_white_misses, mut plain_gold_misses) = (partial_white_misses, partial_gold_misses);
        let mut open_ratios = [1.0f64; 16];
        for i in d..slot_count {
            let BoundSlot { group: g, chain_position: pos } = context.slots[i];
            let rows: &[MissFactors] = &configuration.group_misses[g];
            let level_count = rows.len() / target_count;
            for cc in 0..level_cap_count {
                let r = &rows[cc.min(level_count - 1) * target_count..][..target_count];
                let b = cc * target_count * 2;
                for (z2, rk) in level_misses[b..b + 2 * target_count].chunks_exact_mut(2).zip(r) {
                    z2[0] *= rk.no_gold;
                    z2[1] *= rk.no_hint;
                }
            }
            if pos == usize::MAX {
                for k in 0..target_count {
                    plain_white_misses[k] *= rows[k].no_gold;
                    plain_gold_misses[k] *= rows[k].no_hint;
                }
                let rm = &context.group_ratios[g];
                for o in 0..cap_free_outfit_count {
                    open_ratios[o] *= rm[o];
                }
            }
        }
        // the chain slots, in pick order: (position, rows (an open slot's: its group's lowest), filled, ratio
        // table (a filled slot's own from no card, an open slot's group's best), levels)
        let mut chain_slots =
            [ChainRatioSlot { position: 0, rows: &[], filled: false, ratios: &[], level_count: 0 }; 16];
        for (s, &i) in context.chain_slots.iter().enumerate() {
            let BoundSlot { group: g, chain_position: pos } = context.slots[i];
            let (rows, filled, rm): (&[MissFactors], bool, &[f64]) = if i < d {
                (context.entry_rows[selection[i]].as_deref().unwrap(), true, &entry_ratios[i].as_ref().unwrap()[..])
            } else {
                ((&*configuration.group_misses[g]), false, &context.group_ratios[g][..])
            };
            chain_slots[s] =
                ChainRatioSlot { position: pos, rows, filled, ratios: rm, level_count: rows.len() / target_count };
        }
        // (no filled level-free slot: the filled chain slots' ratios over that base are their tabled ratios from
        // no card)
        let base_empty = partial_white_misses[..target_count].iter().all(|&x| x == 1.0)
            && partial_gold_misses[..target_count].iter().all(|&x| x == 1.0);
        // each zero-base target's factor as a function of one chain slot's level: the slot whose own misses fall
        // the most from no links to all (its `target_carriers`), every other slot at its lowest (a factor at least the
        // target's on every state, rising with that slot's level); None: no chain slot's misses of it fall with
        // its level. Per zero-base target: its misses with every slot at its lowest but its `target_carriers` slot (every slot
        // when None). (Only the zero-base targets of some outfit are used.)
        let (mut target_carriers, mut other_slot_misses) = ([None::<usize>; 32], [(1.0f64, 1.0f64); 32]);
        if all_zero_base_targets != 0 {
            for k in 0..target_count {
                if all_zero_base_targets >> k & 1 == 0 {
                    continue;
                }
                let mut b: Option<(usize, f64)> = None;
                for si in 0..chain_count {
                    let ChainRatioSlot { rows, level_count, .. } = chain_slots[si];
                    let (r0, rt) = (rows[k].no_hint, rows[(level_count - 1) * target_count + k].no_hint);
                    let drop = if r0 > 0.0 { rt / r0 } else { 1.0 };
                    if drop < 1.0 && b.is_none_or(|x| drop < x.1) {
                        b = Some((si, drop));
                    }
                }
                target_carriers[k] = b.map(|x| x.0);
                let (mut a, mut bb) = (plain_white_misses[k], plain_gold_misses[k]);
                for si in 0..chain_count {
                    if target_carriers[k] != Some(si) {
                        let ChainRatioSlot { rows, level_count, .. } = chain_slots[si];
                        a *= rows[(level_count - 1) * target_count + k].no_gold;
                        bb *= rows[(level_count - 1) * target_count + k].no_hint;
                    }
                }
                other_slot_misses[k] = (a, bb);
            }
        }
        let opts = &context.model.opts[context.trainee];
        let mut best = 0.0f64;
        for o in 0..opts.len() {
            // per cap: every target at its lowest misses (A)
            for cc in 0..level_cap_count {
                let mut a = 1.0f64;
                let b = cc * target_count * 2;
                for (lpk, z2) in context.landing_factors[o * target_count..(o + 1) * target_count]
                    .iter()
                    .zip(level_misses[b..b + 2 * target_count].chunks_exact(2))
                {
                    a *= lpk.f(z2[0], z2[1]);
                }
                target_bounds[cc] = a;
            }
            let mut pb = 1.0f64;
            for k in 0..target_count {
                if context.zero_base_targets[o] >> k & 1 == 0 {
                    pb *= context.landing_factors[o * target_count + k]
                        .f(partial_white_misses[k], partial_gold_misses[k]);
                }
            }
            // per chain slot and level its ratio (slot_ratios, slot after slot at stride level_cap_count), times the zero-base targets
            // it carries
            for s in 0..chain_count {
                let ChainRatioSlot { rows, filled, ratios: rm, level_count, .. } = chain_slots[s];
                for l in 0..level_count {
                    slot_ratios[s * level_cap_count + l] = if !filled || base_empty {
                        rm[l * cap_free_outfit_count + o]
                    } else {
                        let mut x = 1.0f64;
                        for k in 0..target_count {
                            if context.zero_base_targets[o] >> k & 1 == 0 {
                                x *= context.landing_factors[o * target_count + k].f(
                                    partial_white_misses[k] * rows[l * target_count + k].no_gold,
                                    partial_gold_misses[k] * rows[l * target_count + k].no_hint,
                                );
                            }
                        }
                        x / pb
                    };
                }
            }
            let mut zconst = 1.0f64;
            for k in 0..target_count {
                if context.zero_base_targets[o] >> k & 1 == 0 {
                    continue;
                }
                match target_carriers[k] {
                    None => {
                        zconst *= context.landing_factors[o * target_count + k]
                            .f(other_slot_misses[k].0, other_slot_misses[k].1);
                    }
                    Some(s0) => {
                        let ChainRatioSlot { rows, level_count, .. } = chain_slots[s0];
                        for l in 0..level_count {
                            slot_ratios[s0 * level_cap_count + l] *= context.landing_factors[o * target_count + k].f(
                                other_slot_misses[k].0 * rows[l * target_count + k].no_gold,
                                other_slot_misses[k].1 * rows[l * target_count + k].no_hint,
                            );
                        }
                    }
                }
            }
            let base = pb * open_ratios[o] * zconst;
            // per n: the lower of A at the cap min(n, maximum_level) and the ratio bound
            let tot = match &context.key_levels {
                None => target_bounds[0].min(base),
                Some(kl) => {
                    // each chain slot's expectation at P(level | n) and their running product, from the first slot
                    // whose ratios changed
                    let mut redo = false;
                    for s in 0..chain_count {
                        let ChainRatioSlot { position: pos, level_count, .. } = chain_slots[s];
                        let (cs, ci) = ((o * chain_count + s) * level_cap_count, o * chain_count + s);
                        let same = cached_level_counts[ci] == level_count
                            && (0..level_count).all(|l| {
                                cached_ratios[cs + l].to_bits() == slot_ratios[s * level_cap_count + l].to_bits()
                            });
                        let eo = ci * event_count;
                        if !same {
                            cached_level_counts[ci] = level_count;
                            cached_ratios[cs..cs + level_count]
                                .copy_from_slice(&slot_ratios[s * level_cap_count..s * level_cap_count + level_count]);
                            let rr = &slot_ratios[s * level_cap_count..s * level_cap_count + level_count];
                            for (ni, &n) in context.event_counts.iter().enumerate() {
                                let m = &kl.marg[configuration.projected_positions[pos]][n][..level_count];
                                let mut x = 0.0f64;
                                for (&ml, &rl) in m.iter().zip(rr) {
                                    x += ml * rl;
                                }
                                expected_ratios[eo + ni] = x;
                            }
                            redo = true;
                        }
                        if redo {
                            for ni in 0..event_count {
                                prefix_expectations[eo + ni] =
                                    if s == 0 { 1.0 } else { prefix_expectations[eo - event_count + ni] }
                                        * expected_ratios[eo + ni];
                            }
                        }
                    }
                    let mut tot = remaining_event_mass * target_bounds[maximum_level];
                    let last = (o * chain_count + chain_count.max(1) - 1) * event_count;
                    for (ni, &n) in context.event_counts.iter().enumerate() {
                        let pn = context.event_probabilities[n];
                        let prod = if chain_count == 0 { 1.0f64 } else { prefix_expectations[last + ni] };
                        tot += pn * target_bounds[n.min(maximum_level)].min(base * prod);
                    }
                    tot
                }
            };
            if tot > best || tot.is_nan() {
                best = if tot.is_nan() { f64::INFINITY } else { tot };
            }
        }
        fixed_factor * best
    }
}

/// The state-conditioned value of one deck (trainee `u`, per `inheritance_chances`, fixed factor included): over the chain
/// progress states x (`chain_states`, early ends and stop risk folded in), P(x) times the product over targets of each
/// target's factor at its misses given x, every group's outcomes at their per-target chances; the best outfit.
/// Derived: given x, every source's outcomes are independent of the others' (`joint_fast`); one group's outcomes are a
/// single draw, so when each outcome hints at most one target its per-target indicators are negatively associated
/// (a one-trial multinomial), and so are increasing functions of disjoint sets of them; each target's factor rises
/// with its own indicators, so the expected product given x is at most the product of the targets' expected factors,
/// equal without coupled groups. Hence value >= the exact P(all), equal when no group touches two targets.
/// None when some entry fails the negative-association certification (`TargetSources::uncertified_association`, `group_assoc`).
pub(super) fn state_value(
    model: &Model,
    u: crate::uma::UmaIndex,
    inheritance_chances: &[f64],
    entries: &[SourceEntry<&TargetSources>],
    filler: usize,
) -> Option<Vec<f64>> {
    if entries.iter().any(|e| e.sources.uncertified_association) {
        return None;
    }
    Some(state_value_any(model, u, inheritance_chances, entries, filler))
}
/// `state_value` for the decks the cap-free search covers (`TargetSources::certified_cap_free`: every group's levels negatively upper-orthant
/// dependent, `group_nuod_lvl`, all the derivation above needs of a group); None otherwise. The same value where both
/// apply.
pub(super) fn state_value_cf(
    model: &Model,
    u: crate::uma::UmaIndex,
    inheritance_chances: &[f64],
    entries: &[SourceEntry<&TargetSources>],
    filler: usize,
) -> Option<Vec<f64>> {
    if entries.iter().any(|e| !e.sources.certified_cap_free) {
        return None;
    }
    Some(state_value_any(model, u, inheritance_chances, entries, filler))
}
fn state_value_any(
    model: &Model,
    u: crate::uma::UmaIndex,
    inheritance_chances: &[f64],
    entries: &[SourceEntry<&TargetSources>],
    filler: usize,
) -> Vec<f64> {
    let uncertain_count = model.uncertain.len();
    let mut chain_entries: Vec<ChainEntry> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind != ChainKind::NONE)
        .map(|(i, e)| ChainEntry { kind: e.kind, source_index: Some(i) })
        .collect();
    for _ in 0..filler {
        chain_entries.push(ChainEntry { kind: ChainKind::FILLER, source_index: None });
    }
    chain_entries.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.source_index.cmp(&b.source_index)));
    let kinds: Vec<ChainKind> = chain_entries.iter().map(|x| x.kind).collect();
    let states = if kinds.is_empty() {
        std::rc::Rc::new(vec![ChainState { progress: vec![], probability: 1.0 }])
    } else {
        chain_states(&kinds)
    };
    // the non-chain entries' misses (constant over states), and per chain position its entry's rows
    let mut cg = vec![1.0f64; uncertain_count];
    let mut chh = vec![1.0f64; uncertain_count];
    for e in entries.iter().filter(|e| e.kind == ChainKind::NONE) {
        let r = src_levels(e.sources, 0, uncertain_count);
        for k in 0..uncertain_count {
            cg[k] *= r[k].no_gold;
            chh[k] *= r[k].no_hint;
        }
    }
    let rows: Vec<Option<Vec<MissFactors>>> = chain_entries
        .iter()
        .map(|&ChainEntry { kind: kd, source_index: e }| {
            e.map(|i| src_levels(entries[i].sources, kd.stop, uncertain_count))
        })
        .collect();
    let opts = &model.opts[u];
    let outfit_count = opts.len();
    let mut acc = vec![0.0f64; inheritance_chances.len() * outfit_count];
    let (mut ng, mut nh) = (vec![0.0f64; uncertain_count], vec![0.0f64; uncertain_count]);
    for ChainState { progress: x, probability: w } in states.iter() {
        ng.copy_from_slice(&cg);
        nh.copy_from_slice(&chh);
        for (p, r) in rows.iter().enumerate() {
            if let Some(r) = r {
                let b = x[p] as usize * uncertain_count;
                for k in 0..uncertain_count {
                    ng[k] *= r[b + k].no_gold;
                    nh[k] *= r[b + k].no_hint;
                }
            }
        }
        for (qi, &q) in inheritance_chances.iter().enumerate() {
            for (o, OutfitOption { coverage: st, .. }) in opts.iter().enumerate() {
                let ew = &model.ews[u][o];
                let f: f64 = (0..uncertain_count).map(|k| land_factor(st[k], ew[k], q, ng[k], nh[k])).product();
                acc[qi * outfit_count + o] += w * f;
            }
        }
    }
    let fixed = model.fixed_factor();
    (0..inheritance_chances.len())
        .map(|qi| fixed * (0..outfit_count).map(|o| acc[qi * outfit_count + o]).fold(0.0f64, f64::max))
        .collect()
}

/// A configuration's state-conditioned bound tables: the chain state distribution projected on the positions whose
/// group has chain targets, per group its slots' positions (usize::MAX: level-independent), per group and chain level
/// each target's lowest misses over the group's entries
pub(super) struct StateBoundConfig {
    pub(super) states: Vec<crate::deck::chains::ChainState>,
    pub(super) slots: Vec<Vec<usize>>,
    pub(super) group_misses: Vec<Vec<MissFactors>>,
    /// the configuration's chain kinds in position order (fillers included) and each projected position's place among them
    pub(super) kinds: Vec<ChainKind>,
    pub(super) projected_positions: Vec<usize>,
    /// (the cap-free bound's early stop) the states by weight, highest first, and the weight from each on (built on first use)
    pub(super) order: std::cell::OnceCell<WeightedStates>,
}

/// State indices in descending probability order and the remaining weight from each index.
pub(super) struct WeightedStates {
    pub(super) indices: Vec<u32>,
    pub(super) suffix_weights: Vec<f64>,
}
