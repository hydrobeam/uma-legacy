//! The state-conditioned bounds of one configuration (`StateBound`): the band pass's (`plain`) and the cap-free band's,
//! its cheap bound (`cheap`) tried before the per-state one (`full`); and the search-wide ratio tables the cap-free
//! bounds read (`CapFreeRatios`).
use super::{KeyLevel, Kind, LandPre, Model, SbCfg, event_pmf_job, key_level};
use std::{cell::RefCell, rc::Rc};

/// The cap-free band's ratio tables (the ratio bound of `StateBound::full` and `cheap`), one search's: per outfit of the
/// trainee its zero-base targets (`z`: factor 0 with no hint, whose ratios would be infinite, so they take the
/// per-target bound), and per group of `groups` (`group`), level and outfit (row l * no + o) the best ratio over its
/// entries: R(e, l) = prod over the other targets of the factor at e's misses at level l over the factor with no card;
/// per entry (`entry`) the same ratio of that entry alone. Both built on first use.
pub(super) struct CapFreeRatios<'tab> {
    nu: usize,
    no: usize,
    nk: usize,
    lp: &'tab [LandPre],
    sb_rows: &'tab [Option<Vec<(f64, f64)>>],
    groups: &'tab [&'tab Vec<usize>],
    kinds: &'tab [Kind],
    pub(super) z: Vec<u32>,
    rmx: RefCell<Vec<Option<Rc<Vec<f64>>>>>,
    rent: RefCell<crate::FxMap<usize, Rc<Vec<f64>>>>,
}

impl<'tab> CapFreeRatios<'tab> {
    /// `no`: the trainee's outfit count in the cap-free band (`cfm`), else 0 (no tables)
    pub(super) fn new(
        cfm: bool,
        nu: usize,
        no: usize,
        nk: usize,
        lp: &'tab [LandPre],
        sb_rows: &'tab [Option<Vec<(f64, f64)>>],
        groups: &'tab [&'tab Vec<usize>],
        kinds: &'tab [Kind],
    ) -> Self {
        let z: Vec<u32> = (0..no)
            .map(|o| (0..nu).filter(|&k| !(lp[o * nu + k].f(1.0, 1.0) > 0.0)).fold(0u32, |m, k| m | 1 << k))
            .collect();
        let rmx = RefCell::new(vec![None; if cfm { groups.len() } else { 0 }]);
        CapFreeRatios { nu, no, nk, lp, sb_rows, groups, kinds, z, rmx, rent: RefCell::default() }
    }

    /// (outfit o's product over the targets outside its Z of the factors with no card: R's denominator)
    fn den(&self, o: usize) -> f64 {
        let (nu, lp, z) = (self.nu, self.lp, &self.z);
        (0..nu).filter(|&k| z[o] >> k & 1 == 0).map(|k| lp[o * nu + k].f(1.0, 1.0)).product()
    }

    /// (outfit o's product over the targets outside its Z of the factors at an entry's level-l misses `row`, over `den`:
    /// R(e, l))
    fn ratio(&self, o: usize, row: &[(f64, f64)], den: f64) -> f64 {
        let (nu, lp, z) = (self.nu, self.lp, &self.z);
        (0..nu).filter(|&k| z[o] >> k & 1 == 0).map(|k| lp[o * nu + k].f(row[k].0, row[k].1)).product::<f64>() / den
    }

    /// Group gi's best ratio per level (up to its kind's stop) and outfit, row l * no + o
    pub(super) fn group(&self, gi: usize) -> Rc<Vec<f64>> {
        if let Some(r) = &self.rmx.borrow()[gi] {
            return Rc::clone(r);
        }
        let (nu, no) = (self.nu, self.no);
        let stop = if gi < self.nk { self.kinds[gi].1 as usize } else { 0 };
        let mut m = vec![0.0f64; (stop + 1) * no];
        for o in 0..no {
            let den = self.den(o);
            for &j in self.groups[gi] {
                if let Some(rows) = &self.sb_rows[j] {
                    for l in 0..=stop.min(rows.len() / nu - 1) {
                        let r = self.ratio(o, &rows[l * nu..(l + 1) * nu], den);
                        if !(r <= m[l * no + o]) {
                            m[l * no + o] = r;
                        }
                    }
                }
            }
        }
        let r = Rc::new(m);
        self.rmx.borrow_mut()[gi] = Some(Rc::clone(&r));
        r
    }

    /// Entry j's ratio per level and outfit, row l * no + o
    pub(super) fn entry(&self, j: usize) -> Rc<Vec<f64>> {
        if let Some(r) = self.rent.borrow().get(&j) {
            return Rc::clone(r);
        }
        let (nu, no) = (self.nu, self.no);
        let rows = self.sb_rows[j].as_ref().unwrap();
        let nl = rows.len() / nu;
        let mut m = vec![0.0f64; nl * no];
        for o in 0..no {
            let den = self.den(o);
            for l in 0..nl {
                m[l * no + o] = self.ratio(o, &rows[l * nu..(l + 1) * nu], den);
            }
        }
        let r = Rc::new(m);
        self.rent.borrow_mut().insert(j, Rc::clone(&r));
        r
    }
}

