//! The page's next career (`next` on a career job, `index_rule`; docs/SOLVER.md "Next career"): the career that brings
//! a fully looped parent closest per run, scored exactly over every result she can roll, with each candidate's
//! continuation (`Cont`) fitted to the pool. It values careers through the simulator's deck cache (`Sim`, `Cache`).
use super::{
    Ancestor, Bar, CardsRaw, Cur, Data, DeckRes, Deserialize, Env, Found, FxMap, HerSlot, Legacy, PROF, Pool, Ref, Sel,
    Sim, SimJob, SkillJob, SkillsRaw, Solved, Value, holds_fix, json, now_ms, prof, progress, rise_factor,
};

/// A cached result may be available from searching, certification or reuse. A known empty result is
/// reusable too; an unsearched entry still needs work. Completeness is tracked separately.
#[derive(Clone)]
enum CachedSearch {
    Unsearched,
    Known(Option<Found>),
}

impl CachedSearch {
    const fn is_unsearched(&self) -> bool {
        matches!(self, Self::Unsearched)
    }

    /// The found career, if any. Callers deciding whether to search must use `is_unsearched` instead.
    const fn found(&self) -> Option<&Found> {
        match self {
            Self::Unsearched => None,
            Self::Known(found) => found.as_ref(),
        }
    }
}

/// Preserve the stored distinction: unsearched entries are absent, searched empty entries are explicit nulls.
fn cached_searches_out(searches: &[CachedSearch]) -> Vec<Value> {
    searches
        .iter()
        .enumerate()
        .filter_map(|(trainee, search)| match search {
            CachedSearch::Unsearched => None,
            CachedSearch::Known(found) => Some(json!([trainee, found.as_ref().map(super::super::found_out)])),
        })
        .collect()
}

/// Each trainee's best career (`Env::solve`, one trainee, top 1) and the decks that search checked, per plan: kept for
/// the next plan of the same setup and places whose pool is this one plus one veteran at the end (the page's Compare
/// and Reroll: your veterans and her). Every career of the larger pool is one of this pool's or has her as a parent, so
/// the best is the better of the kept one and the best with her as a parent (a search with her fixed): exact, the same
/// value; on an exact tie the kept career stands.
/// `CachedSearch` distinguishes an unsearched trainee from a search that returned no career.
/// Search completeness is separate, in `unp`/`lunp`.
pub(in super::super) struct Bests {
    key: (u64, u64),
    best: Vec<CachedSearch>,
    refs: Vec<Vec<Vec<Ref>>>,
    /// her best looped career (`index_rule`), where searched: kept the same way (a pool plus one veteran adds looped pairs
    /// only when she is an 8/8)
    looped: Vec<CachedSearch>,
    /// per uma whether a search behind her kept best (and her looped best) was not complete (`Solved::complete` false: it
    /// ran out of work, a band overflowed, or a scored deck passed its class's cap)
    unp: Vec<bool>,
    lunp: Vec<bool>,
}
thread_local! { static BESTS: std::cell::RefCell<Vec<Bests>> = const { std::cell::RefCell::new(Vec::new()) }; }
/// the kept plans for the page to store (`state_out`): per plan its key and every uma's kept best (her decks' refs are
/// not kept: nothing reads them back)
pub(in super::super) fn bests_out() -> Value {
    BESTS.with(|c| Value::Array(c.borrow().iter().map(|b| json!({"k": [super::super::ux(b.key.0), super::super::ux(b.key.1)], "n": b.best.len(),
        "b": cached_searches_out(&b.best),
        "l": cached_searches_out(&b.looped),
        "u": (0..b.unp.len()).filter(|&u| b.unp[u]).collect::<Vec<_>>(), "lu": (0..b.lunp.len()).filter(|&u| b.lunp[u]).collect::<Vec<_>>()})).collect()))
}
pub(in super::super) fn bests_in(v: &Value) -> Option<Vec<Bests>> {
    use super::super::{found_in, len_in, usz, uxr};
    v.as_array()?
        .iter()
        .map(|e| {
            let k = e["k"].as_array()?;
            // (checked conversions: on wasm32 usize is 32 bits, and a stored count or index must not wrap into range)
            let n = len_in(&e["n"])?;
            let per_uma = |key: &str| -> Option<Vec<CachedSearch>> {
                let mut v = vec![CachedSearch::Unsearched; n];
                for x in e[key].as_array()? {
                    let a = x.as_array()?;
                    let u = usz(a.first()?)?;
                    if u >= n {
                        return None;
                    }
                    v[u] = CachedSearch::Known(if a.get(1)?.is_null() { None } else { Some(found_in(a.get(1)?)?) });
                }
                Some(v)
            };
            // Older stored states omit these flags; those searches are treated as proven.
            let flags = |key: &str| -> Vec<bool> {
                let mut flags = vec![false; n];
                for entry in e[key].as_array().into_iter().flatten() {
                    if let Some(trainee) = usz(entry)
                        && trainee < n
                    {
                        flags[trainee] = true;
                    }
                }
                flags
            };
            Some(Bests {
                key: (uxr(k.first()?)?, uxr(k.get(1)?)?),
                best: per_uma("b")?,
                refs: vec![vec![]; n],
                looped: per_uma("l")?,
                unp: flags("u"),
                lunp: flags("lu"),
            })
        })
        .collect()
}
pub(in super::super) fn bests_set(b: Vec<Bests>) {
    BESTS.with(|c| *c.borrow_mut() = b);
}
/// the kept plans, taken out (the simulator's `index` policy starts each run without them, `sim::kept_swap`)
pub(in super::super) fn bests_take() -> Vec<Bests> {
    BESTS.with(|c| std::mem::take(&mut *c.borrow_mut()))
}
/// Remove one kept plan while `index_rule` owns it; the plan puts retained entries back when it finishes.
fn take_kept_plan(key: (u64, u64)) -> Option<Bests> {
    BESTS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|plan| plan.key == key);
        index.map(|index| cache.remove(index))
    })
}
fn h64(s: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut h);
    h.finish()
}
/// (setup and places, the first `n` legacies; the borrowed-parent rule unless "allow": `setup_fp` leaves it out, and a
/// plan under "later" or "never" keeps other looped careers, under "never" other best careers too)
fn bests_key(job: &Value, fix: &[usize], n: usize) -> (u64, u64) {
    // (the slice's text as the array's: the same serializer, no copy)
    let ls = job
        .get("legacies")
        .and_then(|l| l.as_array())
        .map(|a| serde_json::to_string(&a[..n.min(a.len())]).unwrap())
        .unwrap_or_default();
    let bp = job
        .get("borrowed_parent")
        .and_then(|b| b.as_str())
        .filter(|&b| b != "allow")
        .map_or(0, |b| h64(b).rotate_left(2));
    (crate::deck::career::setup_fp(job) ^ h64(&format!("{fix:?}")).rotate_left(1) ^ bp, h64(&ls))
}
/// trainee u's best career in `pool` (and the refs to absorb, and whether every search behind it was complete,
/// `Solved::complete`), reusing a kept plan of the pool less its last veteran
fn best_of(
    env: &mut Env,
    pool: &Pool,
    fix: &[usize],
    u: usize,
    same: Option<&Bests>,
    prev: Option<&Bests>,
    nl: usize,
) -> (Option<Found>, Vec<Vec<Ref>>, bool) {
    // a kept plan of this very pool (the same job planned again: a reload, a setting changed and back): her best, as kept
    if let Some(same) = same
        && let Some(CachedSearch::Known(found)) = same.best.get(u)
    {
        return (found.clone(), vec![], !same.unp.get(u).copied().unwrap_or(false));
    }
    if let Some(previous) = prev
        && let Some(CachedSearch::Known(old)) = previous.best.get(u)
    {
        let mut refs = previous.refs.get(u).cloned().unwrap_or_default();
        let kept_ok = !previous.unp.get(u).copied().unwrap_or(false);
        if fix.len() >= 2 {
            return (old.clone(), refs, kept_ok);
        } // both parents fixed: she can't be one
        let mut fixed_with_new_veteran = fix.to_vec();
        fixed_with_new_veteran.push(nl - 1);
        // no career with her can beat the kept one (each one's ceiling, as `solve` takes them, is below it): kept, exact
        if let Some(o) = old {
            let her = nl - 1;
            // (her legal pairs, `gen_cands`' ceilings: `ceiling` at `lineage`'s chances)
            // (their maximum folded from 0 in generation order, as a fold over the collected ceilings; no Vec: `()` items)
            let mut ub = 0.0f64;
            env.gen_cands(
                pool,
                fix,
                &[u],
                |_, b| b == her,
                |x, _, _, _, _| {
                    ub = f64::max(ub, x);
                },
            );
            if ub * (1.0 + 1e-9) < o.val {
                return (Some(o.clone()), refs, kept_ok);
            }
        }
        // only a career above the kept one matters: the search's bar starts there (exact; a tie keeps the kept one)
        env.floor = old.as_ref().map_or(f64::NEG_INFINITY, |o| o.val);
        let searched = env.solve(pool, &fixed_with_new_veteran, &[u], false, 1);
        env.floor = f64::NEG_INFINITY;
        if refs.len() < searched.refs.len() {
            refs.resize(searched.refs.len(), vec![]);
        }
        for (trainee, checked) in searched.refs.into_iter().enumerate() {
            refs[trainee].extend(checked);
        }
        let ok = kept_ok && searched.complete;
        let new = searched.found.into_iter().next();
        let best = match (old, new) {
            (Some(old), Some(new)) => Some(if new.val > old.val { new } else { old.clone() }),
            (old, new) => old.clone().or(new),
        };
        return (best, refs, ok);
    }
    let sr = env.solve(pool, fix, &[u], false, 1);
    let ok = sr.complete;
    (sr.found.into_iter().next(), sr.refs, ok)
}
/// What `score_career` reads that grows while a plan runs: the deck cache's decks and fully checked decks (`Cache`,
/// per trainee, both only grow) summed over the trainees `us` (None: every trainee), and the deck searches run so far
/// (`deck_for`'s: the only work in a scoring that changes what a later one reads)
fn sver(s: &Sim, us: Option<&[usize]>) -> (usize, usize, usize) {
    let (decks, proofs) = match us {
        Some(trainees) => trainees.iter().fold((0, 0), |(decks, proofs), &trainee| {
            (decks + s.cache.decks[trainee].len(), proofs + s.cache.refs[trainee].len())
        }),
        None => (s.cache.n_decks(), s.cache.refs.iter().map(std::vec::Vec::len).sum()),
    };
    (decks, proofs, s.n.searches)
}
/// A scoring with its stamp: the cache it read (`sver` over `us`, the trainees whose decks it reads) when it ran no deck
/// search and added nothing, else None. A scoring of the same career against the same base, run when `sver` is again
/// the stamp, reads the same decks and tables, takes the same branches (whether `deck_for` searches is decided by those
/// alone) and gives the same f64s (`stamp_ok`), so it need not run
fn stamped<R>(s: &mut Sim, us: Option<&[usize]>, f: impl FnOnce(&mut Sim) -> R) -> (R, Option<(usize, usize)>) {
    let before = sver(s, us);
    let result = f(s);
    let after = sver(s, us);
    (result, (before == after).then_some((after.0, after.1)))
}
fn stamp_ok(s: &Sim, us: Option<&[usize]>, stamp: Option<(usize, usize)>) -> bool {
    let current = sver(s, us);
    stamp == Some((current.0, current.1))
}
/// the same career as `score_career` reads it: trainee, parents in order, every field of the deck (floats by bits). Not
/// its value (`Found::val`, nor `ub`): `score_career` never reads them, and a proxy's value (the surrogate's corner-table
/// interpolation) and the search's exact one differ in their last bits
fn same_career(f: &Found, g: &Found) -> bool {
    let bits = |a: &[f64], b: &[f64]| a.len() == b.len() && a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
    f.t == g.t
        && f.a == g.a
        && f.b == g.b
        && f.sel.ents == g.sel.ents
        && f.sel.plain == g.sel.plain
        && f.sel.filler == g.sel.filler
        && bits(&f.sel.dw, &g.sel.dw)
        && bits(&f.sel.dg, &g.sel.dg)
}
const fn p_margin() -> f64 {
    1e9
}
const fn p_swork() -> usize {
    5_000_000
}

#[derive(Deserialize, Clone)]
pub(in super::super) struct PlanJob {
    /// the surrogate's refresh margin (`sim` module notes)
    #[serde(default = "p_margin")]
    margin: f64,
    /// work budget (career-search units) of each candidate search beyond the job's own (also each loop bootstrap
    /// search), and of each deck search a continuation makes (a refresh); 0 = the job's budget
    #[serde(default = "p_swork")]
    search_work: usize,
    /// the rotation (chara ids, in order); absent: the loop planner's best loop for the job's settings; empty: no loop base
    #[serde(default)]
    loop_members: Option<Vec<u32>>,
    /// `index_rule`: each uma's best career through the surrogate (`best_sur`) instead of the career search (1), or
    /// both, reporting where they differ (2); the surrogate's refresh margin there
    #[serde(default)]
    sur: u8,
    #[serde(default)]
    sur_margin: f64,
    /// `index_rule` measurement: each builder's career also from her deck class's winning decks alone (no career search),
    /// its lifts and score beside the exact ones (`proxy_check` in the output); changes nothing else
    #[serde(default)]
    proxy_check: bool,
    /// `index_rule`: every uma's best career searched exactly (the rule before builders went through their deck class)
    #[serde(default)]
    exact_all: bool,
    /// `index_rule`: the stage candidates (chara ids, in order) and their continuations (lam, phi, pinf) taken as given,
    /// not fitted to this pool: a comparison prices a pool with one more veteran under the fit of the plan it compares
    /// with (the plan's `fit`), so the two values differ only by what she changes, not by a refit
    #[serde(default)]
    fit: Option<Vec<(u32, f64, f64, f64)>>,
    /// `index_rule` test knob: PROXY_EFF in place of 0.06 (-1 leaves pass 1's builders to their proxies, so the pass-2
    /// searches refit Cont: tests/plan_memo.rs runs that path)
    #[serde(default)]
    proxy_eff: Option<f64>,
    /// a roll of the plan's pick (her stars per job target, 0 where it did not land): answered from the plan's kept roll
    /// state (`roll_kept`), the plan run first when the worker keeps none
    #[serde(default)]
    roll: Option<Vec<u8>>,
}

fn sim_for<'env, 'data>(
    env: &'env mut Env<'data>,
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    pj: &PlanJob,
) -> Result<Sim<'env, 'data>, Value> {
    let ntar = env.ntar;
    if ntar > 12 || env.nu > 10 {
        return Err(json!({"error": "The plan takes at most 12 targets, 10 of them not guaranteed by the scenario."}));
    }
    let cfg: SimJob = serde_json::from_value(json!({"margin": pj.margin}))
        .map_err(|e| json!({"error": format!("The plan's margin does not make a simulator setting: {e}")}))?;
    // as `simulate` builds it (the continuations are the player's later careers: every owned trainee)
    let trainees = env.owned.clone();
    Ok(Sim::new(env, d, sk, cr, job, cfg, trainees, vec![], vec![], false, true))
}

/// The rotation for the loop base `ids` (`PlanJob::loop_members`): (members as uma indices, their {id, name}, or why
/// there is none). Members must be owned umas with a legal deck, at least three of them.
fn rotation(env: &Env, d: &Data, ids: &[u32]) -> (Vec<usize>, Vec<Value>, Option<String>) {
    let info: Vec<Value> = ids
        .iter()
        .map(|&id| {
            let u = d.umas.iter().find(|u| u.id == id);
            json!({"id": id, "name": u.map_or_else(|| format!("Uma {id}"), |u| u.name.clone())})
        })
        .collect();
    if info.is_empty() {
        return (vec![], vec![], Some("no loop base (loop_members empty)".into()));
    }
    let idx: Vec<Option<usize>> = info.iter().map(|m| m["id"].as_u64().and_then(|id| env.uix(id as u32))).collect();
    if idx.len() < 3 || idx.iter().any(|x| x.is_none_or(|t| !env.owned.contains(&t) || env.dmax[t].is_none())) {
        return (vec![], info, Some("the loop's members must be at least three owned umas with a legal deck".into()));
    }
    (idx.into_iter().map(|x| x.unwrap()).collect(), info, None)
}

/// Candidates the index rule weighs at most, and partners kept per trainee where a candidate's veteran is a parent
const IDX_CANDS: usize = 8;
const IDX_PARTNERS: usize = 2;

/// legal parents a, b for trainee u (`Pool::pair_ok`)
fn legal_pair(d: &Data, p: &Pool, u: usize, a: usize, b: usize) -> bool {
    p.pair_ok(d.umas[u].id, a, b)
}
/// Whether builder x's career f (her deck class's proxy) is provably her best, without a career search: over every legal
/// pair one of the bounds `solve` prunes with (`Env::ceiling_out`; `refs_out` over the fully checked decks of her class,
/// exact for the class; `ruled_out` over its product proofs) is at most f's value (`Bar::AtMost`), so no deck and pair
/// beats it (a different deck or pair of equal value may exist; the value is the search's). With mob or affinity-free
/// parents every classmate has the same chances, so one classmate's search proves the rest of her class.
fn certified(env: &mut Env, pool: &Pool, fix: &[usize], x: usize, f: &Found) -> bool {
    let ct = env.cls[x];
    // (each test computed alone, so the order the pairs are taken in changes no number; a NaN bound proves nothing)
    let pairs = env.gen_cands(pool, fix, &[x], |_, _| true, |ub, _, _, a, b| (ub, a, b));
    let mut q = Vec::with_capacity(env.ntar);
    for (ub, a, b) in pairs {
        let bar = Bar::AtMost(f.val);
        if Env::ceiling_out(ub, bar) {
            continue;
        }
        let c = env.lineage_into(pool, x, a, b, &mut q);
        if env.refs_out(x, &q, c, env.seen[ct].iter(), bar) || env.ruled_out(x, &q, c, &env.ruled[ct], bar) {
            continue;
        }
        return false;
    }
    true
}
/// trainee u's P(all) with parents a, b (cached decks; a deck search where she has none)
fn val(s: &mut Sim, p: &Pool, u: usize, a: usize, b: usize, buf: &mut Vec<f64>) -> f64 {
    let mut q = std::mem::take(&mut s.qs);
    let c = s.env.lineage_into(p, u, a, b, &mut q);
    let v = match s.env.ceiling(u, &q, c) {
        None => 0.0,
        Some(ub) => s.deck_for(u, &q, c, ub, 0.0, buf).map_or(0.0, |x| x.0),
    };
    s.qs = q;
    v
}
/// Trainee u's best career through the surrogate (`deck_for`, sim module notes): every legal pair (all of `fix` among
/// the parents), highest ceiling first, valued by her cached decks, an exact deck search where the bound beats the table
/// by more than `margin`; stops when the next ceiling is below the best. margin 0: P(all) at least the exact best, up
/// to deck searches that run out of work.
fn best_sur(s: &mut Sim, pool: &Pool, fix: &[usize], u: usize, margin: f64, buf: &mut Vec<f64>) -> Option<Found> {
    let m0 = s.cfg.margin;
    s.cfg.margin = margin;
    // (the best so far as its pair, value, ceiling and cached deck index: her cached decks only grow, so the index stays
    // valid, and the deck is copied once, at the end)
    let mut best: Option<(usize, usize, f64, f64, usize)> = None;
    for (ub, a, b) in s.env.ranked_pairs(pool, fix, u, |_, _| true) {
        let bar = best.map_or(0.0, |f| f.2);
        if ub <= bar {
            break;
        }
        let mut q = std::mem::take(&mut s.qs);
        let c = s.env.lineage_into(pool, u, a, b, &mut q);
        let r = s.deck_for(u, &q, c, ub, bar, buf);
        s.qs = q;
        if let Some((v, di)) = r
            && v > bar
        {
            best = Some((a, b, v, ub, di));
        }
    }
    s.cfg.margin = m0;
    best.map(|(a, b, val, ub, di)| Found { t: u, a, b, val, ub, sel: s.cache.decks[u][di].sel.clone() })
}
/// an 8/8 veteran (a spark of every job target), yours or borrowed
fn is_full(sj: &SkillJob, sparks: &[(u32, u8)]) -> bool {
    sj.targets.iter().all(|t| sparks.iter().any(|x| x.0 == t.group && x.1 > 0))
}

/// One place a builder's new veteran may be a parent: the stage-1 candidate she may lift (index), per uncertain target
/// (env.uncv order) the miss product without her, per job target the spark scale without and with her spark, her miss
/// factor x per star count (0: absent), how many of the trainee's cached decks it reads (their corner tables; a trainee's
/// cached decks only grow, so the first `nd` are the ones there when the setup was made)
struct Setup {
    ui: usize,
    x0: Vec<f64>,
    pw: Vec<[f64; 2]>,
    xm: [f64; 4],
    nd: usize,
}

/// Scores within this many careers per run of the best (or 5% of it) count as tied
const NEAR: f64 = 0.25;
/// the lowest score tied with `top` (`NEAR`)
fn near_floor(top: f64) -> f64 {
    top - NEAR.max(0.05 * top.abs())
}
/// Why a career was added to the planner's options. Rows retain their insertion order:
/// base careers, looped alternatives, then alternatives without borrowed parents.
#[derive(Clone, Copy, PartialEq, Eq)]
enum RowOrigin {
    Base,
    Looped,
    NoBorrow,
    /// Main-search options used when no planner row can be picked. Not a planner search.
    Fallback,
}

/// A career and its planner state. Replacing the career preserves the previous scoring
/// until pass 2 either reuses it (same career and valid stamp) or scores it again.
struct PlanRow {
    trainee: usize,
    career: Found,
    lift_stats: Vec<(f64, f64)>,
    origin: RowOrigin,
    exact: bool,
    stats_stamp: Option<(usize, usize)>,
    scoring: Option<RowScoring>,
}

struct RowScoring {
    net_saving: f64,
    improvement_chance: f64,
    /// Can be discarded for a row that cannot win, or taken for the pick's roll state.
    /// The row remains scored in either case.
    kept: Option<KeptScoring>,
    /// Covers every trainee's decks: scoring can read any trio's final trainee.
    stamp: Option<(usize, usize)>,
}

impl PlanRow {
    const fn new(
        trainee: usize,
        career: Found,
        lift_stats: Vec<(f64, f64)>,
        origin: RowOrigin,
        exact: bool,
        stats_stamp: Option<(usize, usize)>,
    ) -> Self {
        Self { trainee, career, lift_stats, origin, exact, stats_stamp, scoring: None }
    }

    fn net_saving(&self) -> Option<f64> {
        self.scoring.as_ref().map(|scoring| scoring.net_saving)
    }

