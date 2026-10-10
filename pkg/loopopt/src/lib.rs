//! # loopopt: Uma Musume (Global) legacy loop and deck optimizer
//!
//! The affinity core below finds race schedules for a loop's members that maximise expected legacy affinity; the
//! skill-target model (`skills`), the deck search and the career planner (`deck`) build on it. The CLI runs "deck"
//! and "career" jobs and `pbtable`; the page runs the same through `web`.
//!
//! Objective, normalized to 4 runs:
//!     E = base + 3 * C * sum_{i<j} <P_i, P_j>
//! where P_i[r] = P(member i wins G1 race r at least once) = 1 - prod(1 - p) over her attempts at r,
//! and p = clamp(110 - cost(distance grade) - cost(surface grade) - PEN[streak position], 0, 100) / 100.
//! Self-copy-free breeding, grandparent slots at half weight: C = 2, base = 2*sum pair + 2*sum trio (`make_loop`).
//!
//! Layers:
//! * `dp_exact`: exact best response for one uma against fixed partners. Junior turns run in sequence;
//!   Classic turn t and Senior turn t+24 are decided jointly, so a race offered in both years
//!   (always 24 turns apart) is scored exactly. Validated against an exact MILP on 352 cases.
//! * `ascent`: best-response dynamics; converges to a Nash equilibrium of the identical-interest game.
//! * `group_moves`: coalition deviations (2+ members forced onto the same race, or rest, at one turn),
//!   which escape the coordination equilibria that single-member moves can't leave.
//! * `optimize`: optimistic start + ascent + group moves + iterated local search (noisy best responses).
//! * `pair_ceilings`: exact pairwise ceilings PB(a, b), the bound the deck search screens structures with
//!   (stored in data.json by `loopopt pbtable`).
//!
//! Relocated-venue G1s are merged into their normal race by the exporter (export/export_rs.py).
//! See docs/SOLVER.md for the data format, job fields, validation and benchmarks.

use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
/// wasm32-unknown-unknown has no clock (Instant::now panics); timings read as zero there.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct Instant;
#[cfg(target_arch = "wasm32")]
impl Instant { fn now() -> Self { Instant } fn elapsed(&self) -> std::time::Duration { std::time::Duration::ZERO } }
/// FxHash (rustc's hasher): one rotate-xor-multiply per word. Only for in-process maps keyed by our own data
/// (no untrusted keys), where SipHash's DoS resistance buys nothing. Results never depend on map iteration order.
#[derive(Default, Clone, Copy)]
pub(crate) struct Fx(u64);
impl Fx {
    #[inline(always)] fn add(&mut self, x: u64) { self.0 = (self.0.rotate_left(5) ^ x).wrapping_mul(0x517c_c1b7_2722_0a95); }
}
impl std::hash::Hasher for Fx {
    #[inline] fn write(&mut self, b: &[u8]) {
        let mut c = b.chunks_exact(8);
        for w in &mut c { self.add(u64::from_le_bytes(w.try_into().unwrap())); }
        let r = c.remainder();
        if !r.is_empty() { let mut t = [0u8; 8]; t[..r.len()].copy_from_slice(r); self.add(u64::from_le_bytes(t)); }
    }
    #[inline] fn write_u8(&mut self, x: u8) { self.add(x as u64); }
    #[inline] fn write_u16(&mut self, x: u16) { self.add(x as u64); }
    #[inline] fn write_u32(&mut self, x: u32) { self.add(x as u64); }
    #[inline] fn write_u64(&mut self, x: u64) { self.add(x); }
    #[inline] fn write_usize(&mut self, x: usize) { self.add(x as u64); }
    #[inline] fn finish(&self) -> u64 { self.0 }
}
pub(crate) type FxMap<K, V> = std::collections::HashMap<K, V, std::hash::BuildHasherDefault<Fx>>;
pub(crate) type FxSet<K> = std::collections::HashSet<K, std::hash::BuildHasherDefault<Fx>>;

const NT: usize = 72;
const NS: usize = 7; // streak state 0 (rested) ..= 6 (6+)
const HALF: usize = 48; // turns 1..=48 = Junior+Classic, 49..=72 = Senior

