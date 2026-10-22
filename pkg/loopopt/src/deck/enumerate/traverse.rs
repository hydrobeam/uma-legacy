//! Configuration traversal and its borrowed-card and state bounds.
use super::bounds::DominanceRequest;
use super::config::ConfigurationBounds;
use super::scratch::GroupBounds;
use super::{
    ConfigOrder, ConfigScratch, ConfigurationContext, ConfigurationQueue, DeckConstraints, DeckCursor, DeckPath,
    NodeScratch, SearchConfig, SearchOutcome, SearchStop, WalkScratch,
};
use crate::deck::capfree::{BoundOptions, LandingBoundTables};
use crate::deck::space::EntryMisses;
use crate::deck::{
    CapFreeRatios, ChainKind, DeckSpace, GTab, HashMap, Instant, LandPre, Model, SBN, SPT, SbCfg, SelectedDeck,
    StateBound, chain_states,
};

/// Immutable controls selected before traversal. Scalar copies do not change the search's buffers.
#[derive(Clone, Copy)]
pub(crate) struct SearchControl {
    pub(crate) best: bool,
    pub(crate) rho: f64,
    pub(crate) cap: usize,
    pub(crate) target_count: usize,
    pub(crate) size: usize,
    pub(crate) kind_count: usize,
    pub(crate) set_id: usize,
    pub(crate) fixed: f64,
    pub(crate) cap_free: bool,
    pub(crate) cap_free_node_limit: usize,
    pub(crate) cap_free_check_scale: f64,
    pub(crate) state_bound_mode: u8,
    pub(crate) state_bound_threshold: f64,
    pub(crate) bound_trainee: crate::UmaIndex,
    pub(crate) bound_inheritance: f64,
    pub(crate) class_caps: [f64; 4],
    pub(crate) use_dominance: bool,
    pub(crate) lazy: bool,
    pub(crate) cap_free_outfit_count: usize,
    pub(crate) column_count: usize,
    pub(crate) bound_check_scale: f64,
    pub(crate) check_ratio_bounds: bool,
}

/// Read-only views into preparation's tables. The lazy-bound callbacks borrow those same
/// tables, so preparation retains their ownership until traversal returns.
#[derive(Clone, Copy)]
pub(crate) struct PreparedTables<'tab> {
    pub(crate) ebit: &'tab [u128],
    pub(crate) groups_all: &'tab [&'tab Vec<usize>],
    pub(crate) state_rows: &'tab [Option<Vec<(f64, f64)>>],
    pub(crate) landing_factors: &'tab [LandPre],
    pub(crate) lbit: &'tab [u8],
    pub(crate) all_lk: u8,
    pub(crate) oq: &'tab [(usize, crate::UmaIndex, f64, usize)],
    pub(crate) og: &'tab [(usize, usize, usize)],
    pub(crate) orow: &'tab [crate::skills::OutfitPre],
    pub(crate) zm: &'tab [u32],
    pub(crate) nm: &'tab [u32],
    pub(crate) alt: &'tab [(usize, usize)],
}

/// Reuse preparation's odds buffers and retain diagnostic rows across configurations.
pub(crate) struct TraversalScratch {
    pub(crate) out: Vec<(f64, SelectedDeck)>,
    pub(crate) best_value: f64,
    pub(crate) dw: Vec<f64>,
    pub(crate) dg: Vec<f64>,
    pub(crate) av: Vec<f64>,
    pub(crate) rbs: Vec<Vec<f64>>,
}

