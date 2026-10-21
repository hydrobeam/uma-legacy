//! Joint landing odds of a deck: exact over chain progress and outcome groups, the negative-association bound, and the
//! state-conditioned bound.
use super::*;

/// Exact chances that all targets land, and all but one, for trainee `u` with one deck, at each inheritance chance in
/// `qs`: (P(all), P(all but one), outfit option index in `model.opts[u]`) per q, with the outfit that gives the
/// higher P(all).
/// `ents`: the deck's entries (sources, chain kind); `filler`: cards without target hints, which still take chain
/// slots (as 2-link SRs). The per-target odds (`src_miss`) are the marginals of the same model; the metric the
/// search ranks by multiplies them as if targets were independent. They are not: chain cards share one run's
/// chain events (a run with many completes several chains, a short one starves them all), and one event's
/// outcomes can give one target or another. Here: the distribution of chain progress (`chain_states`); given it,
/// sources are independent, except outcome groups that touch two or more targets, whose outcomes are enumerated.
/// A card's training hints across several targets are one outcome group (`Sources::train_group`), so their
/// dependence is included in the same enumeration.
pub(super) fn joint(
    model: &Model,
    u: usize,
    qs: &[f64],
    ents: &[(Src, Kind)],
    filler: usize,
    n_targets: usize,
    m1: bool,
) -> Vec<(f64, f64, usize)> {
    joint_b(model, u, qs, ents, filler, n_targets, m1)
}
/// `joint` with the sources owned or borrowed (it only reads them)
pub(super) fn joint_b<S: std::borrow::Borrow<Src>>(
    model: &Model,
    u: usize,
    qs: &[f64],
    ents: &[(S, Kind)],
    filler: usize,
    n_targets: usize,
    m1: bool,
) -> Vec<(f64, f64, usize)> {
    JCOUNT.with(|c| c.set(c.get() + 1));
    let r = joint_fast(model, u, qs, ents, filler, n_targets, m1, None);
    if sb_check() {
        let er: Vec<(&Src, Kind)> = ents.iter().map(|e| (e.0.borrow(), e.1)).collect();
        if let Some(sv) = state_value(model, u, qs, &er, filler) {
            SBCHK.with(|c| {
                let mut c = c.borrow_mut();
                for (v, x) in sv.iter().zip(&r) {
                    if x.0 > 0.0 {
                        let rt = v / x.0;
                        c.0 += 1;
                        c.1 = c.1.max(rt);
                        c.2 = c.2.min(rt);
                        if rt < 1.0 - 1e-9 {
                            c.3 += 1;
                        }
                    }
                }
            });
        }
    }
    // CF_CHECK: the cap-free bound's value at a whole deck (its leaf test) against the exact value
    if cf_check() > 0.0 {
        let er: Vec<(&Src, Kind)> = ents.iter().map(|e| (e.0.borrow(), e.1)).collect();
        if let Some(sv) = state_value_cf(model, u, qs, &er, filler) {
            CFCHK.with(|c| {
                let mut c = c.borrow_mut();
                for (v, x) in sv.iter().zip(&r) {
                    c.6 += 1;
                    if x.0 > v * (1.0 + 1e-9) {
                        c.7 += 1;
                    }
                    if *v > 0.0 {
                        c.8 = c.8.max(x.0 / v);
                    }
                }
            });
        }
    }
    static CHECK: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *CHECK.get_or_init(|| std::env::var("JOINT_CHECK").is_ok()) {
        let own: Vec<(Src, Kind)> = ents.iter().map(|e| (e.0.borrow().clone(), e.1)).collect();
        let x = joint_trainee_ref(model, u, qs, &own, filler, n_targets);
        for (a, b) in r.iter().zip(&x) {
            let rel = |p: f64, q: f64| (p - q).abs() / q.abs().max(1e-300);
            assert!(rel(a.0, b.0) < 1e-11 && (!m1 || rel(a.1, b.1) < 1e-11), "joint mismatch: {:?} vs {:?}", a, b);
        }
    }
    r
}

