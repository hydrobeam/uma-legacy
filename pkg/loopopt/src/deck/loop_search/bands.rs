//! Loop-structure bands, schedule solving and budgeted certification passes.
//! Deck scoring and ceiling tables belong to `shared` and `tables`; result formatting belongs to `report`.
use super::shared::SharedDecks;
use super::tables::Best;
use super::{Multi, SharedDeckKey};
use crate::data::{Choice, NT, probs, wval};
use crate::deck::caps::{EXACT_BUDGET, EXACT_RHO, EXACT_STEPS};
use crate::deck::enumerate::ScoredDeck;
use crate::optimize::{NONE, Res, members, optimize_fam};
use crate::skills::lins;
use crate::uma::{UmaIndex, UmaTable};
use std::rc::Rc;

/// A loop awaiting evaluation, in descending product-ceiling order.
#[derive(Clone, Copy)]
struct LoopCandidate {
    ceiling: f64,
    members: [UmaIndex; 4],
}

/// A solved loop: members, schedule, lineage chances, per-member bests (per-uma decks) and value
pub(super) struct SolvedLoop {
    pub(super) members: [UmaIndex; 4],
    pub(super) schedule: Res,
    pub(super) inheritance_chances: Vec<f64>,
    pub(super) bests: Vec<Rc<Best>>,
    pub(super) value: f64,
}
/// The highest value found for a shared deck and where its loop and deck are retained.
pub(super) struct DeckBest {
    /// Mean exact value over the selected loop's lineages.
    pub(super) value: f64,
    /// Indices in the pass's solved loops and shared-deck lists.
    pub(super) solved_index: usize,
    pub(super) list_index: usize,
}

/// One pass's solved loops, cutoff and unexamined bound, counters, and shared-deck results.
pub(super) struct Pass {
    pub(super) solved: Vec<SolvedLoop>,
    pub(super) kth: f64,
    pub(super) next_bound: f64,
    pub(super) scanned: usize,
    pub(super) generated: usize,
    pub(super) schedule_rejects: usize,
    pub(super) deck_best: crate::FxMap<SharedDeckKey, DeckBest>,
    pub(super) lists: Vec<SharedDecks>,
    /// (per-uma decks: the solved loops' values, in solve order)
    values: Vec<f64>,
}
/// A pass's queue of structures: (ceiling, members) in ceiling order, the next to scan, and the sum below which no
/// band has been generated yet
struct Queue {
    candidates: Vec<LoopCandidate>,
    next: usize,
    upper_sum: f64,
}
/// A three-member structure's sum s in four-member sum units
fn e3(s: f64) -> f64 {
    s * 4.0 / 3.0
}

