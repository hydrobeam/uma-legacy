//! The routes through a scored career's veteran (`routes`): each trio taken with her in it, on its own lineage.
use crate::career::inherit::{ExactOptions, Inheritance, inherit};
use crate::career::plan::ScoringContext;
use crate::career::plan::cont::{ScoreBase, SideStep, TrioSlot};
use crate::career::plan::kept::same_deck;
use crate::career::plan::score::build::{Setup, setup_entries};
use crate::career::plan::score::{FinisherChoice, FinisherParents, best_finisher};
use crate::career::plan::search::legal_pair;
use crate::career::plan::step::{LiftRead, MAX_ROUTES, NO_SLOT, Route, ShadowRoute, WhileBuilding};
use crate::career::pool::Pool;
use crate::career::schedule::ScheduleOutfits;
use crate::career::sim::Sim;
use crate::career::{Legacy, Lineage};

/// What `routes` reads of the scored career beyond the scoring's context.
pub(super) struct RouteInputs<'route> {
    /// Her veteran as the extended pool holds her (`CareerSetup::veteran`: every target at the rank's mix, her
    /// schedule's wins and possible pinks); her stars in any slot are the outcome's (`CareerSetup::her_slots`).
    pub(super) her: &'route Legacy,
    /// Her index in the extended pool.
    pub(super) new_veteran: usize,
    /// The scoring's partner setups (`prepare_setups`): per candidate her veteran can lift, one per kept partner.
    pub(super) partner_setups: &'route [Setup],
    /// She can land every target and her parents are not both full veterans (her own full veteran a side then).
    pub(super) full_possible: bool,
    /// Her uma (`Data::umas`).
    pub(super) trainee: crate::uma::UmaIndex,
    /// The outcome entries the finisher setups may take (`build::OUTCOME_BUDGET` less the partner and lift setups'):
    /// each its leaf row and two corner tables per table it reads.
    pub(super) entry_budget: usize,
    /// 4^nu, the leaf rows per setup, and 2^nu, a corner table's entries.
    pub(super) leaf_count: usize,
    pub(super) table_size: usize,
}

/// What `routes` builds: every lift slot it drafted (`lift_setups`, a route's lifts index them) and which the first
/// batch takes (`first_lifts`: her Current side's, then an Alternative side's that a priced route reads while they fit
/// the budget); the first batch's routes and their finisher setups (one per route that reads its own C); per route
/// the budget left out, its check (`ShadowRoute`, where its lift slots are the first batch's) and the route as a later
/// batch takes it (`LeftOutRoute`); and which setups the collapse reads (`collapse_lifts`: the lift slots a priced
/// route reads, kept or not; every left-out route's that reads its own C is in `left_out`).
pub(super) struct RouteSet {
    pub(super) lift_setups: Vec<Setup>,
    pub(super) first_lifts: Vec<bool>,
    pub(super) collapse_lifts: Vec<bool>,
    pub(super) finisher_setups: Vec<Setup>,
    pub(super) routes: Vec<Route>,
    pub(super) shadows: Vec<Option<ShadowRoute>>,
    pub(super) left_out: Vec<LeftOutRoute>,
}

/// One parent of a route's C.
#[derive(Clone, Copy)]
enum RouteParent {
    /// A trio side as the fit has it (`ScoreBase::trio_sides`).
    Side(TrioSlot),
    /// A lift slot's lifted fresh side: her candidate's best career's deck, in the outfit its record runs, with this
    /// veteran and the slot's partner as parents, so her veteran is C's grandparent (that side's second).
    Lifted(usize),
    /// Her own full veteran, C's parent.
    Her,
}

/// A route before its C is found: what it takes, and C's parents.
struct RouteDraft {
    lifts: [usize; 2],
    full_only: bool,
    fixed_cost: f64,
    /// A lower bound on the steps it takes (the least step each of its lift slots took, `LiftRead::least_step`).
    step_floor: f64,
    parents: [RouteParent; 2],
    /// Its C: the trio's (None: one of her own full veteran's routes, its C searched, `best_finisher`).
    finisher: Option<crate::uma::UmaIndex>,
    /// Its C while a lifted side builds (`Route::while_building`); `Slot` holds the paired draft's index here (a paired
    /// draft: one so named, its C a fallback of the drafts naming it, `price_drafts`).
    while_building: WhileBuilding,
}

/// A lift slot: the setup, the lifted side's veteran, and the trio side it lifts (its routes replace that side, and no
/// other, with this career).
pub(super) struct LiftSide {
    pub(super) setup: Setup,
    veteran: Legacy,
    side: TrioSlot,
}

/// A scoring's lift slots (`lift_slots`) and, per slot, what V after her reads of it over the outcomes (`LiftRead`, from
/// an evaluation of the partner and lift slots alone, `build::lift_reads`): each route's steps cost at least its lift
/// slots' least ones.
pub(super) struct LiftSlots {
    pub(super) lifts: Vec<LiftSide>,
    pub(super) reads: Vec<LiftRead>,
}

/// A lift slot's lineage (`lift_side`): the trio side it lifts and her place beside the parent kept with her.
struct LiftedLineage {
    side: TrioSlot,
    place: LiftPlace,
}

/// Her place in a lifted lineage: the parent beside her (a legacy index) and her slot there (`CareerSetup::her_slot`'s
/// parts: a partner setup's for her Current side, `own_parent_places`' for an Alternative side).
struct LiftPlace {
    partner: usize,
    misses_without_her: Vec<f64>,
    target_spark_scales: Vec<[f64; 2]>,
    miss_by_stars: [f64; 4],
}

