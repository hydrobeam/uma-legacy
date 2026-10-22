//! Exact career deck searches and their kept memos.
use crate::career::outcomes::CornerKey;
use crate::career::search::DeckRes;
use crate::career::{CareerSetup, DeckSearchAudit, Env, SearchState};
use crate::deck::caps::{EXACT_BUDGET, EXACT_STEPS};
use crate::deck::enumerate::{BandBound, LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome};
use crate::deck::joint::joint_b;
use crate::deck::joint::upper::upper_na;
use crate::deck::space::SelectedDeck;
use serde_json::{Value, json};
/// What a `best_deck` call without start decks reads: the trainee's deck class (everything of her the deck search reads
/// but the folded chances), her folded event-hint chances per outfit (`set_trainee`: the folded buffer the search runs on,
/// to the bit), the spark scale c, the bar th (every th <= 0 as one: each gives no seed and passes the ceiling test),
/// and the work spent before the call (the work budget's tests read it)
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct DeckSearchKey {
    /// `CareerSetup::deck_class` of the trainee.
    class: crate::uma::UmaIndex,
    /// Her folded event-hint chances, outfit by outfit, as bits.
    hint_chances: Vec<u64>,
    /// The spark scale c, as bits.
    spark_scale: u64,
    /// The bar th as bits, 0 for every th <= 0.
    bar: u64,
    /// `SearchState::work` before the call.
    work: usize,
}

/// The cap-free searches' counters (`DeckSearchAudit::cap_free_stats`)
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) struct CfStats {
    pub(crate) reruns: usize,
    pub(crate) open: usize,
    pub(crate) unhandled: usize,
    pub(crate) work: usize,
}

impl CfStats {
    const fn add(&mut self, other: &Self) {
        self.reruns += other.reruns;
        self.open += other.open;
        self.unhandled += other.unhandled;
        self.work += other.work;
    }
    const fn since(&self, other: &Self) -> Self {
        Self {
            reruns: self.reruns - other.reruns,
            open: self.open - other.open,
            unhandled: self.unhandled - other.unhandled,
            work: self.work - other.work,
        }
    }
}

/// Work budget of one cap-free search (`best_deck_cf`), twice `EXACT_BUDGET` unless `CF_BUDGET` overrides it.
/// Past it the search proves nothing (its call is not full).
fn cf_budget(checks: &crate::checks::Checks) -> usize {
    checks.cf_budget.unwrap_or(2 * EXACT_BUDGET)
}

/// Work units per cap-free node: its bounds sum over event counts and chain states.
const CF_NODE_WORK: usize = 15;

/// A memoised `best_deck` outcome: the result, its product search's proof (`product_bound`), the work and exact
/// scorings it added (and its cap-free searches', `cap_free`)
pub(crate) struct DeckMemo {
    result: DeckRes,
    product_bound: Option<f64>,
    work: usize,
    scored: usize,
    certified_cap: f64,
    overflows: usize,
    cap_free: CfStats,
    /// the deck-search nodes and exact scorings it spent; per class, the call's own largest exact / product ratio and the
    /// deck that first reached it (`note_ratio` from nothing); the last job that used it (`DeckSearchMemo::job_number`)
    nodes: usize,
    joints: usize,
    ratio: [Option<RatioDeck>; 4],
    job: u64,
}

/// A class's largest exact / product ratio and the deck that first reached it (`DeckSearchAudit::max_ratio`,
/// `max_deck`).
#[derive(Clone)]
struct RatioDeck {
    ratio: f64,
    trainee: crate::uma::UmaIndex,
    deck: SelectedDeck,
}

/// One deck search's question beside the chances folded into the trainee buffer (`set_trainee`).
#[derive(Clone, Copy)]
pub(crate) struct DeckSearch {
    pub(crate) trainee: crate::uma::UmaIndex,
    /// C, the career's spark scale.
    pub(crate) spark_scale: f64,
    /// th, the value (C x E) the career must beat; at most 0: none.
    pub(crate) bar: f64,
}

