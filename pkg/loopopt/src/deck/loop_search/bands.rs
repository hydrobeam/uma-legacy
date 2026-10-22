//! Loop-structure bands, schedule solving and budgeted certification passes.
//! Deck scoring and ceiling tables belong to `shared` and `tables`; result formatting belongs to `report`.
use super::LoopSearchState;
use super::shared::SharedDecks;
use super::tables::Best;
use super::{Multi, SharedDeckKey};
use crate::data::{Choice, NT, dot};
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
    /// (per-uma decks: the solved loops' values, highest first)
    values: Vec<f64>,
}
/// A pass's queue of structures: (ceiling, members) in ceiling order, the next to scan, and the sum below which no
/// band has been generated yet
struct Queue {
    candidates: Vec<LoopCandidate>,
    next: usize,
    upper_sum: f64,
}
/// A three-member structure's sum in four-member sum units
fn in_four_member_units(sum: f64) -> f64 {
    sum * 4.0 / 3.0
}
/// The `k`-th highest of `values` (-inf while there are fewer than `k`), sorting them highest first
fn kth_highest(values: &mut [f64], k: usize) -> f64 {
    values.sort_by(|a, b| b.partial_cmp(a).unwrap());
    if values.len() >= k { values[k - 1] } else { f64::NEG_INFINITY }
}

impl Pass {
    /// Adds a shared-deck loop solved with its scored `decks` (`lineage_count` lineages): each deck keeps its best loop,
    /// and the cutoff becomes the `loop_limit`-th best deck's value
    fn add_shared(&mut self, solved: SolvedLoop, decks: SharedDecks, lineage_count: usize, loop_limit: usize) {
        for ScoredDeck { value, deck } in decks.iter() {
            let key = SharedDeckKey {
                card_ids: deck.ids.clone(),
                end_links: deck.ends.clone(),
                variants: deck.vars.clone(),
                plain_pals: deck.plain_ids.len(),
            };
            let best = self.deck_best.entry(key).or_insert(DeckBest {
                value: f64::NEG_INFINITY,
                solved_index: 0,
                list_index: 0,
            });
            if value / lineage_count as f64 > best.value {
                *best = DeckBest {
                    value: value / lineage_count as f64,
                    solved_index: self.solved.len(),
                    list_index: self.lists.len(),
                };
            }
        }
        self.solved.push(solved);
        self.lists.push(decks);
        let mut deck_values: Vec<f64> = self.deck_best.values().map(|best| best.value).collect();
        self.kth = kth_highest(&mut deck_values, loop_limit);
    }

    /// Adds a per-uma-deck loop: the cutoff becomes the `loop_limit`-th best loop's value
    fn add_per_uma(&mut self, solved: SolvedLoop, loop_limit: usize) {
        self.values.push(solved.value);
        self.solved.push(solved);
        self.kth = kth_highest(&mut self.values, loop_limit);
    }
}

