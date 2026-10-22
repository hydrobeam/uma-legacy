//! The visiting order of queued configurations.
use super::config::ConfigurationBounds;
use super::{ConfigurationQueue, PendingConfig, SearchConfig, SearchControl};
use crate::deck::{DeckSpace, GTab};

/// (lazy: the pending configurations by bound, highest first; the computed ones not yet visited, best root first
/// and, among equal roots, first built first, the order of the stable sort above)
struct QueuedConfig {
    root: f64,
    idx: usize,
    cf: SearchConfig,
}

impl PartialEq for QueuedConfig {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == std::cmp::Ordering::Equal
    }
}

impl Eq for QueuedConfig {}

impl PartialOrd for QueuedConfig {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}

impl Ord for QueuedConfig {
    fn cmp(&self, o: &Self) -> std::cmp::Ordering {
        self.root.partial_cmp(&o.root).unwrap().then(o.idx.cmp(&self.idx))
    }
}

/// The partial decks' miss products, one row of `target_count` per depth (the last row: the deck so far): `pw`, `pg` hold one
/// row of target_count per depth 0 ..= the deck's slots, row d at d * target_count (the partial deck with d cards); `parent_id` the last parent
/// id given out (kept across configurations)
pub(crate) struct DeckPath {
    pub(crate) sel: Vec<usize>,
    pub(crate) pw: Vec<f64>,
    pub(crate) pg: Vec<f64>,
    pub(crate) parent_id: usize,
}

/// The configuration to visit next: computed from a pending one (lazy) or the eager list's
pub(crate) enum ConfigRef<'queue> {
    Computed(SearchConfig),
    Listed(&'queue SearchConfig),
}

impl std::ops::Deref for ConfigRef<'_> {
    type Target = SearchConfig;
    fn deref(&self) -> &SearchConfig {
        match self {
            ConfigRef::Computed(c) => c,
            ConfigRef::Listed(c) => c,
        }
    }
}

/// The configurations in visiting order: the eager list in its stable root order; or (lazy) the pending ones by
/// bound, each computed (its root and the key's ratio tables) once it may come next, and popped best root first
pub(crate) struct ConfigOrder {
    lazy: bool,
    check_ratio_bounds: bool,
    configurations: Vec<SearchConfig>,
    pending: Vec<PendingConfig>,
    key_ratios: Vec<Option<std::rc::Rc<GTab>>>,
    pending_order: Vec<usize>,
    heap: std::collections::BinaryHeap<QueuedConfig>,
    pending_position: usize,
    white_chances: Vec<f64>,
    gold_chances: Vec<f64>,
    ratio_products: Vec<f64>,
    prune_bounds: Vec<f64>,
    order_bounds: Vec<f64>,
    eager_position: usize,
}

impl ConfigOrder {
    pub(crate) fn new(queue: ConfigurationQueue, controls: SearchControl) -> Self {
        let SearchControl { lazy, check_ratio_bounds, target_count, column_count, .. } = controls;
        let ConfigurationQueue { cfgs: configurations, pend: pending, pkeys: key_ratios } = queue;
        let mut pending_order: Vec<usize> = (0..pending.len()).collect();
        pending_order.sort_by(|&a, &b| pending[b].u.partial_cmp(&pending[a].u).unwrap());
        let heap: std::collections::BinaryHeap<QueuedConfig> = std::collections::BinaryHeap::default();
        let (white_chances, gold_chances, ratio_products, prune_bounds, order_bounds) = (
            vec![0.0f64; target_count],
            vec![0.0f64; target_count],
            vec![1.0f64; column_count],
            vec![0.0f64; 1],
            vec![0.0f64; 1],
        );
        Self {
            lazy,
            check_ratio_bounds,
            configurations,
            pending,
            key_ratios,
            pending_order,
            heap,
            pending_position: 0,
            white_chances,
            gold_chances,
            ratio_products,
            prune_bounds,
            order_bounds,
            eager_position: 0,
        }
    }

    /// The next configuration to visit at the bar `best_value`, None when none is left
    pub(crate) fn next(
        &mut self,
        space: &DeckSpace<'_>,
        best_value: f64,
        bounds: &ConfigurationBounds<
            impl Fn(&[f64], &[f64], &[u32], u64) -> GTab,
            impl Fn(&[(usize, usize)], &GTab, &mut [f64], &mut [f64]),
            impl Fn(&[(usize, usize)], &GTab, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
    ) -> Option<ConfigRef<'_>> {
        if self.lazy {
            // Compute pending configurations until the heap's top comes first in the sorted order: every one
            // left has a bound, so a root, below it (or cannot reach the bar)
            while self.pending_position < self.pending_order.len() {
                let pending_config = &self.pending[self.pending_order[self.pending_position]];
                if !(pending_config.u * (1.0 + 1e-9) >= best_value
                    && self.heap.peek().is_none_or(|h| pending_config.u >= h.root))
                {
                    break;
                }
                self.pending_position += 1;
                // the eager build's values: the per-target bound, the key's ratio tables, sp0 and the root
                (bounds.odds)(&pending_config.gs, &pending_config.gm, &mut self.white_chances, &mut self.gold_chances);
                let rpk: &std::rc::Rc<GTab> = self.key_ratios[pending_config.key].get_or_insert_with(|| {
                    let km = space.key_miss(pending_config.code);
                    std::rc::Rc::new((bounds.ratio_prefixes)(&km.0, &km.1, &km.2, pending_config.gneed))
                });
                (bounds.root)(
                    &pending_config.gs,
                    rpk,
                    &mut self.white_chances,
                    &mut self.gold_chances,
                    &mut self.ratio_products,
                    &mut self.prune_bounds,
                    &mut self.order_bounds,
                );
                self.heap.push(QueuedConfig {
                    root: self.order_bounds[0],
                    idx: self.pending_order[self.pending_position - 1],
                    cf: SearchConfig {
                        root: self.order_bounds[0],
                        code: pending_config.code,
                        code_real22: pending_config.real22,
                        np: pending_config.np,
                        filler: pending_config.filler,
                        plain: pending_config.plain,
                        gs: pending_config.gs.clone(),
                        gm: std::rc::Rc::clone(&pending_config.gm),
                        rpk: std::rc::Rc::clone(rpk),
                        prune_bound: self.prune_bounds[0],
                        clause_bounds: if self.check_ratio_bounds { self.prune_bounds.clone() } else { vec![] },
                    },
                });
            }
            self.heap.pop().map(|h| ConfigRef::Computed(h.cf))
        } else {
            if self.eager_position >= self.configurations.len() {
                return None;
            }
            self.eager_position += 1;
            Some(ConfigRef::Listed(&self.configurations[self.eager_position - 1]))
        }
    }
}
