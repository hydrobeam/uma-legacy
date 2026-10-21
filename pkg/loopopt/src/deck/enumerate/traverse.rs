//! Configuration traversal and its borrowed-card and state bounds.
use super::{
    ConfigOrder, ConfigScratch, ConfigurationQueue, DeckPath, NodeScratch, SearchConfig, SearchOutcome, SearchStop,
    Walk,
};
use crate::deck::{
    CapFreeRatios, DeckSpace, GTab, HashMap, Instant, Kind, LandPre, Model, SBN, SPT, SbCfg, Sel, StateBound,
    chain_states,
};

/// Immutable controls selected before traversal. Scalar copies do not change the search's buffers.
#[derive(Clone, Copy)]
pub(in crate::deck::enumerate) struct SearchControl {
    pub(in crate::deck::enumerate) best: bool,
    pub(in crate::deck::enumerate) rho: f64,
    pub(in crate::deck::enumerate) cap: usize,
    pub(in crate::deck::enumerate) nu: usize,
    pub(in crate::deck::enumerate) size: usize,
    pub(in crate::deck::enumerate) nk: usize,
    pub(in crate::deck::enumerate) set_id: usize,
    pub(in crate::deck::enumerate) fixed: f64,
    pub(in crate::deck::enumerate) cfm: bool,
    pub(in crate::deck::enumerate) cf_lim: usize,
    pub(in crate::deck::enumerate) cfs: f64,
    pub(in crate::deck::enumerate) sbm: u8,
    pub(in crate::deck::enumerate) sb_bar: f64,
    pub(in crate::deck::enumerate) sb_u: usize,
    pub(in crate::deck::enumerate) sb_q: f64,
    pub(in crate::deck::enumerate) ccap: [f64; 4],
    pub(in crate::deck::enumerate) dom_on: bool,
    pub(in crate::deck::enumerate) lazy: bool,
    pub(in crate::deck::enumerate) cf_no: usize,
    pub(in crate::deck::enumerate) nc: usize,
    pub(in crate::deck::enumerate) rbsl: f64,
    pub(in crate::deck::enumerate) rbc: bool,
}

/// Read-only views into preparation's tables. The lazy-bound callbacks borrow those same
/// tables, so preparation retains their ownership until traversal returns.
#[derive(Clone, Copy)]
pub(in crate::deck::enumerate) struct TraversalTables<'tab> {
    pub(in crate::deck::enumerate) ebit: &'tab [u128],
    pub(in crate::deck::enumerate) groups_all: &'tab [&'tab Vec<usize>],
    pub(in crate::deck::enumerate) sb_rows: &'tab [Option<Vec<(f64, f64)>>],
    pub(in crate::deck::enumerate) lp: &'tab [LandPre],
    pub(in crate::deck::enumerate) lbit: &'tab [u8],
    pub(in crate::deck::enumerate) all_lk: u8,
    pub(in crate::deck::enumerate) oq: &'tab [(usize, usize, f64, usize)],
    pub(in crate::deck::enumerate) og: &'tab [(usize, usize, usize)],
    pub(in crate::deck::enumerate) orow: &'tab [crate::skills::OutfitPre],
    pub(in crate::deck::enumerate) zm: &'tab [u32],
    pub(in crate::deck::enumerate) nm: &'tab [u32],
    pub(in crate::deck::enumerate) alt: &'tab [(usize, usize)],
}

/// Reuse preparation's odds buffers and retain diagnostic rows across configurations.
pub(in crate::deck::enumerate) struct TraversalScratch {
    pub(in crate::deck::enumerate) out: Vec<(f64, Sel)>,
    pub(in crate::deck::enumerate) bestv: f64,
    pub(in crate::deck::enumerate) dw: Vec<f64>,
    pub(in crate::deck::enumerate) dg: Vec<f64>,
    pub(in crate::deck::enumerate) av: Vec<f64>,
    pub(in crate::deck::enumerate) rbs: Vec<Vec<f64>>,
}

