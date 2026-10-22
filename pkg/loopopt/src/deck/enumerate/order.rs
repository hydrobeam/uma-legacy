//! The visiting order of queued configurations.
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

/// The partial decks' miss products, one row of `nu` per depth (the last row: the deck so far): `pw`, `pg` hold one
/// row of nu per depth 0 ..= the deck's slots, row d at d * nu (the partial deck with d cards); `pid` the last parent
/// id given out (kept across configurations)
pub(in crate::deck::enumerate) struct DeckPath {
    pub(in crate::deck::enumerate) sel: Vec<usize>,
    pub(in crate::deck::enumerate) pw: Vec<f64>,
    pub(in crate::deck::enumerate) pg: Vec<f64>,
    pub(in crate::deck::enumerate) pid: usize,
}

/// The configuration to visit next: computed from a pending one (lazy) or the eager list's
pub(in crate::deck::enumerate) enum ConfigRef<'queue> {
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
pub(in crate::deck::enumerate) struct ConfigOrder {
    lazy: bool,
    rbc: bool,
    cfgs: Vec<SearchConfig>,
    pend: Vec<PendingConfig>,
    pkeys: Vec<Option<std::rc::Rc<GTab>>>,
    pord: Vec<usize>,
    heap: std::collections::BinaryHeap<QueuedConfig>,
    pp: usize,
    ldw: Vec<f64>,
    ldg: Vec<f64>,
    lsp0: Vec<f64>,
    lcv: Vec<f64>,
    lcvo: Vec<f64>,
    eager_i: usize,
}

impl ConfigOrder {
    pub(in crate::deck::enumerate) fn new(queue: ConfigurationQueue, controls: SearchControl) -> Self {
        let SearchControl { lazy, rbc, nu, nc, .. } = controls;
        let ConfigurationQueue { cfgs, pend, pkeys } = queue;
        let mut pord: Vec<usize> = (0..pend.len()).collect();
        pord.sort_by(|&a, &b| pend[b].u.partial_cmp(&pend[a].u).unwrap());
        let heap: std::collections::BinaryHeap<QueuedConfig> = std::collections::BinaryHeap::default();
        let (ldw, ldg, lsp0, lcv, lcvo) =
            (vec![0.0f64; nu], vec![0.0f64; nu], vec![1.0f64; nc], vec![0.0f64; 1], vec![0.0f64; 1]);
        Self { lazy, rbc, cfgs, pend, pkeys, pord, heap, pp: 0, ldw, ldg, lsp0, lcv, lcvo, eager_i: 0 }
    }

    /// The next configuration to visit at the bar `bestv`, None when none is left
    pub(in crate::deck::enumerate) fn next(
        &mut self,
        sp: &DeckSpace<'_>,
        bestv: f64,
        rpk_of: &impl Fn(&[f64], &[f64], &[u32], u64) -> GTab,
        cfg_odds: &impl Fn(&[(usize, usize)], &GTab, &mut [f64], &mut [f64]),
        cfg_root: &impl Fn(&[(usize, usize)], &GTab, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> Option<ConfigRef<'_>> {
        let Self { lazy, rbc, cfgs, pend, pkeys, pord, heap, pp, ldw, ldg, lsp0, lcv, lcvo, eager_i } = self;
        if *lazy {
            // compute the pending configurations until the heap's top comes first in the sorted order: every one
            // left has a bound, so a root, below it (or cannot reach the bar)
            while *pp < pord.len() {
                let pe = &pend[pord[*pp]];
                if !(pe.u * (1.0 + 1e-9) >= bestv && heap.peek().is_none_or(|h| pe.u >= h.root)) {
                    break;
                }
                *pp += 1;
                // the eager build's values: the per-target bound, the key's ratio tables, sp0 and the root
                cfg_odds(&pe.gs, &pe.gm, ldw, ldg);
                let rpk: &std::rc::Rc<GTab> = pkeys[pe.key].get_or_insert_with(|| {
                    let km = sp.key_miss(pe.code);
                    std::rc::Rc::new(rpk_of(&km.0, &km.1, &km.2, pe.gneed))
                });
                cfg_root(&pe.gs, rpk, ldw, ldg, lsp0, lcv, lcvo);
                heap.push(QueuedConfig {
                    root: lcvo[0],
                    idx: pord[*pp - 1],
                    cf: SearchConfig {
                        root: lcvo[0],
                        code: pe.code,
                        code_real22: pe.real22,
                        np: pe.np,
                        filler: pe.filler,
                        plain: pe.plain,
                        gs: pe.gs.clone(),
                        gm: std::rc::Rc::clone(&pe.gm),
                        rpk: std::rc::Rc::clone(rpk),
                        prune_bound: lcv[0],
                        clause_bounds: if *rbc { lcv.clone() } else { vec![] },
                    },
                });
            }
            heap.pop().map(|h| ConfigRef::Computed(h.cf))
        } else {
            if *eager_i >= cfgs.len() {
                return None;
            }
            *eager_i += 1;
            Some(ConfigRef::Listed(&cfgs[*eager_i - 1]))
        }
    }
}