pub(crate) struct JointExt {
    pub su: Vec<f64>,
    pub sf: Vec<f64>,
    pub marg: Vec<f64>,
    pub sets: Vec<f64>,
    pub per_outfit: Vec<f64>,
}
/// `joint_trainee` without the reference check; `m1` = also compute P(all but one) (else 0).
/// Faster form of the same sum: chain states are collapsed to the progress that changes some target's factors (a
/// filler's progress, and links that add nothing, drop out), and outcome groups touching two or more targets are
/// enumerated per cluster of targets they connect (clusters are independent given the chain state).
/// `ext` (the career planner): each target's landing factor times its spark-generation scale (`su` per uncertain
/// target, `sf` per guaranteed gold, in job order), and per (chance, outfit, uncertain target) the exact chance it
/// lands, summed into `marg`; with `sets` non-empty on entry, per (chance, outfit) the chance of each set of uncertain
/// targets landing (bit k = uncertain target k), 2^nu entries each. None leaves every number as it is.
pub(super) fn joint_fast<S: std::borrow::Borrow<Src>>(
    model: &Model,
    u: usize,
    qs: &[f64],
    ents: &[(S, Kind)],
    filler: usize,
    n_targets: usize,
    m1: bool,
    ext: Option<&mut JointExt>,
) -> Vec<(f64, f64, usize)> {
    joint_eval(model, u, qs, &joint_prep(model.uncertain.len(), ents, filler), n_targets, m1, true, ext)
}
/// The deck-dependent part of `joint_fast`, before inheritance chances or outfit state are applied.
/// Collapsed states retain first-seen order; each weight is summed in the original chain-state order.
pub(super) struct JointPrep<'g> {
    /// Miss factors and coupled groups from sources outside the chains.
    base: ChainFactors,
    chains: Vec<ChainProgress>,
    /// One class index per relevant chain position, and that collapsed state's probability.
    states: Vec<(Vec<u8>, f64)>,
    groups: Vec<CoupledGroup<'g>>,
}
/// Cumulative factors after a chain prefix, or from all sources outside the chains.
struct ChainFactors {
    no_gold: Vec<f64>,
    no_hint: Vec<f64>,
    coupled: Vec<usize>,
}
/// A chain position whose progress can change target factors. Progress before the next source-bearing link shares a class.
struct ChainProgress {
    position: usize,
    levels: Vec<ChainFactors>,
    class_of_progress: Vec<u8>,
}
/// A coupled group's ascending target indices and outcomes. The implicit no-reward outcome is last if above 1e-15.
struct CoupledGroup<'g> {
    targets: Vec<usize>,
    outcomes: Vec<(f64, &'g [(usize, u8)])>,
}
pub(super) fn joint_prep<S: std::borrow::Borrow<Src>>(nu: usize, ents: &[(S, Kind)], filler: usize) -> JointPrep<'_> {
    fn add_group<'g>(
        group: &'g Group,
        groups: &mut Vec<CoupledGroup<'g>>,
        coupled: &mut Vec<usize>,
        no_gold: &mut [f64],
        no_hint: &mut [f64],
    ) {
        let marginals = group_gh(group);
        if marginals.len() >= 2 {
            let mut outcomes: Vec<(f64, &[(usize, u8)])> =
                group.iter().map(|(p, rewards)| (*p, &rewards[..])).collect();
            let rest = 1.0 - group.iter().map(|outcome| outcome.0).sum::<f64>();
            if rest > 1e-15 {
                outcomes.push((rest, &[][..]));
            }
            groups.push(CoupledGroup { targets: marginals.iter().map(|entry| entry.0).collect(), outcomes });
            coupled.push(groups.len() - 1);
            return;
        }
        for (k, gold, any_hint) in marginals {
            no_gold[k] *= 1.0 - gold;
            no_hint[k] *= 1.0 - any_hint;
        }
    }
    let (mut base_no_gold, mut base_no_hint) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut base_coupled: Vec<usize> = vec![];
    let mut groups: Vec<CoupledGroup> = vec![];
    for (src, _) in ents {
        let src: &Src = src.borrow();
        for &(k, p) in &src.train {
            base_no_hint[k] *= 1.0 - p;
        }
        for group in &src.fixed {
            add_group(group, &mut groups, &mut base_coupled, &mut base_no_gold, &mut base_no_hint);
        }
    }
    let mut chain_entries: Vec<(Kind, Option<usize>)> = ents
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.1 != (0, 0, 0))
        .map(|(i, entry)| (entry.1, Some(i)))
        .collect();
    for _ in 0..filler {
        chain_entries.push(((2, 2, 0), None));
    }
    chain_entries.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let kinds: Vec<Kind> = chain_entries.iter().map(|entry| entry.0).collect();
    let states = if kinds.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kinds) };
    let mut chains: Vec<ChainProgress> = vec![];
    for (position, &(kind, entry)) in chain_entries.iter().enumerate() {
        let Some(i) = entry else { continue };
        let src: &Src = ents[i].0.borrow();
        if src.chains.is_empty() {
            continue;
        }
        let (mut no_gold, mut no_hint, mut coupled) = (vec![1.0f64; nu], vec![1.0f64; nu], vec![]);
        let mut levels =
            vec![ChainFactors { no_gold: no_gold.clone(), no_hint: no_hint.clone(), coupled: coupled.clone() }];
        let mut class_of_progress = vec![0u8];
        for link in 1..=kind.1 {
            let mut changed = false;
            for (group_link, group) in &src.chains {
                if *group_link == link {
                    add_group(group, &mut groups, &mut coupled, &mut no_gold, &mut no_hint);
                    changed = true;
                }
            }
            if changed {
                levels.push(ChainFactors {
                    no_gold: no_gold.clone(),
                    no_hint: no_hint.clone(),
                    coupled: coupled.clone(),
                });
            }
            class_of_progress.push((levels.len() - 1) as u8);
        }
        if levels.len() > 1 {
            chains.push(ChainProgress { position, levels, class_of_progress });
        }
    }
    // Preserve first-seen order separately from the map: it controls the later floating-point sums.
    let mut collapsed: crate::FxMap<Vec<u8>, f64> = Default::default();
    let mut order: Vec<Vec<u8>> = vec![];
    for (progress, probability) in states.iter() {
        let key: Vec<u8> =
            chains.iter().map(|chain| chain.class_of_progress[progress[chain.position] as usize]).collect();
        match collapsed.get_mut(&key) {
            Some(weight) => *weight += probability,
            None => {
                collapsed.insert(key.clone(), *probability);
                order.push(key);
            }
        }
    }
    let states = order
        .into_iter()
        .map(|key| {
            let probability = collapsed[&key];
            (key, probability)
        })
        .collect();
    JointPrep {
        base: ChainFactors { no_gold: base_no_gold, no_hint: base_no_hint, coupled: base_coupled },
        chains,
        states,
        groups,
    }
}
/// `joint_fast` on a prepared deck (`joint_prep`): the same sums in the same order; `marg`: sum `ext`'s `marg` (else
/// left empty, every other number the same)
pub(super) fn joint_eval(
    model: &Model,
    u: usize,
    qs: &[f64],
    pre: &JointPrep,
    n_targets: usize,
    m1: bool,
    marg: bool,
    ext: Option<&mut JointExt>,
) -> Vec<(f64, f64, usize)> {
    joint_eval_in(model, u, qs, pre, n_targets, m1, marg, ext, &mut JointScratch::default())
}
/// `joint_eval`'s working buffers, kept by a caller that evaluates many times (`Env::corners`): each call sets every
/// one to the contents a fresh one starts with (the same lengths and values; the clusters' and the landed sets' inner
/// buffers are cleared where first used, as fresh ones are), so only their capacity carries over
#[derive(Default)]
pub(super) struct JointScratch {
    dist: Vec<Vec<f64>>,
    acc: Vec<(f64, f64)>,
    fv: Vec<[f64; 3]>,
    ng: Vec<f64>,
    nh: Vec<f64>,
    coupled: Vec<usize>,
    cl_of: Vec<usize>,
    lvl: Vec<u8>,
    par: Vec<usize>,
    cl_ts: Vec<Vec<usize>>,
    cl_gs: Vec<Vec<usize>>,
    in_cl: Vec<bool>,
    ua: Vec<f64>,
    ub: Vec<f64>,
    ca: Vec<f64>,
    cb: Vec<f64>,
    pick: Vec<usize>,
    cd: Vec<f64>,
    cm: Vec<usize>,
    pa: Vec<(usize, f64)>,
    pb: Vec<(usize, f64)>,
    nd: Vec<f64>,
}
/// `joint_eval` with its buffers from `sx`
pub(super) fn joint_eval_in(
    model: &Model,
    u: usize,
    qs: &[f64],
    pre: &JointPrep,
    n_targets: usize,
    m1: bool,
    marg: bool,
    mut ext: Option<&mut JointExt>,
    sx: &mut JointScratch,
) -> Vec<(f64, f64, usize)> {
    let nu = model.uncertain.len();
    let JointScratch {
        dist,
        acc,
        fv,
        ng,
        nh,
        coupled,
        cl_of,
        lvl,
        par,
        cl_ts,
        cl_gs,
        in_cl,
        ua,
        ub,
        ca,
        cb,
        pick,
        cd,
        cm,
        pa,
        pb,
        nd,
    } = std::mem::take(sx);
    let (mut dist, mut acc, mut fv, mut ng, mut nh, mut coupled, mut cl_of, mut lvl) =
        (dist, acc, fv, ng, nh, coupled, cl_of, lvl);
    let (mut par, mut cl_ts, mut cl_gs, mut in_cl, mut ua, mut ub, mut ca, mut cb) =
        (par, cl_ts, cl_gs, in_cl, ua, ub, ca, cb);
    let (mut pick, mut cd, mut cm, mut pa, mut pb, mut nd) = (pick, cd, cm, pa, pb, nd);
    let JointPrep { base, chains, states, groups } = pre;
    let opts = &model.opts[u];
    let no = opts.len();
    let nq = qs.len();
    let nfix = model.n_fixed_gold as usize;
    let fixed = match &ext {
        Some(x) => x.sf.iter().fold(1.0, |a, s| a * GEN_GOLD * s),
        None => model.fixed_factor(),
    };
    let want_sets = ext.as_ref().is_some_and(|x| !x.sets.is_empty()) && nu <= 16;
    if let Some(x) = ext.as_deref_mut() {
        if marg {
            x.marg = vec![0.0; nq * no * nu];
        }
        if want_sets {
            x.sets = vec![0.0; (nq * no) << nu];
        }
    }
    // (a scale of exactly 1.0 for every target leaves each factor as it is, x * 1.0 = x: not multiplied)
    let su1 = ext.as_ref().is_none_or(|x| x.su.iter().all(|&s| s == 1.0));
    dist.resize(if want_sets { nq * no } else { 0 }, vec![]);
    acc.clear();
    acc.resize(nq * no, (0.0f64, 0.0f64));
    // per (q, outfit, target): factor with no coupled outcome, with a coupled white (any hint), with a coupled gold
    let nqo = nq * no;
    fv.clear();
    fv.resize(nqo * nu, [0.0f64; 3]);
    ng.clear();
    ng.resize(nu, 1.0);
    nh.clear();
    nh.resize(nu, 1.0);
    coupled.clear();
    cl_of.clear();
    cl_of.resize(nu, usize::MAX);
    lvl.clear();
    lvl.resize(nu, 0);
    // per chain state's buffers, reused (each written before it is read): the union-find, the clusters (targets, groups;
    // the first ncl in use), the per-unit products, per cluster its groups' outcomes and the pick of each, and with
    // `want_sets` its subset distribution per (chance, outfit) (dense: entry j of each holds the mask cm[j]; every combo
    // writes every subset, in the same order), the subsets of one combo (pa, pb) and the next landed-set rows (nd)
    par.clear();
    in_cl.clear();
    in_cl.resize(nu, false);
    for v in [&mut ua, &mut ub, &mut ca, &mut cb] {
        v.clear();
        v.resize(nqo, 0.0);
    }
    let mut alts: Vec<&[(f64, &[(usize, u8)])]> = vec![];
    pick.clear();
    cd.clear();
    cm.clear();
    pa.clear();
    pb.clear();
    nd.clear();
    for (key, w) in states {
        let w = *w;
        ng.copy_from_slice(&base.no_gold);
        nh.copy_from_slice(&base.no_hint);
        coupled.clear();
        coupled.extend(base.coupled.iter().copied());
        for (j, chain) in chains.iter().enumerate() {
            let factors = &chain.levels[key[j] as usize];
            for k in 0..nu {
                ng[k] *= factors.no_gold[k];
                nh[k] *= factors.no_hint[k];
            }
            coupled.extend(factors.coupled.iter().copied());
        }
        for (qi, &q) in qs.iter().enumerate() {
            for (o, (st, _)) in opts.iter().enumerate() {
                let ew = &model.ews[u][o];
                let base = (qi * no + o) * nu;
                for k in 0..nu {
                    let lp = LandPre::new(st[k], ew[k], q);
                    fv[base + k] = [lp.f(ng[k], nh[k]), lp.f(ng[k], 0.0), lp.f(0.0, 0.0)];
                    if !su1 && let Some(x) = &ext {
                        for v in fv[base + k].iter_mut() {
                            *v *= x.su[k];
                        }
                    }
                }
            }
        }
        // clusters of targets joined by coupled groups (union-find over targets)
        par.clear();
        par.extend(0..nu);
        fn root(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for &g in &coupled {
            let ts = &groups[g].targets;
            for w2 in ts.windows(2) {
                let (a, b) = (root(&mut par, w2[0]), root(&mut par, w2[1]));
                if a != b {
                    par[a.max(b)] = a.min(b);
                }
            }
        }
        cl_of.iter_mut().for_each(|x| *x = usize::MAX);
        let mut ncl = 0usize;
        for &g in &coupled {
            let r = root(&mut par, groups[g].targets[0]);
            let ci = match cl_of[r] {
                usize::MAX => {
                    if ncl == cl_ts.len() {
                        cl_ts.push(vec![]);
                        cl_gs.push(vec![]);
                    }
                    cl_ts[ncl].clear();
                    cl_gs[ncl].clear();
                    cl_of[r] = ncl;
                    ncl += 1;
                    ncl - 1
                }
                c => c,
            };
            cl_gs[ci].push(g);
        }
        for k in 0..nu {
            if cl_of[root(&mut par, k)] != usize::MAX {
                let ci = cl_of[root(&mut par, k)];
                cl_ts[ci].push(k);
            }
        }
        for k in 0..nu {
            in_cl[k] = cl_of[root(&mut par, k)] != usize::MAX;
        }
        // per (q, outfit): (P(all land), P(exactly one misses)) of each independent unit, combined
        ua.fill(1.0);
        ub.fill(0.0);
        let push = |ua: &mut [f64], ub: &mut [f64], i: usize, a: f64, b: f64| {
            ub[i] = ub[i] * a + ua[i] * b;
            ua[i] *= a;
        };
        for i in 0..nqo {
            for k in 0..nu {
                if !in_cl[k] {
                    let f = fv[i * nu + k][0];
                    push(&mut ua, &mut ub, i, f, 1.0 - f);
                    if marg && let Some(x) = ext.as_deref_mut() {
                        x.marg[i * nu + k] += w * f;
                    }
                }
            }
        }
        // the landed sets: independent targets first, clusters below (their targets join through `cd`)
        if want_sets {
            for i in 0..nqo {
                let d = &mut dist[i];
                d.clear();
                d.resize(1 << nu, 0.0);
                d[0] = 1.0;
                for k in 0..nu {
                    if in_cl[k] {
                        continue;
                    }
                    let (f, b) = (fv[i * nu + k][0], 1usize << k);
                    for m in 0..(1usize << nu) {
                        if m & b == 0 && d[m] != 0.0 {
                            let x = d[m];
                            d[m] = x * (1.0 - f);
                            d[m | b] += x * f;
                        }
                    }
                }
            }
        }
        for (ts, gs) in cl_ts[..ncl].iter().zip(&cl_gs[..ncl]) {
            alts.clear();
            alts.extend(gs.iter().map(|&g| &groups[g].outcomes[..]));
            ca.fill(0.0);
            cb.fill(0.0);
            // per (chance, outfit): distribution over the subsets of this cluster's targets (as full masks)
            if want_sets {
                let nsub = 1usize << ts.len();
                cd.clear();
                cd.resize(nqo * nsub, 0.0);
                cm.clear();
                cm.resize(nsub, 0);
            }
            pick.clear();
            pick.resize(alts.len(), 0);
            loop {
                let wc: f64 = (0..alts.len()).map(|i| alts[i][pick[i]].0).product();
                for &k in ts {
                    lvl[k] = 0;
                }
                for i in 0..alts.len() {
                    for &(k, l) in alts[i][pick[i]].1 {
                        lvl[k] = lvl[k].max(if l >= 2 { 2 } else { 1 });
                    }
                }
                for i in 0..nqo {
                    let (mut a, mut b) = (1.0f64, 0.0f64);
                    for &k in ts {
                        let f = fv[i * nu + k][lvl[k] as usize];
                        b = b * f + a * (1.0 - f);
                        a *= f;
                    }
                    ca[i] += wc * a;
                    cb[i] += wc * b;
                    if marg && let Some(x) = ext.as_deref_mut() {
                        for &k in ts {
                            x.marg[i * nu + k] += w * wc * fv[i * nu + k][lvl[k] as usize];
                        }
                    }
                    if want_sets {
                        pa.clear();
                        pa.push((0, wc));
                        for &k in ts {
                            let f = fv[i * nu + k][lvl[k] as usize];
                            pb.clear();
                            for &(m, p) in &pa {
                                pb.push((m, p * (1.0 - f)));
                                pb.push((m | 1 << k, p * f));
                            }
                            std::mem::swap(&mut pa, &mut pb);
                        }
                        let ci = &mut cd[i * pa.len()..(i + 1) * pa.len()];
                        for (j, &(m, p)) in pa.iter().enumerate() {
                            ci[j] += p;
                            cm[j] = m;
                        }
                    }
                }
                let mut i = 0;
                while i < alts.len() {
                    pick[i] += 1;
                    if pick[i] < alts[i].len() {
                        break;
                    }
                    pick[i] = 0;
                    i += 1;
                }
                if i == alts.len() {
                    break;
                }
            }
            for i in 0..nqo {
                push(&mut ua, &mut ub, i, ca[i], cb[i]);
            }
            // (each entry m0 | m of a row is written once, m0 holding no target of this cluster: the order is free)
            if want_sets {
                let nsub = cm.len();
                for i in 0..nqo {
                    let d = &mut dist[i];
                    nd.clear();
                    nd.resize(1 << nu, 0.0);
                    for (m0, &x) in d.iter().enumerate() {
                        if x == 0.0 {
                            continue;
                        }
                        for (&m, &p) in cm.iter().zip(&cd[i * nsub..(i + 1) * nsub]) {
                            nd[m0 | m] += x * p;
                        }
                    }
                    std::mem::swap(d, &mut nd);
                }
            }
        }
        for i in 0..nqo {
            let (mut a, mut b) = (ua[i], ub[i]);
            if m1 {
                for j in 0..nfix {
                    let g = match &ext {
                        Some(x) => GEN_GOLD * x.sf[j],
                        None => GEN_GOLD,
                    };
                    b = b * g + a * (1.0 - g);
                    a *= g;
                }
            } else {
                a *= fixed;
            }
            // P(at least n_targets - 1 land) over nu + nfix targets
            let m = if n_targets <= nu + nfix { if n_targets == 0 { 1.0 } else { a + b } } else { 0.0 };
            acc[i].0 += w * a;
            acc[i].1 += w * if m1 { m } else { 0.0 };
            if want_sets && let Some(x) = ext.as_deref_mut() {
                for (m, &p) in dist[i].iter().enumerate() {
                    x.sets[(i << nu) + m] += w * p;
                }
            }
        }
    }
    if let Some(x) = ext {
        x.per_outfit.clear();
        x.per_outfit.extend(acc.iter().map(|a| a.0));
    }
    let r = (0..nq)
        .map(|qi| {
            let mut b = 0;
            for o in 1..no {
                if acc[qi * no + o].0 > acc[qi * no + b].0 {
                    b = o;
                }
            }
            (acc[qi * no + b].0, acc[qi * no + b].1, b)
        })
        .collect();
    *sx = JointScratch {
        dist,
        acc,
        fv,
        ng,
        nh,
        coupled,
        cl_of,
        lvl,
        par,
        cl_ts,
        cl_gs,
        in_cl,
        ua,
        ub,
        ca,
        cb,
        pick,
        cd,
        cm,
        pa,
        pb,
        nd,
    };
    r
}

/// Reference form of `joint_trainee`: every chain state, every combination of coupled outcomes. `JOINT_CHECK=1` runs
/// it beside the fast form on every call and panics on a relative difference above 1e-11.
pub(super) fn joint_trainee_ref(
    model: &Model,
    u: usize,
    qs: &[f64],
    ents: &[(Src, Kind)],
    filler: usize,
    n_targets: usize,
) -> Vec<(f64, f64, usize)> {
    use crate::skills::GEN_WHITE;
    let nu = model.uncertain.len();
    // per-target miss factors (no gold, no hint) of the groups that touch one target; the others are "coupled"
    fn take<'g>(g: &'g Group, coupled: &mut Vec<&'g Group>, ng: &mut [f64], nh: &mut [f64]) {
        let gh = group_gh(g);
        if gh.len() >= 2 {
            coupled.push(g);
            return;
        }
        for (k, pg, ph) in gh {
            ng[k] *= 1.0 - pg;
            nh[k] *= 1.0 - ph;
        }
    }
    // sources that do not depend on the chain
    let (mut bg, mut bh) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut bc: Vec<&Group> = vec![];
    for (src, _) in ents {
        for &(k, p) in &src.train {
            bh[k] *= 1.0 - p;
        }
        for g in &src.fixed {
            take(g, &mut bc, &mut bg, &mut bh);
        }
    }
    // chain entries sorted by kind (with fillers) for the memoised state distribution; per entry and links done,
    // the cumulative factors and coupled groups of the links fired so far
    let mut ch: Vec<(Kind, Option<usize>)> =
        ents.iter().enumerate().filter(|(_, e)| e.1 != (0, 0, 0)).map(|(i, e)| (e.1, Some(i))).collect();
    for _ in 0..filler {
        ch.push(((2, 2, 0), None));
    }
    ch.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let kinds: Vec<Kind> = ch.iter().map(|x| x.0).collect();
    let states = if kinds.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kinds) };
    let lv: Vec<Option<Vec<(Vec<f64>, Vec<f64>, Vec<&Group>)>>> = ch
        .iter()
        .map(|&(kd, e)| {
            e.map(|i| {
                let src = &ents[i].0;
                let (mut g, mut h, mut c) = (vec![1.0f64; nu], vec![1.0f64; nu], vec![]);
                let mut out = vec![(g.clone(), h.clone(), c.clone())];
                for l in 1..=kd.1 {
                    for (cl, grp) in &src.chains {
                        if *cl == l {
                            take(grp, &mut c, &mut g, &mut h);
                        }
                    }
                    out.push((g.clone(), h.clone(), c.clone()));
                }
                out
            })
        })
        .collect();
    let opts = &model.opts[u];
    let no = opts.len();
    let nq = qs.len();
    let nfix = model.n_fixed_gold as usize;
    let fixed = model.fixed_factor();
    let mut acc = vec![(0.0f64, 0.0f64); nq * no];
    let (mut ng, mut nh) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    let mut coupled: Vec<&Group> = vec![];
    let mut alts: Vec<Vec<(f64, &[(usize, u8)])>> = vec![];
    let mut pick: Vec<usize> = vec![];
    let mut dist = [0.0f64; 40];
    for (prog, w) in states.iter() {
        ng.copy_from_slice(&bg);
        nh.copy_from_slice(&bh);
        coupled.clear();
        coupled.extend(bc.iter().copied());
        for (pos, l) in lv.iter().enumerate() {
            let Some(l) = l else { continue };
            let (g, h, c) = &l[prog[pos] as usize];
            for k in 0..nu {
                ng[k] *= g[k];
                nh[k] *= h[k];
            }
            coupled.extend(c.iter().copied());
        }
        // outcomes of the coupled groups: each group's outcomes plus "nothing" (the rest of its weight)
        alts.clear();
        for g in &coupled {
            let mut v: Vec<(f64, &[(usize, u8)])> = g.iter().map(|(p, o)| (*p, &o[..])).collect();
            let rest = 1.0 - g.iter().map(|x| x.0).sum::<f64>();
            if rest > 1e-15 {
                v.push((rest, &[][..]));
            }
            alts.push(v);
        }
        pick.clear();
        pick.resize(alts.len(), 0);
        loop {
            let wc: f64 = (0..alts.len()).map(|i| alts[i][pick[i]].0).product::<f64>() * w;
            let mut fg = [false; 32];
            let mut fh = [false; 32];
            for i in 0..alts.len() {
                for &(k, lvl) in alts[i][pick[i]].1 {
                    fh[k] = true;
                    if lvl >= 2 {
                        fg[k] = true;
                    }
                }
            }
            for (qi, &q) in qs.iter().enumerate() {
                for (o, (st, _)) in opts.iter().enumerate() {
                    let ew = &model.ews[u][o];
                    // P(all land) and the distribution of how many of the uncertain targets land (fixed golds after)
                    let mut all = fixed;
                    dist[..=nu + nfix].iter_mut().for_each(|x| *x = 0.0);
                    dist[0] = 1.0;
                    for k in 0..nu {
                        let pg = if fg[k] { 1.0 } else { 1.0 - ng[k] };
                        let pnh = if fh[k] { 0.0 } else { nh[k] };
                        let f = match st[k] {
                            2 => GEN_GOLD,
                            1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
                            _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - pnh * (1.0 - ew[k]) * (1.0 - q)),
                        };
                        all *= f;
                        for j in (0..=k).rev() {
                            dist[j + 1] += dist[j] * f;
                            dist[j] *= 1.0 - f;
                        }
                    }
                    for _ in 0..nfix {
                        for j in (0..=nu + nfix).rev() {
                            if j + 1 < dist.len() {
                                dist[j + 1] += dist[j] * GEN_GOLD;
                            }
                            dist[j] *= 1.0 - GEN_GOLD;
                        }
                    }
                    let m1: f64 = dist[n_targets.saturating_sub(1).min(nu + nfix)..=nu + nfix].iter().sum();
                    let a = &mut acc[qi * no + o];
                    a.0 += wc * all;
                    a.1 += wc * m1;
                }
            }
            let mut i = 0;
            while i < alts.len() {
                pick[i] += 1;
                if pick[i] < alts[i].len() {
                    break;
                }
                pick[i] = 0;
                i += 1;
            }
            if i == alts.len() {
                break;
            }
        }
    }
    (0..nq)
        .map(|qi| {
            let mut b = 0;
            for o in 1..no {
                if acc[qi * no + o].0 > acc[qi * no + b].0 {
                    b = o;
                }
            }
            (acc[qi * no + b].0, acc[qi * no + b].1, b)
        })
        .collect()
}

