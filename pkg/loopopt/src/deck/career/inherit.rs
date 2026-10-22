//! Legacy slots, affinity and inheritance chances.
use super::{Env, Pool, Who, shared};
use crate::deck::career::Lineage;
use crate::deck::{JointExt, Kind, Sel, Src, joint_fast};

/// The inheritance model's per-slot terms (MODEL.md; /about states the constants), shared by every copy of the
/// inheritance chance (`inherit`, `Env::lineage_qc`, `Env::gen_cands`, `Sim::fast_ub`, the planner's her-slot setups).
/// A slot's affinity factor 1 + A / 100, A its relation part plus its race part
#[inline(always)]
pub(in crate::deck::career) fn aff_factor(rel: f64, race: f64) -> f64 {
    1.0 + (rel + race) / 100.0
}

/// one inspiration's miss of a spark at `st` stars from a slot of affinity factor `fa`
#[inline(always)]
pub(in crate::deck::career) fn insp_miss(st: f64, fa: f64) -> f64 {
    1.0 - (crate::skills::INSP_PER_STAR * st * fa).min(1.0)
}

/// both inspirations' miss of that spark
#[inline(always)]
pub(in crate::deck::career) fn insp_miss2(st: f64, fa: f64) -> f64 {
    let m = insp_miss(st, fa);
    m * m
}

/// a random-star slot's average squared miss over its star mix w (1, 2, 3 stars)
#[inline(always)]
pub(in crate::deck::career) fn mix_term(w: [f64; 3], fa: f64) -> f64 {
    (0..3).map(|i| w[i] * insp_miss2(i as f64 + 1.0, fa)).sum::<f64>()
}

/// the inheritance chance from the product of the fixed-star slots' misses and of the random-star slots' mix terms
#[inline(always)]
pub(in crate::deck::career) fn inherit_q(miss: f64, mix2: f64) -> f64 {
    1.0 - miss * miss * mix2
}

/// The spark-generation scale of a target n of the six legacy slots carry: 1.1^(n - 6) (black_box: the runtime powi
/// every caller computes, never constant-folded)
#[inline(always)]
pub(in crate::deck::career) fn spark_scale(n: i32) -> f64 {
    1.1f64.powi(std::hint::black_box(n - 6))
}

/// `Env::rise`'s factor for one outfit and uncertain target: how far the landing chance can grow from inheritance
/// chance q0 to q, with her own event hint chance `ew` and her outfit's coverage `st` of it (1 where it cannot grow).
/// The one form behind `solve`'s Ref and ruled prunes and `outcome_hi`'s anchors (`rise_leaves`, `rise_factors`,
/// `AncBb::root_min`)
#[inline(always)]
pub(in crate::deck::career) fn rise_factor(st: u8, ew: f64, q: f64, q0: f64) -> f64 {
    if st > 0 || q <= q0 { 1.0 } else { (1.0 - (1.0 - ew) * (1.0 - q)) / (1.0 - (1.0 - ew) * (1.0 - q0)) }
}

/// A slot's affinity part summed as every copy sums it: x0, then each part in order
#[inline(always)]
pub(in crate::deck::career) fn part_sum(x0: f64, parts: impl IntoIterator<Item = f64>) -> f64 {
    parts.into_iter().fold(x0, |s, v| s + v)
}

/// The trainee's relation points a slot walk (`Env::walk_slots`) reads: with legacy x of the pool (`pt`) and with x's
/// grandparent slot gi (`gt`)
pub(in crate::deck::career) trait RelPts {
    fn pt(&self, x: usize) -> f64;
    fn gt(&self, x: usize, gi: usize) -> f64;
}

/// trainee t's, from the relation tables (`Env::pair`, `Env::tri`)
pub(in crate::deck::career) struct RelOf<'env, 'data> {
    pub(in crate::deck::career) env: &'env Env<'data>,
    pub(in crate::deck::career) p: &'env Pool,
    pub(in crate::deck::career) t: usize,
}

