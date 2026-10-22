//! Per-uma exact values, chance-grid ceilings and skyline clusters, including shared product ceilings.
//! These tables bound deck and loop searches; they do not enumerate loop structures or report results.
use super::{CardEntryKey, DeckSources, ExactQueryKey, Multi, SelectionKey, lin_of, qkey};
use crate::deck::caps::src_class;
use crate::deck::enumerate::{LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome};
use crate::deck::joint::upper::upper_na;
use crate::deck::joint::{JointChances, joint, joint_b};
use crate::deck::kept::{Cl, DeckEval, DeckId, PDeck, k_filter, split};
use crate::deck::sources::{SourceEntry, Sources, TargetSources};
use crate::deck::space::{HintCoordinate, UCHK};
use crate::skills::{Model, OutfitOption};
use crate::uma::{UmaIndex, UmaTable};
use crate::{Data, Instant, Sched, members, pair_ceilings, probs, wval};
use std::borrow::Cow;
use std::rc::Rc;

/// Points on the chance grid of the ceiling tables
pub(super) const NG: usize = 64;
/// A grid cell's exact value and the cap under which it was certified.
#[derive(Clone, Copy)]
pub(super) struct ExactCeiling {
    pub(super) value: f64,
    pub(super) certified_cap: f64,
}

/// The chance grid the ceiling tables are kept on: NG points from the floor to the highest chance any uma reaches
pub(super) struct Grid {
    minimum_chance: f64,
    step: f64,
    chances: Vec<f64>,
}
impl Grid {
    pub(super) fn new(qlo: f64, qhi: f64) -> Self {
        let step = if qhi > qlo { (qhi - qlo) / (NG - 1) as f64 } else { 0.0 };
        let q: Vec<f64> = (0..NG).map(|i| if i == NG - 1 { qhi } else { qlo + step * i as f64 }).collect();
        Self { minimum_chance: qlo, step, chances: q }
    }
    /// the first grid point at or above q (the last when the grid is one point)
    pub(super) fn idx(&self, q: f64) -> usize {
        if self.step == 0.0 {
            return NG - 1;
        }
        let mut i = (((q - self.minimum_chance) / self.step).ceil().max(0.0) as usize).min(NG - 1);
        while self.chances[i] < q && i < NG - 1 {
            i += 1;
        }
        i
    }
}

pub(super) struct Best {
    pub(super) sum: f64,
    pub(super) deck: Option<PDeck>,
    pub(super) lineage_chances: Vec<JointChances>,
}
// per (uma, chances): the best found, decks already scored or bounded, and the cap it is exact under
pub(super) struct ExactCacheEntry {
    best: Option<ScoredDeck<PDeck>>,
    /// Detached decks already scored exactly (including skyline seeds).
    scored_decks: crate::FxSet<DeckId>,
    /// Band selections already scored or rejected by the proven bound.
    checked_selections: crate::FxSet<SelectionKey>,
    /// Highest completed cap; -1 means unseeded, 1 means seeded only.
    certified_cap: f64,
    reported_best: Option<Rc<Best>>,
}

/// Each pair's race-win ceiling: the fixed schedules' values where the job fixes them (0 without races), else the
/// pair ceilings
pub(super) fn pair_table(
    d: &Data,
    pool: &[UmaIndex],
    fixed_s: Option<&UmaTable<Option<Sched>>>,
    no_race: bool,
) -> UmaTable<UmaTable<f64>> {
    let n = d.umas.len();
    match fixed_s {
        Some(fs) => {
            let ps: UmaTable<Option<Vec<f64>>> =
                fs.indexed().map(|(i, s)| s.as_ref().map(|s| probs(d, &d.umas[i], s))).collect();
            let mut m = UmaTable(vec![UmaTable(vec![0.0f64; n]); n]);
            if !no_race {
                for &a in pool {
                    for &b in pool {
                        if a != b {
                            m[a][b] = wval(ps[a].as_ref().unwrap(), ps[b].as_ref().unwrap());
                        }
                    }
                }
            }
            m
        }
        None => pair_ceilings(d, pool),
    }
}

