//! Aptitude sparks and career race schedules.
use crate::career::inherit::aff_factor;
use crate::career::pool::{Pool, RaceWin};
use crate::career::{CareerSetup, Lineage};
use crate::chart::{chart_race_name, when};
use crate::data::{Choice, Data, NT, OptRaw, RepeatedRace, Sched, Uma, scored_position};
use crate::loop_planner::schedules::FILL_EPS;
use crate::uma::UmaIndex;
use serde_json::{Value, json};
use std::rc::Rc;
// The career planner's low-basis cost (`LOW_BASIS_MAX`, `LOW_BASIS_COST`): research/results.json's section 4 fit,
// written by build.rs.
include!(concat!(env!("OUT_DIR"), "/race_constants.rs"));

/// A pink spark's base chance to fire at one April inspiration, per star (0 to 3)
const PINK_RATE: [f64; 4] = [0.0, 0.01, 0.03, 0.05];

/// that chance from a slot of affinity factor `affinity_factor`
#[inline(always)]
pub(crate) fn pink_p(stars: u8, affinity_factor: f64) -> f64 {
    (PINK_RATE[stars as usize] * affinity_factor).min(1.0)
}

/// Her veteran's pink spark's stars, as the page's roll draws them (`webapp/src/pink.mjs`): 1 star below the first
/// bound, 2 below the second, else 3 (20/70/10, MODEL.md)
const PINK_STAR_BOUNDS: [f64; 2] = [0.2, 0.9];

/// A legacy's pink spark over a career: her aptitude (data.json `apt` order), the chance it fires at one April
/// inspiration, the legacy slot, stars
#[derive(Clone)]
pub(crate) struct PinkFire {
    pub(crate) aptitude: usize,
    pub(crate) chance: f64,
    pub(crate) slot: usize,
    pub(crate) stars: u8,
}

/// A pink spark a veteran before her result is known may carry (`CareerSetup::veteran`): its aptitude factor group,
/// stars and chance. She carries at most one of them, none with the rest.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) struct PinkChance {
    pub(crate) group: u32,
    pub(crate) stars: u8,
    pub(crate) chance: f64,
}

/// One outcome of a lineage's pink sparks (`PinkCareer::outcomes`): its chance, the trainee's grades after the career
/// start's pinks, and the sparks that may fire at the April inspirations, in slot order.
pub(crate) struct PinkOutcome {
    pub(crate) chance: f64,
    pub(crate) start: [u8; 10],
    pub(crate) fire: Vec<PinkFire>,
}

/// The trainee's aptitude codes (her outfit's) and the outcomes of her legacies' pink sparks: one, at chance 1, when
/// every legacy's pinks are on record; with a veteran before her result is known among the six (`Slot::pink_chances`),
/// one per combination of the pinks such veterans may carry, at its chance. Every number below is exact over that
/// mixture: the race table and `pink_gen` are linear in each outcome's.
pub(crate) struct PinkCareer {
    pub(crate) base: [u8; 10],
    pub(crate) outcomes: Vec<PinkOutcome>,
}

impl PinkCareer {
    /// Whether her distance and surface grades are `aptitudes`' at every turn, in every outcome (the race table reads
    /// only those six: no spark fires on one, and the start grades are `aptitudes`')
    fn keeps_grades(&self, aptitudes: &[u8]) -> bool {
        aptitudes.len() >= 6
            && self.outcomes.iter().all(|outcome| {
                outcome.start[..6] == aptitudes[..6] && outcome.fire.iter().all(|spark| spark.aptitude >= 6)
            })
    }
}

/// The April inspirations (0-based turns: Classic and Senior early April); a race on that turn already counts them
const INSP_TURNS: [usize; 2] = [30, 54];

/// aptitude index (data.json `apt` order) of a pink factor group: 31-34 short..long, 11 turf, 12 dirt, 21-24 front
/// runner..end closer
const fn pink_apt(group: u32) -> Option<usize> {
    match group {
        31..=34 => Some((group - 31) as usize),
        11 => Some(4),
        12 => Some(5),
        21..=24 => Some((group - 21) as usize + 6),
        _ => None,
    }
}

/// the pink factor group of aptitude k (inverse of `pink_apt`)
pub(crate) const fn apt_pink(k: usize) -> u32 {
    match k {
        0..=3 => 31 + k as u32,
        4 => 11,
        5 => 12,
        _ => 21 + (k as u32 - 6),
    }
}

/// distribution of the number of successes over independent `chances` (Poisson binomial)
fn pbinom(chances: &[f64]) -> Vec<f64> {
    let mut distribution = vec![1.0f64];
    for &p in chances {
        let mut next = vec![0.0f64; distribution.len() + 1];
        for (i, &mass) in distribution.iter().enumerate() {
            next[i] += mass * (1.0 - p);
            next[i + 1] += mass * p;
        }
        distribution = next;
    }
    distribution
}

/// the distribution of the sum of two independent counts with distributions `first` and `second`
fn conv(first: &[f64], second: &[f64]) -> Vec<f64> {
    let mut sum = vec![0.0f64; first.len() + second.len() - 1];
    for (i, &first_mass) in first.iter().enumerate() {
        for (j, &second_mass) in second.iter().enumerate() {
            sum[i + j] += first_mass * second_mass;
        }
    }
    sum
}

/// The grade increase of a career start whose six legacies carry `stars` pink stars of one aptitude summed: 1/2/3/4
/// grades at 1/4/7/10 stars (MODEL.md)
const fn start_increase(stars: u32) -> u8 {
    match stars {
        0 => 0,
        1..=3 => 1,
        4..=6 => 2,
        7..=9 => 3,
        _ => 4,
    }
}

/// The chance her veteran's pink is each aptitude (data.json `apt` order) in one outcome of her legacies' pinks
/// (`CareerSetup::pink_gen`)
fn outcome_pink_gen(outcome: &PinkOutcome) -> [f64; 10] {
    let q: Vec<f64> = (0..10)
        .map(|k| {
            let fire_chances: Vec<f64> =
                outcome.fire.iter().filter(|spark| spark.aptitude == k).map(|spark| spark.chance).collect();
            let one = pbinom(&fire_chances);
            let two = conv(&one, &one);
            two.iter().enumerate().filter(|(i, _)| outcome.start[k] as usize + i >= 7).map(|entry| entry.1).sum::<f64>()
        })
        .collect();
    let mut spark_chances = [0.0f64; 10];
    for k in 0..10 {
        let others: Vec<f64> = (0..10).filter(|&j| j != k).map(|j| q[j]).collect();
        let others_distribution = pbinom(&others);
        spark_chances[k] =
            q[k] * others_distribution.iter().enumerate().map(|(i, mass)| mass / (i + 1) as f64).sum::<f64>();
    }
    spark_chances
}

