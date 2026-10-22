//! Anchor bounds and the best partners after an outcome.
use super::{
    Lo, ScoreBase, fm_masks, her_ends, her_pool, her_pool_done, her_trios, leaf_products, leaf_set, legal_pair,
    rise_leaves, v_after,
};
use crate::deck::career::sim::plan::cont::OwnTrio;
use crate::deck::career::sim::plan::{CandidateIndex, OutcomeIndex, ScoringContext};
use crate::deck::career::sim::{
    Data, DeckRes, Env, HerSlot, Legacy, Pool, Sim, Solved, holds_fix, now_ms, prof, rise_factor,
};
use crate::deck::career::{Career, Lineage};

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
    her: &[OwnTrio],
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
    her: &[OwnTrio],
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
pub(in crate::deck::career::sim::plan) struct PartnerAnchors {
    /// Candidate whose odds these anchors bound.
    pub(in crate::deck::career::sim::plan) candidate: CandidateIndex,
    /// Partner's index in the extended legacy pool.
    pub(in crate::deck::career::sim::plan) partner: usize,
    /// Retained (factor, inheritance chances) anchors, in their original order.
    pub(in crate::deck::career::sim::plan) anchors: Option<Vec<(f64, Vec<f64>)>>,
}

/// A fully checked career used as an exact anchor for its deck class.
pub(in crate::deck::career::sim::plan) struct FoundAnchor {
    /// Deck class shared by trainees with the same legal deck search.
    pub(in crate::deck::career::sim::plan) deck_class: usize,
    /// Inheritance chances at the exact check, in job target order.
    pub(in crate::deck::career::sim::plan) inheritance_chances: Vec<f64>,
    /// Best deck value at those chances and unit spark scale.
    pub(in crate::deck::career::sim::plan) value: f64,
}

/// Extra immutable inputs to the bound on a career's outcomes.
pub(in crate::deck::career::sim::plan) struct OutcomeBounds<'bounds> {
    /// Parent indices required by the job.
    pub(in crate::deck::career::sim::plan) fixed_parents: &'bounds [usize],
    /// Fully checked main-search careers, indexed in found-career order.
    pub(in crate::deck::career::sim::plan) found_anchors: &'bounds [Option<FoundAnchor>],
    /// Trio sides contributed by this career's own full veteran.
    pub(in crate::deck::career::sim::plan) own_trios: &'bounds [OwnTrio],
}