/// The routes of a career whose veteran `inputs.her` is the last of `extended_pool` (`RouteSet`): per fresh trio side
/// (`Current` or `Alternative`) and per lift slot of that side, the trio with that side lifted, the other at the base;
/// per trio with both sides fresh, both lifted (each pair of their lift slots); and where she can land every target,
/// her own full veteran (the career's own lineage, her stars the outcome's) paired with every side of every other
/// uma's candidate at the base, and with each lifted side.
///
/// A lift slot lifts one fresh side and reads one career: that side's deck on the lifted lineage, in the one outfit
/// option its record runs (where her outfits' grades differ, the option her exact odds pick at her lineage's chances,
/// `CareerSetup::exact`; else every option's table, the record being the same), its record (G1 wins and possible pinks,
/// `Evaluator::wins` in that option) the lifted side's veteran's (`CareerSetup::veteran`). So a route's step, its odds
/// and its record describe one career. Her Current side's lifted lineages are her veteran beside each partner setup's
/// partner (her best career with the veteran as a parent, as her continuation was fitted); an Alternative side's are
/// her veteran beside each of that side's own two parents (`own_parent_places`), so its lifted career keeps the
/// pedigree its price as a side assumes. A lift slot's odds below are its side's own (`ScoreBase::side_odds`): a lift
/// lifts the side past them, below her odds now or not, and her step there is her continuation's at the lifted odds
/// (`ScoreBase::step_plan`).
/// Each route's C is held fixed: a trio's route takes the trio's own C (the fit's best on its lineages; the route is
/// left out where she is no legal C on the new parents), one of her own full veteran's the allowed uma best on its two
/// parents with her carrying every target at 3 stars (`best_finisher`, the most any outcome gives her, so its ceiling
/// bounds every outcome's); her slots in C's lineage (her as C's parent or grandparent, `CareerSetup::her_slots`) are
/// the finisher setup, so the fold values that C at each outcome's stars. Where a lifted side still builds at its lift's
/// odds, its later builders replace the parents that put her in C's lineage, so the route's C is V0's there
/// (`Route::while_building`: a trio's route the trio's own C; her own full veteran's route with a lifted side the route
/// with that side as it is today, where that route is priced).
///
/// A route whose lower bound (its lift slots' least steps, `LiftRead::least_step`, the least step V after her reads of
/// each at any outcome, + its fixed sides + the least of 1 / the most its C's odds can be, `Setup::odds_bound`, and its
/// C while a side builds) reaches V0 (with 1e-9 V0 to spare for rounding) can never be below V after her and is left
/// out (a lift slot no outcome lifts takes no route), but a paired route's C stays its dependents' fallback
/// (`price_drafts`: these omissions lose nothing). Past the memory budget or `MAX_ROUTES`, the routes with the highest
/// lower bounds are left out too (`dropped`, `keep_routes`): V after her is then at least the model's (fewer routes),
/// the saving at most it
pub(super) fn routes(
    simulator: &mut Sim,
    context: &ScoringContext,
    extended_pool: &mut Pool,
    inputs: &RouteInputs,
    lift_slots: LiftSlots,
    score_buffer: &mut Vec<f64>,
) -> RouteSet {
    let score_base = context.score_base;
    let data = simulator.data;
    let current_value = score_base.current_value;
    let character = |candidate: usize| data.umas[score_base.candidates[candidate]].id;
    let her_character = data.umas[inputs.trainee].id;
    let her_at_most = her_at_most(simulator, inputs.her);
    let LiftSlots { lifts, reads } = lift_slots;
    // (her own full veteran's routes, where she can land every target: per candidate whether she pairs with her sides,
    // not of her uma)
    let full_partners: Option<Vec<bool>> = inputs
        .full_possible
        .then(|| (0..score_base.candidates.len()).map(|candidate| character(candidate) != her_character).collect());
    let drafts = route_drafts(score_base, DraftLifts { lifts: &lifts, reads: &reads }, full_partners.as_deref());
    crate::career::clock::prof("sc_route_drafts", drafts.len() as f64);
    let finishers = FinisherChoice { umas: score_base.finisher_umas, scales: score_base.odds_scales };
    // one draft's C priced (`price_drafts`): C's parents in the pool (fresh and lifted sides pushed, popped after) and
    // her slots there, its C (the trio's where it is legal on the new parents, else the best above `floor`), its
    // finisher setup and the most its odds can be
    let price = |draft: &RouteDraft, floor: f64| {
        let pool_size = extended_pool.veterans.len();
        let mut places = [0usize; 2];
        let mut positions: Vec<(usize, Option<usize>)> = vec![];
        let mut excluded = [0u32; 2];
        for (index, parent) in draft.parents.iter().enumerate() {
            match *parent {
                RouteParent::Side(slot) => {
                    let veteran = &score_base.trio_sides[slot.candidate.0][slot.side];
                    places[index] = match veteran.step {
                        SideStep::Existing(legacy) => legacy,
                        SideStep::Current | SideStep::Alternative(_) => {
                            simulator.env.setup.pool_push(extended_pool, &veteran.legacy);
                            extended_pool.veterans.len() - 1
                        }
                    };
                    excluded[index] = character(slot.candidate.0);
                }
                RouteParent::Lifted(lift) => {
                    simulator.env.setup.pool_push(extended_pool, &lifts[lift].veteran);
                    places[index] = extended_pool.veterans.len() - 1;
                    // (her the lifted side's second parent: that side's second grandparent slot of C's)
                    positions.push((places[index], Some(1)));
                    excluded[index] = character(lifts[lift].setup.candidate);
                }
                RouteParent::Her => {
                    simulator.env.setup.pool_push(extended_pool, &her_at_most);
                    places[index] = extended_pool.veterans.len() - 1;
                    positions.push((places[index], None));
                    excluded[index] = her_character;
                }
            }
        }
        // (a labelled block: each way the draft gets no C leaves it with that answer, the pool restored below)
        let priced = 'price: {
            let finisher_trainee = if let Some(trio_finisher) = draft.finisher {
                let lineage = Lineage { trainee: trio_finisher, first_parent: places[0], second_parent: places[1] };
                if !legal_pair(data, extended_pool, lineage) {
                    break 'price DraftPrice::NoFinisher;
                }
                trio_finisher
            } else {
                let parents = FinisherParents { first_parent: places[0], second_parent: places[1], excluded };
                match best_finisher(simulator, finishers, extended_pool, parents, floor, score_buffer) {
                    Some((_, trainee)) => trainee,
                    None if floor > 0.0 => break 'price DraftPrice::FloorPruned,
                    None => break 'price DraftPrice::NoFinisher,
                }
            };
            let lineage = Lineage { trainee: finisher_trainee, first_parent: places[0], second_parent: places[1] };
            let Some(slot) = simulator.env.setup.her_slots(extended_pool, lineage, &positions) else {
                break 'price DraftPrice::NoFinisher;
            };
            let setup = Setup {
                candidate: usize::MAX,
                trainee: finisher_trainee,
                partner: usize::MAX,
                misses_without_her: slot.misses_without_her,
                target_spark_scales: slot.target_spark_scales,
                miss_by_stars: slot.miss_by_stars,
                decks: 0..simulator.evaluator.cache.decks[finisher_trainee].len(),
                outfit: None,
                odds_below: 0.0,
            };
            // (her odds at most this at any outcome, priced at her uma's scale: C's step at least its inverse)
            let most_odds = setup.odds_bound(simulator) * score_base.odds_scales.of(finisher_trainee);
            DraftPrice::Priced { setup, most_odds }
        };
        while extended_pool.veterans.len() > pool_size {
            extended_pool.pop_veteran();
        }
        priced
    };
    // (dominated routes never read their own C, `drop_dominated`: they leave no setup for the collapse either)
    let priced = drop_dominated(price_drafts(&drafts, current_value, price), &reads, current_value);
    // (a route one of whose lift slots never leaves her grinding takes its C while building wherever it is taken: it
    // needs no finisher slot, `Route::finisher_slot`, and its finisher setup is read by nothing, the collapse included)
    let needs_finisher = |route: &Route| !route.lifts.iter().any(|&slot| slot != NO_SLOT && !reads[slot].grinds);
    let finishers: Vec<Option<usize>> = priced
        .iter()
        .map(|priced_route| {
            if needs_finisher(&priced_route.route) {
                setup_entries(simulator, &priced_route.setup, inputs.leaf_count, inputs.table_size)
            } else {
                Some(0)
            }
        })
        .collect();
    // (the lift slots a priced route reads, kept or not, which the collapse reads; the first batch's: her Current
    // side's within the partner setups' share, `fit_partner_setups`, each reading at most what its partner setup does;
    // an Alternative side's charged here, in order, while they fit; a route through another goes to a later batch)
    let mut read = vec![false; lifts.len()];
    for priced_route in &priced {
        for slot in priced_route.route.lifts {
            if slot != NO_SLOT {
                read[slot] = true;
            }
        }
    }
    let mut lift_used = 0usize;
    let mut collapse_lifts = vec![false; lifts.len()];
    let first_lifts: Vec<bool> = lifts
        .iter()
        .zip(&read)
        .enumerate()
        .map(|(slot, (lift, &is_read))| match score_base.side_step(lift.side) {
            SideStep::Alternative(_) if is_read => {
                collapse_lifts[slot] = true;
                let entries = setup_entries(simulator, &lift.setup, inputs.leaf_count, inputs.table_size)
                    .and_then(|entries| lift_used.checked_add(entries))
                    .filter(|&total| total <= inputs.entry_budget);
                entries.inspect(|&total| lift_used = total).is_some()
            }
            SideStep::Alternative(_) => false,
            SideStep::Current | SideStep::Existing(_) => {
                collapse_lifts[slot] = true;
                true
            }
        })
        .collect();
    let budget = RouteBudget {
        finishers: &finishers,
        first_lifts: &first_lifts,
        entry_budget: inputs.entry_budget - lift_used,
        draft_count: drafts.len(),
    };
    let kept = keep_routes(priced, budget, needs_finisher);
    let lift_setups = lifts.into_iter().map(|lift| lift.setup).collect();
    RouteSet::of(LiftSetups { setups: lift_setups, first: first_lifts, collapse: collapse_lifts }, kept)
}

/// A scoring's lift slots as `RouteSet` holds them: every drafted one's setup, which the first batch takes, and which
/// the collapse reads (`RouteSet`).
struct LiftSetups {
    setups: Vec<Setup>,
    first: Vec<bool>,
    collapse: Vec<bool>,
}

