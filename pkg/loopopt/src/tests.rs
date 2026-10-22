//! Affinity-core correctness tests against independent fixtures and model invariants.
//! Skill, deck and career tests stay beside those modules.
// ---------------------------------------------------------------------------------------------
// Correctness tests of the affinity core. Reference values in tests/fixtures/ were produced independently of this code
// (exact MILP, scipy/HiGHS: val/milp_fixtures.py, rerun after a change to the win model) on the frozen data_v1.json:
//   br_milp.json, br_rest_milp.json  best responses, incl. forced races and rests and the special-goal umas
//   pb_milp.json                     pair ceilings
// Run with: cargo test --release
// ---------------------------------------------------------------------------------------------
use crate::chart::{self, interchangeable};
use crate::data::{
    Choice, Data, NT, Sched, Uma, feasible, load, load_str, option_chance, pk, probs, streak_penalty, wval,
};
use crate::optimize::{C_PAIR, make_loop, optimize, pair_ceilings, score, weights};
use crate::schedule::best_response;
use crate::uma::UmaIndex;
use std::sync::OnceLock;
fn data() -> &'static Data {
    static D: OnceLock<Data> = OnceLock::new();
    D.get_or_init(|| load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json")))
}
fn fixture(name: &str) -> serde_json::Value {
    let p = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
    serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
}
fn ix(d: &Data, id: u32) -> UmaIndex {
    UmaIndex(d.umas.iter().position(|u| u.id == id).unwrap())
}
fn opt_at(u: &Uma, t: usize, prog: u32) -> Choice {
    Choice::Opt(u.opts[t - 1].iter().position(|o| o.prog == prog).unwrap() as u8)
}
struct Lcg(u64);
impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 33
    }
    fn f(&mut self) -> f64 {
        self.next() as f64 / (1u64 << 31) as f64
    }
}

/// Streak positions: consecutive race turns raise k; a rest resets it.
#[test]
fn streak_positions() {
    let d = data();
    let u = &d.umas[ix(d, 1019)]; // Agnes Digital: dirt A, long G
    // Digital: Japan Cup classic (t46) then Champions Cup (t47) then Tokyo Daishoten (t48): k = 1,2,3
    let mut s = [Choice::Rest; NT];
    for (t, name) in [(46, "Japan Cup"), (47, "Champions Cup"), (48, "Tokyo Daishoten")] {
        let o = u.opts[t - 1].iter().position(|op| op.name == name).unwrap();
        s[t - 1] = Choice::Opt(o as u8);
    }
    let rows = chart::chart_rows(d, &[ix(d, 1019)], &[s]);
    let ks: Vec<u64> = rows.as_array().unwrap().iter().map(|r| r["cells"][0]["row"].as_u64().unwrap()).collect();
    assert_eq!(ks, vec![1, 2, 3]);
}

/// The exact DP matches the MILP fixtures with optional forced races and forced rests; returned schedules are
/// feasible and honor the force. Fixtures include turn-49 goals, multi-turn goals and repeated races.
#[test]
fn best_response_matches_milp() {
    let d = data();
    let mut n = 0;
    for (file, rest) in [("br_milp.json", false), ("br_rest_milp.json", true)] {
        for c in fixture(file).as_array().unwrap() {
            let u = &d.umas[ix(d, c["uma"].as_u64().unwrap() as u32)];
            let mut w = vec![0.0; d.nr];
            for e in c["weights"].as_array().unwrap() {
                w[e[0].as_u64().unwrap() as usize] = e[1].as_f64().unwrap();
            }
            let force = if rest {
                Some((c["force_rest"].as_u64().unwrap() as usize - 1, Choice::Rest))
            } else {
                c["force"].as_array().map(|fa| {
                    let t = fa[0].as_u64().unwrap() as usize;
                    (t - 1, opt_at(u, t, fa[1].as_u64().unwrap() as u32))
                })
            };
            let (s, v) = best_response(d, u, &w, force).expect("infeasible");
            let want = c["milp_value"].as_f64().unwrap();
            assert!((v - want).abs() < 1e-6, "{} {} {:?}: dp {v} vs milp {want}", u.name, c["kind"], force);
            assert!(feasible(u, &s));
            if let Some((t, ch)) = force {
                assert_eq!(s[t], ch);
            }
            n += 1;
        }
    }
    assert!(n >= 260, "{n} cases");
}