impl Multi<'_> {
    /// Orders the pool by M_u (the product ceiling at her highest chance), highest first, for `band`, and sets the
    /// highest structure sum
    pub(super) fn rank(&mut self) {
        let mval: UmaTable<f64> = self
            .inputs
            .data
            .umas
            .indices()
            .map(|u| if self.pool.contains(&u) { self.btab(u, self.grid.idx(self.maximum_chances[u])) } else { 0.0 })
            .collect();
        let mut ord: Vec<UmaIndex> = self.pool.clone();
        ord.sort_by(|&a, &b| mval[b].partial_cmp(&mval[a]).unwrap());
        let mv: Vec<f64> = ord.iter().map(|&u| mval[u]).collect();
        let k = self.pool.len();
        self.maximum_structure_sum = (if self.allow_four && k >= 4 { mv[0] + mv[1] + mv[2] + mv[3] } else { 0.0 })
            .max(if self.allow_three && k >= 3 { e3(mv[0] + mv[1] + mv[2]) } else { 0.0 });
        self.ranked_pool = ord;
        self.ranked_ceilings = mv;
    }

    /// A structure's product ceiling is at most the guaranteed golds' factor times the mean of its members' M_u (each
    /// member heads the same number of lineages): (3/12)·sum for four, sum/3 for three. Bands are in four-member sum
    /// units: a three-member structure's sum s counts as e3(s) = 4s/3.
    fn sep(&self, s: f64) -> f64 {
        self.fixed * 3.0 * s / 12.0 * (1.0 + 1e-12)
    }

    // ---- the loop search ----

    /// The structures whose member sum (in four-member units) is in [lo, hi), with their product ceilings
    fn band(&self, lo: f64, hi: f64, out: &mut Vec<LoopCandidate>) {
        if self.allow_four {
            self.band4(lo, hi, out);
        }
        if self.allow_three && self.required_umas.len() <= 3 {
            self.band3(lo, hi, out);
        }
    }
    fn band4(&self, lo: f64, hi: f64, out: &mut Vec<LoopCandidate>) {
        let (mv, ord, k) = (&self.ranked_ceilings, &self.ranked_pool, self.pool.len());
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
                        if !self.required_umas.iter().all(|r| f.contains(r)) {
                            continue;
                        }
                        let ln = lins(4);
                        let qs = self.inputs.model.lineage_q_arr(&f, &|x, y| self.pair_bounds[f[x]][f[y]]);
                        let ub = self.fixed
                            * (0..12).map(|l| self.btab(f[ln[l].trainee], self.grid.idx(qs[l]))).sum::<f64>()
                            / 12.0
                            * (1.0 + 1e-12);
                        out.push(LoopCandidate { ceiling: ub, members: f });
                    }
                }
            }
        }
    }
    fn band3(&self, lo: f64, hi: f64, out: &mut Vec<LoopCandidate>) {
        let (mv, ord, k) = (&self.ranked_ceilings, &self.ranked_pool, self.pool.len());
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
                    if !self.required_umas.iter().all(|r| f.contains(r)) {
                        continue;
                    }
                    let ln = lins(3);
                    let qs = self.inputs.model.lineage_q_arr(&t, &|x, y| self.pair_bounds[t[x]][t[y]]);
                    let ub = self.fixed
                        * (0..3).map(|l| self.btab(t[ln[l].trainee], self.grid.idx(qs[l]))).sum::<f64>()
                        / 3.0
                        * (1.0 + 1e-12);
                    out.push(LoopCandidate { ceiling: ub, members: f });
                }
            }
        }
    }

    /// Try the caps highest first (EXACT_STEPS): the answer is certified under the first whose pass finishes within the
    /// budget; a lower cap's pass reuses everything computed so far and gets a quarter of the budget more. When none
    /// finishes, a pass with the cap at 1 (product odds: no deck searches) gives the answer. (the pass, its cap)
    pub(super) fn certify(&self) -> (Pass, f64) {
        let budget = std::env::var("EXACT_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(EXACT_BUDGET);
        let steps: Vec<f64> = if !self.inputs.deck_job.exact {
            vec![]
        } else if let Ok(x) = std::env::var("EXACT_STEPS") {
            x.split(',').filter_map(|t| t.parse().ok()).collect()
        } else {
            EXACT_STEPS.iter().map(|&x| if x == EXACT_RHO { self.maximum_exact_ratio } else { x }).collect()
        };
        let mut last: Option<Pass> = None;
        let mut cert = 1.0f64;
        let mut allow = budget;
        for &lv in &steps {
            self.tally.budget.set(allow);
            if let Some(pass) = self.run_pass(lv) {
                last = Some(pass);
                cert = lv;
                break;
            } else {
                // Retain scored decks, but grant fresh work before trying a lower cap.
                self.tally.aborted.set(false);
                allow = self.tally.work.get() + budget / 4;
            }
        }
        let p = if let Some(p) = last {
            p
        } else {
            self.tally.budget.set(usize::MAX);
            self.run_pass(1.0).expect("a pass without deck searches")
        };
        (p, cert)
    }

    /// One best-first pass over the structures with cap lv: ceilings are exact tables or product ceilings times lv, the
    /// stop rule compares them with the k-th exact value. None when the budget runs out (the pass certifies nothing).
    fn run_pass(&self, lv: f64) -> Option<Pass> {
        let lo_sum = self.maximum_structure_sum * 0.85;
        let mut candidates: Vec<LoopCandidate> = vec![];
        self.band(lo_sum, f64::INFINITY, &mut candidates);
        candidates.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
        let mut p = Pass {
            solved: vec![],
            kth: f64::NEG_INFINITY,
            next_bound: 0.0,
            scanned: 0,
            generated: candidates.len(),
            schedule_rejects: 0,
            deck_best: crate::FxMap::default(),
            lists: vec![],
            values: vec![],
        };
        let mut q = Queue { candidates, next: 0, upper_sum: lo_sum };
        loop {
            if self.tally.aborted.get() {
                return None;
            }
            // Treat improvements within 4e-12 relative slack as ties, so equal-valued structures need not all be
            // solved.
            let kth = p.kth;
            let beats = |c: f64| c > kth * (1.0 + 4e-12);
            if !(q.next < q.candidates.len() && beats(lv * q.candidates[q.next].ceiling)) {
                if q.upper_sum <= 0.0 || !beats(lv * self.sep(q.upper_sum)) {
                    break;
                }
                p.generated += self.widen(&mut q, kth, lv);
                continue;
            }
            let LoopCandidate { ceiling: _, members: f } = q.candidates[q.next];
            q.next += 1;
            p.scanned += 1;
            self.solve(f, lv, &mut p)?;
        }
        p.next_bound = lv
            * (if q.next < q.candidates.len() { q.candidates[q.next].ceiling } else { 0.0 })
                .max(if q.upper_sum > 0.0 { self.sep(q.upper_sum) } else { 0.0 });
        Some(p)
    }

    /// Adds the next band of structures below the queue's `upper_sum` (down to the sum the k-th value needs, at most 15%
    /// lower), merged into the unscanned rest in ceiling order; how many it added
    fn widen(&self, q: &mut Queue, kth: f64, lv: f64) -> usize {
        let kth_lo = if kth > f64::NEG_INFINITY { kth / lv * 12.0 / (3.0 * self.fixed) * (1.0 - 1e-9) } else { 0.0 };
        let lo = kth_lo.max(q.upper_sum * 0.85);
        let lo = if lo < self.maximum_structure_sum * 1e-6 { 0.0 } else { lo };
        let mut more: Vec<LoopCandidate> = vec![];
        self.band(lo, q.upper_sum, &mut more);
        q.upper_sum = lo;
        let added = more.len();
        let mut rest: Vec<LoopCandidate> = q.candidates.split_off(q.next);
        rest.extend(more);
        rest.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
        q.candidates.extend(rest);
        added
    }

    /// Structure f's turn in a pass: rejected by its ceilings (cheapest first), else scheduled and solved exactly into
    /// the pass. None when the budget runs out.
    fn solve(&self, f: [UmaIndex; 4], lv: f64, p: &mut Pass) -> Option<()> {
        let kth = p.kth;
        let beats = |c: f64| c > kth * (1.0 + 4e-12);
        let fm = members(&f);
        let nm = fm.len();
        let ln = lins(nm);
        let nl = ln.len();
        let qpb = self.inputs.model.lineage_q_arr(fm, &|x, y| self.pair_bounds[fm[x]][fm[y]]);
        // cheapest first: the cell bounds as they stand, then the product ceiling with one deck per trainee
        if !beats((0..nl).map(|l| self.cell(f[ln[l].trainee], self.grid.idx(qpb[l]), lv).0).sum::<f64>() / nl as f64) {
            return Some(());
        }
        if !beats(lv * self.relaxed(f, &qpb)) {
            return Some(());
        }
        if self.shared && !beats(lv * self.relaxed_shared(f, &qpb)) {
            return Some(());
        }
        // exact ceiling at the pairwise-ceiling chances, valid for any schedule
        if !beats(self.exact_ceiling(f, &qpb, kth, lv)) {
            return Some(());
        }
        if self.tally.aborted.get() {
            return None;
        }
        let r = self.schedule(f, nm);
        let d = self.inputs.data;
        let ps: Vec<Vec<f64>> = (0..nm).map(|i| probs(d, &d.umas[f[i]], &r.s[i])).collect();
        let qs = self.inputs.model.lineage_q(fm, &|x, y| if self.no_race { 0.0 } else { wval(&ps[x], &ps[y]) });
        let qa: [f64; 12] = std::array::from_fn(|i| qs.get(i).copied().unwrap_or(0.0));
        // rejected only with the schedule found: a better schedule could change this one's fate
        if (self.shared && !beats(lv * self.relaxed_shared(f, &qa))) || !beats(self.exact_ceiling(f, &qa, kth, lv)) {
            p.schedule_rejects += 1;
            return Some(());
        }
        if self.shared {
            let sl =
                self.exact_shared(f, &qa[..nl], if kth.is_finite() { kth * nl as f64 } else { f64::NEG_INFINITY }, lv);
            if self.tally.aborted.get() {
                return None;
            }
            if sl.is_empty() {
                return Some(());
            }
            let val = sl[0].value / nl as f64;
            for ScoredDeck { value: v, deck: x } in sl.iter() {
                let id = SharedDeckKey {
                    card_ids: x.ids.clone(),
                    end_links: x.ends.clone(),
                    variants: x.vars.clone(),
                    plain_pals: x.plain_ids.len(),
                };
                let e = p.deck_best.entry(id).or_insert(DeckBest {
                    value: f64::NEG_INFINITY,
                    solved_index: 0,
                    list_index: 0,
                });
                if v / nl as f64 > e.value {
                    *e = DeckBest { value: v / nl as f64, solved_index: p.solved.len(), list_index: p.lists.len() };
                }
            }
            p.solved.push(SolvedLoop { members: f, schedule: r, inheritance_chances: qs, bests: vec![], value: val });
            p.lists.push(sl);
            let mut vs: Vec<f64> = p.deck_best.values().map(|x| x.value).collect();
            vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
            p.kth = if vs.len() >= self.loop_limit { vs[self.loop_limit - 1] } else { f64::NEG_INFINITY };
            return Some(());
        }
        let mut tot = 0.0;
        let mut bests = vec![];
        for t in 0..nm {
            let qt: Vec<f64> = (0..nl).filter(|&l| ln[l].trainee == t).map(|l| qa[l]).collect();
            let b = self.exact_best(f[t], &qt, lv);
            tot += b.sum;
            bests.push(b);
        }
        if self.tally.aborted.get() {
            return None;
        }
        let val = tot / nl as f64;
        p.solved.push(SolvedLoop { members: f, schedule: r, inheritance_chances: qs, bests, value: val });
        p.values.push(val);
        let mut vs = p.values.clone();
        vs.sort_by(|a, b| b.partial_cmp(a).unwrap());
        p.kth = if vs.len() >= self.loop_limit { vs[self.loop_limit - 1] } else { f64::NEG_INFINITY };
        Some(())
    }

    /// The schedule of structure f: the job's fixed ones, else the optimiser's (kept for later passes)
    fn schedule(&self, f: [UmaIndex; 4], nm: usize) -> Res {
        let r = self.schedule_cache.borrow().get(&f).cloned();
        r.unwrap_or_else(|| {
            let r = match &self.fixed_schedules {
                Some(fs) => {
                    Res { s: std::array::from_fn(|i| if i < nm { fs[f[i]].unwrap() } else { [Choice::Rest; NT] }) }
                }
                None => {
                    optimize_fam(self.inputs.data, &f, self.inputs.skill_job.iters, self.inputs.skill_job.seed, 60.0)
                }
            };
            self.schedule_cache.borrow_mut().insert(f, r.clone());
            r
        })
    }
}
