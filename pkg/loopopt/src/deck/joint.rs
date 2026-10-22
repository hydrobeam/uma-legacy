//! Joint landing odds of a deck: exact over chain progress and outcome groups, the negative-association bound, and the
//! state-conditioned bound.
use super::{GEN_GOLD, Group, JCOUNT, Kind, Model, Src, chain_states, event_pmf_job, group_gh, group_nuod, key_level};

/// The trainee and ordered inheritance chances to evaluate against one deck.
#[derive(Clone, Copy)]
struct JointQuery<'chance> {
    /// Index in `Model::opts` and `Model::ews`.
    trainee: crate::UmaIndex,
    inheritance_chances: &'chance [f64],
}

/// Which exact landing probabilities the caller needs.
#[derive(Clone, Copy)]
struct JointOptions {
    /// Number of requested targets, including guaranteed golds.
    target_count: usize,
    /// Also compute the chance of landing at least all but one target.
    all_but_one: bool,
    /// Fill `JointExt::marg` when an extension is supplied.
    marginals: bool,
}

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
    u: crate::UmaIndex,
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
    u: crate::UmaIndex,
    qs: &[f64],
    ents: &[(S, Kind)],
    filler: usize,
    n_targets: usize,
    m1: bool,
) -> Vec<(f64, f64, usize)> {
    joint_checked(
        model,
        JointQuery { trainee: u, inheritance_chances: qs },
        ents,
        filler,
        JointOptions { target_count: n_targets, all_but_one: m1, marginals: true },
    )
}

