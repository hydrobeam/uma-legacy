//! Exact career deck searches and their kept memos.
use super::{DeckRes, Env};
use crate::deck::enumerate::{LineageChance, ScoredDeck, SearchClause, SearchMode, SearchOutcome};
use crate::deck::{EXACT_BUDGET, EXACT_STEPS, SelectedDeck, Value, joint_b, json, upper_na};

/// What a `best_deck` call without start decks reads: the trainee's deck class (everything of her the deck search reads
/// but the folded chances), her folded event-hint chances per outfit (`set_trainee`: the model state the search runs on,
/// to the bit), the spark scale c, the bar th (every th <= 0 as one: each gives no seed and passes the ceiling test),
/// and the work spent before the call (the work budget's tests read it)
#[derive(PartialEq, Eq, Hash)]
pub(crate) struct DeckKey {
    /// `Env::deck_class` of the trainee.
    class: crate::UmaIndex,
    /// Her folded event-hint chances, outfit by outfit, as bits.
    hint_chances: Vec<u64>,
    /// The spark scale c, as bits.
    spark_scale: u64,
    /// The bar th as bits, 0 for every th <= 0.
    bar: u64,
    /// `Env::work` before the call.
    work: usize,
}

/// The cap-free searches' counters (`Env::cap_free_stats`)
#[derive(Clone, Copy, Default, PartialEq, Debug)]
pub(crate) struct CfStats {
    pub(crate) reruns: usize,
    pub(crate) open: usize,
    pub(crate) unhandled: usize,
    pub(crate) work: usize,
}

impl CfStats {
    const fn add(&mut self, o: &Self) {
        self.reruns += o.reruns;
        self.open += o.open;
        self.unhandled += o.unhandled;
        self.work += o.work;
    }
    const fn since(&self, o: &Self) -> Self {
        Self {
            reruns: self.reruns - o.reruns,
            open: self.open - o.open,
            unhandled: self.unhandled - o.unhandled,
            work: self.work - o.work,
        }
    }
}

