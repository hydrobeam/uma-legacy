//! Affinity-core correctness tests against independent fixtures and model invariants.
//! Skill, deck and career tests stay beside those modules.
// ---------------------------------------------------------------------------------------------
// Correctness tests of the affinity core. Reference values in tests/fixtures/ were produced independently of this code
// (exact MILP, scipy/HiGHS: val/milp_fixtures.py, rerun after a change to the win model) on the frozen data_v1.json:
//   br_milp.json, br_rest_milp.json  best responses, incl. forced races and rests and the special-goal umas
//   pb_milp.json                     pair ceilings
// Run with: cargo test --release
// ---------------------------------------------------------------------------------------------
use crate::chart;
use crate::data::{Choice, Data, NT, Sched, Uma, dot, load, load_str};
use crate::loop_planner::schedules::{C_PAIR, FILL_EPS, LoopProblem, optimize, pair_ceilings};
use crate::uma::UmaIndex;
use std::sync::OnceLock;
fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json")))
}
fn fixture(name: &str) -> serde_json::Value {
    let path = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn ix(data: &Data, id: u32) -> UmaIndex {
    UmaIndex(data.umas.iter().position(|uma| uma.id == id).unwrap())
}
/// The option with race program `program_id` at 1-based turn `turn`.
fn opt_at(uma: &Uma, turn: usize, program_id: u32) -> Choice {
    Choice::Opt(uma.race_options[turn - 1].iter().position(|option| option.program_id == program_id).unwrap() as u8)
}
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn unit(&mut self) -> f64 {
        self.next() as f64 / (1u64 << 31) as f64
    }
}

/// Streak positions: consecutive race turns raise k; a rest resets it.
#[test]
fn streak_positions() {
    let d = data();
    let digital = &d.umas[ix(d, 1019)]; // Agnes Digital: dirt A, long G
    // Digital: Japan Cup classic (t46) then Champions Cup (t47) then Tokyo Daishoten (t48): k = 1,2,3
    let mut schedule = [Choice::Rest; NT];
    for (t, name) in [(46, "Japan Cup"), (47, "Champions Cup"), (48, "Tokyo Daishoten")] {
        let o = digital.race_options[t - 1].iter().position(|op| op.name == name).unwrap();
        schedule[t - 1] = Choice::Opt(o as u8);
    }
    let rows = chart::chart_rows(d, &[ix(d, 1019)], &[schedule]);
    let positions: Vec<u64> =
        rows.as_array().unwrap().iter().map(|row| row["cells"][0]["row"].as_u64().unwrap()).collect();
    assert_eq!(positions, vec![1, 2, 3]);
}

/// The exact DP matches the MILP fixtures with optional forced races and forced rests; returned schedules are
/// feasible and honor the force. Fixtures include turn-49 goals, multi-turn goals and repeated races.
#[test]
fn best_response_matches_milp() {
    let d = data();
    let mut cases = 0;
    for (file, rest) in [("br_milp.json", false), ("br_rest_milp.json", true)] {
        for case in fixture(file).as_array().unwrap() {
            let uma = &d.umas[ix(d, case["uma"].as_u64().unwrap() as u32)];
            let mut weights = vec![0.0; d.race_count];
            for entry in case["weights"].as_array().unwrap() {
                weights[entry[0].as_u64().unwrap() as usize] = entry[1].as_f64().unwrap();
            }
            let force = if rest {
                Some((case["force_rest"].as_u64().unwrap() as usize - 1, Choice::Rest))
            } else {
                case["force"].as_array().map(|forced| {
                    let t = forced[0].as_u64().unwrap() as usize;
                    (t - 1, opt_at(uma, t, forced[1].as_u64().unwrap() as u32))
                })
            };
            let (schedule, value) = d.best_response(uma, &weights, force).expect("infeasible");
            let want = case["milp_value"].as_f64().unwrap();
            assert!(
                (value - want).abs() < 1e-6,
                "{} {} {:?}: dp {value} vs milp {want}",
                uma.name,
                case["kind"],
                force
            );
            assert!(uma.feasible(&schedule));
            if let Some((t, choice)) = force {
                assert_eq!(schedule[t], choice);
            }
            cases += 1;
        }
    }
    assert!(cases >= 260, "{cases} cases");
}

