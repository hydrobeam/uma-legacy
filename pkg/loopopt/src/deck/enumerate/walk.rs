//! The depth-first walk of decks in one configuration.
use super::RBCHK;
use super::{
    BorrowTables, DOM_GAP, DeckPath, NodeScratch, ParentRatioBounds, SearchConfig, SearchControl, TraversalInputs,
    TraversalTables,
};
use crate::deck::{CFCHK, Kind, SBCHK, SBN, Sel, Src, StateBound, cap_class, gm_g, gm_w, joint_fast, sb_check};

/// The depth-first walk of one configuration's decks: its tables (borrowed from `Traversal::visit`), the node bound's
/// scratch, the state-conditioned bounds, the decks kept and the bar
pub(in crate::deck::enumerate) struct Walk<'walk, 'run, 'space, V> {
    pub(in crate::deck::enumerate) inp: &'walk TraversalInputs<'run, 'space, V>,
    pub(in crate::deck::enumerate) ns: &'walk mut NodeScratch,
    pub(in crate::deck::enumerate) out: &'walk mut Vec<(f64, Sel)>,
    pub(in crate::deck::enumerate) bestv: f64,
    pub(in crate::deck::enumerate) cf: &'walk SearchConfig,
    pub(in crate::deck::enumerate) filler: usize,
    pub(in crate::deck::enumerate) plain: usize,
    pub(in crate::deck::enumerate) gs: &'walk [(&'run [usize], usize)],
    pub(in crate::deck::enumerate) gmin: &'walk [&'walk [f64]],
    pub(in crate::deck::enumerate) rp: &'walk [&'walk [f64]],
    pub(in crate::deck::enumerate) mw: &'walk [f64],
    pub(in crate::deck::enumerate) mg: &'walk [f64],
    pub(in crate::deck::enumerate) tmk: &'walk [u32],
    pub(in crate::deck::enumerate) sufw: &'walk [f64],
    pub(in crate::deck::enumerate) sufg: &'walk [f64],
    pub(in crate::deck::enumerate) spf: &'walk [f64],
    pub(in crate::deck::enumerate) doms: &'walk [std::rc::Rc<Vec<Vec<(u32, u32)>>>],
    pub(in crate::deck::enumerate) doff: &'walk [usize],
    pub(in crate::deck::enumerate) dsuf: &'walk [u128],
    pub(in crate::deck::enumerate) bt: &'walk BorrowTables,
    pub(in crate::deck::enumerate) csuf: &'walk [u64],
    pub(in crate::deck::enumerate) goff: &'walk [usize],
    pub(in crate::deck::enumerate) avail: &'walk [Vec<u8>],
    pub(in crate::deck::enumerate) loff: &'walk [usize],
    pub(in crate::deck::enumerate) ncan: usize,
    pub(in crate::deck::enumerate) ng: usize,
    pub(in crate::deck::enumerate) total_slots: usize,
    pub(in crate::deck::enumerate) sbb: StateBound<'walk>,
    pub(in crate::deck::enumerate) sb_root: f64,
    // (SB_CHECK: the depth-1 node's bound)
    pub(in crate::deck::enumerate) sb_d1: f64,
    pub(in crate::deck::enumerate) cf_path: Vec<f64>,
    // (the cap-free band past its node limit)
    pub(in crate::deck::enumerate) cf_over: bool,
    // the deck's odds, written per leaf (no allocation unless the deck is kept)
    pub(in crate::deck::enumerate) dwa: [f64; 32],
    pub(in crate::deck::enumerate) dga: [f64; 32],
}