/// One race option at a turn. `prog` (the race program id) is read only by the tests, whose fixtures name options by it.
#[derive(Deserialize, Clone)]
struct OptRaw { #[cfg_attr(not(test), allow(dead_code))] prog: u32, race: i32, bp: i32, #[serde(default)] name: String, #[serde(default)] goal: bool }
#[derive(Deserialize)]
struct UmaRaw { id: u32, name: String, turns: Vec<Vec<OptRaw>>, goals: Vec<Vec<(usize, usize)>>,
    /// aptitude codes 1..8 = G..S: short, mile, middle, long, turf, dirt, front runner, pace chaser, late surger, end closer
    #[serde(default)] apt: Vec<u8> }
#[derive(Deserialize)]
struct DataRaw { race_names: Vec<String>, umas: Vec<UmaRaw>,
    /// relation groups: (points, roster indices); pair / tri are rebuilt from them (older data.json files carry the tables)
    #[serde(default)] relations: Option<Vec<(i32, Vec<usize>)>>,
    #[serde(default)] pair: Option<Vec<Vec<i32>>>, #[serde(default)] tri: Option<Vec<Vec<Vec<i32>>>>,
    pen: Vec<i32>, #[serde(default)] pb: Option<PbRaw>,
    /// per race: distance class (0 short .. 3 long) and surface (1 turf, 2 dirt)
    #[serde(default)] race_cat: Vec<u8>, #[serde(default)] race_ground: Vec<u8> }
/// pair[a][b] / tri[a][b][c]: summed points of the relation groups holding every one of the (distinct) umas.
fn rel_tables(n: usize, rels: &[(i32, Vec<usize>)]) -> (Vec<Vec<i32>>, Vec<Vec<Vec<i32>>>) {
    let mut p = vec![vec![0i32; n]; n];
    let mut t = vec![vec![vec![0i32; n]; n]; n];
    for (pts, m) in rels {
        for &a in m { for &b in m {
            if a == b { continue; }
            p[a][b] += pts;
            for &c in m { if c != a && c != b { t[a][b][c] += pts; } }
        } }
    }
    (p, t)
}
/// Precomputed pairwise ceilings (`loopopt pbtable`, merged into data.json by `./build.sh pb`): `fp` is
/// `pb_fingerprint` of the data it was computed from; row a holds PB(a, b) for b > a as 16-hex-digit f64 bit patterns.
#[derive(Deserialize, Serialize)]
struct PbRaw { fp: String, rows: Vec<String> }

#[derive(Clone, Copy, PartialEq, Debug)]
enum Choice { Rest, Opt(u8) }

#[derive(Clone)]
struct Uma {
    id: u32,
    name: String,
    opts: Vec<Vec<OptRaw>>,          // per turn (0-based index = turn-1)
    forced: Vec<Option<Vec<u8>>>,    // per turn: must race one of these options
    bits: Vec<Vec<u32>>,             // per turn per option: multi-turn goal bits set
    full: u32,                       // all multi-turn goal bits
    pm: Vec<f64>,                    // best-case P per race
    ent: Vec<[Ent; 6]>,              // per turn: choice code 0 = rest, code o+1 = option o (precomputed for dp_exact)
    chl: Vec<([u8; 6], usize)>,      // per turn: allowed choice codes (no force), rest first then options in index order
    apt: Vec<u8>,                    // aptitude codes (UmaRaw); empty in older data
}

/// Precomputed per-choice data for dp_exact: race (-1 = none), win chance per streak position k (same values as `pk`), goal bits.
#[derive(Clone, Copy)]
struct Ent { race: i32, rest: bool, bit: u32, p: [f64; 7] }
const ENT_REST: Ent = Ent { race: -1, rest: true, bit: 0, p: [0.0; 7] };

struct Data { nr: usize, umas: Vec<Uma>, pair: Vec<Vec<i32>>, tri: Vec<Vec<Vec<i32>>>, pk: Vec<[f64; 7]>, race_names: Vec<String>,
    pbt: Option<Vec<f64>>,   // n x n pairwise ceilings from data.json's table, when its fingerprint matches
    pen: Vec<i32>,    // win-chance penalty for the k-th race in a row (pen[k - 1]), for the schedule's explanations
    race_cat: Vec<u8>, race_ground: Vec<u8> }

/// Bump when best_response / pair_ceilings change in a way that could move a ceiling's bits: stored tables then
/// stop matching and are recomputed.
const PB_VERSION: u64 = 1;
/// Fingerprint of everything a pairwise ceiling depends on: every uma's id, options (race, bp) per turn and goals,
/// the streak penalties and the race count. Fx over fixed-width words, so native and wasm agree.
fn pb_fingerprint(raw: &DataRaw) -> String {
    use std::hash::Hasher;
    let mut h = Fx::default();
    h.write_u64(PB_VERSION); h.write_u64(raw.race_names.len() as u64);
    for &p in &raw.pen { h.write_u64(p as i64 as u64); }
    for u in &raw.umas {
        h.write_u64(u.id as u64); h.write_u64(u.turns.len() as u64);
        for l in &u.turns { h.write_u64(l.len() as u64); for op in l { h.write_u64(op.race as i64 as u64); h.write_u64(op.bp as i64 as u64); } }
        h.write_u64(u.goals.len() as u64);
        for g in &u.goals { h.write_u64(g.len() as u64); for &(t, o) in g { h.write_u64(t as u64); h.write_u64(o as u64); } }
    }
    format!("{:016x}", h.finish())
}
fn pb_table_from(raw: &DataRaw) -> Option<Vec<f64>> {
    let pr = raw.pb.as_ref()?;
    let n = raw.umas.len();
    if pr.fp != pb_fingerprint(raw) || pr.rows.len() != n { return None; }
    let mut t = vec![0.0f64; n * n];
    for (a, row) in pr.rows.iter().enumerate() {
        if row.len() != 16 * (n - 1 - a) { return None; }
        for (k, b) in (a + 1..n).enumerate() {
            let v = f64::from_bits(u64::from_str_radix(&row[16 * k..16 * k + 16], 16).ok()?);
            t[a * n + b] = v; t[b * n + a] = v;
        }
    }
    Some(t)
}

// pk table indexed by bp+20 (bp can be as low as -10), position k=1..6
fn pk(d: &Data, bp: i32, k: usize) -> f64 { d.pk[(bp + 20) as usize][k] }

fn load(path: &str) -> Data { load_str(&std::fs::read_to_string(path).unwrap()) }
/// Win chance of an attempt = clamp(bp - pen[k], 0, 100)%, where the exporter sets bp = 110 - grade costs
/// (distance + surface; A 0, B 10, C 20, D 30, E 40, F 60, G 90) and pen = streak penalties from data.json.
fn load_str(json: &str) -> Data { load_raw(serde_json::from_str(json).unwrap()) }
fn load_raw(raw: DataRaw) -> Data {
    let pbt = pb_table_from(&raw);
    let n_umas = raw.umas.len();
    // dp_exact relies on: every race offered twice to an uma is offered exactly 24 turns apart (Classic/Senior),
    // never twice in the Junior year, and never more than twice.
    for u in &raw.umas {
        let mut at: std::collections::HashMap<i32, Vec<usize>> = Default::default();
        for (t, l) in u.turns.iter().enumerate() { for op in l { if op.race >= 0 { at.entry(op.race).or_default().push(t); } } }
        for (r, ts) in at {
            assert!(ts.len() <= 2, "{}: race {r} offered {} times", u.name, ts.len());
            if ts.len() == 2 { assert!(ts[1] == ts[0] + 24 && ts[0] >= 24, "{}: race {r} offered at turns {ts:?}, not a Classic/Senior pair", u.name); }
        }
    }
    let nr = raw.race_names.len();
    let mut pkt = vec![[0.0f64; 7]; 140];
    for (i, row) in pkt.iter_mut().enumerate() {
        let bp = i as i32 - 20;
        for k in 1..=6 { row[k] = ((bp - raw.pen[k - 1]).clamp(0, 100)) as f64 / 100.0; }
    }
    let mut umas = vec![];
    for u in raw.umas {
        let mut forced: Vec<Option<Vec<u8>>> = vec![None; NT];
        let mut bits = u.turns.iter().map(|l| vec![0u32; l.len()]).collect::<Vec<_>>();
        let mut nb = 0;
        for g in &u.goals {
            let ts: std::collections::BTreeSet<usize> = g.iter().map(|x| x.0).collect();
            if ts.len() == 1 {
                let t = *ts.iter().next().unwrap() - 1;
                let v: Vec<u8> = g.iter().map(|x| x.1 as u8).collect();
                forced[t] = Some(match &forced[t] { None => v, Some(old) => old.iter().cloned().filter(|o| v.contains(o)).collect() });
            } else {
                for &(t, o) in g { bits[t - 1][o] |= 1 << nb; }
                nb += 1;
            }
        }
        let mut miss = vec![1.0f64; nr];
        for l in u.turns.iter() {
            for op in l.iter() {
                if op.race >= 0 { miss[op.race as usize] *= 1.0 - pkt[(op.bp + 20) as usize][1]; }
            }
        }
        let mut ent = vec![[ENT_REST; 6]; NT];
        let mut chl = vec![([0u8; 6], 0usize); NT];
        for t in 0..NT {
            assert!(u.turns[t].len() <= 5, "more than 5 options at one turn");
            for (o, op) in u.turns[t].iter().enumerate() {
                let mut p = [0.0f64; 7];
                for k in 1..=6 { p[k] = pkt[(op.bp + 20) as usize][k]; }
                ent[t][o + 1] = Ent { race: op.race, rest: false, bit: bits[t][o], p };
            }
            let (a, n) = &mut chl[t];
            if forced[t].is_none() { a[*n] = 0; *n += 1; }
            for o in 0..u.turns[t].len() {
                if forced[t].as_ref().map_or(true, |f| f.contains(&(o as u8))) { a[*n] = o as u8 + 1; *n += 1; }
            }
        }
        umas.push(Uma { id: u.id, name: u.name, opts: u.turns, forced, bits, full: (1u32 << nb) - 1,
            pm: miss.iter().map(|m| 1.0 - m).collect(), ent, chl, apt: u.apt });
    }
    let (pair, tri) = match (raw.pair, raw.tri, &raw.relations) {
        (Some(p), Some(t), _) => (p, t),
        (_, _, Some(r)) => rel_tables(n_umas, r),
        _ => panic!("data.json has neither relations nor pair/tri tables"),
    };
    Data { nr, umas, pair, tri, pk: pkt, race_names: raw.race_names, pbt, pen: raw.pen, race_cat: raw.race_cat, race_ground: raw.race_ground }
}

type Sched = [Choice; NT];

fn probs(d: &Data, u: &Uma, s: &Sched) -> Vec<f64> { let mut v = vec![]; probs_into(d, u, s, &mut v); v }
/// probs into a reused buffer (same arithmetic as `probs`, so bit-identical)
fn probs_into(d: &Data, u: &Uma, s: &Sched, miss: &mut Vec<f64>) {
    miss.clear(); miss.resize(d.nr, 1.0);
    let mut k = 0usize;
    for t in 0..NT {
        match s[t] {
            Choice::Rest => k = 0,
            Choice::Opt(o) => {
                k = (k + 1).min(6);
                // ent[t][o+1].p[k] == pk(d, op.bp, k) bit for bit (copied from the same table at load)
                let e = &u.ent[t][o as usize + 1];
                if e.race >= 0 { miss[e.race as usize] *= 1.0 - e.p[k]; }
            }
        }
    }
    for m in miss.iter_mut() { *m = 1.0 - *m; }
}

fn feasible(u: &Uma, s: &Sched) -> bool {
    let mut m = 0u32;
    for t in 0..NT {
        match s[t] {
            Choice::Rest => { if u.forced[t].is_some() { return false; } }
            Choice::Opt(o) => {
                if (o as usize) >= u.opts[t].len() { return false; }
                if let Some(f) = &u.forced[t] { if !f.contains(&o) { return false; } }
                m |= u.bits[t][o as usize];
            }
        }
    }
    m == u.full
}

fn wval(w: &[f64], p: &[f64]) -> f64 { w.iter().zip(p).map(|(a, b)| a * b).sum() }

/// EXACT best response: maximize sum_r W[r] * P[r], with P[r] = 1 - prod(1 - p) over attempts.
/// Junior turns 1..24 run sequentially; then Classic turn t and Senior turn t+24 are decided together,
/// so a race offered in both years (always exactly 24 turns apart) is scored exactly.
/// State: (classic streak, senior streak, guessed classic streak at end of turn 48 [only if turn 49 has options], goal mask).
/// `force`: optional (turn, choice) the schedule must take at that turn.
/// Implementation notes (performance): choices are codes (0 = rest, o+1 = option o) into per-uma precomputed tables;
/// loops run choice-pair outer / state inner, but each state sees candidates in the same order (classic choice outer,
/// senior choice inner, rest first then options by index) with strict '>' updates, so values and argmax match the
/// straightforward state-outer formulation exactly.
fn dp_exact(_d: &Data, u: &Uma, wr: &[f64], force: Option<(usize, Choice)>) -> Option<(Sched, f64)> {
    SCRATCH.with(|c| dp_exact_in(u, wr, force, &mut c.borrow_mut()))
}

#[derive(Default)]
struct Scratch { v2: Vec<f64>, a2: Vec<[u8; 2]>, v1: Vec<f64>, a1: Vec<u8> }
thread_local! { static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default()); }

/// Allowed choice codes at turn t. A force replaces the list at its turn, but only with a choice the goals allow there.
#[inline]
fn choice_list(u: &Uma, t: usize, force: Option<(usize, Choice)>) -> ([u8; 6], usize) {
    if let Some((ft, fc)) = force {
        if ft == t {
            // a force can't override a single-turn goal: forcing rest, or an option outside the goal set, is infeasible
            let code = match fc {
                Choice::Rest => if u.forced[t].is_none() { Some(0u8) } else { None },
                Choice::Opt(o) => if (o as usize) < u.opts[t].len() && u.forced[t].as_ref().map_or(true, |f| f.contains(&o)) { Some(o + 1) } else { None },
            };
            return match code { Some(c) => ([c, 0, 0, 0, 0, 0], 1), None => ([0; 6], 0) };
        }
    }
    u.chl[t]
}

#[inline]
fn code_choice(c: u8) -> Choice { if c == 0 { Choice::Rest } else { Choice::Opt(c - 1) } }

fn dp_exact_in(u: &Uma, wr: &[f64], force: Option<(usize, Choice)>, scr: &mut Scratch) -> Option<(Sched, f64)> {
    let nm = (u.full + 1) as usize;
    let ns0 = if u.opts[HALF].is_empty() { 1 } else { NS };
    let neg = f64::NEG_INFINITY;
    let kb = ns0 * nm;          // block of (s0, m) states
    let js = NS * NS * kb;      // stride of one pair index j
    let full = u.full as usize;
    // streak position after choosing entry e with incoming streak s (also the next streak)
    #[inline(always)] fn kof(e: &Ent, s: usize) -> usize { if e.rest { 0 } else if s >= 5 { 6 } else { s + 1 } }
    #[inline(always)] fn pof(e: &Ent, k: usize) -> f64 { if e.race >= 0 { e.p[k] } else { 0.0 } }
    // ---- phase 2: pairs j = 0..24 (classic turn 24+j with senior turn 48+j)
    let v2 = &mut scr.v2; let a2 = &mut scr.a2;
    v2.clear(); v2.resize(25 * js, neg);
    a2.clear(); a2.resize(24 * js, [0, 0]);
    for sc in 0..NS { for ss in 0..NS { for s0 in 0..ns0 {
        if ns0 > 1 && sc != s0 { continue; }
        v2[24 * js + (sc * NS + ss) * kb + s0 * nm + full] = 0.0;
    } } }
    for j in (0..24).rev() {
        let (tc, ts) = (24 + j, 48 + j);
        let (cl, ncl) = choice_list(u, tc, force);
        let (sl, nsl) = choice_list(u, ts, force);
        let (lo, hi) = v2.split_at_mut((j + 1) * js);
        let cur = &mut lo[j * js..];
        let nxt = &hi[..js];
        let arg = &mut a2[j * js..(j + 1) * js];
        for &cc in &cl[..ncl] {
            let ec = &u.ent[tc][cc as usize];
            // per classic streak sc: next streak kc and (separate-race) reward term 0.0 + w*p
            let mut kca = [0usize; NS]; let mut ra = [0.0f64; NS]; let mut pca = [0.0f64; NS];
            for sc in 0..NS { let k = kof(ec, sc); kca[sc] = k; pca[sc] = pof(ec, k); ra[sc] = if ec.race >= 0 { 0.0 + wr[ec.race as usize] * pca[sc] } else { 0.0 }; }
            for &cs in &sl[..nsl] {
                let es = &u.ent[ts][cs as usize];
                let mb = (ec.bit | es.bit) as usize;
                let mut ksa = [0usize; NS]; let mut rb = [0.0f64; NS]; let mut psa = [0.0f64; NS];
                for ss in 0..NS { let k = kof(es, ss); ksa[ss] = k; psa[ss] = pof(es, k); rb[ss] = if es.race >= 0 { wr[es.race as usize] * psa[ss] } else { 0.0 }; }
                let same = ec.race >= 0 && ec.race == es.race;
                let wsame = if same { wr[ec.race as usize] } else { 0.0 };
                // reward for (sc, ss); bit-identical to the original formula:
                //   same race: w * (1 - (1-pc)(1-ps)); otherwise (0.0 + w_c*pc) + w_s*ps, with absent terms dropped
                let rew = |sc: usize, ss: usize| -> f64 {
                    if same { wsame * (1.0 - (1.0 - pca[sc]) * (1.0 - psa[ss])) } else { ra[sc] + rb[ss] }
                };
                if kb == 1 {
                    for sc in 0..NS {
                        let nrow = &nxt[kca[sc] * NS..kca[sc] * NS + NS];
                        let crow = &mut cur[sc * NS..sc * NS + NS];
                        let arow = &mut arg[sc * NS..sc * NS + NS];
                        for ss in 0..NS {
                            // nxt == -inf gives x == -inf, which never passes the strict '>' (same as skipping it)
                            let x = rew(sc, ss) + nrow[ksa[ss]];
                            if x > crow[ss] { crow[ss] = x; arow[ss] = [cc, cs]; }
                        }
                    }
                } else {
                    for sc in 0..NS { for ss in 0..NS {
                        let r = rew(sc, ss);
                        let nb = (kca[sc] * NS + ksa[ss]) * kb;
                        let cb = (sc * NS + ss) * kb;
                        for s0 in 0..ns0 { for m in 0..nm {
                            let x = r + nxt[nb + s0 * nm + (m | mb)];
                            let ci = cb + s0 * nm + m;
                            if x > cur[ci] { cur[ci] = x; arg[ci] = [cc, cs]; }
                        } }
                    } }
                }
            }
        }
    }
    // entry value into phase 2 from junior end state (s, m): choose s0 freely; senior streak starts at s0 (or 0)
    let entry = |s: usize, m: usize| -> (f64, usize) {
        let mut b = (neg, 0);
        for s0 in 0..ns0 { let ss = if ns0 > 1 { s0 } else { 0 }; let x = v2[(s * NS + ss) * kb + s0 * nm + m]; if x > b.0 { b = (x, s0); } }
        b
    };
    // ---- phase 1: junior turns 0..24
    let v1 = &mut scr.v1; let a1 = &mut scr.a1;
    let ts1 = NS * nm;
    v1.clear(); v1.resize(25 * ts1, neg);
    a1.clear(); a1.resize(24 * ts1, 0);
    for s in 0..NS { for m in 0..nm { v1[24 * ts1 + s * nm + m] = entry(s, m).0; } }
    for t in (0..24).rev() {
        let (cl, ncl) = choice_list(u, t, force);
        let (lo, hi) = v1.split_at_mut((t + 1) * ts1);
        let cur = &mut lo[t * ts1..];
        let nxt = &hi[..ts1];
        let arg = &mut a1[t * ts1..(t + 1) * ts1];
        for &c in &cl[..ncl] {
            let e = &u.ent[t][c as usize];
            let mb = e.bit as usize;
            for s in 0..NS {
                let k = kof(e, s);
                let rew = if e.race >= 0 { wr[e.race as usize] * e.p[k] } else { 0.0 };
                for m in 0..nm {
                    let x = nxt[k * nm + (m | mb)] + rew;
                    let ci = s * nm + m;
                    if x > cur[ci] { cur[ci] = x; arg[ci] = c; }
                }
            }
        }
    }
    let total = v1[0];
    if total == neg { return None; }
    let mut out = [Choice::Rest; NT];
    let (mut s, mut m) = (0usize, 0usize);
    for t in 0..24 {
        let c = a1[t * ts1 + s * nm + m]; let e = &u.ent[t][c as usize];
        out[t] = code_choice(c); s = kof(e, s); m |= e.bit as usize;
    }
    let (_, s0) = entry(s, m);
    let (mut sc, mut ss) = (s, if ns0 > 1 { s0 } else { 0 });
    for j in 0..24 {
        let [cc, cs] = a2[j * js + (sc * NS + ss) * kb + s0 * nm + m];
        let (ec, es) = (&u.ent[24 + j][cc as usize], &u.ent[48 + j][cs as usize]);
        out[24 + j] = code_choice(cc); out[48 + j] = code_choice(cs);
        sc = kof(ec, sc); ss = kof(es, ss); m |= (ec.bit | es.bit) as usize;
    }
    Some((out, total))
}

/// Best-response memo, active only inside `optimize` (one per thread, cleared per call). dp_exact + the evaluation below
/// are a pure, deterministic function of (uma, force, exact weight bits), so a hit returns exactly what a fresh solve
/// would. Within one optimize() call ~99% of best-response calls repeat an earlier one.
/// Key: [uma id << 16 | force code, weight bits...], built in `key` (reused) so a hit allocates nothing.
#[derive(Default)]
struct BrMemo { map: FxMap<Box<[u64]>, Option<(Sched, f64)>>, key: Vec<u64> }
thread_local! { static BR_MEMO: std::cell::RefCell<Option<BrMemo>> = const { std::cell::RefCell::new(None) }; }

/// Best response (exact): wraps dp_exact and cross-checks its value against direct evaluation.
fn best_response(d: &Data, u: &Uma, wr: &[f64], force: Option<(usize, Choice)>) -> Option<(Sched, f64)> {
    BR_MEMO.with(|mm| {
        let mut g = mm.borrow_mut();
        let Some(m) = g.as_mut() else { drop(g); return best_response_solve(d, u, wr, force); };
        let fk: u16 = match force { None => u16::MAX, Some((t, Choice::Rest)) => (t * 256 + 255) as u16, Some((t, Choice::Opt(o))) => (t * 256 + o as usize) as u16 };
        m.key.clear();
        m.key.push((u.id as u64) << 16 | fk as u64);
        m.key.extend(wr.iter().map(|x| x.to_bits()));
        if let Some(r) = m.map.get(&m.key[..]) { return *r; }
        let r = best_response_solve(d, u, wr, force);
        let k: Box<[u64]> = m.key.as_slice().into();
        m.map.insert(k, r);
        r
    })
}

/// Enables the best-response memo for the current thread until dropped (restores the previous state).
struct MemoScope(Option<BrMemo>);
impl MemoScope {
    fn new() -> Self { MemoScope(BR_MEMO.with(|m| m.borrow_mut().replace(Default::default()))) }
}
impl Drop for MemoScope { fn drop(&mut self) { let prev = self.0.take(); BR_MEMO.with(|m| *m.borrow_mut() = prev); } }

fn best_response_solve(d: &Data, u: &Uma, wr: &[f64], force: Option<(usize, Choice)>) -> Option<(Sched, f64)> {
    let (s, v) = dp_exact(d, u, wr, force)?;
    let ev = wval(wr, &probs(d, u, &s));
    assert!((ev - v).abs() < 1e-6, "dp_exact value {v} != evaluated {ev} for {}", u.name);
    assert!(feasible(u, &s), "best response produced an infeasible schedule for {}", u.name);
    Some((s, ev))
}

/// A loop of N members for the schedule optimiser: N = 4 (the 12-lineage loop) or N = 3 (a three-uma rotation, whose
/// race weights are uniform too: see `skills::lins`).
struct Loop<const N: usize = 4> { m: [usize; N], base: f64 }
/// Race weight C of every pair and the trio coefficient of the base, with grandparent slots at half weight. Per lineage,
/// parent slots carry pair(T,Pi) + pair(P1,P2)/2 + 3*O(P1,P2)/2 each and a grandparent slot tri(T,Pi,G) + 3*O(Pi,G),
/// scaled by w; summed over the 12 self-copy-free lineages and normalized to 4 runs: base = 2*sum pair + 4w*sum trio,
/// C = (2+8w)/3 for every pair. w = 1/2 gives C = 2, trio coefficient 2. C is uniform over the pairs for any w, so w only
/// scales the objective: the schedules are those of plain total affinity.
const C_PAIR: f64 = 2.0;
const K_TRIO: f64 = 2.0;
fn make_loop<const N: usize>(d: &Data, m: [usize; N]) -> Loop<N> {
    let pr = |a: usize, b: usize| d.pair[a][b] as f64;
    let tr = |a: usize, b: usize, x: usize| d.tri[a][b][x] as f64;
    let mut psum = 0.0; let mut tsum = 0.0;
    for i in 0..N { for j in i + 1..N { psum += pr(m[i], m[j]); for k in j + 1..N { tsum += tr(m[i], m[j], m[k]); } } }
    Loop { m, base: 2.0 * psum + K_TRIO * tsum }
}

fn score<const N: usize>(d: &Data, lp: &Loop<N>, s: &[Sched; N]) -> f64 {
    let p: Vec<Vec<f64>> = (0..N).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    score_ps(lp, &p)
}
/// Score from precomputed probs (p[i] must equal probs(s[i]); probs is deterministic, so the result is bit-identical to `score`).
fn score_ps<const N: usize>(lp: &Loop<N>, p: &[Vec<f64>]) -> f64 {
    let mut tot = lp.base;
    for i in 0..N { for j in i + 1..N { tot += 3.0 * C_PAIR * wval(&p[i], &p[j]); } }
    tot
}

fn weights(d: &Data, ps: &[Vec<f64>], i: usize) -> Vec<f64> {
    let mut w = vec![]; weights_into(d, ps, i, &mut w); w
}
fn weights_into(d: &Data, ps: &[Vec<f64>], i: usize, w: &mut Vec<f64>) {
    w.clear(); w.resize(d.nr, 0.0);
    for j in 0..ps.len() { if j != i { for r in 0..d.nr { w[r] += 3.0 * C_PAIR * ps[j][r]; } } }
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
            if !stale[i] { continue; }
            stale[i] = false;
            let u = &d.umas[lp.m[i]];
            weights_into(d, ps, i, &mut w);
            let curv = wval(&w, &ps[i]);
            if let Some((ns, nv)) = best_response(d, u, &w, None) {
                if nv > curv + 1e-9 {
                    s[i] = ns; probs_into(d, u, &ns, &mut ps[i]); improved = true;
                    for j in 0..N { if j != i { stale[j] = true; } }
                }
            }
        }
        if !improved { break; }
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
            for i in 0..N { for op in &d.umas[lp.m[i]].opts[t] { if op.race >= 0 && !targets.contains(&Some(op.race)) { targets.push(Some(op.race)); } } }
            for tg in targets {
                // choice for each member to hit the target at t (None = can't)
                let mut ch: [Option<Choice>; N] = [None; N];
                let mut on = [false; N];
                for i in 0..N {
                    let u = &d.umas[lp.m[i]];
                    let c = match tg {
                        None => if u.forced[t].is_none() { Some(Choice::Rest) } else { None },
                        Some(r) => u.opts[t].iter().position(|op| op.race == r).map(|o| Choice::Opt(o as u8))
                            .filter(|c| match (c, &u.forced[t]) { (Choice::Opt(o), Some(f)) => f.contains(o), _ => true }),
                    };
                    ch[i] = c;
                    on[i] = c.map_or(false, |c| s[i][t] == c);
                }
                let movable: Vec<usize> = (0..N).filter(|&i| ch[i].is_some() && !on[i]).collect();
                let non: usize = on.iter().filter(|x| **x).count();
                if movable.is_empty() { continue; }
                if tg.is_none() && non == 0 && movable.len() < 2 { continue; }
                // subsets of movable with (subset + already-on) >= 2 members and subset >= 1; if nobody is on target, require all movable
                let nmv = movable.len();
                for mask in 1u32..(1 << nmv) {
                    let sz = mask.count_ones() as usize;
                    if sz + non < 2 { continue; }
                    if sz == 1 { continue; } // single-member switches are covered by best responses
                    if non == 0 && tg.is_some() && sz != nmv { continue; }
                    let mut ns = *s;
                    let mut ok = true;
                    ps.clone_from(&ps_s);
                    for b in 0..nmv {
                        if mask & (1 << b) == 0 { continue; }
                        let i = movable[b];
                        let u = &d.umas[lp.m[i]];
                        weights_into(d, &ps, i, &mut w);
                        match best_response(d, u, &w, Some((t, ch[i].unwrap()))) {
                            Some((x, _)) => { ns[i] = x; probs_into(d, u, &x, &mut ps[i]); }
                            None => { ok = false; break; }
                        }
                    }
                    if !ok { continue; }
                    if score_ps(lp, &ps) < cur - margin { continue; } // ps[i] == probs(ns[i]) here
                    let v = ascent_ps(d, lp, &mut ns, &mut ps); // ps == probs(ns) before and after
                    if v > cur + 1e-7 { cur = v; *s = ns; improved = true; ps_s.clone_from(&ps); }
                }
            }
        }
        if !improved { break; }
    }
    cur
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 { self.0 ^= self.0 << 13; self.0 ^= self.0 >> 7; self.0 ^= self.0 << 17; self.0 }
    fn unif(&mut self) -> f64 { (self.next() >> 11) as f64 / (1u64 << 53) as f64 }
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
struct Res { s: [Sched; 4] }
/// The empty fourth slot of a three-member structure (members arrays are [usize; 4] with this last).
pub(crate) const NONE: usize = usize::MAX;
/// The members of a structure stored as [usize; 4] (three members: the fourth is `NONE`).
pub(crate) fn members(f: &[usize; 4]) -> &[usize] { if f[3] == NONE { &f[..3] } else { &f[..] } }

