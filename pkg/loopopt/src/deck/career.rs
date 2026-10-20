//! Career planner: the best next career from the veterans a player actually has (their own, plus borrowed ones).
//!
//! The loop planner assumes a running loop: every legacy carries every target spark (stars from the rank mix) and
//! race overlaps come from planned schedules. Here each candidate career (trainee T, parents L1 and L2 from the
//! veteran list, at most one borrowed) uses what those veterans really carry:
//!   - affinity per legacy from relation points and the G1 wins on record: a parent P (other parent Q, P's own
//!     parents G) counts pair(T,P) + pair(P,Q) + 3 per G1 shared by P and Q + sum over G of [tri(T,P,G) + 3 per G1
//!     shared by P and G]; a grandparent G under P counts tri(T,P,G) + 3 per G1 shared by P and G (skills.rs, same
//!     fit); a veteran whose parents are unknown adds no grandparent slots;
//!   - per target k, the inheritance chance q_k = 1 - prod over the six legacies (1 - min(1, 0.03 s (1 + A/100)))^2,
//!     s = the stars of that legacy's spark of k (0 if it has none);
//!   - spark generation 0.2 / 0.4 x 1.1^n_k for a white / gold, n_k = how many of the six carry a spark of k.
//! Everything else is the loop planner's model. q_k and the trainee's own event hint chance ew enter every formula
//! only as (1 - ew)(1 - q_k), so T's event hint chances are replaced by 1 - (1 - ew)(1 - q_k) and the deck search
//! runs at q = 0: its products, bounds and exact odds are then the career's, apart from the factor 1.1^(n_k - 6) per
//! target, which is the same for every deck and multiplies the result (`JointExt` applies it per target where the
//! number is not a plain product: P(all but one), the per-target chances).
//!
//! Search: every legal candidate gets a ceiling, EXACT_RHO x C x the product value at each target's best possible
//! deck odds (the deck_size lowest miss chances of any legal entries, links firing for sure); candidates are solved
//! best ceiling first (best product deck, then every deck whose product value times EXACT_RHO reaches the best exact
//! value, scored exactly, as in the per-uma search) until the next ceiling is below the top-k exact value.
//!
//! Schedule: every G1 win counts one, plus one per parent who won that race (`best_response` weights), under the same
//! goals, race clashes and streak penalties as the loop scheduler; the report lists every G1 she could enter.
//!
//! The next career the page picks (`next`) is `sim::plan::index_rule`; the simulator (`simulate`) is `sim`.
use super::*;
mod cache;
mod sim;

