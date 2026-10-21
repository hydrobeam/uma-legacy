//! Exact career deck searches and their kept memos.
use super::{DeckRes, Env};
use crate::deck::enumerate::{SearchMode, SearchOutcome};
use crate::deck::{EXACT_BUDGET, EXACT_STEPS, Kind, Sel, Src, Value, joint_b, json, upper_na};

/// What a `best_deck` call without start decks reads: the trainee's deck class (everything of her the deck search reads
/// but the folded chances), her folded event-hint chances per outfit (`set_trainee`: the model state the search runs on,
/// to the bit), the spark scale c, the bar th (every th <= 0 as one: each gives no seed and passes the ceiling test),
/// and the work spent before the call (the work budget's tests read it)
#[derive(PartialEq, Eq, Hash)]
pub(in crate::deck::career) struct DeckKey {
    cls: usize,
    ews: Vec<u64>,
    c: u64,
    th: u64,
    work: usize,
}

/// The cap-free searches' counters (`Env::cfs`)
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

/// Work budget of one cap-free search (`best_deck_cf`), in the units of `EXACT_BUDGET` (about 0.1 us each natively):
/// twice the loop planner's, about 6 s; `CF_BUDGET` in the environment overrides it. Past it the search proves
/// nothing (its call is not full). Measured on tiny Grand Masters slices (8-10 targets, caps forced down so every
/// search reruns): 0.4-4.4M cap-free nodes per search
fn cf_budget() -> usize {
    static C: std::sync::OnceLock<usize> = std::sync::OnceLock::new();
    *C.get_or_init(|| std::env::var("CF_BUDGET").ok().and_then(|s| s.parse().ok()).unwrap_or(2 * EXACT_BUDGET))
}

/// Work units per cap-free node: its bounds sum over event counts and chain states (about 1.5 us per node natively,
/// against 0.2 us for a capped node's 2 units)
const CF_NODE_WORK: usize = 15;

/// A memoised `best_deck` outcome: the result, its product search's proof (`last_pv`), the work and exact scorings it
/// added (and its cap-free searches', `cf`)
pub(in crate::deck::career) struct DeckMemo {
    res: DeckRes,
    pv: Option<f64>,
    work: usize,
    scored: usize,
    cert: f64,
    overflows: usize,
    cf: CfStats,
    /// the deck-search nodes and exact scorings it spent; per class, the call's own largest exact / product ratio and the
    /// deck that first reached it (`note_ratio` from nothing); the last job that used it (`Env::job`)
    nodes: usize,
    joints: usize,
    ratio: [Option<(f64, usize, Sel)>; 4],
    job: u64,
}