/// Upper bound on exact P(all), per inheritance chance in `qs`, including the fixed-gold factor.
/// Each chain card is grouped with the targets it feeds. Given N = n, the checked upper-orthant inequality lets
/// the expectation of the product be bounded by the product of expectations over those groups. One pair of cards
/// may form a joint group (`KeyLevel::pair_ok`); targets that would connect more cards are bounded at full progress.
/// The proof writes each rising group factor as a positive combination of indicators {X_i >= t}; the checked
/// inequality bounds each term, giving U = sum_n P(N = n) product_groups E[group factor | n], times the factors
/// of targets outside those groups. Event outcomes and training hints are independent of the chain allocation.
/// Event groups must pass `group_nuod`, since their per-target marginals replace the full outcome distribution.
/// Returns None when either dependence check is unproven; callers must then use a different bound or score exactly.
pub(super) fn upper_na(model: &Model, u: usize, qs: &[f64], ents: &[(&Src, Kind)], filler: usize) -> Option<Vec<f64>> {
    let nu = model.uncertain.len();
    // every group touching two or more targets must be negatively upper-orthant dependent, or the per-target
    // marginals below could understate a group whose outcomes hint targets together
    // (checked once per source, when it is built: `Src::not_nuod`)
    for (src, _) in ents {
        if src.not_nuod {
            if std::env::var("UPPER_WHY").is_ok()
                && let Some(g) = src
                    .fixed
                    .iter()
                    .chain(src.chains.iter().map(|x| &x.1))
                    .find(|g| group_gh(g).len() >= 2 && !group_nuod(g))
            {
                eprintln!("why: group {:?}", g);
            }
            return None;
        }
    }
    let (mut ng0, mut nh0) = (vec![1.0f64; nu], vec![1.0f64; nu]);
    for (src, _) in ents {
        for &(k, p) in &src.train {
            nh0[k] *= 1.0 - p;
        }
        // (Src::fm: the fixed groups' (k, 1 - P(gold), 1 - P(any hint)) in group order, as group_gh gives them: the same
        // factors in the same order, computed once per source)
        for &(k, mg, mh) in &src.fm {
            ng0[k] *= mg;
            nh0[k] *= mh;
        }
    }
    // chain cards in key order (fillers included): (kind, per target: cumulative (miss gold, miss any) after l links)
    let mut chains: Vec<(Kind, Vec<(usize, Vec<(f64, f64)>)>)> = vec![];
    for (src, kd) in ents {
        if *kd == (0, 0, 0) {
            continue;
        }
        let stop = kd.1 as usize;
        let ts: Vec<(usize, Vec<(f64, f64)>)> = src
            .cm
            .iter()
            .map(|(k, per)| {
                let mut cum = vec![(1.0f64, 1.0f64); stop + 1];
                for l in 1..=stop {
                    cum[l] = cum[l - 1];
                    for &(lk, g, h) in per {
                        if lk as usize == l {
                            cum[l].0 *= g;
                            cum[l].1 *= h;
                        }
                    }
                }
                (*k, cum)
            })
            .collect();
        chains.push((*kd, ts));
    }
    for _ in 0..filler {
        chains.push(((2, 2, 0), vec![]));
    }
    chains.sort_by_key(|a| a.0);
    let kinds: Vec<Kind> = chains.iter().map(|c| c.0).collect();
    // groups: cards joined by a target their chains both feed; at most one group of two cards, none larger. A target
    // that would join a larger group (three or more feeders, or two feeders other than the one pair) is "maxed": its
    // factor is taken at every feeder's full progress, a constant, and it leaves its feeders' target lists. Valid
    // (derived): per target phi = G(1 - ng) + W(ng - nh a) with a = (1 - e)(1 - q) in [0, 1] and G > W, so phi >= 0
    // (nh <= ng) and phi falls with ng and with nh; the miss products only fall as links progress (each link's factor
    // is at most 1), so f_k(L) <= f_k(full progress) on every chain state L, and E[prod_k f_k] <= E[prod_kept f_k] x
    // prod_maxed f_k^max; the kept targets then meet the structure the bound below needs.
    let mut feeders: Vec<Vec<usize>> = vec![vec![]; nu];
    for (ci, (_, ts)) in chains.iter().enumerate() {
        for (k, _) in ts {
            feeders[*k].push(ci);
        }
    }
    let mut pair: Option<(usize, usize)> = None;
    let mut maxed: Vec<Option<(f64, f64)>> = vec![None; nu];
    for k in 0..nu {
        let mx = match feeders[k].len() {
            0 | 1 => false,
            2 => {
                let p = (feeders[k][0], feeders[k][1]);
                if pair.is_some_and(|x| x != p) {
                    true
                } else {
                    pair = Some(p);
                    false
                }
            }
            _ => true,
        };
        if mx {
            let (mut g, mut h) = (ng0[k], nh0[k]);
            for &ci in &feeders[k] {
                let cum = &chains[ci].1.iter().find(|x| x.0 == k).unwrap().1;
                let last = cum[cum.len() - 1];
                g *= last.0;
                h *= last.1;
            }
            maxed[k] = Some((g, h));
        }
    }
    if maxed.iter().any(|m| m.is_some()) {
        for c in chains.iter_mut() {
            c.1.retain(|(k, _)| maxed[*k].is_none());
        }
        for k in 0..nu {
            if maxed[k].is_some() {
                feeders[k].clear();
            }
        }
    }
    let fed: Vec<bool> = (0..nu).map(|k| !feeders[k].is_empty()).collect();
    let kl = if kinds.is_empty() { None } else { Some(key_level(&kinds)) };
    if let Some(k) = &kl {
        let ok = match pair {
            None => k.nuod,
            Some((a, b)) => k.pair_ok(a, b),
        };
        if !ok {
            return None;
        }
    }
    let pmf = event_pmf_job();
    let opts = &model.opts[u];
    let fixed = model.fixed_factor();
    let phi = |o: usize, q: f64, k: usize, ng: f64, nh: f64| -> f64 {
        land_factor(opts[o].0[k], model.ews[u][o][k], q, ng, nh)
    };
    let pm: Option<std::rc::Rc<Vec<Vec<Vec<f64>>>>> = match (pair, &kl) {
        (Some((a, b)), Some(k)) => Some(k.pair_margs(a, b)),
        _ => None,
    };
    Some(
        qs.iter()
            .map(|&q| {
                let mut best = 0.0f64;
                for o in 0..opts.len() {
                    let mut other = 1.0;
                    for k in 0..nu {
                        if let Some((g, h)) = maxed[k] {
                            other *= phi(o, q, k, g, h);
                        } else if !fed[k] {
                            other *= phi(o, q, k, ng0[k], nh0[k]);
                        }
                    }
                    // per single card and links done: the product over the targets only it feeds
                    let single = |ci: usize, l: usize| -> f64 {
                        chains[ci]
                            .1
                            .iter()
                            .filter(|(k, _)| feeders[*k].len() == 1)
                            .map(|(k, cum)| phi(o, q, *k, ng0[*k] * cum[l].0, nh0[*k] * cum[l].1))
                            .product()
                    };
                    let comp: Vec<Vec<f64>> = (0..chains.len())
                        .map(|ci| (0..=chains[ci].0.1 as usize).map(|l| single(ci, l)).collect())
                        .collect();
                    // the pair's group: both cards' single targets and the shared ones, over (la, lb)
                    let pairv: Option<Vec<Vec<f64>>> = pair.map(|(a, b)| {
                        (0..=chains[a].0.1 as usize)
                            .map(|la| {
                                (0..=chains[b].0.1 as usize)
                                    .map(|lb| {
                                        let mut v = comp[a][la] * comp[b][lb];
                                        for (k, cum_a) in &chains[a].1 {
                                            if feeders[*k].len() == 2 {
                                                let cum_b = &chains[b].1.iter().find(|x| x.0 == *k).unwrap().1;
                                                v *= phi(
                                                    o,
                                                    q,
                                                    *k,
                                                    ng0[*k] * cum_a[la].0 * cum_b[lb].0,
                                                    nh0[*k] * cum_a[la].1 * cum_b[lb].1,
                                                );
                                            }
                                        }
                                        v
                                    })
                                    .collect()
                            })
                            .collect()
                    });
                    let mut tot = 0.0;
                    for (m, &pn) in pmf.iter().enumerate() {
                        if pn == 0.0 {
                            continue;
                        }
                        let mut prod = other;
                        if let Some(k) = &kl {
                            for ci in 0..chains.len() {
                                if chains[ci].1.is_empty() || pair.is_some_and(|(a, b)| ci == a || ci == b) {
                                    continue;
                                }
                                prod *= k.marg[ci][m].iter().zip(&comp[ci]).map(|(p, c)| p * c).sum::<f64>();
                            }
                            if let (Some(pv), Some(pmm)) = (&pairv, &pm) {
                                prod *= pmm[m]
                                    .iter()
                                    .zip(pv)
                                    .map(|(r, s)| r.iter().zip(s).map(|(p, c)| p * c).sum::<f64>())
                                    .sum::<f64>();
                            }
                        }
                        tot += pn * prod;
                    }
                    best = best.max(tot);
                }
                fixed * best * (1.0 + 1e-9)
            })
            .collect(),
    )
}