/// wall clock in ms: the host's `env.clock_ms` in wasm (performance.now()), else the process clock
#[cfg(target_arch = "wasm32")]
fn now_ms() -> f64 {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" { fn clock_ms() -> f64; }
    unsafe { clock_ms() }
}
#[cfg(not(target_arch = "wasm32"))]
fn now_ms() -> f64 {
    static T0: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    T0.get_or_init(std::time::Instant::now).elapsed().as_secs_f64() * 1000.0
}
/// the plan's progress for the page's status line, at its phase boundaries (`sim::plan::index_rule`): phase, k of n. In
/// wasm the host's `env.progress` (the worker posts it to the page while the plan runs); elsewhere nothing
#[cfg(target_arch = "wasm32")]
fn progress(phase: u32, k: usize, n: usize) {
    #[link(wasm_import_module = "env")]
    unsafe extern "C" { #[link_name = "progress"] fn host_progress(phase: u32, k: u32, n: u32); }
    unsafe { host_progress(phase, k as u32, n as u32) }
}
#[cfg(not(target_arch = "wasm32"))]
fn progress(_phase: u32, _k: usize, _n: usize) {}
// phase timers and counters for speed work, per plan (`next.work.prof`; planval and the val compares skip `work`)
thread_local! { static PROF: std::cell::RefCell<std::collections::BTreeMap<&'static str, f64>> = Default::default(); }
fn prof(k: &'static str, v: f64) { PROF.with(|p| *p.borrow_mut().entry(k).or_insert(0.0) += v); }

#[derive(Deserialize, Clone, Default)]
pub(crate) struct Ancestor {
    pub chara: u32,
    /// (skill group, stars 1..=3) of its white skill sparks
    #[serde(default)] pub sparks: Vec<(u32, u8)>,
    /// race indices (data.json `races`) of the single-race G1s it won
    #[serde(default)] pub wins: Vec<usize>,
    /// (aptitude factor group, stars) of its pink sparks: 11 turf, 12 dirt, 21-24 front runner .. end closer, 31-34
    /// sprint .. long; reported (inheritance chance per aptitude), not part of the odds
    #[serde(default)] pub pinks: Vec<(u32, u8)>,
}
#[derive(Deserialize, Clone, Default)]
pub(crate) struct Legacy {
    pub chara: u32,
    #[serde(default)] pub borrowed: bool,
    #[serde(default)] pub sparks: Vec<(u32, u8)>,
    #[serde(default)] pub wins: Vec<usize>,
    #[serde(default)] pub pinks: Vec<(u32, u8)>,
    /// its own parents (the new trainee's grandparents); empty when unknown
    #[serde(default)] pub parents: Vec<Ancestor>,
    /// internal (a veteran before her result is known, `Env::veteran`): win chance per race index in place of `wins`,
    /// and the star mix her sparks are drawn from in place of fixed stars
    #[serde(skip)] win_p: Vec<f64>,
    #[serde(skip)] mix: Option<[f64; 3]>,
}
fn d_top() -> usize { 5 }
/// The work budget of one career search (units as in the per-uma search: 2 per deck node tested, 200 per exact
/// scoring, 10,000 per deck search). Measured need for a proven result (2026-10-07, synthetic pools of 12-400
/// veterans, default 8 targets): 10M-85M, the most at 200 veterans (12 s native); the loop planner's 30M
/// (EXACT_BUDGET) left pools of 12, 100 and 200 veterans unproven with the same picks. 300M leaves 3.5x headroom.
/// It does not stop the search: past it each deck search (`best_deck_run`) stops scoring its band, so every later
/// candidate gets the exact values of its product deck and start decks only and the search is not `complete`; `solve` itself goes on to its
/// stop rule (only a `hard` cap, `plan`'s extra searches, ends it early). `EXACT_BUDGET` in the environment overrides it.
const CAREER_BUDGET: usize = 300_000_000;
#[derive(Deserialize)]
struct CareerJob {
    legacies: Vec<Legacy>,
    /// trainees to consider (chara ids); empty = every owned uma
    #[serde(default)] trainees: Vec<u32>,
    #[serde(default = "d_top")] top: usize,
    /// legacies (indices) that must be among the two parents
    #[serde(default)] fix: Vec<usize>,
    /// one result per trainee (false: the best careers whoever trains, e.g. with the trainee fixed: the best parents)
    #[serde(default = "d_true")] per_trainee: bool,
    /// the builder-trace simulator (`sim`): runs instead of the search when present
    #[serde(default)] simulate: Option<Value>,
    /// the page's next career without simulation (`sim::plan::index_rule`): the career saving the most careers to a fully looped veteran
    #[serde(default)] next: Option<Value>,
    /// the page's veteran-analysis comparison (echoed in the output, nothing else)
    #[serde(default)] compare: bool,
    /// the plan-only jobs' flag: the page reads only `next` from them, so the main search's career reports (and the first
    /// one's schedule search) are left out
    #[serde(default)] no_reports: bool,
}

/// What a lineage counts of one legacy: relation index, G1 wins as (race, chance) sorted by race, stars per job target,
/// pink sparks, and the star mix when the stars are random.
#[derive(Clone)]
/// (stars and pinks shared with the lineage slots made from her: `Env::slots` takes them without copying)
struct Who { ix: Option<usize>, chara: u32, wins: Vec<(usize, f64)>, stars: std::rc::Rc<Vec<u8>>, pinks: std::rc::Rc<Vec<(u32, u8)>>, mix: Option<[f64; 3]> }
/// The veterans the parents come from, each with her own parents; per veteran and parent, the race part of that
/// grandparent slot's affinity, 3 x shared(veteran, parent) (constant per veteran: `Env::slots` reads it)
#[derive(Clone)]
struct Pool { me: Vec<Who>, gp: Vec<Vec<Who>>, borrowed: Vec<bool>, gpc: Vec<Vec<f64>> }
/// A legacy's pink spark over a career: her aptitude (data.json `apt` order), the chance it fires at one April
/// inspiration, the legacy slot, stars
#[derive(Clone)]
struct PinkFire { apt: usize, p: f64, slot: usize, stars: u8 }
/// The trainee's aptitude codes (base, after the career start's pinks) and her legacies' pink sparks
struct PinkCareer { base: [u8; 10], start: [u8; 10], fire: Vec<PinkFire> }
/// The April inspirations (0-based turns: Classic and Senior early April); a race on that turn already counts them
const INSP_TURNS: [usize; 2] = [30, 54];
/// aptitude index (data.json `apt` order) of a pink factor group: 31-34 short..long, 11 turf, 12 dirt, 21-24 front
/// runner..end closer
fn pink_apt(g: u32) -> Option<usize> { match g { 31..=34 => Some((g - 31) as usize), 11 => Some(4), 12 => Some(5), 21..=24 => Some((g - 21) as usize + 6), _ => None } }
/// the pink factor group of aptitude k (inverse of `pink_apt`)
fn apt_pink(k: usize) -> u32 { match k { 0..=3 => 31 + k as u32, 4 => 11, 5 => 12, _ => 21 + (k as u32 - 6) } }
/// distribution of the number of successes over independent chances ps (Poisson binomial)
fn pbinom(ps: &[f64]) -> Vec<f64> {
    let mut v = vec![1.0f64];
    for &p in ps { let mut w = vec![0.0f64; v.len() + 1]; for (i, &x) in v.iter().enumerate() { w[i] += x * (1.0 - p); w[i + 1] += x * p; } v = w; }
    v
}
fn conv(a: &[f64], b: &[f64]) -> Vec<f64> { let mut w = vec![0.0f64; a.len() + b.len() - 1]; for (i, &x) in a.iter().enumerate() { for (j, &y) in b.iter().enumerate() { w[i + j] += x * y; } } w }
/// One legacy slot of a lineage: affinity parts and per-target stars.
#[derive(Clone)]
struct Slot { parent: bool, legacy: usize, gp: Option<usize>, chara: u32, rel: f64, race: f64, stars: std::rc::Rc<Vec<u8>>, pinks: std::rc::Rc<Vec<(u32, u8)>>, mix: Option<[f64; 3]> }
/// Whether legacy `x` is at least as good a parent as legacy `y` in every career (`Env::solve`, dominated veterans):
/// the same character, stars at least y's on every job target, G1 wins a superset of y's, and the same number of
/// parents, each (in order) the same character as y's, with stars and wins at least as good the same way. Only fixed
/// stars and certain wins (a veteran before her result is known, with random stars and win chances, never compares).
/// Derived: with partner z and trainee t, swapping y for x keeps every relation term (same characters, grandparents
/// in the same slots) and raises or keeps every shared-win count (x with z, x with her parents; nothing else changes),
/// so every slot's affinity and stars are at least as high: each q_k is at least as high (1 - prod (1 - min(1,
/// 0.03 s (1 + A/100)))^2 rises with every s and A), each carrier count n_k too (so the spark scale C), and with them
/// the ceiling, the `rise` bounds and every deck's product and exact value (each landing factor rises with q_k). The
/// same sums and products in the same order, so this holds for the floats as well (correctly rounded operations are
/// monotone).
fn dominates(p: &Pool, x: usize, y: usize) -> bool {
    let ge = |a: &Who, b: &Who| a.chara == b.chara && a.mix.is_none() && b.mix.is_none() && a.stars.iter().zip(b.stars.iter()).all(|(s, t)| s >= t)
        && a.wins.iter().chain(&b.wins).all(|w| w.1 == 1.0) && b.wins.iter().all(|w| a.wins.iter().any(|v| v.0 == w.0));
    ge(&p.me[x], &p.me[y]) && p.gp[x].len() == p.gp[y].len() && p.gp[x].iter().zip(&p.gp[y]).all(|(a, b)| ge(a, b))
}
/// `Env::set_trainee` on the fields it writes and reads (her model hint chances `cur`, her own `ew0`), for callers that
/// hold a borrow of the deck space
fn set_ews(cur: &mut Vec<Vec<f64>>, ew0: &[Vec<f64>], uncv: &[usize], q: &[f64]) {
    // (in place when the shape is already this one: the same values)
    if cur.len() == ew0.len() && cur.iter().zip(ew0).all(|(c, o)| c.len() == o.len().min(uncv.len())) {
        for (c, o) in cur.iter_mut().zip(ew0) { for ((c, e), &i) in c.iter_mut().zip(o).zip(uncv) { *c = 1.0 - (1.0 - e) * (1.0 - q[i]); } }
    } else {
        *cur = ew0.iter().map(|o| o.iter().zip(uncv).map(|(e, &i)| 1.0 - (1.0 - e) * (1.0 - q[i])).collect()).collect();
    }
}
/// G1s two legacies both won (expected count when a win is a chance)
fn shared(a: &Who, b: &Who) -> f64 {
    let (mut i, mut j, mut s) = (0, 0, 0.0f64);
    while i < a.wins.len() && j < b.wins.len() {
        match a.wins[i].0.cmp(&b.wins[j].0) { std::cmp::Ordering::Less => i += 1, std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => { s += a.wins[i].1 * b.wins[j].1; i += 1; j += 1; } }
    }
    s
}
/// Per job target: the inheritance chance and how many of the six carry the spark. A slot with random stars (`mix`)
/// enters as the average of its squared miss factor (its stars are independent of everything else).
fn inherit(sl: &[Slot], ntar: usize) -> (Vec<f64>, Vec<i32>) {
    let mut q = vec![0.0f64; ntar]; let mut nc = vec![0i32; ntar];
    for k in 0..ntar {
        let (mut miss, mut mix2) = (1.0f64, 1.0f64);
        for s in sl { let st = s.stars[k]; if st == 0 { continue; } nc[k] += 1;
            let m = |st: f64| 1.0 - (0.03 * st * (1.0 + (s.rel + s.race) / 100.0)).min(1.0);
            match s.mix { None => miss *= m(st as f64), Some(w) => mix2 *= (0..3).map(|i| w[i] * m(i as f64 + 1.0).powi(2)).sum::<f64>() } }
        q[k] = 1.0 - miss * miss * mix2;
    }
    (q, nc)
}
/// A job's setup fingerprint: everything but the pool and the places (targets, roster, cards, scenario, ...), the
/// key of what one job's search proves that holds for the next (p_bar, fully checked decks)
pub(super) fn setup_fp(v: &Value) -> u64 {
    let mut key = v.clone();
    if let Some(o) = key.as_object_mut() {
        // (also the page's flags that change no deck value: `compare`, and the plan's own fields, so the plan and compare
        // jobs share what the search jobs proved)
        for k in ["legacies", "trainees", "fix", "top", "per_trainee", "cmd", "mode", "compare", "next", "no_reports"] { o.remove(k); }
    }
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    key.to_string().hash(&mut h);
    h.finish()
}
fn setup_hash(s: &str) -> u64 { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); s.hash(&mut h); h.finish() }
thread_local! {
    /// the last few jobs' main searches (the most recent last), each keyed by the whole job without `next` and
    /// `no_reports` (the page's plan job repeats the job it follows), with what the search's product searches proved
    /// (`Env::ruled` as the search left it: a job that takes a kept search starts its plan from the Env state a combined
    /// job's plan starts from); kept across page reloads (`state_out`)
    static LAST_SOLVE: std::cell::RefCell<Vec<(u64, Solved, Ruled)>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// per deck class, the product searches' proofs (`Env::ruled`)
type Ruled = Vec<Vec<(Vec<f64>, f64)>>;
/// main searches kept (`LAST_SOLVE`): the page's run, and one it goes back to (a setting changed and back, a Compare or
/// Reroll plan in between)
const SOLVE_KEEP: usize = 3;
thread_local! {
    /// per setup fingerprint (the last few), every fully checked deck found so far (`Env::seen`): what the page's
    /// earlier jobs proved bounds the next job's searches. A Ref holds a trainee's chances and her best exact value
    /// there, which depend on the setup only (the job's trainees change no deck value: they only order plain pals).
    static SEEN: std::cell::RefCell<Vec<(u64, Vec<Vec<Ref>>)>> = const { std::cell::RefCell::new(Vec::new()) };
}
/// Fully checked decks kept per trainee across jobs (the most recent): the bound loop over them stays cheap
const SEEN_KEEP: usize = 256;
// ---------- the worker's kept work across page loads ----------
// What a worker keeps between jobs (the kept plans, `sim::plan::BESTS`; the proofs, `SEEN`) leaves as `state_out` and comes
// back as `state_in`, so the first plan after a page reload reuses the work of the last session. Every float travels as
// its bit pattern in hex (decimal JSON does not round-trip here), so what comes back is what left, bit for bit; the page
// keys what it stores by the solver and data fingerprints, so a release starts afresh (the keys inside, `setup_fp` and
// `bests_key`, hash with std's DefaultHasher, fixed within a build). A store that does not parse is refused whole.
const STATE_VERSION: u64 = 3;   // 2: the kept plans carry each uma's looped career; 3: the kept main searches (`LAST_SOLVE`)
fn hx(x: f64) -> Value { Value::String(format!("{:016x}", x.to_bits())) }
fn ux(x: u64) -> Value { Value::String(format!("{:016x}", x)) }
fn fx(v: &Value) -> Option<f64> { Some(f64::from_bits(u64::from_str_radix(v.as_str()?, 16).ok()?)) }
fn uxr(v: &Value) -> Option<u64> { u64::from_str_radix(v.as_str()?, 16).ok() }
fn fvec(v: &[f64]) -> Value { Value::Array(v.iter().map(|&x| hx(x)).collect()) }
fn fvec_r(v: &Value) -> Option<Vec<f64>> { v.as_array()?.iter().map(fx).collect() }
fn usz(v: &Value) -> Option<usize> { v.as_u64().map(|x| x as usize) }
fn sel_out(s: &Sel) -> Value { json!([s.ents, s.plain, s.filler, fvec(&s.dw), fvec(&s.dg)]) }
fn sel_in(v: &Value) -> Option<Sel> {
    let a = v.as_array()?;
    Some(Sel { ents: a.first()?.as_array()?.iter().map(usz).collect::<Option<_>>()?, plain: usz(a.get(1)?)?, filler: usz(a.get(2)?)?,
               dw: fvec_r(a.get(3)?)?, dg: fvec_r(a.get(4)?)? })
}
fn ref_out(r: &Ref) -> Value { json!([fvec(&r.q), hx(r.val), sel_out(&r.sel)]) }
fn ref_in(v: &Value) -> Option<Ref> { let a = v.as_array()?; Some(Ref { q: fvec_r(a.first()?)?, val: fx(a.get(1)?)?, sel: sel_in(a.get(2)?)? }) }
fn found_out(f: &Found) -> Value { json!([f.t, f.a, f.b, hx(f.val), hx(f.ub), sel_out(&f.sel)]) }
fn found_in(v: &Value) -> Option<Found> {
    let a = v.as_array()?;
    Some(Found { t: usz(a.first()?)?, a: usz(a.get(1)?)?, b: usz(a.get(2)?)?, val: fx(a.get(3)?)?, ub: fx(a.get(4)?)?, sel: sel_in(a.get(5)?)? })
}
/// SEEN as [fingerprint, classes, [[class, [refs]], ...]] per setup (only the classes holding proofs)
fn seen_out() -> Value {
    SEEN.with(|c| Value::Array(c.borrow().iter().map(|(fp, vv)| json!([ux(*fp), vv.len(),
        vv.iter().enumerate().filter(|(_, rs)| !rs.is_empty()).map(|(i, rs)| json!([i, rs.iter().map(ref_out).collect::<Vec<_>>()])).collect::<Vec<_>>()])).collect()))
}
fn seen_in(v: &Value) -> Option<Vec<(u64, Vec<Vec<Ref>>)>> {
    v.as_array()?.iter().map(|e| {
        let a = e.as_array()?;
        let (fp, n) = (uxr(a.first()?)?, usz(a.get(1)?)?);
        let mut vv: Vec<Vec<Ref>> = vec![vec![]; n];
        for c in a.get(2)?.as_array()? {
            let ca = c.as_array()?; let i = usz(ca.first()?)?;
            if i >= n { return None; }
            vv[i] = ca.get(1)?.as_array()?.iter().map(ref_in).collect::<Option<_>>()?;
        }
        Some((fp, vv))
    }).collect()
}
/// per-uma (or per-class) lists as [length, [[index, [items]], ...]] (only the non-empty ones)
fn lists_out<T>(vv: &[Vec<T>], f: impl Fn(&T) -> Value) -> Value {
    json!([vv.len(), vv.iter().enumerate().filter(|(_, rs)| !rs.is_empty()).map(|(i, rs)| json!([i, rs.iter().map(&f).collect::<Vec<_>>()])).collect::<Vec<_>>()])
}
fn lists_in<T: Clone>(v: &Value, f: impl Fn(&Value) -> Option<T>) -> Option<Vec<Vec<T>>> {
    let a = v.as_array()?; let n = usz(a.first()?)?;
    let mut vv: Vec<Vec<T>> = vec![vec![]; n];
    for c in a.get(1)?.as_array()? {
        let ca = c.as_array()?; let i = usz(ca.first()?)?;
        if i >= n { return None; }
        vv[i] = ca.get(1)?.as_array()?.iter().map(&f).collect::<Option<_>>()?;
    }
    Some(vv)
}
/// LAST_SOLVE as [key, search, proofs] per kept search: every field of `Solved`, floats as bit patterns
fn solve_out() -> Value {
    LAST_SOLVE.with(|c| Value::Array(c.borrow().iter().map(|(k, s, r)| json!([ux(*k), {
        "found": s.found.iter().map(found_out).collect::<Vec<_>>(), "refs": lists_out(&s.refs, ref_out),
        "n": [s.work, s.cands, s.solved, s.bounded, s.scored, s.skipped], "next_bound": hx(s.next_bound), "complete": s.complete, "proven": s.proven, "cert": hx(s.cert)
    }, lists_out(r, |(q, x)| json!([fvec(q), hx(*x)]))])).collect()))
}
fn solve_in(v: &Value) -> Option<Vec<(u64, Solved, Ruled)>> {
    v.as_array()?.iter().map(|e| {
        let a = e.as_array()?; let s = a.get(1)?;
        let n: Vec<usize> = s["n"].as_array()?.iter().map(usz).collect::<Option<_>>()?;
        if n.len() != 6 { return None; }
        let sv = Solved { found: s["found"].as_array()?.iter().map(found_in).collect::<Option<_>>()?, refs: lists_in(&s["refs"], ref_in)?,
            work: n[0], cands: n[1], solved: n[2], bounded: n[3], scored: n[4], skipped: n[5], next_bound: fx(&s["next_bound"])?,
            complete: s["complete"].as_bool()?, proven: s["proven"].as_bool()?, cert: fx(&s["cert"])? };
        let ruled = lists_in(a.get(2)?, |x| { let p = x.as_array()?; Some((fvec_r(p.first()?)?, fx(p.get(1)?)?)) })?;
        Some((uxr(a.first()?)?, sv, ruled))
    }).collect()
}
fn state_out() -> Value { json!({"ok": true, "state": {"v": STATE_VERSION, "bests": sim::plan::bests_out(), "seen": seen_out(), "solve": solve_out()}}) }
/// replaces the kept plans, proofs and main searches with the page's stored ones (all or nothing: a store that does not
/// parse is refused)
fn state_in(v: &Value) -> Value {
    if v["v"].as_u64() != Some(STATE_VERSION) { return json!({"error": "state: another version"}); }
    let (Some(b), Some(sn), Some(so)) = (sim::plan::bests_in(&v["bests"]), seen_in(&v["seen"]), solve_in(&v["solve"])) else { return json!({"error": "state: does not parse"}) };
    let (nb, ns, nm) = (b.len(), sn.len(), so.len());
    sim::plan::bests_set(b);
    SEEN.with(|c| *c.borrow_mut() = sn);
    LAST_SOLVE.with(|c| *c.borrow_mut() = so);
    json!({"ok": true, "bests": nb, "seen": ns, "solve": nm})
}
/// Multilinear interpolation of corner values v (index bit k = coordinate k at 1) at the point x, with a buffer the
/// caller keeps (the simulator and the plan evaluate tables millions of times).
/// Folds coordinate 0 first, halving the table each step: entry i after k folds is the full sweep's entry i << k (the
/// same multiply-adds on the same values, so the same bits), and only those reach entry 0.
fn interp_in(v: &[f64], x: &[f64], cur: &mut Vec<f64>) -> f64 {
    cur.clear(); cur.extend_from_slice(v);
    let mut n = cur.len();
    for &xk in x {
        n /= 2;
        for i in 0..n { cur[i] = cur[2 * i] * (1.0 - xk) + cur[2 * i + 1] * xk; }
    }
    cur[0]
}
/// A candidate the search solved: trainee, parents (legacy indices), value C x E, its ceiling, its best deck.
#[derive(Clone)]
struct Found { t: usize, a: usize, b: usize, val: f64, ub: f64, sel: Sel }
/// A trainee's fully checked deck: the chances it was found at (job-target order), its exact value there before the
/// spark scale (the best of every legal deck, under the EXACT_RHO certificate), the deck. Its value bounds every deck
/// at other chances through `rise`.
#[derive(Clone)]
struct Ref { q: Vec<f64>, val: f64, sel: Sel }
#[derive(Clone)]
enum DeckRes { NoDeck, Bounded, Done(f64, Sel, bool) }
#[derive(Clone)]
/// (`cert`: the lowest cap a deck search of it was certified under, INFINITY when none was lowered; see `Env::cert`)
struct Solved { found: Vec<Found>, refs: Vec<Vec<Ref>>, work: usize, cands: usize, solved: usize, bounded: usize, scored: usize, skipped: usize, next_bound: f64, complete: bool, proven: bool, cert: f64 }

struct Env<'a> {
    d: &'a Data, sj: &'a SkillJob, model: Model<'a>, ew0: Vec<Vec<Vec<f64>>>, uncv: Vec<usize>, fixed_t: Vec<usize>, ntar: usize, nu: usize,
    unc: Vec<u32>, space: DeckSpace<'a>, dmax: Vec<Option<(Vec<f64>, Vec<f64>)>>, sts0: Vec<Vec<Vec<u8>>>, owned: Vec<usize>, budget: usize, work: usize,
    /// every earlier search's fully checked decks per deck class (`cls`; one model, so each is a proven best at its
    /// chances for every trainee of the class): later searches bound with them (`rise`), but don't start from them, so
    /// ties resolve as in a lone search
    seen: Vec<Vec<Ref>>,
    /// each trainee's deck class, as the index of its first trainee: trainees with the same outfit coverage and own-skill
    /// chances of the targets, the same relevant cards barred and as many plain pals face the same deck problem (her
    /// best deck and its value are one function of the inheritance chances; `rise` and `dmax` agree), so a proof one of
    /// them makes (`Ref`, `ruled`) holds for all of them. Within a class only the slots' affinity (so q) differs.
    cls: Vec<usize>,
    /// whether `solve` bounds with `seen`: off for the search whose fully checked decks seed the simulator's cache
    /// (so the simulator's values do not depend on what earlier searches proved)
    use_seen: bool,
    /// per deck class (`cls`), what its product searches proved (`best_deck` pass 1): (chances q, S) with every legal deck's product
    /// value at q (fixed factor included, before the spark scale) at most S. Bounds her other candidates (`solve`).
    ruled: Vec<Vec<(Vec<f64>, f64)>>,
    /// what the last `best_deck` call's product search proved: Some(S) as in `ruled`, None when it proved nothing
    last_pv: Option<f64>,
    /// `solve` keeps only careers above this value (the plan's search with a new veteran fixed, against the trainee's kept
    /// best: careers below it are pruned as if the bar were already there); NEG_INFINITY for every other search
    pub(crate) floor: f64,
    /// per structure class (`cap_class`), the largest exact / product value of any deck `best_deck` scored (the class's
    /// assumed cap must exceed it; the planner reports it and `val/planval.py` fails when it comes near the cap)
    pub(crate) max_ratio: [f64; 4],
    /// exact scorings whose exact / product ratio passed their class's cap: the cap is an assumption (`EXACT_CAPS`), so
    /// a search that saw one may have pruned a deck that beats its answer; such a search is not `complete`
    pub(crate) cap_breach: usize,
    /// per class, the trainee and deck that set `max_ratio`
    pub(crate) max_deck: [Option<(usize, Sel)>; 4],
    /// the lowest cap any deck search was certified under (`best_deck_run`: a band over EXACT_CAP decks is retried
    /// down the cap ladder), and how many bands overflowed; a search below the top cap is not `full`
    pub(crate) cert: f64,
    pub(crate) overflows: usize,
    /// a hard work cap for `solve` (0: none): past it the search stops taking candidates (`plan`'s extra searches)
    hard: usize,
    /// `best_deck` calls without start decks, by what they read (`DeckKey`): the outcome and what the call did besides
    /// (kept between jobs of one setup, `DECK_KEEP`)
    deck_memo: crate::FxMap<DeckKey, DeckMemo>,
    /// `corners_kept`'s tables by (deck class, the deck's entries, filler), kept between jobs of one setup with `deck_memo`
    corner_memo: crate::FxMap<(usize, Vec<usize>, usize), std::rc::Rc<Vec<Vec<f64>>>>,
    /// the setup the memo is kept under, and this job's number
    memo_fp: u64, job: u64,
}
/// What a `best_deck` call without start decks reads: the trainee's deck class (everything of her the deck search reads
/// but the folded chances), her folded event-hint chances per outfit (`set_trainee`: the model state the search runs on,
/// to the bit), the spark scale c, the bar th (every th <= 0 as one: each gives no seed and passes the ceiling test),
/// and the work spent before the call (the work budget's tests read it)
#[derive(PartialEq, Eq, Hash)]
struct DeckKey { cls: usize, ews: Vec<u64>, c: u64, th: u64, work: usize }
/// A memoised `best_deck` outcome: the result, its product search's proof (`last_pv`), the work and exact scorings it
/// added
struct DeckMemo { res: DeckRes, pv: Option<f64>, work: usize, scored: usize, cert: f64, overflows: usize,
    /// the deck-search nodes and exact scorings it spent; per class, the call's own largest exact / product ratio and the
    /// deck that first reached it (`note_ratio` from nothing); the last job that used it (`Env::job`)
    nodes: usize, joints: usize, ratio: [Option<(f64, usize, Sel)>; 4], job: u64 }
thread_local! {
    /// `best_deck`'s memo kept between jobs (the page's worker runs a plan, then a Reroll's plans with her added: the
    /// builders whose best pair doesn't involve her search the same deck problems again), per setup: `setup_fp` (what
    /// sets the model, the deck space and the class caps, `cap_audit` included) with the job's trainees (the plain pals'
    /// order), the last few. Exact: a `DeckKey` reads the same model, space and caps in every job of one setup, and the
    /// work budget's argument holds for any budget (`best_deck`)
    static DECK_KEEP: std::cell::RefCell<Vec<(u64, Kept)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// jobs run so far (`Env::job`)
    static JOB_NO: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}
/// what a setup keeps between jobs: `Env::deck_memo` and `Env::corner_memo`
#[derive(Default)]
struct Kept { decks: crate::FxMap<DeckKey, DeckMemo>, corners: crate::FxMap<(usize, Vec<usize>, usize), std::rc::Rc<Vec<Vec<f64>>>> }
/// setups whose memo is kept, and the entries past which a setup's memo starts afresh (at a job's start)
const DECK_KEEP_SETUPS: usize = 4;
const DECK_KEEP_CAP: usize = 8192;
/// a new data set (the worker's `init`): nothing kept from the old one
#[cfg(target_arch = "wasm32")]
pub(crate) fn forget_kept_decks() { DECK_KEEP.with(|c| c.borrow_mut().clear()); }
impl Drop for Env<'_> {
    fn drop(&mut self) {
        let m = Kept { decks: std::mem::take(&mut self.deck_memo), corners: std::mem::take(&mut self.corner_memo) }; let fp = self.memo_fp;
        DECK_KEEP.with(|c| { let mut c = c.borrow_mut(); c.retain(|e| e.0 != fp); c.push((fp, m)); while c.len() > DECK_KEEP_SETUPS { c.remove(0); } });
    }
}
impl<'a> Env<'a> {
    /// a scored deck's exact / product ratio r, class k, trainee t (`max_ratio`, `max_deck`)
    fn note_ratio(&mut self, k: usize, r: f64, t: usize, sel: &Sel) { self.note_breach(k, r); if r > self.max_ratio[k] { self.max_ratio[k] = r; self.max_deck[k] = Some((t, sel.clone())); } }
    /// a ratio past its class's cap (`cap_breach`)
    fn note_breach(&mut self, k: usize, r: f64) { if r > self.space.caps()[k] * (1.0 + 1e-9) { self.cap_breach += 1; } }
    /// per class, the deck that set `max_ratio` (trainee name, cards), for the cap audit
    pub(crate) fn max_decks(&self) -> Value {
        json!(self.max_deck.iter().map(|x| x.as_ref().map(|(t, s)| json!({"trainee": self.d.umas[*t].name, "cards": self.space.pdeck(s, &[self.d.umas[*t].id]).names}))).collect::<Vec<_>>())
    }
    fn uix(&self, id: u32) -> Option<usize> { self.d.umas.iter().position(|u| u.id == id) }
    fn stars_of(&self, sp: &[(u32, u8)]) -> Vec<u8> { self.sj.targets.iter().map(|t| sp.iter().filter(|s| s.0 == t.group).map(|s| s.1.min(3)).max().unwrap_or(0)).collect() }
    fn who(&self, chara: u32, sparks: &[(u32, u8)], wins: &[usize], win_p: &[f64], pinks: &[(u32, u8)], mix: Option<[f64; 3]>) -> Who {
        let mut w: Vec<(usize, f64)> = if win_p.is_empty() { let mut v = wins.to_vec(); v.sort(); v.dedup(); v.into_iter().map(|r| (r, 1.0)).collect() }
            else { win_p.iter().enumerate().filter(|x| *x.1 > 0.0).map(|(r, &p)| (r, p)).collect() };
        w.sort_by_key(|x| x.0);
        Who { ix: self.uix(chara), chara, wins: w, stars: std::rc::Rc::new(self.stars_of(sparks)), pinks: std::rc::Rc::new(pinks.to_vec()), mix }
    }
    /// `pool` of the legacies so far plus `l` (pop with `pool_pop`)
    fn pool_push(&self, p: &mut Pool, l: &Legacy) {
        let me = self.who(l.chara, &l.sparks, &l.wins, &l.win_p, &l.pinks, l.mix);
        let gp: Vec<Who> = l.parents.iter().map(|g| self.who(g.chara, &g.sparks, &g.wins, &[], &g.pinks, None)).collect();
        p.gpc.push(gp.iter().map(|g| 3.0 * shared(&me, g)).collect());
        p.me.push(me); p.gp.push(gp); p.borrowed.push(l.borrowed);
    }
    fn pool_pop(&self, p: &mut Pool) { p.me.pop(); p.gp.pop(); p.borrowed.pop(); p.gpc.pop(); }
    fn pool(&self, legs: &[Legacy]) -> Pool {
        let me: Vec<Who> = legs.iter().map(|l| self.who(l.chara, &l.sparks, &l.wins, &l.win_p, &l.pinks, l.mix)).collect();
        let gp: Vec<Vec<Who>> = legs.iter().map(|l| l.parents.iter().map(|g| self.who(g.chara, &g.sparks, &g.wins, &[], &g.pinks, None)).collect()).collect();
        let gpc = me.iter().zip(&gp).map(|(m, g)| g.iter().map(|g| 3.0 * shared(m, g)).collect()).collect();
        Pool { me, gp, borrowed: legs.iter().map(|l| l.borrowed).collect(), gpc }
    }
    fn pair(&self, a: Option<usize>, b: Option<usize>) -> f64 { match (a, b) { (Some(a), Some(b)) => self.d.pair[a][b] as f64, _ => 0.0 } }
    fn tri(&self, a: Option<usize>, b: Option<usize>, c: Option<usize>) -> f64 { match (a, b, c) { (Some(a), Some(b), Some(c)) => self.d.tri[a][b][c] as f64, _ => 0.0 } }
    /// the six legacy slots of trainee t with parents a and b
    fn slots(&self, p: &Pool, t: usize, a: usize, b: usize) -> Vec<Slot> {
        let ti = Some(t); let mut v = Vec::with_capacity(2 + p.gp[a].len() + p.gp[b].len());
        // (the parents' shared wins once: shared(a, b) and shared(b, a) are the same products summed in the same order)
        let sh = 3.0 * shared(&p.me[a], &p.me[b]);
        for (x, y) in [(a, b), (b, a)] {
            let px = &p.me[x];
            let mut rel = self.pair(ti, px.ix) + self.pair(px.ix, p.me[y].ix); let mut race = sh;
            let i0 = v.len();
            v.push(Slot { parent: true, legacy: x, gp: None, chara: px.chara, rel: 0.0, race: 0.0, stars: px.stars.clone(), pinks: px.pinks.clone(), mix: px.mix });
            for (gi, g) in p.gp[x].iter().enumerate() {
                let (gr, gc) = (self.tri(ti, px.ix, g.ix), p.gpc[x][gi]);
                rel += gr; race += gc;
                v.push(Slot { parent: false, legacy: x, gp: Some(gi), chara: g.chara, rel: gr, race: gc, stars: g.stars.clone(), pinks: g.pinks.clone(), mix: g.mix });
            }
            v[i0].rel = rel; v[i0].race = race;
        }
        v
    }
    fn scale(&self, nc: &[i32]) -> (Vec<f64>, Vec<f64>) {
        (self.uncv.iter().map(|&i| 1.1f64.powi(nc[i] - 6)).collect(), self.fixed_t.iter().map(|&i| 1.1f64.powi(nc[i] - 6)).collect())
    }
    /// trainee t's event hint chances with her lineage's inheritance chances folded in (the search then runs at q = 0)
    fn set_trainee(&mut self, t: usize, q: &[f64]) { set_ews(&mut self.model.ews[t], &self.ew0[t], &self.uncv, q) }
    /// a deck's entries (sources borrowed from the deck space: `joint` only reads them)
    fn ents_of(&self, sel: &Sel) -> Vec<(&Src, Kind)> { sel.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect() }
    /// exact P(all) of trainee t (her inheritance already folded in) with deck `sel`, scaled per target; `sets`: also the
    /// chance of each set of uncertain targets landing
    fn exact(&self, t: usize, sel: &Sel, nc: &[i32], m1: bool, sets: bool) -> ((f64, f64, usize), JointExt) {
        let (su, sf) = self.scale(nc);
        let mut ext = JointExt { su, sf, marg: vec![], sets: if sets { vec![0.0] } else { vec![] }, per_outfit: vec![] };
        let j = joint_fast(&self.model, t, &[0.0], &self.ents_of(sel), sel.filler, self.ntar, m1, Some(&mut ext))[0];
        (j, ext)
    }

    /// How far trainee t's best exact value (before the spark scale) can rise from chances q0 to q. A target the
    /// outfit lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no hint,
    /// e: her own event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 - q0_k))
    /// (n = 1, g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products of
    /// these factors, so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of it.
    fn rise(&self, t: usize, q: &[f64], q0: &[f64]) -> f64 {
        self.sts0[t].iter().zip(&self.ew0[t]).map(|(st, ew)| self.uncv.iter().enumerate().map(|(k, &i)| {
            if st[k] > 0 || q[i] <= q0[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - q[i])) / (1.0 - (1.0 - ew[k]) * (1.0 - q0[i])) }
        }).product::<f64>()).fold(0.0f64, f64::max)
    }
    /// The inheritance chances, carriers and spark scale C of trainee t with parents a and b.
    fn lineage(&self, p: &Pool, t: usize, a: usize, b: usize) -> (Vec<f64>, Vec<i32>, f64) {
        let (q, nc) = inherit(&self.slots(p, t, a, b), self.ntar);
        let (su, sf) = self.scale(&nc);
        let c = su.iter().chain(&sf).product::<f64>();
        (q, nc, c)
    }
    /// `lineage`'s spark scale C, and its inheritance chances for the targets `ks` only (into q[i] for i in ks, the
    /// other entries of q left as they are), without allocating: `slots`, `inherit` and `scale` operation for operation
    /// (each slot's affinity summed in `slots`' order, its factors multiplied in slot order, a slot without the spark
    /// skipped). More than four grandparent slots per parent: through `lineage` itself. `tg`: t's relation points with
    /// each legacy x of `p` (`pair`, at 5 x + 4) and with x's grandparent slots (`tri`, at 5 x + the slot), then
    /// `scale`'s factor per carrier count; built here when empty: the caller empties it for another trainee or pool.
    fn lineage_qc(&self, p: &Pool, t: usize, a: usize, b: usize, ks: &[usize], q: &mut [f64], tg: &mut Vec<f64>) -> f64 {
        const G: usize = 4;
        if p.gp[a].len() > G || p.gp[b].len() > G {
            let (qq, _, c) = self.lineage(p, t, a, b); for &i in ks { q[i] = qq[i]; } return c;
        }
        let ti = Some(t);
        let nl = p.me.len();
        if tg.is_empty() {
            tg.resize(nl * (G + 1), 0.0);
            for x in 0..nl { tg[x * (G + 1) + G] = self.pair(ti, p.me[x].ix); for (gi, g) in p.gp[x].iter().enumerate().take(G) { tg[x * (G + 1) + gi] = self.tri(ti, p.me[x].ix, g.ix); } }
            // then 1.1^(n - 6) per carrier count n, as `scale` computes it (black_box: the same runtime powi)
            tg.extend((0..=2 * (G + 1)).map(|n| 1.1f64.powi(std::hint::black_box(n as i32 - 6))));
        }
        let sh = 3.0 * shared(&p.me[a], &p.me[b]);
        // the six slots in `slots`' order: stars, mix, and the affinity term fa = 1 + (rel + race) / 100 (`inherit`'s,
        // computed once per slot), rel and race summed as `slots` sums them
        const NS: usize = 2 * (G + 1);
        let (mut st, mut mx, mut fa): ([&[u8]; NS], [Option<[f64; 3]>; NS], [f64; NS]) = ([&[][..]; NS], [None; NS], [0.0; NS]);
        let mut ns = 0usize;
        for (x, y) in [(a, b), (b, a)] {
            let px = &p.me[x];
            let mut rel = tg[x * (G + 1) + G] + self.pair(px.ix, p.me[y].ix); let mut race = sh;
            let i0 = ns; st[ns] = &px.stars[..]; mx[ns] = px.mix; ns += 1;
            for (gi, g) in p.gp[x].iter().enumerate() {
                let (gr, gc) = (tg[x * (G + 1) + gi], p.gpc[x][gi]);
                rel += gr; race += gc;
                st[ns] = &g.stars[..]; mx[ns] = g.mix; fa[ns] = 1.0 + (gr + gc) / 100.0; ns += 1;
            }
            fa[i0] = 1.0 + (rel + race) / 100.0;
        }
        let m = |s: f64, fa: f64| 1.0 - (0.03 * s * fa).min(1.0);
        for &k in ks {
            let (mut miss, mut mix2) = (1.0f64, 1.0f64);
            for j in 0..ns {
                let s = st[j][k]; if s == 0 { continue; }
                match mx[j] { None => miss *= m(s as f64, fa[j]), Some(w) => mix2 *= (0..3).map(|i| w[i] * m(i as f64 + 1.0, fa[j]).powi(2)).sum::<f64>() }
            }
            q[k] = 1.0 - miss * miss * mix2;
        }
        let pw = &tg[nl * (G + 1)..];
        self.uncv.iter().chain(&self.fixed_t).map(|&i| pw[st[..ns].iter().filter(|s| s[i] != 0).count()]).product::<f64>()
    }
    /// The ceiling of trainee t at chances q (her inheritance folded in here) with spark scale c: EXACT_RHO x C x the
    /// product value at each target's best possible deck odds (None: no legal deck odds for her).
    fn ceiling(&mut self, t: usize, q: &[f64], c: f64) -> Option<f64> {
        self.dmax[t].as_ref()?;
        let (rho, fixed) = (self.space.rho_top(), self.model.fixed_factor());
        Some(self.ceiling_at(t, q, c, rho, fixed))
    }
    /// `ceiling` for a trainee with legal deck odds, EXACT_RHO and the fixed gold factor given (the same values)
    #[inline]
    fn ceiling_at(&mut self, t: usize, q: &[f64], c: f64, rho: f64, fixed: f64) -> f64 {
        self.set_trainee(t, q);
        let (dw, dg) = self.dmax[t].as_ref().unwrap();
        let pv = (0..self.model.n_outfits(t)).map(|o| self.model.outfit_fast(t, o, 0.0, dw, dg)).fold(0.0f64, f64::max);
        rho * c * fixed * pv
    }
    /// `solve`'s candidates: per trainee of `trainees` with legal deck odds (in order; `tp` her position), per legal pair
    /// a < b of `p` (all of `fix` among them, at most one borrowed, neither her character), `mk(ceiling, tp, t, a, b)`,
    /// in that order. The numbers are `lineage` then `ceiling`'s, operation for operation, without the allocations and
    /// with what does not depend on the trainee computed once per pair (the parents' race parts and relation points
    /// between them, the carrier counts and so the spark scale c) or once per trainee and legacy (her relation points
    /// with it, its grandparent slots' miss factors). A slot without a spark of target k multiplies by 1 where
    /// `inherit` skips it (exact), and every model state `ceiling` leaves (`set_trainee` at each candidate's q in
    /// turn) is the same.
    fn gen_cands<C>(&mut self, p: &Pool, fix: &[usize], trainees: &[usize], mut mk: impl FnMut(f64, usize, usize, usize, usize) -> C) -> Vec<C> {
        let d = self.d; let nl = p.me.len(); let ntar = self.ntar;
        // (`inherit`'s miss factor, with its affinity term fa = 1 + A / 100 of the slot computed once)
        #[inline(always)]
        fn m(st: f64, fa: f64) -> f64 { 1.0 - (0.03 * st * fa).min(1.0) }
        // a slot's (miss, mix) factors of target k: `inherit`'s, 1 for the one it leaves alone
        #[inline(always)]
        fn fac(st: u8, mix: Option<[f64; 3]>, fa: f64) -> (f64, f64) {
            if st == 0 { return (1.0, 1.0); }
            match mix { None => (m(st as f64, fa), 1.0), Some(w) => (1.0, (0..3).map(|i| w[i] * m(i as f64 + 1.0, fa).powi(2)).sum::<f64>()) }
        }
        // 1.1^(n - 6) per carrier count n, as `scale` computes it (black_box: the same runtime powi, never folded)
        let maxs = (0..nl).map(|x| 1 + p.gp[x].len()).max().unwrap_or(0) * 2;
        let pw: Vec<f64> = (0..=maxs).map(|n| 1.1f64.powi(std::hint::black_box(n as i32 - 6))).collect();
        struct Pre { a: usize, b: usize, c: f64, race: [f64; 2], pp: [f64; 2] }
        let mut pairs: Vec<Pre> = vec![];
        // per legacy and target, how many of her slots (her own, her parents') carry the spark
        let cnt: Vec<usize> = (0..nl).flat_map(|x| (0..ntar).map(move |k| std::iter::once(&p.me[x]).chain(&p.gp[x]).filter(|w| w.stars[k] != 0).count())).collect();
        let mut nc = vec![0usize; ntar];
        // the pairs a < b in (a, b) order: with a fixed legacy f, only those holding it, (a, f) for a < f then (f, b)
        let pair_list: Box<dyn Iterator<Item = (usize, usize)>> = match fix.first() {
            None => Box::new((0..nl).flat_map(move |a| (a + 1..nl).map(move |b| (a, b)))),
            Some(&f) if f < nl => Box::new((0..f).map(move |a| (a, f)).chain((f + 1..nl).map(move |b| (f, b)))),
            Some(_) => Box::new(std::iter::empty()),
        };
        for (a, b) in pair_list {
            let (ca, cb) = (p.me[a].chara, p.me[b].chara);
            if ca == cb || (p.borrowed[a] && p.borrowed[b]) || !fix.iter().all(|&f| f == a || f == b) { continue; }
            let sh = 3.0 * shared(&p.me[a], &p.me[b]);
            let race = [a, b].map(|x| { let mut r = sh; for &gc in &p.gpc[x] { r += gc; } r });
            let pp = [self.pair(p.me[a].ix, p.me[b].ix), self.pair(p.me[b].ix, p.me[a].ix)];
            for k in 0..ntar { nc[k] = cnt[a * ntar + k] + cnt[b * ntar + k]; }
            let c = self.uncv.iter().map(|&i| pw[nc[i]]).chain(self.fixed_t.iter().map(|&i| pw[nc[i]])).product::<f64>();
            pairs.push(Pre { a, b, c, race, pp });
        }
        let mut out: Vec<C> = Vec::with_capacity(pairs.len() * trainees.iter().filter(|&&t| self.dmax[t].is_some()).count());
        let (rho, fixed) = (self.space.rho_top(), self.model.fixed_factor());
        let mut q = vec![0.0f64; ntar];
        // per legacy x: her relation points with the trainee, and per grandparent slot its relation points and its
        // factors per target (flat, from `go[x]`)
        let mut pt = vec![0.0f64; nl];
        let go: Vec<usize> = std::iter::once(0).chain((0..nl).scan(0, |s, x| { *s += p.gp[x].len(); Some(*s) })).collect();
        let mut gr = vec![0.0f64; go[nl]];
        let mut gf = vec![(1.0f64, 1.0f64); go[nl] * ntar];
        // per outfit and uncertain target, `outfit_fast`'s factor as Fixed (no chance in it) or Open(1 - e, 1 - dw, dg GOLD,
        // 1 - dg): the same subexpressions, each computed once for her
        enum Fac { Fixed(f64), Open(f64, f64, f64, f64) }
        // what one trainee's pair loop reads
        struct Cx<'c> { p: &'c Pool, pairs: &'c [Pre], tid: u32, ntar: usize, pt: &'c [f64], go: &'c [usize], gr: &'c [f64],
                        gf: &'c [(f64, f64)], ft: &'c [Vec<Fac>], uncv: &'c [usize], gw: f64, rho: f64, fixed: f64, tp: usize, t: usize }
        // One trainee's candidates, pair by pair; true when it made one (q then holds the last one's chances). Its own
        // function, entered once per trainee: a wasm call keeps the code it started in (V8 has no on-stack replacement
        // for wasm), so a loop over every trainee in one frame would run in baseline code throughout (docs/PERF.md).
        #[inline(never)]
        fn trainee_cands<C>(cx: &Cx, q: &mut [f64], nq: &mut [f64], out: &mut Vec<C>, mk: &mut impl FnMut(f64, usize, usize, usize, usize) -> C) -> bool {
            let (p, go, gw, ntar) = (cx.p, cx.go, cx.gw, cx.ntar);
            let mut last = false;
            for pr in cx.pairs {
                let (a, b) = (pr.a, pr.b);
                if p.me[a].chara == cx.tid || p.me[b].chara == cx.tid { continue; }
                // each parent slot's affinity, summed as `slots` sums it (as fa)
                let fa = [0, 1].map(|s| {
                    let x = if s == 0 { a } else { b };
                    let mut rel = cx.pt[x] + pr.pp[s];
                    for j in go[x]..go[x + 1] { rel += cx.gr[j]; }
                    1.0 + (rel + pr.race[s]) / 100.0
                });
                for k in 0..ntar {
                    let (mut miss, mut mix2) = (1.0f64, 1.0f64);
                    for (s, x) in [a, b].into_iter().enumerate() {
                        let (fm, fx) = fac(p.me[x].stars[k], p.me[x].mix, fa[s]); miss *= fm; mix2 *= fx;
                        for j in go[x]..go[x + 1] { let (fm, fx) = cx.gf[j * ntar + k]; miss *= fm; mix2 *= fx; }
                    }
                    q[k] = 1.0 - miss * miss * mix2;
                }
                // `ceiling_at` without writing the model state: per outfit `outfit_fast` at q = 0 on the hint chances
                // `set_trainee` would write (each factor (1 - q) = 1 dropped: x 1 is exact), the best outfit as its fold
                // (an open factor: ew = 1 - (1 - e)(1 - q_k), white = GEN_WHITE (1 - (1 - dw)(1 - ew)), dg GOLD + (1 - dg) white)
                for k in 0..nq.len() { nq[k] = 1.0 - q[cx.uncv[k]]; }
                let mut pv = 0.0f64;
                for fo in cx.ft {
                    let mut po = 1.0f64;
                    for (k, f) in fo.iter().enumerate() {
                        po *= match *f { Fac::Fixed(x) => x, Fac::Open(ne, ndw, a, nd) => { let ew = 1.0 - ne * nq[k]; a + nd * (gw * (1.0 - ndw * (1.0 - ew))) } };
                    }
                    pv = f64::max(pv, po);
                }
                last = true;
                out.push(mk(cx.rho * pr.c * cx.fixed * pv, cx.tp, cx.t, a, b));
            }
            last
        }
        for (tp, &t) in trainees.iter().enumerate() {
            if self.dmax[t].is_none() { continue; }
            let (ti, tid) = (Some(t), d.umas[t].id);
            for x in 0..nl {
                let px = &p.me[x];
                pt[x] = self.pair(ti, px.ix);
                for (gi, g) in p.gp[x].iter().enumerate() {
                    let j = go[x] + gi;
                    gr[j] = self.tri(ti, px.ix, g.ix);
                    let fa = 1.0 + (gr[j] + p.gpc[x][gi]) / 100.0;
                    for k in 0..ntar { gf[j * ntar + k] = fac(g.stars[k], g.mix, fa); }
                }
            }
            let (dw, dg) = self.dmax[t].as_ref().unwrap();
            debug_assert!(self.sts0[t].len() == self.model.n_outfits(t));
            let (gg, gw) = (crate::skills::GEN_GOLD, crate::skills::GEN_WHITE);
            let ft: Vec<Vec<Fac>> = self.sts0[t].iter().zip(&self.ew0[t]).map(|(st, e0)| (0..st.len()).map(|k| match st[k] {
                2 => Fac::Fixed(gg), 1 => Fac::Fixed(dg[k] * gg + (1.0 - dg[k]) * gw),
                _ => Fac::Open(1.0 - e0[k], 1.0 - dw[k], dg[k] * gg, 1.0 - dg[k]) }).collect()).collect();
            let mut nq = vec![0.0f64; ft.iter().map(|f| f.len()).max().unwrap_or(0)];
            let cx = Cx { p, pairs: &pairs, tid, ntar, pt: &pt, go: &go, gr: &gr, gf: &gf, ft: &ft, uncv: &self.uncv, gw, rho, fixed, tp, t };
            let last = trainee_cands(&cx, &mut q, &mut nq, &mut out, &mut mk);
            // the model state the last candidate's `ceiling` left (q still holds its chances)
            if last { self.set_trainee(t, &q); }
        }
        out
    }
    /// Trainee t's best deck at the chances already folded in (`set_trainee`), as the per-uma search finds it: the best
    /// product deck, then every deck whose product value times its class's cap (`cap_class`; at most EXACT_RHO, the top
    /// cap) reaches the best exact value, scored exactly (`upper_na` filters). `c` the spark scale, `th` the value (C x E)
    /// the career must beat: below it the search stops after the product deck. `starts`: decks scored first (a higher
    /// start leaves fewer decks to check). Counts the exact scorings in `scored`, its work in `self.work`; `full` is
    /// false when the work budget ran out or the band was certified below the top cap only.
    ///
    /// A band of more than EXACT_CAP decks is not scored: the band is built again at the next lower cap of the ladder
    /// (EXACT_STEPS, then 1: decks whose product value reaches the best exact one; each class at most that), the first
    /// that fits is scored, its cap is noted in `cert` (the answer is exact under it; 0 when even the last overflows:
    /// the best of the product deck and the starts, certified under nothing) and the call is not `full` (no Ref: a
    /// Ref is a proof under the top cap, kept by later searches and jobs). Without an overflow nothing changes.
    ///
    /// Memoised when there are no start decks (`deck_memo`): the call is then a function of what `DeckKey` holds, the
    /// work budget aside, and the budget never bound a call whose work stayed within it (work only grows during a
    /// call, and every budget test is `work <= budget` or `work > budget`), so a stored call that ended at work w
    /// answers again under any budget >= w. Classmates at the same chances (a sample pool of mob veterans gives every
    /// trainee the same chances: no relation points) then search once per class; a hit adds the same work and
    /// scorings and leaves the same proof, so everything after it is as if the search had run (only the node and
    /// exact-scoring counters, and `max_ratio`, which the same decks already set, see nothing).
    fn best_deck(&mut self, t: usize, c: f64, th: f64, starts: &[&Sel], scored: &mut usize) -> DeckRes {
        if !starts.is_empty() { return self.best_deck_run(t, c, th, starts, scored); }
        let key = DeckKey { cls: self.cls[t], ews: self.model.ews[t].iter().flatten().map(|x| x.to_bits()).collect(), c: c.to_bits(),
                            th: if th <= 0.0 { 0 } else { th.to_bits() }, work: self.work };
        if let Some(m) = self.deck_memo.get_mut(&key) { if key.work + m.work <= self.budget {
            self.work += m.work; *scored += m.scored; self.last_pv = m.pv; self.cert = self.cert.min(m.cert); self.overflows += m.overflows;
            // first use in this job of an outcome an earlier job stored (`DECK_KEEP`): this job's own search would have
            // run here, so its node and exact-scoring counters and its per-class ratios count as if it had (a later use
            // in the same job leaves them, as the memo always has). A class's ratio and deck change only when the call's
            // own maximum is above the job's: `note_ratio` keeps the first deck strictly above, so the same as the run
            if m.job != self.job {
                m.job = self.job;
                self.space.nodes.set(self.space.nodes.get() + m.nodes);
                crate::deck::JCOUNT.with(|c| c.set(c.get() + m.joints));
                for k in 0..4 { if let Some((r, t2, sel)) = &m.ratio[k] { if *r > self.max_ratio[k] { self.max_ratio[k] = *r; self.max_deck[k] = Some((*t2, sel.clone())); } } }
            }
            let caps = self.space.caps();
            for k in 0..4 { if let Some((r, _, _)) = &m.ratio[k] { if *r > caps[k] * (1.0 + 1e-9) { self.cap_breach += 1; } } }
            return m.res.clone();
        } }
        let (w0, s0, c0, o0, n0, j0) = (self.work, *scored, std::mem::replace(&mut self.cert, f64::INFINITY), self.overflows, self.space.nodes.get(), crate::deck::JCOUNT.with(|c| c.get()));
        // (the call's own per-class maxima, from nothing, then merged into the job's: see the hit above)
        let (r0, d0) = (std::mem::replace(&mut self.max_ratio, [f64::NEG_INFINITY; 4]), std::mem::take(&mut self.max_deck));
        let res = self.best_deck_run(t, c, th, starts, scored);
        let cert = self.cert; self.cert = c0.min(cert);
        let (r1, d1) = (std::mem::replace(&mut self.max_ratio, r0), std::mem::replace(&mut self.max_deck, d0));
        let ratio: [Option<(f64, usize, Sel)>; 4] = std::array::from_fn(|k| d1[k].as_ref().map(|(t2, sel)| (r1[k], *t2, sel.clone())));
        for k in 0..4 { if let Some((r, t2, sel)) = &ratio[k] { if *r > self.max_ratio[k] { self.max_ratio[k] = *r; self.max_deck[k] = Some((*t2, sel.clone())); } } }
        if self.work <= self.budget {
            self.deck_memo.insert(key, DeckMemo { res: res.clone(), pv: self.last_pv, work: self.work - w0, scored: *scored - s0, cert, overflows: self.overflows - o0,
                nodes: self.space.nodes.get() - n0, joints: crate::deck::JCOUNT.with(|c| c.get()) - j0, ratio, job: self.job });
        }
        res
    }
    fn best_deck_run(&mut self, t: usize, c: f64, th: f64, starts: &[&Sel], scored: &mut usize) -> DeckRes {
        let ntar = self.ntar; let uid = self.d.umas[t].id;
        let n0 = self.space.nodes.get();
        // the product search starts at the bar the career must clear (a hair below the cut below): it prunes every
        // branch that cannot reach it and finds nothing when no deck can, and a deck that clears it is the one an
        // unseeded search finds (the seed is below the maximum, so the first deck reaching it is never pruned)
        let rho = self.space.rho_top();
        let seed = if th.is_finite() && th > 0.0 { th / (rho * c * (1.0 + 1e-9)) * (1.0 - 1e-12) } else { f64::NEG_INFINITY };
        let (r, over) = self.space.search(&self.model, &[t], &[uid], &[(vec![(0, 0.0)], 0.0)], true, seed, 1.0, 1);
        self.work += 2 * (self.space.nodes.get() - n0) + 10_000;
        // what it proves (`ruled`): the deck found is the best (the first deck above the seed the search keeps is only
        // ever replaced by a better one, and a branch is pruned only below the best so far); none found, every deck is
        // at most the seed. Nothing when the search gave up (`over`, more than 128 card characters).
        self.last_pv = if over { None } else if let Some(x) = r.first() { Some(x.0) } else if seed > f64::NEG_INFINITY { Some(seed) } else { None };
        let Some((pv, sel)) = r.into_iter().next() else { return if seed > f64::NEG_INFINITY { DeckRes::Bounded } else { DeckRes::NoDeck } };
        // the candidate's own ceiling from its best product deck
        if rho * c * pv * (1.0 + 1e-9) < th { return DeckRes::Bounded; }
        let score = |env: &Env, sel: &Sel| joint_b(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler, ntar, false)[0].0;
        let mut best = (score(self, &sel), sel); *scored += 1; self.work += 200;
        if pv > 0.0 { let k = self.space.sel_class(&best.1); self.note_ratio(k, best.0 / pv, t, &best.1); }
        for s in starts { let x = score(self, s); *scored += 1; self.work += 200; if x > best.0 { best = (x, (*s).clone()); } }
        let mut full = false;
        // the cap ladder below the top cap (a band over EXACT_CAP decks is retried at the next one)
        let mut ladder = EXACT_STEPS[1..].iter().copied().chain(std::iter::once(1.0)).filter(|&x| x < rho);
        let mut rho = rho;
        while self.work <= self.budget {
            let n0 = self.space.nodes.get();
            crate::deck::SB_ON.with(|c| c.set(true)); crate::deck::SB_USED.with(|c| c.set(0));
            let (mut more, over) = self.space.search(&self.model, &[t], &[uid], &[(vec![(0, 0.0)], best.0)], false, f64::NEG_INFINITY, rho, self.space.band_cap());
            crate::deck::SB_ON.with(|c| c.set(false));
            // the band built without the cap (SBOUND=2): every deck in it may beat the bar, none is cut by rho
            let nocap = crate::deck::SB_USED.with(|c| c.get()) == 2;
            self.work += 2 * (self.space.nodes.get() - n0) + 5 * more.len() + 10_000;
            if over {
                self.overflows += 1;
                match ladder.next() { Some(x) if !nocap => { rho = x; continue; } _ => { self.cert = 0.0; break; } }
            }
            full = rho == self.space.rho_top();
            let mut cut = false;
            more.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            for (v, s) in more {
                if !nocap && rho * v * (1.0 + 1e-9) < best.0 { break; }
                if self.work > self.budget { full = false; cut = true; break; }
                if s.ents == best.1.ents && s.plain == best.1.plain && s.filler == best.1.filler { continue; }
                // below the bar at its own class's cap (the bar rose since the band was built)
                let k = self.space.sel_class(&s);
                if !nocap && self.space.caps()[k].min(rho) * v * (1.0 + 1e-9) < best.0 { continue; }
                let e: Vec<(&Src, Kind)> = s.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect();
                if let Some(ub) = upper_na(&self.model, t, &[0.0], &e, s.filler) { self.work += 25; if ub[0] * (1.0 + 1e-9) < best.0 { continue; } }
                let x = score(self, &s); *scored += 1; self.work += 200;
                if v > 0.0 { self.note_ratio(k, x / v, t, &s); }
                if x > best.0 { best = (x, s); }
            }
            if !cut { self.cert = self.cert.min(rho); }
            break;
        }
        DeckRes::Done(best.0, best.1, full)
    }

    /// The best careers: trainees from `trainees` (uma indices), two legacies of `p` as parents (all of `fix` among
    /// them, at most one borrowed), each with her best deck; `top` of them, one per trainee when `per_trainee`.
    /// Candidates are kept as (ceiling, trainee, parents) only (a pool of 400 veterans has millions); their chances
    /// are recomputed when one is solved (the same numbers, so the same results).
    fn solve(&mut self, p: &Pool, fix: &[usize], trainees: &[usize], per_trainee: bool, top: usize) -> Solved {
        let d = self.d;
        self.work = 0;   // each search has its own work budget
        let cert0 = std::mem::replace(&mut self.cert, f64::INFINITY);
        // (`tp`: the trainee's position in `trainees`; generated in (tp, a, b) order)
        struct Cand { ub: f64, t: u16, tp: u16, a: u16, b: u16 }
        let t_gen = now_ms();
        let nl = p.me.len();
        assert!(nl <= u16::MAX as usize && d.umas.len() <= u16::MAX as usize && trainees.len() <= u16::MAX as usize);
        let mut cands: Vec<Cand> = self.gen_cands(p, fix, trainees, |ub, tp, t, a, b| Cand { ub, t: t as u16, tp: tp as u16, a: a as u16, b: b as u16 });
        // Best ceiling first, ties in generation order: the order a stable sort by ceiling gives (partial_cmp, so 0 and
        // -0 tie as there), as a total order on (ceiling descending, tp, a, b), unique per candidate. Sorted lazily
        // (`upto`): the loop below reads a short prefix, so each step selects the next chunk (everything before it is
        // ordered before everything after it) and sorts only the chunk; the prefix is the full sort's, element for element.
        let ord = |x: &Cand, y: &Cand| y.ub.partial_cmp(&x.ub).unwrap().then(x.tp.cmp(&y.tp)).then(x.a.cmp(&y.a)).then(x.b.cmp(&y.b));
        let mut upto = 0usize;
        // cands[..upto] in final order; makes it reach past i
        let sort_to = |cands: &mut [Cand], i: usize, upto: &mut usize| {
            if i < *upto || *upto >= cands.len() { return; }
            let t0 = now_ms();
            while i >= *upto && *upto < cands.len() {
                let rest = &mut cands[*upto..];
                let m = (*upto).max(1024).min(rest.len());
                if m < rest.len() { rest.select_nth_unstable_by(m - 1, ord); }
                rest[..m].sort_unstable_by(ord);
                *upto += m;
            }
            prof("gen_ms", now_ms() - t0);
        };
        prof("gen_ms", now_ms() - t_gen); prof("gen_cands", cands.len() as f64); prof("solves", 1.0);
        let top = top.max(1);
        let mut complete = true;
        let breach0 = self.cap_breach;
        let mut done: Vec<(f64, usize, Sel)> = vec![];
        let floor = self.floor;
        let kth = |done: &[(f64, usize, Sel)]| if done.len() < top { floor } else { done[top - 1].0.max(floor) };
        let (mut n_solved, mut n_bounded, mut n_scored) = (0usize, 0usize, 0usize);
        let mut next = 0;
        // one result per trainee: a candidate only matters if it beats the k-th best and its trainee's best so far
        let thr = |done: &[(f64, usize, Sel)], cands: &[Cand], t: usize| if !per_trainee { kth(done) } else { done.iter().find(|x| cands[x.1].t as usize == t).map_or(kth(done), |x| x.0.max(kth(done))) };
        // per trainee, her fully checked candidates (`Ref`: chances, exact best value before the spark scale, deck)
        let mut refs: Vec<Vec<Ref>> = vec![vec![]; d.umas.len()];
        // per trainee, where this search's own entries in `ruled` start
        let ruled0: Vec<usize> = self.ruled.iter().map(|r| r.len()).collect();
        // dominated veterans (`dominates`): per legacy y, the legacies x at least as good in every career; and the
        // candidates this search took and did not solve (bounded, no deck)
        // (only veterans of one character can dominate each other: `dominates` tests the characters first, so the other
        // pairs are skipped unasked; each list is in ascending x as before)
        let mut by_chara: crate::FxMap<u32, Vec<usize>> = Default::default();
        for x in 0..nl { by_chara.entry(p.me[x].chara).or_default().push(x); }
        let dom: Vec<Vec<usize>> = (0..nl).map(|y| by_chara[&p.me[y].chara].iter().copied().filter(|&x| x != y && dominates(p, x, y)).collect()).collect();
        let prune = dom.iter().any(|v| !v.is_empty());
        let mut unsolved: crate::FxSet<(u16, u16, u16)> = Default::default();
        // Identical deck problems, searched once (`Twin`): a candidate whose trainee, chances q and spark scale c are
        // bit-identical to an earlier candidate's of this search (interchangeable parents: veterans with the same stars
        // and no relation or race points between them, as every sample pool of mob veterans has; 15 pairs of 6 such
        // veterans are one problem per trainee) takes that candidate's outcome instead of a second `best_deck`.
        // Derived, per outcome of the first (F, at bar th1) for the later one (R, at bar th2 >= th1: the bar only rises):
        // - Done(v, D) fully checked: F's Ref (q, v, D) is in refs[t], so R reaches the deck search only with
        //   th2 <= c v (1 + 1e-9) (the Ref bound above, rise = 1 at equal q). R's product search then starts below the
        //   best product value pv (th2 / (rho c (1 + 1e-9)) <= v / rho <= pv under the cap every search assumes) and finds
        //   F's product deck P, so the same `ruled` proof (q, pv); R's first exact pass scores P, then its starts, which
        //   hold F's (D among them), so its best is at least v from the same decks in the same order up to D; its band
        //   (decks with rho x product >= its best) is within F's, every deck of which F scored or ruled out against a
        //   lower best, and the starts F did not have are decks legal at q, at most v (F's search is exact under the cap).
        //   So R is Done(v, D) with the same floats (the same model state, `set_trainee` at the same q); a tie or lower,
        //   it goes into `done` exactly where R's own search would have put it.
        // - Bounded at th1: no deck above F's seed, or the best product deck below th1 / (rho c (1 + 1e-9)); a higher bar
        //   gives a higher seed, so R is Bounded too (no `ruled` entry: R's would only repeat what F's proves).
        // - NoDeck (no legal deck at all, at any bar): R the same.
        // R's search would also have spent work, so a work budget can now reach further than before (the answer the
        // unbudgeted search gives); outputs are otherwise the same, only the work counters fall.
        enum Twin { Done(f64, Sel, Option<f64>), Bounded(f64), NoDeck }
        let mut twins: crate::FxMap<(u16, Vec<u64>, u64), Twin> = Default::default();
        // a candidate whose ceiling is 0 cannot land every target (a target no legacy carries and nothing else gives):
        // nothing after it can either (sorted), and with every value 0 the bar would never rise, so every career would
        // be solved with unbounded deck searches (the page hung); stop there, leaving such careers out
        while next < cands.len() && { sort_to(&mut cands, next, &mut upto); cands[next].ub > 0.0 } && cands[next].ub * (1.0 + 1e-9) >= kth(&done) {
            if self.hard > 0 && self.work > self.hard { complete = false; break; }
            let ci = next; next += 1;
            let (t, a, b) = (cands[ci].t as usize, cands[ci].a as usize, cands[ci].b as usize);
            let th = thr(&done, &cands, t);
            if prune { unsolved.insert((cands[ci].t, cands[ci].a, cands[ci].b)); }
            if cands[ci].ub * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
            // A candidate whose parent has a dominating veteran x, where (t, x, other parent) was taken earlier and not
            // solved, would end unsolved too: that one was ruled out by its ceiling, a Ref's or a product search's bound,
            // a search that found no deck clearing the bar, or (in turn) this test, at a bar no higher than now (the bar
            // only rises), and each of those bounds is at most as high for this candidate (`dominates`: q, C and every
            // deck's value no higher), so it fails the same test (no deck at all: none here either). So no Ref, no
            // result, the same counters bar `work`; exact for any `top` and either `per_trainee`, and it uses nothing but
            // this search's own candidates. A taken-and-solved dominator proves nothing of the kind (this one could still
            // come out solved, a Ref the later searches start from), so only unsolved ones count.
            if prune {
                let key = |x: usize, y: usize| (t as u16, x.min(y) as u16, x.max(y) as u16);
                if [(a, b), (b, a)].iter().any(|&(x, y)| dom[x].iter().any(|&v| v != y && unsolved.contains(&key(v, y)))) { n_bounded += 1; continue; }
            }
            let (q, _, c) = self.lineage(p, t, a, b);
            let ct = self.cls[t]; let seen: &[Ref] = if self.use_seen { &self.seen[ct] } else { &[] };
            // (the lowest of these bounds below th, as any one of them below it: the minimum is one of the bounds, and
            // rounding x (1 + 1e-9) is monotone; a NaN bound neither lowers the minimum nor passes the test)
            if refs[t].iter().chain(seen).any(|r| c * r.val * self.rise(t, &q, &r.q) * (1.0 + 1e-9) < th) { n_bounded += 1; continue; }
            // what her earlier product searches proved (`ruled`). Derived: with her chances folded in, target k's factor
            // in the product value of one outfit and one deck (odds dw, dg) is f(n) = A - B n, n = (1 - e)(1 - q_k),
            // A = dg G + (1 - dg) W, B = (1 - dg) W (1 - dw) <= A (`outfit_fast` at q = 0; a target the outfit has is
            // q-free). From q to q' with q'_k > q_k: n' < n, and A >= B gives f(n') / f(n) <= (1 - n') / (1 - n), the
            // factor `rise` takes ((A - B n')(1 - n) <= (A - B n)(1 - n') <=> A (n' - n) <= B (n' - n)); with q'_k <= q_k
            // the factor does not grow. So per outfit the product at q' is at most the product at q times that outfit's
            // ratio product, the best outfit at q' at most S x rise(t, q', q) for every deck, and the fixed gold factor
            // and the spark scale c' multiply both sides alike. Then the search below would end `Bounded` on either path:
            // no deck above its seed th / (EXACT_RHO c' (1 + 1e-9)), or its best deck pv' with EXACT_RHO c' pv' (1 + 1e-9)
            // < th. The slack 2e-9 covers the float rounding of pv' and rise (about 1e-15 each). A skipped candidate is
            // one that would have been `Bounded`: no Ref, no result, the same counters bar `work` (less of it, so a search
            // that ran out of its budget before can now get further; none of the checked jobs comes near it). A search whose
            // decks seed the simulator's cache (`use_seen` off) takes only the proofs of its own candidates, so its work does not
            // depend on what earlier searches did either.
            let lo = if self.use_seen { 0 } else { ruled0[ct] };
            if self.ruled[ct][lo..].iter().any(|(rq, s)| self.space.rho_top() * c * s * self.rise(t, &q, rq) * (1.0 + 2e-9) < th) { n_bounded += 1; continue; }
            self.set_trainee(t, &q);
            let tk = (cands[ci].t, q.iter().map(|x| x.to_bits()).collect::<Vec<u64>>(), c.to_bits());
            let res = match twins.get(&tk) {
                Some(Twin::Done(v, sel, pv)) => { if let Some(s) = pv { self.ruled[ct].push((q.clone(), *s)); } DeckRes::Done(*v, sel.clone(), true) }
                Some(Twin::Bounded(th1)) if th >= *th1 => DeckRes::Bounded,
                Some(Twin::NoDeck) => DeckRes::NoDeck,
                _ => {
                    let starts: Vec<&Sel> = refs[t].iter().map(|r| &r.sel).collect();
                    let res = self.best_deck(t, c, th, &starts, &mut n_scored);
                    let pv = self.last_pv.take();
                    if let Some(s) = pv { self.ruled[ct].push((q.clone(), s)); }
                    match &res {
                        DeckRes::Done(v, sel, true) => { twins.insert(tk, Twin::Done(*v, sel.clone(), pv)); }
                        DeckRes::Done(..) => {}
                        DeckRes::Bounded => { twins.insert(tk, Twin::Bounded(th)); }
                        DeckRes::NoDeck => { twins.insert(tk, Twin::NoDeck); }
                    }
                    res
                }
            };
            let (val, sel, full) = match res {
                DeckRes::NoDeck => continue,
                DeckRes::Bounded => { n_bounded += 1; continue }
                DeckRes::Done(v, sel, full) => (v, sel, full),
            };
            n_solved += 1;
            if prune { unsolved.remove(&(cands[ci].t, cands[ci].a, cands[ci].b)); }
            if full { refs[t].push(Ref { q, val, sel: sel.clone() }); } else { complete = false; }
            let val = c * val;
            if per_trainee { if let Some(i) = done.iter().position(|x| cands[x.1].t as usize == t) { if done[i].0 >= val { continue; } done.remove(i); } }
            let pos = done.iter().position(|x| x.0 < val).unwrap_or(done.len());
            done.insert(pos, (val, ci, sel));
        }
        for (t, rs) in refs.iter().enumerate() { let ct = self.cls[t]; for r in rs { if !self.seen[ct].iter().any(|x| x.val == r.val && x.q == r.q) { self.seen[ct].push(r.clone()); } } }
        let next_bound = if next < cands.len() { sort_to(&mut cands, next, &mut upto); cands[next].ub } else { 0.0 };
        if self.cap_breach > breach0 { complete = false; }
        let proven = complete && !done.is_empty() && (done.len() >= top.min(cands.len()) || next >= cands.len()) && next_bound < done[0].0;
        let n = cands.len();
        let cert = self.cert; self.cert = cert0.min(cert);
        Solved { found: done.into_iter().take(top).map(|(val, ci, sel)| Found { t: cands[ci].t as usize, a: cands[ci].a as usize, b: cands[ci].b as usize, val, ub: cands[ci].ub, sel }).collect(),
                 refs, cands: n, solved: n_solved, bounded: n_bounded, scored: n_scored, skipped: n - next, next_bound, complete, proven, work: self.work, cert }
    }

    /// Trainee t's race schedule with parents a and b: every G1 win counts one (a G1 on her record can be shared with a
    /// later partner), plus one per parent who won that race (a win she shares with a parent adds 3 affinity to each of
    /// them when she is a parent herself), so she runs the G1s that win the most and, where two clash, the shared one.
    /// Rows: every G1 she could enter, in turn order; `run` false for those she skips, with what she does that turn.
    /// Also her win chance per race index.
    /// Trainee t's aptitudes over the career with parents a, b (MODEL.md, aptitude sparks): at the start, per aptitude
    /// the six legacies' pink stars summed raise her base grade 1/2/3/4 grades at 1/4/7/10 stars, at most to A; at each
    /// April inspiration (Classic, Senior) every legacy's pink spark fires with base 1/3/5% by stars x (1 + that legacy's
    /// affinity / 100), each firing +1 grade, at most S. None when data.json has no aptitudes.
    fn pink_career(&self, p: &Pool, t: usize, a: usize, b: usize) -> Option<PinkCareer> {
        let apt = &self.d.umas[t].apt; if apt.len() < 10 { return None; }
        let mut base = [0u8; 10]; base.copy_from_slice(&apt[..10]);
        let mut sum = [0u32; 10]; let mut fire = vec![];
        for (si, sl) in self.slots(p, t, a, b).iter().enumerate() {
            for &(g, st) in sl.pinks.iter() {
                let (Some(k), st) = (pink_apt(g), st.min(3)) else { continue }; if st == 0 { continue; }
                sum[k] += st as u32;
                fire.push(PinkFire { apt: k, p: ([0.0, 0.01, 0.03, 0.05][st as usize] * (1.0 + (sl.rel + sl.race) / 100.0)).min(1.0), slot: si, stars: st });
            }
        }
        let mut start = base;
        for k in 0..10 { let up = match sum[k] { 0 => 0, 1..=3 => 1, 4..=6 => 2, 7..=9 => 3, _ => 4 }; if base[k] < 7 { start[k] = (base[k] + up).min(7); } }
        Some(PinkCareer { base, start, fire })
    }
    /// Trainee t's race table with her pink sparks counted (`pink_career`): per G1 option the win chance per streak
    /// position, the expectation over her distance and surface grades at that turn (before Classic April: the start
    /// grades; then after one inspiration; from Senior April after both). None when nothing changes (no aptitude data,
    /// or her grades and the base data agree at every turn).
    /// Also per turn and option her expected aptitude score (110 - grade costs), as the exporter's bp.
    fn pink_uma(&self, pc: &PinkCareer, t: usize) -> Option<(Uma, Vec<Vec<f64>>)> {
        let d = self.d; if d.race_cat.len() < d.nr || d.race_ground.len() < d.nr { return None; }
        let u = &d.umas[t];
        // per aptitude, the grade distribution after 0, 1 and 2 inspirations
        let dist: Vec<[Vec<(u8, f64)>; 3]> = (0..10).map(|k| {
            let ps: Vec<f64> = pc.fire.iter().filter(|f| f.apt == k).map(|f| f.p).collect();
            let one = pbinom(&ps); let two = conv(&one, &one);
            let g = |x: &[f64]| { let mut m: Vec<(u8, f64)> = vec![]; for (i, &w) in x.iter().enumerate() { if w <= 0.0 { continue; }
                let gr = (pc.start[k] as usize + i).min(8) as u8; if let Some(e) = m.iter_mut().find(|e| e.0 == gr) { e.1 += w; } else { m.push((gr, w)); } } m };
            [vec![(pc.start[k], 1.0)], g(&one), g(&two)]
        }).collect();
        if pc.start == pc.base && pc.fire.iter().all(|f| f.apt >= 6) { return None; }
        let cost = |gr: u8| -> i32 { [90, 90, 60, 40, 30, 20, 10, 0, 0][gr as usize] };
        let mut v = u.clone(); let mut ebp: Vec<Vec<f64>> = u.opts.iter().map(|l| vec![0.0; l.len()]).collect();
        for tt in 0..NT {
            let ph = INSP_TURNS.iter().filter(|&&x| tt >= x).count();
            for (o, op) in u.opts[tt].iter().enumerate() {
                if op.race < 0 { continue; }
                let r = op.race as usize; let (kd, ks) = (d.race_cat[r] as usize, if d.race_ground[r] == 1 { 4 } else { 5 });
                let mut pr = [0.0f64; 7];
                for &(gd, wd) in &dist[kd][ph] { for &(gs, ws) in &dist[ks][ph] {
                    let bp = (110 - cost(gd) - cost(gs)).max(-20);
                    ebp[tt][o] += wd * ws * bp as f64;
                    for k in 1..=6 { pr[k] += wd * ws * pk(d, bp, k); } } }
                v.ent[tt][o + 1].p = pr;
            }
        }
        Some((v, ebp))
    }
    /// The chance of each pink spark of her veteran (MODEL.md): one, its aptitude uniform over those she ends at A or S
    /// (stars 20/70/10, not here): per aptitude (data.json `apt` order) P(it is the one); exact, the aptitudes'
    /// inspirations independent. The rest: no aptitude at A or S.
    fn pink_gen(&self, pc: &PinkCareer) -> [f64; 10] {
        let q: Vec<f64> = (0..10).map(|k| {
            let ps: Vec<f64> = pc.fire.iter().filter(|f| f.apt == k).map(|f| f.p).collect();
            let one = pbinom(&ps); let two = conv(&one, &one);
            two.iter().enumerate().filter(|(i, _)| pc.start[k] as usize + i >= 7).map(|x| x.1).sum::<f64>()
        }).collect();
        let mut out = [0.0f64; 10];
        for k in 0..10 {
            let others: Vec<f64> = (0..10).filter(|&j| j != k).map(|j| q[j]).collect();
            let n = pbinom(&others);
            out[k] = q[k] * n.iter().enumerate().map(|(i, w)| w / (i + 1) as f64).sum::<f64>();
        }
        out
    }
    fn schedule(&self, p: &Pool, t: usize, a: usize, b: usize) -> (Value, Vec<f64>) {
        let d = self.d;
        let pu = self.pink_career(p, t, a, b).and_then(|pc| self.pink_uma(&pc, t));
        let u = pu.as_ref().map_or(&d.umas[t], |x| &x.0);
        let mut wr = vec![1.0f64; d.nr];
        for x in [a, b] { for &(r, pr) in &p.me[x].wins { if r < d.nr { wr[r] += pr; } } }
        // her own race table (pinks counted): solved directly (the memo is keyed by uma id)
        let br = if pu.is_some() { crate::best_response_solve(d, u, &wr, None) } else { best_response(d, u, &wr, None) };
        let Some((s, _)) = br else { return (json!([]), vec![0.0; d.nr]) };
        let wp = probs(d, u, &s);
        let won = |x: usize, r: usize| p.me[x].wins.iter().find(|w| w.0 == r).map_or(0.0, |w| w.1);
        let mut rows = vec![]; let mut k = 0usize;
        for tt in 0..NT {
            let chosen = match s[tt] { Choice::Rest => { k = 0; None } Choice::Opt(o) => { k = (k + 1).min(6); Some(o as usize) } };
            for (o, op) in u.opts[tt].iter().enumerate() {
                if op.race < 0 { continue; }
                let r = op.race as usize; let run = chosen == Some(o);
                let instead = match chosen { _ if run => Value::Null, None => json!("rest"), Some(c) => json!(u.opts[tt][c].name) };
                // the loop planner's chart row (one member: `cells`, `shared` = a parent won it, so it builds affinity),
                // plus `run`, `p`, `instead`, and `name` (the race's full name)
                let par = [won(a, r) > 0.0, won(b, r) > 0.0];
                // win chance and aptitude cost (distance + surface grade costs, as 110 - bp; expected) with her pinks
                let pw = u.ent[tt][o + 1].p;
                let apt = pu.as_ref().map_or((110 - op.bp) as f64, |x| 110.0 - x.1[tt][o]).round() as i64;
                let cell = if run { json!({"p": (pw[k] * 100.0).round(), "goal": op.goal, "apt": apt, "row": k, "pen": d.pen.get(k.max(1) - 1).copied().unwrap_or(0)}) } else { Value::Null };
                rows.push(json!({"turn": tt, "when": when(tt), "year": tt / 24, "race": chart_race_name(&d.race_names[r]), "name": d.race_names[r],
                                 "run": run, "skipped": !run, "instead": instead, "p": if run { json!(pw[k]) } else { Value::Null }, "goal": op.goal,
                                 "parents": par, "shared": par[0] || par[1], "cells": [cell], "runners": run as u8, "alts": []}));
            }
        }
        (Value::Array(rows), wp)
    }

    /// One career's report: trainee and outfit, parents, the six legacy slots, per target odds, aptitudes, deck, schedule.
    fn report(&mut self, p: &Pool, f: &Found, with_schedule: bool) -> Value {
        let d = self.d; let ntar = self.ntar; let nu = self.nu;
        let name = |id: u32| d.umas.iter().find(|u| u.id == id).map_or(format!("Uma {id}"), |u| u.name.clone());
        let (t, a, b) = (f.t, f.a, f.b);
        let uid = d.umas[t].id;
        let sl = self.slots(p, t, a, b);
        let (q, nc) = inherit(&sl, ntar);
        self.set_trainee(t, &q);
        let (j, ext) = self.exact(t, &f.sel, &nc, true, with_schedule && ntar <= 12);
        let o = j.2;
        let mut pt = vec![0.0f64; ntar];
        for (pos, &i) in self.uncv.iter().enumerate() { pt[i] = ext.marg[o * nu + pos]; }
        for (pos, &i) in self.fixed_t.iter().enumerate() { pt[i] = GEN_GOLD * ext.sf[pos]; }
        let sj = self.sj;
        let (st, cards) = &self.model.opts[t][o];
        let covers: Vec<Value> = st.iter().zip(&self.uncv).filter(|(lv, i)| **lv > 0 && !(**lv == 1 && sj.targets[**i].guaranteed == 1))
            .map(|(lv, i)| json!({"group": sj.targets[*i].group, "level": lv})).collect();
        let default_st: Vec<u8> = self.uncv.iter().map(|&i| if sj.targets[i].guaranteed == 1 { 1 } else { 0 }).collect();
        let outfits = if *st == default_st { vec![] } else { cards.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() };
        let slots: Vec<Value> = sl.iter().map(|s| json!({"kind": if s.parent { "parent" } else { "grandparent" }, "legacy": s.legacy, "gp": s.gp,
            "chara": s.chara, "name": name(s.chara), "rel": s.rel, "race": s.race, "aff": s.rel + s.race, "stars": &*s.stars, "pinks": &*s.pinks})).collect();
        // per target, buying it and landing it (MODEL.md, skills): bought as gold (her outfit's gold, or a deck gold hint)
        // or white (her outfit's white; else not gold and hinted by the deck, her own events or inheritance), each landing
        // with GEN_GOLD / GEN_WHITE x her spark scale; the deck's hint chances are its per-target marginals (`sel.dw/dg`), so
        // these products match `p` up to the deck chains' correlation. The scenario's golds: bought, landing GEN_GOLD.
        let (su, sf) = self.scale(&nc);
        let ewo = &self.ew0[t][o];
        let buy = |i: usize| -> Value {
            if let Some(pos) = self.uncv.iter().position(|&x| x == i) {
                let (v, dw, dg, e) = (st[pos], f.sel.dw[pos], f.sel.dg[pos], ewo[pos]);
                let gold = if v == 2 { 1.0 } else { dg };
                let white = if v == 2 { 0.0 } else if v == 1 { 1.0 - dg } else { (1.0 - dg) * (1.0 - (1.0 - dw) * (1.0 - e) * (1.0 - q[i])) };
                json!({"gold": gold, "white": white, "land_gold": gold * crate::skills::GEN_GOLD * su[pos], "land_white": white * crate::skills::GEN_WHITE * su[pos],
                       "deck_gold": dg, "deck_white": dw, "own_event": e, "inherit": q[i], "outfit": v})
            } else {
                let pos = self.fixed_t.iter().position(|&x| x == i).unwrap_or(0);
                json!({"gold": 1.0, "white": 0.0, "land_gold": crate::skills::GEN_GOLD * sf.get(pos).copied().unwrap_or(1.0), "land_white": 0.0, "scenario": true})
            }
        };
        let targets: Vec<Value> = (0..ntar).map(|i| json!({"group": sj.targets[i].group, "q": q[i], "carriers": nc[i], "p": pt[i],
            "gold": sj.targets[i].guaranteed == 2, "buy": buy(i)})).collect();
        let pd = self.space.pdeck(&f.sel, &[uid]);
        // pink sparks: per aptitude the chance at least one of the two inheritance rolls passes it on (1/3/5% per star)
        let mut pg: Vec<u32> = sl.iter().flat_map(|s| s.pinks.iter().map(|x| x.0)).collect(); pg.sort(); pg.dedup();
        let pinks: Vec<Value> = pg.iter().map(|&g| {
            let (mut miss, mut n) = (1.0f64, 0);
            for s in &sl { if let Some(st) = s.pinks.iter().filter(|x| x.0 == g).map(|x| x.1.min(3)).max() { if st > 0 { n += 1;
                miss *= 1.0 - ([0.0, 0.01, 0.03, 0.05][st as usize] * (1.0 + (s.rel + s.race) / 100.0)).min(1.0); } } }
            json!({"group": g, "q": 1.0 - miss * miss, "carriers": n})
        }).collect();
        // her aptitudes and her veteran's pink spark (MODEL.md): base and start grades (data.json `apt` order, codes 1..8 =
        // G..S), each legacy pink's chance to fire at one April inspiration, and the chance of each pink type
        let apt = self.pink_career(p, t, a, b).map(|pc| {
            let pg8 = self.pink_gen(&pc);
            json!({"base": pc.base, "start": pc.start,
                   "fire": pc.fire.iter().map(|f| json!({"group": apt_pink(f.apt), "apt": f.apt, "p": f.p, "slot": f.slot, "stars": f.stars})).collect::<Vec<_>>(),
                   "pink_gen": (0..10).map(|k| json!({"group": apt_pink(k), "p": pg8[k]})).collect::<Vec<_>>()})
        });
        let outcomes = if with_schedule && ntar <= 12 { json!(self.outcome_sets(j.2, &ext)) } else { Value::Null };
        let (sched, wp) = if with_schedule { self.schedule(p, t, a, b) } else { (Value::Null, vec![]) };
        json!({"trainee": {"id": uid, "name": d.umas[t].name, "outfits": outfits, "covers": covers},
               "parents": [a, b], "slots": slots, "targets": targets, "p_all": j.0, "p_minus1": j.1, "expected": pt.iter().sum::<f64>(),
               "bound": f.ub, "deck": pd.out(&self.unc), "pinks": pinks, "aptitudes": apt, "schedule": sched, "outcome_sets": outcomes, "g1_wins": if with_schedule { json!(wp.iter().sum::<f64>()) } else { Value::Null }})
    }

    /// Corner table of trainee t with deck `sel`: per outfit, P(all) with every spark-generation scale 1 at the 2^nu
    /// points q_k = 1 - (1 - q0_k) x_k, x_k in {0, 1} (index bit k = uncertain target k at x_k = 1). Derived: q_k enters
    /// every landing factor only through (1 - e)(1 - q_k), affinely, and P(all) per outfit is a sum of products with
    /// one factor per target, so it is multilinear in x and `interp` at any x in [0, 1]^nu gives it exactly.
    /// `corners` at q0 = 0 (a cached deck's table, `Cache::add`), memoised by (deck class, entries, filler). Exact: the
    /// table reads trainee t only through her model (outfit coverage, own-skill chances: `set_trainee` from `ew0`), which
    /// her deck class holds to the bit, and the deck only through its entries and filler; the memo is kept between jobs
    /// of one setup (`DECK_KEEP`: the same model and entries). `corners` leaves her model at its last corner, q = 0 for
    /// every target (1 - (1 - 0) x 1); a hit sets the same state
    pub(super) fn corners_kept(&mut self, t: usize, sel: &Sel) -> Vec<Vec<f64>> {
        let key = (self.cls[t], sel.ents.clone(), sel.filler);
        let z = vec![0.0; self.ntar];
        if let Some(r) = self.corner_memo.get(&key) { let r = (**r).clone(); self.set_trainee(t, &z); return r; }
        let tc = now_ms(); let r = self.corners(t, sel, &z); prof("corner_ms", now_ms() - tc); prof("corner_builds", 1.0);
        self.corner_memo.insert(key, std::rc::Rc::new(r.clone()));
        r
    }
    fn corners(&mut self, t: usize, sel: &Sel, q0: &[f64]) -> Vec<Vec<f64>> {
        let (nu, ntar) = (self.nu, self.ntar);
        let no = self.model.n_outfits(t);
        let mut corners = vec![vec![0.0f64; 1 << nu]; no];
        // per corner `exact(t, sel, [6; ntar], false, false)`: the deck's part of `joint_fast` (`joint_prep`, which reads
        // no chance and no model state) once, its evaluation per corner (the same sums; the scale is 1 throughout)
        let (su, sf) = self.scale(&vec![6; ntar]);
        let ents: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect();
        let pre = joint_prep(self.model.uncertain.len(), &ents, sel.filler);
        for c in 0..(1usize << nu) {
            let mut q = q0.to_vec();
            for (k, &i) in self.uncv.iter().enumerate() { let x = if c >> k & 1 == 1 { 1.0 } else { 0.0 }; q[i] = 1.0 - (1.0 - q0[i]) * x; }
            set_ews(&mut self.model.ews[t], &self.ew0[t], &self.uncv, &q);
            let mut ext = JointExt { su: su.clone(), sf: sf.clone(), marg: vec![], sets: vec![], per_outfit: vec![] };
            joint_eval(&self.model, t, &[0.0], &pre, ntar, false, Some(&mut ext));
            for o in 0..no { corners[o][c] = ext.per_outfit[o]; }
        }
        corners
    }

    /// Joint distribution of full target sets, including guaranteed gold generation.
    fn outcome_sets(&self, outfit: usize, ext: &JointExt) -> Vec<f64> {
        let su = &ext.sets[(outfit << self.nu)..((outfit + 1) << self.nu)];
        let mut ps = vec![0.0f64; 1 << self.ntar];
        for (mu, &x) in su.iter().enumerate() {
            if x == 0.0 { continue; }
            let mut base = 0usize; for (k, &i) in self.uncv.iter().enumerate() { if mu >> k & 1 == 1 { base |= 1 << i; } }
            let mut part = vec![(base, x)];
            for (pos, &i) in self.fixed_t.iter().enumerate() { let g = GEN_GOLD * ext.sf[pos];
                part = part.into_iter().flat_map(|(m, w)| [(m, w * (1.0 - g)), (m | 1 << i, w * g)]).collect(); }
            for (m, w) in part { ps[m] += w; }
        }
        ps
    }

    /// The veteran career (t, a, b) leaves, before her result is known: trainee t, a spark of every target with stars
    /// drawn from the rank mix (`star_dist`), her schedule's win chances `wp`, parents a and b.
    fn veteran(&self, legs: &[Legacy], t: usize, a: usize, b: usize, wp: Vec<f64>) -> Legacy {
        let anc = |l: &Legacy| Ancestor { chara: l.chara, sparks: l.sparks.clone(), wins: l.wins.clone(), pinks: l.pinks.clone() };
        Legacy { chara: self.d.umas[t].id, borrowed: false, sparks: self.sj.targets.iter().map(|x| (x.group, 2)).collect(), wins: vec![], pinks: vec![],
            parents: vec![anc(&legs[a]), anc(&legs[b])], win_p: wp, mix: Some(self.sj.star_dist) }
    }
}

