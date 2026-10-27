//! The routes through a scored career's veteran (`routes`): each trio taken with her in it, on its own lineage.
use crate::career::inherit::{ExactOptions, Inheritance, inherit};
use crate::career::plan::ScoringContext;
use crate::career::plan::cont::{SideStep, TrioSlot};
use crate::career::plan::kept::same_deck;
use crate::career::plan::score::build::{Setup, setup_entries};
use crate::career::plan::score::{FinisherChoice, FinisherParents, best_finisher};
use crate::career::plan::search::legal_pair;
use crate::career::plan::step::{MAX_ROUTES, NO_SLOT, Route, WhileBuilding};
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

/// What `routes` builds: the lift setups (one per partner setup with a lifted side, the slots a route's lifts read),
/// the finisher setups (one per route) and the routes, and how many routes the memory budget left out.
pub(super) struct RouteSet {
    pub(super) lift_setups: Vec<Setup>,
    pub(super) finisher_setups: Vec<Setup>,
    pub(super) routes: Vec<Route>,
    pub(super) dropped: usize,
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
    /// A lower bound on the steps it takes (their lift slots' floors).
    step_floor: f64,
    parents: [RouteParent; 2],
    /// Its C: the trio's (None: one of her own full veteran's routes, its C searched, `best_finisher`).
    finisher: Option<crate::uma::UmaIndex>,
    /// Its C while a lifted side builds (`Route::while_building`); `Slot` holds the paired draft's index here (a paired
    /// draft: one so named, its C a fallback of the drafts naming it, `price_drafts`).
    while_building: WhileBuilding,
}

/// A lift slot: the setup, the lifted side's veteran, and a lower bound on its candidate's step at every outcome.
struct LiftSide {
    setup: Setup,
    veteran: Legacy,
    step_floor: f64,
}

