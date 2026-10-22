//! Search legality and the shared bounds used by enumeration.
use crate::Instant;
use crate::deck::capfree::{CapFreeRatios, LandingBoundTables};
use crate::deck::chains::ChainKind;
use crate::deck::enumerate::bounds::{
    OutfitBounds, RatioColumns, RatioPrefixes, prepare_outfit_bounds, prepare_ratio_prefixes, rb_check,
};
use crate::deck::enumerate::config::{ConfigurationBounds, ConfigurationScratch, ConfigurationTables};
use crate::deck::enumerate::traverse::{PreparedTables, SearchControl, TraversalScratch};
use crate::deck::enumerate::{
    LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome, SearchSetup, SearchStop,
};
use crate::deck::joint::{CF_LIM, CF_ON, LandPre, SB_ON, SB_USED, cf_check, sb_mode, src_levels};
use crate::deck::space::{DeckSpace, GroupSlots, GroupTable, SPT, SelectedDeck, gm_g, gm_w, top_desc};
use crate::skills::Model;
impl DeckSpace<'_> {
    /// Resolve legality and proof modes before preparing configuration bounds or visiting a deck.
    pub(crate) fn configure_search(
        &self,
        model: &Model,
        trainees: &[crate::uma::UmaIndex],
        trainee_ids: &[u32],
        clauses: &[SearchClause],
        mode: SearchMode,
    ) -> Result<SearchSetup, SearchOutcome> {
        let (best, seed, rho, cap) = match mode {
            SearchMode::Best { seed } => (true, seed, 1.0, 1),
            SearchMode::Band { ratio_cap, max_decks } => (false, f64::NEG_INFINITY, ratio_cap, max_decks),
        };
        let target_count = self.target_groups.len();
        let size = self.deck_job.deck_size;
        if self.locked.iter().any(|&l| trainee_ids.iter().any(|&id| self.cards[l].blocks(id))) {
            return Err(SearchOutcome { decks: vec![], incomplete: None });
        }
        let crate::deck::space::ReducedEntries { entries: legal_rc, set_id } =
            self.reduced_cached(model, trainees, trainee_ids);
        let legal_tables = self.legal_tabs(&legal_rc, set_id, trainee_ids);
        if legal_tables.too_many_characters {
            return Err(SearchOutcome { decks: vec![], incomplete: Some(SearchStop::CharacterLimit) });
        }
        let plain_pal_count = legal_tables.plain_pal_count;
        let kind_count = self.kinds_all.len();
        let filler_kind_index = self.kinds_all.iter().position(|&k| k == ChainKind::FILLER).unwrap();
        let kind_place_values: Vec<u64> = (0..kind_count).map(|i| 7u64.pow(i as u32)).collect();
        assert!(kind_count < 64, "{kind_count} chain kinds: the packed key holds fewer than 64");
        let fixed = model.fixed_factor();
        // the state-conditioned node bound (SBOUND; the career's band pass, one trainee): per legal entry its miss rows
        // Per chain level; the ordinary bound is off when an entry fails group_assoc certification (`state_value`).
        // The cap-free band (`CF_ON`, one trainee at one chance; `Env::best_deck_cf`): two proven bounds at every node and
        // leaf (`StateBound::cheap`, then `StateBound::full`, on the state-conditioned tables of mode 2) are its only
        // prunes, no cap in any
        // test; it needs every legal entry `certified_cap_free` (else UnsupportedCapFreeProof) and stops past CF_LIM nodes
        // (CapFreeNodeLimit).
        let cap_free =
            !best && clauses.len() == 1 && clauses[0].lineages.len() == 1 && CF_ON.with(std::cell::Cell::get);
        if cap_free && !legal_rc.iter().all(|&j| self.sources[j].certified_cap_free) {
            return Err(SearchOutcome { decks: vec![], incomplete: Some(SearchStop::UnsupportedCapFreeProof) });
        }
        let cap_free_node_limit = if cap_free { CF_LIM.with(std::cell::Cell::get) } else { usize::MAX };
        let cap_free_check_scale = if cap_free { cf_check() } else { 0.0 };
        let state_bound_mode = if cap_free {
            2
        } else if !best && clauses.len() == 1 && clauses[0].lineages.len() == 1 && SB_ON.with(std::cell::Cell::get) {
            sb_mode()
        } else {
            0
        };
        let state_rows: std::rc::Rc<Vec<Option<Vec<crate::deck::joint::MissFactors>>>> = if state_bound_mode > 0
            && (cap_free || !legal_rc.iter().any(|&j| self.sources[j].uncertified_association))
        {
            let hit = self.state_row_cache.borrow().get(&set_id).cloned();
            hit.unwrap_or_else(|| {
                let r = std::rc::Rc::new(
                    (0..self.card_indices.len())
                        .map(|j| {
                            legal_rc.contains(&j).then(|| src_levels(&self.sources[j], self.kind[j].stop, target_count))
                        })
                        .collect::<Vec<_>>(),
                );
                self.state_row_cache.borrow_mut().insert(set_id, std::rc::Rc::clone(&r));
                r
            })
        } else {
            std::rc::Rc::new(vec![])
        };
        let state_bound_mode = if state_rows.is_empty() { 0 } else { state_bound_mode };
        let state_bound_threshold = if state_bound_mode > 0 { clauses[0].threshold } else { 0.0 };
        let (bound_trainee, bound_inheritance) = if state_bound_mode > 0 {
            (trainees[clauses[0].lineages[0].trainee], clauses[0].lineages[0].inheritance)
        } else {
            (crate::uma::UmaIndex(0), 0.0)
        };
        // the trainee's `land_factor` per (outfit, target), row o * target_count + k (`LandPre`: the same values)
        let landing_factors: Vec<LandPre> = if state_bound_mode > 0 {
            (0..model.opts[bound_trainee].len())
                .flat_map(|o| {
                    (0..target_count).map(move |k| {
                        LandPre::new(
                            model.opts[bound_trainee][o].coverage[k],
                            model.ews[bound_trainee][o][k],
                            bound_inheritance,
                        )
                    })
                })
                .collect()
        } else {
            vec![]
        };
        let rho = if state_bound_mode == 2 { 1e9 } else { rho };
        if state_bound_mode > 0 {
            SB_USED.with(|c| c.set(state_bound_mode));
        }
        // the cap per structure class (`cap_class`), at most rho (uncapped with SBOUND=2); a deck passes at its own
        // class's cap, a node at the cap of the class of every deck it can still reach (its cards and every candidate
        // left: a superset of any completion's, so a cap at least as high)
        let class_caps: [f64; 4] = if state_bound_mode == 2 {
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
            target_count,
            size,
            plain_pal_count,
            kind_count,
            filler_kind_index,
            kind_place_values,
            set_id,
            legal_tables,
            fixed,
            cap_free,
            cap_free_node_limit,
            cap_free_check_scale,
            state_bound_mode,
            state_rows,
            state_bound_threshold,
            bound_trainee,
            bound_inheritance,
            landing_factors,
            class_caps,
            legal_entries: legal_rc,
        })
    }

    /// Prepare shared bounds, then build and traverse configurations. The callbacks borrow
    /// these tables and their lazy caches for both phases; none need copying or dynamic dispatch.
    pub(crate) fn search_with_bounds(
        &self,
        model: &Model,
        trainees: &[crate::uma::UmaIndex],
        clauses: &[SearchClause],
        setup: SearchSetup,
    ) -> SearchOutcome {
        let SearchSetup {
            best,
            seed,
            rho,
            cap,
            target_count,
            size,
            plain_pal_count,
            kind_count,
            filler_kind_index,
            kind_place_values,
            set_id,
            legal_tables,
            fixed,
            cap_free,
            cap_free_node_limit,
            cap_free_check_scale,
            state_bound_mode,
            state_rows,
            state_bound_threshold,
            bound_trainee,
            bound_inheritance,
            landing_factors,
            class_caps,
            legal_entries: _legal_entries,
        } = setup;
        let character_bits: &[u128] = &legal_tables.character_bits;
        let (by_kind, pal_entries) = (&legal_tables.by_kind, &legal_tables.pal_entries);
        let val = |cl: &SearchClause, dw: &[f64], dg: &[f64]| {
            fixed
                * cl.lineages
                    .iter()
                    .map(|&LineageChance { trainee: k, inheritance: q }| model.trainee_fast(trainees[k], q, dw, dg))
                    .sum::<f64>()
        };
        // every chance and outfit odd in [0, 1] and a fixed factor >= 0: then a deck's value rises with each of its odds
        // (white < gold) and every quantity in its evaluation is in [0, 1] up to rounding, which the dominance skip
        // (`dom`), the lazy configurations (`PendingConfig`) and the key bound (`keyskip`) rely on
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let valid = fixed >= 0.0
            && clauses.iter().all(|cl| {
                cl.lineages.iter().all(|&LineageChance { trainee: k, inheritance: q }| {
                    in01(q) && model.ews[trainees[k]].iter().flatten().all(|&x| in01(x))
                })
            });
        let use_dominance = best && valid;
        // the product search builds its configurations lazily (`PendingConfig`): roots and ratio tables only for those it may
        // visit
        let lazy = best && valid;
        let t_cfg = Instant::now();
        let out: Vec<ScoredDeck<SelectedDeck>> = vec![];
        let best_value = if best { seed } else { f64::NEG_INFINITY };
        let counts = self.counts_all();
        let groups_all: Vec<&Vec<usize>> = by_kind.iter().chain(std::iter::once(pal_entries)).collect();
        // the cap-free band's ratio tables (`CapFreeRatios`), built on first use
        let cap_free_outfit_count = if cap_free { model.opts[bound_trainee].len() } else { 0 };
        let cf_ratios = CapFreeRatios::new(
            LandingBoundTables {
                factors: &landing_factors,
                entry_rows: &state_rows,
                target_count,
                outfit_count: cap_free_outfit_count,
            },
            cap_free,
            kind_count,
            &groups_all,
            &self.kinds_all,
        );
        // per group, its entries' `cv_mask`s ORed
        let group_class_masks: Vec<u64> =
            groups_all.iter().map(|l| l.iter().fold(0u64, |m, &j| m | self.class_masks[j])).collect();
        // locked cards each group can hold (a configuration whose groups cannot hold them all is skipped)
        let (lock_bits, all_lock_bits) = {
            let x = self.lock_bits();
            ((&*x.entry_bits), x.all_bits)
        };
        let group_lock_bits: &[u8] = &legal_tables.group_lock_bits;
        let mut dw = vec![0.0f64; target_count];
        let mut dg = vec![0.0f64; target_count];
        // (scratch of the configuration build: a candidate's groups, copied into its SearchConfig only when kept; the root's
        // clause sums)
        let mut group_buffer: Vec<GroupSlots> = vec![];
        let (mut cvr, mut cvro) = (vec![0.0f64; clauses.len()], vec![0.0f64; clauses.len()]);
        let OutfitBounds {
            outfit_rows,
            outfit_runs,
            zero: _zero,
            target_bounds,
            outfits,
            empty_values,
            row_indices,
            zero_masks,
            nonzero_masks,
            divisors,
            alternative_columns,
            column_count,
        } = prepare_outfit_bounds(model, trainees, clauses, target_count);
        // a card's ratio for column c (its single-card odds x, y): N's (a split column), the plain ratio, or infinite
        let ratio_at = |c: usize, x: &[f64], y: &[f64]| -> f64 {
            let i = row_indices[c];
            let crate::deck::enumerate::bounds::OutfitRow { trainee: u, inheritance: q, outfit: o, .. } =
                outfit_rows[i];
            if zero_masks[c] != 0 {
                outfits[i].eval_in(q, x, y, nonzero_masks[c]) / divisors[c]
            } else if empty_values[i] > 0.0 {
                model.outfit_fast(u, o, q, x, y) / empty_values[i]
            } else {
                f64::INFINITY
            }
        };
        // (RB_CHECK=s: the node bounds on the current path, per depth its clause sums, the root's at depth 0; nodes pass
        // at s times their bound, so more leaves are checked, the results unchanged)
        let bound_check_scale = if cap_free { 0.0 } else { rb_check() };
        let check_ratio_bounds = bound_check_scale > 0.0;
        let bound_check_scale = if check_ratio_bounds { bound_check_scale } else { 1.0 };
        let diagnostic_bounds: Vec<Vec<f64>> = vec![];
        // the root bound's ratio products (row 0 of the ratio suffix table, which is built only for configurations the
        // DFS expands)
        let mut ratio_products = vec![1.0f64; column_count];
        let RatioPrefixes { empty_factors, ratio_divisors, factor_prefixes } = prepare_ratio_prefixes(&RatioColumns {
            outfit_rows: &outfit_rows,
            outfits: &outfits,
            empty_values: &empty_values,
            row_indices: &row_indices,
            zero_masks: &zero_masks,
            nonzero_masks: &nonzero_masks,
            divisors: &divisors,
            target_count,
        });
        // Per kind group in `needed_groups` (the others are never read: empty) and column, the products of its entries' r best
        // ratios (the first `size` of their stable descending sort, `top_desc`), r = 0..=size
        // (a `GroupTable`: per group, column c's products at c (size + 1); `top`, the columns' best ratios, reused)
        let rtop: std::cell::RefCell<Vec<Vec<f64>>> =
            std::cell::RefCell::new((0..column_count).map(|_| Vec::with_capacity(size)).collect());
        let ratio_prefixes_for = |mw: &[f64], mg: &[f64], touched_targets: &[u32], needed_groups: u64| -> GroupTable {
            let (mut x, mut y) = ([0.0f64; 32], [0.0f64; 32]);
            let mut top = rtop.borrow_mut();
            let mut tab = GroupTable::new(groups_all.len(), needed_groups, column_count * (size + 1));
            for (gi, l) in groups_all.iter().enumerate() {
                if needed_groups >> gi & 1 == 0 {
                    continue;
                }
                top.iter_mut().for_each(std::vec::Vec::clear);
                for &j in *l {
                    let tm = touched_targets[j];
                    let mut m = tm;
                    while m != 0 {
                        let t = m.trailing_zeros() as usize;
                        m &= m - 1;
                        x[t] = 1.0 - mw[j * target_count + t];
                        y[t] = 1.0 - mg[j * target_count + t];
                    }
                    for c in 0..column_count {
                        let r = match ratio_divisors[c] {
                            None => f64::INFINITY,
                            Some(crate::deck::enumerate::bounds::RatioDivisor { targets: cm, value: den }) => {
                                let (i, s) = (row_indices[c], tm & cm);
                                let p = if s == 0 {
                                    factor_prefixes[c * (target_count + 1) + target_count]
                                } else {
                                    let (o, q, t0) =
                                        (&outfits[i], outfit_rows[i].inheritance, s.trailing_zeros() as usize);
                                    let mut p = factor_prefixes[c * (target_count + 1) + t0];
                                    let mut m = cm >> t0 << t0;
                                    while m != 0 {
                                        let t = m.trailing_zeros() as usize;
                                        m &= m - 1;
                                        p *= if s >> t & 1 != 0 {
                                            o.factor_at(t, q, x[t], y[t])
                                        } else {
                                            empty_factors[i * target_count + t]
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
        // A configuration's per-target bound (`groups`, the key's `group_misses`): each target's odds at the product of
        // its groups' lowest misses, from the last group down (row 0 of the suffix table the DFS builds, the same products
        // in the same order). The eager and the lazy build both take it
        let configuration_odds = |groups: &[GroupSlots], group_misses: &GroupTable, dw: &mut [f64], dg: &mut [f64]| {
            for t in 0..target_count {
                let (mut a, mut b) = (1.0f64, 1.0f64);
                for g in (0..groups.len()).rev() {
                    let gb = group_misses.grp(groups[g].group);
                    a *= gm_w(gb, target_count, groups[g].slots)[t];
                    b *= gm_g(gb, target_count, groups[g].slots)[t];
                }
                dw[t] = 1.0 - a;
                dg[t] = 1.0 - b;
            }
        };
        // Its root bound at those odds with `ratio_prefixes`: the root ratio products (row 0 of the suffix
        // table, from the last group down), then per (chance, outfit) the lower of the per-target bound and the ratio
        // bound, summed per clause with the split (`cv`) and without (`cvo`). With ZSPLIT rows `cvo` sets the visiting
        // order (`root`, as before: the order decides which of two tied decks is found first) and the stop; the split's
        // (`prune_bound`) skips. Without the split the ratios were all infinite, so B was 0 * inf (NaN, which `min` ignores)
        // or, with no card to place, 0 * 1
        let configuration_root_bound = |groups: &[GroupSlots],
                                        ratio_prefixes: &GroupTable,
                                        dw: &[f64],
                                        dg: &[f64],
                                        ratio_products: &mut [f64],
                                        cv: &mut [f64],
                                        cvo: &mut [f64]| {
            let ng = groups.len();
            for i in 0..column_count {
                let mut x = 1.0f64;
                for g in (0..ng).rev() {
                    x *= ratio_prefixes.grp(groups[g].group)[i * (size + 1) + groups[g].slots];
                }
                ratio_products[i] = x;
            }
            cv.fill(0.0);
            cvo.fill(0.0);
            for &crate::deck::enumerate::bounds::OutfitRun { clause: ci, start: i, count: no } in &outfit_runs {
                let q = outfit_rows[i].inheritance;
                let (mut m, mut mo) = (0.0f64, 0.0f64);
                for o in 0..no {
                    let ix = i + o;
                    let a = outfits[ix].eval(q, dw, dg);
                    let (mut t, to) = if zero_masks[ix] != 0 {
                        (
                            a.min(outfits[ix].eval_in(q, dw, dg, zero_masks[ix]) * (divisors[ix] * ratio_products[ix])),
                            a.min(empty_values[ix] * if ng > 0 { f64::INFINITY } else { 1.0 }),
                        )
                    } else {
                        let b = a.min(empty_values[ix] * ratio_products[ix]);
                        (b, b)
                    };
                    // (a low split: the lower B, outside the visiting order)
                    for c in alternative_columns[ix].start..alternative_columns[ix].end {
                        t = t.min(outfits[ix].eval_in(q, dw, dg, zero_masks[c]) * (divisors[c] * ratio_products[c]));
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
        let keyskip = valid && !cap_free && (!best || best_value > f64::NEG_INFINITY);
        let (mut kdw, mut kdg) = (vec![0.0f64; target_count], vec![0.0f64; target_count]);
        let controls = SearchControl {
            best,
            rho,
            cap,
            target_count,
            size,
            kind_count,
            set_id,
            fixed,
            cap_free,
            cap_free_node_limit,
            cap_free_check_scale,
            state_bound_mode,
            state_bound_threshold,
            bound_trainee,
            bound_inheritance,
            class_caps,
            use_dominance,
            lazy,
            cap_free_outfit_count,
            column_count,
            bound_check_scale,
            check_ratio_bounds,
        };
        let bounds = ConfigurationBounds {
            ratio_prefixes: ratio_prefixes_for,
            odds: configuration_odds,
            root: configuration_root_bound,
        };
        let configurations = self.build_configurations(
            clauses,
            controls,
            ConfigurationTables {
                counts: &counts,
                by_kind,
                pal_entries,
                kind_place_values: &kind_place_values,
                groups_all: &groups_all,
                group_lock_bits,
                group_class_masks: &group_class_masks,
                filler_kind_index,
                plain_pal_count,
                all_lock_bits,
                skip_bounded_keys: keyskip,
            },
            ConfigurationScratch {
                white_chances: &mut dw,
                gold_chances: &mut dg,
                groups: &mut group_buffer,
                clause_bounds: &mut cvr,
                order_bounds: &mut cvro,
                ratio_products: &mut ratio_products,
                key_white: &mut kdw,
                key_gold: &mut kdg,
            },
            best_value,
            val,
            &bounds,
        );
        SPT.with(|c| {
            let mut v = c.get();
            v.0 += t_cfg.elapsed().as_secs_f64();
            v.2 += configurations.configurations.len();
            v.3 += counts.len();
            c.set(v);
        });
        self.traverse_configurations(
            model,
            clauses,
            controls,
            PreparedTables {
                character_bits,
                groups_all: &groups_all,
                state_rows: &state_rows,
                landing_factors: &landing_factors,
                lock_bits,
                all_lock_bits,
                outfit_rows: &outfit_rows,
                outfit_runs: &outfit_runs,
                outfits: &outfits,
                zero_masks: &zero_masks,
                nonzero_masks: &nonzero_masks,
                alternative_columns: &alternative_columns,
            },
            configurations,
            TraversalScratch {
                decks: out,
                best_value,
                white_chances: dw,
                gold_chances: dg,
                target_bounds,
                diagnostic_bounds,
            },
            val,
            &bounds,
            ratio_at,
            &cf_ratios,
        )
    }
}