/// A force that conflicts with a single-turn goal must be infeasible, not silently drop the goal.
#[test]
fn force_conflicting_with_goal_is_infeasible() {
    let d = data();
    let w = vec![1.0; d.nr];
    let urara = &d.umas[ix(d, 1052)];
    assert!(urara.forced[71].is_some(), "Urara has a goal at turn 72");
    assert!(best_response(d, urara, &w, Some((71, Choice::Rest))).is_none());
    let chiyono = &d.umas[ix(d, 1069)];
    // turn 23 goal for Chiyono O is Asahi Hai FS; forcing Hanshin JF (program 623) must fail
    assert!(best_response(d, chiyono, &w, Some((22, opt_at(chiyono, 23, 623)))).is_none());
}

/// feasible() rejects bad schedules (it is not vacuously true).
#[test]
fn feasible_rejects_bad_schedules() {
    let d = data();
    let u = &d.umas[ix(d, 1009)]; // Daiwa
    let w = vec![1.0; d.nr];
    let (s, _) = best_response(d, u, &w, None).unwrap();
    assert!(feasible(u, &s));
    // drop a goal race
    let mut t_goal = 0;
    for t in 0..NT {
        if u.forced[t].is_some() {
            t_goal = t;
            break;
        }
    }
    let mut bad = s;
    bad[t_goal] = Choice::Rest;
    assert!(!feasible(u, &bad), "missing goal accepted");
    // option index out of range
    let mut bad2 = s;
    bad2[5] = Choice::Opt(9);
    assert!(!feasible(u, &bad2), "nonexistent option accepted");
    // multi-turn goal alternative: Fuji Kiseki must run NHK Mile (t33) or the Derby (t34)
    let fk = &d.umas[ix(d, 1005)];
    let (s3, _) = best_response(d, fk, &w, None).unwrap();
    let mut bad3 = s3;
    bad3[32] = Choice::Rest;
    bad3[33] = Choice::Rest;
    assert!(!feasible(fk, &bad3), "missing multi-turn goal alternative accepted");
}

/// A stored pairwise-ceiling table that matches its data.json is bit-equal to a fresh computation, and a table
/// stops matching when the data changes. (A stale table is only a warning: the solver ignores it.)
#[test]
fn stored_pb_tables_match_fresh() {
    for path in [
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"),
        concat!(env!("CARGO_MANIFEST_DIR"), "/data/jp/data.json"),
    ] {
        let txt = std::fs::read_to_string(path).unwrap();
        let d = load_str(&txt);
        let Some(t) = d.pbt.as_ref() else {
            eprintln!("warning: {path} has no current pb table (run ./build.sh pb)");
            continue;
        };
        let mut fresh = load_str(&txt);
        fresh.pbt = None;
        let pool: Vec<UmaIndex> = d.umas.indices().collect();
        let pb = pair_ceilings(&fresh, &pool);
        for a in d.umas.indices() {
            for b in d.umas.indices() {
                if a != b {
                    assert_eq!(t[(a, b)].to_bits(), pb[a][b].to_bits(), "{path} PB({a},{b})");
                }
            }
        }
        let mut v: serde_json::Value = serde_json::from_str(&txt).unwrap();
        v["pen"][2] = serde_json::json!(v["pen"][2].as_i64().unwrap() + 1);
        assert!(load_str(&v.to_string()).pbt.is_none(), "{path}: a table survived a data change");
    }
}

/// Pair ceilings equal the MILP-computed table.
#[test]
fn pair_ceilings_match_milp() {
    let d = data();
    let all: Vec<UmaIndex> = d.umas.indices().collect();
    let pb = pair_ceilings(d, &all);
    for e in fixture("pb_milp.json").as_array().unwrap() {
        let (a, b) = (ix(d, e[0].as_u64().unwrap() as u32), ix(d, e[1].as_u64().unwrap() as u32));
        assert!((pb[a][b] - e[2].as_f64().unwrap()).abs() < 1e-9);
    }
}