/// A deck and one value of it: its product value (a product search's best) or its exact value (a search's answer), as
/// the name using it says.
#[derive(Clone)]
struct ValuedDeck {
    value: f64,
    deck: SelectedDeck,
}

/// The capped search's answer and its best product deck when it found one (`CareerSetup::best_deck_capped`).
struct CappedSearch {
    result: DeckRes,
    product_deck: Option<ValuedDeck>,
}

/// What a deck search changes beside its answer, saved before it (`cf_beside` puts it back): the search state's
/// counters, the whole audit, the deck space's node counter.
struct SearchEffects {
    work: usize,
    last_product_bound: Option<f64>,
    joints: usize,
    audit: DeckSearchAudit,
    nodes: usize,
}

impl SearchEffects {
    fn save(env: &Env) -> Self {
        let Env { setup, state, audit, .. } = env;
        Self {
            work: state.work,
            last_product_bound: state.last_product_bound,
            joints: state.exact_scorings,
            // (the search beside changes the audit's decks too: the copy puts the first search's back)
            audit: audit.clone(),
            nodes: setup.space.nodes.get(),
        }
    }
    fn restore(self, env: &mut Env) {
        let Env { setup, state, audit, .. } = env;
        state.work = self.work;
        state.last_product_bound = self.last_product_bound;
        state.exact_scorings = self.joints;
        *audit = self.audit;
        setup.space.nodes.set(self.nodes);
    }
}

/// The counters a memoised `best_deck` call adds to, at its start (the memo stores what the call added).
struct MemoStart {
    work: usize,
    scored: usize,
    /// The job's certificate before the call (the call's own starts from INFINITY).
    certified_cap: f64,
    overflows: usize,
    nodes: usize,
    joints: usize,
    cap_free: CfStats,
}

thread_local! {
    /// Monotonic job identity for memo work accounting (`DeckSearchMemo::job_number`). This thread-local counter
    /// deliberately survives kept-state swaps: an outer job and its nested simulator jobs need distinct identities.
    pub(crate) static JOB_NO: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// What a setup keeps between jobs: `DeckSearchMemo::decks` and `corners`, keyed by `setup_fp` plus trainee order
/// (the plain pals' order). A DeckSearchKey reads the same model, space and caps in every such job; the memo's work
/// accounting holds for any budget. Adding a veteran can reuse builders whose best pair does not involve her.
#[derive(Default)]
pub(crate) struct DeckSearchCache {
    pub(crate) decks: crate::FxMap<DeckSearchKey, DeckMemo>,
    pub(crate) corners: crate::FxMap<CornerKey, std::rc::Rc<Vec<Vec<f64>>>>,
}

/// setups whose memo is kept, and the entries past which a setup's memo starts afresh (at a job's start)
const DECK_KEEP_SETUPS: usize = 4;

pub(crate) const DECK_KEEP_CAP: usize = 8192;

impl crate::career::DeckSearchMemo {
    /// Publish this job's reusable tables at the career entry point, after search and reporting finish.
    pub(super) fn keep(self, worker: &mut crate::career::state::KeptState) {
        let cache = &mut worker.decks;
        cache.retain(|entry| entry.0 != self.fingerprint);
        cache.push((self.fingerprint, DeckSearchCache { decks: self.decks, corners: self.corners }));
        while cache.len() > DECK_KEEP_SETUPS {
            cache.remove(0);
        }
    }
}

impl CareerSetup<'_> {
    /// a scored deck's exact / product `ratio`, its `class` and trainee (`max_ratio`, `max_deck`)
    fn note_ratio(
        &self,
        audit: &mut DeckSearchAudit,
        class: usize,
        ratio: f64,
        trainee: crate::uma::UmaIndex,
        deck: &SelectedDeck,
    ) {
        self.note_breach(audit, class, ratio);
        if ratio > audit.max_ratio[class] {
            audit.max_ratio[class] = ratio;
            audit.max_deck[class] = Some((trainee, deck.clone()));
        }
    }
    /// a ratio past its class's cap (`cap_breach`)
    fn note_breach(&self, audit: &mut DeckSearchAudit, class: usize, ratio: f64) {
        if ratio > self.space.caps()[class] * (1.0 + 1e-9) {
            audit.cap_breach += 1;
        }
    }
    /// per class, the deck that set `max_ratio` (trainee name, cards), for the cap audit
    pub(crate) fn max_decks(&self, audit: &DeckSearchAudit) -> Value {
        json!(audit.max_deck.iter().map(|max_deck| max_deck.as_ref().map(|(trainee, deck)| json!({"trainee": self.data.umas[*trainee].name, "cards": self.space.pdeck(deck, &[self.data.umas[*trainee].id]).names}))).collect::<Vec<_>>())
    }
}

