//! Legal card-count configurations and their bounds.
use super::SearchControl;
use crate::deck::space::{EntryMisses, GroupTableRequest};
use crate::deck::{DeckSpace, GTab, cap_class, gm_g, gm_w};

/// Legal groups and key counts used to enumerate configurations, in their original order.
pub(in crate::deck::enumerate) struct ConfigurationTables<'tab> {
    pub(in crate::deck::enumerate) counts: &'tab [Vec<usize>],
    pub(in crate::deck::enumerate) by_kind: &'tab [Vec<usize>],
    pub(in crate::deck::enumerate) pal_ents: &'tab [usize],
    pub(in crate::deck::enumerate) kpos: &'tab [u64],
    pub(in crate::deck::enumerate) groups_all: &'tab [&'tab Vec<usize>],
    pub(in crate::deck::enumerate) glk: &'tab [u8],
    pub(in crate::deck::enumerate) gcvm: &'tab [u64],
    pub(in crate::deck::enumerate) ki22: usize,
    pub(in crate::deck::enumerate) nplain: usize,
    pub(in crate::deck::enumerate) all_lk: u8,
    pub(in crate::deck::enumerate) keyskip: bool,
}

/// Configuration-build buffers stay owned by preparation; white/gold odds are reused by traversal.
pub(in crate::deck::enumerate) struct ConfigurationScratch<'scratch> {
    pub(in crate::deck::enumerate) dw: &'scratch mut [f64],
    pub(in crate::deck::enumerate) dg: &'scratch mut [f64],
    pub(in crate::deck::enumerate) groups: &'scratch mut Vec<(usize, usize)>,
    pub(in crate::deck::enumerate) clause_bounds: &'scratch mut [f64],
    pub(in crate::deck::enumerate) order_bounds: &'scratch mut [f64],
    pub(in crate::deck::enumerate) ratio_products: &'scratch mut [f64],
    pub(in crate::deck::enumerate) key_white: &'scratch mut [f64],
    pub(in crate::deck::enumerate) key_gold: &'scratch mut [f64],
}

// every (key, real (2, 2) entries, kind-0 entries) with its groups and root bound; visited best bound first
pub(in crate::deck::enumerate) struct SearchConfig {
    pub(in crate::deck::enumerate) root: f64,
    pub(in crate::deck::enumerate) code: u64,
    pub(in crate::deck::enumerate) code_real22: usize,
    pub(in crate::deck::enumerate) np: usize,
    pub(in crate::deck::enumerate) filler: usize,
    pub(in crate::deck::enumerate) plain: usize,
    pub(in crate::deck::enumerate) gs: Vec<(usize, usize)>,
    pub(in crate::deck::enumerate) gm: std::rc::Rc<GTab>,
    pub(in crate::deck::enumerate) rpk: std::rc::Rc<GTab>,
    // A tighter bound may prune without changing the root-bound visit order.
    pub(in crate::deck::enumerate) prune_bound: f64,
    // Root clause values retained only for RB_CHECK.
    pub(in crate::deck::enumerate) clause_bounds: Vec<f64>,
}

// A configuration passing the per-target bound, its root not yet computed: `u` is an upper bound on its root
// (the root sums fixed * max over outfits of min(A, B r) per (trainee, chance), the per-target value fixed *
// the sum of max A, the same A to the bit; terms >= 0, so the root is at most the value up to rounding);
// `key`: its key's slot in `pkeys` (the key's ratio tables, built on first need)
pub(in crate::deck::enumerate) struct PendingConfig {
    pub(in crate::deck::enumerate) u: f64,
    pub(in crate::deck::enumerate) key: usize,
    pub(in crate::deck::enumerate) code: u64,
    pub(in crate::deck::enumerate) real22: usize,
    pub(in crate::deck::enumerate) np: usize,
    pub(in crate::deck::enumerate) filler: usize,
    pub(in crate::deck::enumerate) plain: usize,
    pub(in crate::deck::enumerate) gneed: u64,
    pub(in crate::deck::enumerate) gs: Vec<(usize, usize)>,
    pub(in crate::deck::enumerate) gm: std::rc::Rc<GTab>,
}

/// Eager configurations retain their stable root order; pending configurations retain
/// insertion order until traversal builds its stable upper-bound ordering.
pub(in crate::deck::enumerate) struct ConfigurationQueue {
    pub(in crate::deck::enumerate) cfgs: Vec<SearchConfig>,
    pub(in crate::deck::enumerate) pend: Vec<PendingConfig>,
    pub(in crate::deck::enumerate) pkeys: Vec<Option<std::rc::Rc<GTab>>>,
}

/// The same bound operations serve eager configuration construction and lazy queue expansion.
pub(in crate::deck::enumerate) struct ConfigurationBounds<Ratios, Odds, Root> {
    pub(in crate::deck::enumerate) ratio_prefixes: Ratios,
    pub(in crate::deck::enumerate) odds: Odds,
    pub(in crate::deck::enumerate) root: Root,
}

