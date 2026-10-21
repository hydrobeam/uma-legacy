//! The deck enumeration (`DeckSpace::search`): node bounds, the exact pass with its caps and certificate.
use super::{
    CF_LIM, CF_ON, CFCHK, DeckSpace, GTab, HashMap, Instant, KeyLevel, Kind, LandPre, LegalTabs, Model, SB_ON, SB_USED,
    SBCHK, SBN, SPT, SbCfg, Sel, Src, cap_class, cf_check, chain_states, event_pmf_job, gm_g, gm_w, joint_fast,
    key_level, sb_check, sb_mode, src_levels, top_desc,
};

/// The two enumeration passes have different stopping rules and inputs.
#[derive(Clone, Copy)]
pub(super) enum SearchMode {
    /// Keep the first highest-value deck strictly above the initial floor.
    Best { seed: f64 },
    /// Collect decks that can reach a clause's threshold under its structure-class cap.
    Band { ratio_cap: f64, max_decks: usize },
}

/// Why enumeration stopped before checking all relevant decks. This is not an exact-scoring certificate.
pub(super) enum SearchStop {
    CharacterLimit,
    UnsupportedCapFreeProof,
    CapFreeNodeLimit,
    OutputLimit,
}

pub(super) struct SearchOutcome {
    pub(super) decks: Vec<(f64, Sel)>,
    /// None means enumeration finished under the requested bounds; exact scoring may still be needed.
    pub(super) incomplete: Option<SearchStop>,
}

/// Legal groups and key counts used to enumerate configurations, in their original order.
struct ConfigurationTables<'tab> {
    counts: &'tab [Vec<usize>],
    by_kind: &'tab [Vec<usize>],
    pal_ents: &'tab [usize],
    kpos: &'tab [u64],
    groups_all: &'tab [&'tab Vec<usize>],
    glk: &'tab [u8],
    gcvm: &'tab [u64],
    ki22: usize,
    nplain: usize,
    all_lk: u8,
    keyskip: bool,
}

/// Configuration-build buffers stay owned by preparation; white/gold odds are reused by traversal.
struct ConfigurationScratch<'scratch> {
    dw: &'scratch mut [f64],
    dg: &'scratch mut [f64],
    groups: &'scratch mut Vec<(usize, usize)>,
    clause_bounds: &'scratch mut [f64],
    order_bounds: &'scratch mut [f64],
    ratio_products: &'scratch mut [f64],
    key_white: &'scratch mut [f64],
    key_gold: &'scratch mut [f64],
}

/// Search-wide legal entries and proof configuration. Own the cached tables so the traversal
/// can borrow their group/character slices without a self-referential context.
struct SearchSetup {
    best: bool,
    seed: f64,
    rho: f64,
    cap: usize,
    nu: usize,
    size: usize,
    nplain: usize,
    nk: usize,
    ki22: usize,
    kpos: Vec<u64>,
    set_id: usize,
    lt: std::rc::Rc<LegalTabs>,
    // Keep the cached-entry handle alive through traversal, as before extraction.
    legal_entries: std::rc::Rc<Vec<usize>>,
    fixed: f64,
    cfm: bool,
    cf_lim: usize,
    cfs: f64,
    sbm: u8,
    sb_rows: std::rc::Rc<Vec<Option<Vec<(f64, f64)>>>>,
    sb_bar: f64,
    sb_u: usize,
    sb_q: f64,
    lp: Vec<LandPre>,
    ccap: [f64; 4],
}

/// Outfit rows and their zero/low-target split columns, in clause/chance/outfit order.
/// Bound calculations borrow the tables; traversal takes the `av` scratch buffer.
struct OutfitBounds {
    oq: Vec<(usize, usize, f64, usize)>,
    og: Vec<(usize, usize, usize)>,
    // Retain the preparation buffer until traversal ends, preserving its allocation lifetime.
    zero: Vec<f64>,
    av: Vec<f64>,
    orow: Vec<crate::skills::OutfitPre>,
    p0: Vec<f64>,
    cix: Vec<usize>,
    zm: Vec<u32>,
    nm: Vec<u32>,
    p0n: Vec<f64>,
    alt: Vec<(usize, usize)>,
    nc: usize,
}

/// Empty-card factors, ratio-column masks/divisors, and ordered factor prefixes.
struct RatioPrefixes {
    f0: Vec<f64>,
    cmk: Vec<Option<(u32, f64)>>,
    cpre: Vec<f64>,
}

fn prepare_outfit_bounds(model: &Model, us: &[usize], clauses: &[(Vec<(usize, f64)>, f64)], nu: usize) -> OutfitBounds {
    // (clause, chance, outfit) triples; the product value of one outfit at one chance is submodular in the set of
    // cards (log of each target's factor gains less from a card the more the deck already has), so a deck's value
    // is at most a partial deck's value times the best ratios P({card}) / P({}) of the cards still to add
    // (clause, trainee, chance, outfit), each (clause, trainee, chance) a run of its trainee's outfits: og
    let oq: Vec<(usize, usize, f64, usize)> = clauses
        .iter()
        .enumerate()
        .flat_map(|(ci, cl)| {
            cl.0.iter().flat_map(move |&(k, q)| (0..model.n_outfits(us[k])).map(move |o| (ci, us[k], q, o)))
        })
        .collect();
    let og: Vec<(usize, usize, usize)> = {
        let mut v = vec![];
        let mut i = 0;
        while i < oq.len() {
            let no = model.n_outfits(oq[i].1);
            v.push((oq[i].0, i, no));
            i += no;
        }
        v
    };
    let zero = vec![0.0f64; nu];
    // the node bound's per-target values, one per (clause, chance, outfit)
    let av = vec![0.0f64; oq.len()];
    // each (clause, chance, outfit)'s outfit, prepared once for the bounds below (`OutfitPre`: the same values as outfit_fast)
    let orow: Vec<crate::skills::OutfitPre> = oq.iter().map(|&(_, u, _, o)| model.outfit_pre(u, o)).collect();
    let p0: Vec<f64> = oq.iter().map(|&(_, u, q, o)| model.outfit_fast(u, o, q, &zero, &zero)).collect();
    // The ratio bound split by target (ZSPLIT). An outfit's value is a product of per-target factors; split the
    // targets into Z and N: over the decks below a node, Z's factors are at most A's (each target's at its lowest
    // misses) and N's at most N's factors at the partial deck times the best ratios of N's factors alone (the same
    // per-target log-submodularity as the plain ratio bound, over N), so B = A_Z * P_N(partial) * r_N bounds the
    // value for any split, and a row's B may be the lowest of several. A row whose empty-deck value is 0 (a target
    // with no inheritance, event or outfit chance: its factor is 0 without a hint) has every plain ratio
    // P({card}) / P({}) infinite, which leaves only A: it is split by its zero targets. A row with low-base targets
    // (empty-deck factor at most LOW_SPLIT of the factor with a certain white hint: their ratios overstate what
    // several cards add) gets one more split, Z its zero and low targets, kept beside its own (neither is lower
    // everywhere). The ratio tables have one column per (row, split): columns 0..noq the rows themselves (zm = 0:
    // the plain ratio bound, or for a zero row nothing to split on (nu > 32, N's empty value 0 by underflow): its
    // ratios infinite, as before), then the low splits (`alt[row]`, a range). Per column: its row, Z's and N's
    // target masks, N's empty-deck value
    let noq = oq.len();
    let (cix, zm, nm, p0n, alt): (Vec<usize>, Vec<u32>, Vec<u32>, Vec<f64>, Vec<(usize, usize)>) = {
        let (mut cix, mut zm, mut nm, mut p0n, mut alt) = (
            (0..noq).collect::<Vec<_>>(),
            vec![0u32; noq],
            vec![0u32; noq],
            vec![1.0f64; noq],
            vec![(0usize, 0usize); noq],
        );
        if nu <= 32 {
            for i in 0..noq {
                if orow[i].len() != nu {
                    continue;
                }
                let q = oq[i].2;
                let full = if nu == 32 { u32::MAX } else { (1u32 << nu) - 1 };
                let mut z = 0u32;
                for t in 0..nu {
                    if !(orow[i].factor_at(t, q, 0.0, 0.0) > 0.0) {
                        z |= 1 << t;
                    }
                }
                if z != 0 && !(p0[i] > 0.0) {
                    let pn = orow[i].eval_in(q, &zero, &zero, full & !z);
                    if pn > 0.0 {
                        zm[i] = z;
                        nm[i] = full & !z;
                        p0n[i] = pn;
                    }
                }
                // (an alternative only where it adds targets to the previous split and leaves N something)
                let (start, mut prev) = (cix.len(), z);
                for &tau in &LOW_SPLIT {
                    let mut za = z;
                    for t in 0..nu {
                        if orow[i].factor_at(t, q, 0.0, 0.0) <= tau * orow[i].factor_at(t, q, 1.0, 0.0) {
                            za |= 1 << t;
                        }
                    }
                    if za == prev || za == full {
                        continue;
                    }
                    let pn = orow[i].eval_in(q, &zero, &zero, full & !za);
                    if pn > 0.0 {
                        cix.push(i);
                        zm.push(za);
                        nm.push(full & !za);
                        p0n.push(pn);
                        prev = za;
                    }
                }
                alt[i] = (start, cix.len());
            }
        }
        (cix, zm, nm, p0n, alt)
    };
    let nc = cix.len();
    OutfitBounds { oq, og, zero, av, orow, p0, cix, zm, nm, p0n, alt, nc }
}

fn prepare_ratio_prefixes(
    nu: usize,
    oq: &[(usize, usize, f64, usize)],
    orow: &[crate::skills::OutfitPre],
    p0: &[f64],
    cix: &[usize],
    zm: &[u32],
    nm: &[u32],
    p0n: &[f64],
) -> RatioPrefixes {
    let noq = oq.len();
    let nc = cix.len();
    // A column's ratio of one card (`ratio_at`, or without the split the plain ratio: the same form, every zm 0) is
    // the product of its row's factors over a target mask (nm[c] for a split column, else all the row's targets, as
    // `outfit_fast` multiplies them), in target order from 1.0, over a divisor (p0n[c], p0[i]), or infinite (no
    // split and p0[i] not > 0: `cmk` None). At a target the card does not touch (miss chances exactly 1.0, the key's
    // mask `km.2`) its odds are 1 - 1 = 0, the factor `f0` at (0, 0): so per column the products of `f0` over the
    // mask's targets below each t (`cpre`, the same multiplications in the same order), and a card's ratio starts
    // from the prefix at its first touched target: the same f64s
    let f0: Vec<f64> = (0..noq)
        .flat_map(|i| {
            let (o, q) = (&orow[i], oq[i].2);
            (0..nu).map(move |t| if t < o.len() { o.factor_at(t, q, 0.0, 0.0) } else { 1.0 })
        })
        .collect();
    let cmk: Vec<Option<(u32, f64)>> = (0..nc)
        .map(|c| {
            let i = cix[c];
            let n = orow[i].len();
            if zm[c] != 0 {
                Some((nm[c], p0n[c]))
            } else if p0[i] > 0.0 {
                Some((if n >= 32 { u32::MAX } else { (1u32 << n) - 1 }, p0[i]))
            } else {
                None
            }
        })
        .collect();
    let cpre: Vec<f64> = (0..nc)
        .flat_map(|c| {
            let i = cix[c];
            let m = cmk[c].map_or(0, |x| x.0);
            let mut v = vec![1.0f64; nu + 1];
            for t in 0..nu {
                v[t + 1] = if m >> t & 1 != 0 { v[t] * f0[i * nu + t] } else { v[t] };
            }
            v
        })
        .collect();
    RatioPrefixes { f0, cmk, cpre }
}

// every (key, real (2, 2) entries, kind-0 entries) with its groups and root bound; visited best bound first
struct SearchConfig {
    root: f64,
    code: u64,
    code_real22: usize,
    np: usize,
    filler: usize,
    plain: usize,
    gs: Vec<(usize, usize)>,
    gm: std::rc::Rc<GTab>,
    rpk: std::rc::Rc<GTab>,
    // A tighter bound may prune without changing the root-bound visit order.
    prune_bound: f64,
    // Root clause values retained only for RB_CHECK.
    clause_bounds: Vec<f64>,
}

