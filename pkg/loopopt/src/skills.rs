//! The skill-target model (`Model`) that the deck search and the career planner score structures with, and the
//! schedule chart they report.
//!
//! Loop model per training run (trainee T):
//!   - every target skill's spark sits on all 6 legacies, with stars drawn from a rank distribution
//!     (the expected base rate stands in for a legacy's unknown stars: exact for one roll, but over the career's two rolls
//!     it slightly overstates the chance, since a legacy's stars are the same in both: docs/RESEARCH.md section 2);
//!   - a skill is acquired if guaranteed, on T's own outfit skill list, hinted by the deck, or proc'd from the lineage
//!     (P = 1 - prod(1 - base*(1+aff/100))^INSPIRATIONS over the six legacies, parents and grandparents alike);
//!   - each legacy's affinity follows docs/RESEARCH.md section 5: a parent P counts pair(T,P), the whole
//!     pair(P1,P2), 3 per G1 shared with the other parent, and for each of P's own parents G the trio (T,P,G) plus
//!     3 per G1 shared with G; a grandparent G under P counts trio(T,P,G) plus 3 per G1 shared with P. In a loop P's
//!     own parents are the other parent and the fourth member, so the parents' shared G1s count twice for each;
//!   - it then lands as a spark with GEN_GOLD if the gold version is bought, else GEN_WHITE (20% / 40% x 1.1^6: all six
//!     ancestors carry the spark in a running loop).
//!
//! Structures: four-uma loops (12 lineages) and three-uma rotations (3 lineages, the trainee her own grandparent twice;
//! `lins`). Metric: P(all targets land on the veteran), averaged over the structure's lineages, so both sizes rank
//! together. Schedules are affinity-optimised: in both structures every pair's shared G1s weigh the same.
use super::{
    Choice, Data, Deserialize, NT, Sched, chart_race_name, interchangeable, option_chance, row_label, short_name,
    streak_penalty, when,
};
use crate::{UmaIndex, UmaTable};
use serde_json::{Value, json};

/// Chance a skill learned in a career becomes a spark on the veteran: 20% for a white, 40% for a gold, x1.1 for each of
/// the six ancestors already carrying that spark (assumed; docs/RESEARCH.md section 1).
/// In a running loop all six carry every target.
pub(crate) const GEN_WHITE: f64 = 0.2 * 1.771_561;
pub(crate) const GEN_GOLD: f64 = 0.4 * 1.771_561;
/// Grandparents' base rate relative to parents'; docs/RESEARCH.md section 3, `3.grandparent_weight_fitted`.
const GP_RATE: f64 = 1.0;
/// Inspirations per career (both inheritance models: a spark lands when either inspiration fires it, so a miss is one
/// inspiration's miss squared)
const INSPIRATIONS: i32 = 2;
/// One inspiration's base chance per spark star (the career planner's `insp_miss`; the loop planner's `Model::base`
/// writes it as the literals 0.03, 0.06 and 0.09, since 3 x fl(0.03) is not fl(0.09))
pub(crate) const INSP_PER_STAR: f64 = 0.03;
/// One legacy slot's chance to pass a spark on at one inspiration in the loop planner's model: its base rate (`Model::base`,
/// x GP_RATE for a grandparent) x (1 + affinity / 100), at most 1. The one per-slot term of `lineage_q_arr`,
/// `q_ub_trainee` and `detail`
#[inline(always)]
fn slot_proc(rate: f64, aff: f64) -> f64 {
    (rate * (1.0 + aff / 100.0)).min(1.0)
}
/// A target's landing chance with deck odds dw (white hint) and dg (gold), the trainee's own event hint chance e and the
/// inheritance chance q, by its coverage v in her outfit (2: the gold skill, 1: the white one, 0: neither). The one
/// definition behind `skill_probs`, `skill_probs_d` and `outfit_fast`; `OutfitPre` is its branchless form, tested to the
/// bit.
#[inline(always)]
pub(crate) fn land_dw(v: u8, dw: f64, dg: f64, e: f64, q: f64) -> f64 {
    if v == 2 {
        return GEN_GOLD;
    }
    let white = if v == 1 { GEN_WHITE } else { GEN_WHITE * (1.0 - (1.0 - dw) * (1.0 - e) * (1.0 - q)) };
    dg * GEN_GOLD + (1.0 - dg) * white
}

#[derive(Deserialize, Clone)]
pub(crate) struct CardRaw(pub u32, pub String, pub Vec<(u32, u8)>);
#[derive(Deserialize)]
pub(crate) struct SkillsRaw {
    pub cards: std::collections::HashMap<String, Vec<CardRaw>>,
    pub skills: Vec<(u32, String, Option<String>)>,
}

#[derive(Deserialize, Clone)]
pub(crate) struct Target {
    pub group: u32,
    /// 0 = none, 1 = white guaranteed, 2 = gold guaranteed
    #[serde(default)]
    pub guaranteed: u8,
    #[serde(default)]
    pub deck_white: f64,
    #[serde(default)]
    pub deck_gold: f64,
}
#[derive(Deserialize)]
pub(crate) struct SkillJob {
    pub targets: Vec<Target>,
    /// Assumed 1/2/3-star shares of the lineage's sparks; defaults to the below-17,500-rank mix (`dstar`).
    /// The caller supplies the higher-rank mix when needed (docs/RESEARCH.md section 2).
    #[serde(default = "dstar")]
    pub star_dist: [f64; 3],
    #[serde(default)]
    pub owned: Vec<u32>,
    #[serde(default)]
    pub exclude: Vec<u32>,
    #[serde(default)]
    pub require: Vec<u32>,
    /// outfits (card ids) not owned; an uma whose every outfit is listed counts as not owned
    #[serde(default)]
    pub exclude_outfits: Vec<u32>,
    /// undocumented guarantees: (chara id, skill group, 1 white | 2 gold), applied to every outfit
    #[serde(default)]
    pub extra: Vec<(u32, u32, u8)>,
    /// trainee events that hint a target: (chara id for every outfit, or outfit card id; skill group; chance per
    /// career). The hinted white counts like a deck white hint for that trainee only.
    #[serde(default)]
    pub trainee_hints: Vec<(u32, u32, f64)>,
    #[serde(default = "diters")]
    pub iters: usize,
    #[serde(default = "dseed")]
    pub seed: u64,
    /// structure sizes searched: 3 (three-uma rotations) and/or 4 (four-uma loops), ranked together on the mean over
    /// their lineages
    #[serde(default = "dsizes")]
    pub sizes: Vec<usize>,
}
const fn dstar() -> [f64; 3] {
    [0.50, 0.45, 0.05]
}
const fn diters() -> usize {
    30
}
const fn dseed() -> u64 {
    3
}
fn dsizes() -> Vec<usize> {
    vec![3, 4]
}

