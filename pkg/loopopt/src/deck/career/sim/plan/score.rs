//! Career scoring over the possible veterans and their continuations.
use super::{
    All, Est, EstBufs, FOLD_SCRATCH, FoldCx, FoldScratch, IDX_PARTNERS, Leaves, One, OneLeaf, ScoreBase, Scored, Setup,
    Want, fold_rec, is_full, leaf_set, legal_pair, one_of, trio_v, v_after, val,
};
use crate::deck::career::sim::plan::cont::OwnTrio;
use crate::deck::career::sim::plan::roll::ActiveCandidate;
use crate::deck::career::sim::plan::{CandidateIndex, OutcomeIndex, PoolProvenance, ScoringContext, TargetMask};
use crate::deck::career::sim::{Cur, Env, Legacy, Pool, Sim, now_ms, prof};
use crate::deck::career::{Career, Lineage};

/// A career's own 8/8 as a trio side when her uma is not a stage candidate (her outcome lands every target): the replan
/// would take her uma in (a veteran of hers) with that real 8/8 a free side. Per candidate side (real or fresh) her 8/8 can
/// pair with, C's step: 1 / the best allowed C's value with the two 8/8s as parents (every allowed uma but hers and the
/// partner's, best ceiling first, the value taken until no ceiling can beat the best: the best C by value, as `combos_of`
/// finds one). Her 8/8 is `lv[iv]` (`Env::veteran`: every target at the rank's star mix), valued as every fresh side is
/// (the trios' own 8/8s use the rank's mix too), so the trios with her compare like for like with the others; one per
/// call, not per star pattern (the full set's stars vary per leaf: per leaf it would be 4^nu searches for C).
/// (B's candidate index, partner side real, C's step)
pub(in crate::deck::career::sim::plan) fn her_trios(
    s: &mut Sim,
    bs: &ScoreBase,
    p8: &mut Pool,
    iv: usize,
    x: usize,
    buf: &mut Vec<f64>,
) -> Vec<OwnTrio> {
    let d = s.d;
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
                        d.umas[cu].id != her_id
                            && d.umas[cu].id != d.umas[pu].id
                            && legal_pair(d, &*p8, Lineage { trainee: cu, first_parent: iv, second_parent: il })
                    })
                    .filter_map(|cu| {
                        let c = s.env.lineage_into(
                            &*p8,
                            Lineage { trainee: cu, first_parent: iv, second_parent: il },
                            &mut q,
                        );
                        s.env.ceiling(cu, &q, c).map(|ub| (ub, cu))
                    }),
            );
            cs.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut best = 0.0f64;
            for &(ub, cu) in &cs {
                if ub * (1.0 + 1e-9) <= best {
                    break;
                }
                let v = val(s, &*p8, Lineage { trainee: cu, first_parent: iv, second_parent: il }, buf);
                if v > best {
                    best = v;
                }
            }
            if best > 0.0 {
                out.push(OwnTrio { candidate: CandidateIndex(ib), uses_existing: *real, finisher_cost: 1.0 / best });
            }
            Env::pool_pop(p8);
        }
    }
    out
}

/// V after her with her own 8/8 a trio side (`her_trios`), at the candidates' step costs c1 (a real partner side free)
pub(in crate::deck::career::sim::plan) fn with_her(v: f64, her: &[OwnTrio], c1: &[f64]) -> f64 {
    her.iter()
        .map(|&OwnTrio { candidate: CandidateIndex(ib), uses_existing: real, finisher_cost: c3 }| {
            (if real { 0.0 } else { c1[ib] }) + c3
        })
        .fold(v, f64::min)
}

/// Career x's place among the stage candidates (`xi`) and whether its parents a, b are both 8/8s (the goal: her own 8/8 a
/// fully looped parent, V after her 0): what `score_career`, `outcome_hi` and `pick_her` read of her first
pub(in crate::deck::career::sim::plan) fn her_ends(
    s: &Sim,
    bs: &ScoreBase,
    legs: &[Legacy],
    lineage: Lineage,
) -> (Option<usize>, bool) {
    let Lineage { trainee: x, first_parent: a, second_parent: b } = lineage;
    (bs.cands.iter().position(|&u| u == x), is_full(s.env.sj, &legs[a].sparks) && is_full(s.env.sj, &legs[b].sparks))
}

/// Per mask fm of the scenario-guaranteed targets landed (bit j: `fixed_t[j]`), its landed set over the job targets: an
/// outcome's cell is leaf << nf | fm, its landed set the leaf's (`leaf_set`) | this (`score_career`, `outcome_hi`)
pub(in crate::deck::career::sim::plan) fn fm_masks(fixed_t: &[usize]) -> Vec<TargetMask> {
    (0..(1usize << fixed_t.len()))
        .map(|bits| {
            let fixed = TargetMask::from_bits(bits);
            let mut landed = TargetMask::default();
            for bit in fixed.iter() {
                landed.insert(fixed_t[bit]);
            }
            landed
        })
        .collect()
}