thread_local! {
    /// `best_deck`'s memo kept between jobs (the page's worker runs a plan, then a Reroll's plans with her added: the
    /// builders whose best pair doesn't involve her search the same deck problems again), per setup: `setup_fp` (what
    /// sets the model, the deck space and the class caps, `cap_audit` included) with the job's trainees (the plain pals'
    /// order), the last few. Exact: a `DeckKey` reads the same model, space and caps in every job of one setup, and the
    /// work budget's argument holds for any budget (`best_deck`)
    pub(in crate::deck::career) static DECK_KEEP: std::cell::RefCell<Vec<(u64, Kept)>> = const { std::cell::RefCell::new(Vec::new()) };
    /// jobs run so far (`Env::job`)
    pub(in crate::deck::career) static JOB_NO: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// what a setup keeps between jobs: `Env::deck_memo` and `Env::corner_memo`
#[derive(Default)]
pub(in crate::deck::career) struct Kept {
    pub(in crate::deck::career) decks: crate::FxMap<DeckKey, DeckMemo>,
    pub(in crate::deck::career) corners: crate::FxMap<(usize, Vec<usize>, usize), std::rc::Rc<Vec<Vec<f64>>>>,
}

/// setups whose memo is kept, and the entries past which a setup's memo starts afresh (at a job's start)
const DECK_KEEP_SETUPS: usize = 4;

pub(in crate::deck::career) const DECK_KEEP_CAP: usize = 8192;

impl Drop for Env<'_> {
    fn drop(&mut self) {
        let kept = Kept { decks: std::mem::take(&mut self.deck_memo), corners: std::mem::take(&mut self.corner_memo) };
        let fingerprint = self.memo_fp;
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
    fn note_ratio(&mut self, k: usize, r: f64, t: usize, sel: &Sel) {
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
        json!(self.max_deck.iter().map(|x| x.as_ref().map(|(t, s)| json!({"trainee": self.d.umas[*t].name, "cards": self.space.pdeck(s, &[self.d.umas[*t].id]).names}))).collect::<Vec<_>>())
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
    pub(in crate::deck::career) fn best_deck(
        &mut self,
        t: usize,
        c: f64,
        th: f64,
        starts: &[&Sel],
        scored: &mut usize,
    ) -> DeckRes {
        if !starts.is_empty() {
            return self.best_deck_run(t, c, th, starts, scored);
        }
        let key = DeckKey {
            cls: self.cls[t],
            ews: self.model.ews[t].iter().flatten().map(|x| x.to_bits()).collect(),
            c: c.to_bits(),
            th: if th <= 0.0 { 0 } else { th.to_bits() },
            work: self.work,
        };
        if let Some(m) = self.deck_memo.get_mut(&key)
            && key.work + m.work <= self.budget
        {
            self.work += m.work;
            *scored += m.scored;
            self.last_pv = m.pv;
            self.cert = self.cert.min(m.cert);
            self.overflows += m.overflows;
            self.cfs.add(&m.cf);
            // first use in this job of an outcome an earlier job stored (`DECK_KEEP`): this job's own search would have
            // run here, so its node and exact-scoring counters and its per-class ratios count as if it had (a later use
            // in the same job leaves them, as the memo always has). A class's ratio and deck change only when the call's
            // own maximum is above the job's: `note_ratio` keeps the first deck strictly above, so the same as the run
            if m.job != self.job {
                m.job = self.job;
                self.space.nodes.set(self.space.nodes.get() + m.nodes);
                crate::deck::JCOUNT.with(|c| c.set(c.get() + m.joints));
                for k in 0..4 {
                    if let Some((r, t2, sel)) = &m.ratio[k]
                        && *r > self.max_ratio[k]
                    {
                        self.max_ratio[k] = *r;
                        self.max_deck[k] = Some((*t2, sel.clone()));
                    }
                }
            }
            let caps = self.space.caps();
            for k in 0..4 {
                if let Some((r, _, _)) = &m.ratio[k]
                    && *r > caps[k] * (1.0 + 1e-9)
                {
                    self.cap_breach += 1;
                }
            }
            return m.res.clone();
        }
        let (w0, s0, c0, o0, n0, j0, f0) = (
            self.work,
            *scored,
            std::mem::replace(&mut self.cert, f64::INFINITY),
            self.overflows,
            self.space.nodes.get(),
            crate::deck::JCOUNT.with(std::cell::Cell::get),
            self.cfs,
        );
        // (the call's own per-class maxima, from nothing, then merged into the job's: see the hit above)
        let (r0, d0) =
            (std::mem::replace(&mut self.max_ratio, [f64::NEG_INFINITY; 4]), std::mem::take(&mut self.max_deck));
        let res = self.best_deck_run(t, c, th, starts, scored);
        let cert = self.cert;
        self.cert = c0.min(cert);
        let (r1, d1) = (std::mem::replace(&mut self.max_ratio, r0), std::mem::replace(&mut self.max_deck, d0));
        let ratio: [Option<(f64, usize, Sel)>; 4] =
            std::array::from_fn(|k| d1[k].as_ref().map(|(t2, sel)| (r1[k], *t2, sel.clone())));
        for k in 0..4 {
            if let Some((r, t2, sel)) = &ratio[k]
                && *r > self.max_ratio[k]
            {
                self.max_ratio[k] = *r;
                self.max_deck[k] = Some((*t2, sel.clone()));
            }
        }
        if self.work <= self.budget {
            self.deck_memo.insert(
                key,
                DeckMemo {
                    res: res.clone(),
                    pv: self.last_pv,
                    work: self.work - w0,
                    scored: *scored - s0,
                    cert,
                    overflows: self.overflows - o0,
                    cf: self.cfs.since(&f0),
                    nodes: self.space.nodes.get() - n0,
                    joints: crate::deck::JCOUNT.with(std::cell::Cell::get) - j0,
                    ratio,
                    job: self.job,
                },
            );
        }
        res
    }
    /// `best_deck`'s search. The capped search (`best_deck_capped`) first; when a deck it scored has an exact / product
    /// ratio past its class's cap (`cap_breach`), the cap it pruned by is wrong for this deck space, so a deck it pruned
    /// may beat its answer: the call is answered by the cap-free search instead (`best_deck_cf`; its own budget, the
    /// capped call's band overflows and certificate undone). Without a breach the capped answer stands, unchanged. Job
    /// field `cap_free`: the cap-free search alone. `CAP_RERUN=0` (checks): no rerun, the breach leaves the search not
    /// `complete` (as before the rerun existed).
    fn best_deck_run(&mut self, t: usize, c: f64, th: f64, starts: &[&Sel], scored: &mut usize) -> DeckRes {
        if self.space.dj.cap_free {
            return self.best_deck_cf(t, c, th, starts, scored, None, None);
        }
        let (breaches_before, cert_before, overflows_before) = (self.cap_breach, self.cert, self.overflows);
        let (res, prod) = self.best_deck_capped(t, c, th, starts, scored);
        if crate::deck::cf_check() > 0.0 {
            self.cf_beside(t, c, th, starts, &res, &prod);
        }
        if self.cap_breach == breaches_before {
            return res;
        }
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        if *OFF.get_or_init(|| std::env::var("CAP_RERUN").is_ok_and(|v| v == "0")) {
            self.cfs.unhandled += 1;
            return res;
        }
        // The cap-free answer supersedes this call's certificate and overflow count.
        self.cert = cert_before;
        self.overflows = overflows_before;
        self.cfs.reruns += 1;
        let capped = match &res {
            DeckRes::Done(v, s, _) => Some((*v, s.clone())),
            _ => None,
        };
        self.best_deck_cf(t, c, th, starts, scored, prod, capped)
    }
    /// CF_CHECK: the cap-free search beside a capped one, everything it changes put back, and their answers compared
    /// (`CFCHK`)
    fn cf_beside(&mut self, t: usize, c: f64, th: f64, starts: &[&Sel], res: &DeckRes, prod: &Option<(f64, Sel)>) {
        let keep = (
            self.work,
            self.last_pv,
            self.max_ratio,
            self.max_deck.clone(),
            self.cap_breach,
            self.cfs,
            self.cert,
            self.overflows,
            self.space.nodes.get(),
            crate::deck::JCOUNT.with(std::cell::Cell::get),
        );
        let mut sc = 0usize;
        let r = self.best_deck_cf(t, c, th, starts, &mut sc, prod.clone(), None);
        (
            self.work,
            self.last_pv,
            self.max_ratio,
            self.max_deck,
            self.cap_breach,
            self.cfs,
            self.cert,
            self.overflows,
        ) = (keep.0, keep.1, keep.2, keep.3, keep.4, keep.5, keep.6, keep.7);
        self.space.nodes.set(keep.8);
        crate::deck::JCOUNT.with(|c| c.set(keep.9));
        // a deck below the career's bar counts as nothing (the capped search may stop as `Bounded` there)
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
    /// The best product deck (`prod`, the capped search's, or searched here unseeded: a seed would need a cap to relate
    /// product and exact values) and the starts are scored; then the band of every deck whose cap-free bounds (`CF_ON`:
    /// `cf_cheap` and `cf_ub` in `DeckSpace::search`, on `state_value`'s argument and `group_nuod_lvl`) reach the bar,
    /// the incumbent or the career's bar th / c if higher, is built and scored best product value first, through
    /// `upper_na`. Proven (`full`) when the band finished (no node limit, no band overflow, every legal entry covered:
    /// `Src::cf_ok`) within CF_BUDGET work; then a best below th / c is `Bounded` (every deck is), else `Done`. Unproven:
    /// the better of its best and the capped answer (`capped`), not full, counted in `cfs.open`. Its work is counted
    /// apart (`cfs.work`), so the career's budget sees what it saw without the rerun; nodes and exact scorings are
    /// counted as usual.
    fn best_deck_cf(
        &mut self,
        t: usize,
        c: f64,
        th: f64,
        starts: &[&Sel],
        scored: &mut usize,
        prod: Option<(f64, Sel)>,
        capped: Option<(f64, Sel)>,
    ) -> DeckRes {
        let ntar = self.ntar;
        let uid = self.d.umas[t].id;
        let budget = cf_budget();
        let mut work = 0usize;
        let (pv, sel) = if let Some(x) = prod {
            x
        } else {
            let n0 = self.space.nodes.get();
            let SearchOutcome { decks: r, incomplete } = self.space.search(
                &self.model,
                &[t],
                &[uid],
                &[(vec![(0, 0.0)], 0.0)],
                SearchMode::Best { seed: f64::NEG_INFINITY },
            );
            work += 2 * (self.space.nodes.get() - n0) + 10_000;
            // (a `cap_free` job records no product proof: `solve` and the plan bound with those through EXACT_RHO)
            self.last_pv = if incomplete.is_some() || self.space.dj.cap_free { None } else { r.first().map(|x| x.0) };
            if let Some(x) = r.into_iter().next() {
                x
            } else {
                self.cfs.work += work;
                return DeckRes::NoDeck;
            }
        };
        let score =
            |env: &Env, sel: &Sel| joint_b(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler, ntar, false)[0].0;
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
            &[(vec![(0, 0.0)], best.0.max(the))],
            SearchMode::Band { ratio_cap: 1.0, max_decks: self.space.band_cap() },
        );
        crate::deck::CF_ON.with(|x| x.set(false));
        crate::deck::CF_LIM.with(|x| x.set(usize::MAX));
        work += CF_NODE_WORK * (self.space.nodes.get() - n0) + 5 * more.len() + 10_000;
        let mut full = incomplete.is_none();
        more.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
        for (v, s) in more {
            if work > budget {
                full = false;
                break;
            }
            if s.ents == best.1.ents && s.plain == best.1.plain && s.filler == best.1.filler {
                continue;
            }
            let k = self.space.sel_class(&s);
            let e: Vec<(&Src, Kind)> = s.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect();
            if let Some(ub) = upper_na(&self.model, t, &[0.0], &e, s.filler) {
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
        self.cfs.work += work;
        if !full {
            self.cfs.open += 1;
            if let Some((v, s)) = capped
                && v > best.0
            {
                best = (v, s);
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
    fn best_deck_capped(
        &mut self,
        t: usize,
        c: f64,
        th: f64,
        starts: &[&Sel],
        scored: &mut usize,
    ) -> (DeckRes, Option<(f64, Sel)>) {
        let ntar = self.ntar;
        let uid = self.d.umas[t].id;
        let n0 = self.space.nodes.get();
        // the product search starts at the bar the career must clear (a hair below the cut below): it prunes every
        // branch that cannot reach it and finds nothing when no deck can, and a deck that clears it is the one an
        // unseeded search finds (the seed is below the maximum, so the first deck reaching it is never pruned)
        let rho = self.space.rho_top();
        let seed =
            if th.is_finite() && th > 0.0 { th / (rho * c * (1.0 + 1e-9)) * (1.0 - 1e-12) } else { f64::NEG_INFINITY };
        let SearchOutcome { decks: r, incomplete } =
            self.space.search(&self.model, &[t], &[uid], &[(vec![(0, 0.0)], 0.0)], SearchMode::Best { seed });
        self.work += 2 * (self.space.nodes.get() - n0) + 10_000;
        // what it proves (`ruled`): the deck found is the best (the first deck above the seed the search keeps is only
        // ever replaced by a better one, and a branch is pruned only below the best so far); none found, every deck is
        // at most the seed. Nothing when enumeration was incomplete (more than 128 card characters).
        self.last_pv = if incomplete.is_some() {
            None
        } else if let Some(x) = r.first() {
            Some(x.0)
        } else if seed > f64::NEG_INFINITY {
            Some(seed)
        } else {
            None
        };
        let Some((pv, sel)) = r.into_iter().next() else {
            return (if seed > f64::NEG_INFINITY { DeckRes::Bounded } else { DeckRes::NoDeck }, None);
        };
        // the candidate's own ceiling from its best product deck
        if rho * c * pv * (1.0 + 1e-9) < th {
            return (DeckRes::Bounded, Some((pv, sel)));
        }
        let score =
            |env: &Env, sel: &Sel| joint_b(&env.model, t, &[0.0], &env.ents_of(sel), sel.filler, ntar, false)[0].0;
        let prod = Some((pv, sel.clone()));
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
                &[(vec![(0, 0.0)], best.0)],
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
            more.sort_by(|x, y| y.0.partial_cmp(&x.0).unwrap());
            for (v, s) in more {
                if !nocap && rho * v * (1.0 + 1e-9) < best.0 {
                    break;
                }
                if self.work > self.budget {
                    full = false;
                    cut = true;
                    break;
                }
                if s.ents == best.1.ents && s.plain == best.1.plain && s.filler == best.1.filler {
                    continue;
                }
                // below the bar at its own class's cap (the bar rose since the band was built)
                let k = self.space.sel_class(&s);
                if !nocap && self.space.caps()[k].min(rho) * v * (1.0 + 1e-9) < best.0 {
                    continue;
                }
                let e: Vec<(&Src, Kind)> = s.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect();
                if let Some(ub) = upper_na(&self.model, t, &[0.0], &e, s.filler) {
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
        (DeckRes::Done(best.0, best.1, full), prod)
    }
}
