//! Career candidates, bounds and the best careers.
use super::LAST_SOLVE;
use super::{
    Env, Pool, SOLVE_KEEP, aff_factor, dominates, holds_fix, inherit_q, insp_miss, mix_term, now_ms, part_sum, prof,
    shared, spark_scale,
};
use crate::deck::SelectedDeck;
use crate::deck::career::Lineage;
use crate::{UmaIndex, UmaTable};

/// How a bound proves a career out (`Env::ceiling_out`, `refs_out`, `ruled_out`): `Below(th)`, `solve`'s prune, the bound
/// below the bar with its slack s, b (1 + s) < th (a computed bound a hair under the true one still proves it); or
/// `AtMost(v)`, `certified`'s, the bound at most v up to rounding, b <= v (1 + 1e-12): the career can't beat v, and a
/// tie proves value equality, all a certificate claims. The 1e-12 is rounding only: v is a proxy's value through its
/// deck's corner table and the bound the same deck's exact value, the same number computed two ways (boot_mob: 48
/// proxies one ulp below their class's proof). Before 2026-10-21 the allowance was 1e-9, looser than any rounding
#[derive(Clone, Copy)]
pub(crate) enum Bar {
    Below(f64),
    AtMost(f64),
}

impl Bar {
    #[inline(always)]
    fn out(self, b: f64, s: f64) -> bool {
        match self {
            Self::Below(th) => b * (1.0 + s) < th,
            Self::AtMost(v) => b <= v * (1.0 + 1e-12),
        }
    }
}

/// `Env::ranked_pairs`' pairs, sorted lazily as `solve`'s candidates are: `gen_cands` emits one trainee's pairs in
/// (a, b) order, so the stable sort by ceiling (descending, `partial_cmp`: 0 and -0 tie) is the total order (ceiling
/// descending, a, b), unique per pair; each step past the ordered prefix `v[..upto]` selects the next chunk (everything
/// before it orders before everything after it) and sorts only the chunk, so the sequence is the full sort's, element
/// for element, and a caller that stops early sorts little
pub(crate) struct Ranked {
    v: Vec<(f64, usize, usize)>,
    upto: usize,
    i: usize,
}

impl Iterator for Ranked {
    type Item = (f64, usize, usize);
    fn next(&mut self) -> Option<Self::Item> {
        if self.i >= self.v.len() {
            return None;
        }
        if self.i >= self.upto {
            let ord = |x: &(f64, usize, usize), y: &(f64, usize, usize)| {
                y.0.partial_cmp(&x.0).unwrap().then(x.1.cmp(&y.1)).then(x.2.cmp(&y.2))
            };
            let rest = &mut self.v[self.upto..];
            // (chunks of 256, then 4x what is sorted: each selection scans the whole remainder)
            let m = (4 * self.upto).max(256).min(rest.len());
            if m < rest.len() {
                rest.select_nth_unstable_by(m - 1, ord);
            }
            rest[..m].sort_unstable_by(ord);
            self.upto += m;
        }
        let r = self.v[self.i];
        self.i += 1;
        Some(r)
    }
}

/// A candidate the search solved: trainee, parents (legacy indices), value C x E, its ceiling, its best deck.
#[derive(Clone)]
pub(crate) struct Found {
    /// The trainee (uma index).
    pub(crate) trainee: UmaIndex,
    /// The first parent (legacy index).
    pub(crate) first_parent: usize,
    /// The second parent (legacy index).
    pub(crate) second_parent: usize,
    /// The career's value C x E: the spark scale times her best deck's exact value.
    pub(crate) value: f64,
    /// The ceiling the search bounded the career with before solving it.
    pub(crate) ceiling: f64,
    /// Her best deck.
    pub(crate) deck: SelectedDeck,
}

impl Found {
    /// Her trainee and parents, in the order recorded.
    pub(crate) const fn lineage(&self) -> Lineage {
        Lineage { trainee: self.trainee, first_parent: self.first_parent, second_parent: self.second_parent }
    }
}

/// A trainee's fully checked deck: the chances it was found at (job-target order), its exact value there before the
/// spark scale (the best of every legal deck, under the EXACT_RHO certificate, or cap-free after a rerun), the deck. Its
/// value bounds every deck at other chances through `rise`.
#[derive(Clone)]
pub(crate) struct Ref {
    pub(crate) q: Vec<f64>,
    pub(crate) val: f64,
    pub(crate) deck: SelectedDeck,
}

#[derive(Clone)]
pub(crate) enum DeckRes {
    NoDeck,
    Bounded,
    Done(f64, SelectedDeck, bool),
}

#[derive(Clone)]
/// (`cert`: the lowest cap a deck search of it was certified under, INFINITY when none was lowered; see `Env::cert`)
pub(crate) struct Solved {
    pub(crate) found: Vec<Found>,
    pub(crate) refs: UmaTable<Vec<Ref>>,
    pub(crate) work: usize,
    pub(crate) cands: usize,
    pub(crate) solved: usize,
    pub(crate) bounded: usize,
    pub(crate) scored: usize,
    pub(crate) skipped: usize,
    pub(crate) next_bound: f64,
    pub(crate) complete: bool,
    pub(crate) proven: bool,
    pub(crate) cert: f64,
}

