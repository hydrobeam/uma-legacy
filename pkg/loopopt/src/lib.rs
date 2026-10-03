//! # loopopt: Uma Musume (Global) parent-loop affinity optimizer
//!
//! Chooses 4 umas and a full race schedule for each so that expected legacy affinity is maximal.
//!
//! Objective, normalized to 4 runs:
//!     E = base + 3 * sum_{i<j} C_ij * <P_i, P_j>
//! where P_i[r] = P(member i wins G1 race r at least once) = 1 - prod(1 - p) over her attempts at r,
//! and p = clamp(110 - 10*off(distance) - 10*off(surface) - PEN[streak position], 0, 100) / 100.
//! Self-copy-free breeding ("scf"): C_ij = 10/3, base = 2*sum pair + 4*sum trio.
//! Rotation ("rot", i -> parents i-1, i-2): C = 3 adjacent / 4 diagonal, base = 2*sum pair + 3*sum trio.
//!
//! Layers:
//! * `dp_exact`: exact best response for one uma against fixed partners. Junior turns run in sequence;
//!   Classic turn t and Senior turn t+24 are decided jointly, so a race offered in both years
//!   (always 24 turns apart) is scored exactly. Validated against an exact MILP on 352 cases.
//! * `ascent`: best-response dynamics; converges to a Nash equilibrium of the identical-interest game.
//! * `group_moves`: coalition deviations (2+ members forced onto the same race, or rest, at one turn),
//!   which escape the coordination equilibria that single-member moves can't leave.
//! * `optimize`: optimistic start + ascent + group moves + iterated local search (noisy best responses).
//! * `run_best`: exact pairwise ceilings -> a guaranteed upper bound for every foursome -> best-first
//!   search that stops once no unsearched foursome can enter the top k.
//!
//! Relocated-venue G1s are merged into their normal race by the exporter (export/export_rs.py).
//! See README.md for the data format, modes, validation and benchmarks.

use serde::{Deserialize, Serialize};
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant;
/// wasm32-unknown-unknown has no clock (Instant::now panics); timings read as zero there.
#[cfg(target_arch = "wasm32")]
#[derive(Clone, Copy)]
struct Instant;
#[cfg(target_arch = "wasm32")]
impl Instant { fn now() -> Self { Instant } fn elapsed(&self) -> std::time::Duration { std::time::Duration::ZERO } }
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
pub static PROF: [AtomicU64; 8] = [const { AtomicU64::new(0) }; 8];
fn prof(i: usize, t: Instant) { PROF[i].fetch_add(t.elapsed().as_nanos() as u64, Relaxed); PROF[i+4].fetch_add(1, Relaxed); }
/// PROTOTYPE phase timers (perf review): inclusive nanoseconds and call counts per phase, printed by the CLI when
/// LOOPOPT_PROF is set.
pub static PH: [AtomicU64; 48] = [const { AtomicU64::new(0) }; 48];
pub(crate) const PH_NAMES: [&str; 24] = ["load", "model+sources", "card dominance", "chain_marginals", "enumeration+pareto",
    "skyline", "pair_ceilings", "class thinning", "btab fill", "band gen", "relaxed()", "optimize()", "best_for()",
    "alt deck search", "output json", "shared ceilings", "shared front scoring", "make_key", "run_deck_relaxed", "out:ring+last", "out:count", "out:remove+insert", "k_filter", "out:all"];
pub(crate) struct PhT(pub usize, pub Instant);
impl Drop for PhT { fn drop(&mut self) { PH[self.0].fetch_add(self.1.elapsed().as_nanos() as u64, Relaxed); PH[self.0 + 24].fetch_add(1, Relaxed); } }
#[macro_export]
macro_rules! ph { ($i:expr) => { let _ph = $crate::PhT($i, Instant::now()); } }
pub(crate) fn ph_report() {
    if std::env::var("LOOPOPT_PROF").is_err() { return; }
    for i in 0..24 { let n = PH[i + 24].load(Relaxed); if n == 0 { continue; }
        eprintln!("PH {:>22} {:>9.4}s  calls {:>9}", PH_NAMES[i], PH[i].load(Relaxed) as f64 / 1e9, n); }
}

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

#[derive(Deserialize, Clone)]
struct OptRaw { prog: u32, race: i32, bp: i32, #[serde(default)] name: String, #[serde(default)] goal: bool }
#[derive(Deserialize)]
struct UmaRaw { id: u32, name: String, turns: Vec<Vec<OptRaw>>, goals: Vec<Vec<(usize, usize)>> }
#[derive(Deserialize)]
struct DataRaw { race_names: Vec<String>, umas: Vec<UmaRaw>, pair: Vec<Vec<i32>>, tri: Vec<Vec<Vec<i32>>>, pen: Vec<i32>, #[serde(default)] pb: Option<PbRaw> }
/// Precomputed pairwise ceilings (`loopopt pbtable`, merged into data.json by `./build.sh pb`): `fp` is
/// `pb_fingerprint` of the data it was computed from; row a holds PB(a, b) for b > a as 16-hex-digit f64 bit patterns.
#[derive(Deserialize, Serialize)]
struct PbRaw { fp: String, rows: Vec<String> }

#[derive(Clone, Copy, PartialEq, Debug)]
enum Choice { Rest, Opt(u8) }

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
}

/// Precomputed per-choice data for dp_exact: race (-1 = none), win chance per streak position k (same values as `pk`), goal bits.
#[derive(Clone, Copy)]
struct Ent { race: i32, rest: bool, bit: u32, p: [f64; 7] }
const ENT_REST: Ent = Ent { race: -1, rest: true, bit: 0, p: [0.0; 7] };

struct Data { nr: usize, umas: Vec<Uma>, pair: Vec<Vec<i32>>, tri: Vec<Vec<Vec<i32>>>, pk: Vec<[f64; 7]>, race_names: Vec<String>,
    pbt: Option<Vec<f64>> }   // n x n pairwise ceilings from data.json's table, when its fingerprint matches

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
            pm: miss.iter().map(|m| 1.0 - m).collect(), ent, chl });
    }
    Data { nr, umas, pair: raw.pair, tri: raw.tri, pk: pkt, race_names: raw.race_names, pbt }
}

type Sched = [Choice; NT];

fn probs(d: &Data, u: &Uma, s: &Sched) -> Vec<f64> {
    let t0 = Instant::now(); let r = probs_inner(d, u, s); prof(2, t0); r
}
fn probs_inner(d: &Data, u: &Uma, s: &Sched) -> Vec<f64> { let mut v = vec![]; probs_into(d, u, s, &mut v); v }
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