impl Multi<'_> {
    /// Orders the pool by M_u (the product ceiling at her highest chance), highest first, for `band`, and sets the
    /// highest structure sum
    pub(super) fn rank(&mut self, state: &mut LoopSearchState) {
        let top_ceilings: UmaTable<f64> =
            self.inputs
                .data
                .umas
                .indices()
                .map(|u| {
                    if self.pool.contains(&u) {
                        self.btab(state, u, self.grid.idx(self.maximum_chances[u]))
                    } else {
                        0.0
                    }
                })
                .collect();
        let mut ranked: Vec<UmaIndex> = self.pool.clone();
        ranked.sort_by(|&a, &b| top_ceilings[b].partial_cmp(&top_ceilings[a]).unwrap());
        let ceilings: Vec<f64> = ranked.iter().map(|&u| top_ceilings[u]).collect();
        let pool_size = self.pool.len();
        let four_sum =
            if self.allow_four && pool_size >= 4 { ceilings[0] + ceilings[1] + ceilings[2] + ceilings[3] } else { 0.0 };
        let three_sum = if self.allow_three && pool_size >= 3 {
            in_four_member_units(ceilings[0] + ceilings[1] + ceilings[2])
        } else {
            0.0
        };
        self.maximum_structure_sum = four_sum.max(three_sum);
        self.ranked_pool = ranked;
        self.ranked_ceilings = ceilings;
    }

    /// A structure's product ceiling is at most the guaranteed golds' factor times the mean of its members' M_u (each
    /// member heads the same number of lineages): (3/12)·sum for four, sum/3 for three. Bands are in four-member sum
    /// units: a three-member structure's sum s counts as 4s/3 (`in_four_member_units`).
    fn sum_ceiling(&self, sum: f64) -> f64 {
        self.fixed * 3.0 * sum / 12.0 * (1.0 + 1e-12)
    }

    // ---- the loop search ----

    /// The structures whose member sum (in four-member units) is in [lo, hi), with their product ceilings
    fn band(&self, state: &mut LoopSearchState, lo: f64, hi: f64, out: &mut Vec<LoopCandidate>) {
        if self.allow_four {
            self.band4(state, lo, hi, out);
        }
        if self.allow_three && self.required_umas.len() <= 3 {
            self.band3(state, lo, hi, out);
        }
    }
    /// `band`'s four-member structures: positions a < b < c < e in the ranked pool, each loop stopping once even its
    /// highest remaining sum falls below `lo`
    fn band4(&self, state: &mut LoopSearchState, lo: f64, hi: f64, out: &mut Vec<LoopCandidate>) {
        let (ceilings, ranked, pool_size) = (&self.ranked_ceilings, &self.ranked_pool, self.pool.len());
        for a in 0..pool_size {
            if a + 3 >= pool_size || ceilings[a] + ceilings[a + 1] + ceilings[a + 2] + ceilings[a + 3] < lo {
                break;
            }
            for b in a + 1..pool_size {
                if b + 2 >= pool_size || ceilings[a] + ceilings[b] + ceilings[b + 1] + ceilings[b + 2] < lo {
                    break;
                }
                for c in b + 1..pool_size {
                    if c + 1 >= pool_size || ceilings[a] + ceilings[b] + ceilings[c] + ceilings[c + 1] < lo {
                        break;
                    }
                    for e in c + 1..pool_size {
                        let sum = ceilings[a] + ceilings[b] + ceilings[c] + ceilings[e];
                        if sum < lo {
                            break;
                        }
                        if sum >= hi {
                            continue;
                        }
                        let mut structure = [ranked[a], ranked[b], ranked[c], ranked[e]];
                        structure.sort_unstable();
                        if !self.required_umas.iter().all(|r| structure.contains(r)) {
                            continue;
                        }
                        let lineages = lins(4);
                        let chances = self
                            .inputs
                            .model
                            .lineage_q_arr(&structure, &|x, y| self.pair_bounds[structure[x]][structure[y]]);
                        let ceiling = self.fixed
                            * (0..12)
                                .map(|l| self.btab(state, structure[lineages[l].trainee], self.grid.idx(chances[l])))
                                .sum::<f64>()
                            / 12.0
                            * (1.0 + 1e-12);
                        out.push(LoopCandidate { ceiling, members: structure });
                    }
                }
            }
        }
    }
    /// `band`'s three-member structures (sums in four-member units), as in `band4`
    fn band3(&self, state: &mut LoopSearchState, lo: f64, hi: f64, out: &mut Vec<LoopCandidate>) {
        let (ceilings, ranked, pool_size) = (&self.ranked_ceilings, &self.ranked_pool, self.pool.len());
        for a in 0..pool_size {
            if a + 2 >= pool_size || in_four_member_units(ceilings[a] + ceilings[a + 1] + ceilings[a + 2]) < lo {
                break;
            }
            for b in a + 1..pool_size {
                if b + 1 >= pool_size || in_four_member_units(ceilings[a] + ceilings[b] + ceilings[b + 1]) < lo {
                    break;
                }
                for c in b + 1..pool_size {
                    let sum = in_four_member_units(ceilings[a] + ceilings[b] + ceilings[c]);
                    if sum < lo {
                        break;
                    }
                    if sum >= hi {
                        continue;
                    }
                    let mut trio = [ranked[a], ranked[b], ranked[c]];
                    trio.sort_unstable();
                    let structure = [trio[0], trio[1], trio[2], NONE];
                    if !self.required_umas.iter().all(|r| structure.contains(r)) {
                        continue;
                    }
                    let lineages = lins(3);
                    let chances = self.inputs.model.lineage_q_arr(&trio, &|x, y| self.pair_bounds[trio[x]][trio[y]]);
                    let ceiling = self.fixed
                        * (0..3)
                            .map(|l| self.btab(state, trio[lineages[l].trainee], self.grid.idx(chances[l])))
                            .sum::<f64>()
                        / 3.0
                        * (1.0 + 1e-12);
                    out.push(LoopCandidate { ceiling, members: structure });
                }
            }
        }
    }

    /// Try the caps highest first (EXACT_STEPS): the answer is certified under the first whose pass finishes within the
    /// budget; a lower cap's pass reuses everything computed so far and gets a quarter of the budget more. When none
    /// finishes, a pass with the cap at 1 (product odds: no deck searches) gives the answer. (the pass, its cap)
    pub(super) fn certify(&self, state: &mut LoopSearchState) -> (Pass, f64) {
        let checks = self.inputs.deck_job.checks;
        let budget = checks.exact_budget.unwrap_or(EXACT_BUDGET);
        let default_caps;
        let caps: &[f64] = if !self.inputs.deck_job.exact {
            &[]
        } else if let Some(list) = &checks.exact_steps {
            list
        } else {
            default_caps = EXACT_STEPS.map(|cap| if cap == EXACT_RHO { self.maximum_exact_ratio } else { cap });
            &default_caps
        };
        let mut finished: Option<Pass> = None;
        let mut certified_cap = 1.0f64;
        let mut allowance = budget;
        for &cap in caps {
            state.tally.budget = allowance;
            if let Some(pass) = self.run_pass(state, cap) {
                finished = Some(pass);
                certified_cap = cap;
                break;
            } else {
                // Retain scored decks, but grant fresh work before trying a lower cap.
                state.tally.aborted = false;
                allowance = state.tally.work + budget / 4;
            }
        }
        let pass = if let Some(pass) = finished {
            pass
        } else {
            state.tally.budget = usize::MAX;
            self.run_pass(state, 1.0).expect("a pass without deck searches")
        };
        (pass, certified_cap)
    }

    /// One best-first pass over the structures with cap `cap`: ceilings are exact tables or product ceilings times the
    /// cap, the stop rule compares them with the k-th exact value. None when the budget runs out (the pass certifies
    /// nothing).
    fn run_pass(&self, state: &mut LoopSearchState, cap: f64) -> Option<Pass> {
        let first_band_sum = self.maximum_structure_sum * 0.85;
        let mut candidates: Vec<LoopCandidate> = vec![];
        self.band(state, first_band_sum, f64::INFINITY, &mut candidates);
        candidates.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
        let mut pass = Pass {
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
        let mut queue = Queue { candidates, next: 0, upper_sum: first_band_sum };
        loop {
            if state.tally.aborted {
                return None;
            }
            // Treat improvements within 4e-12 relative slack as ties, so equal-valued structures need not all be
            // solved.
            let kth = pass.kth;
            let beats = |ceiling: f64| ceiling > kth * (1.0 + 4e-12);
            if !(queue.next < queue.candidates.len() && beats(cap * queue.candidates[queue.next].ceiling)) {
                if queue.upper_sum <= 0.0 || !beats(cap * self.sum_ceiling(queue.upper_sum)) {
                    break;
                }
                pass.generated += self.widen(state, &mut queue, kth, cap);
                continue;
            }
            let LoopCandidate { ceiling: _, members: structure } = queue.candidates[queue.next];
            queue.next += 1;
            pass.scanned += 1;
            self.solve(state, structure, cap, &mut pass)?;
        }
        let next_ceiling = if queue.next < queue.candidates.len() { queue.candidates[queue.next].ceiling } else { 0.0 };
        let unbanded_ceiling = if queue.upper_sum > 0.0 { self.sum_ceiling(queue.upper_sum) } else { 0.0 };
        pass.next_bound = cap * next_ceiling.max(unbanded_ceiling);
        Some(pass)
    }

    /// Adds the next band of structures below the queue's `upper_sum` (down to the sum the k-th value needs, at most 15%
    /// lower), merged into the unscanned rest in ceiling order; how many it added
    fn widen(&self, state: &mut LoopSearchState, queue: &mut Queue, kth: f64, cap: f64) -> usize {
        let kth_sum = if kth > f64::NEG_INFINITY { kth / cap * 12.0 / (3.0 * self.fixed) * (1.0 - 1e-9) } else { 0.0 };
        let lo = kth_sum.max(queue.upper_sum * 0.85);
        let lo = if lo < self.maximum_structure_sum * 1e-6 { 0.0 } else { lo };
        let mut more: Vec<LoopCandidate> = vec![];
        self.band(state, lo, queue.upper_sum, &mut more);
        queue.upper_sum = lo;
        let added = more.len();
        let mut rest: Vec<LoopCandidate> = queue.candidates.split_off(queue.next);
        rest.extend(more);
        rest.sort_by(|x, y| y.ceiling.partial_cmp(&x.ceiling).unwrap());
        queue.candidates.extend(rest);
        added
    }

    /// A structure's turn in a pass: rejected by its ceilings (cheapest first), else scheduled and solved exactly into
    /// the pass. None when the budget runs out.
    fn solve(&self, state: &mut LoopSearchState, structure: [UmaIndex; 4], cap: f64, pass: &mut Pass) -> Option<()> {
        let kth = pass.kth;
        let beats = |ceiling: f64| ceiling > kth * (1.0 + 4e-12);
        let members = members(&structure);
        let member_count = members.len();
        let lineages = lins(member_count);
        let lineage_count = lineages.len();
        let pair_ceiling_chances =
            self.inputs.model.lineage_q_arr(members, &|x, y| self.pair_bounds[members[x]][members[y]]);
        // cheapest first: the cell bounds as they stand, then the product ceiling with one deck per trainee
        let cell_ceiling = (0..lineage_count)
            .map(|l| self.cell(state, structure[lineages[l].trainee], self.grid.idx(pair_ceiling_chances[l]), cap).0)
            .sum::<f64>()
            / lineage_count as f64;
        if !beats(cell_ceiling) {
            return Some(());
        }
        if !beats(cap * self.relaxed(state, structure, &pair_ceiling_chances)) {
            return Some(());
        }
        if self.shared && !beats(cap * self.relaxed_shared(state, structure, &pair_ceiling_chances)) {
            return Some(());
        }
        // exact ceiling at the pairwise-ceiling chances, valid for any schedule
        if !beats(self.exact_ceiling(state, structure, &pair_ceiling_chances, kth, cap)) {
            return Some(());
        }
        if state.tally.aborted {
            return None;
        }
        let schedule = self.schedule(state, structure, member_count);
        let data = self.inputs.data;
        let win_chances: Vec<Vec<f64>> =
            (0..member_count).map(|i| data.umas[structure[i]].win_chances(data.nr, &schedule.s[i])).collect();
        let chances = self.inputs.model.lineage_q(members, &|x, y| {
            if self.no_race { 0.0 } else { dot(&win_chances[x], &win_chances[y]) }
        });
        let padded_chances: [f64; 12] = std::array::from_fn(|i| chances.get(i).copied().unwrap_or(0.0));
        // rejected only with the schedule found: a better schedule could change this one's fate
        if (self.shared && !beats(cap * self.relaxed_shared(state, structure, &padded_chances)))
            || !beats(self.exact_ceiling(state, structure, &padded_chances, kth, cap))
        {
            pass.schedule_rejects += 1;
            return Some(());
        }
        if self.shared {
            let floor = if kth.is_finite() { kth * lineage_count as f64 } else { f64::NEG_INFINITY };
            let decks = self.exact_shared(state, structure, &padded_chances[..lineage_count], floor, cap);
            if state.tally.aborted {
                return None;
            }
            if decks.is_empty() {
                return Some(());
            }
            let value = decks[0].value / lineage_count as f64;
            let solved =
                SolvedLoop { members: structure, schedule, inheritance_chances: chances, bests: vec![], value };
            pass.add_shared(solved, decks, lineage_count, self.loop_limit);
            return Some(());
        }
        let mut total = 0.0;
        let mut bests = vec![];
        for trainee in 0..member_count {
            let trainee_chances: Vec<f64> =
                (0..lineage_count).filter(|&l| lineages[l].trainee == trainee).map(|l| padded_chances[l]).collect();
            let best = self.exact_best(state, structure[trainee], &trainee_chances, cap);
            total += best.sum;
            bests.push(best);
        }
        if state.tally.aborted {
            return None;
        }
        let value = total / lineage_count as f64;
        pass.add_per_uma(
            SolvedLoop { members: structure, schedule, inheritance_chances: chances, bests, value },
            self.loop_limit,
        );
        Some(())
    }

    /// A structure's schedule: the job's fixed ones, else the optimiser's (kept for later passes)
    fn schedule(&self, state: &mut LoopSearchState, structure: [UmaIndex; 4], member_count: usize) -> Res {
        let cached = state.schedule_cache.get(&structure).cloned();
        cached.unwrap_or_else(|| {
            let schedule = match &self.fixed_schedules {
                Some(fixed) => Res {
                    s: std::array::from_fn(|i| {
                        if i < member_count { fixed[structure[i]].unwrap() } else { [Choice::Rest; NT] }
                    }),
                },
                None => optimize_fam(
                    self.inputs.data,
                    &structure,
                    self.inputs.skill_job.iters,
                    self.inputs.skill_job.seed,
                    60.0,
                ),
            };
            state.schedule_cache.insert(structure, schedule.clone());
            schedule
        })
    }
}