/// A lineage over member positions (indices into a structure's members): the trainee T, her parents P1 and P2, and X,
/// P1 bred from (P2, X), P2 from (P1, X).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) struct MemberLineage {
    /// T: the member who trains.
    pub(crate) trainee: usize,
    /// P1: a parent, bred from P2 and X.
    pub(crate) first_parent: usize,
    /// P2: the other parent, bred from P1 and X.
    pub(crate) second_parent: usize,
    /// X: the grandparent under both parents (in a rotation, the trainee's own earlier veteran).
    pub(crate) shared_grandparent: usize,
}
/// The four-member loop's 12 lineages: every trainee T and fourth member X (checked by `rotation_lineage_tables`).
const LIN4: [MemberLineage; 12] = [
    MemberLineage { trainee: 0, first_parent: 2, second_parent: 3, shared_grandparent: 1 },
    MemberLineage { trainee: 0, first_parent: 1, second_parent: 3, shared_grandparent: 2 },
    MemberLineage { trainee: 0, first_parent: 1, second_parent: 2, shared_grandparent: 3 },
    MemberLineage { trainee: 1, first_parent: 2, second_parent: 3, shared_grandparent: 0 },
    MemberLineage { trainee: 1, first_parent: 0, second_parent: 3, shared_grandparent: 2 },
    MemberLineage { trainee: 1, first_parent: 0, second_parent: 2, shared_grandparent: 3 },
    MemberLineage { trainee: 2, first_parent: 1, second_parent: 3, shared_grandparent: 0 },
    MemberLineage { trainee: 2, first_parent: 0, second_parent: 3, shared_grandparent: 1 },
    MemberLineage { trainee: 2, first_parent: 0, second_parent: 1, shared_grandparent: 3 },
    MemberLineage { trainee: 3, first_parent: 1, second_parent: 2, shared_grandparent: 0 },
    MemberLineage { trainee: 3, first_parent: 0, second_parent: 2, shared_grandparent: 1 },
    MemberLineage { trainee: 3, first_parent: 0, second_parent: 1, shared_grandparent: 2 },
];
/// The three-member rotation's 3 lineages: each member trains from the other two, whose own parents are the other
/// two of the three, so X is the trainee's own character (her own earlier veteran, with her schedule's wins).
const LIN3: [MemberLineage; 3] = [
    MemberLineage { trainee: 0, first_parent: 1, second_parent: 2, shared_grandparent: 0 },
    MemberLineage { trainee: 1, first_parent: 0, second_parent: 2, shared_grandparent: 1 },
    MemberLineage { trainee: 2, first_parent: 0, second_parent: 1, shared_grandparent: 2 },
];
/// The lineages of a structure of `n` members (3: rotation, 4: the 12-lineage loop). The metric is the mean of P(all)
/// over them: every member trains equally often, each from the veterans the others produced.
///
/// Race weights (derived): with o(a, b) a pair's expected shared G1 wins, a lineage (T, P1, P2, X) counts o(P1, P2) six
/// times (3 per win in each parent's affinity for the other parent and for her own parent, and in the grandparent
/// slot under each parent) and o(P1, X), o(P2, X) twice each (parent and grandparent slot). Summed over the 12 loop
/// lineages every pair gets 6·2 + 2·4 = 20; over the 3 rotation lineages (X = T) every pair gets 6 + 2 + 2 = 10. Both
/// are uniform, so a structure's schedules maximise the plain sum of its pairs' shared wins either way.
pub(crate) fn lins(n: usize) -> &'static [MemberLineage] {
    match n {
        4 => &LIN4,
        3 => &LIN3,
        _ => panic!("a structure has 3 or 4 members"),
    }
}

/// Outfits of one uma that are interchangeable: the same guaranteed coverage and own-event chances.
pub(crate) struct OutfitOption {
    /// The hint level each outfit guarantees per uncertain target (`Model::uncertain` order): 0, 1 or 2.
    pub(crate) coverage: Vec<u8>,
    /// (outfit card id, label) of each outfit giving it, as the skills file lists them (id 0: "any outfit").
    pub(crate) outfits: Vec<(u32, String)>,
}

pub(crate) struct Model<'data> {
    d: &'data Data,
    pub(crate) uncertain: Vec<usize>, // indices into job.targets that are not guaranteed gold
    pub(crate) n_fixed_gold: i32,
    base: f64,
    /// per uma index: her distinct outfit options, in first-seen order
    pub(crate) opts: UmaTable<Vec<OutfitOption>>,
    /// per uma index: every listed outfit is excluded
    pub(crate) no_outfit: UmaTable<bool>,
    job: &'data SkillJob,
    /// coverage vectors without labels, for the allocation-free hot path
    sts: UmaTable<Vec<Vec<u8>>>,
    /// per uma index, per option: chance the trainee's own events hint each uncertain target's white
    pub(crate) ews: UmaTable<Vec<Vec<f64>>>,
}

