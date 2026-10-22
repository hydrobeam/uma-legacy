//! Picks, JSON results and diagnostic summaries from completed loop-search passes.
//! Search and certification belong to `bands`.
use super::bands::{DeckBest, Pass, Solved};
use super::{DeckKey, DeckSources, Multi, RatioExample, lin_of};
use crate::deck::joint::{JointChances, JointExt, joint_b, joint_fast};
use crate::deck::kept::PDeck;
use crate::deck::space::{SPT, UCHK};
use crate::skills::{GEN_GOLD, MemberLineage, OutfitOption, chart_rows, member_json};
use crate::{members, probs, wval};
use serde_json::{Value, json};
use std::cell::Cell;

/// A reported loop: its solved index, each member's deck, per lineage (exact P(all), P(all but one), outfit), value
struct Pick<'pass> {
    solved_index: usize,
    decks: Vec<&'pass PDeck>,
    lineage_chances: Vec<JointChances>,
    value: f64,
}

impl Multi<'_> {
    // ---- the answer ----

    pub(super) fn debug_summary(&self, p: &Pass, cert: f64) {
        if std::env::var("UPPER_CHECK").is_ok() {
            let v = UCHK.with(Cell::get);
            eprintln!("UPPER_CHECK: {} checked, max bound/exact {:.4}, {} decks not covered", v.0, v.1, v.2);
        }
        if std::env::var("DECK_DEBUG").is_ok() {
            let v = SPT.with(Cell::get);
            eprintln!("deck searches: setup {:.2}s tree {:.2}s configs {} count vectors {}", v.0, v.1, v.2, v.3);
        }
        if std::env::var("DECK_DEBUG").is_ok() {
            let t = &self.tally;
            eprintln!(
                "per-uma search {:.2}s cap {} scanned {} solved {} exact: {} deck searches, {} decks scored, {} bounded out, schedule rejects {} | band {:.2}s score {:.2}s seed-scan {:.2}s bound {:.2}s",
                self.started.elapsed().as_secs_f64(),
                cert,
                p.scanned,
                p.solved.len(),
                t.band_decks.get(),
                t.scored_decks.get(),
                t.bound_rejections.get(),
                p.schedule_rejects,
                t.band_seconds.get(),
                t.score_seconds.get(),
                t.seed_seconds.get(),
                t.bound_seconds.get()
            );
        }
    }

    /// The output: the reported loops, the search's counts and what the answer rests on, over the deck search's stats
    pub(super) fn report(&self, p: Pass, cert: f64, stats: Value) -> Value {
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
            dw: self.no_hint_chances.clone(),
            dg: self.no_hint_chances.clone(),
        };
        // what is reported: per-uma, the best loops; shared, the best distinct decks, each with its best loop
        let picks = if self.shared { self.shared_picks(&p) } else { self.per_uma_picks(&p, &empty_deck) };
        let top: Vec<Value> = picks.iter().map(|pk| self.loop_json(&p, pk)).collect();
        let budget_out = self.inputs.deck_job.exact && cert < self.maximum_exact_ratio;
        let complete = p.next_bound <= p.kth * (1.0 + 4e-12);
        let exact = self.exact_json(&p, cert, complete, budget_out);
        let n_band = self.tally.band_decks.get();
        let mut out = stats;
        for (key, val) in [
            ("top", json!(top)),
            ("best", json!([])),
            ("per_uma", json!(!self.shared)),
            ("foursomes", json!(self.total_foursomes())),
            ("foursomes_ranked", json!(p.generated)),
            ("foursomes_scanned", json!(p.scanned)),
            ("foursomes_solved", json!(p.solved.len())),
            ("next_bound", json!(p.next_bound)),
            ("proven", json!(complete)),
            ("searches", json!(1 + n_band)),
            ("exact", exact),
        ] {
            out[key] = val;
        }
        out
    }

    /// The structures the job's sizes and required umas allow
    fn total_foursomes(&self) -> usize {
        let (k, nreq) = (self.pool.len(), self.required_umas.len());
        let choose = |m: usize, c: usize| (0..c).fold(1usize, |acc, i| acc * (m - i) / (i + 1));
        let m = k - nreq.min(k);
        (if self.allow_four { choose(m, 4usize.saturating_sub(nreq)) } else { 0 })
            + (if self.allow_three && nreq <= 3 { choose(m, 3 - nreq) } else { 0 })
    }

    /// shared: the best `top_decks` distinct decks (ties by identity), each with the loop it scored best in
    fn shared_picks<'pass>(&self, p: &'pass Pass) -> Vec<Pick<'pass>> {
        let mut db: Vec<(&DeckKey, &DeckBest)> = p.deck_best.iter().collect();
        db.sort_by(|a, b| b.1.value.partial_cmp(&a.1.value).unwrap().then(a.0.cmp(b.0)));
        db.iter()
            .take(self.loop_limit)
            .map(|&(id, &DeckBest { value: v, solved_index: si, list_index: li })| {
                let x = &p.lists[li]
                    .iter()
                    .find(|y| {
                        y.deck.ids == id.card_ids
                            && y.deck.ends == id.end_links
                            && y.deck.vars == id.variants
                            && y.deck.plain_ids.len() == id.plain_pals
                    })
                    .unwrap()
                    .deck;
                let Solved { members, inheritance_chances, .. } = &p.solved[si];
                let DeckSources { entries: e, filler_count: fl } = self.ents_of(x);
                let ln = lin_of(members);
                let nm = crate::members(members).len();
                let mut per_l = vec![JointChances::default(); ln.len()];
                for t in 0..nm {
                    let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].trainee == t).collect();
                    let qt: Vec<f64> = ls.iter().map(|&l| inheritance_chances[l]).collect();
                    let j = joint_b(self.inputs.model, members[t], &qt, &e, fl, self.requested_target_count, true);
                    for (i, &l) in ls.iter().enumerate() {
                        per_l[l] = j[i];
                    }
                }
                Pick { solved_index: si, decks: vec![x; nm], lineage_chances: per_l, value: v }
            })
            .collect()
    }

    /// per-uma: the best `top_decks` loops (ties in solve order), each member with her best deck
    fn per_uma_picks<'pass>(&self, p: &'pass Pass, empty_deck: &'pass PDeck) -> Vec<Pick<'pass>> {
        let solved = &p.solved;
        let mut order: Vec<usize> = (0..solved.len()).collect();
        // stable: ties keep solve order
        order.sort_by(|&a, &b| solved[b].value.partial_cmp(&solved[a].value).unwrap());
        order
            .iter()
            .take(self.loop_limit)
            .map(|&si| {
                let bests = &solved[si].bests;
                let ln = lin_of(&solved[si].members);
                let nm = members(&solved[si].members).len();
                let mut per_l = vec![JointChances::default(); ln.len()];
                for t in 0..nm {
                    let ls: Vec<usize> = (0..ln.len()).filter(|&l| ln[l].trainee == t).collect();
                    for (i, &l) in ls.iter().enumerate() {
                        per_l[l] = bests[t].lineage_chances[i];
                    }
                }
                let decks = bests.iter().map(|b| b.deck.as_ref().unwrap_or(empty_deck)).collect();
                Pick { solved_index: si, decks, lineage_chances: per_l, value: solved[si].value }
            })
            .collect()
    }

    /// A reported loop: its members, odds, outfits, schedule and detail, with its deck (shared) or decks (per-uma)
    fn loop_json(&self, p: &Pass, pk: &Pick) -> Value {
        let (d, model, unc, ntar) =
            (self.inputs.data, self.inputs.model, &self.target_groups, self.requested_target_count);
        let (dk, per_l, pj) = (&pk.decks, &pk.lineage_chances, pk.value);
        let Solved { members, schedule, inheritance_chances, .. } = &p.solved[pk.solved_index];
        let ln = lin_of(members);
        let member_slice = crate::members(members);
        let nm = member_slice.len();
        let nl = ln.len();
        let mut per = vec![(0.0, 0usize); nm];
        let mut picks: Vec<Vec<usize>> = vec![vec![]; nm];
        for (l, &MemberLineage { trainee: t, .. }) in ln.iter().enumerate() {
            per[t].0 += per_l[l].all;
            per[t].1 += 1;
            picks[t].push(per_l[l].outfit);
        }
        let minus1 = per_l.iter().map(|x| x.all_but_one).sum::<f64>() / nl as f64;
        // the chance of exactly k targets, k = 0 ..= n (exact over each lineage's landed sets, as `p_all`), averaged over
        // the lineages: the uncertain targets' sets from the joint model, each guaranteed gold on its own (GEN_GOLD)
        let mut landed = vec![0.0f64; ntar + 1];
        // (each member's deck's sources once, read by each of her lineages)
        let dke: Vec<_> = (0..nm).map(|t| self.ents_of(dk[t])).collect();
        for l in 0..nl {
            let t = ln[l].trainee;
            let (e, fl) = (&dke[t].entries, dke[t].filler_count);
            let target_count = unc.len();
            let mut ext = JointExt {
                scales: crate::deck::joint::SparkScales {
                    uncertain: vec![1.0; target_count],
                    guaranteed: vec![1.0; ntar - target_count],
                },
                marginals: vec![],
                sets: vec![0.0],
                per_outfit: vec![],
            };
            joint_fast(model, member_slice[t], &[inheritance_chances[l]], e, fl, ntar, false, Some(&mut ext));
            let o = per_l[l].outfit;
            let mut g = vec![1.0f64];
            for _ in target_count..ntar {
                let mut nn = vec![0.0; g.len() + 1];
                for (i, x) in g.iter().enumerate() {
                    nn[i] += x * (1.0 - GEN_GOLD);
                    nn[i + 1] += x * GEN_GOLD;
                }
                g = nn;
            }
            for (mu, &x) in ext.sets[(o << target_count)..((o + 1) << target_count)].iter().enumerate() {
                if x == 0.0 {
                    continue;
                }
                for (j, &y) in g.iter().enumerate() {
                    landed[mu.count_ones() as usize + j] += x * y / nl as f64;
                }
            }
        }
        // the search's product value of this loop with these decks
        let rank_v = self.fixed
            * (0..nl)
                .map(|l| {
                    model.trainee_fast(
                        member_slice[ln[l].trainee],
                        inheritance_chances[l],
                        &dk[ln[l].trainee].dw,
                        &dk[ln[l].trainee].dg,
                    )
                })
                .sum::<f64>()
            / nl as f64;
        let outfits: Vec<Value> = (0..nm)
            .map(|i| {
                let kk = *picks[i].iter().max_by_key(|x| picks[i].iter().filter(|y| y == x).count()).unwrap();
                let OutfitOption { coverage: st, outfits: cs } = &model.opts[member_slice[i]][kk];
                let covers: Vec<Value> = st
                    .iter()
                    .enumerate()
                    .filter(|(_, lv)| **lv > 0)
                    .map(|(j, lv)| json!({"group": unc[j], "level": lv}))
                    .collect();
                let outfits = if covers.is_empty() {
                    vec![]
                } else {
                    cs.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>()
                };
                json!({"outfits": outfits, "covers": covers})
            })
            .collect();
        let pss: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[member_slice[i]], &schedule.s[i])).collect();
        let dks: Vec<(&[f64], &[f64])> = (0..nm).map(|t| (&*dk[t].dw, &*dk[t].dg)).collect();
        let mut detail =
            model.detail(member_slice, &|a, b| if self.no_race { 0.0 } else { wval(&pss[a], &pss[b]) }, Some(&dks));
        if let Some(ls) = detail["lineages"].as_array_mut() {
            for (l, x) in ls.iter_mut().enumerate() {
                x["rank_value"] = x["p_all"].clone();
                x["p_all"] = json!(per_l[l].all);
            }
        }
        let lp = json!({"members": member_json(d, member_slice), "p_all": pj, "p_minus1": minus1, "landed": landed,
               "rank_value": rank_v, "outfits": outfits,
               "per_trainee": per.iter().map(|(s, c)| s / *c as f64).collect::<Vec<_>>(),
               "mean_proc": inheritance_chances.iter().sum::<f64>() / inheritance_chances.len() as f64, "schedule": chart_rows(d, member_slice, &schedule.s[..nm]),
               "detail": detail});
        if self.shared {
            return json!({"loop": lp, "deck": dk[0].out(unc)});
        }
        // the deck most members share, for displays that show one deck
        let base = (0..nm).max_by_key(|&i| (dk.iter().filter(|x| x.names == dk[i].names).count(), nm - i)).unwrap();
        json!({"loop": lp, "deck": dk[base].out(unc), "decks": dk.iter().map(|x| x.out(unc)).collect::<Vec<_>>()})
    }

    /// What the answer rests on: `rho` the highest cap certified (`rho_target` the one sought), `class_caps` the
    /// assumed cap per structure class (each certified at most at rho), `class_max_ratio` the largest exact / product
    /// odds seen per class on any deck scored here (`max_ratio` of them all); `complete` = the search ended by its stop
    /// rule (no unexamined loop or deck can win under the certified caps)
    fn exact_json(&self, p: &Pass, cert: f64, complete: bool, budget_out: bool) -> Value {
        let t = &self.tally;
        let mr = t.max_ratio.get();
        let max_deck: Vec<Option<Value>> = t
            .max_deck
            .borrow()
            .iter()
            .map(|x| x.as_ref().map(|RatioExample { trainee_id: u, cards: n }| json!({"trainee": u, "cards": n})))
            .collect();
        json!({"rho": cert, "rho_target": self.maximum_exact_ratio, "class_caps": self.exact_caps,
               "max_ratio": mr.iter().copied().fold(0.0f64, f64::max), "class_max_ratio": mr,
               "class_max_deck": max_deck, "complete": complete, "decks_scored": t.scored_decks.get(),
               "deck_searches": t.band_decks.get(), "loops_solved": p.solved.len(), "schedule_rejects": p.schedule_rejects,
               "cap_hit": t.cap_hit.get(), "budget_out": budget_out, "bound_filtered": t.bound_rejections.get()})
    }
}