impl RouteSet {
    /// The route set of the lift slots `lifts` and the routes `keep_routes` kept and left out: each left-out route
    /// checked at every outcome where its lift slots are the first batch's, else taken in a later batch unchecked
    fn of(lifts: LiftSetups, kept: KeptRoutes<Setup>) -> Self {
        let LiftSetups { setups: lift_setups, first: first_lifts, collapse: collapse_lifts } = lifts;
        let KeptRoutes { routes, finisher_setups, left_out } = kept;
        let shadows = left_out
            .iter()
            .map(|left| {
                let checkable = left.route.lifts.iter().all(|&slot| slot == NO_SLOT || first_lifts[slot]);
                let Route { lifts, full_only, fixed_cost, .. } = left.route;
                checkable.then_some(ShadowRoute { lifts, full_only, fixed_cost, finisher_floor: left.finisher_floor })
            })
            .collect();
        let left_out = left_out
            .into_iter()
            .map(|LeftOut { route, setup, paired, .. }| LeftOutRoute { route, setup, paired })
            .collect();
        Self { lift_setups, first_lifts, collapse_lifts, finisher_setups, routes, shadows, left_out }
    }
    /// The setups the first batch's collapse reads beside its own slots: the lift slots it reads (`collapse_lifts`:
    /// her Current side's every one, read by a route or not, an Alternative side's where a priced route reads it) that
    /// it does not hold, and every left-out route's finisher setup where that route reads its own C (as a kept one's is
    /// a finisher slot). A setup counts by the drafted slots and whether a route reads it, never by whether the budget
    /// kept it, so leaving a route out moves no representative (`excluding_a_route_moves_no_setup_of_the_collapse`)
    pub(super) fn collapse_setups(&self) -> Vec<&Setup> {
        let lifts = (0..self.lift_setups.len())
            .filter(|&slot| self.collapse_lifts[slot] && !self.first_lifts[slot])
            .map(|slot| &self.lift_setups[slot]);
        let left_out = self.left_out.iter().filter(|left| left.route.finisher_slot != NO_SLOT).map(|left| &left.setup);
        lifts.chain(left_out).collect()
    }
}

/// Her veteran as a route's C sees her: every target at 3 stars, the most any outcome gives (so the C chosen on it, and
/// its ceiling, bound C's odds at every outcome: C's odds only rise with her stars); her slots take the outcome's
fn her_at_most(simulator: &Sim, her: &Legacy) -> Legacy {
    Legacy {
        sparks: simulator.env.setup.skill_job.targets.iter().map(|target| (target.group, 3)).collect(),
        mix: None,
        ..her.clone()
    }
}

/// The lift slots of a career whose veteran `inputs.her` is the last of `extended_pool` (`routes`): her Current side's
/// beside each partner setup's partner (`inputs.partner_setups`), then each Alternative side's beside its own parents
/// (`own_parent_places`), each where its side's deck is cached (`lift_side`)
pub(super) fn lift_slots(
    simulator: &mut Sim,
    context: &ScoringContext,
    extended_pool: &Pool,
    inputs: &RouteInputs,
) -> Vec<LiftSide> {
    let score_base = context.score_base;
    let mut legacies_with_her = context.legacies.to_vec();
    legacies_with_her.push(her_at_most(simulator, inputs.her));
    let current_lineages = inputs.partner_setups.iter().filter_map(|partner_setup| {
        let side = score_base.current_side(partner_setup.candidate)?;
        let place = LiftPlace {
            partner: partner_setup.partner,
            misses_without_her: partner_setup.misses_without_her.clone(),
            target_spark_scales: partner_setup.target_spark_scales.clone(),
            miss_by_stars: partner_setup.miss_by_stars,
        };
        Some(LiftedLineage { side, place })
    });
    let lifted_lineages: Vec<LiftedLineage> =
        current_lineages.chain(own_parent_places(simulator, score_base, extended_pool, inputs)).collect();
    lifted_lineages
        .into_iter()
        .filter_map(|lineage| lift_side(simulator, context, extended_pool, inputs, lineage, &legacies_with_her))
        .collect()
}

/// What the route drafts read of the lift slots: the slots and what V after her read of them (`LiftSlots`).
#[derive(Clone, Copy)]
struct DraftLifts<'lifts> {
    lifts: &'lifts [LiftSide],
    reads: &'lifts [LiftRead],
}

/// The routes' drafts (`routes`, before any C is priced), in this order: per trio, per fresh side of it (A's, then
/// B's) each of that side's lift slots (`lifts`, their `LiftSide::side`) with the other side at the base, then per pair
/// of their lift slots both lifted where both sides are fresh, each with the trio's C (and while a lifted side builds,
/// the trio's C at its step); then, where `full_partners` is given (she can land every target), per candidate she pairs
/// with, her own full veteran beside each of that candidate's sides at the base, and beside each of her lift slots (her
/// C searched; while that side builds, the C of her full veteran beside that same side at the base)
fn route_drafts(score_base: &ScoreBase, lift_slots: DraftLifts, full_partners: Option<&[bool]>) -> Vec<RouteDraft> {
    let DraftLifts { lifts, reads } = lift_slots;
    let is_fresh = |slot: TrioSlot| matches!(score_base.side_step(slot), SideStep::Current | SideStep::Alternative(_));
    let mut drafts: Vec<RouteDraft> = vec![];
    let lifts_of =
        |side: TrioSlot| lifts.iter().enumerate().filter(move |(_, lift)| lift.side == side).map(|(slot, _)| slot);
    for trio in score_base.trios {
        for k in 0..2 {
            let (slot, other) = (trio.sides[k], trio.sides[1 - k]);
            if !is_fresh(slot) {
                continue;
            }
            for lift in lifts_of(slot) {
                drafts.push(RouteDraft {
                    lifts: [lift, NO_SLOT],
                    full_only: false,
                    fixed_cost: score_base.side_cost(other),
                    step_floor: reads[lift].least_step,
                    parents: [RouteParent::Lifted(lift), RouteParent::Side(other)],
                    finisher: Some(trio.finisher),
                    while_building: WhileBuilding::Fixed(trio.finisher_cost),
                });
            }
        }
        let [first, second] = trio.sides;
        if is_fresh(first) && is_fresh(second) {
            for first_lift in lifts_of(first) {
                for second_lift in lifts_of(second) {
                    drafts.push(RouteDraft {
                        lifts: [first_lift, second_lift],
                        full_only: false,
                        fixed_cost: 0.0,
                        step_floor: reads[first_lift].least_step + reads[second_lift].least_step,
                        parents: [RouteParent::Lifted(first_lift), RouteParent::Lifted(second_lift)],
                        finisher: Some(trio.finisher),
                        while_building: WhileBuilding::Fixed(trio.finisher_cost),
                    });
                }
            }
        }
    }
    if let Some(full_partners) = full_partners {
        for candidate in (0..score_base.candidates.len()).filter(|&candidate| full_partners[candidate]) {
            // (per fresh side, her n/n with that side as it is: the C of the routes lifting it while it builds)
            let mut base_drafts = vec![None; score_base.trio_sides[candidate].len()];
            for side in 0..score_base.trio_sides[candidate].len() {
                let slot = TrioSlot { candidate: crate::career::plan::CandidateIndex(candidate), side };
                if is_fresh(slot) {
                    base_drafts[side] = Some(drafts.len());
                }
                drafts.push(RouteDraft {
                    lifts: [NO_SLOT, NO_SLOT],
                    full_only: true,
                    fixed_cost: score_base.side_cost(slot),
                    step_floor: 0.0,
                    parents: [RouteParent::Her, RouteParent::Side(slot)],
                    finisher: None,
                    while_building: WhileBuilding::Skip,
                });
            }
            for (lift, lifted) in lifts.iter().enumerate().filter(|(_, lift)| lift.side.candidate.0 == candidate) {
                drafts.push(RouteDraft {
                    lifts: [lift, NO_SLOT],
                    full_only: true,
                    fixed_cost: 0.0,
                    step_floor: reads[lift].least_step,
                    parents: [RouteParent::Her, RouteParent::Lifted(lift)],
                    finisher: None,
                    while_building: base_drafts[lifted.side.side].map_or(WhileBuilding::Skip, WhileBuilding::Slot),
                });
            }
        }
    }
    drafts
}

/// What pricing one draft's C found (`price_drafts`).
enum DraftPrice<S> {
    /// Its finisher setup, and the most its C's odds can be at any outcome as the base prices them
    /// (`Setup::odds_bound` x her uma's scale; 0: she lands nothing).
    Priced { setup: S, most_odds: f64 },
    /// A searched C (her own full veteran's route) with none above the floor asked (`finisher_floor`, above 0).
    FloorPruned,
    /// No C: the trio's C is no legal C on the new parents, no allowed uma has odds there, or her slots fail.
    NoFinisher,
}

