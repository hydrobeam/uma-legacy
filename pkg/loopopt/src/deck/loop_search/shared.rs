//! Exact shared-deck scans and their cached results, seeded from legal skyline decks.
//! Loop enumeration and certification across structures belong to `bands`.
use super::{DeckSources, Multi, SharedQueryKey, lin_of, qkey};
use crate::Instant;
use crate::data::Data;
use crate::deck::caps::src_class;
use crate::deck::dominance::{DeckId, PDeck};
use crate::deck::enumerate::{LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome};
use crate::deck::joint::joint_b;
use crate::deck::joint::upper::upper_na;
use crate::deck::sources::{SourceEntry, TargetSources};
use crate::optimize::members;
use crate::uma::UmaIndex;
use std::rc::Rc;

/// A loop's shared-deck list: (exact sum over its lineages, deck), best first
pub(super) type SharedDecks = Rc<Vec<ScoredDeck<PDeck>>>;

/// A shared-deck cache result, valid below its cap and above its floor.
pub(super) struct SharedResult {
    certified_cap: f64,
    floor: f64,
    decks: SharedDecks,
}

/// One loop's shared-deck scoring: its members' ids, (trainee, chance) per lineage and each member's chances, the
/// floor, and the decks scored so far
struct SharedScan {
    members: [UmaIndex; 4],
    member_count: usize,
    member_ids: Vec<u32>,
    lineages: Vec<LineageChance>,
    chances_by_member: Vec<Vec<f64>>,
    floor: f64,
    loop_limit: usize,
    scored_decks: crate::FxSet<DeckId>,
    decks: Vec<ScoredDeck<PDeck>>,
}
impl SharedScan {
    fn new(d: &Data, f: [UmaIndex; 4], qa: &[f64], floor: f64, kloops: usize) -> Self {
        let ln = lin_of(&f);
        let fm = members(&f);
        let nm = fm.len();
        let ids: Vec<u32> = fm.iter().map(|&u| d.umas[u].id).collect();
        let lineages: Vec<LineageChance> =
            (0..ln.len()).map(|l| LineageChance { trainee: ln[l].trainee, inheritance: qa[l] }).collect();
        let qt: Vec<Vec<f64>> =
            (0..nm).map(|t| (0..ln.len()).filter(|&l| ln[l].trainee == t).map(|l| qa[l]).collect()).collect();
        Self {
            members: f,
            member_count: nm,
            member_ids: ids,
            lineages,
            chances_by_member: qt,
            floor,
            loop_limit: kloops,
            scored_decks: crate::FxSet::default(),
            decks: vec![],
        }
    }
    /// the threshold a deck must reach: the floor, or the loop's k-th best so far when that is higher
    fn need(&self) -> f64 {
        let mut v: Vec<f64> = self.decks.iter().map(|x| x.value).collect();
        v.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let kl = if v.len() >= self.loop_limit { v[self.loop_limit - 1] } else { f64::NEG_INFINITY };
        let t = self.floor.max(kl);
        if t.is_finite() { t } else { v.last().copied().unwrap_or(f64::NEG_INFINITY) }
    }
}