impl DeckSpace<'_> {
    /// Traverse the prepared configurations in their visiting order (`ConfigOrder`): each one's root tests, then its
    /// tables and the depth-first walk of its decks (`DeckSearchContext::visit`), until one ends the search.
    pub(crate) fn traverse_configurations(
        &self,
        model: &Model,
        clauses: &[(Vec<(usize, f64)>, f64)],
        controls: SearchControl,
        tables: PreparedTables<'_>,
        queue: ConfigurationQueue,
        scratch: TraversalScratch,
        clause_value: impl Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64,
        bounds: &ConfigurationBounds<
            impl Fn(&[f64], &[f64], &[u32], u64) -> GTab,
            impl Fn(&[(usize, usize)], &GTab, &mut [f64], &mut [f64]),
            impl Fn(&[(usize, usize)], &GTab, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
        ratio_at: impl Fn(usize, &[f64], &[f64]) -> f64,
        cf_ratios: &CapFreeRatios<'_>,
    ) -> SearchOutcome {
        let t_dfs = Instant::now();
        let PreparedSearch { context, mut scratch, mut results } =
            DeckSearchContext::prepare(self, model, clauses, controls, tables, scratch, clause_value);
        let mut order = ConfigOrder::new(queue, controls);
        while let Some(cf) = order.next(self, results.best_value, bounds) {
            if context.past_bar(&cf, &results) {
                break;
            }
            if let Some(stop) = context.visit(&mut scratch, &mut results, &cf, &ratio_at, cf_ratios) {
                return SearchOutcome { decks: results.decks, incomplete: Some(stop) };
            }
        }
        SPT.with(|c| {
            let mut v = c.get();
            v.1 += t_dfs.elapsed().as_secs_f64();
            c.set(v);
        });
        SearchOutcome { decks: results.decks, incomplete: None }
    }
}

/// A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
/// whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
/// is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
/// r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
#[derive(Clone)]
pub(crate) struct MissProducts {
    pub(crate) white: Vec<f64>,
    pub(crate) gold: Vec<f64>,
}

pub(crate) struct BorrowBounds {
    pub(crate) own: Vec<MissProducts>,
    pub(crate) one: Vec<MissProducts>,
    pub(crate) owned_ratios: Vec<Vec<f64>>,
    pub(crate) borrow_ratios: Vec<Vec<f64>>,
}

/// The one-borrowed-card tables of one configuration's groups and their suffixes (`own`: no borrowed card left to
/// place; `one`: at most one, in one group); empty without borrowing (`has_borrowed_entries` false)
pub(crate) struct BorrowTables {
    pub(crate) has_borrowed_entries: bool,
    pub(crate) group_bounds: Vec<std::rc::Rc<BorrowBounds>>,
    pub(crate) owned_suffix: Vec<MissProducts>,
    pub(crate) borrow_suffix: Vec<MissProducts>,
    pub(crate) owned_ratio_suffix: Vec<Vec<f64>>,
    pub(crate) borrow_ratio_suffix: Vec<Vec<f64>>,
}

/// The node bound's per-parent rows (see `ConfigurationContext::hopeful`), per (depth, borrowing variant); parent ids never repeat
/// within a search
/// (`factors`: A's factors, row ix * target_count; `prefix_products`: their products from 1.0 over the first t, row ix * (target_count + 1))
pub(crate) struct ParentTargetBounds {
    pub(crate) parent_id: usize,
    pub(crate) factors: Vec<f64>,
    pub(crate) prefix_products: Vec<f64>,
}

/// The same for B, per depth (B has no borrowing variant): the parent's odds 1 - ppw, 1 - ppg, and per row, built
/// at the first child that evaluates its B (`st`: the parent id it was built for), its factors and their products
pub(crate) struct ParentRatioBounds {
    pub(crate) parent_id: usize,
    pub(crate) white_chances: [f64; 32],
    pub(crate) gold_chances: [f64; 32],
    pub(crate) row_stamps: Vec<usize>,
    pub(crate) factors: Vec<f64>,
    pub(crate) prefix_products: Vec<f64>,
}