    fn total(&self) -> f64 {
        self.net_saving().unwrap_or(f64::NEG_INFINITY)
    }
}
/// The pick among the rows `runs` (a row without scoring has score -infinity): the highest score among the rows that can
/// be the pick (`vis`; the first such row before any is scored, by more than 1e-9), then within `near_floor` of that top
/// the best odds (a row that can be the pick), moved to the earlier row it repeats (`undup`). (The pick, top.) The one
/// rule for the plan's pick and the roll state kept for it
fn pick_of(runs: &[PlanRow], vis: impl Fn(&Found) -> bool, undup: &[usize]) -> (usize, f64) {
    let total = |i: usize| runs[i].total();
    let mut pick = (0..runs.len()).find(|&i| vis(&runs[i].career)).unwrap_or(0);
    for i in 0..runs.len() {
        if total(i) > total(pick) + 1e-9 {
            pick = i;
        }
    }
    let top = if runs.is_empty() { 0.0 } else { total(pick) };
    for i in 0..runs.len() {
        if vis(&runs[i].career) && total(i) >= near_floor(top) && runs[i].career.val > runs[pick].career.val {
            pick = i;
        }
    }
    if let Some(&u) = undup.get(pick) {
        pick = u;
    }
    (pick, top)
}
/// A builder's lift efficiency for a candidate at odds p: her veteran lifts the candidate with chance pl to odds sp / pl
/// on average (`score_career`'s stats), so pl (1/p - 1/(sp/pl)) careers saved per career (Cont's builders, the proxies'
/// admission and `proxy_check`; PROXY_EFF was calibrated on it)
fn lift_eff(pl: f64, sp: f64, p: f64) -> f64 {
    pl * (1.0 / p - 1.0 / (sp / pl))
}
/// `f` under the plan's career-search work cap `work` (`PlanJob::search_work`; 0: none), the env's own budget `budget0`
/// put back after (and no hard cap)
fn capped<'env, 'data, R>(
    s: &mut Sim<'env, 'data>,
    work: usize,
    budget0: usize,
    f: impl FnOnce(&mut Sim<'env, 'data>) -> R,
) -> R {
    if work > 0 {
        s.env.budget = work;
        s.env.hard = work;
    }
    let result = f(s);
    s.env.hard = 0;
    s.env.budget = budget0;
    result
}
/// Builders through their deck class (`index_rule`): a proxy is searched exactly when her lift efficiency is within this
/// share of a candidate's second best, or her saving within this share of the lowest saving the shown rows or the pick's
/// near-tie band take. Recalibrated 2026-10-09 (val/plan `proxy_check`, with the own-8/8 credit): the proxy is
/// the exact career for every builder on five of the seven jobs; it reads at most 2.79% low on a contender's efficiency
/// (exact at least 0.7 of a candidate's second best; p11) and 2.67% low on a builder's saving (p11): 0.06 is 2.15x and
/// 2.25x those. (The first measurement, on the planner as it was before the class winners and the merges since, had 11%
/// and 9%, with 0.2.) The errors move with the model: run `proxy_check` again after a change to what builders' careers
/// are made of
const PROXY_EFF: f64 = 0.06;
const PROXY_SAVE: f64 = 0.06;
/// the rows the page shows (career.js `OPT_SHOW`): their careers are searched exactly
const OPT_SHOW: usize = 8;
/// Trainee u's odds with a full loop lineage: parents hypothetical veterans of b1 and b2 (every target, stars at the
/// rank's mix, G1 wins `w1` / `w2`), each bred from the other and from u (grandparents every target at 2 stars)
fn loop_ceiling(
    s: &mut Sim,
    d: &Data,
    pool: &Pool,
    u: usize,
    b1: usize,
    b2: usize,
    wins: &dyn Fn(usize) -> Vec<usize>,
    buf: &mut Vec<f64>,
) -> f64 {
    let sj = s.env.sj;
    let star = s.env.sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let anc = |x: usize| Ancestor { chara: d.umas[x].id, sparks: full.clone(), wins: wins(x), pinks: vec![] };
    let mk = |x: usize, p1: usize, p2: usize| Legacy {
        chara: d.umas[x].id,
        borrowed: false,
        sparks: full.clone(),
        wins: wins(x),
        pinks: vec![],
        parents: vec![anc(p1), anc(p2)],
        win_p: vec![],
        mix: Some(star),
    };
    // (`Env::pool` of legs and the two: the working copy of `pool` with them pushed, then popped)
    let mut p = s.take_pool(pool);
    s.env.pool_push(&mut p, &mk(b1, b2, u));
    s.env.pool_push(&mut p, &mk(b2, b1, u));
    let n = p.me.len();
    let v = if legal_pair(d, &p, u, n - 2, n - 1) { val(s, &p, u, n - 2, n - 1, buf) } else { 0.0 };
    Env::pool_pop(&mut p);
    Env::pool_pop(&mut p);
    s.put_pool(pool, p);
    v
}
/// Below this many stage candidates the plan adds umas by `hint_umas` (the loop planner's search added up to three; a
/// pool with veterans of its loop's umas gives four or more by the other routes: 6-8 on the val/plan pools)
const HINT_FLOOR: usize = 4;
/// The owned umas (a legal deck, not among `cands`) with the highest odds under a full loop lineage, at most `k`, best
/// first: her parents hypothetical veterans of the first two candidates' umas (every target, stars at the rank's mix),
/// each bred from the other and from her, as `loop_ceiling` builds p_inf's lineage (a missing candidate: an ancestor of
/// no uma, so no relation points); no G1 wins on the hypothetical ancestors (the race part of their affinity is the
/// same for every uma ranked, and the wins come from schedules the pool doesn't have yet). Valued by `Env::ceiling` (the
/// per-target best deck odds: no deck search), which ranks the umas the way a full loop would before any building.
fn hint_umas(env: &mut Env, d: &Data, pool: &Pool, cands: &[usize], trainees: &[usize], k: usize) -> Vec<(usize, f64)> {
    let sj = env.sj;
    let star = sj.star_dist;
    let full: Vec<(u32, u8)> = sj.targets.iter().map(|t| (t.group, 2)).collect();
    let mut sides: Vec<u32> = vec![];
    for &c in cands {
        let id = d.umas[c].id;
        if !sides.contains(&id) {
            sides.push(id);
        }
        if sides.len() == 2 {
            break;
        }
    }
    while sides.len() < 2 {
        sides.push(0);
    }
    let (ca, cb) = (sides[0], sides[1]);
    let anc = |chara: u32| Ancestor { chara, sparks: full.clone(), wins: vec![], pinks: vec![] };
    let mk = |chara: u32, p1: u32, p2: u32| Legacy {
        chara,
        borrowed: false,
        sparks: full.clone(),
        wins: vec![],
        pinks: vec![],
        parents: vec![anc(p1), anc(p2)],
        win_p: vec![],
        mix: Some(star),
    };
    let mut out: Vec<(usize, f64)> = vec![];
    // (`Env::pool` of the legacies and the two: one copy of `pool`, `Env::pool(legs)`, the two pushed and popped per uma)
    let mut p = pool.clone();
    for &u in trainees {
        let id = d.umas[u].id;
        if cands.contains(&u) || env.dmax[u].is_none() || id == ca || id == cb {
            continue;
        }
        env.pool_push(&mut p, &mk(ca, cb, id));
        env.pool_push(&mut p, &mk(cb, ca, id));
        let n = p.me.len();
        if legal_pair(d, &p, u, n - 2, n - 1) {
            let (q, c) = env.lineage_q(&p, u, n - 2, n - 1);
            if let Some(v) = env.ceiling(u, &q, c)
                && v > 0.0
            {
                out.push((u, v));
            }
        }
        Env::pool_pop(&mut p);
        Env::pool_pop(&mut p);
    }
    // best first; equal odds in uma order (stable)
    out.sort_by(|x, y| y.1.partial_cmp(&x.1).unwrap());
    out.truncate(k);
    out
}
/// What a career is scored against: stage-1 costs per candidate (index into `cands`; 0 = her 8/8 exists), stages 2-3 per
/// A, V0, and the stage-1 odds now
pub(super) struct ScoreBase<'base> {
    pub(super) cands: &'base [usize],
    pub(super) c1: &'base [f64],
    pub(super) m23: &'base [f64],
    pub(super) v0: f64,
    pub(super) pnow: &'base [f64],
    pub(super) cont: &'base [Cont],
    /// the trios (A, B as candidate indices, C's step, A and B from a real 8/8 of hers): V = min over them of A's step +
    /// B's step + C's step, a side from a real 8/8 costing 0, else c1; empty: min over a of c1[a] + m23[a]
    pub(super) combos: &'base [Trio],
    /// per candidate, the trios' sides: (a real 8/8 veteran of hers, the legacy): her real 8/8s and the 8/8 her best career
    /// now would leave; with `all` (the umas a trio's C may be), what `her_trios` pairs a new real 8/8 with. Empty: none
    pub(super) sides: &'base [Vec<(bool, Legacy)>],
    pub(super) all: &'base [usize],
}
/// Requested scoring outputs. Unrequested numeric results are placeholders, not measured zeroes:
///
/// - Stats: per-candidate lift statistics; expected saving and improvement chance are returned as 0.0.
/// - Score: expected saving and improvement chance; no per-candidate statistics.
/// - Keep: Score plus the prepared scoring needed to evaluate later rolls (`score_career`).
/// - Ys: Score plus a saving for each evaluated outcome (`sc_eval`).
/// - Touch: nothing; the preparation's side effects only (her schedule, the partners' `val` searches, `sc_prep`'s
///   counters), its evaluation skipped, zeros returned (the looped rows, whose stats only `proxy_check` reads).
///
/// This selects outputs, not side effects: even Stats can search and populate the deck cache during preparation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Want {
    Stats,
    Score,
    Keep,
    Ys,
    Touch,
}
/// (A, B, C's step, A from a real 8/8, B from a real 8/8)
pub(super) type Trio = (usize, usize, f64, bool, bool);
/// V over the trios at step costs c1
fn trio_v(c1: &[f64], combos: &[Trio]) -> f64 {
    // (a plain comparison: no value here is NaN, so the same minimum as f64::min)
    let mut best = f64::INFINITY;
    for &(ia, ib, c3, ra, rb) in combos {
        let v = (if ra { 0.0 } else { c1[ia] }) + (if rb { 0.0 } else { c1[ib] }) + c3;
        if v < best {
            best = v;
        }
    }
    best
}
/// An exact memo of a pure function of its key, owned (through an `Rc`) by the one object whose fixed parameters the
/// function reads; a hit returns the value a miss computed for the same key bits, so outputs are the same by
/// construction. The gate only decides whether to look up: after MEMO_PROBE lookups with fewer than one hit in
/// MEMO_MIN_HIT it stops (the hash and insert would cost more than the misses save). Past MEMO_CAP entries it is cleared
/// (the wasm page's memory).
pub(super) struct Memo<K, V> {
    map: FxMap<K, V>,
    look: u64,
    hit: u64,
    off: bool,
    kind: usize,
    cap: usize,
}
const MEMO_PROBE: u64 = 4096;
const MEMO_MIN_HIT: u64 = 8;
/// (measured on val/plan: a Cont memo without a cap would hit 0.2-1.5% more often; 1 << 14 entries hold about 0.6 MB)
const MEMO_CAP: usize = 1 << 14;
/// bit equality of memoised values (the audit's test)
pub(super) trait SameBits {
    fn same_bits(&self, o: &Self) -> bool;
}
impl SameBits for f64 {
    fn same_bits(&self, o: &Self) -> bool {
        self.to_bits() == o.to_bits()
    }
}
impl SameBits for (f64, i32, f64) {
    fn same_bits(&self, o: &Self) -> bool {
        self.0.same_bits(&o.0) && self.1 == o.1 && self.2.same_bits(&o.2)
    }
}
impl<K: std::hash::Hash + Eq, V: Copy + SameBits + std::fmt::Debug> Memo<K, V> {
    // (`PLAN_MEMO_OFF`: every memo off from the start, for the test that compares against no memo)
    // (`PLAN_MEMO_CAP`: another cap, for the test: entries kept long enough to be read after a refit)
    fn new(kind: usize) -> Self {
        let e = memo_env();
        Self { map: FxMap::default(), look: 0, hit: 0, off: e.0, kind, cap: e.2 }
    }
    #[inline]
    fn get_or(&mut self, k: impl FnOnce() -> K, f: impl FnOnce() -> V) -> V {
        if self.off {
            return f();
        }
        self.look += 1;
        let k = k();
        if let Some(&v) = self.map.get(&k) {
            self.hit += 1;
            // (`PLAN_MEMO_CHECK`: every hit computed again and compared bit for bit; tests/plan_memo.rs)
            if memo_env().1 {
                let w = f();
                assert!(v.same_bits(&w), "memo kind {}: hit {v:?}, computed {w:?}", self.kind);
            }
            return v;
        }
        let v = f();
        if self.look >= MEMO_PROBE && self.hit * MEMO_MIN_HIT < self.look {
            self.off = true;
            self.map = FxMap::default();
            return v;
        }
        if self.map.len() >= self.cap {
            self.map.clear();
            memo_stat(7);
        }
        self.map.insert(k, v);
        v
    }
}
// (`PLAN_MEMO`: per plan, each memo kind's lookups, hits and memos gated off, the Cont refits and the cap's clears, on
// stderr)
thread_local! { static MEMO_STATS: std::cell::Cell<[u64; 8]> = const { std::cell::Cell::new([0; 8]) }; }
impl<K, V> Drop for Memo<K, V> {
    fn drop(&mut self) {
        MEMO_STATS.with(|c| {
            let mut s = c.get();
            s[3 * self.kind] += self.look;
            s[3 * self.kind + 1] += self.hit;
            s[3 * self.kind + 2] += u64::from(self.off);
            c.set(s);
        });
    }
}
/// (PLAN_MEMO_OFF, PLAN_MEMO_CHECK, PLAN_MEMO_CAP or MEMO_CAP)
fn memo_env() -> (bool, bool, usize) {
    static E: std::sync::OnceLock<(bool, bool, usize)> = std::sync::OnceLock::new();
    *E.get_or_init(|| {
        (
            std::env::var("PLAN_MEMO_OFF").is_ok(),
            std::env::var("PLAN_MEMO_CHECK").is_ok(),
            std::env::var("PLAN_MEMO_CAP").ok().and_then(|v| v.parse().ok()).unwrap_or(MEMO_CAP),
        )
    })
}
fn memo_stat(i: usize) {
    MEMO_STATS.with(|c| {
        let mut s = c.get();
        s[i] += 1;
        c.set(s);
    });
}
struct MemoReport;
impl Drop for MemoReport {
    fn drop(&mut self) {
        let s = MEMO_STATS.with(|c| c.replace([0; 8]));
        if std::env::var("PLAN_MEMO").is_ok() {
            let r = |l: u64, h: u64| if l == 0 { 0.0 } else { 100.0 * h as f64 / l as f64 };
            eprintln!(
                "PLAN_MEMO plan lookups {} hits {} ({:.1}%) off {} | cost_lo lookups {} hits {} ({:.1}%) off {} | refits {} clears {}",
                s[0],
                s[1],
                r(s[0], s[1]),
                s[2],
                s[3],
                s[4],
                r(s[3], s[4]),
                s[5],
                s[6],
                s[7]
            );
        }
    }
}
/// A step's continuation (form assumed; the rate and step fitted to the pool now, the shape to traces): builder careers
/// lift her odds, each lift closing `phi` of the remaining gap in log odds to `pinf`, her effective ceiling; the lift
/// chance falls with the remaining gap: lam_j = lam ((1 - phi)^j)^LAM_DECAY. pinf = CEIL_SHARE x her full-loop odds.
/// Both constants fitted (val/sim/README.md E5) to traces of building for Agnes Digital on 2-, 5-, 11- and
/// 19-veteran pools (the best builder each career, 6 runs x 400 careers): within 10-20% on 11 and 19, building
/// underestimated on 5 and overestimated early on 2; leave-one-pool-out fits ranged 0.6-1.0 and 1-3. lam = 0: none.
#[derive(Clone)]
pub(super) struct Cont {
    lam: f64,
    phi: f64,
    pinf: f64,
    /// per n (0..=CONT_N): the careers built through the n-th success (the sum `plan` takes, in its order), and (1 - phi)^n:
    /// `plan` reads them instead of evaluating the powers per call (the same expressions, so the same values)
    tab: std::rc::Rc<Vec<(f64, f64)>>,
    /// `plan`'s and `cost_lo`'s results by the bits of p. Exact: both read p and lam, phi, pinf and tab alone, which are
    /// set once in `new` and never written after (private fields; a refit makes new Conts). A clone shares the memos and
    /// carries the same parameters; a new or default Cont gets its own
    memo: std::rc::Rc<ContMemo>,
}
pub(super) struct ContMemo {
    plan: std::cell::RefCell<Memo<u64, (f64, i32, f64)>>,
    lo: std::cell::RefCell<Memo<u64, f64>>,
}
fn cont_memo() -> std::rc::Rc<ContMemo> {
    std::rc::Rc::new(ContMemo {
        plan: std::cell::RefCell::new(Memo::new(0)),
        lo: std::cell::RefCell::new(Memo::new(1)),
    })
}
impl Default for Cont {
    fn default() -> Self {
        Self { lam: 0.0, phi: 0.0, pinf: 0.0, tab: std::rc::Rc::default(), memo: cont_memo() }
    }
}
/// `plan`'s cap on successes before grinding
const CONT_N: usize = 500;
/// Effective ceiling as a share of the full-loop odds, and the lift chance's decline exponent (val/sim/README.md E5)
const CEIL_SHARE: f64 = 0.6;
const LAM_DECAY: f64 = 1.5;
impl Cont {
    fn new(lam: f64, phi: f64, pinf: f64) -> Self {
        let r = 1.0 - phi;
        let mut tab = Vec::with_capacity(CONT_N + 1);
        let mut built = 0.0f64;
        for n in 0..=CONT_N as i32 {
            let b = built + 1.0 / (lam * r.powi(n).powf(LAM_DECAY));
            tab.push((b, r.powi(n)));
            built = b;
        }
        Self { lam, phi, pinf, tab: std::rc::Rc::new(tab), memo: cont_memo() }
    }
    /// (expected careers to her 8/8 from odds p, the best number of successes before grinding, the odds then): build n
    /// successes (sum of 1 / lam_j careers), then grind at p_n; the best n
    fn plan(&self, p: f64) -> (f64, i32, f64) {
        if p <= 0.0 {
            return (f64::INFINITY, 0, p);
        }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return (1.0 / p, 0, p);
        }
        self.memo.plan.borrow_mut().get_or(|| p.to_bits(), || self.plan_n(p))
    }
    /// `plan` past its early returns (the first stop of the building loop). Past them lam > 0, which only `Cont::new`
    /// makes (with its table; `rolls_in` refuses a stored Cont with lam and phi > 0 but no table)
    fn plan_n(&self, p: f64) -> (f64, i32, f64) {
        debug_assert!(self.tab.len() > CONT_N, "plan_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let t = &self.tab;
        let (mut n, mut best) = (0usize, 1.0 / p);
        while n < CONT_N {
            let x = t[n].0 + 1.0 / (self.pinf * (lr * t[n + 1].1).exp());
            if x >= best {
                break;
            }
            best = x;
            n += 1;
        }
        (best, n as i32, self.pinf * (lr * t[n].1).exp())
    }
    fn cost(&self, p: f64) -> f64 {
        self.plan(p).0
    }
    /// a lower bound on `cost` at every odds up to p: the minimum over every n (not the first local one `plan` stops at).
    /// Each n's cost B_n + 1 / p_n falls as p rises (p_n = pinf exp(ln(p / pinf) r^n) rises with p), so this minimum falls
    /// with p, and at any p' <= p it is at most the minimum at p', which is at most `cost(p')`
    fn cost_lo(&self, p: f64) -> f64 {
        if p <= 0.0 {
            return f64::INFINITY;
        }
        if self.lam <= 0.0 || self.phi <= 0.0 || self.pinf <= p {
            return 1.0 / p;
        }
        self.memo.lo.borrow_mut().get_or(|| p.to_bits(), || self.cost_lo_n(p))
    }
    /// `cost_lo` past its early returns (every n: CONT_N exp; `outcome_hi` asks for it per outcome and candidate; the
    /// table as in `plan_n`)
    fn cost_lo_n(&self, p: f64) -> f64 {
        debug_assert!(self.tab.len() > CONT_N, "cost_lo_n needs the continuation's table");
        let lr = (p / self.pinf).ln();
        let mut best = 1.0 / p;
        // every n's cost is its building (rising with n: each term 1/lam_j > 0) plus a positive grind, so once the
        // building alone reaches the best found no later n can be lower: the scan stops there, the same minimum
        let t = &self.tab;
        for n in 0..CONT_N {
            if t[n].0 >= best {
                break;
            }
            best = best.min(t[n].0 + 1.0 / (self.pinf * (lr * t[n + 1].1).exp()));
        }
        best
    }
    /// Whether `cost_lb` holds: `plan`'s path past its early returns at every p in (1e-250 pinf, pinf) (lam > 0, 0 < phi <
    /// 1, pinf finite, the table built: t[n].0 nondecreasing, t[n].1 = (1 - phi)^n >= 0)
    fn lb_ok(&self) -> bool {
        self.lam > 0.0
            && self.lam.is_finite()
            && self.phi > 0.0
            && self.phi < 1.0
            && self.pinf >= 1e-50
            && self.pinf.is_finite()
            && self.tab.len() > CONT_N
    }
    /// With `lb_ok`, a lower bound on `cost(p)` (None: none derived), the same for every p in p's cell (its bits >>
    /// LB_SHIFT, 128 cells per binade), memoised in `cell` (direct-mapped by cell): cost_lo(g) (1 - 1e-9), g the cell's top
    /// (bits (key + 1) << LB_SHIFT, above every p in it), where pinf 1e-250 < p and g < pinf. Derived: computed cost(p) >=
    /// computed cost_lo(p) bit for bit (the same x_n expressions; cost_lo's minimum is over every n until the building
    /// alone t[n].0 reaches its best, so it is at most each x_n, as each is at least its t[n].0, and at most 1/p, where
    /// `plan` returns one of them). In exact arithmetic on the stored t, X_n(p) = t[n].0 + 1 / (pinf exp(ln(p / pinf)
    /// t[n+1].1)) and 1/p fall as p rises, so L(p) = min(1/p, min_n<CONT_N X_n(p)) does; each computed x_n is within a
    /// factor 1 +- eps of X_n, eps <= (2 |ln(p / pinf)| + 6) u < 2e-13 here (ln and exp within 1 ulp, as glibc's and the
    /// musl-derived wasm libm are; |ln(p / pinf)| < 576), and L(g) >= cost_lo(g) / (1 + eps) (the n past cost_lo's stop
    /// are each at least t[n].0 >= its best). So for p < g: cost(p) >= cost_lo(p) >= (1 - eps) L(p) >= (1 - eps) L(g) >=
    /// cost_lo(g) (1 - eps) / (1 + eps) > the bound, rounded once more
    fn cost_lb(&self, p: f64, cell: &mut [(u64, f64); LB_CELLS]) -> Option<f64> {
        if !(p > self.pinf * 1e-250) {
            return None;
        }
        let key = p.to_bits() >> LB_SHIFT;
        let g = f64::from_bits((key + 1) << LB_SHIFT);
        if !(g < self.pinf) {
            return None;
        }
        let e = &mut cell[key as usize & (LB_CELLS - 1)];
        if e.0 != key {
            *e = (key, self.cost_lo(g) * (1.0 - 1e-9));
        }
        Some(e.1)
    }
}
/// `Cont::cost_lb`'s cells: 2^(52 - LB_SHIFT) per binade of p; LB_CELLS of them memoised per candidate
const LB_SHIFT: u32 = 45;
const LB_CELLS: usize = 256;
impl ScoreBase<'_> {
    // (no Cont: a default one's, lam = 0: grinding at p)
    fn cost(&self, ui: usize, p: f64) -> f64 {
        match self.cont.get(ui) {
            Some(c) => c.cost(p),
            None => {
                if p <= 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / p
                }
            }
        }
    }
    fn cost_lo(&self, ui: usize, p: f64) -> f64 {
        match self.cont.get(ui) {
            Some(c) => c.cost_lo(p),
            None => {
                if p <= 0.0 {
                    f64::INFINITY
                } else {
                    1.0 / p
                }
            }
        }
    }
}
/// A career's own 8/8 as a trio side when her uma is not a stage candidate (her outcome lands every target): the replan
/// would take her uma in (a veteran of hers) with that real 8/8 a free side. Per candidate side (real or fresh) her 8/8 can
/// pair with, C's step: 1 / the best allowed C's value with the two 8/8s as parents (every allowed uma but hers and the
/// partner's, best ceiling first, the value taken until no ceiling can beat the best: the best C by value, as `combos_of`
/// finds one). Her 8/8 is `lv[iv]` (`Env::veteran`: every target at the rank's star mix), valued as every fresh side is
/// (the trios' own 8/8s use the rank's mix too), so the trios with her compare like for like with the others; one per
/// call, not per star pattern (the full set's stars vary per leaf: per leaf it would be 4^nu searches for C).
/// (B's candidate index, partner side real, C's step)
fn her_trios(
    s: &mut Sim,
    d: &Data,
    bs: &ScoreBase,
    p8: &mut Pool,
    iv: usize,
    x: usize,
    buf: &mut Vec<f64>,
) -> Vec<(usize, bool, f64)> {
    let her_id = d.umas[x].id;
    let mut out = vec![];
    // the pool with her (`her_pool`: `Env::pool` of the legacies and hers), with each side pushed in turn (the same Pool
    // as `Env::pool` of those and the side) and popped after
    let il = p8.me.len();
    let (mut q, mut cs): (Vec<f64>, Vec<(f64, usize)>) = (Vec::with_capacity(s.env.ntar), vec![]);
    for ib in 0..bs.cands.len() {
        let pu = bs.cands[ib];
        if d.umas[pu].id == her_id {
            continue;
        }
        for (real, leg) in bs.sides.get(ib).map_or(&[][..], |v| &**v) {
            s.env.pool_push(p8, leg);
            // (C ranges over the trainees on one pair: `lineage` and `ceiling` per trainee, not `gen_cands`, whose
            // per-trainee tables cover every legacy of the pool: on p16 +4.5% instructions in all)
            cs.clear();
            cs.extend(
                bs.all
                    .iter()
                    .copied()
                    .filter(|&cu| {
                        d.umas[cu].id != her_id && d.umas[cu].id != d.umas[pu].id && legal_pair(d, &*p8, cu, iv, il)
                    })
                    .filter_map(|cu| {
                        let c = s.env.lineage_into(&*p8, cu, iv, il, &mut q);
                        s.env.ceiling(cu, &q, c).map(|ub| (ub, cu))
                    }),
            );
            cs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut best = 0.0f64;
            for &(ub, cu) in &cs {
                if ub * (1.0 + 1e-9) <= best {
                    break;
                }
                let v = val(s, &*p8, cu, iv, il, buf);
                if v > best {
                    best = v;
                }
            }
            if best > 0.0 {
                out.push((ib, *real, 1.0 / best));
            }
            Env::pool_pop(p8);
        }
    }
    out
}
/// V after her with her own 8/8 a trio side (`her_trios`), at the candidates' step costs c1 (a real partner side free)
fn with_her(v: f64, her: &[(usize, bool, f64)], c1: &[f64]) -> f64 {
    her.iter().map(|&(ib, real, c3)| (if real { 0.0 } else { c1[ib] }) + c3).fold(v, f64::min)
}
/// Career x's place among the stage candidates (`xi`) and whether its parents a, b are both 8/8s (the goal: her own 8/8 a
/// fully looped parent, V after her 0): what `score_career`, `outcome_hi` and `pick_her` read of her first
fn her_ends(s: &Sim, bs: &ScoreBase, legs: &[Legacy], x: usize, a: usize, b: usize) -> (Option<usize>, bool) {
    (bs.cands.iter().position(|&u| u == x), is_full(s.env.sj, &legs[a].sparks) && is_full(s.env.sj, &legs[b].sparks))
}
/// Per mask fm of the scenario-guaranteed targets landed (bit j: `fixed_t[j]`), its landed set over the job targets: an
/// outcome's cell is leaf << nf | fm, its landed set the leaf's (`leaf_set`) | this (`score_career`, `outcome_hi`)
fn fm_masks(fixed_t: &[usize]) -> Vec<usize> {
    (0..(1usize << fixed_t.len()))
        .map(|fm| {
            let mut m = 0;
            for (j, &i) in fixed_t.iter().enumerate() {
                if fm >> j & 1 == 1 {
                    m |= 1 << i;
                }
            }
            m
        })
        .collect()
}
/// `Env::pool` of `legs` and her veteran (career x with parents a, b, schedule `c`): with `of_legs` (`pool` is
/// `Env::pool(legs)`) `pool` with her entry pushed, the same Pool (`Env::pool_push` builds an entry as `Env::pool` does)
fn her_pool(s: &mut Sim, legs: &[Legacy], pool: &Pool, of_legs: bool, x: usize, a: usize, b: usize, c: &Cur) -> Pool {
    let veteran = s.env.veteran(legs, x, a, b, c.wp.to_vec());
    if of_legs {
        debug_assert_eq!(pool.me.len(), legs.len(), "one legacy per pool member");
        let mut extended = s.take_pool(pool);
        s.env.pool_push(&mut extended, &veteran);
        extended
    } else {
        let mut extended = legs.to_vec();
        extended.push(veteran);
        s.env.pool(&extended)
    }
}
/// `her_pool`'s pool done with: with `of_legs`, her entry popped and the working copy kept (`Sim::put_pool`)
fn her_pool_done(s: &mut Sim, pool: &Pool, of_legs: bool, mut pv: Pool) {
    if of_legs {
        Env::pool_pop(&mut pv);
        s.put_pool(pool, pv);
    }
}
/// Prepared scoring retained for the chosen career's later rolls.
struct KeptScoring {
    scored: Scored,
    /// At least one all-targets-landed outcome has a saving below 0.5; used when preparing the roll bound.
    full_landing_below_threshold: bool,
}

/// Named outputs of career scoring. `Want` determines which fields are computed; Stats leaves the two
/// scalar fields at zero, Score leaves `lift_stats` empty, Touch returns zeros everywhere (no evaluation), and only Keep
/// retains the scoring tables.
pub(super) struct CareerScore {
    /// Expected reduction in remaining careers, before subtracting the current career's cost.
    expected_saving: f64,
    improvement_chance: f64,
    kept: Option<KeptScoring>,
    /// Per candidate: probability of improvement and probability-weighted improved odds.
    pub(super) lift_stats: Vec<(f64, f64)>,
}