/// `Env::pool` of `legs` and her veteran (career x with parents a, b, schedule `c`): with `of_legs` (`pool` is
/// `Env::pool(legs)`) `pool` with her entry pushed, the same Pool (`Env::pool_push` builds an entry as `Env::pool` does)
pub(in crate::deck::career::sim::plan) fn her_pool(
    s: &mut Sim,
    legs: &[Legacy],
    pool: &Pool,
    of_legs: bool,
    lineage: Lineage,
    c: &Cur,
) -> Pool {
    let Lineage { trainee: x, first_parent: a, second_parent: b } = lineage;
    let veteran = s.env.veteran(legs, Lineage { trainee: x, first_parent: a, second_parent: b }, c.wp.to_vec());
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
pub(in crate::deck::career::sim::plan) fn her_pool_done(s: &mut Sim, pool: &Pool, of_legs: bool, mut pv: Pool) {
    if of_legs {
        Env::pool_pop(&mut pv);
        s.put_pool(pool, pv);
    }
}

/// Prepared scoring retained for the chosen career's later rolls.
pub(in crate::deck::career::sim::plan) struct KeptScoring {
    pub(in crate::deck::career::sim::plan) scored: Scored,
    /// At least one all-targets-landed outcome has a saving below 0.5; used when preparing the roll bound.
    pub(in crate::deck::career::sim::plan) full_landing_below_threshold: bool,
}

/// Named outputs of career scoring. `Want` determines which fields are computed; Stats leaves the two
/// scalar fields at zero, Score leaves `lift_stats` empty, Touch returns zeros everywhere (no evaluation), and only Keep
/// retains the scoring tables.
pub(in crate::deck::career::sim) struct CareerScore {
    /// Expected reduction in remaining careers, before subtracting the current career's cost.
    pub(in crate::deck::career::sim::plan) expected_saving: f64,
    pub(in crate::deck::career::sim::plan) improvement_chance: f64,
    pub(in crate::deck::career::sim::plan) kept: Option<KeptScoring>,
    /// Per candidate: probability of improvement and probability-weighted improved odds.
    pub(in crate::deck::career::sim) lift_stats: Vec<(f64, f64)>,
}

/// Score of career x (parents a, b of `legs`, deck `sel`) against `bs`: E[V0 - V0 with her veteran], the chance it is
/// above 0, the scoring kept for a roll (`Want::Keep`: `Scored`, with whether an outcome landing every target reads y < 0.5), and per candidate (P(her veteran lifts her), the summed lifted odds): what `want` asks for (`Want`). Exact over her landed set (`outcome_sets`) and her stars per uncertain target (the
/// rank's mix): her veteran as a parent of each stage-1 candidate (her best IDX_PARTNERS partners, the trainee's cached
/// decks' corner tables, folded one target at a time); her own 8/8 ends her stage when she is a candidate.
///
/// `pool` must be `Env::pool(legs)` (the planner's callers): the pool with her veteran is then that pool with her entry
/// pushed (`Env::pool_push` builds an entry as `Env::pool` does). `score_career_of` takes any pool.
pub(in crate::deck::career::sim) fn score_career(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = score_career_(s, context, career, want, PoolProvenance::RecordedLegacies, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}

/// `score_career` for a pool built otherwise (`sim`'s): the pool with her veteran built from `legs` and her
pub(in crate::deck::career::sim) fn score_career_of(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let started = now_ms();
    let score = score_career_(s, context, career, want, PoolProvenance::Extended, buf);
    prof("score_ms", now_ms() - started);
    prof("score_calls", 1.0);
    score
}

fn score_career_(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    want: Want,
    provenance: PoolProvenance,
    buf: &mut Vec<f64>,
) -> CareerScore {
    let d = s.d;
    let x = career.lineage.trainee;
    let of_legs = provenance == PoolProvenance::RecordedLegacies;

    let n = context.score_base.cands.len();
    let ntar = s.env.ntar;
    let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone();
    let fixed_t = s.env.fixed_t.clone();
    let w = s.env.sj.star_dist;
    let (xi, goal) = her_ends(s, context.score_base, context.legacies, career.lineage);
    let t_mc = now_ms();
    let c = s.make_cur(context.pool, career, false);
    let t_lv = now_ms();
    prof("sc_cur_ms", t_lv - t_mc);
    let mut pv = her_pool(s, context.legacies, context.pool, of_legs, career.lineage, &c);
    let iv = pv.me.len() - 1;
    // where she can be a parent: per stage-1 candidate (stage 1 still to run) her best partners
    let t_su = now_ms();
    prof("sc_pool_ms", t_su - t_lv);
    let mut setups: Vec<Setup> = vec![];
    for ui in 0..n {
        let u = context.score_base.cands[ui];
        if d.umas[u].id == d.umas[x].id || context.score_base.c1[ui] == 0.0 || !context.score_base.c1[ui].is_finite() {
            continue;
        }
        // her best IDX_PARTNERS (2) partners by value, ties in partner order: every legal partner valued in order (each
        // `val` as before), the two kept by strict comparisons, as the stable sort's first two
        const _: () = assert!(IDX_PARTNERS == 2, "the partner pick below keeps exactly two");
        let mut ps: [Option<(f64, usize)>; 2] = [None, None];
        for y in 0..iv {
            if !legal_pair(d, &pv, Lineage { trainee: u, first_parent: y, second_parent: iv }) {
                continue;
            }
            let v = val(s, &pv, Lineage { trainee: u, first_parent: y, second_parent: iv }, buf);
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
            let Some(hs) = s.env.her_slot(&pv, Lineage { trainee: u, first_parent: y, second_parent: iv }) else {
                continue;
            };
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
            p0.push(context.score_base.pnow[st.ui]);
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
        sc_prep(s, context.score_base, &setups, &mut sd);
    }
    prof("sc_fold_ms", now_ms() - t_fo);
    // the candidates the loop below can lift, in candidate order: those with setups, and (never, as the odds are not
    // negative) one whose odds now already clear her threshold pnow (1 + 1e-9); every other keeps up = pnow, below it
    let thr: Vec<f64> = (0..n).map(|ui| context.score_base.pnow[ui] * (1.0 + 1e-9)).collect();
    let active: Vec<usize> =
        (0..n).filter(|&ui| aslot[ui].is_some() || context.score_base.pnow[ui] > thr[ui]).collect();
    // her own 8/8 as a trio side when her uma is not a stage candidate (the full set only; `her_trios`)
    let full_m = (1usize << ntar) - 1;
    let t_h = now_ms();
    sd.her = if xi.is_none() && !goal && !context.score_base.sides.is_empty() && c.os[full_m] > 0.0 {
        her_trios(s, context.score_base, &mut pv, iv, x, buf)
    } else {
        vec![]
    };
    prof("sc_her_ms", now_ms() - t_h);
    // per active candidate: (index, her slot or none, her threshold, her odds now)
    sd.act = active
        .iter()
        .map(|&ui| ActiveCandidate {
            candidate: CandidateIndex(ui),
            slot: aslot[ui],
            threshold: thr[ui],
            current_chance: context.score_base.pnow[ui],
        })
        .collect();
    her_pool_done(s, context.pool, of_legs, pv);
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
        sc_eval(context.score_base, &sd, &All, if want == Want::Keep { Want::Score } else { want }, &mut sc);
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
pub(in crate::deck::career::sim::plan) fn sc_roll(bs: &ScoreBase, sd: &Scored, outcome: OutcomeIndex) -> f64 {
    let leaf = outcome.leaf(sd.nf);
    let fm = outcome.fixed_mask(sd.nf).bits();
    let lv = OneLeaf::new(leaf, sd.rep_of(leaf));
    let mut sc = FOLD_SCRATCH.with(|c| std::mem::take(&mut *c.borrow_mut()));
    let (_, _, ys, _, _, _) = sc_eval(bs, sd, &lv, Want::Ys, &mut sc);
    FOLD_SCRATCH.with(|c| *c.borrow_mut() = sc);
    ys[(lv.row(leaf) << sd.nf) | fm]
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
    for &ActiveCandidate { candidate: CandidateIndex(ui), slot: sl, threshold: th, current_chance: pn } in &sd.act {
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
    let lazy =
        bs.c1.iter().all(|&c| ok(c)) && bs.m23.iter().all(|&c| ok(c)) && bs.combos.iter().all(|t| ok(t.finisher_cost));
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
        full_m: sd.full_m().bits(),
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
    fmm: &'out [TargetMask],
    u: &'out [f64],
    scf: &'out [f64],
    rg: &'out [usize],
    p0: &'out [f64],
    uis: &'out [usize],
    ths: &'out [f64],
    actn: &'out [(usize, f64, f64)],
    her: &'out [OwnTrio],
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
            let m = mu | mf.bits();
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
pub(in crate::deck::career::sim::plan) fn v0_after(bs: &ScoreBase, c1: &[f64]) -> f64 {
    if bs.combos.is_empty() {
        (0..bs.cands.len()).map(|a| c1[a] + bs.m23[a]).fold(f64::INFINITY, f64::min)
    } else {
        trio_v(c1, bs.combos)
    }
}