/// One entry's miss chances at each number of its chain links done: row d = links 1..=d fired, per target (P(no gold),
/// P(no hint)); training hints and non-chain groups in every row. `stop` rows after row 0. The same factors
/// `joint_fast` applies to an entry at chain progress d (each group's per-target chances, `fm` / `cm`).
pub(super) fn src_levels(s: &Src, stop: u8, nu: usize) -> Vec<(f64, f64)> {
    let mut base = vec![(1.0f64, 1.0f64); nu];
    for &(k, p) in &s.train {
        base[k].1 *= 1.0 - p;
    }
    for &(k, ng, nh) in &s.fm {
        base[k].0 *= ng;
        base[k].1 *= nh;
    }
    let mut out = Vec::with_capacity((stop as usize + 1) * nu);
    out.extend_from_slice(&base);
    for d in 1..=stop {
        let mut row: Vec<(f64, f64)> = out[(d as usize - 1) * nu..d as usize * nu].to_vec();
        for (k, links) in &s.cm {
            for &(l, ng, nh) in links {
                if l == d {
                    row[*k].0 *= ng;
                    row[*k].1 *= nh;
                }
            }
        }
        out.extend_from_slice(&row);
    }
    out
}
/// Target k's landing factor at its misses (no gold `ng`, no hint `nh`), the one definition `joint_eval`, `upper_na`
/// and the state bounds take (falls with ng and with nh). `skills::land_dw` writes the same model in one deck's
/// per-target odds; its floats are its own.
#[inline]
pub(super) fn land_factor(st: u8, ew: f64, q: f64, ng: f64, nh: f64) -> f64 {
    LandPre::new(st, ew, q).f(ng, nh)
}
/// `land_factor` of one (outfit, target) at one chance, its constants taken once: 1 - ew and 1 - q kept apart, so
/// nh * (1 - ew) * (1 - q) is multiplied in the same order (the same f64 for every ng, nh)
#[derive(Clone, Copy)]
pub(super) struct LandPre {
    st: u8,
    ne: f64,
    omq: f64,
}
impl LandPre {
    pub(super) fn new(st: u8, ew: f64, q: f64) -> Self {
        LandPre { st, ne: 1.0 - ew, omq: 1.0 - q }
    }
    #[inline]
    pub(super) fn f(&self, ng: f64, nh: f64) -> f64 {
        use crate::skills::GEN_WHITE;
        let pg = 1.0 - ng;
        match self.st {
            2 => GEN_GOLD,
            1 => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg),
            _ => GEN_GOLD * pg + GEN_WHITE * (1.0 - pg - nh * self.ne * self.omq),
        }
    }
}
/// The state-conditioned value of one deck (trainee `u`, per chance in `qs`, fixed factor included): over the chain
/// progress states x (`chain_states`, early ends and stop risk folded in), P(x) times the product over targets of each
/// target's factor at its misses given x, every group's outcomes at their per-target chances; the best outfit.
/// Derived: given x, every source's outcomes are independent of the others' (`joint_fast`); one group's outcomes are a
/// single draw, so when each outcome hints at most one target its per-target indicators are negatively associated
/// (a one-trial multinomial), and so are increasing functions of disjoint sets of them; each target's factor rises
/// with its own indicators, so the expected product given x is at most the product of the targets' expected factors,
/// equal without coupled groups. Hence value >= the exact P(all), equal when no group touches two targets.
/// None when some entry fails the negative-association certification (`Src::multi`, `group_assoc`).
pub(super) fn state_value(
    model: &Model,
    u: usize,
    qs: &[f64],
    ents: &[(&Src, Kind)],
    filler: usize,
) -> Option<Vec<f64>> {
    if ents.iter().any(|e| e.0.multi) {
        return None;
    }
    state_value_any(model, u, qs, ents, filler)
}
/// `state_value` for the decks the cap-free search covers (`Src::cf_ok`: every group's levels negatively upper-orthant
/// dependent, `group_nuod_lvl`, all the derivation above needs of a group); None otherwise. The same value where both
/// apply.
pub(super) fn state_value_cf(
    model: &Model,
    u: usize,
    qs: &[f64],
    ents: &[(&Src, Kind)],
    filler: usize,
) -> Option<Vec<f64>> {
    if ents.iter().any(|e| !e.0.cf_ok) {
        return None;
    }
    state_value_any(model, u, qs, ents, filler)
}
fn state_value_any(model: &Model, u: usize, qs: &[f64], ents: &[(&Src, Kind)], filler: usize) -> Option<Vec<f64>> {
    let nu = model.uncertain.len();
    let mut ch: Vec<(Kind, Option<usize>)> =
        ents.iter().enumerate().filter(|(_, e)| e.1 != (0, 0, 0)).map(|(i, e)| (e.1, Some(i))).collect();
    for _ in 0..filler {
        ch.push(((2, 2, 0), None));
    }
    ch.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(&b.1)));
    let kinds: Vec<Kind> = ch.iter().map(|x| x.0).collect();
    let states = if kinds.is_empty() { std::rc::Rc::new(vec![(vec![], 1.0)]) } else { chain_states(&kinds) };
    // the non-chain entries' misses (constant over states), and per chain position its entry's rows
    let mut cg = vec![1.0f64; nu];
    let mut chh = vec![1.0f64; nu];
    for e in ents.iter().filter(|e| e.1 == (0, 0, 0)) {
        let r = src_levels(e.0, 0, nu);
        for k in 0..nu {
            cg[k] *= r[k].0;
            chh[k] *= r[k].1;
        }
    }
    let rows: Vec<Option<Vec<(f64, f64)>>> =
        ch.iter().map(|&(kd, e)| e.map(|i| src_levels(ents[i].0, kd.1, nu))).collect();
    let opts = &model.opts[u];
    let no = opts.len();
    let mut acc = vec![0.0f64; qs.len() * no];
    let (mut ng, mut nh) = (vec![0.0f64; nu], vec![0.0f64; nu]);
    for (x, w) in states.iter() {
        ng.copy_from_slice(&cg);
        nh.copy_from_slice(&chh);
        for (p, r) in rows.iter().enumerate() {
            if let Some(r) = r {
                let b = x[p] as usize * nu;
                for k in 0..nu {
                    ng[k] *= r[b + k].0;
                    nh[k] *= r[b + k].1;
                }
            }
        }
        for (qi, &q) in qs.iter().enumerate() {
            for (o, (st, _)) in opts.iter().enumerate() {
                let ew = &model.ews[u][o];
                let f: f64 = (0..nu).map(|k| land_factor(st[k], ew[k], q, ng[k], nh[k])).product();
                acc[qi * no + o] += w * f;
            }
        }
    }
    let fixed = model.fixed_factor();
    Some((0..qs.len()).map(|qi| fixed * (0..no).map(|o| acc[qi * no + o]).fold(0.0f64, f64::max)).collect())
}
thread_local! {
    /// SB_CHECK: per exact scoring, the state-conditioned value against the exact value: (decks, max value/exact, min
    /// value/exact, violations); and the node bounds against the decks below them: (leaves checked, violations)
    pub(crate) static SBCHK: std::cell::RefCell<(usize, f64, f64, usize, usize, usize)> = const { std::cell::RefCell::new((0, 0.0, f64::INFINITY, 0, 0, 0)) };
}
thread_local! { pub(crate) static SBN: std::cell::RefCell<(usize, usize, usize)> = const { std::cell::RefCell::new((0, 0, 0)) }; }
/// A configuration's state-conditioned bound tables: the chain state distribution projected on the positions whose
/// group has chain targets, per group its slots' positions (usize::MAX: level-independent), per group and chain level
/// each target's lowest misses over the group's entries
pub(super) struct SbCfg {
    pub(super) states: Vec<(Vec<u8>, f64)>,
    pub(super) slots: Vec<Vec<usize>>,
    pub(super) gmin: Vec<Vec<(f64, f64)>>,
    /// the configuration's chain kinds in position order (fillers included) and each projected position's place among them
    pub(super) kl: Vec<Kind>,
    pub(super) rel_pos: Vec<usize>,
    /// (the cap-free bound's early stop) the states by weight, highest first, and the weight from each on (built on first use)
    pub(super) order: std::cell::OnceCell<(Vec<u32>, Vec<f64>)>,
}
pub(super) fn sb_check() -> bool {
    static C: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("SB_CHECK").is_ok())
}
/// The state-conditioned bound in the career searches' band pass (`best_deck` pass 2), `SBOUND` to change it: 3 (the
/// default) at each configuration's root, an extra prune beside the rho one (exact: it only drops configurations no deck
/// of which can reach the bar); 1 also at depth 1; 2 the band pass without the cap (the bound at every node, a leaf kept
/// when its state-conditioned value reaches the bar; measured far slower); 0 off. Measured (val/plan, user CPU): roots
/// only, Grand Masters -24 to -28%, nodes -57%; Global within noise; depth 1 costs more than it saves on Global.
pub(crate) fn sb_mode() -> u8 {
    static C: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("SBOUND").ok().and_then(|v| v.parse().ok()).unwrap_or(3))
}
thread_local! { pub(crate) static SB_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) }; }
thread_local! { pub(crate) static SB_USED: std::cell::Cell<u8> = const { std::cell::Cell::new(0) }; }
thread_local! {
    /// the next band search is cap-free (`Env::best_deck_cf`): pruned by its two proven bounds alone (`cf_cheap`, `cf_ub`), at every
    /// node and leaf, no cap anywhere; it stops (`over`) past CF_LIM partial decks tested (the space's node count)
    pub(crate) static CF_ON: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    pub(crate) static CF_LIM: std::cell::Cell<usize> = const { std::cell::Cell::new(usize::MAX) };
    /// CF_CHECK: (cap-free searches run beside a capped one, capped answers below the cap-free one (a deck the caps
    /// pruned), cap-free answers below the capped one (a bug), unproven cap-free runs; leaves checked against their
    /// ancestors' bounds, violations (an exact value above a bound), exact scorings checked against the state value,
    /// violations, the largest exact / bound)
    pub(crate) static CFCHK: std::cell::RefCell<(usize, usize, usize, usize, usize, usize, usize, usize, f64)> = const { std::cell::RefCell::new((0, 0, 0, 0, 0, 0, 0, 0, 0.0)) };
}
/// CF_CHECK=s (checks only): every capped deck search also runs the cap-free one (state restored after it) and compares
/// answers; inside a cap-free search every node passes at s times its bound (s >= 1: more leaves reached; a leaf keeps
/// the real test, so the band is the same) and every leaf reached is scored exactly against the bounds of every node
/// above it (an exact value above one would mean the bound could prune a deck that beats the bar). 0 = off.
pub(crate) fn cf_check() -> f64 {
    static C: std::sync::OnceLock<f64> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("CF_CHECK").ok().map_or(0.0, |v| v.parse().unwrap_or(1.0)))
}
