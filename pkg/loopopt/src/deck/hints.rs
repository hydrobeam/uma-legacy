//! Training hints: the base draw count per scenario and the chance a card's draws miss a target.

/// An Independent Training scenario, as a job names it (a choice the job makes: the page's `scen`).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, serde::Serialize, serde::Deserialize, ts_rs::TS)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Scenario {
    #[default]
    Unity,
    Gc,
    Ura,
    Trackblazer,
    /// the JP Grand Masters era's
    Grandmasters,
}

impl Scenario {
    /// Its name in a job, which also keys the card table's bans (`CardsRaw::restrict`).
    pub(super) const fn key(self) -> &'static str {
        match self {
            Self::Unity => "unity",
            Self::Gc => "gc",
            Self::Ura => "ura",
            Self::Trackblazer => "trackblazer",
            Self::Grandmasters => "grandmasters",
        }
    }
    /// Its display name.
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Unity => "Unity Cup",
            Self::Gc => "Grand Concert",
            Self::Ura => "URA Finale",
            Self::Trackblazer => "Trackblazer",
            Self::Grandmasters => "Grand Masters",
        }
    }
}
// The bases and specialty weights: research/results.json's section 7 fit, rounded to 3 decimals by build.rs.
include!(concat!(env!("OUT_DIR"), "/hint_constants.rs"));

/// A card's training-hint draws per run at hint frequency 0 and specialty priority 0 (docs/RESEARCH.md section 7).
pub(crate) const fn hint_base(scenario: Scenario) -> f64 {
    match scenario {
        Scenario::Gc => BASE_GRAND_CONCERT,
        Scenario::Ura => BASE_URA_FINALE,
        Scenario::Trackblazer => BASE_TRACKBLAZER,
        Scenario::Grandmasters => BASE_GRAND_MASTERS,
        Scenario::Unity => BASE_UNITY_CUP,
    }
}
/// Specialty priority's weight in the draws: h scales by 1 + d x specialty/100 (fitted with `hint_base`).
pub(crate) const fn hint_spec(scenario: Scenario) -> f64 {
    match scenario {
        Scenario::Gc => SPEC_GRAND_CONCERT,
        Scenario::Ura => SPEC_URA_FINALE,
        Scenario::Trackblazer => SPEC_TRACKBLAZER,
        Scenario::Grandmasters => SPEC_GRAND_MASTERS,
        Scenario::Unity => SPEC_UNITY_CUP,
    }
}
/// P(none of the target skills is drawn) in `draws` training-hint draws over `pool_size` skills, each draw
/// uniform over the skills still below Lv 5; a skill leaves the pool after `cap` draws (ceil(5 / hint levels per
/// draw)), so a +5 card never draws a skill twice. Exact over how many draws each other skill has taken.
pub(super) fn miss_draws(target_count: usize, pool_size: usize, draws: usize, cap: usize) -> f64 {
    if target_count == 0 {
        return 1.0;
    }
    if target_count >= pool_size {
        return if draws == 0 { 1.0 } else { 0.0 };
    }
    if draws <= cap {
        return (1.0 - target_count as f64 / pool_size as f64).powi(draws as i32);
    } // no skill can leave the pool before the last draw
    // state: how many of the other skills have taken 0, 1, .., cap draws (cap: left the pool); a sorted map, so
    // sums run in one order rather than depend on a HashMap's iteration order
    let mut states: std::collections::BTreeMap<Vec<u8>, f64> = std::collections::BTreeMap::default();
    let mut initial_counts = vec![0u8; cap + 1];
    initial_counts[0] = (pool_size - target_count) as u8;
    states.insert(initial_counts, 1.0);
    for _ in 0..draws {
        let mut next_states: std::collections::BTreeMap<Vec<u8>, f64> = std::collections::BTreeMap::default();
        for (counts, probability) in &states {
            let available = (pool_size - target_count - counts[cap] as usize + target_count) as f64; // target_count > 0: never empty
            for draws_taken in 0..cap {
                if counts[draws_taken] == 0 {
                    continue;
                }
                let mut next_counts = counts.clone();
                next_counts[draws_taken] -= 1;
                next_counts[draws_taken + 1] += 1;
                *next_states.entry(next_counts).or_insert(0.0) +=
                    probability * f64::from(counts[draws_taken]) / available;
            }
        }
        states = next_states;
    }
    states.values().sum()
}
/// `miss_draws` at a fractional expected draw count: IT gives a near-fixed whole number of draws per setup.
/// Use the floor or floor + 1 draws, with the fractional part as the weight of the second.
pub(crate) fn hint_miss(target_count: usize, pool_size: usize, expected_draws: f64, hint_levels: u32) -> f64 {
    let cap = 5_usize.div_ceil(hint_levels.max(1) as usize);
    let whole_draws = expected_draws.max(0.0).floor();
    let fraction = expected_draws - whole_draws;
    let whole_draws = whole_draws as usize;
    (1.0 - fraction) * miss_draws(target_count, pool_size, whole_draws, cap)
        + if fraction > 0.0 { fraction * miss_draws(target_count, pool_size, whole_draws + 1, cap) } else { 0.0 }
}
