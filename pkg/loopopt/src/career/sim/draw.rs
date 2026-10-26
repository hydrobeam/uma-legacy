//! Keyed uniforms and veteran draws (landed set, stars, wins, pink), including the pinned random-number test.
//! Policy selection and path execution live in the sibling modules.

use crate::career::schedule::{PinkCareer, apt_pink};
use crate::career::sim::play::PathState;
use crate::career::sim::{Cur, Sim};
use crate::career::{Ancestor, Legacy, Lineage};

// ---- random numbers ----
const fn fmix(mut hash: u32) -> u32 {
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash
}
/// 53 bits in [0, 1) from the keys
fn u01(keys: &[u32]) -> f64 {
    let (mut high_bits, mut low_bits) = (0x9e37_79b9u32, 0x7f4a_7c15u32);
    for &key in keys {
        high_bits = fmix((high_bits ^ key).wrapping_add(0x632b_e5ab));
        low_bits = fmix(low_bits.wrapping_add(key.wrapping_mul(0x27d4_eb2f)) ^ high_bits);
    }
    (f64::from(high_bits) * 2f64.powi(21) + f64::from(low_bits >> 11)) / 2f64.powi(53)
}
const STREAM_SET: u32 = 1;
const STREAM_STAR: u32 = 2;
const STREAM_WIN: u32 = 3;
const STREAM_PINK: u32 = 4;
/// Whose uniforms: a run's own path
#[derive(Clone, Copy)]
pub(super) enum Keys {
    Main { seed: u32, run: u32 },
}
impl Keys {
    pub(super) fn uniform(&self, career_index: usize, stream: u32, item: usize) -> f64 {
        match *self {
            Self::Main { seed, run } => u01(&[seed, run, career_index as u32, stream, item as u32]),
        }
    }
}

/// Which landed sets a career's veteran is drawn from (module notes in sim/mod.rs).
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Outcomes {
    /// The failed sets: a finishing career, whose success is kept analytic in the survival.
    Failed,
    /// Every set, the all-landed one last: a career whose success does not end the run (goal `looped`).
    Any,
}

/// One veteran draw: whose uniforms, at which career, over which landed sets.
#[derive(Clone, Copy)]
pub(super) struct VeteranDraw {
    pub(super) keys: Keys,
    pub(super) career_index: usize,
    pub(super) outcomes: Outcomes,
}

/// The April inspirations a legacy's pink gets a chance to fire at (`schedule.rs`, `INSP_TURNS`; webapp/src/pink.mjs).
const INSPIRATIONS: usize = 2;
/// Aptitude codes 1..8 = G..S.
const GRADE_A: u8 = 7;
const GRADE_S: u8 = 8;

/// Her pink spark drawn as the page's roll draws it (webapp/src/pink.mjs `drawPink`, docs/MODEL.md *Sparks the odds
/// don't use*): from her start grades each legacy pink fires at each inspiration with its chance (+1 grade, at most S),
/// then one aptitude uniform over those she ends at A or S, stars 20/70/10; none when she ends with none. `uniform(i)`:
/// the i-th uniform of the draw, in pink.mjs's order (every fire chance per inspiration, then the aptitude, then the
/// stars).
// TODO(integration): swap for the model lane's shared helper, `CareerSetup::draw_veteran_pink(pool, lineage, outfit,
// uniform)` (career/schedule.rs: the career's outfit from `Cur`'s record, `uniform` an FnMut over this draw's keyed
// uniforms), so the plan's hypothetical veterans and these draws share one definition. Its order is this one's (fires
// per inspiration, the aptitude, the stars; one more number first only for a veteran whose result is not known, which
// the simulator's pools never hold), so the swap keeps the order of the uniforms (its grades are the outfit's).
fn draw_pink(pink_career: &PinkCareer, uniform: impl Fn(usize) -> f64) -> Option<(u32, u8)> {
    let mut grades = pink_career.start;
    let mut item = 0;
    let mut next_uniform = || {
        item += 1;
        uniform(item - 1)
    };
    for _ in 0..INSPIRATIONS {
        for fire in &pink_career.fire {
            if next_uniform() < fire.chance {
                grades[fire.aptitude] = (grades[fire.aptitude] + 1).min(GRADE_S);
            }
        }
    }
    let at_least_a: Vec<usize> = (0..grades.len()).filter(|&aptitude| grades[aptitude] >= GRADE_A).collect();
    if at_least_a.is_empty() {
        return None;
    }
    let pick = (next_uniform() * at_least_a.len() as f64) as usize;
    let star_draw = next_uniform();
    let stars = if star_draw < 0.2 {
        1
    } else if star_draw < 0.9 {
        2
    } else {
        3
    };
    Some((apt_pink(at_least_a[pick.min(at_least_a.len() - 1)]), stars))
}