/// A force that conflicts with a single-turn goal must be infeasible, not silently drop the goal.
#[test]
fn force_conflicting_with_goal_is_infeasible() {
    let d = data();
    let weights = vec![1.0; d.race_count];
    let urara = &d.umas[ix(d, 1052)];
    assert!(urara.forced[71].is_some(), "Urara has a goal at turn 72");
    assert!(d.best_response(urara, &weights, Some((71, Choice::Rest))).is_none());
    let chiyono = &d.umas[ix(d, 1069)];
    // turn 23 goal for Chiyono O is Asahi Hai FS; forcing Hanshin JF (program 623) must fail
    assert!(d.best_response(chiyono, &weights, Some((22, opt_at(chiyono, 23, 623)))).is_none());
}

/// Uma::feasible rejects bad schedules (it is not vacuously true).
#[test]
fn feasible_rejects_bad_schedules() {
    let d = data();
    let daiwa = &d.umas[ix(d, 1009)];
    let weights = vec![1.0; d.race_count];
    let (schedule, _) = d.best_response(daiwa, &weights, None).unwrap();
    assert!(daiwa.feasible(&schedule));
    // drop a goal race
    let mut goal_turn = 0;
    for t in 0..NT {
        if daiwa.forced[t].is_some() {
            goal_turn = t;
            break;
        }
    }
    let mut missing_goal = schedule;
    missing_goal[goal_turn] = Choice::Rest;
    assert!(!daiwa.feasible(&missing_goal), "missing goal accepted");
    // option index out of range
    let mut missing_option = schedule;
    missing_option[5] = Choice::Opt(9);
    assert!(!daiwa.feasible(&missing_option), "nonexistent option accepted");
    // multi-turn goal alternative: Fuji Kiseki must run NHK Mile (t33) or the Derby (t34)
    let fuji = &d.umas[ix(d, 1005)];
    let (fuji_schedule, _) = d.best_response(fuji, &weights, None).unwrap();
    let mut neither = fuji_schedule;
    neither[32] = Choice::Rest;
    neither[33] = Choice::Rest;
    assert!(!fuji.feasible(&neither), "missing multi-turn goal alternative accepted");
}

/// A stored pairwise-ceiling table that matches its data.json is bit-equal to a fresh computation, and a table
/// stops matching when the data changes. (A stale table is only a warning: the solver ignores it.)
#[test]
fn stored_pb_tables_match_fresh() {
    for path in [
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/jp/data.json"),
    ] {
        let text = std::fs::read_to_string(path).unwrap();
        let d = load_str(&text);
        let Some(stored) = d.pair_ceilings.as_ref() else {
            eprintln!("warning: {path} has no current pb table (run ./build.sh pb)");
            continue;
        };
        let mut fresh = load_str(&text);
        fresh.pair_ceilings = None;
        let roster: Vec<UmaIndex> = d.umas.indices().collect();
        let ceilings = pair_ceilings(&fresh, &roster);
        for a in d.umas.indices() {
            for b in d.umas.indices() {
                if a != b {
                    assert_eq!(stored[(a, b)].to_bits(), ceilings[a][b].to_bits(), "{path} PB({a},{b})");
                }
            }
        }
        let mut changed: serde_json::Value = serde_json::from_str(&text).unwrap();
        changed["pen"][2] = serde_json::json!(changed["pen"][2].as_i64().unwrap() + 1);
        assert!(load_str(&changed.to_string()).pair_ceilings.is_none(), "{path}: a table survived a data change");
    }
}

/// Pair ceilings equal the MILP-computed table.
#[test]
fn pair_ceilings_match_milp() {
    let d = data();
    let roster: Vec<UmaIndex> = d.umas.indices().collect();
    let ceilings = pair_ceilings(d, &roster);
    for entry in fixture("pb_milp.json").as_array().unwrap() {
        let (a, b) = (ix(d, entry[0].as_u64().unwrap() as u32), ix(d, entry[1].as_u64().unwrap() as u32));
        assert!((ceilings[a][b] - entry[2].as_f64().unwrap()).abs() < 1e-9);
    }
}