impl RelPts for RelOf<'_, '_> {
    #[inline(always)]
    fn pt(&self, x: usize) -> f64 {
        self.env.pair(Some(self.t), self.p.me[x].ix)
    }
    #[inline(always)]
    fn gt(&self, x: usize, gi: usize) -> f64 {
        self.env.tri(Some(self.t), self.p.me[x].ix, self.p.gp[x][gi].ix)
    }
}

/// What a slot walk feeds: each of the six legacy slots in `slots`' order (parent a, her grandparent slots, then parent
/// b and hers) with its legacy x, its grandparent slot (None: the parent herself), who fills it, and its relation and
/// race parts (a parent's: the sums over her and her grandparent slots)
pub(in crate::deck::career) trait SlotSink<'pool> {
    fn slot(&mut self, x: usize, gi: Option<usize>, w: &'pool Who, rel: f64, race: f64);
}

/// `inherit`'s accumulation, slot by slot (per target the same factors in slot order): the fixed-star slots' miss
/// product, the random-star slots' mix-term product and the carrier count. `skip`: a legacy whose parent slot is left
/// out (as if she carried nothing), its affinity factor kept in `her`
struct Inherit {
    miss: Vec<f64>,
    mix2: Vec<f64>,
    nc: Vec<i32>,
    skip: Option<usize>,
    her: Option<f64>,
}

impl Inherit {
    fn new(ntar: usize, skip: Option<usize>) -> Self {
        Self { miss: vec![1.0; ntar], mix2: vec![1.0; ntar], nc: vec![0; ntar], skip, her: None }
    }
    #[inline(always)]
    fn add(&mut self, stars: &[u8], mix: Option<[f64; 3]>, fa: f64) {
        for k in 0..self.nc.len() {
            let st = stars[k];
            if st == 0 {
                continue;
            }
            self.nc[k] += 1;
            match mix {
                None => self.miss[k] *= insp_miss(f64::from(st), fa),
                Some(w) => self.mix2[k] *= mix_term(w, fa),
            }
        }
    }
    /// (q per target, carriers per target)
    fn finish(self) -> (Vec<f64>, Vec<i32>) {
        (self.miss.iter().zip(&self.mix2).map(|(&m, &x)| inherit_q(m, x)).collect(), self.nc)
    }
}

/// `Inherit` without a skipped slot, in fixed arrays (at most `LQN` job targets; `Env::lineage`'s): the same
/// factors multiplied in the same slot order
const LQN: usize = 16;

struct InheritA {
    n: usize,
    miss: [f64; LQN],
    mix2: [f64; LQN],
    nc: [i32; LQN],
}

impl SlotSink<'_> for InheritA {
    #[inline(always)]
    fn slot(&mut self, _: usize, _: Option<usize>, w: &Who, rel: f64, race: f64) {
        let fa = aff_factor(rel, race);
        for k in 0..self.n {
            let st = w.stars[k];
            if st == 0 {
                continue;
            }
            self.nc[k] += 1;
            match w.mix {
                None => self.miss[k] *= insp_miss(f64::from(st), fa),
                Some(m) => self.mix2[k] *= mix_term(m, fa),
            }
        }
    }
}

impl SlotSink<'_> for Inherit {
    #[inline(always)]
    fn slot(&mut self, x: usize, gi: Option<usize>, w: &Who, rel: f64, race: f64) {
        if gi.is_none() && self.skip == Some(x) {
            self.her = Some(aff_factor(rel, race));
            return;
        }
        self.add(&w.stars, w.mix, aff_factor(rel, race));
    }
}