impl DeckSpace<'_> {
    /// Traverse the prepared configurations in their visiting order (`ConfigOrder`): each one's root tests, then its
    /// tables and the depth-first walk of its decks (`Traversal::visit`), until one ends the search.
    pub(in crate::deck::enumerate) fn traverse_configurations(
        &self,
        model: &Model,
        clauses: &[(Vec<(usize, f64)>, f64)],
        controls: SearchControl,
        tables: TraversalTables<'_>,
        queue: ConfigurationQueue,
        scratch: TraversalScratch,
        val: impl Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64,
        ratio_at: impl Fn(usize, &[f64], &[f64]) -> f64,
        rpk_of: impl Fn(&[f64], &[f64], &[u32], u64) -> GTab,
        cfg_odds: impl Fn(&[(usize, usize)], &GTab, &mut [f64], &mut [f64]),
        cfg_root: impl Fn(&[(usize, usize)], &GTab, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        cf_ratios: &CapFreeRatios<'_>,
    ) -> SearchOutcome {
        let t_dfs = Instant::now();
        let mut tr = Traversal::new(self, model, clauses, controls, tables, scratch, val);
        let mut order = ConfigOrder::new(queue, controls);
        while let Some(cf) = order.next(self, tr.bestv, &rpk_of, &cfg_odds, &cfg_root) {
            if tr.past_bar(&cf) {
                break;
            }
            if let Some(stop) = tr.visit(&cf, &ratio_at, cf_ratios) {
                return SearchOutcome { decks: tr.out, incomplete: Some(stop) };
            }
        }
        SPT.with(|c| {
            let mut v = c.get();
            v.1 += t_dfs.elapsed().as_secs_f64();
            c.set(v);
        });
        SearchOutcome { decks: tr.out, incomplete: None }
    }
}

/// A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
/// whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
/// is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
/// r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
pub(in crate::deck::enumerate) struct BorrowBounds {
    pub(in crate::deck::enumerate) own: Vec<(Vec<f64>, Vec<f64>)>,
    pub(in crate::deck::enumerate) one: Vec<(Vec<f64>, Vec<f64>)>,
    pub(in crate::deck::enumerate) rown: Vec<Vec<f64>>,
    pub(in crate::deck::enumerate) rone: Vec<Vec<f64>>,
}

/// The one-borrowed-card tables of one configuration's groups and their suffixes (`own`: no borrowed card left to
/// place; `one`: at most one, in one group); empty without borrowing (`one_b` false)
pub(in crate::deck::enumerate) struct BorrowTables {
    pub(in crate::deck::enumerate) one_b: bool,
    pub(in crate::deck::enumerate) gbs: Vec<std::rc::Rc<BorrowBounds>>,
    pub(in crate::deck::enumerate) so: Vec<(Vec<f64>, Vec<f64>)>,
    pub(in crate::deck::enumerate) s1: Vec<(Vec<f64>, Vec<f64>)>,
    pub(in crate::deck::enumerate) po: Vec<Vec<f64>>,
    pub(in crate::deck::enumerate) p1: Vec<Vec<f64>>,
}

/// The node bound's per-parent rows (see `Walk::hopeful`), per (depth, borrowing variant); parent ids never repeat
/// within a search
/// (`fa`: A's factors, row ix * nu; `pp`: their products from 1.0 over the first t, row ix * (nu + 1))
pub(in crate::deck::enumerate) struct ParentTargetBounds {
    pub(in crate::deck::enumerate) pid: usize,
    pub(in crate::deck::enumerate) fa: Vec<f64>,
    pub(in crate::deck::enumerate) pp: Vec<f64>,
}

/// The same for B, per depth (B has no borrowing variant): the parent's odds 1 - ppw, 1 - ppg, and per row, built
/// at the first child that evaluates its B (`st`: the parent id it was built for), its factors and their products
pub(in crate::deck::enumerate) struct ParentRatioBounds {
    pub(in crate::deck::enumerate) pid: usize,
    pub(in crate::deck::enumerate) pdw: [f64; 32],
    pub(in crate::deck::enumerate) pdg: [f64; 32],
    pub(in crate::deck::enumerate) st: Vec<usize>,
    pub(in crate::deck::enumerate) fb: Vec<f64>,
    pub(in crate::deck::enumerate) pb: Vec<f64>,
}

