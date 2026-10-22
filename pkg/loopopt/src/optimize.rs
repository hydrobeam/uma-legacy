//! Loop affinity scoring, coordinated schedule optimization and pairwise ceilings.
//! Single-uma best responses belong to `schedule`; chart formatting belongs to `chart`.
use crate::data::{Choice, Data, NT, Sched, Uma, feasible, option_chance, probs, probs_into, wval};
use crate::schedule::{MemoScope, best_response};
use crate::uma::{UmaIndex, UmaTable};

/// A loop of N members for the schedule optimiser: N = 4 (the 12-lineage loop) or N = 3 (a three-uma rotation, whose
/// race weights are uniform too: see `skills::lins`).
pub(crate) struct Loop<const N: usize = 4> {
    m: [UmaIndex; N],
    pub(crate) base: f64,
}
/// Race weight C of every pair and the trio coefficient of the base, with grandparent slots at half weight. Per lineage,
/// parent slots carry pair(T,Pi) + pair(P1,P2)/2 + 3*O(P1,P2)/2 each and a grandparent slot tri(T,Pi,G) + 3*O(Pi,G),
/// scaled by w; summed over the 12 self-copy-free lineages and normalized to 4 runs: base = 2*sum pair + 4w*sum trio,
/// C = (2+8w)/3 for every pair. w = 1/2 gives C = 2, trio coefficient 2. C is uniform over the pairs for any w, so w only
/// scales the objective: the schedules are those of plain total affinity.
pub(crate) const C_PAIR: f64 = 2.0;
const K_TRIO: f64 = 2.0;
pub(crate) fn make_loop<const N: usize>(d: &Data, m: [UmaIndex; N]) -> Loop<N> {
    let pr = |a: UmaIndex, b: UmaIndex| f64::from(d.pair[a][b]);
    let tr = |a: UmaIndex, b: UmaIndex, x: UmaIndex| f64::from(d.tri[a][b][x]);
    let mut pair_sum = 0.0;
    let mut trio_sum = 0.0;
    for i in 0..N {
        for j in i + 1..N {
            pair_sum += pr(m[i], m[j]);
            for k in j + 1..N {
                trio_sum += tr(m[i], m[j], m[k]);
            }
        }
    }
    Loop { m, base: 2.0 * pair_sum + K_TRIO * trio_sum }
}

pub(crate) fn score<const N: usize>(d: &Data, lp: &Loop<N>, s: &[Sched; N]) -> f64 {
    let p: Vec<Vec<f64>> = (0..N).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    score_ps(lp, &p)
}
/// Score from precomputed probs (p[i] must equal probs(s[i]); probs is deterministic, so the result is bit-identical to `score`).
fn score_ps<const N: usize>(lp: &Loop<N>, p: &[Vec<f64>]) -> f64 {
    let mut tot = lp.base;
    for i in 0..N {
        for j in i + 1..N {
            tot += 3.0 * C_PAIR * wval(&p[i], &p[j]);
        }
    }
    tot
}

pub(crate) fn weights(d: &Data, ps: &[Vec<f64>], i: usize) -> Vec<f64> {
    let mut w = vec![];
    weights_into(d, ps, i, &mut w);
    w
}
fn weights_into(d: &Data, ps: &[Vec<f64>], i: usize, w: &mut Vec<f64>) {
    w.clear();
    w.resize(d.nr, 0.0);
    for j in 0..ps.len() {
        if j != i {
            for r in 0..d.nr {
                w[r] += 3.0 * C_PAIR * ps[j][r];
            }
        }
    }
}

