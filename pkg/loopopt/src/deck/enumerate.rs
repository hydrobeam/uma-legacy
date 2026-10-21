//! The deck enumeration (`DeckSpace::search`): node bounds, the exact pass with its caps and certificate.
use super::{
    CF_LIM, CF_ON, CFCHK, CapFreeRatios, DeckSpace, GTab, HashMap, Instant, Kind, LandPre, LegalTabs, Model, SB_ON,
    SB_USED, SBCHK, SBN, SPT, SbCfg, Sel, Src, StateBound, cap_class, cf_check, chain_states, gm_g, gm_w, joint_fast,
    sb_check, sb_mode, src_levels, top_desc,
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
#[derive(Clone, Copy)]
struct TraversalTables<'tab> {
    ebit: &'tab [u128],
    groups_all: &'tab [&'tab Vec<usize>],
    sb_rows: &'tab [Option<Vec<(f64, f64)>>],
    lp: &'tab [LandPre],
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
    rbs: Vec<Vec<f64>>,
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

    /// Traverse the prepared configurations in their visiting order (`ConfigOrder`): each one's root tests, then its
    /// tables and the depth-first walk of its decks (`Traversal::visit`), until one ends the search.
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

/// A deck holds at most `allowed_borrow` (0 or 1) borrowed cards; the tables above take each group's best entries
/// whatever they are, so with borrowing they count several borrowed cards at once (every card the player lacks
/// is one), which leaves the bound loose. Per (key, group): the products of its r best owned entries, and of
/// r - 1 owned with its best borrowed one, for the miss chances (lowest first) and the ratios (highest first).
struct BorrowBounds {
    own: Vec<(Vec<f64>, Vec<f64>)>,
    one: Vec<(Vec<f64>, Vec<f64>)>,
    rown: Vec<Vec<f64>>,
    rone: Vec<Vec<f64>>,
}

/// The one-borrowed-card tables of one configuration's groups and their suffixes (`own`: no borrowed card left to
/// place; `one`: at most one, in one group); empty without borrowing (`one_b` false)
struct BorrowTables {
    one_b: bool,
    gbs: Vec<std::rc::Rc<BorrowBounds>>,
    so: Vec<(Vec<f64>, Vec<f64>)>,
    s1: Vec<(Vec<f64>, Vec<f64>)>,
    po: Vec<Vec<f64>>,
    p1: Vec<Vec<f64>>,
}

/// The node bound's per-parent rows (see `Walk::hopeful`), per (depth, borrowing variant); parent ids never repeat
/// within a search
/// (`fa`: A's factors, row ix * nu; `pp`: their products from 1.0 over the first t, row ix * (nu + 1))
struct ParentTargetBounds {
    pid: usize,
    fa: Vec<f64>,
    pp: Vec<f64>,
}

/// The same for B, per depth (B has no borrowing variant): the parent's odds 1 - ppw, 1 - ppg, and per row, built
/// at the first child that evaluates its B (`st`: the parent id it was built for), its factors and their products
struct ParentRatioBounds {
    pid: usize,
    pdw: [f64; 32],
    pdg: [f64; 32],
    st: Vec<usize>,
    fb: Vec<f64>,
    pb: Vec<f64>,
}

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
struct DeckPath {
    sel: Vec<usize>,
    pw: Vec<f64>,
    pg: Vec<f64>,
    pid: usize,
}