/// The objective: LoopProblem::score equals a direct per-legacy sum (umaishow's split, no rounding) over the 12
/// self-copy-free lineages, grandparent slots at half weight, normalized to 4 runs; checks the closed-form base and
/// race weight.
#[test]
fn score_matches_per_legacy_sum() {
    let d = data();
    let mut rng = Lcg(2024);
    let pair = |a: UmaIndex, b: UmaIndex| f64::from(d.pair_relations[a][b]);
    let trio = |a: UmaIndex, b: UmaIndex, c: UmaIndex| {
        if a == c || b == c { 0.0 } else { f64::from(d.trio_relations[a][b][c]) }
    };
    for _ in 0..60 {
        let mut members = [UmaIndex(0); 4];
        loop {
            for member in &mut members {
                *member = UmaIndex((rng.next() as usize) % d.umas.len());
            }
            let mut sorted = members;
            sorted.sort_unstable();
            if sorted.windows(2).all(|w| w[0] != w[1]) {
                break;
            }
        }
        let schedules: Vec<Sched> = (0..4)
            .map(|i| {
                let weights: Vec<f64> = (0..d.race_count).map(|_| rng.unit() * 3.0).collect();
                d.best_response(&d.umas[members[i]], &weights, None).unwrap().0
            })
            .collect();
        let chances: Vec<Vec<f64>> = (0..4).map(|i| d.win_chances(&d.umas[members[i]], &schedules[i])).collect();
        let overlap = |a: usize, b: usize| dot(&chances[a], &chances[b]);
        let um = |i: usize| members[i];
        let w = 0.5;
        // lineage affinity: parents (T,P1,P2), P1's grandparents g1, P2's grandparents g2 (indices into members)
        let lineage = |t: usize, p1: usize, p2: usize, g1: [usize; 2], g2: [usize; 2]| -> f64 {
            let parents = pair(um(t), um(p1)) + pair(um(t), um(p2)) + pair(um(p1), um(p2)) + 3.0 * overlap(p1, p2);
            let mut grandparents = 0.0;
            for &g in &g1 {
                grandparents += trio(um(t), um(p1), um(g)) + 3.0 * overlap(p1, g);
            }
            for &g in &g2 {
                grandparents += trio(um(t), um(p2), um(g)) + 3.0 * overlap(p2, g);
            }
            parents + w * grandparents
        };
        // P1 bred from (P2, X), P2 bred from (P1, X); normalized to 4 runs
        let mut per_legacy = 0.0;
        for t in 0..4 {
            for x in 0..4 {
                if x == t {
                    continue;
                }
                let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
                per_legacy += lineage(t, ps[0], ps[1], [ps[1], x], [ps[0], x]);
            }
        }
        per_legacy /= 3.0;
        let score = LoopProblem::new(d, members).score(&[schedules[0], schedules[1], schedules[2], schedules[3]]);
        assert!((score - per_legacy).abs() < 1e-9, "score {score} vs per-legacy {per_legacy}");
    }
}

/// Optional G1s (run by one member only) cost no affinity: stripping them from optimized schedules leaves the score
/// unchanged (within the fill tolerance).
/// The career schedule's tie-break (`FILL_EPS` on every G1, career/schedule.rs) never trades the parents' races for a G1
/// neither parent won: on each uma, with every third G1 weighted as a parent's win, the schedule solved with the
/// tie-break scores the parents' races within its total bonus of the schedule solved without it.
#[test]
fn tie_break_costs_the_parents_races_nothing() {
    let d = data();
    for uma in &d.umas {
        let parents: Vec<f64> = (0..d.race_count).map(|race| if race % 3 == 0 { 1.0 } else { 0.0 }).collect();
        let with_tie_break: Vec<f64> = parents.iter().map(|weight| weight + FILL_EPS).collect();
        let (Some((best, _)), Some((tied, _))) =
            (d.best_response(uma, &parents, None), d.best_response(uma, &with_tie_break, None))
        else {
            continue;
        };
        let best_value = dot(&parents, &d.win_chances(uma, &best));
        let tied_value = dot(&parents, &d.win_chances(uma, &tied));
        let bonus = FILL_EPS * d.race_count as f64;
        assert!(
            tied_value >= best_value - bonus,
            "uma {}: the tie-break lost {} of her parents' races",
            uma.id,
            best_value - tied_value
        );
    }
}