// A configuration passing the per-target bound, its root not yet computed: `u` is an upper bound on its root
// (the root sums fixed * max over outfits of min(A, B r) per (trainee, chance), the per-target value fixed *
// the sum of max A, the same A to the bit; terms >= 0, so the root is at most the value up to rounding);
// `key`: its key's slot in `pkeys` (the key's ratio tables, built on first need)
struct PendingConfig {
    u: f64,
    key: usize,
    code: u64,
    real22: usize,
    np: usize,
    filler: usize,
    plain: usize,
    gneed: u64,
    gs: Vec<(usize, usize)>,
    gm: std::rc::Rc<GTab>,
}

/// Immutable controls selected before traversal. Scalar copies do not change the search's buffers.
#[derive(Clone, Copy)]
struct SearchControl {
    best: bool,
    rho: f64,
    cap: usize,
    nu: usize,
    size: usize,
    nk: usize,
    set_id: usize,
    fixed: f64,
    cfm: bool,
    cf_lim: usize,
    cfs: f64,
    sbm: u8,
    sb_bar: f64,
    sb_u: usize,
    sb_q: f64,
    ccap: [f64; 4],
    dom_on: bool,
    lazy: bool,
    cf_no: usize,
    nc: usize,
    rbsl: f64,
    rbc: bool,
}

/// Read-only views into preparation's tables. The lazy-bound callbacks borrow those same
/// tables, so preparation retains their ownership until traversal returns.
struct TraversalTables<'tab> {
    ebit: &'tab [u128],
    groups_all: &'tab [&'tab Vec<usize>],
    sb_rows: &'tab [Option<Vec<(f64, f64)>>],
    lp: &'tab [LandPre],
    cf_z: &'tab [u32],
    lbit: &'tab [u8],
    all_lk: u8,
    oq: &'tab [(usize, usize, f64, usize)],
    og: &'tab [(usize, usize, usize)],
    orow: &'tab [crate::skills::OutfitPre],
    zm: &'tab [u32],
    nm: &'tab [u32],
    alt: &'tab [(usize, usize)],
}

/// Eager configurations retain their stable root order; pending configurations retain
/// insertion order until traversal builds its stable upper-bound ordering.
struct ConfigurationQueue {
    cfgs: Vec<SearchConfig>,
    pend: Vec<PendingConfig>,
    pkeys: Vec<Option<std::rc::Rc<GTab>>>,
}

/// Reuse preparation's odds buffers and retain diagnostic rows across configurations.
struct TraversalScratch {
    out: Vec<(f64, Sel)>,
    bestv: f64,
    dw: Vec<f64>,
    dg: Vec<f64>,
    av: Vec<f64>,
    rbs: std::cell::RefCell<Vec<Vec<f64>>>,
}