/// The configuration to visit next: computed from a pending one (lazy) or the eager list's
enum ConfigRef<'queue> {
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
struct ConfigOrder {
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
    fn new(queue: ConfigurationQueue, controls: SearchControl) -> Self {
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
    fn next(
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

/// What every configuration of one traversal reads: the search, its controls and tables, the clause value (`val`)
struct TraversalInputs<'run, 'space, V> {
    sp: &'run DeckSpace<'space>,
    model: &'run Model<'run>,
    clauses: &'run [(Vec<(usize, f64)>, f64)],
    ctl: SearchControl,
    tb: TraversalTables<'run>,
    val: V,
    noq: usize,
    // the per-parent rows need every outfit row to read the nu targets
    a_rows: bool,
    oqq: Vec<f64>,
}

/// The node bound's buffers and per-parent rows (`Walk::hopeful`), and the dominance skip's, reused across
/// configurations
struct NodeScratch {
    dw: Vec<f64>,
    dg: Vec<f64>,
    av: Vec<f64>,
    // (the expanded configuration's clause sums and B's odds)
    cvx: Vec<f64>,
    pdwx: Vec<f64>,
    pdgx: Vec<f64>,
    // the node bound's per-configuration constants, flat and reused (see `hopeful`)
    kab: Vec<f64>,
    kr: Vec<f64>,
    kab_have: Vec<bool>,
    kr_have: Vec<bool>,
    hc: Vec<ParentTargetBounds>,
    hb: Vec<ParentRatioBounds>,
    // (a child's touched targets: their odds and the parent's factors they replace)
    tw: [f64; 32],
    tg: [f64; 32],
    told: [f64; 32],
    // (and for B: their odds 1 - pw, 1 - pg, and one row's factors with the touched ones in place)
    bdw: [f64; 32],
    bdg: [f64; 32],
    brow: [f64; 32],
    // (a parent's odds for its A factors, the first nu slots written before each use)
    bw: [f64; 32],
    bg: [f64; 32],
    // (`Walk::dom`'s per-target ppg * rlo: written before read in each call)
    dpr: [f64; 32],
    // (RB_CHECK: the node bounds on the current path, per depth its clause sums, the root's at depth 0)
    rbs: Vec<Vec<f64>>,
}

impl NodeScratch {
    /// Grow the per-parent rows to `nhc` (A) and, with `a_rows`, `nrows` (B)
    fn grow_parent_rows(&mut self, nhc: usize, nrows: usize, noq: usize, nu: usize, a_rows: bool) {
        while self.hc.len() < nhc {
            self.hc.push(ParentTargetBounds {
                pid: usize::MAX,
                fa: vec![0.0; noq * nu],
                pp: vec![1.0; noq * (nu + 1)],
            });
        }
        if a_rows {
            while self.hb.len() < nrows {
                self.hb.push(ParentRatioBounds {
                    pid: usize::MAX,
                    pdw: [0.0; 32],
                    pdg: [0.0; 32],
                    st: vec![usize::MAX; noq],
                    fb: vec![0.0; noq * nu],
                    pb: vec![1.0; noq * (nu + 1)],
                });
            }
        }
    }

    /// Clear the node bound's per-configuration constants for `nrow` rows (see `Walk::hopeful`)
    fn reset_constants(&mut self, nrow: usize, nu: usize, nc: usize) {
        self.kab.clear();
        self.kab.resize(nrow * 2 * nu, 0.0);
        self.kr.clear();
        self.kr.resize(nrow * nc, 0.0);
        self.kab_have.clear();
        self.kab_have.resize(nrow, false);
        self.kr_have.clear();
        self.kr_have.resize(nrow, false);
    }
}

/// The expanded configuration's tables, flat and reused across configurations
struct ConfigScratch {
    // the suffix tables: per target (g * nu + t) the products of the groups' lowest misses from group g on (white,
    // gold), per (clause, chance, outfit) (g * noq + i) those of the best ratios
    sufw: Vec<f64>,
    sufg: Vec<f64>,
    spf: Vec<f64>,
    // (the class masks: csuf[goff[g] + p] = the `cv_mask`s ORed of group g's entries from position p on and of every
    // later group's)
    csuf: Vec<u64>,
    goff: Vec<usize>,
    // locked cards as bits; avail[g][p]: locked cards with an entry at position >= p of group g or in a later group
    avail: Vec<Vec<u8>>,
    path: DeckPath,
}

impl ConfigScratch {
    /// The suffix tables of `cf` (its groups' key tables `gmin`, `rp`): the same products in the same order as the
    /// per-configuration tables they replace
    fn fill_suffixes(&mut self, cf: &SearchConfig, gmin: &[&[f64]], rp: &[&[f64]], nu: usize, nc: usize, size: usize) {
        let Self { sufw, sufg, spf, .. } = self;
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

    /// The class masks of groups `gs` (entries' `cv_mask`s `cvm`)
    fn fill_class_suffixes(&mut self, gs: &[(&[usize], usize)], cvm: &[u64]) {
        let Self { csuf, goff, .. } = self;
        let ng = gs.len();
        goff.clear();
        goff.push(0);
        for g in 0..ng {
            goff.push(goff[g] + gs[g].0.len());
        }
        csuf.clear();
        csuf.resize(goff[ng] + 1, 0);
        for g in (0..ng).rev() {
            for (p, &j) in gs[g].0.iter().enumerate().rev() {
                csuf[goff[g] + p] = csuf[goff[g] + p + 1] | cvm[j];
            }
        }
    }

    /// The locked-card table of groups `gs`; false when they cannot place every locked card (`all_locked`)
    fn fill_avail(&mut self, gs: &[(&[usize], usize)], lbit: &[u8], all_locked: u8) -> bool {
        let avail = &mut self.avail;
        let ng = gs.len();
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
        all_locked & !avail[0][0] == 0
    }
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

/// The depth-first walk of one configuration's decks: its tables (borrowed from `Traversal::visit`), the node bound's
/// scratch, the state-conditioned bounds, the decks kept and the bar
struct Walk<'walk, 'run, 'space, V> {
    inp: &'walk TraversalInputs<'run, 'space, V>,
    ns: &'walk mut NodeScratch,
    out: &'walk mut Vec<(f64, Sel)>,
    bestv: f64,
    cf: &'walk SearchConfig,
    filler: usize,
    plain: usize,
    gs: &'walk [(&'run [usize], usize)],
    gmin: &'walk [&'walk [f64]],
    rp: &'walk [&'walk [f64]],
    mw: &'walk [f64],
    mg: &'walk [f64],
    tmk: &'walk [u32],
    sufw: &'walk [f64],
    sufg: &'walk [f64],
    spf: &'walk [f64],
    doms: &'walk [std::rc::Rc<Vec<Vec<(u32, u32)>>>],
    doff: &'walk [usize],
    dsuf: &'walk [u128],
    bt: &'walk BorrowTables,
    csuf: &'walk [u64],
    goff: &'walk [usize],
    avail: &'walk [Vec<u8>],
    loff: &'walk [usize],
    ncan: usize,
    ng: usize,
    total_slots: usize,
    sbb: StateBound<'walk>,
    sb_root: f64,
    // (SB_CHECK: the depth-1 node's bound)
    sb_d1: f64,
    cf_path: Vec<f64>,
    // (the cap-free band past its node limit)
    cf_over: bool,
    // the deck's odds, written per leaf (no allocation unless the deck is kept)
    dwa: [f64; 32],
    dga: [f64; 32],
}

impl<V: Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64> Walk<'_, '_, '_, V> {
    /// The decks below a node: group g from position `start` with `left` slots of it to fill, characters `chm`,
    /// borrowed cards `nb`, locked cards `have`, class mask `cmk`, characters `need` its decks must take; the path
    /// `st` holds the picks so far. True: the search stops (the output is full)
    fn rec(
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
    fn leaf(&mut self, sel: &[usize], pw: &[f64], pg: &[f64], cmk: u64) -> bool {
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
