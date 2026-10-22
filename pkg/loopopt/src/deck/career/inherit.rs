//! Legacy slots, affinity and inheritance chances.
use super::{Env, Pool, Who, shared};
use crate::deck::career::Lineage;
use crate::deck::{JointExt, SelectedDeck, TargetSources, joint_fast};

/// The inheritance model's per-slot terms (MODEL.md; /about states the constants), shared by every copy of the
/// inheritance chance (`inherit`, `Env::lineage_qc`, `Env::gen_cands`, `Sim::fast_ub`, the planner's her-slot setups).
/// A slot's affinity factor 1 + A / 100, A its relation part plus its race part
#[inline(always)]
pub(crate) fn aff_factor(rel: f64, race: f64) -> f64 {
    1.0 + (rel + race) / 100.0
}

/// one inspiration's miss of a spark at `st` stars from a slot of affinity factor `fa`
#[inline(always)]
pub(crate) fn insp_miss(st: f64, fa: f64) -> f64 {
    1.0 - (crate::skills::INSP_PER_STAR * st * fa).min(1.0)
}

/// both inspirations' miss of that spark
#[inline(always)]
pub(crate) fn insp_miss2(st: f64, fa: f64) -> f64 {
    let m = insp_miss(st, fa);
    m * m
}

/// a random-star slot's average squared miss over its star mix w (1, 2, 3 stars)
#[inline(always)]
pub(crate) fn mix_term(w: [f64; 3], fa: f64) -> f64 {
    (0..3).map(|i| w[i] * insp_miss2(i as f64 + 1.0, fa)).sum::<f64>()
}

/// the inheritance chance from the product of the fixed-star slots' misses and of the random-star slots' mix terms
#[inline(always)]
pub(crate) fn inherit_q(miss: f64, mix2: f64) -> f64 {
    1.0 - miss * miss * mix2
}

/// The spark-generation scale of a target n of the six legacy slots carry: 1.1^(n - 6) (black_box: the runtime powi
/// every caller computes, never constant-folded)
#[inline(always)]
pub(crate) fn spark_scale(n: i32) -> f64 {
    1.1f64.powi(std::hint::black_box(n - 6))
}

/// `Env::rise`'s factor for one outfit and uncertain target: how far the landing chance can grow from inheritance
/// chance q0 to q, with her own event hint chance `ew` and her outfit's coverage `st` of it (1 where it cannot grow).
/// The one form behind `solve`'s Ref and ruled prunes and `outcome_hi`'s anchors (`rise_leaves`, `rise_factors`,
/// `AnchorBound::root_min`)
#[inline(always)]
pub(crate) fn rise_factor(st: u8, ew: f64, q: f64, q0: f64) -> f64 {
    if st > 0 || q <= q0 { 1.0 } else { (1.0 - (1.0 - ew) * (1.0 - q)) / (1.0 - (1.0 - ew) * (1.0 - q0)) }
}

/// A slot's affinity part summed as every copy sums it: x0, then each part in order
#[inline(always)]
pub(crate) fn part_sum(x0: f64, parts: impl IntoIterator<Item = f64>) -> f64 {
    parts.into_iter().fold(x0, |s, v| s + v)
}

/// The trainee's relation points a slot walk (`Env::walk_slots`) reads: with legacy x of the pool (`pt`) and with x's
/// grandparent slot gi (`gt`)
pub(crate) trait RelPts {
    fn pt(&self, x: usize) -> f64;
    fn gt(&self, x: usize, gi: usize) -> f64;
}

/// trainee t's, from the relation tables (`Env::pair`, `Env::tri`)
pub(crate) struct RelOf<'env, 'data> {
    pub(crate) env: &'env Env<'data>,
    pub(crate) p: &'env Pool,
    pub(crate) t: crate::UmaIndex,
}

impl RelPts for RelOf<'_, '_> {
    #[inline(always)]
    fn pt(&self, x: usize) -> f64 {
        self.env.pair(Some(self.t), self.p.veterans[x].uma_index)
    }
    #[inline(always)]
    fn gt(&self, x: usize, gi: usize) -> f64 {
        self.env.tri(Some(self.t), self.p.veterans[x].uma_index, self.p.parents[x][gi].uma_index)
    }
}