/// A draft priced and kept for the memory budget's choice (`price_drafts`): its route (`while_building`'s `Slot` a
/// draft index until `keep_routes` renumbers it), its finisher setup, the lower bound the budget ranks it by (its own,
/// or for a paired route kept only as a fallback, the least of the dependents' that need it) and its draft.
struct PricedRoute<S> {
    route: Route,
    setup: S,
    rank: f64,
    draft_index: usize,
    /// The least its C can cost at any outcome: the least of 1 / its C's most odds and its C while building (`own`),
    /// and its C while building alone (`building`), what a route that never reads its own C takes (`ShadowRoute`).
    floors: FinisherFloors,
}

/// A priced route's C's floors (`PricedRoute::floors`).
#[derive(Clone, Copy)]
struct FinisherFloors {
    own: f64,
    building: f64,
}

/// Each draft's C priced (`price`, given the draft and the floor its C's priced odds must beat, `finisher_floor`) and
/// the routes that can bring V after her under V0, in draft order.
///
/// A paired draft (one a lifted draft's `WhileBuilding::Slot` names: her own full veteran with that side as it is
/// today) is priced at the floor its dependents need too (the least `others` among it and them), and its C serves as
/// their fallback whether or not it passes on its own. One that cannot pass is kept as a route only where a priced
/// dependent's fallback can bring that dependent under V0, ranked at that dependent's lower bound; it is never V after
/// her's least itself (its every value is at least its own lower bound, at least V0). So what is left out here loses
/// nothing: a draft whose `others`, and its dependents', reach V0; a C below its floor (`sc_routes_floor_pruned`: no C
/// and no fallback can bring it, or a dependent through it, under V0, C's odds bounded at every outcome by its odds
/// with her at 3 stars, `best_finisher`'s); a lower bound at V0; and a paired route that no kept dependent can use
fn price_drafts<S>(
    drafts: &[RouteDraft],
    current_value: f64,
    mut price: impl FnMut(&RouteDraft, f64) -> DraftPrice<S>,
) -> Vec<PricedRoute<S>> {
    let limit = current_value * (1.0 + 1e-9);
    let others_of = |draft: &RouteDraft| draft.step_floor + draft.fixed_cost;
    // per draft, the least `others` among the drafts whose C while building is its (infinite: none)
    let mut dependents_others = vec![f64::INFINITY; drafts.len()];
    for draft in drafts {
        if let WhileBuilding::Slot(paired) = draft.while_building {
            dependents_others[paired] = dependents_others[paired].min(others_of(draft));
        }
    }
    // per draft priced, the most its C's odds can be (priced); per paired draft priced that does not pass on its own,
    // its route and setup held, and the least lower bound among the priced dependents its fallback can bring under V0
    let mut fallback_odds: Vec<Option<f64>> = vec![None; drafts.len()];
    let mut held: Vec<Option<(Route, S, FinisherFloors)>> = (0..drafts.len()).map(|_| None).collect();
    let mut needed_rank = vec![f64::INFINITY; drafts.len()];
    let mut priced = vec![];
    for (draft_index, draft) in drafts.iter().enumerate() {
        let others = others_of(draft);
        let least_others = others.min(dependents_others[draft_index]);
        if !(least_others < current_value) {
            continue;
        }
        let fallback_cost = |while_building: WhileBuilding| match while_building {
            WhileBuilding::Fixed(cost) => cost,
            WhileBuilding::Slot(paired) => fallback_odds[paired].map_or(f64::INFINITY, |odds| 1.0 / odds),
            WhileBuilding::Skip => f64::INFINITY,
        };
        let floor = finisher_floor(current_value, least_others, fallback_cost(draft.while_building));
        let (setup, most_odds) = match price(draft, floor) {
            DraftPrice::Priced { setup, most_odds } => (setup, most_odds),
            DraftPrice::FloorPruned => {
                crate::career::clock::prof("sc_routes_floor_pruned", 1.0);
                continue;
            }
            DraftPrice::NoFinisher => continue,
        };
        // (while a lifted side builds, its C is V0's: the trio's, or the paired draft's, at most its odds)
        let while_building = match draft.while_building {
            WhileBuilding::Slot(paired) if fallback_odds[paired].is_none() => WhileBuilding::Skip,
            other => other,
        };
        let building_floor = fallback_cost(while_building);
        let own_floor = if most_odds > 0.0 { 1.0 / most_odds } else { f64::INFINITY };
        if most_odds > 0.0 {
            fallback_odds[draft_index] = Some(most_odds);
        }
        let lower_bound = others + own_floor.min(building_floor);
        let floors = FinisherFloors { own: own_floor.min(building_floor), building: building_floor };
        let route = Route {
            lifts: draft.lifts,
            full_only: draft.full_only,
            fixed_cost: draft.fixed_cost,
            finisher_slot: 0,
            while_building,
        };
        if lower_bound < limit {
            if let WhileBuilding::Slot(paired) = while_building
                && others + building_floor < limit
            {
                needed_rank[paired] = needed_rank[paired].min(lower_bound);
            }
            priced.push(PricedRoute { route, setup, rank: lower_bound, draft_index, floors });
        } else if dependents_others[draft_index] < current_value {
            held[draft_index] = Some((route, setup, floors));
        }
    }
    for (draft_index, held_route) in held.into_iter().enumerate() {
        if let Some((route, setup, floors)) = held_route
            && needed_rank[draft_index].is_finite()
        {
            priced.push(PricedRoute { route, setup, rank: needed_rank[draft_index], draft_index, floors });
        }
    }
    priced.sort_by_key(|priced_route| priced_route.draft_index);
    priced
}

/// The priced routes (`price_drafts`, in draft order) less those another one makes redundant at every outcome
/// (`sc_routes_dominated`). The routes lifting one side with the other as it is (one lift slot, the trio's C while it
/// builds, `WhileBuilding::Fixed`), where no outcome leaves her grinding at once after that lift (`LiftRead::grinds`),
/// take their C while building wherever they are taken: each one's value is ((the slot's step + 0) + its other side) +
/// that C. One whose other side and C each cost at least another's (rounding is monotone), or whose sum of the two
/// passes another's by more than rounding can reverse below V0 (1e-12 of 2 V0 + both sums: a value below V0 has a step
/// below V0, and each sum's rounding is within 2^-52 of the total), is never below that one where it could be V after
/// her; of two the same, the later goes. The relation only lowers the sum, or keeps it and moves earlier, so every route
/// left out has a kept one at most it: V after her is the same f64 at every outcome
fn drop_dominated<S>(priced: Vec<PricedRoute<S>>, reads: &[LiftRead], current_value: f64) -> Vec<PricedRoute<S>> {
    // (per lift slot, the priced routes of that kind on it: their places, other sides' and Cs' careers)
    let mut by_lift: Vec<Vec<(usize, f64, f64)>> = vec![vec![]; reads.len()];
    for (index, priced_route) in priced.iter().enumerate() {
        let Route { lifts: [lift, second], full_only, fixed_cost, while_building, .. } = priced_route.route;
        if let WhileBuilding::Fixed(finisher) = while_building
            && !full_only
            && second == NO_SLOT
            && !reads[lift].grinds
        {
            by_lift[lift].push((index, fixed_cost, finisher));
        }
    }
    let dominates = |kept: (f64, f64), left: (f64, f64), earlier: bool| {
        let ((kept_side, kept_finisher), (left_side, left_finisher)) = (kept, left);
        let (kept_sum, left_sum) = (kept_side + kept_finisher, left_side + left_finisher);
        let same = kept_side.to_bits() == left_side.to_bits() && kept_finisher.to_bits() == left_finisher.to_bits();
        if same {
            return earlier;
        }
        (kept_side <= left_side && kept_finisher <= left_finisher)
            || left_sum - kept_sum > 1e-12 * (2.0 * current_value + kept_sum + left_sum)
    };
    let mut redundant = vec![false; priced.len()];
    for group in &by_lift {
        for &(index, side, finisher) in group {
            redundant[index] = group.iter().any(|&(other, other_side, other_finisher)| {
                other != index && dominates((other_side, other_finisher), (side, finisher), other < index)
            });
        }
    }
    let dropped = redundant.iter().filter(|&&left| left).count();
    if dropped > 0 {
        crate::career::clock::prof("sc_routes_dominated", dropped as f64);
    }
    // (a dominated route never reads its own C: its finisher setup is read by nothing, the collapse included)
    priced.into_iter().zip(redundant).filter_map(|(priced_route, left)| (!left).then_some(priced_route)).collect()
}