/// Score of career x (parents a, b of `legs`, deck `sel`) against `bs`: E[V0 - V0 with her veteran], the chance it is
/// above 0, the scoring kept for a roll (`Want::Keep`: `Scored`, with whether an outcome landing every target reads y < 0.5), and per candidate (P(her veteran lifts her), the summed lifted odds): what `want` asks for (`Want`). Exact over her landed set (`outcome_sets`) and her stars per uncertain target (the
/// rank's mix): her veteran as a parent of each stage-1 candidate (her best IDX_PARTNERS partners, the trainee's cached
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
///
/// `pool` must be `Env::pool(legs)` (the planner's callers): the pool with her veteran is then that pool with her entry
/// pushed (`Env::pool_push` builds an entry as `Env::pool` does). `score_career_of` takes any pool.
pub(super) fn score_career(
    s: &mut Sim,
    d: &Data,
    bs: &ScoreBase,
    legs: &[Legacy],
    pool: &Pool,
    x: usize,
    a: usize,
    b: usize,
    sel: &Sel,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = score_career_(s, d, bs, legs, pool, true, x, a, b, sel, want, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}
/// `score_career` for a pool built otherwise (`sim`'s): the pool with her veteran built from `legs` and her
pub(super) fn score_career_of(
    s: &mut Sim,
    d: &Data,
    bs: &ScoreBase,
    legs: &[Legacy],
    pool: &Pool,
    x: usize,
    a: usize,
    b: usize,
    sel: &Sel,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = score_career_(s, d, bs, legs, pool, false, x, a, b, sel, want, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}
fn score_career_(
    s: &mut Sim,
    d: &Data,
    bs: &ScoreBase,
    legs: &[Legacy],
    pool: &Pool,
    of_legs: bool,
    x: usize,
    a: usize,
    b: usize,
    sel: &Sel,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let n = bs.cands.len();
    let ntar = s.env.ntar;
    let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone();
    let fixed_t = s.env.fixed_t.clone();
    let w = s.env.sj.star_dist;
    let (xi, goal) = her_ends(s, bs, legs, x, a, b);
    let t_mc = now_ms();
    let c = s.make_cur(pool, x, a, b, sel, false);
    let t_lv = now_ms();
    prof("sc_cur_ms", t_lv - t_mc);
    let mut pv = her_pool(s, legs, pool, of_legs, x, a, b, &c);
    let iv = pv.me.len() - 1;
    // where she can be a parent: per stage-1 candidate (stage 1 still to run) her best partners
    let t_su = now_ms();
    prof("sc_pool_ms", t_su - t_lv);
    let mut setups: Vec<Setup> = vec![];
    for ui in 0..n {
        let u = bs.cands[ui];
        if d.umas[u].id == d.umas[x].id || bs.c1[ui] == 0.0 || !bs.c1[ui].is_finite() {
            continue;
        }
        // her best IDX_PARTNERS (2) partners by value, ties in partner order: every legal partner valued in order (each
        // `val` as before), the two kept by strict comparisons, as the stable sort's first two
        const _: () = assert!(IDX_PARTNERS == 2, "the partner pick below keeps exactly two");
        let mut ps: [Option<(f64, usize)>; 2] = [None, None];
        for y in 0..iv {
            if !legal_pair(d, &pv, u, y, iv) {
                continue;
            }
            let v = val(s, &pv, u, y, iv, buf);
            match ps {
                [None, _] => ps[0] = Some((v, y)),
                [Some(b1), _] if v > b1.0 => {
                    ps[1] = ps[0];
                    ps[0] = Some((v, y));
                }
                [_, None] => ps[1] = Some((v, y)),
                [_, Some(b2)] if v > b2.0 => ps[1] = Some((v, y)),
                _ => {}
            }
        }
        for (_, y) in ps.into_iter().flatten() {
            let Some(hs) = s.env.her_slot(&pv, u, y, iv) else { continue };
            let nd = s.cache.decks[u].len();
            if s.cache.decks[u].iter().all(|dk| dk.tab.is_empty()) {
                continue;
            }
            setups.push(Setup { ui, x0: hs.x0, pw: hs.pw, xm: hs.xms, nd });
        }
    }
    prof("sc_setup_ms", now_ms() - t_su);
    let t_fo = now_ms();
    // per setup, per star pattern of the uncertain targets (base 4, position k = uncv[k]; 0 absent), the best table value
    // at scale 1: the tables folded one coordinate at a time (interp's order and arithmetic), every setup's tables at once
    // (`fold_rec`); per outcome (leaf, guaranteed golds fm) and lifted candidate, that value times the setup's spark scale,
    // the best over her setups and her odds now (`fold_emit`): the products and maxima the outcome loop took per outcome
    let nf = fixed_t.len();
    let fmm = fm_masks(&fixed_t);
    // the candidates with setups (candidate order): her slot per candidate, per setup, and odds now per slot
    let mut aslot: Vec<Option<usize>> = vec![None; n];
    let mut p0: Vec<f64> = vec![];
    for st in &setups {
        if aslot[st.ui].is_none() {
            aslot[st.ui] = Some(p0.len());
            p0.push(bs.pnow[st.ui]);
        }
    }
    let na = p0.len();
    let sslot: Vec<usize> = setups.iter().map(|st| aslot[st.ui].unwrap()).collect();
    // what the evaluation reads (`Scored`; the spark scales per setup and landed set m: the product the loop took per
    // outcome, the same order, `Scored::derive`)
    let mut sd = Scored {
        nu,
        nf,
        ntar,
        na,
        xi,
        goal,
        w,
        uncv,
        fmm,
        x0: setups.iter_mut().map(|st| std::mem::take(&mut st.x0)).collect(),
        xm: setups.iter().map(|st| st.xm).collect(),
        pw: setups.iter_mut().map(|st| std::mem::take(&mut st.pw)).collect(),
        tsi: vec![],
        t0: vec![],
        col: vec![true; nu],
        sslot,
        p0,
        os: c.os.clone(),
        act: vec![],
        her: vec![],
        xs: vec![],
        xc: vec![],
        scf: vec![],
    };
    sd.derive();
    // the kept tables and, per uncertain target, collapsed (`sc_prep`; with no setup slot no outcome reads a table: every
    // target)
    if na > 0 {
        sc_prep(s, bs, &setups, &mut sd);
    }
    prof("sc_fold_ms", now_ms() - t_fo);
    // the candidates the loop below can lift, in candidate order: those with setups, and (never, as the odds are not
    // negative) one whose odds now already clear her threshold pnow (1 + 1e-9); every other keeps up = pnow, below it
    let thr: Vec<f64> = (0..n).map(|ui| bs.pnow[ui] * (1.0 + 1e-9)).collect();
    let active: Vec<usize> = (0..n).filter(|&ui| aslot[ui].is_some() || bs.pnow[ui] > thr[ui]).collect();
    // her own 8/8 as a trio side when her uma is not a stage candidate (the full set only; `her_trios`)
    let full_m = (1usize << ntar) - 1;
    let t_h = now_ms();
    sd.her = if xi.is_none() && !goal && !bs.sides.is_empty() && c.os[full_m] > 0.0 {
        her_trios(s, d, bs, &mut pv, iv, x, buf)
    } else {
        vec![]
    };
    prof("sc_her_ms", now_ms() - t_h);
    // per active candidate: (index, her slot or none, her threshold, her odds now)
    sd.act = active.iter().map(|&ui| (ui, aslot[ui], thr[ui], bs.pnow[ui])).collect();
    her_pool_done(s, pool, of_legs, pv);
    if want == Want::Touch {
        return CareerScore {
            expected_saving: 0.0,
            improvement_chance: 0.0,
            kept: None,
            lift_stats: vec![(0.0, 0.0); n],
        };
    }
    let mut sc = FOLD_SCRATCH.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let (ey, lift, _, stats, full_lo, [n_out, n_ch, n_cost, n_same, n_one]) =
        sc_eval(bs, &sd, &All, if want == Want::Keep { Want::Score } else { want }, &mut sc);
    FOLD_SCRATCH.with(|c| *c.borrow_mut() = sc);
    prof("sc_out", n_out as f64);
    prof("sc_trio", n_ch as f64);
    prof("sc_cost", n_cost as f64);
    prof("sc_same", n_same as f64);
    prof("sc_one", n_one as f64);
    CareerScore {
        expected_saving: ey,
        improvement_chance: lift,
        kept: (want == Want::Keep).then_some(KeptScoring { scored: sd, full_landing_below_threshold: full_lo }),
        lift_stats: stats,
    }
}
/// A scoring's evaluation (`Scored`) at the leaves `lv` produces: the fold of its tables into the outcome rows (with na >
/// 0 setup slots: `fold_rec`), then the outcome loop (`sc_outcomes`): what `sc_outcomes` returns, with whether an outcome
/// that lands every target reads y < 0.5 (with y). The plan's scorings take `All`; a roll `OneLeaf` (`sc_roll`), the same
/// arithmetic in the same order at the leaves it produces
#[inline(never)]
fn sc_eval<L: Leaves>(
    bs: &ScoreBase,
    sd: &Scored,
    lv: &L,
    want: Want,
    sc: &mut FoldScratch,
) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>, bool, [usize; 5]) {
    let (nu, nf, ns, na) = (sd.nu, sd.nf, sd.ns(), sd.na);
    let pl = sd.pl();
    let nr = lv.nrows(1usize << (2 * nu));
    let nrow = if pl { nr * ns } else { (nr << nf) * na };
    // (reused across calls: every row is written before it is read)
    if sc.u.len() < nrow {
        sc.u.resize(nrow, 0.0);
    }
    if na > 0 {
        let nt = sd.tsi.len();
        if sc.lv.len() < nu + 1 {
            sc.lv.resize(nu + 1, vec![]);
        }
        for k in 1..=nu {
            let l = nt << (nu - k);
            if sc.lv[k].len() < l {
                sc.lv[k].resize(l, 0.0);
            }
        }
        if sc.fv.len() < 16 * ns {
            sc.fv.resize(16 * ns, 0.0);
        }
        if sc.acc.len() < 16 * na {
            sc.acc.resize(16 * na, 0.0);
        }
        let cx = FoldCx {
            pl,
            nu,
            nf,
            ns,
            na,
            t0: &sd.t0,
            tsi: &sd.tsi,
            xs: &sd.xs,
            xc: &sd.xc,
            uncv: &sd.uncv,
            fmm: &sd.fmm,
            scf: &sd.scf,
            sslot: &sd.sslot,
            p0: &sd.p0,
            col: &sd.col,
            two: [0, 1],
            lv,
        };
        let FoldScratch { lv: lvb, fv, acc, u, .. } = sc;
        let t_fo = now_ms();
        fold_rec(&cx, 0, 0, 0, lvb, fv, &mut acc[..16 * na], &mut u[..nrow]);
        prof("sc_fold_ms", now_ms() - t_fo);
    }
    let FoldScratch { u, pst, mus, pkey, yb, lb, lf, est, .. } = sc;
    let t_lo = now_ms();
    let r = sc_outcomes(bs, sd, lv, &u[..nrow], want, (pst, mus, pkey), yb, lb, lf, est);
    prof("sc_loop_ms", now_ms() - t_lo);
    r
}
/// A roll's y (`roll_answer`): the kept scoring evaluated at one outcome (her stars `leaf`, the guaranteed targets
/// landed `fm`), `sc_eval` at `OneLeaf` (the leaf and its representative); 0 where the outcome can't happen, as the
/// plan's full table held
fn sc_roll(bs: &ScoreBase, sd: &Scored, leaf: usize, fm: usize) -> f64 {
    let lv = OneLeaf::new(leaf, sd.rep_of(leaf));
    let mut sc = FOLD_SCRATCH.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let (_, _, ys, _, _, _) = sc_eval(bs, sd, &lv, Want::Ys, &mut sc);
    FOLD_SCRATCH.with(|c| *c.borrow_mut() = sc);
    ys[lv.row(leaf) << sd.nf | fm]
}

/// `score_career_`'s tables (with na > 0 setup slots): every setup's tables, the dominated ones dropped, as level 0 of the
/// fold (`sd.t0`, table j at j 2^nu, its setup `sd.tsi[j]`); per uncertain target whether it is collapsed (below,
/// `sd.col`). Its own function, entered once per scoring, so V8 compiles it optimised after the first few
/// (`docs/PERF.md`, *In the page*).
///
/// Collapse: target k is collapsed when, for every setup, either her x is the same f64 at 1, 2 and 3 stars (x0 = 0: then
/// the fold takes the same f64s at those levels, so it gives the same values bit for bit), or every kept table of the
/// setup has the same value with target k landed and not (`t[i] == t[i | 1 << k]`): the table's interpolation then reads
/// no x_k, t (1 - x) + t x = t, and the fold's f64s differ by at most rounding (a few units in the last place) between
/// her star levels. A collapsed target is folded at 0 and 1 star only; the outcome loop reads 1 star's values for her 2
/// and 3 stars there (`sc_outcomes`). Every value folded is the fold's value at a leaf of the
/// full fold (the same operations on the same f64s), so the dominated-table drop stays exact at every leaf folded; and
/// the tables dropped are none that could set a maximum, so deciding the collapse on the kept tables decides it on the
/// values the outcome loop reads.
#[inline(never)]
fn sc_prep(s: &Sim, bs: &ScoreBase, setups: &[Setup], sd: &mut Scored) {
    let nu = sd.nu;
    let (xs, xc) = (&sd.xs, &sd.xc);
    // every setup's tables, level 0 of the fold (table j at j 2^nu), and the setup of each
    let (mut tsi, mut t0): (Vec<usize>, Vec<f64>) = (vec![], vec![]);
    let mut n_dom = 0usize;
    for (si, st) in setups.iter().enumerate() {
        let tabs: Vec<&Vec<f64>> =
            s.cache.decks[bs.cands[st.ui]][..st.nd].iter().flat_map(|dk| dk.tab.iter()).collect();
        // a table at most another everywhere (an equal one: the later) folds to at most that one's value at every leaf
        // (every fold coefficient is >= 0 and rounding is monotone), so the maximum is the same without it
        let nonneg = xs[si].iter().chain(&xc[si]).all(|r| r.iter().all(|&v| v >= 0.0));
        for (j, tab) in tabs.iter().enumerate() {
            debug_assert_eq!(tab.len(), 1usize << nu, "one table entry per subset of the uncertain targets");
            if nonneg
                && tabs.iter().enumerate().any(|(k, o)| {
                    k != j
                        && tab.iter().zip(o.iter()).all(|(a, b)| a <= b)
                        && (k < j || tab.iter().zip(o.iter()).any(|(a, b)| a < b))
                })
            {
                n_dom += 1;
                continue;
            }
            t0.extend_from_slice(tab);
            tsi.push(si);
        }
    }
    prof("sc_dom", n_dom as f64);
    let nt = tsi.len();
    // the collapse (above): per target, every setup's x the same at 1-3 stars, or her tables the same either side of bit k
    let n0 = 1usize << nu;
    let mut col = vec![true; nu];
    let mut n_tab = 0usize;
    for (k, ck) in col.iter_mut().enumerate() {
        let mut by_tab = false;
        for (si, xsi) in xs.iter().enumerate() {
            let r = &xsi[k];
            if r[1].to_bits() == r[2].to_bits() && r[2].to_bits() == r[3].to_bits() {
                continue;
            }
            let flat = (0..nt).filter(|&j| tsi[j] == si).all(|j| {
                let t = &t0[j * n0..(j + 1) * n0];
                (0..n0).all(|i| i >> k & 1 == 1 || t[i] == t[i | 1 << k])
            });
            if !flat {
                *ck = false;
                break;
            }
            by_tab = true;
        }
        if *ck && by_tab {
            n_tab += 1;
        }
    }
    prof("sc_col", col.iter().filter(|&&c| c).count() as f64);
    prof("sc_col_tab", n_tab as f64);
    (sd.tsi, sd.t0, sd.col) = (tsi, t0, col);
}
/// `score_career_`'s outcome loop: every outcome `lv` produces (her stars per uncertain target, the guaranteed golds
/// present or not), its chance, V after her and y; (E[y], P(y > 1e-9), y per outcome (at its leaf's row), per candidate
/// stats: what `want` asks for; whether an outcome landing every target reads y < 0.5; counters). Its own function,
/// entered once per scoring, so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*).
/// On a collapsed target (`col`, `sc_prep`: folded at 0 and 1 star only) a leaf with 2 or 3 stars there reads its
/// representative's values (`rep_of`: those digits set to 1, so a smaller leaf, met first): y as computed there (`yb`) and
/// each candidate's odds for the stats; every sum (E[y], the lift, the stats) still takes every outcome in the loop's order
/// at its own chance, the same sums as without the collapse wherever the representative's values are the leaf's own.
///
/// What is fixed per call is decided here, once: the loop (`sc_loop`) is specialised by whether it takes the products
/// itself (pl), whether any target is collapsed and what is wanted (y or the stats), and the candidates it can lift are
/// split into those with a slot (slot j is the j-th of them: slots are given in candidate order) and those without one
/// (odds fixed at their odds now)
#[inline(never)]
fn sc_outcomes<L: Leaves>(
    bs: &ScoreBase,
    sd: &Scored,
    lv: &L,
    u: &[f64],
    want: Want,
    (pst, mus, pkey): (&mut Vec<f64>, &mut Vec<usize>, &mut Vec<u64>),
    yb: &mut Vec<f64>,
    lb: &mut Vec<u32>,
    lf: &mut Vec<(usize, f64)>,
    eb: &mut EstBufs,
) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>, bool, [usize; 5]) {
    let n = bs.cands.len();
    let (nu, na, w) = (sd.nu, sd.na, sd.w);
    // V after her when one candidate's step alone moved (`one_of`, `one_v`: trio_v's minimum, split by the trios that read
    // her step)
    let one: Vec<One> = if bs.combos.is_empty() || want == Want::Stats {
        vec![]
    } else {
        (0..n).map(|i| one_of(bs.combos, bs.c1, i)).collect()
    };
    // per leaf (at its row) her landed set on the uncertain targets and her stars' chance: the product over k of w[her
    // stars on target k - 1] where she has any, in k order from 1.0 (every leaf: the leaves with digit k > 0 extend those
    // below 4^k; else per leaf, `leaf_set`, the same f64s)
    // (they depend on w and uncv alone: built for every leaf, they are kept until those change, `pkey`)
    if L::ALL {
        let same = pkey.len() == 3 + nu
            && pkey[..3].iter().zip(&w).all(|(&k, x)| k == x.to_bits())
            && pkey[3..].iter().zip(&sd.uncv).all(|(&k, &i)| k == i as u64);
        if !same {
            pst.clear();
            mus.clear();
            pst.push(1.0);
            mus.push(0usize);
            for k in 0..nu {
                let lo = 1usize << (2 * k);
                for dg in 1..4usize {
                    for i in 0..lo {
                        pst.push(pst[i] * w[dg - 1]);
                        mus.push(mus[i] | 1 << sd.uncv[k]);
                    }
                }
            }
            pkey.clear();
            pkey.extend(w.iter().map(|x| x.to_bits()));
            pkey.extend(sd.uncv.iter().map(|&i| i as u64));
        }
    } else {
        pkey.clear();
        pst.clear();
        mus.clear();
        for &l in lv.list() {
            let (mu, p) = leaf_set(l, &sd.uncv, w);
            pst.push(p);
            mus.push(mu);
        }
    }
    // the collapsed targets' digits (cm) and their low bits: a leaf's representative has each nonzero one at 1
    let cm: usize = (0..nu).filter(|&k| sd.col[k]).map(|k| 3usize << (2 * k)).sum();
    let lo_bits = cm & (usize::MAX / 3);
    // the candidates it can lift: with a slot (in slot order) and without
    let (mut uis, mut ths, mut actn) = (vec![], vec![], vec![]);
    for &(ui, sl, th, pn) in &sd.act {
        match sl {
            Some(sl) => {
                assert_eq!(sl, uis.len(), "slotted candidates come in slot order");
                uis.push(ui);
                ths.push(th);
            }
            None => actn.push((ui, th, pn)),
        }
    }
    assert_eq!(uis.len(), na, "one slotted candidate per slot");
    // per slot its setups: sslot is ascending (setups are made in candidate order), so slot j's are rg[j]..rg[j + 1]
    let mut rg = vec![0usize; na + 1];
    for (si, &sl) in sd.sslot.iter().enumerate() {
        assert!(si == 0 || sd.sslot[si - 1] <= sl, "setups come in slot order");
        rg[sl + 1] = si + 1;
    }
    // V after her found lazily (`sc_loop`): no step, stage 2-3 cost or C's step NaN, -0.0 or below 0, so no sum of them is
    // NaN or -0.0 and equal minima are the same bits
    let ok = |c: f64| c >= 0.0 && c.to_bits() != (-0.0f64).to_bits();
    let lazy = bs.c1.iter().all(|&c| ok(c)) && bs.m23.iter().all(|&c| ok(c)) && bs.combos.iter().all(|t| ok(t.2));
    let cx = OutCx {
        bs,
        os: &sd.os,
        fmm: &sd.fmm,
        u,
        scf: &sd.scf,
        rg: &rg,
        p0: &sd.p0,
        uis: &uis,
        ths: &ths,
        actn: &actn,
        her: &sd.her,
        xi: sd.xi,
        goal: sd.goal,
        want_ys: want == Want::Ys,
        nf: sd.nf,
        ns: sd.ns(),
        na,
        full_m: sd.full_m(),
        cm,
        lo_bits,
        pst,
        mus,
        one: &one,
        lazy,
        nleaf: 1usize << (2 * nu),
        lv,
    };
    // (Y: y and its sums, S: the stats)
    match (sd.pl(), cm != 0, want == Want::Stats) {
        (false, false, false) => sc_loop::<false, false, true, false, L>(&cx, yb, lb, lf, eb),
        (false, true, false) => sc_loop::<false, true, true, false, L>(&cx, yb, lb, lf, eb),
        (true, false, false) => sc_loop::<true, false, true, false, L>(&cx, yb, lb, lf, eb),
        (true, true, false) => sc_loop::<true, true, true, false, L>(&cx, yb, lb, lf, eb),
        (false, false, true) => sc_loop::<false, false, false, true, L>(&cx, yb, lb, lf, eb),
        (false, true, true) => sc_loop::<false, true, false, true, L>(&cx, yb, lb, lf, eb),
        (true, false, true) => sc_loop::<true, false, false, true, L>(&cx, yb, lb, lf, eb),
        (true, true, true) => sc_loop::<true, true, false, true, L>(&cx, yb, lb, lf, eb),
    }
}
/// What `sc_loop` reads (`sc_outcomes`): per slot its setups (rg), candidate and threshold; the candidates without a slot
/// (index, threshold, odds now); the collapsed digits (cm, lo_bits); per leaf (at its row) her stars' chance and landed
/// set; whether V after her may be found lazily; the leaves taken (`lv`, of nleaf)
struct OutCx<'out, 'base, L: Leaves> {
    bs: &'out ScoreBase<'base>,
    os: &'out [f64],
    fmm: &'out [usize],
    u: &'out [f64],
    scf: &'out [f64],
    rg: &'out [usize],
    p0: &'out [f64],
    uis: &'out [usize],
    ths: &'out [f64],
    actn: &'out [(usize, f64, f64)],
    her: &'out [(usize, bool, f64)],
    xi: Option<usize>,
    goal: bool,
    want_ys: bool,
    nf: usize,
    ns: usize,
    na: usize,
    full_m: usize,
    cm: usize,
    lo_bits: usize,
    pst: &'out [f64],
    mus: &'out [usize],
    one: &'out [One],
    lazy: bool,
    nleaf: usize,
    lv: &'out L,
}
/// Each slot's odds after her at row r (outcome m, fm): with PL the best of her odds now and her setups' spark scale times
/// table value, per slot over its setups in setup order (the same strict maxima `fold_emit` takes), else the folded row
#[inline(always)]
fn sc_row<'out, const PL: bool, L: Leaves>(
    o: &'out OutCx<L>,
    r: usize,
    m: usize,
    fm: usize,
    ubuf: &'out mut [f64],
) -> &'out [f64] {
    if PL {
        let (fr, sr) = (&o.u[r * o.ns..][..o.ns], &o.scf[m * o.ns..][..o.ns]);
        for ((b, &p), w) in ubuf.iter_mut().zip(o.p0).zip(o.rg.windows(2)) {
            let mut best = p;
            for (&sc, &f) in sr[w[0]..w[1]].iter().zip(&fr[w[0]..w[1]]) {
                let v = sc * f;
                if v > best {
                    best = v;
                }
            }
            *b = best;
        }
        ubuf
    } else {
        &o.u[(r << o.nf | fm) * o.na..][..o.na]
    }
}
/// `sc_outcomes`' loop, specialised: PL (the products taken here, `sc_row`), COL (a target collapsed), Y (y and its sums
/// computed; else E[y] and the lift 0.0, no step costed), S (the stats; else none), L (the leaves taken, in order; rows,
/// y and the representatives' records at each leaf's row). With S a representative outcome that
/// other leaves read records the candidates it lifts and their odds (`lf` from `lb[outcome]`, ended by usize::MAX;
/// u32::MAX: not recorded, past the cap), which the leaves reading it add to their stats (each candidate's sums still in
/// outcome order: the same f64s); unrecorded, they take its row again
#[inline(never)]
fn sc_loop<const PL: bool, const COL: bool, const Y: bool, const S: bool, L: Leaves>(
    o: &OutCx<L>,
    yb: &mut Vec<f64>,
    lb: &mut Vec<u32>,
    lf: &mut Vec<(usize, f64)>,
    eb: &mut EstBufs,
) -> (f64, f64, Vec<f64>, Vec<(f64, f64)>, bool, [usize; 5]) {
    let bs = o.bs;
    let n = bs.cands.len();
    let (nf, na, full_m) = (o.nf, o.na, o.full_m);
    let (uis, ths, actn, os, fmm) = (o.uis, o.ths, o.actn, o.os, o.fmm);
    let (nleaf, lv) = (o.nleaf, o.lv);
    let nr = lv.nrows(nleaf);
    // every outcome: uncertain targets' star pattern, guaranteed golds present or not
    let (mut ey, mut lift) = (0.0f64, 0.0f64);
    // y per outcome (careers saved), at row << nf | fm (0 where the outcome can't happen); only when the caller reads it
    let mut ys: Vec<f64> = if o.want_ys { vec![0.0; nr << nf] } else { vec![] };
    // an outcome that can happen, lands every target and reads y < 0.5 (`pick_her`)
    let mut full_lo = false;
    let mut c1 = bs.c1.to_vec();
    // per stage-1 candidate: the chance her veteran raises that candidate's odds, and the odds raised to (summed, weighted)
    let mut stats = if S { vec![(0.0f64, 0.0f64); n] } else { vec![] };
    let mut ch: Vec<usize> = vec![];
    let mut n_out = 0usize;
    let mut ubuf = vec![0.0f64; na];
    // V after her (`v_after` at the estimate's step costs, `Est`), from the candidates each outcome lifts (row, candidate,
    // odds: `lifted`)
    let mut est = if Y { Est::new(bs, na + actn.len(), o.lazy, o.one, std::mem::take(eb)) } else { Est::default() };
    let mut lifted: Vec<(usize, usize, f64)> = vec![];
    let (cm, lo_bits) = (o.cm, o.lo_bits);
    let rep_of = |l: usize| {
        let z = l & cm;
        (l & !cm) | ((z | z >> 1) & lo_bits)
    };
    // (the records' cap: about twice yb's memory)
    let lf_cap = (nleaf << nf).clamp(1 << 16, u32::MAX as usize - 1);
    // (yb and lb are only grown: an entry is read (rx below) only at a leaf r != leaf whose outcome passed the chance test
    // (pr > 0: os[m] > 0), and its representative r (a smaller leaf: met first) has the same landed set (a collapsed
    // digit 2 or 3 is 1 there, still landed), so m; r has a collapsed digit set (`read`), so with os[m] > 0 it passed
    // the test and wrote both entries, y (Y) and its record or u32::MAX (S), this call)
    if COL {
        if Y && yb.len() < nr << nf {
            yb.resize(nr << nf, 0.0);
        }
        if S {
            if lb.len() < nr << nf {
                lb.resize(nr << nf, u32::MAX);
            }
            lf.clear();
            lf.push((usize::MAX, 0.0));
        }
    }
    let nl = if L::ALL { nleaf } else { lv.list().len() };
    for li in 0..nl {
        let leaf = if L::ALL { li } else { lv.list()[li] };
        let lr = lv.row(leaf);
        let (mu, pr_st) = (o.mus[lr], o.pst[lr]);
        let r = if COL { rep_of(leaf) } else { leaf };
        // (a representative that other leaves read: y computed wherever her landed set can happen, even at chance 0 itself)
        let read = COL && r == leaf && leaf & cm != 0;
        for (fm, &mf) in fmm.iter().enumerate() {
            let m = mu | mf;
            let pr = os[m] * pr_st;
            if pr <= 0.0 && !(read && os[m] > 0.0) {
                continue;
            }
            if COL && r != leaf {
                // the representative's y, the stats at the representative's odds and this outcome's chance
                n_out += 1;
                let rr = lv.row(r);
                let rx = rr << nf | fm;
                if S {
                    let s0 = lb[rx];
                    if s0 == u32::MAX {
                        let ur = sc_row::<PL, L>(o, rr, m, fm, &mut ubuf);
                        for ((&upv, &th), &ui) in ur.iter().zip(ths).zip(uis) {
                            if upv > th {
                                stats[ui].0 += pr;
                                stats[ui].1 += pr * upv;
                            }
                        }
                        for &(ui, th, pn) in actn {
                            if pn > th {
                                stats[ui].0 += pr;
                                stats[ui].1 += pr * pn;
                            }
                        }
                    } else {
                        for &(ui, upv) in lf[s0 as usize..].iter().take_while(|e| e.0 != usize::MAX) {
                            stats[ui].0 += pr;
                            stats[ui].1 += pr * upv;
                        }
                    }
                }
                if Y {
                    let y = yb[rx];
                    ey += pr * y;
                    if y > 1e-9 {
                        lift += pr;
                    }
                    if m == full_m && y < 0.5 {
                        full_lo = true;
                    }
                    if o.want_ys {
                        ys[lr << nf | fm] = y;
                    }
                }
                continue;
            }
            // each candidate's odds after her: the best of her setups' and her odds now (`fold_emit`; with pl, here)
            let ur = sc_row::<PL, L>(o, lr, m, fm, &mut ubuf);
            let live = pr > 0.0;
            if live {
                n_out += 1;
            }
            // (a representative records what it lifts, within the cap)
            let rec = S && read && lf.len() + na + actn.len() < lf_cap;
            let s0 = lf.len();
            // the candidates it lifts: their stats and records here, V after her from the list (in this order)
            lifted.clear();
            macro_rules! lifted {
                ($ci:expr, $ui:expr, $upv:expr) => {{
                    let (ui, upv): (usize, f64) = ($ui, $upv);
                    if S && live {
                        stats[ui].0 += pr;
                        stats[ui].1 += pr * upv;
                    }
                    if rec {
                        lf.push((ui, upv));
                    }
                    if Y {
                        lifted.push(($ci, ui, upv));
                    }
                }};
            }
            for (j, ((&upv, &th), &ui)) in ur.iter().zip(ths).zip(uis).enumerate() {
                if upv > th {
                    lifted!(j, ui, upv);
                }
            }
            for (j, &(ui, th, pn)) in actn.iter().enumerate() {
                if pn > th {
                    lifted!(na + j, ui, pn);
                }
            }
            if S && read {
                let ix = lr << nf | fm;
                lb[ix] = if !rec {
                    u32::MAX
                } else if lf.len() == s0 {
                    0
                } else {
                    lf.push((usize::MAX, 0.0));
                    s0 as u32
                };
            }
            if !Y {
                continue;
            }
            let y = v_after(bs, &mut est, &mut c1, &mut ch, &lifted, m, full_m, o.xi, o.her, o.goal);
            if read {
                yb[lr << nf | fm] = y;
            }
            if !live {
                continue;
            }
            ey += pr * y;
            if y > 1e-9 {
                lift += pr;
            }
            if m == full_m && y < 0.5 {
                full_lo = true;
            }
            if o.want_ys {
                ys[lr << nf | fm] = y;
            }
        }
    }
    if Y {
        *eb = EstBufs { last: std::mem::take(&mut est.last), cell: std::mem::take(&mut est.cell) };
    }
    (ey, lift, ys, stats, full_lo, [n_out, est.n_ch, est.n_cost, est.n_same, est.n_one])
}
/// V after her before her own 8/8 and the goal, at the candidates' step costs c1: the trios' least cost (`trio_v`; with no
/// trio, the least of each candidate's step plus her stages 2-3)
fn v0_after(bs: &ScoreBase, c1: &[f64]) -> f64 {
    if bs.combos.is_empty() {
        (0..bs.cands.len()).map(|a| c1[a] + bs.m23[a]).fold(f64::INFINITY, f64::min)
    } else {
        trio_v(c1, bs.combos)
    }
}
/// How V after her (`v_after`) costs the steps of the candidates her veteran lifts: the estimate's (`Est`, `Cont::cost`,
/// for y) or the lower bound's (`Lo`, `Cont::cost_lo`, for y_hi). Monomorphised: each path keeps its own code
trait StepCost {
    /// candidate ui lifted to odds p (row ci of the outcome's lifts): her step taken into c1 where it is lower (`ch`: the
    /// steps that moved), or left for `va`
    fn take(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, ci: usize, ui: usize, p: f64);
    /// candidate xi's step set to 0 (her own 8/8 on the full set)
    fn zeroed(&mut self, _xi: usize) {}
    /// V after her before her own 8/8's trios, from c1 (`her_all`: those trios follow, reading every step)
    fn va(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, her_all: bool) -> f64;
}
/// The bound's steps: each lifted candidate at `Cont::cost_lo` (`saving_lo`), V after her by `v0_after`
struct Lo;
impl StepCost for Lo {
    #[inline]
    fn take(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, _: usize, ui: usize, p: f64) {
        let cc = bs.cost_lo(ui, p);
        if cc < c1[ui] {
            c1[ui] = cc;
            ch.push(ui);
        }
    }
    #[inline]
    fn va(&mut self, bs: &ScoreBase, c1: &mut [f64], _: &mut Vec<usize>, _: bool) -> f64 {
        v0_after(bs, c1)
    }
}
/// The estimate's steps (`sc_loop`): a step's cost through a per-row cache of the odds costed (`last`, slot j at j, the
/// candidates without a slot after the slots; `bs.cost` is a pure function of them, direct-mapped by the odds' bits), V
/// after her lazily and through its shortcuts (no step moved: `v_same`; one: `one_v`), with the counters `sc_eval`
/// reports.
///
/// V after her found lazily (with `lazy`): a lifted candidate on Cont's plan path (`Cont::lb_ok`) is first given her
/// odds' cell bound lo (`Cont::cost_lb`, direct-mapped by cell, LC per candidate): lo >= her step, she can't lower it
/// (cost >= lo); below, she is pending. Then the trios (by their base value, ascending; with no trio, each candidate's
/// c1 + m23), each valued at lo for its pending candidates: one below the best so far has its pending candidates
/// costed (`cost`, as above) and its value taken. A trio passed over is at least that bound (lo <= the step she takes,
/// and rounded sums are monotone), so at least the minimum: every trio a pending candidate left uncosted reads is at
/// least the minimum, and with her step left at the base's (above the step she would take) still is, while every
/// trio at the minimum reads only costed steps. So the least trio value found is the minimum `v0_after` (or `one_v`)
/// takes over every trio, the same f64 (equal minima the same bits: `sc_outcomes`), and V after her. Her own 8/8's
/// trios (`her`) read every step: then all are costed, and V after her is taken as without the lazy pass
#[derive(Default)]
struct Est<'base> {
    last: Vec<(u64, f64)>,
    lbc: Vec<Option<&'base Cont>>,
    cell: Vec<(u64, f64)>,
    tord: Vec<usize>,
    one: &'base [One],
    v_same: f64,
    pend: Vec<(usize, usize, f64)>,
    lo: Vec<f64>,
    pf: Vec<bool>,
    n_ch: usize,
    n_cost: usize,
    n_same: usize,
    n_one: usize,
}
const EST_CC: usize = 256;
/// `Est`'s cache buffers between scorings (`FoldScratch`)
#[derive(Default)]
struct EstBufs {
    last: Vec<(u64, f64)>,
    cell: Vec<(u64, f64)>,
}
impl<'base> Est<'base> {
    /// for `rows` lift rows; `lazy` (`sc_outcomes`: every step, stage 2-3 cost and C's step a number >= 0, not -0.0)
    fn new(bs: &'base ScoreBase<'_>, rows: usize, lazy: bool, one: &'base [One], eb: EstBufs) -> Self {
        let n = bs.cands.len();
        let lbc: Vec<Option<&Cont>> = (0..n).map(|ui| bs.cont.get(ui).filter(|c| lazy && c.lb_ok())).collect();
        let tord: Vec<usize> = if lazy {
            let tv: Vec<f64> = bs.combos.iter().map(|t| trio_v(bs.c1, std::slice::from_ref(t))).collect();
            let mut t: Vec<usize> = (0..bs.combos.len()).collect();
            t.sort_by(|&a, &b| tv[a].total_cmp(&tv[b]));
            t
        } else {
            vec![]
        };
        // (V after her when no candidate's step moves, most outcomes lifting no one: once)
        // (the caches' buffers reused, refilled: NaN's bits mark an empty `last` slot, u64::MAX an empty cell)
        let EstBufs { mut last, mut cell } = eb;
        last.clear();
        last.resize(rows * EST_CC, (f64::NAN.to_bits(), 0.0));
        cell.clear();
        if lazy {
            cell.resize(n * LB_CELLS, (u64::MAX, 0.0));
        }
        Est {
            last,
            lbc,
            cell,
            tord,
            one,
            v_same: v0_after(bs, bs.c1),
            pend: vec![],
            lo: vec![0.0f64; n],
            pf: vec![false; n],
            n_ch: 0,
            n_cost: 0,
            n_same: 0,
            n_one: 0,
        }
    }
    /// her step at odds p: costed (through `last`), taken where it is lower
    #[inline(always)]
    fn cost(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, ci: usize, ui: usize, p: f64) {
        let ub = p.to_bits();
        let k = ci * EST_CC + (ub.wrapping_mul(0x9E37_79B9_7F4A_7C15) >> 56) as usize;
        let cc = if self.last[k].0 == ub {
            self.n_same += 1;
            self.last[k].1
        } else {
            self.n_cost += 1;
            let cc = bs.cost(ui, p);
            self.last[k] = (ub, cc);
            cc
        };
        if cc < c1[ui] {
            c1[ui] = cc;
            ch.push(ui);
        }
    }
    /// a pending step a trio at the minimum may read, costed
    #[inline(always)]
    fn settle(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, i: usize) {
        if self.pf[i] {
            self.pf[i] = false;
            let &(ci, _, p) = self.pend.iter().find(|e| e.1 == i).unwrap();
            self.cost(bs, c1, ch, ci, i, p);
        }
    }
}
impl StepCost for Est<'_> {
    #[inline(always)]
    fn take(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, ci: usize, ui: usize, p: f64) {
        let b = match self.lbc[ui] {
            Some(ct) => ct.cost_lb(p, (&mut self.cell[ui * LB_CELLS..(ui + 1) * LB_CELLS]).try_into().unwrap()),
            None => None,
        };
        match b {
            Some(b) if b >= c1[ui] => {}
            Some(b) if b < c1[ui] => {
                self.lo[ui] = b;
                self.pf[ui] = true;
                self.pend.push((ci, ui, p));
            }
            _ => self.cost(bs, c1, ch, ci, ui, p),
        }
    }
    #[inline(always)]
    fn zeroed(&mut self, xi: usize) {
        self.pf[xi] = false;
    }
    #[inline(always)]
    fn va(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, her_all: bool) -> f64 {
        let n = bs.cands.len();
        // (V after her where the lazy pass found it: the minimum)
        let mut va_l: Option<f64> = None;
        if !self.pend.is_empty() {
            if her_all {
                for k in 0..self.pend.len() {
                    let i = self.pend[k].1;
                    self.settle(bs, c1, ch, i);
                }
            } else if bs.combos.is_empty() {
                let mut best = f64::INFINITY;
                for a in 0..n {
                    if (if self.pf[a] { self.lo[a] } else { c1[a] }) + bs.m23[a] >= best {
                        continue;
                    }
                    self.settle(bs, c1, ch, a);
                    let v = c1[a] + bs.m23[a];
                    if v < best {
                        best = v;
                    }
                }
                va_l = Some(best);
            } else {
                let mut best = f64::INFINITY;
                for k in 0..self.tord.len() {
                    let (ia, ib, c3, ra, rb) = bs.combos[self.tord[k]];
                    let st = |s: &Self, i: usize, real: bool| {
                        if real {
                            0.0
                        } else if s.pf[i] {
                            s.lo[i]
                        } else {
                            c1[i]
                        }
                    };
                    if st(self, ia, ra) + st(self, ib, rb) + c3 >= best {
                        continue;
                    }
                    if !ra {
                        self.settle(bs, c1, ch, ia);
                    }
                    if !rb {
                        self.settle(bs, c1, ch, ib);
                    }
                    let v = (if ra { 0.0 } else { c1[ia] }) + (if rb { 0.0 } else { c1[ib] }) + c3;
                    if v < best {
                        best = v;
                    }
                }
                va_l = Some(best);
            }
            for k in 0..self.pend.len() {
                let i = self.pend[k].1;
                self.pf[i] = false;
            }
            self.pend.clear();
        }
        if !ch.is_empty() {
            self.n_ch += 1;
        }
        if let Some(v) = va_l {
            v
        } else if ch.is_empty() {
            self.v_same
        } else if ch.len() == 1 && !self.one.is_empty() && self.one[ch[0]].ok {
            self.n_one += 1;
            one_v(&self.one[ch[0]], c1[ch[0]])
        } else {
            v0_after(bs, c1)
        }
    }
}
/// The careers her veteran saves at one outcome (her landed set m), from the candidates it lifts (`lifted`: (row,
/// candidate, odds), in order) and the steps' costs `cost` (`StepCost`): each lifted step taken where it is lower, her
/// own step 0 on the full set (`xi`), V after her (`StepCost::va`), her own 8/8's trios (`her`) on the full set, and y =
/// V0 - V after (at least 0; the goal's V0 on the full set). c1 must hold the base's steps; it is put back. The one rule
/// of the estimate y (`sc_loop`, `Est`) and the bound y_hi (`saving_lo`, `Lo`)
#[inline(always)]
fn v_after<C: StepCost>(
    bs: &ScoreBase,
    cost: &mut C,
    c1: &mut [f64],
    ch: &mut Vec<usize>,
    lifted: &[(usize, usize, f64)],
    m: usize,
    full_m: usize,
    xi: Option<usize>,
    her: &[(usize, bool, f64)],
    goal: bool,
) -> f64 {
    // (the candidates whose step moved: if none, c1 is the base's)
    ch.clear();
    for &(ci, ui, p) in lifted {
        cost.take(bs, c1, ch, ci, ui, p);
    }
    if m == full_m
        && let Some(xi) = xi
    {
        c1[xi] = 0.0;
        ch.push(xi);
        cost.zeroed(xi);
    }
    let her_all = m == full_m && !her.is_empty();
    let mut va = cost.va(bs, c1, ch, her_all);
    if her_all {
        va = with_her(va, her, c1);
    }
    // c1 back to the base's
    for &i in ch.iter() {
        c1[i] = bs.c1[i];
    }
    if goal && m == full_m { bs.v0 } else { (bs.v0 - va).max(0.0) }
}
/// `trio_v` with one candidate's step changed (to x, from the base's c1), per candidate i: the minimum over the trios that
/// do not read her step (their values at the base's steps, trio_v's expression), and the trios that read it once, as
/// (key = rest + C's step, rest, C's step), key ascending: a trio's value (x + rest) + C's step, the same float as trio_v's
/// sum (addition commutes exactly). Not `ok` when a trio reads her step twice
struct One {
    not: f64,
    rd: Vec<(f64, f64, f64)>,
    ok: bool,
}
fn one_of(combos: &[Trio], c1: &[f64], i: usize) -> One {
    let (mut not, mut rd, mut ok) = (f64::INFINITY, vec![], true);
    for &(ia, ib, c3, ra, rb) in combos {
        let (av, bv) = (if ra { 0.0 } else { c1[ia] }, if rb { 0.0 } else { c1[ib] });
        match (ia == i && !ra, ib == i && !rb) {
            (false, false) => {
                let v = av + bv + c3;
                if v < not {
                    not = v;
                }
            }
            (true, false) => rd.push((bv + c3, bv, c3)),
            (false, true) => rd.push((av + c3, av, c3)),
            (true, true) => ok = false,
        }
    }
    rd.sort_by(|p, q| p.0.partial_cmp(&q.0).unwrap());
    One { not, rd, ok }
}
/// trio_v at the base's steps with candidate i's at x (`One`): the minimum, the reading trios scanned by key until no
/// later one can be below it. Derived: every term is >= 0, so a trio's value is at least fl(x + key) (1 - u)^5 (u the
/// unit roundoff; key is rounded once, x + key once, the value's two sums once each) > fl(x + key) (1 - 1e-9), and keys
/// only rise; a trio at or above the minimum leaves it unchanged (min is order-free)
fn one_v(o: &One, x: f64) -> f64 {
    let mut best = o.not;
    for &(key, rest, c3) in &o.rd {
        if (x + key) * (1.0 - 1e-9) >= best {
            break;
        }
        let v = (x + rest) + c3;
        if v < best {
            best = v;
        }
    }
    best
}
/// The leaves (her star patterns on the uncertain targets) a scoring's evaluation produces (`sc_eval`): `All`, the
/// plan's, every leaf in order (each test below is constant and compiles away), or `OneLeaf`, a roll's: one outcome's leaf
/// and its representative (`Scored::rep_of`). The fold goes down a prefix only where `wants` it, and writes and reads
/// each leaf's rows at `row`; the outcome loop takes the leaves in `list`'s order (ascending, as `All` takes them). A
/// leaf's values are computed down its own path alone (the fold's f64s at a leaf read no other leaf's), so every value
/// at a leaf produced is the one `All` computes there
pub(super) trait Leaves {
    const ALL: bool;
    /// whether the fold goes down prefix p of k digits
    fn wants(&self, k: usize, p: usize) -> bool;
    /// the leaves the outcome loop takes, ascending (with `ALL`, unused: every leaf)
    fn list(&self) -> &[usize];
    /// a produced leaf's row (`ALL`: the leaf itself)
    fn row(&self, l: usize) -> usize;
    /// the rows produced, of nleaf leaves
    fn nrows(&self, nleaf: usize) -> usize;
}
pub(super) struct All;
impl Leaves for All {
    const ALL: bool = true;
    #[inline(always)]
    fn wants(&self, _: usize, _: usize) -> bool {
        true
    }
    fn list(&self) -> &[usize] {
        &[]
    }
    #[inline(always)]
    fn row(&self, l: usize) -> usize {
        l
    }
    #[inline(always)]
    fn nrows(&self, nleaf: usize) -> usize {
        nleaf
    }
}
/// one outcome's leaf: the fold produces its representative only (row 0; a leaf that is not its own representative reads
/// the representative's y, row 1 its own)
pub(super) struct OneLeaf {
    ls: [usize; 2],
    n: usize,
}
impl OneLeaf {
    const fn new(leaf: usize, rep: usize) -> Self {
        if rep == leaf { Self { ls: [leaf, leaf], n: 1 } } else { Self { ls: [rep, leaf], n: 2 } }
    }
}
impl Leaves for OneLeaf {
    const ALL: bool = false;
    fn wants(&self, k: usize, p: usize) -> bool {
        p == self.ls[0] & ((1usize << (2 * k)) - 1)
    }
    fn list(&self) -> &[usize] {
        &self.ls[..self.n]
    }
    fn row(&self, l: usize) -> usize {
        usize::from(l != self.ls[0])
    }
    fn nrows(&self, _: usize) -> usize {
        self.n
    }
}
/// `score_career`'s fold buffers, kept between calls (each call writes what it reads): per level the folded tables, the
/// last two levels' 16 leaves per setup and per candidate slot, the outcome rows, per leaf her stars' chance and set
/// (`pkey`: what pst and mus were built for, every leaf: the star chances' bits, then the uncertain targets; empty: not
/// built for every leaf. `est`: `Est`'s two direct-mapped caches, refilled per call)
#[derive(Default)]
struct FoldScratch {
    lv: Vec<Vec<f64>>,
    fv: Vec<f64>,
    acc: Vec<f64>,
    u: Vec<f64>,
    pst: Vec<f64>,
    mus: Vec<usize>,
    pkey: Vec<u64>,
    yb: Vec<f64>,
    lb: Vec<u32>,
    lf: Vec<(usize, f64)>,
    est: EstBufs,
}
thread_local! { static FOLD_SCRATCH: std::cell::RefCell<FoldScratch> = std::cell::RefCell::new(FoldScratch::default()); }
/// What `fold_rec` reads: the kept tables (level 0, `t0`) and each one's setup, per setup x and 1 - x per uncertain
/// target and star level; for `fold_emit`: whether it writes per leaf the setups' table values (pl: fewer rows than per
/// outcome and slot; the outcome loop then takes the products and maxima itself), else per outcome the slots' odds; the
/// guaranteed golds' sets, the spark scales (m-major), each setup's candidate slot, the slots' odds now (`col`, `two`:
/// per target whether it is collapsed, and then the star levels folded, 0 and 1: `sc_prep`); the leaves produced (`lv`)
struct FoldCx<'fold, L: Leaves> {
    pl: bool,
    nu: usize,
    nf: usize,
    ns: usize,
    na: usize,
    t0: &'fold [f64],
    tsi: &'fold [usize],
    xs: &'fold [Vec<[f64; 4]>],
    xc: &'fold [Vec<[f64; 4]>],
    uncv: &'fold [usize],
    fmm: &'fold [usize],
    scf: &'fold [f64],
    sslot: &'fold [usize],
    p0: &'fold [f64],
    col: &'fold [bool],
    two: [usize; 2],
    lv: &'fold L,
}
impl<L: Leaves> FoldCx<'_, L> {
    /// her star levels folded on target k
    const fn sgs(&self, k: usize) -> &[usize] {
        if self.col[k] { &self.two } else { &[0, 1, 2, 3] }
    }
    /// level k's tables (level 0 the kept ones)
    fn level<'tab>(&'tab self, lv: &'tab [Vec<f64>], k: usize) -> &'tab [f64] {
        if k == 0 { self.t0 } else { &lv[k] }
    }
}
/// One leaf's rows (each fm; her landed set mu on the uncertain targets) at its row: per candidate slot the best of her
/// odds now and each setup's spark scale at m times its table value f (the products and the strict maxima the outcome
/// loop took, in any setup order: a maximum)
fn fold_emit<L: Leaves>(cx: &FoldCx<L>, leaf: usize, mu: usize, f: &[f64], u: &mut [f64]) {
    let lr = cx.lv.row(leaf);
    if cx.pl {
        u[lr * cx.ns..(lr + 1) * cx.ns].copy_from_slice(f);
        return;
    }
    for (fm, &mf) in cx.fmm.iter().enumerate() {
        let m = mu | mf;
        let row = &mut u[((lr << cx.nf) | fm) * cx.na..][..cx.na];
        row.copy_from_slice(cx.p0);
        let sr = &cx.scf[m * cx.ns..(m + 1) * cx.ns];
        for ((&sc, &fv), &sl) in sr.iter().zip(f).zip(cx.sslot) {
            let v = sc * fv;
            if v > row[sl] {
                row[sl] = v;
            }
        }
    }
}
/// `fold_emit` for the 16 leaves of the last two levels (f[si * 16 + b], b = digit k | digit k+1 << 2), through a slot-major
/// block acc[slot * 16 + b]; `bl`: the blocks folded (`fold_rec`)
fn fold_emit16<L: Leaves>(
    cx: &FoldCx<L>,
    k: usize,
    leaf: usize,
    mu: usize,
    f: &[f64],
    bl: &[usize],
    acc: &mut [f64],
    u: &mut [f64],
) {
    let (ns, na) = (cx.ns, cx.na);
    if cx.pl {
        for &b in bl {
            let li = cx.lv.row(leaf | (b & 3) << (2 * k) | (b >> 2) << (2 * (k + 1)));
            for (si, r) in u[li * ns..(li + 1) * ns].iter_mut().enumerate() {
                *r = f[si * 16 + b];
            }
        }
        return;
    }
    let (ba, bb) = (1usize << cx.uncv[k], 1usize << cx.uncv[k + 1]);
    for (fm, &mf) in cx.fmm.iter().enumerate() {
        // per leaf of the block, where its spark scales start (m-major)
        let mo: [usize; 16] =
            std::array::from_fn(|b| (mu | mf | if b & 3 > 0 { ba } else { 0 } | if b >> 2 > 0 { bb } else { 0 }) * ns);
        for (sl, a) in acc.chunks_exact_mut(16).enumerate() {
            a.fill(cx.p0[sl]);
        }
        for (si, &sl) in cx.sslot.iter().enumerate() {
            let (fs, a) = (&f[si * 16..si * 16 + 16], &mut acc[sl * 16..sl * 16 + 16]);
            for &b in bl {
                let v = cx.scf[mo[b] + si] * fs[b];
                if v > a[b] {
                    a[b] = v;
                }
            }
        }
        for &b in bl {
            let li = cx.lv.row(leaf | (b & 3) << (2 * k) | (b >> 2) << (2 * (k + 1)));
            let row = &mut u[((li << cx.nf) | fm) * na..][..na];
            for (sl, r) in row.iter_mut().enumerate() {
                *r = acc[sl * 16 + b];
            }
        }
    }
}
/// Every table folded at level k (level 0 `t0`, then lv[k]: table j at j 2^(nu-k)) down each branch of her stars on
/// uncertain target k that `cx.lv` wants, with the arithmetic of the one-table fold (`t[2i] c + t[2i+1] x`, the last two
/// levels unrolled); per leaf and setup the best of 0 and her tables' values (a maximum: any order), then `fold_emit`.
/// mu: her landed set on the targets so far
fn fold_rec<L: Leaves>(
    cx: &FoldCx<L>,
    k: usize,
    leaf: usize,
    mu: usize,
    lv: &mut [Vec<f64>],
    fv: &mut [f64],
    acc: &mut [f64],
    u: &mut [f64],
) {
    let (nu, ns) = (cx.nu, cx.ns);
    if k == nu {
        // (nu < 2: one value per table)
        let f = &mut fv[..ns];
        f.fill(0.0);
        let t = cx.level(lv, k);
        for (j, &si) in cx.tsi.iter().enumerate() {
            let v = t[j];
            if v > f[si] {
                f[si] = v;
            }
        }
        fold_emit(cx, leaf, mu, f, u);
        return;
    }
    if nu - k == 2 {
        // the last two levels: 4 values per table, 16 leaves per setup (f[si * 16 + (digit k | digit k+1 << 2)]), the
        // blocks folded every one but a collapsed target's other star levels and those not wanted
        let mut bl = [0usize; 16];
        let mut nb = 0;
        for &sg2 in cx.sgs(k + 1) {
            for &sg in cx.sgs(k) {
                if !L::ALL && !cx.lv.wants(nu, leaf | sg << (2 * k) | sg2 << (2 * (k + 1))) {
                    continue;
                }
                bl[nb] = sg | sg2 << 2;
                nb += 1;
            }
        }
        let f = &mut fv[..16 * ns];
        f.fill(0.0);
        let t = cx.level(lv, k);
        for (j, &si) in cx.tsi.iter().enumerate() {
            let (t0, t1, t2, t3) = (t[4 * j], t[4 * j + 1], t[4 * j + 2], t[4 * j + 3]);
            let (xa, ca, xb, cb) = (&cx.xs[si][k], &cx.xc[si][k], &cx.xs[si][k + 1], &cx.xc[si][k + 1]);
            let f0: [f64; 4] = std::array::from_fn(|sg| t0 * ca[sg] + t1 * xa[sg]);
            let f1: [f64; 4] = std::array::from_fn(|sg| t2 * ca[sg] + t3 * xa[sg]);
            let o = &mut f[si * 16..si * 16 + 16];
            for &b in &bl[..nb] {
                let (sg, sg2) = (b & 3, b >> 2);
                let v = f0[sg] * cb[sg2] + f1[sg] * xb[sg2];
                let oo = &mut o[b];
                if v > *oo {
                    *oo = v;
                }
            }
        }
        fold_emit16(cx, k, leaf, mu, f, &bl[..nb], acc, u);
        return;
    }
    let h = 1usize << (nu - k - 1);
    for &sg in cx.sgs(k) {
        let pc = leaf | sg << (2 * k);
        if !L::ALL && !cx.lv.wants(k + 1, pc) {
            continue;
        }
        {
            let (lo, hi) = lv.split_at_mut(k + 1);
            let (t, f) = (if k == 0 { cx.t0 } else { &*lo[k] }, &mut hi[0]);
            for (j, &si) in cx.tsi.iter().enumerate() {
                let (x, c) = (cx.xs[si][k][sg], cx.xc[si][k][sg]);
                let (tj, fj) = (&t[2 * h * j..2 * h * (j + 1)], &mut f[h * j..h * (j + 1)]);
                for i in 0..h {
                    fj[i] = tj[2 * i] * c + tj[2 * i + 1] * x;
                }
            }
        }
        fold_rec(cx, k + 1, pc, if sg > 0 { mu | 1 << cx.uncv[k] } else { mu }, lv, fv, acc, u);
    }
}
/// Leaf `leaf`'s landed set on the uncertain targets (digit k: her stars on target uncv[k], 0 absent) and her stars'
/// chance: the product over k of w[stars - 1] where she has any, in k order from 1.0 (the f64 `sc_outcomes`' per-leaf
/// table holds: each of its entries is a lower leaf's times its top digit's w, the same product in the same order)
fn leaf_set(leaf: usize, uncv: &[usize], w: [f64; 3]) -> (usize, f64) {
    let mut mu = 0usize;
    let mut pr_st = 1.0f64;
    for (k, &i) in uncv.iter().enumerate() {
        let sg = (leaf >> (2 * k)) & 3;
        if sg > 0 {
            mu |= 1 << i;
            pr_st *= w[sg - 1];
        }
    }
    (mu, pr_st)
}
/// Per leaf (base-4 digits, digit k at bits 2k..2k+1) the product of g[k][digit k] over k, into `out`: level by level,
/// each level's products the previous level's (at the leaf's lower digits) times g[k][digit k], from 1.0. The same
/// operations in the same order as `let mut pr = 1.0; for k in 0..nu { pr *= g[k][digit k]; }` per leaf, so the same
/// f64s; each prefix is computed once instead of once per leaf (4/3 4^nu products for nu 4^nu)
fn leaf_products(g: &[[f64; 4]], out: &mut Vec<f64>, tmp: &mut Vec<f64>) {
    out.clear();
    out.push(1.0);
    for (k, gk) in g.iter().enumerate() {
        let low = (1usize << (2 * k)) - 1;
        tmp.clear();
        for &gd in gk {
            tmp.extend(out[..=low].iter().map(|&o| o * gd));
        }
        std::mem::swap(out, tmp);
    }
}
/// Per leaf (as `leaf_products`) the best outfit's `Env::rise` of trainee (sts, ews: her outfits' coverage and own
/// chances) from chances qa (job-target order) to q'(leaf), q'_k = 1 - x0[k] xms[her stars on uncertain target k]:
/// per outfit and target the factor at each of her four star levels (1 where it can't rise), their product per leaf
fn rise_leaves(
    sts: &[Vec<u8>],
    ews: &[Vec<f64>],
    x0: &[f64],
    xms: &[f64; 4],
    qa: &[f64],
    uncv: &[usize],
    lp: &mut Vec<f64>,
    lp_tmp: &mut Vec<f64>,
) -> Vec<f64> {
    let nu = uncv.len();
    let mut rz = vec![0.0f64; 1usize << (2 * nu)];
    for (st, ew) in sts.iter().zip(ews) {
        let g: Vec<[f64; 4]> = (0..nu)
            .map(|k| {
                let i = uncv[k];
                [0, 1, 2, 3].map(|sg| rise_factor(st[k], ew[k], 1.0 - x0[k] * xms[sg], qa[i]))
            })
            .collect();
        leaf_products(&g, lp, lp_tmp);
        for (rv, &pr) in rz.iter_mut().zip(lp.iter()) {
            if pr > *rv {
                *rv = pr;
            }
        }
    }
    rz
}
/// `outcome_hi`'s bound for one (candidate, partner), bd per leaf: the least of the ceiling rfc = rf x the best fast
/// outfit's product and, over the anchors (f, qa), f x `rise_leaves`(qa); the same f64s as `rfc.min(anc)` with anc
/// every anchor's value folded in at every leaf (a min may be taken in any order; a NaN rfc counts as +inf there, as
/// `f64::min` drops it). Found by an exact branch and bound over the leaves' digit prefixes (digit k at bits 2k..2k+1,
/// the products' order). An anchor's factors are all > 0 and f > 0 (checked; else it goes through `rise_leaves` whole),
/// so a leaf's product is at least its prefix's product times the least factor of each remaining digit; with slack
/// BB_SLACK for rounding (far more than the (1 + 2^-53)^(2 nu + 4) the chains can drift) that bounds f x the leaf's
/// value from below at every leaf under the prefix (correctly rounded products are monotone). Where the bound reaches
/// the most bd holds under the prefix (`tmax`), the anchor lowers no leaf there: the subtree is skipped.
struct AncBb {
    nu: usize,
    no: usize,
    tmax: Vec<Vec<f64>>,
    pp: Vec<f64>,
    smin: Vec<f64>,
    nodes: u64,
    roots: u64,
    sparse: bool,
    acc: Vec<f64>,
}
const BB_SLACK: f64 = 1.0 - 1e-12;
impl AncBb {
    const fn new() -> Self {
        Self { nu: 0, no: 0, tmax: vec![], pp: vec![], smin: vec![], nodes: 0, roots: 0, sparse: false, acc: vec![] }
    }
    /// bd per leaf (level nu of `tmax`)
    fn bd(&self) -> &[f64] {
        &self.tmax[self.nu]
    }
    /// a fresh (candidate, partner) with `no` outfits for the anchors and the ceiling's fast outfits gc ([outfit][target
    /// k][her stars], nfo of them): bd = rfc (per leaf rf x, from 0.0, the most of each outfit's product in
    /// `leaf_products`' order: the old ceiling's f64s) at the marked leaves (`Marks`), -inf at the others (so no anchor
    /// is folded in where nothing reads the bound: every test below compares with a most that holds marked leaves only),
    /// `tmax` built
    fn reset(
        &mut self,
        nu: usize,
        no: usize,
        gc: &[f64],
        nfo: usize,
        rf: f64,
        mk: &Marks,
        pre: &mut Vec<f64>,
        tmp: &mut Vec<f64>,
    ) {
        self.nu = nu;
        self.no = no;
        self.pp.clear();
        self.pp.resize((nu + 1) * no, 1.0);
        self.smin.clear();
        self.smin.resize((nu + 1) * no, 1.0);
        self.tmax.resize(nu + 1, vec![]);
        let nan_inf = |b: f64| if b.is_nan() { f64::INFINITY } else { b };
        let mut bd = std::mem::take(&mut self.tmax[nu]);
        if nu == 0 {
            bd.clear();
            bd.push(if mk.lv[0][0] { nan_inf(rf * if nfo > 0 { 1.0 } else { 0.0 }) } else { f64::NEG_INFINITY });
            self.tmax[0] = bd;
            return;
        }
        let np = 1usize << (2 * (nu - 1));
        if !mk.dense {
            // few marked leaves: only the marked prefixes are built (`reset_marked`). Every other entry of every level is
            // -inf, as the full build leaves it (an unmarked leaf's -inf, a prefix the most of its quarters): set here
            // once per AncBb (it serves one `Marks`), and nothing raises it after (`run` and the direct anchors only
            // lower a leaf; a prefix is again the most of its quarters)
            if !self.sparse || bd.len() != np * 4 {
                bd.clear();
                bd.resize(np * 4, f64::NEG_INFINITY);
                for j in 0..nu {
                    let t = &mut self.tmax[j];
                    t.clear();
                    t.resize(1usize << (2 * j), f64::NEG_INFINITY);
                }
                self.sparse = true;
            }
            self.tmax[nu] = bd;
            self.reset_marked(nu, gc, nfo, rf, mk, pre, tmp);
            return;
        }
        self.sparse = false;
        bd.clear();
        bd.resize(np * 4, 0.0);
        let mut g: Vec<[f64; 4]> = Vec::with_capacity(nu);
        for o in 0..nfo {
            g.clear();
            g.extend((0..nu).map(|k| {
                let b = (o * nu + k) * 4;
                [gc[b], gc[b + 1], gc[b + 2], gc[b + 3]]
            }));
            leaf_products(&g[..nu - 1], pre, tmp);
            // the last digit's level straight into the running most
            for (dg, &gl) in g[nu - 1].iter().enumerate() {
                for (c, &pv) in bd[dg * np..(dg + 1) * np].iter_mut().zip(pre.iter()) {
                    let pr = pv * gl;
                    if pr > *c {
                        *c = pr;
                    }
                }
            }
        }
        for c in &mut bd {
            *c = nan_inf(rf * *c);
        }
        for &l in &mk.unm {
            bd[l] = f64::NEG_INFINITY;
        }
        self.tmax[nu] = bd;
        self.levels();
    }
    /// `reset`'s rebuild at the marked prefixes only (sparse marks, every other entry -inf already): per outfit the
    /// products level by level down `Marks::ix` (each prefix's its parent's times its last digit's factor, from 1.0:
    /// `leaf_products`' f64s at those leaves), the running most per marked leaf over the outfits in order from 0.0, rf x it
    /// (NaN as +inf) into bd, then each marked prefix's most of its four quarters as `levels` takes it
    #[inline(never)]
    fn reset_marked(
        &mut self,
        nu: usize,
        gc: &[f64],
        nfo: usize,
        rf: f64,
        mk: &Marks,
        cur: &mut Vec<f64>,
        nxt: &mut Vec<f64>,
    ) {
        let nml = mk.ix[nu].len();
        let mut acc = std::mem::take(&mut self.acc);
        acc.clear();
        acc.resize(nml, 0.0);
        for o in 0..nfo {
            let g = &gc[o * nu * 4..(o + 1) * nu * 4];
            cur.clear();
            cur.push(1.0);
            for j in 1..=nu {
                let gk = &g[(j - 1) * 4..j * 4];
                let sh = 2 * (j - 1);
                if j == nu {
                    for ((a, &pc), &pa) in acc.iter_mut().zip(&mk.ix[j]).zip(&mk.par[j]) {
                        let v = cur[pa as usize] * gk[(pc >> sh) as usize & 3];
                        if v > *a {
                            *a = v;
                        }
                    }
                } else {
                    nxt.clear();
                    nxt.extend(
                        mk.ix[j]
                            .iter()
                            .zip(&mk.par[j])
                            .map(|(&pc, &pa)| cur[pa as usize] * gk[(pc >> sh) as usize & 3]),
                    );
                    std::mem::swap(cur, nxt);
                }
            }
        }
        {
            let bd = &mut self.tmax[nu];
            for (&l, &a) in mk.ix[nu].iter().zip(&acc) {
                let b = rf * a;
                bd[l as usize] = if b.is_nan() { f64::INFINITY } else { b };
            }
        }
        self.acc = acc;
        for j in (0..nu).rev() {
            let np = 1usize << (2 * j);
            let (lo, hi) = self.tmax.split_at_mut(j + 1);
            let (lv, up) = (&mut lo[j], &hi[0]);
            for &p in &mk.ix[j] {
                let p = p as usize;
                lv[p] = up[p].max(up[np + p]).max(up[2 * np + p]).max(up[3 * np + p]);
            }
        }
    }
    /// per prefix the most bd holds under it (level j: the 4^j prefixes of j digits; prefix p of j digits holds leaves
    /// p + 4^j r: the most of level j + 1's four quarters; no NaN here)
    fn levels(&mut self) {
        for j in (0..self.nu).rev() {
            let np = 1usize << (2 * j);
            let mut lv = std::mem::take(&mut self.tmax[j]);
            let up = &self.tmax[j + 1];
            lv.clear();
            lv.extend((0..np).map(|p| up[p].max(up[np + p]).max(up[2 * np + p]).max(up[3 * np + p])));
            self.tmax[j] = lv;
        }
    }
    /// an anchor's suffix products of its least factors (`smin`), and whether its bound at the root is below th (the most bd
    /// holds, tmax[0][0], or a bar at least that: tmax only falls, so a no now is a no later too); f > 0, g its factors as `run`'s
    fn root_open(&mut self, f: f64, g: &[f64], th: f64) -> bool {
        let (nu, no) = (self.nu, self.no);
        if nu == 0 {
            return true;
        }
        let mut lb = 0.0f64;
        for o in 0..no {
            for j in (0..nu).rev() {
                let gk = &g[(o * nu + j) * 4..(o * nu + j) * 4 + 4];
                let mn = gk[0].min(gk[1]).min(gk[2]).min(gk[3]);
                self.smin[o * (nu + 1) + j] = self.smin[o * (nu + 1) + j + 1] * mn;
            }
            let l = self.smin[o * (nu + 1)];
            if l > lb {
                lb = l;
            }
        }
        f * BB_SLACK * lb < th
    }
    /// `root_open`'s answer for an anchor (f > 0, chances qa) from its 0-star factors alone (`rise_factors`' expressions;
    /// the same products in the same order; `smin` is not written, `run` writes it). Premises (the caller's): e, qa, x0 in
    /// [0, 1], xms falling and >= 0, nu > 0. Derived: qn = 1 - x0 xms[s] rises with s (rounding is monotone); a factor is 1
    /// where qn <= qa, else N(qn) / D, D = N(qa) = 1 - (1 - e)(1 - qa) in [0, 1] and N rising with qn: N >= D, so it is
    /// >= 1 (+inf where D = 0, as N > 0 there), never NaN, and rises with qn: each digit's least is its 0-star factor
    fn root_min(
        &self,
        f: f64,
        sts: &[Vec<u8>],
        ews: &[Vec<f64>],
        x0: &[f64],
        xms: &[f64; 4],
        qa: &[f64],
        uncv: &[usize],
        th: f64,
    ) -> bool {
        let nu = self.nu;
        let mut lb = 0.0f64;
        for (st, ew) in sts.iter().zip(ews) {
            let mut l = 1.0f64;
            for k in (0..nu).rev() {
                l *= rise_factor(st[k], ew[k], 1.0 - x0[k] * xms[0], qa[uncv[k]]);
            }
            if l > lb {
                lb = l;
            }
        }
        f * BB_SLACK * lb < th
    }
    /// fold one anchor in: f > 0, g its factors [outfit][target k][her stars], all > 0
    fn run(&mut self, f: f64, g: &[f64]) {
        let (nu, no) = (self.nu, self.no);
        if nu == 0 {
            let v = f * if no > 0 { 1.0 } else { 0.0 };
            if v < self.tmax[0][0] {
                self.tmax[0][0] = v;
            }
            return;
        }
        if !self.root_open(f, g, self.tmax[0][0]) {
            return;
        }
        for o in 0..no {
            self.pp[o] = 1.0;
        }
        self.roots += 1;
        self.dfs(0, 0, f, f * BB_SLACK, g);
    }
    fn dfs(&mut self, j: usize, p: usize, f: f64, fs: f64, g: &[f64]) {
        let (nu, no) = (self.nu, self.no);
        self.nodes += 1;
        for dg in 0..4usize {
            let pc = p | dg << (2 * j);
            for o in 0..no {
                self.pp[(j + 1) * no + o] = self.pp[j * no + o] * g[(o * nu + j) * 4 + dg];
            }
            if j + 1 == nu {
                let mut rv = 0.0f64;
                for o in 0..no {
                    let pr = self.pp[nu * no + o];
                    if pr > rv {
                        rv = pr;
                    }
                }
                let v = f * rv;
                let b = &mut self.tmax[nu][pc];
                if v < *b {
                    *b = v;
                }
            } else {
                let mut lb = 0.0f64;
                for o in 0..no {
                    let l = self.pp[(j + 1) * no + o] * self.smin[o * (nu + 1) + j + 1];
                    if l > lb {
                        lb = l;
                    }
                }
                if fs * lb >= self.tmax[j + 1][pc] {
                    continue;
                }
                self.dfs(j + 1, pc, f, fs, g);
            }
        }
        let up = &self.tmax[j + 1];
        let s = 1usize << (2 * j);
        self.tmax[j][p] = up[p].max(up[p + s]).max(up[p + 2 * s]).max(up[p + 3 * s]);
    }
}
/// The leaves (and outcomes) whose bound something reads: `lv[j][p]`, prefix p of j digits holds a marked leaf (level nu:
/// the leaves); `dense` when most leaves are marked (the ceiling is then computed at every leaf, vectorised; `unm` the
/// unmarked leaves); else `ix[j]` the marked prefixes of j digits ascending and `par[j]` each one's parent's place in
/// `ix[j - 1]` (level 0 none)
struct Marks {
    lv: Vec<Vec<bool>>,
    dense: bool,
    unm: Vec<usize>,
    ix: Vec<Vec<u32>>,
    par: Vec<Vec<u32>>,
}
/// `rise_leaves`'s factors for chances qa, [outfit][target k][her stars] (the same expressions), into g
fn rise_factors(
    sts: &[Vec<u8>],
    ews: &[Vec<f64>],
    x0: &[f64],
    xms: &[f64; 4],
    qa: &[f64],
    uncv: &[usize],
    g: &mut Vec<f64>,
) {
    let nu = uncv.len();
    g.clear();
    for (st, ew) in sts.iter().zip(ews) {
        for k in 0..nu {
            for sg in 0..4 {
                g.push(rise_factor(st[k], ew[k], 1.0 - x0[k] * xms[sg], qa[uncv[k]]));
            }
        }
    }
}
/// `outcome_hi` at one outcome (her stars `leaf`, landed set m): each candidate's odds `up` at the most any partner's
/// bound allows (into `up`), then `saving_lo`; `c1` scratch
fn hi_at(
    bs: &ScoreBase,
    hs: &[(usize, Vec<f64>, Vec<f64>)],
    leaf: usize,
    m: usize,
    full_m: usize,
    xi: Option<usize>,
    her: &[(usize, bool, f64)],
    goal: bool,
    c1: &mut [f64],
    up: &mut [f64],
) -> f64 {
    up.copy_from_slice(bs.pnow);
    for (ui, bd, sc) in hs {
        let v = sc[m] * bd[leaf];
        if v > up[*ui] {
            up[*ui] = v;
        }
    }
    saving_lo(bs, up, m, full_m, xi, her, goal, c1)
}
/// V - V_lo at candidate odds `up`: each step that rose at `Cont::cost_lo`, her own 8/8 (`xi`, `her_trios`) on the full
/// set, the goal's V; `c1` scratch
fn saving_lo(
    bs: &ScoreBase,
    up: &[f64],
    m: usize,
    full_m: usize,
    xi: Option<usize>,
    her: &[(usize, bool, f64)],
    goal: bool,
    c1: &mut [f64],
) -> f64 {
    // (the candidates above their threshold, `score_career`'s pnow (1 + 1e-9), in candidate order)
    let lifted: Vec<(usize, usize, f64)> =
        (0..bs.cands.len()).filter(|&ui| up[ui] > bs.pnow[ui] * (1.0 + 1e-9)).map(|ui| (ui, ui, up[ui])).collect();
    c1.copy_from_slice(bs.c1);
    v_after(bs, &mut Lo, c1, &mut vec![], &lifted, m, full_m, xi, her, goal)
}
/// The pick's bound's exact anchors (`outcome_hi`): her best deck searched with her at each of these stars on every
/// uncertain target, for at most HI_SEARCHES / 2 partners (with no limit p16 took 5 and left the same). Callgrind per
/// search: p16 55M, gm16 151M instructions (the val/plan jobs +6.0% and +11.1% in all). A leaf whose stars are all at
/// most 1 is rare at the rank's mix: adding 1 star moved p16's provisional 0.0275 -> 0.0272, gm16 0.0796 -> 0.0742 for
/// +8.8% and +16.5%
const HI_STARS: [usize; 2] = [2, 3];
/// Off by default (not worth the plan's +6-11%); 10 gives the numbers above.
const HI_SEARCHES: usize = 0;
/// ... and only for band outcomes whose partners' bounds are within this factor of p_now: a cost rule, not part of the
/// bound. It skips p10 and p11, whose band needs partners at 2.5 times p_now and more: there 15 searches (at 1, 2 and 3
/// stars) freed nothing (p11: 233 ms native spent), and only 63 (805 ms) moved p10 (0.474 -> 0.378); 1.3 and 1.5 chose
/// the same
const HI_RISE: f64 = 1.5;
/// `outcome_hi` for one (candidate ui, partner y): her slot's miss factors, the bound per leaf (`AncBb`), each marked
/// outcome's most of the partners' (`tu`: candidate ui's row of the table). Its own function, entered once per partner,
/// so V8 compiles it optimised after the first few (`docs/PERF.md`, *In the page*)
struct HiCx<'base> {
    bs: &'base ScoreBase<'base>,
    d: &'base Data,
    legs: &'base [Legacy],
    pool: &'base Pool,
    fix: &'base [usize],
    uncv: &'base [usize],
    mk: &'base Marks,
    mc: &'base [usize],
    lm: &'base [u32],
    nu: usize,
    ntar: usize,
    nm: usize,
    nf: usize,
    ncell: usize,
    nmc: usize,
    iv: usize,
    rf: f64,
}
/// `hi_partner`'s scratch, kept across partners (each call writes what it reads)
struct HiScr {
    bb: AncBb,
    g1: Vec<f64>,
    gc: Vec<f64>,
    lp: Vec<f64>,
    lp_tmp: Vec<f64>,
    qz: Vec<f64>,
    tg: Vec<f64>,
}
/// One (candidate, partner)'s anchors as `hi_partner` keeps them for the next call of the same pick (a roll's: none until
/// its first roll): (candidate, partner), and per anchor (f, chances) that can lower some leaf, in their order
type HiAnc = ((usize, usize), Option<Vec<(f64, Vec<f64>)>>);
#[inline(never)]
#[allow(clippy::absurd_extreme_comparisons, reason = "HI_SEARCHES is a tuning constant, now 0 (anchors off)")]
fn hi_partner(
    s: &Sim,
    hx: &HiCx,
    pv: &Pool,
    ui: usize,
    u: usize,
    y: usize,
    sts: &[Vec<u8>],
    ews: &[Vec<f64>],
    ew_ok: bool,
    fr: &[usize],
    fr_of: &[Option<(usize, Vec<f64>, f64)>],
    tu: &mut [f64],
    hp: &mut HiScr,
    hs: &mut Vec<(usize, Vec<f64>, Vec<f64>)>,
    hq: &mut Vec<(usize, Vec<f64>, Vec<f64>, [f64; 4])>,
    ac: &mut Option<Vec<(f64, Vec<f64>)>>,
) {
    let HiCx { bs, d, legs, pool, fix, uncv, mk, mc, lm, nu, ntar, nm, nf, ncell, nmc, iv, rf } = *hx;
    let HiScr { bb, g1, gc, lp, lp_tmp, qz, tg } = hp;
    // her slot's miss factors and spark scales (`Env::her_slot`, as `score_career`'s setups)
    let Some(HerSlot { q0, x0, xms, pw }) = s.env.her_slot(pv, u, y, iv) else { return };
    // the best outfit's product per leaf: per outfit and target the factor A - B (1 - q') at each of her four star
    // levels, 1 - q' = x0 xm (`outfit_fast` at her chances folded in, as `Sim::fast_ub` takes it)
    let t0 = now_ms();
    gc.clear();
    for o in &s.fastc[u] {
        for k in 0..nu {
            let (aa, bb) = o[k];
            for sg in 0..4 {
                gc.push(aa - bb * (x0[k] * xms[sg]));
            }
        }
    }
    // the anchored rise bound per leaf: the least over today's pairs (y, z) keeping the places of
    // (pnow / c_yz) rise(q'(leaf), q_yz), q'_k(leaf) = 1 - x0[k] xm[her stars], and over the main search's proven
    // careers of her deck class (E_best(q_f) = val_f there: val_f rise(q', q_f)); at most the ceiling too (under
    // EXACT_RHO): bd = min(rf ceil, anc) per leaf, by `AncBb` (anchors in any order: a min)
    bb.reset(nu, sts.len(), gc, s.fastc[u].len(), rf, mk, lp, lp_tmp);
    let t1 = now_ms();
    // the anchors whose bound at the root is open (f, factors), and whether any went in whole
    let mut direct = false;
    let (mut gs, mut fs): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    // (`AncBb::root_min`'s premises but the anchor's chances)
    let safe = ew_ok
        && nu > 0
        && xms[0] >= xms[1]
        && xms[1] >= xms[2]
        && xms[2] >= xms[3]
        && xms[3] >= 0.0
        && x0.iter().all(|v| (0.0..=1.0).contains(v));
    // The most the ceiling holds at any leaf (`AncBb::reset`'s bd at the leaf of each digit's largest factor: per outfit the
    // product in the leaves' order from 1.0, the most over outfits from 0.0, rf x it; with every factor >= 0 rounding is
    // monotone, so no leaf's is above it), +inf where a factor is negative or NaN. An anchor whose root test fails against
    // it fails at every set of marked leaves (the test's own bar, the most bd holds at the marked leaves, is at most it):
    // the anchors kept for later calls (`ac`, a roll's: the same pair, pool and odds now at every outcome) are the others,
    // in their order, so a later call folds in the same anchors as computing them all would, minus ones that lower nothing
    let tcap = {
        let (no, mut acc, mut ok) = (s.fastc[u].len(), 0.0f64, true);
        for o in 0..no {
            let mut p = 1.0f64;
            for k in 0..nu {
                let g = &gc[(o * nu + k) * 4..][..4];
                ok &= g.iter().all(|v| *v >= 0.0);
                p *= g[0].max(g[1]).max(g[2]).max(g[3]);
            }
            if p > acc {
                acc = p;
            }
        }
        let b = rf * acc;
        if ok && !b.is_nan() { b } else { f64::INFINITY }
    };
    let building = ac.is_none();
    let mut kept: Vec<(f64, Vec<f64>)> = vec![];
    let mut add = |bb: &mut AncBb,
                   f: f64,
                   qa: &[f64],
                   gs: &mut Vec<f64>,
                   fs: &mut Vec<f64>,
                   direct: &mut bool,
                   kept: &mut Vec<(f64, Vec<f64>)>| {
        if f > 0.0 && safe && uncv.iter().all(|&i| (0.0..=1.0).contains(&qa[i])) {
            // every factor >= 1, each digit's least its 0-star one: the root test on those alone, the rest only
            // for an anchor it keeps (as the test below would: all factors > 0, `root_open` the same answer)
            if building {
                if !bb.root_min(f, sts, ews, &x0, &xms, qa, uncv, tcap) {
                    return;
                }
                kept.push((f, qa.to_vec()));
            }
            if !bb.root_min(f, sts, ews, &x0, &xms, qa, uncv, bb.tmax[0][0]) {
                return;
            }
            rise_factors(sts, ews, &x0, &xms, qa, uncv, g1);
            debug_assert!(
                g1.iter().all(|&v| v > 0.0) && bb.root_open(f, g1, bb.tmax[0][0]),
                "a candidate kept for a rise has positive factors and an open root"
            );
            fs.push(f);
            gs.extend_from_slice(g1);
            return;
        }
        rise_factors(sts, ews, &x0, &xms, qa, uncv, g1);
        if f > 0.0 && g1.iter().all(|&v| v > 0.0) {
            if building {
                if !bb.root_open(f, g1, tcap) {
                    return;
                }
                kept.push((f, qa.to_vec()));
            }
            if bb.root_open(f, g1, bb.tmax[0][0]) {
                fs.push(f);
                gs.extend_from_slice(g1);
            }
        } else {
            // outside the bound's premises: every leaf, as before
            if building {
                kept.push((f, qa.to_vec()));
            }
            *direct = true;
            let rz = rise_leaves(sts, ews, &x0, &xms, qa, uncv, lp, lp_tmp);
            for (av, rv) in bb.tmax[bb.nu].iter_mut().zip(&rz) {
                let v = f * rv;
                if v < *av {
                    *av = v;
                }
            }
        }
    };
    if let Some(ks) = ac.as_ref() {
        for (f, qa) in ks {
            add(bb, *f, qa, &mut gs, &mut fs, &mut direct, &mut kept);
        }
    } else {
        if bs.pnow[ui] > 0.0 {
            let mut nz = 0usize;
            for z in (0..legs.len()).filter(|&z| legal_pair(d, pool, u, y, z) && holds_fix(fix, y, z)) {
                let cz = s.env.lineage_qc(pool, u, y, z, uncv, qz, tg);
                if cz <= 0.0 {
                    continue;
                }
                nz += 1;
                add(bb, bs.pnow[ui] / cz, qz, &mut gs, &mut fs, &mut direct, &mut kept);
            }
            if nz > 0 {
                prof("hi_z", nz as f64);
            }
        }
        for &i in fr {
            let (_, fq, fv) = fr_of[i].as_ref().unwrap();
            add(bb, *fv, fq, &mut gs, &mut fs, &mut direct, &mut kept);
        }
        prof("hi_kept", kept.len() as f64);
        *ac = Some(kept);
    }
    if direct {
        bb.levels();
    }
    // the likeliest lowest first (f x the best outfit's product at 2 stars on every target): only speed
    let ga = sts.len() * nu * 4;
    let mut ord: Vec<(f64, usize)> = fs
        .iter()
        .enumerate()
        .map(|(i, &f)| {
            let gi = &gs[i * ga..(i + 1) * ga];
            (
                f * (0..sts.len())
                    .map(|o| (0..nu).map(|k| gi[(o * nu + k) * 4 + 2]).product::<f64>())
                    .fold(0.0f64, f64::max),
                i,
            )
        })
        .collect();
    ord.sort_by(|p, q| p.0.total_cmp(&q.0).then(p.1.cmp(&q.1)));
    let t3 = now_ms();
    let nodes0 = bb.nodes;
    for &(_, i) in &ord {
        bb.run(fs[i], &gs[i * ga..(i + 1) * ga]);
    }
    prof("hi_nodes", (bb.nodes - nodes0) as f64);
    prof("hi_open", fs.len() as f64);
    prof("hi_roots", bb.roots as f64);
    bb.roots = 0;
    let t4 = now_ms();
    // the spark scale per landed set m: every m's when the marked cells are as many (or the exact anchors keep them), else each
    // marked cell's own (the same product)
    let scm = |m: usize| -> f64 { (0..ntar).map(|k| pw[k][(m >> k) & 1]).product() };
    let sc: Vec<f64> = if HI_SEARCHES > 0 || nmc >= nm { (0..nm).map(&scm).collect() } else { vec![] };
    // each outcome's most of the partners' sc[m] bd[leaf] (from pnow; in partner order per candidate, as `hi_at`)
    let bd = bb.bd();
    if nmc == ncell && nf == 0 {
        for ((t, &m), &b) in tu.iter_mut().zip(lm).zip(bd) {
            let v = sc[m as usize] * b;
            if v > *t {
                *t = v;
            }
        }
    } else {
        for ((t, &i), &m) in tu.iter_mut().zip(mc).zip(lm) {
            let v = (if sc.is_empty() { scm(m as usize) } else { sc[m as usize] }) * bd[i >> nf];
            if v > *t {
                *t = v;
            }
        }
    }
    prof("hi_ceil_ms", t1 - t0);
    prof("hi_anc_ms", t3 - t1);
    prof("hi_bb_ms", t4 - t3);
    prof("hi_fold_ms", now_ms() - t4);
    prof("hi_pairs", 1.0);
    // (both read only by the exact anchors, HI_SEARCHES > 0)
    if HI_SEARCHES > 0 {
        hs.push((ui, bb.bd().to_vec(), sc));
        hq.push((u, q0, x0, xms));
    }
}
/// The pick's upper bound at the outcomes wanted (a roll's `y_hi`, `roll_answer`): the most V can fall through her
/// veteran's DIRECT lifts. The outcomes, base and own-8/8 credit (`her_trios`) are `score_career`'s; but each stage
/// candidate's odds with her as a parent, with EVERY legal partner y (not the table value of her IDX_PARTNERS best), are
/// bounded from the candidate's exact best now (pnow): every pair (y, z) of today's pool that keeps the plan's places is
/// a career of hers worth at most pnow, so (`Env::rise`: E(q') <= E(q) x rise(q', q) for every deck)
///     value(y, her) = c' E_best(q') <= c' (pnow / c_yz) rise(q', q_yz),
/// the least over z (the veteran most like her: rise 1 when one already carries what she does), and over the main
/// search's found careers f of her deck class that it fully checked (a `Ref` at q_f: every deck of every classmate is
/// worth at most val_f there, `Env::seen`, under the caps every search assumes; the found careers are the search's
/// answer, so the anchors depend on the job alone)
///     value(y, her) <= c' val_f rise(q', q_f);
/// and, for the partners behind the band's likeliest outcomes (y < 0.5 <= the bound so far; `HI_SEARCHES`, `HI_RISE`),
/// over her best deck searched at her s = 2, 3 stars on every uncertain target (q^s, `best_deck` fully checked: E_best
/// there, under the caps every search assumes)
///     value(y, her) <= c' E_best(q^s) rise(q', q^s),
/// rise 1 where her stars are at most s (the band's outcomes are bounded again after); also at most the ceiling
/// (`Env::ceiling` through `Sim::fastc`, under EXACT_RHO: per outfit and target A - B (1 - q'), `outfit_fast`'s factor at
/// q'; before 2026-10-09 the bound took A + B q', B more per target, valid and on p16 3.3 times as high at the pick's
/// partners). A candidate's step costs the lowest cost at those odds
/// (`Cont::cost_lo`). Derived, per outcome:
/// (1) For each partner, the bound >= the best career's exact value >= the table value `score_career` reads. A cached
///     deck's corner table is multilinear in the targets' miss chances, as P(all) of a fixed deck is (each chance enters
///     one target's factor linearly), so its interpolation is that deck's exact value; a real deck is at most the best.
///     Her best partners are among every partner. So up_hi >= up for every candidate.
/// (2) cost_lo(up_hi) <= cost(up) (`Cont::cost_lo`), a step cost never rises from the base, and V over the trios (and
///     `with_her`) does not fall when a step cost rises; so V_lo <= V_table and y_hi >= y.
/// (3) Each anchor A (a pair (y, z), a found career, a search at q^s) has a value E_A with E_best(q_A) <= E_A (a career
///     of today's pool is worth at most pnow; a fully checked deck search is the best under the caps). Every deck D legal
///     for the candidate has E_D(q') <= E_D(q_A) rise(q', q_A) (`Env::rise`) <= E_A rise(q', q_A), so her best career
///     with partner y is at most c' E_A rise(q', q_A) for every A, and at most its ceiling. In the exact replan with her
///     in the pool, with today's Cont and trios, her step then costs at least cost_lo(up_hi): y_hi bounds the direct-lift
///     saving. Not bounded (channel 2, out of scope by design): what the replan's refits do (Cont fitted
///     again, other careers' best careers with her as a parent, the stage candidates chosen again beyond her own 8/8).
fn outcome_hi(
    s: &mut Sim,
    d: &Data,
    bs: &ScoreBase,
    legs: &[Legacy],
    pool: &Pool,
    fix: &[usize],
    fr_of: &[Option<(usize, Vec<f64>, f64)>],
    want: &[(usize, f64)],
    her: &[(usize, bool, f64)],
    x: usize,
    a: usize,
    b: usize,
    sel: &Sel,
    acache: &mut Vec<HiAnc>,
) -> Vec<f64> {
    let n = bs.cands.len();
    let ntar = s.env.ntar;
    let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone();
    let fixed_t = s.env.fixed_t.clone();
    let w = s.env.sj.star_dist;
    let (xi, goal) = her_ends(s, bs, legs, x, a, b);
    let c = s.make_cur(pool, x, a, b, sel, false);
    let pv = her_pool(s, legs, pool, true, x, a, b, &c);
    let iv = pv.me.len() - 1;
    // per candidate and EVERY legal partner: her slot's miss factors and the spark scale, as `score_career`'s setups
    let nleaf = 1usize << (2 * nu);
    let nm = 1usize << ntar;
    // (candidate, bound at scale 1 per leaf, spark scale per m): kept only for the exact anchors (`HI_SEARCHES` > 0); else
    // each outcome reads `tab`, the same maxima folded in as each partner is done
    let mut hs: Vec<(usize, Vec<f64>, Vec<f64>)> = vec![];
    let mut hq: Vec<(usize, Vec<f64>, Vec<f64>, [f64; 4])> = vec![]; // per partner: (trainee, q0, x0, xms), for the anchors
    let rf = s.env.space.rho_top() * s.fixed;
    let (lp, lp_tmp): (Vec<f64>, Vec<f64>) = (vec![], vec![]);
    // an outcome (her stars `leaf`, the fixed targets landed fm: cell leaf << nf | fm): its chance and landed set
    let nf = fixed_t.len();
    let ncell = nleaf << nf;
    // (`score_career`'s cells: the landed set of her stars' leaf and the fixed targets' fm, its chance os[m] x her stars')
    let fmm = fm_masks(&fixed_t);
    let cell = |i: usize| {
        let (mu, pr_st) = leaf_set(i >> nf, &uncv, w);
        let m = mu | fmm[i & ((1usize << nf) - 1)];
        (c.os[m] * pr_st, m)
    };
    // the outcomes whose bound is read (`want`, ascending, each with its y < 0.5: the page reads y_hi only there) that
    // can happen; elsewhere the bound is not computed (no cell's value depends on another's: each is the most over
    // partners of a per-leaf least, computed alone)
    let wc: Vec<(f64, usize)> = want.iter().map(|&(i, _)| cell(i)).collect();
    let mc: Vec<usize> = want.iter().zip(&wc).filter(|(_, p)| p.0 > 0.0).map(|(w, _)| w.0).collect();
    let mut mk = Marks {
        lv: vec![vec![]; nu + 1],
        dense: false,
        unm: vec![],
        ix: vec![vec![]; nu + 1],
        par: vec![vec![]; nu + 1],
    };
    mk.lv[nu] = vec![false; nleaf];
    for &i in &mc {
        mk.lv[nu][i >> nf] = true;
    }
    for j in (0..nu).rev() {
        let np = 1usize << (2 * j);
        let up = &mk.lv[j + 1];
        mk.lv[j] = (0..np).map(|p| up[p] || up[np + p] || up[2 * np + p] || up[3 * np + p]).collect();
    }
    let nml = mk.lv[nu].iter().filter(|&&b| b).count();
    mk.dense = 4 * nml >= nleaf;
    if mk.dense {
        mk.unm = (0..nleaf).filter(|&l| !mk.lv[nu][l]).collect();
    } else {
        for j in 0..=nu {
            mk.ix[j] = (0..mk.lv[j].len()).filter(|&p| mk.lv[j][p]).map(|p| p as u32).collect();
            if j > 0 {
                // prefix p of j digits: parent p mod 4^(j-1), found in the ascending `ix[j - 1]`
                let (lo, cu) = (&mk.ix[j - 1], &mk.ix[j]);
                let m = (1u32 << (2 * (j - 1))) - 1;
                mk.par[j] = cu.iter().map(|&p| lo.binary_search(&(p & m)).unwrap() as u32).collect();
            }
        }
    }
    prof("hi_cells", mc.len() as f64);
    prof("hi_mleaves", nml as f64);
    if mc.is_empty() {
        return vec![0.0; want.len()];
    }
    // the cells the table holds: the marked ones, or with most leaves marked every possible one, folded as before
    // (vectorised where every cell is possible; an unmarked leaf's bound is -inf, so its cells keep p_now, never read)
    let mc: Vec<usize> = if mk.dense { (0..ncell).filter(|&i| cell(i).0 > 0.0).collect() } else { mc };
    let ms: Vec<usize> = mc.iter().map(|&i| cell(i).1).collect();
    let lm: Vec<u32> = ms.iter().map(|&m| m as u32).collect();
    let nmc = mc.len();
    // per candidate and marked outcome her odds' bound so far: pnow, then the most of every partner's (`hi_at` without `hs`)
    let mut tab: Vec<f64> = (0..n).flat_map(|ui| std::iter::repeat_n(bs.pnow[ui], nmc)).collect();
    // an anchor's chances (`Env::lineage_qc`: only the uncertain targets' entries, the ones the factors read)
    let mut hp = HiScr { bb: AncBb::new(), g1: vec![], gc: vec![], lp, lp_tmp, qz: vec![0.0f64; ntar], tg: vec![] };
    let hx =
        HiCx { bs, d, legs, pool, fix, uncv: &uncv, mk: &mk, mc: &mc, lm: &lm, nu, ntar, nm, nf, ncell, nmc, iv, rf };
    // (each (candidate, partner)'s anchors as a call keeps them for the next, `acache`: in this loop's order)
    let mut np = 0usize;
    for ui in 0..n {
        let u = bs.cands[ui];
        if d.umas[u].id == d.umas[x].id || bs.c1[ui] == 0.0 || s.fastc[u].is_empty() {
            continue;
        }
        // her outfits' skill coverage and own chances (as `Env::rise` reads them)
        let (sts, ews) = (&s.env.sts0[u], &s.env.ew0[u]);
        hp.tg.clear(); // (`Env::lineage_qc`'s table: another trainee)
        // the main search's found careers of her deck class that it fully checked (`found_anchors`)
        let ct = s.env.cls[u];
        let fr: Vec<usize> = (0..fr_of.len()).filter(|&i| fr_of[i].as_ref().is_some_and(|f| f.0 == ct)).collect();
        // `AncBb::root_min`'s premises on her own chances
        let ew_ok = ews.iter().all(|ew| ew.iter().all(|e| (0.0..=1.0).contains(e)));
        for y in (0..iv).filter(|&y| legal_pair(d, &pv, u, y, iv)) {
            if acache.len() <= np {
                acache.push(((ui, y), None));
            }
            if acache[np].0 != (ui, y) {
                acache[np] = ((ui, y), None);
            }
            hi_partner(
                s,
                &hx,
                &pv,
                ui,
                u,
                y,
                sts,
                ews,
                ew_ok,
                &fr,
                fr_of,
                &mut tab[ui * nmc..(ui + 1) * nmc],
                &mut hp,
                &mut hs,
                &mut hq,
                &mut acache[np].1,
            );
            np += 1;
        }
    }
    her_pool_done(s, pool, true, pv);
    let HiScr { mut lp, mut lp_tmp, .. } = hp;
    let full_m = nm - 1;
    // per wanted cell its bound (0 where it can't happen), from its place among the cells held (`mc`, ascending)
    let mut ys: Vec<f64> = vec![0.0; want.len()];
    let at: Vec<Option<usize>> =
        want.iter().zip(&wc).map(|(w, p)| if p.0 > 0.0 { mc.binary_search(&w.0).ok() } else { None }).collect();
    let mut c1 = bs.c1.to_vec();
    let mut up = vec![0.0f64; n];
    let t5 = now_ms();
    for (k, &j) in at.iter().enumerate() {
        let Some(j) = j else { continue };
        for ui in 0..n {
            up[ui] = tab[ui * nmc + j];
        }
        ys[k] = saving_lo(bs, &up, ms[j], full_m, xi, her, goal, &mut c1);
    }
    drop(tab);
    prof("hi_ys_ms", now_ms() - t5);
    // exact anchors (`HI_SEARCHES`) for the partners that hold the most likely outcomes of the band (y < 0.5 <= y_hi) in
    // it: per band outcome, most likely first, the partners that must all fall for it to leave the band (every partner
    // above p_now of each candidate whose odds move V; none further than HI_RISE above p_now), taken whole while they fit
    let mut band: Vec<usize> = (0..want.len()).filter(|&k| at[k].is_some() && ys[k].max(want[k].1) >= 0.5).collect();
    band.sort_by(|&i, &j| wc[j].0.partial_cmp(&wc[i].0).unwrap().then(want[i].0.cmp(&want[j].0)));
    let cap = HI_SEARCHES / HI_STARS.len();
    let mut chosen: Vec<usize> = vec![];
    for &k in &band {
        if chosen.len() >= cap {
            break;
        }
        let (leaf, m) = (want[k].0 >> nf, wc[k].1);
        let h = hi_at(bs, &hs, leaf, m, full_m, xi, her, goal, &mut c1, &mut up);
        let (mut need, mut far): (Vec<usize>, bool) = (vec![], false);
        for ui in 0..n {
            if up[ui] <= bs.pnow[ui] * (1.0 + 1e-9) {
                continue;
            }
            let keep = up[ui];
            up[ui] = bs.pnow[ui];
            let h2 = saving_lo(bs, &up, m, full_m, xi, her, goal, &mut c1);
            up[ui] = keep;
            if h2 >= h - 1e-9 {
                continue;
            }
            for (pi, (pu, bd, sc)) in hs.iter().enumerate() {
                if *pu != ui {
                    continue;
                }
                let v = sc[m] * bd[leaf];
                if v > bs.pnow[ui] * (1.0 + 1e-9) {
                    need.push(pi);
                    if v > HI_RISE * bs.pnow[ui] {
                        far = true;
                    }
                }
            }
        }
        if far || need.is_empty() {
            continue;
        }
        need.retain(|pi| !chosen.contains(pi));
        if chosen.len() + need.len() <= cap {
            chosen.extend(need);
        }
    }
    // each one: her best deck searched at her s stars on every uncertain target (q^s, `HI_STARS`), proven (`best_deck`
    // full, under the caps): value(y, her) <= c' E_best(q^s) rise(q', q^s) for every outcome (rise 1 where her stars <= s)
    for &pi in &chosen {
        let (u, q0, x0, xms) = &hq[pi];
        let (sts, ews) = (s.env.sts0[*u].clone(), s.env.ew0[*u].clone());
        for &st in &HI_STARS {
            let mut q = q0.clone();
            for k in 0..nu {
                q[uncv[k]] = 1.0 - x0[k] * xms[st];
            }
            s.env.set_trainee(*u, &q);
            s.env.work = 0;
            let mut nsc = 0;
            if let DeckRes::Done(v, _, true) = s.env.best_deck(*u, 1.0, 0.0, &[], &mut nsc) {
                let rz = rise_leaves(&sts, &ews, x0, xms, &q, &uncv, &mut lp, &mut lp_tmp);
                for (bv, r) in hs[pi].1.iter_mut().zip(&rz) {
                    let v2 = v * r;
                    if v2 < *bv {
                        *bv = v2;
                    }
                }
            }
        }
    }
    if !chosen.is_empty() {
        for &k in &band {
            ys[k] = hi_at(bs, &hs, want[k].0 >> nf, wc[k].1, full_m, xi, her, goal, &mut c1, &mut up);
        }
    }
    ys
}
/// The pick's own 8/8 as a trio side (`her_trios`) for `outcome_hi`, taken as the plan ends (the deck cache then:
/// `her_trios` reads C's cached decks), where an outcome that lands every target reads y < 0.5 (`full_lo`: only there
/// does `saving_lo` read it; else none)
fn pick_her(
    s: &mut Sim,
    d: &Data,
    bs: &ScoreBase,
    legs: &[Legacy],
    pool: &Pool,
    x: usize,
    a: usize,
    b: usize,
    sel: &Sel,
    full_lo: bool,
    buf: &mut Vec<f64>,
) -> Vec<(usize, bool, f64)> {
    let (xi, goal) = her_ends(s, bs, legs, x, a, b);
    if !full_lo || xi.is_some() || goal || bs.sides.is_empty() {
        return vec![];
    }
    let c = s.make_cur(pool, x, a, b, sel, false);
    let full_m = (1usize << s.env.ntar) - 1;
    if c.os[full_m] <= 0.0 {
        return vec![];
    }
    let mut pv = her_pool(s, legs, pool, true, x, a, b, &c);
    let iv = pv.me.len() - 1;
    let r = her_trios(s, d, bs, &mut pv, iv, x, buf);
    her_pool_done(s, pool, true, pv);
    r
}
/// `outcome_hi`'s found-career anchors: per found career of the main search `msv` (its order), (her deck class, q_f,
/// val_f) when the search fully checked it (a Ref at the career's own chances: val_f is the best value of any deck at q_f
/// for every trainee of the class, `Env::seen`, scale 1), else none
fn found_anchors(env: &Env, pool: &Pool, msv: &Solved) -> Vec<Option<(usize, Vec<f64>, f64)>> {
    msv.found
        .iter()
        .map(|f| {
            let (fq, _) = env.lineage_q(pool, f.t, f.a, f.b);
            msv.refs.get(f.t).and_then(|rs| rs.iter().find(|r| r.q == fq)).map(|r| (env.cls[f.t], r.q.clone(), r.val))
        })
        .collect()
}