impl<'data> Model<'data> {
    pub(crate) fn new(d: &'data Data, sk: &SkillsRaw, job: &'data SkillJob) -> Self {
        let uncertain: Vec<usize> = (0..job.targets.len()).filter(|&i| job.targets[i].guaranteed != 2).collect();
        let n_fixed_gold = (job.targets.len() - uncertain.len()) as i32;
        // (INSP_PER_STAR at 1, 2 and 3 stars)
        let base = job.star_dist[0] * 0.03 + job.star_dist[1] * 0.06 + job.star_dist[2] * 0.09;
        let mut opts = vec![];
        let mut no_outfit = vec![];
        let mut ews: Vec<Vec<Vec<f64>>> = vec![];
        for uma in &d.umas {
            let mut cards: Vec<CardRaw> = sk.cards.get(&uma.id.to_string()).cloned().unwrap_or_default();
            let had_outfits = !cards.is_empty();
            cards.retain(|card| !job.exclude_outfits.contains(&card.0));
            no_outfit.push(had_outfits && cards.is_empty());
            if cards.is_empty() {
                cards.push(CardRaw(0, "any outfit".into(), vec![]));
            }
            let mut options: Vec<OutfitOption> = vec![];
            let mut event_chances: Vec<Vec<f64>> = vec![];
            for card in &cards {
                let hint_chances: Vec<f64> = uncertain
                    .iter()
                    .map(|&i| {
                        let group = job.targets[i].group;
                        1.0 - job
                            .trainee_hints
                            .iter()
                            .filter(|hint| (hint.0 == uma.id || (card.0 != 0 && hint.0 == card.0)) && hint.1 == group)
                            .map(|hint| 1.0 - hint.2)
                            .product::<f64>()
                    })
                    .collect();
                let coverage: Vec<u8> = uncertain
                    .iter()
                    .map(|&i| {
                        let group = job.targets[i].group;
                        let mut level = card
                            .2
                            .iter()
                            .filter(|(candidate_group, _)| *candidate_group == group)
                            .map(|(_, hint_level)| *hint_level)
                            .max()
                            .unwrap_or(0);
                        for (character, candidate_group, hint_level) in &job.extra {
                            if *character == uma.id && *candidate_group == group {
                                level = level.max(*hint_level);
                            }
                        }
                        if job.targets[i].guaranteed == 1 {
                            level = level.max(1);
                        }
                        level
                    })
                    .collect();
                // Outfits are interchangeable only when both guaranteed coverage and own-event chances match.
                // Keep their first-seen option order, which decides ties in the outfit scorers.
                if let Some(i) =
                    (0..options.len()).find(|&i| options[i].coverage == coverage && event_chances[i] == hint_chances)
                {
                    options[i].outfits.push((card.0, card.1.clone()));
                } else {
                    options.push(OutfitOption { coverage, outfits: vec![(card.0, card.1.clone())] });
                    event_chances.push(hint_chances);
                }
            }
            opts.push(options);
            ews.push(event_chances);
        }
        let sts = opts.iter().map(|options| options.iter().map(|option| option.coverage.clone()).collect()).collect();
        Model {
            d,
            uncertain,
            n_fixed_gold,
            base,
            opts: UmaTable(opts),
            no_outfit: UmaTable(no_outfit),
            job,
            sts,
            ews: UmaTable(ews),
        }
    }

    pub(crate) fn skill_probs(&self, st: &[u8], ew: &[f64], q: f64) -> Vec<f64> {
        st.iter()
            .zip(&self.uncertain)
            .enumerate()
            .map(|(k, (&v, &i))| {
                let t = &self.job.targets[i];
                land_dw(v, t.deck_white, t.deck_gold, ew[k], q)
            })
            .collect()
    }
    /// best outfit for trainee `u` given per-career proc q: (product over uncertain skills, option index)
    fn trainee(&self, u: UmaIndex, q: f64) -> (f64, usize) {
        let mut best = (-1.0, 0);
        for (k, OutfitOption { coverage: st, .. }) in self.opts[u].iter().enumerate() {
            let v: f64 = self.skill_probs(st, &self.ews[u][k], q).iter().product();
            if v > best.0 {
                best = (v, k);
            }
        }
        best
    }
    fn fixed(&self) -> f64 {
        GEN_GOLD.powi(self.n_fixed_gold)
    }

