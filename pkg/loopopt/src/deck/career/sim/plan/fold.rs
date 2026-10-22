//! Folding spark outcomes and their rises.
use super::{EstBufs, Leaves};
use crate::deck::career::sim::plan::TargetMask;
use crate::deck::career::sim::rise_factor;

/// `score_career`'s fold buffers, kept between calls (each call writes what it reads): per level the folded tables, the
/// last two levels' 16 leaves per setup and per candidate slot, the outcome rows, per leaf her stars' chance and set
/// (`pkey`: what pst and mus were built for, every leaf: the star chances' bits, then the uncertain targets; empty: not
/// built for every leaf. `est`: `Est`'s two direct-mapped caches, refilled per call)
#[derive(Default)]
pub(in crate::deck::career::sim::plan) struct FoldScratch {
    pub(in crate::deck::career::sim::plan) lv: Vec<Vec<f64>>,
    pub(in crate::deck::career::sim::plan) fv: Vec<f64>,
    pub(in crate::deck::career::sim::plan) acc: Vec<f64>,
    pub(in crate::deck::career::sim::plan) u: Vec<f64>,
    pub(in crate::deck::career::sim::plan) pst: Vec<f64>,
    pub(in crate::deck::career::sim::plan) mus: Vec<usize>,
    pub(in crate::deck::career::sim::plan) pkey: Vec<u64>,
    pub(in crate::deck::career::sim::plan) yb: Vec<f64>,
    pub(in crate::deck::career::sim::plan) lb: Vec<u32>,
    pub(in crate::deck::career::sim::plan) lf: Vec<(usize, f64)>,
    pub(in crate::deck::career::sim::plan) est: EstBufs,
}

thread_local! { pub(in crate::deck::career::sim::plan) static FOLD_SCRATCH: std::cell::RefCell<FoldScratch> = std::cell::RefCell::new(FoldScratch::default()); }

/// What `fold_rec` reads: the kept tables (level 0, `t0`) and each one's setup, per setup x and 1 - x per uncertain
/// target and star level; for `fold_emit`: whether it writes per leaf the setups' table values (pl: fewer rows than per
/// outcome and slot; the outcome loop then takes the products and maxima itself), else per outcome the slots' odds; the
/// guaranteed golds' sets, the spark scales (m-major), each setup's candidate slot, the slots' odds now (`col`, `two`:
/// per target whether it is collapsed, and then the star levels folded, 0 and 1: `sc_prep`); the leaves produced (`lv`)
pub(in crate::deck::career::sim::plan) struct FoldCx<'fold, L: Leaves> {
    pub(in crate::deck::career::sim::plan) pl: bool,
    pub(in crate::deck::career::sim::plan) nu: usize,
    pub(in crate::deck::career::sim::plan) nf: usize,
    pub(in crate::deck::career::sim::plan) ns: usize,
    pub(in crate::deck::career::sim::plan) na: usize,
    pub(in crate::deck::career::sim::plan) t0: &'fold [f64],
    pub(in crate::deck::career::sim::plan) tsi: &'fold [usize],
    pub(in crate::deck::career::sim::plan) xs: &'fold [Vec<[f64; 4]>],
    pub(in crate::deck::career::sim::plan) xc: &'fold [Vec<[f64; 4]>],
    pub(in crate::deck::career::sim::plan) uncv: &'fold [usize],
    pub(in crate::deck::career::sim::plan) fmm: &'fold [TargetMask],
    pub(in crate::deck::career::sim::plan) scf: &'fold [f64],
    pub(in crate::deck::career::sim::plan) sslot: &'fold [usize],
    pub(in crate::deck::career::sim::plan) p0: &'fold [f64],
    pub(in crate::deck::career::sim::plan) col: &'fold [bool],
    pub(in crate::deck::career::sim::plan) two: [usize; 2],
    pub(in crate::deck::career::sim::plan) lv: &'fold L,
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
        let mf = mf.bits();
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
        let mf = mf.bits();
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
pub(in crate::deck::career::sim::plan) fn fold_rec<L: Leaves>(
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
pub(in crate::deck::career::sim::plan) fn leaf_set(leaf: usize, uncv: &[usize], w: [f64; 3]) -> (usize, f64) {
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
pub(in crate::deck::career::sim::plan) fn leaf_products(g: &[[f64; 4]], out: &mut Vec<f64>, tmp: &mut Vec<f64>) {
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
pub(in crate::deck::career::sim::plan) fn rise_leaves(
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