/// A legacy slot of the planner's veteran-to-be (`Env::her_slot`): the trainee's inheritance chances and carriers with
/// her slot left out (q0, nc0), per uncertain target 1 - q0 (x0), her miss factor squared per star count (xms; 1 at 0
/// stars) and per job target the spark scale without and with her spark (pw)
pub(in crate::deck) struct HerSlot {
    pub(in crate::deck) q0: Vec<f64>,
    pub(in crate::deck) x0: Vec<f64>,
    pub(in crate::deck) xms: [f64; 4],
    pub(in crate::deck) pw: Vec<[f64; 2]>,
}

/// One legacy slot of a lineage: affinity parts and per-target stars.
#[derive(Clone)]
pub(in crate::deck::career) struct Slot {
    pub(in crate::deck::career) parent: bool,
    pub(in crate::deck::career) legacy: usize,
    pub(in crate::deck::career) gp: Option<usize>,
    pub(in crate::deck::career) chara: u32,
    pub(in crate::deck::career) rel: f64,
    pub(in crate::deck::career) race: f64,
    pub(in crate::deck::career) stars: std::rc::Rc<Vec<u8>>,
    pub(in crate::deck::career) pinks: std::rc::Rc<Vec<(u32, u8)>>,
    pub(in crate::deck::career) mix: Option<[f64; 3]>,
}

/// `Env::set_trainee` on the fields it writes and reads (her model hint chances `cur`, her own `ew0`), for callers that
/// hold a borrow of the deck space
pub(in crate::deck::career) fn set_ews(cur: &mut Vec<Vec<f64>>, ew0: &[Vec<f64>], uncv: &[usize], q: &[f64]) {
    // (in place when the shape is already this one: the same values)
    if cur.len() == ew0.len() && cur.iter().zip(ew0).all(|(c, o)| c.len() == o.len().min(uncv.len())) {
        for (c, o) in cur.iter_mut().zip(ew0) {
            for ((c, e), &i) in c.iter_mut().zip(o).zip(uncv) {
                *c = 1.0 - (1.0 - e) * (1.0 - q[i]);
            }
        }
    } else {
        *cur = ew0.iter().map(|o| o.iter().zip(uncv).map(|(e, &i)| 1.0 - (1.0 - e) * (1.0 - q[i])).collect()).collect();
    }
}

/// Per job target: the inheritance chance and how many of the six carry the spark. A slot with random stars (`mix`)
/// enters as the average of its squared miss factor (its stars are independent of everything else).
pub(in crate::deck::career) fn inherit(sl: &[Slot], ntar: usize) -> (Vec<f64>, Vec<i32>) {
    let mut acc = Inherit::new(ntar, None);
    for s in sl {
        acc.add(&s.stars, s.mix, aff_factor(s.rel, s.race));
    }
    acc.finish()
}