impl<V: Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64> Walk<'_, '_, '_, V> {
    /// The decks below a node: group g from position `start` with `left` slots of it to fill, characters `chm`,
    /// borrowed cards `nb`, locked cards `have`, class mask `cmk`, characters `need` its decks must take; the path
    /// `st` holds the picks so far. True: the search stops (the output is full)
    pub(in crate::deck::enumerate) fn rec(
        &mut self,
        st: &mut DeckPath,
        g: usize,
        start: usize,
        left: usize,
        chm: u128,
        nb: usize,
        have: u8,
        cmk: u64,
        need: u128,
    ) -> bool {
        let (sp, nu, gs) = (self.inp.sp, self.inp.ctl.nu, self.gs);
        let TraversalTables { ebit, lbit, all_lk: all_locked, .. } = self.inp.tb;
        let (avail, mw, mg, tmk) = (self.avail, self.mw, self.mg, self.tmk);
        if left == 0 {
            if g + 1 >= gs.len() {
                if need & !chm != 0 {
                    return false;
                }
                let b = st.sel.len() * nu;
                return self.leaf(&st.sel, &st.pw[b..b + nu], &st.pg[b..b + nu], cmk);
            }
            return self.rec(st, g + 1, 0, gs[g + 1].1, chm, nb, have, cmk, need);
        }
        if all_locked & !have & !avail[g][start] != 0 {
            return false;
        }
        let list = gs[g].0;
        // the deck so far is row sel.len(); a child's row is the next one (overwritten per child)
        let b = st.sel.len() * nu;
        // this parent's id and depth (the per-parent rows of `hopeful`)
        st.pid += 1;
        let (pid, dep) = (st.pid, st.sel.len());
        for p in start..list.len() {
            if list.len() - p < left {
                break;
            }
            let j = list[p];
            if chm & ebit[j] != 0 || (sp.ebor[j] && nb >= sp.allowed_borrow) {
                continue;
            }
            let Some(nd) = self.dom(g, p, start, left, chm, &st.pg[b..b + nu], &st.sel, need) else {
                continue;
            };
            sp.nodes.set(sp.nodes.get() + 1);
            // the child's row (depth + 1): the parent's times the card's misses
            {
                let (par, ch) = st.pw.split_at_mut(b + nu);
                let (mj, par, ch) = (&mw[j * nu..(j + 1) * nu], &par[b..], &mut ch[..nu]);
                for t in 0..nu {
                    ch[t] = par[t] * mj[t];
                }
            }
            {
                let (par, ch) = st.pg.split_at_mut(b + nu);
                let (mj, par, ch) = (&mg[j * nu..(j + 1) * nu], &par[b..], &mut ch[..nu]);
                for t in 0..nu {
                    ch[t] = par[t] * mj[t];
                }
            }
            let cj = cmk | sp.cvm[j];
            if !self.hopeful(
                g,
                left - 1,
                &st.pw[b + nu..b + 2 * nu],
                &st.pg[b + nu..b + 2 * nu],
                nb + usize::from(sp.ebor[j]),
                &st.pw[b..b + nu],
                &st.pg[b..b + nu],
                dep,
                pid,
                tmk[j],
                cj,
                p,
            ) {
                continue;
            }
            st.sel.push(j);
            if !self.sbx(&st.sel) {
                st.sel.pop();
                continue;
            }
            let stop =
                self.rec(st, g, p + 1, left - 1, chm | ebit[j], nb + usize::from(sp.ebor[j]), have | lbit[j], cj, nd);
            st.sel.pop();
            if stop {
                return true;
            }
        }
        false
    }

    /// The child taking entry B = list[p] of group g, at a node with `start`, `left` slots of g to fill, characters
    /// `chm`, gold misses `ppg`, picks `sel` and characters `need` its decks must take: None when it can be
    /// skipped, else its decks' needed characters. An entry A dominating B (`dom_table`) that is free at the node
    /// and takes B's place in the pick order (a sibling, or no pick of g after A touching B's targets) gives every
    /// deck through B whose picks after B take no card of A's character an equal or better deck before it; when
    /// nothing of A's character can follow, that is every deck through B, else those decks must take one
    /// (per target, ppg * rlo, the same for every dominator: computed at its first use in a call, `prm`'s bit t
    /// set; the slots are reused across calls, never read unset)
    fn dom(
        &mut self,
        g: usize,
        p: usize,
        start: usize,
        left: usize,
        chm: u128,
        ppg: &[f64],
        sel: &[usize],
        need: u128,
    ) -> Option<u128> {
        let SearchControl { dom_on, nu, .. } = self.inp.ctl;
        if !dom_on {
            return Some(need);
        }
        let (gs, doff, doms, dsuf) = (self.gs, self.doff, self.doms, self.dsuf);
        let (ebit, tmk, mg) = (self.inp.tb.ebit, self.tmk, self.mg);
        let l = gs[g].0;
        let bj = l[p];
        let f = doff[g] + p;
        let mut nd = need;
        let dl = &doms[g][p];
        if !dl.is_empty() {
            // (the deck's picks in group g so far, all at positions below `start`)
            let sg = &sel[sel.len() - (gs[g].1 - left)..];
            let (pr, mut prm) = (&mut self.ns.dpr, 0u32);
            for &(a, sm) in dl {
                let a = a as usize;
                let aj = l[a];
                if chm & ebit[aj] != 0 {
                    continue;
                }
                if a < start && sg.iter().any(|&j| j > aj && tmk[j] & tmk[bj] != 0) {
                    continue;
                }
                if sm != 0 {
                    let mut ok = true;
                    let mut m = sm;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        if prm >> t & 1 == 0 {
                            let rlo = gm_g(self.gmin[g], nu, left - 1)[t] * self.sufg[(g + 1) * nu + t];
                            pr[t] = ppg[t] * rlo;
                            prm |= 1 << t;
                        }
                        if !(pr[t] * (mg[bj * nu + t] - mg[aj * nu + t]) >= DOM_GAP) {
                            ok = false;
                            break;
                        }
                    }
                    if !ok {
                        continue;
                    }
                }
                if ebit[aj] != ebit[bj] && ebit[aj] & dsuf[f + 1] != 0 {
                    nd |= ebit[aj];
                    continue;
                }
                return None;
            }
        }
        // a needed character that no later pick can bring
        if nd & !(chm | ebit[bj]) & !dsuf[f + 1] != 0 {
            return None;
        }
        Some(nd)
    }

    /// Can a deck from here (group g with `left` still to pick, `pw`/`pg` the partial miss) pass?
    /// (ppw, ppg: the parent's row; dep, pid: the parent's depth and id; tm: the targets the child's card
    /// touches; cm, p: the child's class mask (`cv_mask`s ORed) and its card's position in group g)
    fn hopeful(
        &mut self,
        g: usize,
        left: usize,
        pw: &[f64],
        pg: &[f64],
        nb: usize,
        ppw: &[f64],
        ppg: &[f64],
        dep: usize,
        pid: usize,
        tm: u32,
        cm: u64,
        p: usize,
    ) -> bool {
        // (the cap-free band prunes by the state-conditioned bound alone, `sbx`)
        if self.inp.ctl.cfm {
            return true;
        }
        // (each table is read where it is first needed: a read through `inp` or `bt` written before the node's stores
        // stays there, as the compiler cannot tell those stores leave it unchanged, and the early return then pays it)
        let inp = self.inp;
        let nu = inp.ctl.nu;
        let NodeScratch {
            dw,
            dg,
            av,
            cvx,
            pdwx,
            pdgx,
            kab,
            kr,
            kab_have,
            kr_have,
            hc,
            hb,
            tw,
            tg,
            told,
            bdw,
            bdg,
            brow,
            bw,
            bg,
            rbs,
            ..
        } = &mut *self.ns;
        let (cv, pdw, pdg) = (cvx.as_mut_slice(), pdwx.as_mut_slice(), pdgx.as_mut_slice());
        // the cap of every deck the child can still reach: its cards' classes and every entry left
        let rho = if inp.ctl.best {
            inp.ctl.rho
        } else {
            let (csuf, goff) = (self.csuf, self.goff);
            inp.ctl.ccap[cap_class(cm | if left > 0 { csuf[goff[g] + p + 1] } else { csuf[goff[g + 1]] })]
        };
        // borrowed cards still allowed (only with the one-borrowed tables)
        let can = nb < 1;
        let mut have_a = false;
        let one_b = self.bt.one_b;
        let row = if g >= self.ng { usize::MAX } else { (self.loff[g] + left) * self.ncan + usize::from(one_b && can) };
        let (pw, pg) = (&pw[..nu], &pg[..nu]);
        if row == usize::MAX {
            for t in 0..nu {
                dw[t] = 1.0 - pw[t] * 1.0;
                dg[t] = 1.0 - pg[t] * 1.0;
                pdw[t] = 1.0 - pw[t];
                pdg[t] = 1.0 - pg[t];
            }
        } else {
            if !kab_have[row] {
                kab_have[row] = true;
                let (gmin, sufw, sufg) = (self.gmin, self.sufw, self.sufg);
                let BorrowTables { gbs, so, s1, .. } = self.bt;
                for t in 0..nu {
                    let (a, b) = if !one_b {
                        (
                            gm_w(gmin[g], nu, left)[t] * sufw[(g + 1) * nu + t],
                            gm_g(gmin[g], nu, left)[t] * sufg[(g + 1) * nu + t],
                        )
                    } else if can {
                        let gb = &gbs[g];
                        (
                            (gb.one[left].0[t] * so[g + 1].0[t]).min(gb.own[left].0[t] * s1[g + 1].0[t]),
                            (gb.one[left].1[t] * so[g + 1].1[t]).min(gb.own[left].1[t] * s1[g + 1].1[t]),
                        )
                    } else {
                        (gbs[g].own[left].0[t] * so[g + 1].0[t], gbs[g].own[left].1[t] * so[g + 1].1[t])
                    };
                    kab[row * 2 * nu + t] = a;
                    kab[row * 2 * nu + nu + t] = b;
                }
            }
            let (ka, kb) = kab[row * 2 * nu..(row + 1) * 2 * nu].split_at(nu);
            if inp.a_rows {
                let (noq, orow, oqq) = (inp.noq, inp.tb.orow, &*inp.oqq);
                // A child is its parent plus one card: for a target the card does not touch (its miss
                // chances exactly 1.0) the child's pw[t] = parent's pw[t] * 1.0 is the parent's, bit for
                // bit, and so are dw[t], dg[t] and every outfit's A factor of t (the same expressions on
                // the same f64s; ka, kb depend only on the row, the same for all children of one parent
                // and borrowing variant). So the factors are computed once per parent (here, at its first
                // child), a child recomputes only its touched targets', and A is the product in target
                // order (`OutfitPre::eval`'s). The factors before the first touched target t0 are the
                // parent's, and so is their product from 1.0 (`pp`, kept per parent): a child's product
                // starts there and multiplies on from t0, the same multiplications in the same order
                let c = &mut hc[dep * 2 + usize::from(one_b && can)];
                if c.pid != pid {
                    c.pid = pid;
                    for t in 0..nu {
                        bw[t] = 1.0 - ppw[t] * ka[t];
                        bg[t] = 1.0 - ppg[t] * kb[t];
                    }
                    for ix in 0..noq {
                        orow[ix].factors(oqq[ix], &bw[..nu], &bg[..nu], &mut c.fa[ix * nu..(ix + 1) * nu]);
                    }
                    for (row, pr) in c.fa.chunks_exact(nu).zip(c.pp.chunks_exact_mut(nu + 1)) {
                        for t in 0..nu {
                            pr[t + 1] = pr[t] * row[t];
                        }
                    }
                }
                let mut m = tm;
                while m != 0 {
                    let t = m.trailing_zeros() as usize;
                    m &= m - 1;
                    tw[t] = 1.0 - pw[t] * ka[t];
                    tg[t] = 1.0 - pg[t] * kb[t];
                }
                let t0 = if tm == 0 { nu } else { tm.trailing_zeros() as usize };
                for (ix, (row, pr)) in c.fa.chunks_exact_mut(nu).zip(c.pp.chunks_exact(nu + 1)).enumerate() {
                    let (o, q) = (&orow[ix], oqq[ix]);
                    let mut m = tm;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        told[t] = row[t];
                        row[t] = o.factor_at(t, q, tw[t], tg[t]);
                    }
                    let mut p = pr[t0];
                    for &x in &row[t0..] {
                        p *= x;
                    }
                    let mut m = tm;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        row[t] = told[t];
                    }
                    av[ix] = p;
                }
                // (B's inputs pdw, pdg: below, only when B is evaluated)
                have_a = true;
            } else {
                let (dw, dg, pdw, pdg) = (&mut dw[..nu], &mut dg[..nu], &mut pdw[..nu], &mut pdg[..nu]);
                for t in 0..nu {
                    dw[t] = 1.0 - pw[t] * ka[t];
                    dg[t] = 1.0 - pg[t] * kb[t];
                    pdw[t] = 1.0 - pw[t];
                    pdg[t] = 1.0 - pg[t];
                }
            }
        }
        // (rbsl: 1, or RB_CHECK's s; x * 1.0 is x)
        let (best, bestv, clauses, rbsl) = (inp.ctl.best, self.bestv, inp.clauses, inp.ctl.rbsl);
        let pass = |cv: &[f64]| {
            if best {
                cv[0] * rbsl * (1.0 + 1e-9) >= bestv
            } else {
                (0..clauses.len()).any(|ci| rho * cv[ci] * rbsl * (1.0 + 1e-9) >= clauses[ci].1)
            }
        };
        // the per-target values A first (each outfit's term is min(A, B r) <= A, so the clause sums with
        // A alone are upper bounds of the real ones: when they fail, so does the real test; A is finite,
        // its inputs being probabilities, and a NaN falls through to the real test)
        cv.fill(0.0);
        let TraversalTables { oq, og, orow, .. } = inp.tb;
        let fixed = inp.ctl.fixed;
        let mut nan = false;
        for &(ci, i, no) in og {
            let q = oq[i].2;
            let mut m = 0.0f64;
            for o in 0..no {
                let a = if have_a { av[i + o] } else { orow[i + o].eval(q, dw, dg) };
                av[i + o] = a;
                nan |= a.is_nan();
                m = m.max(a);
            }
            cv[ci] += fixed * m;
        }
        if !nan && !pass(cv) {
            return false;
        }
        let nc = inp.ctl.nc;
        if row != usize::MAX && !kr_have[row] {
            kr_have[row] = true;
            let (size, rp, spf) = (inp.ctl.size, self.rp, self.spf);
            let BorrowTables { gbs, po, p1, .. } = self.bt;
            for ix in 0..nc {
                kr[row * nc + ix] = if !one_b {
                    rp[g][ix * (size + 1) + left] * spf[(g + 1) * nc + ix]
                } else if can {
                    (gbs[g].rone[ix][left] * po[g + 1][ix]).max(gbs[g].rown[ix][left] * p1[g + 1][ix])
                } else {
                    gbs[g].rown[ix][left] * po[g + 1][ix]
                };
            }
        }
        // B's factors are at the child's odds 1 - pw, 1 - pg. With the per-parent rows (have_a) they are
        // the parent's at its targets the card does not touch (pw[t] is the parent's, bit for bit, as
        // for A): per row, the parent's factors (`ParentRatioBounds`, built at the first child that evaluates the row)
        // with the touched targets' recomputed, and B the product over its targets in target
        // order from 1.0 (`eval`'s and `eval_in`'s), the full one from the parent's prefix at t0.
        // Otherwise at pdw, pdg (filled above)
        let t0 = if tm == 0 { nu } else { tm.trailing_zeros() as usize };
        if have_a {
            let h = &mut hb[dep];
            if h.pid != pid {
                h.pid = pid;
                for t in 0..nu {
                    h.pdw[t] = 1.0 - ppw[t];
                    h.pdg[t] = 1.0 - ppg[t];
                }
            }
            let mut m = tm;
            while m != 0 {
                let t = m.trailing_zeros() as usize;
                m &= m - 1;
                bdw[t] = 1.0 - pw[t];
                bdg[t] = 1.0 - pg[t];
            }
        }
        // (have_a: row ix's B factors into `brow`, the parent's row with the touched ones recomputed;
        // with `full`, their product, else 0)
        let oqq = &*inp.oqq;
        let mut bfill = |ix: usize, full: bool, brow: &mut [f64; 32]| -> f64 {
            let (o, q) = (&orow[ix], oqq[ix]);
            let ParentRatioBounds { st, pdw: hw, pdg: hg, fb, pb, .. } = &mut hb[dep];
            let (fr, pr) = (&mut fb[ix * nu..(ix + 1) * nu], &mut pb[ix * (nu + 1)..(ix + 1) * (nu + 1)]);
            if st[ix] != pid {
                st[ix] = pid;
                o.factors(q, &hw[..nu], &hg[..nu], fr);
                for t in 0..nu {
                    pr[t + 1] = pr[t] * fr[t];
                }
            }
            brow[..nu].copy_from_slice(fr);
            let mut m = tm;
            while m != 0 {
                let t = m.trailing_zeros() as usize;
                m &= m - 1;
                brow[t] = o.factor_at(t, q, bdw[t], bdg[t]);
            }
            if !full {
                return 0.0;
            }
            let mut p = pr[t0];
            for &x in &brow[t0..nu] {
                p *= x;
            }
            p
        };
        // (the product of `brow` over the targets in z, in target order: `eval_in`'s)
        let bin = |brow: &[f64; 32], z: u32| -> f64 {
            let (mut z, mut p) = (z, 1.0f64);
            while z != 0 {
                let t = z.trailing_zeros() as usize;
                z &= z - 1;
                p *= brow[t];
            }
            p
        };
        // the real bound, the same expression in the same order; an outfit whose A is at most the running
        // maximum leaves it unchanged (min(A, B r) <= A <= m) and is skipped
        cv.fill(0.0);
        {
            let TraversalTables { zm, nm, alt, .. } = inp.tb;
            // (ZSPLIT; a row without a split, zm 0 and no low split, takes min(A, B r)) A's product over a
            // split's Z: from the per-parent row with the child's touched targets recomputed (A's own
            // factors), or at the node's odds
            let hrow = if have_a { dep * 2 + usize::from(one_b && can) } else { usize::MAX };
            let a_z = |ix: usize, z: u32| -> f64 {
                let (o, q) = (&orow[ix], oqq[ix]);
                if hrow == usize::MAX {
                    return o.eval_in(q, dw, dg, z);
                }
                let fa = &hc[hrow].fa[ix * nu..(ix + 1) * nu];
                let (mut z, mut p) = (z, 1.0f64);
                while z != 0 {
                    let t = z.trailing_zeros() as usize;
                    z &= z - 1;
                    p *= if tm >> t & 1 != 0 { o.factor_at(t, q, tw[t], tg[t]) } else { fa[t] };
                }
                p
            };
            for &(ci, i, no) in og {
                let q = oq[i].2;
                let mut m = 0.0f64;
                for o in 0..no {
                    let ix = i + o;
                    let a = av[ix];
                    if a <= m {
                        continue;
                    }
                    let r = if row == usize::MAX { 1.0 } else { kr[row * nc + ix] };
                    // A's factors of the zero targets times N's ratio bound; a low split's, the lower
                    let mut b = if have_a {
                        let full = bfill(ix, zm[ix] == 0, brow);
                        if zm[ix] != 0 { a_z(ix, zm[ix]) * (bin(brow, nm[ix]) * r) } else { full * r }
                    } else if zm[ix] != 0 {
                        a_z(ix, zm[ix]) * (orow[ix].eval_in(q, pdw, pdg, nm[ix]) * r)
                    } else {
                        orow[ix].eval(q, pdw, pdg) * r
                    };
                    for c in alt[ix].0..alt[ix].1 {
                        let r = if row == usize::MAX { 1.0 } else { kr[row * nc + c] };
                        let bn = if have_a { bin(brow, nm[c]) } else { orow[ix].eval_in(q, pdw, pdg, nm[c]) };
                        b = b.min(a_z(ix, zm[c]) * (bn * r));
                    }
                    m = m.max(a.min(b));
                }
                cv[ci] += fixed * m;
            }
        }
        let ok = pass(cv);
        if inp.ctl.rbc && ok {
            let s = rbs;
            if s.len() < dep + 2 {
                s.resize(dep + 2, vec![]);
            }
            s[dep + 1].clear();
            s[dep + 1].extend_from_slice(cv);
        }
        ok
    }

    /// The state-conditioned bound at depth <= 1 (mode 1) or at every depth, the leaf included (mode 2); with
    /// SB_CHECK, every leaf's value against its root's and depth-1 node's bounds. False: the node is pruned
    fn sbx(&mut self, sel: &[usize]) -> bool {
        let inp = self.inp;
        if inp.ctl.cfm {
            let SearchControl { nu, cf_lim, cfs, sb_bar, sb_u, sb_q, .. } = inp.ctl;
            let (sp, model, total_slots) = (inp.sp, inp.model, self.total_slots);
            // the cap-free band: the bound at every node and leaf, nothing else prunes; past the node
            // limit every child fails and the search ends with CapFreeNodeLimit
            if self.cf_over {
                return false;
            }
            if sp.nodes.get() > cf_lim {
                self.cf_over = true;
                return false;
            }
            let d = sel.len();
            let leafd = d == total_slots;
            let s = if leafd { 1.0 } else { cfs.max(1.0) };
            // the cheap bound first, the state-conditioned one only where it passes
            let ubc = self.sbb.cheap(sel);
            // (CF_CHECK: both always, so the check below tests each: an exact value above either is above their minimum)
            let ub = if cfs == 0.0 && ubc * s * (1.0 + 1e-9) < sb_bar {
                ubc
            } else {
                ubc.min(self.sbb.full(sel, if cfs == 0.0 { sb_bar / s } else { 0.0 }))
            };
            SBN.with(|c| c.borrow_mut().1 += 1);
            if cfs > 0.0 {
                // CF_CHECK: a leaf's exact value against its own bound and every node's above it
                let path = &mut self.cf_path;
                path.truncate(d);
                path.push(ub);
                if leafd {
                    let ents: Vec<(&Src, Kind)> = sel.iter().map(|&j| (&sp.srcs[j], sp.kind[j])).collect();
                    let x = joint_fast(
                        model,
                        sb_u,
                        &[sb_q],
                        &ents,
                        self.filler,
                        nu + model.n_fixed_gold as usize,
                        false,
                        None,
                    )[0]
                    .0;
                    CFCHK.with(|c| {
                        let mut c = c.borrow_mut();
                        c.4 += 1;
                        for &b in path.iter() {
                            if x > b * (1.0 + 1e-9) {
                                c.5 += 1;
                            }
                            if b > 0.0 {
                                c.8 = c.8.max(x / b);
                            }
                        }
                    });
                }
            }
            let pass = ub * s * (1.0 + 1e-9) >= sb_bar;
            if !pass {
                SBN.with(|c| c.borrow_mut().2 += 1);
            }
            return pass;
        }
        let sbm = inp.ctl.sbm;
        if sbm == 0 || (sbm == 3 && !sb_check()) {
            return true;
        }
        let d = sel.len();
        let chk = sb_check();
        let leafd = d == self.total_slots;
        if (sbm == 1 || sbm == 3) && d > 1 && !(chk && leafd) {
            return true;
        }
        if sbm == 3 && d == 1 && !chk {
            return true;
        }
        let ub = self.sbb.bound(sel);
        SBN.with(|c| c.borrow_mut().1 += 1);
        if d == 1 {
            self.sb_d1 = ub;
        }
        if chk && leafd {
            let (sb_root, sb_d1) = (self.sb_root, self.sb_d1);
            SBCHK.with(|c| {
                let mut c = c.borrow_mut();
                c.4 += 1;
                if ub > sb_root * (1.0 + 1e-9) || (d > 1 && ub > sb_d1 * (1.0 + 1e-9)) {
                    c.5 += 1;
                }
            });
        }
        if (sbm == 1 && d > 1) || sbm == 3 {
            return true;
        }
        let pass = ub * (1.0 + 1e-9) >= inp.ctl.sb_bar;
        if !pass {
            SBN.with(|c| c.borrow_mut().2 += 1);
        }
        pass
    }

    /// A complete deck `sel` (its miss products `pw`, `pg`, class mask `cmk`): kept when it beats the bar (product
    /// search) or passes a clause at its class cap (band). True: the output is full, the search stops
    pub(in crate::deck::enumerate) fn leaf(&mut self, sel: &[usize], pw: &[f64], pg: &[f64], cmk: u64) -> bool {
        let SearchControl { best, cap, nu, cfm, ccap, rbc, .. } = self.inp.ctl;
        let (sp, clauses, val) = (self.inp.sp, self.inp.clauses, &self.inp.val);
        let (plain, filler) = (self.plain, self.filler);
        let pals = sel.iter().filter(|&&j| sp.cards[sp.rel[j]].pal).count() + plain;
        if let Some(p) = sp.dj.pals
            && p != pals
        {
            return false;
        }
        if !sp.locked.iter().all(|l| sel.iter().any(|&j| sp.rel[j] == *l)) {
            return false;
        }
        for t in 0..nu {
            self.dwa[t] = 1.0 - pw[t];
            self.dga[t] = 1.0 - pg[t];
        }
        let (dw, dg) = (&self.dwa[..nu], &self.dga[..nu]);
        // RB_CHECK: each clause's value against the bounds of the root and of every node above the deck
        if rbc {
            let (s, cf) = (&self.ns.rbs, self.cf);
            RBCHK.with(|c| {
                let mut c = c.borrow_mut();
                c.0 += 1;
                for (ci, cl) in clauses.iter().enumerate() {
                    if best && ci > 0 {
                        break;
                    }
                    let x = val(cl, dw, dg);
                    for d in 0..=sel.len() {
                        let ub = if d == 0 { cf.clause_bounds[ci] } else { s[d][ci] };
                        c.1 += 1;
                        if x > ub * (1.0 + 1e-9) {
                            c.2 += 1;
                        }
                        if ub > 0.0 {
                            c.3 = c.3.max(x / ub);
                        }
                    }
                }
            });
        }
        let mk = |v: f64| {
            let mut e = sel.to_vec();
            e.sort_unstable();
            (v, Sel { ents: e, plain, filler, dw: dw.to_vec(), dg: dg.to_vec() })
        };
        if best {
            let v = val(&clauses[0], dw, dg);
            if v > self.bestv {
                self.bestv = v;
                self.out.clear();
                self.out.push(mk(v));
            }
            false
        } else {
            // each clause's value once: the best of them, and whether any passes its bar at the deck's class cap
            let (mut v, mut pass) = (f64::NEG_INFINITY, false);
            let rho = ccap[cap_class(cmk)];
            for cl in clauses {
                let x = val(cl, dw, dg);
                v = v.max(x);
                pass = pass || cfm || rho * x * (1.0 + 1e-9) >= cl.1;
            }
            if pass {
                self.out.push(mk(v));
                if self.out.len() >= cap {
                    return true;
                }
            }
            false
        }
    }
}