/// The objective: score() equals a direct per-legacy sum (umaishow's split, no rounding) over the 12 self-copy-free
/// lineages, grandparent slots at half weight, normalized to 4 runs; checks the closed-form base and race weight.
#[test]
fn score_matches_per_legacy_sum() {
    let d = data();
    let mut rng = Lcg(2024);
    let pr = |a: UmaIndex, b: UmaIndex| f64::from(d.pair[a][b]);
    let tr = |a: UmaIndex, b: UmaIndex, c: UmaIndex| if a == c || b == c { 0.0 } else { f64::from(d.tri[a][b][c]) };
    for _ in 0..60 {
        let mut f = [UmaIndex(0); 4];
        loop {
            for x in &mut f {
                *x = UmaIndex((rng.next() as usize) % d.umas.len());
            }
            let mut g = f;
            g.sort_unstable();
            if g.windows(2).all(|w| w[0] != w[1]) {
                break;
            }
        }
        let s: Vec<Sched> = (0..4)
            .map(|i| {
                let u = &d.umas[f[i]];
                let w: Vec<f64> = (0..d.nr).map(|_| rng.f() * 3.0).collect();
                best_response(d, u, &w, None).unwrap().0
            })
            .collect();
        let p: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &s[i])).collect();
        let o = |a: usize, b: usize| wval(&p[a], &p[b]);
        let um = |i: usize| f[i];
        let w = 0.5;
        // lineage affinity: parents (T,P1,P2), P1's grandparents g1, P2's grandparents g2 (indices into f)
        let lin = |t: usize, p1: usize, p2: usize, g1: [usize; 2], g2: [usize; 2]| -> f64 {
            let par = pr(um(t), um(p1)) + pr(um(t), um(p2)) + pr(um(p1), um(p2)) + 3.0 * o(p1, p2);
            let mut gp = 0.0;
            for &g in &g1 {
                gp += tr(um(t), um(p1), um(g)) + 3.0 * o(p1, g);
            }
            for &g in &g2 {
                gp += tr(um(t), um(p2), um(g)) + 3.0 * o(p2, g);
            }
            par + w * gp
        };
        // P1 bred from (P2, X), P2 bred from (P1, X); normalized to 4 runs
        let mut scf = 0.0;
        for t in 0..4 {
            for x in 0..4 {
                if x == t {
                    continue;
                }
                let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
                scf += lin(t, ps[0], ps[1], [ps[1], x], [ps[0], x]);
            }
        }
        scf /= 3.0;
        let v = score(d, &make_loop(d, f), &[s[0], s[1], s[2], s[3]]);
        assert!((v - scf).abs() < 1e-9, "score {v} vs per-legacy {scf}");
    }
}

/// Optional G1s (run by one member only) cost no affinity: stripping them from optimized schedules leaves the score
/// unchanged (within the fill tolerance).
#[test]
fn optional_g1s_cost_nothing() {
    let d = data();
    let cases: [[u32; 4]; 4] =
        [[1007, 1019, 1021, 1049], [1009, 1014, 1019, 1029], [1015, 1017, 1023, 1030], [1006, 1019, 1029, 1034]];
    for ids in cases {
        let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
        let so = optimize(d, m, 3, 5, 60.0).s;
        let lp = make_loop(d, m);
        let v = score(d, &lp, &so);
        let mut runners = vec![0u8; d.nr];
        for i in 0..4 {
            let mut seen = vec![false; d.nr];
            for t in 0..NT {
                if let Choice::Opt(o) = so[i][t] {
                    let rr = d.umas[m[i]].opts[t][o as usize].race;
                    if rr >= 0 && !seen[rr as usize] {
                        seen[rr as usize] = true;
                        runners[rr as usize] += 1;
                    }
                }
            }
        }
        let mut stripped = so;
        for i in 0..4 {
            for t in 0..NT {
                if let Choice::Opt(o) = stripped[i][t] {
                    let op = &d.umas[m[i]].opts[t][o as usize];
                    if op.race >= 0 && runners[op.race as usize] == 1 && !op.goal {
                        stripped[i][t] = Choice::Rest;
                    }
                }
            }
        }
        for i in 0..4 {
            assert!(feasible(&d.umas[m[i]], &stripped[i]));
        }
        let vs = score(d, &lp, &stripped);
        assert!((vs - v).abs() < 1e-5, "{ids:?}: stripped {vs} vs {v}");
    }
}