// ---------- a roll of the plan's pick, on demand (`next.roll`) ----------
/// One scoring's outcome evaluation as `sc_eval` reads it (built by `score_career_`; a plan keeps its pick's for a roll,
/// `RollKeep`). Per setup her miss factor's parts (x0 per uncertain target, xm per star level) and her spark scales (pw
/// per job target, without and with her spark), with what the fold and the outcome loop read of them (`derive`: xs, xc
/// and scf); the tables kept (`sc_prep`: level 0 of the fold, table j at j 2^nu) and each one's setup; the collapsed
/// targets; per setup its candidate slot and per slot the odds now; her landed sets' chances (`os`); the candidates the
/// loop can lift (`act`); her own 8/8's trio sides (`her`); her candidate index, the goal, the star mix
pub(super) struct Scored {
    nu: usize,
    nf: usize,
    ntar: usize,
    na: usize,
    xi: Option<usize>,
    goal: bool,
    w: [f64; 3],
    uncv: Vec<usize>,
    fmm: Vec<usize>,
    x0: Vec<Vec<f64>>,
    xm: Vec<[f64; 4]>,
    pw: Vec<Vec<[f64; 2]>>,
    tsi: Vec<usize>,
    t0: Vec<f64>,
    col: Vec<bool>,
    sslot: Vec<usize>,
    p0: Vec<f64>,
    os: Vec<f64>,
    act: Vec<(usize, Option<usize>, f64, f64)>,
    her: Vec<(usize, bool, f64)>,
    xs: Vec<Vec<[f64; 4]>>,
    xc: Vec<Vec<[f64; 4]>>,
    scf: Vec<f64>,
}
impl Scored {
    /// what the fold and the outcome loop read of the setups: per setup, target and star level x = x0 xm and 1 - x; per
    /// landed set m and setup the spark scale, the product over job targets of pw at her spark there (m-major:
    /// scf[m * ns + si])
    fn derive(&mut self) {
        let nu = self.nu;
        self.xs = self
            .x0
            .iter()
            .zip(&self.xm)
            .map(|(x0, xm)| (0..nu).map(|k| [0usize, 1, 2, 3].map(|sg| x0[k] * xm[sg])).collect())
            .collect();
        self.xc = self.xs.iter().map(|v| v.iter().map(|r| r.map(|x| 1.0 - x)).collect()).collect();
        let (nm, ntar) = (1usize << self.ntar, self.ntar);
        self.scf = Vec::with_capacity(nm * self.ns());
        for m in 0..nm {
            self.scf.extend(self.pw.iter().map(|pw| (0..ntar).map(|k| pw[k][(m >> k) & 1]).product::<f64>()));
        }
    }
    const fn ns(&self) -> usize {
        self.x0.len()
    }
    /// the fold writes per leaf the setups' table values (fewer rows than per outcome and slot), else per outcome the
    /// slots' odds (`fold_emit`)
    const fn pl(&self) -> bool {
        (1usize << self.nf) * self.na > self.ns()
    }
    const fn full_m(&self) -> usize {
        (1usize << self.ntar) - 1
    }
    /// a leaf's representative: every collapsed target's nonzero digit set to 1 (`sc_outcomes`)
    fn rep_of(&self, l: usize) -> usize {
        let cm: usize = (0..self.nu).filter(|&k| self.col[k]).map(|k| 3usize << (2 * k)).sum();
        let lo_bits = cm & (usize::MAX / 3);
        let z = l & cm;
        (l & !cm) | ((z | z >> 1) & lo_bits)
    }
}
/// The base a kept scoring was scored against (`ScoreBase` without the sides and C's umas, which only `her_trios` reads)
struct KeptBase {
    cands: Vec<usize>,
    c1: Vec<f64>,
    v0: f64,
    pnow: Vec<f64>,
    cont: Vec<Cont>,
    combos: Vec<Trio>,
    m23: Vec<f64>,
}
impl KeptBase {
    fn of(bs: &ScoreBase) -> Self {
        Self {
            cands: bs.cands.to_vec(),
            c1: bs.c1.to_vec(),
            v0: bs.v0,
            pnow: bs.pnow.to_vec(),
            cont: bs.cont.to_vec(),
            combos: bs.combos.to_vec(),
            m23: bs.m23.to_vec(),
        }
    }
    fn base(&self) -> ScoreBase<'_> {
        ScoreBase {
            cands: &self.cands,
            c1: &self.c1,
            m23: &self.m23,
            v0: self.v0,
            pnow: &self.pnow,
            cont: &self.cont,
            combos: &self.combos,
            sides: &[],
            all: &[],
        }
    }
}
/// A plan's roll state (`next.roll`), kept per plan job (`roll_key`): the pick's last scoring (`Scored`) and the base it
/// was scored against, the pick's career, and what its bound reads beyond them (`outcome_hi`): her own 8/8's trio sides as
/// the plan's end has them (`pick_her`) and the main search's found-career anchors (`found_anchors`)
pub(in super::super) struct RollKeep {
    key: u64,
    sd: Scored,
    base: KeptBase,
    x: usize,
    a: usize,
    b: usize,
    sel: Sel,
    her_hi: Vec<(usize, bool, f64)>,
    anc: Vec<Option<(usize, Vec<f64>, f64)>>,
    /// the bound's anchors per (candidate, partner), kept by its first roll for the next (`HiAnc`; not stored by `state_out`)
    hi_anc: Vec<HiAnc>,
}
thread_local! { static ROLLS: std::cell::RefCell<Vec<RollKeep>> = const { std::cell::RefCell::new(Vec::new()) }; }
/// plans' roll states kept (the most recent last): the run's plan and the plan Compare makes with her added
const ROLL_KEEP: usize = 4;
/// a plan job's roll key: the whole job but its roll (`next.roll`) and the page's flags that change no plan
/// (`no_reports`, `compare`)
fn roll_key(job: &Value) -> u64 {
    h64(&crate::deck::career::json_without(job, &["no_reports", "compare", "roll"], &["roll"]))
}
fn keep_roll(roll: RollKeep) {
    ROLLS.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|entry| entry.key != roll.key);
        cache.push(roll);
        while cache.len() > ROLL_KEEP {
            cache.remove(0);
        }
    });
}
/// The plan's answer to a roll (`next.roll`: her stars per job target, 0 where it did not land) from its kept roll state,
/// with no search (None when the worker keeps none for this plan job: then the plan runs and answers it, `index_rule`).
/// y: the careers her veteran saves at that outcome, the pick's kept scoring evaluated at that one outcome (`sc_eval`,
/// the plan's own arithmetic in its order: the f64 the plan's full table held); where y < 0.5 the bound y_hi
/// (`outcome_hi` at that one cell). Rounded as the table was: y to 0.01, y_hi up to 0.01 (null where y >= 0.5)
pub(in super::super) fn roll_kept<'data>(
    env: &mut Env<'data>,
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    legs: &[Legacy],
    pool: &Pool,
    fix: &[usize],
    pj: &PlanJob,
) -> Option<Value> {
    let stars = pj.roll.as_ref()?;
    let key = roll_key(job);
    let mut kept = ROLLS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|roll| roll.key == key)?;
        Some(cache.remove(index))
    })?;
    // (the roll's own timers and counters, `prof`; what the job counted before goes back after)
    let started = now_ms();
    let prof_before = PROF.with(|prof| std::mem::take(&mut *prof.borrow_mut()));
    let answer = match sim_for(env, d, sk, cr, job, pj) {
        Ok(mut sim) => roll_answer(&mut sim, d, legs, pool, fix, &mut kept, stars),
        Err(error) => error,
    };
    keep_roll(kept);
    let mut answer = answer;
    let roll_prof = PROF.with(|prof| std::mem::replace(&mut *prof.borrow_mut(), prof_before));
    if answer.is_object() {
        answer["ms"] = json!(now_ms() - started);
        answer["prof"] = json!(roll_prof);
    }
    Some(answer)
}
fn roll_answer(
    s: &mut Sim,
    d: &Data,
    legs: &[Legacy],
    pool: &Pool,
    fix: &[usize],
    k: &mut RollKeep,
    stars: &[u8],
) -> Value {
    let (uncv, fixed_t) = (s.env.uncv.clone(), s.env.fixed_t.clone());
    if stars.len() != s.env.ntar {
        return json!({"error": format!("roll: {} stars for {} targets", stars.len(), s.env.ntar)});
    }
    // the outcome: per uncertain target her stars (at most 3) as base-4 digits in `uncv` order, per guaranteed one
    // whether it landed
    let leaf = uncv.iter().enumerate().fold(0usize, |l, (j, &i)| l | (stars[i].min(3) as usize) << (2 * j));
    let fm = fixed_t.iter().enumerate().fold(0usize, |m, (j, &i)| if stars[i] > 0 { m | 1 << j } else { m });
    let mut ac = std::mem::take(&mut k.hi_anc);
    let bs = k.base.base();
    let y = sc_roll(&bs, &k.sd, leaf, fm);
    let hi = (y < 0.5).then(|| {
        outcome_hi(
            s,
            d,
            &bs,
            legs,
            pool,
            fix,
            &k.anc,
            &[(leaf << fixed_t.len() | fm, y)],
            &k.her_hi,
            k.x,
            k.a,
            k.b,
            &k.sel,
            &mut ac,
        )[0]
    });
    k.hi_anc = ac;
    // (y_hi >= y is derived, `outcome_hi` (1)-(3): checked here, within rounding, where the max below would hide a bound
    // that drifted under the estimate; the max stays for the page)
    debug_assert!(hi.is_none_or(|h| h >= y - 1e-9 * y.abs().max(1.0)), "roll: y_hi {hi:?} below y {y}");
    json!({"y": (y * 100.0).round() / 100.0, "y_hi": hi.map(|h| (h.max(y) * 100.0).ceil() / 100.0)})
}
/// the kept roll states for the page to store (`state_out`), every float as its bits
pub(in super::super) fn rolls_out() -> Value {
    use super::super::{fvec, hx, sel_out, ux};
    let tri = |v: &[(usize, bool, f64)]| v.iter().map(|&(i, r, c)| json!([i, r, hx(c)])).collect::<Vec<_>>();
    ROLLS.with(|c| Value::Array(c.borrow().iter().map(|r| { let (sd, b) = (&r.sd, &r.base); json!({
        "k": ux(r.key), "x": [r.x, r.a, r.b], "sel": sel_out(&r.sel), "her": tri(&r.her_hi),
        "anc": r.anc.iter().map(|a| a.as_ref().map(|(c, q, v)| json!([c, fvec(q), hx(*v)]))).collect::<Vec<_>>(),
        "base": {"cands": b.cands, "c1": fvec(&b.c1), "v0": hx(b.v0), "pnow": fvec(&b.pnow), "m23": fvec(&b.m23),
                 "cont": b.cont.iter().map(|c| json!([hx(c.lam), hx(c.phi), hx(c.pinf), !c.tab.is_empty()])).collect::<Vec<_>>(),
                 "combos": b.combos.iter().map(|&(ia, ib, c3, ra, rb)| json!([ia, ib, hx(c3), ra, rb])).collect::<Vec<_>>()},
        "sd": {"n": [sd.nu, sd.nf, sd.ntar, sd.na], "xi": sd.xi, "goal": sd.goal, "w": fvec(&sd.w), "uncv": sd.uncv, "fmm": sd.fmm,
               "x0": sd.x0.iter().map(|v| fvec(v)).collect::<Vec<_>>(), "xm": sd.xm.iter().map(|v| fvec(v)).collect::<Vec<_>>(),
               "pw": sd.pw.iter().map(|v| fvec(&v.iter().flatten().copied().collect::<Vec<f64>>())).collect::<Vec<_>>(),
               "tsi": sd.tsi, "t0": fvec(&sd.t0), "col": sd.col, "sslot": sd.sslot, "p0": fvec(&sd.p0), "os": fvec(&sd.os),
               "act": sd.act.iter().map(|&(ui, sl, th, pn)| json!([ui, sl, hx(th), hx(pn)])).collect::<Vec<_>>(), "her": tri(&sd.her)}}) }).collect()))
}
pub(in super::super) fn rolls_in(v: &Value) -> Option<Vec<RollKeep>> {
    use super::super::{STATE_MAX_N, fvec_r, fx, pvec_r, sel_in, usz, uxr};
    // (absent in a state stored before them: none)
    let Some(a) = v.as_array() else { return v.is_null().then(Vec::new) };
    let uv = |v: &Value| -> Option<Vec<usize>> { v.as_array()?.iter().map(usz).collect() };
    let tri = |v: &Value| -> Option<Vec<(usize, bool, f64)>> {
        v.as_array()?.iter().map(|t| Some((usz(&t[0])?, t[1].as_bool()?, fx(&t[2])?))).collect()
    };
    let arr4 = |v: &Value| -> Option<[f64; 4]> {
        let x = fvec_r(v)?;
        x.try_into().ok()
    };
    a.iter()
        .map(|e| {
            let (b, s) = (&e["base"], &e["sd"]);
            let n = uv(&s["n"])?;
            // (the plan's limits, `sim_for`: at most 12 targets, 10 of them uncertain, the rest guaranteed; checked before
            // any table is sized by them)
            if n.len() != 4 || n[2] > 12 || n[0] > 10 || n[1] > 12 || n[0] + n[1] != n[2] {
                return None;
            }
            let cont = b["cont"]
                .as_array()?
                .iter()
                .map(|c| {
                    let (lam, phi, pinf) = (fx(&c[0])?, fx(&c[1])?, fx(&c[2])?);
                    // (a Cont without its table has no building: `Cont::default`'s lam 0; one that reads as building is refused)
                    Some(if c[3].as_bool()? {
                        Cont::new(lam, phi, pinf)
                    } else if lam > 0.0 && phi > 0.0 {
                        return None;
                    } else {
                        Cont { lam, phi, pinf, ..Cont::default() }
                    })
                })
                .collect::<Option<Vec<Cont>>>()?;
            let combos = b["combos"]
                .as_array()?
                .iter()
                .map(|t| Some((usz(&t[0])?, usz(&t[1])?, fx(&t[2])?, t[3].as_bool()?, t[4].as_bool()?)))
                .collect::<Option<Vec<Trio>>>()?;
            let base = KeptBase {
                cands: uv(&b["cands"])?,
                c1: fvec_r(&b["c1"])?,
                v0: fx(&b["v0"])?,
                pnow: fvec_r(&b["pnow"])?,
                cont,
                combos,
                m23: fvec_r(&b["m23"])?,
            };
            let w: [f64; 3] = pvec_r(&s["w"])?.try_into().ok()?;
            let pw = s["pw"]
                .as_array()?
                .iter()
                .map(|v| {
                    let f = fvec_r(v)?;
                    (f.len() == 2 * n[2]).then(|| f.chunks(2).map(|c| [c[0], c[1]]).collect())
                })
                .collect::<Option<Vec<Vec<[f64; 2]>>>>()?;
            let act = s["act"]
                .as_array()?
                .iter()
                .map(|t| {
                    Some((usz(&t[0])?, if t[1].is_null() { None } else { Some(usz(&t[1])?) }, fx(&t[2])?, fx(&t[3])?))
                })
                .collect::<Option<Vec<_>>>()?;
            let mut sd = Scored {
                nu: n[0],
                nf: n[1],
                ntar: n[2],
                na: n[3],
                xi: if s["xi"].is_null() { None } else { Some(usz(&s["xi"])?) },
                goal: s["goal"].as_bool()?,
                w,
                uncv: uv(&s["uncv"])?,
                fmm: uv(&s["fmm"])?,
                x0: s["x0"].as_array()?.iter().map(fvec_r).collect::<Option<_>>()?,
                xm: s["xm"].as_array()?.iter().map(arr4).collect::<Option<_>>()?,
                pw,
                tsi: uv(&s["tsi"])?,
                t0: fvec_r(&s["t0"])?,
                col: s["col"].as_array()?.iter().map(serde_json::Value::as_bool).collect::<Option<_>>()?,
                sslot: uv(&s["sslot"])?,
                p0: fvec_r(&s["p0"])?,
                os: pvec_r(&s["os"])?,
                act,
                her: tri(&s["her"])?,
                xs: vec![],
                xc: vec![],
                scf: vec![],
            };
            // (a store that does not fit together is refused whole, as any other: the vectors' lengths and every index
            // within what it indexes, as `score_career_` builds them, before `derive` sizes its tables)
            let (ns, nc) = (sd.x0.len(), base.cands.len());
            let fixed: Vec<usize> = (0..sd.ntar).filter(|i| !sd.uncv.contains(i)).collect();
            let fits_base = nc <= STATE_MAX_N
                && base.c1.len() == nc
                && base.pnow.len() == nc
                && base.m23.len() == nc
                && base.cont.len() <= nc
                && base.combos.iter().all(|t| t.0 < nc && t.1 < nc);
            // her setups (at most IDX_PARTNERS per candidate), the candidates the loop lifts (those with a slot in slot
            // order, one per slot), her own candidate index and trio sides: candidate indices
            let fits_cands = ns <= IDX_PARTNERS * nc
                && sd.act.iter().filter_map(|a| a.1).eq(0..sd.na)
                && sd.act.iter().all(|a| a.0 < nc)
                && sd.xi.is_none_or(|x| x < nc)
                && sd.her.iter().all(|h| h.0 < nc);
            if !fits_base
                || !fits_cands
                || !sd.uncv.windows(2).all(|p| p[0] < p[1])
                || sd.uncv.last().is_some_and(|&i| i >= sd.ntar)
                || sd.fmm != fm_masks(&fixed)
                || sd.sslot.windows(2).any(|p| p[0] > p[1])
                || sd.uncv.len() != sd.nu
                || sd.col.len() != sd.nu
                || sd.xm.len() != ns
                || sd.pw.len() != ns
                || sd.sslot.len() != ns
                || sd.os.len() != 1usize << sd.ntar
                || sd.fmm.len() != 1usize << sd.nf
                || sd.t0.len() != sd.tsi.len() << sd.nu
                || sd.x0.iter().any(|v| v.len() != sd.nu)
                || sd.tsi.iter().any(|&si| si >= ns)
                || sd.sslot.iter().any(|&sl| sl >= sd.na)
                || sd.p0.len() != sd.na
            {
                return None;
            }
            sd.derive();
            let x = uv(&e["x"])?;
            if x.len() != 3 {
                return None;
            }
            let anc = e["anc"]
                .as_array()?
                .iter()
                .map(|a| if a.is_null() { Some(None) } else { Some(Some((usz(&a[0])?, pvec_r(&a[1])?, fx(&a[2])?))) })
                .collect::<Option<Vec<_>>>()?;
            // (the anchors' chances: one per job target; her trio sides at the plan's end: candidate indices)
            let her_hi = tri(&e["her"])?;
            if anc.iter().flatten().any(|a| a.1.len() != sd.ntar) || her_hi.iter().any(|h| h.0 >= nc) {
                return None;
            }
            Some(RollKeep {
                key: uxr(&e["k"])?,
                sd,
                base,
                x: x[0],
                a: x[1],
                b: x[2],
                sel: sel_in(&e["sel"])?,
                her_hi,
                anc,
                hi_anc: vec![],
            })
        })
        .collect()
}
pub(in super::super) fn rolls_set(r: Vec<RollKeep>) {
    ROLLS.with(|c| *c.borrow_mut() = r);
}