/// Her veteran's pink spark, (factor group, stars), drawn as the page's roll draws it (`webapp/src/pink.mjs`,
/// `drawPink`) from successive numbers of `uniform` (each in [0, 1)), in its order: each spark's firing at the Classic
/// then the Senior April inspiration, in `fire` order; then the aptitude, uniform over those she ends at A or S (no
/// more numbers when there is none: no pink); then her stars (`PINK_STAR_BOUNDS`). With several outcomes (a veteran
/// before her result is known among her legacies; the page never rolls one) one number first draws the outcome by its
/// chance.
pub(crate) fn draw_pink(pink_career: &PinkCareer, mut uniform: impl FnMut() -> f64) -> Option<(u32, u8)> {
    let outcome = match pink_career.outcomes.as_slice() {
        [only] => only,
        outcomes => {
            let mut remaining = uniform();
            let last = outcomes.last()?;
            outcomes
                .iter()
                .find(|outcome| {
                    remaining -= outcome.chance;
                    remaining < 0.0
                })
                .unwrap_or(last)
        }
    };
    let mut grades = outcome.start;
    for _ in INSP_TURNS {
        for spark in &outcome.fire {
            if uniform() < spark.chance {
                grades[spark.aptitude] = (grades[spark.aptitude] + 1).min(8);
            }
        }
    }
    let at_least_a: Vec<usize> = (0..10).filter(|&k| grades[k] >= 7).collect();
    if at_least_a.is_empty() {
        return None;
    }
    let aptitude = at_least_a[(uniform() * at_least_a.len() as f64) as usize];
    let star_draw = uniform();
    let stars = if star_draw < PINK_STAR_BOUNDS[0] {
        1
    } else if star_draw < PINK_STAR_BOUNDS[1] {
        2
    } else {
        3
    };
    Some((apt_pink(aptitude), stars))
}

/// One aptitude's persistent grades after the Classic and Senior inspirations.
struct GradeHistory {
    classic: u8,
    senior: u8,
    mass: f64,
}

struct AptitudeGrades {
    /// Before either inspiration, after Classic, after Senior.
    marginals: [Vec<(u8, f64)>; 3],
    histories: Vec<GradeHistory>,
}

/// Per aptitude, the grade distribution of one outcome of her pinks after 0, 1 and 2 inspirations, and its histories
fn grade_distributions(outcome: &PinkOutcome) -> Vec<AptitudeGrades> {
    (0..10)
        .map(|k| {
            let fire_chances: Vec<f64> =
                outcome.fire.iter().filter(|spark| spark.aptitude == k).map(|spark| spark.chance).collect();
            let one = pbinom(&fire_chances);
            let two = conv(&one, &one);
            let grade_distribution = |start: u8, success_distribution: &[f64]| {
                let mut distribution: Vec<(u8, f64)> = vec![];
                for (i, &weight) in success_distribution.iter().enumerate() {
                    if weight <= 0.0 {
                        continue;
                    }
                    let grade = (start as usize + i).min(8) as u8;
                    if let Some(entry) = distribution.iter_mut().find(|entry| entry.0 == grade) {
                        entry.1 += weight;
                    } else {
                        distribution.push((grade, weight));
                    }
                }
                distribution
            };
            let start = outcome.start[k];
            let classic_grades = grade_distribution(start, &one);
            let mut histories = vec![];
            for &(classic, classic_mass) in &classic_grades {
                for (senior, senior_mass) in grade_distribution(classic, &one) {
                    histories.push(GradeHistory { classic, senior, mass: classic_mass * senior_mass });
                }
            }
            AptitudeGrades {
                marginals: [vec![(start, 1.0)], classic_grades, grade_distribution(start, &two)],
                histories,
            }
        })
        .collect()
}

/// One outcome of her pinks with its grade distributions (`career_uma`)
struct OutcomeGrades {
    chance: f64,
    aptitudes: Vec<AptitudeGrades>,
}

/// The aptitudes a race reads (data.json `apt` order): its distance's and its surface's
#[derive(Clone, Copy)]
struct RaceAptitudes {
    distance: usize,
    surface: usize,
}

impl RaceAptitudes {
    fn of(data: &Data, race: usize) -> Self {
        Self { distance: data.race_distance[race] as usize, surface: if data.race_surface[race] == 1 { 4 } else { 5 } }
    }
}

/// The table's aptitude basis of a distance grade and a surface grade (codes 1..8 = G..S): 110 less each one's cost
/// (S/A 0, B 10, C 20, D 30, E 40, F 60, G 90), floored at -20 as the table is (any basis <= 0 wins nothing)
fn grade_basis(distance_grade: u8, surface_grade: u8) -> i32 {
    let cost = |grade: u8| -> i32 { [90, 90, 60, 40, 30, 20, 10, 0, 0][grade as usize] };
    (110 - cost(distance_grade) - cost(surface_grade)).max(-20)
}

/// A cell's aptitude basis in career mode: `LOW_BASIS_COST` points less where the table's basis is at most
/// `LOW_BASIS_MAX` (docs/RESEARCH.md section 4: on raised grades, the career planner's, the table predicts too many wins
/// there; on base grades, the loop planner's, it errs the other way, and the loop planner keeps the table), floored as
/// the table is
fn career_basis(table_basis: i32) -> i32 {
    if table_basis <= LOW_BASIS_MAX { (table_basis - LOW_BASIS_COST).max(-20) } else { table_basis }
}

/// One option's chances summed over her grade distribution (`career_uma`): per streak position, its goal shift
/// applied, and the expected table and career bases
#[derive(Default)]
struct CellChances {
    win: [f64; 7],
    table_basis: f64,
    basis: f64,
}

impl CellChances {
    /// Add the cell's chances at table basis `table_basis` with weight `weight`
    fn add(&mut self, data: &Data, option: &OptRaw, weight: f64, table_basis: i32) {
        let basis = career_basis(table_basis);
        self.table_basis += weight * f64::from(table_basis);
        self.basis += weight * f64::from(basis);
        for k in 1..=6 {
            self.win[k] += weight * data.attempt_chance(basis, scored_position(option.goal, k));
        }
    }
}

/// How many April inspirations her grades have had by 0-based turn `turn`
fn inspirations_by(turn: usize) -> usize {
    INSP_TURNS.iter().filter(|&&april_turn| turn >= april_turn).count()
}

/// A grade history's grade after `inspirations` April inspirations (0: `start`, the career start's)
const fn grade_after(history: &GradeHistory, start: u8, inspirations: usize) -> u8 {
    match inspirations {
        0 => start,
        1 => history.classic,
        _ => history.senior,
    }
}