pub(crate) fn run_career(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    // the worker's kept work, out to the page and back in after a reload (`state_out` / `state_in`)
    match v["cmd"].as_str() { Some("state_out") => return state_out(), Some("state_in") => return state_in(&v["state"]), _ => {} }
    let sj: SkillJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let dj: DeckJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let cj: CareerJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    CHAIN_EVENTS.with(|c| c.set(dj.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let t_all = Instant::now();
    let model = Model::new(d, sk, &sj);
    let ew0 = model.ews.clone();
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let ntar = sj.targets.len();
    let uncv = model.uncertain.clone();
    let fixed_t: Vec<usize> = (0..ntar).filter(|i| !uncv.contains(i)).collect();
    let n = d.umas.len();
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let lock_blocked = |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).map_or(false, |c| c.blocks(d.umas[u].id)));
    let id_ok = |id: u32| (sj.owned.is_empty() || sj.owned.contains(&id)) && !sj.exclude.contains(&id);
    // every uma that can train (the plan's next career takes any of them); the job's trainees are among these
    let owned: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let trainees: Vec<usize> = owned.iter().copied().filter(|&i| cj.trainees.is_empty() || cj.trainees.contains(&d.umas[i].id)).collect();
    if trainees.is_empty() { return json!({"error": "No trainee to search: pick at least one owned uma."}); }
    let legs = &cj.legacies;
    if legs.len() < 2 { return json!({"error": "Add at least two veterans to choose parents from."}); }
    let pool_ids: Vec<u32> = trainees.iter().map(|&i| d.umas[i].id).collect();
    let ent = match deck_entries(cr, &dj, &unc, &pool_ids) { Ok(e) => e, Err(e) => return e };
    let space = deck_space(cr, &dj, sk, &unc, &ent);

    // ---- per uma: the best possible deck odds per target (a ceiling for every legal deck) ----
    let size = dj.deck_size;
    let strong: Vec<(Vec<f64>, Vec<f64>)> = (0..space.rel.len()).map(|j| {
        let (mut mw, mut mg) = (vec![1.0f64; nu], vec![1.0f64; nu]);
        src_miss(&space.srcs[j], &vec![1.0; space.kind[j].1 as usize], &mut mw, &mut mg);
        let mh: Vec<f64> = (0..nu).map(|k| mg[k] * mw[k]).collect();
        (mg, mh)
    }).collect();
    let dmax: Vec<Option<(Vec<f64>, Vec<f64>)>> = (0..n).map(|u| {
        if !owned.contains(&u) { return None; }
        let uid = d.umas[u].id;
        let legal: Vec<usize> = (0..space.rel.len()).filter(|&j| !space.cards[space.rel[j]].blocks(uid)).collect();
        let lowest = |f: &dyn Fn(usize) -> f64| { let mut m: Vec<f64> = legal.iter().map(|&j| f(j)).collect(); m.sort_by(|a, b| a.partial_cmp(b).unwrap()); m.iter().take(size).product::<f64>() };
        let (mut dw, mut dg) = (vec![0.0f64; nu], vec![0.0f64; nu]);
        for k in 0..nu {
            let (g, h) = (1.0 - lowest(&|j| strong[j].0[k]), 1.0 - lowest(&|j| strong[j].1[k]));
            // (g, h) = (P(gold hint), P(any hint)); the product formula takes dw with (1 - dw)(1 - dg) = 1 - h
            dg[k] = g; dw[k] = if g < 1.0 { (1.0 - (1.0 - h) / (1.0 - g)).max(0.0) } else { 1.0 };
        }
        Some((dw, dg))
    }).collect();
    let sts0: Vec<Vec<Vec<u8>>> = model.opts.iter().map(|m| m.iter().map(|x| x.0.clone()).collect()).collect();
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(CAREER_BUDGET);
    let rho_top = space.rho_top();
    // the kept `best_deck` memo of this setup (`DECK_KEEP`), and this job's number
    let memo_fp = { use std::hash::{Hash, Hasher}; let mut h = std::collections::hash_map::DefaultHasher::new(); setup_fp(v).hash(&mut h); pool_ids.hash(&mut h); h.finish() };
    let job = JOB_NO.with(|c| { c.set(c.get() + 1); c.get() });
    let kept = DECK_KEEP.with(|c| { let mut c = c.borrow_mut(); c.iter().position(|e| e.0 == memo_fp).map(|i| c.remove(i).1) }).unwrap_or_default();
    let deck_memo = if kept.decks.len() < DECK_KEEP_CAP { kept.decks } else { Default::default() };
    let corner_memo = if kept.corners.len() < DECK_KEEP_CAP { kept.corners } else { Default::default() };
    let mut env = Env { d, sj: &sj, model, ew0, uncv, fixed_t, ntar, nu, unc, space, dmax, sts0, owned, budget, work: 0, seen: vec![vec![]; n], use_seen: true, ruled: vec![vec![]; n], last_pv: None, floor: f64::NEG_INFINITY, hard: 0, max_ratio: [0.0; 4], cap_breach: 0, max_deck: Default::default(), cert: f64::INFINITY, overflows: 0, deck_memo, corner_memo, memo_fp, job, cls: (0..n).collect() };
    // deck classes: the key holds everything of a trainee the deck search reads (outfit coverage, own-skill chances to
    // the bit, `dmax`, the relevant cards she may not take, how many plain pals she may)
    {
        let mut first: crate::FxMap<Vec<u64>, usize> = Default::default();
        for u in 0..n {
            let uid = env.d.umas[u].id; let mut k: Vec<u64> = vec![];
            for o in &env.sts0[u] { k.push(o.len() as u64); k.extend(o.iter().map(|&x| x as u64)); }
            k.push(u64::MAX);
            for o in &env.ew0[u] { k.push(o.len() as u64); k.extend(o.iter().map(|x| x.to_bits())); }
            k.push(u64::MAX);
            match &env.dmax[u] { Some((w, g)) => { k.push(1); k.extend(w.iter().chain(g).map(|x| x.to_bits())); } None => k.push(0) }
            k.push(u64::MAX);
            k.extend((0..env.space.rel.len()).filter(|&j| env.space.cards[env.space.rel[j]].blocks(uid)).map(|j| j as u64));
            k.push(u64::MAX);
            k.push(env.space.legal_plain(&[uid]).len() as u64);
            env.cls[u] = *first.entry(k).or_insert(u);
        }
    }
    // what earlier jobs on this setup proved (the page's worker keeps it between jobs)
    let fp = setup_fp(v);
    if let Some(s) = SEEN.with(|c| c.borrow().iter().find(|e| e.0 == fp).map(|e| e.1.clone())) { if s.len() == n { env.seen = s; } }
    let keep = |env: &Env| SEEN.with(|c| {
        let mut c = c.borrow_mut(); c.retain(|e| e.0 != fp);
        c.push((fp, env.seen.iter().map(|rs| rs[rs.len().saturating_sub(SEEN_KEEP)..].to_vec()).collect()));
        if c.len() > 4 { c.remove(0); }
    });
    if let Some(sm) = &cj.simulate { return sim::simulate(&mut env, d, sk, cr, v, legs, &trainees, sm); }
    let pool = env.pool(legs);
    env.use_seen = true;
    // the page sends a career job, then the same job with `next`: the plan job reuses that job's main search (the solver
    // worker keeps the last one; a search is deterministic for its job, so the result is the one it would compute)
    let skey = { let mut k = v.clone(); if let Some(o) = k.as_object_mut() { o.remove("next"); o.remove("no_reports"); } setup_hash(&k.to_string()) };
    // the main search's own work (deck-search nodes, exact scorings; deterministic, comparable at any load): 0 when the
    // search was kept from the job before
    let (sn0, sj0) = (env.space.nodes.get(), crate::deck::JCOUNT.with(|c| c.get()));
    // (a kept search brings back what its product searches proved: `env.ruled` as a combined job's plan finds it)
    let kept_sv = LAST_SOLVE.with(|c| { let mut c = c.borrow_mut(); let i = c.iter().position(|x| x.0 == skey)?; let e = c.remove(i); c.push(e); c.last().map(|e| (e.1.clone(), e.2.clone())) });
    let sv = match kept_sv {
        Some((s, r)) if r.len() == n => { env.ruled = r; s }
        _ => { let s = env.solve(&pool, &cj.fix, &trainees, cj.per_trainee, cj.top);
               LAST_SOLVE.with(|c| { let mut c = c.borrow_mut(); c.retain(|x| x.0 != skey); c.push((skey, s.clone(), env.ruled.clone())); while c.len() > SOLVE_KEEP { c.remove(0); } }); s }
    };
    let search_work = json!({"nodes": env.space.nodes.get() - sn0, "exact": crate::deck::JCOUNT.with(|c| c.get()) - sj0});
    env.use_seen = true;
    if std::env::var("CAREER_WORK").is_ok() { eprintln!("career search: work {} of {}, solved {}, proven {}", sv.work, env.budget, sv.solved, sv.proven); }
    if sv.cands == 0 { keep(&env); return json!({"error": "No legal career: a trainee needs two veterans of other characters (not her own, not the same as each other), at most one of them borrowed."}); }
    // the race schedule only for the best career (the one the page shows; each takes a schedule search)
    let results: Vec<Value> = if cj.no_reports { vec![] } else { sv.found.iter().enumerate().map(|(i, f)| env.report(&pool, f, i == 0)).collect() };
    let mut out = json!({"results": results, "candidates": sv.cands, "solved": sv.solved, "bounded": sv.bounded, "scored": sv.scored, "skipped": sv.skipped,
           "next_bound": sv.next_bound, "complete": sv.complete, "proven_top": sv.proven, "cap": rho_top.min(sv.cert), "class_caps": env.space.caps(), "work": search_work});
    // a deck band over EXACT_CAP decks (in the main search or any later one of this job) was certified at a lower cap
    // only: `cap` says which, `complete` is false
    if sv.cert < rho_top { out["complete"] = json!(false); }
    if cj.compare { out["compare"] = json!(true); }
    if let Some(x) = &cj.next { if !sv.found.is_empty() {
        out["next"] = match serde_json::from_value::<sim::plan::PlanJob>(x.clone()) {
            Ok(nj) => sim::plan::index_rule(&mut env, d, sk, cr, v, legs, &pool, &sv, &cj.fix, &trainees, &nj),
            Err(e) => json!({"error": format!("next: {e}")}) };
    } }
    keep(&env);
    if env.overflows > 0 { out["complete"] = json!(false); out["cap"] = json!(rho_top.min(env.cert)); out["band_overflows"] = json!(env.overflows); }
    // a scored deck past its class's assumed cap (in any search of the job): per class [class, ratio, cap]
    if env.cap_breach > 0 { let caps = env.space.caps();
        out["cap_exceeded"] = json!((0..4).filter(|&k| env.max_ratio[k] > caps[k] * (1.0 + 1e-9)).map(|k| json!([k, env.max_ratio[k], caps[k]])).collect::<Vec<_>>()); }
    out["timing_s"] = json!(t_all.elapsed().as_secs_f64());
    out
}

