//! Career planner: the best next career from the veterans a player actually has (their own, plus borrowed ones).
//!
//! The loop planner assumes a running loop: every legacy carries every target spark (stars from the rank mix) and
//! race overlaps come from planned schedules. Here each candidate career (trainee T, parents L1 and L2 from the
//! veteran list, at most one borrowed) uses what those veterans really carry:
//!   - affinity per legacy from relation points and the G1 wins on record: a parent P (other parent Q, P's own
//!     parents G) counts pair(T,P) + pair(P,Q) + 3 per G1 shared by P and Q + sum over G of [tri(T,P,G) + 3 per G1
//!     shared by P and G]; a grandparent G under P counts tri(T,P,G) + 3 per G1 shared by P and G (skills.rs, same
//!     fit); a veteran whose parents are unknown adds no grandparent slots;
//!   - per target k, the inheritance chance q_k = 1 - prod over the six legacies (1 - min(1, 0.03 s (1 + A/100)))^2,
//!     s = the stars of that legacy's spark of k (0 if it has none);
//!   - spark generation 0.2 / 0.4 x 1.1^n_k for a white / gold, n_k = how many of the six carry a spark of k.
//! Everything else is the loop planner's model. q_k and the trainee's own event hint chance ew enter every formula
//! only as (1 - ew)(1 - q_k), so T's event hint chances are replaced by 1 - (1 - ew)(1 - q_k) and the deck search
//! runs at q = 0: its products, bounds and exact odds are then the career's, apart from the factor 1.1^(n_k - 6) per
//! target, which is the same for every deck and multiplies the result (`JointExt` applies it per target where the
//! number is not a plain product: P(all but one), the per-target chances).
//!
//! Search: every legal candidate gets a ceiling, EXACT_RHO x C x the product value at each target's best possible
//! deck odds (the deck_size lowest miss chances of any legal entries, links firing for sure); candidates are solved
//! best ceiling first (best product deck, then every deck whose product value times EXACT_RHO reaches the best exact
//! value, scored exactly, as in the per-uma search) until the next ceiling is below the top-k exact value. The top k
//! are distinct trainees, each with her best parents and deck.
use super::*;

#[derive(Deserialize, Clone, Default)]
pub(crate) struct Ancestor {
    pub chara: u32,
    /// (skill group, stars 1..=3) of its white skill sparks
    #[serde(default)] pub sparks: Vec<(u32, u8)>,
    /// race indices (data.json `races`) of the single-race G1s it won
    #[serde(default)] pub wins: Vec<usize>,
    /// (aptitude factor group, stars) of its pink sparks: 11 turf, 12 dirt, 21-24 front runner .. end closer, 31-34
    /// sprint .. long; reported (inheritance chance per aptitude), not part of the odds
    #[serde(default)] pub pinks: Vec<(u32, u8)>,
}
#[derive(Deserialize, Clone, Default)]
pub(crate) struct Legacy {
    pub chara: u32,
    #[serde(default)] pub borrowed: bool,
    #[serde(default)] pub sparks: Vec<(u32, u8)>,
    #[serde(default)] pub wins: Vec<usize>,
    #[serde(default)] pub pinks: Vec<(u32, u8)>,
    /// its own parents (the new trainee's grandparents); empty when unknown
    #[serde(default)] pub parents: Vec<Ancestor>,
}
fn d_top() -> usize { 5 }
#[derive(Deserialize)]
struct CareerJob {
    legacies: Vec<Legacy>,
    /// trainees to consider (chara ids); empty = every owned uma
    #[serde(default)] trainees: Vec<u32>,
    #[serde(default = "d_top")] top: usize,
    /// legacies (indices) that must be among the two parents
    #[serde(default)] fix: Vec<usize>,
    /// one result per trainee (false: the best careers whoever trains, e.g. with the trainee fixed: the best parents)
    #[serde(default = "d_true")] per_trainee: bool,
}

/// One legacy slot of a lineage: affinity parts and per-target stars.
struct Slot { parent: bool, legacy: usize, gp: Option<usize>, chara: u32, rel: f64, race: f64, stars: Vec<u8>, pinks: Vec<(u32, u8)> }

