//! Per-uma decks (`run_multi`): the per-uma tables of exact values by inheritance chance and the loop search over them.
use super::enumerate::{SearchMode, SearchOutcome};
use super::{
    Card, CardsRaw, Choice, Cl, Data, DeckEval, DeckId, DeckJob, DeckSpace, EXACT_BUDGET, EXACT_RHO, EXACT_STEPS,
    GEN_GOLD, Instant, JointExt, Kind, Model, NONE, NT, Name, PDeck, Res, SPT, Sched, SkillJob, Sources, Src, UCHK,
    Value, chart_rows, check_sizes, joint, joint_b, joint_fast, json, k_filter, lins, member_json, members,
    optimize_fam, pair_ceilings, prebuilt, probs, split, src_class, upper_na, wval,
};

/// Per-uma decks: each trainee runs her own best legal deck, the same one in all the lineages she heads (three in a
/// four-uma loop, one in a three-uma rotation).
///   value(f) = fixed/L * sum_t max over decks D legal for f[t] of sum over her lineages l of T(f[t], q_l, D), L = 12 or 3
/// The skyline (decks no deck beats on every coordinate) holds every uma's best deck as long as none of its decks
/// carries a card that blocks her; for a uma it does, her legal decks come from a second deck search without
/// those cards, run the first time a foursome with her is solved. Bounds use the whole skyline (legal or not),
/// which no legal deck beats, so the search is exact.
pub(super) fn run_multi(
    d: &Data,
    cr: &CardsRaw,
    model: &Model,
    sj: &SkillJob,
    dj: &DeckJob,
    sky: Vec<PDeck>,
    stats: Value,
    space: &DeckSpace,
) -> Value {
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    // (looked up only, never iterated: the hasher changes no order)
    let card_of: crate::FxMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let n = d.umas.len();
    let id_ok = |id: u32| {
        (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id)
    };
    // a locked card goes in every deck, so a uma it blocks cannot train at all
    let lock_blocked =
        |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).is_some_and(|c| c.blocks(d.umas[u].id)));
    let pool: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    let req: Vec<usize> = sj.require.iter().filter_map(|&id| d.umas.iter().position(|u| u.id == id)).collect();
    if req.iter().any(|r| !pool.contains(r)) {
        return json!({"error": "A required uma cannot train with a locked card of her own."});
    }
    if let Err(e) = check_sizes(&sj.sizes, pool.len(), req.len()) {
        return json!({"error": e});
    }
    let (with3, with4) = (sj.sizes.contains(&3), sj.sizes.contains(&4));
    let tq = Instant::now();
    let fixed_s: Option<Vec<Option<Sched>>> = dj
        .schedule
        .as_deref()
        .map(|mode| (0..n).map(|i| pool.contains(&i).then(|| prebuilt(d, &d.umas[i], mode))).collect());
    let no_race = dj.schedule.as_deref() == Some("none");
    let pb = match &fixed_s {
        Some(fs) => {
            let ps: Vec<Option<Vec<f64>>> =
                fs.iter().enumerate().map(|(i, s)| s.as_ref().map(|s| probs(d, &d.umas[i], s))).collect();
            let mut m = vec![vec![0.0f64; n]; n];
            if !no_race {
                for &a in &pool {
                    for &b in &pool {
                        if a != b {
                            m[a][b] = wval(ps[a].as_ref().unwrap(), ps[b].as_ref().unwrap());
                        }
                    }
                }
            }
            m
        }
        None => pair_ceilings(d, &pool),
    };
    let k = pool.len();
    // Structures: four-uma loops (12 lineages) and three-uma rotations (3 lineages; `lins`), members as [usize; 4]
    // with `NONE` last for three; a structure's value is the mean over its lineages, so both sizes rank together.
    let lin_of = |f: &[usize; 4]| lins(members(f).len());
    let fixed = model.fixed_factor();
    let qmax: Vec<f64> =
        (0..n).map(|u| if pool.contains(&u) { model.q_ub_trainee(u, &pool, &pb, with3) } else { 0.0 }).collect();
    let qlo = model.q_floor();
    let qhi = pool.iter().map(|&u| qmax[u]).fold(qlo, f64::max);
    const NG: usize = 64;
    let step = if qhi > qlo { (qhi - qlo) / (NG - 1) as f64 } else { 0.0 };
    let grid: Vec<f64> = (0..NG).map(|i| if i == NG - 1 { qhi } else { qlo + step * i as f64 }).collect();
    let gidx = |q: f64| -> usize {
        if step == 0.0 {
            return NG - 1;
        }
        let mut i = (((q - qlo) / step).ceil().max(0.0) as usize).min(NG - 1);
        while grid[i] < q && i < NG - 1 {
            i += 1;
        }
        i
    };
    let zero = vec![0.0f64; nu];
    // A uma's value depends only on the targets some outfit of hers lacks: (g, h) = (P(gold), P(any hint)) where
    // an outfit lacks the skill, g alone where her outfits have at most the white. Decks are thinned per class of
    // umas with the same coordinates: a deck another beats on all of them never wins for those umas.
    let ucoords = |u: usize| -> Vec<(usize, bool)> {
        let mut c = vec![];
        for k in 0..nu {
            if model.opts[u].iter().any(|(st, _)| st[k] == 0) {
                c.push((k, true));
                c.push((k, false));
            } else if model.opts[u].iter().any(|(st, _)| st[k] == 1) {
                c.push((k, true));
            }
        }
        c
    };
    let thin = |list: &[PDeck], co: &[(usize, bool)]| -> Vec<usize> {
        if list.is_empty() {
            return vec![];
        }
        if co.is_empty() {
            return vec![0];
        }
        let tagged: Vec<DeckEval> = list
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let dv: Vec<f64> = co
                    .iter()
                    .map(|&(k, g)| if g { x.dg[k] } else { 1.0 - (1.0 - x.dw[k]) * (1.0 - x.dg[k]) })
                    .collect();
                let s = dv.iter().sum();
                DeckEval {
                    pals: 0,
                    borrowed: None,
                    cards: vec![i],
                    ends: vec![],
                    vars: vec![],
                    plain: 0,
                    filler: 0,
                    dw: vec![],
                    dg: vec![],
                    dv,
                    s,
                }
            })
            .collect();
        let mut v: Vec<usize> = k_filter(tagged, 1, co.len()).into_list().into_iter().map(|y| y.cards[0]).collect();
        v.sort_unstable();
        v
    };
    // the full coordinate set needs no thinning: the skyline is already thin on it
    let full: Vec<(usize, bool)> = (0..nu).flat_map(|k| [(k, true), (k, false)]).collect();
    let mut class_ix: crate::FxMap<Vec<(usize, bool)>, usize> = Default::default();
    let mut class_sky: Vec<Vec<usize>> = vec![];
    let mut ucls: Vec<usize> = vec![usize::MAX; n];
    for &u in &pool {
        let co = ucoords(u);
        let ci = if let Some(&c) = class_ix.get(&co) {
            c
        } else {
            class_sky.push(if co == full { (0..sky.len()).collect() } else { thin(&sky, &co) });
            class_ix.insert(co, class_sky.len() - 1);
            class_sky.len() - 1
        };
        ucls[u] = ci;
    }
    let uview = |u: usize| -> &Vec<usize> { &class_sky[ucls[u]] };
    // Clusters of each class's decks (recursive median splits on the widest odds coordinate) with their
    // componentwise maximum odds. T(u, q, deck) rises with every deck odds coordinate (for 0 <= q <= 1, since a
    // white is worth less than a gold), so T at a cluster's maximum bounds every member; maxima over a class are
    // found cluster by cluster, best bound first, skipping clusters whose bound (with 1e-9 relative slack for
    // rounding) is below the best value so far. The maximum of a set does not depend on visit order, so the
    // result is the same value bit for bit.
    let class_cl: Vec<Vec<Cl>> = class_sky
        .iter()
        .map(|v| {
            let mut ix = v.clone();
            let mut out = vec![];
            if nu > 0 && !ix.is_empty() {
                split(&sky, &mut ix, nu, &mut out);
            }
            out
        })
        .collect();
    // max(init, max over u's class of val(deck)), with bound(cluster maxima) >= val of every member
    // (the clusters' bounds in a reused buffer, popped from a heap in the stable sort's order: `ByBound`)
    let bb_buf: std::cell::RefCell<Vec<ByBound>> = Default::default();
    let bb_max = |u: usize, init: f64, val: &dyn Fn(&[f64], &[f64]) -> f64| -> f64 {
        let cls = &class_cl[ucls[u]];
        if cls.is_empty() {
            return uview(u).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max);
        }
        let mut bs = std::mem::take(&mut *bb_buf.borrow_mut());
        bs.clear();
        bs.extend(cls.iter().enumerate().map(|(i, c)| ByBound { bound: val(&c.mdw, &c.mdg), cluster: i }));
        let mut h = std::collections::BinaryHeap::from(bs);
        let mut best = init;
        while let Some(ByBound { bound: b, cluster: i }) = h.pop() {
            if b * (1.0 + 1e-9) < best {
                break;
            }
            for &j in &cls[i].mem {
                best = best.max(val(&sky[j].dw, &sky[j].dg));
            }
        }
        *bb_buf.borrow_mut() = h.into_vec();
        best
    };
    if std::env::var("DECK_DEBUG").is_ok() {
        eprintln!(
            "classes {} sizes {:?} ({:.2}s)",
            class_sky.len(),
            class_sky.iter().map(std::vec::Vec::len).collect::<Vec<_>>(),
            tq.elapsed().as_secs_f64()
        );
    }
    // B[u][i] = max over all skyline decks (legal for u or not) of T(u, grid[i], deck): no legal deck beats it
    let btab_c: std::cell::RefCell<Vec<[f64; NG]>> = std::cell::RefCell::new(vec![[f64::NAN; NG]; n]);
    let btab = |u: usize, i: usize| -> f64 {
        let v = btab_c.borrow()[u][i];
        if !v.is_nan() {
            return v;
        }
        let q = grid[i];
        let init = model.trainee_fast(u, q, &zero, &zero);
        let v = if (0.0..=1.0).contains(&q) {
            bb_max(u, init, &|dw, dg| model.trainee_fast(u, q, dw, dg))
        } else {
            uview(u).iter().map(|&j| model.trainee_fast(u, q, &sky[j].dw, &sky[j].dg)).fold(init, f64::max)
        };
        btab_c.borrow_mut()[u][i] = v;
        v
    };
    // ---- exact per-uma values ----
    // Loops are ranked by exact odds. A trainee's exact value at her lineages' inheritance chances is the best exact
    // P(all) over every legal deck (`DeckSpace`, not only the skyline). It is seeded with her best legal skyline decks
    // by product odds; then every legal deck whose product value times EXACT_RHO reaches the best exact value is scored
    // exactly. That is exact under one assumption: no deck's exact odds exceed EXACT_RHO times its product odds.
    let srcb = Sources { dj, unc: &unc };
    let ntar = sj.targets.len();
    // A deck's sources: the deck space's entry for (card, end link, variant) where it has one of that card (built by
    // the same `Sources::src` and `kind` on the same job and targets, so the same values), borrowed; else built
    let ent_ix: crate::FxMap<(u32, u8, u32), usize> =
        (0..space.rel.len()).map(|j| ((space.cards[space.rel[j]].id, space.endk[j], space.vark[j]), j)).collect();
    let ents_of = |x: &PDeck| -> (Vec<(std::borrow::Cow<Src>, Kind)>, usize) {
        let e: Vec<(std::borrow::Cow<Src>, Kind)> = x
            .ids
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                card_of.get(id).map(|c| {
                    let st = x.ends.get(i).copied().filter(|&e| e > 0);
                    let v = x.vars.get(i).copied().unwrap_or(0);
                    match ent_ix.get(&(*id, st.unwrap_or(0), v)) {
                        Some(&j) if std::ptr::eq(space.cards[space.rel[j]], *c) => {
                            (std::borrow::Cow::Borrowed(&space.srcs[j]), space.kind[j])
                        }
                        _ => (std::borrow::Cow::Owned(srcb.src(c, st, v)), srcb.kind(c, st)),
                    }
                })
            })
            .collect();
        (e, dj.deck_size.saturating_sub(x.ids.len() + x.plain_ids.len()))
    };
    struct Best {
        sum: f64,
        deck: Option<PDeck>,
        per: Vec<(f64, f64, usize)>,
    }
    // per (uma, chances): the best found, decks already scored or bounded, and the cap it is exact under
    struct ExactCacheEntry {
        best: Option<(f64, PDeck)>,
        /// Detached decks already scored exactly (including skyline seeds).
        scored_decks: crate::FxSet<DeckId>,
        /// Band selections already scored or rejected by the proven bound.
        checked_selections: crate::FxSet<(Vec<usize>, usize, usize)>,
        /// Highest completed cap; -1 means unseeded, 1 means seeded only.
        certified_cap: f64,
        reported_best: Option<std::rc::Rc<Best>>,
    }
    let ex_cache: std::cell::RefCell<crate::FxMap<(usize, QKey), ExactCacheEntry>> = Default::default();
    let n_scored = std::cell::Cell::new(0usize);
    let n_band = std::cell::Cell::new(0usize);
    // (per structure class, `cap_class`)
    let cap_hit = std::cell::Cell::new(false);
    let max_ratio = std::cell::Cell::new([0.0f64; 4]);
    let caps = space.caps();
    let rho_top = caps[3];
    let max_deck: std::cell::RefCell<[Option<(u32, Vec<Name>)>; 4]> = Default::default();
    let ratio_at = |k: usize, r: f64, u: u32, x: &PDeck| {
        let mut m = max_ratio.get();
        if r > m[k] {
            m[k] = r;
            max_ratio.set(m);
            max_deck.borrow_mut()[k] = Some((u, x.names.clone()));
        }
    };
    let (t_band, t_score, t_seed) =
        (std::cell::Cell::new(0.0f64), std::cell::Cell::new(0.0f64), std::cell::Cell::new(0.0f64));
    let work = std::cell::Cell::new(0usize);
    let n_ufilt = std::cell::Cell::new(0usize);
    let t_ub = std::cell::Cell::new(0.0f64);
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(EXACT_BUDGET);
    let budget_c = std::cell::Cell::new(budget);
    let spend = |w: usize| work.set(work.get() + w);
    // set when a pass runs out of budget (or a deck search out of room): that pass certifies nothing
    let aborted = std::cell::Cell::new(false);
    // Exact value of trainee u at chances qs under cap lv: the best exact P(all) over her best legal skyline decks
    // and every legal deck whose product value times lv reaches it (each first through the proven bound).
    let exact_best = |u: usize, qs: &[f64], lv: f64| -> std::rc::Rc<Best> {
        let key = (u, qkey(qs));
        if let Some(entry) = ex_cache.borrow().get(&key)
            && entry.certified_cap >= lv
            && let Some(result) = &entry.reported_best
        {
            return result.clone();
        }
        let mut ent = ex_cache.borrow_mut().remove(&key).unwrap_or(ExactCacheEntry {
            best: None,
            scored_decks: Default::default(),
            checked_selections: Default::default(),
            certified_cap: -1.0,
            reported_best: None,
        });
        let uid = d.umas[u].id;
        let tsum = |x: &PDeck| fixed * qs.iter().map(|&q| model.trainee_fast(u, q, &x.dw, &x.dg)).sum::<f64>();
        let score = |x: &PDeck, ent: &mut ExactCacheEntry| {
            if !ent.scored_decks.insert(x.id()) {
                return;
            }
            let t0 = Instant::now();
            let (e, fl) = ents_of(x);
            let j = joint_b(model, u, qs, &e, fl, ntar, false);
            t_score.set(t_score.get() + t0.elapsed().as_secs_f64());
            if std::env::var("UPPER_CHECK").is_ok() {
                let er: Vec<(&Src, Kind)> = e.iter().map(|(s, k)| (&**s, *k)).collect();
                if let Some(ub) = upper_na(model, u, qs, &er, fl) {
                    for (i, y) in j.iter().enumerate() {
                        assert!(
                            ub[i] >= y.0 * (1.0 - 1e-12),
                            "upper bound below exact: {} < {} for {:?}",
                            ub[i],
                            y.0,
                            x.names
                        );
                        UCHK.with(|c| {
                            let mut v = c.get();
                            v.0 += 1;
                            v.1 = v.1.max(ub[i] / y.0);
                            c.set(v);
                        });
                    }
                } else {
                    UCHK.with(|c| {
                        let mut v = c.get();
                        v.2 += 1;
                        c.set(v);
                    });
                }
            }
            n_scored.set(n_scored.get() + 1);
            spend(200);
            let k = src_class(e.iter().map(|y| &*y.0));
            for (i, &q) in qs.iter().enumerate() {
                let p = fixed * model.trainee_fast(u, q, &x.dw, &x.dg);
                if p > 0.0 {
                    ratio_at(k, j[i].0 / p, uid, x);
                }
            }
            let sv: f64 = j.iter().map(|y| y.0).sum();
            if ent.best.as_ref().is_none_or(|b| sv > b.0) {
                ent.best = Some((sv, x.clone()));
            }
        };
        if ent.certified_cap < 0.0 {
            // seed: her best legal skyline decks by product value (or the best legal deck when none is on the skyline)
            let t0 = Instant::now();
            // the first 3 of a stable sort by value, highest first: each deck goes after every kept one at least as high
            let mut seeds: Vec<(f64, &PDeck)> = Vec::with_capacity(4);
            // (the value first, the blocks lookups only for a deck that would enter: one that can't is passed over blocked
            // or not, so the seeds are the same)
            for x in uview(u).iter().map(|&j| &sky[j]) {
                let t = tsum(x);
                let at = seeds
                    .iter()
                    .position(|s| s.0.partial_cmp(&t).unwrap() == std::cmp::Ordering::Less)
                    .unwrap_or(seeds.len());
                if at < 3 && !x.blocks(uid, &card_of) {
                    seeds.insert(at, (t, x));
                    seeds.truncate(3);
                }
            }
            t_seed.set(t_seed.get() + t0.elapsed().as_secs_f64());
            for (_, x) in seeds.iter().take(3) {
                score(x, &mut ent);
            }
            if ent.best.is_none() {
                let SearchOutcome { decks: r, .. } = space.search(
                    model,
                    &[u],
                    &[uid],
                    &[(qs.iter().map(|&q| (0, q)).collect(), 0.0)],
                    SearchMode::Best { seed: f64::NEG_INFINITY },
                );
                if let Some((_, sel)) = r.first() {
                    let x = space.pdeck(sel, &[uid]);
                    score(&x, &mut ent);
                }
            }
            ent.certified_cap = 1.0;
        }
        // every legal deck that might beat it under the cap lv
        let mut complete = true;
        if lv > ent.certified_cap {
            if let Some(thr) = ent.best.as_ref().map(|b| b.0) {
                n_band.set(n_band.get() + 1);
                let t0 = Instant::now();
                let n0 = space.nodes.get();
                let SearchOutcome { decks: mut cands, incomplete } = space.search(
                    model,
                    &[u],
                    &[uid],
                    &[(qs.iter().map(|&q| (0, q)).collect(), thr)],
                    SearchMode::Band { ratio_cap: lv, max_decks: space.band_cap() },
                );
                spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
                t_band.set(t_band.get() + t0.elapsed().as_secs_f64());
                if incomplete.is_some() {
                    cap_hit.set(true);
                    complete = false;
                }
                cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
                if complete {
                    for (v, sel) in &cands {
                        if lv * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 {
                            break;
                        }
                        if work.get() > budget_c.get() {
                            complete = false;
                            break;
                        }
                        // below the best at its own class's cap (the best rose since the band was built)
                        if space.sel_cap(sel, lv) * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 {
                            continue;
                        }
                        if !ent.checked_selections.insert((sel.ents.clone(), sel.plain, sel.filler)) {
                            continue;
                        }
                        // the proven bound first: a deck it puts below the best exact value cannot beat it
                        let e: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                        let t1 = Instant::now();
                        let ubo = upper_na(model, u, qs, &e, sel.filler);
                        t_ub.set(t_ub.get() + t1.elapsed().as_secs_f64());
                        if let Some(ub) = ubo {
                            spend(25);
                            if ub.iter().sum::<f64>() * (1.0 + 1e-9) < ent.best.as_ref().unwrap().0 {
                                n_ufilt.set(n_ufilt.get() + 1);
                                continue;
                            }
                        }
                        let x = space.pdeck(sel, &[uid]);
                        score(&x, &mut ent);
                    }
                }
                if complete {
                    ent.certified_cap = lv;
                } else {
                    aborted.set(true);
                }
            } else {
                ent.certified_cap = lv;
            }
        }
        let b = if let Some((sum, x)) = &ent.best {
            let (e, fl) = ents_of(x);
            let per = joint_b(model, u, qs, &e, fl, ntar, true);
            Best { sum: *sum, deck: Some(x.clone()), per }
        } else {
            let per = joint(model, u, qs, &[], 0, ntar, true);
            Best { sum: per.iter().map(|y| y.0).sum(), deck: None, per }
        };
        let b = std::rc::Rc::new(b);
        ent.reported_best = Some(b.clone());
        ex_cache.borrow_mut().insert(key, ent);
        b
    };
    // exact ceiling table: X[u][i] = best exact value over every legal deck at chance grid[i] under the cap it was
    // computed for (rises with the chance, so a lineage with chance q is bounded at grid[gidx(q)]; each lineage may
    // take its own deck)
    let xtab_c: std::cell::RefCell<Vec<[(f64, f64); NG]>> = std::cell::RefCell::new(vec![[(f64::NAN, 0.0); NG]; n]);
    let xtab = |u: usize, i: usize, lv: f64| -> f64 {
        let (v, l) = xtab_c.borrow()[u][i];
        if !v.is_nan() && l >= lv {
            return v;
        }
        let v = exact_best(u, &[grid[i]], lv).sum;
        // a value from a deck search the budget cut short is not exact under lv: not kept
        if !aborted.get() {
            xtab_c.borrow_mut()[u][i] = (v, lv);
        }
        v
    };
    // Ceiling of a cell (uma, grid point) under cap lv: the exact value if computed under lv or more, else the lowest
    // such value at a higher grid point, else the product-odds bound times lv; (bound, exact?)
    let cell = |u: usize, i: usize, lv: f64| -> (f64, bool) {
        let mut b = lv * fixed * btab(u, i) * (1.0 + 1e-12);
        let xb = xtab_c.borrow();
        let row = &xb[u];
        for j in i..NG {
            if !row[j].0.is_nan() && row[j].1 >= lv {
                b = b.min(row[j].0 * (1.0 + 1e-12));
                return (b, j == i);
            }
        }
        (b, false)
    };
    // Clusters of the whole skyline, and the best m skyline decks by `val` among those passing `ok` (cluster maxima
    // bound their members, best bound first; `val` must rise with every odds coordinate)
    let full_cl: Vec<Cl> = if dj.per_uma {
        vec![]
    } else {
        let mut ix: Vec<usize> = (0..sky.len()).collect();
        let mut out = vec![];
        if nu > 0 && !ix.is_empty() {
            split(&sky, &mut ix, nu, &mut out);
        } else {
            out.push(Cl { mdw: vec![], mdg: vec![], mem: ix });
        }
        out
    };
    // (as `bb_max`: the clusters in the stable sort's order, equal bounds by index, from a heap over a reused buffer)
    let st_buf: std::cell::RefCell<Vec<ByBound>> = Default::default();
    let sky_top = |m: usize, val: &dyn Fn(&[f64], &[f64]) -> f64, ok: &dyn Fn(&PDeck) -> bool| -> Vec<(f64, usize)> {
        let mut bs = std::mem::take(&mut *st_buf.borrow_mut());
        bs.clear();
        bs.extend(
            full_cl
                .iter()
                .enumerate()
                .map(|(i, c)| ByBound { bound: if nu > 0 { val(&c.mdw, &c.mdg) } else { f64::INFINITY }, cluster: i }),
        );
        let mut h = std::collections::BinaryHeap::from(bs);
        let mut top: Vec<(f64, usize)> = vec![];
        while let Some(ByBound { bound: b, cluster: ci }) = h.pop() {
            if top.len() >= m && b * (1.0 + 1e-9) < top[m - 1].0 {
                break;
            }
            for &j in &full_cl[ci].mem {
                if !ok(&sky[j]) {
                    continue;
                }
                let v = val(&sky[j].dw, &sky[j].dg);
                if top.len() < m || v > top[m - 1].0 {
                    let pos = top.iter().position(|x| v > x.0).unwrap_or(top.len());
                    top.insert(pos, (v, j));
                    top.truncate(m);
                }
            }
        }
        *st_buf.borrow_mut() = h.into_vec();
        top
    };
    // shared-deck ceiling: the best single skyline deck (legal or not) over the loop's twelve lineages, product odds
    let relaxed_shared = |f: [usize; 4], qa: &[f64; 12]| -> f64 {
        let ln = lin_of(&f);
        let val =
            |dw: &[f64], dg: &[f64]| (0..ln.len()).map(|l| model.trainee_fast(f[ln[l].0], qa[l], dw, dg)).sum::<f64>();
        sky_top(1, &val, &|_| true).first().map_or(0.0, |x| x.0) * fixed / ln.len() as f64 * (1.0 + 1e-12)
    };
    // ---- shared deck ----
    // With one deck for the whole loop, a loop's value is the best exact sum over its twelve lineages among decks legal
    // for all four members. Per loop: every such deck whose exact sum is at least `floor` (the k-th best distinct
    // deck so far, times 12) or among the loop's own best `top_decks`, under cap lv; sorted, best first. Ceilings
    // stay the per-uma ones (each trainee's best deck bounds her share of any shared deck).
    let shared = !dj.per_uma;
    let kloops = dj.top_decks.max(1);
    type SList = std::rc::Rc<Vec<(f64, PDeck)>>;
    let sh_cache: std::cell::RefCell<crate::FxMap<([usize; 4], QKey), (f64, f64, SList)>> = Default::default();
    let exact_shared = |f: [usize; 4], qa: &[f64], floor: f64, lv: f64| -> SList {
        let key = (f, qkey(qa));
        if let Some((l, fl, r)) = sh_cache.borrow().get(&key)
            && *l >= lv
            && *fl <= floor
        {
            return r.clone();
        }
        let ln = lin_of(&f);
        let fm = members(&f);
        let nm = fm.len();
        let us: &[usize] = fm;
        let ids: Vec<u32> = fm.iter().map(|&u| d.umas[u].id).collect();
        let pairs: Vec<(usize, f64)> = (0..ln.len()).map(|l| (ln[l].0, qa[l])).collect();
        let qt: Vec<Vec<f64>> =
            (0..nm).map(|t| (0..ln.len()).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect()).collect();
        let mut done: crate::FxSet<DeckId> = Default::default();
        let mut list: Vec<(f64, PDeck)> = vec![];
        let mut score = |x: &PDeck, list: &mut Vec<(f64, PDeck)>| {
            if !done.insert(x.id()) {
                return;
            }
            let t0 = Instant::now();
            let (e, fl) = ents_of(x);
            let k = src_class(e.iter().map(|y| &*y.0));
            let mut sv = 0.0;
            for t in 0..nm {
                let j = joint_b(model, f[t], &qt[t], &e, fl, ntar, false);
                for (i, &q) in qt[t].iter().enumerate() {
                    let p = fixed * model.trainee_fast(f[t], q, &x.dw, &x.dg);
                    if p > 0.0 {
                        ratio_at(k, j[i].0 / p, d.umas[f[t]].id, x);
                    }
                    sv += j[i].0;
                }
            }
            t_score.set(t_score.get() + t0.elapsed().as_secs_f64());
            n_scored.set(n_scored.get() + 1);
            spend(800);
            list.push((sv, x.clone()));
        };
        // the threshold a deck must reach: the floor, or the loop's k-th best so far when that is higher
        let need = |list: &Vec<(f64, PDeck)>| -> f64 {
            let mut v: Vec<f64> = list.iter().map(|x| x.0).collect();
            v.sort_by(|a, b| b.partial_cmp(a).unwrap());
            let kl = if v.len() >= kloops { v[kloops - 1] } else { f64::NEG_INFINITY };
            let t = floor.max(kl);
            if t.is_finite() { t } else { v.last().copied().unwrap_or(f64::NEG_INFINITY) }
        };
        let t0 = Instant::now();
        let val = |dw: &[f64], dg: &[f64]| {
            fixed * pairs.iter().map(|&(t, q)| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>()
        };
        let seeds = sky_top(kloops + 2, &val, &|x| ids.iter().all(|&id| !x.blocks(id, &card_of)));
        t_seed.set(t_seed.get() + t0.elapsed().as_secs_f64());
        for &(_, j) in &seeds {
            score(&sky[j], &mut list);
        }
        if list.is_empty() {
            let SearchOutcome { decks: r, .. } =
                space.search(model, us, &ids, &[(pairs.clone(), 0.0)], SearchMode::Best { seed: f64::NEG_INFINITY });
            if let Some((_, sel)) = r.first() {
                let x = space.pdeck(sel, &ids);
                score(&x, &mut list);
            }
        }
        let mut complete = true;
        if lv > 1.0 && !list.is_empty() {
            n_band.set(n_band.get() + 1);
            let t0 = Instant::now();
            let n0 = space.nodes.get();
            let SearchOutcome { decks: mut cands, incomplete } = space.search(
                model,
                us,
                &ids,
                &[(pairs.clone(), need(&list))],
                SearchMode::Band { ratio_cap: lv, max_decks: space.band_cap() },
            );
            spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
            t_band.set(t_band.get() + t0.elapsed().as_secs_f64());
            if incomplete.is_some() {
                cap_hit.set(true);
                complete = false;
            }
            cands.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
            let mut seen: crate::FxSet<(Vec<usize>, usize, usize)> = Default::default();
            if complete {
                for (v, sel) in &cands {
                    let thr = need(&list);
                    if lv * v * (1.0 + 1e-9) < thr {
                        break;
                    }
                    if work.get() > budget_c.get() {
                        complete = false;
                        break;
                    }
                    if space.sel_cap(sel, lv) * v * (1.0 + 1e-9) < thr {
                        continue;
                    }
                    if !seen.insert((sel.ents.clone(), sel.plain, sel.filler)) {
                        continue;
                    }
                    let e: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                    let t1 = Instant::now();
                    let ub: Option<f64> = (0..nm)
                        .map(|t| upper_na(model, f[t], &qt[t], &e, sel.filler).map(|b| b.iter().sum::<f64>()))
                        .sum();
                    t_ub.set(t_ub.get() + t1.elapsed().as_secs_f64());
                    if let Some(ub) = ub {
                        spend(100);
                        if ub * (1.0 + 1e-9) < thr {
                            n_ufilt.set(n_ufilt.get() + 1);
                            continue;
                        }
                    }
                    let x = space.pdeck(sel, &ids);
                    score(&x, &mut list);
                }
            }
            if !complete {
                aborted.set(true);
            }
        }
        let thr = need(&list);
        list.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap());
        list.retain(|x| x.0 >= thr);
        let r: SList = std::rc::Rc::new(list);
        if complete {
            sh_cache.borrow_mut().insert(key, (lv, floor, r.clone()));
        }
        r
    };
    // Exact ceiling of a foursome at chances `qa`, refined lazily: while it beats `kth`, the loosest inexact term is
    // tightened (first computed at the uma's highest grid point, then halfway down towards its own) until the ceiling
    // stops beating `kth` or every term is exact. Each step is one exact deck search for one uma.
    let exact_ceiling = |f: [usize; 4], qa: &[f64; 12], kth: f64, lv: f64| -> f64 {
        loop {
            let mut tot = 0.0;
            let mut worst: Option<(f64, usize, usize)> = None;
            let ln = lin_of(&f);
            for l in 0..ln.len() {
                let (u, i) = (f[ln[l].0], gidx(qa[l]));
                let (b, ex) = cell(u, i, lv);
                tot += b;
                if !ex && worst.is_none_or(|w| b > w.0) {
                    worst = Some((b, u, i));
                }
            }
            let ce = tot / ln.len() as f64;
            let Some((_, u, i)) = worst else { return ce };
            if ce <= kth * (1.0 + 4e-12) || aborted.get() {
                return ce;
            }
            let j = {
                let xb = xtab_c.borrow();
                let row = &xb[u];
                (i..NG).find(|&j| !row[j].0.is_nan() && row[j].1 >= lv)
            };
            let tgt = match j {
                None => i.max(gidx(qmax[u])),
                Some(j) => i + (j - i) / 2,
            };
            xtab(u, tgt, lv);
        }
    };
    // ceiling with each uma's best skyline deck (legal or not), the same deck across her lineages
    let relaxed = |f: [usize; 4], qa: &[f64; 12]| -> f64 {
        let mut tot = 0.0;
        let ln = lin_of(&f);
        for t in 0..members(&f).len() {
            let qs: Vec<f64> = (0..ln.len()).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect();
            let init = qs.iter().map(|&q| model.trainee_fast(f[t], q, &zero, &zero)).sum::<f64>();
            let val = |dw: &[f64], dg: &[f64]| qs.iter().map(|&q| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>();
            tot += if qs.iter().all(|q| (0.0..=1.0).contains(q)) {
                bb_max(f[t], init, &val)
            } else {
                uview(f[t]).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max)
            };
        }
        fixed * tot / ln.len() as f64 * (1.0 + 1e-12)
    };
    let mval: Vec<f64> = (0..n).map(|u| if pool.contains(&u) { btab(u, gidx(qmax[u])) } else { 0.0 }).collect();
    let mut ord: Vec<usize> = pool.clone();
    ord.sort_by(|&a, &b| mval[b].partial_cmp(&mval[a]).unwrap());
    let mv: Vec<f64> = ord.iter().map(|&u| mval[u]).collect();
    // A structure's product ceiling is at most the guaranteed golds' factor times the mean of its members' M_u (each
    // member heads the same number of lineages): (3/12)·sum for four, sum/3 for three. Bands are in four-member sum
    // units: a three-member structure's sum s counts as e3(s) = 4s/3.
    let sep = |s: f64| fixed * 3.0 * s / 12.0 * (1.0 + 1e-12);
    let e3 = |s: f64| s * 4.0 / 3.0;
    let nreq = req.len();
    let choose = |m: usize, c: usize| (0..c).fold(1usize, |acc, i| acc * (m - i) / (i + 1));
    let total_foursomes: usize = {
        let m = k - nreq.min(k);
        (if with4 { choose(m, 4usize.saturating_sub(nreq)) } else { 0 })
            + (if with3 && nreq <= 3 { choose(m, 3 - nreq) } else { 0 })
    };
    let band = |lo: f64, hi: f64, out: &mut Vec<(f64, [usize; 4])>| {
        if with4 {
            for a in 0..k {
                if a + 3 >= k || mv[a] + mv[a + 1] + mv[a + 2] + mv[a + 3] < lo {
                    break;
                }
                for b in a + 1..k {
                    if b + 2 >= k || mv[a] + mv[b] + mv[b + 1] + mv[b + 2] < lo {
                        break;
                    }
                    for c in b + 1..k {
                        if c + 1 >= k || mv[a] + mv[b] + mv[c] + mv[c + 1] < lo {
                            break;
                        }
                        for e in c + 1..k {
                            let sm = mv[a] + mv[b] + mv[c] + mv[e];
                            if sm < lo {
                                break;
                            }
                            if sm >= hi {
                                continue;
                            }
                            let mut f = [ord[a], ord[b], ord[c], ord[e]];
                            f.sort_unstable();
                            if !req.iter().all(|r| f.contains(r)) {
                                continue;
                            }
                            let ln = lins(4);
                            let qs = model.lineage_q_arr(&f, &|x, y| pb[f[x]][f[y]]);
                            let ub = fixed * (0..12).map(|l| btab(f[ln[l].0], gidx(qs[l]))).sum::<f64>() / 12.0
                                * (1.0 + 1e-12);
                            out.push((ub, f));
                        }
                    }
                }
            }
        }
        if with3 && nreq <= 3 {
            for a in 0..k {
                if a + 2 >= k || e3(mv[a] + mv[a + 1] + mv[a + 2]) < lo {
                    break;
                }
                for b in a + 1..k {
                    if b + 1 >= k || e3(mv[a] + mv[b] + mv[b + 1]) < lo {
                        break;
                    }
                    for c in b + 1..k {
                        let sm = e3(mv[a] + mv[b] + mv[c]);
                        if sm < lo {
                            break;
                        }
                        if sm >= hi {
                            continue;
                        }
                        let mut t = [ord[a], ord[b], ord[c]];
                        t.sort_unstable();
                        let f = [t[0], t[1], t[2], NONE];
                        if !req.iter().all(|r| f.contains(r)) {
                            continue;
                        }
                        let ln = lins(3);
                        let qs = model.lineage_q_arr(&t, &|x, y| pb[t[x]][t[y]]);
                        let ub =
                            fixed * (0..3).map(|l| btab(t[ln[l].0], gidx(qs[l]))).sum::<f64>() / 3.0 * (1.0 + 1e-12);
                        out.push((ub, f));
                    }
                }
            }
        }
    };
    let smax = (if with4 && k >= 4 { mv[0] + mv[1] + mv[2] + mv[3] } else { 0.0 }).max(if with3 && k >= 3 {
        e3(mv[0] + mv[1] + mv[2])
    } else {
        0.0
    });
    // schedules found, per foursome (shared by the passes)
    let sched_c: std::cell::RefCell<crate::FxMap<[usize; 4], Res>> = Default::default();
    struct Pass {
        solved: Vec<([usize; 4], Res, Vec<f64>, Vec<std::rc::Rc<Best>>, f64)>,
        kth: f64,
        next_bound: f64,
        scanned: usize,
        generated: usize,
        sched_rejects: usize,
        deck_best: crate::FxMap<(Vec<u32>, Vec<u8>, Vec<u32>, usize), (f64, usize, usize)>,
        lists: Vec<SList>,
    }
    // One best-first pass over the foursomes with cap lv: ceilings are exact tables or product ceilings times lv, the
    // stop rule compares them with the k-th exact value. None when the budget runs out (the pass certifies nothing).
    let run_pass = |lv: f64| -> Option<Pass> {
        let mut hi_sum = f64::INFINITY;
        let lo_sum = smax * 0.85;
        let mut cand: Vec<(f64, [usize; 4])> = vec![];
        band(lo_sum, hi_sum, &mut cand);
        hi_sum = lo_sum;
        cand.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        let mut solved: Vec<([usize; 4], Res, Vec<f64>, Vec<std::rc::Rc<Best>>, f64)> = vec![];
        let mut vals: Vec<f64> = vec![];
        let mut next = 0;
        let mut scanned = 0;
        let mut generated = cand.len();
        let mut kth_val = f64::NEG_INFINITY;
        let mut sched_rejects = 0usize;
        // shared deck: each deck's best value and where (solved index, list index); the per-loop deck lists
        let mut deck_best: crate::FxMap<(Vec<u32>, Vec<u8>, Vec<u32>, usize), (f64, usize, usize)> = Default::default();
        let mut lists: Vec<SList> = vec![];
        loop {
            if aborted.get() {
                return None;
            }
            // Ceilings carry a (1 + 1e-12) guard against rounding, so a loop whose ceiling beats the k-th value by no
            // more than that only ties it and cannot change the result. Without this, a job whose targets are all
            // guaranteed (every loop has the same value) solved every foursome.
            let beats = |c: f64| c > kth_val * (1.0 + 4e-12);
            if !(next < cand.len() && beats(lv * cand[next].0)) {
                if hi_sum <= 0.0 || !beats(lv * sep(hi_sum)) {
                    break;
                }
                let kth_lo =
                    if kth_val > f64::NEG_INFINITY { kth_val / lv * 12.0 / (3.0 * fixed) * (1.0 - 1e-9) } else { 0.0 };
                let lo = kth_lo.max(hi_sum * 0.85);
                let lo = if lo < smax * 1e-6 { 0.0 } else { lo };
                let mut more: Vec<(f64, [usize; 4])> = vec![];
                band(lo, hi_sum, &mut more);
                hi_sum = lo;
                generated += more.len();
                let mut rest: Vec<(f64, [usize; 4])> = cand.split_off(next);
                rest.extend(more);
                rest.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
                cand.extend(rest);
                continue;
            }
            let (_, f) = cand[next];
            next += 1;
            scanned += 1;
            let fm = members(&f);
            let nm = fm.len();
            let ln = lins(nm);
            let nl = ln.len();
            let qpb = model.lineage_q_arr(fm, &|x, y| pb[fm[x]][fm[y]]);
            // cheapest first: the cell bounds as they stand, then the product ceiling with one deck per trainee
            if !beats((0..nl).map(|l| cell(f[ln[l].0], gidx(qpb[l]), lv).0).sum::<f64>() / nl as f64) {
                continue;
            }
            if !beats(lv * relaxed(f, &qpb)) {
                continue;
            }
            if shared && !beats(lv * relaxed_shared(f, &qpb)) {
                continue;
            }
            // exact ceiling at the pairwise-ceiling chances, valid for any schedule
            if !beats(exact_ceiling(f, &qpb, kth_val, lv)) {
                continue;
            }
            if aborted.get() {
                return None;
            }
            let r = sched_c.borrow().get(&f).cloned();
            let r = r.unwrap_or_else(|| {
                let r = match &fixed_s {
                    Some(fs) => {
                        Res { s: std::array::from_fn(|i| if i < nm { fs[f[i]].unwrap() } else { [Choice::Rest; NT] }) }
                    }
                    None => optimize_fam(d, &f, sj.iters, sj.seed, 60.0),
                };
                sched_c.borrow_mut().insert(f, r.clone());
                r
            });
            let ps: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
            let qs = model.lineage_q(fm, &|x, y| if no_race { 0.0 } else { wval(&ps[x], &ps[y]) });
            let qa: [f64; 12] = std::array::from_fn(|i| qs.get(i).copied().unwrap_or(0.0));
            // rejected only with the schedule found: a better schedule could change this one's fate
            if (shared && !beats(lv * relaxed_shared(f, &qa))) || !beats(exact_ceiling(f, &qa, kth_val, lv)) {
                sched_rejects += 1;
                continue;
            }
            if shared {
                let sl = exact_shared(
                    f,
                    &qa[..nl],
                    if kth_val.is_finite() { kth_val * nl as f64 } else { f64::NEG_INFINITY },
                    lv,
                );
                if aborted.get() {
                    return None;
                }
                if sl.is_empty() {
                    continue;
                }
                let val = sl[0].0 / nl as f64;
                for (v, x) in sl.iter() {
                    let id = (x.ids.clone(), x.ends.clone(), x.vars.clone(), x.plain_ids.len());
                    let e = deck_best.entry(id).or_insert((f64::NEG_INFINITY, 0, 0));
                    if v / nl as f64 > e.0 {
                        *e = (v / nl as f64, solved.len(), lists.len());
                    }
                }
                solved.push((f, r, qs, vec![], val));
                lists.push(sl);
                let mut vs: Vec<f64> = deck_best.values().map(|x| x.0).collect();
                vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
                kth_val = if vs.len() >= kloops { vs[kloops - 1] } else { f64::NEG_INFINITY };
                continue;
            }
            let mut tot = 0.0;
            let mut bests = vec![];
            for t in 0..nm {
                let qt: Vec<f64> = (0..nl).filter(|&l| ln[l].0 == t).map(|l| qa[l]).collect();
                let b = exact_best(f[t], &qt, lv);
                tot += b.sum;
                bests.push(b);
            }
            if aborted.get() {
                return None;
            }
            let val = tot / nl as f64;
            solved.push((f, r, qs, bests, val));
            vals.push(val);
            let mut vs = vals.clone();
            vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
            kth_val = if vs.len() >= kloops { vs[kloops - 1] } else { f64::NEG_INFINITY };
        }
        let next_bound =
            lv * (if next < cand.len() { cand[next].0 } else { 0.0 }).max(if hi_sum > 0.0 { sep(hi_sum) } else { 0.0 });
        Some(Pass { solved, kth: kth_val, next_bound, scanned, generated, sched_rejects, deck_best, lists })
    };
    // Try the caps highest first (EXACT_STEPS): the answer is certified under the first whose pass finishes within the
    // budget; a lower cap's pass reuses everything computed so far and gets a quarter of the budget more. When none
    // finishes, a pass with the cap at 1 (product odds: no deck searches) gives the answer.
    let steps: Vec<f64> = if !dj.exact {
        vec![]
    } else if let Ok(x) = std::env::var("EXACT_STEPS") {
        x.split(',').filter_map(|t| t.parse().ok()).collect()
    } else {
        EXACT_STEPS.iter().map(|&x| if x == EXACT_RHO { rho_top } else { x }).collect()
    };
    let mut last: Option<Pass> = None;
    let mut cert = 1.0f64;
    let mut allow = budget;
    for &lv in &steps {
        budget_c.set(allow);
        if let Some(pass) = run_pass(lv) {
            last = Some(pass);
            cert = lv;
            break;
        } else {
            // Retain scored decks, but grant fresh work before trying a lower cap.
            aborted.set(false);
            allow = work.get() + budget / 4;
        }
    }
    let p = if let Some(p) = last {
        p
    } else {
        budget_c.set(usize::MAX);
        run_pass(1.0).expect("a pass without deck searches")
    };
    let Pass { solved, kth: kth_val, next_bound, scanned, generated, sched_rejects, deck_best, lists } = p;
    if solved.is_empty() {
        return json!({"error": "No loop contains all the required umas."});
    }
    if std::env::var("UPPER_CHECK").is_ok() {
        let v = UCHK.with(std::cell::Cell::get);
        eprintln!("UPPER_CHECK: {} checked, max bound/exact {:.4}, {} decks not covered", v.0, v.1, v.2);
    }
    if std::env::var("DECK_DEBUG").is_ok() {
        let v = SPT.with(std::cell::Cell::get);
        eprintln!("deck searches: setup {:.2}s tree {:.2}s configs {} count vectors {}", v.0, v.1, v.2, v.3);
    }
    if std::env::var("DECK_DEBUG").is_ok() {
        eprintln!(
            "per-uma search {:.2}s cap {} scanned {} solved {} exact: {} deck searches, {} decks scored, {} bounded out, schedule rejects {} | band {:.2}s score {:.2}s seed-scan {:.2}s bound {:.2}s",
            tq.elapsed().as_secs_f64(),
            cert,
            scanned,
            solved.len(),
            n_band.get(),
            n_scored.get(),
            n_ufilt.get(),
            sched_rejects,
            t_band.get(),
            t_score.get(),
            t_seed.get(),
            t_ub.get()
        );
    }
    let mut order: Vec<usize> = (0..solved.len()).collect();
    // stable: ties keep solve order
    order.sort_by(|&a, &b| solved[b].4.partial_cmp(&solved[a].4).unwrap());
    let empty_deck = PDeck {
        names: vec![],
        ids: vec![],
        ends: vec![],
        vars: vec![],
        choices: vec![],
        plain_ids: vec![],
        pal_names: vec![],
        pals: 0,
        borrowed: None,
        dw: zero.clone(),
        dg: zero.clone(),
    };
    // what is reported: per-uma, the best loops; shared, the best distinct decks, each with its best loop
    let picks_out: Vec<(usize, Vec<&PDeck>, Vec<(f64, f64, usize)>, f64)> = if shared {
        let mut db: Vec<(&(Vec<u32>, Vec<u8>, Vec<u32>, usize), &(f64, usize, usize))> = deck_best.iter().collect();
        db.sort_by(|a, b| b.1.0.partial_cmp(&a.1.0).unwrap().then(a.0.cmp(b.0)));
        db.iter()
            .take(kloops)
            .map(|&(id, &(v, si, li))| {
                let x = &lists[li]
                    .iter()
                    .find(|y| y.1.ids == id.0 && y.1.ends == id.1 && y.1.vars == id.2 && y.1.plain_ids.len() == id.3)
                    .unwrap()
                    .1;
                let (f, _, qs, _, _) = &solved[si];
                let (e, fl) = ents_of(x);
                let ln = lin_of(f);
                let nm = members(f).len();
                let mut per_l = vec![(0.0, 0.0, 0usize); ln.len()];
                for t in 0..nm {
                    let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].0 == t).collect();
                    let qt: Vec<f64> = ls.iter().map(|&l| qs[l]).collect();
                    let j = joint_b(model, f[t], &qt, &e, fl, ntar, true);
                    for (i, &l) in ls.iter().enumerate() {
                        per_l[l] = j[i];
                    }
                }
                (si, vec![x; nm], per_l, v)
            })
            .collect()
    } else {
        order
            .iter()
            .take(kloops)
            .map(|&si| {
                let bests = &solved[si].3;
                let ln = lin_of(&solved[si].0);
                let nm = members(&solved[si].0).len();
                let mut per_l = vec![(0.0, 0.0, 0usize); ln.len()];
                for t in 0..nm {
                    let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].0 == t).collect();
                    for (i, &l) in ls.iter().enumerate() {
                        per_l[l] = bests[t].per[i];
                    }
                }
                (si, bests.iter().map(|b| b.deck.as_ref().unwrap_or(&empty_deck)).collect(), per_l, solved[si].4)
            })
            .collect()
    };
    let top: Vec<Value> = picks_out.iter().map(|(si, dk, per_l, pj)| {
        let (f, r, qs, _, _) = &solved[*si];
        let ln = lin_of(f); let f = members(f); let nm = f.len(); let nl = ln.len();
        let mut per = vec![(0.0, 0usize); nm]; let mut picks: Vec<Vec<usize>> = vec![vec![]; nm];
        for (l, &(t, ..)) in ln.iter().enumerate() { per[t].0 += per_l[l].0; per[t].1 += 1; picks[t].push(per_l[l].2); }
        let minus1 = per_l.iter().map(|x| x.1).sum::<f64>() / nl as f64;
        // the chance of exactly k targets, k = 0 ..= n (exact over each lineage's landed sets, as `p_all`), averaged over
        // the lineages: the uncertain targets' sets from the joint model, each guaranteed gold on its own (GEN_GOLD)
        let mut landed = vec![0.0f64; ntar + 1];
        // (each member's deck's sources once, read by each of her lineages)
        let dke: Vec<_> = (0..nm).map(|t| ents_of(dk[t])).collect();
        for l in 0..nl {
            let t = ln[l].0; let (e, fl) = (&dke[t].0, dke[t].1); let nu = unc.len();
            let mut ext = JointExt { su: vec![1.0; nu], sf: vec![1.0; ntar - nu], marg: vec![], sets: vec![0.0], per_outfit: vec![] };
            joint_fast(model, f[t], &[qs[l]], e, fl, ntar, false, Some(&mut ext));
            let o = per_l[l].2;
            let mut g = vec![1.0f64];
            for _ in nu..ntar { let mut nn = vec![0.0; g.len() + 1]; for (i, x) in g.iter().enumerate() { nn[i] += x * (1.0 - GEN_GOLD); nn[i + 1] += x * GEN_GOLD; } g = nn; }
            for (mu, &x) in ext.sets[(o << nu)..((o + 1) << nu)].iter().enumerate() {
                if x == 0.0 { continue; }
                for (j, &y) in g.iter().enumerate() { landed[mu.count_ones() as usize + j] += x * y / nl as f64; }
            }
        }
        // the search's product value of this loop with these decks
        let rank_v = fixed * (0..nl).map(|l| model.trainee_fast(f[ln[l].0], qs[l], &dk[ln[l].0].dw, &dk[ln[l].0].dg)).sum::<f64>() / nl as f64;
        let outfits: Vec<Value> = (0..nm).map(|i| {
            let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
            let (st, cs) = &model.opts[f[i]][kk];
            let covers: Vec<Value> = st.iter().enumerate().filter(|(_, lv)| **lv > 0).map(|(j, lv)| json!({"group": unc[j], "level": lv})).collect();
            json!({"outfits": if covers.is_empty() { vec![] } else { cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers})
        }).collect();
        let pss: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let dks: Vec<(&[f64], &[f64])> = (0..nm).map(|t| ((&*dk[t].dw), (&*dk[t].dg))).collect();
        let mut detail = model.detail(f, &|a, b| if no_race { 0.0 } else { wval(&pss[a], &pss[b]) }, Some(&dks));
        if let Some(ls) = detail["lineages"].as_array_mut() { for (l, x) in ls.iter_mut().enumerate() { x["rank_value"] = x["p_all"].clone(); x["p_all"] = json!(per_l[l].0); } }
        let lp = json!({"members": member_json(d, f), "p_all": pj, "p_minus1": minus1, "landed": landed, "rank_value": rank_v, "outfits": outfits,
               "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
               "mean_proc": qs.iter().sum::<f64>() / qs.len() as f64, "schedule": chart_rows(d, f, &r.s[..nm]), "detail": detail});
        if shared { return json!({"loop": lp, "deck": dk[0].out(&unc)}); }
        // the deck most members share, for displays that show one deck
        let base = (0..nm).max_by_key(|&i| (dk.iter().filter(|x| x.names == dk[i].names).count(), nm - i)).unwrap();
        json!({"loop": lp, "deck": dk[base].out(&unc), "decks": dk.iter().map(|x| x.out(&unc)).collect::<Vec<_>>()})
    }).collect();
    let budget_out = dj.exact && cert < rho_top;
    let complete = next_bound <= kth_val * (1.0 + 4e-12);
    // what the answer rests on: `rho` the highest cap certified (`rho_target` the one sought), `class_caps` the assumed
    // cap per structure class (each certified at most at rho), `class_max_ratio` the largest exact / product odds seen
    // per class on any deck scored here (`max_ratio` of them all); `complete` = the search ended by its stop rule (no
    // unexamined loop or deck can win under the certified caps)
    let mr = max_ratio.get();
    let exact = json!({"rho": cert, "rho_target": rho_top, "class_caps": caps, "max_ratio": mr.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": mr,
                       "class_max_deck": max_deck.borrow().iter().map(|x| x.as_ref().map(|(u, n)| json!({"trainee": u, "cards": n}))).collect::<Vec<_>>(), "complete": complete, "decks_scored": n_scored.get(),
                       "deck_searches": n_band.get(), "loops_solved": solved.len(), "schedule_rejects": sched_rejects, "cap_hit": cap_hit.get(), "budget_out": budget_out, "bound_filtered": n_ufilt.get()});
    let mut out = stats;
    for (key, val) in [
        ("top", json!(top)),
        ("best", json!([])),
        ("per_uma", json!(!shared)),
        ("foursomes", json!(total_foursomes)),
        ("foursomes_ranked", json!(generated)),
        ("foursomes_scanned", json!(scanned)),
        ("foursomes_solved", json!(solved.len())),
        ("next_bound", json!(next_bound)),
        ("proven", json!(complete)),
        ("searches", json!(1 + n_band.get())),
        ("exact", exact),
    ] {
        out[key] = val;
    }
    out
}
/// (bound, cluster) ordered as the stable sort by bound, highest first, visits them: a heap's pops give the higher bound
/// first and equal bounds (`partial_cmp`: 0 and -0 tie) in index order, the same sequence (a NaN bound panics in
/// either)
struct ByBound {
    bound: f64,
    cluster: usize,
}
impl PartialEq for ByBound {
    fn eq(&self, o: &Self) -> bool {
        self.cmp(o) == std::cmp::Ordering::Equal
    }
}
impl Eq for ByBound {}
impl PartialOrd for ByBound {
    fn partial_cmp(&self, o: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for ByBound {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.bound
            .partial_cmp(&other.bound)
            .unwrap()
            // BinaryHeap pops the greatest item: reverse indices so earlier clusters win ties.
            .then(other.cluster.cmp(&self.cluster))
    }
}
/// A memo key of up to 12 chances (a loop's lineages): their bits and count, a fixed array (equal exactly when the
/// chance lists are bit for bit)
type QKey = ([u64; 12], u8);
fn qkey(qs: &[f64]) -> QKey {
    assert!(qs.len() <= 12, "more than 12 chances in a memo key");
    let mut k = [0u64; 12];
    for (x, q) in k.iter_mut().zip(qs) {
        *x = q.to_bits();
    }
    (k, qs.len() as u8)
}