/// What every configuration of one traversal reads: the search, its controls and tables, the clause value (`val`)
pub(in crate::deck::enumerate) struct TraversalInputs<'run, 'space, V> {
    pub(in crate::deck::enumerate) sp: &'run DeckSpace<'space>,
    pub(in crate::deck::enumerate) model: &'run Model<'run>,
    pub(in crate::deck::enumerate) clauses: &'run [(Vec<(usize, f64)>, f64)],
    pub(in crate::deck::enumerate) ctl: SearchControl,
    pub(in crate::deck::enumerate) tb: TraversalTables<'run>,
    pub(in crate::deck::enumerate) val: V,
    pub(in crate::deck::enumerate) noq: usize,
    // the per-parent rows need every outfit row to read the nu targets
    pub(in crate::deck::enumerate) a_rows: bool,
    pub(in crate::deck::enumerate) oqq: Vec<f64>,
}

/// One traversal: its inputs, the buffers and caches its configurations share, the decks kept and the bar
struct Traversal<'run, 'space, V> {
    inp: TraversalInputs<'run, 'space, V>,
    ns: NodeScratch,
    cs: ConfigScratch,
    gbc: HashMap<(u64, usize), std::rc::Rc<BorrowBounds>>,
    out: Vec<(f64, Sel)>,
    bestv: f64,
}

impl<'run, 'space, V: Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64> Traversal<'run, 'space, V> {
    fn new(
        sp: &'run DeckSpace<'space>,
        model: &'run Model<'run>,
        clauses: &'run [(Vec<(usize, f64)>, f64)],
        ctl: SearchControl,
        tb: TraversalTables<'run>,
        scratch: TraversalScratch,
        val: V,
    ) -> Self {
        let TraversalScratch { out, bestv, dw, dg, av, rbs } = scratch;
        let noq = tb.oq.len();
        let a_rows = tb.orow.iter().all(|o| o.len() == ctl.nu);
        let oqq: Vec<f64> = tb.oq.iter().map(|x| x.2).collect();
        let z = [0.0f64; 32];
        Self {
            inp: TraversalInputs { sp, model, clauses, ctl, tb, val, noq, a_rows, oqq },
            ns: NodeScratch {
                dw,
                dg,
                av,
                cvx: vec![],
                pdwx: vec![],
                pdgx: vec![],
                kab: vec![],
                kr: vec![],
                kab_have: vec![],
                kr_have: vec![],
                hc: vec![],
                hb: vec![],
                tw: z,
                tg: z,
                told: z,
                bdw: z,
                bdg: z,
                brow: z,
                bw: z,
                bg: z,
                dpr: z,
                rbs,
            },
            cs: ConfigScratch {
                sufw: vec![],
                sufg: vec![],
                spf: vec![],
                csuf: vec![],
                goff: vec![],
                avail: vec![],
                path: DeckPath { sel: vec![], pw: vec![], pg: vec![], pid: 0 },
            },
            gbc: HashMap::new(),
            out,
            bestv,
        }
    }

    /// No configuration from `cf` on can reach the bar (they come in root order)
    fn past_bar(&self, cf: &SearchConfig) -> bool {
        let SearchControl { best, cfm, .. } = self.inp.ctl;
        (best && cf.root * (1.0 + 1e-9) < self.bestv) || (!best && !cfm && cf.root * (1.0 + 1e-9) < 1.0)
    }