/// Tie-break pass: each member re-solves against her partners with a tiny bonus (EPS per expected G1 win) on every G1,
/// so races that cost no affinity get added (they can still pass down race sparks). Accepted only if the affinity
/// score does not drop by more than 1e-6; races nobody else runs leave every partner's weights unchanged.
const FILL_EPS: f64 = 1e-7;
fn fill_optional<const N: usize>(d: &Data, lp: &Loop<N>, s: &mut [Sched; N], v: f64) -> f64 {
    let mut cur = score(d, lp, s);
    debug_assert!((cur - v).abs() < 1e-6);
    for i in 0..N {
        let ps: Vec<Vec<f64>> = (0..N).map(|k| probs(d, &d.umas[lp.m[k]], &s[k])).collect();
        let w: Vec<f64> = weights(d, &ps, i).iter().map(|x| x + FILL_EPS).collect();
        if let Some((x, _)) = best_response(d, &d.umas[lp.m[i]], &w, None) {
            let mut t = *s; t[i] = x;
            let nv = score(d, lp, &t);
            if nv >= cur - 1e-6 { *s = t; cur = nv; }
        }
    }
    // Then enter every remaining G1 that costs nothing, even at 0% (the table's 0% may really be a few percent):
    // on each rest turn, try the best-odds G1 offered; keep it only if the loop's score does not drop at all AND
    // none of that uma's other attempts (goal races included) loses win chance from the longer streak.
    let attempts = |u: &Uma, sc: &Sched| -> Vec<f64> {
        let mut k = 0usize;
        (0..NT).map(|t| match sc[t] { Choice::Rest => { k = 0; -1.0 } Choice::Opt(o) => { k = (k + 1).min(6);
            let op = &u.opts[t][o as usize]; if op.race >= 0 { pk(d, op.bp, k) } else { -1.0 } } }).collect()
    };
    loop {
        let mut changed = false;
        for i in 0..N {
            let u = &d.umas[lp.m[i]];
            for t in 0..NT {
                if !matches!(s[i][t], Choice::Rest) || u.forced[t].is_some() { continue; }
                let mut opts: Vec<usize> = (0..u.opts[t].len()).filter(|&o| u.opts[t][o].race >= 0).collect();
                opts.sort_by_key(|&o| std::cmp::Reverse(u.opts[t][o].bp));
                for o in opts {
                    let mut tr = *s; tr[i][t] = Choice::Opt(o as u8);
                    if !feasible(u, &tr[i]) { continue; }
                    let (a0, a1) = (attempts(u, &s[i]), attempts(u, &tr[i]));
                    if (0..NT).any(|tt| tt != t && a0[tt] >= 0.0 && a1[tt] < a0[tt]) { continue; }
                    let nv = score(d, lp, &tr);
                    if nv >= cur - 1e-12 { *s = tr; cur = cur.max(nv); changed = true; break; }
                }
            }
        }
        if !changed { break; }
    }
    cur
}