/// The routes of a career whose veteran `inputs.her` is the last of `extended_pool` (`RouteSet`): per trio side on a
/// candidate's best career now (`Current`) and per lift slot of hers, the trio with that side lifted, the other at the
/// base; per trio with both sides so, both lifted (each lift slot pair); and where she can land every target, her own
/// full veteran (the career's own lineage, her stars the outcome's) paired with every side of every other uma's
/// candidate at the base, and with each lifted side.
///
/// A lift slot belongs to one partner setup and reads one career: her candidate's best career's deck on the lifted
/// lineage (her veteran and that partner as parents), in the one outfit option its record runs (where her outfits'
/// grades differ, the option her exact odds pick at her lineage's chances, `CareerSetup::exact`; else every option's
/// table, the record being the same), its record (G1 wins and possible pinks, `Evaluator::wins` in that option) the
/// lifted side's veteran's (`CareerSetup::veteran`). So a route's step, its odds and its record describe one career.
/// Each route's C is held fixed: a trio's route takes the trio's own C (the fit's best on its lineages; the route is
/// left out where she is no legal C on the new parents), one of her own full veteran's the allowed uma best on its two
/// parents with her carrying every target at 3 stars (`best_finisher`, the most any outcome gives her, so its ceiling
/// bounds every outcome's); her slots in C's lineage (her as C's parent or grandparent, `CareerSetup::her_slots`) are
/// the finisher setup, so the fold values that C at each outcome's stars. Where a lifted side still builds at its lift's
/// odds, its later builders replace the parents that put her in C's lineage, so the route's C is V0's there
/// (`Route::while_building`: a trio's route the trio's own C; her own full veteran's route with a lifted side the route
/// with that side as it is today, where that route is priced).
///
/// A route whose lower bound (its steps' floors, `ScoreBase::step_floor`, + its fixed sides + the least of 1 / the most
/// its C's odds can be, `Setup::odds_bound`, and its C while a side builds) reaches V0 (with 1e-9 V0 to spare for
/// rounding) can never be below V after her and is left out, but a paired route's C stays its dependents' fallback
/// (`price_drafts`: these omissions lose nothing). Past the memory budget or `MAX_ROUTES`, the routes with the highest
/// lower bounds are left out too (`dropped`, `keep_routes`): V after her is then at least the model's (fewer routes),
/// the saving at most it
pub(super) fn routes(
    simulator: &mut Sim,
    context: &ScoringContext,
    extended_pool: &mut Pool,
    inputs: &RouteInputs,
    score_buffer: &mut Vec<f64>,
) -> RouteSet {
    let score_base = context.score_base;
    let data = simulator.data;
    let current_value = score_base.current_value;
    let character = |candidate: usize| data.umas[score_base.candidates[candidate]].id;
    let her_character = data.umas[inputs.trainee].id;
    let is_current = |slot: TrioSlot| score_base.side_step(slot) == SideStep::Current;
    // her as a route's C sees her: every target at 3 stars, the most any outcome gives (so the C chosen on it, and
    // its ceiling, bound C's odds at every outcome: C's odds only rise with her stars); her slots take the outcome's
    let her_at_most = Legacy {
        sparks: simulator.env.setup.skill_job.targets.iter().map(|target| (target.group, 3)).collect(),
        mix: None,
        ..inputs.her.clone()
    };
    let mut legacies_with_her = context.legacies.to_vec();
    legacies_with_her.push(her_at_most.clone());
    let wins_started = crate::career::clock::now_ms();
    let lifts: Vec<LiftSide> = inputs
        .partner_setups
        .iter()
        .filter_map(|partner_setup| {
            lift_side(simulator, context, extended_pool, inputs, partner_setup, &legacies_with_her)
        })
        .collect();
    let lifts_of = |candidate: usize| {
        lifts.iter().enumerate().filter(move |(_, lift)| lift.setup.candidate == candidate).map(|(slot, _)| slot)
    };
    let mut drafts: Vec<RouteDraft> = vec![];
    for trio in score_base.trios {
        for k in 0..2 {
            let (slot, other) = (trio.sides[k], trio.sides[1 - k]);
            if !is_current(slot) {
                continue;
            }
            for lift in lifts_of(slot.candidate.0) {
                drafts.push(RouteDraft {
                    lifts: [lift, NO_SLOT],
                    full_only: false,
                    fixed_cost: score_base.side_cost(other),
                    step_floor: lifts[lift].step_floor,
                    parents: [RouteParent::Lifted(lift), RouteParent::Side(other)],
                    finisher: Some(trio.finisher),
                    while_building: WhileBuilding::Fixed(trio.finisher_cost),
                });
            }
        }
        let [first, second] = trio.sides;
        if is_current(first) && is_current(second) {
            for first_lift in lifts_of(first.candidate.0) {
                for second_lift in lifts_of(second.candidate.0) {
                    drafts.push(RouteDraft {
                        lifts: [first_lift, second_lift],
                        full_only: false,
                        fixed_cost: 0.0,
                        step_floor: lifts[first_lift].step_floor + lifts[second_lift].step_floor,
                        parents: [RouteParent::Lifted(first_lift), RouteParent::Lifted(second_lift)],
                        finisher: Some(trio.finisher),
                        while_building: WhileBuilding::Fixed(trio.finisher_cost),
                    });
                }
            }
        }
    }
    if inputs.full_possible {
        for candidate in (0..score_base.candidates.len()).filter(|&candidate| character(candidate) != her_character) {
            // (her n/n with this candidate's best career now as it is: the lifted routes' C while that side builds)
            let mut current_draft = None;
            for side in 0..score_base.trio_sides[candidate].len() {
                let slot = TrioSlot { candidate: crate::career::plan::CandidateIndex(candidate), side };
                if score_base.side_step(slot) == SideStep::Current {
                    current_draft = Some(drafts.len());
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
            for lift in lifts_of(candidate) {
                drafts.push(RouteDraft {
                    lifts: [lift, NO_SLOT],
                    full_only: true,
                    fixed_cost: 0.0,
                    step_floor: lifts[lift].step_floor,
                    parents: [RouteParent::Her, RouteParent::Lifted(lift)],
                    finisher: None,
                    while_building: current_draft.map_or(WhileBuilding::Skip, WhileBuilding::Slot),
                });
            }
        }
    }
    crate::career::clock::prof("sc_route_wins_ms", crate::career::clock::now_ms() - wins_started);
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
    let priced = price_drafts(&drafts, current_value, price);
    let entries: Vec<Option<usize>> = priced
        .iter()
        .map(|priced_route| setup_entries(simulator, &priced_route.setup, inputs.leaf_count, inputs.table_size))
        .collect();
    let KeptRoutes { routes, finisher_setups, dropped } =
        keep_routes(priced, &entries, inputs.entry_budget, drafts.len());
    RouteSet { lift_setups: lifts.into_iter().map(|lift| lift.setup).collect(), finisher_setups, routes, dropped }
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
    let mut held: Vec<Option<(Route, S)>> = (0..drafts.len()).map(|_| None).collect();
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
            priced.push(PricedRoute { route, setup, rank: lower_bound, draft_index });
        } else if dependents_others[draft_index] < current_value {
            held[draft_index] = Some((route, setup));
        }
    }
    for (draft_index, held_route) in held.into_iter().enumerate() {
        if let Some((route, setup)) = held_route
            && needed_rank[draft_index].is_finite()
        {
            priced.push(PricedRoute { route, setup, rank: needed_rank[draft_index], draft_index });
        }
    }
    priced.sort_by_key(|priced_route| priced_route.draft_index);
    priced
}

/// The routes `keep_routes` kept, their finisher setups (route k's at its `finisher_slot`) and how many it left out.
struct KeptRoutes<S> {
    routes: Vec<Route>,
    finisher_setups: Vec<S>,
    dropped: usize,
}

