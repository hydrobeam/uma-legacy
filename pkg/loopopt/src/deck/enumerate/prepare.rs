//! Search legality and the shared bounds used by enumeration.
use super::{
    ConfigurationScratch, ConfigurationTables, OutfitBounds, RatioPrefixes, SearchControl, SearchMode, SearchOutcome,
    SearchSetup, SearchStop, TraversalScratch, TraversalTables, prepare_outfit_bounds, prepare_ratio_prefixes,
    rb_check,
};
use crate::deck::{
    CF_LIM, CF_ON, CapFreeRatios, DeckSpace, GTab, Instant, LandPre, Model, SB_ON, SB_USED, SPT, Sel, cf_check, gm_g,
    gm_w, sb_mode, src_levels, top_desc,
};

impl DeckSpace<'_> {
    /// Resolve legality and proof modes before preparing configuration bounds or visiting a deck.
    pub(in crate::deck::enumerate) fn configure_search(
        &self,
        model: &Model,
        us: &[usize],
        ids: &[u32],
        clauses: &[(Vec<(usize, f64)>, f64)],
        mode: SearchMode,
    ) -> Result<SearchSetup, SearchOutcome> {
        // Keep the numerical kernel's existing inputs and branch order. These unused-mode defaults were
        // previously repeated at every call site.
        let (best, seed, rho, cap) = match mode {
            SearchMode::Best { seed } => (true, seed, 1.0, 1),
            SearchMode::Band { ratio_cap, max_decks } => (false, f64::NEG_INFINITY, ratio_cap, max_decks),
        };
        let nu = self.unc.len();
        let size = self.dj.deck_size;
        if self.locked.iter().any(|&l| ids.iter().any(|&id| self.cards[l].blocks(id))) {
            return Err(SearchOutcome { decks: vec![], incomplete: None });
        }
        let (legal_rc, set_id) = self.reduced_cached(model, us, ids);
        let lt = self.legal_tabs(&legal_rc, set_id, ids);
        if lt.over {
            return Err(SearchOutcome { decks: vec![], incomplete: Some(SearchStop::CharacterLimit) });
        }
        let nplain = lt.nplain;
        let nk = self.kinds_all.len();
        let ki22 = self.kinds_all.iter().position(|&k| k == (2, 2, 0)).unwrap();
        let kpos: Vec<u64> = (0..nk).map(|i| 7u64.pow(i as u32)).collect();
        assert!(nk < 64, "{nk} chain kinds: the packed key holds fewer than 64");
        let fixed = model.fixed_factor();
        // the state-conditioned node bound (SBOUND; the career's band pass, one trainee): per legal entry its miss rows
        // Per chain level; the ordinary bound is off when an entry fails group_assoc certification (`state_value`).
        // The cap-free band (`CF_ON`, one trainee at one chance; `Env::best_deck_cf`): two proven bounds at every node and
        // leaf (`cf_cheap`, then `cf_ub`, on the state-conditioned tables of mode 2) are its only prunes, no cap in any
        // test; it needs every legal entry `cf_ok` (else UnsupportedCapFreeProof) and stops past CF_LIM nodes
        // (CapFreeNodeLimit).
        let cfm = !best && clauses.len() == 1 && clauses[0].0.len() == 1 && CF_ON.with(std::cell::Cell::get);
        if cfm && !legal_rc.iter().all(|&j| self.srcs[j].cf_ok) {
            return Err(SearchOutcome { decks: vec![], incomplete: Some(SearchStop::UnsupportedCapFreeProof) });
        }
        let cf_lim = if cfm { CF_LIM.with(std::cell::Cell::get) } else { usize::MAX };
        let cfs = if cfm { cf_check() } else { 0.0 };
        let sbm = if cfm {
            2
        } else if !best && clauses.len() == 1 && clauses[0].0.len() == 1 && SB_ON.with(std::cell::Cell::get) {
            sb_mode()
        } else {
            0
        };
        let sb_rows: std::rc::Rc<Vec<Option<Vec<(f64, f64)>>>> =
            if sbm > 0 && (cfm || !legal_rc.iter().any(|&j| self.srcs[j].multi)) {
                let hit = self.sbrows.borrow().get(&set_id).cloned();
                hit.unwrap_or_else(|| {
                    let r = std::rc::Rc::new(
                        (0..self.rel.len())
                            .map(|j| legal_rc.contains(&j).then(|| src_levels(&self.srcs[j], self.kind[j].1, nu)))
                            .collect::<Vec<_>>(),
                    );
                    self.sbrows.borrow_mut().insert(set_id, std::rc::Rc::clone(&r));
                    r
                })
            } else {
                std::rc::Rc::new(vec![])
            };
        let sbm = if sb_rows.is_empty() { 0 } else { sbm };
        let sb_bar = if sbm > 0 { clauses[0].1 } else { 0.0 };
        let (sb_u, sb_q) = if sbm > 0 { (us[clauses[0].0[0].0], clauses[0].0[0].1) } else { (0, 0.0) };
        // the trainee's `land_factor` per (outfit, target), row o * nu + k (`LandPre`: the same values)
        let lp: Vec<LandPre> = if sbm > 0 {
            (0..model.opts[sb_u].len())
                .flat_map(|o| (0..nu).map(move |k| LandPre::new(model.opts[sb_u][o].0[k], model.ews[sb_u][o][k], sb_q)))
                .collect()
        } else {
            vec![]
        };
        let rho = if sbm == 2 { 1e9 } else { rho };
        if sbm > 0 {
            SB_USED.with(|c| c.set(sbm));
        }
        // the cap per structure class (`cap_class`), at most rho (uncapped with SBOUND=2); a deck passes at its own
        // class's cap, a node at the cap of the class of every deck it can still reach (its cards and every candidate
        // left: a superset of any completion's, so a cap at least as high)
        let ccap: [f64; 4] = if sbm == 2 {
            [rho; 4]
        } else {
            let c = self.caps();
            [c[0].min(rho), c[1].min(rho), c[2].min(rho), c[3].min(rho)]
        };
        Ok(SearchSetup {
            best,
            seed,
            rho,
            cap,
            nu,
            size,
            nplain,
            nk,
            ki22,
            kpos,
            set_id,
            lt,
            fixed,
            cfm,
            cf_lim,
            cfs,
            sbm,
            sb_rows,
            sb_bar,
            sb_u,
            sb_q,
            lp,
            ccap,
            legal_entries: legal_rc,
        })
    }

    /// Prepare shared bounds, then build and traverse configurations. The callbacks borrow
    /// these tables and their lazy caches for both phases; none need copying or dynamic dispatch.
    pub(in crate::deck::enumerate) fn search_with_bounds(
        &self,
        model: &Model,
        us: &[usize],
        clauses: &[(Vec<(usize, f64)>, f64)],
        setup: SearchSetup,
    ) -> SearchOutcome {
        let SearchSetup {
            best,
            seed,
            rho,
            cap,
            nu,
            size,
            nplain,
            nk,
            ki22,
            kpos,
            set_id,
            lt,
            fixed,
            cfm,
            cf_lim,
            cfs,
            sbm,
            sb_rows,
            sb_bar,
            sb_u,
            sb_q,
            lp,
            ccap,
            legal_entries: _legal_entries,
        } = setup;
        let ebit: &[u128] = &lt.ebit;
        let (by_kind, pal_ents) = (&lt.by_kind, &lt.pal_ents);
        let val = |cl: &(Vec<(usize, f64)>, f64), dw: &[f64], dg: &[f64]| {
            fixed * cl.0.iter().map(|&(k, q)| model.trainee_fast(us[k], q, dw, dg)).sum::<f64>()
        };
        // every chance and outfit odd in [0, 1] and a fixed factor >= 0: then a deck's value rises with each of its odds
        // (white < gold) and every quantity in its evaluation is in [0, 1] up to rounding, which the dominance skip
        // (`dom`), the lazy configurations (`PendingConfig`) and the key bound (`keyskip`) rely on
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let valid = fixed >= 0.0
            && clauses
                .iter()
                .all(|cl| cl.0.iter().all(|&(k, q)| in01(q) && model.ews[us[k]].iter().flatten().all(|&x| in01(x))));
        let dom_on = best && valid;
        // the product search builds its configurations lazily (`PendingConfig`): roots and ratio tables only for those it may
        // visit
        let lazy = best && valid;
        let t_cfg = Instant::now();
        let out: Vec<(f64, Sel)> = vec![];
        let bestv = if best { seed } else { f64::NEG_INFINITY };
        let counts = self.counts_all();
        let groups_all: Vec<&Vec<usize>> = by_kind.iter().chain(std::iter::once(pal_ents)).collect();
        // the cap-free band's ratio tables (`CapFreeRatios`), built on first use
        let cf_no = if cfm { model.opts[sb_u].len() } else { 0 };
        let cf_ratios = CapFreeRatios::new(cfm, nu, cf_no, nk, &lp, &sb_rows, &groups_all, &self.kinds_all);
        // per group, its entries' `cv_mask`s ORed
        let gcvm: Vec<u64> = groups_all.iter().map(|l| l.iter().fold(0u64, |m, &j| m | self.cvm[j])).collect();
        // locked cards each group can hold (a configuration whose groups cannot hold them all is skipped)
        let (lbit, all_lk) = {
            let x = self.lock_bits();
            ((&*x.0), x.1)
        };
        let glk: &[u8] = &lt.glk;
        let mut dw = vec![0.0f64; nu];
        let mut dg = vec![0.0f64; nu];
        // (scratch of the configuration build: a candidate's groups, copied into its SearchConfig only when kept; the root's
        // clause sums)
        let mut gsb: Vec<(usize, usize)> = vec![];
        let (mut cvr, mut cvro) = (vec![0.0f64; clauses.len()], vec![0.0f64; clauses.len()]);
        let OutfitBounds { oq, og, zero: _zero, av, orow, p0, cix, zm, nm, p0n, alt, nc } =
            prepare_outfit_bounds(model, us, clauses, nu);
        // a card's ratio for column c (its single-card odds x, y): N's (a split column), the plain ratio, or infinite
        let ratio_at = |c: usize, x: &[f64], y: &[f64]| -> f64 {
            let i = cix[c];
            let (_, u, q, o) = oq[i];
            if zm[c] != 0 {
                orow[i].eval_in(q, x, y, nm[c]) / p0n[c]
            } else if p0[i] > 0.0 {
                model.outfit_fast(u, o, q, x, y) / p0[i]
            } else {
                f64::INFINITY
            }
        };
        // (RB_CHECK=s: the node bounds on the current path, per depth its clause sums, the root's at depth 0; nodes pass
        // at s times their bound, so more leaves are checked, the results unchanged)
        let rbsl = if cfm { 0.0 } else { rb_check() };
        let rbc = rbsl > 0.0;
        let rbsl = if rbc { rbsl } else { 1.0 };
        let rbs: Vec<Vec<f64>> = vec![];
        // the root bound's ratio products (row 0 of the ratio suffix table, which is built only for configurations the
        // DFS expands)
        let mut sp0 = vec![1.0f64; nc];
        let RatioPrefixes { f0, cmk, cpre } = prepare_ratio_prefixes(nu, &oq, &orow, &p0, &cix, &zm, &nm, &p0n);
        // per kind group in `gneed` (the others are never read: empty) and column, the products of its entries' r best
        // ratios (the first `size` of their stable descending sort, `top_desc`), r = 0..=size
        // (a `GTab`: per group, column c's products at c (size + 1); `top`, the columns' best ratios, reused)
        let rtop: std::cell::RefCell<Vec<Vec<f64>>> =
            std::cell::RefCell::new((0..nc).map(|_| Vec::with_capacity(size)).collect());
        let rpk_of = |mw: &[f64], mg: &[f64], tmk: &[u32], gneed: u64| -> GTab {
            let (mut x, mut y) = ([0.0f64; 32], [0.0f64; 32]);
            let mut top = rtop.borrow_mut();
            let mut tab = GTab::new(groups_all.len(), gneed, nc * (size + 1));
            for (gi, l) in groups_all.iter().enumerate() {
                if gneed >> gi & 1 == 0 {
                    continue;
                }
                top.iter_mut().for_each(std::vec::Vec::clear);
                for &j in *l {
                    let tm = tmk[j];
                    let mut m = tm;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        x[t] = 1.0 - mw[j * nu + t];
                        y[t] = 1.0 - mg[j * nu + t];
                    }
                    for c in 0..nc {
                        let r = match cmk[c] {
                            None => f64::INFINITY,
                            Some((cm, den)) => {
                                let (i, s) = (cix[c], tm & cm);
                                let p = if s == 0 {
                                    cpre[c * (nu + 1) + nu]
                                } else {
                                    let (o, q, t0) = (&orow[i], oq[i].2, s.trailing_zeros() as usize);
                                    let mut p = cpre[c * (nu + 1) + t0];
                                    let mut m = cm >> t0 << t0;
                                    while m != 0 {
                                        let t = m.trailing_zeros() as usize;
                                        m &= m - 1;
                                        p *= if s >> t & 1 != 0 {
                                            o.factor_at(t, q, x[t], y[t])
                                        } else {
                                            f0[i * nu + t]
                                        };
                                    }
                                    p
                                };
                                p / den
                            }
                        };
                        top_desc(&mut top[c], size, r);
                    }
                }
                let blk = tab.grp_mut(gi);
                for (r, pre) in top.iter().zip(blk.chunks_exact_mut(size + 1)) {
                    for k in 1..=size {
                        pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0);
                    }
                }
            }
            tab
        };
        // A configuration's per-target bound (groups `gs`, the key's tables `gm`): each target's odds at the product of
        // its groups' lowest misses, from the last group down (row 0 of the suffix table the DFS builds, the same products
        // in the same order). The eager and the lazy build both take it
        let cfg_odds = |gs: &[(usize, usize)], gm: &GTab, dw: &mut [f64], dg: &mut [f64]| {
            for t in 0..nu {
                let (mut a, mut b) = (1.0f64, 1.0f64);
                for g in (0..gs.len()).rev() {
                    let gb = gm.grp(gs[g].0);
                    a *= gm_w(gb, nu, gs[g].1)[t];
                    b *= gm_g(gb, nu, gs[g].1)[t];
                }
                dw[t] = 1.0 - a;
                dg[t] = 1.0 - b;
            }
        };
        // Its root bound at those odds with the key's ratio tables `rpk`: sp0, the ratio products (sp[0] of the suffix
        // table, from the last group down), then per (chance, outfit) the lower of the per-target bound and the ratio
        // bound, summed per clause with the split (`cv`) and without (`cvo`). With ZSPLIT rows `cvo` sets the visiting
        // order (`root`, as before: the order decides which of two tied decks is found first) and the stop; the split's
        // (`prune_bound`) skips. Without the split the ratios were all infinite, so B was 0 * inf (NaN, which `min` ignores)
        // or, with no card to place, 0 * 1
        let cfg_root = |gs: &[(usize, usize)],
                        rpk: &GTab,
                        dw: &[f64],
                        dg: &[f64],
                        sp0: &mut [f64],
                        cv: &mut [f64],
                        cvo: &mut [f64]| {
            let ng = gs.len();
            for i in 0..nc {
                let mut x = 1.0f64;
                for g in (0..ng).rev() {
                    x *= rpk.grp(gs[g].0)[i * (size + 1) + gs[g].1];
                }
                sp0[i] = x;
            }
            cv.fill(0.0);
            cvo.fill(0.0);
            for &(ci, i, no) in &og {
                let q = oq[i].2;
                let (mut m, mut mo) = (0.0f64, 0.0f64);
                for o in 0..no {
                    let ix = i + o;
                    let a = orow[ix].eval(q, dw, dg);
                    let (mut t, to) = if zm[ix] != 0 {
                        (
                            a.min(orow[ix].eval_in(q, dw, dg, zm[ix]) * (p0n[ix] * sp0[ix])),
                            a.min(p0[ix] * if ng > 0 { f64::INFINITY } else { 1.0 }),
                        )
                    } else {
                        let b = a.min(p0[ix] * sp0[ix]);
                        (b, b)
                    };
                    // (a low split: the lower B, outside the visiting order)
                    for c in alt[ix].0..alt[ix].1 {
                        t = t.min(orow[ix].eval_in(q, dw, dg, zm[c]) * (p0n[c] * sp0[c]));
                    }
                    m = m.max(t);
                    mo = mo.max(to);
                }
                cv[ci] += fixed * m;
                cvo[ci] += fixed * mo;
            }
        };
        // The key bound (`keyskip`; in the product search only with a bar): a key none of whose configurations (its
        // variants: real (2, 2) and pal entries 0 ..= their most) can pass the per-target bound is skipped whole, which
        // leaves out the same configurations. Its per-target products take, for the (2, 2) and pal groups, the lowest
        // miss products over the counts any variant has (row 0 is 1.0, a group a variant lacks), the others at the key's
        // counts, multiplied in the variants' order: with factors >= 0 each partial product is at most the variant's, so
        // its odds are at least every variant's, to the bit. The value rises with every odds coordinate (`valid`), and
        // each evaluation is within about 4e-14 per (trainee, chance) of exact (every quantity in it within [0, 1] up to
        // rounding, each operation's error at most 2^-53 of one, about 300 operations), so a variant's value is at most
        // the bound's plus `key_slack` (1e-11 per pair)
        let keyskip = valid && !cfm && (!best || bestv > f64::NEG_INFINITY);
        let (mut kdw, mut kdg) = (vec![0.0f64; nu], vec![0.0f64; nu]);
        let controls = SearchControl {
            best,
            rho,
            cap,
            nu,
            size,
            nk,
            set_id,
            fixed,
            cfm,
            cf_lim,
            cfs,
            sbm,
            sb_bar,
            sb_u,
            sb_q,
            ccap,
            dom_on,
            lazy,
            cf_no,
            nc,
            rbsl,
            rbc,
        };
        let configurations = self.build_configurations(
            clauses,
            controls,
            ConfigurationTables {
                counts: &counts,
                by_kind,
                pal_ents,
                kpos: &kpos,
                groups_all: &groups_all,
                glk,
                gcvm: &gcvm,
                ki22,
                nplain,
                all_lk,
                keyskip,
            },
            ConfigurationScratch {
                dw: &mut dw,
                dg: &mut dg,
                groups: &mut gsb,
                clause_bounds: &mut cvr,
                order_bounds: &mut cvro,
                ratio_products: &mut sp0,
                key_white: &mut kdw,
                key_gold: &mut kdg,
            },
            bestv,
            val,
            rpk_of,
            cfg_odds,
            cfg_root,
        );
        SPT.with(|c| {
            let mut v = c.get();
            v.0 += t_cfg.elapsed().as_secs_f64();
            v.2 += configurations.cfgs.len();
            v.3 += counts.len();
            c.set(v);
        });
        self.traverse_configurations(
            model,
            clauses,
            controls,
            TraversalTables {
                ebit,
                groups_all: &groups_all,
                sb_rows: &sb_rows,
                lp: &lp,
                lbit,
                all_lk,
                oq: &oq,
                og: &og,
                orow: &orow,
                zm: &zm,
                nm: &nm,
                alt: &alt,
            },
            configurations,
            TraversalScratch { out, bestv, dw, dg, av, rbs },
            val,
            ratio_at,
            rpk_of,
            cfg_odds,
            cfg_root,
            &cf_ratios,
        )
    }
}
