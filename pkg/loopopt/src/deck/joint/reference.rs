//! The reference evaluation of a deck's joint landing odds (`joint_trainee_ref`): every chain state and every
//! combination of coupled outcomes, without the collapsing `exact` does. Setting `JOINT_CHECK` compares each exact
//! scoring with it; outside that check only tests call it.
use crate::deck::chains::{ChainKind, ChainState, chain_states};
use crate::deck::joint::{ChainEntry, JointChances, JointOptions, JointQuery};
use crate::deck::sources::{
    ChainOutcome, EventOutcome, Group, SourceEntry, TargetReward, TargetSources, TrainingHint, group_gh,
};
use crate::skills::{GEN_GOLD, Model, OutfitOption};
/// Reference form of `joint_b`: every chain state, every combination of coupled outcomes. Setting `JOINT_CHECK` runs
/// it in `joint_checked` and panics on a relative difference at or above 1e-11.
pub(in crate::deck) fn joint_trainee_ref(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<TargetSources>],
    filler: usize,
    requested_target_count: usize,
) -> Vec<JointChances> {
    reference_joint(
        model,
        query,
        entries,
        filler,
        JointOptions { target_count: requested_target_count, all_but_one: true, marginals: false },
    )
}

fn reference_joint(
    model: &Model,
    query: JointQuery,
    entries: &[SourceEntry<TargetSources>],
    filler: usize,
    options: JointOptions,
) -> Vec<JointChances> {
    use crate::skills::GEN_WHITE;
    let uncertain_count = model.uncertain.len();
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
    let (mut bg, mut bh) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count]);
    let mut bc: Vec<&Group> = vec![];
    for SourceEntry { sources: src, .. } in entries {
        for &TrainingHint { target: k, probability: p } in &src.training_hints {
            bh[k] *= 1.0 - p;
        }
        for g in &src.fixed_outcomes {
            take(g, &mut bc, &mut bg, &mut bh);
        }
    }
    // chain entries sorted by kind (with fillers) for the memoised state distribution; per entry and links done,
    // the cumulative factors and coupled groups of the links fired so far
    let mut chain_entries: Vec<ChainEntry> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.kind != ChainKind::NONE)
        .map(|(i, e)| ChainEntry { kind: e.kind, source_index: Some(i) })
        .collect();
    for _ in 0..filler {
        chain_entries.push(ChainEntry { kind: ChainKind::FILLER, source_index: None });
    }
    chain_entries.sort_by(|a, b| a.kind.cmp(&b.kind).then(a.source_index.cmp(&b.source_index)));
    let kinds: Vec<ChainKind> = chain_entries.iter().map(|x| x.kind).collect();
    let states = if kinds.is_empty() {
        std::rc::Rc::new(vec![ChainState { progress: vec![], probability: 1.0 }])
    } else {
        chain_states(&kinds)
    };
    let lv: Vec<Option<Vec<(Vec<f64>, Vec<f64>, Vec<&Group>)>>> = chain_entries
        .iter()
        .map(|&ChainEntry { kind: kd, source_index: e }| {
            e.map(|i| {
                let src = &entries[i].sources;
                let (mut g, mut h, mut c) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count], vec![]);
                let mut out = vec![(g.clone(), h.clone(), c.clone())];
                for l in 1..=kd.stop {
                    for ChainOutcome { link: cl, outcomes: grp } in &src.chain_outcomes {
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
    let outfit_count = opts.len();
    let chance_count = query.inheritance_chances.len();
    let nfix = model.n_fixed_gold as usize;
    let fixed = model.fixed_factor();
    let mut acc = vec![(0.0f64, 0.0f64); chance_count * outfit_count];
    let (mut ng, mut nh) = (vec![1.0f64; uncertain_count], vec![1.0f64; uncertain_count]);
    let mut coupled: Vec<&Group> = vec![];
    let mut alts: Vec<Vec<EventOutcome<&[TargetReward]>>> = vec![];
    let mut pick: Vec<usize> = vec![];
    let mut dist = [0.0f64; 40];
    for ChainState { progress: prog, probability: w } in states.iter() {
        ng.copy_from_slice(&bg);
        nh.copy_from_slice(&bh);
        coupled.clear();
        coupled.extend(bc.iter().copied());
        for (pos, l) in lv.iter().enumerate() {
            let Some(l) = l else { continue };
            let (g, h, c) = &l[prog[pos] as usize];
            for k in 0..uncertain_count {
                ng[k] *= g[k];
                nh[k] *= h[k];
            }
            coupled.extend(c.iter().copied());
        }
        // outcomes of the coupled groups: each group's outcomes plus "nothing" (the rest of its weight)
        alts.clear();
        for g in &coupled {
            let mut v: Vec<EventOutcome<&[TargetReward]>> = g
                .iter()
                .map(|outcome| EventOutcome { probability: outcome.probability, rewards: &*outcome.rewards })
                .collect();
            let rest = 1.0 - g.iter().map(|x| x.probability).sum::<f64>();
            if rest > 1e-15 {
                v.push(EventOutcome { probability: rest, rewards: &[][..] });
            }
            alts.push(v);
        }
        pick.clear();
        pick.resize(alts.len(), 0);
        loop {
            let wc: f64 = (0..alts.len()).map(|i| alts[i][pick[i]].probability).product::<f64>() * w;
            let mut fg = [false; 32];
            let mut fh = [false; 32];
            for i in 0..alts.len() {
                for &TargetReward { target: k, level: lvl } in alts[i][pick[i]].rewards {
                    fh[k] = true;
                    if lvl >= 2 {
                        fg[k] = true;
                    }
                }
            }
            for (qi, &q) in query.inheritance_chances.iter().enumerate() {
                for (o, OutfitOption { coverage: st, .. }) in opts.iter().enumerate() {
                    let ew = &query.event_chances[o];
                    // P(all land) and the distribution of how many of the uncertain targets land (fixed golds after)
                    let mut all = fixed;
                    dist[..=uncertain_count + nfix].iter_mut().for_each(|x| *x = 0.0);
                    dist[0] = 1.0;
                    for k in 0..uncertain_count {
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
                        for j in (0..=uncertain_count + nfix).rev() {
                            if j + 1 < dist.len() {
                                dist[j + 1] += dist[j] * GEN_GOLD;
                            }
                            dist[j] *= 1.0 - GEN_GOLD;
                        }
                    }
                    let m1: f64 = dist
                        [options.target_count.saturating_sub(1).min(uncertain_count + nfix)..=uncertain_count + nfix]
                        .iter()
                        .sum();
                    let a = &mut acc[qi * outfit_count + o];
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
    (0..chance_count)
        .map(|qi| {
            let mut b = 0;
            for o in 1..outfit_count {
                if acc[qi * outfit_count + o].0 > acc[qi * outfit_count + b].0 {
                    b = o;
                }
            }
            JointChances { all: acc[qi * outfit_count + b].0, all_but_one: acc[qi * outfit_count + b].1, outfit: b }
        })
        .collect()
}