fn optimize<const N: usize>(d: &Data, m: [usize; N], iters: usize, seed: u64, margin: f64) -> Res {
    let _memo = MemoScope::new();
    let lp = make_loop(d, m);
    let mut rng = Rng(seed.wrapping_mul(2654435761).wrapping_add(1) | 1);
    let mut s = optimistic_start(d, &lp);
    let mut v = ascent(d, &lp, &mut s);
    v = v.max(group_moves(d, &lp, &mut s, margin));
    // iterated local search: perturb 1-2 members against noisy weights, re-ascend, group moves
    for _ in 0..iters {
        let mut ns = s;
        let mut ps: Vec<Vec<f64>> = (0..N).map(|i| probs(d, &d.umas[lp.m[i]], &ns[i])).collect();
        let nmove = 1 + (rng.next() % 2) as usize;
        for _ in 0..nmove {
            let i = (rng.next() % N as u64) as usize;
            let mut w = weights(d, &ps, i);
            for x in w.iter_mut() { *x *= 0.2 + 1.6 * rng.unif(); }
            if let Some((x, _)) = best_response(d, &d.umas[lp.m[i]], &w, None) { ns[i] = x; ps[i] = probs(d, &d.umas[lp.m[i]], &x); }
        }
        ascent_ps(d, &lp, &mut ns, &mut ps);
        let nv = group_moves(d, &lp, &mut ns, margin);
        if nv > v + 1e-7 { v = nv; s = ns; }
    }
    // add optional G1s that cost no affinity (race sparks)
    fill_optional(d, &lp, &mut s, v);
    let mut sc = [[Choice::Rest; NT]; 4];
    sc[..N].copy_from_slice(&s);
    Res { s: sc }
}
/// `optimize` for a structure of three or four members (`members` of a [usize; 4] with `NONE` last for three).
pub(crate) fn optimize_fam(d: &Data, f: &[usize; 4], iters: usize, seed: u64, margin: f64) -> Res {
    if f[3] == NONE { optimize(d, [f[0], f[1], f[2]], iters, seed, margin) } else { optimize(d, *f, iters, seed, margin) }
}