/// Inputs shared by the phases. Under `borrowed_parent: "later"`, `pick` is the pool
/// without borrowed parents and its main search. The candidates, builders, trios and
/// scoring partners still use `pool` and `sv`; only runnable rows use the pick's pool.
struct PlanInputs<'plan, 'data> {
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    legs: &'plan [Legacy],
    pool: &'plan Pool,
    sv: &'plan Solved,
    fix: &'plan [usize],
    trainees: &'plan [usize],
    pj: &'plan PlanJob,
    pick: Option<(&'plan Pool, &'plan Solved)>,
}

impl<'plan> PlanInputs<'plan, '_> {
    /// The pool runnable rows use: the pick's, else `pool`.
    fn pick_pool(&self) -> &'plan Pool {
        self.pick.map_or(self.pool, |p| p.0)
    }

    /// The main search runnable rows fall back to: the pick's, else `sv`.
    fn pick_solved(&self) -> &'plan Solved {
        self.pick.map_or(self.sv, |p| p.1)
    }

    /// A career that may run: its parents are legal on the pick's pool (the borrowing rule). Rows repeating another
    /// row's career are dropped later, at output.
    fn is_runnable(&self, f: &Found) -> bool {
        self.pick_pool().bor_ok(f.a, f.b)
    }
}

