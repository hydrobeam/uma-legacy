//! Per-uma decks (`run_multi`): the per-uma tables of exact values by inheritance chance and the loop search over them.
use super::enumerate::{SearchMode, SearchOutcome};
use super::{
    Card, CardsRaw, Choice, Cl, Data, DeckEval, DeckId, DeckJob, DeckSpace, EXACT_BUDGET, EXACT_RHO, EXACT_STEPS,
    GEN_GOLD, Instant, JointExt, Kind, Model, NONE, NT, Name, PDeck, Res, SPT, Sched, SkillJob, Sources, Src, UCHK,
    Value, chart_rows, check_sizes, joint, joint_b, joint_fast, json, k_filter, lins, member_json, members,
    optimize_fam, pair_ceilings, prebuilt, probs, split, src_class, upper_na, wval,
};
use std::borrow::Cow;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

/// Per-uma decks: each trainee runs her own best legal deck, the same one in all the lineages she heads (three in a
/// four-uma loop, one in a three-uma rotation).
///   value(f) = fixed/L * sum_t max over decks D legal for f[t] of sum over her lineages l of T(f[t], q_l, D), L = 12 or 3
/// The skyline (decks no deck beats on every coordinate) holds every uma's best deck as long as none of its decks
/// carries a card that blocks her; for a uma it does, her legal decks come from a second deck search without
/// those cards, run the first time a foursome with her is solved. Bounds use the whole skyline (legal or not),
/// which no legal deck beats, so the search is exact.
pub(super) fn run_multi(
    d: &Data,
    cr: &CardsRaw,
    model: &Model,
    sj: &SkillJob,
    dj: &DeckJob,
    sky: &[PDeck],
    stats: Value,
    space: &DeckSpace,
) -> Value {
    let m = match Multi::new(d, cr, model, sj, dj, sky, space) {
        Ok(m) => m,
        Err(e) => return e,
    };
    let (pass, cert) = m.certify();
    if pass.solved.is_empty() {
        return json!({"error": "No loop contains all the required umas."});
    }
    m.debug_summary(&pass, cert);
    m.report(pass, cert, stats)
}

/// Points on the chance grid of the ceiling tables
const NG: usize = 64;
/// A memo key of up to 12 chances (a loop's lineages): their bits and count, a fixed array (equal exactly when the
/// chance lists are bit for bit)
type QKey = ([u64; 12], u8);
/// A loop's shared-deck list: (exact sum over its lineages, deck), best first
type SList = Rc<Vec<(f64, PDeck)>>;
/// A deck's identity in the shared-deck results: (card ids, end links, variants, plain pals)
type DeckKey = (Vec<u32>, Vec<u8>, Vec<u32>, usize);

/// What the search's phases share: the job, the skyline with its per-class views and clusters, the ceiling tables,
/// the exact caches and the tally. Tables filled on demand sit in cells, so every phase works through `&self`.
struct Multi<'data> {
    d: &'data Data,
    model: &'data Model<'data>,
    sj: &'data SkillJob,
    dj: &'data DeckJob,
    space: &'data DeckSpace<'data>,
    sky: &'data [PDeck],
    unc: Vec<u32>,
    nu: usize,
    ntar: usize,
    // (looked up only, never iterated: the hasher changes no order)
    card_of: crate::FxMap<u32, &'data Card>,
    pool: Vec<usize>,
    req: Vec<usize>,
    with3: bool,
    with4: bool,
    fixed_s: Option<Vec<Option<Sched>>>,
    no_race: bool,
    /// pair ceilings (or the fixed schedules' pair values), by uma index
    pb: Vec<Vec<f64>>,
    fixed: f64,
    /// each uma's highest inheritance chance
    qmax: Vec<f64>,
    grid: Grid,
    zero: Vec<f64>,
    /// per class of umas with the same value coordinates (`ucoords`): its skyline decks and their clusters; each
    /// uma's class
    class_sky: Vec<Vec<usize>>,
    class_cl: Vec<Vec<Cl>>,
    ucls: Vec<usize>,
    /// clusters of the whole skyline (empty with per-uma decks)
    full_cl: Vec<Cl>,
    /// the clusters' bounds in reused buffers (`bb_max`, `sky_top`)
    bb_buf: RefCell<Vec<ByBound>>,
    st_buf: RefCell<Vec<ByBound>>,
    /// B[u][i] = max over all skyline decks (legal for u or not) of T(u, grid[i], deck): no legal deck beats it
    btab_c: RefCell<Vec<[f64; NG]>>,
    /// exact ceiling table: X[u][i] = best exact value over every legal deck at chance grid[i] under the cap it was
    /// computed for (rises with the chance, so a lineage with chance q is bounded at grid[gidx(q)]; each lineage may
    /// take its own deck)
    xtab_c: RefCell<Vec<[(f64, f64); NG]>>,
    /// the deck space's entry per (card id, end link, variant)
    ent_ix: crate::FxMap<(u32, u8, u32), usize>,
    ex_cache: RefCell<crate::FxMap<(usize, QKey), ExactCacheEntry>>,
    sh_cache: RefCell<crate::FxMap<([usize; 4], QKey), (f64, f64, SList)>>,
    /// schedules found, per foursome (shared by the passes)
    sched_c: RefCell<crate::FxMap<[usize; 4], Res>>,
    tally: Tally,
    caps: [f64; 4],
    rho_top: f64,
    shared: bool,
    kloops: usize,
    /// the pool by M_u (`rank`), highest first, and their M_u; the highest structure sum
    ord: Vec<usize>,
    mv: Vec<f64>,
    smax: f64,
    tq: Instant,
}

/// The chance grid the ceiling tables are kept on: NG points from the floor to the highest chance any uma reaches
struct Grid {
    qlo: f64,
    step: f64,
    q: Vec<f64>,
}
impl Grid {
    fn new(qlo: f64, qhi: f64) -> Self {
        let step = if qhi > qlo { (qhi - qlo) / (NG - 1) as f64 } else { 0.0 };
        let q: Vec<f64> = (0..NG).map(|i| if i == NG - 1 { qhi } else { qlo + step * i as f64 }).collect();
        Self { qlo, step, q }
    }
    /// the first grid point at or above q (the last when the grid is one point)
    fn idx(&self, q: f64) -> usize {
        if self.step == 0.0 {
            return NG - 1;
        }
        let mut i = (((q - self.qlo) / self.step).ceil().max(0.0) as usize).min(NG - 1);
        while self.q[i] < q && i < NG - 1 {
            i += 1;
        }
        i
    }
}

/// The exact search's counters, timers and work budget (`cap_hit` and `max_ratio` per structure class, `cap_class`)
#[derive(Default)]
struct Tally {
    n_scored: Cell<usize>,
    n_band: Cell<usize>,
    n_ufilt: Cell<usize>,
    cap_hit: Cell<bool>,
    max_ratio: Cell<[f64; 4]>,
    max_deck: RefCell<[Option<(u32, Vec<Name>)>; 4]>,
    t_band: Cell<f64>,
    t_score: Cell<f64>,
    t_seed: Cell<f64>,
    t_ub: Cell<f64>,
    work: Cell<usize>,
    budget: Cell<usize>,
    /// set when a pass runs out of budget (or a deck search out of room): that pass certifies nothing
    aborted: Cell<bool>,
}
impl Tally {
    fn spend(&self, w: usize) {
        self.work.set(self.work.get() + w);
    }
    fn ratio_at(&self, k: usize, r: f64, u: u32, x: &PDeck) {
        let mut m = self.max_ratio.get();
        if r > m[k] {
            m[k] = r;
            self.max_ratio.set(m);
            self.max_deck.borrow_mut()[k] = Some((u, x.names.clone()));
        }
    }
}

struct Best {
    sum: f64,
    deck: Option<PDeck>,
    per: Vec<(f64, f64, usize)>,
}
// per (uma, chances): the best found, decks already scored or bounded, and the cap it is exact under
struct ExactCacheEntry {
    best: Option<(f64, PDeck)>,
    /// Detached decks already scored exactly (including skyline seeds).
    scored_decks: crate::FxSet<DeckId>,
    /// Band selections already scored or rejected by the proven bound.
    checked_selections: crate::FxSet<(Vec<usize>, usize, usize)>,
    /// Highest completed cap; -1 means unseeded, 1 means seeded only.
    certified_cap: f64,
    reported_best: Option<Rc<Best>>,
}