/// Work budget of one cap-free search (`best_deck_cf`), twice `EXACT_BUDGET` unless `CF_BUDGET` overrides it.
/// Past it the search proves nothing (its call is not full).
fn cf_budget() -> usize {
    static C: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("CF_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(2 * EXACT_BUDGET))
}

/// Work units per cap-free node: its bounds sum over event counts and chain states.
const CF_NODE_WORK: usize = 15;

/// A memoised `best_deck` outcome: the result, its product search's proof (`last_pv`), the work and exact scorings it
/// added (and its cap-free searches', `cf`)
pub(crate) struct DeckMemo {
    res: DeckRes,
    pv: Option<f64>,
    work: usize,
    scored: usize,
    cert: f64,
    overflows: usize,
    cf: CfStats,
    /// the deck-search nodes and exact scorings it spent; per class, the call's own largest exact / product ratio and the
    /// deck that first reached it (`note_ratio` from nothing); the last job that used it (`Env::job_number`)
    nodes: usize,
    joints: usize,
    ratio: [Option<RatioDeck>; 4],
    job: u64,
}

/// A class's largest exact / product ratio and the deck that first reached it (`Env::max_ratio`, `Env::max_deck`).
#[derive(Clone)]
struct RatioDeck {
    ratio: f64,
    trainee: crate::UmaIndex,
    deck: SelectedDeck,
}

/// One deck search's question beside the chances folded into the model (`set_trainee`).
#[derive(Clone, Copy)]
struct DeckSearch {
    trainee: crate::UmaIndex,
    /// C, the career's spark scale.
    spark_scale: f64,
    /// th, the value (C x E) the career must beat; at most 0: none.
    bar: f64,
}

/// A deck and one value of it: its product value (a product search's best) or its exact value (a search's answer), as
/// the name using it says.
#[derive(Clone)]
struct ValuedDeck {
    value: f64,
    deck: SelectedDeck,
}

/// The capped search's answer and its best product deck when it found one (`Env::best_deck_capped`).
struct CappedSearch {
    result: DeckRes,
    product_deck: Option<ValuedDeck>,
}

/// What a deck search changes in the Env beside its answer, saved before it (`cf_beside` puts it back).
struct SearchEffects {
    work: usize,
    last_pv: Option<f64>,
    max_ratio: [f64; 4],
    max_deck: [Option<(crate::UmaIndex, SelectedDeck)>; 4],
    cap_breach: usize,
    cfs: CfStats,
    cert: f64,
    overflows: usize,
    nodes: usize,
    joints: usize,
}

impl SearchEffects {
    fn save(env: &Env) -> Self {
        Self {
            work: env.work,
            last_pv: env.last_product_bound,
            max_ratio: env.max_ratio,
            max_deck: env.max_deck.clone(),
            cap_breach: env.cap_breach,
            cfs: env.cap_free_stats,
            cert: env.cert,
            overflows: env.overflows,
            nodes: env.space.nodes.get(),
            joints: crate::deck::JCOUNT.with(std::cell::Cell::get),
        }
    }
    fn restore(self, env: &mut Env) {
        env.work = self.work;
        env.last_product_bound = self.last_pv;
        env.max_ratio = self.max_ratio;
        env.max_deck = self.max_deck;
        env.cap_breach = self.cap_breach;
        env.cap_free_stats = self.cfs;
        env.cert = self.cert;
        env.overflows = self.overflows;
        env.space.nodes.set(self.nodes);
        crate::deck::JCOUNT.with(|c| c.set(self.joints));
    }
}

/// The counters a memoised `best_deck` call adds to, at its start (the memo stores what the call added).
struct MemoStart {
    work: usize,
    scored: usize,
    /// The job's certificate before the call (the call's own starts from INFINITY).
    cert: f64,
    overflows: usize,
    nodes: usize,
    joints: usize,
    cap_free: CfStats,
}

thread_local! {
    /// `best_deck`'s memo kept between jobs (the page's worker runs a plan, then a Reroll's plans with her added: the
    /// builders whose best pair doesn't involve her search the same deck problems again), per setup: `setup_fp` (what
    /// sets the model, the deck space and the class caps, `cap_audit` included) with the job's trainees (the plain pals'
    /// order), the last few. Exact: a `DeckKey` reads the same model, space and caps in every job of one setup, and the
    /// work budget's argument holds for any budget (`best_deck`)
    pub(crate) static DECK_KEEP: std::cell::RefCell<Vec<(u64, Kept)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// jobs run so far (`Env::job_number`)
    pub(crate) static JOB_NO: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// what a setup keeps between jobs: `Env::deck_memo` and `Env::corner_memo`
#[derive(Default)]
pub(crate) struct Kept {
    pub(crate) decks: crate::FxMap<DeckKey, DeckMemo>,
    pub(crate) corners: crate::FxMap<(crate::UmaIndex, Vec<usize>, usize), std::rc::Rc<Vec<Vec<f64>>>>,
}

/// setups whose memo is kept, and the entries past which a setup's memo starts afresh (at a job's start)
const DECK_KEEP_SETUPS: usize = 4;

pub(crate) const DECK_KEEP_CAP: usize = 8192;

impl Drop for Env<'_> {
    fn drop(&mut self) {
        let kept = Kept { decks: std::mem::take(&mut self.deck_memo), corners: std::mem::take(&mut self.corner_memo) };
        let fingerprint = self.memo_fingerprint;
        DECK_KEEP.with(|cache| {
            let mut cache = cache.borrow_mut();
            cache.retain(|entry| entry.0 != fingerprint);
            cache.push((fingerprint, kept));
            while cache.len() > DECK_KEEP_SETUPS {
                cache.remove(0);
            }
        });
    }
}

impl Env<'_> {
    /// a scored deck's exact / product ratio r, class k, trainee t (`max_ratio`, `max_deck`)
    fn note_ratio(&mut self, k: usize, r: f64, t: crate::UmaIndex, sel: &SelectedDeck) {
        self.note_breach(k, r);
        if r > self.max_ratio[k] {
            self.max_ratio[k] = r;
            self.max_deck[k] = Some((t, sel.clone()));
        }
    }
    /// a ratio past its class's cap (`cap_breach`)
    fn note_breach(&mut self, k: usize, r: f64) {
        if r > self.space.caps()[k] * (1.0 + 1e-9) {
            self.cap_breach += 1;
        }
    }
    /// per class, the deck that set `max_ratio` (trainee name, cards), for the cap audit
    pub(crate) fn max_decks(&self) -> Value {
        json!(self.max_deck.iter().map(|x| x.as_ref().map(|(t, s)| json!({"trainee": self.data.umas[*t].name, "cards": self.space.pdeck(s, &[self.data.umas[*t].id]).names}))).collect::<Vec<_>>())
    }
    /// Trainee t's best deck at the chances already folded in (`set_trainee`), as the per-uma search finds it: the best
    /// product deck, then every deck whose product value times its class's cap (`cap_class`; at most EXACT_RHO, the top
    /// cap) reaches the best exact value, scored exactly (`upper_na` filters). `c` the spark scale, `th` the value (C x E)
    /// the career must beat: below it the search stops after the product deck. `starts`: decks scored first (a higher
    /// start leaves fewer decks to check). Counts the exact scorings in `scored`, its work in `self.work`; `full` is
    /// false when the work budget ran out or the band was certified below the top cap only.
    ///
    /// A band of more than EXACT_CAP decks is not scored: the band is built again at the next lower cap of the ladder
    /// (EXACT_STEPS, then 1: decks whose product value reaches the best exact one; each class at most that), the first
    /// that fits is scored, its cap is noted in `cert` (the answer is exact under it; 0 when even the last overflows:
    /// the best of the product deck and the starts, certified under nothing) and the call is not `full` (no Ref: a
    /// Ref is a proof under the top cap, kept by later searches and jobs). Without an overflow nothing changes.
    ///
    /// Memoised when there are no start decks (`deck_memo`): the call is then a function of what `DeckKey` holds, the
    /// work budget aside, and the budget never bound a call whose work stayed within it (work only grows during a
    /// call, and every budget test is `work <= budget` or `work > budget`), so a stored call that ended at work w
    /// answers again under any budget >= w. Classmates at the same chances (a sample pool of mob veterans gives every
    /// trainee the same chances: no relation points) then search once per class; a hit adds the same work and
    /// scorings and leaves the same proof, so everything after it is as if the search had run (only the node and
    /// exact-scoring counters, and `max_ratio`, which the same decks already set, see nothing).
    pub(crate) fn best_deck(
        &mut self,
        t: crate::UmaIndex,
        c: f64,
        th: f64,
        starts: &[&SelectedDeck],
        scored: &mut usize,
    ) -> DeckRes {
        if !starts.is_empty() {
            return self.best_deck_run(DeckSearch { trainee: t, spark_scale: c, bar: th }, starts, scored);
        }
        let key = DeckKey {
            class: self.deck_class[t],
            hint_chances: self.model.ews[t].iter().flatten().map(|x| x.to_bits()).collect(),
            spark_scale: c.to_bits(),
            bar: if th <= 0.0 { 0 } else { th.to_bits() },
            work: self.work,
        };
        if let Some(m) = self.deck_memo.get_mut(&key)
            && key.work + m.work <= self.budget
        {
            self.work += m.work;
            *scored += m.scored;
            self.last_product_bound = m.pv;
            self.cert = self.cert.min(m.cert);
            self.overflows += m.overflows;
            self.cap_free_stats.add(&m.cf);
            // first use in this job of an outcome an earlier job stored (`DECK_KEEP`): this job's own search would have
            // run here, so its node and exact-scoring counters and its per-class ratios count as if it had (a later use
            // in the same job leaves them, as the memo always has). A class's ratio and deck change only when the call's
            // own maximum is above the job's: `note_ratio` keeps the first deck strictly above, so the same as the run
            if m.job != self.job_number {
                m.job = self.job_number;
                self.space.nodes.set(self.space.nodes.get() + m.nodes);
                crate::deck::JCOUNT.with(|c| c.set(c.get() + m.joints));
                Self::merge_ratios(&mut self.max_ratio, &mut self.max_deck, &m.ratio);
            }
            let caps = self.space.caps();
            for k in 0..4 {
                if let Some(rd) = &m.ratio[k]
                    && rd.ratio > caps[k] * (1.0 + 1e-9)
                {
                    self.cap_breach += 1;
                }
            }
            return m.res.clone();
        }
        let start = MemoStart {
            work: self.work,
            scored: *scored,
            cert: std::mem::replace(&mut self.cert, f64::INFINITY),
            overflows: self.overflows,
            nodes: self.space.nodes.get(),
            joints: crate::deck::JCOUNT.with(std::cell::Cell::get),
            cap_free: self.cap_free_stats,
        };
        // (the call's own per-class maxima, from nothing, then merged into the job's: see the hit above)
        let (r0, d0) =
            (std::mem::replace(&mut self.max_ratio, [f64::NEG_INFINITY; 4]), std::mem::take(&mut self.max_deck));
        let res = self.best_deck_run(DeckSearch { trainee: t, spark_scale: c, bar: th }, starts, scored);
        let cert = self.cert;
        self.cert = start.cert.min(cert);
        let (r1, d1) = (std::mem::replace(&mut self.max_ratio, r0), std::mem::replace(&mut self.max_deck, d0));
        let ratio: [Option<RatioDeck>; 4] = std::array::from_fn(|k| {
            d1[k].as_ref().map(|(trainee, deck)| RatioDeck { ratio: r1[k], trainee: *trainee, deck: deck.clone() })
        });
        Self::merge_ratios(&mut self.max_ratio, &mut self.max_deck, &ratio);
        if self.work <= self.budget {
            self.deck_memo.insert(
                key,
                DeckMemo {
                    res: res.clone(),
                    pv: self.last_product_bound,
                    work: self.work - start.work,
                    scored: *scored - start.scored,
                    cert,
                    overflows: self.overflows - start.overflows,
                    cf: self.cap_free_stats.since(&start.cap_free),
                    nodes: self.space.nodes.get() - start.nodes,
                    joints: crate::deck::JCOUNT.with(std::cell::Cell::get) - start.joints,
                    ratio,
                    job: self.job_number,
                },
            );
        }
        res
    }
    /// A call's per-class maxima into the job's (`max_ratio`, `max_deck`): a class changes only where the call's ratio
    /// is strictly above, as `note_ratio` keeps the first deck strictly above
    fn merge_ratios(
        max_ratio: &mut [f64; 4],
        max_deck: &mut [Option<(crate::UmaIndex, SelectedDeck)>; 4],
        ratio: &[Option<RatioDeck>; 4],
    ) {
        for k in 0..4 {
            if let Some(rd) = &ratio[k]
                && rd.ratio > max_ratio[k]
            {
                max_ratio[k] = rd.ratio;
                max_deck[k] = Some((rd.trainee, rd.deck.clone()));
            }
        }
    }
    /// `best_deck`'s search. The capped search (`best_deck_capped`) first; when a deck it scored has an exact / product
    /// ratio past its class's cap (`cap_breach`), the cap it pruned by is wrong for this deck space, so a deck it pruned
    /// may beat its answer: the call is answered by the cap-free search instead (`best_deck_cf`; its own budget, the
    /// capped call's band overflows and certificate undone). Without a breach the capped answer stands, unchanged. Job
    /// field `cap_free`: the cap-free search alone. `CAP_RERUN=0` (checks): no rerun, the breach leaves the search not
    /// `complete`.
    fn best_deck_run(&mut self, search: DeckSearch, starts: &[&SelectedDeck], scored: &mut usize) -> DeckRes {
        if self.space.deck_job.cap_free {
            return self.best_deck_cf(search, starts, scored, None, None);
        }
        let (breaches_before, cert_before, overflows_before) = (self.cap_breach, self.cert, self.overflows);
        let CappedSearch { result: res, product_deck } = self.best_deck_capped(search, starts, scored);
        if crate::deck::cf_check() > 0.0 {
            self.cf_beside(search, starts, &res, &product_deck);
        }
        if self.cap_breach == breaches_before {
            return res;
        }
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *OFF.get_or_init(|| std::env::var("CAP_RERUN").is_ok_and(|v| v == "0")) {
            self.cap_free_stats.unhandled += 1;
            return res;
        }
        // The cap-free answer supersedes this call's certificate and overflow count.
        self.cert = cert_before;
        self.overflows = overflows_before;
        self.cap_free_stats.reruns += 1;
        let capped = match &res {
            DeckRes::Done(v, s, _) => Some(ValuedDeck { value: *v, deck: s.clone() }),
            _ => None,
        };
        self.best_deck_cf(search, starts, scored, product_deck, capped)
    }
    /// CF_CHECK: the cap-free search beside a capped one, everything it changes put back, and their answers compared
    /// (`CFCHK`)
    fn cf_beside(
        &mut self,
        search: DeckSearch,
        starts: &[&SelectedDeck],
        res: &DeckRes,
        product_deck: &Option<ValuedDeck>,
    ) {
        let saved = SearchEffects::save(self);
        let mut sc = 0usize;
        let r = self.best_deck_cf(search, starts, &mut sc, product_deck.clone(), None);
        saved.restore(self);
        // a deck below the career's bar counts as nothing (the capped search may stop as `Bounded` there)
        let DeckSearch { spark_scale: c, bar: th, .. } = search;
        let the = if th.is_finite() && th > 0.0 { th / (c * (1.0 + 1e-9)) } else { f64::NEG_INFINITY };
        let val = |r: &DeckRes| match r {
            DeckRes::Done(v, _, _) if *v >= the => Some(*v),
            _ => None,
        };
        crate::deck::CFCHK.with(|k| {
            let mut k = k.borrow_mut();
            k.0 += 1;
            match &r {
                DeckRes::Done(_, _, false) => {
                    k.3 += 1;
                }
                _ => match (val(res), val(&r)) {
                    (Some(a), Some(b)) => {
                        if b > a * (1.0 + 1e-9) {
                            k.1 += 1;
                        }
                        if a > b * (1.0 + 1e-9) {
                            k.2 += 1;
                        }
                    }
                    (None, Some(_)) => {
                        k.1 += 1;
                    }
                    (Some(_), None) => {
                        k.2 += 1;
                    }
                    (None, None) => {}
                },
            }
        });
    }
    /// The cap-free search: the best deck over every legal deck at the chances folded in, pruned only by proven bounds.
    /// The best product deck (`product_deck`, the capped search's, or searched here unseeded: a seed would need a cap
    /// to relate product and exact values) and the starts are scored; then the band of every deck whose cap-free bounds
    /// (`CF_ON`: `StateBound::cheap` and `StateBound::full`, on `state_value`'s argument and `group_nuod_lvl`) reach
    /// the bar, the incumbent or the career's bar th / c if higher, is built and scored best product value first,
    /// through `upper_na`. Proven (`full`) when the band finished (no node limit, no band overflow, every legal entry
    /// covered: `TargetSources::certified_cap_free`) within CF_BUDGET work; then a best below th / c is `Bounded` (every deck is), else
    /// `Done`. Unproven: the better of its best and the capped answer (`capped`), not full, counted in
    /// `cap_free_stats.open`. Its work is counted apart (`cap_free_stats.work`), so the career's budget sees what it
    /// saw without the rerun; nodes and exact scorings are counted as usual.
    fn best_deck_cf(
        &mut self,
        search: DeckSearch,
        starts: &[&SelectedDeck],
        scored: &mut usize,
        product_deck: Option<ValuedDeck>,
        capped: Option<ValuedDeck>,
    ) -> DeckRes {
        let DeckSearch { trainee: t, spark_scale: c, bar: th } = search;
        let ntar = self.target_count;
        let uid = self.data.umas[t].id;
        let budget = cf_budget();
        let mut work = 0usize;
        let ScoredDeck { value: pv, deck: sel } = if let Some(ValuedDeck { value, deck }) = product_deck {
            ScoredDeck { value, deck }
        } else {
            let n0 = self.space.nodes.get();
            let SearchOutcome { decks: r, incomplete } = self.space.search(
                &self.model,
                &[t],
                &[uid],
                &[SearchClause { lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }], threshold: 0.0 }],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            work += 2 * (self.space.nodes.get() - n0) + 10_000;
            // (a `cap_free` job records no product proof: `solve` and the plan bound with those through EXACT_RHO)
            self.last_product_bound =
                if incomplete.is_some() || self.space.deck_job.cap_free { None } else { r.first().map(|x| x.value) };
            if let Some(x) = r.into_iter().next() {
                x
            } else {
                self.cap_free_stats.work += work;
                return DeckRes::NoDeck;
            }
        };
        let score = |env: &Env, sel: &SelectedDeck| {
            joint_b(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler_cards, ntar, false)[0].all
        };
        let mut best = (score(self, &sel), sel);
        *scored += 1;
        work += 200;
        if pv > 0.0 {
            let k = self.space.sel_class(&best.1);
            self.note_ratio(k, best.0 / pv, t, &best.1);
        }
        for s in starts {
            let x = score(self, s);
            *scored += 1;
            work += 200;
            if x > best.0 {
                best = (x, (*s).clone());
            }
        }
        // a deck below th / c cannot make the career (every deck with c x value >= th is at least this)
        let the = if th.is_finite() && th > 0.0 { th / (c * (1.0 + 1e-9)) } else { f64::NEG_INFINITY };
        let n0 = self.space.nodes.get();
        crate::deck::CF_ON.with(|x| x.set(true));
        crate::deck::CF_LIM.with(|x| x.set(n0.saturating_add(budget.saturating_sub(work) / CF_NODE_WORK)));
        let SearchOutcome { decks: mut more, incomplete } = self.space.search(
            &self.model,
            &[t],
            &[uid],
            &[SearchClause {
                lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }],
                threshold: best.0.max(the),
            }],
            SearchMode::Band { ratio_cap: 1.0, max_decks: self.space.band_cap() },
        );
        crate::deck::CF_ON.with(|x| x.set(false));
        crate::deck::CF_LIM.with(|x| x.set(usize::MAX));
        work += CF_NODE_WORK * (self.space.nodes.get() - n0) + 5 * more.len() + 10_000;
        let mut full = incomplete.is_none();
        more.sort_by(|x, y| y.value.partial_cmp(&x.value).unwrap());
        for ScoredDeck { value: v, deck: s } in more {
            if work > budget {
                full = false;
                break;
            }
            if s.entries == best.1.entries && s.plain_pals == best.1.plain_pals && s.filler_cards == best.1.filler_cards
            {
                continue;
            }
            let k = self.space.sel_class(&s);
            let e = self.ents_of(&s);
            if let Some(ub) = upper_na(&self.model, t, &[0.0], &e, s.filler_cards) {
                work += 25;
                if ub[0] * (1.0 + 1e-9) < best.0.max(the) {
                    continue;
                }
            }
            let x = score(self, &s);
            *scored += 1;
            work += 200;
            if v > 0.0 {
                self.note_ratio(k, x / v, t, &s);
            }
            if x > best.0 {
                best = (x, s);
            }
        }
        if work > budget {
            full = false;
        }
        self.cap_free_stats.work += work;
        if !full {
            self.cap_free_stats.open += 1;
            if let Some(ValuedDeck { value, deck }) = capped
                && value > best.0
            {
                best = (value, deck);
            }
            return DeckRes::Done(best.0, best.1, false);
        }
        if best.0 < the {
            return DeckRes::Bounded;
        }
        DeckRes::Done(best.0, best.1, true)
    }
    /// The capped search (the deck search under the assumed caps, `EXACT_CAPS`), and its best product deck when it found
    /// one (the cap-free rerun starts from it)
    fn best_deck_capped(&mut self, search: DeckSearch, starts: &[&SelectedDeck], scored: &mut usize) -> CappedSearch {
        let DeckSearch { trainee: t, spark_scale: c, bar: th } = search;
        let ntar = self.target_count;
        let uid = self.data.umas[t].id;
        let n0 = self.space.nodes.get();
        // the product search starts at the bar the career must clear (a hair below the cut below): it prunes every
        // branch that cannot reach it and finds nothing when no deck can, and a deck that clears it is the one an
        // unseeded search finds (the seed is below the maximum, so the first deck reaching it is never pruned)
        let rho = self.space.rho_top();
        let seed =
            if th.is_finite() && th > 0.0 { th / (rho * c * (1.0 + 1e-9)) * (1.0 - 1e-12) } else { f64::NEG_INFINITY };
        let SearchOutcome { decks: r, incomplete } = self.space.search(
            &self.model,
            &[t],
            &[uid],
            &[SearchClause { lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }], threshold: 0.0 }],
            SearchMode::Best { seed },
        );
        self.work += 2 * (self.space.nodes.get() - n0) + 10_000;
        // what it proves (`ruled`): the deck found is the best (the first deck above the seed the search keeps is only
        // ever replaced by a better one, and a branch is pruned only below the best so far); none found, every deck is
        // at most the seed. Nothing when enumeration was incomplete (more than 128 card characters).
        self.last_product_bound = if incomplete.is_some() {
            None
        } else if let Some(x) = r.first() {
            Some(x.value)
        } else if seed > f64::NEG_INFINITY {
            Some(seed)
        } else {
            None
        };
        let Some(ScoredDeck { value: pv, deck: sel }) = r.into_iter().next() else {
            let result = if seed > f64::NEG_INFINITY { DeckRes::Bounded } else { DeckRes::NoDeck };
            return CappedSearch { result, product_deck: None };
        };
        // the candidate's own ceiling from its best product deck
        if rho * c * pv * (1.0 + 1e-9) < th {
            return CappedSearch { result: DeckRes::Bounded, product_deck: Some(ValuedDeck { value: pv, deck: sel }) };
        }
        let score = |env: &Env, sel: &SelectedDeck| {
            joint_b(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler_cards, ntar, false)[0].all
        };
        let product_deck = Some(ValuedDeck { value: pv, deck: sel.clone() });
        let mut best = (score(self, &sel), sel);
        *scored += 1;
        self.work += 200;
        if pv > 0.0 {
            let k = self.space.sel_class(&best.1);
            self.note_ratio(k, best.0 / pv, t, &best.1);
        }
        for s in starts {
            let x = score(self, s);
            *scored += 1;
            self.work += 200;
            if x > best.0 {
                best = (x, (*s).clone());
            }
        }
        let mut full = false;
        // the cap ladder below the top cap (a band over EXACT_CAP decks is retried at the next one)
        let mut ladder = EXACT_STEPS[1..].iter().copied().chain(std::iter::once(1.0)).filter(|&x| x < rho);
        let mut rho = rho;
        while self.work <= self.budget {
            let n0 = self.space.nodes.get();
            crate::deck::SB_ON.with(|c| c.set(true));
            crate::deck::SB_USED.with(|c| c.set(0));
            let SearchOutcome { decks: mut more, incomplete } = self.space.search(
                &self.model,
                &[t],
                &[uid],
                &[SearchClause { lineages: vec![LineageChance { trainee: 0, inheritance: 0.0 }], threshold: best.0 }],
                SearchMode::Band { ratio_cap: rho, max_decks: self.space.band_cap() },
            );
            crate::deck::SB_ON.with(|c| c.set(false));
            // the band built without the cap (SBOUND=2): every deck in it may beat the bar, none is cut by rho
            let nocap = crate::deck::SB_USED.with(std::cell::Cell::get) == 2;
            self.work += 2 * (self.space.nodes.get() - n0) + 5 * more.len() + 10_000;
            if incomplete.is_some() {
                self.overflows += 1;
                match ladder.next() {
                    Some(x) if !nocap => {
                        rho = x;
                        continue;
                    }
                    _ => {
                        self.cert = 0.0;
                        break;
                    }
                }
            }
            full = rho == self.space.rho_top();
            let mut cut = false;
            more.sort_by(|x, y| y.value.partial_cmp(&x.value).unwrap());
            for ScoredDeck { value: v, deck: s } in more {
                if !nocap && rho * v * (1.0 + 1e-9) < best.0 {
                    break;
                }
                if self.work > self.budget {
                    full = false;
                    cut = true;
                    break;
                }
                if s.entries == best.1.entries
                    && s.plain_pals == best.1.plain_pals
                    && s.filler_cards == best.1.filler_cards
                {
                    continue;
                }
                // below the bar at its own class's cap (the bar rose since the band was built)
                let k = self.space.sel_class(&s);
                if !nocap && self.space.caps()[k].min(rho) * v * (1.0 + 1e-9) < best.0 {
                    continue;
                }
                let e = self.ents_of(&s);
                if let Some(ub) = upper_na(&self.model, t, &[0.0], &e, s.filler_cards) {
                    self.work += 25;
                    if ub[0] * (1.0 + 1e-9) < best.0 {
                        continue;
                    }
                }
                let x = score(self, &s);
                *scored += 1;
                self.work += 200;
                if v > 0.0 {
                    self.note_ratio(k, x / v, t, &s);
                }
                if x > best.0 {
                    best = (x, s);
                }
            }
            if !cut {
                self.cert = self.cert.min(rho);
            }
            break;
        }
        CappedSearch { result: DeckRes::Done(best.0, best.1, full), product_deck }
    }
}