/// What every configuration of one traversal reads: the search, its controls and tables, the clause value (`val`)
pub(crate) struct DeckSearchContext<'run, 'space, V> {
    pub(crate) space: &'run DeckSpace<'space>,
    pub(crate) model: &'run Model<'run>,
    pub(crate) clauses: &'run [(Vec<(usize, f64)>, f64)],
    pub(crate) controls: SearchControl,
    pub(crate) tables: PreparedTables<'run>,
    pub(crate) clause_value: V,
    pub(crate) outfit_row_count: usize,
    // the per-parent rows need every outfit row to read the target_count targets
    pub(crate) cache_parent_rows: bool,
    pub(crate) inheritance_chances: Vec<f64>,
}

/// One traversal: its inputs, the buffers and caches its configurations share, the decks kept and the bar
struct DeckSearchScratch {
    nodes: NodeScratch,
    configuration_scratch: ConfigScratch,
    borrow_cache: HashMap<(u64, usize), std::rc::Rc<BorrowBounds>>,
}

pub(crate) struct SearchResults {
    pub(crate) decks: Vec<(f64, SelectedDeck)>,
    pub(crate) best_value: f64,
}

/// Owners returned by search preparation; traversal borrows their parts independently.
struct PreparedSearch<'run, 'space, Value> {
    context: DeckSearchContext<'run, 'space, Value>,
    scratch: DeckSearchScratch,
    results: SearchResults,
}

impl<'run, 'space, V: Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64> DeckSearchContext<'run, 'space, V> {
    fn prepare(
        space: &'run DeckSpace<'space>,
        model: &'run Model<'run>,
        clauses: &'run [(Vec<(usize, f64)>, f64)],
        ctl: SearchControl,
        tb: PreparedTables<'run>,
        scratch: TraversalScratch,
        val: V,
    ) -> PreparedSearch<'run, 'space, V> {
        let TraversalScratch { out, best_value, dw, dg, av, rbs } = scratch;
        let outfit_row_count = tb.oq.len();
        let cache_parent_rows = tb.orow.iter().all(|o| o.len() == ctl.target_count);
        let inheritance_chances: Vec<f64> = tb.oq.iter().map(|x| x.2).collect();
        let context = Self {
            space,
            model,
            clauses,
            controls: ctl,
            tables: tb,
            clause_value: val,
            outfit_row_count,
            cache_parent_rows,
            inheritance_chances,
        };
        let z = [0.0f64; 32];
        let workspace = DeckSearchScratch {
            nodes: NodeScratch {
                white_chances: dw,
                gold_chances: dg,
                outfit_bounds: av,
                clause_values: vec![],
                partial_white: vec![],
                partial_gold: vec![],
                target_constants: vec![],
                ratio_constants: vec![],
                has_target_constants: vec![],
                has_ratio_constants: vec![],
                parent_targets: vec![],
                parent_ratios: vec![],
                touched_white: z,
                touched_gold: z,
                previous_factors: z,
                ratio_white: z,
                ratio_gold: z,
                ratio_factors: z,
                parent_white: z,
                parent_gold: z,
                dominance_products: z,
                diagnostic_bounds: rbs,
            },
            configuration_scratch: ConfigScratch {
                sufw: vec![],
                sufg: vec![],
                spf: vec![],
                csuf: vec![],
                goff: vec![],
                avail: vec![],
                path: DeckPath { sel: vec![], pw: vec![], pg: vec![], parent_id: 0 },
            },
            borrow_cache: HashMap::new(),
        };
        PreparedSearch { context, scratch: workspace, results: SearchResults { decks: out, best_value } }
    }

    /// No configuration from `cf` on can reach the bar (they come in root order)
    fn past_bar(&self, cf: &SearchConfig, results: &SearchResults) -> bool {
        let SearchControl { best, cap_free, .. } = self.controls;
        (best && cf.root * (1.0 + 1e-9) < results.best_value) || (!best && !cap_free && cf.root * (1.0 + 1e-9) < 1.0)
    }