    /// Visit one configuration: its split root bound, its tables, its state-conditioned root bound, then the walk of
    /// its decks. Some: the search stops there
    fn visit(
        &mut self,
        cf: &SearchConfig,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
        cf_ratios: &CapFreeRatios<'_>,
    ) -> Option<SearchStop> {
        let Self { inp, ns, cs, gbc, out, bestv } = self;
        let SearchControl { best, nu, size, fixed, cfm, cf_lim, cfs, sbm, sb_bar, sb_u, cf_no, nc, rbsl, .. } = inp.ctl;
        let TraversalTables { groups_all, sb_rows, lp, lbit, all_lk, .. } = inp.tb;
        let sp = inp.sp;
        // (the split's root bound: skip, the order kept)
        if (best && cf.prune_bound * rbsl * (1.0 + 1e-9) < *bestv)
            || (!best && !cfm && cf.prune_bound * rbsl * (1.0 + 1e-9) < 1.0)
        {
            return None;
        }
        let km = sp.key_miss(cf.code);
        let (mw, mg, tmk) = (&km.0, &km.1, (&*km.2));
        let gs: Vec<(&[usize], usize)> = cf.gs.iter().map(|&(i, c)| ((&**groups_all[i]), c)).collect();
        // per group of this configuration: its key's tables (gmin[g] = gm[group], rp[g] = rpk[group])
        // (blocks: gmin[g]'s row r by `gm_w`, `gm_g`; rp[g]'s column ix at ix (size + 1))
        let gmin: Vec<&[f64]> = cf.gs.iter().map(|&(i, _)| cf.gm.grp(i)).collect();
        let rp: Vec<&[f64]> = cf.gs.iter().map(|&(i, _)| cf.rpk.grp(i)).collect();
        cs.fill_suffixes(cf, &gmin, &rp, nu, nc, size);
        let (doms, doff, dsuf) = inp.dominance(cf, &gs, mw, mg);
        let bt = inp.borrow_tables(gbc, cf, &gs, mw, mg, ratio_at);
        ns.cvx.clear();
        ns.cvx.resize(inp.clauses.len(), 0.0);
        ns.pdwx.clear();
        ns.pdwx.resize(nu, 0.0);
        ns.pdgx.clear();
        ns.pdgx.resize(nu, 0.0);
        let ng = gs.len();
        if !best {
            cs.fill_class_suffixes(&gs, &sp.cvm);
        }
        // the configuration's state-conditioned bounds (`StateBound`) and its root: no deck of it can reach the bar
        let sbc = inp.state_cfg(cf, &gs);
        let mut sbb = StateBound::new(sbc, &gs, &cf.gs, inp.model, sb_u, sb_rows, lp, cf_ratios, nu, cf_no, fixed, cfm);
        let sb_root = sbb.root(cfs, sb_bar, sbm);
        if cfm {
            // (the cap-free band: past its node limit it stops; CF_CHECK passes the root at cfs times its bound)
            if sp.nodes.get() > cf_lim {
                return Some(SearchStop::CapFreeNodeLimit);
            }
            if sb_root * cfs.max(1.0) * (1.0 + 1e-9) < sb_bar {
                SBN.with(|c| c.borrow_mut().0 += 1);
                return None;
            }
        } else if sbm > 0 && sb_root * (1.0 + 1e-9) < sb_bar {
            SBN.with(|c| c.borrow_mut().0 += 1);
            return None;
        }
        // (CF_CHECK: the bounds of the nodes on the current path, per depth, the root's at 0)
        let cf_path = if cfs > 0.0 { vec![sb_root] } else { vec![] };
        let nrows = gs.iter().map(|x| x.1).sum::<usize>() + 1;
        let st = &mut cs.path;
        st.sel.clear();
        st.pw.clear();
        st.pw.resize(nrows * nu, 1.0);
        st.pg.clear();
        st.pg.resize(nrows * nu, 1.0);
        ns.grow_parent_rows(2 * (gs.iter().map(|x| x.1).sum::<usize>() + 1), nrows, inp.noq, nu, inp.a_rows);
        // per (group g, cards left, borrowing allowed): the per-target suffix factors (a, b) and the per-outfit
        // ratio products r depend on nothing else, so they are constants of the configuration: tabulated on
        // first use of their row (the same expressions, so the same f64s as computing them per node); row
        // (loff[g] + left) * ncan + can, a row of 2 nu factors (a then b) and one of noq ratios
        let ncan = if bt.one_b { 2 } else { 1 };
        let mut loff = vec![0usize; ng + 1];
        for g in 0..ng {
            loff[g + 1] = loff[g] + gs[g].1 + 1;
        }
        ns.reset_constants(loff[ng] * ncan, nu, nc);
        // (a branch that can no longer place every locked card is dropped)
        if !cs.fill_avail(&gs, lbit, all_lk) {
            return None;
        }
        let ConfigScratch { sufw, sufg, spf, csuf, goff, avail, path } = cs;
        let mut walk = Walk {
            inp,
            ns,
            out,
            bestv: *bestv,
            cf,
            filler: cf.filler,
            plain: cf.plain,
            gs: &gs,
            gmin: &gmin,
            rp: &rp,
            mw,
            mg,
            tmk,
            sufw,
            sufg,
            spf,
            doms: &doms,
            doff: &doff,
            dsuf: &dsuf,
            bt: &bt,
            csuf,
            goff,
            avail,
            loff: &loff,
            ncan,
            ng,
            total_slots: gs.iter().map(|x| x.1).sum(),
            sbb,
            sb_root,
            sb_d1: f64::INFINITY,
            cf_path,
            cf_over: false,
            dwa: [0.0f64; 32],
            dga: [0.0f64; 32],
        };
        let stop = if gs.is_empty() {
            walk.leaf(&[], &[1.0; 32][..nu], &[1.0; 32][..nu], 0)
        } else {
            walk.rec(path, 0, 0, gs[0].1, 0, 0, 0, 0, 0)
        };
        *bestv = walk.bestv;
        if stop {
            return Some(SearchStop::OutputLimit);
        }
        if walk.cf_over {
            return Some(SearchStop::CapFreeNodeLimit);
        }
        None
    }
}