fn when(t: usize) -> String {
    let tt = t; let y = if tt < 24 { "Jr" } else if tt < 48 { "Cl" } else { "Sr" };
    let w = tt % 24; let mon = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][w / 2];
    format!("{} {}-{}", y, if w % 2 == 0 { "E" } else { "L" }, mon)
}
/// Short uma label for chart columns.
fn short_name(name: &str) -> String {
    let m: &[(&str, &str)] = &[("T.M. Opera O", "Opera O"), ("El Condor Pasa", "Pasa"), ("Agnes Digital", "Digital"), ("Agnes Tachyon", "Tachyon"),
        ("Biwa Hayahide", "Biwa"), ("Super Creek", "Creek"), ("Gold Ship", "GoldShip"), ("Gold City", "GoldCity"), ("Symboli Rudolf", "Rudolf"),
        ("Narita Brian", "Brian"), ("Narita Taishin", "Taishin"), ("Narita Top Road", "TopRoad"), ("Sakura Bakushin O", "Bakushin"), ("Sakura Chiyono O", "Chiyono"),
        ("Special Week", "SpWeek"), ("Silence Suzuka", "Suzuka"), ("Tokai Teio", "Teio"), ("Grass Wonder", "Grass"), ("Hishi Amazon", "Amazon"),
        ("Hishi Akebono", "Akebono"), ("Mihono Bourbon", "Bourbon"), ("Yamanin Zephyr", "Zephyr"), ("Satono Diamond", "Diamond"), ("Kitasan Black", "Kitasan"),
        ("Haru Urara", "Urara"), ("Smart Falcon", "Falcon"), ("Curren Chan", "Curren"), ("King Halo", "Halo"), ("Nice Nature", "Nature"),
        ("Seiun Sky", "Seiun"), ("Taiki Shuttle", "Taiki"), ("Meisho Doto", "Doto"), ("Mayano Top Gun", "Mayano"), ("Air Groove", "Groove"),
        ("Air Shakur", "Shakur"), ("Matikanefukukitaru", "Fukukitr"), ("Matikanetannhauser", "Tannhaus"), ("Copano Rickey", "Copano"), ("Wonder Acute", "Acute"), ("Nakayama Festa", "Festa"), ("Yamanin Zephyr", "Zephyr"), ("Kawakami Princess", "Kawakami"), ("Fine Motion", "FineMotn")];
    if let Some((_, a)) = m.iter().find(|(n, _)| *n == name) { return a.to_string(); }
    if let Some(rest) = name.strip_prefix("Mejiro ") { return rest.chars().take(8).collect(); }
    name.split_whitespace().next().unwrap_or(name).chars().take(8).collect()
}
/// Row label: the race, or "A/B" (alphabetical) when interchangeable alternatives exist at that turn.
fn row_label(d: &Data, m: &[usize], s: &[Sched], t: usize, race: i32) -> String {
    let mut names = vec![chart_race_name(&d.race_names[race as usize])];
    for a in interchangeable(d, m, s, t, race) { names.push(chart_race_name(&d.race_names[a as usize])); }
    names.sort();
    names.join("/")
}
/// Race label for the chart (<= 22 chars).
fn chart_race_name(name: &str) -> String {
    let m: &[(&str, &str)] = &[("Tokyo Yushun (Japanese Derby)", "Japanese Derby"), ("Asahi Hai Futurity Stakes", "Asahi Hai FS"),
        ("Mile Championship Nambu Hai", "MCS Nambu Hai"), ("Hanshin Juvenile Fillies", "Hanshin JF"), ("Zen-Nippon Junior Yushun", "Zen-Nippon Jr Yushun")];
    m.iter().find(|(n, _)| *n == name).map(|(_, a)| a.to_string()).unwrap_or_else(|| name.chars().take(22).collect())
}