/// E[p_Classic p_Senior] of a repeated race whose Classic attempt is on 0-based turn `classic_turn`, by actual streak
/// positions, over its grade histories in every outcome of her pinks (each attempt at its turn's grades), the goal
/// shift applied to each attempt where `shifts` says (Classic, Senior)
fn both_wins(
    data: &Data,
    outcomes: &[OutcomeGrades],
    race_aptitudes: RaceAptitudes,
    classic_turn: usize,
    shifts: [bool; 2],
) -> [[f64; 7]; 7] {
    let (classic_inspirations, senior_inspirations) =
        (inspirations_by(classic_turn), inspirations_by(classic_turn + 24));
    let mut both_wins = [[0.0; 7]; 7];
    for outcome in outcomes {
        let (distance_grades, surface_grades) =
            (&outcome.aptitudes[race_aptitudes.distance], &outcome.aptitudes[race_aptitudes.surface]);
        let (distance_start, surface_start) = (distance_grades.marginals[0][0].0, surface_grades.marginals[0][0].0);
        for distance in &distance_grades.histories {
            for surface in &surface_grades.histories {
                let mass = outcome.chance * distance.mass * surface.mass;
                let basis_after = |inspirations: usize| {
                    career_basis(grade_basis(
                        grade_after(distance, distance_start, inspirations),
                        grade_after(surface, surface_start, inspirations),
                    ))
                };
                let (classic_basis, senior_basis) =
                    (basis_after(classic_inspirations), basis_after(senior_inspirations));
                for classic_streak in 1..=6 {
                    let classic_chance = data.attempt_chance(classic_basis, scored_position(shifts[0], classic_streak));
                    for senior_streak in 1..=6 {
                        let senior_chance =
                            data.attempt_chance(senior_basis, scored_position(shifts[1], senior_streak));
                        both_wins[classic_streak][senior_streak] += mass * classic_chance * senior_chance;
                    }
                }
            }
        }
    }
    both_wins
}

/// Her race table in career mode (`career_uma`), owned beside the roster's
struct CareerUma {
    uma: Uma,
    /// Per turn and option, the expected table basis (110 - her grade costs) and career basis (`career_basis`: less
    /// the low-basis cost), for the report.
    expected_bases: Vec<Vec<ExpectedBases>>,
}

/// One option's expected bases (`CareerUma::expected_bases`)
#[derive(Clone, Copy, Default)]
struct ExpectedBases {
    table: f64,
    career: f64,
}

/// Trainee t's race table in career mode (`CareerSetup::plan_schedule`): per G1 option the win chance per streak
/// position at the career basis (`career_basis`), the expectation over her distance and surface grades at that turn
/// (each outcome of `pink_career`: before Classic April the start grades; then after one inspiration; from Senior April
/// after both). A repeated race whose grades are uncertain gets E[p_C p_S] over its shared history (`RepeatedRace`).
/// Without aptitude data (`pink_career` None, or no race distances and surfaces) and where her grades are the data's at
/// every turn, each cell's basis is the data's bp. None when the table is the data's: the grades are, and no cell's
/// basis is at most `LOW_BASIS_MAX`.
fn career_uma(data: &Data, pink_career: Option<&PinkCareer>, trainee: UmaIndex) -> Option<CareerUma> {
    let uma = &data.umas[trainee];
    let has_race_aptitudes = data.race_distance.len() >= data.race_count && data.race_surface.len() >= data.race_count;
    let changed_grades = pink_career.filter(|pink| has_race_aptitudes && !pink.keeps_grades(&uma.aptitudes));
    let has_low_basis = LOW_BASIS_COST != 0
        && uma.race_options.iter().flatten().any(|option| option.race >= 0 && option.win_rate_basis <= LOW_BASIS_MAX);
    if changed_grades.is_none() && !has_low_basis {
        return None;
    }
    // (none: every cell at the data's bp)
    let outcomes: Vec<OutcomeGrades> = changed_grades.map_or_else(Vec::new, |pink| {
        pink.outcomes
            .iter()
            .map(|outcome| OutcomeGrades { chance: outcome.chance, aptitudes: grade_distributions(outcome) })
            .collect()
    });
    // The adjusted table is an owned variant; the base uma remains available to other lineages.
    let mut adjusted_uma = uma.clone();
    let mut expected_bases: Vec<Vec<ExpectedBases>> =
        uma.race_options.iter().map(|options| vec![ExpectedBases::default(); options.len()]).collect();
    for turn in 0..NT {
        let inspirations = INSP_TURNS.iter().filter(|&&april_turn| turn >= april_turn).count();
        for (option_index, option) in uma.race_options[turn].iter().enumerate() {
            if option.race < 0 {
                continue;
            }
            let mut cell = CellChances::default();
            if outcomes.is_empty() {
                cell.add(data, option, 1.0, option.win_rate_basis);
            } else {
                let race_aptitudes = RaceAptitudes::of(data, option.race as usize);
                for outcome in &outcomes {
                    let (distances, surfaces) = (
                        &outcome.aptitudes[race_aptitudes.distance].marginals[inspirations],
                        &outcome.aptitudes[race_aptitudes.surface].marginals[inspirations],
                    );
                    for &(distance_grade, distance_weight) in distances {
                        for &(surface_grade, surface_weight) in surfaces {
                            let weight = outcome.chance * distance_weight * surface_weight;
                            cell.add(data, option, weight, grade_basis(distance_grade, surface_grade));
                        }
                    }
                }
            }
            expected_bases[turn][option_index] = ExpectedBases { table: cell.table_basis, career: cell.basis };
            adjusted_uma.choices[turn][option_index + 1].win_chances = cell.win;
        }
    }
    // Repeated races share the Classic inspiration and the outcome of her pinks. In a single outcome a Classic attempt
    // before Classic April has certain grades, and so do both attempts where the histories are certain: those keep the
    // original independent-attempt arithmetic. Several outcomes correlate even those attempts.
    let first_classic_turn = if outcomes.len() > 1 { 24 } else { INSP_TURNS[0] };
    for classic_turn in first_classic_turn..48 {
        if outcomes.is_empty() {
            break;
        }
        for (classic_option, classic) in uma.race_options[classic_turn].iter().enumerate() {
            if classic.race < 0 {
                continue;
            }
            let Some((senior_option, senior)) =
                uma.race_options[classic_turn + 24].iter().enumerate().find(|(_, option)| option.race == classic.race)
            else {
                continue;
            };
            let race = classic.race as usize;
            let race_aptitudes = RaceAptitudes::of(data, race);
            let certain = outcomes.len() == 1
                && outcomes[0].aptitudes[race_aptitudes.distance].histories.len() == 1
                && outcomes[0].aptitudes[race_aptitudes.surface].histories.len() == 1;
            if certain {
                continue;
            }
            adjusted_uma.repeated_races.push(RepeatedRace {
                race,
                classic_turn,
                classic_option: classic_option as u8,
                senior_option: senior_option as u8,
                both_wins: both_wins(data, &outcomes, race_aptitudes, classic_turn, [classic.goal, senior.goal]),
            });
        }
    }
    Some(CareerUma { uma: adjusted_uma, expected_bases })
}

/// The outfits a career's schedule may run (`CareerSetup::plan_schedule`).
#[derive(Clone, Copy)]
pub(crate) enum ScheduleOutfits {
    /// Her deck's outfit option (a `Model::outfit_options` index: the one her career's exact odds pick,
    /// `CareerChances::outfit`): she runs its best outfit for her schedule.
    DeckOption(usize),
    /// One outfit, by card id (0: her character's grades).
    Card(u32),
}

/// One outfit a career may run, as her schedule reads it: its card id and aptitude codes.
#[derive(Clone, Copy)]
pub(crate) struct OutfitGrades<'data> {
    pub(crate) card: u32,
    pub(crate) aptitudes: &'data [u8],
}