/// One loop's shared-deck scoring: its members' ids, (trainee, chance) per lineage and each member's chances, the
/// floor, and the decks scored so far
struct SharedScan {
    f: [usize; 4],
    nm: usize,
    ids: Vec<u32>,
    pairs: Vec<(usize, f64)>,
    qt: Vec<Vec<f64>>,
    floor: f64,
    kloops: usize,
    done: crate::FxSet<DeckId>,
    list: Vec<(f64, PDeck)>,
}
impl SharedScan {
    fn new(d: &Data, f: [usize; 4], qa: &[f64], floor: f64, kloops: usize) -> Self {
        let ln = lin_of(&f);
        let fm = members(&f);
        let nm = fm.len();
        let ids: Vec<u32> = fm.iter().map(|&u| d.umas[u].id).collect();
        let pairs: Vec<(usize, f64)> = (0..ln.len()).map(|l| (ln[l].0, qa[l])).collect();
        let qt: Vec<Vec<f64>> =
            (0..nm).map(|t| (0..ln.len()).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect()).collect();
        Self { f, nm, ids, pairs, qt, floor, kloops, done: crate::FxSet::default(), list: vec![] }
    }
    /// the threshold a deck must reach: the floor, or the loop's k-th best so far when that is higher
    fn need(&self) -> f64 {
        let mut v: Vec<f64> = self.list.iter().map(|x| x.0).collect();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let kl = if v.len() >= self.kloops { v[self.kloops - 1] } else { f64::NEG_INFINITY };
        let t = self.floor.max(kl);
        if t.is_finite() { t } else { v.last().copied().unwrap_or(f64::NEG_INFINITY) }
    }
}

/// A solved loop: members, schedule, lineage chances, per-member bests (per-uma decks) and value
struct Solved {
    f: [usize; 4],
    r: Res,
    qs: Vec<f64>,
    bests: Vec<Rc<Best>>,
    val: f64,
}
/// One pass's result: the loops solved, the k-th best value, the bound on what it left unexamined and its tallies;
/// with a shared deck, each deck's best value and where (solved index, list index) and the per-loop deck lists
struct Pass {
    solved: Vec<Solved>,
    kth: f64,
    next_bound: f64,
    scanned: usize,
    generated: usize,
    sched_rejects: usize,
    deck_best: crate::FxMap<DeckKey, (f64, usize, usize)>,
    lists: Vec<SList>,
    /// (per-uma decks: the solved loops' values, in solve order)
    vals: Vec<f64>,
}
/// A pass's queue of structures: (ceiling, members) in ceiling order, the next to scan, and the sum below which no
/// band has been generated yet
struct Queue {
    cand: Vec<(f64, [usize; 4])>,
    next: usize,
    hi_sum: f64,
}
/// A reported loop: its solved index, each member's deck, per lineage (exact P(all), P(all but one), outfit), value
struct Pick<'pass> {
    si: usize,
    decks: Vec<&'pass PDeck>,
    per_l: Vec<(f64, f64, usize)>,
    value: f64,
}

// Structures: four-uma loops (12 lineages) and three-uma rotations (3 lineages; `lins`), members as [usize; 4] with
// `NONE` last for three; a structure's value is the mean over its lineages, so both sizes rank together.
fn lin_of(f: &[usize; 4]) -> &'static [crate::skills::Lin] {
    lins(members(f).len())
}
/// A three-member structure's sum s in four-member sum units
fn e3(s: f64) -> f64 {
    s * 4.0 / 3.0
}

/// The umas that can train (owned or required, not excluded, with an outfit, not blocked by a locked card) and the
/// required ones
fn trainable(
    d: &Data,
    card_of: &crate::FxMap<u32, &Card>,
    model: &Model,
    sj: &SkillJob,
    dj: &DeckJob,
) -> (Vec<usize>, Vec<usize>) {
    let n = d.umas.len();
    let id_ok = |id: u32| {
        (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id)
    };
    // a locked card goes in every deck, so a uma it blocks cannot train at all
    let lock_blocked =
        |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).is_some_and(|c| c.blocks(d.umas[u].id)));
    let pool: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let req: Vec<usize> = sj.require.iter().filter_map(|&id| d.umas.iter().position(|u| u.id == id)).collect();
    (pool, req)
}

/// Each pair's race-win ceiling: the fixed schedules' values where the job fixes them (0 without races), else the
/// pair ceilings
fn pair_table(d: &Data, pool: &[usize], fixed_s: Option<&[Option<Sched>]>, no_race: bool) -> Vec<Vec<f64>> {
    let n = d.umas.len();
    match fixed_s {
        Some(fs) => {
            let ps: Vec<Option<Vec<f64>>> =
                fs.iter().enumerate().map(|(i, s)| s.as_ref().map(|s| probs(d, &d.umas[i], s))).collect();
            let mut m = vec![vec![0.0f64; n]; n];
            if !no_race {
                for &a in pool {
                    for &b in pool {
                        if a != b {
                            m[a][b] = wval(ps[a].as_ref().unwrap(), ps[b].as_ref().unwrap());
                        }
                    }
                }
            }
            m
        }
        None => pair_ceilings(d, pool),
    }
}

// A uma's value depends only on the targets some outfit of hers lacks: (g, h) = (P(gold), P(any hint)) where
// an outfit lacks the skill, g alone where her outfits have at most the white. Decks are thinned per class of
// umas with the same coordinates: a deck another beats on all of them never wins for those umas.
fn ucoords(model: &Model, u: usize, nu: usize) -> Vec<(usize, bool)> {
    let mut c = vec![];
    for k in 0..nu {
        if model.opts[u].iter().any(|(st, _)| st[k] == 0) {
            c.push((k, true));
            c.push((k, false));
        } else if model.opts[u].iter().any(|(st, _)| st[k] == 1) {
            c.push((k, true));
        }
    }
    c
}
/// The decks of `list` no other beats on the coordinates `co`, by index, ascending
fn thin(list: &[PDeck], co: &[(usize, bool)]) -> Vec<usize> {
    if list.is_empty() {
        return vec![];
    }
    if co.is_empty() {
        return vec![0];
    }
    let tagged: Vec<DeckEval> = list
        .iter()
        .enumerate()
        .map(|(i, x)| {
            let dv: Vec<f64> =
                co.iter().map(|&(k, g)| if g { x.dg[k] } else { 1.0 - (1.0 - x.dw[k]) * (1.0 - x.dg[k]) }).collect();
            let s = dv.iter().sum();
            DeckEval {
                pals: 0,
                borrowed: None,
                cards: vec![i],
                ends: vec![],
                vars: vec![],
                plain: 0,
                filler: 0,
                dw: vec![],
                dg: vec![],
                dv,
                s,
            }
        })
        .collect();
    let mut v: Vec<usize> = k_filter(tagged, 1, co.len()).into_list().into_iter().map(|y| y.cards[0]).collect();
    v.sort_unstable();
    v
}
/// The pool's classes of umas with the same coordinates: each class's skyline decks, and each uma's class
fn classes(model: &Model, pool: &[usize], sky: &[PDeck], nu: usize, n: usize) -> (Vec<Vec<usize>>, Vec<usize>) {
    // the full coordinate set needs no thinning: the skyline is already thin on it
    let full: Vec<(usize, bool)> = (0..nu).flat_map(|k| [(k, true), (k, false)]).collect();
    let mut class_ix: crate::FxMap<Vec<(usize, bool)>, usize> = crate::FxMap::default();
    let mut class_sky: Vec<Vec<usize>> = vec![];
    let mut ucls: Vec<usize> = vec![usize::MAX; n];
    for &u in pool {
        let co = ucoords(model, u, nu);
        let ci = if let Some(&c) = class_ix.get(&co) {
            c
        } else {
            class_sky.push(if co == full { (0..sky.len()).collect() } else { thin(sky, &co) });
            class_ix.insert(co, class_sky.len() - 1);
            class_sky.len() - 1
        };
        ucls[u] = ci;
    }
    (class_sky, ucls)
}
/// Clusters of the whole skyline (one cluster of every deck without uncertain targets); none with per-uma decks
fn full_clusters(sky: &[PDeck], nu: usize, per_uma: bool) -> Vec<Cl> {
    if per_uma {
        vec![]
    } else {
        let mut ix: Vec<usize> = (0..sky.len()).collect();
        let mut out = vec![];
        if nu > 0 && !ix.is_empty() {
            split(sky, &mut ix, nu, &mut out);
        } else {
            out.push(Cl { mdw: vec![], mdg: vec![], mem: ix });
        }
        out
    }
}