/// G1s that can replace shared race `race` at turn `t` with no change to the score: every member running it there
/// is offered the alternative at that turn with the same win-rate basis (so the same odds at the same streak position),
/// it is nobody's goal, and neither race appears anywhere else in any member's schedule (so all pair overlaps are
/// preserved when all runners switch together).
fn interchangeable(d: &Data, m: &[usize], s: &[Sched], t: usize, race: i32) -> Vec<i32> {
    let n = m.len();
    let appears = |r: i32, skip_t: Option<usize>| (0..n).any(|i| (0..NT).any(|tt| Some(tt) != skip_t && matches!(s[i][tt], Choice::Opt(o) if d.umas[m[i]].opts[tt][o as usize].race == r)));
    if appears(race, Some(t)) { return vec![]; }
    let runners: Vec<usize> = (0..n).filter(|&i| matches!(s[i][t], Choice::Opt(o) if d.umas[m[i]].opts[t][o as usize].race == race)).collect();
    if runners.iter().any(|&i| match s[i][t] { Choice::Opt(o) => d.umas[m[i]].opts[t][o as usize].goal, _ => false }) { return vec![]; }
    let mut alts: Vec<i32> = vec![];
    for i in &runners { for op in &d.umas[m[*i]].opts[t] { if op.race >= 0 && op.race != race && !alts.contains(&op.race) { alts.push(op.race); } } }
    alts.retain(|&a| !appears(a, None) && runners.iter().all(|&i| {
        let cur = match s[i][t] { Choice::Opt(o) => &d.umas[m[i]].opts[t][o as usize], _ => unreachable!() };
        d.umas[m[i]].opts[t].iter().any(|op| op.race == a && op.bp == cur.bp && !op.goal)
    }));
    alts
}