// A uma's value depends only on the targets some outfit of hers lacks: (g, h) = (P(gold), P(any hint)) where
// an outfit lacks the skill, g alone where her outfits have at most the white. Decks are thinned per class of
// umas with the same coordinates: a deck another beats on all of them never wins for those umas.
fn ucoords(model: &Model, u: UmaIndex, target_count: usize) -> Vec<HintCoordinate> {
    let mut c = vec![];
    for k in 0..target_count {
        if model.opts[u].iter().any(|OutfitOption { coverage: st, .. }| st[k] == 0) {
            c.push(HintCoordinate { target: k, gold: true });
            c.push(HintCoordinate { target: k, gold: false });
        } else if model.opts[u].iter().any(|OutfitOption { coverage: st, .. }| st[k] == 1) {
            c.push(HintCoordinate { target: k, gold: true });
        }
    }
    c
}
/// The decks of `list` no other beats on the coordinates `co`, by index, ascending
fn thin(list: &[PDeck], co: &[HintCoordinate]) -> Vec<usize> {
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
                .map(
                    |&HintCoordinate { target: k, gold: g }| {
                        if g { x.dg[k] } else { 1.0 - (1.0 - x.dw[k]) * (1.0 - x.dg[k]) }
                    },
                )
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
}
/// The pool's classes of umas with the same coordinates: each class's skyline decks, and each uma's class
pub(super) fn classes(
    model: &Model,
    pool: &[UmaIndex],
    sky: &[PDeck],
    target_count: usize,
    n: usize,
) -> (Vec<Vec<usize>>, UmaTable<usize>) {
    // the full coordinate set needs no thinning: the skyline is already thin on it
    let full: Vec<HintCoordinate> = (0..target_count)
        .flat_map(|k| [HintCoordinate { target: k, gold: true }, HintCoordinate { target: k, gold: false }])
        .collect();
    let mut class_ix: crate::FxMap<Vec<HintCoordinate>, usize> = crate::FxMap::default();
    let mut class_sky: Vec<Vec<usize>> = vec![];
    let mut ucls = UmaTable(vec![usize::MAX; n]);
    for &u in pool {
        let co = ucoords(model, u, target_count);
        let ci = if let Some(&c) = class_ix.get(&co) {
            c
        } else {
            class_sky.push(if co == full { (0..sky.len()).collect() } else { thin(sky, &co) });
            class_ix.insert(co, class_sky.len() - 1);
            class_sky.len() - 1
        };
        ucls[u] = ci;
    }
    (class_sky, ucls)
}
/// Clusters of the whole skyline (one cluster of every deck without uncertain targets); none with per-uma decks
pub(super) fn full_clusters(sky: &[PDeck], target_count: usize, per_uma: bool) -> Vec<Cl> {
    if per_uma {
        vec![]
    } else {
        let mut ix: Vec<usize> = (0..sky.len()).collect();
        let mut out = vec![];
        if target_count > 0 && !ix.is_empty() {
            split(sky, &mut ix, target_count, &mut out);
        } else {
            out.push(Cl { mdw: vec![], mdg: vec![], mem: ix });
        }
        out
    }
}

/// Clusters for each class retain the skyline's ordering before the median splits.
pub(super) fn class_clusters(skyline: &[PDeck], class_decks: &[Vec<usize>], target_count: usize) -> Vec<Vec<Cl>> {
    class_decks
        .iter()
        .map(|decks| {
            let mut indices = decks.clone();
            let mut clusters = vec![];
            if target_count > 0 && !indices.is_empty() {
                split(skyline, &mut indices, target_count, &mut clusters);
            }
            clusters
        })
        .collect()
}

impl<'data> Multi<'data> {
    fn uview(&self, u: UmaIndex) -> &Vec<usize> {
        &self.class_decks[self.class_by_trainee[u]]
    }

    // ---- product-odds ceilings ----