/// A route the budget left out (`keep_routes`): the route (its lifts as drafted; its finisher slot 0 where it reads
/// its own C, else NO_SLOT), its finisher setup (the collapse reads it where the route reads its own C, as it does a
/// kept one's), the least its C can cost at any outcome (`ShadowRoute::finisher_floor`), and the paired route its C
/// while building reads (`WhileBuilding::Slot`), where that is (`PairedAt`).
struct LeftOut<S> {
    route: Route,
    setup: S,
    finisher_floor: f64,
    paired: Option<PairedAt>,
}

/// Where a left-out route's paired route is (`LeftOut::paired`): kept in the first batch (the route as kept, its
/// finisher slot there), or left out too (its place among the left-out routes).
#[derive(Clone, Copy, Debug)]
pub(super) enum PairedAt {
    First(Route),
    LeftOut(usize),
}

/// A route the budget left out, as a later batch takes it (`build::LaterBatches`): the route (lifts as drafted, its
/// finisher slot 0 where it reads its own C), its finisher setup, and where its paired route is (`PairedAt`).
pub(super) struct LeftOutRoute {
    pub(super) route: Route,
    pub(super) setup: Setup,
    pub(super) paired: Option<PairedAt>,
}

/// The routes `keep_routes` kept, their finisher setups (route k's at its `finisher_slot`, where it has one), and the
/// routes it left out.
struct KeptRoutes<S> {
    routes: Vec<Route>,
    finisher_setups: Vec<S>,
    left_out: Vec<LeftOut<S>>,
}

/// What `keep_routes` counts against the budget: per priced route its finisher setup's entries (None: past any
/// budget; 0 for a route that reads no C of its own), the lift slots the first batch takes (a route through another
/// is left out), the entries left for the finisher setups, and the drafts' count (`WhileBuilding::Slot` indexes them).
#[derive(Clone, Copy)]
struct RouteBudget<'entries> {
    finishers: &'entries [Option<usize>],
    first_lifts: &'entries [bool],
    entry_budget: usize,
    draft_count: usize,
}

/// The memory budget and the route limit (`MAX_ROUTES`, what a stored roll state may hold): the priced routes with the
/// lowest ranks kept (ties in draft order) while their finisher setups' entries fit, at most MAX_ROUTES, each with the
/// paired route its C while building reads (`WhileBuilding::Slot`, kept with it or not at all, so no kept route loses
/// its fallback), in draft order; a route through a lift slot the first batch does not take is left out. Each kept
/// route that reads its own C (`needs_finisher`) takes the next finisher slot, else none (NO_SLOT); a paired route's
/// slot is renumbered to that route's finisher slot (left out: `Skip`). The routes left out are the ones with the
/// highest lower bounds
fn keep_routes<S>(
    priced: Vec<PricedRoute<S>>,
    budget: RouteBudget,
    needs_finisher: impl Fn(&Route) -> bool,
) -> KeptRoutes<S> {
    let RouteBudget { finishers, first_lifts, entry_budget, draft_count } = budget;
    let mut priced_of_draft: Vec<Option<usize>> = vec![None; draft_count];
    for (index, priced_route) in priced.iter().enumerate() {
        priced_of_draft[priced_route.draft_index] = Some(index);
    }
    let mut order: Vec<usize> = (0..priced.len()).collect();
    order.sort_by(|&first, &second| priced[first].rank.total_cmp(&priced[second].rank).then(first.cmp(&second)));
    let (mut kept, mut used, mut count) = (vec![false; priced.len()], 0usize, 0usize);
    for index in order {
        if kept[index] {
            continue;
        }
        let lifts_first = priced[index].route.lifts.iter().all(|&slot| slot == NO_SLOT || first_lifts[slot]);
        // (it and the paired route its C while building reads, where that is priced and not kept yet)
        let paired = match priced[index].route.while_building {
            WhileBuilding::Slot(paired) => priced_of_draft[paired].filter(|&paired| !kept[paired]),
            _ => None,
        };
        let taken = std::iter::once(index).chain(paired);
        let entries = taken.clone().try_fold(0usize, |sum, place| sum.checked_add(finishers[place]?));
        let total = entries.and_then(|entries| used.checked_add(entries)).filter(|&total| total <= entry_budget);
        let added = 1 + usize::from(paired.is_some());
        if let Some(total) = total
            && lifts_first
            && count + added <= MAX_ROUTES
        {
            (used, count) = (total, count + added);
            for place in taken {
                kept[place] = true;
            }
        }
    }
    let dropped = kept.iter().filter(|&&keep| !keep).count();
    if dropped > 0 {
        crate::career::clock::prof("sc_routes_dropped", dropped as f64);
    }
    // (a kept route's finisher slot, a left-out route's place among the left out, each by its draft)
    let (mut finisher_of_draft, mut left_of_draft): (Vec<Option<usize>>, Vec<Option<usize>>) =
        (vec![None; draft_count], vec![None; draft_count]);
    let (mut next_slot, mut next_left) = (0usize, 0usize);
    for (index, priced_route) in priced.iter().enumerate() {
        if !kept[index] {
            left_of_draft[priced_route.draft_index] = Some(next_left);
            next_left += 1;
        } else if needs_finisher(&priced_route.route) {
            finisher_of_draft[priced_route.draft_index] = Some(next_slot);
            next_slot += 1;
        }
    }
    // (each kept route as the first batch has it, by its draft: its finisher slot, its paired route's)
    let route_of_draft: Vec<Option<Route>> = {
        let mut routes = vec![None; draft_count];
        for (index, priced_route) in priced.iter().enumerate() {
            if kept[index] {
                let route = priced_route.route;
                let while_building = match route.while_building {
                    WhileBuilding::Slot(paired) => {
                        finisher_of_draft[paired].map_or(WhileBuilding::Skip, WhileBuilding::Slot)
                    }
                    other => other,
                };
                let finisher_slot = finisher_of_draft[priced_route.draft_index].unwrap_or(NO_SLOT);
                routes[priced_route.draft_index] = Some(Route { finisher_slot, while_building, ..route });
            }
        }
        routes
    };
    let (mut routes, mut finisher_setups, mut left_out) = (vec![], vec![], vec![]);
    for (PricedRoute { route, setup, floors, draft_index, .. }, keep) in priced.into_iter().zip(kept) {
        let reads_own = needs_finisher(&route);
        if !keep {
            let finisher_floor = if reads_own { floors.own } else { floors.building };
            let paired = match route.while_building {
                WhileBuilding::Slot(paired) => match (left_of_draft[paired], route_of_draft[paired]) {
                    (Some(place), _) => Some(PairedAt::LeftOut(place)),
                    (None, Some(paired_route)) => Some(PairedAt::First(paired_route)),
                    (None, None) => None,
                },
                _ => None,
            };
            let finisher_slot = if reads_own { 0 } else { NO_SLOT };
            left_out.push(LeftOut { route: Route { finisher_slot, ..route }, setup, finisher_floor, paired });
            continue;
        }
        let route = route_of_draft[draft_index].expect("a kept route");
        if reads_own {
            finisher_setups.push(setup);
        }
        routes.push(route);
    }
    KeptRoutes { routes, finisher_setups, left_out }
}

/// The odds (as the base prices them, `OddsScales`) a full-veteran route's C must beat for the route to be priced:
/// 1 / (V0 (1 + 1e-9) - `others`), the C whose step brings the route to V0 (for a paired draft, `others` the least
/// among it and the drafts it is the fallback of, `price_drafts`), except where its C while a lifted side builds
/// (`fallback_cost`: the trio's, or the paired draft's at most its odds; infinite with none) brings it under V0 on its
/// own: then none (0), as the route's lower bound (`others` + the least of its own C's floor and `fallback_cost`) can
/// pass whatever the new C's odds
fn finisher_floor(current_value: f64, others: f64, fallback_cost: f64) -> f64 {
    if others + fallback_cost < current_value * (1.0 + 1e-9) {
        0.0
    } else {
        1.0 / (current_value * (1.0 + 1e-9) - others)
    }
}