    /// per-lineage proc q from slot affinities computed with overlap function `o(a, b)` (member positions), for the
    /// lineages of `lins(f.len())`. In a rotation X = T: tri(T, P, T) = 0 (`rel_tables` counts a group only for distinct
    /// characters; docs/RESEARCH.md section 5, `5.trials_attributable_to_one_ancestor`).
    /// o(P, T) is the overlap with the trainee's own schedule.
    pub(crate) fn lineage_q(&self, f: &[UmaIndex], o: &dyn Fn(usize, usize) -> f64) -> Vec<f64> {
        self.lineage_q_arr(f, o)[..lins(f.len()).len()].to_vec()
    }
    /// `lineage_q` into a fixed array (no allocation; `lineage_q` is its prefix).
    /// (Lineages of `lins(f.len())`; a rotation fills the first 3 entries.)
    pub(crate) fn lineage_q_arr(&self, f: &[UmaIndex], o: &dyn Fn(usize, usize) -> f64) -> [f64; 12] {
        let d = self.d;
        let mut out = [0.0f64; 12];
        for (l, lineage) in lins(f.len()).iter().enumerate() {
            let MemberLineage { trainee: t, first_parent: p1, second_parent: p2, shared_grandparent: x } = *lineage;
            let mut miss = 1.0f64;
            for (par, other) in [(p1, p2), (p2, p1)] {
                let ap = f64::from(d.pair[f[t]][f[par]])
                    + f64::from(d.pair[f[p1]][f[p2]])
                    + 3.0 * o(par, other)
                    + f64::from(d.tri[f[t]][f[par]][f[other]])
                    + 3.0 * o(par, other)
                    + f64::from(d.tri[f[t]][f[par]][f[x]])
                    + 3.0 * o(par, x);
                miss *= 1.0 - slot_proc(self.base, ap);
                for g in [other, x] {
                    let ag = f64::from(d.tri[f[t]][f[par]][f[g]]) + 3.0 * o(par, g);
                    miss *= 1.0 - slot_proc(GP_RATE * self.base, ag);
                }
            }
            out[l] = 1.0 - miss.powi(INSPIRATIONS);
        }
        out
    }
    /// Upper bound on the inheritance chance q of every lineage whose trainee is `t`, with parents and the fourth
    /// member drawn from `pool` and race overlaps bounded by `pb`. `lineage_q_arr`'s slots with o = pb, except that the
    /// fourth member X's terms take their best value over every possible fourth member, xm = max_X (tri(T, P, X) +
    /// 3 pb(P, X)), in both slots that read them. q rises with every affinity, so this is >= every such q in exact
    /// arithmetic. Not in the same order: a parent slot's affinity here is S + xm (S the terms before X's), where the
    /// lineage adds (S + tri) + 3 o, so the two can round apart by an ulp, and this can sit that much below a lineage's
    /// q. Harmless where it is read (`multi.rs`: the q grid's top and each uma's ceiling cell), as the tests against
    /// those ceilings carry relative slacks of 1e-12 or more. With `rot` the X slot may also be the trainee's own
    /// character (a three-member rotation: tri(T, P, T) + 3·o(P, T), the trio 0).
    pub(crate) fn q_ub_trainee(&self, t: UmaIndex, pool: &[UmaIndex], pb: &UmaTable<UmaTable<f64>>, rot: bool) -> f64 {
        let d = self.d;
        let others: Vec<UmaIndex> = pool.iter().copied().filter(|&u| u != t).collect();
        let xmax: Vec<f64> = others
            .iter()
            .map(|&par| {
                let m = others
                    .iter()
                    .filter(|&&x| x != par)
                    .map(|&x| f64::from(d.tri[t][par][x]) + 3.0 * pb[par][x])
                    .fold(f64::NEG_INFINITY, f64::max);
                if rot { m.max(f64::from(d.tri[t][par][t]) + 3.0 * pb[par][t]) } else { m }
            })
            .collect();
        let mut best = 0.0f64;
        for i in 0..others.len() {
            for j in i + 1..others.len() {
                let (p1, p2) = (others[i], others[j]);
                let mut miss = 1.0f64;
                for (par, other, xm) in [(p1, p2, xmax[i]), (p2, p1, xmax[j])] {
                    // the fourth member's terms take their best value over every fourth member, in both slots
                    let ap = f64::from(d.pair[t][par])
                        + f64::from(d.pair[p1][p2])
                        + 3.0 * pb[par][other]
                        + f64::from(d.tri[t][par][other])
                        + 3.0 * pb[par][other]
                        + xm;
                    miss *= 1.0 - slot_proc(self.base, ap);
                    let ag = f64::from(d.tri[t][par][other]) + 3.0 * pb[par][other];
                    miss *= 1.0 - slot_proc(GP_RATE * self.base, ag);
                    miss *= 1.0 - slot_proc(GP_RATE * self.base, xm);
                }
                best = best.max(1.0 - miss.powi(INSPIRATIONS));
            }
        }
        best
    }
    /// Lower bound on every lineage's q: all affinities at zero (relation points and race overlaps are >= 0).
    pub(crate) fn q_floor(&self) -> f64 {
        let miss = ((1.0 - self.base) * (1.0 - GP_RATE * self.base).powi(2)).powi(2);
        1.0 - miss.powi(INSPIRATIONS)
    }
    /// Best-outfit product over the uncertain targets for trainee `u` at proc `q` (the per-lineage term of
    /// the structure's metric, before the fixed-gold factor); increasing in q and in every deck odd.
    #[inline]
    pub(crate) fn trainee_fast(&self, u: UmaIndex, q: f64, dw: &[f64], dg: &[f64]) -> f64 {
        let mut best = 0.0f64;
        for o in 0..self.sts[u].len() {
            let p = self.outfit_fast(u, o, q, dw, dg);
            if p > best {
                best = p;
            }
        }
        best
    }
    pub(crate) fn fixed_factor(&self) -> f64 {
        self.fixed()
    }
    /// `trainee_fast` for one outfit option `o` of trainee `u`.
    #[inline]
    pub(crate) fn outfit_fast(&self, u: UmaIndex, o: usize, q: f64, dw: &[f64], dg: &[f64]) -> f64 {
        let (st, ew) = (&self.sts[u][o], &self.ews[u][o]);
        let mut p = 1.0f64;
        for k in 0..st.len() {
            p *= land_dw(st[k], dw[k], dg[k], ew[k], q);
        }
        p
    }
    /// `outfit_fast` for many calls on one outfit: its inputs resolved and prepared once (`OutfitPre`)
    pub(crate) fn outfit_pre(&self, u: UmaIndex, o: usize) -> OutfitPre {
        OutfitPre::new(&self.sts[u][o], &self.ews[u][o])
    }
    pub(crate) fn n_outfits(&self, u: UmaIndex) -> usize {
        self.sts[u].len()
    }
    /// Per-skill landing odds with deck hint odds supplied per uncertain target (white, gold) instead of the job's.
    pub(crate) fn skill_probs_d(st: &[u8], ew: &[f64], q: f64, dw: &[f64], dg: &[f64]) -> Vec<f64> {
        st.iter().enumerate().map(|(k, &v)| land_dw(v, dw[k], dg[k], ew[k], q)).collect()
    }
    pub(crate) fn trainee_d(&self, u: UmaIndex, q: f64, dw: &[f64], dg: &[f64]) -> (f64, usize) {
        let mut best = (-1.0, 0);
        for (k, OutfitOption { coverage: st, .. }) in self.opts[u].iter().enumerate() {
            let v: f64 = Self::skill_probs_d(st, &self.ews[u][k], q, dw, dg).iter().product();
            if v > best.0 {
                best = (v, k);
            }
        }
        best
    }
    /// Everything behind the metric for one loop, for the "how this loop works" view: pairwise relation points and
    /// expected shared G1 wins, then per lineage the six legacy slots (relation part, race part, affinity, proc),
    /// the per-career inheritance chance q, each target's landing chance and P(all land).
    /// `deck`: one (white odds, gold odds) pair of slices per member in `f` order. Each slice is indexed by
    /// `self.uncertain`, not by every job target. None uses the job's own deck fields.
    /// `f`: the members (3 or 4); lineages as in `lins(f.len())`, a rotation's X being the trainee.
    pub(crate) fn detail(
        &self,
        f: &[UmaIndex],
        o: &dyn Fn(usize, usize) -> f64,
        deck: Option<&[(&[f64], &[f64])]>,
    ) -> Value {
        let d = self.d;
        let pr = |a: usize, b: usize| f64::from(d.pair[f[a]][f[b]]);
        let tr = |a: usize, b: usize, c: usize| f64::from(d.tri[f[a]][f[b]][f[c]]);
        let mut pairs = vec![];
        for a in 0..f.len() {
            for b in a + 1..f.len() {
                pairs.push(json!({"a": a, "b": b, "rel": pr(a, b), "wins": o(a, b)}));
            }
        }
        let mut lout = vec![];
        for &MemberLineage { trainee: t, first_parent: p1, second_parent: p2, shared_grandparent: x } in lins(f.len()) {
            let mut slots = vec![];
            let mut miss = 1.0f64;
            for (par, other) in [(p1, p2), (p2, p1)] {
                let rel = pr(t, par) + pr(p1, p2) + tr(t, par, other) + tr(t, par, x);
                let race = 3.0 * o(par, other) + 3.0 * o(par, other) + 3.0 * o(par, x);
                let aff = rel + race;
                let proc = slot_proc(self.base, aff);
                miss *= 1.0 - proc;
                slots.push(json!({"kind": "parent", "who": [par], "rel": rel, "race": race, "aff": aff, "proc": proc}));
                for g in [other, x] {
                    let rel = tr(t, par, g);
                    let race = 3.0 * o(par, g);
                    let aff = rel + race;
                    let proc = slot_proc(GP_RATE * self.base, aff);
                    miss *= 1.0 - proc;
                    slots.push(json!({"kind": "grandparent", "who": [par, g], "rel": rel, "race": race, "aff": aff, "proc": proc}));
                }
            }
            let q = 1.0 - miss.powi(INSPIRATIONS);
            let (pv, k) = match deck.map(|dk| dk[t]) {
                Some((w, g)) => self.trainee_d(f[t], q, w, g),
                None => self.trainee(f[t], q),
            };
            let probs = match deck.map(|dk| dk[t]) {
                Some((w, g)) => Self::skill_probs_d(&self.opts[f[t]][k].coverage, &self.ews[f[t]][k], q, w, g),
                None => self.skill_probs(&self.opts[f[t]][k].coverage, &self.ews[f[t]][k], q),
            };
            let skills: Vec<Value> = self
                .uncertain
                .iter()
                .zip(&probs)
                .zip(&self.opts[f[t]][k].coverage)
                .map(|((&i, p), lv)| json!({"group": self.job.targets[i].group, "p": p, "outfit": lv}))
                .collect();
            lout.push(json!({"t": t, "p": [p1, p2], "x": x, "slots": slots, "q": q, "p_all": self.fixed() * pv, "skills": skills,
                             "fixed_gold": self.n_fixed_gold}));
        }
        json!({"base": self.base, "gp_rate": GP_RATE, "inspirations": INSPIRATIONS, "gen_white": GEN_WHITE, "gen_gold": GEN_GOLD, "pairs": pairs, "lineages": lout})
    }
}