    /// max(init, max over u's class of val(deck)), with bound(cluster maxima) >= val of every member
    /// (the clusters' bounds in a reused buffer, popped from a heap in the stable sort's order: `ByBound`)
    fn bb_max(&self, u: UmaIndex, init: f64, val: &dyn Fn(&[f64], &[f64]) -> f64) -> f64 {
        let cls = &self.class_clusters[self.class_by_trainee[u]];
        let sky = &self.inputs.skyline;
        if cls.is_empty() {
            return self.uview(u).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max);
        }
        let mut bs = std::mem::take(&mut *self.cluster_bound_scratch.borrow_mut());
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
        *self.cluster_bound_scratch.borrow_mut() = h.into_vec();
        best
    }

    /// B[u][i], memoised in `relaxed_ceiling_cache` at first need.
    pub(super) fn btab(&self, u: UmaIndex, i: usize) -> f64 {
        let v = self.relaxed_ceiling_cache.borrow()[u][i];
        if !v.is_nan() {
            return v;
        }
        let (model, sky) = (self.inputs.model, &self.inputs.skyline);
        let q = self.grid.chances[i];
        let init = model.trainee_fast(u, q, &self.no_hint_chances, &self.no_hint_chances);
        let v = if (0.0..=1.0).contains(&q) {
            self.bb_max(u, init, &|dw, dg| model.trainee_fast(u, q, dw, dg))
        } else {
            self.uview(u).iter().map(|&j| model.trainee_fast(u, q, &sky[j].dw, &sky[j].dg)).fold(init, f64::max)
        };
        self.relaxed_ceiling_cache.borrow_mut()[u][i] = v;
        v
    }

    /// The best m skyline decks by `val` among those passing `ok` (cluster maxima bound their members, best bound
    /// first; `val` must rise with every odds coordinate)
    // (as `bb_max`: the clusters in the stable sort's order, equal bounds by index, from a heap over a reused buffer)
    pub(super) fn sky_top(
        &self,
        m: usize,
        val: &dyn Fn(&[f64], &[f64]) -> f64,
        ok: &dyn Fn(&PDeck) -> bool,
    ) -> Vec<(f64, usize)> {
        let (sky, full_cl, target_count) = (&self.inputs.skyline, &self.skyline_clusters, self.target_count);
        let mut bs = std::mem::take(&mut *self.skyline_bound_scratch.borrow_mut());
        bs.clear();
        bs.extend(full_cl.iter().enumerate().map(|(i, c)| ByBound {
            bound: if target_count > 0 { val(&c.mdw, &c.mdg) } else { f64::INFINITY },
            cluster: i,
        }));
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
        *self.skyline_bound_scratch.borrow_mut() = h.into_vec();
        top
    }

    /// Shared-deck ceiling: the best single skyline deck (legal or not) over the structure's lineages, product odds.
    pub(super) fn relaxed_shared(&self, f: [UmaIndex; 4], qa: &[f64; 12]) -> f64 {
        let model = self.inputs.model;
        let ln = lin_of(&f);
        let val = |dw: &[f64], dg: &[f64]| {
            (0..ln.len()).map(|l| model.trainee_fast(f[ln[l].trainee], qa[l], dw, dg)).sum::<f64>()
        };
        self.sky_top(1, &val, &|_| true).first().map_or(0.0, |x| x.0) * self.fixed / ln.len() as f64 * (1.0 + 1e-12)
    }

    /// ceiling with each uma's best skyline deck (legal or not), the same deck across her lineages
    pub(super) fn relaxed(&self, f: [UmaIndex; 4], qa: &[f64; 12]) -> f64 {
        let (model, sky) = (self.inputs.model, &self.inputs.skyline);
        let mut tot = 0.0;
        let ln = lin_of(&f);
        for t in 0..members(&f).len() {
            let qs: Vec<f64> = (0..ln.len()).filter(|&l| ln[l].trainee == t).map(|l| qa[l]).collect();
            let init = qs
                .iter()
                .map(|&q| model.trainee_fast(f[t], q, &self.no_hint_chances, &self.no_hint_chances))
                .sum::<f64>();
            let val = |dw: &[f64], dg: &[f64]| qs.iter().map(|&q| model.trainee_fast(f[t], q, dw, dg)).sum::<f64>();
            tot += if qs.iter().all(|q| (0.0..=1.0).contains(q)) {
                self.bb_max(f[t], init, &val)
            } else {
                self.uview(f[t]).iter().map(|&j| val(&sky[j].dw, &sky[j].dg)).fold(init, f64::max)
            };
        }
        self.fixed * tot / ln.len() as f64 * (1.0 + 1e-12)
    }
    // ---- exact per-uma values ----
    // Loops are ranked by exact odds. A trainee's exact value at her lineages' inheritance chances is the best exact
    // P(all) over every legal deck (`DeckSpace`, not only the skyline). It is seeded with her best legal skyline decks
    // by product odds; other decks are bounded or scored under their structure-class caps, limited by the pass's cap.
    // Exactness assumes each deck's exact/product ratio stays within that cap and the pass finishes within budget.

    /// A deck's sources: the deck space's entry for (card, end link, variant) where it has one of that card, borrowed;
    /// else built. With the filler count.
    pub(super) fn ents_of(&self, x: &PDeck) -> DeckSources<'data> {
        let space = self.inputs.deck_space;
        let source_builder = Sources { dj: self.inputs.deck_job, unc: &self.target_groups };
        let e: Vec<SourceEntry<Cow<TargetSources>>> = x
            .ids
            .iter()
            .enumerate()
            .filter_map(|(i, id)| {
                self.card_of.get(id).map(|c| {
                    let st = x.ends.get(i).copied().filter(|&e| e > 0);
                    let v = x.vars.get(i).copied().unwrap_or(0);
                    match self.entry_by_card.get(&CardEntryKey { card_id: *id, end_link: st.unwrap_or(0), variant: v })
                    {
                        Some(&j) if std::ptr::eq(space.cards[space.card_indices[j]], *c) => {
                            SourceEntry { sources: Cow::Borrowed(&space.sources[j]), kind: space.kind[j] }
                        }
                        _ => SourceEntry {
                            sources: Cow::Owned(source_builder.src(c, st, v)),
                            kind: source_builder.kind(c, st),
                        },
                    }
                })
            })
            .collect();
        DeckSources {
            entries: e,
            filler_count: self.inputs.deck_job.deck_size.saturating_sub(x.ids.len() + x.plain_ids.len()),
        }
    }

    /// Exact value of trainee u at chances qs under cap lv: the best exact P(all) over her best legal skyline decks
    /// and every legal deck whose product value times lv reaches it (each first through the proven bound).
    pub(super) fn exact_best(&self, u: UmaIndex, qs: &[f64], lv: f64) -> Rc<Best> {
        let key = ExactQueryKey { trainee: u, chances: qkey(qs) };
        if let Some(entry) = self.exact_cache.borrow().get(&key)
            && entry.certified_cap >= lv
            && let Some(result) = &entry.reported_best
        {
            return Rc::clone(result);
        }
        let mut ent = self.exact_cache.borrow_mut().remove(&key).unwrap_or(ExactCacheEntry {
            best: None,
            scored_decks: crate::FxSet::default(),
            checked_selections: crate::FxSet::default(),
            certified_cap: -1.0,
            reported_best: None,
        });
        if ent.certified_cap < 0.0 {
            self.seed_exact(u, qs, &mut ent);
            ent.certified_cap = 1.0;
        }
        if lv > ent.certified_cap {
            self.raise_exact(u, qs, lv, &mut ent);
        }
        let b = Rc::new(self.best_of(u, qs, &ent));
        ent.reported_best = Some(Rc::clone(&b));
        self.exact_cache.borrow_mut().insert(key, ent);
        b
    }

    /// Scores deck x exactly for trainee u at chances qs (once per entry) and keeps it if it is the best
    fn score_exact(&self, u: UmaIndex, qs: &[f64], x: &PDeck, ent: &mut ExactCacheEntry) {
        if !ent.scored_decks.insert(x.id()) {
            return;
        }
        let (model, tally) = (self.inputs.model, &self.tally);
        let uid = self.inputs.data.umas[u].id;
        let t0 = Instant::now();
        let DeckSources { entries: e, filler_count: fl } = self.ents_of(x);
        let j = joint_b(model, u, qs, &e, fl, self.requested_target_count, false);
        tally.score_seconds.set(tally.score_seconds.get() + t0.elapsed().as_secs_f64());
        if std::env::var("UPPER_CHECK").is_ok() {
            let er: Vec<SourceEntry<&TargetSources>> =
                e.iter().map(|entry| SourceEntry { sources: &*entry.sources, kind: entry.kind }).collect();
            if let Some(ub) = upper_na(model, u, qs, &er, fl) {
                for (i, y) in j.iter().enumerate() {
                    assert!(
                        ub[i] >= y.all * (1.0 - 1e-12),
                        "upper bound below exact: {} < {} for {:?}",
                        ub[i],
                        y.all,
                        x.names
                    );
                    UCHK.with(|c| {
                        let mut v = c.get();
                        v.0 += 1;
                        v.1 = v.1.max(ub[i] / y.all);
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
        tally.scored_decks.set(tally.scored_decks.get() + 1);
        tally.spend(200);
        let k = src_class(e.iter().map(|y| &*y.sources));
        for (i, &q) in qs.iter().enumerate() {
            let p = self.fixed * model.trainee_fast(u, q, &x.dw, &x.dg);
            if p > 0.0 {
                tally.ratio_at(k, j[i].all / p, uid, x);
            }
        }
        let sv: f64 = j.iter().map(|y| y.all).sum();
        if ent.best.as_ref().is_none_or(|b| sv > b.value) {
            ent.best = Some(ScoredDeck { value: sv, deck: x.clone() });
        }
    }

    /// seed: her best legal skyline decks by product value (or the best legal deck when none is on the skyline)
    fn seed_exact(&self, u: UmaIndex, qs: &[f64], ent: &mut ExactCacheEntry) {
        let (model, space) = (self.inputs.model, self.inputs.deck_space);
        let uid = self.inputs.data.umas[u].id;
        let t0 = Instant::now();
        // the first 3 of a stable sort by value, highest first: each deck goes after every kept one at least as high
        let mut seeds: Vec<(f64, &PDeck)> = Vec::with_capacity(4);
        // (the value first, the blocks lookups only for a deck that would enter: one that can't is passed over blocked
        // or not, so the seeds are the same)
        for x in self.uview(u).iter().map(|&j| &self.inputs.skyline[j]) {
            let t = self.fixed * qs.iter().map(|&q| model.trainee_fast(u, q, &x.dw, &x.dg)).sum::<f64>();
            let at = seeds
                .iter()
                .position(|s| s.0.partial_cmp(&t).unwrap() == std::cmp::Ordering::Less)
                .unwrap_or(seeds.len());
            if at < 3 && !x.blocks(uid, &self.card_of) {
                seeds.insert(at, (t, x));
                seeds.truncate(3);
            }
        }
        self.tally.seed_seconds.set(self.tally.seed_seconds.get() + t0.elapsed().as_secs_f64());
        for (_, x) in seeds.iter().take(3) {
            self.score_exact(u, qs, x, ent);
        }
        if ent.best.is_none() {
            let SearchOutcome { decks: r, .. } = space.search(
                model,
                &[u],
                &[uid],
                &[SearchClause {
                    lineages: qs.iter().map(|&q| LineageChance { trainee: 0, inheritance: q }).collect(),
                    threshold: 0.0,
                }],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            if let Some(ScoredDeck { value: _, deck: sel }) = r.first() {
                let x = space.pdeck(sel, &[uid]);
                self.score_exact(u, qs, &x, ent);
            }
        }
    }

    /// Raises an entry's certified cap to lv: every legal deck that might beat its best under the cap is scored (after
    /// the proven bound); a deck search cut short or the budget running out certifies nothing and aborts the pass
    fn raise_exact(&self, u: UmaIndex, qs: &[f64], lv: f64, ent: &mut ExactCacheEntry) {
        let (model, space, tally) = (self.inputs.model, self.inputs.deck_space, &self.tally);
        let uid = self.inputs.data.umas[u].id;
        let mut complete = true;
        let Some(thr) = ent.best.as_ref().map(|b| b.value) else {
            ent.certified_cap = lv;
            return;
        };
        tally.band_decks.set(tally.band_decks.get() + 1);
        let t0 = Instant::now();
        let n0 = space.nodes.get();
        let SearchOutcome { decks: mut cands, incomplete } = space.search(
            model,
            &[u],
            &[uid],
            &[SearchClause {
                lineages: qs.iter().map(|&q| LineageChance { trainee: 0, inheritance: q }).collect(),
                threshold: thr,
            }],
            SearchMode::Band { ratio_cap: lv, max_decks: space.band_cap() },
        );
        tally.spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
        tally.band_seconds.set(tally.band_seconds.get() + t0.elapsed().as_secs_f64());
        if incomplete.is_some() {
            tally.cap_hit.set(true);
            complete = false;
        }
        cands.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        if complete {
            for ScoredDeck { value: v, deck: sel } in &cands {
                if lv * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().value {
                    break;
                }
                if tally.work.get() > tally.budget.get() {
                    complete = false;
                    break;
                }
                // below the best at its own class's cap (the best rose since the band was built)
                if space.sel_cap(sel, lv) * v * (1.0 + 1e-9) < ent.best.as_ref().unwrap().value {
                    continue;
                }
                if !ent.checked_selections.insert(SelectionKey {
                    entries: sel.entries.clone(),
                    plain_pals: sel.plain_pals,
                    filler_cards: sel.filler_cards,
                }) {
                    continue;
                }
                // the proven bound first: a deck it puts below the best exact value cannot beat it
                let e: Vec<SourceEntry<&TargetSources>> = sel
                    .entries
                    .iter()
                    .map(|&j| SourceEntry { sources: &space.sources[j], kind: space.kind[j] })
                    .collect();
                let t1 = Instant::now();
                let ubo = upper_na(model, u, qs, &e, sel.filler_cards);
                tally.bound_seconds.set(tally.bound_seconds.get() + t1.elapsed().as_secs_f64());
                if let Some(ub) = ubo {
                    tally.spend(25);
                    if ub.iter().sum::<f64>() * (1.0 + 1e-9) < ent.best.as_ref().unwrap().value {
                        tally.bound_rejections.set(tally.bound_rejections.get() + 1);
                        continue;
                    }
                }
                let x = space.pdeck(sel, &[uid]);
                self.score_exact(u, qs, &x, ent);
            }
        }
        if complete {
            ent.certified_cap = lv;
        } else {
            tally.aborted.set(true);
        }
    }

    /// The entry's best with its per-lineage values (the deckless value when no deck is legal)
    fn best_of(&self, u: UmaIndex, qs: &[f64], ent: &ExactCacheEntry) -> Best {
        if let Some(ScoredDeck { value: sum, deck: x }) = &ent.best {
            let DeckSources { entries: e, filler_count: fl } = self.ents_of(x);
            let per = joint_b(self.inputs.model, u, qs, &e, fl, self.requested_target_count, true);
            Best { sum: *sum, deck: Some(x.clone()), lineage_chances: per }
        } else {
            let per = joint(self.inputs.model, u, qs, &[], 0, self.requested_target_count, true);
            Best { sum: per.iter().map(|y| y.all).sum(), deck: None, lineage_chances: per }
        }
    }

    /// X[u][i] under cap lv, memoised in `exact_ceiling_cache` when certified under lv or more.
    fn xtab(&self, u: UmaIndex, i: usize, lv: f64) -> f64 {
        let ExactCeiling { value: v, certified_cap: l } = self.exact_ceiling_cache.borrow()[u][i];
        if !v.is_nan() && l >= lv {
            return v;
        }
        let v = self.exact_best(u, &[self.grid.chances[i]], lv).sum;
        // a value from a deck search the budget cut short is not exact under lv: not kept
        if !self.tally.aborted.get() {
            self.exact_ceiling_cache.borrow_mut()[u][i] = ExactCeiling { value: v, certified_cap: lv };
        }
        v
    }

    /// Ceiling of a cell (uma, grid point) under cap lv: the exact value if computed under lv or more, else the lowest
    /// such value at a higher grid point, else the product-odds bound times lv; (bound, exact?)
    pub(super) fn cell(&self, u: UmaIndex, i: usize, lv: f64) -> (f64, bool) {
        let mut b = lv * self.fixed * self.btab(u, i) * (1.0 + 1e-12);
        let xb = self.exact_ceiling_cache.borrow();
        let row = &xb[u];
        for j in i..NG {
            if !row[j].value.is_nan() && row[j].certified_cap >= lv {
                b = b.min(row[j].value * (1.0 + 1e-12));
                return (b, j == i);
            }
        }
        (b, false)
    }

    /// Exact ceiling of a foursome at chances `qa`, refined lazily: while it beats `kth`, the loosest inexact term is
    /// tightened (first computed at the uma's highest grid point, then halfway down towards its own) until the ceiling
    /// stops beating `kth` or every term is exact. Each step is one exact deck search for one uma.
    pub(super) fn exact_ceiling(&self, f: [UmaIndex; 4], qa: &[f64; 12], kth: f64, lv: f64) -> f64 {
        loop {
            let mut tot = 0.0;
            let mut worst: Option<(f64, UmaIndex, usize)> = None;
            let ln = lin_of(&f);
            for l in 0..ln.len() {
                let (u, i) = (f[ln[l].trainee], self.grid.idx(qa[l]));
                let (b, ex) = self.cell(u, i, lv);
                tot += b;
                if !ex && worst.is_none_or(|w| b > w.0) {
                    worst = Some((b, u, i));
                }
            }
            let ce = tot / ln.len() as f64;
            let Some((_, u, i)) = worst else { return ce };
            if ce <= kth * (1.0 + 4e-12) || self.tally.aborted.get() {
                return ce;
            }
            let j = {
                let xb = self.exact_ceiling_cache.borrow();
                let row = &xb[u];
                (i..NG).find(|&j| !row[j].value.is_nan() && row[j].certified_cap >= lv)
            };
            let tgt = match j {
                None => i.max(self.grid.idx(self.maximum_chances[u])),
                Some(j) => i + (j - i) / 2,
            };
            self.xtab(u, tgt, lv);
        }
    }
}
/// (bound, cluster) ordered as the stable sort by bound, highest first, visits them: a heap's pops give the higher bound
/// first and equal bounds (`partial_cmp`: 0 and -0 tie) in index order, the same sequence (a NaN bound panics in
/// either)
pub(super) struct ByBound {
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