impl Env<'_> {
    /// The bound a fully checked deck search `r` (a `Ref`: chances r.q, best value r.val before the spark scale) puts on
    /// every career of trainee t's deck class at chances q with spark scale c: c r.val rise(t, q, r.q) (`Env::rise`)
    #[inline]
    pub(crate) fn ref_bound(&self, t: UmaIndex, q: &[f64], c: f64, r: &Ref) -> f64 {
        c * r.val * self.rise(t, q, &r.q)
    }
    /// The bound a product search's proof (`ruled`: chances rq, best product value s) puts on it there: EXACT_RHO c s
    /// rise(t, q, rq) (`solve`'s derivation at its `ruled` test)
    #[inline]
    fn ruled_bound(&self, t: UmaIndex, q: &[f64], c: f64, rq: &[f64], s: f64) -> f64 {
        self.space.rho_top() * c * s * self.rise(t, q, rq)
    }
    /// Whether a career's ceiling `ub` (`gen_cands`) proves it out at `bar` (`Bar`; `solve`: ub (1 + 1e-9) < th)
    #[inline]
    pub(crate) fn ceiling_out(ub: f64, bar: Bar) -> bool {
        bar.out(ub, 1e-9)
    }
    /// Whether a fully checked deck of `refs` proves career (t, q, c) out at `bar` (`solve`: some `ref_bound` (1 + 1e-9) < th)
    #[inline]
    pub(crate) fn refs_out<'refs>(
        &self,
        t: UmaIndex,
        q: &[f64],
        c: f64,
        mut refs: impl Iterator<Item = &'refs Ref>,
        bar: Bar,
    ) -> bool {
        refs.any(|r| bar.out(self.ref_bound(t, q, c, r), 1e-9))
    }
    /// Whether a product proof of `ruled` does (`solve`: some `ruled_bound` (1 + 2e-9) < th; 2e-9: the rounding of the
    /// product value and of rise)
    #[inline]
    pub(crate) fn ruled_out(&self, t: UmaIndex, q: &[f64], c: f64, ruled: &[(Vec<f64>, f64)], bar: Bar) -> bool {
        ruled.iter().any(|(rq, s)| bar.out(self.ruled_bound(t, q, c, rq, *s), 2e-9))
    }
    /// Trainee t's legal pairs of `p` holding `fix` and kept by `keep`, with their ceilings (`gen_cands`: `ceiling` at
    /// `lineage`'s chances, to the bit), best first, ties in pair order (the stable sort by ceiling every ranked scan
    /// took). The model state after it: `ceiling` at the last pair's chances, as a loop of `ceiling` over the pairs left
    /// it (every reader of the model state sets it first)
    pub(crate) fn ranked_pairs(
        &mut self,
        p: &Pool,
        fix: &[usize],
        t: UmaIndex,
        keep: impl Fn(usize, usize) -> bool,
    ) -> Ranked {
        Ranked { v: self.gen_cands(p, fix, &[t], keep, |ub, _, _, a, b| (ub, a, b)), upto: 0, i: 0 }
    }
    /// The ceiling of trainee t at chances q (her inheritance folded in here) with spark scale c: EXACT_RHO x C x the
    /// product value at each target's best possible deck odds (None: no legal deck odds for her).
    pub(crate) fn ceiling(&mut self, t: UmaIndex, q: &[f64], c: f64) -> Option<f64> {
        self.deck_odds_ceilings[t].as_ref()?;
        let (rho, fixed) = (self.space.rho_top(), self.model.fixed_factor());
        Some(self.ceiling_at(t, q, c, rho, fixed))
    }
    /// `ceiling` for a trainee with legal deck odds, EXACT_RHO and the fixed gold factor given (the same values)
    #[inline]
    fn ceiling_at(&mut self, t: UmaIndex, q: &[f64], c: f64, rho: f64, fixed: f64) -> f64 {
        self.set_trainee(t, q);
        let (dw, dg) = self.deck_odds_ceilings[t].as_ref().unwrap();
        let pv = (0..self.model.n_outfits(t)).map(|o| self.model.outfit_fast(t, o, 0.0, dw, dg)).fold(0.0f64, f64::max);
        rho * c * fixed * pv
    }
    /// `solve`'s candidates: per trainee of `trainees` with legal deck odds (in order; `tp` her position), per legal pair
    /// a < b of `p` (all of `fix` among them, at most `p.max_borrowed` borrowed, neither her character), `mk(ceiling, tp, t, a, b)`,
    /// in that order. The numbers are `lineage` then `ceiling`'s, operation for operation, without the allocations and
    /// with what does not depend on the trainee computed once per pair (the parents' race parts and relation points
    /// between them, the carrier counts and so the spark scale c) or once per trainee and legacy (her relation points
    /// with it, its grandparent slots' miss factors). A slot without a spark of target k multiplies by 1 where
    /// `inherit` skips it (exact), and every model state `ceiling` leaves (`set_trainee` at each candidate's q in
    /// turn) is the same.
    /// `keep`: a filter on the pairs (a, b) beside the rules (`solve`: every pair).
    pub(crate) fn gen_cands<C>(
        &mut self,
        p: &Pool,
        fix: &[usize],
        trainees: &[UmaIndex],
        keep: impl Fn(usize, usize) -> bool,
        mut mk: impl FnMut(f64, usize, UmaIndex, usize, usize) -> C,
    ) -> Vec<C> {
        let d = self.data;
        let nl = p.veterans.len();
        let ntar = self.target_count;
        // a slot's (miss, mix) factors of target k: `inherit`'s, 1 for the one it leaves alone (its affinity factor fa
        // computed once per slot)
        #[inline(always)]
        fn fac(st: u8, mix: Option<[f64; 3]>, fa: f64) -> (f64, f64) {
            if st == 0 {
                return (1.0, 1.0);
            }
            match mix {
                None => (insp_miss(f64::from(st), fa), 1.0),
                Some(w) => (1.0, mix_term(w, fa)),
            }
        }
        // `spark_scale` per carrier count n
        let maxs = (0..nl).map(|x| 1 + p.parents[x].len()).max().unwrap_or(0) * 2;
        let pw: Vec<f64> = (0..=maxs).map(|n| spark_scale(n as i32)).collect();
        struct Pre {
            a: usize,
            b: usize,
            c: f64,
            race: [f64; 2],
            pp: [f64; 2],
        }
        let mut pairs: Vec<Pre> = vec![];
        // per legacy and target, how many of her slots (her own, her parents') carry the spark
        let cnt: Vec<usize> = (0..nl)
            .flat_map(|x| {
                (0..ntar).map(move |k| {
                    std::iter::once(&p.veterans[x]).chain(&p.parents[x]).filter(|w| w.stars[k] != 0).count()
                })
            })
            .collect();
        let mut nc = vec![0usize; ntar];
        // the pairs a < b in (a, b) order: with a fixed legacy f, only those holding it, (a, f) for a < f then (f, b)
        let pair_list: Box<dyn Iterator<Item = (usize, usize)>> = match fix.first() {
            None => Box::new((0..nl).flat_map(move |a| (a + 1..nl).map(move |b| (a, b)))),
            Some(&f) if f < nl => Box::new((0..f).map(move |a| (a, f)).chain((f + 1..nl).map(move |b| (f, b)))),
            Some(_) => Box::new(std::iter::empty()),
        };
        for (a, b) in pair_list {
            if !p.parents_ok(a, b) || !holds_fix(fix, a, b) || !keep(a, b) {
                continue;
            }
            let sh = 3.0 * shared(&p.veterans[a], &p.veterans[b]);
            let race = [a, b].map(|x| part_sum(sh, p.grandparent_race_affinity[x].iter().copied()));
            let pp = [
                self.pair(p.veterans[a].uma_index, p.veterans[b].uma_index),
                self.pair(p.veterans[b].uma_index, p.veterans[a].uma_index),
            ];
            for k in 0..ntar {
                nc[k] = cnt[a * ntar + k] + cnt[b * ntar + k];
            }
            let c = self
                .uncertain_targets
                .iter()
                .map(|&i| pw[nc[i]])
                .chain(self.fixed_targets.iter().map(|&i| pw[nc[i]]))
                .product::<f64>();
            pairs.push(Pre { a, b, c, race, pp });
        }
        let mut out: Vec<C> = Vec::with_capacity(
            pairs.len() * trainees.iter().filter(|&&t| self.deck_odds_ceilings[t].is_some()).count(),
        );
        let (rho, fixed) = (self.space.rho_top(), self.model.fixed_factor());
        let mut q = vec![0.0f64; ntar];
        // per legacy x: her relation points with the trainee, and per grandparent slot its relation points and its
        // factors per target (flat, from `go[x]`)
        let mut pt = vec![0.0f64; nl];
        let go: Vec<usize> = std::iter::once(0)
            .chain((0..nl).scan(0, |s, x| {
                *s += p.parents[x].len();
                Some(*s)
            }))
            .collect();
        let mut gr = vec![0.0f64; go[nl]];
        let mut gf = vec![(1.0f64, 1.0f64); go[nl] * ntar];
        // per outfit and uncertain target, `outfit_fast`'s factor as Fixed (no chance in it) or Open(1 - e, 1 - dw, dg GOLD,
        // 1 - dg): the same subexpressions, each computed once for her
        enum Fac {
            Fixed(f64),
            Open(f64, f64, f64, f64),
        }
        // what one trainee's pair loop reads
        struct Cx<'cx> {
            p: &'cx Pool,
            pairs: &'cx [Pre],
            tid: u32,
            ntar: usize,
            pt: &'cx [f64],
            go: &'cx [usize],
            gr: &'cx [f64],
            gf: &'cx [(f64, f64)],
            ft: &'cx [Vec<Fac>],
            uncv: &'cx [usize],
            gw: f64,
            rho: f64,
            fixed: f64,
            tp: usize,
            t: UmaIndex,
        }
        // One trainee's candidates, pair by pair; true when it made one (q then holds the last one's chances). Its own
        // function, entered once per trainee: a wasm call keeps the code it started in (V8 has no on-stack replacement
        // for wasm), so a loop over every trainee in one frame would run in baseline code throughout (docs/PERF.md).
        #[inline(never)]
        fn trainee_cands<C>(
            cx: &Cx,
            q: &mut [f64],
            nq: &mut [f64],
            out: &mut Vec<C>,
            mk: &mut impl FnMut(f64, usize, UmaIndex, usize, usize) -> C,
        ) -> bool {
            let (p, go, gw, ntar) = (cx.p, cx.go, cx.gw, cx.ntar);
            let mut last = false;
            for pr in cx.pairs {
                let (a, b) = (pr.a, pr.b);
                if !p.trainee_ok(cx.tid, a, b) {
                    continue;
                }
                // each parent slot's affinity, summed as `slots` sums it (as fa)
                let fa = [0, 1].map(|s| {
                    let x = if s == 0 { a } else { b };
                    aff_factor(part_sum(cx.pt[x] + pr.pp[s], cx.gr[go[x]..go[x + 1]].iter().copied()), pr.race[s])
                });
                for k in 0..ntar {
                    let (mut miss, mut mix2) = (1.0f64, 1.0f64);
                    for (s, x) in [a, b].into_iter().enumerate() {
                        let (fm, fx) = fac(p.veterans[x].stars[k], p.veterans[x].star_mix, fa[s]);
                        miss *= fm;
                        mix2 *= fx;
                        for j in go[x]..go[x + 1] {
                            let (fm, fx) = cx.gf[j * ntar + k];
                            miss *= fm;
                            mix2 *= fx;
                        }
                    }
                    q[k] = inherit_q(miss, mix2);
                }
                // `ceiling_at` without writing the model state: per outfit `outfit_fast` at q = 0 on the hint chances
                // `set_trainee` would write (each factor (1 - q) = 1 dropped: x 1 is exact), the best outfit as its fold
                // (an open factor: ew = 1 - (1 - e)(1 - q_k), white = GEN_WHITE (1 - (1 - dw)(1 - ew)), dg GOLD + (1 - dg) white)
                for k in 0..nq.len() {
                    nq[k] = 1.0 - q[cx.uncv[k]];
                }
                let mut pv = 0.0f64;
                for fo in cx.ft {
                    let mut po = 1.0f64;
                    for (k, f) in fo.iter().enumerate() {
                        po *= match *f {
                            Fac::Fixed(x) => x,
                            Fac::Open(ne, ndw, a, nd) => {
                                let ew = 1.0 - ne * nq[k];
                                a + nd * (gw * (1.0 - ndw * (1.0 - ew)))
                            }
                        };
                    }
                    pv = f64::max(pv, po);
                }
                last = true;
                out.push(mk(cx.rho * pr.c * cx.fixed * pv, cx.tp, cx.t, a, b));
            }
            last
        }
        for (tp, &t) in trainees.iter().enumerate() {
            if self.deck_odds_ceilings[t].is_none() {
                continue;
            }
            let (ti, tid) = (Some(t), d.umas[t].id);
            for x in 0..nl {
                let px = &p.veterans[x];
                pt[x] = self.pair(ti, px.uma_index);
                for (gi, g) in p.parents[x].iter().enumerate() {
                    let j = go[x] + gi;
                    gr[j] = self.tri(ti, px.uma_index, g.uma_index);
                    let fa = aff_factor(gr[j], p.grandparent_race_affinity[x][gi]);
                    for k in 0..ntar {
                        gf[j * ntar + k] = fac(g.stars[k], g.star_mix, fa);
                    }
                }
            }
            let (dw, dg) = self.deck_odds_ceilings[t].as_ref().unwrap();
            debug_assert!(
                self.outfit_coverage[t].len() == self.model.n_outfits(t),
                "one skill table per outfit of trainee {t}"
            );
            let (gg, gw) = (crate::skills::GEN_GOLD, crate::skills::GEN_WHITE);
            let ft: Vec<Vec<Fac>> = self.outfit_coverage[t]
                .iter()
                .zip(&self.own_event_chances[t])
                .map(|(st, e0)| {
                    (0..st.len())
                        .map(|k| match st[k] {
                            2 => Fac::Fixed(gg),
                            1 => Fac::Fixed(dg[k] * gg + (1.0 - dg[k]) * gw),
                            _ => Fac::Open(1.0 - e0[k], 1.0 - dw[k], dg[k] * gg, 1.0 - dg[k]),
                        })
                        .collect()
                })
                .collect();
            let mut nq = vec![0.0f64; ft.iter().map(std::vec::Vec::len).max().unwrap_or(0)];
            let cx = Cx {
                p,
                pairs: &pairs,
                tid,
                ntar,
                pt: &pt,
                go: &go,
                gr: &gr,
                gf: &gf,
                ft: &ft,
                uncv: &self.uncertain_targets,
                gw,
                rho,
                fixed,
                tp,
                t,
            };
            let last = trainee_cands(&cx, &mut q, &mut nq, &mut out, &mut mk);
            // the model state the last candidate's `ceiling` left (q still holds its chances)
            if last {
                self.set_trainee(t, &q);
            }
        }
        out
    }

    /// The best careers: trainees from `trainees` (uma indices), two legacies of `p` as parents (all of `fix` among
    /// them, at most one borrowed), each with her best deck; `top` of them, one per trainee when `per_trainee`.
    /// Candidates are kept as (ceiling, trainee, parents) only (a pool of 400 veterans has millions); their chances
    /// are recomputed when one is solved (the same numbers, so the same results). The search is `complete` unless a
    /// deck search was not `full` (the work budget ran out, or a band was certified below the top cap) or a deck scored
    /// during it passed its class's cap (`cap_breach`).
    pub(crate) fn solve(
        &mut self,
        p: &Pool,
        fix: &[usize],
        trainees: &[UmaIndex],
        per_trainee: bool,
        top: usize,
    ) -> Solved {
        let d = self.data;
        self.work = 0; // each search has its own work budget
        let cert0 = std::mem::replace(&mut self.cert, f64::INFINITY);
        // (`tp`: the trainee's position in `trainees`; generated in (tp, a, b) order)
        // (packed as u16, not a `Lineage`: a pool of 400 veterans has millions of candidates)
        struct Cand {
            ub: f64,
            t: u16,
            tp: u16,
            a: u16,
            b: u16,
        }
        impl Cand {
            const fn lineage(&self) -> Lineage {
                Lineage {
                    trainee: UmaIndex(self.t as usize),
                    first_parent: self.a as usize,
                    second_parent: self.b as usize,
                }
            }
        }
        let t_gen = now_ms();
        let nl = p.veterans.len();
        assert!(
            nl <= u16::MAX as usize && d.umas.len() <= u16::MAX as usize && trainees.len() <= u16::MAX as usize,
            "legacies, umas and trainees are packed as u16"
        );
        let mut cands: Vec<Cand> = self.gen_cands(
            p,
            fix,
            trainees,
            |_, _| true,
            |ub, tp, t, a, b| Cand { ub, t: t.0 as u16, tp: tp as u16, a: a as u16, b: b as u16 },
        );
        // Best ceiling first, ties in generation order: the order a stable sort by ceiling gives (partial_cmp, so 0 and
        // -0 tie as there), as a total order on (ceiling descending, tp, a, b), unique per candidate. Sorted lazily
        // (`upto`): the loop below reads a short prefix, so each step selects the next chunk (everything before it is
        // ordered before everything after it) and sorts only the chunk; the prefix is the full sort's, element for element.
        let ord = |x: &Cand, y: &Cand| {
            y.ub.partial_cmp(&x.ub).unwrap().then(x.tp.cmp(&y.tp)).then(x.a.cmp(&y.a)).then(x.b.cmp(&y.b))
        };
        let mut upto = 0usize;
        // cands[..upto] in final order; makes it reach past i
        let sort_to = |cands: &mut [Cand], i: usize, upto: &mut usize| {
            if i < *upto || *upto >= cands.len() {
                return;
            }
            let t0 = now_ms();
            while i >= *upto && *upto < cands.len() {
                let rest = &mut cands[*upto..];
                // (chunks of 1024, then 4x what is sorted: each selection scans the whole remainder)
                let m = (4 * *upto).max(1024).min(rest.len());
                if m < rest.len() {
                    rest.select_nth_unstable_by(m - 1, ord);
                }
                rest[..m].sort_unstable_by(ord);
                *upto += m;
            }
            prof("gen_ms", now_ms() - t0);
        };
        prof("gen_ms", now_ms() - t_gen);
        prof("gen_cands", cands.len() as f64);
        prof("solves", 1.0);
        let top = top.max(1);
        let mut complete = true;
        // a breach no cap-free search answered (`CAP_RERUN=0`): the search may have pruned a better deck. A rerun that
        // proves nothing returns a deck search that is not full, so `complete` falls below as for any such search
        let open0 = self.cap_free_stats.unhandled;
        let mut done: Vec<(f64, usize, SelectedDeck)> = vec![];
        let floor = self.floor;
        let kth =
            |done: &[(f64, usize, SelectedDeck)]| if done.len() < top { floor } else { done[top - 1].0.max(floor) };
        let (mut n_solved, mut n_bounded, mut n_scored) = (0usize, 0usize, 0usize);
        let mut next = 0;
        // one result per trainee: a candidate only matters if it beats the k-th best and its trainee's best so far
        let thr = |done: &[(f64, usize, SelectedDeck)], cands: &[Cand], t: UmaIndex| {
            if per_trainee {
                done.iter()
                    .find(|x| UmaIndex(cands[x.1].t as usize) == t)
                    .map_or_else(|| kth(done), |x| x.0.max(kth(done)))
            } else {
                kth(done)
            }
        };
        // per trainee, her fully checked candidates (`Ref`: chances, exact best value before the spark scale, deck)
        let mut refs = UmaTable(vec![vec![]; d.umas.len()]);
        // per trainee, where this search's own entries in `ruled` start
        let ruled0: UmaTable<usize> = self.ruled.iter().map(std::vec::Vec::len).collect();
        // dominated veterans (`dominance_lists`), and the candidates this search took and did not solve (bounded, no
        // deck)
        let dom = dominance_lists(p);
        let prune = dom.iter().any(|v| !v.is_empty());
        let mut unsolved: crate::FxSet<(u16, u16, u16)> = crate::FxSet::default();
        // the deck problems searched so far (`Twin`)
        let mut twins: crate::FxMap<TwinKey, Twin> = crate::FxMap::default();
        // a candidate whose ceiling is 0 cannot land every target (a target no legacy carries and nothing else gives):
        // nothing after it can either (sorted), and with every value 0 the bar would never rise, so every career would
        // be solved with unbounded deck searches (the page hung); stop there, leaving such careers out
        while next < cands.len()
            && {
                sort_to(&mut cands, next, &mut upto);
                cands[next].ub > 0.0
            }
            && cands[next].ub * (1.0 + 1e-9) >= kth(&done)
        {
            if self.hard_work_cap > 0 && self.work > self.hard_work_cap {
                complete = false;
                break;
            }
            let ci = next;
            next += 1;
            let lineage = cands[ci].lineage();
            let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
            let th = thr(&done, &cands, t);
            if prune {
                unsolved.insert((cands[ci].t, cands[ci].a, cands[ci].b));
            }
            if Self::ceiling_out(cands[ci].ub, Bar::Below(th)) {
                n_bounded += 1;
                continue;
            }
            // A candidate whose parent has a dominating veteran x, where (t, x, other parent) was taken earlier and not
            // solved, would end unsolved too: that one was ruled out by its ceiling, a Ref's or a product search's bound,
            // a search that found no deck clearing the bar, or (in turn) this test, at a bar no higher than now (the bar
            // only rises), and each of those bounds is at most as high for this candidate (`dominates`: q, C and every
            // deck's value no higher), so it fails the same test (no deck at all: none here either). So no Ref, no
            // result, the same counters bar `work`; exact for any `top` and either `per_trainee`, and it uses nothing but
            // this search's own candidates. A taken-and-solved dominator proves nothing of the kind (this one could still
            // come out solved, a Ref the later searches start from), so only unsolved ones count.
            if prune {
                let key = |x: usize, y: usize| (t.0 as u16, x.min(y) as u16, x.max(y) as u16);
                if [(a, b), (b, a)]
                    .iter()
                    .any(|&(x, y)| dom[x].iter().any(|&v| v != y && unsolved.contains(&key(v, y))))
                {
                    n_bounded += 1;
                    continue;
                }
            }
            let (q, c) = self.lineage_q(p, lineage);
            // (the ceiling this candidate was ranked and stopped by is `ceiling` at these chances, to the bit:
            // `gen_cands`' claim, checked with the model state put back)
            debug_assert!(
                {
                    let ew = self.model.ews[t].clone();
                    let ub = self.ceiling(t, &q, c);
                    self.model.ews[t] = ew;
                    ub.map(f64::to_bits) == Some(cands[ci].ub.to_bits())
                },
                "gen_cands' ceiling of ({t}, {a}, {b}) is not ceiling(lineage)"
            );
            let ct = self.deck_class[t];
            let seen: &[Ref] = if self.use_seen { &self.seen[ct] } else { &[] };
            // (the lowest of these bounds below th, as any one of them below it: the minimum is one of the bounds, and
            // rounding x (1 + 1e-9) is monotone; a NaN bound neither lowers the minimum nor passes the test)
            if self.refs_out(t, &q, c, refs[t].iter().chain(seen), Bar::Below(th)) {
                n_bounded += 1;
                continue;
            }
            // what her earlier product searches proved (`ruled`). Derived: with her chances folded in, target k's factor
            // in the product value of one outfit and one deck (odds dw, dg) is f(n) = A - B n, n = (1 - e)(1 - q_k),
            // A = dg G + (1 - dg) W, B = (1 - dg) W (1 - dw) <= A (`outfit_fast` at q = 0; a target the outfit has is
            // q-free). From q to q' with q'_k > q_k: n' < n, and A >= B gives f(n') / f(n) <= (1 - n') / (1 - n), the
            // factor `rise` takes ((A - B n')(1 - n) <= (A - B n)(1 - n') <=> A (n' - n) <= B (n' - n)); with q'_k <= q_k
            // the factor does not grow. So per outfit the product at q' is at most the product at q times that outfit's
            // ratio product, the best outfit at q' at most S x rise(t, q', q) for every deck, and the fixed gold factor
            // and the spark scale c' multiply both sides alike. Then the search below would end `Bounded` on either path:
            // no deck above its seed th / (EXACT_RHO c' (1 + 1e-9)), or its best deck pv' with EXACT_RHO c' pv' (1 + 1e-9)
            // < th. The slack 2e-9 covers the float rounding of pv' and rise (about 1e-15 each). A skipped candidate is
            // one that would have been `Bounded`: no Ref, no result, the same counters bar `work` (less of it, so a search
            // that ran out of its budget before can now get further; none of the checked jobs comes near it). A search whose
            // decks seed the simulator's cache (`use_seen` off) takes only the proofs of its own candidates, so its work does not
            // depend on what earlier searches did either.
            let lo = if self.use_seen { 0 } else { ruled0[ct] };
            if self.ruled_out(t, &q, c, &self.ruled[ct][lo..], Bar::Below(th)) {
                n_bounded += 1;
                continue;
            }
            self.set_trainee(t, &q);
            let res = self.twin_deck_search(
                &mut twins,
                TwinSearch { trainee: t, chances: &q, spark_scale: c, bar: th, starts: &refs[t] },
                &mut n_scored,
            );
            let (val, sel, full) = match res {
                DeckRes::NoDeck => continue,
                DeckRes::Bounded => {
                    n_bounded += 1;
                    continue;
                }
                DeckRes::Done(v, sel, full) => (v, sel, full),
            };
            n_solved += 1;
            if prune {
                unsolved.remove(&(cands[ci].t, cands[ci].a, cands[ci].b));
            }
            if full {
                refs[t].push(Ref { q, val, deck: sel.clone() });
            } else {
                complete = false;
            }
            let val = c * val;
            if per_trainee && let Some(i) = done.iter().position(|x| UmaIndex(cands[x.1].t as usize) == t) {
                if done[i].0 >= val {
                    continue;
                }
                done.remove(i);
            }
            let pos = done.iter().position(|x| x.0 < val).unwrap_or(done.len());
            done.insert(pos, (val, ci, sel));
        }
        self.merge_seen(&refs);
        let next_bound = if next < cands.len() {
            sort_to(&mut cands, next, &mut upto);
            cands[next].ub
        } else {
            0.0
        };
        if self.cap_free_stats.unhandled > open0 {
            complete = false;
        }
        let proven = complete
            && !done.is_empty()
            && (done.len() >= top.min(cands.len()) || next >= cands.len())
            && next_bound < done[0].0;
        let n = cands.len();
        let cert = self.cert;
        self.cert = cert0.min(cert);
        Solved {
            found: done
                .into_iter()
                .take(top)
                .map(|(val, ci, sel)| Found {
                    trainee: UmaIndex(cands[ci].t as usize),
                    first_parent: cands[ci].a as usize,
                    second_parent: cands[ci].b as usize,
                    value: val,
                    ceiling: cands[ci].ub,
                    deck: sel,
                })
                .collect(),
            refs,
            cands: n,
            solved: n_solved,
            bounded: n_bounded,
            scored: n_scored,
            skipped: n - next,
            next_bound,
            complete,
            proven,
            work: self.work,
            cert,
        }
    }
}