fn ascent<const N: usize>(d: &Data, lp: &Loop<N>, s: &mut [Sched; N]) -> f64 {
    let mut ps: Vec<Vec<f64>> = (0..N).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    ascent_ps(d, lp, s, &mut ps)
}
/// `ascent` with the members' probs supplied (ps[i] must equal probs(s[i])); ps is kept equal to probs(s[i]).
fn ascent_ps<const N: usize>(d: &Data, lp: &Loop<N>, s: &mut [Sched; N], ps: &mut [Vec<f64>]) -> f64 {
    let mut w: Vec<f64> = Vec::with_capacity(d.nr);
    // stale[i]: some partner's schedule changed since member i was last solved (or she was never solved here).
    // If not stale, her weights are bit-identical to her last solve, so the deterministic best response is the same
    // schedule: either it was rejected then (and would be again) or it was accepted and now equals s[i] exactly
    // (nv == curv, not a strict improvement). Skipping her therefore cannot change the trajectory.
    let mut stale = [true; N];
    loop {
        let mut improved = false;
        for i in 0..N {
            if !stale[i] {
                continue;
            }
            stale[i] = false;
            let u = &d.umas[lp.m[i]];
            weights_into(d, ps, i, &mut w);
            let curv = wval(&w, &ps[i]);
            if let Some((ns, nv)) = best_response(d, u, &w, None)
                && nv > curv + 1e-9
            {
                s[i] = ns;
                probs_into(d, u, &ns, &mut ps[i]);
                improved = true;
                for j in 0..N {
                    if j != i {
                        stale[j] = true;
                    }
                }
            }
        }
        if !improved {
            break;
        }
    }
    score_ps(lp, ps) // ps[i] == probs(s[i]) is maintained above
}