pub(crate) fn run_career(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let dj: DeckJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    let cj: CareerJob = match serde_json::from_value(v.clone()) { Ok(x) => x, Err(e) => return json!({"error": e.to_string()}) };
    CHAIN_EVENTS.with(|c| c.set(dj.chain_events.unwrap_or((CHAIN_N, CHAIN_P))));
    let t_all = Instant::now();
    let mut model = Model::new(d, sk, &sj);
    let ew0 = model.ews.clone();
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let nu = unc.len();
    let ntar = sj.targets.len();
    let uncv = model.uncertain.clone();
    let fixed_t: Vec<usize> = (0..ntar).filter(|i| !uncv.contains(i)).collect();
    let n = d.umas.len();
    let uix = |id: u32| d.umas.iter().position(|u| u.id == id);
    let card_of: HashMap<u32, &Card> = cr.cards.iter().map(|c| (c.id, c)).collect();
    let lock_blocked = |u: usize| dj.lock_cards.iter().any(|id| card_of.get(id).map_or(false, |c| c.blocks(d.umas[u].id)));
    let id_ok = |id: u32| (sj.owned.is_empty() || sj.owned.contains(&id)) && !sj.exclude.contains(&id) && (cj.trainees.is_empty() || cj.trainees.contains(&id));
    let pool: Vec<usize> = (0..n).filter(|&i| id_ok(d.umas[i].id) && !model.no_outfit[i] && !lock_blocked(i)).collect();
    if pool.is_empty() { return json!({"error": "No trainee to search: pick at least one owned uma."}); }
    let legs = &cj.legacies;
    if legs.len() < 2 { return json!({"error": "Add at least two veterans to choose parents from."}); }
    let pool_ids: Vec<u32> = pool.iter().map(|&i| d.umas[i].id).collect();
    let ent = match deck_entries(cr, &dj, &unc, &pool_ids) { Ok(e) => e, Err(e) => return e };
    let space = deck_space(cr, &dj, sk, &unc, &ent);

    // ---- per-trainee best possible deck odds per target (a ceiling for every legal deck) ----
    let size = dj.deck_size;
    let strong: Vec<(Vec<f64>, Vec<f64>)> = (0..space.rel.len()).map(|j| {
        let (mut mw, mut mg) = (vec![1.0f64; nu], vec![1.0f64; nu]);
        src_miss(&space.srcs[j], &vec![1.0; space.kind[j].1 as usize], &mut mw, &mut mg);
        let mh: Vec<f64> = (0..nu).map(|k| mg[k] * mw[k]).collect();
        (mg, mh)
    }).collect();
    let dmax: Vec<Option<(Vec<f64>, Vec<f64>)>> = (0..n).map(|u| {
        if !pool.contains(&u) { return None; }
        let uid = d.umas[u].id;
        let legal: Vec<usize> = (0..space.rel.len()).filter(|&j| !space.cards[space.rel[j]].blocks(uid)).collect();
        let lowest = |f: &dyn Fn(usize) -> f64| { let mut m: Vec<f64> = legal.iter().map(|&j| f(j)).collect(); m.sort_by(|a, b| a.partial_cmp(b).unwrap()); m.iter().take(size).product::<f64>() };
        let (mut dw, mut dg) = (vec![0.0f64; nu], vec![0.0f64; nu]);
        for k in 0..nu {
            let (g, h) = (1.0 - lowest(&|j| strong[j].0[k]), 1.0 - lowest(&|j| strong[j].1[k]));
            // (g, h) = (P(gold hint), P(any hint)); the product formula takes dw with (1 - dw)(1 - dg) = 1 - h
            dg[k] = g; dw[k] = if g < 1.0 { (1.0 - (1.0 - h) / (1.0 - g)).max(0.0) } else { 1.0 };
        }
        Some((dw, dg))
    }).collect();

    // ---- legacies: relation indices, wins ----
    let lix: Vec<Option<usize>> = legs.iter().map(|l| uix(l.chara)).collect();
    let wins_of = |w: &[usize]| { let mut v = w.to_vec(); v.sort(); v.dedup(); v };
    let lw: Vec<Vec<usize>> = legs.iter().map(|l| wins_of(&l.wins)).collect();
    let gw: Vec<Vec<Vec<usize>>> = legs.iter().map(|l| l.parents.iter().map(|g| wins_of(&g.wins)).collect()).collect();
    let shared = |a: &[usize], b: &[usize]| a.iter().filter(|x| b.binary_search(x).is_ok()).count() as f64;
    let pair = |a: Option<usize>, b: Option<usize>| match (a, b) { (Some(a), Some(b)) => d.pair[a][b] as f64, _ => 0.0 };
    let tri = |a: Option<usize>, b: Option<usize>, c: Option<usize>| match (a, b, c) { (Some(a), Some(b), Some(c)) => d.tri[a][b][c] as f64, _ => 0.0 };
    // stars per job target (all targets, job order)
    let stars_of = |sp: &[(u32, u8)]| -> Vec<u8> { sj.targets.iter().map(|t| sp.iter().filter(|s| s.0 == t.group).map(|s| s.1.min(3)).max().unwrap_or(0)).collect() };
    let lstars: Vec<Vec<u8>> = legs.iter().map(|l| stars_of(&l.sparks)).collect();
    let gstars: Vec<Vec<Vec<u8>>> = legs.iter().map(|l| l.parents.iter().map(|g| stars_of(&g.sparks)).collect()).collect();
    let slots_of = |t: usize, a: usize, b: usize| -> Vec<Slot> {
        let ti = Some(t);
        let mut v = vec![];
        for (p, q) in [(a, b), (b, a)] {
            let mut rel = pair(ti, lix[p]) + pair(lix[p], lix[q]); let mut race = 3.0 * shared(&lw[p], &lw[q]);
            let mut gs = vec![];
            for (gi, g) in legs[p].parents.iter().enumerate() {
                let gx = uix(g.chara);
                let (gr, gc) = (tri(ti, lix[p], gx), 3.0 * shared(&lw[p], &gw[p][gi]));
                rel += gr; race += gc;
                gs.push(Slot { parent: false, legacy: p, gp: Some(gi), chara: g.chara, rel: gr, race: gc, stars: gstars[p][gi].clone(), pinks: g.pinks.clone() });
            }
            v.push(Slot { parent: true, legacy: p, gp: None, chara: legs[p].chara, rel, race, stars: lstars[p].clone(), pinks: legs[p].pinks.clone() });
            v.extend(gs);
        }
        v
    };
    // per target (job order): inheritance chance and carriers
    let inherit = |sl: &[Slot]| -> (Vec<f64>, Vec<i32>) {
        let mut q = vec![0.0f64; ntar]; let mut nc = vec![0i32; ntar];
        for k in 0..ntar {
            let mut miss = 1.0f64;
            for s in sl { let st = s.stars[k]; if st > 0 { nc[k] += 1; miss *= 1.0 - (0.03 * st as f64 * (1.0 + (s.rel + s.race) / 100.0)).min(1.0); } }
            q[k] = 1.0 - miss * miss;
        }
        (q, nc)
    };
    let scale = |nc: &[i32]| -> (Vec<f64>, Vec<f64>) {
        (uncv.iter().map(|&i| 1.1f64.powi(nc[i] - 6)).collect(), fixed_t.iter().map(|&i| 1.1f64.powi(nc[i] - 6)).collect())
    };
    let set_trainee = |model: &mut Model, t: usize, q: &[f64]| {
        model.ews[t] = ew0[t].iter().map(|o| o.iter().zip(&uncv).map(|(e, &i)| 1.0 - (1.0 - e) * (1.0 - q[i])).collect()).collect();
    };

    // ---- candidates and their ceilings ----
    struct Cand { t: usize, a: usize, b: usize, ub: f64, c: f64, q: Vec<f64> }
    let mut cands: Vec<Cand> = vec![];
    for &t in &pool {
        let tid = d.umas[t].id;
        let (dw, dg) = dmax[t].as_ref().unwrap();
        for a in 0..legs.len() { for b in a + 1..legs.len() {
            let (ca, cb) = (legs[a].chara, legs[b].chara);
            if ca == cb || ca == tid || cb == tid || (legs[a].borrowed && legs[b].borrowed) || !cj.fix.iter().all(|&f| f == a || f == b) { continue; }
            let sl = slots_of(t, a, b);
            let (q, nc) = inherit(&sl);
            let (su, sf) = scale(&nc);
            let c = su.iter().chain(&sf).product::<f64>();
            set_trainee(&mut model, t, &q);
            let p = (0..model.n_outfits(t)).map(|o| model.outfit_fast(t, o, 0.0, dw, dg)).fold(0.0f64, f64::max);
            cands.push(Cand { t, a, b, ub: EXACT_RHO * c * model.fixed_factor() * p, c, q });
        } }
    }
    if cands.is_empty() { return json!({"error": "No legal career: a trainee needs two veterans of other characters (not her own, not the same as each other), at most one of them borrowed."}); }
    cands.sort_by(|x, y| y.ub.partial_cmp(&x.ub).unwrap());

    // ---- solve best ceiling first ----
    let top = cj.top.max(1);
    let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(EXACT_BUDGET);
    let mut work = 0usize; let mut complete = true;
    // (value C x E, candidate, best deck, its exact E)
    let mut done: Vec<(f64, usize, Option<Sel>)> = vec![];
    let kth = |done: &[(f64, usize, Option<Sel>)]| if done.len() < top { f64::NEG_INFINITY } else { done[top - 1].0 };
    let (mut n_solved, mut n_bounded, mut n_scored) = (0usize, 0usize, 0usize);
    let mut next = 0;
    let ents_of = |sel: &Sel| -> Vec<(Src, Kind)> { sel.ents.iter().map(|&j| (space.srcs[j].clone(), space.kind[j])).collect() };
    // one result per trainee: a candidate only matters if it beats the k-th best and its trainee's best so far
    let per_t = cj.per_trainee;
    let thr = |done: &[(f64, usize, Option<Sel>)], t: usize| if !per_t { kth(done) } else { done.iter().find(|x| cands[x.1].t == t).map_or(kth(done), |x| x.0.max(kth(done))) };
    // How far trainee t's best exact value (before the spark scale) can rise from chances q0 to q. A target the outfit
    // lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no hint, e: her own
    // event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 - q0_k)) (n = 1,
    // g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products of these factors,
    // so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of the product.
    let sts0: Vec<Vec<Vec<u8>>> = model.opts.iter().map(|m| m.iter().map(|x| x.0.clone()).collect()).collect();
    let rise = |t: usize, q: &[f64], q0: &[f64]| -> f64 {
        sts0[t].iter().zip(&ew0[t]).map(|(st, ew)| uncv.iter().enumerate().map(|(k, &i)| {
            if st[k] > 0 || q[i] <= q0[i] { 1.0 } else { (1.0 - (1.0 - ew[k]) * (1.0 - q[i])) / (1.0 - (1.0 - ew[k]) * (1.0 - q0[i])) }
        }).product::<f64>()).fold(0.0f64, f64::max)
    };
    // per trainee, her fully checked candidates: (candidate, exact best value before the spark scale, its deck)
    let mut refs: Vec<Vec<(usize, f64, Sel)>> = vec![vec![]; n];
    while next < cands.len() && cands[next].ub * (1.0 + 1e-9) >= kth(&done) {
        let ci = next; next += 1;
        let (t, c) = (cands[ci].t, cands[ci].c);
        let th = thr(&done, t);
        if cands[ci].ub * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
        let rb = refs[t].iter().map(|r| c * r.1 * rise(t, &cands[ci].q, &cands[r.0].q)).fold(f64::INFINITY, f64::min);
        if rb * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
        let uid = d.umas[t].id;
        set_trainee(&mut model, t, &cands[ci].q);
        let n0 = space.nodes.get();
        let (r, _) = space.search(&model, &[t], &[uid], &[(vec![(0, 0.0)], 0.0)], true, f64::NEG_INFINITY, 1.0, 1);
        work += 2 * (space.nodes.get() - n0) + 10_000;
        let Some((pv, sel)) = r.into_iter().next() else { continue };
        // the candidate's own ceiling from its best product deck
        if EXACT_RHO * c * pv * (1.0 + 1e-9) < th { n_bounded += 1; continue; }
        n_solved += 1;
        let score = |sel: &Sel| joint(&model, t, &[0.0], &ents_of(sel), sel.filler, ntar, false)[0].0;
        let mut best = (score(&sel), sel); n_scored += 1; work += 200;
        // her other checked candidates' decks are good starts (a higher start leaves fewer decks to check)
        for r in &refs[t] { let x = score(&r.2); n_scored += 1; work += 200; if x > best.0 { best = (x, r.2.clone()); } }
        let mut full = false;
        if work <= budget {
            let n0 = space.nodes.get();
            let (mut more, over) = space.search(&model, &[t], &[uid], &[(vec![(0, 0.0)], best.0)], false, f64::NEG_INFINITY, EXACT_RHO, EXACT_CAP);
            work += 2 * (space.nodes.get() - n0) + 5 * more.len() + 10_000;
            full = !over;
            more.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            if full { for (v, s) in more {
                if EXACT_RHO * v * (1.0 + 1e-9) < best.0 { break; }
                if work > budget { full = false; break; }
                if s.ents == best.1.ents && s.plain == best.1.plain && s.filler == best.1.filler { continue; }
                let e: Vec<(&Src, Kind)> = s.ents.iter().map(|&j| (&space.srcs[j], space.kind[j])).collect();
                if let Some(ub) = upper_na(&model, t, &[0.0], &e, s.filler) { work += 25; if ub[0] * (1.0 + 1e-9) < best.0 { continue; } }
                let x = score(&s); n_scored += 1; work += 200;
                if x > best.0 { best = (x, s); }
            } }
        }
        if full { refs[t].push((ci, best.0, best.1.clone())); } else { complete = false; }
        let val = c * best.0;
        if per_t { if let Some(i) = done.iter().position(|x| cands[x.1].t == t) { if done[i].0 >= val { continue; } done.remove(i); } }
        let pos = done.iter().position(|x| x.0 < val).unwrap_or(done.len());
        done.insert(pos, (val, ci, Some(best.1)));
    }
    let next_bound = if next < cands.len() { cands[next].ub } else { 0.0 };

    // ---- report ----
    let name = |id: u32| uix(id).map_or(format!("Uma {id}"), |i| d.umas[i].name.clone());
    let results: Vec<Value> = done.iter().take(top).map(|(_, ci, sel)| {
        let Cand { t, a, b, ub, .. } = cands[*ci];
        let uid = d.umas[t].id;
        let sl = slots_of(t, a, b);
        let (q, nc) = inherit(&sl);
        set_trainee(&mut model, t, &q);
        let sel = sel.as_ref().unwrap();
        let (su, sf) = scale(&nc);
        let mut ext = JointExt { su, sf, marg: vec![] };
        let j = joint_fast(&model, t, &[0.0], &ents_of(sel), sel.filler, ntar, true, Some(&mut ext))[0];
        let o = j.2;
        let mut p = vec![0.0f64; ntar];
        for (pos, &i) in model.uncertain.iter().enumerate() { p[i] = ext.marg[o * nu + pos]; }
        for (pos, &i) in fixed_t.iter().enumerate() { p[i] = GEN_GOLD * ext.sf[pos]; }
        let (st, cards) = &model.opts[t][o];
        let covers: Vec<Value> = st.iter().zip(&model.uncertain).filter(|(lv, i)| **lv > 0 && !(**lv == 1 && sj.targets[**i].guaranteed == 1))
            .map(|(lv, i)| json!({"group": sj.targets[*i].group, "level": lv})).collect();
        let default_st: Vec<u8> = model.uncertain.iter().map(|&i| if sj.targets[i].guaranteed == 1 { 1 } else { 0 }).collect();
        let slots: Vec<Value> = sl.iter().map(|s| json!({"kind": if s.parent { "parent" } else { "grandparent" }, "legacy": s.legacy, "gp": s.gp,
            "chara": s.chara, "name": name(s.chara), "rel": s.rel, "race": s.race, "aff": s.rel + s.race,
            "stars": s.stars, "pinks": s.pinks})).collect();
        let targets: Vec<Value> = (0..ntar).map(|i| json!({"group": sj.targets[i].group, "q": q[i], "carriers": nc[i], "p": p[i],
            "gold": sj.targets[i].guaranteed == 2})).collect();
        let pd = space.pdeck(sel, &[uid]);
        // pink sparks: per aptitude the chance at least one of the two inheritance rolls passes it on (1/3/5% per star)
        let mut pg: Vec<u32> = sl.iter().flat_map(|s| s.pinks.iter().map(|x| x.0)).collect(); pg.sort(); pg.dedup();
        let pinks: Vec<Value> = pg.iter().map(|&g| {
            let (mut miss, mut nc) = (1.0f64, 0);
            for s in &sl { if let Some(st) = s.pinks.iter().filter(|x| x.0 == g).map(|x| x.1.min(3)).max() { if st > 0 { nc += 1;
                miss *= 1.0 - ([0.0, 0.01, 0.03, 0.05][st as usize] * (1.0 + (s.rel + s.race) / 100.0)).min(1.0); } } }
            json!({"group": g, "q": 1.0 - miss * miss, "carriers": nc})
        }).collect();
        json!({"trainee": {"id": uid, "name": d.umas[t].name, "outfits": if *st == default_st { vec![] } else { cards.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>() }, "covers": covers},
               "parents": [a, b], "slots": slots, "targets": targets, "p_all": j.0, "p_minus1": j.1, "expected": p.iter().sum::<f64>(),
               "bound": ub, "deck": pd.out(&unc), "pinks": pinks})
    }).collect();
    let proven = complete && !done.is_empty() && (done.len() >= top.min(cands.len()) || next >= cands.len()) && next_bound < done[0].0;
    json!({"results": results, "candidates": cands.len(), "solved": n_solved, "bounded": n_bounded, "scored": n_scored, "skipped": cands.len() - next,
           "next_bound": next_bound, "complete": complete, "proven_top": proven, "cap": EXACT_RHO, "timing_s": t_all.elapsed().as_secs_f64()})
}