    /// Visit one configuration: its split root bound, its tables, its state-conditioned root bound, then the walk of
    /// its decks. Some: the search stops there
    fn visit(
        &self,
        scratch: &mut DeckSearchScratch,
        results: &mut SearchResults,
        cf: &SearchConfig,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
        cf_ratios: &CapFreeRatios<'_>,
    ) -> Option<SearchStop> {
        let search = self;
        let DeckSearchScratch { nodes, configuration_scratch, borrow_cache } = scratch;
        let controls = search.controls;
        let tables = search.tables;
        let space = search.space;
        // (the split's root bound: skip, the order kept)
        if (controls.best && cf.prune_bound * controls.bound_check_scale * (1.0 + 1e-9) < results.best_value)
            || (!controls.best
                && !controls.cap_free
                && cf.prune_bound * controls.bound_check_scale * (1.0 + 1e-9) < 1.0)
        {
            return None;
        }
        let km = space.key_miss(cf.code);
        let (white_misses, gold_misses, touched_targets) = (&km.0, &km.1, (&*km.2));
        let groups: Vec<(&[usize], usize)> = cf.gs.iter().map(|&(i, c)| ((&**tables.groups_all[i]), c)).collect();
        // per group of this configuration: its key's tables (group_misses[g] = gm[group], group_ratios[g] = rpk[group])
        // (blocks: group_misses[g]'s row r by `gm_w`, `gm_g`; group_ratios[g]'s column ix at ix (controls.size + 1))
        let group_misses: Vec<&[f64]> = cf.gs.iter().map(|&(i, _)| cf.gm.grp(i)).collect();
        let group_ratios: Vec<&[f64]> = cf.gs.iter().map(|&(i, _)| cf.rpk.grp(i)).collect();
        configuration_scratch.fill_suffixes(
            cf,
            &GroupBounds {
                misses: &group_misses,
                ratios: &group_ratios,
                target_count: controls.target_count,
                column_count: controls.column_count,
                deck_size: controls.size,
            },
        );
        let misses = EntryMisses { white: white_misses, gold: gold_misses, target_count: controls.target_count };
        let dominance = search.dominance(cf, &groups, misses);
        let bt = search.borrow_tables(borrow_cache, cf, &groups, misses, ratio_at);
        nodes.clause_values.clear();
        nodes.clause_values.resize(search.clauses.len(), 0.0);
        nodes.partial_white.clear();
        nodes.partial_white.resize(controls.target_count, 0.0);
        nodes.partial_gold.clear();
        nodes.partial_gold.resize(controls.target_count, 0.0);
        let ng = groups.len();
        if !controls.best {
            configuration_scratch.fill_class_suffixes(&groups, &space.cvm);
        }
        // the configuration's state-conditioned bounds (`StateBound`) and its root: no deck of it can reach the bar
        let sbc = search.state_cfg(cf, &groups);
        let mut sbb = StateBound::new(
            sbc,
            &groups,
            &cf.gs,
            search.model,
            LandingBoundTables {
                factors: tables.landing_factors,
                entry_rows: tables.state_rows,
                target_count: controls.target_count,
                outfit_count: controls.cap_free_outfit_count,
            },
            cf_ratios,
            BoundOptions { cap_free: controls.cap_free, trainee: controls.bound_trainee, fixed_factor: controls.fixed },
        );
        let sb_root =
            sbb.root(controls.cap_free_check_scale, controls.state_bound_threshold, controls.state_bound_mode);
        if controls.cap_free {
            // (the cap-free band: past its node limit it stops; CF_CHECK passes the root at controls.cap_free_check_scale times its bound)
            if space.nodes.get() > controls.cap_free_node_limit {
                return Some(SearchStop::CapFreeNodeLimit);
            }
            if sb_root * controls.cap_free_check_scale.max(1.0) * (1.0 + 1e-9) < controls.state_bound_threshold {
                SBN.with(|c| c.borrow_mut().0 += 1);
                return None;
            }
        } else if controls.state_bound_mode > 0 && sb_root * (1.0 + 1e-9) < controls.state_bound_threshold {
            SBN.with(|c| c.borrow_mut().0 += 1);
            return None;
        }
        // (CF_CHECK: the bounds of the nodes on the current path, per depth, the root's at 0)
        let cf_path = if controls.cap_free_check_scale > 0.0 { vec![sb_root] } else { vec![] };
        let nrows = groups.iter().map(|x| x.1).sum::<usize>() + 1;
        let st = &mut configuration_scratch.path;
        st.sel.clear();
        st.pw.clear();
        st.pw.resize(nrows * controls.target_count, 1.0);
        st.pg.clear();
        st.pg.resize(nrows * controls.target_count, 1.0);
        nodes.grow_parent_rows(
            2 * (groups.iter().map(|x| x.1).sum::<usize>() + 1),
            nrows,
            search.outfit_row_count,
            controls.target_count,
            search.cache_parent_rows,
        );
        // per (group g, cards left, borrowing allowed): the per-target suffix factors (a, b) and the per-outfit
        // ratio products r depend on nothing else, so they are constants of the configuration: tabulated on
        // first use of their row (the same expressions, so the same f64s as computing them per node); row
        // (loff[g] + left) * ncan + can, a row of 2 controls.target_count factors (a then b) and one of outfit_row_count ratios
        let ncan = if bt.has_borrowed_entries { 2 } else { 1 };
        let mut loff = vec![0usize; ng + 1];
        for g in 0..ng {
            loff[g + 1] = loff[g] + groups[g].1 + 1;
        }
        nodes.reset_constants(loff[ng] * ncan, controls.target_count, controls.column_count);
        // (a branch that can no longer place every locked card is dropped)
        if !configuration_scratch.fill_avail(&groups, tables.lbit, tables.all_lk) {
            return None;
        }
        let ConfigScratch { sufw, sufg, spf, csuf, goff, avail, path } = configuration_scratch;
        let walk = ConfigurationContext {
            search,
            configuration: cf,
            filler: cf.filler,
            plain: cf.plain,
            groups: &groups,
            group_misses: &group_misses,
            group_ratios: &group_ratios,
            white_misses,
            gold_misses,
            touched_targets,
            suffix_white: sufw,
            suffix_gold: sufg,
            suffix_ratios: spf,
            dominators: &dominance.tables,
            dominance_offsets: &dominance.offsets,
            dominance_suffix: &dominance.suffix_characters,
            borrow_bounds: &bt,
            class_suffix: csuf,
            group_offsets: goff,
            available_locks: avail,
            remaining_offsets: &loff,
            borrow_state_count: ncan,
            group_count: ng,
            total_slots: groups.iter().map(|x| x.1).sum(),
            root_bound: sb_root,
        };
        let mut walk_scratch = WalkScratch {
            nodes,
            state_bound: sbb,
            depth_one_bound: f64::INFINITY,
            bound_path: cf_path,
            limit_reached: false,
            white_chances: [0.0f64; 32],
            gold_chances: [0.0f64; 32],
        };
        let stop = if groups.is_empty() {
            walk.leaf(
                &mut walk_scratch,
                results,
                &[],
                &[1.0; 32][..controls.target_count],
                &[1.0; 32][..controls.target_count],
                0,
            )
        } else {
            walk.rec(
                &mut walk_scratch,
                results,
                path,
                DeckCursor { group: 0, start: 0, remaining: groups[0].1 },
                &DeckConstraints {
                    characters: 0,
                    borrowed_count: 0,
                    locked_cards: 0,
                    class_mask: 0,
                    required_characters: 0,
                },
            )
        };
        if stop {
            return Some(SearchStop::OutputLimit);
        }
        if walk_scratch.limit_reached {
            return Some(SearchStop::CapFreeNodeLimit);
        }
        None
    }
}