/// Per Alternative side of a candidate her veteran can lift (not of her uma, a finite step), her place beside each of
/// that side's own two parents, where her veteran and that parent are a legal pair for the side's trainee: the lineages
/// an Alternative side is lifted on (`LiftedLineage`), in candidate and side order, the side's first parent first
fn own_parent_places(
    simulator: &Sim,
    score_base: &ScoreBase,
    extended_pool: &Pool,
    inputs: &RouteInputs,
) -> Vec<LiftedLineage> {
    let data = simulator.data;
    let her_character = data.umas[inputs.trainee].id;
    let mut lineages = vec![];
    for (candidate, sides) in score_base.trio_sides.iter().enumerate() {
        let trainee = score_base.candidates[candidate];
        if data.umas[trainee].id == her_character {
            continue;
        }
        for (side_index, side) in sides.iter().enumerate() {
            let (SideStep::Alternative(step), Some((first_parent, second_parent))) = (side.step, side.parents) else {
                continue;
            };
            if !(step.is_finite() && step > 0.0) {
                continue;
            }
            let slot = TrioSlot { candidate: crate::career::plan::CandidateIndex(candidate), side: side_index };
            for partner in [first_parent, second_parent] {
                let lineage = Lineage { trainee, first_parent: partner, second_parent: inputs.new_veteran };
                if !legal_pair(data, extended_pool, lineage) {
                    continue;
                }
                let Some(her_slot) = simulator.env.setup.her_slot(extended_pool, lineage) else { continue };
                let place = LiftPlace {
                    partner,
                    misses_without_her: her_slot.misses_without_her,
                    target_spark_scales: her_slot.target_spark_scales,
                    miss_by_stars: her_slot.miss_by_stars,
                };
                lineages.push(LiftedLineage { side: slot, place });
            }
        }
    }
    lineages
}

/// A lifted lineage's lift slot (`lift_slots`), where its side's deck is cached: that deck with her veteran and the
/// place's partner as parents, its odds below its side's (`ScoreBase::side_odds`)
fn lift_side(
    simulator: &mut Sim,
    context: &ScoringContext,
    extended_pool: &Pool,
    inputs: &RouteInputs,
    lifted: LiftedLineage,
    legacies_with_her: &[Legacy],
) -> Option<LiftSide> {
    let score_base = context.score_base;
    let LiftedLineage { side, place } = lifted;
    let candidate = side.candidate.0;
    let trainee = score_base.candidates[candidate];
    let deck = score_base.trio_sides[candidate][side.side].deck.as_ref()?;
    let deck_index =
        simulator.evaluator.cache.decks[trainee].iter().position(|cached| same_deck(&cached.deck, deck))?;
    let lineage = Lineage { trainee, first_parent: place.partner, second_parent: inputs.new_veteran };
    // the outfit option her record runs: her exact odds' pick at the lineage's chances where her outfits' grades
    // differ (as `Evaluator::career_wins` picks it), else option 0 with every option's table (one record for all)
    let outfit = if simulator.data.umas[trainee].outfit_aptitudes.is_empty() {
        None
    } else {
        let setup = &simulator.env.setup;
        let Inheritance { chances, carriers } = inherit(&setup.slots(extended_pool, lineage), setup.target_count);
        setup.set_trainee(&mut simulator.env.state, trainee, &chances);
        let options = ExactOptions { all_but_one: false, landed_sets: false };
        Some(setup.exact(&simulator.env.state, trainee, deck, &carriers, options).outfit)
    };
    let record = simulator.evaluator.wins(
        &simulator.env.setup,
        extended_pool,
        lineage,
        ScheduleOutfits::DeckOption(outfit.unwrap_or(0)),
    );
    let veteran = simulator.env.setup.veteran(legacies_with_her, lineage, &record);
    #[cfg(test)]
    LIFTED.with(|lifted| {
        if let Some(lifted) = lifted.borrow_mut().as_mut() {
            let side_veteran = &score_base.trio_sides[candidate][side.side];
            lifted.push(LiftedRecord {
                trainee,
                alternative: matches!(side_veteran.step, SideStep::Alternative(_)),
                side_parents: side_veteran.parents,
                lineage,
                parents: veteran.parents.iter().map(|parent| parent.character_id).collect(),
                partner_chara: legacies_with_her[place.partner].character_id,
                her_chara: legacies_with_her[inputs.new_veteran].character_id,
            });
        }
    });
    let odds_below = score_base.side_odds(side);
    let setup = Setup {
        candidate,
        trainee,
        partner: place.partner,
        misses_without_her: place.misses_without_her,
        target_spark_scales: place.target_spark_scales,
        miss_by_stars: place.miss_by_stars,
        decks: deck_index..deck_index + 1,
        outfit,
        odds_below,
    };
    Some(LiftSide { setup, veteran, side })
}

#[cfg(test)]
mod floor_tests {
    use super::*;
    use crate::career::plan::cont::{Cont, ScoringKnobs};
    use crate::career::plan::step::{AfterHer, CandidateLift, LiftStep, StepCost};

    /// A lifted candidate that still builds, at a step of `step` careers.
    struct Builds {
        step: f64,
    }

    impl StepCost for Builds {
        fn step(&mut self, _: &ScoreBase, _: CandidateLift) -> LiftStep {
            LiftStep { step: self.step, builds: true }
        }
    }

    /// Astra's case (astra-review-V5 #1) through `price_drafts` and `keep_routes`, then V after her: V0 100; the paired
    /// draft (her own full veteran with a side as it is today) costs 70 for that side and its C 40 (odds 0.025), 110 in
    /// all, so it is no route on its own; a lifted route of hers with floor 10 and a new-lineage C of odds 0.01 (100).
    /// Where the lifted side builds at 20, the route is 20 + the paired C's 40 = 60 < 100, so it must be kept with the
    /// paired C as its fallback (before the fix the paired draft's C was floor-pruned, at 1 / (100 - 70), and the route's
    /// C with it, at 1 / (100 - 10), so V after her stayed 100)
    #[test]
    fn a_pruned_paired_drafts_c_stays_the_lifted_routes_fallback() {
        let current_value = 100.0;
        let draft = |lifts: [usize; 2], fixed_cost: f64, step_floor: f64, while_building: WhileBuilding| RouteDraft {
            lifts,
            full_only: true,
            fixed_cost,
            step_floor,
            parents: [RouteParent::Her, RouteParent::Her],
            finisher: None,
            while_building,
        };
        let drafts = [
            draft([NO_SLOT, NO_SLOT], 70.0, 0.0, WhileBuilding::Skip),
            draft([0, NO_SLOT], 0.0, 10.0, WhileBuilding::Slot(0)),
        ];
        let odds = [0.025, 0.01];
        let price = |draft: &RouteDraft, floor: f64| {
            let index = usize::from(draft.lifts[0] == 0);
            if odds[index] > floor {
                DraftPrice::Priced { setup: index, most_odds: odds[index] }
            } else {
                DraftPrice::FloorPruned
            }
        };
        let priced = price_drafts(&drafts, current_value, price);
        let budget = RouteBudget {
            finishers: &[Some(1), Some(1)],
            first_lifts: &[true],
            entry_budget: 10,
            draft_count: drafts.len(),
        };
        let KeptRoutes { routes, finisher_setups, left_out } = keep_routes(priced, budget, |_: &Route| true);
        assert_eq!((finisher_setups, left_out.len()), (vec![0, 1], 0), "both kept, the paired one as the fallback");
        assert_eq!(routes[1].while_building, WhileBuilding::Slot(0));
        let base = ScoreBase {
            candidates: &[crate::uma::UmaIndex(0)],
            step_costs: &[50.0],
            current_value,
            current_odds: &[1e-3],
            continuations: &[Cont::default()],
            step_caps: &[],
            trios: &[],
            trio_sides: &[],
            finisher_umas: &[],
            knobs: ScoringKnobs::default(),
            odds_scales: crate::career::plan::cont::OddsScales::ONE,
        };
        let after =
            AfterHer { completes_goal: false, full_mask: 1, routes: &routes, lift_slot_count: 1, finisher_umas: &[] };
        let lift = [CandidateLift { slot: 0, candidate: 0, probability: 2e-3 }];
        let saving = base.v_after(&mut Builds { step: 20.0 }, &mut vec![], &lift, &odds, 1, &after);
        assert!((saving - 40.0).abs() < 1e-9, "V after her 20 + 40 = 60: saving {saving}");
    }