impl Multi<'_> {
    // ---- shared deck ----
    // With one deck for the whole structure, its value is the best exact sum over its lineages among decks legal
    // for every member. Per structure: every such deck whose exact sum is at least `floor` (the k-th best distinct
    // deck so far, times the lineage count) or among its own best `top_decks`, under cap lv; sorted, best first.
    // Ceilings
    // stay the per-uma ones (each trainee's best deck bounds her share of any shared deck).
    pub(super) fn exact_shared(&self, f: [UmaIndex; 4], qa: &[f64], floor: f64, lv: f64) -> SharedDecks {
        let key = SharedQueryKey { members: f, chances: qkey(qa) };
        if let Some(SharedResult { certified_cap: l, floor: fl, decks: r }) = self.shared_cache.borrow().get(&key)
            && *l >= lv
            && *fl <= floor
        {
            return Rc::clone(r);
        }
        let mut sc = SharedScan::new(self.inputs.data, f, qa, floor, self.loop_limit);
        self.seed_shared(&mut sc);
        let complete = if lv > 1.0 && !sc.decks.is_empty() { self.band_shared(&mut sc, lv) } else { true };
        let thr = sc.need();
        let mut list = sc.decks;
        list.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        list.retain(|x| x.value >= thr);
        let r: SharedDecks = Rc::new(list);
        if complete {
            self.shared_cache.borrow_mut().insert(key, SharedResult { certified_cap: lv, floor, decks: Rc::clone(&r) });
        }
        r
    }

    /// Scores deck x exactly over the loop's lineages (once per loop) and lists it
    fn score_shared(&self, sc: &mut SharedScan, x: &PDeck) {
        if !sc.scored_decks.insert(x.id()) {
            return;
        }
        let (model, tally, f) = (self.inputs.model, &self.tally, sc.members);
        let t0 = Instant::now();
        let DeckSources { entries: e, filler_count: fl } = self.ents_of(x);
        let k = src_class(e.iter().map(|y| &*y.sources));
        let mut sv = 0.0;
        for t in 0..sc.member_count {
            let j = joint_b(model, f[t], &sc.chances_by_member[t], &e, fl, self.requested_target_count, false);
            for (i, &q) in sc.chances_by_member[t].iter().enumerate() {
                let p = self.fixed * model.trainee_fast(f[t], q, &x.dw, &x.dg);
                if p > 0.0 {
                    tally.ratio_at(k, j[i].all / p, self.inputs.data.umas[f[t]].id, x);
                }
                sv += j[i].all;
            }
        }
        tally.score_seconds.set(tally.score_seconds.get() + t0.elapsed().as_secs_f64());
        tally.scored_decks.set(tally.scored_decks.get() + 1);
        tally.spend(800);
        sc.decks.push(ScoredDeck { value: sv, deck: x.clone() });
    }

    /// seed: the loop's best `top_decks` + 2 skyline decks legal for every member by product value (or its best legal
    /// deck when none is on the skyline)
    fn seed_shared(&self, sc: &mut SharedScan) {
        let (model, space, fixed, f) = (self.inputs.model, self.inputs.deck_space, self.fixed, sc.members);
        let t0 = Instant::now();
        let pairs = &sc.lineages;
        let val = |dw: &[f64], dg: &[f64]| {
            fixed
                * pairs
                    .iter()
                    .map(|&LineageChance { trainee: t, inheritance: q }| model.trainee_fast(f[t], q, dw, dg))
                    .sum::<f64>()
        };
        let ids = &sc.member_ids;
        let seeds = self.sky_top(self.loop_limit + 2, &val, &|x| ids.iter().all(|&id| !x.blocks(id, &self.card_of)));
        self.tally.seed_seconds.set(self.tally.seed_seconds.get() + t0.elapsed().as_secs_f64());
        for &(_, j) in &seeds {
            self.score_shared(sc, &self.inputs.skyline[j]);
        }
        if sc.decks.is_empty() {
            let SearchOutcome { decks: r, .. } = space.search(
                model,
                members(&f),
                &sc.member_ids,
                &[SearchClause { lineages: sc.lineages.clone(), threshold: 0.0 }],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            if let Some(ScoredDeck { value: _, deck: sel }) = r.first() {
                let x = space.pdeck(sel, &sc.member_ids);
                self.score_shared(sc, &x);
            }
        }
    }

    /// Every legal deck that might reach the loop's threshold under cap lv, scored (after the proven bound); false (and
    /// the pass aborted) when a deck search was cut short or the budget ran out
    fn band_shared(&self, sc: &mut SharedScan, lv: f64) -> bool {
        let (model, space, tally, f) = (self.inputs.model, self.inputs.deck_space, &self.tally, sc.members);
        let mut complete = true;
        tally.band_decks.set(tally.band_decks.get() + 1);
        let t0 = Instant::now();
        let n0 = space.nodes.get();
        let SearchOutcome { decks: mut cands, incomplete } = space.search(
            model,
            members(&f),
            &sc.member_ids,
            &[SearchClause { lineages: sc.lineages.clone(), threshold: sc.need() }],
            SearchMode::Band { ratio_cap: lv, max_decks: space.band_cap() },
        );
        tally.spend(2 * (space.nodes.get() - n0) + 5 * cands.len() + 10_000);
        tally.band_seconds.set(tally.band_seconds.get() + t0.elapsed().as_secs_f64());
        if incomplete.is_some() {
            tally.cap_hit.set(true);
            complete = false;
        }
        cands.sort_by(|a, b| b.value.partial_cmp(&a.value).unwrap());
        let mut seen: crate::FxSet<(Vec<usize>, usize, usize)> = crate::FxSet::default();
        if complete {
            for ScoredDeck { value: v, deck: sel } in &cands {
                let thr = sc.need();
                if lv * v * (1.0 + 1e-9) < thr {
                    break;
                }
                if tally.work.get() > tally.budget.get() {
                    complete = false;
                    break;
                }
                if space.sel_cap(sel, lv) * v * (1.0 + 1e-9) < thr {
                    continue;
                }
                if !seen.insert((sel.entries.clone(), sel.plain_pals, sel.filler_cards)) {
                    continue;
                }
                let e: Vec<SourceEntry<&TargetSources>> = sel
                    .entries
                    .iter()
                    .map(|&j| SourceEntry { sources: &space.sources[j], kind: space.kind[j] })
                    .collect();
                let t1 = Instant::now();
                let ub: Option<f64> = (0..sc.member_count)
                    .map(|t| {
                        upper_na(model, f[t], &sc.chances_by_member[t], &e, sel.filler_cards)
                            .map(|b| b.iter().sum::<f64>())
                    })
                    .sum();
                tally.bound_seconds.set(tally.bound_seconds.get() + t1.elapsed().as_secs_f64());
                if let Some(ub) = ub {
                    tally.spend(100);
                    if ub * (1.0 + 1e-9) < thr {
                        tally.bound_rejections.set(tally.bound_rejections.get() + 1);
                        continue;
                    }
                }
                let x = space.pdeck(sel, &sc.member_ids);
                self.score_shared(sc, &x);
            }
        }
        if !complete {
            tally.aborted.set(true);
        }
        complete
    }
}
