//! Training hints: the base draw count per scenario and the chance a card's draws miss a target.

/// A scenario's display name. (Training-hint draws per run by scenario are `hint_base` and `hint_spec` below:
/// h = base x (1 + hf/100) x (1 + d x specialty/100), fitted 2026-10-11 on 3,179 runs with the Lv-5 cap,
/// docs/RESEARCH.md section 7; Grand Masters (JP) has no runs: the pooled fit, a placeholder.)
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
pub(crate) fn hint_base(scenario: &str) -> f64 {
    // fitted on uma-it IT runs (docs/RESEARCH.md section 7); Grand Masters has no data: the pooled fit, a placeholder
    match scenario {
        "gc" => 1.97,
        "ura" => 1.34,
        "trackblazer" => 1.70,
        "grandmasters" => 1.86,
        _ => 2.36,
    }
}
/// Specialty priority's weight in the draws: h scales by 1 + d x specialty/100 (fitted with `hint_base`).
pub(crate) fn hint_spec(scenario: &str) -> f64 {
    match scenario {
        "gc" => 0.08,
        "ura" => 0.39,
        "trackblazer" => 0.11,
        "grandmasters" => 0.26,
        _ => 0.36,
    }
}
/// P(none of `k` given skills of a card's pool is drawn) in `t` training-hint draws over its `n` skills, each draw
/// uniform over the skills still below Lv 5; a skill leaves the pool after `cap` draws (ceil(5 / hint levels per
/// draw)), so a +5 card never draws a skill twice. Exact over how many draws each other skill has taken.
pub(super) fn miss_draws(k: usize, n: usize, t: usize, cap: usize) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if k >= n {
        return if t == 0 { 1.0 } else { 0.0 };
    }
    if t <= cap {
        return (1.0 - k as f64 / n as f64).powi(t as i32);
    } // no skill can leave the pool before the last draw
    // state: how many of the n - k other skills have taken 0, 1, .., cap draws (cap: left the pool); a sorted map, so
    // sums run in one order (a std HashMap's order changes from run to run, and so did the last bits of every result)
    let mut states: std::collections::BTreeMap<Vec<u8>, f64> = std::collections::BTreeMap::default();
    let mut initial_counts = vec![0u8; cap + 1];
    initial_counts[0] = (n - k) as u8;
    states.insert(initial_counts, 1.0);
    for _ in 0..t {
        let mut next_states: std::collections::BTreeMap<Vec<u8>, f64> = std::collections::BTreeMap::default();
        for (counts, probability) in &states {
            let available = (n - k - counts[cap] as usize + k) as f64; // k > 0: never empty
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
/// `miss_draws` at a fractional draw count h: IT gives a near-fixed whole number of draws per setup, so h is read as
/// floor(h) or floor(h) + 1 draws with the fractional part as the weight of the second.
pub(crate) fn hint_miss(k: usize, n: usize, h: f64, hinc: u32) -> f64 {
    let cap = 5_usize.div_ceil(hinc.max(1) as usize);
    let whole_draws = h.max(0.0).floor();
    let fraction = h - whole_draws;
    let whole_draws = whole_draws as usize;
    (1.0 - fraction) * miss_draws(k, n, whole_draws, cap)
        + if fraction > 0.0 { fraction * miss_draws(k, n, whole_draws + 1, cap) } else { 0.0 }
}