#[allow(dead_code)]
/// Additive DP (no repeat-race coupling): maximize sum over raced options of w[t][o] * pk(bp, streak position), subject to goals and `fixed` choices.
fn dp(d: &Data, u: &Uma, w: &[Vec<f64>], fixed: &[Option<Choice>]) -> Option<Sched> {
    let nm = (u.full + 1) as usize;
    let neg = f64::NEG_INFINITY;
    // V[t][s][m], t in 0..=NT
    let idx = |t: usize, s: usize, m: usize| (t * NS + s) * nm + m;
    let mut v = vec![neg; (NT + 1) * NS * nm];
    for s in 0..NS { v[idx(NT, s, u.full as usize)] = 0.0; }
    let mut arg = vec![0u8; NT * NS * nm]; // 255 = rest, else option idx
    for t in (0..NT).rev() {
        let nopt = u.opts[t].len();
        for s in 0..NS {
            for m in 0..nm {
                let mut best = neg; let mut bc = 255u8;
                let allowed = |c: Choice| -> bool {
                    if let Some(fc) = fixed[t] { return fc == c; }
                    match c { Choice::Rest => u.forced[t].is_none(), Choice::Opt(o) => u.forced[t].as_ref().map_or(true, |f| f.contains(&o)) }
                };
                if allowed(Choice::Rest) {
                    let x = v[idx(t + 1, 0, m)];
                    if x > best { best = x; bc = 255; }
                }
                let k = (s + 1).min(6);
                for o in 0..nopt {
                    if !allowed(Choice::Opt(o as u8)) { continue; }
                    let op = &u.opts[t][o];
                    let nm2 = m | u.bits[t][o] as usize;
                    let nxt = v[idx(t + 1, k, nm2)];
                    if nxt == neg { continue; }
                    let r = if op.race >= 0 { w[t][o] * pk(d, op.bp, k) } else { 0.0 };
                    let x = r + nxt;
                    if x > best { best = x; bc = o as u8; }
                }
                v[idx(t, s, m)] = best;
                arg[(t * NS + s) * nm + m] = bc;
            }
        }
    }
    if v[idx(0, 0, 0)] == neg { return None; }
    let mut out = [Choice::Rest; NT];
    let (mut s, mut m) = (0usize, 0usize);
    for t in 0..NT {
        let c = arg[(t * NS + s) * nm + m];
        if c == 255 { out[t] = Choice::Rest; s = 0; } else {
            out[t] = Choice::Opt(c); s = (s + 1).min(6); m |= u.bits[t][c as usize] as usize;
        }
    }
    Some(out)
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
fn best_response(d: &Data, u: &Uma, wr: &[f64], _cur: Option<&Sched>, force: Option<(usize, Choice)>) -> Option<(Sched, f64)> {
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
    let t0 = Instant::now();
    let r = dp_exact(d, u, wr, force);
    prof(0, t0);
    let (s, v) = r?;
    let ev = wval(wr, &probs(d, u, &s));
    assert!((ev - v).abs() < 1e-6, "dp_exact value {v} != evaluated {ev} for {}", u.name);
    assert!(feasible(u, &s), "best response produced an infeasible schedule for {}", u.name);
    Some((s, ev))
}

struct Loop { m: [usize; 4], c: [[f64; 4]; 4], base: f64 }

/// Parse a scheme string: "scf" | "rot", optionally "@w" where w = grandparent-slot weight
/// (grandparent spark base rates as a fraction of parents'; w = 1 is plain total affinity).
fn parse_scheme(scheme: &str) -> (bool, f64) {
    let (kind, w) = match scheme.split_once('@') { Some((k, w)) => (k, w.parse::<f64>().expect("bad grandparent weight")), None => (scheme, 1.0) };
    match kind { "scf" => (true, w), "rot" => (false, w), _ => panic!("unknown scheme {scheme}") }
}
/// Race weights (C_adjacent, C_diagonal) and trio coefficient for grandparent weight w.
/// Per lineage, parent slots carry pair(T,Pi) + pair(P1,P2)/2 + 3*O(P1,P2)/2 each; a grandparent slot carries
/// tri(T,Pi,G) + 3*O(Pi,G), scaled by w. Summed over the lineages and normalized to 4 runs:
///   scf: base = 2*sum pair + 4w*sum trio, C = (2+8w)/3 for every pair
///   rot: base = 2*sum pair + 3w*sum trio, C = 1+2w adjacent, 4w diagonal
/// w = 1 gives the plain model (10/3; 3 and 4).
fn scheme_weights(scf: bool, w: f64) -> (f64, f64, f64) {
    if scf { let c = (2.0 + 8.0 * w) / 3.0; (c, c, 4.0 * w) } else { (1.0 + 2.0 * w, 4.0 * w, 3.0 * w) }
}
/// Per-legacy affinity for each lineage: (trainee, [(uma, is_grandparent, affinity); 6]) in slot order P1, G11, G12, P2, G21, G22.
/// Parent i: pair(T,Pi) + pair(P1,P2)/2 + 3*O(P1,P2)/2. Grandparent G under Pi: tri(T,Pi,G) + 3*O(Pi,G) (0 trio if G = T).
/// O = expected shared G1 wins. scf: 12 lineages (P1 bred from P2+X, P2 from P1+X); rot: 4 runs in the given order.
fn legacy_affinities(d: &Data, m: [usize; 4], s: &[Sched], scf: bool) -> Vec<(usize, Vec<(usize, bool, f64)>)> {
    let p: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[m[i]], &s[i])).collect();
    let o = |a: usize, b: usize| wval(&p[a], &p[b]);
    let pr = |a: usize, b: usize| d.pair[m[a]][m[b]] as f64;
    let tr = |a: usize, b: usize, c: usize| if c == a || c == b { 0.0 } else { d.tri[m[a]][m[b]][m[c]] as f64 };
    let lin = |t: usize, p1: usize, p2: usize, g1: [usize; 2], g2: [usize; 2]| {
        let shared = pr(p1, p2) / 2.0 + 1.5 * o(p1, p2);
        let mut v = vec![(m[p1], false, pr(t, p1) + shared)];
        for g in g1 { v.push((m[g], true, tr(t, p1, g) + 3.0 * o(p1, g))); }
        v.push((m[p2], false, pr(t, p2) + shared));
        for g in g2 { v.push((m[g], true, tr(t, p2, g) + 3.0 * o(p2, g))); }
        (m[t], v)
    };
    let mut out = vec![];
    if scf {
        for t in 0..4 { for x in 0..4 { if x == t { continue; }
            let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
            out.push(lin(t, ps[0], ps[1], [ps[1], x], [ps[0], x])); } }
    } else {
        for i in 0..4 { let a = |k: usize| (i + 4 - k) % 4; out.push(lin(i, a(1), a(2), [a(2), a(3)], [a(3), a(0)])); }
    }
    out
}
fn make_loop(d: &Data, m: [usize; 4], scheme: &str) -> Loop {
    let (scf, w) = parse_scheme(scheme);
    let (cadj, cdiag, kt) = scheme_weights(scf, w);
    let mut c = [[0.0; 4]; 4];
    let base;
    let pr = |a: usize, b: usize| d.pair[a][b] as f64;
    let tr = |a: usize, b: usize, x: usize| d.tri[a][b][x] as f64;
    let mut psum = 0.0; let mut tsum = 0.0;
    for i in 0..4 { for j in i + 1..4 { psum += pr(m[i], m[j]); for k in j + 1..4 { tsum += tr(m[i], m[j], m[k]); } } }
    // scf: every pair weighted alike. rot (given order): positions two apart are the diagonal. Base is order-invariant.
    for i in 0..4 { for j in 0..4 { if i != j { c[i][j] = if (i + 2) % 4 == j { cdiag } else { cadj }; } } }
    base = 2.0 * psum + kt * tsum;
    Loop { m, c, base }
}

fn score(d: &Data, lp: &Loop, s: &[Sched; 4]) -> f64 {
    let t0 = Instant::now(); let r = score_inner(d, lp, s); prof(1, t0); r
}
fn score_inner(d: &Data, lp: &Loop, s: &[Sched; 4]) -> f64 {
    let p: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    score_ps(lp, &p)
}
/// Score from precomputed probs (p[i] must equal probs(s[i]); probs is deterministic, so the result is bit-identical to `score`).
fn score_ps(lp: &Loop, p: &[Vec<f64>]) -> f64 {
    let mut tot = lp.base;
    for i in 0..4 { for j in i + 1..4 { tot += 3.0 * lp.c[i][j] * wval(&p[i], &p[j]); } }
    tot
}

fn weights(d: &Data, lp: &Loop, ps: &[Vec<f64>], i: usize) -> Vec<f64> {
    let mut w = vec![]; weights_into(d, lp, ps, i, &mut w); w
}
fn weights_into(d: &Data, lp: &Loop, ps: &[Vec<f64>], i: usize, w: &mut Vec<f64>) {
    w.clear(); w.resize(d.nr, 0.0);
    for j in 0..4 { if j != i { for r in 0..d.nr { w[r] += 3.0 * lp.c[i][j] * ps[j][r]; } } }
}

fn ascent(d: &Data, lp: &Loop, s: &mut [Sched; 4]) -> f64 {
    let mut ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    ascent_ps(d, lp, s, &mut ps)
}
/// `ascent` with the members' probs supplied (ps[i] must equal probs(s[i])); ps is kept equal to probs(s[i]).
fn ascent_ps(d: &Data, lp: &Loop, s: &mut [Sched; 4], ps: &mut [Vec<f64>]) -> f64 {
    let mut w: Vec<f64> = Vec::with_capacity(d.nr);
    // stale[i]: some partner's schedule changed since member i was last solved (or she was never solved here).
    // If not stale, her weights are bit-identical to her last solve, so the deterministic best response is the same
    // schedule: either it was rejected then (and would be again) or it was accepted and now equals s[i] exactly
    // (nv == curv, not a strict improvement). Skipping her therefore cannot change the trajectory.
    let mut stale = [true; 4];
    loop {
        let mut improved = false;
        for i in 0..4 {
            if !stale[i] { continue; }
            stale[i] = false;
            let u = &d.umas[lp.m[i]];
            weights_into(d, lp, ps, i, &mut w);
            let curv = wval(&w, &ps[i]);
            if let Some((ns, nv)) = best_response(d, u, &w, Some(&s[i]), None) {
                if nv > curv + 1e-9 {
                    s[i] = ns; probs_into(d, u, &ns, &mut ps[i]); improved = true;
                    for j in 0..4 { if j != i { stale[j] = true; } }
                }
            }
        }
        if !improved { break; }
    }
    score_ps(lp, ps) // ps[i] == probs(s[i]) is maintained above
}