impl DeckSpace<'_> {
    /// Decks legal for trainees `us` (uma indices; `ids` their ids). `clauses`: ((trainee position in `us`,
    /// inheritance chance) pairs, threshold); a deck's clause value is the sum of its product values (fixed factor
    /// included) over those pairs. Best mode requires one clause and keeps its highest-value deck above `seed`
    /// (none if no deck beats it; ties: the first found). Band mode collects decks for which some clause has
    /// c * value >= threshold, c the deck's class cap (`cap_class`, `caps`) at most `ratio_cap`.
    /// Reaching `max_decks` ends the band immediately and reports `OutputLimit`, even if that was the last deck.
    pub(super) fn search(
        &self,
        model: &Model,
        us: &[usize],
        ids: &[u32],
        clauses: &[(Vec<(usize, f64)>, f64)],
        mode: SearchMode,
    ) -> SearchOutcome {
        let setup = match self.configure_search(model, us, ids, clauses, mode) {
            Ok(setup) => setup,
            Err(outcome) => return outcome,
        };
        self.search_with_bounds(model, us, clauses, setup)
    }

    /// Resolve legality and proof modes before preparing configuration bounds or visiting a deck.
    fn configure_search(
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
    fn search_with_bounds(
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
        // (the cap-free band's ratio bound, `cf_ub`) per outfit of the trainee, its zero-base targets (factor 0 with no
        // hint: their ratios would be infinite, so they take the per-target bound), and per group of `groups_all`, level
        // and outfit (row l * no + o) the best ratio over its entries: R(e, l) = prod over the other targets of the factor
        // at e's misses at level l over the factor with no card. Built on first use
        let cf_no = if cfm { model.opts[sb_u].len() } else { 0 };
        let cf_z: Vec<u32> = (0..cf_no)
            .map(|o| (0..nu).filter(|&k| !(lp[o * nu + k].f(1.0, 1.0) > 0.0)).fold(0u32, |m, k| m | 1 << k))
            .collect();
        // (outfit o's product over the targets outside its Z of the factors at an entry's level-l misses `row`, over the
        // same with no card: R(e, l))
        let cf_den = |o: usize| -> f64 {
            (0..nu).filter(|&k| cf_z[o] >> k & 1 == 0).map(|k| lp[o * nu + k].f(1.0, 1.0)).product()
        };
        let cf_ratio = |o: usize, row: &[(f64, f64)], den: f64| -> f64 {
            (0..nu).filter(|&k| cf_z[o] >> k & 1 == 0).map(|k| lp[o * nu + k].f(row[k].0, row[k].1)).product::<f64>()
                / den
        };
        let cf_rmx: std::cell::RefCell<Vec<Option<std::rc::Rc<Vec<f64>>>>> =
            std::cell::RefCell::new(vec![None; if cfm { groups_all.len() } else { 0 }]);
        let cf_rmx_of = |gi: usize| -> std::rc::Rc<Vec<f64>> {
            if let Some(r) = &cf_rmx.borrow()[gi] {
                return std::rc::Rc::clone(r);
            }
            let stop = if gi < nk { self.kinds_all[gi].1 as usize } else { 0 };
            let mut m = vec![0.0f64; (stop + 1) * cf_no];
            for o in 0..cf_no {
                let den = cf_den(o);
                for &j in groups_all[gi] {
                    if let Some(rows) = &sb_rows[j] {
                        for l in 0..=stop.min(rows.len() / nu - 1) {
                            let r = cf_ratio(o, &rows[l * nu..(l + 1) * nu], den);
                            if !(r <= m[l * cf_no + o]) {
                                m[l * cf_no + o] = r;
                            }
                        }
                    }
                }
            }
            let r = std::rc::Rc::new(m);
            cf_rmx.borrow_mut()[gi] = Some(std::rc::Rc::clone(&r));
            r
        };
        // the same ratio of one entry (row l * no + o), built on first use
        let cf_rent: std::cell::RefCell<crate::FxMap<usize, std::rc::Rc<Vec<f64>>>> = std::cell::RefCell::default();
        let cf_rent_of = |j: usize| -> std::rc::Rc<Vec<f64>> {
            if let Some(r) = cf_rent.borrow().get(&j) {
                return std::rc::Rc::clone(r);
            }
            let rows = sb_rows[j].as_ref().unwrap();
            let nl = rows.len() / nu;
            let mut m = vec![0.0f64; nl * cf_no];
            for o in 0..cf_no {
                let den = cf_den(o);
                for l in 0..nl {
                    m[l * cf_no + o] = cf_ratio(o, &rows[l * nu..(l + 1) * nu], den);
                }
            }
            let r = std::rc::Rc::new(m);
            cf_rent.borrow_mut().insert(j, std::rc::Rc::clone(&r));
            r
        };
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
        let rbs: std::cell::RefCell<Vec<Vec<f64>>> = std::cell::RefCell::new(vec![]);
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
                cf_z: &cf_z,
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
            cf_rmx_of,
            cf_rent_of,
        )
    }

    /// Enumerate legal card-count configurations, preserving stable root-bound order and
    /// deferring ratio tables for the product search until traversal actually needs them.
    fn build_configurations(
        &self,
        clauses: &[(Vec<(usize, f64)>, f64)],
        controls: SearchControl,
        tables: ConfigurationTables<'_>,
        scratch: ConfigurationScratch<'_>,
        bestv: f64,
        val: impl Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64,
        rpk_of: impl Fn(&[f64], &[f64], &[u32], u64) -> GTab,
        cfg_odds: impl Fn(&[(usize, usize)], &GTab, &mut [f64], &mut [f64]),
        cfg_root: impl Fn(&[(usize, usize)], &GTab, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
    ) -> ConfigurationQueue {
        let SearchControl { best, rho, nu, size, nk, set_id, fixed, cfm, ccap, lazy, rbc, .. } = controls;
        let ConfigurationTables {
            counts,
            by_kind,
            pal_ents,
            kpos,
            groups_all,
            glk,
            gcvm,
            ki22,
            nplain,
            all_lk,
            keyskip,
        } = tables;
        let ConfigurationScratch {
            dw,
            dg,
            groups: gsb,
            clause_bounds: cvr,
            order_bounds: cvro,
            ratio_products: sp0,
            key_white: kdw,
            key_gold: kdg,
        } = scratch;
        let key_slack = |cl: &(Vec<(usize, f64)>, f64)| 1e-11 * cl.0.len() as f64 * fixed.max(1.0);
        let mut cfgs: Vec<SearchConfig> = vec![];
        let mut pend: Vec<PendingConfig> = vec![];
        let mut pkeys: Vec<Option<std::rc::Rc<GTab>>> = vec![];
        for cnt in counts {
            let csum: usize = cnt.iter().sum();
            if (0..nk).any(|i| i != ki22 && cnt[i] > by_kind[i].len()) {
                continue;
            }
            let mut pkey_new = true;
            let code: u64 = cnt.iter().zip(kpos).map(|(&c, &p)| c as u64 * p).sum();
            let km = self.key_miss(code);
            let (mw, mg) = (&km.0, &km.1);
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much); cached per trainee and key
            // (the groups a configuration of this key can use: its kinds and, with slots left, the pal entries)
            let gneed: u64 =
                (0..nk).filter(|&i| cnt[i] > 0).fold(0u64, |m, i| m | 1 << i) | if csum < size { 1 << nk } else { 0 };
            let gm = self.gm_cached(set_id, code, groups_all, gneed, mw, mg, nu, size);
            // the key's bound: no variant (real (2, 2) and pal entries counted 0 ..= their most) can pass when the value
            // at odds above every variant's fails with room for rounding (`key_slack`)
            if keyskip {
                let (r22, npm) = (cnt[ki22].min(by_kind[ki22].len()), (size - csum).min(pal_ents.len()));
                let lo = |i: usize, c: usize, t: usize| -> (f64, f64) {
                    let gb = gm.grp(i);
                    (0..=c).fold((f64::INFINITY, f64::INFINITY), |m, x| {
                        (m.0.min(gm_w(gb, nu, x)[t]), m.1.min(gm_g(gb, nu, x)[t]))
                    })
                };
                for t in 0..nu {
                    let (mut a, mut b) = (1.0f64, 1.0f64);
                    if npm > 0 {
                        let x = lo(nk, npm, t);
                        a *= x.0;
                        b *= x.1;
                    }
                    for i in (0..nk).rev() {
                        let c = if i == ki22 { r22 } else { cnt[i] };
                        if c == 0 {
                            continue;
                        }
                        let x = if i == ki22 {
                            lo(i, c, t)
                        } else {
                            let gb = gm.grp(i);
                            (gm_w(gb, nu, c)[t], gm_g(gb, nu, c)[t])
                        };
                        a *= x.0;
                        b *= x.1;
                    }
                    kdw[t] = 1.0 - a;
                    kdg[t] = 1.0 - b;
                }
                // (a band's class caps are at most rho)
                let none = if best {
                    (val(&clauses[0], kdw, kdg) + key_slack(&clauses[0])) * (1.0 + 1e-9) < bestv
                } else {
                    clauses.iter().all(|cl| rho * (val(cl, kdw, kdg) + key_slack(cl)) * (1.0 + 1e-9) < cl.1)
                };
                if none {
                    continue;
                }
            }
            // per kind group and (clause, chance, outfit): products of the r best single-card ratios, r = 0..=size;
            // built only for keys with a configuration the per-target bound lets through
            let mut rpk: Option<std::rc::Rc<GTab>> = None;
            for real22 in 0..=cnt[ki22].min(by_kind[ki22].len()) {
                let filler = cnt[ki22] - real22;
                let rem = size - csum;
                for np in 0..=rem.min(pal_ents.len()) {
                    let plain = rem - np;
                    if plain > nplain {
                        continue;
                    }
                    if let Some(p) = self.dj.pals
                        && (np + plain < p || plain > p)
                    {
                        continue;
                    }
                    let gs = &mut *gsb;
                    gs.clear();
                    for i in 0..nk {
                        let c = if i == ki22 { real22 } else { cnt[i] };
                        if c > 0 {
                            gs.push((i, c));
                        }
                    }
                    if np > 0 {
                        gs.push((nk, np));
                    }
                    let gs = &**gsb;
                    if all_lk & !gs.iter().fold(0u8, |m, &(i, _)| m | glk[i]) != 0 {
                        continue;
                    }
                    // the per-target bound first, the ratio products only for configurations it lets through
                    cfg_odds(gs, &gm, dw, dg);
                    // the configuration's class cap (every deck of it is in a class at most this one's)
                    let rc = if best { rho } else { ccap[cap_class(gs.iter().fold(0u64, |m, &(i, _)| m | gcvm[i]))] };
                    let iv = if best { val(&clauses[0], dw, dg) } else { 0.0 };
                    let ideal = if cfm {
                        true
                    } else if best {
                        iv * (1.0 + 1e-9) >= bestv
                    } else {
                        clauses.iter().any(|cl| rc * val(cl, dw, dg) * (1.0 + 1e-9) >= cl.1)
                    };
                    if !ideal {
                        continue;
                    }
                    if lazy {
                        if pkey_new {
                            pkeys.push(None);
                            pkey_new = false;
                        }
                        pend.push(PendingConfig {
                            u: iv * (1.0 + 1e-12),
                            key: pkeys.len() - 1,
                            code,
                            real22,
                            np,
                            filler,
                            plain,
                            gneed,
                            gs: gs.to_vec(),
                            gm: std::rc::Rc::clone(&gm),
                        });
                        continue;
                    }
                    if rpk.is_none() {
                        rpk = Some(std::rc::Rc::new(rpk_of(mw, mg, &km.2, gneed)));
                    }
                    let rpk = rpk.as_ref().unwrap();
                    let (cv, cvo) = (&mut *cvr, &mut *cvro);
                    cfg_root(gs, rpk, dw, dg, sp0, cv, cvo);
                    let rt = |cv: &[f64]| {
                        if best {
                            cv[0]
                        } else {
                            (0..clauses.len())
                                .map(|ci| rho * cv[ci] / clauses[ci].1.max(1e-300))
                                .fold(f64::NEG_INFINITY, f64::max)
                        }
                    };
                    let (root, prune_bound) = (rt(cvo), rt(cv));
                    // (the visiting order stays by `root`; a configuration whose root at its class cap fails is left out)
                    if !best
                        && !cfm
                        && rc < rho
                        && (0..clauses.len()).all(|ci| rc * cv[ci] * (1.0 + 1e-9) < clauses[ci].1)
                    {
                        continue;
                    }
                    cfgs.push(SearchConfig {
                        root,
                        code,
                        code_real22: real22,
                        np,
                        filler,
                        plain,
                        gs: gs.to_vec(),
                        gm: std::rc::Rc::clone(&gm),
                        rpk: std::rc::Rc::clone(rpk),
                        prune_bound,
                        clause_bounds: if rbc { cv.to_vec() } else { vec![] },
                    });
                }
            }
        }
        // stable: equal bounds keep enumeration order
        cfgs.sort_by(|a, b| b.root.partial_cmp(&a.root).unwrap());
        ConfigurationQueue { cfgs, pend, pkeys }
    }

    /// Traverse the prepared configuration queue. Shared bound callbacks remain statically
    /// dispatched and fill their original caches only when a visited configuration needs them.
    fn traverse_configurations(
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
        cf_rmx_of: impl Fn(usize) -> std::rc::Rc<Vec<f64>>,
        cf_rent_of: impl Fn(usize) -> std::rc::Rc<Vec<f64>>,
    ) -> SearchOutcome {
        let SearchControl {
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
        } = controls;
        let TraversalTables { ebit, groups_all, sb_rows, lp, cf_z, lbit, all_lk, oq, og, orow, zm, nm, alt } = tables;
        let ConfigurationQueue { cfgs, pend, mut pkeys } = queue;
        let TraversalScratch { mut out, mut bestv, mut dw, mut dg, mut av, rbs } = scratch;
        let t_dfs = Instant::now();
        // A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
        // whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
        // is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
        // r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
        struct BorrowBounds {
            own: Vec<(Vec<f64>, Vec<f64>)>,
            one: Vec<(Vec<f64>, Vec<f64>)>,
            rown: Vec<Vec<f64>>,
            rone: Vec<Vec<f64>>,
        }
        let mut gbc: HashMap<(u64, usize), std::rc::Rc<BorrowBounds>> = HashMap::new();
        let gb_of = |gi: usize, mw: &[f64], mg: &[f64]| -> BorrowBounds {
            let l = groups_all[gi];
            let (own, bor): (Vec<usize>, Vec<usize>) = l.iter().partition(|&&j| !self.ebor[j]);
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
        };
        // the expanded configuration's suffix tables, flat and reused: per target (g * nu + t) the products of the
        // groups' lowest misses from group g on (white, gold), per (clause, chance, outfit) (g * noq + i) those of the
        // best ratios
        let noq = oq.len();
        let (mut sufw, mut sufg, mut spf): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
        // the node bound's per-configuration constants, flat and reused (see `hopeful`)
        let (mut kab, mut kr, mut kab_have, mut kr_have): (Vec<f64>, Vec<f64>, Vec<bool>, Vec<bool>) =
            (vec![], vec![], vec![], vec![]);
        // the node bound's per-parent rows (see `hopeful`), per (depth, borrowing variant); parent ids never repeat
        // within a search
        // (`fa`: A's factors, row ix * nu; `pp`: their products from 1.0 over the first t, row ix * (nu + 1))
        struct ParentTargetBounds {
            pid: usize,
            fa: Vec<f64>,
            pp: Vec<f64>,
        }
        let mut hc: Vec<ParentTargetBounds> = vec![];
        // the same for B, per depth (B has no borrowing variant): the parent's odds 1 - ppw, 1 - ppg, and per row, built
        // at the first child that evaluates its B (`st`: the parent id it was built for), its factors and their products
        struct ParentRatioBounds {
            pid: usize,
            pdw: [f64; 32],
            pdg: [f64; 32],
            st: Vec<usize>,
            fb: Vec<f64>,
            pb: Vec<f64>,
        }
        let mut hb: Vec<ParentRatioBounds> = vec![];
        let mut pid_next = 0usize;
        // (buffers of the expanded configuration, reused: clause sums, B's odds, the DFS rows, the locked-card table)
        let (mut cvx, mut pdwx, mut pdgx): (Vec<f64>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
        let (mut st_sel, mut st_pw, mut st_pg): (Vec<usize>, Vec<f64>, Vec<f64>) = (vec![], vec![], vec![]);
        let mut avail_buf: Vec<Vec<u8>> = vec![];
        // (the class masks of the expanded configuration: csuf[goff[g] + p] = the `cv_mask`s ORed of group g's entries
        // from position p on and of every later group's)
        let (mut csuf, mut goff): (Vec<u64>, Vec<usize>) = (vec![], vec![]);
        // (a child's touched targets: their odds and the parent's factors they replace)
        let (mut tw, mut tg, mut told) = ([0.0f64; 32], [0.0f64; 32], [0.0f64; 32]);
        // (and for B: their odds 1 - pw, 1 - pg, and one row's factors with the touched ones in place)
        let (mut bdw, mut bdg, mut brow) = ([0.0f64; 32], [0.0f64; 32], [0.0f64; 32]);
        // (a parent's odds for its A factors, the first nu slots written before each use)
        let (mut bw, mut bg) = ([0.0f64; 32], [0.0f64; 32]);
        // the per-parent rows need every outfit row to read the nu targets
        let a_rows = orow.iter().all(|o| o.len() == nu);
        let oqq: Vec<f64> = oq.iter().map(|x| x.2).collect();
        // (lazy: the pending configurations by bound, highest first; the computed ones not yet visited, best root first
        // and, among equal roots, first built first, the order of the stable sort above)
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
        let mut pord: Vec<usize> = (0..pend.len()).collect();
        pord.sort_by(|&a, &b| pend[b].u.partial_cmp(&pend[a].u).unwrap());
        let mut heap: std::collections::BinaryHeap<QueuedConfig> = std::collections::BinaryHeap::default();
        let mut pp = 0usize;
        let (mut ldw, mut ldg, mut lsp0, mut lcv, mut lcvo) =
            (vec![0.0f64; nu], vec![0.0f64; nu], vec![1.0f64; nc], vec![0.0f64; 1], vec![0.0f64; 1]);
        let mut eager_i = 0usize;
        loop {
            let cf_own: SearchConfig;
            let cf: &SearchConfig = if lazy {
                // compute the pending configurations until the heap's top comes first in the sorted order: every one
                // left has a bound, so a root, below it (or cannot reach the bar)
                while pp < pord.len() {
                    let pe = &pend[pord[pp]];
                    if !(pe.u * (1.0 + 1e-9) >= bestv && heap.peek().is_none_or(|h| pe.u >= h.root)) {
                        break;
                    }
                    pp += 1;
                    // the eager build's values: the per-target bound, the key's ratio tables, sp0 and the root
                    cfg_odds(&pe.gs, &pe.gm, &mut ldw, &mut ldg);
                    if pkeys[pe.key].is_none() {
                        let km = self.key_miss(pe.code);
                        pkeys[pe.key] = Some(std::rc::Rc::new(rpk_of(&km.0, &km.1, &km.2, pe.gneed)));
                    }
                    let rpk = pkeys[pe.key].as_ref().unwrap();
                    cfg_root(&pe.gs, rpk, &ldw, &ldg, &mut lsp0, &mut lcv, &mut lcvo);
                    heap.push(QueuedConfig {
                        root: lcvo[0],
                        idx: pord[pp - 1],
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
                            clause_bounds: if rbc { lcv.clone() } else { vec![] },
                        },
                    });
                }
                match heap.pop() {
                    Some(h) => {
                        cf_own = h.cf;
                        &cf_own
                    }
                    None => break,
                }
            } else {
                if eager_i >= cfgs.len() {
                    break;
                }
                eager_i += 1;
                &cfgs[eager_i - 1]
            };
            if best && cf.root * (1.0 + 1e-9) < bestv {
                break;
            }
            if !best && !cfm && cf.root * (1.0 + 1e-9) < 1.0 {
                break;
            }
            // (the split's root bound: skip, the order kept)
            if (best && cf.prune_bound * rbsl * (1.0 + 1e-9) < bestv)
                || (!best && !cfm && cf.prune_bound * rbsl * (1.0 + 1e-9) < 1.0)
            {
                continue;
            }
            let km = self.key_miss(cf.code);
            let (mw, mg, tmk) = (&km.0, &km.1, (&*km.2));
            let (filler, plain) = (cf.filler, cf.plain);
            let gs: Vec<(&[usize], usize)> = cf.gs.iter().map(|&(i, c)| ((&**groups_all[i]), c)).collect();
            // per group of this configuration: its key's tables (gmin[g] = gm[group], rp[g] = rpk[group])
            // (blocks: gmin[g]'s row r by `gm_w`, `gm_g`; rp[g]'s column ix at ix (size + 1))
            let gmin: Vec<&[f64]> = cf.gs.iter().map(|&(i, _)| cf.gm.grp(i)).collect();
            let rp: Vec<&[f64]> = cf.gs.iter().map(|&(i, _)| cf.rpk.grp(i)).collect();
            // the same products in the same order as the per-configuration tables they replace
            {
                let ng = cf.gs.len();
                sufw.clear();
                sufw.resize((ng + 1) * nu, 1.0);
                sufg.clear();
                sufg.resize((ng + 1) * nu, 1.0);
                spf.clear();
                spf.resize((ng + 1) * nc, 1.0);
                for g in (0..ng).rev() {
                    let (gw, gg) = (gm_w(gmin[g], nu, cf.gs[g].1), gm_g(gmin[g], nu, cf.gs[g].1));
                    for t in 0..nu {
                        sufw[g * nu + t] = sufw[(g + 1) * nu + t] * gw[t];
                        sufg[g * nu + t] = sufg[(g + 1) * nu + t] * gg[t];
                    }
                }
                for g in (0..ng).rev() {
                    let rpg = rp[g];
                    for i in 0..nc {
                        spf[g * nc + i] = spf[(g + 1) * nc + i] * rpg[i * (size + 1) + cf.gs[g].1];
                    }
                }
            }
            let (sufw, sufg, spf) = ((&*sufw), (&*sufg), (&*spf));
            // the dominance tables of this configuration's groups (`dom_table`), and per flat index f (the groups' entries
            // in turn) the characters of the entries from f on
            let doms: Vec<std::rc::Rc<Vec<Vec<(u32, u32)>>>> = if dom_on {
                cf.gs.iter().map(|&(i, _)| self.dom_table(set_id, cf.code, i, groups_all[i], mw, mg, lbit)).collect()
            } else {
                vec![]
            };
            let (mut doff, mut dsuf): (Vec<usize>, Vec<u128>) = (vec![0], vec![]);
            if dom_on {
                for &(l, _) in &gs {
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
            // the child taking entry B = list[p] of group g, at a node with `start`, `left` slots of g to fill, characters
            // `chm`, gold misses `ppg`, picks `sel` and characters `need` its decks must take: None when it can be
            // skipped, else its decks' needed characters. An entry A dominating B (`dom_table`) that is free at the node
            // and takes B's place in the pick order (a sibling, or no pick of g after A touching B's targets) gives every
            // deck through B whose picks after B take no card of A's character an equal or better deck before it; when
            // nothing of A's character can follow, that is every deck through B, else those decks must take one
            // (per target, ppg * rlo, the same for every dominator: computed at its first use in a call, `prm`'s bit t
            // set; the slots are reused across calls, never read unset)
            let mut dpr = [0.0f64; 32];
            let mut dom = |g: usize,
                           p: usize,
                           start: usize,
                           left: usize,
                           chm: u128,
                           ppg: &[f64],
                           sel: &[usize],
                           need: u128|
             -> Option<u128> {
                if !dom_on {
                    return Some(need);
                }
                let l = gs[g].0;
                let bj = l[p];
                let f = doff[g] + p;
                let mut nd = need;
                let dl = &doms[g][p];
                if !dl.is_empty() {
                    // (the deck's picks in group g so far, all at positions below `start`)
                    let sg = &sel[sel.len() - (gs[g].1 - left)..];
                    let (pr, mut prm) = (&mut dpr, 0u32);
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
                                    let rlo = gm_g(gmin[g], nu, left - 1)[t] * sufg[(g + 1) * nu + t];
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
            };
            // the one-borrowed-card tables of this configuration's groups and their suffixes (`own`: no borrowed card
            // left to place; `one`: at most one, in one group)
            let one_b = self.allowed_borrow == 1 && gs.iter().any(|(l, _)| l.iter().any(|&j| self.ebor[j]));
            let gbs: Vec<std::rc::Rc<BorrowBounds>> = if one_b {
                cf.gs
                    .iter()
                    .map(|&(i, _)| {
                        std::rc::Rc::clone(
                            gbc.entry((cf.code, i)).or_insert_with(|| std::rc::Rc::new(gb_of(i, mw, mg))),
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
            cvx.clear();
            cvx.resize(clauses.len(), 0.0);
            pdwx.clear();
            pdwx.resize(nu, 0.0);
            pdgx.clear();
            pdgx.resize(nu, 0.0);
            let (cv, pdw, pdg) = ((&mut *cvx), (&mut *pdwx), (&mut *pdgx));
            let ng = gs.len();
            if !best {
                goff.clear();
                goff.push(0);
                for g in 0..ng {
                    goff.push(goff[g] + gs[g].0.len());
                }
                csuf.clear();
                csuf.resize(goff[ng] + 1, 0);
                for g in (0..ng).rev() {
                    for (p, &j) in gs[g].0.iter().enumerate().rev() {
                        csuf[goff[g] + p] = csuf[goff[g] + p + 1] | self.cvm[j];
                    }
                }
            }
            let (csuf, goff) = ((&*csuf), (&*goff));
            // the state-conditioned bound of this configuration: chain positions (sorted kinds, the key's state
            // distribution projected on the positions whose group has chain targets), per group its slots' positions
            // and, per chain level, each target's lowest misses over the group's entries (a slot may take any of them,
            // the same one twice: a relaxation)
            let sbkey = (set_id, cf.code, cf.code_real22, cf.np);
            let sbc: Option<std::rc::Rc<SbCfg>> = if sbm == 0 {
                None
            } else if let Some(r) = self.sbcfg.borrow().get(&sbkey).cloned() {
                Some(r)
            } else {
                let mut kinds: Vec<(Kind, usize)> = vec![]; // (kind, group, or usize::MAX for a filler)
                for (g, &(i, c)) in cf.gs.iter().enumerate() {
                    if i < nk {
                        for _ in 0..c {
                            kinds.push((self.kinds_all[i], g));
                        }
                    }
                }
                for _ in 0..filler {
                    kinds.push(((2, 2, 0), usize::MAX));
                }
                kinds.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
                let kl: Vec<Kind> = kinds.iter().map(|x| x.0).collect();
                let full = if kl.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kl) };
                let has_chain: Vec<bool> =
                    gs.iter().map(|(l, _)| l.iter().any(|&j| !self.srcs[j].cm.is_empty())).collect();
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
                    cf.gs.iter().map(|&(i, _)| if i < nk { self.kinds_all[i].1 as usize } else { 0 }).collect();
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
                let r =
                    std::rc::Rc::new(SbCfg { states, slots, gmin, kl, rel_pos, order: std::cell::OnceCell::default() });
                self.sbcfg.borrow_mut().insert(sbkey, std::rc::Rc::clone(&r));
                Some(r)
            };
            // the bound at a partial deck `sel` (entries in pick order, filling the groups in turn)
            let sb_ub = |sel: &[usize]| -> f64 {
                let c = sbc.as_ref().unwrap();
                let (mut kg, mut kh) = ([1.0f64; 32], [1.0f64; 32]);
                let mut chain: Vec<(usize, &[(f64, f64)])> = vec![];
                let mut idx = 0usize;
                for g in 0..ng {
                    for t in 0..gs[g].1 {
                        let rows: &[(f64, f64)] =
                            if idx < sel.len() { sb_rows[sel[idx]].as_deref().unwrap() } else { &c.gmin[g] };
                        idx += 1;
                        let p = c.slots[g][t];
                        if p == usize::MAX {
                            for k in 0..nu {
                                kg[k] *= rows[k].0;
                                kh[k] *= rows[k].1;
                            }
                        } else {
                            chain.push((p, rows));
                        }
                    }
                }
                let opts = &model.opts[sb_u];
                let mut acc = [0.0f64; 16];
                let (mut xg, mut xh) = ([0.0f64; 32], [0.0f64; 32]);
                for (x, w) in &c.states {
                    xg[..nu].copy_from_slice(&kg[..nu]);
                    xh[..nu].copy_from_slice(&kh[..nu]);
                    for &(p, rows) in &chain {
                        let b = x[p] as usize * nu;
                        for k in 0..nu {
                            xg[k] *= rows[b + k].0;
                            xh[k] *= rows[b + k].1;
                        }
                    }
                    for o in 0..opts.len() {
                        let mut f = 1.0f64;
                        for k in 0..nu {
                            f *= lp[o * nu + k].f(xg[k], xh[k]);
                        }
                        acc[o] += w * f;
                    }
                }
                fixed * acc[..opts.len()].iter().copied().fold(0.0f64, f64::max)
            };
            // (each slot's group and chain position in pick order, the chain slots, each group's best ratios)
            let mut cc_slot: Vec<(usize, usize)> = vec![];
            let mut cc_grm: Vec<std::rc::Rc<Vec<f64>>> = vec![];
            if cfm && let Some(c) = &sbc {
                for g in 0..ng {
                    for t in 0..gs[g].1 {
                        cc_slot.push((g, c.slots[g][t]));
                    }
                }
                cc_grm = (0..ng).map(|g| cf_rmx_of(cf.gs[g].0)).collect();
            }
            let cc_chain: Vec<usize> = (0..cc_slot.len()).filter(|&i| cc_slot[i].1 != usize::MAX).collect();
            let cc_cmax = sbc.as_ref().map_or(0, |c| c.kl.iter().map(|k| k.1 as usize).max().unwrap_or(0));
            // The cap-free band's bound (`cfm`): per chain state x the lower of `sb_ub`'s per-target bound A_x and a ratio
            // bound B_x, summed over states, the best outfit. Derived: given x the deck's state value is a product over
            // targets of phi_k at its misses given x (phi_k = G - (G - W) ng - W a nh, a = (1 - e)(1 - q)); its log gains
            // less from a card the lower the misses already are (phi(ng y, nh y') / phi(ng, nh) rises with ng and with
            // nh for y' <= y <= 1), so over the targets N with phi_k(1, 1) > 0 the completion's product is at most the
            // partial deck's times each open slot's card's ratio from no card, at most the best ratio of its group at the
            // slot's level (`cf_rmx_of`); the zero-base targets Z take A's factors. Every completion's state value is at
            // most both, so their minimum bounds it state by state, and the sum over states (`state_value`'s argument,
            // `Src::cf_ok`) bounds its exact value.
            // `cut` (> 0: the value a node must reach to pass): the states are taken heaviest first and the sum stops once
            // it decides the test, returning its partial sum plus the weight left times every state's most (A with every
            // slot at its most links done, at least any state's term): an upper bound either way, below `cut` exactly
            // when the full sum's test would fail to the slack. 0: the full sum, in state order.
            // (Kept between calls, this configuration's. Per number of filled slots and visiting order (`maps`), each
            // state's classes in the order the states are visited, extended as far as a call has reached: `fid` the
            // class of its levels at the filled chain positions (`fk`), `rep` the first state visited whose levels agree
            // at the filled positions and, per group, up to their order at its open positions (`ck`). Per call (`stamp`):
            // each filled class's misses and ratio-bound factor (`fx`, `fst` its call), each visited state's terms.)
            struct StateClasses {
                fid: Vec<u32>,
                rep: Vec<u32>,
                fk: crate::FxMap<u64, u32>,
                ck: crate::FxMap<u64, u32>,
            }
            struct StateBoundCache {
                maps: Vec<Option<StateClasses>>,
                fst: Vec<u32>,
                fx: Vec<f64>,
                term: Vec<f64>,
                stamp: u32,
            }
            let ub_cache = std::cell::RefCell::new(StateBoundCache {
                maps: vec![],
                fst: vec![],
                fx: vec![],
                term: vec![],
                stamp: 0,
            });
            let cf_ub = |sel: &[usize], cut: f64| -> f64 {
                let c = sbc.as_ref().unwrap();
                // filled slots without a level (pg, ph), open ones (ag, ah: their groups' lowest misses; rn: their best
                // ratios); filled chain slots (position, rows), open ones (position, lowest rows, ratio table, group);
                // every slot at its most links done (ug, uh)
                let (mut pg, mut ph, mut ag, mut ah) = ([1.0f64; 32], [1.0f64; 32], [1.0f64; 32], [1.0f64; 32]);
                let (mut ug, mut uh) = ([1.0f64; 32], [1.0f64; 32]);
                let mut rn = [1.0f64; 16];
                let (mut fch, mut nf): ([(usize, &[(f64, f64)]); 16], usize) = ([(0, &[]); 16], 0);
                let (mut ochv, mut no_): ([(usize, &[(f64, f64)], &[f64], usize); 16], usize) =
                    ([(0, &[], &[], 0); 16], 0);
                let mut open = false;
                let mut idx = 0usize;
                for g in 0..ng {
                    for t in 0..gs[g].1 {
                        let filled = idx < sel.len();
                        let rows: &[(f64, f64)] =
                            if filled { sb_rows[sel[idx]].as_deref().unwrap() } else { &c.gmin[g] };
                        idx += 1;
                        let top = &rows[(rows.len() / nu - 1) * nu..];
                        for k in 0..nu {
                            ug[k] *= top[k].0;
                            uh[k] *= top[k].1;
                        }
                        let p = c.slots[g][t];
                        if filled {
                            if p == usize::MAX {
                                for k in 0..nu {
                                    pg[k] *= rows[k].0;
                                    ph[k] *= rows[k].1;
                                }
                            } else {
                                fch[nf] = (p, rows);
                                nf += 1;
                            }
                        } else {
                            open = true;
                            let rm = &cc_grm[g][..];
                            if p == usize::MAX {
                                for k in 0..nu {
                                    ag[k] *= rows[k].0;
                                    ah[k] *= rows[k].1;
                                }
                                for o in 0..cf_no {
                                    rn[o] *= rm[o];
                                }
                            } else {
                                ochv[no_] = (p, rows, rm, g);
                                no_ += 1;
                            }
                        }
                    }
                }
                let (fch, och) = (&fch[..nf], &ochv[..no_]);
                let opts = &model.opts[sb_u];
                let no = opts.len();
                let early = cut > 0.0;
                let mut umax = [0.0f64; 16];
                if early {
                    for o in 0..opts.len() {
                        let mut a = 1.0f64;
                        for k in 0..nu {
                            a *= lp[o * nu + k].f(ug[k], uh[k]);
                        }
                        umax[o] = a;
                    }
                }
                let ns = c.states.len();
                let (ord, tail) = if early {
                    let x = c.order.get_or_init(|| {
                        let mut o: Vec<u32> = (0..ns as u32).collect();
                        o.sort_by(|&a, &b| {
                            c.states[b as usize].1.partial_cmp(&c.states[a as usize].1).unwrap().then(a.cmp(&b))
                        });
                        let mut t = vec![0.0f64; ns + 1];
                        for i in (0..ns).rev() {
                            t[i] = t[i + 1] + c.states[o[i] as usize].1;
                        }
                        (o, t)
                    });
                    ((&*x.0), (&*x.1))
                } else {
                    (&[][..], &[][..])
                };
                let mut acc = [0.0f64; 16];
                let mut chk = true;
                let (mut xg, mut xh) = ([0.0f64; 32], [0.0f64; 32]);
                // x*: state x's misses of the partial deck (its level-free filled slots', then its filled chain slots'
                // rows at x's levels)
                let xstar = |x: &[u8], xg: &mut [f64; 32], xh: &mut [f64; 32]| {
                    xg[..nu].copy_from_slice(&pg[..nu]);
                    xh[..nu].copy_from_slice(&ph[..nu]);
                    for &(p, rows) in fch {
                        let r = &rows[x[p] as usize * nu..][..nu];
                        for ((y, z), rk) in xg[..nu].iter_mut().zip(xh[..nu].iter_mut()).zip(r) {
                            *y *= rk.0;
                            *z *= rk.1;
                        }
                    }
                };
                // per outfit, the ratio bound's factor of the targets outside Z at x*
                let bn_of = |o: usize, xg: &[f64], xh: &[f64]| -> f64 {
                    let mut bn = 1.0f64;
                    for k in 0..nu {
                        if cf_z[o] >> k & 1 == 0 {
                            bn *= lp[o * nu + k].f(xg[k], xh[k]);
                        }
                    }
                    bn
                };
                // the state's term per outfit from x* and its bn (open slots only): y*, every open slot at its group's
                // lowest, then A at y* (and Z's part of it, bz), with an open slot min(A, bz bn r) (r: the open slots' best
                // ratios at x's levels), else A
                let terms = |x: &[u8], xg: &[f64], xh: &[f64], bn: &[f64; 16], f: &mut [f64; 16]| {
                    let (mut yg, mut yh) = ([0.0f64; 32], [0.0f64; 32]);
                    for k in 0..nu {
                        yg[k] = xg[k] * ag[k];
                        yh[k] = xh[k] * ah[k];
                    }
                    for &(p, rows, _, _) in och {
                        let r = &rows[x[p] as usize * nu..][..nu];
                        for ((y, z), rk) in yg[..nu].iter_mut().zip(yh[..nu].iter_mut()).zip(r) {
                            *y *= rk.0;
                            *z *= rk.1;
                        }
                    }
                    for o in 0..no {
                        let (mut a, mut bz) = (1.0f64, 1.0f64);
                        for k in 0..nu {
                            let fa = lp[o * nu + k].f(yg[k], yh[k]);
                            a *= fa;
                            if cf_z[o] >> k & 1 != 0 {
                                bz *= fa;
                            }
                        }
                        f[o] = if open {
                            let mut r = rn[o];
                            for &(p, _, rm, _) in och {
                                r *= rm[x[p] as usize * cf_no + o];
                            }
                            a.min(bz * bn[o] * r)
                        } else {
                            a
                        };
                    }
                };
                let (mut bnv, mut fv) = ([0.0f64; 16], [0.0f64; 16]);
                // With an open chain slot, the states share work. A state's filled part (its misses x* given x and, per
                // outfit, the ratio bound's factor of the targets outside Z at x*) depends only on its levels at the
                // filled positions, so it is computed once per class (the same expressions on the same f64s: the same
                // values). And the term of a state is a symmetric function of the levels at one group's open positions
                // (those slots share their group's lowest rows and best ratios, multiplied in), so the states that agree
                // up to that order have one term (exactly, in real arithmetic), taken from the first of them visited:
                // the sum is the same, up to the rounding of the products.
                let mut ucache = ub_cache.borrow_mut();
                let memo = !och.is_empty() && cc_cmax < 16;
                if memo {
                    let uc = &mut *ucache;
                    let mi = sel.len() * 2 + usize::from(early);
                    if uc.maps.len() <= mi {
                        uc.maps.resize_with(mi + 1, || None);
                    }
                    uc.stamp = uc.stamp.wrapping_add(1);
                    if uc.stamp == 0 {
                        uc.fst.iter_mut().for_each(|x| *x = 0);
                        uc.stamp = 1;
                    }
                }
                for i in 0..ns {
                    let (x, w) = if early { &c.states[ord[i] as usize] } else { &c.states[i] };
                    if memo {
                        let StateBoundCache { maps, fst, fx, term, stamp } = &mut *ucache;
                        let mp = maps[sel.len() * 2 + usize::from(early)].get_or_insert_with(|| StateClasses {
                            fid: vec![],
                            rep: vec![],
                            fk: HashMap::default(),
                            ck: HashMap::default(),
                        });
                        if mp.rep.len() <= i {
                            // (the state's keys: its levels at the filled positions, then per group its open positions'
                            // levels sorted; 4 bits each, at most 16 positions)
                            let mut fkey = 0u64;
                            for &(p, _) in fch {
                                fkey = fkey << 4 | u64::from(x[p]);
                            }
                            let mut ckey = fkey;
                            let mut s0 = 0usize;
                            while s0 < och.len() {
                                let mut s1 = s0;
                                while s1 < och.len() && och[s1].3 == och[s0].3 {
                                    s1 += 1;
                                }
                                let mut lv = [0u8; 16];
                                for s in s0..s1 {
                                    lv[s - s0] = x[och[s].0];
                                }
                                lv[..s1 - s0].sort_unstable();
                                for &l in &lv[..s1 - s0] {
                                    ckey = ckey << 4 | u64::from(l);
                                }
                                s0 = s1;
                            }
                            let nfk = mp.fk.len() as u32;
                            let f = *mp.fk.entry(fkey).or_insert(nfk);
                            let r = *mp.ck.entry(ckey).or_insert(i as u32);
                            mp.fid.push(f);
                            mp.rep.push(r);
                        }
                        let r = mp.rep[i] as usize;
                        if term.len() < (i + 1) * no {
                            term.resize((i + 1) * no, 0.0);
                        }
                        if r == i {
                            let f = mp.fid[i] as usize;
                            let fw = 2 * nu + no;
                            if fst.len() <= f {
                                fst.resize(f + 1, 0);
                                fx.resize((f + 1) * fw, 0.0);
                            }
                            let fxr = &mut fx[f * fw..(f + 1) * fw];
                            if fst[f] != *stamp {
                                fst[f] = *stamp;
                                xstar(x, &mut xg, &mut xh);
                                fxr[..nu].copy_from_slice(&xg[..nu]);
                                fxr[nu..2 * nu].copy_from_slice(&xh[..nu]);
                                for o in 0..no {
                                    fxr[2 * nu + o] = bn_of(o, &xg, &xh);
                                }
                            }
                            bnv[..no].copy_from_slice(&fxr[2 * nu..2 * nu + no]);
                            terms(x, &fxr[..nu], &fxr[nu..2 * nu], &bnv, &mut fv);
                            for o in 0..no {
                                term[i * no + o] = fv[o];
                                acc[o] += w * fv[o];
                            }
                        } else {
                            for o in 0..no {
                                acc[o] += w * term[r * no + o];
                            }
                        }
                    } else {
                        xstar(x, &mut xg, &mut xh);
                        if open {
                            for o in 0..no {
                                bnv[o] = bn_of(o, &xg, &xh);
                            }
                        }
                        terms(x, &xg, &xh, &bnv, &mut fv);
                        for o in 0..no {
                            acc[o] += w * fv[o];
                        }
                    }
                    // (every 4 states: a test decided by a partial sum is the full sum's, so how often it is tried
                    // changes no outcome, only where the sum stops)
                    if early && chk && (i & 3 == 3 || i + 1 == ns) {
                        // (the weight left with a hair of slack for its rounding)
                        let rem = tail[i + 1] * (1.0 + 1e-12);
                        let (mut hi, mut lo) = (0.0f64, 0.0f64);
                        for o in 0..opts.len() {
                            hi = hi.max(acc[o] + rem * umax[o]);
                            lo = lo.max(acc[o]);
                        }
                        // (an infinite or NaN term: no early stop, the full sum)
                        if !hi.is_finite() {
                            chk = false;
                            continue;
                        }
                        if fixed * hi * (1.0 + 1e-9) < cut || fixed * lo * (1.0 + 1e-9) >= cut {
                            return fixed * hi;
                        }
                    }
                }
                fixed * acc[..opts.len()].iter().copied().fold(0.0f64, f64::max)
            };
            let sb_ub = |sel: &[usize]| -> f64 { if cfm { cf_ub(sel, 0.0) } else { sb_ub(sel) } };
            // The cap-free band's cheap bound, tried before `cf_ub` (a node either fails is pruned): given N = n events
            // only. Derived: per state the completion's value is at most prod_Z phi_k at its lowest misses (every slot
            // at its most links done; the zero-base targets) times prod_N phi_k at the filled level-free slots' misses
            // times, per chain slot, its card's ratio at its level over that base (a filled slot's own, an open slot's
            // group's best from no card, `cf_rmx_of`) and each open level-free slot's best ratio: the log-submodularity
            // of `cf_ub`, card by card. Each chain slot's ratio rises with its level (its misses only fall), so given
            // N = n, where the configuration's progress vector is negatively upper-orthant dependent (`key_level`,
            // checked exactly, the projection on these positions included), the expected product is at most the product
            // of the slots' expected ratios at P(level | n); summed over P(n). Infinite (no prune) when the key fails
            // the orthant check.
            let cf_kl: Option<std::rc::Rc<KeyLevel>> =
                if cfm { sbc.as_ref().and_then(|c| (!c.kl.is_empty()).then(|| key_level(&c.kl))) } else { None };
            let cf_pmf = if cfm { event_pmf_job() } else { std::rc::Rc::new(vec![]) };
            // (the event counts summed term by term, those of P(n) >= 1e-12; the rest's mass, each such term at most A
            // at every slot's most links done)
            let cf_ns: Vec<usize> = (0..cf_pmf.len()).filter(|&n| cf_pmf[n] >= 1e-12).collect();
            let cf_rest: f64 = (0..cf_pmf.len()).filter(|&n| cf_pmf[n] < 1e-12).map(|n| cf_pmf[n]).sum::<f64>()
                * (1.0 + 1e-9)
                + 1e-300;
            // (Kept between calls, this configuration's, the same values as computing each from scratch: the products are
            // formed in the same order on the same f64s. `sel`: the deck whose slots 0..sel.len() the prefix rows hold,
            // `z` per depth i and level cap the products of slots 0..i's rows (zg, zh), `p` those of their level-free
            // slots' level-0 rows (pg, ph: slots fill in order, so the filled ones come first), `tab` each filled chain
            // slot's ratio table; per outfit and chain slot its last ratios (`rs`, `rsl` levels, 0 none yet), their
            // expectations per event count (`e`) and the product of the expectations of the chain slots up to it (`pp`):
            // a slot whose ratios are unchanged keeps its expectations, and the products are redone from the first
            // slot that changed.)
            struct EventBoundCache {
                sel: Vec<usize>,
                tab: Vec<Option<std::rc::Rc<Vec<f64>>>>,
                z: Vec<f64>,
                p: Vec<f64>,
                zw: Vec<f64>,
                ac: Vec<f64>,
                rs: Vec<f64>,
                rsl: Vec<usize>,
                rsn: Vec<f64>,
                e: Vec<f64>,
                pp: Vec<f64>,
            }
            let cc_zall: u32 = cf_z.iter().fold(0u32, |m, &z| m | z);
            // (at most 16 chain slots, as outfits)
            assert!(cc_chain.len() <= 16, "{} chain slots: at most 16", cc_chain.len());
            let cc_cache = std::cell::RefCell::new({
                let (nsl, cz, nch, nn, on) =
                    (cc_slot.len(), cc_cmax + 1, cc_chain.len(), cf_ns.len(), usize::from(cfm));
                EventBoundCache {
                    sel: vec![],
                    tab: vec![None; on * nsl],
                    z: vec![1.0f64; on * (nsl + 1) * cz * nu * 2],
                    p: vec![1.0f64; on * (nsl + 1) * nu * 2],
                    zw: vec![1.0f64; on * cz * nu * 2],
                    ac: vec![0.0f64; on * cz],
                    rs: vec![0.0f64; cf_no * nch * cz],
                    rsl: vec![0usize; cf_no * nch],
                    rsn: vec![0.0f64; nch * cz],
                    e: vec![0.0f64; cf_no * nch * nn],
                    pp: vec![0.0f64; cf_no * nch * nn],
                }
            });
            let cf_cheap = |sel: &[usize]| -> f64 {
                let c = sbc.as_ref().unwrap();
                if cf_kl.as_ref().is_some_and(|k| !k.nuod) {
                    return f64::INFINITY;
                }
                let mut cache = cc_cache.borrow_mut();
                let EventBoundCache { sel: csel, tab, z, p, zw, ac, rs: crs, rsl, rsn, e: ce, pp } = &mut *cache;
                let (nsl, cmax, nch, nn) = (cc_slot.len(), cc_cmax, cc_chain.len(), cf_ns.len());
                let cz = cmax + 1;
                let d = sel.len();
                // the prefix rows: kept up to the first pick that differs from the last deck's, extended to this one's
                let mut l0 = 0usize;
                while l0 < d && l0 < csel.len() && csel[l0] == sel[l0] {
                    l0 += 1;
                }
                csel.truncate(l0);
                for i in l0..d {
                    let rows: &[(f64, f64)] = sb_rows[sel[i]].as_deref().unwrap();
                    let nl = rows.len() / nu;
                    let (zlo, zhi) = z.split_at_mut((i + 1) * cz * nu * 2);
                    let (zp, zc) = (&zlo[i * cz * nu * 2..], &mut zhi[..cz * nu * 2]);
                    for cc in 0..cz {
                        let r = &rows[cc.min(nl - 1) * nu..][..nu];
                        let b = cc * nu * 2;
                        for ((z2, p2), rk) in
                            zc[b..b + 2 * nu].chunks_exact_mut(2).zip(zp[b..b + 2 * nu].chunks_exact(2)).zip(r)
                        {
                            z2[0] = p2[0] * rk.0;
                            z2[1] = p2[1] * rk.1;
                        }
                    }
                    let (plo, phi) = p.split_at_mut((i + 1) * nu * 2);
                    let (pp0, pc) = (&plo[i * nu * 2..], &mut phi[..nu * 2]);
                    if cc_slot[i].1 == usize::MAX {
                        for k in 0..nu {
                            pc[2 * k] = pp0[2 * k] * rows[k].0;
                            pc[2 * k + 1] = pp0[2 * k + 1] * rows[k].1;
                        }
                    } else {
                        pc.copy_from_slice(pp0);
                        tab[i] = Some(cf_rent_of(sel[i]));
                    }
                    csel.push(sel[i]);
                }
                // filled level-free slots' misses (pg, ph); per level cap c (every slot at most c links done, at most its
                // stop) each slot at its lowest misses (zw at c: the prefix's, then the open slots'); open level-free
                // slots' best ratios (rn); every level-free slot's lowest misses (lg, lh: the zero-base targets'
                // level-free part)
                let (mut pg, mut ph) = ([1.0f64; 32], [1.0f64; 32]);
                for k in 0..nu {
                    pg[k] = p[d * nu * 2 + 2 * k];
                    ph[k] = p[d * nu * 2 + 2 * k + 1];
                }
                zw.copy_from_slice(&z[d * cz * nu * 2..(d + 1) * cz * nu * 2]);
                let (mut lg, mut lh) = (pg, ph);
                let mut rn = [1.0f64; 16];
                for i in d..nsl {
                    let (g, pos) = cc_slot[i];
                    let rows: &[(f64, f64)] = &c.gmin[g];
                    let nl = rows.len() / nu;
                    for cc in 0..cz {
                        let r = &rows[cc.min(nl - 1) * nu..][..nu];
                        let b = cc * nu * 2;
                        for (z2, rk) in zw[b..b + 2 * nu].chunks_exact_mut(2).zip(r) {
                            z2[0] *= rk.0;
                            z2[1] *= rk.1;
                        }
                    }
                    if pos == usize::MAX {
                        for k in 0..nu {
                            lg[k] *= rows[k].0;
                            lh[k] *= rows[k].1;
                        }
                        let rm = &cc_grm[g];
                        for o in 0..cf_no {
                            rn[o] *= rm[o];
                        }
                    }
                }
                // the chain slots, in pick order: (position, rows (an open slot's: its group's lowest), filled, ratio
                // table (a filled slot's own from no card, an open slot's group's best), levels)
                let mut chv: [(usize, &[(f64, f64)], bool, &[f64], usize); 16] = [(0, &[], false, &[], 0); 16];
                for (s, &i) in cc_chain.iter().enumerate() {
                    let (g, pos) = cc_slot[i];
                    let (rows, filled, rm): (&[(f64, f64)], bool, &[f64]) = if i < d {
                        (sb_rows[sel[i]].as_deref().unwrap(), true, &tab[i].as_ref().unwrap()[..])
                    } else {
                        ((&*c.gmin[g]), false, &cc_grm[g][..])
                    };
                    chv[s] = (pos, rows, filled, rm, rows.len() / nu);
                }
                let chs = |s: usize| chv[s];
                // (no filled level-free slot: the filled chain slots' ratios over that base are their tabled ratios from
                // no card)
                let base_empty = pg[..nu].iter().all(|&x| x == 1.0) && ph[..nu].iter().all(|&x| x == 1.0);
                // each zero-base target's factor as a function of one chain slot's level: the slot whose own misses fall
                // the most from no links to all (its `zs`), every other slot at its lowest (a factor at least the
                // target's on every state, rising with that slot's level); None: no chain slot's misses of it fall with
                // its level. Per zero-base target: its misses with every slot at its lowest but its `zs` slot (every slot
                // when None). (Only the zero-base targets of some outfit are used.)
                let (mut zs, mut zrest) = ([None::<usize>; 32], [(1.0f64, 1.0f64); 32]);
                if cc_zall != 0 {
                    for k in 0..nu {
                        if cc_zall >> k & 1 == 0 {
                            continue;
                        }
                        let mut b: Option<(usize, f64)> = None;
                        for si in 0..nch {
                            let (_, rows, _, _, nl) = chs(si);
                            let (r0, rt) = (rows[k].1, rows[(nl - 1) * nu + k].1);
                            let drop = if r0 > 0.0 { rt / r0 } else { 1.0 };
                            if drop < 1.0 && b.is_none_or(|x| drop < x.1) {
                                b = Some((si, drop));
                            }
                        }
                        zs[k] = b.map(|x| x.0);
                        let (mut a, mut bb) = (lg[k], lh[k]);
                        for si in 0..nch {
                            if zs[k] != Some(si) {
                                let (_, rows, _, _, nl) = chs(si);
                                a *= rows[(nl - 1) * nu + k].0;
                                bb *= rows[(nl - 1) * nu + k].1;
                            }
                        }
                        zrest[k] = (a, bb);
                    }
                }
                let opts = &model.opts[sb_u];
                let mut best = 0.0f64;
                for o in 0..opts.len() {
                    // per cap: every target at its lowest misses (A)
                    for cc in 0..cz {
                        let mut a = 1.0f64;
                        let b = cc * nu * 2;
                        for (lpk, z2) in lp[o * nu..(o + 1) * nu].iter().zip(zw[b..b + 2 * nu].chunks_exact(2)) {
                            a *= lpk.f(z2[0], z2[1]);
                        }
                        ac[cc] = a;
                    }
                    let mut pb = 1.0f64;
                    for k in 0..nu {
                        if cf_z[o] >> k & 1 == 0 {
                            pb *= lp[o * nu + k].f(pg[k], ph[k]);
                        }
                    }
                    // per chain slot and level its ratio (rsn, slot after slot at stride cz), times the zero-base targets
                    // it carries
                    for s in 0..nch {
                        let (_, rows, filled, rm, nl) = chs(s);
                        for l in 0..nl {
                            rsn[s * cz + l] = if !filled || base_empty {
                                rm[l * cf_no + o]
                            } else {
                                let mut x = 1.0f64;
                                for k in 0..nu {
                                    if cf_z[o] >> k & 1 == 0 {
                                        x *= lp[o * nu + k].f(pg[k] * rows[l * nu + k].0, ph[k] * rows[l * nu + k].1);
                                    }
                                }
                                x / pb
                            };
                        }
                    }
                    let mut zconst = 1.0f64;
                    for k in 0..nu {
                        if cf_z[o] >> k & 1 == 0 {
                            continue;
                        }
                        match zs[k] {
                            None => {
                                zconst *= lp[o * nu + k].f(zrest[k].0, zrest[k].1);
                            }
                            Some(s0) => {
                                let (_, rows, _, _, nl) = chs(s0);
                                for l in 0..nl {
                                    rsn[s0 * cz + l] *= lp[o * nu + k]
                                        .f(zrest[k].0 * rows[l * nu + k].0, zrest[k].1 * rows[l * nu + k].1);
                                }
                            }
                        }
                    }
                    let base = pb * rn[o] * zconst;
                    // per n: the lower of A at the cap min(n, cmax) and the ratio bound
                    let tot = match &cf_kl {
                        None => ac[0].min(base),
                        Some(kl) => {
                            // each chain slot's expectation at P(level | n) and their running product, from the first slot
                            // whose ratios changed
                            let mut redo = false;
                            for s in 0..nch {
                                let (pos, _, _, _, nl) = chs(s);
                                let (cs, ci) = ((o * nch + s) * cz, o * nch + s);
                                let same = rsl[ci] == nl
                                    && (0..nl).all(|l| crs[cs + l].to_bits() == rsn[s * cz + l].to_bits());
                                let eo = ci * nn;
                                if !same {
                                    rsl[ci] = nl;
                                    crs[cs..cs + nl].copy_from_slice(&rsn[s * cz..s * cz + nl]);
                                    let rr = &rsn[s * cz..s * cz + nl];
                                    for (ni, &n) in cf_ns.iter().enumerate() {
                                        let m = &kl.marg[c.rel_pos[pos]][n][..nl];
                                        let mut x = 0.0f64;
                                        for (&ml, &rl) in m.iter().zip(rr) {
                                            x += ml * rl;
                                        }
                                        ce[eo + ni] = x;
                                    }
                                    redo = true;
                                }
                                if redo {
                                    for ni in 0..nn {
                                        pp[eo + ni] = if s == 0 { 1.0 } else { pp[eo - nn + ni] } * ce[eo + ni];
                                    }
                                }
                            }
                            let mut tot = cf_rest * ac[cmax];
                            let last = (o * nch + nch.max(1) - 1) * nn;
                            for (ni, &n) in cf_ns.iter().enumerate() {
                                let pn = cf_pmf[n];
                                let prod = if nch == 0 { 1.0f64 } else { pp[last + ni] };
                                tot += pn * ac[n.min(cmax)].min(base * prod);
                            }
                            tot
                        }
                    };
                    if tot > best || tot.is_nan() {
                        best = if tot.is_nan() { f64::INFINITY } else { tot };
                    }
                }
                fixed * best
            };
            // the configuration's root: no deck of it can reach the bar
            let sb_root = if cfm {
                let x = cf_cheap(&[]);
                if cfs == 0.0 && x * (1.0 + 1e-9) < sb_bar {
                    x
                } else {
                    x.min(cf_ub(&[], if cfs == 0.0 { sb_bar } else { 0.0 }))
                }
            } else if sbm > 0 {
                sb_ub(&[])
            } else {
                0.0
            };
            if cfm {
                // (the cap-free band: past its node limit it stops; CF_CHECK passes the root at cfs times its bound)
                if self.nodes.get() > cf_lim {
                    return SearchOutcome { decks: out, incomplete: Some(SearchStop::CapFreeNodeLimit) };
                }
                if sb_root * cfs.max(1.0) * (1.0 + 1e-9) < sb_bar {
                    SBN.with(|c| c.borrow_mut().0 += 1);
                    continue;
                }
            } else if sbm > 0 && sb_root * (1.0 + 1e-9) < sb_bar {
                SBN.with(|c| c.borrow_mut().0 += 1);
                continue;
            }
            // (CF_CHECK: the bounds of the nodes on the current path, per depth, the root's at 0)
            let cf_path: std::cell::RefCell<Vec<f64>> =
                std::cell::RefCell::new(if cfs > 0.0 { vec![sb_root] } else { vec![] });
            let cf_over = std::cell::Cell::new(false);
            {
                {
                    // the partial decks' miss products, one row of `nu` per depth (the last row: the deck so far)
                    struct DeckPath {
                        sel: Vec<usize>,
                        pw: Vec<f64>,
                        pg: Vec<f64>,
                        pid: usize,
                    }
                    // (pw, pg: one row of nu per depth 0 ..= the deck's slots, row d at d * nu: the partial deck with d cards)
                    let nrows = gs.iter().map(|x| x.1).sum::<usize>() + 1;
                    let mut st = DeckPath {
                        sel: std::mem::take(&mut st_sel),
                        pw: std::mem::take(&mut st_pw),
                        pg: std::mem::take(&mut st_pg),
                        pid: pid_next,
                    };
                    st.sel.clear();
                    st.pw.clear();
                    st.pw.resize(nrows * nu, 1.0);
                    st.pg.clear();
                    st.pg.resize(nrows * nu, 1.0);
                    let nhc = 2 * (gs.iter().map(|x| x.1).sum::<usize>() + 1);
                    while hc.len() < nhc {
                        hc.push(ParentTargetBounds {
                            pid: usize::MAX,
                            fa: vec![0.0; noq * nu],
                            pp: vec![1.0; noq * (nu + 1)],
                        });
                    }
                    if a_rows {
                        while hb.len() < nrows {
                            hb.push(ParentRatioBounds {
                                pid: usize::MAX,
                                pdw: [0.0; 32],
                                pdg: [0.0; 32],
                                st: vec![usize::MAX; noq],
                                fb: vec![0.0; noq * nu],
                                pb: vec![1.0; noq * (nu + 1)],
                            });
                        }
                    }
                    // per (group g, cards left, borrowing allowed): the per-target suffix factors (a, b) and the per-outfit
                    // ratio products r depend on nothing else, so they are constants of the configuration: tabulated on
                    // first use of their row (the same expressions, so the same f64s as computing them per node); row
                    // (loff[g] + left) * ncan + can, a row of 2 nu factors (a then b) and one of noq ratios
                    let ncan = if one_b { 2 } else { 1 };
                    let mut loff = vec![0usize; ng + 1];
                    for g in 0..ng {
                        loff[g + 1] = loff[g] + gs[g].1 + 1;
                    }
                    let nrow = loff[ng] * ncan;
                    kab.clear();
                    kab.resize(nrow * 2 * nu, 0.0);
                    kr.clear();
                    kr.resize(nrow * nc, 0.0);
                    kab_have.clear();
                    kab_have.resize(nrow, false);
                    kr_have.clear();
                    kr_have.resize(nrow, false);
                    // can a deck from here (group g with `left` still to pick, `pw`/`pg` the partial miss) pass?
                    // (ppw, ppg: the parent's row; dep, pid: the parent's depth and id; tm: the targets the child's card
                    // touches; cm, p: the child's class mask (`cv_mask`s ORed) and its card's position in group g)
                    let mut hopeful = |g: usize,
                                       left: usize,
                                       pw: &[f64],
                                       pg: &[f64],
                                       bestv: f64,
                                       nb: usize,
                                       ppw: &[f64],
                                       ppg: &[f64],
                                       dep: usize,
                                       pid: usize,
                                       tm: u32,
                                       cm: u64,
                                       p: usize|
                     -> bool {
                        // (the cap-free band prunes by the state-conditioned bound alone, `sbx`)
                        if cfm {
                            return true;
                        }
                        // the cap of every deck the child can still reach: its cards' classes and every entry left
                        let rho = if best {
                            rho
                        } else {
                            ccap[cap_class(cm | if left > 0 { csuf[goff[g] + p + 1] } else { csuf[goff[g + 1]] })]
                        };
                        // borrowed cards still allowed (only with the one-borrowed tables)
                        let can = nb < 1;
                        let mut have_a = false;
                        let row =
                            if g >= ng { usize::MAX } else { (loff[g] + left) * ncan + usize::from(one_b && can) };
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
                                for t in 0..nu {
                                    let (a, b) = if !one_b {
                                        (
                                            gm_w(gmin[g], nu, left)[t] * sufw[(g + 1) * nu + t],
                                            gm_g(gmin[g], nu, left)[t] * sufg[(g + 1) * nu + t],
                                        )
                                    } else if can {
                                        let gb = &gbs[g];
                                        (
                                            (gb.one[left].0[t] * so[g + 1].0[t])
                                                .min(gb.own[left].0[t] * s1[g + 1].0[t]),
                                            (gb.one[left].1[t] * so[g + 1].1[t])
                                                .min(gb.own[left].1[t] * s1[g + 1].1[t]),
                                        )
                                    } else {
                                        (gbs[g].own[left].0[t] * so[g + 1].0[t], gbs[g].own[left].1[t] * so[g + 1].1[t])
                                    };
                                    kab[row * 2 * nu + t] = a;
                                    kab[row * 2 * nu + nu + t] = b;
                                }
                            }
                            let (ka, kb) = kab[row * 2 * nu..(row + 1) * 2 * nu].split_at(nu);
                            if a_rows {
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
                                        orow[ix].factors(
                                            oqq[ix],
                                            &bw[..nu],
                                            &bg[..nu],
                                            &mut c.fa[ix * nu..(ix + 1) * nu],
                                        );
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
                                for (ix, (row, pr)) in
                                    c.fa.chunks_exact_mut(nu).zip(c.pp.chunks_exact(nu + 1)).enumerate()
                                {
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
                        let mut nan = false;
                        for &(ci, i, no) in og {
                            let q = oq[i].2;
                            let mut m = 0.0f64;
                            for o in 0..no {
                                let a = if have_a { av[i + o] } else { orow[i + o].eval(q, &dw, &dg) };
                                av[i + o] = a;
                                nan |= a.is_nan();
                                m = m.max(a);
                            }
                            cv[ci] += fixed * m;
                        }
                        if !nan && !pass(cv) {
                            return false;
                        }
                        if row != usize::MAX && !kr_have[row] {
                            kr_have[row] = true;
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
                        let mut bfill = |ix: usize, full: bool, brow: &mut [f64; 32]| -> f64 {
                            let (o, q) = (&orow[ix], oqq[ix]);
                            let ParentRatioBounds { st, pdw: hw, pdg: hg, fb, pb, .. } = &mut hb[dep];
                            let (fr, pr) =
                                (&mut fb[ix * nu..(ix + 1) * nu], &mut pb[ix * (nu + 1)..(ix + 1) * (nu + 1)]);
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
                            // (ZSPLIT; a row without a split, zm 0 and no low split, takes min(A, B r)) A's product over a
                            // split's Z: from the per-parent row with the child's touched targets recomputed (A's own
                            // factors), or at the node's odds
                            let hrow = if have_a { dep * 2 + usize::from(one_b && can) } else { usize::MAX };
                            let a_z = |ix: usize, z: u32| -> f64 {
                                let (o, q) = (&orow[ix], oqq[ix]);
                                if hrow == usize::MAX {
                                    return o.eval_in(q, &dw, &dg, z);
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
                                        let full = bfill(ix, zm[ix] == 0, &mut brow);
                                        if zm[ix] != 0 { a_z(ix, zm[ix]) * (bin(&brow, nm[ix]) * r) } else { full * r }
                                    } else if zm[ix] != 0 {
                                        a_z(ix, zm[ix]) * (orow[ix].eval_in(q, pdw, pdg, nm[ix]) * r)
                                    } else {
                                        orow[ix].eval(q, pdw, pdg) * r
                                    };
                                    for c in alt[ix].0..alt[ix].1 {
                                        let r = if row == usize::MAX { 1.0 } else { kr[row * nc + c] };
                                        let bn = if have_a {
                                            bin(&brow, nm[c])
                                        } else {
                                            orow[ix].eval_in(q, pdw, pdg, nm[c])
                                        };
                                        b = b.min(a_z(ix, zm[c]) * (bn * r));
                                    }
                                    m = m.max(a.min(b));
                                }
                                cv[ci] += fixed * m;
                            }
                        }
                        let ok = pass(cv);
                        if rbc && ok {
                            let mut s = rbs.borrow_mut();
                            if s.len() < dep + 2 {
                                s.resize(dep + 2, vec![]);
                            }
                            s[dep + 1].clear();
                            s[dep + 1].extend_from_slice(cv);
                        }
                        ok
                    };
                    // locked cards as bits; avail[g][p]: locked cards with an entry at position >= p of group g or in a
                    // later group (a branch that can no longer place every locked card is dropped)
                    let all_locked = all_lk;
                    let mut avail = std::mem::take(&mut avail_buf);
                    avail.truncate(ng + 1);
                    avail.resize_with(ng + 1, Vec::new);
                    avail[ng].clear();
                    avail[ng].push(0);
                    for g in (0..ng).rev() {
                        let l = gs[g].0;
                        let next = avail[g + 1][0];
                        let v = &mut avail[g];
                        v.clear();
                        v.resize(l.len() + 1, 0u8);
                        v[l.len()] = next;
                        for p in (0..l.len()).rev() {
                            v[p] = v[p + 1] | lbit[l[p]];
                        }
                    }
                    if all_locked & !avail[0][0] != 0 {
                        avail_buf = avail;
                        st_sel = st.sel;
                        st_pw = st.pw;
                        st_pg = st.pg;
                        continue;
                    }
                    fn rec(
                        sp: &DeckSpace,
                        gs: &[(&[usize], usize)],
                        g: usize,
                        start: usize,
                        left: usize,
                        chm: u128,
                        nb: usize,
                        have: u8,
                        cmk: u64,
                        lk: (&[u8], u8, &[Vec<u8>]),
                        st: &mut DeckPath,
                        ebit: &[u128],
                        mw: &[f64],
                        mg: &[f64],
                        tmk: &[u32],
                        nu: usize,
                        hopeful: &mut dyn FnMut(
                            usize,
                            usize,
                            &[f64],
                            &[f64],
                            f64,
                            usize,
                            &[f64],
                            &[f64],
                            usize,
                            usize,
                            u32,
                            u64,
                            usize,
                        ) -> bool,
                        leaf: &mut dyn FnMut(&[usize], &[f64], &[f64], u64) -> bool,
                        bestv: &dyn Fn() -> f64,
                        sbx: &mut dyn FnMut(&[usize]) -> bool,
                        dom: &mut dyn FnMut(usize, usize, usize, usize, u128, &[f64], &[usize], u128) -> Option<u128>,
                        need: u128,
                    ) -> bool {
                        let (lbit, all_locked, avail) = lk;
                        if left == 0 {
                            if g + 1 >= gs.len() {
                                if need & !chm != 0 {
                                    return false;
                                }
                                let b = st.sel.len() * nu;
                                return leaf(&st.sel, &st.pw[b..b + nu], &st.pg[b..b + nu], cmk);
                            }
                            return rec(
                                sp,
                                gs,
                                g + 1,
                                0,
                                gs[g + 1].1,
                                chm,
                                nb,
                                have,
                                cmk,
                                lk,
                                st,
                                ebit,
                                mw,
                                mg,
                                tmk,
                                nu,
                                hopeful,
                                leaf,
                                bestv,
                                sbx,
                                dom,
                                need,
                            );
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
                            let Some(nd) = dom(g, p, start, left, chm, &st.pg[b..b + nu], &st.sel, need) else {
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
                            if !hopeful(
                                g,
                                left - 1,
                                &st.pw[b + nu..b + 2 * nu],
                                &st.pg[b + nu..b + 2 * nu],
                                bestv(),
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
                            if !sbx(&st.sel) {
                                st.sel.pop();
                                continue;
                            }
                            let stop = rec(
                                sp,
                                gs,
                                g,
                                p + 1,
                                left - 1,
                                chm | ebit[j],
                                nb + usize::from(sp.ebor[j]),
                                have | lbit[j],
                                cj,
                                lk,
                                st,
                                ebit,
                                mw,
                                mg,
                                tmk,
                                nu,
                                hopeful,
                                leaf,
                                bestv,
                                sbx,
                                dom,
                                nd,
                            );
                            st.sel.pop();
                            if stop {
                                return true;
                            }
                        }
                        false
                    }
                    // the state-conditioned bound at depth <= 1 (mode 1) or at every depth, the leaf included (mode 2); with
                    // SB_CHECK, every leaf's value against its root's and depth-1 node's bounds
                    let total_slots: usize = gs.iter().map(|x| x.1).sum();
                    let mut sb_d1 = f64::INFINITY;
                    let mut sbx = |sel: &[usize]| -> bool {
                        if cfm {
                            // the cap-free band: the bound at every node and leaf, nothing else prunes; past the node
                            // limit every child fails and the search ends with CapFreeNodeLimit
                            if cf_over.get() {
                                return false;
                            }
                            if self.nodes.get() > cf_lim {
                                cf_over.set(true);
                                return false;
                            }
                            let d = sel.len();
                            let leafd = d == total_slots;
                            let s = if leafd { 1.0 } else { cfs.max(1.0) };
                            // the cheap bound first, the state-conditioned one only where it passes
                            let ubc = cf_cheap(sel);
                            // (CF_CHECK: both always, so the check below tests each: an exact value above either is above their minimum)
                            let ub = if cfs == 0.0 && ubc * s * (1.0 + 1e-9) < sb_bar {
                                ubc
                            } else {
                                ubc.min(cf_ub(sel, if cfs == 0.0 { sb_bar / s } else { 0.0 }))
                            };
                            SBN.with(|c| c.borrow_mut().1 += 1);
                            if cfs > 0.0 {
                                // CF_CHECK: a leaf's exact value against its own bound and every node's above it
                                let mut path = cf_path.borrow_mut();
                                path.truncate(d);
                                path.push(ub);
                                if leafd {
                                    let ents: Vec<(&Src, Kind)> =
                                        sel.iter().map(|&j| (&self.srcs[j], self.kind[j])).collect();
                                    let x = joint_fast(
                                        model,
                                        sb_u,
                                        &[sb_q],
                                        &ents,
                                        filler,
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
                        if sbm == 0 || (sbm == 3 && !sb_check()) {
                            return true;
                        }
                        let d = sel.len();
                        let chk = sb_check();
                        let leafd = d == total_slots;
                        if (sbm == 1 || sbm == 3) && d > 1 && !(chk && leafd) {
                            return true;
                        }
                        if sbm == 3 && d == 1 && !chk {
                            return true;
                        }
                        let ub = sb_ub(sel);
                        SBN.with(|c| c.borrow_mut().1 += 1);
                        if d == 1 {
                            sb_d1 = ub;
                        }
                        if chk && leafd {
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
                        let pass = ub * (1.0 + 1e-9) >= sb_bar;
                        if !pass {
                            SBN.with(|c| c.borrow_mut().2 += 1);
                        }
                        pass
                    };
                    let bcell = std::cell::Cell::new(bestv);
                    // the deck's odds, written per leaf (no allocation unless the deck is kept)
                    let (mut dwa, mut dga) = ([0.0f64; 32], [0.0f64; 32]);
                    let mut leaf = |sel: &[usize], pw: &[f64], pg: &[f64], cmk: u64| -> bool {
                        let pals = sel.iter().filter(|&&j| self.cards[self.rel[j]].pal).count() + plain;
                        if let Some(p) = self.dj.pals
                            && p != pals
                        {
                            return false;
                        }
                        if !self.locked.iter().all(|l| sel.iter().any(|&j| self.rel[j] == *l)) {
                            return false;
                        }
                        for t in 0..nu {
                            dwa[t] = 1.0 - pw[t];
                            dga[t] = 1.0 - pg[t];
                        }
                        let (dw, dg) = (&dwa[..nu], &dga[..nu]);
                        // RB_CHECK: each clause's value against the bounds of the root and of every node above the deck
                        if rbc {
                            let s = rbs.borrow();
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
                            if v > bcell.get() {
                                bcell.set(v);
                                out.clear();
                                out.push(mk(v));
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
                                out.push(mk(v));
                                if out.len() >= cap {
                                    return true;
                                }
                            }
                            false
                        }
                    };
                    let bget = || bcell.get();
                    let stop = if gs.is_empty() {
                        leaf(&[], &[1.0; 32][..nu], &[1.0; 32][..nu], 0)
                    } else {
                        rec(
                            self,
                            &gs,
                            0,
                            0,
                            gs[0].1,
                            0,
                            0,
                            0,
                            0,
                            (lbit, all_locked, &avail),
                            &mut st,
                            ebit,
                            mw,
                            mg,
                            tmk,
                            nu,
                            &mut hopeful,
                            &mut leaf,
                            &bget,
                            &mut sbx,
                            &mut dom,
                            0,
                        )
                    };
                    pid_next = st.pid;
                    avail_buf = avail;
                    st_sel = st.sel;
                    st_pw = st.pw;
                    st_pg = st.pg;
                    bestv = bcell.get();
                    if stop {
                        return SearchOutcome { decks: out, incomplete: Some(SearchStop::OutputLimit) };
                    }
                    if cf_over.get() {
                        return SearchOutcome { decks: out, incomplete: Some(SearchStop::CapFreeNodeLimit) };
                    }
                }
            }
        }
        SPT.with(|c| {
            let mut v = c.get();
            v.1 += t_dfs.elapsed().as_secs_f64();
            c.set(v);
        });
        SearchOutcome { decks: out, incomplete: None }
    }
    /// The product search's dominance table of group `gi` (entries `l`) under key `code`: per position p, the earlier
    /// positions a whose entry A could take B = l[p]'s place in any deck (borrowed only if B is, B's locked card if B
    /// is one, B's pal status when the job fixes the pals) with every miss chance in [0, 1] and at most B's, white and
    /// gold, for every target; with the targets where A's gold miss is the lower (bit t).
    ///
    /// Why the product search (`best`) may skip a child B of a node (`dom` in `search`): let A be such an entry, free at
    /// the node (no pick of its character) and taking B's place in the pick order: a sibling (position in [start, p),
    /// right after the node's picks of group g) or an earlier one no later pick of g touches B's targets with (their
    /// factors are exactly 1.0 there). Any deck D through B and D' = D - B + A then multiply their miss chances in the
    /// same order but for that one factor, so with rounding monotone every miss product of D' is at most D's: every
    /// white odd at least D's, every gold odd equal to D's (equal gold misses) or above it by more than any rounding
    /// (`DOM_GAP`). The value is monotone in the white odds operation by operation, and a gold odd's factor rises by at
    /// least (GEN_GOLD - GEN_WHITE) times its gap, far above its rounding; so val(D') >= val(D) as computed. D' is legal
    /// when no pick of D after B is of A's character (nor of B's: A shares it or is free) and comes before D in the
    /// enumeration (the same configuration, lexicographically lower). The search returns the first deck of the highest
    /// value in enumeration order (strictly higher replaces it; a branch is pruned only below the best so far), which is
    /// never such a D: the decks through B are skipped when nothing of A's character can follow B, else those without a
    /// pick of A's character (`need`). Only which decks are visited (the rest in the same order) and how the bar rises
    /// change: the deck found, its value and odds do not.
    pub(super) fn dom_table(
        &self,
        set_id: usize,
        code: u64,
        gi: usize,
        l: &[usize],
        mw: &[f64],
        mg: &[f64],
        lbit: &[u8],
    ) -> std::rc::Rc<Vec<Vec<(u32, u32)>>> {
        if let Some(r) = self.domc.borrow().get(&(set_id, code, gi)) {
            return std::rc::Rc::clone(r);
        }
        let nu = self.unc.len();
        let in01 = |x: f64| (0.0..=1.0).contains(&x);
        let ok = |j: usize| (0..nu).all(|t| in01(mw[j * nu + t]) && in01(mg[j * nu + t]));
        let pal = |j: usize| self.cards[self.rel[j]].pal;
        let r = std::rc::Rc::new(
            (0..l.len())
                .map(|p| {
                    let b = l[p];
                    if !ok(b) {
                        return vec![];
                    }
                    (0..p)
                        .filter_map(|a| {
                            let aj = l[a];
                            if self.ebor[aj] && !self.ebor[b] {
                                return None;
                            }
                            if lbit[b] != 0 && lbit[aj] != lbit[b] {
                                return None;
                            }
                            if self.dj.pals.is_some() && pal(aj) != pal(b) {
                                return None;
                            }
                            if !ok(aj)
                                || !(0..nu)
                                    .all(|t| mw[aj * nu + t] <= mw[b * nu + t] && mg[aj * nu + t] <= mg[b * nu + t])
                            {
                                return None;
                            }
                            Some((
                                a as u32,
                                (0..nu).filter(|&t| mg[aj * nu + t] < mg[b * nu + t]).fold(0u32, |m, t| m | 1 << t),
                            ))
                        })
                        .collect()
                })
                .collect::<Vec<_>>(),
        );
        self.domc.borrow_mut().insert((set_id, code, gi), std::rc::Rc::clone(&r));
        r
    }
}
/// The least exact gap (parent's gold-miss product times the picks left's lowest times the misses' difference) for
/// which a lower gold miss is sure to give a gold odd and factor above the other deck's after rounding (both are
/// within about 2^-50 of exact)
const DOM_GAP: f64 = 1.0 / (1u64 << 28) as f64;
thread_local! {
    /// RB_CHECK: (leaves checked, (leaf, clause, ancestor) tests, violations: a value above an ancestor's bound by more
    /// than 1e-9, the highest value / bound)
    pub(crate) static RBCHK: std::cell::RefCell<(usize, usize, usize, f64)> = const { std::cell::RefCell::new((0, 0, 0, 0.0)) };
}
fn rb_check() -> f64 {
    static C: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("RB_CHECK").ok().map_or(0.0, |v| v.parse().unwrap_or(1.0)))
}

/// The low-base split's threshold (ZSPLIT): a target whose empty-deck factor is at most this share of its factor with a
/// certain white hint. Measured on tiny 8-target jobs (instructions): 0.25 and 0.5 the same, 0.75 alone or as a second
/// split (0.5 and 0.75) fewer nodes but more instructions
const LOW_SPLIT: [f64; 1] = [0.5];