impl Env<'_> {
    pub(in crate::deck::career) fn pair(&self, a: Option<usize>, b: Option<usize>) -> f64 {
        match (a, b) {
            (Some(a), Some(b)) => f64::from(self.d.pair[a][b]),
            _ => 0.0,
        }
    }
    pub(in crate::deck::career) fn tri(&self, a: Option<usize>, b: Option<usize>, c: Option<usize>) -> f64 {
        match (a, b, c) {
            (Some(a), Some(b), Some(c)) => f64::from(self.d.tri[a][b][c]),
            _ => 0.0,
        }
    }
    /// The six legacy slots with parents a and b, in order, into `s` (`SlotSink`), the trainee's relation points from
    /// `r`: per parent x (partner y) her relation part, pt(x) + pair(x, y) then her grandparent slots' gt(x, gi) in
    /// order, and her race part, 3 x shared(a, b) then theirs (`Pool::gpc`) in order (`part_sum`); a grandparent slot's
    /// parts are its own. The one definition of the slots' affinity every copy reads (`slots`, `lineage`,
    /// `lineage_qc`, `her_slot`, `Sim::fast_ub`; `gen_cands` hoists the same sums)
    #[inline(always)]
    pub(in crate::deck::career) fn walk_slots<'pool, R: RelPts, S: SlotSink<'pool>>(
        &self,
        p: &'pool Pool,
        a: usize,
        b: usize,
        r: &R,
        s: &mut S,
    ) {
        // (the parents' shared wins once: shared(a, b) and shared(b, a) are the same products summed in the same order)
        let sh = 3.0 * shared(&p.me[a], &p.me[b]);
        for (x, y) in [(a, b), (b, a)] {
            let px = &p.me[x];
            let ng = p.gp[x].len();
            let rel = part_sum(r.pt(x) + self.pair(px.ix, p.me[y].ix), (0..ng).map(|gi| r.gt(x, gi)));
            let race = part_sum(sh, p.gpc[x].iter().copied());
            s.slot(x, None, px, rel, race);
            for (gi, g) in p.gp[x].iter().enumerate() {
                s.slot(x, Some(gi), g, r.gt(x, gi), p.gpc[x][gi]);
            }
        }
    }
    /// the six legacy slots of trainee t with parents a and b
    pub(in crate::deck::career) fn slots(&self, p: &Pool, t: usize, a: usize, b: usize) -> Vec<Slot> {
        struct V(Vec<Slot>);
        impl SlotSink<'_> for V {
            fn slot(&mut self, x: usize, gi: Option<usize>, w: &Who, rel: f64, race: f64) {
                self.0.push(Slot {
                    parent: gi.is_none(),
                    legacy: x,
                    gp: gi,
                    chara: w.chara,
                    rel,
                    race,
                    stars: std::rc::Rc::clone(&w.stars),
                    pinks: std::rc::Rc::clone(&w.pinks),
                    mix: w.mix,
                });
            }
        }
        let mut v = V(Vec::with_capacity(2 + p.gp[a].len() + p.gp[b].len()));
        self.walk_slots(p, a, b, &RelOf { env: self, p, t }, &mut v);
        v.0
    }
    /// Legacy `iv`'s slot as a parent of trainee t beside partner y (`HerSlot`; None when she is not one of the two);
    /// what `score_career`'s setups and `outcome_hi`'s partners take
    pub(in crate::deck) fn her_slot(&self, p: &Pool, lineage: Lineage) -> Option<HerSlot> {
        let Lineage { trainee: t, first_parent: y, second_parent: iv } = lineage;
        let mut acc = Inherit::new(self.ntar, Some(iv));
        self.walk_slots(p, y, iv, &RelOf { env: self, p, t }, &mut acc);
        let fa = acc.her?;
        let (q0, nc0) = acc.finish();
        let xm = |st: u8| insp_miss2(f64::from(st), fa);
        Some(HerSlot {
            x0: self.uncv.iter().map(|&i| 1.0 - q0[i]).collect(),
            xms: [1.0, xm(1), xm(2), xm(3)],
            pw: nc0.iter().map(|&n| [spark_scale(n), spark_scale(n + 1)]).collect(),
            q0,
        })
    }
    pub(in crate::deck::career) fn scale(&self, nc: &[i32]) -> (Vec<f64>, Vec<f64>) {
        (
            self.uncv.iter().map(|&i| spark_scale(nc[i])).collect(),
            self.fixed_t.iter().map(|&i| spark_scale(nc[i])).collect(),
        )
    }
    /// trainee t's event hint chances with her lineage's inheritance chances folded in (the search then runs at q = 0)
    pub(in crate::deck::career) fn set_trainee(&mut self, t: usize, q: &[f64]) {
        set_ews(&mut self.model.ews[t], &self.ew0[t], &self.uncv, q);
    }
    /// a deck's entries (sources borrowed from the deck space: `joint` only reads them)
    pub(in crate::deck::career) fn ents_of(&self, sel: &Sel) -> Vec<(&Src, Kind)> {
        sel.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect()
    }
    /// exact P(all) of trainee t (her inheritance already folded in) with deck `sel`, scaled per target; `sets`: also the
    /// chance of each set of uncertain targets landing
    pub(in crate::deck::career) fn exact(
        &self,
        t: usize,
        sel: &Sel,
        nc: &[i32],
        m1: bool,
        sets: bool,
    ) -> ((f64, f64, usize), JointExt) {
        let (su, sf) = self.scale(nc);
        let mut ext =
            JointExt { su, sf, marg: vec![], sets: if sets { vec![0.0] } else { vec![] }, per_outfit: vec![] };
        let j = joint_fast(&self.model, t, &[0.0], &self.ents_of(sel), sel.filler, self.ntar, m1, Some(&mut ext))[0];
        (j, ext)
    }

    /// How far trainee t's best exact value (before the spark scale) can rise from chances q0 to q. A target the
    /// outfit lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no hint,
    /// e: her own event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 - q0_k))
    /// (n = 1, g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products of
    /// these factors, so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of it.
    pub(in crate::deck::career) fn rise(&self, t: usize, q: &[f64], q0: &[f64]) -> f64 {
        self.sts0[t]
            .iter()
            .zip(&self.ew0[t])
            .map(|(st, ew)| {
                self.uncv.iter().enumerate().map(|(k, &i)| rise_factor(st[k], ew[k], q[i], q0[i])).product::<f64>()
            })
            .fold(0.0f64, f64::max)
    }
    /// The inheritance chances, carriers and spark scale C of trainee t with parents a and b.
    pub(in crate::deck::career) fn lineage(&self, p: &Pool, t: usize, a: usize, b: usize) -> (Vec<f64>, Vec<i32>, f64) {
        if self.ntar > LQN {
            let mut acc = Inherit::new(self.ntar, None);
            self.walk_slots(p, a, b, &RelOf { env: self, p, t }, &mut acc);
            let (q, nc) = acc.finish();
            let c = self.c_of(&nc);
            return (q, nc, c);
        }
        let acc = self.lineage_acc(p, t, a, b);
        (
            (0..acc.n).map(|k| inherit_q(acc.miss[k], acc.mix2[k])).collect(),
            acc.nc[..acc.n].to_vec(),
            self.c_of(&acc.nc),
        )
    }
    /// `lineage`'s chances and spark scale C (the same numbers), without the carriers
    pub(in crate::deck::career) fn lineage_q(&self, p: &Pool, lineage: Lineage) -> (Vec<f64>, f64) {
        let mut q = Vec::with_capacity(self.ntar);
        let c = self.lineage_into(p, lineage, &mut q);
        (q, c)
    }
    /// `lineage`'s chances into `q` (cleared first) and its spark scale C, the same numbers
    pub(in crate::deck::career) fn lineage_into(&self, p: &Pool, lineage: Lineage, q: &mut Vec<f64>) -> f64 {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        q.clear();
        if self.ntar > LQN {
            let (qq, _, c) = self.lineage(p, t, a, b);
            q.extend_from_slice(&qq);
            return c;
        }
        let acc = self.lineage_acc(p, t, a, b);
        q.extend((0..acc.n).map(|k| inherit_q(acc.miss[k], acc.mix2[k])));
        self.c_of(&acc.nc)
    }
    /// `inherit`'s accumulation over the six slots of trainee t with parents a and b (ntar <= LQN)
    #[inline(always)]
    fn lineage_acc(&self, p: &Pool, t: usize, a: usize, b: usize) -> InheritA {
        let mut acc = InheritA { n: self.ntar, miss: [1.0; LQN], mix2: [1.0; LQN], nc: [0; LQN] };
        self.walk_slots(p, a, b, &RelOf { env: self, p, t }, &mut acc);
        acc
    }
    /// the spark scale C of carrier counts nc: `scale`'s factors multiplied from 1.0, uncertain targets then fixed ones
    #[inline(always)]
    fn c_of(&self, nc: &[i32]) -> f64 {
        self.uncv.iter().chain(&self.fixed_t).map(|&i| spark_scale(nc[i])).product::<f64>()
    }
    /// `lineage`'s spark scale C, and its inheritance chances for the targets `ks` only (into q[i] for i in ks, the
    /// other entries of q left as they are), without allocating: the same slot walk (`walk_slots`) into `inherit`'s
    /// accumulation for those targets (each slot's factors multiplied in slot order, a slot without the spark skipped)
    /// and the carriers of every target, then `scale`'s product. More than four grandparent slots per parent, or more
    /// than QN targets: through `lineage` itself. `tg`: t's relation points with each legacy x of `p` (`pair`, at 5 x +
    /// 4) and with x's grandparent slots (`tri`, at 5 x + the slot), then `scale`'s factor per carrier count; built here
    /// when empty: the caller empties it for another trainee or pool.
    pub(in crate::deck::career) fn lineage_qc(
        &self,
        p: &Pool,
        lineage: Lineage,
        ks: &[usize],
        q: &mut [f64],
        tg: &mut Vec<f64>,
    ) -> f64 {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        const G: usize = 4;
        const QN: usize = 16;
        if p.gp[a].len() > G || p.gp[b].len() > G || self.ntar > QN {
            let (qq, _, c) = self.lineage(p, t, a, b);
            for &i in ks {
                q[i] = qq[i];
            }
            return c;
        }
        let ti = Some(t);
        let nl = p.me.len();
        if tg.is_empty() {
            tg.resize(nl * (G + 1), 0.0);
            for x in 0..nl {
                tg[x * (G + 1) + G] = self.pair(ti, p.me[x].ix);
                for (gi, g) in p.gp[x].iter().enumerate().take(G) {
                    tg[x * (G + 1) + gi] = self.tri(ti, p.me[x].ix, g.ix);
                }
            }
            // then `spark_scale` per carrier count n
            tg.extend((0..=2 * (G + 1)).map(|n| spark_scale(n as i32)));
        }
        struct Tg<'tg>(&'tg [f64]);
        impl RelPts for Tg<'_> {
            #[inline(always)]
            fn pt(&self, x: usize) -> f64 {
                self.0[x * (G + 1) + G]
            }
            #[inline(always)]
            fn gt(&self, x: usize, gi: usize) -> f64 {
                self.0[x * (G + 1) + gi]
            }
        }
        // `Inherit` for the targets `ks` (and the carriers of every target), in fixed arrays
        struct Qc<'ks> {
            ks: &'ks [usize],
            ntar: usize,
            miss: [f64; QN],
            mix2: [f64; QN],
            nc: [usize; QN],
        }
        impl SlotSink<'_> for Qc<'_> {
            #[inline(always)]
            fn slot(&mut self, _: usize, _: Option<usize>, w: &Who, rel: f64, race: f64) {
                let fa = aff_factor(rel, race);
                for &k in self.ks {
                    let s = w.stars[k];
                    if s == 0 {
                        continue;
                    }
                    match w.mix {
                        None => self.miss[k] *= insp_miss(f64::from(s), fa),
                        Some(m) => self.mix2[k] *= mix_term(m, fa),
                    }
                }
                for k in 0..self.ntar {
                    if w.stars[k] != 0 {
                        self.nc[k] += 1;
                    }
                }
            }
        }
        let mut acc = Qc { ks, ntar: self.ntar, miss: [1.0; QN], mix2: [1.0; QN], nc: [0; QN] };
        self.walk_slots(p, a, b, &Tg(&tg[..nl * (G + 1)]), &mut acc);
        for &k in ks {
            q[k] = inherit_q(acc.miss[k], acc.mix2[k]);
        }
        let pw = &tg[nl * (G + 1)..];
        self.uncv.iter().chain(&self.fixed_t).map(|&i| pw[acc.nc[i]]).product::<f64>()
    }
}