/// Coordinated moves: force a subset of >=2 members onto the same race (or rest) at one turn, then re-ascend.
fn group_moves(d: &Data, lp: &Loop, s: &mut [Sched; 4], margin: f64, stats: &mut u64) -> f64 {
    let mut cur = score(d, lp, s);
    // probs of the current *s, refreshed whenever *s changes (probs is deterministic, so this equals recomputing per mask)
    let mut ps_s: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[lp.m[i]], &s[i])).collect();
    let mut ps: Vec<Vec<f64>> = ps_s.clone();
    let mut w: Vec<f64> = Vec::with_capacity(d.nr);
    loop {
        let mut improved = false;
        for t in 0..NT {
            // candidate targets: each race offered at t by any member, plus rest
            let mut targets: Vec<Option<i32>> = vec![None];
            for i in 0..4 { for op in &d.umas[lp.m[i]].opts[t] { if op.race >= 0 && !targets.contains(&Some(op.race)) { targets.push(Some(op.race)); } } }
            for tg in targets {
                // choice for each member to hit the target at t (None = can't)
                let mut ch: [Option<Choice>; 4] = [None; 4];
                let mut on = [false; 4];
                for i in 0..4 {
                    let u = &d.umas[lp.m[i]];
                    let c = match tg {
                        None => if u.forced[t].is_none() { Some(Choice::Rest) } else { None },
                        Some(r) => u.opts[t].iter().position(|op| op.race == r).map(|o| Choice::Opt(o as u8))
                            .filter(|c| match (c, &u.forced[t]) { (Choice::Opt(o), Some(f)) => f.contains(o), _ => true }),
                    };
                    ch[i] = c;
                    on[i] = c.map_or(false, |c| s[i][t] == c);
                }
                let movable: Vec<usize> = (0..4).filter(|&i| ch[i].is_some() && !on[i]).collect();
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
                        weights_into(d, lp, &ps, i, &mut w);
                        match best_response(d, u, &w, Some(&ns[i]), Some((t, ch[i].unwrap()))) {
                            Some((x, _)) => { ns[i] = x; probs_into(d, u, &x, &mut ps[i]); }
                            None => { ok = false; break; }
                        }
                    }
                    if !ok { continue; }
                    *stats += 1;
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

fn optimistic_start(d: &Data, lp: &Loop) -> [Sched; 4] {
    let ps: Vec<Vec<f64>> = (0..4).map(|i| d.umas[lp.m[i]].pm.clone()).collect();
    let mut s = [[Choice::Rest; NT]; 4];
    for i in 0..4 {
        let w = weights(d, lp, &ps, i);
        s[i] = best_response(d, &d.umas[lp.m[i]], &w, None, None).expect("infeasible goals").0;
    }
    s
}

struct Res { val: f64, s: [Sched; 4], order: [usize; 4], evals: u64, secs: f64 }

/// Tie-break pass: each member re-solves against her partners with a tiny bonus (EPS per expected G1 win) on every G1,
/// so races that cost no affinity get added (they can still pass down race sparks). Accepted only if the affinity
/// score does not drop by more than 1e-6; races nobody else runs leave every partner's weights unchanged.
const FILL_EPS: f64 = 1e-7;
fn fill_optional(d: &Data, lp: &Loop, s: &mut [Sched; 4], v: f64) -> f64 {
    let mut cur = score(d, lp, s);
    debug_assert!((cur - v).abs() < 1e-6);
    for i in 0..4 {
        let ps: Vec<Vec<f64>> = (0..4).map(|k| probs(d, &d.umas[lp.m[k]], &s[k])).collect();
        let w: Vec<f64> = weights(d, lp, &ps, i).iter().map(|x| x + FILL_EPS).collect();
        if let Some((x, _)) = best_response(d, &d.umas[lp.m[i]], &w, None, None) {
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
        for i in 0..4 {
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

fn optimize(d: &Data, m: [usize; 4], scheme: &str, iters: usize, seed: u64, margin: f64, init: &[[Sched; 4]]) -> Res {
    ph!(11);
    let t0 = Instant::now();
    let _memo = MemoScope::new();
    let orders: Vec<[usize; 4]> = if parse_scheme(scheme).0 { vec![m] } else { vec![m, [m[0], m[2], m[1], m[3]], [m[0], m[1], m[3], m[2]]] };
    let mut best: Option<Res> = None;
    let mut evals = 0u64;
    for (oi, ord) in orders.iter().enumerate() {
        let lp = make_loop(d, *ord, scheme);
        let mut rng = Rng(seed.wrapping_mul(2654435761).wrapping_add(oi as u64 + 1) | 1);
        let mut s = optimistic_start(d, &lp);
        let mut v = ascent(d, &lp, &mut s);
        v = v.max(group_moves(d, &lp, &mut s, margin, &mut evals));
        for st in init {
            // init schedules given in canonical member order m; map to this order
            let mut s2 = [[Choice::Rest; NT]; 4];
            for i in 0..4 { let j = m.iter().position(|x| *x == ord[i]).unwrap(); s2[i] = st[j]; }
            if (0..4).all(|i| feasible(&d.umas[ord[i]], &s2[i])) {
                let mut v2 = ascent(d, &lp, &mut s2);
                v2 = v2.max(group_moves(d, &lp, &mut s2, margin, &mut evals));
                if v2 > v { v = v2; s = s2; }
            }
        }
        // iterated local search: perturb 1-2 members against noisy weights, re-ascend, group moves
        for _ in 0..iters {
            let mut ns = s;
            let mut ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[lp.m[i]], &ns[i])).collect();
            let nmove = 1 + (rng.next() % 2) as usize;
            for _ in 0..nmove {
                let i = (rng.next() % 4) as usize;
                let mut w = weights(d, &lp, &ps, i);
                for x in w.iter_mut() { *x *= 0.2 + 1.6 * rng.unif(); }
                if let Some((x, _)) = best_response(d, &d.umas[lp.m[i]], &w, None, None) { ns[i] = x; ps[i] = probs(d, &d.umas[lp.m[i]], &x); }
            }
            ascent_ps(d, &lp, &mut ns, &mut ps);
            let nv = group_moves(d, &lp, &mut ns, margin, &mut evals);
            if nv > v + 1e-7 { v = nv; s = ns; }
        }
        // add optional G1s that cost no affinity (race sparks), then map back to canonical member order m
        let v = fill_optional(d, &lp, &mut s, v);
        let mut sc = [[Choice::Rest; NT]; 4];
        for i in 0..4 { let j = ord.iter().position(|x| *x == m[i]).unwrap(); sc[i] = s[j]; }
        if best.as_ref().map_or(true, |b| v > b.val + 1e-9) { best = Some(Res { val: v, s: sc, order: *ord, evals: 0, secs: 0.0 }); }
    }
    let mut b = best.unwrap(); b.evals = evals; b.secs = t0.elapsed().as_secs_f64(); b
}


fn when(t: usize) -> String {
    let tt = t; let y = if tt < 24 { "Jr" } else if tt < 48 { "Cl" } else { "Sr" };
    let w = tt % 24; let mon = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"][w / 2];
    format!("{} {}-{}", y, if w % 2 == 0 { "E" } else { "L" }, mon)
}
/// Per-race detail for one schedule: turn, label, race, goal, streak position, win chance.
fn sched_detail(d: &Data, u: &Uma, s: &Sched) -> Vec<serde_json::Value> {
    let mut out = vec![]; let mut k = 0usize;
    for t in 0..NT {
        match s[t] {
            Choice::Rest => k = 0,
            Choice::Opt(o) => {
                k = (k + 1).min(6);
                let op = &u.opts[t][o as usize];
                let win = if op.race >= 0 { serde_json::json!(pk(d, op.bp, k)) } else { serde_json::Value::Null };
                out.push(serde_json::json!({"turn": t + 1, "when": when(t), "race": op.name, "program": op.prog, "g1": op.race >= 0, "goal": op.goal, "streak": k, "win": win}));
            }
        }
    }
    out
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
/// Schedule chart in three sections, rows in calendar order, one column per uma:
///   1. shared G1s: races (by identity, either year) run by 2+ members -- these are the affinity;
///   2. goal races (non-G1) that are required;
///   3. optional G1s: races only one member runs anywhere -- no affinity effect, kept because they cost nothing
///      (the schedule already accounts for their streak effect) and can pass down race sparks.
/// G1 cell = win % at that attempt (`*` = goal). `·` = not running it.
fn race_chart(d: &Data, m: &[usize], s: &[Sched]) -> String {
    let n = m.len();
    let ks: Vec<Vec<usize>> = (0..n).map(|i| { let mut k = 0; (0..NT).map(|t| { match s[i][t] { Choice::Rest => k = 0, Choice::Opt(_) => k = (k + 1).min(6) }; k }).collect() }).collect();
    // who runs each G1 (by race identity, any year)
    let mut runners: Vec<Vec<bool>> = vec![vec![false; n]; d.nr];
    for i in 0..n { for t in 0..NT { if let Choice::Opt(o) = s[i][t] { let op = &d.umas[m[i]].opts[t][o as usize]; if op.race >= 0 { runners[op.race as usize][i] = true; } } } }
    let shared = |r: usize| runners[r].iter().filter(|x| **x).count() >= 2;
    let names: Vec<String> = m.iter().map(|&x| short_name(&d.umas[x].name)).collect();
    let header = |title: &str, extra: &str| {
        let mut h = format!("{:<24} {:<9}", title, "when");
        for nm in &names { h += &format!(" {:>8}", nm); }
        h += extra; h += "\n"; h += &"─".repeat(34 + 9 * n + extra.len()); h += "\n"; h
    };
    // rows: (turn, label, kind) kind 0 = shared G1, 1 = goal non-G1, 2 = optional G1
    let mut rows: Vec<(usize, String, u8, i32)> = vec![];
    for t in 0..NT { for i in 0..n {
        if let Choice::Opt(o) = s[i][t] { let op = &d.umas[m[i]].opts[t][o as usize];
            let key = if op.race >= 0 { (t, d.race_names[op.race as usize].clone(), if shared(op.race as usize) { 0 } else { 2 }, op.race) }
                      else { (t, op.name.clone(), 1, -1) };
            if !rows.contains(&key) { rows.push(key); } } } }
    rows.sort();
    let cell = |i: usize, t: usize, label: &str, race: i32| -> (String, bool) {
        match s[i][t] {
            Choice::Opt(o) => { let op = &d.umas[m[i]].opts[t][o as usize];
                let same = if race >= 0 { op.race == race } else { op.race < 0 && op.name == label };
                if !same { ("·".into(), false) }
                else if race < 0 { ((if op.goal { "goal" } else { "run" }).into(), true) }
                else { (format!("{}{:.0}", if op.goal { "*" } else { "" }, pk(d, op.bp, ks[i][t]) * 100.0), true) } }
            Choice::Rest => ("·".into(), false),
        }
    };
    let mut out = String::new();
    let mut any_alt = false;
    // non-G1 goal races (kind 1) are required and carry no sparks or affinity, so they are not listed
    for (kind, title, extra) in [(0u8, "Shared G1s", "   runners"), (2, "Optional G1s", "")] {
        let sel: Vec<&(usize, String, u8, i32)> = rows.iter().filter(|r| r.2 == kind).collect();
        if sel.is_empty() { continue; }
        if !out.is_empty() { out += "\n"; }
        out += &header(title, extra);
        let mut last_year = 99;
        for (t, label, _, race) in sel {
            let y = t / 24;
            if kind == 0 && y != last_year && last_year != 99 { out += "\n"; }
            last_year = y;
            let lab = if *race >= 0 { row_label(d, m, s, *t, *race) } else { chart_race_name(label) };
            if lab.contains('/') { any_alt = true; }
            let mut line = format!("{:<24} {:<9}", lab, when(*t));
            let mut cnt = 0;
            for i in 0..n { let (c, hit) = cell(i, *t, label, *race); if hit { cnt += 1; } line += &format!(" {:>8}", c); }
            if kind == 0 { line += &format!("   {}/{}", cnt, n); }
            out += line.trim_end(); out += "\n";
        }
    }
    if any_alt { out += "\nA/B = either race, same odds; every runner must pick the same one.\n"; }
    out
}

/// Markdown sheet: schedule chart, P(win at least once) and pair overlaps.
fn render_md(d: &Data, m: [usize; 4], s: &[Sched; 4], scheme: &str, val: f64) -> String {
    let lp = make_loop(d, m, scheme);
    let names: Vec<&str> = m.iter().map(|&x| d.umas[x].name.as_str()).collect();
    let what = if parse_scheme(scheme).1 != 1.0 { "effective affinity per 4 runs (grandparent slots weighted)" } else { "expected affinity per 4 runs" };
    let mut md = format!("## {}\n\nScheme `{}` | {} **{:.1}** = base {:.0} + G1 bonus {:.1} | per run {:.1}\n\n", names.join(" / "), scheme, what, val, lp.base, val - lp.base, val / 4.0);
    md += "**Shared G1s** are the affinity: a race (in either year) run by 2+ members. A row run by one member there is a second attempt at a race the others run in the other year. **Optional G1s** are run by one member only: no affinity effect, included because they cost nothing (streaks are already accounted for) and can pass down race sparks — skip them freely. Number = win % at that attempt (streak penalty included), `*` = goal, `·` = not running it. Turns not listed are rest. Add one extra Junior race win before Jr E-Nov if you have no fan bonus (Junior G1s need 1000 fans; a debut win gives 700).\n\n```text\n";
    md += &race_chart(d, &m, s);
    md += "```\n";
    let ps: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[m[i]], &s[i])).collect();
    md += &format!("\n| G1: P(win at least once) | {} |\n|---|{}\n", names.join(" | "), "---|".repeat(4));
    for r in 0..d.nr {
        if ps.iter().all(|p| p[r] == 0.0) { continue; }
        md += &format!("| {} | {} |\n", d.race_names[r], ps.iter().map(|p| if p[r] > 0.0 { format!("{:.2}", p[r]) } else { "—".into() }).collect::<Vec<_>>().join(" | "));
    }
    md += "\n| Pair | expected shared G1 wins |\n|---|---|\n";
    for i in 0..4 { for j in i + 1..4 { md += &format!("| {} – {} | {:.2} |\n", names[i], names[j], wval(&ps[i], &ps[j])); } }
    md + "\n"
}

#[derive(Serialize)]
struct OutMember { id: u32, name: String, schedule: Vec<(usize, u32)>, detail: Vec<serde_json::Value> }
#[derive(Serialize)]
struct Out { members: Vec<OutMember>, order: Vec<u32>, value: f64, base: f64, scheme: String, secs: f64, group_trials: u64, #[serde(skip)] md: String }

fn sched_to_out(u: &Uma, s: &Sched) -> Vec<(usize, u32)> {
    (0..NT).filter_map(|t| match s[t] { Choice::Opt(o) => Some((t + 1, u.opts[t][o as usize].prog)), _ => None }).collect()
}
/// Parse a schedule of (turn, program) pairs. Returns None for a turn outside 1..=72, two entries on the
/// same turn, or a program not offered to this uma at that turn.
fn sched_from(u: &Uma, v: &[(usize, u32)]) -> Option<Sched> {
    let mut s = [Choice::Rest; NT];
    let mut seen = [false; NT];
    for &(t, prog) in v {
        if t == 0 || t > NT || seen[t - 1] { return None; }
        seen[t - 1] = true;
        let o = u.opts[t - 1].iter().position(|op| op.prog == prog)?;
        s[t - 1] = Choice::Opt(o as u8);
    }
    Some(s)
}

#[derive(Deserialize)]
struct Job { members: Vec<u32>, #[serde(default)] init: Vec<Vec<Vec<(usize, u32)>>> }
#[derive(Deserialize)]
struct Jobs {
    scheme: String, iters: usize, seed: u64,
    #[serde(default = "dm")] margin: f64,
    #[serde(default = "dt")] threads: usize,
    #[serde(default)] jobs: Vec<Job>,
    #[serde(default)] mode: String,
    /// "best" mode: umas you own (empty = whole roster), umas to exclude, how many loops to return
    #[serde(default)] owned: Vec<u32>,
    #[serde(default)] exclude: Vec<u32>,
    /// "best" mode: umas every returned loop must contain (at most 4)
    #[serde(default)] require: Vec<u32>,
    /// grandparent spark base rate as a fraction of parents' (1 = plain total affinity; community estimate 0.5)
    #[serde(default = "dgw")] gp_weight: f64,
    #[serde(default = "dtop")] top: usize,
    /// optional: write a readable Markdown sheet of the results to this path
    #[serde(default)] markdown: String,
}
fn dtop() -> usize { 5 }
fn dgw() -> f64 { 1.0 }
fn dm() -> f64 { 60.0 }
fn dt() -> usize { 2 }


/// Exact pairwise ceilings PB(a,b) = min(BR_a(W = PM_b), BR_b(W = PM_a)) for all pairs in `pool`.
/// Valid because P_b <= PM_b elementwise and P_a >= 0, so <P_a,P_b> <= <P_a,PM_b> <= BR_a(PM_b).
fn pair_ceilings(d: &Data, pool: &[usize]) -> Vec<Vec<f64>> {
    ph!(6);
    let n = d.umas.len();
    let mut pb = vec![vec![0.0f64; n]; n];
    if let Some(t) = &d.pbt {
        for (x, &a) in pool.iter().enumerate() { for &b in &pool[x + 1..] { pb[a][b] = t[a * n + b]; pb[b][a] = t[b * n + a]; } }
        return pb;
    }
    let br = |a: usize, b: usize| best_response(d, &d.umas[a], &d.umas[b].pm, None, None).map(|r| r.1).unwrap_or(0.0);
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

/// Guaranteed ceiling for foursome f: no schedules can score above this.
/// scf: 2*sum pair + 4*sum trio + 10*sum PB.  rot: 2*sum pair + 3*sum trio + 3*(3*sum PB + best diagonal pair-sum).
fn ceiling(d: &Data, pb: &[Vec<f64>], f: [usize; 4], scheme: &str) -> f64 {
    let mut ps = 0.0; let mut ts = 0.0; let mut sb = 0.0;
    for a in 0..4 { for b in a + 1..4 { ps += d.pair[f[a]][f[b]] as f64; sb += pb[f[a]][f[b]];
        for c in b + 1..4 { ts += d.tri[f[a]][f[b]][f[c]] as f64; } } }
    let (scf, w) = parse_scheme(scheme);
    let (cadj, cdiag, kt) = scheme_weights(scf, w);
    if scf { 2.0 * ps + kt * ts + 3.0 * cadj * sb } else {
        // rot: 3*(cadj*(sum of the 4 adjacent PB) + cdiag*(the 2 diagonal PB)), maximized over the 3 orders
        let ds = [pb[f[0]][f[1]] + pb[f[2]][f[3]], pb[f[0]][f[2]] + pb[f[1]][f[3]], pb[f[0]][f[3]] + pb[f[1]][f[2]]];
        let diag = if cdiag >= cadj { ds.iter().cloned().fold(f64::MIN, f64::max) } else { ds.iter().cloned().fold(f64::MAX, f64::min) };
        2.0 * ps + kt * ts + 3.0 * (cadj * sb + (cdiag - cadj) * diag)
    }
}

/// Screen + best-first search. Every foursome gets a guaranteed ceiling UB >= any achievable score.
/// Foursomes are optimized in descending-UB order until the next ceiling is below the top-k score,
/// so every foursome NOT optimized is provably outside the top k (under the model).
struct BestRun { pool: Vec<usize>, cand: Vec<(f64, [usize; 4])>, done: Vec<(f64, [usize; 4], Res)>, next: usize, t_pb: f64, t_screen: f64, total: f64 }

fn run_best(d: &Data, jobs: &Jobs) {
    let b = best_search(d, jobs);
    let BestRun { pool, cand, done, next, t_pb, t_screen, total } = b;
    print_best(d, jobs, &pool, &cand, &done, next, t_pb, t_screen, total);
}

fn best_search(d: &Data, jobs: &Jobs) -> BestRun {
    assert!(jobs.top >= 1, "\"top\" must be at least 1");
    let t0 = Instant::now();
    let n = d.umas.len();
    let pool: Vec<usize> = (0..n).filter(|&i| {
        let id = d.umas[i].id;
        (jobs.owned.is_empty() || jobs.owned.contains(&id) || jobs.require.contains(&id)) && !jobs.exclude.contains(&id)
    }).collect();
    assert!(jobs.require.len() <= 4, "\"require\" can list at most 4 umas");
    let req: Vec<usize> = jobs.require.iter().map(|&id| {
        let i = d.umas.iter().position(|u| u.id == id).unwrap_or_else(|| panic!("required uma {id} is not in the data"));
        assert!(!jobs.exclude.contains(&id), "uma {id} is both required and excluded");
        i
    }).collect();
    // exact pairwise ceilings on the pool
    let pb = pair_ceilings(d, &pool);
    let t_pb = t0.elapsed().as_secs_f64();
    // ceilings for every foursome
    let mut cand: Vec<(f64, [usize; 4])> = vec![];
    let k = pool.len();
    for i in 0..k { for j in i + 1..k { for l in j + 1..k { for m in l + 1..k {
        let f = [pool[i], pool[j], pool[l], pool[m]];
        if !req.iter().all(|r| f.contains(r)) { continue; }
        cand.push((ceiling(d, &pb, f, &jobs.scheme), f));
    } } } }
    cand.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
    let t_screen = t0.elapsed().as_secs_f64();
    // best-first search in parallel batches
    let mut done: Vec<(f64, [usize; 4], Res)> = vec![];
    let mut next = 0usize;
    let kth = |done: &Vec<(f64, [usize; 4], Res)>| -> f64 {
        if done.len() < jobs.top { f64::NEG_INFINITY } else {
            let mut v: Vec<f64> = done.iter().map(|x| x.2.val).collect(); v.sort_by(|a, b| b.partial_cmp(a).unwrap()); v[jobs.top - 1] }
    };
    while next < cand.len() && cand[next].0 >= kth(&done) {
        let bar = kth(&done);
        let batch: Vec<(f64, [usize; 4])> = cand[next..].iter().take(jobs.threads.max(1)).cloned().filter(|c| c.0 >= bar).collect();
        if batch.is_empty() { break; }
        next += batch.len();
        #[cfg(not(target_arch = "wasm32"))]
        let res: Vec<Res> = std::thread::scope(|sc| {
            let hs: Vec<_> = batch.iter().map(|&(_, f)| sc.spawn(move || optimize(d, f, &jobs.scheme, jobs.iters, jobs.seed, jobs.margin, &[]))).collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        #[cfg(target_arch = "wasm32")]
        let res: Vec<Res> = batch.iter().map(|&(_, f)| optimize(d, f, &jobs.scheme, jobs.iters, jobs.seed, jobs.margin, &[])).collect();
        for ((ub, f), r) in batch.into_iter().zip(res) {
            #[cfg(not(target_arch = "wasm32"))] eprintln!("  ceiling {:>8.1}  score {:>8.2}  {:>5.1}s  {}", ub, r.val, r.secs, f.iter().map(|&x| d.umas[x].name.as_str()).collect::<Vec<_>>().join(" / "));
            done.push((ub, f, r));
        }
    }
    done.sort_by(|a, b| b.2.val.partial_cmp(&a.2.val).unwrap());
    let total = t0.elapsed().as_secs_f64();
    BestRun { pool, cand, done, next, t_pb, t_screen, total }
}

#[allow(clippy::too_many_arguments)]
fn print_best(d: &Data, jobs: &Jobs, pool: &[usize], cand: &[(f64, [usize; 4])], done: &[(f64, [usize; 4], Res)], next: usize, t_pb: f64, t_screen: f64, total: f64) {
    let next_ceiling = if next < cand.len() { cand[next].0 } else { f64::NEG_INFINITY };
    let top: Vec<serde_json::Value> = done.iter().take(jobs.top).map(|(ub, f, r)| {
        let lp = make_loop(d, *f, &jobs.scheme);
        serde_json::json!({
            "members": f.iter().map(|&x| d.umas[x].name.clone()).collect::<Vec<_>>(),
            "ids": f.iter().map(|&x| d.umas[x].id).collect::<Vec<_>>(),
            "order": r.order.iter().map(|&x| d.umas[x].name.clone()).collect::<Vec<_>>(),
            "score": r.val, "base": lp.base, "g1_bonus": r.val - lp.base, "ceiling": ub,
            "schedules": (0..4).map(|i| serde_json::json!({"id": d.umas[f[i]].id, "name": d.umas[f[i]].name, "races": sched_to_out(&d.umas[f[i]], &r.s[i]), "detail": sched_detail(d, &d.umas[f[i]], &r.s[i])})).collect::<Vec<_>>()
        })
    }).collect();
    let out = serde_json::json!({
        "scheme": jobs.scheme, "pool_size": pool.len(), "foursomes": cand.len(), "optimized": done.len(),
        "ruled_out_by_ceiling": cand.len() - done.len(),
        "next_unoptimized_ceiling": if next_ceiling.is_finite() { serde_json::json!(next_ceiling) } else { serde_json::Value::Null },
        "top": top,
        "timing_s": {"ceilings": t_pb, "screen": t_screen - t_pb, "search": total - t_screen, "total": total}
    });
    if !jobs.markdown.is_empty() {
        let mut md = format!("# Best loops (scheme `{}`)\n\nPool {} umas, {} foursomes; {} searched, the rest ruled out by guaranteed ceilings.\n\n", jobs.scheme, pool.len(), cand.len(), done.len());
        for (_, f, r) in done.iter().take(jobs.top) { md += &render_md(d, *f, &r.s, &jobs.scheme, r.val); }
        std::fs::write(&jobs.markdown, md).unwrap();
    }
    println!("{}", serde_json::to_string_pretty(&out).unwrap());
}

pub fn cli_main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 3 && args[1] == "pbtable" {
        // print the pairwise-ceiling table for a data.json (./build.sh pb merges it in as "pb")
        println!("{}", pb_table_json(&std::fs::read_to_string(&args[2]).unwrap()));
        return;
    }
    let job_text = std::fs::read_to_string(&args[2]).unwrap();
    let mut jobs: Jobs = serde_json::from_str(&job_text).unwrap();
    let t_load = Instant::now();
    let d = load(&args[1]);
    let load_ns = t_load.elapsed().as_nanos() as u64;
    if jobs.mode == "skills" {
        // skill-target search; needs "skills_file" (web_skills.json) plus the SkillJob fields
        let v: serde_json::Value = serde_json::from_str(&job_text).unwrap();
        let sk: skills::SkillsRaw = serde_json::from_str(&std::fs::read_to_string(v["skills_file"].as_str().expect("skills_file")).unwrap()).unwrap();
        let sj: skills::SkillJob = serde_json::from_value(v).unwrap();
        println!("{}", skills::run_skills(&d, &sk, &sj));
        return;
    }
    if jobs.mode == "deck" {
        // joint deck + loop search; needs "skills_file" and "cards_file" (web_cards.json)
        let mut v: serde_json::Value = serde_json::from_str(&job_text).unwrap();
        let sk: skills::SkillsRaw = serde_json::from_str(&std::fs::read_to_string(v["skills_file"].as_str().expect("skills_file")).unwrap()).unwrap();
        if let Some(path) = v["events_file"].as_str() {
            let db: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
            deck::apply_events_db(&mut v, &sk, &db);
        }
        let cr: deck::CardsRaw = serde_json::from_str(&std::fs::read_to_string(v["cards_file"].as_str().expect("cards_file")).unwrap()).unwrap();
        PH[0].fetch_add(load_ns + t_load.elapsed().as_nanos() as u64 - load_ns, Relaxed); PH[24].fetch_add(1, Relaxed);
        let t0 = Instant::now();
        let mut out = deck::run_deck(&d, &sk, &cr, &v);
        out["timing_s"] = serde_json::json!(t0.elapsed().as_secs_f64());
        let t_out = Instant::now();
        let txt = out.to_string();
        PH[14].fetch_add(t_out.elapsed().as_nanos() as u64, Relaxed);
        println!("{}", txt);
        ph_report();
        return;
    }
    if jobs.scheme != "scf" && jobs.scheme != "rot" { eprintln!("error: \"scheme\" must be \"scf\" or \"rot\""); std::process::exit(2); }
    if !(0.0..=1.0).contains(&jobs.gp_weight) { eprintln!("error: \"gp_weight\" must be between 0 and 1"); std::process::exit(2); }
    if jobs.gp_weight != 1.0 { jobs.scheme = format!("{}@{}", jobs.scheme, jobs.gp_weight); }
    let ix = |id: u32| d.umas.iter().position(|u| u.id == id).unwrap_or_else(|| panic!("unknown uma {id}"));
    if jobs.mode == "legacy" {
        // per-legacy affinity for every lineage (umaishow's split, expected G1 overlaps, no rounding)
        let mut outs = vec![];
        for j in &jobs.jobs {
            let m = [ix(j.members[0]), ix(j.members[1]), ix(j.members[2]), ix(j.members[3])];
            let parsed: Option<Vec<Sched>> = (0..4).map(|i| sched_from(&d.umas[m[i]], &j.init[0][i])).collect();
            let Some(s) = parsed else { eprintln!("error: invalid schedule"); std::process::exit(2); };
            let lin = legacy_affinities(&d, m, &s, parse_scheme(&jobs.scheme).0);
            outs.push(serde_json::json!({"members": j.members, "lineages": lin.iter().map(|l| serde_json::json!({
                "trainee": d.umas[l.0].id, "slots": l.1.iter().map(|(u, gp, a)| serde_json::json!({"uma": d.umas[*u].id, "grandparent": gp, "affinity": a})).collect::<Vec<_>>()
            })).collect::<Vec<_>>()}));
        }
        println!("{}", serde_json::to_string_pretty(&outs).unwrap());
        return;
    }
    if jobs.mode == "score" {
        // score supplied schedules exactly (validation)
        let mut outs = vec![];
        for j in &jobs.jobs {
            let m = [ix(j.members[0]), ix(j.members[1]), ix(j.members[2]), ix(j.members[3])];
            let lp = make_loop(&d, m, &jobs.scheme);
            let st = &j.init[0];
            let parsed: Option<Vec<Sched>> = (0..4).map(|i| sched_from(&d.umas[m[i]], &st[i])).collect();
            let Some(s) = parsed else {
                outs.push(serde_json::json!({"members": j.members, "value": null, "error": "invalid schedule: turn outside 1..72, duplicate turn, or program not offered at that turn"}));
                continue;
            };
            let feas: Vec<bool> = (0..4).map(|i| feasible(&d.umas[m[i]], &s[i])).collect();
            outs.push(serde_json::json!({"members": j.members, "value": score(&d, &lp, &[s[0], s[1], s[2], s[3]]), "feasible": feas}));
        }
        println!("{}", serde_json::to_string(&outs).unwrap());
        return;
    }
    if jobs.mode == "best" {
        if jobs.top == 0 { eprintln!("error: \"top\" must be at least 1"); std::process::exit(2); }
        if jobs.require.len() > 4 { eprintln!("error: \"require\" can list at most 4 umas"); std::process::exit(2); }
        for &id in &jobs.require {
            if !d.umas.iter().any(|u| u.id == id) { eprintln!("error: required uma {id} is not in the data"); std::process::exit(2); }
            if jobs.exclude.contains(&id) { eprintln!("error: uma {id} is both required and excluded"); std::process::exit(2); }
        }
        run_best(&d, &jobs); return;
    }
    if jobs.mode == "prof" {
        let j = &jobs.jobs[0];
        let m = [ix(j.members[0]), ix(j.members[1]), ix(j.members[2]), ix(j.members[3])];
        let t0 = Instant::now();
        let r = optimize(&d, m, &jobs.scheme, jobs.iters, jobs.seed, jobs.margin, &[]);
        let tot = t0.elapsed().as_secs_f64();
        let names = ["dp_exact", "score", "probs"];
        eprintln!("total {:.2}s value {:.2} group trials {}", tot, r.val, r.evals);
        for i in 0..3 { eprintln!("  {:9} calls {:>8}  time {:>6.2}s  ({:.1}%)  avg {:.1} us", names[i], PROF[i+4].load(Relaxed), PROF[i].load(Relaxed) as f64/1e9, PROF[i].load(Relaxed) as f64/1e9/tot*100.0, PROF[i].load(Relaxed) as f64/1e3/PROF[i+4].load(Relaxed).max(1) as f64); }
        return;
    }
    if jobs.mode == "pb" {
        // pairwise ceilings: PB(a,b) = min(BR_a(W = PM_b), BR_b(W = PM_a)), exact best responses
        let n = d.umas.len();
        let t0 = Instant::now();
        let mut pb = vec![vec![0.0f64; n]; n];
        let br = |a: usize, b: usize| best_response(&d, &d.umas[a], &d.umas[b].pm, None, None).map(|x| x.1).unwrap_or(0.0);
        for a in 0..n { for b in a + 1..n { let v = br(a, b).min(br(b, a)); pb[a][b] = v; pb[b][a] = v; } }
        eprintln!("pb: {} pairs in {:.2}s", n * (n - 1) / 2, t0.elapsed().as_secs_f64());
        println!("{}", serde_json::to_string(&serde_json::json!({"ids": d.umas.iter().map(|u| u.id).collect::<Vec<_>>(), "pb": pb})).unwrap());
        return;
    }
    if jobs.mode == "br" {
        // best-response test: init[0][0] holds weights encoded as (race_idx, weight*1e6) pairs; members[0] is the uma
        let mut outs = vec![];
        for j in &jobs.jobs {
            let u = &d.umas[ix(j.members[0])];
            let mut w = vec![0.0; d.nr];
            for &(r, x) in &j.init[0][0] { w[r] = x as f64 / 1e6; }
            let force = match j.init[0].get(1).and_then(|f| f.first()) {
                None => None,
                Some(&(t, prog)) => {
                    if t == 0 || t > NT { outs.push(serde_json::json!({"id": u.id, "value": null, "error": "force turn out of range"})); continue; }
                    if prog == 0 { Some((t - 1, Choice::Rest)) } else {
                        match u.opts[t - 1].iter().position(|op| op.prog == prog) {
                            Some(o) => Some((t - 1, Choice::Opt(o as u8))),
                            None => { outs.push(serde_json::json!({"id": u.id, "value": null, "error": "forced program not offered at that turn"})); continue; }
                        }
                    }
                }
            };
            match best_response(&d, u, &w, None, force) {
                Some((s, v)) => outs.push(serde_json::json!({"id": u.id, "value": v, "schedule": sched_to_out(u, &s)})),
                None => outs.push(serde_json::json!({"id": u.id, "value": null})),
            }
        }
        println!("{}", serde_json::to_string(&outs).unwrap());
        return;
    }
    let nth = jobs.threads.max(1);
    let results: Vec<Out> = std::thread::scope(|sc| {
        let chunks: Vec<Vec<(usize, &Job)>> = (0..nth).map(|k| jobs.jobs.iter().enumerate().filter(|(i, _)| i % nth == k).collect()).collect();
        let hs: Vec<_> = chunks.into_iter().map(|ch| {
            let d = &d; let jobs = &jobs;
            sc.spawn(move || {
                ch.into_iter().map(|(_, j)| {
                    let m = [ix(j.members[0]), ix(j.members[1]), ix(j.members[2]), ix(j.members[3])];
                    let init: Vec<[Sched; 4]> = j.init.iter().filter_map(|st| {
                        let v: Option<Vec<Sched>> = (0..4).map(|i| sched_from(&d.umas[m[i]], &st[i])).collect();
                        v.map(|v| [v[0], v[1], v[2], v[3]])
                    }).collect();
                    let r = optimize(d, m, &jobs.scheme, jobs.iters, jobs.seed, jobs.margin, &init);
                    let lp = make_loop(d, m, &jobs.scheme);
                    eprintln!("{:>9.2}  {:>6.1}s  {}", r.val, r.secs, (0..4).map(|i| d.umas[m[i]].name.clone()).collect::<Vec<_>>().join(" / "));
                    Out { members: (0..4).map(|i| OutMember { id: d.umas[m[i]].id, name: d.umas[m[i]].name.clone(), schedule: sched_to_out(&d.umas[m[i]], &r.s[i]), detail: sched_detail(d, &d.umas[m[i]], &r.s[i]) }).collect(),
                          order: r.order.iter().map(|&x| d.umas[x].id).collect(), value: r.val, base: lp.base, scheme: jobs.scheme.clone(), secs: r.secs, group_trials: r.evals, md: render_md(d, m, &r.s, &jobs.scheme, r.val) }
                }).collect::<Vec<_>>()
            })
        }).collect();
        hs.into_iter().flat_map(|h| h.join().unwrap()).collect()
    });
    if !jobs.markdown.is_empty() {
        std::fs::write(&jobs.markdown, results.iter().map(|r| r.md.clone()).collect::<String>()).unwrap();
    }
    println!("{}", serde_json::to_string_pretty(&results).unwrap());
}

// ---------------------------------------------------------------------------------------------
// Correctness tests. Reference values in tests/fixtures/ were produced independently of this code:
//   br_milp.json         exact MILP (scipy/HiGHS) best responses, incl. forced turns and special-goal umas
//   pb_milp.json         pair ceilings computed with that MILP
//   known_schedules.json schedules scored by the Python reference implementation
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
    fn ix(d: &Data, id: u64) -> usize { d.umas.iter().position(|u| u.id as u64 == id).unwrap() }
    fn sched_of(u: &Uma, v: &serde_json::Value) -> Sched {
        let pairs: Vec<(usize, u32)> = v.as_array().unwrap().iter()
            .map(|e| (e[0].as_u64().unwrap() as usize, e[1].as_u64().unwrap() as u32)).collect();
        sched_from(u, &pairs).unwrap()
    }
    struct Lcg(u64);
    impl Lcg { fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
               fn f(&mut self) -> f64 { self.next() as f64 / (1u64 << 31) as f64 } }

    /// Win chance table follows the stated formula exactly.
    #[test]
    fn win_chance_formula() {
        let d = data();
        for bp in [-10, 50, 60, 100, 110] {
            for k in 1..=6 {
                let pen = [0, 0, 10, 25, 35, 50][k - 1];
                let want = ((bp - pen).clamp(0, 100)) as f64 / 100.0;
                assert_eq!(pk(d, bp, k), want, "bp {bp} k {k}");
            }
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
        let det = sched_detail(d, u, &s);
        let ks: Vec<u64> = det.iter().map(|e| e["streak"].as_u64().unwrap()).collect();
        assert_eq!(ks, vec![1, 2, 3]);
    }

    /// Scores of known schedules equal the independent Python reference (both schemes).
    #[test]
    fn score_matches_python_reference() {
        let d = data();
        let k = fixture("known_schedules.json");
        let mut n = 0;
        for c in k.as_array().unwrap() {
            let ids: Vec<usize> = c["members"].as_array().unwrap().iter().map(|x| ix(d, x.as_u64().unwrap())).collect();
            let m = [ids[0], ids[1], ids[2], ids[3]];
            let s: Vec<Sched> = (0..4).map(|i| sched_of(&d.umas[m[i]], &c["schedules"][i])).collect();
            for i in 0..4 { assert!(feasible(&d.umas[m[i]], &s[i]), "infeasible reference schedule"); }
            let lp = make_loop(d, m, c["scheme"].as_str().unwrap());
            let got = score(d, &lp, &[s[0], s[1], s[2], s[3]]);
            let want = c["python_value"].as_f64().unwrap();
            assert!((got - want).abs() < 1e-9, "score {got} vs reference {want}");
            n += 1;
        }
        assert!(n >= 20);
    }

    /// The exact DP equals the independent MILP optimum on 200 cases (incl. forced turns, turn-49 goals,
    /// multi-turn goal alternatives, repeat-race-heavy weights); returned schedules are feasible and honor the force.
    #[test]
    fn best_response_matches_milp() {
        let d = data();
        let f = fixture("br_milp.json");
        for c in f.as_array().unwrap() {
            let u = &d.umas[ix(d, c["uma"].as_u64().unwrap())];
            let mut w = vec![0.0; d.nr];
            for e in c["weights"].as_array().unwrap() { w[e[0].as_u64().unwrap() as usize] = e[1].as_f64().unwrap(); }
            let force = c["force"].as_array().map(|fa| {
                let t = fa[0].as_u64().unwrap() as usize; let prog = fa[1].as_u64().unwrap() as u32;
                (t - 1, Choice::Opt(u.opts[t - 1].iter().position(|op| op.prog == prog).unwrap() as u8))
            });
            let (s, v) = best_response(d, u, &w, None, force).expect("infeasible");
            let want = c["milp_value"].as_f64().unwrap();
            assert!((v - want).abs() < 1e-6, "{} {}: dp {v} vs milp {want}", u.name, c["kind"]);
            assert!(feasible(u, &s));
            if let Some((t, ch)) = force { assert_eq!(s[t], ch); }
        }
    }

    /// Any feasible schedule scores at most the best response (dominance over 2,000 random schedules).
    #[test]
    fn best_response_dominates_random_schedules() {
        let d = data();
        let mut rng = Lcg(42);
        for _ in 0..200 {
            let u = &d.umas[(rng.next() as usize) % d.umas.len()];
            let w: Vec<f64> = (0..d.nr).map(|_| rng.f() * 50.0).collect();
            let (_, best) = best_response(d, u, &w, None, None).unwrap();
            for _ in 0..10 {
                // random feasible schedule: exact DP on random weights
                let w2: Vec<f64> = (0..d.nr).map(|_| rng.f() * 50.0).collect();
                let (s2, _) = best_response(d, u, &w2, None, None).unwrap();
                let v2 = wval(&w, &probs(d, u, &s2));
                assert!(v2 <= best + 1e-9, "{}: random schedule {v2} beats best response {best}", u.name);
            }
        }
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
            let (a, b) = (ix(d, e[0].as_u64().unwrap()), ix(d, e[1].as_u64().unwrap()));
            assert!((pb[a][b] - e[2].as_f64().unwrap()).abs() < 1e-9);
        }
    }

    /// Ceilings bound every achieved score: optimized schedules and random schedules, both schemes.
    #[test]
    fn ceilings_bound_achieved_scores() {
        let d = data();
        let all: Vec<usize> = (0..d.umas.len()).collect();
        let pb = pair_ceilings(d, &all);
        let mut rng = Lcg(7);
        for scheme in ["scf", "rot"] {
            for _ in 0..12 {
                let mut f = [0usize; 4];
                loop { for x in f.iter_mut() { *x = (rng.next() as usize) % d.umas.len(); }
                       let mut g = f; g.sort(); if g.windows(2).all(|w| w[0] != w[1]) { break; } }
                let ub = ceiling(d, &pb, f, scheme);
                let r = optimize(d, f, scheme, 0, 1, 60.0, &[]);
                assert!(r.val <= ub + 1e-9, "optimized {} > ceiling {ub}", r.val);
                for _ in 0..5 {
                    let s: Vec<Sched> = (0..4).map(|i| {
                        let w: Vec<f64> = (0..d.nr).map(|_| rng.f()).collect();
                        best_response(d, &d.umas[f[i]], &w, None, None).unwrap().0 }).collect();
                    let orders: Vec<[usize; 4]> = vec![[0, 1, 2, 3], [0, 2, 1, 3], [0, 1, 3, 2]];
                    for o in orders {
                        let lp = make_loop(d, [f[o[0]], f[o[1]], f[o[2]], f[o[3]]], scheme);
                        let v = score(d, &lp, &[s[o[0]], s[o[1]], s[o[2]], s[o[3]]]);
                        assert!(v <= ub + 1e-9, "random schedule {v} > ceiling {ub}");
                    }
                }
            }
        }
    }

    /// The search reproduces known optima (regression guard for search-quality changes).
    #[test]
    fn known_optima_reproduced() {
        let d = data();
        let cases: [([u64; 4], &str, f64); 3] = [
            ([1009, 1014, 1019, 1029], "scf", 2058.924),
            ([1015, 1017, 1023, 1030], "scf", 1961.015),
            ([1008, 1009, 1014, 1019], "rot", 1833.028),
        ];
        for (ids, scheme, want) in cases {
            let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
            let r = optimize(d, m, scheme, 10, 3, 60.0, &[]);
            assert!(r.val >= want - 1e-3, "{ids:?} {scheme}: got {} expected >= {want}", r.val);
            for i in 0..4 { assert!(feasible(&d.umas[m[i]], &r.s[i])); }
        }
    }

    /// Best-first search stops correctly: every foursome not optimized has a ceiling below the k-th best score.
    #[test]
    fn best_search_stopping_rule() {
        let d = data();
        let jobs: Jobs = serde_json::from_str(r#"{"mode":"best","scheme":"scf","iters":5,"seed":3,"threads":2,"top":3,
            "owned":[1008,1009,1014,1015,1017,1019,1023,1030,1069]}"#).unwrap();
        let b = best_search(d, &jobs);
        let mut vals: Vec<f64> = b.done.iter().map(|x| x.2.val).collect();
        vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let kth = vals[jobs.top - 1];
        for (ub, _) in &b.cand[b.next..] { assert!(*ub < kth, "unsearched ceiling {ub} >= k-th best {kth}"); }
        for (ub, _, r) in &b.done { assert!(r.val <= ub + 1e-9); }
        assert_eq!(b.cand.len(), 126);
    }
}

// ---------------------------------------------------------------------------------------------
// Additional tests added after the independent review (bugs A-C and coverage gaps).
// br_rest_milp.json: rest-forced best responses from the MILP, incl. turns 24/25/48/49/50 for the
// turn-49-goal umas (Meisho Doto 1058, Mejiro Palmer 1064) and Haru Urara 1052.
// ---------------------------------------------------------------------------------------------
#[cfg(test)]
mod review_tests {
    use super::*;
    use std::sync::OnceLock;
    fn data() -> &'static Data {
        static D: OnceLock<Data> = OnceLock::new();
        D.get_or_init(|| load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json")))
    }
    fn ix(d: &Data, id: u32) -> usize { d.umas.iter().position(|u| u.id == id).unwrap() }
    fn opt_at(u: &Uma, t: usize, prog: u32) -> Choice { Choice::Opt(u.opts[t - 1].iter().position(|o| o.prog == prog).unwrap() as u8) }
    struct Lcg(u64);
    impl Lcg { fn next(&mut self) -> u64 { self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407); self.0 >> 33 }
               fn f(&mut self) -> f64 { self.next() as f64 / (1u64 << 31) as f64 } }

    /// `require`: every candidate contains the required umas, the count is exact, and the stopping rule still holds.
    #[test]
    fn best_search_require() {
        let d = data();
        let jobs: Jobs = serde_json::from_str(r#"{"mode":"best","scheme":"scf","iters":5,"seed":3,"threads":2,"top":2,
            "owned":[1008,1009,1014,1015,1017,1023,1030],"require":[1029,1019,1034]}"#).unwrap();
        let b = best_search(d, &jobs);
        let req: Vec<usize> = [1029u32, 1019, 1034].iter().map(|&id| ix(d, id)).collect();
        assert_eq!(b.cand.len(), 7); // required umas are added to the pool; 7 choices for the 4th slot
        for (_, f) in &b.cand { for r in &req { assert!(f.contains(r)); } }
        let mut vals: Vec<f64> = b.done.iter().map(|x| x.2.val).collect();
        vals.sort_by(|a, b| b.partial_cmp(a).unwrap());
        for (ub, _) in &b.cand[b.next..] { assert!(*ub < vals[jobs.top - 1]); }
    }

    #[test]
    #[should_panic(expected = "both required and excluded")]
    fn best_search_require_excluded_conflict() {
        let jobs: Jobs = serde_json::from_str(r#"{"mode":"best","scheme":"scf","iters":1,"seed":1,"require":[1029],"exclude":[1029]}"#).unwrap();
        best_search(data(), &jobs);
    }

    /// Bug A: a force that conflicts with a single-turn goal must be infeasible, not silently drop the goal.
    #[test]
    fn force_conflicting_with_goal_is_infeasible() {
        let d = data();
        let w = vec![1.0; d.nr];
        let urara = &d.umas[ix(d, 1052)];
        assert!(urara.forced[71].is_some(), "Urara has a goal at turn 72");
        assert!(best_response(d, urara, &w, None, Some((71, Choice::Rest))).is_none());
        let chiyono = &d.umas[ix(d, 1069)];
        // turn 23 goal for Chiyono O is Asahi Hai FS; forcing Hanshin JF (program 623) must fail
        assert!(best_response(d, chiyono, &w, None, Some((22, opt_at(chiyono, 23, 623)))).is_none());
    }

    /// Rest-forced best responses equal the MILP (this case was previously untested).
    #[test]
    fn rest_forced_best_response_matches_milp() {
        let d = data();
        let p = format!("{}/tests/fixtures/br_rest_milp.json", env!("CARGO_MANIFEST_DIR"));
        let f: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap();
        let mut n = 0;
        for c in f.as_array().unwrap() {
            let u = &d.umas[ix(d, c["uma"].as_u64().unwrap() as u32)];
            let mut w = vec![0.0; d.nr];
            for e in c["weights"].as_array().unwrap() { w[e[0].as_u64().unwrap() as usize] = e[1].as_f64().unwrap(); }
            let t = c["force_rest"].as_u64().unwrap() as usize - 1;
            let (s, v) = best_response(d, u, &w, None, Some((t, Choice::Rest))).expect("infeasible");
            let want = c["milp_value"].as_f64().unwrap();
            assert!((v - want).abs() < 1e-6, "{} rest@{}: dp {v} vs milp {want}", u.name, t + 1);
            assert_eq!(s[t], Choice::Rest);
            n += 1;
        }
        assert!(n >= 60);
    }

    /// feasible() rejects bad schedules (it is not vacuously true).
    #[test]
    fn feasible_rejects_bad_schedules() {
        let d = data();
        let u = &d.umas[ix(d, 1009)]; // Daiwa
        let w = vec![1.0; d.nr];
        let (s, _) = best_response(d, u, &w, None, None).unwrap();
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
        let (s3, _) = best_response(d, fk, &w, None, None).unwrap();
        let mut bad3 = s3; bad3[32] = Choice::Rest; bad3[33] = Choice::Rest;
        assert!(!feasible(fk, &bad3), "missing multi-turn goal alternative accepted");
    }

    /// Bug C: schedule parsing rejects duplicate turns, out-of-range turns and programs not offered.
    #[test]
    fn sched_from_rejects_bad_input() {
        let d = data();
        let u = &d.umas[ix(d, 1009)];
        assert!(sched_from(u, &[(23, 623), (23, 623)]).is_none(), "duplicate turn");
        assert!(sched_from(u, &[(0, 623)]).is_none());
        assert!(sched_from(u, &[(73, 623)]).is_none());
        assert!(sched_from(u, &[(23, 999_999)]).is_none(), "program not offered");
        assert!(sched_from(u, &[(23, 623)]).is_some());
    }

    /// Bug B: top = 0 is rejected with a clear error instead of an index underflow.
    #[test]
    #[should_panic(expected = "must be at least 1")]
    fn best_search_rejects_top_zero() {
        let d = data();
        let jobs: Jobs = serde_json::from_str(r#"{"mode":"best","scheme":"scf","iters":1,"seed":1,"top":0,"owned":[1008,1009,1014,1015,1017]}"#).unwrap();
        best_search(d, &jobs);
    }

    /// top larger than the number of foursomes: every foursome gets optimized, nothing is ruled out.
    #[test]
    fn best_search_top_exceeds_pool() {
        let d = data();
        let jobs: Jobs = serde_json::from_str(r#"{"mode":"best","scheme":"scf","iters":1,"seed":1,"top":50,"owned":[1008,1009,1014,1015,1019]}"#).unwrap();
        let b = best_search(d, &jobs);
        assert_eq!(b.cand.len(), 5);
        assert_eq!(b.done.len(), 5);
        assert_eq!(b.next, 5);
    }

    /// Ceilings bound genuinely random feasible schedules (random goal alternatives + random extra races), all schemes/orders.
    #[test]
    fn ceilings_bound_truly_random_schedules() {
        let d = data();
        let all: Vec<usize> = (0..d.umas.len()).collect();
        let pb = pair_ceilings(d, &all);
        let mut rng = Lcg(99);
        let random_sched = |u: &Uma, rng: &mut Lcg| -> Sched {
            loop {
                let mut s = [Choice::Rest; NT];
                for t in 0..NT {
                    let n = u.opts[t].len();
                    if let Some(f) = &u.forced[t] { s[t] = Choice::Opt(f[(rng.next() as usize) % f.len()]); }
                    else if n > 0 && rng.f() < 0.6 { s[t] = Choice::Opt(((rng.next() as usize) % n) as u8); }
                }
                if feasible(u, &s) { return s; } // retry if a multi-turn goal alternative was missed
            }
        };
        for scheme in ["scf", "rot", "scf@0.5", "rot@0.5", "rot@0.3", "scf@0"] {
            for _ in 0..150 {
                let mut f = [0usize; 4];
                loop { for x in f.iter_mut() { *x = (rng.next() as usize) % d.umas.len(); }
                       let mut g = f; g.sort(); if g.windows(2).all(|w| w[0] != w[1]) { break; } }
                let ub = ceiling(d, &pb, f, scheme);
                let s: Vec<Sched> = (0..4).map(|i| random_sched(&d.umas[f[i]], &mut rng)).collect();
                for o in [[0, 1, 2, 3], [0, 2, 1, 3], [0, 1, 3, 2]] {
                    let lp = make_loop(d, [f[o[0]], f[o[1]], f[o[2]], f[o[3]]], scheme);
                    let v = score(d, &lp, &[s[o[0]], s[o[1]], s[o[2]], s[o[3]]]);
                    assert!(v <= ub + 1e-9, "random schedule {v} > ceiling {ub}");
                }
            }
        }
    }

    /// Grandparent weighting: score() equals a direct per-legacy sum (umaishow's split, no rounding) over every lineage,
    /// with grandparent slots scaled by w. Checks the closed-form weights for both schemes and several w.
    #[test]
    fn gp_weight_matches_per_legacy_sum() {
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
                best_response(d, u, &w, None, None).unwrap().0
            }).collect();
            let p: Vec<Vec<f64>> = (0..4).map(|i| probs(d, &d.umas[f[i]], &s[i])).collect();
            let o = |a: usize, b: usize| wval(&p[a], &p[b]);
            let um = |i: usize| f[i];
            for w in [1.0, 0.5, 0.3, 0.0] {
                // lineage affinity: parents (T,P1,P2), P1's grandparents g1, P2's grandparents g2 (indices into f)
                let lin = |t: usize, p1: usize, p2: usize, g1: [usize; 2], g2: [usize; 2]| -> f64 {
                    let par = pr(um(t), um(p1)) + pr(um(t), um(p2)) + pr(um(p1), um(p2)) + 3.0 * o(p1, p2);
                    let mut gp = 0.0;
                    for &g in &g1 { gp += tr(um(t), um(p1), um(g)) + 3.0 * o(p1, g); }
                    for &g in &g2 { gp += tr(um(t), um(p2), um(g)) + 3.0 * o(p2, g); }
                    par + w * gp
                };
                // scf: 12 lineages; P1 bred from (P2, X), P2 bred from (P1, X); normalized to 4 runs
                let mut scf = 0.0;
                for t in 0..4 { for x in 0..4 { if x == t { continue; }
                    let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
                    scf += lin(t, ps[0], ps[1], [ps[1], x], [ps[0], x]); } }
                scf /= 3.0;
                let lp = make_loop(d, f, &format!("scf@{w}"));
                let v = score(d, &lp, &[s[0], s[1], s[2], s[3]]);
                assert!((v - scf).abs() < 1e-9, "scf w={w}: score {v} vs per-legacy {scf}");
                // rot in order 0,1,2,3: trainee i, parents i-1, i-2; P1's parents i-2, i-3; P2's parents i-3, i-4 (= trainee)
                let mut rot = 0.0;
                for i in 0..4 { let a = |k: usize| (i + 4 - k) % 4; rot += lin(i, a(1), a(2), [a(2), a(3)], [a(3), a(0)]); }
                let lp = make_loop(d, f, &format!("rot@{w}"));
                let v = score(d, &lp, &[s[0], s[1], s[2], s[3]]);
                assert!((v - rot).abs() < 1e-9, "rot w={w}: score {v} vs per-legacy {rot}");
            }
        }
    }

    /// legacy mode: per-slot affinities sum to the scored objective (parents + w * grandparents) for both schemes.
    #[test]
    fn legacy_affinities_sum_to_score() {
        let d = data();
        let mut rng = Lcg(77);
        for _ in 0..30 {
            let mut f = [0usize; 4];
            loop { for x in f.iter_mut() { *x = (rng.next() as usize) % d.umas.len(); }
                   let mut g = f; g.sort(); if g.windows(2).all(|w| w[0] != w[1]) { break; } }
            let s: Vec<Sched> = (0..4).map(|i| { let w: Vec<f64> = (0..d.nr).map(|_| rng.f() * 3.0).collect();
                best_response(d, &d.umas[f[i]], &w, None, None).unwrap().0 }).collect();
            for (scf, norm) in [(true, 3.0), (false, 1.0)] {
                for w in [1.0, 0.5] {
                    let lin = legacy_affinities(d, f, &s, scf);
                    let tot: f64 = lin.iter().flat_map(|l| l.1.iter()).map(|(_, gp, a)| if *gp { w * a } else { *a }).sum::<f64>() / norm;
                    let lp = make_loop(d, f, &format!("{}@{w}", if scf { "scf" } else { "rot" }));
                    let v = score(d, &lp, &[s[0], s[1], s[2], s[3]]);
                    assert!((v - tot).abs() < 1e-9, "scf={scf} w={w}: {v} vs {tot}");
                }
            }
        }
    }

    /// Optional G1s (run by one member only) cost no affinity: stripping them leaves the score unchanged (within the
    /// fill tolerance), and the fill pass never lowers the optimized score.
    #[test]
    fn optional_g1s_cost_nothing() {
        let d = data();
        let cases: [[u32; 4]; 4] = [[1007, 1019, 1021, 1049], [1009, 1014, 1019, 1029], [1015, 1017, 1023, 1030], [1006, 1019, 1029, 1034]];
        for ids in cases {
            let m = [ix(d, ids[0]), ix(d, ids[1]), ix(d, ids[2]), ix(d, ids[3])];
            for scheme in ["scf", "scf@0.5", "rot"] {
                let r = optimize(d, m, scheme, 3, 5, 60.0, &[]);
                let lp = make_loop(d, m, scheme);
                // order may differ for rot; score in r.order
                let ord = r.order; let lp_o = make_loop(d, ord, scheme);
                let so: [Sched; 4] = std::array::from_fn(|i| r.s[m.iter().position(|x| *x == ord[i]).unwrap()]);
                let v = score(d, &lp_o, &so);
                assert!((v - r.val).abs() < 1e-6, "{ids:?} {scheme}: {v} vs {}", r.val);
                let mut runners = vec![0u8; d.nr];
                for i in 0..4 { let mut seen = vec![false; d.nr];
                    for t in 0..NT { if let Choice::Opt(o) = so[i][t] { let rr = d.umas[ord[i]].opts[t][o as usize].race; if rr >= 0 && !seen[rr as usize] { seen[rr as usize] = true; runners[rr as usize] += 1; } } } }
                let mut stripped = so;
                for i in 0..4 { for t in 0..NT { if let Choice::Opt(o) = stripped[i][t] { let op = &d.umas[ord[i]].opts[t][o as usize];
                    if op.race >= 0 && runners[op.race as usize] == 1 && !op.goal { stripped[i][t] = Choice::Rest; } } } }
                for i in 0..4 { assert!(feasible(&d.umas[ord[i]], &stripped[i])); }
                let vs = score(d, &lp_o, &stripped);
                assert!((vs - v).abs() < 1e-5, "{ids:?} {scheme}: stripped {vs} vs {v}");
                let _ = lp;
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
            let r = optimize(d, m, "scf@0.5", 3, 5, 60.0, &[]);
            let lp = make_loop(d, m, "scf@0.5");
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

#[allow(dead_code)]
mod skills;
mod deck;
#[cfg(target_arch = "wasm32")]
mod web;