impl Sim<'_, '_> {
    /// the veteran a career leaves: her landed set drawn from `draw.outcomes`' sets, her stars from the rank's mix,
    /// her wins from her schedule's chances, her pink as the page draws it (`draw_pink`)
    pub(super) fn draw(&self, career: &Cur, path: &PathState, draw: VeteranDraw) -> Legacy {
        let VeteranDraw { keys, career_index, outcomes } = draw;
        let full_set = career.outcome_sets.len() - 1;
        let full_probability = if outcomes == Outcomes::Any { career.outcome_sets[full_set] } else { 0.0 };
        let failed_probability: f64 =
            self.failed_sets.iter().map(|&outcome_mask| career.outcome_sets[outcome_mask]).sum();
        let mut remaining_mass = keys.uniform(career_index, STREAM_SET, 0) * (failed_probability + full_probability);
        let (mut landed_mask, mut last_drawable_mask) = (usize::MAX, usize::MAX);
        // (the all-landed set last: it has the most targets, so this is still the fewest-landed-first order)
        let drawable = self.failed_sets.iter().copied().chain((outcomes == Outcomes::Any).then_some(full_set));
        for outcome_mask in drawable {
            if career.outcome_sets[outcome_mask] <= 0.0 {
                continue;
            }
            last_drawable_mask = outcome_mask;
            if remaining_mass < career.outcome_sets[outcome_mask] {
                landed_mask = outcome_mask;
                break;
            }
            remaining_mass -= career.outcome_sets[outcome_mask];
        }
        if landed_mask == usize::MAX {
            landed_mask = last_drawable_mask;
        }
        let mut sparks = vec![];
        for (i, target) in self.env.setup.skill_job.targets.iter().enumerate() {
            if landed_mask >> i & 1 == 0 {
                continue;
            }
            let star_draw = keys.uniform(career_index, STREAM_STAR, i);
            let stars = if star_draw < self.star_mix[0] {
                1
            } else if star_draw < self.star_mix[0] + self.star_mix[1] {
                2
            } else {
                3
            };
            sparks.push((target.group, stars));
        }
        let wins: Vec<usize> = career
            .win_chances
            .iter()
            .enumerate()
            .filter(|&(race_index, &win_chance)| {
                win_chance > 0.0 && keys.uniform(career_index, STREAM_WIN, race_index) < win_chance
            })
            .map(|(race_index, _)| race_index)
            .collect();
        let pinks: Vec<(u32, u8)> = self
            .env
            .setup
            .pink_career(&path.pool, career.lineage)
            .and_then(|pink_career| draw_pink(&pink_career, |item| keys.uniform(career_index, STREAM_PINK, item)))
            .into_iter()
            .collect();
        let Lineage { trainee, first_parent, second_parent } = career.lineage;
        let legacies = &path.legacies;
        Legacy {
            character_id: self.data.umas[trainee].id,
            borrowed: false,
            sparks,
            wins,
            pinks,
            parents: vec![Ancestor::of_legacy(&legacies[first_parent]), Ancestor::of_legacy(&legacies[second_parent])],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::career::CareerSetup;
    use crate::career::schedule::{PinkCareer, PinkFire, apt_pink};

    #[test]
    /// the random numbers are pinned: changing them would change every recorded run
    fn u01_values() {
        assert_eq!(super::u01(&[1, 3, 0, 1, 0]), 0.006_101_473_502_044_041);
        assert_eq!(super::u01(&[1, 0, 17, 2, 5]), 0.786_756_654_747_679);
        assert_eq!(super::u01(&[7, 123_456, 99_999, 3, 33]), 0.923_524_891_523_141_8);
    }

    #[test]
    /// The simulator's pink draw agrees with the model's exact chance of each pink (`CareerSetup::pink_gen`, the
    /// chances the page's suite pins its own draw against): over 200,000 keyed draws of a career with grades at B and C
    /// that her legacies' pinks can lift, each aptitude's share is within 5 standard errors, and the stars are 20/70/10.
    fn sim_pink_draw_matches_pink_gen() {
        let fire = |aptitude: usize, chance: f64, slot: usize| PinkFire { aptitude, chance, slot, stars: 2 };
        // (codes 1..8 = G..S: two aptitudes at A, two at B and one at C that the fires can lift)
        let start = [7, 6, 5, 1, 7, 6, 1, 1, 1, 1];
        let pink_career = PinkCareer {
            base: start,
            start,
            fire: vec![fire(1, 0.3, 0), fire(1, 0.2, 1), fire(2, 0.4, 2), fire(2, 0.5, 3), fire(5, 0.1, 4)],
        };
        let exact = CareerSetup::pink_gen(&pink_career);
        let draws = 200_000u32;
        let (mut counts, mut star_counts) = ([0u32; 10], [0u32; 4]);
        for draw in 0..draws {
            if let Some((group, stars)) = super::draw_pink(&pink_career, |item| super::u01(&[9, draw, item as u32])) {
                let aptitude = (0..10).find(|&k| apt_pink(k) == group).unwrap();
                counts[aptitude] += 1;
                star_counts[stars as usize] += 1;
            }
        }
        for aptitude in 0..10 {
            let share = f64::from(counts[aptitude]) / f64::from(draws);
            let standard_error = (exact[aptitude] * (1.0 - exact[aptitude]) / f64::from(draws)).sqrt().max(1e-9);
            assert!(
                (share - exact[aptitude]).abs() <= 5.0 * standard_error,
                "aptitude {aptitude}: drawn {share}, exact {}",
                exact[aptitude]
            );
        }
        let pinks: u32 = star_counts.iter().sum();
        for (stars, expected) in [(1, 0.2), (2, 0.7), (3, 0.1)] {
            let share = f64::from(star_counts[stars]) / f64::from(pinks);
            assert!((share - expected).abs() <= 5.0 * (expected * (1.0 - expected) / f64::from(pinks)).sqrt());
        }
    }
}