/// A job file's mode ("deck" or "career"); the rest of the file is the job itself.
#[derive(Deserialize)]
struct Jobs { #[serde(default)] mode: String }


/// Exact pairwise ceilings PB(a,b) = min(BR_a(W = PM_b), BR_b(W = PM_a)) for all pairs in `pool`.
/// Valid because P_b <= PM_b elementwise and P_a >= 0, so <P_a,P_b> <= <P_a,PM_b> <= BR_a(PM_b).
fn pair_ceilings(d: &Data, pool: &[usize]) -> Vec<Vec<f64>> {
    let n = d.umas.len();
    let mut pb = vec![vec![0.0f64; n]; n];
    if let Some(t) = &d.pbt {
        for (x, &a) in pool.iter().enumerate() { for &b in &pool[x + 1..] { pb[a][b] = t[a * n + b]; pb[b][a] = t[b * n + a]; } }
        return pb;
    }
    let br = |a: usize, b: usize| best_response(d, &d.umas[a], &d.umas[b].pm, None).map(|r| r.1).unwrap_or(0.0);
    for (x, &a) in pool.iter().enumerate() { for &b in &pool[x + 1..] { let v = br(a, b).min(br(b, a)); pb[a][b] = v; pb[b][a] = v; } }
    pb
}
/// The full pairwise-ceiling table of a data.json, as stored in its "pb" field (computed fresh, ignoring any table).
fn pb_table_json(json: &str) -> serde_json::Value {
    let raw: DataRaw = serde_json::from_str(json).unwrap();
    let fp = pb_fingerprint(&raw);
    let mut d = load_str(json); d.pbt = None;
    let n = d.umas.len();
    let all: Vec<usize> = (0..n).collect();
    let pb = pair_ceilings(&d, &all);
    let rows: Vec<String> = (0..n).map(|a| (a + 1..n).map(|b| format!("{:016x}", pb[a][b].to_bits())).collect()).collect();
    serde_json::to_value(PbRaw { fp, rows }).unwrap()
}

pub fn cli_main() {
    struct SbReport; impl Drop for SbReport { fn drop(&mut self) { if std::env::var("SB_CHECK").is_ok() { deck::SBCHK.with(|c| { let c = c.borrow(); eprintln!("SBCHK decks {} max value/exact {:.6} min {:.9} below {} | leaves {} node violations {}", c.0, c.1, c.2, c.3, c.4, c.5); }); }
        if std::env::var("SBOUND").is_ok() || std::env::var("SB_CHECK").is_ok() { deck::SBN.with(|c| { let c = c.borrow(); eprintln!("SBN cfgs cut {} bound evals {} nodes cut {}", c.0, c.1, c.2); }); } } }
    let _sbr = SbReport;
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "pbtable" {
        // print the pairwise-ceiling table for a data.json (./build.sh pb merges it in as "pb")
        println!("{}", pb_table_json(&std::fs::read_to_string(&args[2]).unwrap()));
        return;
    }
    let job_text = std::fs::read_to_string(&args[2]).unwrap();
    let jobs: Jobs = serde_json::from_str(&job_text).unwrap();
    if jobs.mode != "deck" && jobs.mode != "career" { eprintln!("error: \"mode\" must be \"deck\" or \"career\""); std::process::exit(2); }
    let d = load(&args[1]);
    // joint deck + loop search (or the career planner); needs "skills_file" and "cards_file" (web_cards.json)
    let mut v: serde_json::Value = serde_json::from_str(&job_text).unwrap();
    let sk: skills::SkillsRaw = serde_json::from_str(&std::fs::read_to_string(v["skills_file"].as_str().expect("skills_file")).unwrap()).unwrap();
    if let Some(path) = v["events_file"].as_str() {
        let db: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
        deck::apply_events_db(&mut v, &sk, &db);
    }
    let cr: deck::CardsRaw = serde_json::from_str(&std::fs::read_to_string(v["cards_file"].as_str().expect("cards_file")).unwrap()).unwrap();
    // the worker's kept work across processes, as the page keeps it across reloads (LOOPOPT_STATE_IN: a file
    // `state_out` wrote, restored before the jobs; LOOPOPT_STATE_OUT: written after them)
    if jobs.mode == "career" { if let Ok(path) = std::env::var("LOOPOPT_STATE_IN") {
        let st: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        eprintln!("state_in: {}", deck::run_career(&d, &sk, &cr, &serde_json::json!({"cmd": "state_in", "state": st["state"]})));
    } }
    let state_save = |d: &Data| if jobs.mode == "career" { if let Ok(path) = std::env::var("LOOPOPT_STATE_OUT") {
        std::fs::write(&path, deck::run_career(d, &sk, &cr, &serde_json::json!({"cmd": "state_out"})).to_string()).unwrap();
    } };
    // several career jobs (same skills, cards and events files): run in turn in one process, one output line each,
    // as the page's solver worker runs them (what one job leaves in the solver's caches the next can use)
    if jobs.mode == "career" && args.len() > 3 {
        for path in &args[2..] {
            let mut v: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            if let Some(ev) = v["events_file"].as_str() {
                let db: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(ev).unwrap()).unwrap();
                deck::apply_events_db(&mut v, &sk, &db);
            }
            let t0 = Instant::now();
            let mut out = deck::run_career(&d, &sk, &cr, &v);
            out["timing_s"] = serde_json::json!(t0.elapsed().as_secs_f64());
            println!("{}", out);
        }
        state_save(&d);
        return;
    }
    let t0 = Instant::now();
    let mut out = if jobs.mode == "career" { deck::run_career(&d, &sk, &cr, &v) } else { deck::run_deck(&d, &sk, &cr, &v) };
    out["timing_s"] = serde_json::json!(t0.elapsed().as_secs_f64());
    println!("{}", out);
    state_save(&d);
}