impl<V> TraversalInputs<'_, '_, V> {
    /// The dominance tables of `cf`'s groups (`dom_table`), and per flat index f (the groups' entries in turn) the
    /// characters of the entries from f on (`dsuf`, group g's from `doff[g]`); empty without the dominance skip
    fn dominance(
        &self,
        cf: &SearchConfig,
        gs: &[(&[usize], usize)],
        mw: &[f64],
        mg: &[f64],
    ) -> (Vec<std::rc::Rc<Vec<Vec<(u32, u32)>>>>, Vec<usize>, Vec<u128>) {
        let SearchControl { dom_on, set_id, .. } = self.ctl;
        let TraversalTables { ebit, groups_all, lbit, .. } = self.tb;
        let doms: Vec<std::rc::Rc<Vec<Vec<(u32, u32)>>>> = if dom_on {
            cf.gs.iter().map(|&(i, _)| self.sp.dom_table(set_id, cf.code, i, groups_all[i], mw, mg, lbit)).collect()
        } else {
            vec![]
        };
        let (mut doff, mut dsuf): (Vec<usize>, Vec<u128>) = (vec![0], vec![]);
        if dom_on {
            for &(l, _) in gs {
                for &j in l {
                    dsuf.push(ebit[j]);
                }
                doff.push(dsuf.len());
            }
            dsuf.push(0);
            for f in (0..dsuf.len() - 1).rev() {
                dsuf[f] |= dsuf[f + 1];
            }
        }
        (doms, doff, dsuf)
    }