impl Env<'_> {
    /// Trainee t's best deck at the chances already folded in (`set_trainee`), as the per-uma search finds it: the best
    /// product deck, then every deck whose product value times its class's cap (`cap_class`; at most EXACT_RHO, the top
    /// cap) reaches the best exact value, scored exactly (`upper_na` filters). `search.spark_scale` is C, `search.bar`
    /// the value (C x E) the career must beat: below it the search stops after the product deck. `starts`: decks scored
    /// first (a higher start leaves fewer decks to check). Counts the exact scorings in `scored`, its work in
    /// `state.work`; `full` is false when the work budget ran out or the band was certified below the top cap only.
    ///
    /// A band of more than EXACT_CAP decks is not scored: the band is built again at the next lower cap of the ladder
    /// (EXACT_STEPS, then 1: decks whose product value reaches the best exact one; each class at most that), the first
    /// that fits is scored, its cap is noted in `certified_cap` (the answer is exact under it; 0 when even the last overflows:
    /// the best of the product deck and the starts, certified under nothing) and the call is not `full` (no Ref: a
    /// Ref is a proof under the top cap, kept by later searches and jobs). Without an overflow nothing changes.
    ///
    /// Memoised when there are no start decks (`DeckSearchMemo::decks`): the call is then a function of what
    /// `DeckSearchKey` holds, the work budget aside, and the budget never bound a call whose work stayed within it
    /// (work only grows during a call, and every budget test is `work <= budget` or `work > budget`), so a stored call
    /// that ended at work w answers again under any budget >= w. Classmates at the same chances (a sample pool of mob
    /// veterans gives every trainee the same chances: no relation points) then search once per class; a hit adds the
    /// same work and scorings and leaves the same proof, so everything after it is as if the search had run (only the
    /// node and exact-scoring counters, and `max_ratio`, which the same decks already set, see nothing).
    pub(crate) fn best_deck(&mut self, search: DeckSearch, starts: &[&SelectedDeck], scored: &mut usize) -> DeckRes {
        if !starts.is_empty() {
            return self.best_deck_run(search, starts, scored);
        }
        let key = DeckSearchKey {
            class: self.setup.deck_class[search.trainee],
            hint_chances: self.state.folded_events[search.trainee]
                .iter()
                .flatten()
                .map(|chance| chance.to_bits())
                .collect(),
            spark_scale: search.spark_scale.to_bits(),
            bar: if search.bar <= 0.0 { 0 } else { search.bar.to_bits() },
            work: self.state.work,
        };
        if let Some(stored) = self.memo.decks.get_mut(&key)
            && key.work + stored.work <= self.state.budget
        {
            self.state.work += stored.work;
            *scored += stored.scored;
            self.state.last_product_bound = stored.product_bound;
            self.audit.certified_cap = self.audit.certified_cap.min(stored.certified_cap);
            self.audit.overflows += stored.overflows;
            self.audit.cap_free_stats.add(&stored.cap_free);
            // first use in this job of an outcome an earlier job stored (`KeptState::decks`): this job's own search would have
            // run here, so its node and exact-scoring counters and its per-class ratios count as if it had (a later use
            // in the same job leaves them, as the memo always has). A class's ratio and deck change only when the call's
            // own maximum is above the job's: `note_ratio` keeps the first deck strictly above, so the same as the run
            if stored.job != self.memo.job_number {
                stored.job = self.memo.job_number;
                self.setup.space.nodes.set(self.setup.space.nodes.get() + stored.nodes);
                self.state.exact_scorings += stored.joints;
                CareerSetup::merge_ratios(&mut self.audit.max_ratio, &mut self.audit.max_deck, &stored.ratio);
            }
            let caps = self.setup.space.caps();
            for k in 0..4 {
                if let Some(ratio_deck) = &stored.ratio[k]
                    && ratio_deck.ratio > caps[k] * (1.0 + 1e-9)
                {
                    self.audit.cap_breach += 1;
                }
            }
            return stored.result.clone();
        }
        let start = MemoStart {
            work: self.state.work,
            scored: *scored,
            certified_cap: std::mem::replace(&mut self.audit.certified_cap, f64::INFINITY),
            overflows: self.audit.overflows,
            nodes: self.setup.space.nodes.get(),
            joints: self.state.exact_scorings,
            cap_free: self.audit.cap_free_stats,
        };
        // (the call's own per-class maxima, from nothing, then merged into the job's: see the hit above)
        let (previous_ratios, previous_decks) = (
            std::mem::replace(&mut self.audit.max_ratio, [f64::NEG_INFINITY; 4]),
            std::mem::take(&mut self.audit.max_deck),
        );
        let result = self.best_deck_run(search, starts, scored);
        let certified_cap = self.audit.certified_cap;
        self.audit.certified_cap = start.certified_cap.min(certified_cap);
        let (search_ratios, search_decks) = (
            std::mem::replace(&mut self.audit.max_ratio, previous_ratios),
            std::mem::replace(&mut self.audit.max_deck, previous_decks),
        );
        let ratio: [Option<RatioDeck>; 4] = std::array::from_fn(|k| {
            search_decks[k].as_ref().map(|(trainee, deck)| RatioDeck {
                ratio: search_ratios[k],
                trainee: *trainee,
                deck: deck.clone(),
            })
        });
        CareerSetup::merge_ratios(&mut self.audit.max_ratio, &mut self.audit.max_deck, &ratio);
        if self.state.work <= self.state.budget {
            self.memo.decks.insert(
                key,
                DeckMemo {
                    result: result.clone(),
                    product_bound: self.state.last_product_bound,
                    work: self.state.work - start.work,
                    scored: *scored - start.scored,
                    certified_cap,
                    overflows: self.audit.overflows - start.overflows,
                    cap_free: self.audit.cap_free_stats.since(&start.cap_free),
                    nodes: self.setup.space.nodes.get() - start.nodes,
                    joints: self.state.exact_scorings - start.joints,
                    ratio,
                    job: self.memo.job_number,
                },
            );
        }
        result
    }
}