impl Env<'_> {
    /// `solve`'s deck search of a candidate at chances `search.chances` (folded in: `set_trainee`), or the outcome of
    /// a bit-identical problem it already searched (`Twin`); records the product search's proof in `ruled`
    fn twin_deck_search(
        &mut self,
        twins: &mut crate::FxMap<TwinKey, Twin>,
        search: TwinSearch,
        n_scored: &mut usize,
    ) -> DeckRes {
        let TwinSearch { trainee: t, chances: q, spark_scale: c, bar: th, starts } = search;
        let ct = self.deck_class[t];
        let tk = (t.0 as u16, q.iter().map(|x| x.to_bits()).collect::<Vec<u64>>(), c.to_bits());
        match twins.get(&tk) {
            Some(Twin::Done(v, sel, pv)) => {
                if let Some(s) = pv {
                    self.ruled[ct].push((q.to_vec(), *s));
                }
                DeckRes::Done(*v, sel.clone(), true)
            }
            Some(Twin::Bounded(th1)) if th >= *th1 => DeckRes::Bounded,
            Some(Twin::NoDeck) => DeckRes::NoDeck,
            _ => {
                let starts: Vec<&SelectedDeck> = starts.iter().map(|r| &r.deck).collect();
                let res = self.best_deck(t, c, th, &starts, n_scored);
                let pv = self.last_product_bound.take();
                if let Some(s) = pv {
                    self.ruled[ct].push((q.to_vec(), s));
                }
                match &res {
                    DeckRes::Done(v, sel, true) => {
                        twins.insert(tk, Twin::Done(*v, sel.clone(), pv));
                    }
                    DeckRes::Done(..) => {}
                    DeckRes::Bounded => {
                        twins.insert(tk, Twin::Bounded(th));
                    }
                    DeckRes::NoDeck => {
                        twins.insert(tk, Twin::NoDeck);
                    }
                }
                res
            }
        }
    }
    /// A search's fully checked decks (`refs`, per trainee) into `seen` (per deck class), each new one once
    fn merge_seen(&mut self, refs: &UmaTable<Vec<Ref>>) {
        for (t, rs) in refs.indexed() {
            let ct = self.deck_class[t];
            for r in rs {
                if !self.seen[ct].iter().any(|x| x.val == r.val && x.q == r.q) {
                    self.seen[ct].push(r.clone());
                }
            }
        }
    }
}