#[test]
fn optional_g1s_cost_nothing() {
    let d = data();
    let cases: [[u32; 4]; 4] =
        [[1007, 1019, 1021, 1049], [1009, 1014, 1019, 1029], [1015, 1017, 1023, 1030], [1006, 1019, 1029, 1034]];
    for ids in cases {
        let members = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
        let optimized = optimize(d, members, 3, 5, 60.0).schedules;
        let problem = LoopProblem::new(d, members);
        let value = problem.score(&optimized);
        let mut runners = vec![0u8; d.race_count];
        for i in 0..4 {
            let mut seen = vec![false; d.race_count];
            for t in 0..NT {
                if let Choice::Opt(o) = optimized[i][t] {
                    let race = d.umas[members[i]].race_options[t][o as usize].race;
                    if race >= 0 && !seen[race as usize] {
                        seen[race as usize] = true;
                        runners[race as usize] += 1;
                    }
                }
            }
        }
        let mut stripped = optimized;
        for i in 0..4 {
            for t in 0..NT {
                if let Choice::Opt(o) = stripped[i][t] {
                    let op = &d.umas[members[i]].race_options[t][o as usize];
                    if op.race >= 0 && runners[op.race as usize] == 1 && !op.goal {
                        stripped[i][t] = Choice::Rest;
                    }
                }
            }
        }
        for i in 0..4 {
            assert!(d.umas[members[i]].feasible(&stripped[i]));
        }
        let stripped_value = problem.score(&stripped);
        assert!((stripped_value - value).abs() < 1e-5, "{ids:?}: stripped {stripped_value} vs {value}");
    }
}

/// A three-member rotation's schedules: the score is the base plus 3·C·(the sum of its three pairs' shared wins),
/// no member gains from re-solving against the other two (a Nash point of the uniform objective), the schedules are
/// feasible, and the unused fourth slot is empty.
#[test]
fn three_member_schedules() {
    let d = data();
    for ids in [[1019u32, 1021, 1034], [1006, 1019, 1029], [1007, 1009, 1049]] {
        let members = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2])];
        let result = optimize(d, members, 5, 3, 60.0);
        assert!(result.schedules[3].iter().all(|c| *c == Choice::Rest));
        let schedules: [Sched; 3] = [result.schedules[0], result.schedules[1], result.schedules[2]];
        for i in 0..3 {
            assert!(d.umas[members[i]].feasible(&schedules[i]));
        }
        let problem = LoopProblem::new(d, members);
        let chances: Vec<Vec<f64>> = (0..3).map(|i| d.win_chances(&d.umas[members[i]], &schedules[i])).collect();
        let direct = problem.base
            + 3.0
                * C_PAIR
                * (dot(&chances[0], &chances[1]) + dot(&chances[0], &chances[2]) + dot(&chances[1], &chances[2]));
        assert!((direct - problem.score(&schedules)).abs() < 1e-9, "{ids:?}");
        for i in 0..3 {
            let weights = problem.weights(&chances, i);
            let (_, best) = d.best_response(&d.umas[members[i]], &weights, None).unwrap();
            let current = dot(&weights, &chances[i]);
            assert!(best <= current + 1e-6, "{ids:?}: member {i} gains {}", best - current);
        }
    }
}