    /// Astra's case (astra-review-V4 #2): V0 100, the lifted side's floor 10, the paired route's C 40 while it builds, a
    /// new-lineage C of odds 0.01 (cost 100). The route is worth 10 + 40 = 50 < 100 through its fallback, so that C must
    /// pass; the floor without the fallback, 1 / (100 (1 + 1e-9) - 10) = 0.0111, rejected it (today's code before the fix)
    #[test]
    fn a_while_building_fallback_keeps_a_cheap_route_priced() {
        let (current_value, others, new_c_odds) = (100.0, 10.0, 0.01);
        let without_fallback = 1.0 / (current_value * (1.0 + 1e-9) - others);
        assert!(new_c_odds < without_fallback, "the old floor rejects this C");
        assert!(new_c_odds > finisher_floor(current_value, others, 40.0), "the fallback keeps it");
        assert_eq!(finisher_floor(current_value, others, f64::INFINITY), without_fallback);
        assert_eq!(finisher_floor(current_value, others, 95.0), without_fallback, "a fallback above V0 adds nothing");
    }
}

/// A lift slot as `lift_side` built it, recorded under test (`LIFTED`): the side's trainee, whether the side is an
/// Alternative one and its parents, the lifted lineage, its veteran's parents' characters, and the partner's and her
/// veteran's.
#[cfg(test)]
#[derive(Clone)]
pub(crate) struct LiftedRecord {
    pub(crate) trainee: crate::uma::UmaIndex,
    pub(crate) alternative: bool,
    pub(crate) side_parents: Option<(usize, usize)>,
    pub(crate) lineage: Lineage,
    pub(crate) parents: Vec<u32>,
    pub(crate) partner_chara: u32,
    pub(crate) her_chara: u32,
}

#[cfg(test)]
thread_local! {
    /// The lift slots built on this thread while set (`lift_side`).
    pub(crate) static LIFTED: std::cell::RefCell<Option<Vec<LiftedRecord>>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
mod collapse_tests {
    use super::*;
    use crate::career::plan::cont::OddsScales;

    /// A route's setup as the test names it: its id in `partner`.
    fn setup(id: usize) -> Setup {
        Setup {
            candidate: 0,
            trainee: crate::uma::UmaIndex(0),
            partner: id,
            misses_without_her: vec![],
            target_spark_scales: vec![],
            miss_by_stars: [1.0; 4],
            decks: 0..0,
            outfit: None,
            odds_below: 0.0,
        }
    }

    /// Astra (astra-iv2.out 1, astra-iv3.out 5): which setups the collapse reads must not depend on which routes the
    /// budget keeps. Two lift slots (setups 10 and 11), slot 0 grinding, slot 1 never. Five priced routes: on slot 0
    /// two that read their own C (setups 1 and 2); on slot 1, never reading their own C, one taking the trio's C while
    /// building (setup 3), one the same but dearer, so dominated (setup 4, `drop_dominated` drops it), and one whose C
    /// while building is the second reader's (setup 5, kept with that reader or not at all). Every route is kept at a
    /// budget of 10 entries; at 1 entry the never-reading setup 5 is left out with its reader; with slot 1 outside the
    /// first batch, the never-reading setups 3 and 5 are left out by the lift. Each time the setups the collapse reads
    /// (the finisher slots, the first batch's lift slots, `collapse_setups`) are 1, 2, 10 and 11: no never-reading
    /// route's setup, kept or left out, and no dominated one's
    #[test]
    fn excluding_a_route_moves_no_setup_of_the_collapse() {
        let _ = OddsScales::ONE;
        let route = |lift: usize, full_only: bool, fixed_cost: f64, while_building: WhileBuilding| Route {
            lifts: [lift, NO_SLOT],
            full_only,
            fixed_cost,
            finisher_slot: 0,
            while_building,
        };
        let floors = FinisherFloors { own: 5.0, building: 10.0 };
        let fixed = WhileBuilding::Fixed(10.0);
        let priced = |id: usize, route: Route, rank: f64| PricedRoute {
            route,
            setup: setup(id),
            rank,
            draft_index: id - 1,
            floors,
        };
        let drafted = || {
            vec![
                priced(1, route(0, false, 1.0, fixed), 0.5),
                priced(2, route(0, true, 1.0, fixed), 2.0),
                priced(3, route(1, false, 1.0, fixed), 3.0),
                priced(4, route(1, false, 2.0, fixed), 4.0),
                priced(5, route(1, false, 1.0, WhileBuilding::Slot(1)), 1.5),
            ]
        };
        let reads = [LiftRead { least_step: 1.0, grinds: true }, LiftRead { least_step: 1.0, grinds: false }];
        let undominated = drop_dominated(drafted(), &reads, 100.0);
        let ids_left: Vec<usize> = undominated.iter().map(|priced_route| priced_route.setup.partner).collect();
        assert_eq!(ids_left, vec![1, 2, 3, 5], "the dearer never-grinding route is dominated");
        let needs_finisher = |route: &Route| reads[route.lifts[0]].grinds;
        let collapse_ids = |entry_budget: usize, first_lifts: [bool; 2]| {
            let priced = drop_dominated(drafted(), &reads, 100.0);
            let finishers: Vec<Option<usize>> =
                priced.iter().map(|priced_route| Some(usize::from(needs_finisher(&priced_route.route)))).collect();
            let budget = RouteBudget { finishers: &finishers, first_lifts: &first_lifts, entry_budget, draft_count: 5 };
            let kept = keep_routes(priced, budget, needs_finisher);
            let lifts = LiftSetups {
                setups: vec![setup(10), setup(11)],
                first: first_lifts.to_vec(),
                collapse: vec![true, true],
            };
            let route_set = RouteSet::of(lifts, kept);
            let first_lift_setups = (0..2)
                .filter(|&slot| route_set.first_lifts[slot] && route_set.collapse_lifts[slot])
                .map(|slot| &route_set.lift_setups[slot]);
            let mut ids: Vec<usize> = route_set
                .finisher_setups
                .iter()
                .chain(first_lift_setups)
                .chain(route_set.collapse_setups())
                .map(|setup| setup.partner)
                .collect();
            ids.sort_unstable();
            let mut left: Vec<usize> = route_set.left_out.iter().map(|left| left.setup.partner).collect();
            left.sort_unstable();
            (ids, left)
        };
        let (all_kept, none_left) = collapse_ids(10, [true, true]);
        let (one_entry, entry_left) = collapse_ids(1, [true, true]);
        let (first_only, lift_left) = collapse_ids(10, [true, false]);
        assert_eq!(none_left, Vec::<usize>::new());
        assert_eq!(entry_left, vec![2, 5], "the entry budget leaves the never-reading setup 5 out with its reader");
        assert_eq!(lift_left, vec![3, 5], "a lift slot outside the first batch leaves its never-reading routes out");
        assert_eq!(all_kept, vec![1, 2, 10, 11]);
        assert_eq!(one_entry, all_kept, "the same setups whatever the entry budget keeps");
        assert_eq!(first_only, all_kept, "the same setups whatever lift slots the first batch takes");
    }
}

#[cfg(test)]
mod lifted_tests {
    use super::{LIFTED, LiftedRecord};

    /// Astra (astra-iv2.out 5): an Alternative side's lift slots are built on its own lineage. On the val/plan p10 pool
    /// Agnes Digital's Alternative side on legacies 6 and 7 is lifted by a scored career's veteran beside 6 and beside 7:
    /// each lifted lineage is Agnes Digital on (that parent, her veteran), the scored veteran in the second parent's
    /// place (so C's second grandparent slot through it), and the lifted veteran's recorded parents are that parent's and
    /// her veteran's characters, in that order
    #[test]
    fn an_alternative_side_is_lifted_beside_its_own_parents() {
        let folder = concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/");
        let read = |file: &str| std::fs::read_to_string(format!("{folder}{file}")).unwrap();
        let data = crate::data::load_str(&read("data.json"));
        let skills: crate::skills::SkillsRaw = serde_json::from_str(&read("web_skills.json")).unwrap();
        let cards: crate::deck::CardsRaw = serde_json::from_str(&read("web_cards.json")).unwrap();
        let mut job: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../val/plan/p10.json")).unwrap(),
        )
        .unwrap();
        let events: serde_json::Value = serde_json::from_str(&read("card_events.json")).unwrap();
        crate::deck::sources::apply_events_db(&mut job, &skills, &events);
        job["no_reports"] = serde_json::json!(true);
        let legacy_count = job["legacies"].as_array().unwrap().len();
        let agnes = data.umas.iter().position(|uma| uma.name == "Agnes Digital").map(crate::uma::UmaIndex).unwrap();
        LIFTED.with(|lifted| *lifted.borrow_mut() = Some(vec![]));
        let mut kept = crate::career::state::KeptState::default();
        let out = crate::career::run_career_in(&data, &skills, &cards, &job, &mut kept);
        let records = LIFTED.with(|lifted| lifted.borrow_mut().take().unwrap());
        assert!(out["next"]["v0"].is_number(), "the plan ran: {}", out["next"]["error"]);
        let side: Vec<&LiftedRecord> = records
            .iter()
            .filter(|record| {
                record.alternative && record.trainee == agnes && matches!(record.side_parents, Some((6, 7) | (7, 6)))
            })
            .collect();
        assert!(!side.is_empty(), "Agnes Digital's Alternative side on 6 and 7 is lifted");
        for &partner in &[6, 7] {
            assert!(side.iter().any(|record| record.lineage.first_parent == partner), "beside legacy {partner}");
        }
        for record in side {
            assert_eq!(record.lineage.trainee, agnes);
            assert!([6, 7].contains(&record.lineage.first_parent), "one of the side's own parents");
            assert_eq!(record.lineage.second_parent, legacy_count, "her veteran, last of the pool, second");
            assert_eq!(record.parents, vec![record.partner_chara, record.her_chara], "the lifted veteran's parents");
        }
    }
}