impl CareerSetup<'_> {
    /// A call's per-class maxima into the job's (`max_ratio`, `max_deck`): a class changes only where the call's ratio
    /// is strictly above, as `note_ratio` keeps the first deck strictly above
    fn merge_ratios(
        max_ratio: &mut [f64; 4],
        max_deck: &mut [Option<(crate::uma::UmaIndex, SelectedDeck)>; 4],
        ratio: &[Option<RatioDeck>; 4],
    ) {
        for k in 0..4 {
            if let Some(ratio_deck) = &ratio[k]
                && ratio_deck.ratio > max_ratio[k]
            {
                max_ratio[k] = ratio_deck.ratio;
                max_deck[k] = Some((ratio_deck.trainee, ratio_deck.deck.clone()));
            }
        }
    }
    /// Trainee t's exact P(all targets) with `deck` at her folded chances, counted in `exact_scorings`.
    fn score_exact(&self, state: &mut SearchState, trainee: crate::uma::UmaIndex, deck: &SelectedDeck) -> f64 {
        state.exact_scorings += 1;
        joint_b(
            self.model,
            self.joint_query(state, trainee),
            &self.ents_of(deck),
            deck.filler_cards,
            self.target_count,
            false,
        )[0]
        .all
    }
}

impl Env<'_> {
    /// `best_deck`'s search. The capped search (`best_deck_capped`) first; when a deck it scored has an exact / product
    /// ratio past its class's cap (`cap_breach`), the cap it pruned by is wrong for this deck space, so a deck it pruned
    /// may beat its answer: the call is answered by the cap-free search instead (`best_deck_cf`; its own budget, the
    /// capped call's band overflows and certificate undone). Without a breach the capped answer stands, unchanged. Job
    /// field `cap_free`: the cap-free search alone. `CAP_RERUN=0` (checks): no rerun, the breach leaves the search not
    /// `complete`.
    fn best_deck_run(&mut self, search: DeckSearch, starts: &[&SelectedDeck], scored: &mut usize) -> DeckRes {
        if self.setup.space.deck_job.cap_free {
            return self.best_deck_cf(search, starts, scored, None, None);
        }
        let (breaches_before, cert_before, overflows_before) =
            (self.audit.cap_breach, self.audit.certified_cap, self.audit.overflows);
        let capped_search = self.best_deck_capped(search, starts, scored);
        if crate::checks::get().cf_check > 0.0 {
            self.cf_beside(search, starts, &capped_search);
        }
        let CappedSearch { result, product_deck } = capped_search;
        if self.audit.cap_breach == breaches_before {
            return result;
        }
        if crate::checks::get().cap_rerun_off {
            self.audit.cap_free_stats.unhandled += 1;
            return result;
        }
        // The cap-free answer supersedes this call's certificate and overflow count.
        self.audit.certified_cap = cert_before;
        self.audit.overflows = overflows_before;
        self.audit.cap_free_stats.reruns += 1;
        let capped = match &result {
            DeckRes::Done(value, deck, _) => Some(ValuedDeck { value: *value, deck: deck.clone() }),
            _ => None,
        };
        self.best_deck_cf(search, starts, scored, product_deck, capped)
    }
    /// CF_CHECK: the cap-free search beside a capped one, everything it changes put back, and their answers compared
    /// (`CFCHK`)
    fn cf_beside(&mut self, search: DeckSearch, starts: &[&SelectedDeck], capped_search: &CappedSearch) {
        let CappedSearch { result, product_deck } = capped_search;
        let saved = SearchEffects::save(self);
        let mut cap_free_scored = 0usize;
        let cap_free_result = self.best_deck_cf(search, starts, &mut cap_free_scored, product_deck.clone(), None);
        saved.restore(self);
        // a deck below the career's bar counts as nothing (the capped search may stop as `Bounded` there)
        let DeckSearch { spark_scale: c, bar: threshold, .. } = search;
        let value_bar =
            if threshold.is_finite() && threshold > 0.0 { threshold / (c * (1.0 + 1e-9)) } else { f64::NEG_INFINITY };
        let counted_value = |cap_free_result: &DeckRes| match cap_free_result {
            DeckRes::Done(value, _, _) if *value >= value_bar => Some(*value),
            _ => None,
        };
        crate::checks::CFCHK.with(|counts| {
            let mut counts = counts.borrow_mut();
            counts.searches += 1;
            match &cap_free_result {
                DeckRes::Done(_, _, false) => {
                    counts.unproven += 1;
                }
                _ => match (counted_value(result), counted_value(&cap_free_result)) {
                    (Some(capped_value), Some(cap_free_value)) => {
                        if cap_free_value > capped_value * (1.0 + 1e-9) {
                            counts.capped_below += 1;
                        }
                        if capped_value > cap_free_value * (1.0 + 1e-9) {
                            counts.cap_free_below += 1;
                        }
                    }
                    (None, Some(_)) => {
                        counts.capped_below += 1;
                    }
                    (Some(_), None) => {
                        counts.cap_free_below += 1;
                    }
                    (None, None) => {}
                },
            }
        });
    }
    /// The cap-free search: the best deck over every legal deck at the chances folded in, pruned only by proven bounds.
    /// The best product deck (`product_deck`, the capped search's, or searched here unseeded: a seed would need a cap
    /// to relate product and exact values) and the starts are scored; then the band of every deck whose cap-free bounds
    /// (`BandBound::CapFree`: `StateBound::cheap` and `StateBound::full`, on `state_value`'s argument and
    /// `group_nuod_lvl`) reach the bar, the incumbent or the career's bar th / c if higher, is built and scored best
    /// product value first, through `upper_na`. Proven (`full`) when the band finished (no node limit, no band
    /// overflow, every legal entry covered: `TargetSources::certified_cap_free`) within CF_BUDGET work; then a best
    /// below th / c is `Bounded` (every deck is), else `Done`. Unproven: the better of its best and the capped answer
    /// (`capped`), not full, counted in `cap_free_stats.open`. Its work is counted apart (`cap_free_stats.work`), so
    /// the career's budget sees what it saw without the rerun; nodes and exact scorings are counted as usual.
    fn best_deck_cf(
        &mut self,
        search: DeckSearch,
        starts: &[&SelectedDeck],
        scored: &mut usize,
        product_deck: Option<ValuedDeck>,
        capped: Option<ValuedDeck>,
    ) -> DeckRes {
        let Env { setup, state, audit, memo } = self;
        let state_bounds = &mut memo.state_bounds;
        let DeckSearch { trainee, spark_scale: c, bar: threshold } = search;
        let uma_id = setup.data.umas[trainee].id;
        let budget = cf_budget(crate::checks::get());
        let mut work = 0usize;
        let ScoredDeck { value: product_value, deck } = if let Some(ValuedDeck { value, deck }) = product_deck {
            ScoredDeck { value, deck }
        } else {
            let nodes_before = setup.space.nodes.get();
            let SearchOutcome { decks: found_decks, incomplete, .. } = setup.space.search(
                state_bounds,
                &setup.deck_chances(state),
                &[trainee],
                &[uma_id],
                &[SearchClause { lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }], threshold: 0.0 }],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            work += 2 * (setup.space.nodes.get() - nodes_before) + 10_000;
            // (a `cap_free` job records no product proof: `solve` and the plan bound with those through EXACT_RHO)
            state.last_product_bound = if incomplete.is_some() || setup.space.deck_job.cap_free {
                None
            } else {
                found_decks.first().map(|best_found| best_found.value)
            };
            if let Some(best_found) = found_decks.into_iter().next() {
                best_found
            } else {
                audit.cap_free_stats.work += work;
                return DeckRes::NoDeck;
            }
        };
        let mut best = (setup.score_exact(state, trainee, &deck), deck);
        *scored += 1;
        work += 200;
        if product_value > 0.0 {
            let class = setup.space.sel_class(&best.1);
            setup.note_ratio(audit, class, best.0 / product_value, trainee, &best.1);
        }
        for start in starts {
            let value = setup.score_exact(state, trainee, start);
            *scored += 1;
            work += 200;
            if value > best.0 {
                best = (value, (*start).clone());
            }
        }
        // a deck below th / c cannot make the career (every deck with c x value >= th is at least this)
        let value_bar =
            if threshold.is_finite() && threshold > 0.0 { threshold / (c * (1.0 + 1e-9)) } else { f64::NEG_INFINITY };
        let nodes_before = setup.space.nodes.get();
        let node_limit = nodes_before.saturating_add(budget.saturating_sub(work) / CF_NODE_WORK);
        let SearchOutcome { decks: mut more, incomplete, .. } = setup.space.search(
            state_bounds,
            &setup.deck_chances(state),
            &[trainee],
            &[uma_id],
            &[SearchClause {
                lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }],
                threshold: best.0.max(value_bar),
            }],
            SearchMode::Band {
                ratio_cap: 1.0,
                max_decks: setup.space.band_cap(),
                bound: BandBound::CapFree { node_limit },
            },
        );
        work += CF_NODE_WORK * (setup.space.nodes.get() - nodes_before) + 5 * more.len() + 10_000;
        let mut full = incomplete.is_none();
        more.sort_by(|first, second| second.value.partial_cmp(&first.value).unwrap());
        for ScoredDeck { value: candidate_product_value, deck: candidate } in more {
            if work > budget {
                full = false;
                break;
            }
            if candidate.entries == best.1.entries
                && candidate.plain_pals == best.1.plain_pals
                && candidate.filler_cards == best.1.filler_cards
            {
                continue;
            }
            let class = setup.space.sel_class(&candidate);
            let entries = setup.ents_of(&candidate);
            if let Some(ub) = upper_na(setup.model, setup.joint_query(state, trainee), &entries, candidate.filler_cards)
            {
                work += 25;
                if ub[0] * (1.0 + 1e-9) < best.0.max(value_bar) {
                    continue;
                }
            }
            let value = setup.score_exact(state, trainee, &candidate);
            *scored += 1;
            work += 200;
            if candidate_product_value > 0.0 {
                setup.note_ratio(audit, class, value / candidate_product_value, trainee, &candidate);
            }
            if value > best.0 {
                best = (value, candidate);
            }
        }
        if work > budget {
            full = false;
        }
        audit.cap_free_stats.work += work;
        if !full {
            audit.cap_free_stats.open += 1;
            if let Some(ValuedDeck { value, deck }) = capped
                && value > best.0
            {
                best = (value, deck);
            }
            return DeckRes::Done(best.0, best.1, false);
        }
        if best.0 < value_bar {
            return DeckRes::Bounded;
        }
        DeckRes::Done(best.0, best.1, true)
    }
    /// The capped search (the deck search under the assumed caps, `EXACT_CAPS`), and its best product deck when it found
    /// one (the cap-free rerun starts from it)
    fn best_deck_capped(&mut self, search: DeckSearch, starts: &[&SelectedDeck], scored: &mut usize) -> CappedSearch {
        let Env { setup, state, audit, memo } = self;
        let state_bounds = &mut memo.state_bounds;
        let DeckSearch { trainee, spark_scale: c, bar: threshold } = search;
        let uma_id = setup.data.umas[trainee].id;
        let nodes_before = setup.space.nodes.get();
        // the product search starts at the bar the career must clear (a hair below the cut below): it prunes every
        // branch that cannot reach it and finds nothing when no deck can, and a deck that clears it is the one an
        // unseeded search finds (the seed is below the maximum, so the first deck reaching it is never pruned)
        let rho = setup.space.rho_top();
        let seed = if threshold.is_finite() && threshold > 0.0 {
            threshold / (rho * c * (1.0 + 1e-9)) * (1.0 - 1e-12)
        } else {
            f64::NEG_INFINITY
        };
        let SearchOutcome { decks: found_decks, incomplete, .. } = setup.space.search(
            state_bounds,
            &setup.deck_chances(state),
            &[trainee],
            &[uma_id],
            &[SearchClause { lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }], threshold: 0.0 }],
            SearchMode::Best { seed },
        );
        state.work += 2 * (setup.space.nodes.get() - nodes_before) + 10_000;
        // what it proves (`ruled`): the deck found is the best (the first deck above the seed the search keeps is only
        // ever replaced by a better one, and a branch is pruned only below the best so far); none found, every deck is
        // at most the seed. Nothing when enumeration was incomplete (more than 128 card characters).
        state.last_product_bound = if incomplete.is_some() {
            None
        } else if let Some(best_found) = found_decks.first() {
            Some(best_found.value)
        } else if seed > f64::NEG_INFINITY {
            Some(seed)
        } else {
            None
        };
        let Some(ScoredDeck { value: product_value, deck }) = found_decks.into_iter().next() else {
            let result = if seed > f64::NEG_INFINITY { DeckRes::Bounded } else { DeckRes::NoDeck };
            return CappedSearch { result, product_deck: None };
        };
        // the candidate's own ceiling from its best product deck
        if rho * c * product_value * (1.0 + 1e-9) < threshold {
            return CappedSearch {
                result: DeckRes::Bounded,
                product_deck: Some(ValuedDeck { value: product_value, deck }),
            };
        }
        let product_deck = Some(ValuedDeck { value: product_value, deck: deck.clone() });
        let mut best = (setup.score_exact(state, trainee, &deck), deck);
        *scored += 1;
        state.work += 200;
        if product_value > 0.0 {
            let class = setup.space.sel_class(&best.1);
            setup.note_ratio(audit, class, best.0 / product_value, trainee, &best.1);
        }
        for start in starts {
            let value = setup.score_exact(state, trainee, start);
            *scored += 1;
            state.work += 200;
            if value > best.0 {
                best = (value, (*start).clone());
            }
        }
        let mut full = false;
        // the cap ladder below the top cap (a band over EXACT_CAP decks is retried at the next one)
        let mut ladder = EXACT_STEPS[1..].iter().copied().chain(std::iter::once(1.0)).filter(|&cap| cap < rho);
        let mut rho = rho;
        while state.work <= state.budget {
            let nodes_before = setup.space.nodes.get();
            // `uncapped`: the band was built without the cap (SBOUND=2): every deck in it may beat the bar
            let SearchOutcome { decks: mut more, incomplete, uncapped } = setup.space.search(
                state_bounds,
                &setup.deck_chances(state),
                &[trainee],
                &[uma_id],
                &[SearchClause { lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }], threshold: best.0 }],
                SearchMode::Band {
                    ratio_cap: rho,
                    max_decks: setup.space.band_cap(),
                    bound: BandBound::StateBound { mode: crate::checks::get().sbound },
                },
            );
            state.work += 2 * (setup.space.nodes.get() - nodes_before) + 5 * more.len() + 10_000;
            if incomplete.is_some() {
                audit.overflows += 1;
                match ladder.next() {
                    Some(next_cap) if !uncapped => {
                        rho = next_cap;
                        continue;
                    }
                    _ => {
                        audit.certified_cap = 0.0;
                        break;
                    }
                }
            }
            full = rho == setup.space.rho_top();
            let mut cut = false;
            more.sort_by(|first, second| second.value.partial_cmp(&first.value).unwrap());
            for ScoredDeck { value: candidate_product_value, deck: candidate } in more {
                if !uncapped && rho * candidate_product_value * (1.0 + 1e-9) < best.0 {
                    break;
                }
                if state.work > state.budget {
                    full = false;
                    cut = true;
                    break;
                }
                if candidate.entries == best.1.entries
                    && candidate.plain_pals == best.1.plain_pals
                    && candidate.filler_cards == best.1.filler_cards
                {
                    continue;
                }
                // below the bar at its own class's cap (the bar rose since the band was built)
                let class = setup.space.sel_class(&candidate);
                if !uncapped && setup.space.caps()[class].min(rho) * candidate_product_value * (1.0 + 1e-9) < best.0 {
                    continue;
                }
                let entries = setup.ents_of(&candidate);
                if let Some(ub) =
                    upper_na(setup.model, setup.joint_query(state, trainee), &entries, candidate.filler_cards)
                {
                    state.work += 25;
                    if ub[0] * (1.0 + 1e-9) < best.0 {
                        continue;
                    }
                }
                let value = setup.score_exact(state, trainee, &candidate);
                *scored += 1;
                state.work += 200;
                if candidate_product_value > 0.0 {
                    setup.note_ratio(audit, class, value / candidate_product_value, trainee, &candidate);
                }
                if value > best.0 {
                    best = (value, candidate);
                }
            }
            if !cut {
                audit.certified_cap = audit.certified_cap.min(rho);
            }
            break;
        }
        CappedSearch { result: DeckRes::Done(best.0, best.1, full), product_deck }
    }
}