impl DeckSpace<'_> {
    /// Enumerate legal card-count configurations, preserving stable root-bound order and
    /// deferring ratio tables for the product search until traversal actually needs them.
    pub(in crate::deck::enumerate) fn build_configurations(
        &self,
        clauses: &[(Vec<(usize, f64)>, f64)],
        controls: SearchControl,
        tables: ConfigurationTables<'_>,
        scratch: ConfigurationScratch<'_>,
        bestv: f64,
        clause_value: impl Fn(&(Vec<(usize, f64)>, f64), &[f64], &[f64]) -> f64,
        bounds: &ConfigurationBounds<
            impl Fn(&[f64], &[f64], &[u32], u64) -> GTab,
            impl Fn(&[(usize, usize)], &GTab, &mut [f64], &mut [f64]),
            impl Fn(&[(usize, usize)], &GTab, &[f64], &[f64], &mut [f64], &mut [f64], &mut [f64]),
        >,
    ) -> ConfigurationQueue {
        let SearchControl {
            best,
            rho,
            target_count,
            size,
            kind_count,
            set_id,
            fixed,
            cap_free,
            class_caps,
            lazy,
            check_ratio_bounds,
            ..
        } = controls;
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
            if (0..kind_count).any(|i| i != ki22 && cnt[i] > by_kind[i].len()) {
                continue;
            }
            let mut pkey_new = true;
            let code: u64 = cnt.iter().zip(kpos).map(|(&c, &p)| c as u64 * p).sum();
            let km = self.key_miss(code);
            let (mw, mg) = (&km.0, &km.1);
            // per kind group and target: products of the r lowest miss chances of distinct entries, r = 0..=size
            // (any r entries of the group miss at least that much); cached per trainee and key
            // (the groups a configuration of this key can use: its kinds and, with slots left, the pal entries)
            let gneed: u64 = (0..kind_count).filter(|&i| cnt[i] > 0).fold(0u64, |m, i| m | 1 << i)
                | if csum < size { 1 << kind_count } else { 0 };
            let gm = self.gm_cached(
                GroupTableRequest { set_id, code, groups: groups_all, needed_groups: gneed, deck_size: size },
                EntryMisses { white: mw, gold: mg, target_count },
            );
            // the key's bound: no variant (real (2, 2) and pal entries counted 0 ..= their most) can pass when the value
            // at odds above every variant's fails with room for rounding (`key_slack`)
            if keyskip {
                let (r22, npm) = (cnt[ki22].min(by_kind[ki22].len()), (size - csum).min(pal_ents.len()));
                let lo = |i: usize, c: usize, t: usize| -> (f64, f64) {
                    let gb = gm.grp(i);
                    (0..=c).fold((f64::INFINITY, f64::INFINITY), |m, x| {
                        (m.0.min(gm_w(gb, target_count, x)[t]), m.1.min(gm_g(gb, target_count, x)[t]))
                    })
                };
                for t in 0..target_count {
                    let (mut a, mut b) = (1.0f64, 1.0f64);
                    if npm > 0 {
                        let x = lo(kind_count, npm, t);
                        a *= x.0;
                        b *= x.1;
                    }
                    for i in (0..kind_count).rev() {
                        let c = if i == ki22 { r22 } else { cnt[i] };
                        if c == 0 {
                            continue;
                        }
                        let x = if i == ki22 {
                            lo(i, c, t)
                        } else {
                            let gb = gm.grp(i);
                            (gm_w(gb, target_count, c)[t], gm_g(gb, target_count, c)[t])
                        };
                        a *= x.0;
                        b *= x.1;
                    }
                    kdw[t] = 1.0 - a;
                    kdg[t] = 1.0 - b;
                }
                // (a band's class caps are at most rho)
                let none = if best {
                    (clause_value(&clauses[0], kdw, kdg) + key_slack(&clauses[0])) * (1.0 + 1e-9) < bestv
                } else {
                    clauses.iter().all(|cl| rho * (clause_value(cl, kdw, kdg) + key_slack(cl)) * (1.0 + 1e-9) < cl.1)
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
                    for i in 0..kind_count {
                        let c = if i == ki22 { real22 } else { cnt[i] };
                        if c > 0 {
                            gs.push((i, c));
                        }
                    }
                    if np > 0 {
                        gs.push((kind_count, np));
                    }
                    let gs = &**gsb;
                    if all_lk & !gs.iter().fold(0u8, |m, &(i, _)| m | glk[i]) != 0 {
                        continue;
                    }
                    // the per-target bound first, the ratio products only for configurations it lets through
                    (bounds.odds)(gs, &gm, dw, dg);
                    // the configuration's class cap (every deck of it is in a class at most this one's)
                    let rc =
                        if best { rho } else { class_caps[cap_class(gs.iter().fold(0u64, |m, &(i, _)| m | gcvm[i]))] };
                    let iv = if best { clause_value(&clauses[0], dw, dg) } else { 0.0 };
                    let ideal = if cap_free {
                        true
                    } else if best {
                        iv * (1.0 + 1e-9) >= bestv
                    } else {
                        clauses.iter().any(|cl| rc * clause_value(cl, dw, dg) * (1.0 + 1e-9) >= cl.1)
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
                        rpk = Some(std::rc::Rc::new((bounds.ratio_prefixes)(mw, mg, &km.2, gneed)));
                    }
                    let rpk = rpk.as_ref().unwrap();
                    let (cv, cvo) = (&mut *cvr, &mut *cvro);
                    (bounds.root)(gs, rpk, dw, dg, sp0, cv, cvo);
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
                        && !cap_free
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
                        clause_bounds: if check_ratio_bounds { cv.to_vec() } else { vec![] },
                    });
                }
            }
        }
        // stable: equal bounds keep enumeration order
        cfgs.sort_by(|a, b| b.root.partial_cmp(&a.root).unwrap());
        ConfigurationQueue { cfgs, pend, pkeys }
    }
}