/// `Model::outfit_fast` for one outfit, its inputs prepared once for a caller that evaluates it many times (the deck
/// search's node bound): per target the masks of "white given" (st = 1) and "gold given" (st = 2), all ones or zero,
/// and 1 - ew (the same subtraction `outfit_fast` does per call, so the same f64). Every target's two arms are computed
/// and the state picks one by a bitwise select, which takes one of the two f64s bit for bit, as the branch does; so the
/// result is the same whatever the inputs (finite or not), and the factor loop has no branch and vectorizes.
pub(crate) struct OutfitPre {
    m1: Vec<u64>,
    m2: Vec<u64>,
    ne: Vec<f64>,
}
impl OutfitPre {
    pub(crate) fn new(st: &[u8], ew: &[f64]) -> Self {
        let mask = |b: bool| if b { u64::MAX } else { 0 };
        Self {
            m1: st.iter().map(|&v| mask(v == 1)).collect(),
            m2: st.iter().map(|&v| mask(v == 2)).collect(),
            ne: ew[..st.len()].iter().map(|&e| 1.0 - e).collect(),
        }
    }
    /// the same f64 as `outfit_fast` on this outfit: the same operations on the same values ((1 - q) once instead of per
    /// target: the same f64), the factors into an array, then multiplied one by one in target order
    #[inline]
    pub(crate) fn eval(&self, q: f64, dw: &[f64], dg: &[f64]) -> f64 {
        let factor = Self::factor;
        const C: usize = 32;
        let n = self.ne.len();
        let (m1, m2, ne, dw, dg) = (&self.m1[..n], &self.m2[..n], &self.ne[..n], &dw[..n], &dg[..n]);
        let omq = 1.0 - q;
        let mut p = 1.0f64;
        if n > C {
            for k in 0..n {
                p *= factor(m1[k], m2[k], ne[k], omq, dw[k], dg[k]);
            }
            return p;
        }
        let mut f = [0.0f64; C];
        for k in 0..n {
            f[k] = factor(m1[k], m2[k], ne[k], omq, dw[k], dg[k]);
        }
        for &x in &f[..n] {
            p *= x;
        }
        p
    }
    /// how many targets `eval` reads
    #[inline]
    pub(crate) const fn len(&self) -> usize {
        self.ne.len()
    }
    /// `eval`'s factors one by one: out[k] for k < len(), the same f64s as `eval` multiplies (in target order) for these
    /// inputs
    #[inline]
    pub(crate) fn factors(&self, q: f64, dw: &[f64], dg: &[f64], out: &mut [f64]) {
        let n = self.ne.len();
        let (m1, m2, ne, dw, dg, out) =
            (&self.m1[..n], &self.m2[..n], &self.ne[..n], &dw[..n], &dg[..n], &mut out[..n]);
        let omq = 1.0 - q;
        for k in 0..n {
            out[k] = Self::factor(m1[k], m2[k], ne[k], omq, dw[k], dg[k]);
        }
    }
    /// `eval`'s factor of target k (k < len()) at odds w, g for that target
    #[inline]
    pub(crate) fn factor_at(&self, k: usize, q: f64, w: f64, g: f64) -> f64 {
        Self::factor(self.m1[k], self.m2[k], self.ne[k], 1.0 - q, w, g)
    }
    /// the product of `eval`'s factors of the targets in `m` (bit k: target k < min(len(), 32)), in target order
    #[inline]
    pub(crate) fn eval_in(&self, q: f64, dw: &[f64], dg: &[f64], m: u32) -> f64 {
        let omq = 1.0 - q;
        let (mut m, mut p) = (m, 1.0f64);
        while m != 0 {
            let k = m.trailing_zeros() as usize;
            m &= m - 1;
            p *= Self::factor(self.m1[k], self.m2[k], self.ne[k], omq, dw[k], dg[k]);
        }
        p
    }
    #[inline(always)]
    fn factor(m1: u64, m2: u64, ne: f64, omq: f64, w: f64, g: f64) -> f64 {
        #[inline(always)]
        const fn sel(a: f64, b: f64, m: u64) -> f64 {
            f64::from_bits((a.to_bits() & !m) | (b.to_bits() & m))
        }
        let hinted = GEN_WHITE * (1.0 - (1.0 - w) * ne * omq);
        let white = sel(hinted, GEN_WHITE, m1);
        sel(g * GEN_GOLD + (1.0 - g) * white, GEN_GOLD, m2)
    }
}
/// Structured schedule chart: shared G1s (affinity) and optional G1s (one runner, race sparks only).
pub(crate) fn chart_rows(d: &Data, members: &[UmaIndex], schedules: &[Sched]) -> Value {
    let n = members.len();
    let streaks: Vec<Vec<usize>> = (0..n)
        .map(|i| {
            let mut streak = 0;
            (0..NT)
                .map(|turn| {
                    match schedules[i][turn] {
                        Choice::Rest => streak = 0,
                        Choice::Opt(_) => streak = (streak + 1).min(6),
                    };
                    streak
                })
                .collect()
        })
        .collect();
    let mut runners: Vec<Vec<bool>> = vec![vec![false; n]; d.nr];
    for i in 0..n {
        for turn in 0..NT {
            if let Choice::Opt(option) = schedules[i][turn] {
                let chosen = &d.umas[members[i]].opts[turn][option as usize];
                if chosen.race >= 0 {
                    runners[chosen.race as usize][i] = true;
                }
            }
        }
    }
    let mut rows: Vec<(usize, i32)> = vec![];
    for turn in 0..NT {
        for i in 0..n {
            if let Choice::Opt(option) = schedules[i][turn] {
                let chosen = &d.umas[members[i]].opts[turn][option as usize];
                if chosen.race >= 0 && !rows.contains(&(turn, chosen.race)) {
                    rows.push((turn, chosen.race));
                }
            }
        }
    }
    rows.sort_unstable();
    let out: Vec<Value> = rows
        .iter()
        .map(|&(turn, race)| {
            let mut runner_count = 0;
            let cells: Vec<Value> = (0..n)
                .map(|i| match schedules[i][turn] {
                    Choice::Opt(option) => {
                        let chosen = &d.umas[members[i]].opts[turn][option as usize];
                        if chosen.race == race {
                            runner_count += 1;
                            let streak = streaks[i][turn];
                            // Win chance accounts for aptitude grades (distance + surface, out of 110) and consecutive races.
                            json!({"p": (option_chance(d, chosen, streak) * 100.0).round(), "goal": chosen.goal,
                           "apt": 110 - chosen.bp, "row": streak,
                           "pen": streak_penalty(d, chosen.goal, streak)})
                        } else {
                            Value::Null
                        }
                    }
                    Choice::Rest => Value::Null,
                })
                .collect();
            let shared = runners[race as usize].iter().filter(|x| **x).count() >= 2;
            let alts: Vec<String> = interchangeable(d, members, schedules, turn, race)
                .iter()
                .map(|&alternative| chart_race_name(&d.race_names[alternative as usize]))
                .collect();
            json!({"race": row_label(d, members, schedules, turn, race), "when": when(turn), "year": turn / 24,
               "shared": shared, "cells": cells, "runners": runner_count, "alts": alts})
        })
        .collect();
    Value::Array(out)
}