/// What a slot walk feeds: each of the six legacy slots in `slots`' order (parent a, her grandparent slots, then parent
/// b and hers) with its legacy x, its grandparent slot (None: the parent herself), who fills it, and its relation and
/// race parts (a parent's: the sums over her and her grandparent slots)
pub(crate) trait SlotSink<'pool> {
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
    fn finish(self) -> Inheritance {
        Inheritance {
            chances: self.miss.iter().zip(&self.mix2).map(|(&m, &x)| inherit_q(m, x)).collect(),
            carriers: self.nc,
        }
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
            match w.star_mix {
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
        self.add(&w.stars, w.star_mix, aff_factor(rel, race));
    }
}

/// A legacy slot of the planner's veteran-to-be (`Env::her_slot`): the trainee's inheritance with her slot left out,
/// and what her veteran in the slot adds.
pub(crate) struct HerSlot {
    /// q0: per job target, the trainee's inheritance chance with her slot left out.
    pub(crate) chances_without_her: Vec<f64>,
    /// x0: per uncertain target (`Env::uncertain_targets` order), 1 - q0.
    pub(crate) misses_without_her: Vec<f64>,
    /// xms: her miss factor squared per star count (1 at 0 stars).
    pub(crate) miss_by_stars: [f64; 4],
    /// pw: per job target, the spark scale without and with her spark (from the carriers without her, nc0).
    pub(crate) target_spark_scales: Vec<[f64; 2]>,
}

/// One legacy slot of a lineage: affinity parts and per-target stars.
#[derive(Clone)]
pub(crate) struct Slot {
    pub(crate) parent: bool,
    pub(crate) legacy: usize,
    pub(crate) gp: Option<usize>,
    pub(crate) chara: u32,
    pub(crate) rel: f64,
    pub(crate) race: f64,
    pub(crate) stars: std::rc::Rc<Vec<u8>>,
    pub(crate) pinks: std::rc::Rc<Vec<(u32, u8)>>,
    pub(crate) mix: Option<[f64; 3]>,
}