/// Candidate order and the starting odds stay fixed while later searches add decks.
struct PlanCandidates {
    /// Main searches with borrowing allowed, and with the pick's borrowing rule.
    main_career: Found,
    pick_career: Found,
    members: Vec<usize>,
    loop_member_info: Vec<Value>,
    loop_member_note: Option<String>,
    hint: Vec<Value>,
    trainees: Vec<usize>,
    allowed_trainees: Vec<usize>,
    /// Initial candidate careers: later builder searches do not replace these.
    careers: Vec<Option<Found>>,
    pnow: Vec<f64>,
    have8: Vec<bool>,
    zeros: Vec<f64>,
    ones: Vec<f64>,
    proxy_on: bool,
}

impl PlanCandidates {
    /// Placeholder step costs gather every candidate's lifts, before the continuation is fitted.
    fn stats_base(&self) -> ScoreBase<'_> {
        ScoreBase {
            cands: &self.trainees,
            c1: &self.ones,
            m23: &self.zeros,
            v0: 0.0,
            pnow: &self.pnow,
            cont: &[],
            combos: &[],
            sides: &[],
            all: &[],
        }
    }
}

/// Searches and their completeness, including the kept plans used to extend this pool.
struct PlanSearches {
    best: Vec<CachedSearch>,
    proof_refs: Vec<Vec<Vec<Ref>>>,
    /// Per-trainee incomplete searches: base, looped and no-borrow careers.
    base_incomplete: Vec<bool>,
    looped_incomplete: Vec<bool>,
    no_borrow_incomplete: Vec<bool>,
    looped_careers: Vec<CachedSearch>,
    kept_same_pool: Option<Bests>,
    kept_previous_pool: Option<Bests>,
    original_budget: usize,
}

/// The plan's timers and work counters, and the surrogate's differences (`sur` 2): recorded by the phases, reported
/// with the output.
struct PlanProfile {
    /// The plan's start, and pass 1's.
    t_all: f64,
    t3: f64,
    /// Choosing the candidates, and the career searches.
    loop_ms: f64,
    search_ms: f64,
    /// The work counters at the start (deck-search nodes, exact scorings): the plan reports its own.
    nodes0: usize,
    joints0: usize,
    /// Admitted proxies certified by their class's proofs, without a search.
    n_cert: usize,
    exact_n: usize,
    sur_diff: Vec<Value>,
}

/// Pass 1's rows (`RowOrigin::Base`), and each one's G1 wins, read when fitting a continuation.
struct BaseRows {
    runs: Vec<PlanRow>,
    /// indexed by uma (her index in `d.umas`), filled for each row's trainee: the G1 races her latest row's career
    /// wins with a chance above 0.5 (`set_plan_wins`), the continuation fit's shared-win input; empty for other umas
    wins_by_trainee: Vec<Vec<usize>>,
}

/// A and B index `PlanCandidates::trainees`; the finisher indexes `Data::umas` directly.
/// Each side uses either an existing full veteran (a free step) or a newly built one.
#[derive(Clone, Copy)]
struct PlanCombination {
    candidate_a_index: usize,
    candidate_b_index: usize,
    /// Expected careers for the finisher, after A and B are full veterans.
    finisher_cost: f64,
    finisher_trainee: usize,
    a_uses_existing: bool,
    b_uses_existing: bool,
}

impl PlanCombination {
    fn total_cost(&self, stage_costs: &[f64]) -> f64 {
        let a_cost = if self.a_uses_existing { 0.0 } else { stage_costs[self.candidate_a_index] };
        let b_cost = if self.b_uses_existing { 0.0 } else { stage_costs[self.candidate_b_index] };
        a_cost + b_cost + self.finisher_cost
    }

    const fn as_trio(&self) -> Trio {
        (self.candidate_a_index, self.candidate_b_index, self.finisher_cost, self.a_uses_existing, self.b_uses_existing)
    }
}

/// Continuation and trio values fitted together for scoring the plan's rows.
/// Per-candidate buffers use the fixed order in `PlanCandidates::trainees`.
struct PlanFit {
    continuations: Vec<Cont>,
    step_costs: Vec<f64>,
    trios: Vec<Trio>,
    sides: Vec<Vec<(bool, Legacy)>>,
    current_value: f64,
}

impl PlanFit {
    fn score_base<'fit>(&'fit self, candidates: &'fit PlanCandidates) -> ScoreBase<'fit> {
        ScoreBase {
            cands: &candidates.trainees,
            c1: &self.step_costs,
            m23: &candidates.zeros,
            v0: self.current_value,
            pnow: &candidates.pnow,
            cont: &self.continuations,
            combos: &self.trios,
            sides: &self.sides,
            all: &candidates.allowed_trainees,
        }
    }
}

/// Pass 2's completed fit and reporting results, used by the proxy check and output assembly.
struct PlanScores {
    fit: PlanFit,
    best_combo: Option<PlanCombination>,
    any_scored: bool,
    combo_ms: f64,
    index_ms: f64,
    work: Value,
}

/// The page's next career, exact where it can be and without simulation: the career that brings a FULLY LOOPED veteran
/// (an 8/8 whose parents are 8/8s) the closest. V = the cheapest trio: A and B two stage candidates, each step "build
/// while it pays, then run her" (`Cont`, fitted to the pool now; free when an 8/8 of hers exists), C the best allowed
/// uma with A's and B's 8/8s as parents (1 / her full-loop odds). A career is scored E[V - V with her veteran] - 1, exact
/// over her outcomes: her veteran as a parent raising any candidate's odds lowers that step, her own 8/8 ends her own
/// step; a roll that changes nothing leaves every score as it was (repeat). Pick: the highest;
/// within NEAR, the better own odds. The goal is always the next fully looped parent, any uma: one you have already is a
/// real 8/8 (free) with 8/8 parents, a strong side of the trios.
pub(in super::super) fn index_rule<'data>(
    env: &mut Env<'data>,
    d: &'data Data,
    sk: &'data SkillsRaw,
    cr: &'data CardsRaw,
    job: &'data Value,
    legs: &[Legacy],
    pool: &Pool,
    sv: &Solved,
    fix: &[usize],
    trainees: &[usize],
    pj: &PlanJob,
    pick: Option<(&Pool, &Solved)>,
) -> Value {
    let t_all = now_ms();
    let input = PlanInputs { d, sk, cr, job, legs, pool, sv, fix, trainees, pj, pick };
    let gp = input.pick_solved().found[0].clone();
    progress(1, 0, 0);
    // First in, dropped last, after every memo belonging to this plan.
    let _memo_report = MemoReport;
    // (the phases' scratch for deck values)
    let mut buf = vec![];
    let PlanSetup { sim: mut s, candidates, mut searches, mut profile, cache0 } =
        match setup_plan_candidates(env, &input, t_all, gp, &mut buf) {
            Ok(prepared) => prepared,
            Err(error) => return error,
        };
    let BaseRows { mut runs, mut wins_by_trainee } =
        score_and_admit_proxies(&mut s, &input, &candidates, &mut searches, &mut profile, &mut buf);
    let looped = add_looped_rows(&mut s, &input, &candidates, &runs, &mut searches, &mut profile, &mut buf);
    runs.extend(looped);
    let no_borrow = add_no_borrow_rows(&mut s, &input, &candidates, &runs, &mut searches, &mut profile, &mut buf);
    runs.extend(no_borrow);
    let scores = score_and_promote_rows(
        &mut s,
        &input,
        &candidates,
        &mut searches,
        &mut runs,
        &mut wins_by_trainee,
        &mut profile,
        &mut buf,
    );
    let proxy_check = check_plan_proxies(&mut s, &input, &candidates, &searches.best, &runs, &scores, cache0, &mut buf);
    assemble_plan_output(s, &input, &candidates, searches, runs, &scores, &profile, proxy_check, &mut buf)
}