pub(crate) fn member_json(d: &Data, m: &[UmaIndex]) -> Vec<Value> {
    m.iter()
        .map(|&x| json!({"id": d.umas[x].id, "name": d.umas[x].name, "short": short_name(&d.umas[x].name)}))
        .collect()
}

/// Checks a job's `sizes` and `require` against the pool: an error message, or the smallest size searched.
pub(crate) fn check_sizes(sizes: &[usize], pool: usize, req: usize) -> Result<usize, String> {
    if sizes.is_empty() || sizes.iter().any(|&s| s != 3 && s != 4) {
        return Err("sizes: list 3 and/or 4".into());
    }
    let lo = *sizes.iter().min().unwrap();
    let hi = *sizes.iter().max().unwrap();
    if req > hi {
        return Err(format!("At most {hi} umas can be required."));
    }
    if pool < lo {
        return Err(format!("Pick at least {lo} umas to search."));
    }
    Ok(lo)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{load, rel_tables};
    use std::sync::OnceLock;
    fn data() -> &'static Data {
        static D: OnceLock<Data> = OnceLock::new();
        D.get_or_init(|| load(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/data_v1.json")))
    }
    fn skills() -> SkillsRaw {
        serde_json::from_str(
            &std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/data/global/web_skills.json")).unwrap(),
        )
        .unwrap()
    }

    /// `OutfitPre::eval` is `outfit_fast`'s loop bit for bit: random states, odds and lengths (past the 32-lane array
    /// too), plus non-finite and signed-zero inputs, which the branchy loop ignores on gold targets
    #[test]
    fn outfit_pre_matches_outfit_fast() {
        // (`outfit_fast`'s loop)
        let reference = |st: &[u8], ew: &[f64], q: f64, dw: &[f64], dg: &[f64]| -> f64 {
            let mut p = 1.0f64;
            for k in 0..st.len() {
                p *= land_dw(st[k], dw[k], dg[k], ew[k], q);
            }
            p
        };
        let mut s = 0x9e37_79b9_7f4a_7c15_u64;
        let mut rnd = || {
            s ^= s << 13;
            s ^= s >> 7;
            s ^= s << 17;
            (s >> 11) as f64 / (1u64 << 53) as f64
        };
        let odd = [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0, 0.0, 1.0, 1e-300];
        for it in 0..20000 {
            let n = (rnd() * 40.0) as usize;
            let st: Vec<u8> = (0..n).map(|_| (rnd() * 3.0) as u8).collect();
            let mut f = |i: usize| if it % 7 == 0 && rnd() < 0.2 { odd[i % odd.len()] } else { rnd() };
            let ew: Vec<f64> = (0..n).map(&mut f).collect();
            let dw: Vec<f64> = (0..n).map(|i| f(i + 1)).collect();
            let dg: Vec<f64> = (0..n).map(|i| f(i + 2)).collect();
            let q = if it % 11 == 0 { odd[it % odd.len()] } else { rnd() };
            let (a, b) = (reference(&st, &ew, q, &dw, &dg), OutfitPre::new(&st, &ew).eval(q, &dw, &dg));
            assert!(
                a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()),
                "{st:?} {ew:?} {q} {dw:?} {dg:?}: {a} vs {b}"
            );
        }
    }

    /// Excluding outfits drops them from an uma's options; excluding every outfit of an uma marks her unowned.
    #[test]
    fn outfit_exclusion() {
        let d = data();
        let sk = skills();
        let ix = |id: u32| UmaIndex(d.umas.iter().position(|u| u.id == id).unwrap());
        let outfits = |m: &Model, i: UmaIndex| -> Vec<u32> {
            m.opts[i].iter().flat_map(|o| o.outfits.iter().map(|c| c.0)).collect()
        };
        let job: SkillJob =
            serde_json::from_value(json!({"targets": [{"group":20159}], "exclude_outfits": [100_102]})).unwrap();
        let m = Model::new(d, &sk, &job);
        let sw = outfits(&m, ix(1001));
        assert!(!sw.contains(&100_102) && !sw.is_empty() && !m.no_outfit[ix(1001)], "{sw:?}");
        let job2: SkillJob =
            serde_json::from_value(json!({"targets": [{"group":20159}], "exclude_outfits": [101_901, 101_902]}))
                .unwrap();
        let m2 = Model::new(d, &sk, &job2);
        assert!(m2.no_outfit[ix(1019)]);
        assert!(!m2.no_outfit[ix(1001)]);
    }

    /// The lineage tables: the 12-lineage loop, and the three-member rotation (each member
    /// trains from the other two, whose own parents are the other two of the three: X is the trainee herself).
    #[test]
    fn rotation_lineage_tables() {
        // the 12 self-copy-free lineages: every trainee T and fourth member X, P1 bred from (P2, X), P2 from (P1, X)
        let mut lin4 = vec![];
        for t in 0..4 {
            for x in 0..4 {
                if x == t {
                    continue;
                }
                let ps: Vec<usize> = (0..4).filter(|&i| i != t && i != x).collect();
                lin4.push(MemberLineage {
                    trainee: t,
                    first_parent: ps[0],
                    second_parent: ps[1],
                    shared_grandparent: x,
                });
            }
        }
        assert_eq!(lins(4).to_vec(), lin4);
        let lin3: Vec<(usize, usize, usize, usize)> = lins(3)
            .iter()
            .map(|lineage| (lineage.trainee, lineage.first_parent, lineage.second_parent, lineage.shared_grandparent))
            .collect();
        assert_eq!(lin3, vec![(0, 1, 2, 0), (1, 0, 2, 1), (2, 0, 1, 2)]);
        for &MemberLineage { trainee: t, first_parent: p1, second_parent: p2, shared_grandparent: x } in lins(3) {
            assert_eq!(x, t);
            // B's parents are (C, A) and C's are (B, A): the other parent and the trainee
            assert!(p1 != t && p2 != t && p1 != p2);
        }
        // repeated characters: a group counts only for distinct members, so pair(a, a) = 0 and tri(T, P, T) = 0
        let [a, b, c] = [0, 1, 2].map(UmaIndex);
        let (p, t) = rel_tables(3, &[(5, vec![a, b]), (7, vec![a, b, c]), (11, vec![a, c])]);
        assert_eq!((p[a][b], p[a][c], p[b][c], p[a][a]), (12, 18, 7, 0));
        assert_eq!((t[a][b][c], t[c][b][a], t[a][b][a], t[a][b][b], t[a][a][b]), (7, 7, 0, 0, 0));
        let d = data();
        for a in d.umas.indices() {
            for b in d.umas.indices() {
                assert_eq!(d.tri[a][b][a], 0);
                assert_eq!(d.tri[a][b][b], 0);
                assert_eq!(d.tri[a][a][b], 0);
            }
        }
    }

    /// A three-member rotation's slot affinities, written out by hand from the per-legacy formula, against `detail`;
    /// and the race weights: summed over a structure's lineages each pair's shared wins count 3 × 10 (rotation) or
    /// 3 × 20 (loop) affinity, uniform over the pairs.
    #[test]
    fn rotation_affinities_by_hand() {
        let d = data();
        let sk = skills();
        let job: SkillJob = serde_json::from_value(json!({"targets": [{"group":20159},{"group":20161}]})).unwrap();
        let m = Model::new(d, &sk, &job);
        // Agnes Digital, Tamamo Cross, Inari One and made-up shared wins per pair
        let f = [18, 20, 33].map(UmaIndex);
        let ov = |a: usize, b: usize| -> f64 {
            match (a.min(b), a.max(b)) {
                (0, 1) => 2.5,
                (0, 2) => 4.0,
                (1, 2) => 7.25,
                _ => panic!(),
            }
        };
        let det = m.detail(&f, &ov, None);
        let pr = |a: usize, b: usize| f64::from(d.pair[f[a]][f[b]]);
        let tr = |a: usize, b: usize, c: usize| f64::from(d.tri[f[a]][f[b]][f[c]]);
        let ls = det["lineages"].as_array().unwrap();
        assert_eq!(ls.len(), 3);
        for (l, &(t, b, c, _)) in [(0usize, 1usize, 2usize, 0usize), (1, 0, 2, 1), (2, 0, 1, 2)].iter().enumerate() {
            let slots = ls[l]["slots"].as_array().unwrap();
            let got: Vec<(f64, f64)> =
                slots.iter().map(|x| (x["rel"].as_f64().unwrap(), x["race"].as_f64().unwrap())).collect();
            // trainee T, parents B (bred from C and T) and C (bred from B and T)
            #[rustfmt::skip] // a table: each row's comment names its slot
            let want = [
                (pr(t, b) + pr(b, c) + tr(t, b, c), 3.0 * ov(b, c) + 3.0 * ov(b, c) + 3.0 * ov(b, t)),   // parent B
                (tr(t, b, c), 3.0 * ov(b, c)),                                                       // C under B
                (0.0, 3.0 * ov(b, t)),                                                               // T under B: trio 0
                (pr(t, c) + pr(b, c) + tr(t, c, b), 3.0 * ov(c, b) + 3.0 * ov(c, b) + 3.0 * ov(c, t)),   // parent C
                (tr(t, c, b), 3.0 * ov(c, b)),                                                       // B under C
                (0.0, 3.0 * ov(c, t)),                                                               // T under C
            ];
            for (g, w) in got.iter().zip(&want) {
                assert!((g.0 - w.0).abs() < 1e-12 && (g.1 - w.1).abs() < 1e-12, "lineage {l}: {g:?} vs {w:?}");
            }
            assert_eq!(slots[2]["who"], json!([b, t]));
            assert_eq!(slots[5]["who"], json!([c, t]));
            // the inheritance chance from those six affinities
            let mut miss = 1.0f64;
            for (k, w) in want.iter().enumerate() {
                let base = if k % 3 == 0 { m.base } else { GP_RATE * m.base };
                miss *= 1.0 - (base * (1.0 + (w.0 + w.1) / 100.0)).min(1.0);
            }
            assert!((ls[l]["q"].as_f64().unwrap() - (1.0 - miss * miss)).abs() < 1e-15);
        }
        // race weights: a unit of shared wins on one pair moves the summed affinity by 30 (rotation), 60 (loop)
        for f in [&[18usize, 20, 33][..], &[5, 18, 20, 33][..]] {
            let f: Vec<UmaIndex> = f.iter().copied().map(UmaIndex).collect();
            let n = f.len();
            let tot = |o: &dyn Fn(usize, usize) -> f64| -> f64 {
                m.detail(&f, o, None)["lineages"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .flat_map(|l| {
                        l["slots"].as_array().unwrap().iter().map(|x| x["race"].as_f64().unwrap()).collect::<Vec<_>>()
                    })
                    .sum()
            };
            for a in 0..n {
                for b in a + 1..n {
                    let one = tot(&|x: usize, y: usize| if (x.min(y), x.max(y)) == (a, b) { 1.0 } else { 0.0 });
                    assert_eq!(one, if n == 3 { 30.0 } else { 60.0 }, "pair {a}-{b} of {n}");
                }
            }
        }
    }
}