/// Per-group dominance candidates and the flattened character suffix used to reject impossible completions.
struct ConfigurationDominance {
    tables: Vec<std::rc::Rc<Vec<Vec<(u32, u32)>>>>,
    offsets: Vec<usize>,
    suffix_characters: Vec<u128>,
}

impl<V> DeckSearchContext<'_, '_, V> {
    /// The dominance tables of `cf`'s groups (`dom_table`), and per flat index f (the groups' entries in turn) the
    /// characters of the entries from f on (`dsuf`, group g's from `doff[g]`); empty without the dominance skip
    fn dominance(
        &self,
        cf: &SearchConfig,
        groups: &[(&[usize], usize)],
        misses: EntryMisses<'_>,
    ) -> ConfigurationDominance {
        let SearchControl { use_dominance, set_id, .. } = self.controls;
        let PreparedTables { ebit, groups_all, lbit, .. } = self.tables;
        let doms: Vec<std::rc::Rc<Vec<Vec<(u32, u32)>>>> = if use_dominance {
            cf.gs
                .iter()
                .map(|&(i, _)| {
                    self.space.dom_table(
                        DominanceRequest { set_id, code: cf.code, group: i, entries: groups_all[i], lock_bits: lbit },
                        misses,
                    )
                })
                .collect()
        } else {
            vec![]
        };
        let (mut doff, mut dsuf): (Vec<usize>, Vec<u128>) = (vec![0], vec![]);
        if use_dominance {
            for &(l, _) in groups {
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
        ConfigurationDominance { tables: doms, offsets: doff, suffix_characters: dsuf }
    }

    /// Group gi's `BorrowBounds` under the key's miss chances `white_misses`, `gold_misses` (`ratio_at`: a card's ratio per column)
    fn borrow_bounds(
        &self,
        gi: usize,
        misses: EntryMisses<'_>,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
    ) -> BorrowBounds {
        let SearchControl { target_count, size, column_count, .. } = self.controls;
        let l = self.tables.groups_all[gi];
        let (own, bor): (Vec<usize>, Vec<usize>) = l.iter().partition(|&&j| !self.space.ebor[j]);
        let mut om = vec![MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] }; size + 1];
        let mut bm = MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] };
        for t in 0..target_count {
            let mut a: Vec<f64> = own.iter().map(|&j| misses.white[j * target_count + t]).collect();
            a.sort_by(|x, y| x.partial_cmp(y).unwrap());
            let mut b: Vec<f64> = own.iter().map(|&j| misses.gold[j * target_count + t]).collect();
            b.sort_by(|x, y| x.partial_cmp(y).unwrap());
            for r in 1..=size {
                om[r].white[t] = om[r - 1].white[t] * a.get(r - 1).copied().unwrap_or(1.0);
                om[r].gold[t] = om[r - 1].gold[t] * b.get(r - 1).copied().unwrap_or(1.0);
            }
            for &j in &bor {
                bm.white[t] = bm.white[t].min(misses.white[j * target_count + t]);
                bm.gold[t] = bm.gold[t].min(misses.gold[j * target_count + t]);
            }
        }
        let mut one = om.clone();
        for r in 1..=size {
            for t in 0..target_count {
                one[r].white[t] = om[r].white[t].min(om[r - 1].white[t] * bm.white[t]);
                one[r].gold[t] = om[r].gold[t].min(om[r - 1].gold[t] * bm.gold[t]);
            }
        }
        let (mut x, mut y) = (vec![0.0f64; target_count], vec![0.0f64; target_count]);
        // (`ratio_at`: without a split column, every zm 0 and cix the identity, the plain ratio)
        let mut ratio = |j: usize| -> Vec<f64> {
            for t in 0..target_count {
                x[t] = 1.0 - misses.white[j * target_count + t];
                y[t] = 1.0 - misses.gold[j * target_count + t];
            }
            (0..column_count).map(|i| ratio_at(i, &x, &y)).collect()
        };
        let ro: Vec<Vec<f64>> = own.iter().map(|&j| ratio(j)).collect();
        let rb: Vec<Vec<f64>> = bor.iter().map(|&j| ratio(j)).collect();
        let (mut owned_ratios, mut borrow_ratios) = (vec![], vec![]);
        for i in 0..column_count {
            let mut r: Vec<f64> = ro.iter().map(|v| v[i]).collect();
            r.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let best_b = rb.iter().map(|v| v[i]).fold(f64::NEG_INFINITY, f64::max);
            let mut pre = vec![1.0f64; size + 1];
            for k in 1..=size {
                pre[k] = pre[k - 1] * r.get(k - 1).copied().unwrap_or(1.0);
            }
            let mut borrow_ratio_suffix = pre.clone();
            if best_b > f64::NEG_INFINITY {
                for k in 1..=size {
                    borrow_ratio_suffix[k] = pre[k].max(pre[k - 1] * best_b);
                }
            }
            owned_ratios.push(pre);
            borrow_ratios.push(borrow_ratio_suffix);
        }
        BorrowBounds { own: om, one, owned_ratios, borrow_ratios }
    }

    /// `cf`'s one-borrowed-card tables (each group's `BorrowBounds` cached per (key, group) in `borrow_cache`)
    fn borrow_tables(
        &self,
        borrow_cache: &mut HashMap<(u64, usize), std::rc::Rc<BorrowBounds>>,
        cf: &SearchConfig,
        groups: &[(&[usize], usize)],
        misses: EntryMisses<'_>,
        ratio_at: &impl Fn(usize, &[f64], &[f64]) -> f64,
    ) -> BorrowTables {
        let SearchControl { target_count, column_count, .. } = self.controls;
        let space = self.space;
        let has_borrowed_entries =
            space.allowed_borrow == 1 && groups.iter().any(|(l, _)| l.iter().any(|&j| space.ebor[j]));
        let group_bounds: Vec<std::rc::Rc<BorrowBounds>> = if has_borrowed_entries {
            cf.gs
                .iter()
                .map(|&(i, _)| {
                    std::rc::Rc::clone(
                        borrow_cache
                            .entry((cf.code, i))
                            .or_insert_with(|| std::rc::Rc::new(self.borrow_bounds(i, misses, ratio_at))),
                    )
                })
                .collect()
        } else {
            vec![]
        };
        let ngs = cf.gs.len();
        // (allocated only when used)
        let nb = if has_borrowed_entries { ngs + 1 } else { 0 };
        // (vec![x; 0] would still build x: none without borrowing)
        let (mut owned_suffix, mut borrow_suffix) = if nb > 0 {
            (
                vec![MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] }; nb],
                vec![MissProducts { white: vec![1.0; target_count], gold: vec![1.0; target_count] }; nb],
            )
        } else {
            (vec![], vec![])
        };
        let (mut owned_ratio_suffix, mut borrow_ratio_suffix) = if nb > 0 {
            (vec![vec![1.0f64; column_count]; nb], vec![vec![1.0f64; column_count]; nb])
        } else {
            (vec![], vec![])
        };
        if has_borrowed_entries {
            for g in (0..ngs).rev() {
                let (gb, c) = (&group_bounds[g], cf.gs[g].1);
                for t in 0..target_count {
                    owned_suffix[g].white[t] = gb.own[c].white[t] * owned_suffix[g + 1].white[t];
                    owned_suffix[g].gold[t] = gb.own[c].gold[t] * owned_suffix[g + 1].gold[t];
                    borrow_suffix[g].white[t] = (gb.one[c].white[t] * owned_suffix[g + 1].white[t])
                        .min(gb.own[c].white[t] * borrow_suffix[g + 1].white[t]);
                    borrow_suffix[g].gold[t] = (gb.one[c].gold[t] * owned_suffix[g + 1].gold[t])
                        .min(gb.own[c].gold[t] * borrow_suffix[g + 1].gold[t]);
                }
                for i in 0..column_count {
                    owned_ratio_suffix[g][i] = gb.owned_ratios[i][c] * owned_ratio_suffix[g + 1][i];
                    borrow_ratio_suffix[g][i] = (gb.borrow_ratios[i][c] * owned_ratio_suffix[g + 1][i])
                        .max(gb.owned_ratios[i][c] * borrow_ratio_suffix[g + 1][i]);
                }
            }
        }
        BorrowTables {
            has_borrowed_entries,
            group_bounds,
            owned_suffix,
            borrow_suffix,
            owned_ratio_suffix,
            borrow_ratio_suffix,
        }
    }

    /// The state-conditioned bound's tables of `cf` (groups `groups`), cached per (set, key, real (2, 2), pal entries):
    /// chain positions (sorted kinds, the key's state distribution projected on the positions whose group has chain
    /// targets), per group its slots' positions and, per chain level, each target's lowest misses over the group's
    /// entries (a slot may take any of them, the same one twice: a relaxation). None without the bound (`state_bound_mode` 0)
    fn state_cfg(&self, cf: &SearchConfig, groups: &[(&[usize], usize)]) -> Option<std::rc::Rc<SbCfg>> {
        let SearchControl { target_count, kind_count, set_id, state_bound_mode, .. } = self.controls;
        let (space, state_rows, filler, ng) = (self.space, self.tables.state_rows, cf.filler, groups.len());
        let sbkey = (set_id, cf.code, cf.code_real22, cf.np);
        if state_bound_mode == 0 {
            None
        } else if let Some(r) = space.sbcfg.borrow().get(&sbkey).cloned() {
            Some(r)
        } else {
            let mut kinds: Vec<(ChainKind, usize)> = vec![]; // (kind, group, or usize::MAX for a filler)
            for (g, &(i, c)) in cf.gs.iter().enumerate() {
                if i < kind_count {
                    for _ in 0..c {
                        kinds.push((space.kinds_all[i], g));
                    }
                }
            }
            for _ in 0..filler {
                kinds.push((ChainKind::FILLER, usize::MAX));
            }
            kinds.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
            let kl: Vec<ChainKind> = kinds.iter().map(|x| x.0).collect();
            let full = if kl.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kl) };
            let has_chain: Vec<bool> =
                groups.iter().map(|(l, _)| l.iter().any(|&j| !space.srcs[j].cm.is_empty())).collect();
            let rel_pos: Vec<usize> =
                (0..kinds.len()).filter(|&p| kinds[p].1 != usize::MAX && has_chain[kinds[p].1]).collect();
            let mut slots: Vec<Vec<usize>> = vec![vec![]; ng];
            for (ri, &p) in rel_pos.iter().enumerate() {
                slots[kinds[p].1].push(ri);
            }
            for g in 0..ng {
                while slots[g].len() < groups[g].1 {
                    slots[g].push(usize::MAX);
                }
            }
            // (with at most 16 positions and levels below 16, the projected key packed 4 bits per position, the first
            // highest: the same sums, each in the order of `full`, sorted the same way)
            let states: Vec<(Vec<u8>, f64)> = if rel_pos.len() <= 16 && kl.iter().all(|k| k.stop < 16) {
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
                cf.gs.iter().map(|&(i, _)| if i < kind_count { space.kinds_all[i].stop as usize } else { 0 }).collect();
            let group_misses: Vec<Vec<(f64, f64)>> = (0..ng)
                .map(|g| {
                    let mut m = vec![(1.0f64, 1.0f64); (stops[g] + 1) * target_count];
                    for &j in groups[g].0 {
                        if let Some(r) = &state_rows[j] {
                            for (x, y) in m.iter_mut().zip(r.iter()) {
                                x.0 = x.0.min(y.0);
                                x.1 = x.1.min(y.1);
                            }
                        }
                    }
                    m
                })
                .collect();
            let r = std::rc::Rc::new(SbCfg {
                states,
                slots,
                gmin: group_misses,
                kl,
                rel_pos,
                order: std::cell::OnceCell::default(),
            });
            space.sbcfg.borrow_mut().insert(sbkey, std::rc::Rc::clone(&r));
            Some(r)
        }
    }
}
