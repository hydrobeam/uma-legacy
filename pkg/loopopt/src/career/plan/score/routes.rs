//! The routes through a scored career's veteran (`routes`): each trio taken with her in it, on its own lineage.
use crate::career::inherit::{ExactOptions, Inheritance, inherit};
use crate::career::plan::ScoringContext;
use crate::career::plan::cont::{SideStep, TrioSlot};
use crate::career::plan::kept::same_deck;
use crate::career::plan::score::build::{Setup, setup_entries};
use crate::career::plan::score::{FinisherParents, best_finisher};
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
    /// Its C while a lifted side builds (`Route::while_building`); `Slot` holds the paired draft's index here.
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
/// with that side as it is today, where that route is kept).
///
/// A route whose lower bound (its steps' floors, `ScoreBase::step_floor`, + its fixed sides + the least of 1 / the most
/// its C's odds can be, `Setup::odds_bound`, and its C while a side builds) reaches V0 (with 1e-9 V0 to spare for rounding) can never be below V after her and
/// is left out. Past `finisher_budget` routes, those with the highest lower bounds are left out too (`dropped`): V
/// after her is then at least the model's (fewer routes), the saving at most it
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
    // (route, finisher setup, lower bound, draft index); per draft priced, the most its C's odds can be
    let mut priced: Vec<(Route, Setup, f64, usize)> = vec![];
    let mut most_odds_of_draft: Vec<Option<f64>> = vec![None; drafts.len()];
    for (draft_index, draft) in drafts.into_iter().enumerate() {
        let others = draft.step_floor + draft.fixed_cost;
        if !(others < current_value) {
            continue;
        }
        // C's parents in the pool (fresh and lifted sides pushed, popped below) and her slots there
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
        let finisher = if let Some(trio_finisher) = draft.finisher {
            let lineage = Lineage { trainee: trio_finisher, first_parent: places[0], second_parent: places[1] };
            legal_pair(data, extended_pool, lineage).then_some(trio_finisher)
        } else {
            // (a C at most 1 / (V0 (1 + 1e-9) - its others), as the base prices odds (`odds_scale`), is never below V0:
            // the route need not be priced)
            let parents = FinisherParents { first_parent: places[0], second_parent: places[1], excluded };
            let floor = 1.0 / (current_value * (1.0 + 1e-9) - others) / score_base.odds_scale;
            best_finisher(simulator, score_base.finisher_umas, extended_pool, parents, floor, score_buffer)
                .map(|(_, trainee)| trainee)
        };
        if let Some(finisher_trainee) = finisher {
            let lineage = Lineage { trainee: finisher_trainee, first_parent: places[0], second_parent: places[1] };
            if let Some(slot) = simulator.env.setup.her_slots(extended_pool, lineage, &positions) {
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
                let most_odds = setup.odds_bound(simulator);
                let own_floor = if most_odds > 0.0 { 1.0 / (most_odds * score_base.odds_scale) } else { f64::INFINITY };
                // (while a lifted side builds, its C is V0's: the trio's, or the paired route's, at most its odds)
                let while_building = match draft.while_building {
                    WhileBuilding::Slot(paired) if most_odds_of_draft[paired].is_none() => WhileBuilding::Skip,
                    other => other,
                };
                let building_floor = match while_building {
                    WhileBuilding::Fixed(cost) => cost,
                    WhileBuilding::Slot(paired) => {
                        most_odds_of_draft[paired].map_or(f64::INFINITY, |odds| 1.0 / (odds * score_base.odds_scale))
                    }
                    WhileBuilding::Skip => f64::INFINITY,
                };
                let lower_bound = others + own_floor.min(building_floor);
                if lower_bound < current_value * (1.0 + 1e-9) {
                    if most_odds > 0.0 {
                        most_odds_of_draft[draft_index] = Some(most_odds);
                    }
                    let route = Route {
                        lifts: draft.lifts,
                        full_only: draft.full_only,
                        fixed_cost: draft.fixed_cost,
                        finisher_slot: 0,
                        while_building,
                    };
                    priced.push((route, setup, lower_bound, draft_index));
                }
            }
        }
        while extended_pool.veterans.len() > pool_size {
            extended_pool.pop_veteran();
        }
    }
    // the memory budget and the route limit (`MAX_ROUTES`, what a stored roll state may hold): the routes with the
    // lowest lower bounds kept (ties in route order) while their entries fit, at most MAX_ROUTES, in route order
    let entries: Vec<Option<usize>> = priced
        .iter()
        .map(|(_, setup, _, _)| setup_entries(simulator, setup, inputs.leaf_count, inputs.table_size))
        .collect();
    let mut order: Vec<usize> = (0..priced.len()).collect();
    order.sort_by(|&first, &second| priced[first].2.total_cmp(&priced[second].2).then(first.cmp(&second)));
    let (mut kept, mut used, mut count) = (vec![false; priced.len()], 0usize, 0usize);
    for index in order {
        if count == MAX_ROUTES {
            break;
        }
        if let Some(total) = entries[index].and_then(|entry| used.checked_add(entry))
            && total <= inputs.entry_budget
        {
            (used, count) = (total, count + 1);
            kept[index] = true;
        }
    }
    let dropped = kept.iter().filter(|&&keep| !keep).count();
    let mut index = 0;
    priced.retain(|_| {
        index += 1;
        kept[index - 1]
    });
    // (a paired draft's route by its final place: its finisher slot; a paired route left out leaves none)
    let mut slot_of_draft: Vec<Option<usize>> = vec![None; most_odds_of_draft.len()];
    for (slot, (_, _, _, draft_index)) in priced.iter().enumerate() {
        slot_of_draft[*draft_index] = Some(slot);
    }
    let (mut routes, mut finisher_setups) = (vec![], vec![]);
    for (route, setup, _, _) in priced {
        let while_building = match route.while_building {
            WhileBuilding::Slot(paired) => slot_of_draft[paired].map_or(WhileBuilding::Skip, WhileBuilding::Slot),
            other => other,
        };
        routes.push(Route { finisher_slot: finisher_setups.len(), while_building, ..route });
        finisher_setups.push(setup);
    }
    RouteSet { lift_setups: lifts.into_iter().map(|lift| lift.setup).collect(), finisher_setups, routes, dropped }
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