/// A three-member rotation's schedules: the score is the base plus 3·C·(the sum of its three pairs' shared wins),
/// no member gains from re-solving against the other two (a Nash point of the uniform objective), the schedules are
/// feasible, and the unused fourth slot is empty.
#[test]
fn three_member_schedules() {
    let d = data();
    for ids in [[1019u32, 1021, 1034], [1006, 1019, 1029], [1007, 1009, 1049]] {
        let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2])];
        let r = optimize(d, m, 5, 3, 60.0);
        assert!(r.s[3].iter().all(|c| *c == Choice::Rest));
        let s3: [Sched; 3] = [r.s[0], r.s[1], r.s[2]];
        for i in 0..3 {
            assert!(feasible(&d.umas[m[i]], &s3[i]));
        }
        let lp = make_loop(d, m);
        let ps: Vec<Vec<f64>> = (0..3).map(|i| probs(d, &d.umas[m[i]], &s3[i])).collect();
        let direct = lp.base + 3.0 * C_PAIR * (wval(&ps[0], &ps[1]) + wval(&ps[0], &ps[2]) + wval(&ps[1], &ps[2]));
        assert!((direct - score(d, &lp, &s3)).abs() < 1e-9, "{ids:?}");
        for i in 0..3 {
            let w = weights(d, &ps, i);
            let (_, bv) = best_response(d, &d.umas[m[i]], &w, None).unwrap();
            assert!(bv <= wval(&w, &ps[i]) + 1e-6, "{ids:?}: member {i} gains {}", bv - wval(&w, &ps[i]));
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
        let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
        let r = optimize(d, m, 3, 5, 60.0);
        let lp = make_loop(d, m);
        let v = score(d, &lp, &r.s);
        for t in 0..NT {
            let mut races: Vec<i32> = vec![];
            for i in 0..4 {
                if let Choice::Opt(o) = r.s[i][t] {
                    let rr = d.umas[m[i]].opts[t][o as usize].race;
                    if rr >= 0 && !races.contains(&rr) {
                        races.push(rr);
                    }
                }
            }
            for race in races {
                for a in interchangeable(d, &m, &r.s, t, race) {
                    let mut s2 = r.s;
                    for i in 0..4 {
                        if let Choice::Opt(o) = s2[i][t]
                            && d.umas[m[i]].opts[t][o as usize].race == race
                        {
                            let k = d.umas[m[i]].opts[t].iter().position(|op| op.race == a).unwrap();
                            s2[i][t] = Choice::Opt(k as u8);
                        }
                    }
                    for i in 0..4 {
                        assert!(feasible(&d.umas[m[i]], &s2[i]));
                    }
                    assert!((score(d, &lp, &s2) - v).abs() < 1e-9, "swap {race}->{a} at {t} changed the score");
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
        let bp = (110 - cost).max(-20);
        for k in 1..=6 {
            assert_eq!(pk(&d, bp, k), f64::from(vals[k - 1].clamp(0, 100)) / 100.0, "cost {cost} k {k}");
        }
    }
}

/// A goal race takes the previous streak position's penalty (0/0/0/5/20/30 in the shipped data), a chosen race its
/// own; every option's precomputed chances, which the DP reads, follow that rule.
#[test]
fn goal_race_takes_previous_position() {
    let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/data.json"));
    let goal_penalties: Vec<i32> = (1..=6).map(|k| streak_penalty(&d, true, k)).collect();
    let chosen_penalties: Vec<i32> = (1..=6).map(|k| streak_penalty(&d, false, k)).collect();
    assert_eq!(goal_penalties, [0, 0, 0, 5, 20, 30]);
    assert_eq!(chosen_penalties, [0, 0, 5, 20, 30, 50]);
    let mut goal_options = 0;
    for u in &d.umas.0 {
        for (t, options) in u.opts.iter().enumerate() {
            for (o, option) in options.iter().enumerate() {
                goal_options += usize::from(option.goal && option.race >= 0);
                for k in 1..=6 {
                    assert_eq!(u.ent[t][o + 1].p[k], option_chance(&d, option, k), "{} turn {t} option {o}", u.name);
                }
            }
        }
    }
    assert!(goal_options > 0, "no G1 goal options in the data");
}