/// Per legacy y, the legacies x at least as good in every career (`dominates`), ascending. Only veterans of one
/// character can dominate each other (`dominates` tests the characters first), so the other pairs are skipped unasked.
fn dominance_lists(p: &Pool) -> Vec<Vec<usize>> {
    let nl = p.veterans.len();
    let mut by_chara: crate::FxMap<u32, Vec<usize>> = crate::FxMap::default();
    for x in 0..nl {
        by_chara.entry(p.veterans[x].chara).or_default().push(x);
    }
    (0..nl)
        .map(|y| by_chara[&p.veterans[y].chara].iter().copied().filter(|&x| x != y && dominates(p, x, y)).collect())
        .collect()
}

/// Identical deck problems, searched once (`Env::solve`): a candidate whose trainee, chances q and spark scale c are
/// bit-identical to an earlier candidate's of this search (interchangeable parents: veterans with the same stars
/// and no relation or race points between them, as every sample pool of mob veterans has; 15 pairs of 6 such
/// veterans are one problem per trainee) takes that candidate's outcome instead of a second `best_deck`.
/// Derived, per outcome of the first (F, at bar th1) for the later one (R, at bar th2 >= th1: the bar only rises):
/// - Done(v, D) fully checked: F's Ref (q, v, D) is in refs[t], so R reaches the deck search only with
///   th2 <= c v (1 + 1e-9) (the Ref bound above, rise = 1 at equal q). R's product search then starts below the
///   best product value pv (th2 / (rho c (1 + 1e-9)) <= v / rho <= pv under the cap every search assumes) and finds
///   F's product deck P, so the same `ruled` proof (q, pv); R's first exact pass scores P, then its starts, which
///   hold F's (D among them), so its best is at least v from the same decks in the same order up to D; its band
///   (decks with rho x product >= its best) is within F's, every deck of which F scored or ruled out against a
///   lower best, and the starts F did not have are decks legal at q, at most v (F's search is exact under the cap).
///   So R is Done(v, D) with the same floats (the same model state, `set_trainee` at the same q); a tie or lower,
///   it goes into `done` exactly where R's own search would have put it.
/// - Bounded at th1: no deck above F's seed, or the best product deck below th1 / (rho c (1 + 1e-9)); a higher bar
///   gives a higher seed, so R is Bounded too (no `ruled` entry: R's would only repeat what F's proves).
/// - NoDeck (no legal deck at all, at any bar): R the same.
///
/// R's search would also have spent work, so a work budget can now reach further than before (the answer the
/// unbudgeted search gives); outputs are otherwise the same, only the work counters fall.
enum Twin {
    /// Fully checked: value, deck, the product search's proof.
    Done(f64, SelectedDeck, Option<f64>),
    /// Bounded at this bar.
    Bounded(f64),
    NoDeck,
}