// ---------------------------------------------------------------------------------------------
// Correctness tests of the affinity core. Reference values in tests/fixtures/ were produced independently of this code
// (exact MILP, scipy/HiGHS) on the frozen data_v1.json:
//   br_milp.json, br_rest_milp.json  best responses, incl. forced races and rests and the special-goal umas
//   pb_milp.json                     pair ceilings
// Run with: cargo test --release
// ---------------------------------------------------------------------------------------------
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::OnceLock;

    fn data() -> &'static Data {
        static D: OnceLock<Data> = OnceLock::new();
        D.get_or_init(|| load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json")))
    }
    fn fixture(name: &str) -> serde_json::Value {
        let p = format!("{}/tests/fixtures/{}", env!("CARGO_MANIFEST_DIR"), name);
        serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()
    }
    fn ix(d: &Data, id: u32) -> usize { d.umas.iter().position(|u| u.id == id).unwrap() }
    fn opt_at(u: &Uma, t: usize, prog: u32) -> Choice { Choice::Opt(u.opts[t - 1].iter().position(|o| o.prog == prog).unwrap() as u8) }
    struct Lcg(u64);
    impl Lcg { fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
               fn f(&mut self) -> f64 { self.next() as f64 / (1u64 << 31) as f64 } }

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
        let rows = skills::chart_rows(d, &[ix(d, 1019)], &[s]);
        let ks: Vec<u64> = rows.as_array().unwrap().iter().map(|r| r["cells"][0]["row"].as_u64().unwrap()).collect();
        assert_eq!(ks, vec![1, 2, 3]);
    }

    /// The exact DP equals the independent MILP optimum (scipy/HiGHS) on 200 cases with optional forced races (turn-49
    /// goals, multi-turn goal alternatives, repeat-race-heavy weights) and 71 rest-forced ones (turns 24/25/48/49/50 for
    /// the turn-49-goal umas Meisho Doto 1058 and Mejiro Palmer 1064, and Haru Urara 1052); returned schedules are
    /// feasible and honor the force.
    #[test]
    fn best_response_matches_milp() {
        let d = data();
        let mut n = 0;
        for (file, rest) in [("br_milp.json", false), ("br_rest_milp.json", true)] {
            for c in fixture(file).as_array().unwrap() {
                let u = &d.umas[ix(d, c["uma"].as_u64().unwrap() as u32)];
                let mut w = vec![0.0; d.nr];
                for e in c["weights"].as_array().unwrap() { w[e[0].as_u64().unwrap() as usize] = e[1].as_f64().unwrap(); }
                let force = if rest { Some((c["force_rest"].as_u64().unwrap() as usize - 1, Choice::Rest)) } else {
                    c["force"].as_array().map(|fa| { let t = fa[0].as_u64().unwrap() as usize; (t - 1, opt_at(u, t, fa[1].as_u64().unwrap() as u32)) }) };
                let (s, v) = best_response(d, u, &w, force).expect("infeasible");
                let want = c["milp_value"].as_f64().unwrap();
                assert!((v - want).abs() < 1e-6, "{} {} {:?}: dp {v} vs milp {want}", u.name, c["kind"], force);
                assert!(feasible(u, &s));
                if let Some((t, ch)) = force { assert_eq!(s[t], ch); }
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
        let mut t_goal = 0; for t in 0..NT { if u.forced[t].is_some() { t_goal = t; break; } }
        let mut bad = s; bad[t_goal] = Choice::Rest;
        assert!(!feasible(u, &bad), "missing goal accepted");
        // option index out of range
        let mut bad2 = s; bad2[5] = Choice::Opt(9);
        assert!(!feasible(u, &bad2), "nonexistent option accepted");
        // multi-turn goal alternative: Fuji Kiseki must run NHK Mile (t33) or the Derby (t34)
        let fk = &d.umas[ix(d, 1005)];
        let (s3, _) = best_response(d, fk, &w, None).unwrap();
        let mut bad3 = s3; bad3[32] = Choice::Rest; bad3[33] = Choice::Rest;
        assert!(!feasible(fk, &bad3), "missing multi-turn goal alternative accepted");
    }

    /// A stored pairwise-ceiling table that matches its data.json is bit-equal to a fresh computation, and a table
    /// stops matching when the data changes. (A stale table is only a warning: the solver ignores it.)
    #[test]
    fn stored_pb_tables_match_fresh() {
        for path in [concat!(env!("CARGO_MANIFEST_DIR"), "/data.json"), concat!(env!("CARGO_MANIFEST_DIR"), "/../../jp/data.json")] {
            let txt = std::fs::read_to_string(path).unwrap();
            let d = load_str(&txt);
            let Some(t) = d.pbt.as_ref() else { eprintln!("warning: {path} has no current pb table (run ./build.sh pb)"); continue; };
            let mut fresh = load_str(&txt); fresh.pbt = None;
            let n = d.umas.len();
            let pool: Vec<usize> = (0..n).collect();
            let pb = pair_ceilings(&fresh, &pool);
            for a in 0..n { for b in 0..n { if a != b { assert_eq!(t[a * n + b].to_bits(), pb[a][b].to_bits(), "{path} PB({a},{b})"); } } }
            let mut v: serde_json::Value = serde_json::from_str(&txt).unwrap();
            v["pen"][2] = serde_json::json!(v["pen"][2].as_i64().unwrap() + 1);
            assert!(load_str(&v.to_string()).pbt.is_none(), "{path}: a table survived a data change");
        }
    }

    /// Pair ceilings equal the MILP-computed table.
    #[test]
    fn pair_ceilings_match_milp() {
        let d = data();
        let all: Vec<usize> = (0..d.umas.len()).collect();
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
        let pr = |a: usize, b: usize| d.pair[a][b] as f64;
        let tr = |a: usize, b: usize, c: usize| if a == c || b == c { 0.0 } else { d.tri[a][b][c] as f64 };
        for _ in 0..60 {
            let mut f = [0usize; 4];
            loop { for x in f.iter_mut() { *x = (rng.next() as usize) % d.umas.len(); }
                   let mut g = f; g.sort(); if g.windows(2).all(|w| w[0] != w[1]) { break; } }
            let s: Vec<Sched> = (0..4).map(|i| {
                let u = &d.umas[f[i]];
                let w: Vec<f64> = (0..d.nr).map(|_| rng.f() * 3.0).collect();
                best_response(d, u, &w, None).unwrap().0
            }).collect();
            let p: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &s[i])).collect();
            let o = |a: usize, b: usize| wval(&p[a], &p[b]);
            let um = |i: usize| f[i];
            let w = 0.5;
            // lineage affinity: parents (T,P1,P2), P1's grandparents g1, P2's grandparents g2 (indices into f)
            let lin = |t: usize, p1: usize, p2: usize, g1: [usize; 2], g2: [usize; 2]| -> f64 {
                let par = pr(um(t), um(p1)) + pr(um(t), um(p2)) + pr(um(p1), um(p2)) + 3.0 * o(p1, p2);
                let mut gp = 0.0;
                for &g in &g1 { gp += tr(um(t), um(p1), um(g)) + 3.0 * o(p1, g); }
                for &g in &g2 { gp += tr(um(t), um(p2), um(g)) + 3.0 * o(p2, g); }
                par + w * gp
            };
            // P1 bred from (P2, X), P2 bred from (P1, X); normalized to 4 runs
            let mut scf = 0.0;
            for t in 0..4 { for x in 0..4 { if x == t { continue; }
                let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
                scf += lin(t, ps[0], ps[1], [ps[1], x], [ps[0], x]); } }
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
        let cases: [[u32; 4]; 4] = [[1007, 1019, 1021, 1049], [1009, 1014, 1019, 1029], [1015, 1017, 1023, 1030], [1006, 1019, 1029, 1034]];
        for ids in cases {
            let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
            let so = optimize(d, m, 3, 5, 60.0).s;
            let lp = make_loop(d, m);
            let v = score(d, &lp, &so);
            let mut runners = vec![0u8; d.nr];
            for i in 0..4 { let mut seen = vec![false; d.nr];
                for t in 0..NT { if let Choice::Opt(o) = so[i][t] { let rr = d.umas[m[i]].opts[t][o as usize].race; if rr >= 0 && !seen[rr as usize] { seen[rr as usize] = true; runners[rr as usize] += 1; } } } }
            let mut stripped = so;
            for i in 0..4 { for t in 0..NT { if let Choice::Opt(o) = stripped[i][t] { let op = &d.umas[m[i]].opts[t][o as usize];
                if op.race >= 0 && runners[op.race as usize] == 1 && !op.goal { stripped[i][t] = Choice::Rest; } } } }
            for i in 0..4 { assert!(feasible(&d.umas[m[i]], &stripped[i])); }
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
            for i in 0..3 { assert!(feasible(&d.umas[m[i]], &s3[i])); }
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
        for ids in [[1007u32, 1019, 1021, 1049], [1009, 1014, 1019, 1029], [1006, 1019, 1029, 1034], [1015, 1017, 1023, 1030]] {
            let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
            let r = optimize(d, m, 3, 5, 60.0);
            let lp = make_loop(d, m);
            let v = score(d, &lp, &r.s);
            for t in 0..NT {
                let mut races: Vec<i32> = vec![];
                for i in 0..4 { if let Choice::Opt(o) = r.s[i][t] { let rr = d.umas[m[i]].opts[t][o as usize].race; if rr >= 0 && !races.contains(&rr) { races.push(rr); } } }
                for race in races {
                    for a in interchangeable(d, &m, &r.s, t, race) {
                        let mut s2 = r.s;
                        for i in 0..4 { if let Choice::Opt(o) = s2[i][t] { if d.umas[m[i]].opts[t][o as usize].race == race {
                            let k = d.umas[m[i]].opts[t].iter().position(|op| op.race == a).unwrap(); s2[i][t] = Choice::Opt(k as u8); } } }
                        for i in 0..4 { assert!(feasible(&d.umas[m[i]], &s2[i])); }
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
        let d = load(concat!(env!("CARGO_MANIFEST_DIR"), "/data.json"));
        let table: [(i32, [i32; 6]); 13] = [(0, [110, 110, 105, 90, 80, 60]), (10, [100, 100, 95, 80, 70, 50]), (20, [90, 90, 85, 70, 60, 40]),
            (30, [80, 80, 75, 60, 50, 30]), (40, [70, 70, 65, 50, 40, 20]), (60, [50, 50, 45, 30, 20, 0]), (90, [20, 20, 15, 0, 0, 0]),
            (100, [10, 10, 5, 0, 0, 0]), (110, [0; 6]), (120, [0; 6]), (130, [0; 6]), (150, [0; 6]), (180, [0; 6])];
        for (cost, vals) in table {
            let bp = (110 - cost).max(-20);
            for k in 1..=6 { assert_eq!(pk(&d, bp, k), (vals[k - 1].clamp(0, 100)) as f64 / 100.0, "cost {cost} k {k}"); }
        }
    }
}

mod skills;
mod deck;
#[cfg(target_arch = "wasm32")]
mod web;