/// The memory budget and the route limit (`MAX_ROUTES`, what a stored roll state may hold): the priced routes with the
/// lowest ranks kept (ties in draft order) while their `entries` (one per priced route; None: past any budget) fit
/// `entry_budget`, at most MAX_ROUTES, in draft order; each route's finisher slot its place, and a paired route's slot
/// (`WhileBuilding::Slot`, a draft index of `draft_count`) renumbered to its route's place (left out: `Skip`)
fn keep_routes<S>(
    priced: Vec<PricedRoute<S>>,
    entries: &[Option<usize>],
    entry_budget: usize,
    draft_count: usize,
) -> KeptRoutes<S> {
    let mut order: Vec<usize> = (0..priced.len()).collect();
    order.sort_by(|&first, &second| priced[first].rank.total_cmp(&priced[second].rank).then(first.cmp(&second)));
    let (mut kept, mut used, mut count) = (vec![false; priced.len()], 0usize, 0usize);
    for index in order {
        if count == MAX_ROUTES {
            break;
        }
        if let Some(total) = entries[index].and_then(|entry| used.checked_add(entry))
            && total <= entry_budget
        {
            (used, count) = (total, count + 1);
            kept[index] = true;
        }
    }
    let dropped = kept.iter().filter(|&&keep| !keep).count();
    let kept_routes: Vec<PricedRoute<S>> =
        priced.into_iter().zip(kept).filter_map(|(priced_route, keep)| keep.then_some(priced_route)).collect();
    // (a paired draft's route by its final place: its finisher slot)
    let mut slot_of_draft: Vec<Option<usize>> = vec![None; draft_count];
    for (slot, priced_route) in kept_routes.iter().enumerate() {
        slot_of_draft[priced_route.draft_index] = Some(slot);
    }
    let (mut routes, mut finisher_setups) = (vec![], vec![]);
    for PricedRoute { route, setup, .. } in kept_routes {
        let while_building = match route.while_building {
            WhileBuilding::Slot(paired) => slot_of_draft[paired].map_or(WhileBuilding::Skip, WhileBuilding::Slot),
            other => other,
        };
        routes.push(Route { finisher_slot: finisher_setups.len(), while_building, ..route });
        finisher_setups.push(setup);
    }
    KeptRoutes { routes, finisher_setups, dropped }
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

/// A partner setup's lift slot (`routes`), where her candidate's best career's deck is cached and her best career now
/// is a trio side
fn lift_side(
    simulator: &mut Sim,
    context: &ScoringContext,
    extended_pool: &Pool,
    inputs: &RouteInputs,
    partner_setup: &Setup,
    legacies_with_her: &[Legacy],
) -> Option<LiftSide> {
    let score_base = context.score_base;
    let candidate = partner_setup.candidate;
    let trainee = score_base.candidates[candidate];
    let current = score_base.trio_sides[candidate].iter().find(|veteran| veteran.step == SideStep::Current)?;
    let deck = current.deck.as_ref()?;
    let deck_index =
        simulator.evaluator.cache.decks[trainee].iter().position(|cached| same_deck(&cached.deck, deck))?;
    let lineage = Lineage { trainee, first_parent: partner_setup.partner, second_parent: inputs.new_veteran };
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
    let setup = Setup {
        candidate,
        trainee,
        partner: partner_setup.partner,
        misses_without_her: partner_setup.misses_without_her.clone(),
        target_spark_scales: partner_setup.target_spark_scales.clone(),
        miss_by_stars: partner_setup.miss_by_stars,
        decks: deck_index..deck_index + 1,
        outfit,
    };
    let most_odds = setup.odds_bound(simulator).max(score_base.current_odds[candidate]);
    Some(LiftSide { step_floor: score_base.step_floor(candidate, most_odds), setup, veteran })
}

#[cfg(test)]
mod floor_tests {
    use super::*;
    use crate::career::plan::cont::{Cont, ScoreBase, ScoringKnobs};
    use crate::career::plan::step::{AfterHer, CandidateLift, LiftStep, StepCost};

    /// A lifted candidate that still builds, at a step of `step` careers.
    struct Builds {
        step: f64,
    }

    impl StepCost for Builds {
        fn step(&mut self, _: &ScoreBase, _: CandidateLift) -> LiftStep {
            LiftStep { step: self.step, builds: Some(true) }
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
        let KeptRoutes { routes, finisher_setups, dropped } =
            keep_routes(priced, &[Some(1), Some(1)], 10, drafts.len());
        assert_eq!((finisher_setups, dropped), (vec![0, 1], 0), "both kept, the paired one as the fallback");
        assert_eq!(routes[1].while_building, WhileBuilding::Slot(0));
        let base = ScoreBase {
            candidates: &[crate::uma::UmaIndex(0)],
            step_costs: &[50.0],
            current_value,
            current_odds: &[1e-3],
            certified: &[true],
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