/// (`StateBound::full`'s cache, per number of filled slots and visiting order: each state's classes in the order the
/// states are visited, extended as far as a call has reached: `fid` the class of its levels at the filled chain
/// positions (`fk`), `rep` the first state visited whose levels agree at the filled positions and, per group, up to
/// their order at its open positions (`ck`))
struct StateClasses {
    fid: Vec<u32>,
    rep: Vec<u32>,
    fk: crate::FxMap<u64, u32>,
    ck: crate::FxMap<u64, u32>,
}

/// (Kept between calls of `full`, this configuration's: `maps` (`StateClasses`); per call (`stamp`): each filled class's
/// misses and ratio-bound factor (`fx`, `fst` its call), each visited state's terms.)
struct StateBoundCache {
    maps: Vec<Option<StateClasses>>,
    fst: Vec<u32>,
    fx: Vec<f64>,
    term: Vec<f64>,
    stamp: u32,
}

/// (Kept between calls of `cheap`, this configuration's, the same values as computing each from scratch: the products are
/// formed in the same order on the same f64s. `sel`: the deck whose slots 0..sel.len() the prefix rows hold, `z` per
/// depth i and level cap the products of slots 0..i's rows (zg, zh), `p` those of their level-free slots' level-0 rows
/// (pg, ph: slots fill in order, so the filled ones come first), `tab` each filled chain slot's ratio table; per outfit
/// and chain slot its last ratios (`rs`, `rsl` levels, 0 none yet), their expectations per event count (`e`) and the
/// product of the expectations of the chain slots up to it (`pp`): a slot whose ratios are unchanged keeps its
/// expectations, and the products are redone from the first slot that changed.)
struct EventBoundCache {
    sel: Vec<usize>,
    tab: Vec<Option<Rc<Vec<f64>>>>,
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

/// One configuration's state-conditioned bounds at a partial deck `sel` (entries in pick order, filling the groups in
/// turn), over its tables `sbc` (`SbCfg`; None when the search uses none, and then no bound is asked for)
pub(super) struct StateBound<'cfg> {
    sbc: Option<Rc<SbCfg>>,
    gs: &'cfg [(&'cfg [usize], usize)],
    model: &'cfg Model<'cfg>,
    sb_u: usize,
    sb_rows: &'cfg [Option<Vec<(f64, f64)>>],
    lp: &'cfg [LandPre],
    cf_z: &'cfg [u32],
    ratios: &'cfg CapFreeRatios<'cfg>,
    nu: usize,
    cf_no: usize,
    fixed: f64,
    cfm: bool,
    // (each slot's group and chain position in pick order, the chain slots, each group's best ratios)
    cc_slot: Vec<(usize, usize)>,
    cc_grm: Vec<Rc<Vec<f64>>>,
    cc_chain: Vec<usize>,
    cc_cmax: usize,
    ub_cache: StateBoundCache,
    cf_kl: Option<Rc<KeyLevel>>,
    cf_pmf: Rc<Vec<f64>>,
    cf_ns: Vec<usize>,
    cf_rest: f64,
    cc_zall: u32,
    cc_cache: EventBoundCache,
}