/// One outcome and its estimated saving, before bounding that same outcome.
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim::plan) struct OutcomeEstimate {
    /// Cell built from uncertain stars and the fixed-target landing mask.
    pub(in crate::deck::career::sim::plan) index: OutcomeIndex,
    /// Expected careers saved at this outcome.
    pub(in crate::deck::career::sim::plan) saving: f64,
}

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
    fr_of: &[Option<FoundAnchor>],
    tu: &mut [f64],
    hp: &mut HiScr,
    hs: &mut Vec<(usize, Vec<f64>, Vec<f64>)>,
    hq: &mut Vec<(usize, Vec<f64>, Vec<f64>, [f64; 4])>,
    ac: &mut Option<Vec<(f64, Vec<f64>)>>,
) {
    let HiCx { bs, d, legs, pool, fix, uncv, mk, mc, lm, nu, ntar, nm, nf, ncell, nmc, iv, rf } = *hx;
    let HiScr { bb, g1, gc, lp, lp_tmp, qz, tg } = hp;
    // her slot's miss factors and spark scales (`Env::her_slot`, as `score_career`'s setups)
    let Some(HerSlot { q0, x0, xms, pw }) =
        s.env.her_slot(pv, Lineage { trainee: u, first_parent: y, second_parent: iv })
    else {
        return;
    };
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
            for z in (0..legs.len()).filter(|&z| {
                legal_pair(d, pool, Lineage { trainee: u, first_parent: y, second_parent: z }) && holds_fix(fix, y, z)
            }) {
                let cz =
                    s.env.lineage_qc(pool, Lineage { trainee: u, first_parent: y, second_parent: z }, uncv, qz, tg);
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
            let FoundAnchor { inheritance_chances: fq, value: fv, .. } = fr_of[i].as_ref().unwrap();
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
pub(in crate::deck::career::sim::plan) fn outcome_hi(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    bounds: &OutcomeBounds,
    outcome: OutcomeEstimate,
    acache: &mut Vec<PartnerAnchors>,
) -> f64 {
    let d = s.d;
    let x = career.lineage.trainee;
    let want = &[(outcome.index.0, outcome.saving)];

    let n = context.score_base.cands.len();
    let ntar = s.env.ntar;
    let nu = s.env.uncv.len();
    let uncv = s.env.uncv.clone();
    let fixed_t = s.env.fixed_t.clone();
    let w = s.env.sj.star_dist;
    let (xi, goal) = her_ends(s, context.score_base, context.legacies, career.lineage);
    let c = s.make_cur(context.pool, career, false);
    let pv = her_pool(s, context.legacies, context.pool, true, career.lineage, &c);
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
        let m = mu | fmm[i & ((1usize << nf) - 1)].bits();
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
        return 0.0;
    }
    // the cells the table holds: the marked ones, or with most leaves marked every possible one, folded as before
    // (vectorised where every cell is possible; an unmarked leaf's bound is -inf, so its cells keep p_now, never read)
    let mc: Vec<usize> = if mk.dense { (0..ncell).filter(|&i| cell(i).0 > 0.0).collect() } else { mc };
    let ms: Vec<usize> = mc.iter().map(|&i| cell(i).1).collect();
    let lm: Vec<u32> = ms.iter().map(|&m| m as u32).collect();
    let nmc = mc.len();
    // per candidate and marked outcome her odds' bound so far: pnow, then the most of every partner's (`hi_at` without `hs`)
    let mut tab: Vec<f64> = (0..n).flat_map(|ui| std::iter::repeat_n(context.score_base.pnow[ui], nmc)).collect();
    // an anchor's chances (`Env::lineage_qc`: only the uncertain targets' entries, the ones the factors read)
    let mut hp = HiScr { bb: AncBb::new(), g1: vec![], gc: vec![], lp, lp_tmp, qz: vec![0.0f64; ntar], tg: vec![] };
    let hx = HiCx {
        bs: context.score_base,
        d,
        legs: context.legacies,
        pool: context.pool,
        fix: bounds.fixed_parents,
        uncv: &uncv,
        mk: &mk,
        mc: &mc,
        lm: &lm,
        nu,
        ntar,
        nm,
        nf,
        ncell,
        nmc,
        iv,
        rf,
    };
    // (each (candidate, partner)'s anchors as a call keeps them for the next, `acache`: in this loop's order)
    let mut np = 0usize;
    for ui in 0..n {
        let u = context.score_base.cands[ui];
        if d.umas[u].id == d.umas[x].id || context.score_base.c1[ui] == 0.0 || s.fastc[u].is_empty() {
            continue;
        }
        // her outfits' skill coverage and own chances (as `Env::rise` reads them)
        let (sts, ews) = (&s.env.sts0[u], &s.env.ew0[u]);
        hp.tg.clear(); // (`Env::lineage_qc`'s table: another trainee)
        // the main search's found careers of her deck class that it fully checked (`found_anchors`)
        let ct = s.env.cls[u];
        let fr: Vec<usize> = (0..bounds.found_anchors.len())
            .filter(|&i| bounds.found_anchors[i].as_ref().is_some_and(|f| f.deck_class == ct))
            .collect();
        // `AncBb::root_min`'s premises on her own chances
        let ew_ok = ews.iter().all(|ew| ew.iter().all(|e| (0.0..=1.0).contains(e)));
        for y in (0..iv).filter(|&y| legal_pair(d, &pv, Lineage { trainee: u, first_parent: y, second_parent: iv })) {
            if acache.len() <= np {
                acache.push(PartnerAnchors { candidate: CandidateIndex(ui), partner: y, anchors: None });
            }
            if acache[np].candidate != CandidateIndex(ui) || acache[np].partner != y {
                acache[np] = PartnerAnchors { candidate: CandidateIndex(ui), partner: y, anchors: None };
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
                bounds.found_anchors,
                &mut tab[ui * nmc..(ui + 1) * nmc],
                &mut hp,
                &mut hs,
                &mut hq,
                &mut acache[np].anchors,
            );
            np += 1;
        }
    }
    her_pool_done(s, context.pool, true, pv);
    let HiScr { mut lp, mut lp_tmp, .. } = hp;
    let full_m = nm - 1;
    // per wanted cell its bound (0 where it can't happen), from its place among the cells held (`mc`, ascending)
    let mut ys = [0.0];
    let at: Vec<Option<usize>> =
        want.iter().zip(&wc).map(|(w, p)| if p.0 > 0.0 { mc.binary_search(&w.0).ok() } else { None }).collect();
    let mut c1 = context.score_base.c1.to_vec();
    let mut up = vec![0.0f64; n];
    let t5 = now_ms();
    for (k, &j) in at.iter().enumerate() {
        let Some(j) = j else { continue };
        for ui in 0..n {
            up[ui] = tab[ui * nmc + j];
        }
        ys[k] = saving_lo(context.score_base, &up, ms[j], full_m, xi, bounds.own_trios, goal, &mut c1);
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
        let h = hi_at(context.score_base, &hs, leaf, m, full_m, xi, bounds.own_trios, goal, &mut c1, &mut up);
        let (mut need, mut far): (Vec<usize>, bool) = (vec![], false);
        for ui in 0..n {
            if up[ui] <= context.score_base.pnow[ui] * (1.0 + 1e-9) {
                continue;
            }
            let keep = up[ui];
            up[ui] = context.score_base.pnow[ui];
            let h2 = saving_lo(context.score_base, &up, m, full_m, xi, bounds.own_trios, goal, &mut c1);
            up[ui] = keep;
            if h2 >= h - 1e-9 {
                continue;
            }
            for (pi, (pu, bd, sc)) in hs.iter().enumerate() {
                if *pu != ui {
                    continue;
                }
                let v = sc[m] * bd[leaf];
                if v > context.score_base.pnow[ui] * (1.0 + 1e-9) {
                    need.push(pi);
                    if v > HI_RISE * context.score_base.pnow[ui] {
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
            ys[k] = hi_at(
                context.score_base,
                &hs,
                want[k].0 >> nf,
                wc[k].1,
                full_m,
                xi,
                bounds.own_trios,
                goal,
                &mut c1,
                &mut up,
            );
        }
    }
    ys[0]
}

/// The pick's own 8/8 as a trio side (`her_trios`) for `outcome_hi`, taken as the plan ends (the deck cache then:
/// `her_trios` reads C's cached decks), where an outcome that lands every target reads y < 0.5 (`full_lo`: only there
/// does `saving_lo` read it; else none)
pub(in crate::deck::career::sim::plan) fn pick_her(
    s: &mut Sim,
    context: &ScoringContext,
    career: Career,
    full_lo: bool,
    buf: &mut Vec<f64>,
) -> Vec<OwnTrio> {
    let (xi, goal) = her_ends(s, context.score_base, context.legacies, career.lineage);
    if !full_lo || xi.is_some() || goal || context.score_base.sides.is_empty() {
        return vec![];
    }
    let c = s.make_cur(context.pool, career, false);
    let full_m = (1usize << s.env.ntar) - 1;
    if c.os[full_m] <= 0.0 {
        return vec![];
    }
    let mut pv = her_pool(s, context.legacies, context.pool, true, career.lineage, &c);
    let iv = pv.me.len() - 1;
    let r = her_trios(s, context.score_base, &mut pv, iv, career.lineage.trainee, buf);
    her_pool_done(s, context.pool, true, pv);
    r
}

/// `outcome_hi`'s found-career anchors: per found career of the main search `msv` (its order), (her deck class, q_f,
/// val_f) when the search fully checked it (a Ref at the career's own chances: val_f is the best value of any deck at q_f
/// for every trainee of the class, `Env::seen`, scale 1), else none
pub(in crate::deck::career::sim::plan) fn found_anchors(
    env: &Env,
    pool: &Pool,
    msv: &Solved,
) -> Vec<Option<FoundAnchor>> {
    msv.found
        .iter()
        .map(|f| {
            let (fq, _) = env.lineage_q(pool, Lineage { trainee: f.t, first_parent: f.a, second_parent: f.b });
            msv.refs.get(f.t).and_then(|rs| rs.iter().find(|r| r.q == fq)).map(|r| FoundAnchor {
                deck_class: env.cls[f.t],
                inheritance_chances: r.q.clone(),
                value: r.val,
            })
        })
        .collect()
}