impl<'data> Multi<'data> {
    /// Setup: the pool, its pair ceilings and chance grid, the skyline's classes and clusters, and the pool's order;
    /// Err with the job's error when the required umas cannot train or the sizes do not fit
    fn new(
        d: &'data Data,
        cr: &'data CardsRaw,
        model: &'data Model<'data>,
        sj: &'data SkillJob,
        dj: &'data DeckJob,
        sky: &'data [PDeck],
        space: &'data DeckSpace<'data>,
    ) -> Result<Self, Value> {
        let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
        let nu = unc.len();
        let card_of: crate::FxMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
        let n = d.umas.len();
        let (pool, req) = trainable(d, &card_of, model, sj, dj);
        if req.iter().any(|r| !pool.contains(r)) {
            return Err(json!({"error": "A required uma cannot train with a locked card of her own."}));
        }
        if let Err(e) = check_sizes(&sj.sizes, pool.len(), req.len()) {
            return Err(json!({"error": e}));
        }
        let (with3, with4) = (sj.sizes.contains(&3), sj.sizes.contains(&4));
        let tq = Instant::now();
        let fixed_s: Option<Vec<Option<Sched>>> = dj
            .schedule
            .as_deref()
            .map(|mode| (0..n).map(|i| pool.contains(&i).then(|| prebuilt(d, &d.umas[i], mode))).collect());
        let no_race = dj.schedule.as_deref() == Some("none");
        let pb = pair_table(d, &pool, fixed_s.as_deref(), no_race);
        let fixed = model.fixed_factor();
        let qmax: Vec<f64> =
            (0..n).map(|u| if pool.contains(&u) { model.q_ub_trainee(u, &pool, &pb, with3) } else { 0.0 }).collect();
        let qlo = model.q_floor();
        let qhi = pool.iter().map(|&u| qmax[u]).fold(qlo, f64::max);
        let grid = Grid::new(qlo, qhi);
        let (class_sky, ucls) = classes(model, &pool, sky, nu, n);
        // Clusters of each class's decks (recursive median splits on the widest odds coordinate) with their
        // componentwise maximum odds. T(u, q, deck) rises with every deck odds coordinate (for 0 <= q <= 1, since a
        // white is worth less than a gold), so T at a cluster's maximum bounds every member; maxima over a class are
        // found cluster by cluster, best bound first, skipping clusters whose bound (with 1e-9 relative slack for
        // rounding) is below the best value so far. The maximum of a set does not depend on visit order, so the
        // result is the same value bit for bit.
        let class_cl: Vec<Vec<Cl>> = class_sky
            .iter()
            .map(|v| {
                let mut ix = v.clone();
                let mut out = vec![];
                if nu > 0 && !ix.is_empty() {
                    split(sky, &mut ix, nu, &mut out);
                }
                out
            })
            .collect();
        if std::env::var("DECK_DEBUG").is_ok() {
            eprintln!(
                "classes {} sizes {:?} ({:.2}s)",
                class_sky.len(),
                class_sky.iter().map(std::vec::Vec::len).collect::<Vec<_>>(),
                tq.elapsed().as_secs_f64()
            );
        }
        // A deck's sources: the deck space's entry for (card, end link, variant) where it has one of that card (built
        // by the same `Sources::src` and `kind` on the same job and targets, so the same values), borrowed; else built
        let ent_ix: crate::FxMap<(u32, u8, u32), usize> =
            (0..space.rel.len()).map(|j| ((space.cards[space.rel[j]].id, space.endk[j], space.vark[j]), j)).collect();
        let caps = space.caps();
        let full_cl = full_clusters(sky, nu, dj.per_uma);
        let mut m = Multi {
            d,
            model,
            sj,
            dj,
            space,
            sky,
            unc,
            nu,
            ntar: sj.targets.len(),
            card_of,
            pool,
            req,
            with3,
            with4,
            fixed_s,
            no_race,
            pb,
            fixed,
            qmax,
            grid,
            zero: vec![0.0f64; nu],
            class_sky,
            class_cl,
            ucls,
            full_cl,
            bb_buf: RefCell::default(),
            st_buf: RefCell::default(),
            btab_c: RefCell::new(vec![[f64::NAN; NG]; n]),
            xtab_c: RefCell::new(vec![[(f64::NAN, 0.0); NG]; n]),
            ent_ix,
            ex_cache: RefCell::default(),
            sh_cache: RefCell::default(),
            sched_c: RefCell::default(),
            tally: Tally::default(),
            caps,
            rho_top: caps[3],
            shared: !dj.per_uma,
            kloops: dj.top_decks.max(1),
            ord: vec![],
            mv: vec![],
            smax: 0.0,
            tq,
        };
        m.rank();
        Ok(m)
    }

    /// Orders the pool by M_u (the product ceiling at her highest chance), highest first, for `band`, and sets the
    /// highest structure sum
    fn rank(&mut self) {
        let n = self.d.umas.len();
        let mval: Vec<f64> = (0..n)
            .map(|u| if self.pool.contains(&u) { self.btab(u, self.grid.idx(self.qmax[u])) } else { 0.0 })
            .collect();
        let mut ord: Vec<usize> = self.pool.clone();
        ord.sort_by(|&a, &b| mval[b].partial_cmp(&mval[a]).unwrap());
        let mv: Vec<f64> = ord.iter().map(|&u| mval[u]).collect();
        let k = self.pool.len();
        self.smax = (if self.with4 && k >= 4 { mv[0] + mv[1] + mv[2] + mv[3] } else { 0.0 })
            .max(if self.with3 && k >= 3 { e3(mv[0] + mv[1] + mv[2]) } else { 0.0 });
        self.ord = ord;
        self.mv = mv;
    }

    fn uview(&self, u: usize) -> &Vec<usize> {
        &self.class_sky[self.ucls[u]]
    }

    // ---- product-odds ceilings ----