/// What setup hands the phases: the simulator, the fixed candidates, the search history, the profile, and (with
/// `proxy_check`) the cache as it was before any builder's search, for the proxy check only
struct PlanSetup<'env, 'data> {
    sim: Sim<'env, 'data>,
    candidates: PlanCandidates,
    searches: PlanSearches,
    profile: PlanProfile,
    cache0: Option<super::Cache>,
}

fn setup_plan_candidates<'env, 'data>(
    env: &'env mut Env<'data>,
    input: &PlanInputs<'_, 'data>,
    t_all: f64,
    gp: Found,
    buf: &mut Vec<f64>,
) -> Result<PlanSetup<'env, 'data>, Value> {
    let PlanInputs { d, sk, cr, job, legs, pool, sv, fix, trainees, pj, .. } = *input;
    // the work counters at the start (deck-search nodes, exact scorings): reported as this plan's own
    let (nodes0, joints0) = (env.space.nodes.get(), crate::deck::JCOUNT.with(std::cell::Cell::get));
    // the loop planner's members only when the job names them (`loop_members`, an override; the plan no longer runs the
    // loop planner's search itself: on the val/plan pools its members always entered the candidates by the routes below)
    let (members, loop_member_info, loop_member_note) = match &pj.loop_members {
        Some(ids) => rotation(env, d, ids),
        None => (vec![], vec![], None),
    };
    let g = sv.found[0].clone();
    // stage-1 candidates (who A can be; whose odds a builder may lift): the best career's trainee, the named loop members,
    // the umas of the best career's lineage, the umas you have veterans of, the search's other trainees; at most IDX_CANDS
    let mut cands: Vec<usize> = vec![g.t];
    let push = |u: usize, c: &mut Vec<usize>| {
        if !c.contains(&u) && trainees.contains(&u) {
            c.push(u);
        }
    };
    for &m in &members {
        push(m, &mut cands);
    }
    for x in [g.a, g.b] {
        for c in std::iter::once(legs[x].chara).chain(legs[x].parents.iter().map(|p| p.chara)) {
            if let Some(u) = env.uix(c) {
                push(u, &mut cands);
            }
        }
    }
    for l in legs.iter().rev() {
        if !l.borrowed
            && let Some(u) = env.uix(l.chara)
        {
            push(u, &mut cands);
        }
    }
    for f in &sv.found {
        push(f.t, &mut cands);
    }
    cands.truncate(IDX_CANDS);
    if let Some(f) = &pj.fit {
        cands = f.iter().filter_map(|&(c, ..)| env.uix(c)).filter(|u| trainees.contains(u)).collect();
    }
    // a pool that gives fewer than HINT_FLOOR candidates (a loop being started: veterans of umas that won't be in it, mob
    // parents) is filled from the planner's own model: the owned umas ranked by their odds with a full loop lineage
    // (`hint_umas`), no deck search
    let mut hint: Vec<Value> = vec![];
    if pj.loop_members.is_none() && pj.fit.is_none() && cands.len() < HINT_FLOOR {
        for (u, v) in hint_umas(env, d, pool, &cands, trainees, HINT_FLOOR - cands.len()) {
            cands.push(u);
            hint.push(json!({"id": d.umas[u].id, "name": d.umas[u].name, "ceiling": v}));
        }
    }
    let loop_ms = now_ms() - t_all;
    let all: Vec<usize> = trainees.iter().copied().filter(|&u| env.dmax[u].is_some()).collect();
    let t1 = now_ms();
    let original_budget = env.budget;
    // (the candidates' career searches below under the cap, as `capped` sets it: the Sim it takes is made after them)
    if pj.search_work > 0 {
        env.budget = pj.search_work;
        env.hard = pj.search_work;
    }
    env.use_seen = true;
    // best careers, searched on demand (stage-1 candidates now, builders when scored exactly)
    let mut best = vec![CachedSearch::Unsearched; d.umas.len()];
    best[g.t] = CachedSearch::Known(Some(g.clone()));
    // a kept plan of this pool less its last veteran (`Bests`), and this plan's own, kept at the end
    let nl = legs.len();
    // (taken out while this plan runs and put back after it: every reroll of a career reuses the same one)
    let kept_same_pool = take_kept_plan(bests_key(job, fix, nl));
    let kept_previous_pool = if nl > 1 { take_kept_plan(bests_key(job, fix, nl - 1)) } else { None };
    let mut proof_refs: Vec<Vec<Vec<Ref>>> = vec![vec![]; d.umas.len()];
    // per uma: her best career's search was not complete (`best_of`), and her looped one's
    let mut base_incomplete: Vec<bool> = vec![false; d.umas.len()];
    let looped_incomplete: Vec<bool> = vec![false; d.umas.len()];
    // (and her nb row's, under "later")
    let no_borrow_incomplete: Vec<bool> = vec![false; d.umas.len()];
    if pj.sur != 1 {
        for &u in &cands {
            if best[u].is_unsearched() {
                let (found, refs, complete) =
                    best_of(env, pool, fix, u, kept_same_pool.as_ref(), kept_previous_pool.as_ref(), nl);
                proof_refs[u] = refs;
                best[u] = CachedSearch::Known(found);
                base_incomplete[u] = !complete;
            }
        }
    }
    env.hard = 0;
    env.budget = original_budget;
    let mut search_ms = now_ms() - t1;
    let mut s = sim_for(env, d, sk, cr, job, pj)?;
    // the cache the planner's table values read (trios' third members, candidates' lifted odds): each search's exact
    // winners only, the main search's best careers and each uma's best one, so what it holds (and every number read from
    // it) depends on the job alone, not on what the pruning happened to check fully (earlier jobs' proofs, the order, a
    // kept plan). Before, every fully checked deck went in: a candidate then had several decks, and her lifted odds
    // read the best of them; with one the planner's numbers moved once (val/plan: picks unchanged)
    for f in &sv.found {
        s.cache.add(s.env, f.t, &f.sel);
    }
    for &u in &cands {
        if let Some(f) = best[u].found() {
            s.cache.add(s.env, u, &f.sel);
        }
    }
    // (`proxy_check`: the cache as it is before any builder's search, for the proxies at the end)
    let cache0 = pj.proxy_check.then(|| s.cache.clone());
    // the surrogate's best careers (sur 1: in place of the searches; 2: beside them, differences reported)
    let mut sur_diff: Vec<Value> = vec![];
    if pj.sur > 0 {
        let ts = now_ms();
        capped(&mut s, pj.search_work, original_budget, |s| {
            for &u in &cands {
                if pj.sur == 2 || best[u].is_unsearched() {
                    plan_surrogate(s, input, &mut best, u, buf, &mut sur_diff);
                }
            }
        });
        search_ms += now_ms() - ts;
    }
    let fb: Vec<Option<Found>> = cands.iter().map(|&u| best[u].found().cloned()).collect();
    let n = cands.len();
    let pnow: Vec<f64> = fb.iter().map(|f| f.as_ref().map_or(0.0, |f| f.val)).collect();
    let sj = s.env.sj;
    // per candidate whether an 8/8 of hers exists (a free side of the trios)
    let have8: Vec<bool> = cands
        .iter()
        .map(|&u| legs.iter().any(|l| l.chara == d.umas[u].id && is_full(sj, &l.sparks) && pool.bor_one(l.borrowed)))
        .collect();
    let zeros = vec![0.0f64; n];
    let t3 = now_ms();
    // Builders through their deck class: a builder's career is first taken from her class's winning decks (the main
    // search's and the candidates' best careers of her class: legal for her, the same deck problem, valued at her own
    // chances through the surrogate, `best_sur`), no career search. Exact searches only where the plan can turn on her:
    // a lift efficiency within PROXY_EFF of a candidate's second best (Cont's two builders come from the exactly
    // searched careers), or a saving within PROXY_SAVE of the shown rows (the page's top OPT_SHOW) or the pick's near-tie
    // band. Measured on val/plan (`proxy_check`): the proxy's odds only ever read low (it is a
    // career she can run); its efficiency on contenders for a candidate's top two at most 2.8% low (p11; it can also
    // read high, which only searches more), its saving at most 2.7% low; exact on the other five jobs.
    // `exact_all` restores a search for every uma.
    let proxy_on = pj.sur == 0 && !pj.exact_all && !pj.proxy_check;
    let ones = vec![1.0f64; n];
    let candidates = PlanCandidates {
        main_career: g,
        pick_career: gp,
        members,
        loop_member_info,
        loop_member_note,
        hint,
        trainees: cands,
        allowed_trainees: all,
        careers: fb,
        pnow,
        have8,
        zeros,
        ones,
        proxy_on,
    };
    let searches = PlanSearches {
        best,
        proof_refs,
        base_incomplete,
        looped_incomplete,
        no_borrow_incomplete,
        looped_careers: vec![],
        kept_same_pool,
        kept_previous_pool,
        original_budget,
    };
    let profile = PlanProfile { t_all, t3, loop_ms, search_ms, nodes0, joints0, n_cert: 0, exact_n: 0, sur_diff };
    Ok(PlanSetup { sim: s, candidates, searches, profile, cache0 })
}

fn score_and_admit_proxies(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> BaseRows {
    let PlanInputs { d, legs, pool, sv, fix, pj, .. } = *input;
    let n = candidates.trainees.len();
    let nl = legs.len();
    let mut runs: Vec<PlanRow> = vec![];
    let mut wins_by_trainee: Vec<Vec<usize>> = vec![vec![]; d.umas.len()];
    let wins: Vec<(usize, Sel)> = sv
        .found
        .iter()
        .map(|f| (f.t, f.sel.clone()))
        .chain(candidates.trainees.iter().filter_map(|&u| searches.best[u].found().cloned().map(|f| (u, f.sel))))
        .collect();
    // pass 1: every allowed uma's best career; per candidate, the chance her veteran raises that candidate's odds and the
    // mean odds raised to (exact over her outcomes)
    // (placeholder step costs: 1, so every candidate's lifts are gathered; an 8/8 of hers may be a poorer parent than a
    // fresh one)
    let base_s = candidates.stats_base();
    {
        let mut order: Vec<usize> = vec![candidates.main_career.t];
        for &x in &candidates.allowed_trainees {
            if x != candidates.main_career.t {
                order.push(x);
            }
        }
        for &x in &order {
            progress(2, order.iter().position(|&y| y == x).unwrap_or(0), order.len()); // pass 1: uma k of n
            if candidates.proxy_on && searches.best[x].is_unsearched() {
                let ts = now_ms();
                let cx = s.env.cls[x];
                for (t, sel) in &wins {
                    if *t != x && s.env.cls[*t] == cx {
                        s.cache.add(s.env, x, sel);
                    }
                }
                let pf = best_sur(s, pool, fix, x, 1e9, buf);
                profile.search_ms += now_ms() - ts;
                let Some(f) = pf else { continue };
                let (CareerScore { lift_stats: st, .. }, sv) = stamped(s, Some(&candidates.trainees), |s| {
                    score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, Want::Stats, buf)
                });
                runs.push(PlanRow::new(x, f, st, RowOrigin::Base, false, sv));
                continue;
            }
            if searches.best[x].is_unsearched() || (pj.sur == 2 && !candidates.trainees.contains(&x)) {
                let ts = now_ms();
                capped(s, pj.search_work, searches.original_budget, |s| {
                    if pj.sur != 1 && searches.best[x].is_unsearched() {
                        let (f, r, ok) = best_of(
                            s.env,
                            pool,
                            fix,
                            x,
                            searches.kept_same_pool.as_ref(),
                            searches.kept_previous_pool.as_ref(),
                            nl,
                        );
                        if let Some(f) = &f {
                            s.cache.add(s.env, x, &f.sel);
                        }
                        searches.proof_refs[x] = r;
                        searches.best[x] = CachedSearch::Known(f);
                        searches.base_incomplete[x] = !ok;
                    }
                    if pj.sur > 0 {
                        plan_surrogate(s, input, &mut searches.best, x, buf, &mut profile.sur_diff);
                    }
                });
                profile.search_ms += now_ms() - ts;
            }
            let Some(f) = searches.best[x].found().cloned() else { continue };
            let CareerScore { lift_stats: st, .. } =
                score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, Want::Stats, buf);
            runs.push(PlanRow::new(x, f, st, RowOrigin::Base, true, None));
        }
        // the near-top builders searched exactly: per candidate every proxy whose efficiency reaches (1 - PROXY_EFF) of the
        // candidate's second best (exact and proxy careers together), until none is left
        #[allow(clippy::while_immutable_condition, reason = "proxy_on is fixed per call; the loop ends at its break")]
        while candidates.proxy_on {
            let mut add: Vec<usize> = vec![];
            for ui in 0..n {
                if candidates.pnow[ui] <= 0.0 {
                    continue;
                }
                let (u, p) = (candidates.trainees[ui], candidates.pnow[ui]);
                let mut e: Vec<(f64, usize)> = runs
                    .iter()
                    .enumerate()
                    .filter(|(_, r)| d.umas[r.trainee].id != d.umas[u].id && r.lift_stats[ui].0 > 0.0)
                    .map(|(i, r)| {
                        let (pl, sp) = r.lift_stats[ui];
                        (lift_eff(pl, sp, p), i)
                    })
                    .collect();
                e.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
                let Some(&(e1, _)) = e.first() else { continue };
                let second = e.get(1).map_or(e1, |x| x.0);
                for &(v, i) in &e {
                    if v < second * (1.0 - pj.proxy_eff.unwrap_or(PROXY_EFF)) {
                        break;
                    }
                    if !runs[i].exact && !add.contains(&i) {
                        add.push(i);
                    }
                }
            }
            if add.is_empty() {
                break;
            }
            add.sort_unstable();
            let ts = now_ms();
            for i in add {
                let x = runs[i].trainee;
                // a proxy the class's proofs certify needs no search (it is her best career's value)
                if certified(s.env, pool, fix, x, &runs[i].career) {
                    searches.best[x] = CachedSearch::Known(Some(runs[i].career.clone()));
                    runs[i].exact = true;
                    profile.n_cert += 1;
                    continue;
                }
                let r = {
                    let ri = &runs[i];
                    exact_plan_career(
                        s,
                        input,
                        candidates,
                        searches,
                        x,
                        buf,
                        Some((&ri.career, ri.lift_stats.as_slice(), runs[i].stats_stamp)),
                    )
                };
                if let Some((f, st)) = r {
                    runs[i].career = f;
                    runs[i].lift_stats = st;
                }
                runs[i].exact = true;
                runs[i].stats_stamp = None;
            }
            profile.search_ms += now_ms() - ts;
        }
        for r in &runs {
            set_plan_wins(s, input, &mut wins_by_trainee, r.trainee, &r.career);
        }
    }
    BaseRows { runs, wins_by_trainee }
}

/// The looped rows (below) of `runs`' base rows, in their order.
fn add_looped_rows(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    runs: &[PlanRow],
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let mut added: Vec<PlanRow> = vec![];
    let base_s = candidates.stats_base();
    let PlanInputs { d, legs, pool, fix, pj, .. } = *input;
    let pick_pool = input.pick_pool();
    let nl = legs.len();
    let sj = s.env.sj;
    let is_runnable = |f: &Found| input.is_runnable(f);
    // Looped careers: per uma whose career above has parents that are not both 8/8s, her best career on two real 8/8
    // veterans (a legal pair, all of `fix` among them), a row of its own beside it. Only there is her own 8/8 a fully
    // looped parent (the goal), and the trios' last step is such a career: without these rows the career V itself
    // prices (C on A's and B's 8/8s) was never among the options, and a uma whose best-odds pair happened to be two 8/8s
    // took the goal's credit alone (inputs of 2026-10-17: Taiki Shuttle 0.454 over Inari One's looped career, 1.000).
    // Exact (the surrogate's under `sur` 1); pairs by ceiling, stopping below the best found. Not among Cont's builders
    // (Cont is fitted on each uma's best career) and not added to the cache (what the tables read stays the job's
    // winners), so V and every other row are as before.
    let looped = |f: &Found| is_full(sj, &legs[f.a].sparks) && is_full(sj, &legs[f.b].sparks);
    // Per uma her best looped career where searched (`Unsearched` or `Known`, including no legal pair), kept with
    // the plan's best careers: a kept plan of this very pool gives it as kept; one of this pool less its last veteran
    // gives it as kept unless she is an 8/8 (then only her pairs are new: the better of the kept one and those, a tie
    // keeping the kept one)
    searches.looped_careers = vec![CachedSearch::Unsearched; d.umas.len()];
    progress(3, 0, 0); // the looped careers
    if fix.len() < 2 {
        let full8: Vec<usize> = (0..nl).filter(|&i| is_full(sj, &legs[i].sparks)).collect();
        let is8: Vec<bool> = (0..nl).map(|i| full8.contains(&i)).collect();
        let her8 = nl > 0 && full8.contains(&(nl - 1));
        let ts = now_ms();
        let lnodes0 = s.env.space.nodes.get();
        capped(s, pj.search_work, searches.original_budget, |s| {
            for row in runs {
                if row.origin != RowOrigin::Base {
                    continue;
                }
                let x = row.trainee;
                if looped(&row.career) && is_runnable(&row.career) {
                    continue;
                }
                let kept = |b: &Option<Bests>| {
                    b.as_ref().and_then(|b| match b.looped.get(x) {
                        Some(CachedSearch::Known(found)) => Some(found.clone()),
                        _ => None,
                    })
                };
                let kept_incomplete =
                    |b: &Option<Bests>| b.as_ref().is_some_and(|b| b.lunp.get(x).copied().unwrap_or(false));
                let lf: Option<Found> = if let Some(v) = kept(&searches.kept_same_pool) {
                    searches.looped_incomplete[x] = kept_incomplete(&searches.kept_same_pool);
                    v
                } else {
                    let start = kept(&searches.kept_previous_pool);
                    let only_her = start.is_some();
                    if only_her {
                        searches.looped_incomplete[x] = kept_incomplete(&searches.kept_previous_pool);
                    }
                    if only_her && !her8 {
                        start.flatten()
                    } else {
                        // (the legal pairs of two 8/8s, only those with her when the kept plan had the rest; `pick_pool` and
                        // `pool` differ in `max_bor` alone, which the ceilings do not read)
                        let cs = s
                            .env
                            .ranked_pairs(pick_pool, fix, x, |a, b| is8[a] && is8[b] && (!only_her || b == nl - 1));
                        let mut lf: Option<Found> = start.flatten();
                        for (ub, a, b) in cs {
                            if ub <= lf.as_ref().map_or(0.0, |f| f.val) {
                                break;
                            }
                            // (the exact search's bar starts at the best looped career so far: only a career above it can
                            // replace it, as in `best_of`; a tie keeps the earlier pair either way)
                            let f = if pj.sur == 1 {
                                best_sur(s, pick_pool, &[a, b], x, pj.sur_margin, buf)
                            } else {
                                s.env.floor = lf.as_ref().map_or(f64::NEG_INFINITY, |f| f.val);
                                let sr = s.env.solve(pick_pool, &[a, b], &[x], false, 1);
                                if !sr.complete {
                                    searches.looped_incomplete[x] = true;
                                }
                                s.env.floor = f64::NEG_INFINITY;
                                sr.found.into_iter().next()
                            };
                            if let Some(f) = f
                                && f.val > lf.as_ref().map_or(0.0, |found| found.val)
                            {
                                lf = Some(f);
                            }
                        }
                        lf
                    }
                };
                searches.looped_careers[x] = CachedSearch::Known(lf.clone());
                if let Some(f) = lf {
                    // (its stats feed only `proxy_check`; without it the scoring runs for its state effects alone: its `val`
                    // searches, under this block's cap, add decks later scorings read)
                    let want = if pj.proxy_check { Want::Stats } else { Want::Touch };
                    let CareerScore { lift_stats: st, .. } =
                        score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, want, buf);
                    added.push(PlanRow::new(x, f, st, RowOrigin::Looped, true, None));
                }
            }
        });
        profile.search_ms += now_ms() - ts;
        prof("looped_ms", now_ms() - ts);
        prof("looped_nodes", (s.env.space.nodes.get() - lnodes0) as f64);
    }
    added
}

