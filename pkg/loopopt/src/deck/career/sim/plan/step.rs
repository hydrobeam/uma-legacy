//! Step costs and the leaves used to estimate the next career.
use super::{Cont, LB_CELLS, ScoreBase, Trio, trio_v, v0_after, with_her};

/// How V after her (`v_after`) costs the steps of the candidates her veteran lifts: the estimate's (`Est`, `Cont::cost`,
/// for y) or the lower bound's (`Lo`, `Cont::cost_lo`, for y_hi). Monomorphised: each path keeps its own code
pub(in crate::deck::career::sim::plan) trait StepCost {
    /// candidate ui lifted to odds p (row ci of the outcome's lifts): her step taken into c1 where it is lower (`ch`: the
    /// steps that moved), or left for `va`
    fn take(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, ci: usize, ui: usize, p: f64);
    /// candidate xi's step set to 0 (her own 8/8 on the full set)
    fn zeroed(&mut self, _xi: usize) {}
    /// V after her before her own 8/8's trios, from c1 (`her_all`: those trios follow, reading every step)
    fn va(&mut self, bs: &ScoreBase, c1: &mut [f64], ch: &mut Vec<usize>, her_all: bool) -> f64;
}

/// The bound's steps: each lifted candidate at `Cont::cost_lo` (`saving_lo`), V after her by `v0_after`
pub(in crate::deck::career::sim::plan) struct Lo;

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
pub(in crate::deck::career::sim::plan) struct Est<'base> {
    pub(in crate::deck::career::sim::plan) last: Vec<(u64, f64)>,
    lbc: Vec<Option<&'base Cont>>,
    pub(in crate::deck::career::sim::plan) cell: Vec<(u64, f64)>,
    tord: Vec<usize>,
    one: &'base [One],
    v_same: f64,
    pend: Vec<(usize, usize, f64)>,
    lo: Vec<f64>,
    pf: Vec<bool>,
    pub(in crate::deck::career::sim::plan) n_ch: usize,
    pub(in crate::deck::career::sim::plan) n_cost: usize,
    pub(in crate::deck::career::sim::plan) n_same: usize,
    pub(in crate::deck::career::sim::plan) n_one: usize,
}

const EST_CC: usize = 256;

/// `Est`'s cache buffers between scorings (`FoldScratch`)
#[derive(Default)]
pub(in crate::deck::career::sim::plan) struct EstBufs {
    pub(in crate::deck::career::sim::plan) last: Vec<(u64, f64)>,
    pub(in crate::deck::career::sim::plan) cell: Vec<(u64, f64)>,
}

impl<'base> Est<'base> {
    /// for `rows` lift rows; `lazy` (`sc_outcomes`: every step, stage 2-3 cost and C's step a number >= 0, not -0.0)
    pub(in crate::deck::career::sim::plan) fn new(
        bs: &'base ScoreBase<'_>,
        rows: usize,
        lazy: bool,
        one: &'base [One],
        eb: EstBufs,
    ) -> Self {
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
pub(in crate::deck::career::sim::plan) fn v_after<C: StepCost>(
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
pub(in crate::deck::career::sim::plan) struct One {
    not: f64,
    rd: Vec<(f64, f64, f64)>,
    ok: bool,
}

pub(in crate::deck::career::sim::plan) fn one_of(combos: &[Trio], c1: &[f64], i: usize) -> One {
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
pub(in crate::deck::career::sim) trait Leaves {
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

pub(in crate::deck::career::sim) struct All;

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
pub(in crate::deck::career::sim) struct OneLeaf {
    ls: [usize; 2],
    n: usize,
}

impl OneLeaf {
    pub(in crate::deck::career::sim::plan) const fn new(leaf: usize, rep: usize) -> Self {
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