/// Races flagged interchangeable really are: switching every runner to the alternative leaves the score unchanged.
#[test]
fn interchangeable_swaps_preserve_score() {
    let d = data();
    let mut checked = 0;
    for ids in
        [[1007u32, 1019, 1021, 1049], [1009, 1014, 1019, 1029], [1006, 1019, 1029, 1034], [1015, 1017, 1023, 1030]]
    {
        let members = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
        let result = optimize(d, members, 3, 5, 60.0);
        let problem = LoopProblem::new(d, members);
        let value = problem.score(&result.schedules);
        for t in 0..NT {
            let mut races: Vec<i32> = vec![];
            for i in 0..4 {
                if let Choice::Opt(o) = result.schedules[i][t] {
                    let race = d.umas[members[i]].race_options[t][o as usize].race;
                    if race >= 0 && !races.contains(&race) {
                        races.push(race);
                    }
                }
            }
            for race in races {
                for alternative in chart::interchangeable(d, &members, &result.schedules, t, race) {
                    let mut swapped = result.schedules;
                    for i in 0..4 {
                        if let Choice::Opt(o) = swapped[i][t]
                            && d.umas[members[i]].race_options[t][o as usize].race == race
                        {
                            let k = d.umas[members[i]].race_options[t]
                                .iter()
                                .position(|op| op.race == alternative)
                                .unwrap();
                            swapped[i][t] = Choice::Opt(k as u8);
                        }
                    }
                    for i in 0..4 {
                        assert!(d.umas[members[i]].feasible(&swapped[i]));
                    }
                    assert!(
                        (problem.score(&swapped) - value).abs() < 1e-9,
                        "swap {race}->{alternative} at {t} changed the score"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert!(checked > 0);
}

/// The shipped data uses the community win table (Oct 2026): 110 - grade costs (A 0, B 10, C 20, D 30, E 40,
/// F 60, G 90) minus 0/0/5/20/30/50 for the 1st..6th+ race in a row, capped to 0..100%. Checks every cell.
#[test]
fn win_table_matches_community_table() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"));
    let table: [(i32, [i32; 6]); 13] = [
        (0, [110, 110, 105, 90, 80, 60]),
        (10, [100, 100, 95, 80, 70, 50]),
        (20, [90, 90, 85, 70, 60, 40]),
        (30, [80, 80, 75, 60, 50, 30]),
        (40, [70, 70, 65, 50, 40, 20]),
        (60, [50, 50, 45, 30, 20, 0]),
        (90, [20, 20, 15, 0, 0, 0]),
        (100, [10, 10, 5, 0, 0, 0]),
        (110, [0; 6]),
        (120, [0; 6]),
        (130, [0; 6]),
        (150, [0; 6]),
        (180, [0; 6]),
    ];
    for (cost, vals) in table {
        let win_rate_basis = (110 - cost).max(-20);
        for k in 1..=6 {
            assert_eq!(
                d.attempt_chance(win_rate_basis, k),
                f64::from(vals[k - 1].clamp(0, 100)) / 100.0,
                "cost {cost} k {k}"
            );
        }
    }
}

/// A goal race takes the previous streak position's penalty (0/0/0/5/20/30 in the shipped data), a chosen race its
/// own; every option's precomputed chances, which the DP reads, follow that rule.
#[test]
fn goal_race_takes_previous_position() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"));
    let goal_penalties: Vec<i32> = (1..=6).map(|k| d.streak_penalty(true, k)).collect();
    let chosen_penalties: Vec<i32> = (1..=6).map(|k| d.streak_penalty(false, k)).collect();
    assert_eq!(goal_penalties, [0, 0, 0, 5, 20, 30]);
    assert_eq!(chosen_penalties, [0, 0, 5, 20, 30, 50]);
    let mut goal_options = 0;
    for uma in &d.umas.0 {
        for (t, options) in uma.race_options.iter().enumerate() {
            for (o, option) in options.iter().enumerate() {
                goal_options += usize::from(option.goal && option.race >= 0);
                for k in 1..=6 {
                    assert_eq!(
                        uma.choices[t][o + 1].win_chances[k],
                        d.option_chance(option, k),
                        "{} turn {t} option {o}",
                        uma.name
                    );
                }
            }
        }
    }
    assert!(goal_options > 0, "no G1 goal options in the data");
}