/// What a career's schedule and aptitudes leave her veteran before her result is known (`Evaluator::wins`,
/// `CareerSetup::veteran`).
#[derive(Clone)]
pub(crate) struct ExpectedRecord {
    /// Her chance of at least one win, per race index.
    pub(crate) win_chances: Rc<Vec<f64>>,
    /// The chance of each pink spark her veteran may carry (`CareerSetup::pink_chances`; empty without aptitude data).
    pub(crate) pink_chances: Rc<Vec<PinkChance>>,
    /// The outfit her schedule runs (card id; 0: her character's grades), for her pink's draw (`draw_veteran_pink`).
    pub(crate) outfit: u32,
}

/// How close (relative) two outfits' schedule values count as a tie (`best_outfits`)
const OUTFIT_TIE: f64 = 1e-12;

/// Among outfits whose schedules score `values`, the one a career runs (the best, the first on a tie) and every one
/// tied with it (within `OUTFIT_TIE`), it first, the others in order
fn best_outfits(values: &[f64]) -> (usize, Vec<usize>) {
    let mut best = 0;
    for (index, &value) in values.iter().enumerate() {
        if value > values[best] {
            best = index;
        }
    }
    let bar = values[best] - OUTFIT_TIE * values[best].abs().max(1.0);
    let others = (0..values.len()).filter(|&index| index != best && values[index] >= bar);
    (best, std::iter::once(best).chain(others).collect())
}

/// A career's schedule in the outfit it runs (`CareerSetup::plan_schedule`), before the report's rows.
pub(crate) struct PlannedSchedule<'data> {
    pub(crate) outfit: OutfitGrades<'data>,
    /// Her outfits' grades whose schedules score as well (`best_outfits`), `outfit`'s included: the report names them
    /// all.
    pub(crate) tied: Vec<OutfitGrades<'data>>,
    /// Her grades and pinks in that outfit (None without aptitude data).
    pub(crate) pink: Option<PinkCareer>,
    /// Her career table where it differs from the roster's.
    table: Option<CareerUma>,
    /// Her schedule (None: no feasible one).
    schedule: Option<Sched>,
    /// Her chance of at least one win, per race index.
    pub(crate) win_chances: Vec<f64>,
}

/// What a career's schedule reads (`Evaluator::wins`' memo key): the trainee, the outfits it may run (card ids and
/// grades, `CareerSetup::outfit_grades`), its race weights' bits, and her legacies' pink inputs (per slot its
/// affinity factor's bits and its recorded and possible pinks).
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct ScheduleKey {
    trainee: UmaIndex,
    outfits: Vec<u32>,
    weights: Vec<u64>,
    pinks: Vec<u64>,
}

impl<'data> CareerSetup<'data> {
    /// The distinct grades among `outfits`' cards, each with its first card, in the option's order (one entry when her
    /// outfits share grades).
    pub(crate) fn outfit_grades(&self, trainee: UmaIndex, outfits: ScheduleOutfits) -> Vec<OutfitGrades<'data>> {
        let uma = &self.data.umas[trainee];
        let mut distinct: Vec<OutfitGrades<'data>> = vec![];
        let mut add = |card: u32| {
            let aptitudes = uma.aptitudes_of(card);
            if !distinct.iter().any(|grades| grades.aptitudes == aptitudes) {
                distinct.push(OutfitGrades { card, aptitudes });
            }
        };
        match outfits {
            ScheduleOutfits::DeckOption(option) => {
                for &(card, _) in &self.model.outfit_options[trainee][option].outfits {
                    add(card);
                }
            }
            ScheduleOutfits::Card(card) => add(card),
        }
        distinct
    }

    /// The lineage trainee's race schedule with her parents (`race_weights`), in the best of `outfits` for it: per
    /// distinct grades her career table (`career_uma`: her outfit's grades, her legacies' pinks, the low-basis cost) and
    /// its exact best response; the highest weighted value wins, the first on a tie (`best_outfits`), and the tied
    /// grades are kept (`PlannedSchedule::tied`).
    pub(crate) fn plan_schedule(
        &self,
        pool: &Pool,
        lineage: Lineage,
        outfits: ScheduleOutfits,
    ) -> PlannedSchedule<'data> {
        let data = self.data;
        let race_weights = self.race_weights(pool, lineage);
        let candidates = self.outfit_grades(lineage.trainee, outfits);
        let mut planned: Vec<PlannedSchedule<'data>> = vec![];
        let mut values: Vec<f64> = vec![];
        for &outfit in &candidates {
            let pink = self.pink_career(pool, lineage, outfit.aptitudes);
            let table = career_uma(data, pink.as_ref(), lineage.trainee);
            let uma = table.as_ref().map_or(&data.umas[lineage.trainee], |career| &career.uma);
            // her own race table: solved directly (the memo is keyed by uma id)
            let best_response = if table.is_some() {
                data.best_response_solve(uma, &race_weights, None)
            } else {
                data.best_response(uma, &race_weights, None)
            };
            let (schedule, value, win_chances) = match best_response {
                Some((schedule, value)) => (Some(schedule), value, data.win_chances(uma, &schedule)),
                None => (None, f64::NEG_INFINITY, vec![0.0; data.race_count]),
            };
            values.push(value);
            planned.push(PlannedSchedule { outfit, tied: vec![], pink, table, schedule, win_chances });
        }
        // (a career has at least one outfit: `outfit_grades` of an option or a card)
        let (best, tied) = best_outfits(&values);
        let mut chosen = planned.swap_remove(best);
        chosen.tied = tied.into_iter().map(|index| candidates[index]).collect();
        chosen
    }

    /// Her schedule's race weights: per race `FILL_EPS` plus each parent's chance of having won it (`schedule_rows`)
    fn race_weights(&self, pool: &Pool, lineage: Lineage) -> Vec<f64> {
        let mut race_weights = vec![FILL_EPS; self.data.race_count];
        for parent in [lineage.first_parent, lineage.second_parent] {
            for &RaceWin { race, chance: parent_chance } in &pool.veterans[parent].wins {
                if race < self.data.race_count {
                    race_weights[race] += parent_chance;
                }
            }
        }
        race_weights
    }

    /// What `plan_schedule` reads (`ScheduleKey`).
    pub(crate) fn schedule_key(&self, pool: &Pool, lineage: Lineage, outfits: ScheduleOutfits) -> ScheduleKey {
        const SEPARATOR: u64 = u64::MAX;
        let mut outfit_key = vec![];
        for OutfitGrades { card, aptitudes } in self.outfit_grades(lineage.trainee, outfits) {
            outfit_key.push(card);
            outfit_key.extend(aptitudes.iter().map(|&code| u32::from(code)));
            outfit_key.push(u32::MAX);
        }
        let mut pinks = vec![];
        for slot in self.slots(pool, lineage) {
            pinks.push(aff_factor(slot.relation_points, slot.race).to_bits());
            pinks.extend(slot.pinks.iter().map(|&(group, stars)| (u64::from(group) << 8) | u64::from(stars)));
            for pink in slot.pink_chances.iter() {
                pinks.extend([(u64::from(pink.group) << 8) | u64::from(pink.stars), pink.chance.to_bits()]);
            }
            pinks.push(SEPARATOR);
        }
        ScheduleKey {
            trainee: lineage.trainee,
            outfits: outfit_key,
            weights: self.race_weights(pool, lineage).iter().map(|weight| weight.to_bits()).collect(),
            pinks,
        }
    }
}