    /// max(init, max over u's class of val(deck)), with bound(cluster maxima) >= val of every member
    /// (the clusters' bounds in a reused buffer, popped from a heap in the stable sort's order: `ByBound`)
    fn bb_max(&self, u: usize, init: f64, val: &dyn Fn(&[f64], &[f64]) -> f64) -> f64 {
        let cls = &self.class_cl[self.ucls[u]];
        let sky = &self.sky;
        if cls.is_empty() {
            return self.uview(u).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max);
        }
        let mut bs = std::mem::take(&mut *self.bb_buf.borrow_mut());
        bs.clear();
        bs.extend(cls.iter().enumerate().map(|(i, c)| ByBound { bound: val(&c.mdw, &c.mdg), cluster: i }));
        let mut h = std::collections::BinaryHeap::from(bs);
        let mut best = init;
        while let Some(ByBound { bound: b, cluster: i }) = h.pop() {
            if b * (1.0 + 1e-9) < best {
                break;
            }
            for &j in &cls[i].mem {
                best = best.max(val(&sky[j].dw, &sky[j].dg));
            }
        }
        *self.bb_buf.borrow_mut() = h.into_vec();
        best
    }

    /// B[u][i] (`btab_c`), computed at first need
    fn btab(&self, u: usize, i: usize) -> f64 {
        let v = self.btab_c.borrow()[u][i];
        if !v.is_nan() {
            return v;
        }
        let (model, sky) = (self.model, &self.sky);
        let q = self.grid.q[i];
        let init = model.trainee_fast(u, q, &self.zero, &self.zero);
        let v = if (0.0..=1.0).contains(&q) {
            self.bb_max(u, init, &|dw, dg| model.trainee_fast(u, q, dw, dg))
        } else {
            self.uview(u).iter().map(|&j| model.trainee_fast(u, q, &sky[j].dw, &sky[j].dg)).fold(init, f64::max)
        };
        self.btab_c.borrow_mut()[u][i] = v;
        v
    }

    /// The best m skyline decks by `val` among those passing `ok` (cluster maxima bound their members, best bound
    /// first; `val` must rise with every odds coordinate)
    // (as `bb_max`: the clusters in the stable sort's order, equal bounds by index, from a heap over a reused buffer)
    fn sky_top(&self, m: usize, val: &dyn Fn(&[f64], &[f64]) -> f64, ok: &dyn Fn(&PDeck) -> bool) -> Vec<(f64, usize)> {
        let (sky, full_cl, nu) = (&self.sky, &self.full_cl, self.nu);
        let mut bs = std::mem::take(&mut *self.st_buf.borrow_mut());
        bs.clear();
        bs.extend(
            full_cl
                .iter()
                .enumerate()
                .map(|(i, c)| ByBound { bound: if nu > 0 { val(&c.mdw, &c.mdg) } else { f64::INFINITY }, cluster: i }),
        );
        let mut h = std::collections::BinaryHeap::from(bs);
        let mut top: Vec<(f64, usize)> = vec![];
        while let Some(ByBound { bound: b, cluster: ci }) = h.pop() {
            if top.len() >= m && b * (1.0 + 1e-9) < top[m - 1].0 {
                break;
            }
            for &j in &full_cl[ci].mem {
                if !ok(&sky[j]) {
                    continue;
                }
                let v = val(&sky[j].dw, &sky[j].dg);
                if top.len() < m || v > top[m - 1].0 {
                    let pos = top.iter().position(|x| v > x.0).unwrap_or(top.len());
                    top.insert(pos, (v, j));
                    top.truncate(m);
                }
            }
        }
        *self.st_buf.borrow_mut() = h.into_vec();
        top
    }

    /// shared-deck ceiling: the best single skyline deck (legal or not) over the loop's twelve lineages, product odds
    fn relaxed_shared(&self, f: [usize; 4], qa: &[f64; 12]) -> f64 {
        let model = self.model;
        let ln = lin_of(&f);
        let val =
            |dw: &[f64], dg: &[f64]| (0..ln.len()).map(|l| model.trainee_fast(f[ln[l].0], qa[l], dw, dg)).sum::<f64>();
        self.sky_top(1, &val, &|_| true).first().map_or(0.0, |x| x.0) * self.fixed / ln.len() as f64 * (1.0 + 1e-12)
    }

    /// ceiling with each uma's best skyline deck (legal or not), the same deck across her lineages
    fn relaxed(&self, f: [usize; 4], qa: &[f64; 12]) -> f64 {
        let (model, sky) = (self.model, &self.sky);
        let mut tot = 0.0;
        let ln = lin_of(&f);
        for t in 0..members(&f).len() {
            let qs: Vec<f64> = (0..ln.len()).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect();
            let init = qs.iter().map(|&q| model.trainee_fast(f[t], q, &self.zero, &self.zero)).sum::<f64>();
            let val = |dw: &[f64], dg: &[f64]| qs.iter().map(|&q| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>();
            tot += if qs.iter().all(|q| (0.0..=1.0).contains(q)) {
                self.bb_max(f[t], init, &val)
            } else {
                self.uview(f[t]).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max)
            };
        }
        self.fixed * tot / ln.len() as f64 * (1.0 + 1e-12)
    }

    /// A structure's product ceiling is at most the guaranteed golds' factor times the mean of its members' M_u (each
    /// member heads the same number of lineages): (3/12)·sum for four, sum/3 for three. Bands are in four-member sum
    /// units: a three-member structure's sum s counts as e3(s) = 4s/3.
    fn sep(&self, s: f64) -> f64 {
        self.fixed * 3.0 * s / 12.0 * (1.0 + 1e-12)
    }

    // ---- exact per-uma values ----
    // Loops are ranked by exact odds. A trainee's exact value at her lineages' inheritance chances is the best exact
    // P(all) over every legal deck (`DeckSpace`, not only the skyline). It is seeded with her best legal skyline decks
    // by product odds; then every legal deck whose product value times EXACT_RHO reaches the best exact value is scored
    // exactly. That is exact under one assumption: no deck's exact odds exceed EXACT_RHO times its product odds.

    /// A deck's sources: the deck space's entry for (card, end link, variant) where it has one of that card, borrowed;
    /// else built. With the filler count.
    fn ents_of(&self, x: &PDeck) -> (Vec<(Cow<'data, Src>, Kind)>, usize) {
        let space = self.space;
        let srcb = Sources { dj: self.dj, unc: &self.unc };
        let e: Vec<(Cow<Src>, Kind)> = x
            .ids
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                self.card_of.get(id).map(|c| {
                    let st = x.ends.get(i).copied().filter(|&e| e > 0);
                    let v = x.vars.get(i).copied().unwrap_or(0);
                    match self.ent_ix.get(&(*id, st.unwrap_or(0), v)) {
                        Some(&j) if std::ptr::eq(space.cards[space.rel[j]], *c) => {
                            (Cow::Borrowed(&space.srcs[j]), space.kind[j])
                        }
                        _ => (Cow::Owned(srcb.src(c, st, v)), srcb.kind(c, st)),
                    }
                })
            })
            .collect();
        (e, self.dj.deck_size.saturating_sub(x.ids.len() + x.plain_ids.len()))
    }

    /// Exact value of trainee u at chances qs under cap lv: the best exact P(all) over her best legal skyline decks
    /// and every legal deck whose product value times lv reaches it (each first through the proven bound).
    fn exact_best(&self, u: usize, qs: &[f64], lv: f64) -> Rc<Best> {
        let key = (u, qkey(qs));
        if let Some(entry) = self.ex_cache.borrow().get(&key)
            && entry.certified_cap >= lv
            && let Some(result) = &entry.reported_best
        {
            return Rc::clone(result);
        }
        let mut ent = self.ex_cache.borrow_mut().remove(&key).unwrap_or(ExactCacheEntry {
            best: None,
            scored_decks: crate::FxSet::default(),
            checked_selections: crate::FxSet::default(),
            certified_cap: -1.0,
            reported_best: None,
        });
        if ent.certified_cap < 0.0 {
            self.seed_exact(u, qs, &mut ent);
            ent.certified_cap = 1.0;
        }
        if lv > ent.certified_cap {
            self.raise_exact(u, qs, lv, &mut ent);
        }
        let b = Rc::new(self.best_of(u, qs, &ent));
        ent.reported_best = Some(Rc::clone(&b));
        self.ex_cache.borrow_mut().insert(key, ent);
        b
    }

    /// Scores deck x exactly for trainee u at chances qs (once per entry) and keeps it if it is the best
    fn score_exact(&self, u: usize, qs: &[f64], x: &PDeck, ent: &mut ExactCacheEntry) {
        if !ent.scored_decks.insert(x.id()) {
            return;
        }
        let (model, tally) = (self.model, &self.tally);
        let uid = self.d.umas[u].id;
        let t0 = Instant::now();
        let (e, fl) = self.ents_of(x);
        let j = joint_b(model, u, qs, &e, fl, self.ntar, false);
        tally.t_score.set(tally.t_score.get() + t0.elapsed().as_secs_f64());
        if std::env::var("UPPER_CHECK").is_ok() {
            let er: Vec<(&Src, Kind)> = e.iter().map(|(s, k)| (&**s, *k)).collect();
            if let Some(ub) = upper_na(model, u, qs, &er, fl) {
                for (i, y) in j.iter().enumerate() {
                    assert!(
                        ub[i] >= y.0 * (1.0 - 1e-12),
                        "upper bound below exact: {} < {} for {:?}",
                        ub[i],
                        y.0,
                        x.names
                    );
                    UCHK.with(|c| {
                        let mut v = c.get();
                        v.0 += 1;
                        v.1 = v.1.max(ub[i] / y.0);
                        c.set(v);
                    });
                }
            } else {
                UCHK.with(|c| {
                    let mut v = c.get();
                    v.2 += 1;
                    c.set(v);
                });
            }
        }
        tally.n_scored.set(tally.n_scored.get() + 1);
        tally.spend(200);
        let k = src_class(e.iter().map(|y| &*y.0));
        for (i, &q) in qs.iter().enumerate() {
            let p = self.fixed * model.trainee_fast(u, q, &x.dw, &x.dg);
            if p > 0.0 {
                tally.ratio_at(k, j[i].0 / p, uid, x);
            }
        }
        let sv: f64 = j.iter().map(|y| y.0).sum();
        if ent.best.as_ref().is_none_or(|b| sv > b.0) {
            ent.best = Some((sv, x.clone()));
        }
    }

    /// seed: her best legal skyline decks by product value (or the best legal deck when none is on the skyline)
    fn seed_exact(&self, u: usize, qs: &[f64], ent: &mut ExactCacheEntry) {
        let (model, space) = (self.model, self.space);
        let uid = self.d.umas[u].id;
        let t0 = Instant::now();
        // the first 3 of a stable sort by value, highest first: each deck goes after every kept one at least as high
        let mut seeds: Vec<(f64, &PDeck)> = Vec::with_capacity(4);
        // (the value first, the blocks lookups only for a deck that would enter: one that can't is passed over blocked
        // or not, so the seeds are the same)
        for x in self.uview(u).iter().map(|&j| &self.sky[j]) {
            let t = self.fixed * qs.iter().map(|&q| model.trainee_fast(u, q, &x.dw, &x.dg)).sum::<f64>();
            let at = seeds
                .iter()
                .position(|s| s.0.partial_cmp(&t).unwrap() == std::cmp::Ordering::Less)
                .unwrap_or(seeds.len());
            if at < 3 && !x.blocks(uid, &self.card_of) {
                seeds.insert(at, (t, x));
                seeds.truncate(3);
            }
        }
        self.tally.t_seed.set(self.tally.t_seed.get() + t0.elapsed().as_secs_f64());
        for (_, x) in seeds.iter().take(3) {
            self.score_exact(u, qs, x, ent);
        }
        if ent.best.is_none() {
            let SearchOutcome { decks: r, .. } = space.search(
                model,
                &[u],
                &[uid],
                &[(qs.iter().map(|&q| (0, q)).collect(), 0.0)],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            if let Some((_, sel)) = r.first() {
                let x = space.pdeck(sel, &[uid]);
                self.score_exact(u, qs, &x, ent);
            }
        }
    }

    /// Raises an entry's certified cap to lv: every legal deck that might beat its best under the cap is scored (after
    /// the proven bound); a deck search cut short or the budget running out certifies nothing and aborts the pass
    fn raise_exact(&self, u: usize, qs: &[f64], lv: f64, ent: &mut ExactCacheEntry) {
        let (model, space, tally) = (self.model, self.space, &self.tally);
        let uid = self.d.umas[u].id;
        let mut complete = true;
        let Some(thr) = ent.best.as_ref().map(|b| b.0) else {
            ent.certified_cap = lv;
            return;
        };
        tally.n_band.set(tally.n_band.get() + 1);
        let t0 = Instant::now();
        let n0 = space.nodes.get();
        let SearchOutcome { decks: mut cands, incomplete } = space.search(
            model,
            &[u],
            &[uid],
            &[(qs.iter().map(|&q| (0, q)).collect(), thr)],
            SearchMode::Band { ratio_cap: lv, max_decks: space.band_cap() },
        );
        tally.spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
        tally.t_band.set(tally.t_band.get() + t0.elapsed().as_secs_f64());
        if incomplete.is_some() {
            tally.cap_hit.set(true);
            complete = false;
        }
        cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        if complete {
            for (v, sel) in &cands {
                if lv * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 {
                    break;
                }
                if tally.work.get() > tally.budget.get() {
                    complete = false;
                    break;
                }
                // below the best at its own class's cap (the best rose since the band was built)
                if space.sel_cap(sel, lv) * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 {
                    continue;
                }
                if !ent.checked_selections.insert((sel.ents.clone(), sel.plain, sel.filler)) {
                    continue;
                }
                // the proven bound first: a deck it puts below the best exact value cannot beat it
                let e: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                let t1 = Instant::now();
                let ubo = upper_na(model, u, qs, &e, sel.filler);
                tally.t_ub.set(tally.t_ub.get() + t1.elapsed().as_secs_f64());
                if let Some(ub) = ubo {
                    tally.spend(25);
                    if ub.iter().sum::<f64>() * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 {
                        tally.n_ufilt.set(tally.n_ufilt.get() + 1);
                        continue;
                    }
                }
                let x = space.pdeck(sel, &[uid]);
                self.score_exact(u, qs, &x, ent);
            }
        }
        if complete {
            ent.certified_cap = lv;
        } else {
            tally.aborted.set(true);
        }
    }

    /// The entry's best with its per-lineage values (the deckless value when no deck is legal)
    fn best_of(&self, u: usize, qs: &[f64], ent: &ExactCacheEntry) -> Best {
        if let Some((sum, x)) = &ent.best {
            let (e, fl) = self.ents_of(x);
            let per = joint_b(self.model, u, qs, &e, fl, self.ntar, true);
            Best { sum: *sum, deck: Some(x.clone()), per }
        } else {
            let per = joint(self.model, u, qs, &[], 0, self.ntar, true);
            Best { sum: per.iter().map(|y| y.0).sum(), deck: None, per }
        }
    }

    /// X[u][i] under cap lv (`xtab_c`), computed when not known under lv or more
    fn xtab(&self, u: usize, i: usize, lv: f64) -> f64 {
        let (v, l) = self.xtab_c.borrow()[u][i];
        if !v.is_nan() && l >= lv {
            return v;
        }
        let v = self.exact_best(u, &[self.grid.q[i]], lv).sum;
        // a value from a deck search the budget cut short is not exact under lv: not kept
        if !self.tally.aborted.get() {
            self.xtab_c.borrow_mut()[u][i] = (v, lv);
        }
        v
    }

    /// Ceiling of a cell (uma, grid point) under cap lv: the exact value if computed under lv or more, else the lowest
    /// such value at a higher grid point, else the product-odds bound times lv; (bound, exact?)
    fn cell(&self, u: usize, i: usize, lv: f64) -> (f64, bool) {
        let mut b = lv * self.fixed * self.btab(u, i) * (1.0 + 1e-12);
        let xb = self.xtab_c.borrow();
        let row = &xb[u];
        for j in i..NG {
            if !row[j].0.is_nan() && row[j].1 >= lv {
                b = b.min(row[j].0 * (1.0 + 1e-12));
                return (b, j == i);
            }
        }
        (b, false)
    }

    /// Exact ceiling of a foursome at chances `qa`, refined lazily: while it beats `kth`, the loosest inexact term is
    /// tightened (first computed at the uma's highest grid point, then halfway down towards its own) until the ceiling
    /// stops beating `kth` or every term is exact. Each step is one exact deck search for one uma.
    fn exact_ceiling(&self, f: [usize; 4], qa: &[f64; 12], kth: f64, lv: f64) -> f64 {
        loop {
            let mut tot = 0.0;
            let mut worst: Option<(f64, usize, usize)> = None;
            let ln = lin_of(&f);
            for l in 0..ln.len() {
                let (u, i) = (f[ln[l].0], self.grid.idx(qa[l]));
                let (b, ex) = self.cell(u, i, lv);
                tot += b;
                if !ex && worst.is_none_or(|w| b > w.0) {
                    worst = Some((b, u, i));
                }
            }
            let ce = tot / ln.len() as f64;
            let Some((_, u, i)) = worst else { return ce };
            if ce <= kth * (1.0 + 4e-12) || self.tally.aborted.get() {
                return ce;
            }
            let j = {
                let xb = self.xtab_c.borrow();
                let row = &xb[u];
                (i..NG).find(|&j| !row[j].0.is_nan() && row[j].1 >= lv)
            };
            let tgt = match j {
                None => i.max(self.grid.idx(self.qmax[u])),
                Some(j) => i + (j - i) / 2,
            };
            self.xtab(u, tgt, lv);
        }
    }

    // ---- shared deck ----
    // With one deck for the whole loop, a loop's value is the best exact sum over its twelve lineages among decks legal
    // for all four members. Per loop: every such deck whose exact sum is at least `floor` (the k-th best distinct
    // deck so far, times 12) or among the loop's own best `top_decks`, under cap lv; sorted, best first. Ceilings
    // stay the per-uma ones (each trainee's best deck bounds her share of any shared deck).
    fn exact_shared(&self, f: [usize; 4], qa: &[f64], floor: f64, lv: f64) -> SList {
        let key = (f, qkey(qa));
        if let Some((l, fl, r)) = self.sh_cache.borrow().get(&key)
            && *l >= lv
            && *fl <= floor
        {
            return Rc::clone(r);
        }
        let mut sc = SharedScan::new(self.d, f, qa, floor, self.kloops);
        self.seed_shared(&mut sc);
        let complete = if lv > 1.0 && !sc.list.is_empty() { self.band_shared(&mut sc, lv) } else { true };
        let thr = sc.need();
        let mut list = sc.list;
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        list.retain(|x| x.0 >= thr);
        let r: SList = Rc::new(list);
        if complete {
            self.sh_cache.borrow_mut().insert(key, (lv, floor, Rc::clone(&r)));
        }
        r
    }

    /// Scores deck x exactly over the loop's lineages (once per loop) and lists it
    fn score_shared(&self, sc: &mut SharedScan, x: &PDeck) {
        if !sc.done.insert(x.id()) {
            return;
        }
        let (model, tally, f) = (self.model, &self.tally, sc.f);
        let t0 = Instant::now();
        let (e, fl) = self.ents_of(x);
        let k = src_class(e.iter().map(|y| &*y.0));
        let mut sv = 0.0;
        for t in 0..sc.nm {
            let j = joint_b(model, f[t], &sc.qt[t], &e, fl, self.ntar, false);
            for (i, &q) in sc.qt[t].iter().enumerate() {
                let p = self.fixed * model.trainee_fast(f[t], q, &x.dw, &x.dg);
                if p > 0.0 {
                    tally.ratio_at(k, j[i].0 / p, self.d.umas[f[t]].id, x);
                }
                sv += j[i].0;
            }
        }
        tally.t_score.set(tally.t_score.get() + t0.elapsed().as_secs_f64());
        tally.n_scored.set(tally.n_scored.get() + 1);
        tally.spend(800);
        sc.list.push((sv, x.clone()));
    }

    /// seed: the loop's best `top_decks` + 2 skyline decks legal for every member by product value (or its best legal
    /// deck when none is on the skyline)
    fn seed_shared(&self, sc: &mut SharedScan) {
        let (model, space, fixed, f) = (self.model, self.space, self.fixed, sc.f);
        let t0 = Instant::now();
        let pairs = &sc.pairs;
        let val = |dw: &[f64], dg: &[f64]| {
            fixed * pairs.iter().map(|&(t, q)| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>()
        };
        let ids = &sc.ids;
        let seeds = self.sky_top(self.kloops + 2, &val, &|x| ids.iter().all(|&id| !x.blocks(id, &self.card_of)));
        self.tally.t_seed.set(self.tally.t_seed.get() + t0.elapsed().as_secs_f64());
        for &(_, j) in &seeds {
            self.score_shared(sc, &self.sky[j]);
        }
        if sc.list.is_empty() {
            let SearchOutcome { decks: r, .. } = space.search(
                model,
                members(&f),
                &sc.ids,
                &[(sc.pairs.clone(), 0.0)],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            if let Some((_, sel)) = r.first() {
                let x = space.pdeck(sel, &sc.ids);
                self.score_shared(sc, &x);
            }
        }
    }

    /// Every legal deck that might reach the loop's threshold under cap lv, scored (after the proven bound); false (and
    /// the pass aborted) when a deck search was cut short or the budget ran out
    fn band_shared(&self, sc: &mut SharedScan, lv: f64) -> bool {
        let (model, space, tally, f) = (self.model, self.space, &self.tally, sc.f);
        let mut complete = true;
        tally.n_band.set(tally.n_band.get() + 1);
        let t0 = Instant::now();
        let n0 = space.nodes.get();
        let SearchOutcome { decks: mut cands, incomplete } = space.search(
            model,
            members(&f),
            &sc.ids,
            &[(sc.pairs.clone(), sc.need())],
            SearchMode::Band { ratio_cap: lv, max_decks: space.band_cap() },
        );
        tally.spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
        tally.t_band.set(tally.t_band.get() + t0.elapsed().as_secs_f64());
        if incomplete.is_some() {
            tally.cap_hit.set(true);
            complete = false;
        }
        cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        let mut seen: crate::FxSet<(Vec<usize>, usize, usize)> = crate::FxSet::default();
        if complete {
            for (v, sel) in &cands {
                let thr = sc.need();
                if lv * v * (1.0 + 1e-9) < thr {
                    break;
                }
                if tally.work.get() > tally.budget.get() {
                    complete = false;
                    break;
                }
                if space.sel_cap(sel, lv) * v * (1.0 + 1e-9) < thr {
                    continue;
                }
                if !seen.insert((sel.ents.clone(), sel.plain, sel.filler)) {
                    continue;
                }
                let e: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                let t1 = Instant::now();
                let ub: Option<f64> = (0..sc.nm)
                    .map(|t| upper_na(model, f[t], &sc.qt[t], &e, sel.filler).map(|b| b.iter().sum::<f64>()))
                    .sum();
                tally.t_ub.set(tally.t_ub.get() + t1.elapsed().as_secs_f64());
                if let Some(ub) = ub {
                    tally.spend(100);
                    if ub * (1.0 + 1e-9) < thr {
                        tally.n_ufilt.set(tally.n_ufilt.get() + 1);
                        continue;
                    }
                }
                let x = space.pdeck(sel, &sc.ids);
                self.score_shared(sc, &x);
            }
        }
        if !complete {
            tally.aborted.set(true);
        }
        complete
    }

    // ---- the loop search ----

    /// The structures whose member sum (in four-member units) is in [lo, hi), with their product ceilings
    fn band(&self, lo: f64, hi: f64, out: &mut Vec<(f64, [usize; 4])>) {
        if self.with4 {
            self.band4(lo, hi, out);
        }
        if self.with3 && self.req.len() <= 3 {
            self.band3(lo, hi, out);
        }
    }
    fn band4(&self, lo: f64, hi: f64, out: &mut Vec<(f64, [usize; 4])>) {
        let (mv, ord, k) = (&self.mv, &self.ord, self.pool.len());
        for a in 0..k {
            if a + 3 >= k || mv[a] + mv[a + 1] + mv[a + 2] + mv[a + 3] < lo {
                break;
            }
            for b in a + 1..k {
                if b + 2 >= k || mv[a] + mv[b] + mv[b + 1] + mv[b + 2] < lo {
                    break;
                }
                for c in b + 1..k {
                    if c + 1 >= k || mv[a] + mv[b] + mv[c] + mv[c + 1] < lo {
                        break;
                    }
                    for e in c + 1..k {
                        let sm = mv[a] + mv[b] + mv[c] + mv[e];
                        if sm < lo {
                            break;
                        }
                        if sm >= hi {
                            continue;
                        }
                        let mut f = [ord[a], ord[b], ord[c], ord[e]];
                        f.sort_unstable();
                        if !self.req.iter().all(|r| f.contains(r)) {
                            continue;
                        }
                        let ln = lins(4);
                        let qs = self.model.lineage_q_arr(&f, &|x, y| self.pb[f[x]][f[y]]);
                        let ub = self.fixed * (0..12).map(|l| self.btab(f[ln[l].0], self.grid.idx(qs[l]))).sum::<f64>()
                            / 12.0
                            * (1.0 + 1e-12);
                        out.push((ub, f));
                    }
                }
            }
        }
    }
    fn band3(&self, lo: f64, hi: f64, out: &mut Vec<(f64, [usize; 4])>) {
        let (mv, ord, k) = (&self.mv, &self.ord, self.pool.len());
        for a in 0..k {
            if a + 2 >= k || e3(mv[a] + mv[a + 1] + mv[a + 2]) < lo {
                break;
            }
            for b in a + 1..k {
                if b + 1 >= k || e3(mv[a] + mv[b] + mv[b + 1]) < lo {
                    break;
                }
                for c in b + 1..k {
                    let sm = e3(mv[a] + mv[b] + mv[c]);
                    if sm < lo {
                        break;
                    }
                    if sm >= hi {
                        continue;
                    }
                    let mut t = [ord[a], ord[b], ord[c]];
                    t.sort_unstable();
                    let f = [t[0], t[1], t[2], NONE];
                    if !self.req.iter().all(|r| f.contains(r)) {
                        continue;
                    }
                    let ln = lins(3);
                    let qs = self.model.lineage_q_arr(&t, &|x, y| self.pb[t[x]][t[y]]);
                    let ub = self.fixed * (0..3).map(|l| self.btab(t[ln[l].0], self.grid.idx(qs[l]))).sum::<f64>()
                        / 3.0
                        * (1.0 + 1e-12);
                    out.push((ub, f));
                }
            }
        }
    }

    /// Try the caps highest first (EXACT_STEPS): the answer is certified under the first whose pass finishes within the
    /// budget; a lower cap's pass reuses everything computed so far and gets a quarter of the budget more. When none
    /// finishes, a pass with the cap at 1 (product odds: no deck searches) gives the answer. (the pass, its cap)
    fn certify(&self) -> (Pass, f64) {
        let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(EXACT_BUDGET);
        let steps: Vec<f64> = if !self.dj.exact {
            vec![]
        } else if let Ok(x) = std::env::var("EXACT_STEPS") {
            x.split(',').filter_map(|t| t.parse().ok()).collect()
        } else {
            EXACT_STEPS.iter().map(|&x| if x == EXACT_RHO { self.rho_top } else { x }).collect()
        };
        let mut last: Option<Pass> = None;
        let mut cert = 1.0f64;
        let mut allow = budget;
        for &lv in &steps {
            self.tally.budget.set(allow);
            if let Some(pass) = self.run_pass(lv) {
                last = Some(pass);
                cert = lv;
                break;
            } else {
                // Retain scored decks, but grant fresh work before trying a lower cap.
                self.tally.aborted.set(false);
                allow = self.tally.work.get() + budget / 4;
            }
        }
        let p = if let Some(p) = last {
            p
        } else {
            self.tally.budget.set(usize::MAX);
            self.run_pass(1.0).expect("a pass without deck searches")
        };
        (p, cert)
    }

    /// One best-first pass over the foursomes with cap lv: ceilings are exact tables or product ceilings times lv, the
    /// stop rule compares them with the k-th exact value. None when the budget runs out (the pass certifies nothing).
    fn run_pass(&self, lv: f64) -> Option<Pass> {
        let lo_sum = self.smax * 0.85;
        let mut cand: Vec<(f64, [usize; 4])> = vec![];
        self.band(lo_sum, f64::INFINITY, &mut cand);
        cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let mut p = Pass {
            solved: vec![],
            kth: f64::NEG_INFINITY,
            next_bound: 0.0,
            scanned: 0,
            generated: cand.len(),
            sched_rejects: 0,
            deck_best: crate::FxMap::default(),
            lists: vec![],
            vals: vec![],
        };
        let mut q = Queue { cand, next: 0, hi_sum: lo_sum };
        loop {
            if self.tally.aborted.get() {
                return None;
            }
            // Ceilings carry a (1 + 1e-12) guard against rounding, so a loop whose ceiling beats the k-th value by no
            // more than that only ties it and cannot change the result. Without this, a job whose targets are all
            // guaranteed (every loop has the same value) solved every foursome.
            let kth = p.kth;
            let beats = |c: f64| c > kth * (1.0 + 4e-12);
            if !(q.next < q.cand.len() && beats(lv * q.cand[q.next].0)) {
                if q.hi_sum <= 0.0 || !beats(lv * self.sep(q.hi_sum)) {
                    break;
                }
                p.generated += self.widen(&mut q, kth, lv);
                continue;
            }
            let (_, f) = q.cand[q.next];
            q.next += 1;
            p.scanned += 1;
            self.solve(f, lv, &mut p)?;
        }
        p.next_bound = lv
            * (if q.next < q.cand.len() { q.cand[q.next].0 } else { 0.0 }).max(if q.hi_sum > 0.0 {
                self.sep(q.hi_sum)
            } else {
                0.0
            });
        Some(p)
    }

    /// Adds the next band of structures below the queue's `hi_sum` (down to the sum the k-th value needs, at most 15%
    /// lower), merged into the unscanned rest in ceiling order; how many it added
    fn widen(&self, q: &mut Queue, kth: f64, lv: f64) -> usize {
        let kth_lo = if kth > f64::NEG_INFINITY { kth / lv * 12.0 / (3.0 * self.fixed) * (1.0 - 1e-9) } else { 0.0 };
        let lo = kth_lo.max(q.hi_sum * 0.85);
        let lo = if lo < self.smax * 1e-6 { 0.0 } else { lo };
        let mut more: Vec<(f64, [usize; 4])> = vec![];
        self.band(lo, q.hi_sum, &mut more);
        q.hi_sum = lo;
        let added = more.len();
        let mut rest: Vec<(f64, [usize; 4])> = q.cand.split_off(q.next);
        rest.extend(more);
        rest.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        q.cand.extend(rest);
        added
    }

    /// Structure f's turn in a pass: rejected by its ceilings (cheapest first), else scheduled and solved exactly into
    /// the pass. None when the budget runs out.
    fn solve(&self, f: [usize; 4], lv: f64, p: &mut Pass) -> Option<()> {
        let kth = p.kth;
        let beats = |c: f64| c > kth * (1.0 + 4e-12);
        let fm = members(&f);
        let nm = fm.len();
        let ln = lins(nm);
        let nl = ln.len();
        let qpb = self.model.lineage_q_arr(fm, &|x, y| self.pb[fm[x]][fm[y]]);
        // cheapest first: the cell bounds as they stand, then the product ceiling with one deck per trainee
        if !beats((0..nl).map(|l| self.cell(f[ln[l].0], self.grid.idx(qpb[l]), lv).0).sum::<f64>() / nl as f64) {
            return Some(());
        }
        if !beats(lv * self.relaxed(f, &qpb)) {
            return Some(());
        }
        if self.shared && !beats(lv * self.relaxed_shared(f, &qpb)) {
            return Some(());
        }
        // exact ceiling at the pairwise-ceiling chances, valid for any schedule
        if !beats(self.exact_ceiling(f, &qpb, kth, lv)) {
            return Some(());
        }
        if self.tally.aborted.get() {
            return None;
        }
        let r = self.schedule(f, nm);
        let d = self.d;
        let ps: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let qs = self.model.lineage_q(fm, &|x, y| if self.no_race { 0.0 } else { wval(&ps[x], &ps[y]) });
        let qa: [f64; 12] = std::array::from_fn(|i| qs.get(i).copied().unwrap_or(0.0));
        // rejected only with the schedule found: a better schedule could change this one's fate
        if (self.shared && !beats(lv * self.relaxed_shared(f, &qa))) || !beats(self.exact_ceiling(f, &qa, kth, lv)) {
            p.sched_rejects += 1;
            return Some(());
        }
        if self.shared {
            let sl =
                self.exact_shared(f, &qa[..nl], if kth.is_finite() { kth * nl as f64 } else { f64::NEG_INFINITY }, lv);
            if self.tally.aborted.get() {
                return None;
            }
            if sl.is_empty() {
                return Some(());
            }
            let val = sl[0].0 / nl as f64;
            for (v, x) in sl.iter() {
                let id = (x.ids.clone(), x.ends.clone(), x.vars.clone(), x.plain_ids.len());
                let e = p.deck_best.entry(id).or_insert((f64::NEG_INFINITY, 0, 0));
                if v / nl as f64 > e.0 {
                    *e = (v / nl as f64, p.solved.len(), p.lists.len());
                }
            }
            p.solved.push(Solved { f, r, qs, bests: vec![], val });
            p.lists.push(sl);
            let mut vs: Vec<f64> = p.deck_best.values().map(|x| x.0).collect();
            vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
            p.kth = if vs.len() >= self.kloops { vs[self.kloops - 1] } else { f64::NEG_INFINITY };
            return Some(());
        }
        let mut tot = 0.0;
        let mut bests = vec![];
        for t in 0..nm {
            let qt: Vec<f64> = (0..nl).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect();
            let b = self.exact_best(f[t], &qt, lv);
            tot += b.sum;
            bests.push(b);
        }
        if self.tally.aborted.get() {
            return None;
        }
        let val = tot / nl as f64;
        p.solved.push(Solved { f, r, qs, bests, val });
        p.vals.push(val);
        let mut vs = p.vals.clone();
        vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
        p.kth = if vs.len() >= self.kloops { vs[self.kloops - 1] } else { f64::NEG_INFINITY };
        Some(())
    }

    /// The schedule of structure f: the job's fixed ones, else the optimiser's (kept for later passes)
    fn schedule(&self, f: [usize; 4], nm: usize) -> Res {
        let r = self.sched_c.borrow().get(&f).cloned();
        r.unwrap_or_else(|| {
            let r = match &self.fixed_s {
                Some(fs) => {
                    Res { s: std::array::from_fn(|i| if i < nm { fs[f[i]].unwrap() } else { [Choice::Rest; NT] }) }
                }
                None => optimize_fam(self.d, &f, self.sj.iters, self.sj.seed, 60.0),
            };
            self.sched_c.borrow_mut().insert(f, r.clone());
            r
        })
    }

    // ---- the answer ----

    fn debug_summary(&self, p: &Pass, cert: f64) {
        if std::env::var("UPPER_CHECK").is_ok() {
            let v = UCHK.with(Cell::get);
            eprintln!("UPPER_CHECK: {} checked, max bound/exact {:.4}, {} decks not covered", v.0, v.1, v.2);
        }
        if std::env::var("DECK_DEBUG").is_ok() {
            let v = SPT.with(Cell::get);
            eprintln!("deck searches: setup {:.2}s tree {:.2}s configs {} count vectors {}", v.0, v.1, v.2, v.3);
        }
        if std::env::var("DECK_DEBUG").is_ok() {
            let t = &self.tally;
            eprintln!(
                "per-uma search {:.2}s cap {} scanned {} solved {} exact: {} deck searches, {} decks scored, {} bounded out, schedule rejects {} | band {:.2}s score {:.2}s seed-scan {:.2}s bound {:.2}s",
                self.tq.elapsed().as_secs_f64(),
                cert,
                p.scanned,
                p.solved.len(),
                t.n_band.get(),
                t.n_scored.get(),
                t.n_ufilt.get(),
                p.sched_rejects,
                t.t_band.get(),
                t.t_score.get(),
                t.t_seed.get(),
                t.t_ub.get()
            );
        }
    }

    /// The output: the reported loops, the search's counts and what the answer rests on, over the deck search's stats
    fn report(&self, p: Pass, cert: f64, stats: Value) -> Value {
        let empty_deck = PDeck {
            names: vec![],
            ids: vec![],
            ends: vec![],
            vars: vec![],
            choices: vec![],
            plain_ids: vec![],
            pal_names: vec![],
            pals: 0,
            borrowed: None,
            dw: self.zero.clone(),
            dg: self.zero.clone(),
        };
        // what is reported: per-uma, the best loops; shared, the best distinct decks, each with its best loop
        let picks = if self.shared { self.shared_picks(&p) } else { self.per_uma_picks(&p, &empty_deck) };
        let top: Vec<Value> = picks.iter().map(|pk| self.loop_json(&p, pk)).collect();
        let budget_out = self.dj.exact && cert < self.rho_top;
        let complete = p.next_bound <= p.kth * (1.0 + 4e-12);
        let exact = self.exact_json(&p, cert, complete, budget_out);
        let n_band = self.tally.n_band.get();
        let mut out = stats;
        for (key, val) in [
            ("top", json!(top)),
            ("best", json!([])),
            ("per_uma", json!(!self.shared)),
            ("foursomes", json!(self.total_foursomes())),
            ("foursomes_ranked", json!(p.generated)),
            ("foursomes_scanned", json!(p.scanned)),
            ("foursomes_solved", json!(p.solved.len())),
            ("next_bound", json!(p.next_bound)),
            ("proven", json!(complete)),
            ("searches", json!(1 + n_band)),
            ("exact", exact),
        ] {
            out[key] = val;
        }
        out
    }

    /// The structures the job's sizes and required umas allow
    fn total_foursomes(&self) -> usize {
        let (k, nreq) = (self.pool.len(), self.req.len());
        let choose = |m: usize, c: usize| (0..c).fold(1usize, |acc, i| acc * (m - i) / (i + 1));
        let m = k - nreq.min(k);
        (if self.with4 { choose(m, 4usize.saturating_sub(nreq)) } else { 0 })
            + (if self.with3 && nreq <= 3 { choose(m, 3 - nreq) } else { 0 })
    }

    /// shared: the best `top_decks` distinct decks (ties by identity), each with the loop it scored best in
    fn shared_picks<'pass>(&self, p: &'pass Pass) -> Vec<Pick<'pass>> {
        let mut db: Vec<(&DeckKey, &(f64, usize, usize))> = p.deck_best.iter().collect();
        db.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap().then(a.0.cmp(b.0)));
        db.iter()
            .take(self.kloops)
            .map(|&(id, &(v, si, li))| {
                let x = &p.lists[li]
                    .iter()
                    .find(|y| y.1.ids == id.0 && y.1.ends == id.1 && y.1.vars == id.2 && y.1.plain_ids.len() == id.3)
                    .unwrap()
                    .1;
                let Solved { f, qs, .. } = &p.solved[si];
                let (e, fl) = self.ents_of(x);
                let ln = lin_of(f);
                let nm = members(f).len();
                let mut per_l = vec![(0.0, 0.0, 0usize); ln.len()];
                for t in 0..nm {
                    let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].0 == t).collect();
                    let qt: Vec<f64> = ls.iter().map(|&l| qs[l]).collect();
                    let j = joint_b(self.model, f[t], &qt, &e, fl, self.ntar, true);
                    for (i, &l) in ls.iter().enumerate() {
                        per_l[l] = j[i];
                    }
                }
                Pick { si, decks: vec![x; nm], per_l, value: v }
            })
            .collect()
    }

    /// per-uma: the best `top_decks` loops (ties in solve order), each member with her best deck
    fn per_uma_picks<'pass>(&self, p: &'pass Pass, empty_deck: &'pass PDeck) -> Vec<Pick<'pass>> {
        let solved = &p.solved;
        let mut order: Vec<usize> = (0..solved.len()).collect();
        // stable: ties keep solve order
        order.sort_by(|&a, &b| solved[b].val.partial_cmp(&solved[a].val).unwrap());
        order
            .iter()
            .take(self.kloops)
            .map(|&si| {
                let bests = &solved[si].bests;
                let ln = lin_of(&solved[si].f);
                let nm = members(&solved[si].f).len();
                let mut per_l = vec![(0.0, 0.0, 0usize); ln.len()];
                for t in 0..nm {
                    let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].0 == t).collect();
                    for (i, &l) in ls.iter().enumerate() {
                        per_l[l] = bests[t].per[i];
                    }
                }
                let decks = bests.iter().map(|b| b.deck.as_ref().unwrap_or(empty_deck)).collect();
                Pick { si, decks, per_l, value: solved[si].val }
            })
            .collect()
    }

    /// A reported loop: its members, odds, outfits, schedule and detail, with its deck (shared) or decks (per-uma)
    fn loop_json(&self, p: &Pass, pk: &Pick) -> Value {
        let (d, model, unc, ntar) = (self.d, self.model, &self.unc, self.ntar);
        let (dk, per_l, pj) = (&pk.decks, &pk.per_l, pk.value);
        let Solved { f, r, qs, .. } = &p.solved[pk.si];
        let ln = lin_of(f);
        let f = members(f);
        let nm = f.len();
        let nl = ln.len();
        let mut per = vec![(0.0, 0usize); nm];
        let mut picks: Vec<Vec<usize>> = vec![vec![]; nm];
        for (l, &(t, ..)) in ln.iter().enumerate() {
            per[t].0 += per_l[l].0;
            per[t].1 += 1;
            picks[t].push(per_l[l].2);
        }
        let minus1 = per_l.iter().map(|x| x.1).sum::<f64>() / nl as f64;
        // the chance of exactly k targets, k = 0 ..= n (exact over each lineage's landed sets, as `p_all`), averaged over
        // the lineages: the uncertain targets' sets from the joint model, each guaranteed gold on its own (GEN_GOLD)
        let mut landed = vec![0.0f64; ntar + 1];
        // (each member's deck's sources once, read by each of her lineages)
        let dke: Vec<_> = (0..nm).map(|t| self.ents_of(dk[t])).collect();
        for l in 0..nl {
            let t = ln[l].0;
            let (e, fl) = (&dke[t].0, dke[t].1);
            let nu = unc.len();
            let mut ext = JointExt {
                su: vec![1.0; nu],
                sf: vec![1.0; ntar - nu],
                marg: vec![],
                sets: vec![0.0],
                per_outfit: vec![],
            };
            joint_fast(model, f[t], &[qs[l]], e, fl, ntar, false, Some(&mut ext));
            let o = per_l[l].2;
            let mut g = vec![1.0f64];
            for _ in nu..ntar {
                let mut nn = vec![0.0; g.len() + 1];
                for (i, x) in g.iter().enumerate() {
                    nn[i] += x * (1.0 - GEN_GOLD);
                    nn[i + 1] += x * GEN_GOLD;
                }
                g = nn;
            }
            for (mu, &x) in ext.sets[(o << nu)..((o + 1) << nu)].iter().enumerate() {
                if x == 0.0 {
                    continue;
                }
                for (j, &y) in g.iter().enumerate() {
                    landed[mu.count_ones() as usize + j] += x * y / nl as f64;
                }
            }
        }
        // the search's product value of this loop with these decks
        let rank_v = self.fixed
            * (0..nl).map(|l| model.trainee_fast(f[ln[l].0], qs[l], &dk[ln[l].0].dw, &dk[ln[l].0].dg)).sum::<f64>()
            / nl as f64;
        let outfits: Vec<Value> = (0..nm)
            .map(|i| {
                let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
                let (st, cs) = &model.opts[f[i]][kk];
                let covers: Vec<Value> = st
                    .iter()
                    .enumerate()
                    .filter(|(_, lv)| **lv > 0)
                    .map(|(j, lv)| json!({"group": unc[j], "level": lv}))
                    .collect();
                let outfits = if covers.is_empty() {
                    vec![]
                } else {
                    cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>()
                };
                json!({"outfits": outfits, "covers": covers})
            })
            .collect();
        let pss: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let dks: Vec<(&[f64], &[f64])> = (0..nm).map(|t| (&*dk[t].dw, &*dk[t].dg)).collect();
        let mut detail = model.detail(f, &|a, b| if self.no_race { 0.0 } else { wval(&pss[a], &pss[b]) }, Some(&dks));
        if let Some(ls) = detail["lineages"].as_array_mut() {
            for (l, x) in ls.iter_mut().enumerate() {
                x["rank_value"] = x["p_all"].clone();
                x["p_all"] = json!(per_l[l].0);
            }
        }
        let lp = json!({"members": member_json(d, f), "p_all": pj, "p_minus1": minus1, "landed": landed,
               "rank_value": rank_v, "outfits": outfits,
               "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
               "mean_proc": qs.iter().sum::<f64>() / qs.len() as f64, "schedule": chart_rows(d, f, &r.s[..nm]),
               "detail": detail});
        if self.shared {
            return json!({"loop": lp, "deck": dk[0].out(unc)});
        }
        // the deck most members share, for displays that show one deck
        let base = (0..nm).max_by_key(|&i| (dk.iter().filter(|x| x.names == dk[i].names).count(), nm - i)).unwrap();
        json!({"loop": lp, "deck": dk[base].out(unc), "decks": dk.iter().map(|x| x.out(unc)).collect::<Vec<_>>()})
    }

    /// What the answer rests on: `rho` the highest cap certified (`rho_target` the one sought), `class_caps` the
    /// assumed cap per structure class (each certified at most at rho), `class_max_ratio` the largest exact / product
    /// odds seen per class on any deck scored here (`max_ratio` of them all); `complete` = the search ended by its stop
    /// rule (no unexamined loop or deck can win under the certified caps)
    fn exact_json(&self, p: &Pass, cert: f64, complete: bool, budget_out: bool) -> Value {
        let t = &self.tally;
        let mr = t.max_ratio.get();
        let max_deck: Vec<Option<Value>> =
            t.max_deck.borrow().iter().map(|x| x.as_ref().map(|(u, n)| json!({"trainee": u, "cards": n}))).collect();
        json!({"rho": cert, "rho_target": self.rho_top, "class_caps": self.caps,
               "max_ratio": mr.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": mr,
               "class_max_deck": max_deck, "complete": complete, "decks_scored": t.n_scored.get(),
               "deck_searches": t.n_band.get(), "loops_solved": p.solved.len(), "schedule_rejects": p.sched_rejects,
               "cap_hit": t.cap_hit.get(), "budget_out": budget_out, "bound_filtered": t.n_ufilt.get()})
    }
}

/// (bound, cluster) ordered as the stable sort by bound, highest first, visits them: a heap's pops give the higher bound
/// first and equal bounds (`partial_cmp`: 0 and -0 tie) in index order, the same sequence (a NaN bound panics in
/// either)
struct ByBound {
    bound: f64,
    cluster: usize,
}
impl PartialEq for ByBound {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == std::cmp::Ordering::Equal
    }
}
impl Eq for ByBound {}
impl PartialOrd for ByBound {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for ByBound {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bound
            .partial_cmp(&other.bound)
            .unwrap()
            // BinaryHeap pops the greatest item: reverse indices so earlier clusters win ties.
            .then(other.cluster.cmp(&self.cluster))
    }
}
fn qkey(qs: &[f64]) -> QKey {
    assert!(qs.len() <= 12, "more than 12 chances in a memo key");
    let mut k = [0u64; 12];
    for (x, q) in k.iter_mut().zip(qs) {
        *x = q.to_bits();
    }
    (k, qs.len() as u8)
}