impl<'cfg> StateBound<'cfg> {
    /// `gs`: the configuration's groups (entries, slots), `cgs` the same as (group index, slots)
    pub(super) fn new(
        sbc: Option<Rc<SbCfg>>,
        gs: &'cfg [(&'cfg [usize], usize)],
        cgs: &[(usize, usize)],
        model: &'cfg Model<'cfg>,
        sb_u: usize,
        sb_rows: &'cfg [Option<Vec<(f64, f64)>>],
        lp: &'cfg [LandPre],
        ratios: &'cfg CapFreeRatios<'cfg>,
        nu: usize,
        cf_no: usize,
        fixed: f64,
        cfm: bool,
    ) -> Self {
        let ng = gs.len();
        let cf_z = &*ratios.z;
        let mut cc_slot: Vec<(usize, usize)> = vec![];
        let mut cc_grm: Vec<Rc<Vec<f64>>> = vec![];
        if cfm && let Some(c) = &sbc {
            for g in 0..ng {
                for t in 0..gs[g].1 {
                    cc_slot.push((g, c.slots[g][t]));
                }
            }
            cc_grm = (0..ng).map(|g| ratios.group(cgs[g].0)).collect();
        }
        let cc_chain: Vec<usize> = (0..cc_slot.len()).filter(|&i| cc_slot[i].1 != usize::MAX).collect();
        let cc_cmax = sbc.as_ref().map_or(0, |c| c.kl.iter().map(|k| k.1 as usize).max().unwrap_or(0));
        let ub_cache = StateBoundCache { maps: vec![], fst: vec![], fx: vec![], term: vec![], stamp: 0 };
        let cf_kl: Option<Rc<KeyLevel>> =
            if cfm { sbc.as_ref().and_then(|c| (!c.kl.is_empty()).then(|| key_level(&c.kl))) } else { None };
        let cf_pmf = if cfm { event_pmf_job() } else { Rc::new(vec![]) };
        // (the event counts summed term by term, those of P(n) >= 1e-12; the rest's mass, each such term at most A
        // at every slot's most links done)
        let cf_ns: Vec<usize> = (0..cf_pmf.len()).filter(|&n| cf_pmf[n] >= 1e-12).collect();
        let cf_rest: f64 =
            (0..cf_pmf.len()).filter(|&n| cf_pmf[n] < 1e-12).map(|n| cf_pmf[n]).sum::<f64>() * (1.0 + 1e-9) + 1e-300;
        let cc_zall: u32 = cf_z.iter().fold(0u32, |m, &z| m | z);
        // (at most 16 chain slots, as outfits)
        assert!(cc_chain.len() <= 16, "{} chain slots: at most 16", cc_chain.len());
        let cc_cache = {
            let (nsl, cz, nch, nn, on) = (cc_slot.len(), cc_cmax + 1, cc_chain.len(), cf_ns.len(), usize::from(cfm));
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
        };
        StateBound {
            sbc,
            gs,
            model,
            sb_u,
            sb_rows,
            lp,
            cf_z,
            ratios,
            nu,
            cf_no,
            fixed,
            cfm,
            cc_slot,
            cc_grm,
            cc_chain,
            cc_cmax,
            ub_cache,
            cf_kl,
            cf_pmf,
            cf_ns,
            cf_rest,
            cc_zall,
            cc_cache,
        }
    }

    /// The configuration's root bound: no deck of it can reach the bar `sb_bar` below it (0 without a bound)
    pub(super) fn root(&mut self, cfs: f64, sb_bar: f64, sbm: u8) -> f64 {
        if self.cfm {
            let x = self.cheap(&[]);
            if cfs == 0.0 && x * (1.0 + 1e-9) < sb_bar {
                x
            } else {
                x.min(self.full(&[], if cfs == 0.0 { sb_bar } else { 0.0 }))
            }
        } else if sbm > 0 {
            self.bound(&[])
        } else {
            0.0
        }
    }

    /// The state-conditioned bound at `sel`: the cap-free band's per-state one (`full`, the full sum), else `plain`
    pub(super) fn bound(&mut self, sel: &[usize]) -> f64 {
        if self.cfm { self.full(sel, 0.0) } else { self.plain(sel) }
    }

    /// The band pass's state-conditioned bound: per chain state, each target at its partial deck's misses and its open
    /// slots' groups' lowest, the state's weight times the product of the factors, summed; the best outfit
    pub(super) fn plain(&self, sel: &[usize]) -> f64 {
        let StateBound { sbc, gs, model, sb_u, sb_rows, lp, nu, fixed, .. } = self;
        let (nu, fixed, ng) = (*nu, *fixed, gs.len());
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
        let opts = &model.opts[*sb_u];
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
    }

    /// The cap-free band's bound (`cfm`): per chain state x the lower of `plain`'s per-target bound A_x and a ratio
    /// bound B_x, summed over states, the best outfit. Derived: given x the deck's state value is a product over
    /// targets of phi_k at its misses given x (phi_k = G - (G - W) ng - W a nh, a = (1 - e)(1 - q)); its log gains
    /// less from a card the lower the misses already are (phi(ng y, nh y') / phi(ng, nh) rises with ng and with
    /// nh for y' <= y <= 1), so over the targets N with phi_k(1, 1) > 0 the completion's product is at most the
    /// partial deck's times each open slot's card's ratio from no card, at most the best ratio of its group at the
    /// slot's level (`CapFreeRatios::group`); the zero-base targets Z take A's factors. Every completion's state value
    /// is at most both, so their minimum bounds it state by state, and the sum over states (`state_value`'s argument,
    /// `Src::cf_ok`) bounds its exact value.
    /// `cut` (> 0: the value a node must reach to pass): the states are taken heaviest first and the sum stops once
    /// it decides the test, returning its partial sum plus the weight left times every state's most (A with every
    /// slot at its most links done, at least any state's term): an upper bound either way, below `cut` exactly
    /// when the full sum's test would fail to the slack. 0: the full sum, in state order.
    pub(super) fn full(&mut self, sel: &[usize], cut: f64) -> f64 {
        let StateBound { sbc, gs, model, sb_u, sb_rows, lp, cf_z, nu, cf_no, fixed, cc_grm, cc_cmax, ub_cache, .. } =
            self;
        let (nu, cf_no, fixed, cc_cmax, ng) = (*nu, *cf_no, *fixed, *cc_cmax, gs.len());
        let (sb_rows, lp, cf_z) = (*sb_rows, *lp, *cf_z);
        let c = sbc.as_ref().unwrap();
        // filled slots without a level (pg, ph), open ones (ag, ah: their groups' lowest misses; rn: their best
        // ratios); filled chain slots (position, rows), open ones (position, lowest rows, ratio table, group);
        // every slot at its most links done (ug, uh)
        let (mut pg, mut ph, mut ag, mut ah) = ([1.0f64; 32], [1.0f64; 32], [1.0f64; 32], [1.0f64; 32]);
        let (mut ug, mut uh) = ([1.0f64; 32], [1.0f64; 32]);
        let mut rn = [1.0f64; 16];
        let (mut fch, mut nf): ([(usize, &[(f64, f64)]); 16], usize) = ([(0, &[]); 16], 0);
        let (mut ochv, mut no_): ([(usize, &[(f64, f64)], &[f64], usize); 16], usize) = ([(0, &[], &[], 0); 16], 0);
        let mut open = false;
        let mut idx = 0usize;
        for g in 0..ng {
            for t in 0..gs[g].1 {
                let filled = idx < sel.len();
                let rows: &[(f64, f64)] = if filled { sb_rows[sel[idx]].as_deref().unwrap() } else { &c.gmin[g] };
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
        let opts = &model.opts[*sb_u];
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
            (&*x.0, &*x.1)
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
        let ucache = ub_cache;
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
                    fk: crate::FxMap::default(),
                    ck: crate::FxMap::default(),
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
    }

    /// The cap-free band's cheap bound, tried before `full` (a node either fails is pruned): given N = n events
    /// only. Derived: per state the completion's value is at most prod_Z phi_k at its lowest misses (every slot
    /// at its most links done; the zero-base targets) times prod_N phi_k at the filled level-free slots' misses
    /// times, per chain slot, its card's ratio at its level over that base (a filled slot's own, an open slot's
    /// group's best from no card, `CapFreeRatios`) and each open level-free slot's best ratio: the log-submodularity
    /// of `full`, card by card. Each chain slot's ratio rises with its level (its misses only fall), so given
    /// N = n, where the configuration's progress vector is negatively upper-orthant dependent (`key_level`,
    /// checked exactly, the projection on these positions included), the expected product is at most the product
    /// of the slots' expected ratios at P(level | n); summed over P(n). Infinite (no prune) when the key fails
    /// the orthant check.
    pub(super) fn cheap(&mut self, sel: &[usize]) -> f64 {
        let StateBound {
            sbc,
            model,
            sb_u,
            sb_rows,
            lp,
            cf_z,
            ratios,
            nu,
            cf_no,
            fixed,
            cc_slot,
            cc_grm,
            cc_chain,
            cc_cmax,
            cf_kl,
            cf_pmf,
            cf_ns,
            cf_rest,
            cc_zall,
            cc_cache,
            ..
        } = self;
        let (nu, cf_no, fixed, cf_rest, cc_zall) = (*nu, *cf_no, *fixed, *cf_rest, *cc_zall);
        let (sb_rows, lp, cf_z) = (*sb_rows, *lp, *cf_z);
        let c = sbc.as_ref().unwrap();
        if cf_kl.as_ref().is_some_and(|k| !k.nuod) {
            return f64::INFINITY;
        }
        let EventBoundCache { sel: csel, tab, z, p, zw, ac, rs: crs, rsl, rsn, e: ce, pp } = cc_cache;
        let (nsl, cmax, nch, nn) = (cc_slot.len(), *cc_cmax, cc_chain.len(), cf_ns.len());
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
                tab[i] = Some(ratios.entry(sel[i]));
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
        let opts = &model.opts[*sb_u];
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
                            rsn[s0 * cz + l] *=
                                lp[o * nu + k].f(zrest[k].0 * rows[l * nu + k].0, zrest[k].1 * rows[l * nu + k].1);
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
                        let same = rsl[ci] == nl && (0..nl).all(|l| crs[cs + l].to_bits() == rsn[s * cz + l].to_bits());
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
    }
}
