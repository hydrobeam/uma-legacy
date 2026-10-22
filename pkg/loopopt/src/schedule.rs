//! Exact single-uma schedule search and its scratch buffers and scoped best-response memo.
//! Loop-wide optimization belongs to `optimize`; race data and win probabilities belong to `data`.
use crate::FxMap;
use crate::data::{Choice, Data, Ent, NT, Sched, Uma, feasible, probs, wval};

const NS: usize = 7; // streak state 0 (rested) ..= 6 (6+)
const HALF: usize = 48; // turns 1..=48 = Junior+Classic, 49..=72 = Senior

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
struct Scratch {
    // Phase 2: paired Classic/Senior turns. Values include a terminal layer; choices do not.
    v2: Vec<f64>,
    a2: Vec<[u8; 2]>,
    // Phase 1: Junior turns, with phase 2's entry values as the terminal layer.
    v1: Vec<f64>,
    a1: Vec<u8>,
}
thread_local! { static SCRATCH: std::cell::RefCell<Scratch> = std::cell::RefCell::new(Scratch::default()); }

/// Allowed choice codes at turn t. A force replaces the list at its turn, but only with a choice the goals allow there.
#[inline]
fn choice_list(u: &Uma, t: usize, force: Option<(usize, Choice)>) -> ([u8; 6], usize) {
    if let Some((ft, fc)) = force
        && ft == t
    {
        // a force can't override a single-turn goal: forcing rest, or an option outside the goal set, is infeasible
        let code = match fc {
            Choice::Rest => u.forced[t].is_none().then_some(0u8),
            Choice::Opt(o) => {
                ((o as usize) < u.opts[t].len() && u.forced[t].as_ref().is_none_or(|f| f.contains(&o))).then(|| o + 1)
            }
        };
        return match code {
            Some(c) => ([c, 0, 0, 0, 0, 0], 1),
            None => ([0; 6], 0),
        };
    }
    u.chl[t]
}

#[inline]
const fn code_choice(c: u8) -> Choice {
    if c == 0 { Choice::Rest } else { Choice::Opt(c - 1) }
}