fn joint_checked<S: std::borrow::Borrow<Src>>(
    model: &Model,
    query: JointQuery,
    ents: &[(S, Kind)],
    filler: usize,
    options: JointOptions,
) -> Vec<(f64, f64, usize)> {
    JCOUNT.with(|c| c.set(c.get() + 1));
    let r = evaluate_joint(model, query, ents, filler, options, None);
    if sb_check() {
        let er: Vec<(&Src, Kind)> = ents.iter().map(|e| (e.0.borrow(), e.1)).collect();
        if let Some(sv) = state_value(model, query.trainee, query.inheritance_chances, &er, filler) {
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
        if let Some(sv) = state_value_cf(model, query.trainee, query.inheritance_chances, &er, filler) {
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
        let x = joint_trainee_ref(model, query.trainee, query.inheritance_chances, &own, filler, options.target_count);
        for (a, b) in r.iter().zip(&x) {
            let rel = |p: f64, q: f64| (p - q).abs() / q.abs().max(1e-300);
            assert!(
                rel(a.0, b.0) < 1e-11 && (!options.all_but_one || rel(a.1, b.1) < 1e-11),
                "joint mismatch: {a:?} vs {b:?}"
            );
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
    u: crate::UmaIndex,
    qs: &[f64],
    ents: &[(S, Kind)],
    filler: usize,
    n_targets: usize,
    m1: bool,
    ext: Option<&mut JointExt>,
) -> Vec<(f64, f64, usize)> {
    evaluate_joint(
        model,
        JointQuery { trainee: u, inheritance_chances: qs },
        ents,
        filler,
        JointOptions { target_count: n_targets, all_but_one: m1, marginals: true },
        ext,
    )
}

fn evaluate_joint<S: std::borrow::Borrow<Src>>(
    model: &Model,
    query: JointQuery,
    ents: &[(S, Kind)],
    filler: usize,
    options: JointOptions,
    ext: Option<&mut JointExt>,
) -> Vec<(f64, f64, usize)> {
    joint_eval(model, query, &joint_prep(model.uncertain.len(), ents, filler), options, ext)
}
/// The deck-dependent part of `joint_fast`, before inheritance chances or outfit state are applied.
/// Collapsed states retain first-seen order; each weight is summed in the original chain-state order.
pub(super) struct JointPrep<'group> {
    /// Miss factors and coupled groups from sources outside the chains.
    base: ChainFactors,
    chains: Vec<ChainProgress>,
    /// One class index per relevant chain position, and that collapsed state's probability.
    states: Vec<CollapsedChainState>,
    groups: Vec<CoupledGroup<'group>>,
}
struct CollapsedChainState {
    /// Class index at each relevant chain position.
    classes: Vec<u8>,
    probability: f64,
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
struct CoupledGroup<'group> {
    targets: Vec<usize>,
    outcomes: Vec<(f64, &'group [(usize, u8)])>,
}
pub(super) fn joint_prep<S: std::borrow::Borrow<Src>>(nu: usize, ents: &[(S, Kind)], filler: usize) -> JointPrep<'_> {
    fn add_group<'group>(
        group: &'group Group,
        groups: &mut Vec<CoupledGroup<'group>>,
        coupled: &mut Vec<usize>,
        no_gold: &mut [f64],
        no_hint: &mut [f64],
    ) {
        let marginals = group_gh(group);
        if marginals.len() >= 2 {
            let mut outcomes: Vec<(f64, &[(usize, u8)])> =
                group.iter().map(|(p, rewards)| (*p, (&**rewards))).collect();
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
    let mut collapsed: crate::FxMap<Vec<u8>, f64> = crate::FxMap::default();
    let mut order: Vec<Vec<u8>> = vec![];
    for (progress, probability) in states.iter() {
        let key: Vec<u8> =
            chains.iter().map(|chain| chain.class_of_progress[progress[chain.position] as usize]).collect();
        if let Some(weight) = collapsed.get_mut(&key) {
            *weight += probability;
        } else {
            collapsed.insert(key.clone(), *probability);
            order.push(key);
        }
    }
    let states = order
        .into_iter()
        .map(|key| {
            let probability = collapsed[&key];
            CollapsedChainState { classes: key, probability }
        })
        .collect();
    JointPrep {
        base: ChainFactors { no_gold: base_no_gold, no_hint: base_no_hint, coupled: base_coupled },
        chains,
        states,
        groups,
    }
}
/// Evaluate a prepared deck with fresh scratch. Disabling marginals leaves the extension's marginal buffer empty.
fn joint_eval(
    model: &Model,
    query: JointQuery,
    prepared: &JointPrep,
    options: JointOptions,
    extension: Option<&mut JointExt>,
) -> Vec<(f64, f64, usize)> {
    evaluate_prepared(model, query, prepared, options, extension, &mut JointScratch::default())
}
#[derive(Clone, Copy)]
struct LandingProbability {
    all: f64,
    all_but_one: f64,
}

/// `joint_eval`'s working buffers, kept by a caller that evaluates many times (`Env::corners`): each call sets every
/// one to the contents a fresh one starts with (the same lengths and values; the clusters' and the landed sets' inner
/// buffers are cleared where first used, as fresh ones are), so only their capacity carries over
#[derive(Default)]
pub(super) struct JointScratch {
    landed_sets: Vec<Vec<f64>>,
    totals: Vec<LandingProbability>,
    landing_factors: Vec<[f64; 3]>,
    no_gold: Vec<f64>,
    no_hint: Vec<f64>,
    coupled_groups: Vec<usize>,
    cluster_of_target: Vec<usize>,
    hint_levels: Vec<u8>,
    target_parents: Vec<usize>,
    cluster_targets: Vec<Vec<usize>>,
    cluster_groups: Vec<Vec<usize>>,
    in_cluster: Vec<bool>,
    all_land: Vec<f64>,
    one_misses: Vec<f64>,
    cluster_all_land: Vec<f64>,
    cluster_one_misses: Vec<f64>,
    outcome_indices: Vec<usize>,
    cluster_distribution: Vec<f64>,
    cluster_masks: Vec<usize>,
    subsets: Vec<(usize, f64)>,
    next_subsets: Vec<(usize, f64)>,
    next_distribution: Vec<f64>,
}
impl JointScratch {
    /// Clear values while retaining capacities, including each cluster's inner buffers.
    fn reset(&mut self, target_count: usize, query_outfit_count: usize, want_sets: bool) {
        self.landed_sets.resize(if want_sets { query_outfit_count } else { 0 }, vec![]);
        self.totals.clear();
        self.totals.resize(query_outfit_count, LandingProbability { all: 0.0, all_but_one: 0.0 });
        // Per (chance, outfit, target): no coupled hint, coupled white hint, coupled gold hint.
        self.landing_factors.clear();
        self.landing_factors.resize(query_outfit_count * target_count, [0.0f64; 3]);
        self.no_gold.clear();
        self.no_gold.resize(target_count, 1.0);
        self.no_hint.clear();
        self.no_hint.resize(target_count, 1.0);
        self.coupled_groups.clear();
        self.cluster_of_target.clear();
        self.cluster_of_target.resize(target_count, usize::MAX);
        self.hint_levels.clear();
        self.hint_levels.resize(target_count, 0);
        // Each chain state writes these buffers before reading them. Cluster distributions are dense:
        // entry j holds cluster_masks[j], and each outcome combination writes subsets in the same order.
        self.target_parents.clear();
        self.in_cluster.clear();
        self.in_cluster.resize(target_count, false);
        for v in [&mut self.all_land, &mut self.one_misses, &mut self.cluster_all_land, &mut self.cluster_one_misses] {
            v.clear();
            v.resize(query_outfit_count, 0.0);
        }
        self.outcome_indices.clear();
        self.cluster_distribution.clear();
        self.cluster_masks.clear();
        self.subsets.clear();
        self.next_subsets.clear();
        self.next_distribution.clear();
    }

    /// Union targets connected by coupled outcome groups, retaining first-seen cluster order.
    fn build_clusters(&mut self, groups: &[CoupledGroup], target_count: usize) -> usize {
        self.target_parents.clear();
        self.target_parents.extend(0..target_count);
        const fn root(p: &mut [usize], mut x: usize) -> usize {
            while p[x] != x {
                p[x] = p[p[x]];
                x = p[x];
            }
            x
        }
        for &g in &self.coupled_groups {
            let targets = &groups[g].targets;
            for w2 in targets.windows(2) {
                let (a, b) = (root(&mut self.target_parents, w2[0]), root(&mut self.target_parents, w2[1]));
                if a != b {
                    self.target_parents[a.max(b)] = a.min(b);
                }
            }
        }
        self.cluster_of_target.fill(usize::MAX);
        let mut cluster_count = 0usize;
        for &g in &self.coupled_groups {
            let r = root(&mut self.target_parents, groups[g].targets[0]);
            let cluster_index = match self.cluster_of_target[r] {
                usize::MAX => {
                    if cluster_count == self.cluster_targets.len() {
                        self.cluster_targets.push(vec![]);
                        self.cluster_groups.push(vec![]);
                    }
                    self.cluster_targets[cluster_count].clear();
                    self.cluster_groups[cluster_count].clear();
                    self.cluster_of_target[r] = cluster_count;
                    cluster_count += 1;
                    cluster_count - 1
                }
                c => c,
            };
            self.cluster_groups[cluster_index].push(g);
        }
        for k in 0..target_count {
            if self.cluster_of_target[root(&mut self.target_parents, k)] != usize::MAX {
                let cluster_index = self.cluster_of_target[root(&mut self.target_parents, k)];
                self.cluster_targets[cluster_index].push(k);
            }
        }
        for k in 0..target_count {
            self.in_cluster[k] = self.cluster_of_target[root(&mut self.target_parents, k)] != usize::MAX;
        }
        cluster_count
    }
}

/// `joint_eval` with its buffers from `sx`
pub(super) fn joint_eval_in(
    model: &Model,
    u: crate::UmaIndex,
    qs: &[f64],
    pre: &JointPrep,
    n_targets: usize,
    m1: bool,
    marg: bool,
    ext: Option<&mut JointExt>,
    sx: &mut JointScratch,
) -> Vec<(f64, f64, usize)> {
    evaluate_prepared(
        model,
        JointQuery { trainee: u, inheritance_chances: qs },
        pre,
        JointOptions { target_count: n_targets, all_but_one: m1, marginals: marg },
        ext,
        sx,
    )
}

fn evaluate_prepared(
    model: &Model,
    query: JointQuery,
    prepared: &JointPrep,
    options: JointOptions,
    mut extension: Option<&mut JointExt>,
    workspace: &mut JointScratch,
) -> Vec<(f64, f64, usize)> {
    let target_count = model.uncertain.len();
    let mut scratch = std::mem::take(workspace);
    let JointPrep { base, chains, states, groups } = prepared;
    let outfits = &model.opts[query.trainee];
    let outfit_count = outfits.len();
    let chance_count = query.inheritance_chances.len();
    let fixed_target_count = model.n_fixed_gold as usize;
    let fixed = match &extension {
        Some(x) => x.sf.iter().fold(1.0, |a, s| a * GEN_GOLD * s),
        None => model.fixed_factor(),
    };
    let want_sets = extension.as_ref().is_some_and(|x| !x.sets.is_empty()) && target_count <= 16;
    if let Some(x) = extension.as_deref_mut() {
        if options.marginals {
            x.marg = vec![0.0; chance_count * outfit_count * target_count];
        }
        if want_sets {
            x.sets = vec![0.0; (chance_count * outfit_count) << target_count];
        }
    }
    // (a scale of exactly 1.0 for every target leaves each factor as it is, x * 1.0 = x: not multiplied)
    let unit_scales = extension.as_ref().is_none_or(|x| x.su.iter().all(|&s| s == 1.0));
    let query_outfit_count = chance_count * outfit_count;
    scratch.reset(target_count, query_outfit_count, want_sets);
    let mut outcomes: Vec<&[(f64, &[(usize, u8)])]> = vec![];
    for state in states {
        let state_probability = state.probability;
        scratch.no_gold.copy_from_slice(&base.no_gold);
        scratch.no_hint.copy_from_slice(&base.no_hint);
        scratch.coupled_groups.clear();
        scratch.coupled_groups.extend(base.coupled.iter().copied());
        for (j, chain) in chains.iter().enumerate() {
            let factors = &chain.levels[state.classes[j] as usize];
            for k in 0..target_count {
                scratch.no_gold[k] *= factors.no_gold[k];
                scratch.no_hint[k] *= factors.no_hint[k];
            }
            scratch.coupled_groups.extend(factors.coupled.iter().copied());
        }
        for (qi, &q) in query.inheritance_chances.iter().enumerate() {
            for (o, (st, _)) in outfits.iter().enumerate() {
                let ew = &model.ews[query.trainee][o];
                let base = (qi * outfit_count + o) * target_count;
                for k in 0..target_count {
                    let lp = LandPre::new(st[k], ew[k], q);
                    scratch.landing_factors[base + k] =
                        [lp.f(scratch.no_gold[k], scratch.no_hint[k]), lp.f(scratch.no_gold[k], 0.0), lp.f(0.0, 0.0)];
                    if !unit_scales && let Some(x) = &extension {
                        for v in &mut scratch.landing_factors[base + k] {
                            *v *= x.su[k];
                        }
                    }
                }
            }
        }
        let cluster_count = scratch.build_clusters(groups, target_count);
        // per (q, outfit): (P(all land), P(exactly one misses)) of each independent unit, combined
        scratch.all_land.fill(1.0);
        scratch.one_misses.fill(0.0);
        let push = |all: &mut [f64], one: &mut [f64], i: usize, a: f64, b: f64| {
            one[i] = one[i] * a + all[i] * b;
            all[i] *= a;
        };
        for i in 0..query_outfit_count {
            for k in 0..target_count {
                if !scratch.in_cluster[k] {
                    let f = scratch.landing_factors[i * target_count + k][0];
                    push(&mut scratch.all_land, &mut scratch.one_misses, i, f, 1.0 - f);
                    if options.marginals
                        && let Some(x) = extension.as_deref_mut()
                    {
                        x.marg[i * target_count + k] += state_probability * f;
                    }
                }
            }
        }
        // the landed sets: independent targets first, clusters below (their targets join through `scratch.cluster_distribution`)
        if want_sets {
            for i in 0..query_outfit_count {
                let d = &mut scratch.landed_sets[i];
                d.clear();
                d.resize(1 << target_count, 0.0);
                d[0] = 1.0;
                for k in 0..target_count {
                    if scratch.in_cluster[k] {
                        continue;
                    }
                    let (f, b) = (scratch.landing_factors[i * target_count + k][0], 1usize << k);
                    for m in 0..(1usize << target_count) {
                        if m & b == 0 && d[m] != 0.0 {
                            let x = d[m];
                            d[m] = x * (1.0 - f);
                            d[m | b] += x * f;
                        }
                    }
                }
            }
        }
        for (targets, group_indices) in
            scratch.cluster_targets[..cluster_count].iter().zip(&scratch.cluster_groups[..cluster_count])
        {
            outcomes.clear();
            outcomes.extend(group_indices.iter().map(|&g| &*groups[g].outcomes));
            scratch.cluster_all_land.fill(0.0);
            scratch.cluster_one_misses.fill(0.0);
            // per (chance, outfit): distribution over the subsets of this cluster's targets (as full masks)
            if want_sets {
                let nsub = 1usize << targets.len();
                scratch.cluster_distribution.clear();
                scratch.cluster_distribution.resize(query_outfit_count * nsub, 0.0);
                scratch.cluster_masks.clear();
                scratch.cluster_masks.resize(nsub, 0);
            }
            scratch.outcome_indices.clear();
            scratch.outcome_indices.resize(outcomes.len(), 0);
            loop {
                let outcome_probability: f64 =
                    (0..outcomes.len()).map(|i| outcomes[i][scratch.outcome_indices[i]].0).product();
                for &k in targets {
                    scratch.hint_levels[k] = 0;
                }
                for i in 0..outcomes.len() {
                    for &(k, l) in outcomes[i][scratch.outcome_indices[i]].1 {
                        scratch.hint_levels[k] = scratch.hint_levels[k].max(if l >= 2 { 2 } else { 1 });
                    }
                }
                for i in 0..query_outfit_count {
                    let (mut a, mut b) = (1.0f64, 0.0f64);
                    for &k in targets {
                        let f = scratch.landing_factors[i * target_count + k][scratch.hint_levels[k] as usize];
                        b = b * f + a * (1.0 - f);
                        a *= f;
                    }
                    scratch.cluster_all_land[i] += outcome_probability * a;
                    scratch.cluster_one_misses[i] += outcome_probability * b;
                    if options.marginals
                        && let Some(x) = extension.as_deref_mut()
                    {
                        for &k in targets {
                            x.marg[i * target_count + k] += state_probability
                                * outcome_probability
                                * scratch.landing_factors[i * target_count + k][scratch.hint_levels[k] as usize];
                        }
                    }
                    if want_sets {
                        scratch.subsets.clear();
                        scratch.subsets.push((0, outcome_probability));
                        for &k in targets {
                            let f = scratch.landing_factors[i * target_count + k][scratch.hint_levels[k] as usize];
                            scratch.next_subsets.clear();
                            for &(m, p) in &scratch.subsets {
                                scratch.next_subsets.push((m, p * (1.0 - f)));
                                scratch.next_subsets.push((m | 1 << k, p * f));
                            }
                            std::mem::swap(&mut scratch.subsets, &mut scratch.next_subsets);
                        }
                        let ci = &mut scratch.cluster_distribution
                            [i * scratch.subsets.len()..(i + 1) * scratch.subsets.len()];
                        for (j, &(m, p)) in scratch.subsets.iter().enumerate() {
                            ci[j] += p;
                            scratch.cluster_masks[j] = m;
                        }
                    }
                }
                let mut i = 0;
                while i < outcomes.len() {
                    scratch.outcome_indices[i] += 1;
                    if scratch.outcome_indices[i] < outcomes[i].len() {
                        break;
                    }
                    scratch.outcome_indices[i] = 0;
                    i += 1;
                }
                if i == outcomes.len() {
                    break;
                }
            }
            for i in 0..query_outfit_count {
                push(
                    &mut scratch.all_land,
                    &mut scratch.one_misses,
                    i,
                    scratch.cluster_all_land[i],
                    scratch.cluster_one_misses[i],
                );
            }
            // (each entry m0 | m of a row is written once, m0 holding no target of this cluster: the order is free)
            if want_sets {
                let nsub = scratch.cluster_masks.len();
                for i in 0..query_outfit_count {
                    let d = &mut scratch.landed_sets[i];
                    scratch.next_distribution.clear();
                    scratch.next_distribution.resize(1 << target_count, 0.0);
                    for (m0, &x) in d.iter().enumerate() {
                        if x == 0.0 {
                            continue;
                        }
                        for (&m, &p) in
                            scratch.cluster_masks.iter().zip(&scratch.cluster_distribution[i * nsub..(i + 1) * nsub])
                        {
                            scratch.next_distribution[m0 | m] += x * p;
                        }
                    }
                    std::mem::swap(d, &mut scratch.next_distribution);
                }
            }
        }
        for i in 0..query_outfit_count {
            let (mut a, mut b) = (scratch.all_land[i], scratch.one_misses[i]);
            if options.all_but_one {
                for j in 0..fixed_target_count {
                    let g = match &extension {
                        Some(x) => GEN_GOLD * x.sf[j],
                        None => GEN_GOLD,
                    };
                    b = b * g + a * (1.0 - g);
                    a *= g;
                }
            } else {
                a *= fixed;
            }
            // P(at least options.target_count - 1 land) over target_count + fixed_target_count targets
            let m = if options.target_count <= target_count + fixed_target_count {
                if options.target_count == 0 { 1.0 } else { a + b }
            } else {
                0.0
            };
            scratch.totals[i].all += state_probability * a;
            scratch.totals[i].all_but_one += state_probability * if options.all_but_one { m } else { 0.0 };
            if want_sets && let Some(x) = extension.as_deref_mut() {
                for (m, &p) in scratch.landed_sets[i].iter().enumerate() {
                    x.sets[(i << target_count) + m] += state_probability * p;
                }
            }
        }
    }
    if let Some(x) = extension {
        x.per_outfit.clear();
        x.per_outfit.extend(scratch.totals.iter().map(|a| a.all));
    }
    let r = (0..chance_count)
        .map(|qi| {
            let mut b = 0;
            for o in 1..outfit_count {
                if scratch.totals[qi * outfit_count + o].all > scratch.totals[qi * outfit_count + b].all {
                    b = o;
                }
            }
            (scratch.totals[qi * outfit_count + b].all, scratch.totals[qi * outfit_count + b].all_but_one, b)
        })
        .collect();
    *workspace = scratch;
    r
}

/// Reference form of `joint_trainee`: every chain state, every combination of coupled outcomes. `JOINT_CHECK=1` runs
/// it beside the fast form on every call and panics on a relative difference above 1e-11.
pub(super) fn joint_trainee_ref(
    model: &Model,
    u: crate::UmaIndex,
    qs: &[f64],
    ents: &[(Src, Kind)],
    filler: usize,
    n_targets: usize,
) -> Vec<(f64, f64, usize)> {
    reference_joint(
        model,
        JointQuery { trainee: u, inheritance_chances: qs },
        ents,
        filler,
        JointOptions { target_count: n_targets, all_but_one: true, marginals: false },
    )
}

fn reference_joint(
    model: &Model,
    query: JointQuery,
    ents: &[(Src, Kind)],
    filler: usize,
    options: JointOptions,
) -> Vec<(f64, f64, usize)> {
    use crate::skills::GEN_WHITE;
    let nu = model.uncertain.len();
    // per-target miss factors (no gold, no hint) of the groups that touch one target; the others are "coupled"
    fn take<'group>(g: &'group Group, coupled: &mut Vec<&'group Group>, ng: &mut [f64], nh: &mut [f64]) {
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
    let opts = &model.opts[query.trainee];
    let no = opts.len();
    let nq = query.inheritance_chances.len();
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
            let mut v: Vec<(f64, &[(usize, u8)])> = g.iter().map(|(p, o)| (*p, (&**o))).collect();
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
            for (qi, &q) in query.inheritance_chances.iter().enumerate() {
                for (o, (st, _)) in opts.iter().enumerate() {
                    let ew = &model.ews[query.trainee][o];
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
                    let m1: f64 = dist[options.target_count.saturating_sub(1).min(nu + nfix)..=nu + nfix].iter().sum();
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

#[derive(Clone, Copy)]
struct MissFactors {
    no_gold: f64,
    no_hint: f64,
}

struct BoundTarget {
    target: usize,
    /// Cumulative misses after each number of completed chain links, starting at zero.
    by_progress: Vec<MissFactors>,
}

struct BoundChain {
    kind: Kind,
    targets: Vec<BoundTarget>,
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
pub(super) fn upper_na(
    model: &Model,
    trainee: crate::UmaIndex,
    inheritance_chances: &[f64],
    entries: &[(&Src, Kind)],
    filler: usize,
) -> Option<Vec<f64>> {
    let target_count = model.uncertain.len();
    // every group touching two or more targets must be negatively upper-orthant dependent, or the per-target
    // marginals below could understate a group whose outcomes hint targets together
    // (checked once per source, when it is built: `Src::not_nuod`)
    for (src, _) in entries {
        if src.not_nuod {
            if std::env::var("UPPER_WHY").is_ok()
                && let Some(g) = src
                    .fixed
                    .iter()
                    .chain(src.chains.iter().map(|x| &x.1))
                    .find(|g| group_gh(g).len() >= 2 && !group_nuod(g))
            {
                eprintln!("why: group {g:?}");
            }
            return None;
        }
    }
    let (mut base_no_gold, mut base_no_hint) = (vec![1.0f64; target_count], vec![1.0f64; target_count]);
    for (src, _) in entries {
        for &(k, p) in &src.train {
            base_no_hint[k] *= 1.0 - p;
        }
        // (Src::fm: the fixed groups' (k, 1 - P(gold), 1 - P(any hint)) in group order, as group_gh gives them: the same
        // factors in the same order, computed once per source)
        for &(k, mg, mh) in &src.fm {
            base_no_gold[k] *= mg;
            base_no_hint[k] *= mh;
        }
    }
    // chain cards in key order (fillers included): (kind, per target: cumulative (miss gold, miss any) after l links)
    let mut chains: Vec<BoundChain> = vec![];
    for (src, kind) in entries {
        if *kind == (0, 0, 0) {
            continue;
        }
        let stop = kind.1 as usize;
        let targets: Vec<BoundTarget> = src
            .cm
            .iter()
            .map(|(k, per)| {
                let mut cumulative = vec![MissFactors { no_gold: 1.0, no_hint: 1.0 }; stop + 1];
                for l in 1..=stop {
                    cumulative[l] = cumulative[l - 1];
                    for &(lk, g, h) in per {
                        if lk as usize == l {
                            cumulative[l].no_gold *= g;
                            cumulative[l].no_hint *= h;
                        }
                    }
                }
                BoundTarget { target: *k, by_progress: cumulative }
            })
            .collect();
        chains.push(BoundChain { kind: *kind, targets });
    }
    for _ in 0..filler {
        chains.push(BoundChain { kind: (2, 2, 0), targets: vec![] });
    }
    chains.sort_by_key(|chain| chain.kind);
    let kinds: Vec<Kind> = chains.iter().map(|chain| chain.kind).collect();
    // groups: cards joined by a target their chains both feed; at most one group of two cards, none larger. A target
    // that would join a larger group (three or more feeders, or two feeders other than the one pair) is "maxed": its
    // factor is taken at every feeder's full progress, a constant, and it leaves its feeders' target lists. Valid
    // (derived): per target phi = G(1 - ng) + W(ng - nh a) with a = (1 - e)(1 - q) in [0, 1] and G > W, so phi >= 0
    // (nh <= ng) and phi falls with ng and with nh; the miss products only fall as links progress (each link's factor
    // is at most 1), so f_k(L) <= f_k(full progress) on every chain state L, and E[prod_k f_k] <= E[prod_kept f_k] x
    // prod_maxed f_k^max; the kept targets then meet the structure the bound below needs.
    let mut feeders: Vec<Vec<usize>> = vec![vec![]; target_count];
    for (chain_index, chain) in chains.iter().enumerate() {
        for target in &chain.targets {
            feeders[target.target].push(chain_index);
        }
    }
    let mut pair: Option<(usize, usize)> = None;
    let mut maxed: Vec<Option<MissFactors>> = vec![None; target_count];
    for k in 0..target_count {
        let use_full_progress = match feeders[k].len() {
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
        if use_full_progress {
            let (mut g, mut h) = (base_no_gold[k], base_no_hint[k]);
            for &chain_index in &feeders[k] {
                #[allow(clippy::unwrap_in_result, reason = "feeders[k] lists only chains with target k (built above)")]
                let cumulative = &chains[chain_index].targets.iter().find(|row| row.target == k).unwrap().by_progress;
                let last = cumulative[cumulative.len() - 1];
                g *= last.no_gold;
                h *= last.no_hint;
            }
            maxed[k] = Some(MissFactors { no_gold: g, no_hint: h });
        }
    }
    if maxed.iter().any(std::option::Option::is_some) {
        for c in &mut chains {
            c.targets.retain(|row| maxed[row.target].is_none());
        }
        for k in 0..target_count {
            if maxed[k].is_some() {
                feeders[k].clear();
            }
        }
    }
    let fed: Vec<bool> = (0..target_count).map(|k| !feeders[k].is_empty()).collect();
    let chain_levels = if kinds.is_empty() { None } else { Some(key_level(&kinds)) };
    if let Some(k) = &chain_levels {
        let ok = match pair {
            None => k.nuod,
            Some((a, b)) => k.pair_ok(a, b),
        };
        if !ok {
            return None;
        }
    }
    let event_probabilities = event_pmf_job();
    let outfits = &model.opts[trainee];
    let fixed = model.fixed_factor();
    let phi = |o: usize, q: f64, k: usize, ng: f64, nh: f64| -> f64 {
        land_factor(outfits[o].0[k], model.ews[trainee][o][k], q, ng, nh)
    };
    let pair_marginals: Option<std::rc::Rc<Vec<Vec<Vec<f64>>>>> = match (pair, &chain_levels) {
        (Some((a, b)), Some(k)) => Some(k.pair_margs(a, b)),
        _ => None,
    };
    Some(
        inheritance_chances
            .iter()
            .map(|&q| {
                let mut best = 0.0f64;
                for o in 0..outfits.len() {
                    let mut other = 1.0;
                    for k in 0..target_count {
                        if let Some(MissFactors { no_gold: g, no_hint: h }) = maxed[k] {
                            other *= phi(o, q, k, g, h);
                        } else if !fed[k] {
                            other *= phi(o, q, k, base_no_gold[k], base_no_hint[k]);
                        }
                    }
                    // per single card and links done: the product over the targets only it feeds
                    let single = |chain_index: usize, l: usize| -> f64 {
                        chains[chain_index]
                            .targets
                            .iter()
                            .filter(|row| feeders[row.target].len() == 1)
                            .map(|row| {
                                phi(
                                    o,
                                    q,
                                    row.target,
                                    base_no_gold[row.target] * row.by_progress[l].no_gold,
                                    base_no_hint[row.target] * row.by_progress[l].no_hint,
                                )
                            })
                            .product()
                    };
                    let single_products: Vec<Vec<f64>> = (0..chains.len())
                        .map(|chain_index| {
                            (0..=chains[chain_index].kind.1 as usize).map(|l| single(chain_index, l)).collect()
                        })
                        .collect();
                    // the pair's group: both cards' single targets and the shared ones, over (la, lb)
                    let pair_products: Option<Vec<Vec<f64>>> = pair.map(|(a, b)| {
                        (0..=chains[a].kind.1 as usize)
                            .map(|la| {
                                (0..=chains[b].kind.1 as usize)
                                    .map(|lb| {
                                        let mut v = single_products[a][la] * single_products[b][lb];
                                        for target in &chains[a].targets {
                                            let k = target.target;
                                            let cum_a = &target.by_progress;
                                            if feeders[k].len() == 2 {
                                                let cum_b = &chains[b]
                                                    .targets
                                                    .iter()
                                                    .find(|row| row.target == k)
                                                    .unwrap()
                                                    .by_progress;
                                                v *= phi(
                                                    o,
                                                    q,
                                                    k,
                                                    base_no_gold[k] * cum_a[la].no_gold * cum_b[lb].no_gold,
                                                    base_no_hint[k] * cum_a[la].no_hint * cum_b[lb].no_hint,
                                                );
                                            }
                                        }
                                        v
                                    })
                                    .collect()
                            })
                            .collect()
                    });
                    let mut total = 0.0;
                    for (m, &event_probability) in event_probabilities.iter().enumerate() {
                        if event_probability == 0.0 {
                            continue;
                        }
                        let mut product = other;
                        if let Some(k) = &chain_levels {
                            for chain_index in 0..chains.len() {
                                if chains[chain_index].targets.is_empty()
                                    || pair.is_some_and(|(a, b)| chain_index == a || chain_index == b)
                                {
                                    continue;
                                }
                                product *= k.marg[chain_index][m]
                                    .iter()
                                    .zip(&single_products[chain_index])
                                    .map(|(p, c)| p * c)
                                    .sum::<f64>();
                            }
                            if let (Some(pair_values), Some(pair_rows)) = (&pair_products, &pair_marginals) {
                                product *= pair_rows[m]
                                    .iter()
                                    .zip(pair_values)
                                    .map(|(r, s)| r.iter().zip(s).map(|(p, c)| p * c).sum::<f64>())
                                    .sum::<f64>();
                            }
                        }
                        total += event_probability * product;
                    }
                    best = best.max(total);
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
        Self { st, ne: 1.0 - ew, omq: 1.0 - q }
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
    u: crate::UmaIndex,
    qs: &[f64],
    ents: &[(&Src, Kind)],
    filler: usize,
) -> Option<Vec<f64>> {
    if ents.iter().any(|e| e.0.multi) {
        return None;
    }
    Some(state_value_any(model, u, qs, ents, filler))
}
/// `state_value` for the decks the cap-free search covers (`Src::cf_ok`: every group's levels negatively upper-orthant
/// dependent, `group_nuod_lvl`, all the derivation above needs of a group); None otherwise. The same value where both
/// apply.
pub(super) fn state_value_cf(
    model: &Model,
    u: crate::UmaIndex,
    qs: &[f64],
    ents: &[(&Src, Kind)],
    filler: usize,
) -> Option<Vec<f64>> {
    if ents.iter().any(|e| !e.0.cf_ok) {
        return None;
    }
    Some(state_value_any(model, u, qs, ents, filler))
}
fn state_value_any(model: &Model, u: crate::UmaIndex, qs: &[f64], ents: &[(&Src, Kind)], filler: usize) -> Vec<f64> {
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
    (0..qs.len()).map(|qi| fixed * (0..no).map(|o| acc[qi * no + o]).fold(0.0f64, f64::max)).collect()
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
