//! The visiting order of queued configurations.
use crate::deck::enumerate::config::{ConfigurationBounds, ConfigurationQueue, PendingConfig, SearchConfig};
use crate::deck::enumerate::traverse::SearchControl;
use crate::deck::space::{DeckSpace, GroupSlots, GroupTable};
/// (lazy: the pending configurations by bound, highest first; the computed ones not yet visited, best root first
/// and, among equal roots, first built first, the order of the stable sort above)
struct QueuedConfig {
    root: f64,
    insertion_index: usize,
    configuration: SearchConfig,
}

impl PartialEq for QueuedConfig {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == std::cmp::Ordering::Equal
    }
}

impl Eq for QueuedConfig {}

impl PartialOrd for QueuedConfig {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for QueuedConfig {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.root.partial_cmp(&other.root).unwrap().then(other.insertion_index.cmp(&self.insertion_index))
    }
}

/// The partial decks' miss products: `white_misses` and `gold_misses` each hold one
/// row of target_count per depth 0 ..= the deck's slots, row d at d * target_count (the partial deck with d cards); `parent_id` the last parent
/// id given out (kept across configurations)
pub(crate) struct DeckPath {
    pub(crate) entries: Vec<usize>,
    pub(crate) white_misses: Vec<f64>,
    pub(crate) gold_misses: Vec<f64>,
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
    key_ratios: Vec<Option<std::rc::Rc<GroupTable>>>,
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
        let ConfigurationQueue { configurations, pending, key_ratios, .. } = queue;
        let mut pending_order: Vec<usize> = (0..pending.len()).collect();
        pending_order.sort_by(|&a, &b| pending[b].upper_bound.partial_cmp(&pending[a].upper_bound).unwrap());
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
            impl Fn(&[f64], &[f64], &[u32], u64) -> GroupTable,
            impl Fn(&[GroupSlots], &GroupTable, &mut [f64], &mut [f64]),
            impl Fn(&[GroupSlots], &GroupTable, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
    ) -> Option<ConfigRef<'_>> {
        if self.lazy {
            // Compute pending configurations until the heap's top comes first in the sorted order: every one
            // left has a bound, so a root, below it (or cannot reach the bar)
            while self.pending_position < self.pending_order.len() {
                let pending_config = &self.pending[self.pending_order[self.pending_position]];
                if !(pending_config.upper_bound * (1.0 + 1e-9) >= best_value
                    && self.heap.peek().is_none_or(|top| pending_config.upper_bound >= top.root))
                {
                    break;
                }
                self.pending_position += 1;
                // The eager build's values: the per-target bound, the key's ratio tables, ratio products and root.
                (bounds.odds)(
                    &pending_config.groups,
                    &pending_config.group_misses,
                    &mut self.white_chances,
                    &mut self.gold_chances,
                );
                let ratio_prefixes: &std::rc::Rc<GroupTable> = self.key_ratios[pending_config.ratio_key_index]
                    .get_or_insert_with(|| {
                        let key_misses = space.key_miss(pending_config.code, pending_config.events);
                        std::rc::Rc::new((bounds.ratio_prefixes)(
                            &key_misses.white,
                            &key_misses.gold,
                            &key_misses.touched_targets,
                            pending_config.needed_groups,
                        ))
                    });
                (bounds.root)(
                    &pending_config.groups,
                    ratio_prefixes,
                    &mut self.white_chances,
                    &mut self.gold_chances,
                    &mut self.ratio_products,
                    &mut self.prune_bounds,
                    &mut self.order_bounds,
                );
                // (the covered product search's factor where a nested chain may be reached: 1 elsewhere)
                let root_scale = pending_config.root_scale;
                self.heap.push(QueuedConfig {
                    root: self.order_bounds[0] * root_scale,
                    insertion_index: self.pending_order[self.pending_position - 1],
                    configuration: SearchConfig {
                        root: self.order_bounds[0] * root_scale,
                        code: pending_config.code,
                        events: pending_config.events,
                        real_filler_count: pending_config.real_filler_count,
                        pal_entry_count: pending_config.pal_entry_count,
                        other_entry_count: pending_config.other_entry_count,
                        filler_count: pending_config.filler_count,
                        plain_pal_count: pending_config.plain_pal_count,
                        groups: pending_config.groups.clone(),
                        group_misses: std::rc::Rc::clone(&pending_config.group_misses),
                        ratio_prefixes: std::rc::Rc::clone(ratio_prefixes),
                        prune_bound: self.prune_bounds[0] * root_scale,
                        clause_bounds: if self.check_ratio_bounds { self.prune_bounds.clone() } else { vec![] },
                        pair_cap: pending_config.pair_cap,
                    },
                });
            }
            self.heap.pop().map(|top| ConfigRef::Computed(top.configuration))
        } else {
            if self.eager_position >= self.configurations.len() {
                return None;
            }
            self.eager_position += 1;
            Some(ConfigRef::Listed(&self.configurations[self.eager_position - 1]))
        }
    }
}