    /// Group gi's `BorrowBounds` under the key's miss chances `mw`, `mg` (`ratio_at`: a card's ratio per column)
    fn borrow_bounds(
        &self,
        gi: usize,
        mw: &[f64],
        mg: &[f64],
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
    ) -> BorrowBounds {
        let SearchControl { nu, size, nc, .. } = self.ctl;
        let l = self.tb.groups_all[gi];
        let (own, bor): (Vec<usize>, Vec<usize>) = l.iter().partition(|&&j| !self.sp.ebor[j]);
        let mut om = vec![(vec![1.0f64; nu], vec![1.0f64; nu]); size + 1];
        let mut bm = (vec![1.0f64; nu], vec![1.0f64; nu]);
        for t in 0..nu {
            let mut a: Vec<f64> = own.iter().map(|&j| mw[j * nu + t]).collect();
            a.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let mut b: Vec<f64> = own.iter().map(|&j| mg[j * nu + t]).collect();
            b.sort_by(|x, y| x.partial_cmp(y).unwrap());
            for r in 1..=size {
                om[r].0[t] = om[r - 1].0[t] * a.get(r - 1).copied().unwrap_or(1.0);
                om[r].1[t] = om[r - 1].1[t] * b.get(r - 1).copied().unwrap_or(1.0);
            }
            for &j in &bor {
                bm.0[t] = bm.0[t].min(mw[j * nu + t]);
                bm.1[t] = bm.1[t].min(mg[j * nu + t]);
            }
        }
        let mut one = om.clone();
        for r in 1..=size {
            for t in 0..nu {
                one[r].0[t] = om[r].0[t].min(om[r - 1].0[t] * bm.0[t]);
                one[r].1[t] = om[r].1[t].min(om[r - 1].1[t] * bm.1[t]);
            }
        }
        let (mut x, mut y) = (vec![0.0f64; nu], vec![0.0f64; nu]);
        // (`ratio_at`: without a split column, every zm 0 and cix the identity, the plain ratio)
        let mut ratio = |j: usize| -> Vec<f64> {
            for t in 0..nu {
                x[t] = 1.0 - mw[j * nu + t];
                y[t] = 1.0 - mg[j * nu + t];
            }
            (0..nc).map(|i| ratio_at(i, &x, &y)).collect()
        };
        let ro: Vec<Vec<f64>> = own.iter().map(|&j| ratio(j)).collect();
        let rb: Vec<Vec<f64>> = bor.iter().map(|&j| ratio(j)).collect();
        let (mut rown, mut rone) = (vec![], vec![]);
        for i in 0..nc {
            let mut r: Vec<f64> = ro.iter().map(|v| v[i]).collect();
            r.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let best_b = rb.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max);
            let mut pre = vec![1.0f64; size + 1];
            for k in 1..=size {
                pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0);
            }
            let mut p1 = pre.clone();
            if best_b > f64::NEG_INFINITY {
                for k in 1..=size {
                    p1[k] = pre[k].max(pre[k - 1] * best_b);
                }
            }
            rown.push(pre);
            rone.push(p1);
        }
        BorrowBounds { own: om, one, rown, rone }
    }

    /// `cf`'s one-borrowed-card tables (each group's `BorrowBounds` cached per (key, group) in `gbc`)
    fn borrow_tables(
        &self,
        gbc: &mut HashMap<(u64, usize), std::rc::Rc<BorrowBounds>>,
        cf: &SearchConfig,
        gs: &[(&[usize], usize)],
        mw: &[f64],
        mg: &[f64],
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
    ) -> BorrowTables {
        let SearchControl { nu, nc, .. } = self.ctl;
        let sp = self.sp;
        let one_b = sp.allowed_borrow == 1 && gs.iter().any(|(l, _)| l.iter().any(|&j| sp.ebor[j]));
        let gbs: Vec<std::rc::Rc<BorrowBounds>> = if one_b {
            cf.gs
                .iter()
                .map(|&(i, _)| {
                    std::rc::Rc::clone(
                        gbc.entry((cf.code, i))
                            .or_insert_with(|| std::rc::Rc::new(self.borrow_bounds(i, mw, mg, ratio_at))),
                    )
                })
                .collect()
        } else {
            vec![]
        };
        let ngs = cf.gs.len();
        // (allocated only when used)
        let nb = if one_b { ngs + 1 } else { 0 };
        // (vec![x; 0] would still build x: none without borrowing)
        let (mut so, mut s1) = if nb > 0 {
            (vec![(vec![1.0f64; nu], vec![1.0f64; nu]); nb], vec![(vec![1.0f64; nu], vec![1.0f64; nu]); nb])
        } else {
            (vec![], vec![])
        };
        let (mut po, mut p1) =
            if nb > 0 { (vec![vec![1.0f64; nc]; nb], vec![vec![1.0f64; nc]; nb]) } else { (vec![], vec![]) };
        if one_b {
            for g in (0..ngs).rev() {
                let (gb, c) = (&gbs[g], cf.gs[g].1);
                for t in 0..nu {
                    so[g].0[t] = gb.own[c].0[t] * so[g + 1].0[t];
                    so[g].1[t] = gb.own[c].1[t] * so[g + 1].1[t];
                    s1[g].0[t] = (gb.one[c].0[t] * so[g + 1].0[t]).min(gb.own[c].0[t] * s1[g + 1].0[t]);
                    s1[g].1[t] = (gb.one[c].1[t] * so[g + 1].1[t]).min(gb.own[c].1[t] * s1[g + 1].1[t]);
                }
                for i in 0..nc {
                    po[g][i] = gb.rown[i][c] * po[g + 1][i];
                    p1[g][i] = (gb.rone[i][c] * po[g + 1][i]).max(gb.rown[i][c] * p1[g + 1][i]);
                }
            }
        }
        BorrowTables { one_b, gbs, so, s1, po, p1 }
    }

    /// The state-conditioned bound's tables of `cf` (groups `gs`), cached per (set, key, real (2, 2), pal entries):
    /// chain positions (sorted kinds, the key's state distribution projected on the positions whose group has chain
    /// targets), per group its slots' positions and, per chain level, each target's lowest misses over the group's
    /// entries (a slot may take any of them, the same one twice: a relaxation). None without the bound (`sbm` 0)
    fn state_cfg(&self, cf: &SearchConfig, gs: &[(&[usize], usize)]) -> Option<std::rc::Rc<SbCfg>> {
        let SearchControl { nu, nk, set_id, sbm, .. } = self.ctl;
        let (sp, sb_rows, filler, ng) = (self.sp, self.tb.sb_rows, cf.filler, gs.len());
        let sbkey = (set_id, cf.code, cf.code_real22, cf.np);
        if sbm == 0 {
            None
        } else if let Some(r) = sp.sbcfg.borrow().get(&sbkey).cloned() {
            Some(r)
        } else {
            let mut kinds: Vec<(Kind, usize)> = vec![]; // (kind, group, or usize::MAX for a filler)
            for (g, &(i, c)) in cf.gs.iter().enumerate() {
                if i < nk {
                    for _ in 0..c {
                        kinds.push((sp.kinds_all[i], g));
                    }
                }
            }
            for _ in 0..filler {
                kinds.push(((2, 2, 0), usize::MAX));
            }
            kinds.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let kl: Vec<Kind> = kinds.iter().map(|x| x.0).collect();
            let full = if kl.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kl) };
            let has_chain: Vec<bool> = gs.iter().map(|(l, _)| l.iter().any(|&j| !sp.srcs[j].cm.is_empty())).collect();
            let rel_pos: Vec<usize> =
                (0..kinds.len()).filter(|&p| kinds[p].1 != usize::MAX && has_chain[kinds[p].1]).collect();
            let mut slots: Vec<Vec<usize>> = vec![vec![]; ng];
            for (ri, &p) in rel_pos.iter().enumerate() {
                slots[kinds[p].1].push(ri);
            }
            for g in 0..ng {
                while slots[g].len() < gs[g].1 {
                    slots[g].push(usize::MAX);
                }
            }
            // (with at most 16 positions and levels below 16, the projected key packed 4 bits per position, the first
            // highest: the same sums, each in the order of `full`, sorted the same way)
            let states: Vec<(Vec<u8>, f64)> = if rel_pos.len() <= 16 && kl.iter().all(|k| k.1 < 16) {
                let mut proj: crate::FxMap<u64, f64> = crate::FxMap::default();
                for (x, w) in full.iter() {
                    let key = rel_pos.iter().fold(0u64, |m, &p| m << 4 | u64::from(x[p]));
                    *proj.entry(key).or_insert(0.0) += w;
                }
                let mut v: Vec<(u64, f64)> = proj.into_iter().collect();
                v.sort_unstable_by_key(|a| a.0);
                let np = rel_pos.len();
                v.into_iter()
                    .map(|(k, w)| ((0..np).map(|i| (k >> (4 * (np - 1 - i)) & 15) as u8).collect(), w))
                    .collect()
            } else {
                let mut proj: crate::FxMap<Vec<u8>, f64> = crate::FxMap::default();
                for (x, w) in full.iter() {
                    let key: Vec<u8> = rel_pos.iter().map(|&p| x[p]).collect();
                    *proj.entry(key).or_insert(0.0) += w;
                }
                let mut states: Vec<(Vec<u8>, f64)> = proj.into_iter().collect();
                states.sort_by(|a, b| a.0.cmp(&b.0));
                states
            };
            let stops: Vec<usize> =
                cf.gs.iter().map(|&(i, _)| if i < nk { sp.kinds_all[i].1 as usize } else { 0 }).collect();
            let gmin: Vec<Vec<(f64, f64)>> = (0..ng)
                .map(|g| {
                    let mut m = vec![(1.0f64, 1.0f64); (stops[g] + 1) * nu];
                    for &j in gs[g].0 {
                        if let Some(r) = &sb_rows[j] {
                            for (x, y) in m.iter_mut().zip(r.iter()) {
                                x.0 = x.0.min(y.0);
                                x.1 = x.1.min(y.1);
                            }
                        }
                    }
                    m
                })
                .collect();
            let r = std::rc::Rc::new(SbCfg { states, slots, gmin, kl, rel_pos, order: std::cell::OnceCell::default() });
            sp.sbcfg.borrow_mut().insert(sbkey, std::rc::Rc::clone(&r));
            Some(r)
        }
    }
}