impl CareerSetup<'_> {
    /// The lineage trainee's aptitudes over the career with her parents, in an outfit of grades `aptitudes` (MODEL.md,
    /// aptitude sparks): at the start, per aptitude the six legacies' pink stars summed raise her grade 1/2/3/4 grades
    /// at 1/4/7/10 stars, at most to A; at each April inspiration (Classic, Senior) every legacy's pink spark fires with
    /// base 1/3/5% by stars x (1 + that legacy's affinity / 100), each firing +1 grade, at most S. A veteran before
    /// her result is known adds her possible pinks as outcomes (`PinkCareer`). None when data.json has no aptitudes.
    pub(crate) fn pink_career(&self, pool: &Pool, lineage: Lineage, aptitudes: &[u8]) -> Option<PinkCareer> {
        if aptitudes.len() < 10 {
            return None;
        }
        let mut base = [0u8; 10];
        base.copy_from_slice(&aptitudes[..10]);
        // per outcome: its chance, its pink stars per aptitude, its sparks, and per veteran before her result is known
        // (by the first slot she fills) the pink she carries in it (an index into her `pink_chances`; None: none)
        struct Partial {
            chance: f64,
            stars: [u32; 10],
            fire: Vec<PinkFire>,
            drawn: Vec<(usize, Option<usize>)>,
        }
        let mut partials = vec![Partial { chance: 1.0, stars: [0; 10], fire: vec![], drawn: vec![] }];
        let slots = self.slots(pool, lineage);
        for (slot_index, slot) in slots.iter().enumerate() {
            let affinity_factor = aff_factor(slot.relation_points, slot.race);
            let spark_of = |group: u32, stars: u8| -> Option<PinkFire> {
                let (aptitude, stars) = (pink_apt(group)?, stars.min(3));
                if stars == 0 {
                    return None;
                }
                Some(PinkFire { aptitude, chance: pink_p(stars, affinity_factor), slot: slot_index, stars })
            };
            for spark in slot.pinks.iter().filter_map(|&(group, stars)| spark_of(group, stars)) {
                for partial in &mut partials {
                    partial.stars[spark.aptitude] += u32::from(spark.stars);
                    partial.fire.push(spark.clone());
                }
            }
            if slot.pink_chances.is_empty() {
                continue;
            }
            // A veteran before her result is known carries one pink in every slot she fills (the plan's routes schedule
            // a lineage with her as a parent; through her child she can also be the other parent's parent, with her
            // record copied). Slots keep no identity, so a slot of the same character with the same possible pinks as
            // an earlier slot is taken as her again: it takes the pink each outcome drew there, at its own affinity.
            // Other slots expand independently. Exact for one veteran per character and for distinct veterans whose
            // possible pinks differ; two distinct veterans of one character with identical possible pinks in one
            // lineage would be correlated wrongly (no caller builds one).
            let same_veteran = slots[..slot_index].iter().position(|earlier| {
                earlier.character_id == slot.character_id && earlier.pink_chances == slot.pink_chances
            });
            if let Some(first_slot) = same_veteran {
                for partial in &mut partials {
                    let drawn = partial.drawn.iter().find(|(slot, _)| *slot == first_slot).and_then(|(_, pink)| *pink);
                    let Some(spark) = drawn.and_then(|pink| {
                        let pink = slot.pink_chances[pink];
                        spark_of(pink.group, pink.stars)
                    }) else {
                        continue;
                    };
                    partial.stars[spark.aptitude] += u32::from(spark.stars);
                    partial.fire.push(spark);
                }
                continue;
            }
            // her possible pinks: each outcome so far, without one (the rest of her chance) and with each
            let without_pink = 1.0 - slot.pink_chances.iter().map(|pink| pink.chance).sum::<f64>();
            let mut expanded = vec![];
            for partial in partials {
                for (pink_index, pink) in slot.pink_chances.iter().enumerate() {
                    let Some(spark) = spark_of(pink.group, pink.stars).filter(|_| pink.chance > 0.0) else { continue };
                    // (each outcome owns its sparks and draws: the copies are the point)
                    let mut with_pink = Partial {
                        chance: partial.chance * pink.chance,
                        stars: partial.stars,
                        fire: partial.fire.clone(),
                        drawn: partial.drawn.clone(),
                    };
                    with_pink.stars[spark.aptitude] += u32::from(spark.stars);
                    with_pink.fire.push(spark);
                    with_pink.drawn.push((slot_index, Some(pink_index)));
                    expanded.push(with_pink);
                }
                if without_pink > 0.0 {
                    let mut without = Partial { chance: partial.chance * without_pink, ..partial };
                    without.drawn.push((slot_index, None));
                    expanded.push(without);
                }
            }
            partials = expanded;
        }
        let outcomes = partials
            .into_iter()
            .map(|Partial { chance, stars, fire, .. }| {
                let mut start = base;
                for k in 0..10 {
                    if base[k] < 7 {
                        start[k] = (base[k] + start_increase(stars[k])).min(7);
                    }
                }
                PinkOutcome { chance, start, fire }
            })
            .collect();
        Some(PinkCareer { base, outcomes })
    }
    /// The chance of each pink spark of her veteran (MODEL.md): one, its aptitude uniform over those she ends at A or S
    /// (stars 20/70/10: `pink_chances`): per aptitude (data.json `apt` order) P(it is the one); exact, the aptitudes'
    /// inspirations independent within an outcome of her legacies' pinks, summed over those outcomes. The rest: no
    /// aptitude at A or S.
    pub(crate) fn pink_gen(pink_career: &PinkCareer) -> [f64; 10] {
        let mut spark_chances = [0.0f64; 10];
        for outcome in &pink_career.outcomes {
            let outcome_chances = outcome_pink_gen(outcome);
            for k in 0..10 {
                spark_chances[k] += outcome.chance * outcome_chances[k];
            }
        }
        spark_chances
    }
    /// Her veteran's pink spark as a mixture (`PinkChance`): per aptitude its `pink_gen` chance, split over 1/2/3 stars
    /// by the page's star draw (`PINK_STAR_BOUNDS`); the pinks with a chance above 0.
    pub(crate) fn pink_chances(pink_career: &PinkCareer) -> Vec<PinkChance> {
        let star_shares = [PINK_STAR_BOUNDS[0], PINK_STAR_BOUNDS[1] - PINK_STAR_BOUNDS[0], 1.0 - PINK_STAR_BOUNDS[1]];
        let pink_gen = Self::pink_gen(pink_career);
        let mut chances = vec![];
        for (k, &aptitude_chance) in pink_gen.iter().enumerate() {
            for (stars, &share) in (1u8..=3).zip(&star_shares) {
                if aptitude_chance * share > 0.0 {
                    chances.push(PinkChance { group: apt_pink(k), stars, chance: aptitude_chance * share });
                }
            }
        }
        chances
    }
    /// Her veteran's pink spark after the career `lineage` runs in outfit `outfit` (card id; `ExpectedRecord::outfit`),
    /// drawn as the page's roll draws it (`draw_pink`), as `Legacy::pinks` holds it (empty: none, or no aptitude data).
    pub(crate) fn draw_veteran_pink(
        &self,
        pool: &Pool,
        lineage: Lineage,
        outfit: u32,
        uniform: impl FnMut() -> f64,
    ) -> Vec<(u32, u8)> {
        let aptitudes = self.data.umas[lineage.trainee].aptitudes_of(outfit);
        self.pink_career(pool, lineage, aptitudes)
            .and_then(|pink_career| draw_pink(&pink_career, uniform))
            .into_iter()
            .collect()
    }
    /// The report's rows of a planned schedule (`plan_schedule`): every G1 she could enter, in turn order; `run`
    /// false for those she skips, with what she does that turn. A G1 counts each parent's chance of having won it (a
    /// win she shares with a parent adds 3 affinity to each of them when she is a parent herself, and her parents' wins
    /// are the guide to what her later partners will have won), so she runs the races her parents won; every G1 also
    /// counts `FILL_EPS`, the loop planner's tie-break, so a G1 neither parent won enters where it costs the parents'
    /// races (almost) nothing: a bounded scalar tie-break, losing them at most `FILL_EPS` x the G1s it adds (about 3.4e-6
    /// expected weighted wins), and nothing on base tables with certain parent wins, whose values lie on a 1e-4 lattice
    /// (with pinks or parents' chances below 1 a smaller loss can be traded). With no G1 among her parents' wins she runs
    /// the G1s that win the most.
    pub(crate) fn schedule_rows(&self, pool: &Pool, lineage: Lineage, planned: &PlannedSchedule) -> Value {
        let Lineage { trainee, first_parent, second_parent } = lineage;
        let data = self.data;
        let Some(schedule) = &planned.schedule else { return json!([]) };
        let uma = planned.table.as_ref().map_or(&data.umas[trainee], |career| &career.uma);
        let won = |parent: usize, race: usize| {
            pool.veterans[parent].wins.iter().find(|win| win.race == race).map_or(0.0, |win| win.chance)
        };
        let mut rows = vec![];
        let mut k = 0usize;
        for turn in 0..NT {
            let chosen = match schedule[turn] {
                Choice::Rest => {
                    k = 0;
                    None
                }
                Choice::Opt(option_index) => {
                    k = (k + 1).min(6);
                    Some(option_index as usize)
                }
            };
            for (option_index, option) in uma.race_options[turn].iter().enumerate() {
                if option.race < 0 {
                    continue;
                }
                let race = option.race as usize;
                let run = chosen == Some(option_index);
                let instead = match chosen {
                    _ if run => Value::Null,
                    None => json!("rest"),
                    Some(other_option) => json!(uma.race_options[turn][other_option].name),
                };
                // the loop planner's chart row (one member: `cells`, `shared` = a parent won it, so it builds affinity),
                // plus `run`, `p`, `instead`, and `name` (the race's full name)
                let parent_wins = [won(first_parent, race) > 0.0, won(second_parent, race) > 0.0];
                let goal = option.goal;
                // win chance, aptitude cost (110 - her expected table basis: distance and surface grade costs with her
                // pinks) and low-basis cost (her expected `LOW_BASIS_COST`), rounded so the two sum to 110 less her
                // expected career basis, rounded
                let option_chances = uma.choices[turn][option_index + 1].win_chances;
                let bases = planned.table.as_ref().map_or_else(
                    || {
                        let basis = f64::from(option.win_rate_basis);
                        ExpectedBases { table: basis, career: basis }
                    },
                    |career| career.expected_bases[turn][option_index],
                );
                let aptitude_cost = (110.0 - bases.table).round() as i64;
                let low_basis_cost = (110.0 - bases.career).round() as i64 - aptitude_cost;
                let cell = if run {
                    json!({"p": (option_chances[k] * 100.0).round(), "goal": goal, "apt": aptitude_cost, "low": low_basis_cost, "row": k, "pen": data.streak_penalty(goal, k)})
                } else {
                    Value::Null
                };
                rows.push(json!({"turn": turn, "when": when(turn), "year": turn / 24, "race": chart_race_name(&data.race_names[race]), "name": data.race_names[race],
                                 "run": run, "skipped": !run, "instead": instead, "p": if run { json!(option_chances[k]) } else { Value::Null }, "goal": goal,
                                 "parents": parent_wins, "shared": parent_wins[0] || parent_wins[1], "cells": [cell], "runners": u8::from(run), "alts": []}));
            }
        }
        Value::Array(rows)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        LOW_BASIS_COST, LOW_BASIS_MAX, PinkCareer, PinkFire, PinkOutcome, best_outfits, career_uma, draw_pink,
    };
    use crate::career::CareerSetup;
    use crate::data::{Choice, Data, NT, Sched, load_str};
    use crate::uma::UmaIndex;
    use serde_json::json;

    const COST: [i32; 9] = [90, 90, 60, 40, 30, 20, 10, 0, 0];
    const PENALTY: [i32; 6] = [0, 0, 5, 20, 30, 50];

    /// Forced lead-in races set each attempt's streak; a second Senior option competes with the repeat.
    fn repeat_data(classic_turn: usize, streaks: [usize; 2], goals: [bool; 2], aptitudes: [u8; 10]) -> Data {
        let mut turns = vec![vec![]; NT];
        let mut forced = vec![];
        let basis = 110 - COST[aptitudes[0] as usize] - COST[aptitudes[4] as usize];
        for (year, turn) in [classic_turn, classic_turn + 24].into_iter().enumerate() {
            for lead_in in turn + 1 - streaks[year]..turn {
                turns[lead_in].push(json!({"prog": 0, "race": -1, "bp": 0}));
                forced.push(json!([[lead_in + 1, 0]]));
            }
            turns[turn].push(json!({"prog": 1, "race": 0, "bp": basis, "goal": goals[year]}));
        }
        forced.push(json!([[classic_turn + 1, 0]]));
        turns[classic_turn + 24].push(json!({"prog": 2, "race": 1, "bp": 110}));
        load_str(
            &json!({
                "race_names": ["Repeat", "Alternative"], "race_cat": [0, 1], "race_ground": [1, 2],
                "relations": [], "pen": PENALTY,
                "umas": [{"id": 1, "name": "Synthetic", "apt": aptitudes, "turns": turns, "goals": forced}]
            })
            .to_string(),
        )
    }

    fn repeat_schedule(classic_turn: usize, streaks: [usize; 2]) -> Sched {
        let mut schedule = [Choice::Rest; NT];
        for (year, turn) in [classic_turn, classic_turn + 24].into_iter().enumerate() {
            schedule[turn + 1 - streaks[year]..=turn].fill(Choice::Opt(0));
        }
        schedule
    }

    /// A lineage's pinks with one outcome, at chance 1
    fn recorded(start: [u8; 10], fire: Vec<PinkFire>) -> PinkCareer {
        PinkCareer { base: start, outcomes: vec![PinkOutcome { chance: 1.0, start, fire }] }
    }

    /// Independent oracle: per outcome, enumerate every individual spark's two Bernoulli firings, apply them in
    /// turn order, then average the conditional miss product; sum the outcomes by their chances. The career basis
    /// (the low-basis cost) is written out here. No production distribution/table helpers.
    fn enumerated_win(pink: &PinkCareer, classic_turn: usize, streaks: [usize; 2], goals: [bool; 2]) -> f64 {
        let mut total = 0.0;
        for outcome in &pink.outcomes {
            for firings in 0..(1usize << (2 * outcome.fire.len())) {
                let mut grades = outcome.start;
                let mut mass = outcome.chance;
                let mut miss = 1.0;
                let mut inspiration = 0;
                for (year, turn) in [classic_turn, classic_turn + 24].into_iter().enumerate() {
                    while inspiration < 2 && turn >= [30, 54][inspiration] {
                        for (index, spark) in outcome.fire.iter().enumerate() {
                            let fires = firings & (1 << (inspiration * outcome.fire.len() + index)) != 0;
                            if fires {
                                mass *= spark.chance;
                                grades[spark.aptitude] = (grades[spark.aptitude] + 1).min(8);
                            } else {
                                mass *= 1.0 - spark.chance;
                            }
                        }
                        inspiration += 1;
                    }
                    let position = if goals[year] { streaks[year].saturating_sub(1).max(1) } else { streaks[year] };
                    let table_basis = 110 - COST[grades[0] as usize] - COST[grades[4] as usize];
                    let basis = if table_basis <= LOW_BASIS_MAX { table_basis - LOW_BASIS_COST } else { table_basis };
                    let chance = f64::from((basis - PENALTY[position - 1]).clamp(0, 100)) / 100.0;
                    miss *= 1.0 - chance;
                }
                // Integrate any inspiration after the final attempt as well, so each bit pattern has its mass.
                for later in inspiration..2 {
                    for (index, spark) in outcome.fire.iter().enumerate() {
                        mass *= if firings & (1 << (later * outcome.fire.len() + index)) != 0 {
                            spark.chance
                        } else {
                            1.0 - spark.chance
                        };
                    }
                }
                total += mass * (1.0 - miss);
            }
        }
        total
    }

    /// The table, the DP value and a single attempt's marginal against the oracle, at every streak pair
    fn check_against_enumeration(pink: &PinkCareer, classic_turn: usize, data_grades: [u8; 10]) {
        for classic_streak in 1..=6 {
            for senior_streak in 1..=6 {
                let streaks = [classic_streak, senior_streak];
                let goals = [true, false];
                let data = repeat_data(classic_turn, streaks, goals, data_grades);
                let adjusted = career_uma(&data, Some(pink), UmaIndex(0)).unwrap();
                let schedule = repeat_schedule(classic_turn, streaks);
                let expected = enumerated_win(pink, classic_turn, streaks, goals);
                let actual = data.win_chances(&adjusted.uma, &schedule)[0];
                assert!(
                    (actual - expected).abs() < 1e-12,
                    "turn {classic_turn}, streaks {streaks:?}: {actual} != {expected}"
                );
                let (_, value) = data.best_response_solve(&adjusted.uma, &[1.0, 0.0], None).unwrap();
                assert!((value - expected).abs() < 1e-12, "DP value {value} != {expected}");
                // Entering just one attempt must still use its marginal chance.
                let mut single = schedule;
                single[classic_turn + 24] = Choice::Rest;
                let marginal = adjusted.uma.choices[classic_turn][1].win_chances[classic_streak];
                assert_eq!(data.win_chances(&adjusted.uma, &single)[0].to_bits(), (1.0 - (1.0 - marginal)).to_bits());
            }
        }
    }

    #[test]
    fn pink_repeats_match_independent_enumeration() {
        // (4, 6): basis 70, at the low-basis bound before any firing
        for (classic_turn, distance, surface) in [(35, 4, 6), (29, 4, 6), (35, 7, 7), (35, 8, 8)] {
            let mut start = [7; 10];
            start[0] = distance;
            start[4] = surface;
            let fire = [(0, 0.17), (0, 0.29), (4, 0.23)]
                .into_iter()
                .enumerate()
                .map(|(slot, (aptitude, chance))| PinkFire { aptitude, chance, slot, stars: 3 })
                .collect();
            check_against_enumeration(&recorded(start, fire), classic_turn, start);
        }
    }

    #[test]
    fn possible_pinks_match_independent_enumeration() {
        // a veteran before her result is known: her pink raises the start grade (C) with chance .3 and fires on
        // distance with chance .25; beside a recorded spark on surface
        let mut data_grades = [7; 10];
        data_grades[0] = 4;
        data_grades[4] = 5;
        let recorded_spark = PinkFire { aptitude: 4, chance: 0.2, slot: 0, stars: 2 };
        let mut raised = data_grades;
        raised[0] = 5;
        let pink = PinkCareer {
            base: data_grades,
            outcomes: vec![
                PinkOutcome {
                    chance: 0.3,
                    start: raised,
                    fire: vec![recorded_spark.clone(), PinkFire { aptitude: 0, chance: 0.25, slot: 1, stars: 3 }],
                },
                PinkOutcome { chance: 0.7, start: data_grades, fire: vec![recorded_spark] },
            ],
        };
        for classic_turn in [29, 35] {
            check_against_enumeration(&pink, classic_turn, data_grades);
        }
    }

    #[test]
    fn pink_correlation_changes_the_best_schedule() {
        let mut start = [7; 10];
        start[0] = 4;
        let pink = recorded(start, vec![PinkFire { aptitude: 0, chance: 0.2, slot: 0, stars: 3 }]);
        let data = repeat_data(35, [1, 1], [false, false], start);
        let adjusted = career_uma(&data, Some(&pink), UmaIndex(0)).unwrap();
        let exact = enumerated_win(&pink, 35, [1, 1], [false, false]);
        assert!((exact - 0.9696).abs() < 1e-12, "analytic control: {exact}");
        // Independent marginals give .9712, above the alternative's .82 + .1504.
        // The exact repeat is below it, so the DP must choose the alternative.
        let (schedule, value) = data.best_response_solve(&adjusted.uma, &[1.0, 0.1504], None).unwrap();
        assert_eq!(schedule[59], Choice::Opt(1));
        assert!((value - 0.9704).abs() < 1e-12, "alternative value: {value}");
    }

    #[test]
    fn no_pink_repeats_keep_the_original_bits() {
        let mut start = [7; 10];
        start[0] = 4;
        let pink = recorded(start, vec![]);
        for classic_streak in 1..=6 {
            for senior_streak in 1..=6 {
                let streaks = [classic_streak, senior_streak];
                let data = repeat_data(35, streaks, [false, true], start);
                assert!(career_uma(&data, Some(&pink), UmaIndex(0)).is_none(), "no pinks must use the base table");
                let uma = &data.umas[UmaIndex(0)];
                assert!(uma.repeated_races.is_empty(), "certain aptitudes need no joint table");
                let classic = uma.choices[35][1].win_chances[classic_streak];
                let senior = uma.choices[59][1].win_chances[senior_streak];
                let mut original_miss = 1.0;
                original_miss *= 1.0 - classic;
                original_miss *= 1.0 - senior;
                let expected = 1.0 - original_miss;
                let schedule = repeat_schedule(35, streaks);
                assert_eq!(data.win_chances(uma, &schedule)[0].to_bits(), expected.to_bits());
                let (chosen, value) = data.best_response_solve(uma, &[1.0, 0.0], None).unwrap();
                assert_eq!(chosen, schedule);
                assert_eq!(value.to_bits(), expected.to_bits());
                assert_eq!((1.0 - (1.0 - classic) * (1.0 - senior)).to_bits(), expected.to_bits());
            }
        }
    }

    #[test]
    fn low_basis_costs_more_in_career_mode_only() {
        // no pinks, grades D and D: basis 50 (at most LOW_BASIS_MAX at the current fit), so the career table charges
        // LOW_BASIS_COST more; the data's table does not
        let mut start = [7; 10];
        start[0] = 4;
        start[4] = 4;
        let data = repeat_data(35, [1, 1], [false, false], start);
        assert_eq!(data.umas[UmaIndex(0)].choices[35][1].win_chances[1], 0.5);
        let low_basis_cost = if 50 <= LOW_BASIS_MAX { LOW_BASIS_COST } else { 0 };
        let career = career_uma(&data, Some(&recorded(start, vec![])), UmaIndex(0));
        assert_eq!(career.is_some(), low_basis_cost != 0, "a low basis changes the table");
        if let Some(career) = career {
            assert_eq!(career.uma.choices[35][1].win_chances[1], f64::from(50 - low_basis_cost) / 100.0);
            assert!(career.uma.repeated_races.is_empty(), "certain grades need no joint table");
        }
    }

    #[test]
    fn a_career_runs_her_outfits_own_grades() {
        // Vodka on Global data: Tenno Sho (Spring) is long turf; [Fiery Aqua Vitae] has long E (basis 70, at most
        // LOW_BASIS_MAX: 70 - LOW_BASIS_COST), [Wild Top Gear] the data's long F (basis 50, 50 - LOW_BASIS_COST)
        let data = crate::data::load(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"));
        let vodka = data.umas.indices().find(|&uma| data.umas[uma].id == 1008).unwrap();
        let race = data.race_names.iter().position(|name| name == "Tenno Sho (Spring)").unwrap() as i32;
        let (turn, option) = (0..NT)
            .find_map(|turn| {
                let options = &data.umas[vodka].race_options[turn];
                options.iter().position(|option| option.race == race).map(|option| (turn, option))
            })
            .unwrap();
        let first_chance = |card: u32| {
            let grades: [u8; 10] = data.umas[vodka].aptitudes_of(card).try_into().unwrap();
            let career = career_uma(&data, Some(&recorded(grades, vec![])), vodka).unwrap();
            career.uma.choices[turn][option + 1].win_chances[1]
        };
        assert_eq!(first_chance(100_802), f64::from(70 - LOW_BASIS_COST) / 100.0);
        assert_eq!(first_chance(100_801), f64::from(50 - LOW_BASIS_COST) / 100.0);
    }

    #[test]
    fn the_docs_state_the_low_basis_cost() {
        // MODEL.md and SOLVER.md type the fitted constants build.rs reads (/about takes them at its build)
        for (doc, statement) in [
            ("MODEL.md", format!("is {LOW_BASIS_MAX} or less costs {LOW_BASIS_COST} more")),
            ("SOLVER.md", format!("is {LOW_BASIS_COST} points lower where it is {LOW_BASIS_MAX} or less")),
        ] {
            let path = format!("{}/../../docs/{doc}", env!("CARGO_MANIFEST_DIR"));
            let text = std::fs::read_to_string(&path).unwrap();
            assert!(
                text.contains(&statement),
                "{doc} does not say \"{statement}\": update it to research/results.json"
            );
        }
    }

    #[test]
    fn the_best_outfit_is_the_first_of_those_tied() {
        assert_eq!(best_outfits(&[20.48, 20.88]), (1, vec![1]));
        assert_eq!(best_outfits(&[21.633, 21.633, 3.0]), (0, vec![0, 1]));
        assert_eq!(best_outfits(&[f64::NEG_INFINITY]), (0, vec![0]));
        // within the tie: the strictly best is run and named first
        assert_eq!(best_outfits(&[21.633 - 1e-13, 21.633]), (1, vec![1, 0]));
    }

    #[test]
    fn veteran_pink_draw_follows_the_page_order() {
        // two sparks on aptitude 0 (start B): the first fires at the Classic inspiration, so she ends at A on
        // aptitude 0 beside the A grades 1..=9; the aptitude number picks the first of the ten, the stars number 2
        let mut start = [7; 10];
        start[0] = 6;
        let fire = vec![
            PinkFire { aptitude: 0, chance: 0.5, slot: 0, stars: 3 },
            PinkFire { aptitude: 0, chance: 0.5, slot: 1, stars: 1 },
        ];
        let pink = recorded(start, fire);
        let mut numbers = [0.4, 0.9, 0.9, 0.9, 0.05, 0.5].into_iter();
        assert_eq!(draw_pink(&pink, || numbers.next().unwrap()), Some((31, 2)));
        assert!(numbers.next().is_none(), "four firing numbers, one aptitude, one stars");
        // no grade at A or S: no pink, after the firing numbers only
        let pink = recorded([6; 10], vec![PinkFire { aptitude: 0, chance: 0.5, slot: 0, stars: 3 }]);
        let mut numbers = [0.9, 0.9].into_iter();
        assert_eq!(draw_pink(&pink, || numbers.next().unwrap()), None);
        assert!(numbers.next().is_none());
    }

    #[test]
    fn pink_chances_split_pink_gen_by_stars() {
        let mut start = [6; 10];
        start[1] = 7;
        let pink = recorded(start, vec![PinkFire { aptitude: 2, chance: 0.4, slot: 0, stars: 3 }]);
        let pink_gen = CareerSetup::pink_gen(&pink);
        let chances = CareerSetup::pink_chances(&pink);
        let total: f64 = chances.iter().map(|pink| pink.chance).sum();
        assert!((total - pink_gen.iter().sum::<f64>()).abs() < 1e-15);
        // mile at A for sure, middle at A with chance 1 - .6^2 = .64: mile is hers with .36 + .64 / 2
        assert!((pink_gen[1] - 0.68).abs() < 1e-12, "{pink_gen:?}");
        let mile: Vec<_> = chances.iter().filter(|pink| pink.group == 32).collect();
        assert_eq!(mile.iter().map(|pink| pink.stars).collect::<Vec<_>>(), [1, 2, 3]);
        assert!((mile[1].chance - 0.68 * 0.7).abs() < 1e-12);
    }
}