/// `Env::set_trainee` on the fields it writes and reads (her model hint chances `cur`, her own `own_event_chances`),
/// for callers that hold a borrow of the deck space
pub(crate) fn set_ews(cur: &mut Vec<Vec<f64>>, ew0: &[Vec<f64>], uncv: &[usize], q: &[f64]) {
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

/// What `Env::exact` computes beside P(all).
#[derive(Clone, Copy)]
pub(crate) struct ExactOptions {
    /// Also P(all but one) (else 0).
    pub(crate) all_but_one: bool,
    /// Also the chance of each set of uncertain targets landing (`JointExt::sets`, read by `Env::outcome_sets`).
    pub(crate) landed_sets: bool,
}

/// One career's exact numbers (`Env::exact`), at her best outfit.
pub(crate) struct ExactCareer {
    pub(crate) p_all: f64,
    /// P(every target but at most one lands); 0 unless asked for.
    pub(crate) p_all_but_one: f64,
    /// The best outfit's index in her model options.
    pub(crate) outfit: usize,
    /// The per-target scales, marginals and landed-set chances (`JointExt`).
    pub(crate) ext: JointExt,
}

/// A lineage's inheritance, per job target.
pub(crate) struct Inheritance {
    /// The chance q_k that the trainee inherits the target's spark.
    pub(crate) chances: Vec<f64>,
    /// How many of the six legacy slots carry a spark of the target (n_k, the spark scale's count).
    pub(crate) carriers: Vec<i32>,
}

/// `Inheritance` with the spark scale C of its carrier counts (`Env::lineage`).
pub(crate) struct LineageChances {
    /// The chance q_k per job target.
    pub(crate) chances: Vec<f64>,
    /// Carrier count n_k per job target.
    pub(crate) carriers: Vec<i32>,
    /// C, the product of the targets' spark scales.
    pub(crate) spark_scale: f64,
}

/// Per job target: the inheritance chance and how many of the six carry the spark. A slot with random stars (`mix`)
/// enters as the average of its squared miss factor (its stars are independent of everything else).
pub(crate) fn inherit(sl: &[Slot], ntar: usize) -> Inheritance {
    let mut acc = Inherit::new(ntar, None);
    for s in sl {
        acc.add(&s.stars, s.mix, aff_factor(s.rel, s.race));
    }
    acc.finish()
}

impl Env<'_> {
    pub(crate) fn pair(&self, a: Option<crate::UmaIndex>, b: Option<crate::UmaIndex>) -> f64 {
        match (a, b) {
            (Some(a), Some(b)) => f64::from(self.data.pair[a][b]),
            _ => 0.0,
        }
    }
    pub(crate) fn tri(
        &self,
        a: Option<crate::UmaIndex>,
        b: Option<crate::UmaIndex>,
        c: Option<crate::UmaIndex>,
    ) -> f64 {
        match (a, b, c) {
            (Some(a), Some(b), Some(c)) => f64::from(self.data.tri[a][b][c]),
            _ => 0.0,
        }
    }
    /// The six legacy slots with parents a and b, in order, into `s` (`SlotSink`), the trainee's relation points from
    /// `r`: per parent x (partner y) her relation part, pt(x) + pair(x, y) then her grandparent slots' gt(x, gi) in
    /// order, and her race part, 3 x shared(a, b) then theirs (`Pool::grandparent_race_affinity`) in order
    /// (`part_sum`); a grandparent slot's parts are its own. The one definition of the slots' affinity every copy reads
    /// (`slots`, `lineage`, `lineage_qc`, `her_slot`, `Sim::fast_ub`; `gen_cands` hoists the same sums)
    #[inline(always)]
    pub(crate) fn walk_slots<'pool, R: RelPts, S: SlotSink<'pool>>(
        &self,
        p: &'pool Pool,
        a: usize,
        b: usize,
        r: &R,
        s: &mut S,
    ) {
        // (the parents' shared wins once: shared(a, b) and shared(b, a) are the same products summed in the same order)
        let sh = 3.0 * shared(&p.veterans[a], &p.veterans[b]);
        for (x, y) in [(a, b), (b, a)] {
            let px = &p.veterans[x];
            let ng = p.parents[x].len();
            let rel =
                part_sum(r.pt(x) + self.pair(px.uma_index, p.veterans[y].uma_index), (0..ng).map(|gi| r.gt(x, gi)));
            let race = part_sum(sh, p.grandparent_race_affinity[x].iter().copied());
            s.slot(x, None, px, rel, race);
            for (gi, g) in p.parents[x].iter().enumerate() {
                s.slot(x, Some(gi), g, r.gt(x, gi), p.grandparent_race_affinity[x][gi]);
            }
        }
    }
    /// the six legacy slots of the lineage's trainee with her parents, first parent's first
    pub(crate) fn slots(&self, p: &Pool, lineage: Lineage) -> Vec<Slot> {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
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
                    mix: w.star_mix,
                });
            }
        }
        let mut v = V(Vec::with_capacity(2 + p.parents[a].len() + p.parents[b].len()));
        self.walk_slots(p, a, b, &RelOf { env: self, p, t }, &mut v);
        v.0
    }
    /// Legacy `iv`'s slot as a parent of trainee t beside partner y (`HerSlot`; None when she is not one of the two);
    /// what `score_career`'s setups and `outcome_hi`'s partners take
    pub(crate) fn her_slot(&self, p: &Pool, lineage: Lineage) -> Option<HerSlot> {
        let Lineage { trainee: t, first_parent: y, second_parent: iv } = lineage;
        let mut acc = Inherit::new(self.target_count, Some(iv));
        self.walk_slots(p, y, iv, &RelOf { env: self, p, t }, &mut acc);
        let fa = acc.her?;
        let Inheritance { chances: q0, carriers: nc0 } = acc.finish();
        let xm = |st: u8| insp_miss2(f64::from(st), fa);
        Some(HerSlot {
            misses_without_her: self.uncertain_targets.iter().map(|&i| 1.0 - q0[i]).collect(),
            miss_by_stars: [1.0, xm(1), xm(2), xm(3)],
            target_spark_scales: nc0.iter().map(|&n| [spark_scale(n), spark_scale(n + 1)]).collect(),
            chances_without_her: q0,
        })
    }
    pub(crate) fn scale(&self, nc: &[i32]) -> crate::deck::joint::SparkScales {
        crate::deck::joint::SparkScales {
            uncertain: self.uncertain_targets.iter().map(|&i| spark_scale(nc[i])).collect(),
            guaranteed: self.fixed_targets.iter().map(|&i| spark_scale(nc[i])).collect(),
        }
    }
    /// trainee t's event hint chances with her lineage's inheritance chances folded in (the search then runs at q = 0)
    pub(crate) fn set_trainee(&mut self, t: crate::UmaIndex, q: &[f64]) {
        set_ews(&mut self.model.ews[t], &self.own_event_chances[t], &self.uncertain_targets, q);
    }
    /// a deck's entries (sources borrowed from the deck space: `joint` only reads them)
    pub(crate) fn ents_of(&self, sel: &SelectedDeck) -> Vec<crate::deck::SourceEntry<&TargetSources>> {
        sel.entries
            .iter()
            .map(|&j| crate::deck::SourceEntry { sources: &self.space.sources[j], kind: self.space.kind[j] })
            .collect()
    }
    /// exact P(all) of trainee t (her inheritance already folded in) with deck `sel`, scaled per target by the
    /// carrier counts `nc`; `options` says what else to compute
    pub(crate) fn exact(
        &self,
        t: crate::UmaIndex,
        sel: &SelectedDeck,
        nc: &[i32],
        options: ExactOptions,
    ) -> ExactCareer {
        let scales = self.scale(nc);
        let mut ext = JointExt {
            scales,
            marginals: vec![],
            sets: if options.landed_sets { vec![0.0] } else { vec![] },
            per_outfit: vec![],
        };
        let crate::deck::joint::JointChances { all: p_all, all_but_one: p_all_but_one, outfit } = joint_fast(
            &self.model,
            t,
            &[0.0],
            &self.ents_of(sel),
            sel.filler_cards,
            self.target_count,
            options.all_but_one,
            Some(&mut ext),
        )[0];
        ExactCareer { p_all, p_all_but_one, outfit, ext }
    }

    /// How far trainee t's best exact value (before the spark scale) can rise from chances q0 to q. A target the
    /// outfit lacks lands with GG g + GW (1 - g - n (1 - e)(1 - q_k)) per chain state (g: gold hinted, n: no hint,
    /// e: her own event hint chance); from q0 to q it grows at most by (1 - (1 - e)(1 - q_k)) / (1 - (1 - e)(1 - q0_k))
    /// (n = 1, g = 0), and not at all when q_k <= q0_k or the outfit has the skill. P(all) is a sum of products of
    /// these factors, so it grows at most by the product, for every deck: E(q) <= E(q0) x max over outfits of it.
    pub(crate) fn rise(&self, t: crate::UmaIndex, q: &[f64], q0: &[f64]) -> f64 {
        self.outfit_coverage[t]
            .iter()
            .zip(&self.own_event_chances[t])
            .map(|(st, ew)| {
                self.uncertain_targets
                    .iter()
                    .enumerate()
                    .map(|(k, &i)| rise_factor(st[k], ew[k], q[i], q0[i]))
                    .product::<f64>()
            })
            .fold(0.0f64, f64::max)
    }
    /// The lineage's inheritance chances, carriers and spark scale C.
    pub(crate) fn lineage(&self, p: &Pool, lineage: Lineage) -> LineageChances {
        if self.target_count > LQN {
            let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
            let mut acc = Inherit::new(self.target_count, None);
            self.walk_slots(p, a, b, &RelOf { env: self, p, t }, &mut acc);
            let Inheritance { chances, carriers } = acc.finish();
            let spark_scale = self.c_of(&carriers);
            return LineageChances { chances, carriers, spark_scale };
        }
        let acc = self.lineage_acc(p, lineage);
        LineageChances {
            chances: (0..acc.n).map(|k| inherit_q(acc.miss[k], acc.mix2[k])).collect(),
            carriers: acc.nc[..acc.n].to_vec(),
            spark_scale: self.c_of(&acc.nc),
        }
    }
    /// `lineage`'s chances and spark scale C (the same numbers), without the carriers
    pub(crate) fn lineage_q(&self, p: &Pool, lineage: Lineage) -> (Vec<f64>, f64) {
        let mut q = Vec::with_capacity(self.target_count);
        let c = self.lineage_into(p, lineage, &mut q);
        (q, c)
    }
    /// `lineage`'s chances into `q` (cleared first) and its spark scale C, the same numbers
    pub(crate) fn lineage_into(&self, p: &Pool, lineage: Lineage, q: &mut Vec<f64>) -> f64 {
        q.clear();
        if self.target_count > LQN {
            let chances = self.lineage(p, lineage);
            q.extend_from_slice(&chances.chances);
            return chances.spark_scale;
        }
        let acc = self.lineage_acc(p, lineage);
        q.extend((0..acc.n).map(|k| inherit_q(acc.miss[k], acc.mix2[k])));
        self.c_of(&acc.nc)
    }
    /// `inherit`'s accumulation over the lineage's six slots (ntar <= LQN)
    #[inline(always)]
    fn lineage_acc(&self, p: &Pool, lineage: Lineage) -> InheritA {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        let mut acc = InheritA { n: self.target_count, miss: [1.0; LQN], mix2: [1.0; LQN], nc: [0; LQN] };
        self.walk_slots(p, a, b, &RelOf { env: self, p, t }, &mut acc);
        acc
    }
    /// the spark scale C of carrier counts nc: `scale`'s factors multiplied from 1.0, uncertain targets then fixed ones
    #[inline(always)]
    fn c_of(&self, nc: &[i32]) -> f64 {
        self.uncertain_targets.iter().chain(&self.fixed_targets).map(|&i| spark_scale(nc[i])).product::<f64>()
    }
    /// `lineage`'s spark scale C, and its inheritance chances for the targets `ks` only (into q[i] for i in ks, the
    /// other entries of q left as they are), without allocating: the same slot walk (`walk_slots`) into `inherit`'s
    /// accumulation for those targets (each slot's factors multiplied in slot order, a slot without the spark skipped)
    /// and the carriers of every target, then `scale`'s product. More than four grandparent slots per parent, or more
    /// than QN targets: through `lineage` itself. `tg`: t's relation points with each legacy x of `p` (`pair`, at 5 x +
    /// 4) and with x's grandparent slots (`tri`, at 5 x + the slot), then `scale`'s factor per carrier count; built here
    /// when empty: the caller empties it for another trainee or pool.
    pub(crate) fn lineage_qc(&self, p: &Pool, lineage: Lineage, ks: &[usize], q: &mut [f64], tg: &mut Vec<f64>) -> f64 {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        const G: usize = 4;
        const QN: usize = 16;
        if p.parents[a].len() > G || p.parents[b].len() > G || self.target_count > QN {
            let chances = self.lineage(p, lineage);
            for &i in ks {
                q[i] = chances.chances[i];
            }
            return chances.spark_scale;
        }
        let ti = Some(t);
        let nl = p.veterans.len();
        if tg.is_empty() {
            tg.resize(nl * (G + 1), 0.0);
            for x in 0..nl {
                tg[x * (G + 1) + G] = self.pair(ti, p.veterans[x].uma_index);
                for (gi, g) in p.parents[x].iter().enumerate().take(G) {
                    tg[x * (G + 1) + gi] = self.tri(ti, p.veterans[x].uma_index, g.uma_index);
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
                    match w.star_mix {
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
        let mut acc = Qc { ks, ntar: self.target_count, miss: [1.0; QN], mix2: [1.0; QN], nc: [0; QN] };
        self.walk_slots(p, a, b, &Tg(&tg[..nl * (G + 1)]), &mut acc);
        for &k in ks {
            q[k] = inherit_q(acc.miss[k], acc.mix2[k]);
        }
        let pw = &tg[nl * (G + 1)..];
        self.uncertain_targets.iter().chain(&self.fixed_targets).map(|&i| pw[acc.nc[i]]).product::<f64>()
    }
}