/// The no-borrow rows (below) of `runs`' base rows, in their order.
fn add_no_borrow_rows(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    runs: &[PlanRow],
    searches: &mut PlanSearches,
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> Vec<PlanRow> {
    let mut added: Vec<PlanRow> = vec![];
    let PlanInputs { fix, pick, .. } = *input;
    let pick_pool = input.pick_pool();
    let n = candidates.trainees.len();
    let is_runnable = |career: &Found| input.is_runnable(career);
    // nb rows (`pick`): per base row whose career has a borrowed parent, her best career without one, a row of its own:
    // through her cached decks (`best_sur`, as a builder's proxy) while proxies are on, searched exactly once a shown
    // row or the pick's band can turn on it (pass 2); exact from the start when proxies are off. Like the looped rows
    // neither Cont's builders nor added to the cache; no lifts (zeros: only Cont and `proxy_check` read a row's lifts; the
    // looped rows' are zeros too unless `proxy_check`)
    if pick.is_some() {
        let ts = now_ms();
        for row in runs {
            if row.origin != RowOrigin::Base {
                continue;
            }
            if is_runnable(&row.career) {
                continue;
            }
            let trainee = row.trainee;
            let (career, is_exact) = if candidates.proxy_on {
                (best_sur(s, pick_pool, fix, trainee, 1e9, buf), false)
            } else {
                (exact_no_borrow_career(s, input, searches, trainee, buf), true)
            };
            if let Some(career) = career {
                added.push(PlanRow::new(trainee, career, vec![(0.0, 0.0); n], RowOrigin::NoBorrow, is_exact, None));
            }
        }
        profile.search_ms += now_ms() - ts;
    }
    added
}

fn score_and_promote_rows(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    searches: &mut PlanSearches,
    runs: &mut Vec<PlanRow>,
    wins_by_trainee: &mut [Vec<usize>],
    profile: &mut PlanProfile,
    buf: &mut Vec<f64>,
) -> PlanScores {
    let PlanInputs { d, legs, pool, fix, .. } = *input;
    let pick_pool = input.pick_pool();
    let n = candidates.trainees.len();
    let sj = s.env.sj;
    let is_runnable = |f: &Found| input.is_runnable(f);
    // each candidate's step with building: her most efficient builder now (rate, log-odds step), her full-loop ceiling with
    // her two most efficient builders (among the exactly searched careers)
    let continuations = plan_continuations(s, input, candidates, runs, wins_by_trainee, buf);
    // a fresh 8/8 of hers (her step); a real 8/8 of hers costs nothing (the trios' side choice)
    let step_costs: Vec<f64> = (0..n).map(|ui| continuations[ui].cost(candidates.pnow[ui])).collect();
    // the trios: A and B two candidates (unordered), C the best allowed uma with an 8/8 of A and one of B as parents. Each
    // side: a real 8/8 veteran of hers (free), or a fresh one, her step, the 8/8 her best career now would leave
    // (`Env::veteran`: its parents, every target at the rank's star mix, her G1 wins at their chances), whichever trio is
    // cheaper; C's 8/8 is then a fully looped parent
    let t_c = now_ms();
    let real8: Vec<Vec<usize>> = candidates
        .trainees
        .iter()
        .map(|&u| {
            (0..legs.len())
                .filter(|&i| {
                    legs[i].chara == d.umas[u].id && is_full(sj, &legs[i].sparks) && pool.bor_one(legs[i].borrowed)
                })
                .collect()
        })
        .collect();
    let mut combos = plan_combinations(s, input, candidates, &real8, &step_costs, buf);
    let sides = plan_sides(s, input, candidates, &real8, &step_costs);
    let mut combo_ms = now_ms() - t_c;
    let trios: Vec<Trio> = combos.iter().map(PlanCombination::as_trio).collect();
    let current_value = trio_v(&step_costs, &trios);
    let mut fit = PlanFit { continuations, step_costs, trios, sides, current_value };
    // pass 2: every career scored against V (exact over her outcomes): E[V - V with her veteran] - 1
    // (a row that is never the pick, a builder with a borrowed parent under "later", is not scored: no score, no kept scoring)
    progress(4, 0, runs.len()); // pass 2: every career against V
    if !runs.is_empty() && fit.current_value.is_finite() {
        for i in 0..runs.len() {
            let ((net_score, lift, kept), stamp) =
                stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
            runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
            prune_plan_scorings(input, runs);
        }
        // the proxies a shown row or the pick's near-tie band can turn on: searched exactly (their
        // saving, score + 1, within PROXY_SAVE of the lowest saving those take), then scored again; when one of them joins a
        // candidate's two most efficient builders, Cont, the trios, V and every score are computed again
        #[allow(clippy::while_immutable_condition, reason = "proxy_on is fixed per call; the loop ends at its break")]
        while candidates.proxy_on {
            // (over the rows that can be shown or picked, `is_runnable`)
            let mut sv: Vec<f64> =
                runs.iter().filter(|r| is_runnable(&r.career)).filter_map(PlanRow::net_saving).collect();
            sv.sort_by(|x, y| y.partial_cmp(x).unwrap());
            if sv.is_empty() {
                break;
            }
            let (top, k8) = (sv[0], sv[(OPT_SHOW - 1).min(sv.len() - 1)]);
            let t = k8.min(near_floor(top)) + 1.0;
            let add: Vec<usize> = (0..runs.len())
                .filter(|&i| {
                    !runs[i].exact && is_runnable(&runs[i].career) && runs[i].total() + 1.0 >= t * (1.0 - PROXY_SAVE)
                })
                .collect();
            if add.is_empty() {
                break;
            }
            let ts = now_ms();
            // (the careers scored: a row whose search found the same one, or that was certified, may keep its score)
            let old: Vec<Found> = add.iter().map(|&i| runs[i].career.clone()).collect();
            for (k, &i) in add.iter().enumerate() {
                let x = runs[i].trainee;
                // an nb row: her best career without a borrowed parent, certified or searched on the pick's pool
                if runs[i].origin == RowOrigin::NoBorrow {
                    if certified(s.env, pick_pool, fix, x, &runs[i].career) {
                        profile.n_cert += 1;
                    } else if let Some(f) = exact_no_borrow_career(s, input, searches, x, buf) {
                        runs[i].career = f;
                    }
                    runs[i].exact = true;
                    continue;
                }
                if certified(s.env, pool, fix, x, &runs[i].career) {
                    searches.best[x] = CachedSearch::Known(Some(runs[i].career.clone()));
                    runs[i].exact = true;
                    profile.n_cert += 1;
                    continue;
                }
                let r = {
                    let ri = &runs[i];
                    exact_plan_career(
                        s,
                        input,
                        candidates,
                        searches,
                        x,
                        buf,
                        Some((&ri.career, ri.lift_stats.as_slice(), runs[i].stats_stamp)),
                    )
                };
                if let Some((f, st)) = r {
                    set_plan_wins(s, input, wins_by_trainee, x, &f);
                    runs[i].career = f;
                    runs[i].lift_stats = st;
                }
                runs[i].exact = true;
                runs[i].stats_stamp = None;
                // ("later") her best career has a borrowed parent: that row is a builder now, and the proxy's career,
                // without one, her nb row (scored below)
                if !is_runnable(&runs[i].career) {
                    runs.push(PlanRow::new(x, old[k].clone(), vec![(0.0, 0.0); n], RowOrigin::NoBorrow, false, None));
                }
            }
            profile.search_ms += now_ms() - ts;
            let next_continuations = plan_continuations(s, input, candidates, runs, wins_by_trainee, buf);
            let same_fit = next_continuations
                .iter()
                .zip(&fit.continuations)
                .all(|(a, b)| a.lam == b.lam && a.phi == b.phi && a.pinf == b.pinf);
            if same_fit {
                for (k, &i) in add.iter().enumerate() {
                    // the same career against the same base (Cont, so step_costs, V, the trios and sides, as when it was scored),
                    // nothing it reads changed: its score, lift and kept scoring stand (a dropped one stays dropped: its score
                    // is as low as it was, and lo only rises here)
                    if same_career(&old[k], &runs[i].career)
                        && stamp_ok(s, None, runs[i].scoring.as_ref().unwrap().stamp)
                    {
                        prof("reuse_sc", 1.0);
                        continue;
                    }
                    let ((net_score, lift, kept), stamp) =
                        stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
                    runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
                }
                // (the nb rows added above)
                for i in 0..runs.len() {
                    if runs[i].scoring.is_some() {
                        continue;
                    }
                    let ((net_score, lift, kept), stamp) =
                        stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
                    runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
                }
                prune_plan_scorings(input, runs);
            } else {
                memo_stat(6);
                fit.continuations = next_continuations;
                fit.step_costs = (0..n).map(|ui| fit.continuations[ui].cost(candidates.pnow[ui])).collect();
                let tc = now_ms();
                combos = plan_combinations(s, input, candidates, &real8, &fit.step_costs, buf);
                fit.sides = plan_sides(s, input, candidates, &real8, &fit.step_costs);
                combo_ms += now_ms() - tc;
                fit.trios = combos.iter().map(PlanCombination::as_trio).collect();
                fit.current_value = trio_v(&fit.step_costs, &fit.trios);
                if !fit.current_value.is_finite() {
                    for run in &mut *runs {
                        run.scoring = None;
                    }
                    break;
                }
                // (every row scored again: the old kept scorings go first, the new ones kept as above)
                for run in &mut *runs {
                    run.scoring = None;
                }
                for i in 0..runs.len() {
                    let ((net_score, lift, kept), stamp) =
                        stamped(s, None, |s| score_plan_row(s, input, candidates, &fit, &runs[i], buf));
                    runs[i].scoring = Some(RowScoring { net_saving: net_score, improvement_chance: lift, kept, stamp });
                    prune_plan_scorings(input, runs);
                }
            }
        }
        profile.exact_n = runs.iter().filter(|r| r.scoring.is_some() && is_runnable(&r.career)).count();
    }
    let best_combo = combos
        .iter()
        .min_by(|x, y| x.total_cost(&fit.step_costs).partial_cmp(&y.total_cost(&fit.step_costs)).unwrap())
        .copied();
    let index_ms = now_ms() - profile.t3;
    let work = json!({"nodes": s.env.space.nodes.get() - profile.nodes0, "exact": crate::deck::JCOUNT.with(std::cell::Cell::get) - profile.joints0, "max_ratio": s.env.max_ratio.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": s.env.max_ratio, "class_max_deck": s.env.max_decks(), "cap": s.env.space.rho_top().min(s.env.cert), "class_caps": s.env.space.caps()});
    let any_scored = runs.iter().any(|r| r.scoring.is_some());
    PlanScores { fit, best_combo, any_scored, combo_ms, index_ms, work }
}

fn check_plan_proxies(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    best: &[CachedSearch],
    runs: &[PlanRow],
    scores: &PlanScores,
    cache0: Option<super::Cache>,
    buf: &mut Vec<f64>,
) -> Value {
    let PlanInputs { d, legs, pool, sv, fix, pj, .. } = *input;
    let n = candidates.trainees.len();
    // measurement (`proxy_check`): each builder's career from her deck class's winning decks (the main search's and the
    // candidates' best careers of her class, legal for her and valued at her own chances) through the surrogate, no career
    // search; its lifts per candidate (efficiency, as Cont takes them) and its score beside the exact ones. Runs after
    // everything else, on the cache as it was before the builders' searches, so the plan's own numbers are unchanged
    match (pj.proxy_check, cache0) {
        (true, Some(mut c0)) if !runs.is_empty() => {
            std::mem::swap(&mut s.cache, &mut c0);
            let wins: Vec<(usize, Sel)> = sv
                .found
                .iter()
                .map(|f| (f.t, f.sel.clone()))
                .chain(candidates.trainees.iter().filter_map(|&u| best[u].found().cloned().map(|f| (u, f.sel))))
                .collect();
            let base_s = candidates.stats_base();
            let base = scores.fit.score_base(candidates);
            let effs = |x: usize, st: &[(f64, f64)]| -> Vec<f64> {
                (0..n)
                    .map(|ui| {
                        let (pl, sp) = st[ui];
                        if d.umas[candidates.trainees[ui]].id == d.umas[x].id || pl <= 0.0 || candidates.pnow[ui] <= 0.0
                        {
                            0.0
                        } else {
                            lift_eff(pl, sp, candidates.pnow[ui])
                        }
                    })
                    .collect()
            };
            let mut rows = vec![];
            for r in runs {
                let x = r.trainee;
                let is_c = candidates.trainees.contains(&x);
                let ee = effs(x, &r.lift_stats);
                let (pf, pe, psc) = if is_c {
                    (Some(r.career.clone()), ee.clone(), r.net_saving())
                } else {
                    let cx = s.env.cls[x];
                    for (t, sel) in &wins {
                        if *t != x && s.env.cls[*t] == cx {
                            s.cache.add(s.env, x, sel);
                        }
                    }
                    match best_sur(s, pool, fix, x, 1e9, buf) {
                        None => (None, vec![0.0; n], None),
                        Some(f) => {
                            let CareerScore { lift_stats: st, .. } =
                                score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, Want::Stats, buf);
                            let pe = effs(x, &st);
                            let psc = scores.any_scored.then(|| {
                                score_career(s, d, &base, legs, pool, x, f.a, f.b, &f.sel, Want::Score, buf)
                                    .expected_saving
                                    - 1.0
                            });
                            (Some(f), pe, psc)
                        }
                    }
                };
                let same_pair = pf.as_ref().is_some_and(|f| {
                    (f.a.min(f.b), f.a.max(f.b)) == (r.career.a.min(r.career.b), r.career.a.max(r.career.b))
                });
                let same_deck = pf.as_ref().is_some_and(|f| {
                    f.sel.ents == r.career.sel.ents
                        && f.sel.plain == r.career.sel.plain
                        && f.sel.filler == r.career.sel.filler
                });
                rows.push(json!({"trainee": d.umas[x].name, "cand": is_c, "class": s.env.cls[x], "exact_val": r.career.val, "proxy_val": pf.as_ref().map(|f| f.val),
                "same_pair": same_pair, "same_deck": same_deck, "score": r.net_saving(), "proxy_score": psc, "eff": ee, "proxy_eff": pe}));
            }
            std::mem::swap(&mut s.cache, &mut c0);
            json!({"cands": candidates.trainees.iter().map(|&u| d.umas[u].name.clone()).collect::<Vec<_>>(), "rows": rows})
        }
        _ => Value::Null,
    }
}

fn assemble_plan_output<'data>(
    mut s: Sim<'_, 'data>,
    input: &PlanInputs<'_, 'data>,
    candidates: &PlanCandidates,
    searches: PlanSearches,
    mut runs: Vec<PlanRow>,
    scores: &PlanScores,
    profile: &PlanProfile,
    proxy_check: Value,
    buf: &mut Vec<f64>,
) -> Value {
    let sj = s.env.sj;
    let PlanInputs { d, sk, cr, job, legs, pool, sv, fix, pj, .. } = *input;
    let psv = input.pick_solved();
    let n = candidates.trainees.len();
    let nl = legs.len();
    let is_runnable = |f: &Found| input.is_runnable(f);
    let looped = |f: &Found| is_full(sj, &legs[f.a].sparks) && is_full(sj, &legs[f.b].sparks);
    let step = |ui: usize, real: bool| if real { 0.0 } else { scores.fit.step_costs[ui] };
    // a looped or nb row that repeats an earlier row's career (a builder's exact search can land on the same 8/8 pair; an
    // nb row on her looped one): left out, the pick moved to the earlier one (the same career, the same score)
    let pair = |f: &Found| (f.a.min(f.b), f.a.max(f.b));
    let dup: Vec<bool> = (0..runs.len())
        .map(|i| (0..i).any(|j| runs[j].trainee == runs[i].trainee && pair(&runs[j].career) == pair(&runs[i].career)))
        .collect();
    let undup: Vec<usize> = (0..runs.len())
        .map(|i| {
            if dup[i] {
                (0..i)
                    .find(|&j| runs[j].trainee == runs[i].trainee && pair(&runs[j].career) == pair(&runs[i].career))
                    .unwrap()
            } else {
                i
            }
        })
        .collect();
    // the pick's roll state (`RollKeep`; the pick as below, with the same scores): its kept scoring and base, and what its
    // bound reads beyond them, as the plan ends (`pick_her`, `found_anchors`)
    let rk: Option<RollKeep> =
        (runs.iter().any(|r| is_runnable(&r.career)) && scores.any_scored && scores.fit.current_value.is_finite())
            .then(|| {
                let (pk, _) = pick_of(&runs, is_runnable, &undup);
                let base = scores.fit.score_base(candidates);
                let (x, f) = (runs[pk].trainee, runs[pk].career.clone());
                let KeptScoring { scored: sd, full_landing_below_threshold: full_lo } =
                    runs[pk].scoring.as_mut().unwrap().kept.take().expect("the pick's kept scoring was dropped");
                let her_hi = pick_her(&mut s, d, &base, legs, pool, x, f.a, f.b, &f.sel, full_lo, buf);
                RollKeep {
                    key: roll_key(job),
                    sd,
                    base: KeptBase::of(&base),
                    x,
                    a: f.a,
                    b: f.b,
                    sel: f.sel,
                    her_hi,
                    anc: found_anchors(s.env, pool, sv),
                    hi_anc: vec![],
                }
            });
    // Release the simulator's caches before publishing kept searches and reporting options.
    let env = {
        let finished = s;
        finished.env
    };
    {
        let key = bests_key(job, fix, nl);
        let mut kept_best = searches.best.clone();
        // The main search's best is per job, searched again (cheap) next time.
        kept_best[candidates.main_career.t] = CachedSearch::Unsearched;
        BESTS.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.retain(|plan| plan.key != key);
            cache.push(Bests {
                key,
                best: kept_best,
                refs: searches.proof_refs,
                looped: searches.looped_careers,
                unp: searches.base_incomplete.clone(),
                lunp: searches.looped_incomplete.clone(),
            });
            if let Some(previous) = searches.kept_previous_pool {
                cache.push(previous);
            }
            while cache.len() > 4 {
                cache.remove(0);
            }
        });
    }
    // nothing scored (no trio: fewer than two candidates of different umas): the best odds is the one option and the pick
    // (no row that can be the pick, `is_runnable`: the same, the pick's main search's)
    if !runs.iter().any(|r| is_runnable(&r.career)) {
        for f in &psv.found {
            runs.push(PlanRow::new(f.t, f.clone(), vec![], RowOrigin::Fallback, false, None));
        }
    }
    // pick: the highest score; within NEAR of it (the model's noise), the better odds of her own 8/8
    let total = |i: usize| runs[i].total();
    let (pick, top) = pick_of(&runs, is_runnable, &undup);
    let near = |i: usize| is_runnable(&runs[i].career) && total(i) >= near_floor(top);
    // per career: the trios' last step (C on A's and B's real 8/8s) only on two 8/8 parents
    let role = |u: usize, f: &Found| match scores.best_combo {
        Some(combination)
            if u == candidates.trainees[combination.candidate_a_index]
                || u == candidates.trainees[combination.candidate_b_index] =>
        {
            "plan"
        }
        Some(combination)
            if u == combination.finisher_trainee
                && combination.a_uses_existing
                && combination.b_uses_existing
                && looped(f) =>
        {
            "finish"
        }
        _ => {
            if candidates.members.contains(&u) {
                "loop"
            } else if candidates.trainees.contains(&u) {
                "candidate"
            } else {
                "builder"
            }
        }
    };
    let mut opts: Vec<Value> = vec![];
    for (i, row) in runs.iter().enumerate() {
        let (x, f) = (&row.trainee, &row.career);
        // (a builder with a borrowed parent under "later" is no option)
        if dup.get(i).copied().unwrap_or(false) || !is_runnable(f) {
            continue;
        }
        let mut rp = env.report(pool, f, false);
        let o = rp.as_object_mut().unwrap();
        o.insert("role".into(), json!(role(*x, f)));
        o.insert("best_odds".into(), json!(*x == candidates.pick_career.t && pair(f) == pair(&candidates.pick_career)));
        o.insert("chosen".into(), json!(i == pick));
        // both parents 8/8s: her 8/8 here is a fully looped parent
        o.insert("looped".into(), json!(looped(f)));
        // false: a builder the plan could not turn on, her career from her deck class's winning decks (no career search)
        o.insert("exact".into(), json!(row.exact || row.origin == RowOrigin::Fallback));
        // her career search was not complete (its work cap `search_work`, a band overflow or a deck past its class's cap):
        // the best it found, not proven her best
        if row.exact
            && match row.origin {
                RowOrigin::Base => searches.base_incomplete[*x],
                RowOrigin::Looped => searches.looped_incomplete[*x],
                RowOrigin::NoBorrow => searches.no_borrow_incomplete[*x],
                RowOrigin::Fallback => false,
            }
        {
            o.insert("proven".into(), json!(false));
        }
        o.insert(
            "stage_cost".into(),
            json!(candidates.trainees.iter().position(|&c| c == *x).map(|ui| scores.fit.step_costs[ui])),
        );
        o.insert("index".into(), json!(row.net_saving()));
        o.insert("saving".into(), json!(row.net_saving().map(|v| v + 1.0)));
        o.insert("lift".into(), json!(row.scoring.as_ref().map(|scoring| scoring.improvement_chance)));
        o.insert("near".into(), json!(near(i) && i != pick));
        o.insert("score".into(), json!(scores.any_scored.then(|| total(i))));
        opts.push(rp);
    }
    opts.sort_by(|a, b| {
        b["score"]
            .as_f64()
            .unwrap_or(f64::NEG_INFINITY)
            .partial_cmp(&a["score"].as_f64().unwrap_or(f64::NEG_INFINITY))
            .unwrap()
    });
    let choice = if runs.is_empty() { Value::Null } else { env.report(pool, &runs[pick].career, true) };
    let uname = |u: usize| if u == usize::MAX { Value::Null } else { json!(d.umas[u].name) };
    // a member: `real` when the trio uses a real 8/8 of hers (her step free), else her fresh 8/8's step
    let member = |ui: usize, real: bool| {
        let continuation = &scores.fit.continuations[ui];
        let (_, builds, switch_chance) = continuation.plan(candidates.pnow[ui]);
        json!({
            "trainee": uname(candidates.trainees[ui]),
            "p": candidates.pnow[ui],
            "have8": candidates.have8[ui],
            "real": real,
            "p_inf": (continuation.lam > 0.0).then_some(continuation.pinf),
            "lam": continuation.lam,
            "phi": continuation.phi,
            "builds": builds,
            "p_switch": switch_chance,
            "careers": step(ui, real)
        })
    };
    let (plan_json, members_json) = match scores.best_combo {
        Some(combination) => {
            // A: the cheaper of the two steps first
            let a = (combination.candidate_a_index, combination.a_uses_existing);
            let b = (combination.candidate_b_index, combination.b_uses_existing);
            let ((first_index, first_uses_existing), (second_index, second_uses_existing)) =
                if step(a.0, a.1) <= step(b.0, b.1) { (a, b) } else { (b, a) };
            (
                json!({"a": uname(candidates.trainees[first_index]), "b": uname(candidates.trainees[second_index]), "c": uname(combination.finisher_trainee), "careers": [step(first_index, first_uses_existing), step(second_index, second_uses_existing), combination.finisher_cost]}),
                json!([member(first_index, first_uses_existing), member(second_index, second_uses_existing)]),
            )
        }
        None => (Value::Null, json!([])),
    };
    // the career a roll of this plan is of (`next.roll`, `roll_kept`: its saving per outcome, computed on demand from the
    // roll state kept here): the pick, her uncertain targets (`uncv`) and the scenario's guaranteed ones (`fixed`)
    let outcomes = match runs.get(pick) {
        Some(row) if row.scoring.is_some() => {
            json!({"trainee": d.umas[row.trainee].id, "parents": [row.career.a, row.career.b],
        "uncv": env.uncv, "fixed": env.fixed_t})
        }
        _ => Value::Null,
    };
    if let Some(r) = rk
        .filter(|r| runs.get(pick).is_some_and(|row| row.trainee == r.x && row.career.a == r.a && row.career.b == r.b))
    {
        keep_roll(r);
    }
    let mut out = json!({"choice": choice, "rule": "index", "mode": "trio", "v0": if scores.fit.current_value.is_finite() { json!(scores.fit.current_value) } else { Value::Null }, "outcomes": outcomes,
           "members": members_json,
           "v0_plan": plan_json, "fit": (0..n).map(|ui| json!([d.umas[candidates.trainees[ui]].id, scores.fit.continuations[ui].lam, scores.fit.continuations[ui].phi, scores.fit.continuations[ui].pinf])).collect::<Vec<_>>(), "loop_members": candidates.loop_member_info, "loop_note": candidates.loop_member_note, "hint": candidates.hint, "umas_scored": profile.exact_n, "umas_searched": runs.iter().filter(|r| r.exact).count() - profile.n_cert, "umas_certified": profile.n_cert, "umas_unproven": (0..d.umas.len()).filter(|&u| searches.base_incomplete[u] || searches.looped_incomplete[u] || searches.no_borrow_incomplete[u]).count(), "work": scores.work,
           "sur": pj.sur, "sur_diff": profile.sur_diff, "loop_ms": profile.loop_ms, "search_ms": profile.search_ms, "combo_ms": scores.combo_ms, "index_ms": scores.index_ms, "total_ms": now_ms() - profile.t_all});
    // (the option reports moved in, not copied by `json!`: the object's keys are ordered, so the same text)
    out["options"] = Value::Array(opts);
    if !proxy_check.is_null() {
        out["proxy_check"] = proxy_check;
    }
    out["work"]["prof"] = json!(PROF.with(|p| std::mem::take(&mut *p.borrow_mut())));
    // a roll sent with the plan job (`next.roll`) when the worker kept no roll state for it: answered from the state just
    // kept (none when nothing was scored)
    if pj.roll.is_some() {
        out["roll"] = match roll_kept(env, d, sk, cr, job, legs, pool, fix, pj) {
            Some(mut r) => {
                r["planned"] = json!(true);
                r
            }
            None => json!({"error": "the plan has no pick to roll"}),
        };
    }
    out
}

fn plan_surrogate(
    s: &mut Sim,
    input: &PlanInputs,
    best: &mut [CachedSearch],
    u: usize,
    buf: &mut Vec<f64>,
    sur_diff: &mut Vec<Value>,
) {
    let PlanInputs { d, pool, fix, pj, .. } = *input;

    let f = best_sur(s, pool, fix, u, pj.sur_margin, buf);
    if pj.sur == 2 {
        let e = best[u].found().cloned();
        let (ve, vs) = (e.as_ref().map_or(0.0, |f| f.val), f.as_ref().map_or(0.0, |f| f.val));
        let same = e.as_ref().map(|f| (f.a.min(f.b), f.a.max(f.b))) == f.as_ref().map(|f| (f.a.min(f.b), f.a.max(f.b)));
        if !same || (ve - vs).abs() > 1e-9 * ve.max(vs) {
            sur_diff.push(json!({"trainee": d.umas[u].name, "search": [e.as_ref().map(|f| [f.a, f.b]), ve], "sur": [f.as_ref().map(|f| [f.a, f.b]), vs]}));
        }
    } else {
        best[u] = CachedSearch::Known(f);
    }
}

// uma x's exact best career (searched now if not yet) and its lifts; `reuse`: her proxy career, its lifts and their
// stamp (`stamped`, over the candidates' decks: this base reads no other's), taken as they are when the search found
// the same career and nothing they read has changed
fn exact_plan_career(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    searches: &mut PlanSearches,
    x: usize,
    buf: &mut Vec<f64>,
    reuse: Option<(&Found, &[(f64, f64)], Option<(usize, usize)>)>,
) -> Option<(Found, Vec<(f64, f64)>)> {
    let PlanInputs { d, legs, pool, fix, pj, .. } = *input;
    let nl = input.legs.len();
    let base_s = candidates.stats_base();

    if searches.best[x].is_unsearched() {
        capped(s, pj.search_work, searches.original_budget, |s| {
            let (found, refs, complete) = best_of(
                s.env,
                pool,
                fix,
                x,
                searches.kept_same_pool.as_ref(),
                searches.kept_previous_pool.as_ref(),
                nl,
            );
            if let Some(found) = &found {
                s.cache.add(s.env, x, &found.sel);
            }
            searches.proof_refs[x] = refs;
            searches.best[x] = CachedSearch::Known(found);
            searches.base_incomplete[x] = !complete;
        });
    }
    let f = searches.best[x].found().cloned()?;
    if let Some((proxy, proxy_stats, stamp)) = reuse
        && same_career(proxy, &f)
        && stamp_ok(s, Some(&candidates.trainees), stamp)
    {
        prof("reuse_st", 1.0);
        return Some((f, proxy_stats.to_vec()));
    }
    let CareerScore { lift_stats: st, .. } =
        score_career(s, d, &base_s, legs, pool, x, f.a, f.b, &f.sel, Want::Stats, buf);
    Some((f, st))
}

fn set_plan_wins(s: &mut Sim, input: &PlanInputs, wins_by_trainee: &mut [Vec<usize>], x: usize, f: &Found) {
    let PlanInputs { pool, .. } = *input;

    let w = s.wins(pool, x, f.a, f.b);
    wins_by_trainee[x] = w.iter().enumerate().filter(|x| *x.1 > 0.5).map(|x| x.0).collect();
}

fn exact_no_borrow_career(
    s: &mut Sim,
    input: &PlanInputs,
    searches: &mut PlanSearches,
    x: usize,
    buf: &mut Vec<f64>,
) -> Option<Found> {
    let PlanInputs { fix, pj, .. } = *input;
    let pick_pool = input.pick_pool();
    let nl = input.legs.len();

    capped(s, pj.search_work, searches.original_budget, |s| {
        if pj.sur == 1 {
            best_sur(s, pick_pool, fix, x, pj.sur_margin, buf)
        } else {
            let (f, _, ok) = best_of(s.env, pick_pool, fix, x, None, None, nl);
            if !ok {
                searches.no_borrow_incomplete[x] = true;
            }
            f
        }
    })
}

fn plan_continuations(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    runs: &[PlanRow],
    wins_by_trainee: &[Vec<usize>],
    buf: &mut Vec<f64>,
) -> Vec<Cont> {
    let PlanInputs { d, pool, pj, .. } = *input;
    let n = candidates.trainees.len();

    let wt = |x: usize| wins_by_trainee[x].clone();
    let mut cont = vec![Cont::default(); n];
    for ui in 0..n {
        if candidates.pnow[ui] <= 0.0 || runs.is_empty() {
            continue;
        }
        let u = candidates.trainees[ui];
        let p = candidates.pnow[ui];
        let mut eff: Vec<(f64, usize, f64, f64)> = runs
            .iter()
            .filter(|r| {
                r.origin == RowOrigin::Base
                    && r.exact
                    && d.umas[r.trainee].id != d.umas[u].id
                    && r.lift_stats[ui].0 > 0.0
            })
            .map(|r| {
                let (pl, sp) = r.lift_stats[ui];
                (lift_eff(pl, sp, p), r.trainee, pl, sp / pl)
            })
            .collect();
        eff.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let Some(&(_, b1, lam, p1)) = eff.first() else { continue };
        let b2 = eff.iter().skip(1).map(|e| e.1).find(|&b| d.umas[b].id != d.umas[b1].id);
        let pinf = match b2 {
            Some(b2) => (CEIL_SHARE * loop_ceiling(s, d, pool, u, b1, b2, &wt, buf)).max(p1),
            None => p1,
        };
        let phi = if pinf > p && p1 > p { ((p1 / p).ln() / (pinf / p).ln()).clamp(0.0, 1.0) } else { 0.0 };
        cont[ui] = Cont::new(lam, phi, pinf);
    }
    // a given fit replaces the pool's (`PlanJob::fit`)
    if let Some(f) = &pj.fit {
        for ui in 0..n {
            if let Some(&(_, lam, phi, pinf)) = f.iter().find(|x| d.umas[candidates.trainees[ui]].id == x.0) {
                cont[ui] = Cont::new(lam, phi, pinf);
            }
        }
    }
    cont
}

/// Combinations in candidate-pair order, then fresh/existing side order.
fn plan_combinations(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    real8: &[Vec<usize>],
    c1: &[f64],
    buf: &mut Vec<f64>,
) -> Vec<PlanCombination> {
    let PlanInputs { d, legs, pool, .. } = *input;
    let n = candidates.trainees.len();

    let hyp8: Vec<Option<Legacy>> = (0..n)
        .map(|ui| {
            if c1[ui].is_finite() {
                candidates.careers[ui].as_ref().map(|f| {
                    let wp = s.wins(pool, candidates.trainees[ui], f.a, f.b);
                    s.env.veteran(legs, candidates.trainees[ui], f.a, f.b, wp.to_vec())
                })
            } else {
                None
            }
        })
        .collect();
    let mut combos: Vec<PlanCombination> = vec![];
    for ia in 0..n {
        for ib in ia + 1..n {
            let (ua, ub) = (candidates.trainees[ia], candidates.trainees[ib]);
            if d.umas[ua].id == d.umas[ub].id {
                continue;
            }
            // (`Env::pool` of the legacies and the fresh 8/8s: the working copy of `pool` with them pushed, A's first,
            // popped below)
            let mut p8 = s.take_pool(pool);
            // per side: (real, legacy index)
            let side = |s: &mut Sim, p8: &mut Pool, ui: usize| -> Vec<(bool, usize)> {
                let mut v: Vec<(bool, usize)> = real8[ui].iter().map(|&i| (true, i)).collect();
                if let Some(h) = &hyp8[ui] {
                    s.env.pool_push(p8, h);
                    v.push((false, p8.me.len() - 1));
                }
                v
            };
            let (xs, ys) = (side(s, &mut p8, ia), side(s, &mut p8, ib));
            // per side choice (fresh/real for A, fresh/real for B): the best C
            let mut bc = [(0.0f64, usize::MAX); 4];
            for &c in &candidates.allowed_trainees {
                if d.umas[c].id == d.umas[ua].id || d.umas[c].id == d.umas[ub].id {
                    continue;
                }
                for &(ra, x) in &xs {
                    for &(rb, y) in &ys {
                        if legal_pair(d, &p8, c, x, y) {
                            let k = usize::from(ra) * 2 + usize::from(rb);
                            let v = val(s, &p8, c, x, y, buf);
                            if v > bc[k].0 {
                                bc[k] = (v, c);
                            }
                        }
                    }
                }
            }
            for (k, &(v, cu)) in bc.iter().enumerate() {
                if cu != usize::MAX {
                    combos.push(PlanCombination {
                        candidate_a_index: ia,
                        candidate_b_index: ib,
                        finisher_cost: 1.0 / v,
                        finisher_trainee: cu,
                        a_uses_existing: k >= 2,
                        b_uses_existing: k % 2 == 1,
                    });
                }
            }
            while p8.me.len() > pool.me.len() {
                Env::pool_pop(&mut p8);
            }
            s.put_pool(pool, p8);
        }
    }
    combos
}

// the trios' sides per candidate (her real 8/8s, then the fresh 8/8 her best career now would leave), for `her_trios`
fn plan_sides(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    real8: &[Vec<usize>],
    c1: &[f64],
) -> Vec<Vec<(bool, Legacy)>> {
    let PlanInputs { legs, pool, .. } = *input;
    let n = candidates.trainees.len();

    (0..n)
        .map(|ui| {
            let mut v: Vec<(bool, Legacy)> = real8[ui].iter().map(|&i| (true, legs[i].clone())).collect();
            if c1[ui].is_finite()
                && let Some(f) = candidates.careers[ui].as_ref()
            {
                let wp = s.wins(pool, candidates.trainees[ui], f.a, f.b);
                v.push((false, s.env.veteran(legs, candidates.trainees[ui], f.a, f.b, wp.to_vec())));
            }
            v
        })
        .collect()
}

fn score_plan_row(
    s: &mut Sim,
    input: &PlanInputs,
    candidates: &PlanCandidates,
    fit: &PlanFit,
    r: &PlanRow,
    buf: &mut Vec<f64>,
) -> (f64, f64, Option<KeptScoring>) {
    let PlanInputs { d, legs, pool, .. } = *input;
    let is_runnable = |f: &Found| input.is_runnable(f);

    if !is_runnable(&r.career) {
        return (f64::NEG_INFINITY, 0.0, None);
    }
    let base = fit.score_base(candidates);
    let CareerScore { expected_saving, improvement_chance, kept, .. } =
        score_career(s, d, &base, legs, pool, r.trainee, r.career.a, r.career.b, &r.career.sel, Want::Keep, buf);
    (expected_saving - 1.0, improvement_chance, kept)
}

// Each row's kept scoring (`Scored`: what a roll of the pick reads, its tables and outcome chances) is kept only
// while the row can still be the pick. The pick (below) is a row within NEAR.max(0.05 |top|) of top, top at least the
// highest score less 1e-9 (the argmax's tolerance), or the base row a looped pick repeats (`undup`). A row's score
// changes only when it is scored again, which keeps a fresh one, and an exactly searched row is scored again only
// when every row is (a refit). So lo, the highest score of an exact row among the rows scored since every row last
// was, is at most the final top; thr(t) = t - NEAR.max(0.05 |t|) rises with t; a row below thr(lo) (less a slack far
// above its rounding) is not near the final top unless it is scored again. Kept also: every row a later row repeats
// (same uma and pair: a looped or nb row the pick moves from; a row whose career changes is scored again). Only the
// pick's is read (`RollKeep`); lo is over the rows that can be the pick (`is_runnable`).
fn prune_plan_scorings(input: &PlanInputs, runs: &mut [PlanRow]) {
    let is_runnable = |f: &Found| input.is_runnable(f);

    let lo = runs
        .iter()
        .filter(|r| r.exact && is_runnable(&r.career))
        .filter_map(PlanRow::net_saving)
        .fold(f64::NEG_INFINITY, f64::max);
    if !lo.is_finite() {
        return;
    }
    let thr = near_floor(lo) - 1e-6 * (1.0 + lo.abs());
    let pr = |f: &Found| (f.a.min(f.b), f.a.max(f.b));
    for i in 0..runs.len() {
        let Some(scoring) = &runs[i].scoring else { continue };
        if scoring.kept.is_none() || !(scoring.net_saving < thr) {
            continue;
        }
        if (i + 1..runs.len()).any(|k| runs[k].trainee == runs[i].trainee && pr(&runs[k].career) == pr(&runs[i].career))
        {
            continue;
        }
        runs[i].scoring.as_mut().unwrap().kept = None;
    }
}

#[cfg(test)]
mod cached_search_tests {
    use super::*;

    #[test]
    fn stored_searches_distinguish_missing_empty_and_found() {
        let found = Found {
            t: 2,
            a: 0,
            b: 1,
            val: 0.25,
            ub: 0.5,
            sel: Sel { ents: vec![1], plain: 1, filler: 0, dw: vec![-0.0], dg: vec![0.5] },
        };
        let searches =
            vec![CachedSearch::Unsearched, CachedSearch::Known(None), CachedSearch::Known(Some(found.clone()))];
        let rows = json!(cached_searches_out(&searches));
        assert_eq!(rows, json!([[1, null], [2, super::super::super::found_out(&found)]]));

        let stored = json!([{
            "k": ["0000000000000001", "0000000000000002"], "n": 3,
            "b": rows, "l": rows, "u": [2], "lu": [1],
        }]);
        let decoded = bests_in(&stored).unwrap();
        let cached = &decoded[0];
        assert!(cached.best[0].is_unsearched());
        assert!(matches!(cached.best[1], CachedSearch::Known(None)));
        assert!(!cached.best[1].is_unsearched());
        assert!(cached.best[1].found().is_none());
        assert_eq!(cached.best[2].found().unwrap().sel.dw[0].to_bits(), (-0.0f64).to_bits());
        assert_eq!(json!(cached_searches_out(&cached.best)), stored[0]["b"]);
        assert_eq!(json!(cached_searches_out(&cached.looped)), stored[0]["l"]);
        assert_eq!(cached.unp, [false, false, true]);
        assert_eq!(cached.lunp, [false, true, false]);

        // counts and indices are converted checked (usize is 32 bits on wasm32): an oversized count is refused, an
        // index past u32::MAX is out of range (not wrapped into it), an unusable flag is skipped
        let wide = u64::from(u32::MAX) + 2;
        let refused =
            |n: Value, b: Value| bests_in(&json!([{"k": stored[0]["k"], "n": n, "b": b, "l": [], "u": [], "lu": []}]));
        assert!(refused(json!(u64::MAX), json!([])).is_none(), "a count beyond STATE_MAX_N is refused");
        assert!(refused(json!(3), json!([[wide, null]])).is_none(), "an index past u32::MAX is refused");
        let flagged = json!([{"k": stored[0]["k"], "n": 3, "b": [], "l": [], "u": [wide, 1], "lu": []}]);
        assert_eq!(bests_in(&flagged).unwrap()[0].unp, [false, true, false]);
    }
}