#[cfg(test)]
mod alternative_tests {
    use super::*;
    use crate::career::plan::CandidateIndex;
    use crate::career::plan::cont::{Cont, OddsScales, ScoringKnobs, Trio, TrioSideVeteran, trio_v};
    use crate::career::plan::step::{AfterHer, CandidateLift, EstimateBuffers, StepEstimate};
    use crate::uma::UmaIndex;

    /// A trio of candidate 0's side `side` (her continuation `continuation`, her odds now `odds_now`) and candidate 1's
    /// real full veteran, C at `finisher` careers, and her sides `sides`; with the routes built from its one lift slot
    /// on that side (`route_drafts`, `price_drafts`, `keep_routes`, each route's C on the lifted lineage at
    /// `own_finisher_odds`), V after her where that slot lifts her to `lifted_odds` (`v_after` at the estimate's steps):
    /// (V, routes kept, saving)
    fn lifted(
        sides: Vec<TrioSideVeteran>,
        side: usize,
        continuation: &Cont,
        odds_now: f64,
        lift: Lift,
    ) -> (f64, usize, f64) {
        let Lift { finisher, own_finisher_odds, lifted_odds } = lift;
        let trio_sides = vec![sides, vec![real_side()]];
        let slot = TrioSlot { candidate: CandidateIndex(0), side };
        let trios = [Trio {
            sides: [slot, TrioSlot { candidate: CandidateIndex(1), side: 0 }],
            finisher: UmaIndex(2),
            finisher_cost: finisher,
        }];
        let continuations = [continuation.clone(), Cont::default()];
        let step_costs = [continuation.cost(odds_now), 0.0];
        let mut base = ScoreBase {
            candidates: &[UmaIndex(0), UmaIndex(1)],
            step_costs: &step_costs,
            current_value: f64::INFINITY,
            current_odds: &[odds_now, 1.0],
            continuations: &continuations,
            step_caps: &[],
            trios: &trios,
            trio_sides: &trio_sides,
            finisher_umas: &[],
            knobs: ScoringKnobs::default(),
            odds_scales: OddsScales::ONE,
        };
        base.current_value = trio_v(&step_costs, &trio_sides, &trios);
        let setup = Setup {
            candidate: 0,
            trainee: UmaIndex(0),
            partner: 1,
            misses_without_her: vec![],
            target_spark_scales: vec![],
            miss_by_stars: [1.0; 4],
            decks: 0..0,
            outfit: None,
            odds_below: base.side_odds(slot),
        };
        let lifts = [LiftSide { setup, veteran: Legacy::default(), side: slot }];
        // (the least step V after her reads of the slot: her step at the lifted odds, the one outcome lifting it)
        let reads = [LiftRead { least_step: base.step_plan(0, lifted_odds).0, grinds: false }];
        let drafts = route_drafts(&base, DraftLifts { lifts: &lifts, reads: &reads }, None);
        let priced = price_drafts(&drafts, base.current_value, |_: &RouteDraft, _: f64| DraftPrice::Priced {
            setup: (),
            most_odds: own_finisher_odds,
        });
        let finishers = vec![Some(1); priced.len()];
        let budget =
            RouteBudget { finishers: &finishers, first_lifts: &[true], entry_budget: 100, draft_count: drafts.len() };
        let needs_finisher = |route: &Route| !route.lifts.iter().any(|&slot| slot != NO_SLOT && !reads[slot].grinds);
        let KeptRoutes { routes, .. } = keep_routes(priced, budget, needs_finisher);
        assert!(routes.iter().all(|route| route.finisher_slot == NO_SLOT), "a slot that never grinds reads no own C");
        let after =
            AfterHer { completes_goal: false, full_mask: 1, routes: &routes, lift_slot_count: 1, finisher_umas: &[] };
        let lift = [CandidateLift { slot: 0, candidate: 0, probability: lifted_odds }];
        let mut estimate = StepEstimate::new(1, EstimateBuffers::default());
        let saving = base.v_after(&mut estimate, &mut vec![], &lift, &[own_finisher_odds], 0, &after);
        (base.current_value, routes.len(), saving)
    }

    /// The trio's C and the lift (`lifted`).
    #[derive(Clone, Copy)]
    struct Lift {
        finisher: f64,
        own_finisher_odds: f64,
        lifted_odds: f64,
    }

    fn real_side() -> TrioSideVeteran {
        TrioSideVeteran {
            step: SideStep::Existing(0),
            legacy: Legacy::default(),
            odds: 0.0,
            parents: None,
            deck: None,
            proven: true,
        }
    }

    fn fresh_side(step: SideStep, odds: f64) -> TrioSideVeteran {
        TrioSideVeteran { step, legacy: Legacy::default(), odds, parents: Some((1, 3)), deck: None, proven: true }
    }

    /// Astra's acceptance test for Alternative sides (astra-mono.out, 4): V's trio takes candidate 0's Alternative side
    /// (another of her careers, at odds q below her odds now p0, where she builds), and a builder's veteran lifts it on
    /// its own lineage by the step her continuation was fitted to close, to odds still below p0 and still building. The
    /// route replacing that side with the lifted career keeps the trio's C while it builds (the C on the lifted lineage
    /// is cheaper here, so taking it would show), and saves 1 / lam: the builder, lifting with chance lam, scores
    /// lam (V - V after her) - 1 = 0. The same side as her Current one (her odds now q) gives the same V and the same
    /// saving to the bit: the price does not depend on the label. Today's Current-only routes give no route here
    #[test]
    fn an_alternative_sides_own_builder_scores_zero() {
        let (lam, phi, pinf, p0) = (0.3, 0.1, 0.002, 3e-4);
        let continuation = Cont::new(lam, phi, pinf, p0);
        let q = p0 / 3.0;
        let lifted_odds = pinf * ((q / pinf).ln() * (1.0 - phi)).exp();
        assert!(q < lifted_odds && lifted_odds < p0, "the lift lies between her side's odds and her odds now");
        assert!(continuation.plan(q).building_careers > 0.0, "she builds at the side's odds");
        assert!(continuation.plan(lifted_odds).building_careers > 0.0, "and still builds when lifted");
        let finisher = 500.0;
        let lift = Lift { finisher, own_finisher_odds: 2.0 / finisher, lifted_odds };
        let alternative =
            vec![fresh_side(SideStep::Current, p0), fresh_side(SideStep::Alternative(continuation.cost(q)), q)];
        let (v, routes, saving) = lifted(alternative, 1, &continuation, p0, lift);
        assert_eq!(v, continuation.cost(q) + finisher);
        assert_eq!(routes, 1, "one route: the Alternative side lifted, the real full veteran as it is");
        assert!((lam * saving - 1.0).abs() < 1e-9, "the builder's score {}", lam * saving - 1.0);
        let current = vec![fresh_side(SideStep::Current, q)];
        let (relabelled_v, relabelled_routes, relabelled_saving) = lifted(current, 0, &continuation, q, lift);
        assert_eq!((relabelled_v.to_bits(), relabelled_routes), (v.to_bits(), routes), "V as her Current side");
        assert_eq!(relabelled_saving.to_bits(), saving.to_bits(), "the saving as her Current side");
    }
}