/// A deck problem's identity in `solve`: trainee, chances q and spark scale c, to the bit.
type TwinKey = (u16, Vec<u64>, u64);

/// One candidate's deck problem in `solve` (`Env::twin_deck_search`).
#[derive(Clone, Copy)]
struct TwinSearch<'search> {
    trainee: UmaIndex,
    /// Her inheritance chances q, already folded into the model.
    chances: &'search [f64],
    /// C, the spark scale.
    spark_scale: f64,
    /// th, the value the career must beat.
    bar: f64,
    /// Her fully checked decks so far in this search: the deck search's start decks.
    starts: &'search [Ref],
}

/// What a career search covers (`Env::solve`'s arguments, as `solve_kept` takes them).
#[derive(Clone, Copy)]
pub(crate) struct SearchRequest<'request> {
    /// The legacies the parents come from.
    pub(crate) pool: &'request Pool,
    /// Legacy indices that must be among the two parents (the job's places).
    pub(crate) fixed_parents: &'request [usize],
    /// Trainees to search (uma indices).
    pub(crate) trainees: &'request [UmaIndex],
    /// One result per trainee (false: the best careers whoever trains).
    pub(crate) per_trainee: bool,
    /// How many results to keep.
    pub(crate) top: usize,
}

/// The main search of the job keyed `key` (`LAST_SOLVE`): the kept one, with what its product searches proved
/// (`env.ruled` as a combined job's plan finds it), else searched now and kept
pub(crate) fn solve_kept(env: &mut Env, key: u64, request: &SearchRequest) -> Solved {
    let kept = LAST_SOLVE.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|entry| entry.0 == key)?;
        let entry = cache.remove(index);
        cache.push(entry);
        cache.last().map(|entry| (entry.1.clone(), entry.2.clone()))
    });
    match kept {
        Some((solved, ruled)) if env.solved_fits(&solved, &ruled, request.pool) => {
            env.ruled = ruled;
            solved
        }
        _ => {
            let solved =
                env.solve(request.pool, request.fixed_parents, request.trainees, request.per_trainee, request.top);
            LAST_SOLVE.with(|cache| {
                let mut cache = cache.borrow_mut();
                cache.retain(|entry| entry.0 != key);
                cache.push((key, solved.clone(), env.ruled.clone()));
                while cache.len() > SOLVE_KEEP {
                    cache.remove(0);
                }
            });
            solved
        }
    }
}
