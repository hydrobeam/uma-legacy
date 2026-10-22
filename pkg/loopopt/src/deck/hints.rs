//! Training hints: the base draw count per scenario and the chance a card's draws miss a target.

/// A scenario's display name; unknown names pass through unchanged.
pub(super) fn scenario_name(scenario: &str) -> &str {
    match scenario {
        "unity" => "Unity Cup",
        "gc" => "Grand Concert",
        "ura" => "URA Finale",
        "trackblazer" => "Trackblazer",
        "grandmasters" => "Grand Masters",
        other => other,
    }
}
// The bases and specialty weights: research/results.json's section 7 fit, rounded to 3 decimals by build.rs.
include!(concat!(env!("OUT_DIR"), "/hint_constants.rs"));

/// A card's training-hint draws per run at hint frequency 0 and specialty priority 0 (docs/RESEARCH.md section 7);
/// an unknown scenario reads as Unity Cup.
pub(crate) fn hint_base(scenario: &str) -> f64 {
    match scenario {
        "gc" => BASE_GRAND_CONCERT,
        "ura" => BASE_URA_FINALE,
        "trackblazer" => BASE_TRACKBLAZER,
        "grandmasters" => BASE_GRAND_MASTERS,
        _ => BASE_UNITY_CUP,
    }
}
/// Specialty priority's weight in the draws: h scales by 1 + d x specialty/100 (fitted with `hint_base`).
pub(crate) fn hint_spec(scenario: &str) -> f64 {
    match scenario {
        "gc" => SPEC_GRAND_CONCERT,
        "ura" => SPEC_URA_FINALE,
        "trackblazer" => SPEC_TRACKBLAZER,
        "grandmasters" => SPEC_GRAND_MASTERS,
        _ => SPEC_UNITY_CUP,
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