fn dp_exact_in(u: &Uma, wr: &[f64], force: Option<(usize, Choice)>, scr: &mut Scratch) -> Option<(Sched, f64)> {
    let nm = (u.full + 1) as usize;
    let ns0 = if u.opts[HALF].is_empty() { 1 } else { NS };
    let neg = f64::NEG_INFINITY;
    let kb = ns0 * nm; // block of (s0, m) states
    let js = NS * NS * kb; // stride of one pair index j
    let full = u.full as usize;
    // streak position after choosing entry e with incoming streak s (also the next streak)
    #[inline(always)]
    const fn kof(e: &Ent, s: usize) -> usize {
        if e.rest {
            0
        } else if s >= 5 {
            6
        } else {
            s + 1
        }
    }
    #[inline(always)]
    const fn pof(e: &Ent, k: usize) -> f64 {
        if e.race >= 0 { e.p[k] } else { 0.0 }
    }
    // ---- phase 2: pairs j = 0..24 (classic turn 24+j with senior turn 48+j)
    let v2 = &mut scr.v2;
    let a2 = &mut scr.a2;
    v2.clear();
    v2.resize(25 * js, neg);
    a2.clear();
    a2.resize(24 * js, [0, 0]);
    for sc in 0..NS {
        for ss in 0..NS {
            for s0 in 0..ns0 {
                if ns0 > 1 && sc != s0 {
                    continue;
                }
                v2[24 * js + (sc * NS + ss) * kb + s0 * nm + full] = 0.0;
            }
        }
    }
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
            let mut kca = [0usize; NS];
            let mut ra = [0.0f64; NS];
            let mut pca = [0.0f64; NS];
            for sc in 0..NS {
                let k = kof(ec, sc);
                kca[sc] = k;
                pca[sc] = pof(ec, k);
                ra[sc] = if ec.race >= 0 { 0.0 + wr[ec.race as usize] * pca[sc] } else { 0.0 };
            }
            for &cs in &sl[..nsl] {
                let es = &u.ent[ts][cs as usize];
                let mb = (ec.bit | es.bit) as usize;
                let mut ksa = [0usize; NS];
                let mut rb = [0.0f64; NS];
                let mut psa = [0.0f64; NS];
                for ss in 0..NS {
                    let k = kof(es, ss);
                    ksa[ss] = k;
                    psa[ss] = pof(es, k);
                    rb[ss] = if es.race >= 0 { wr[es.race as usize] * psa[ss] } else { 0.0 };
                }
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
                            if x > crow[ss] {
                                crow[ss] = x;
                                arow[ss] = [cc, cs];
                            }
                        }
                    }
                } else {
                    for sc in 0..NS {
                        for ss in 0..NS {
                            let r = rew(sc, ss);
                            let nb = (kca[sc] * NS + ksa[ss]) * kb;
                            let cb = (sc * NS + ss) * kb;
                            for s0 in 0..ns0 {
                                for m in 0..nm {
                                    let x = r + nxt[nb + s0 * nm + (m | mb)];
                                    let ci = cb + s0 * nm + m;
                                    if x > cur[ci] {
                                        cur[ci] = x;
                                        arg[ci] = [cc, cs];
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // entry value into phase 2 from junior end state (s, m): choose s0 freely; senior streak starts at s0 (or 0)
    let entry = |s: usize, m: usize| -> (f64, usize) {
        let mut b = (neg, 0);
        for s0 in 0..ns0 {
            let ss = if ns0 > 1 { s0 } else { 0 };
            let x = v2[(s * NS + ss) * kb + s0 * nm + m];
            if x > b.0 {
                b = (x, s0);
            }
        }
        b
    };
    // ---- phase 1: junior turns 0..24
    let v1 = &mut scr.v1;
    let a1 = &mut scr.a1;
    let ts1 = NS * nm;
    v1.clear();
    v1.resize(25 * ts1, neg);
    a1.clear();
    a1.resize(24 * ts1, 0);
    for s in 0..NS {
        for m in 0..nm {
            v1[24 * ts1 + s * nm + m] = entry(s, m).0;
        }
    }
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
                    if x > cur[ci] {
                        cur[ci] = x;
                        arg[ci] = c;
                    }
                }
            }
        }
    }
    let total = v1[0];
    if total == neg {
        return None;
    }
    let mut out = [Choice::Rest; NT];
    let (mut s, mut m) = (0usize, 0usize);
    for t in 0..24 {
        let c = a1[t * ts1 + s * nm + m];
        let e = &u.ent[t][c as usize];
        out[t] = code_choice(c);
        s = kof(e, s);
        m |= e.bit as usize;
    }
    let (_, s0) = entry(s, m);
    let (mut sc, mut ss) = (s, if ns0 > 1 { s0 } else { 0 });
    for j in 0..24 {
        let [cc, cs] = a2[j * js + (sc * NS + ss) * kb + s0 * nm + m];
        let (ec, es) = (&u.ent[24 + j][cc as usize], &u.ent[48 + j][cs as usize]);
        out[24 + j] = code_choice(cc);
        out[48 + j] = code_choice(cs);
        sc = kof(ec, sc);
        ss = kof(es, ss);
        m |= (ec.bit | es.bit) as usize;
    }
    Some((out, total))
}

/// Best-response memo, active only inside `optimize` (one per thread, cleared per call). dp_exact + the evaluation below
/// are a pure, deterministic function of (uma, force, exact weight bits), so a hit returns exactly what a fresh solve
/// would.
/// Key: [uma id << 16 | force code, weight bits...], built in `key` (reused) so a hit allocates nothing.
#[derive(Default)]
struct BrMemo {
    map: FxMap<Box<[u64]>, Option<(Sched, f64)>>,
    key: Vec<u64>,
}
thread_local! { static BR_MEMO: std::cell::RefCell<Option<BrMemo>> = const { std::cell::RefCell::new(None) }; }

/// Best response (exact): wraps dp_exact and cross-checks its value against direct evaluation.
pub(crate) fn best_response(d: &Data, u: &Uma, wr: &[f64], force: Option<(usize, Choice)>) -> Option<(Sched, f64)> {
    BR_MEMO.with(|cell| {
        let mut borrowed = cell.borrow_mut();
        let Some(memo) = borrowed.as_mut() else {
            drop(borrowed);
            return best_response_solve(d, u, wr, force);
        };
        // One byte each for turn and option; 255 denotes rest. All-ones denotes no force.
        let force_code: u16 = match force {
            None => u16::MAX,
            Some((turn, Choice::Rest)) => (turn * 256 + 255) as u16,
            Some((turn, Choice::Opt(option))) => (turn * 256 + option as usize) as u16,
        };
        memo.key.clear();
        memo.key.push(u64::from(u.id) << 16 | u64::from(force_code));
        memo.key.extend(wr.iter().map(|weight| weight.to_bits()));
        if let Some(result) = memo.map.get(&*memo.key) {
            return *result;
        }
        let result = best_response_solve(d, u, wr, force);
        let key: Box<[u64]> = memo.key.as_slice().into();
        memo.map.insert(key, result);
        result
    })
}

/// Enables the best-response memo for the current thread until dropped (restores the previous state).
pub(crate) struct MemoScope(Option<BrMemo>);
impl MemoScope {
    pub(crate) fn new() -> Self {
        Self(BR_MEMO.with(|memo| memo.borrow_mut().replace(BrMemo::default())))
    }
}
impl Drop for MemoScope {
    fn drop(&mut self) {
        let previous = self.0.take();
        BR_MEMO.with(|memo| *memo.borrow_mut() = previous);
    }
}

pub(crate) fn best_response_solve(
    d: &Data,
    u: &Uma,
    wr: &[f64],
    force: Option<(usize, Choice)>,
) -> Option<(Sched, f64)> {
    let (s, v) = dp_exact(d, u, wr, force)?;
    let ev = wval(wr, &probs(d, u, &s));
    assert!((ev - v).abs() < 1e-6, "dp_exact value {v} != evaluated {ev} for {}", u.name);
    assert!(feasible(u, &s), "best response produced an infeasible schedule for {}", u.name);
    Some((s, ev))
}