/// Coordinated moves: force a subset of >=2 members onto the same race (or rest) at one turn, then re-ascend.
fn group_moves<const N: usize>(d: &Data, lp: &Loop<N>, s: &mut [Sched; N], margin: f64) -> f64 {
    let mut cur = score(d, lp, s);
    // probs of the current *s, refreshed whenever *s changes (probs is deterministic, so this equals recomputing per mask)
    let mut ps_s: Vec<Vec<f64>> = (0..N).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    let mut ps: Vec<Vec<f64>> = ps_s.clone();
    let mut w: Vec<f64> = Vec::with_capacity(d.nr);
    loop {
        let mut improved = false;
        for t in 0..NT {
            // candidate targets: each race offered at t by any member, plus rest
            let mut targets: Vec<Option<i32>> = vec![None];
            for i in 0..N {
                for op in &d.umas[lp.m[i]].opts[t] {
                    if op.race >= 0 && !targets.contains(&Some(op.race)) {
                        targets.push(Some(op.race));
                    }
                }
            }
            for tg in targets {
                // choice for each member to hit the target at t (None = can't)
                let mut ch: [Option<Choice>; N] = [None; N];
                let mut on = [false; N];
                for i in 0..N {
                    let u = &d.umas[lp.m[i]];
                    let c = match tg {
                        None => u.forced[t].is_none().then_some(Choice::Rest),
                        Some(r) => u.opts[t].iter().position(|op| op.race == r).map(|o| Choice::Opt(o as u8)).filter(
                            |c| match (c, &u.forced[t]) {
                                (Choice::Opt(o), Some(f)) => f.contains(o),
                                _ => true,
                            },
                        ),
                    };
                    ch[i] = c;
                    on[i] = c.is_some_and(|c| s[i][t] == c);
                }
                let movable: Vec<usize> = (0..N).filter(|&i| ch[i].is_some() && !on[i]).collect();
                let non: usize = on.iter().filter(|x| **x).count();
                if movable.is_empty() {
                    continue;
                }
                if tg.is_none() && non == 0 && movable.len() < 2 {
                    continue;
                }
                // subsets of movable with (subset + already-on) >= 2 members and subset >= 1; if nobody is on target, require all movable
                let nmv = movable.len();
                for mask in 1u32..(1 << nmv) {
                    let sz = mask.count_ones() as usize;
                    if sz + non < 2 {
                        continue;
                    }
                    if sz == 1 {
                        continue;
                    } // single-member switches are covered by best responses
                    if non == 0 && tg.is_some() && sz != nmv {
                        continue;
                    }
                    let mut ns = *s;
                    let mut ok = true;
                    ps.clone_from(&ps_s);
                    for b in 0..nmv {
                        if mask & (1 << b) == 0 {
                            continue;
                        }
                        let i = movable[b];
                        let u = &d.umas[lp.m[i]];
                        weights_into(d, &ps, i, &mut w);
                        if let Some((x, _)) = best_response(d, u, &w, Some((t, ch[i].unwrap()))) {
                            ns[i] = x;
                            probs_into(d, u, &x, &mut ps[i]);
                        } else {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        continue;
                    }
                    if score_ps(lp, &ps) < cur - margin {
                        continue;
                    } // ps[i] == probs(ns[i]) here
                    let v = ascent_ps(d, lp, &mut ns, &mut ps); // ps == probs(ns) before and after
                    if v > cur + 1e-7 {
                        cur = v;
                        *s = ns;
                        improved = true;
                        ps_s.clone_from(&ps);
                    }
                }
            }
        }
        if !improved {
            break;
        }
    }
    cur
}

struct Rng(u64);
impl Rng {
    const fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn unif(&mut self) -> f64 {
        (self.next() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn optimistic_start<const N: usize>(d: &Data, lp: &Loop<N>) -> [Sched; N] {
    let ps: Vec<Vec<f64>> = (0..N).map(|i| d.umas[lp.m[i]].pm.clone()).collect();
    let mut s = [[Choice::Rest; NT]; N];
    for i in 0..N {
        let w = weights(d, &ps, i);
        s[i] = best_response(d, &d.umas[lp.m[i]], &w, None).expect("infeasible goals").0;
    }
    s
}

/// Schedules found for a loop, in member order; a three-member structure fills s[0..3] and leaves s[3] all rest.
#[derive(Clone)]
pub(crate) struct Res {
    pub(crate) s: [Sched; 4],
}
/// Missing fourth member in the fixed-size representation of a three-uma rotation.
/// `members` removes this sentinel before callers use the entries as indices into the uma table.
pub(crate) const NONE: UmaIndex = UmaIndex(usize::MAX);
/// The members of a structure stored as [UmaIndex; 4] (three members: the fourth is `NONE`).
pub(crate) fn members(f: &[UmaIndex; 4]) -> &[UmaIndex] {
    if f[3] == NONE { &f[..3] } else { &f[..] }
}

/// Tie-break pass: each member re-solves against her partners with a tiny bonus (EPS per expected G1 win) on every G1,
/// so races that cost no affinity get added (they can still pass down race sparks). Accepted only if the affinity
/// score does not drop by more than 1e-6; races nobody else runs leave every partner's weights unchanged.
const FILL_EPS: f64 = 1e-7;
fn fill_optional<const N: usize>(d: &Data, lp: &Loop<N>, s: &mut [Sched; N], v: f64) -> f64 {
    let mut cur = score(d, lp, s);
    debug_assert!((cur - v).abs() < 1e-6, "fill_optional: the caller's score {v} is not the schedule's {cur}");
    for i in 0..N {
        let ps: Vec<Vec<f64>> = (0..N).map(|k| probs(d, &d.umas[lp.m[k]], &s[k])).collect();
        let w: Vec<f64> = weights(d, &ps, i).iter().map(|x| x + FILL_EPS).collect();
        if let Some((x, _)) = best_response(d, &d.umas[lp.m[i]], &w, None) {
            let mut t = *s;
            t[i] = x;
            let nv = score(d, lp, &t);
            if nv >= cur - 1e-6 {
                *s = t;
                cur = nv;
            }
        }
    }
    // Then enter every remaining G1 that costs nothing, even at 0% (the table's 0% may really be a few percent):
    // on each rest turn, try the best-odds G1 offered; keep it only if the loop's score does not drop at all AND
    // none of that uma's other attempts (goal races included) loses win chance from the longer streak.
    let attempts = |u: &Uma, sc: &Sched| -> Vec<f64> {
        let mut k = 0usize;
        (0..NT)
            .map(|t| match sc[t] {
                Choice::Rest => {
                    k = 0;
                    -1.0
                }
                Choice::Opt(o) => {
                    k = (k + 1).min(6);
                    let op = &u.opts[t][o as usize];
                    if op.race >= 0 { option_chance(d, op, k) } else { -1.0 }
                }
            })
            .collect()
    };
    loop {
        let mut changed = false;
        for i in 0..N {
            let u = &d.umas[lp.m[i]];
            for t in 0..NT {
                if !matches!(s[i][t], Choice::Rest) || u.forced[t].is_some() {
                    continue;
                }
                let mut opts: Vec<usize> = (0..u.opts[t].len()).filter(|&o| u.opts[t][o].race >= 0).collect();
                opts.sort_by_key(|&o| std::cmp::Reverse(u.opts[t][o].bp));
                for o in opts {
                    let mut tr = *s;
                    tr[i][t] = Choice::Opt(o as u8);
                    if !feasible(u, &tr[i]) {
                        continue;
                    }
                    let (a0, a1) = (attempts(u, &s[i]), attempts(u, &tr[i]));
                    if (0..NT).any(|tt| tt != t && a0[tt] >= 0.0 && a1[tt] < a0[tt]) {
                        continue;
                    }
                    let nv = score(d, lp, &tr);
                    if nv >= cur - 1e-12 {
                        *s = tr;
                        cur = cur.max(nv);
                        changed = true;
                        break;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
    cur
}

pub(crate) fn optimize<const N: usize>(d: &Data, m: [UmaIndex; N], iters: usize, seed: u64, margin: f64) -> Res {
    let _memo = MemoScope::new();
    let lp = make_loop(d, m);
    let mut rng = Rng(seed.wrapping_mul(2_654_435_761).wrapping_add(1) | 1);
    let mut schedules = optimistic_start(d, &lp);
    let mut value = ascent(d, &lp, &mut schedules);
    value = value.max(group_moves(d, &lp, &mut schedules, margin));
    // iterated local search: perturb 1-2 members against noisy weights, re-ascend, group moves
    for _ in 0..iters {
        let mut candidate_schedules = schedules;
        let mut candidate_probs: Vec<Vec<f64>> =
            (0..N).map(|i| probs(d, &d.umas[lp.m[i]], &candidate_schedules[i])).collect();
        let moves = 1 + (rng.next() % 2) as usize;
        for _ in 0..moves {
            let i = (rng.next() % N as u64) as usize;
            let mut noisy_weights = weights(d, &candidate_probs, i);
            for weight in &mut noisy_weights {
                *weight *= 0.2 + 1.6 * rng.unif();
            }
            if let Some((response, _)) = best_response(d, &d.umas[lp.m[i]], &noisy_weights, None) {
                candidate_schedules[i] = response;
                candidate_probs[i] = probs(d, &d.umas[lp.m[i]], &response);
            }
        }
        ascent_ps(d, &lp, &mut candidate_schedules, &mut candidate_probs);
        let candidate_value = group_moves(d, &lp, &mut candidate_schedules, margin);
        if candidate_value > value + 1e-7 {
            value = candidate_value;
            schedules = candidate_schedules;
        }
    }
    // add optional G1s that cost no affinity (race sparks)
    fill_optional(d, &lp, &mut schedules, value);
    let mut result_schedules = [[Choice::Rest; NT]; 4];
    result_schedules[..N].copy_from_slice(&schedules);
    Res { s: result_schedules }
}
/// `optimize` for a structure of three or four members (`members` of a [UmaIndex; 4] with `NONE` last for three).
pub(crate) fn optimize_fam(d: &Data, f: &[UmaIndex; 4], iters: usize, seed: u64, margin: f64) -> Res {
    if f[3] == NONE {
        optimize(d, [f[0], f[1], f[2]], iters, seed, margin)
    } else {
        optimize(d, *f, iters, seed, margin)
    }
}

/// Exact pairwise ceilings PB(a,b) = min(BR_a(W = PM_b), BR_b(W = PM_a)) for all pairs in `pool`.
/// Valid because P_b <= PM_b elementwise and P_a >= 0, so <P_a,P_b> <= <P_a,PM_b> <= BR_a(PM_b).
pub(crate) fn pair_ceilings(d: &Data, pool: &[UmaIndex]) -> UmaTable<UmaTable<f64>> {
    let n = d.umas.len();
    let mut pb = UmaTable(vec![UmaTable(vec![0.0f64; n]); n]);
    if let Some(t) = &d.pbt {
        for (x, &a) in pool.iter().enumerate() {
            for &b in &pool[x + 1..] {
                pb[a][b] = t[(a, b)];
                pb[b][a] = t[(b, a)];
            }
        }
        return pb;
    }
    let br = |a: UmaIndex, b: UmaIndex| best_response(d, &d.umas[a], &d.umas[b].pm, None).map_or(0.0, |r| r.1);
    for (x, &a) in pool.iter().enumerate() {
        for &b in &pool[x + 1..] {
            let v = br(a, b).min(br(b, a));
            pb[a][b] = v;
            pb[b][a] = v;
        }
    }
    pb
}
