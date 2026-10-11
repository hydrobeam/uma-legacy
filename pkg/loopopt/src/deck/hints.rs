//! Training hints: the base draw count per scenario and the chance a card's draws miss a target.

/// A scenario's display name. (Training-hint draws per run by scenario are `hint_base` and `hint_spec` below:
/// h = base x (1 + hf/100) x (1 + d x specialty/100), fitted 2026-10-11 on 3,179 runs with the Lv-5 cap,
/// docs/RESEARCH.md section 6; Grand Masters (JP) has no runs: the pooled fit, a placeholder.)
pub(super) fn scenario_name(s: &str) -> &str {
    match s { "unity" => "Unity Cup", "gc" => "Grand Concert", "ura" => "URA Finale", "trackblazer" => "Trackblazer", "grandmasters" => "Grand Masters", x => x }
}
pub(crate) fn hint_base(scenario: &str) -> f64 {
    // fitted on uma-it IT runs (docs/RESEARCH.md section 6); Grand Masters has no data: the pooled fit, a placeholder
    match scenario { "gc" => 1.97, "ura" => 1.34, "trackblazer" => 1.70, "grandmasters" => 1.86, _ => 2.36 }
}
/// Specialty priority's weight in the draws: h scales by 1 + d x specialty/100 (fitted with `hint_base`).
pub(crate) fn hint_spec(scenario: &str) -> f64 {
    match scenario { "gc" => 0.08, "ura" => 0.39, "trackblazer" => 0.11, "grandmasters" => 0.26, _ => 0.36 }
}
/// P(none of `k` given skills of a card's pool is drawn) in `t` training-hint draws over its `n` skills, each draw
/// uniform over the skills still below Lv 5; a skill leaves the pool after `cap` draws (ceil(5 / hint levels per
/// draw)), so a +5 card never draws a skill twice. Exact over how many draws each other skill has taken.
pub(super) fn miss_draws(k: usize, n: usize, t: usize, cap: usize) -> f64 {
    if k == 0 { return 1.0; }
    if k >= n { return if t == 0 { 1.0 } else { 0.0 }; }
    if t <= cap { return (1.0 - k as f64 / n as f64).powi(t as i32); }   // no skill can leave the pool before the last draw
    // state: how many of the n - k other skills have taken 0, 1, .., cap draws (cap: left the pool); a sorted map, so
    // sums run in one order (a std HashMap's order changes from run to run, and so did the last bits of every result)
    let mut st: std::collections::BTreeMap<Vec<u8>, f64> = Default::default();
    let mut s0 = vec![0u8; cap + 1]; s0[0] = (n - k) as u8; st.insert(s0, 1.0);
    for _ in 0..t {
        let mut nx: std::collections::BTreeMap<Vec<u8>, f64> = Default::default();
        for (s, p) in &st {
            let avail = (n - k - s[cap] as usize + k) as f64;   // k > 0: never empty
            for j in 0..cap {
                if s[j] == 0 { continue; }
                let mut u = s.clone(); u[j] -= 1; u[j + 1] += 1;
                *nx.entry(u).or_insert(0.0) += p * s[j] as f64 / avail;
            }
        }
        st = nx;
    }
    st.values().sum()
}
/// `miss_draws` at a fractional draw count h: IT gives a near-fixed whole number of draws per setup, so h is read as
/// floor(h) or floor(h) + 1 draws with the fractional part as the weight of the second.
pub(crate) fn hint_miss(k: usize, n: usize, h: f64, hinc: u32) -> f64 {
    let cap = (5 + hinc.max(1) as usize - 1) / hinc.max(1) as usize;
    let t0 = h.max(0.0).floor(); let f = h - t0; let t0 = t0 as usize;
    (1.0 - f) * miss_draws(k, n, t0, cap) + if f > 0.0 { f * miss_draws(k, n, t0 + 1, cap) } else { 0.0 }
}
