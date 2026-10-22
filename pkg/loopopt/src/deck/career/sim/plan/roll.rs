//! The roll of the plan's pick and its kept state.
use super::{
    Cont, IDX_PARTNERS, PartnerAnchors, PlanJob, ScoreBase, Trio, fm_masks, h64, outcome_hi, sc_roll, sim_for,
};
use crate::deck::career::sim::plan::anchors::{FoundAnchor, OutcomeBounds, OutcomeEstimate};
use crate::deck::career::sim::plan::cont::OwnTrio;
use crate::deck::career::sim::plan::{CandidateIndex, OutcomeIndex, ScoringContext, TargetMask};
use crate::deck::career::sim::{Env, Legacy, PROF, Pool, Sel, Sim, Value, json, now_ms};
use crate::deck::career::{Career, JobInputs, Lineage};

// ---------- a roll of the plan's pick, on demand (`next.roll`) ----------
/// Candidate whose step may change at an outcome.
#[derive(Clone, Copy)]
pub(in crate::deck::career::sim::plan) struct ActiveCandidate {
    /// Position in the fixed planner candidate table.
    pub(in crate::deck::career::sim::plan) candidate: CandidateIndex,
    /// Slot in the prepared outcome table, if this candidate has one.
    pub(in crate::deck::career::sim::plan) slot: Option<usize>,
    /// Odds above which this candidate's step can improve.
    pub(in crate::deck::career::sim::plan) threshold: f64,
    /// Candidate's odds in the original pool.
    pub(in crate::deck::career::sim::plan) current_chance: f64,
}

/// One scoring's outcome evaluation as `sc_eval` reads it (built by `score_career_`; a plan keeps its pick's for a roll,
/// `RollKeep`). Per setup her miss factor's parts (x0 per uncertain target, xm per star level) and her spark scales (pw
/// per job target, without and with her spark), with what the fold and the outcome loop read of them (`derive`: xs, xc
/// and scf); the tables kept (`sc_prep`: level 0 of the fold, table j at j 2^nu) and each one's setup; the collapsed
/// targets; per setup its candidate slot and per slot the odds now; her landed sets' chances (`os`); the candidates the
/// loop can lift (`act`); her own 8/8's trio sides (`her`); her candidate index, the goal, the star mix
pub(in crate::deck::career::sim) struct Scored {
    pub(in crate::deck::career::sim::plan) nu: usize,
    pub(in crate::deck::career::sim::plan) nf: usize,
    pub(in crate::deck::career::sim::plan) ntar: usize,
    pub(in crate::deck::career::sim::plan) na: usize,
    pub(in crate::deck::career::sim::plan) xi: Option<usize>,
    pub(in crate::deck::career::sim::plan) goal: bool,
    pub(in crate::deck::career::sim::plan) w: [f64; 3],
    pub(in crate::deck::career::sim::plan) uncv: Vec<usize>,
    pub(in crate::deck::career::sim::plan) fmm: Vec<TargetMask>,
    pub(in crate::deck::career::sim::plan) x0: Vec<Vec<f64>>,
    pub(in crate::deck::career::sim::plan) xm: Vec<[f64; 4]>,
    pub(in crate::deck::career::sim::plan) pw: Vec<Vec<[f64; 2]>>,
    pub(in crate::deck::career::sim::plan) tsi: Vec<usize>,
    pub(in crate::deck::career::sim::plan) t0: Vec<f64>,
    pub(in crate::deck::career::sim::plan) col: Vec<bool>,
    pub(in crate::deck::career::sim::plan) sslot: Vec<usize>,
    pub(in crate::deck::career::sim::plan) p0: Vec<f64>,
    pub(in crate::deck::career::sim::plan) os: Vec<f64>,
    pub(in crate::deck::career::sim::plan) act: Vec<ActiveCandidate>,
    pub(in crate::deck::career::sim::plan) her: Vec<OwnTrio>,
    pub(in crate::deck::career::sim::plan) xs: Vec<Vec<[f64; 4]>>,
    pub(in crate::deck::career::sim::plan) xc: Vec<Vec<[f64; 4]>>,
    pub(in crate::deck::career::sim::plan) scf: Vec<f64>,
}

impl Scored {
    /// what the fold and the outcome loop read of the setups: per setup, target and star level x = x0 xm and 1 - x; per
    /// landed set m and setup the spark scale, the product over job targets of pw at her spark there (m-major:
    /// scf[m * ns + si])
    pub(in crate::deck::career::sim::plan) fn derive(&mut self) {
        let nu = self.nu;
        self.xs = self
            .x0
            .iter()
            .zip(&self.xm)
            .map(|(x0, xm)| (0..nu).map(|k| [0usize, 1, 2, 3].map(|sg| x0[k] * xm[sg])).collect())
            .collect();
        self.xc = self.xs.iter().map(|v| v.iter().map(|r| r.map(|x| 1.0 - x)).collect()).collect();
        let (nm, ntar) = (1usize << self.ntar, self.ntar);
        self.scf = Vec::with_capacity(nm * self.ns());
        for m in 0..nm {
            self.scf.extend(self.pw.iter().map(|pw| {
                (0..ntar).map(|k| pw[k][usize::from(TargetMask::from_bits(m).contains(k))]).product::<f64>()
            }));
        }
    }
    pub(in crate::deck::career::sim::plan) const fn ns(&self) -> usize {
        self.x0.len()
    }
    /// the fold writes per leaf the setups' table values (fewer rows than per outcome and slot), else per outcome the
    /// slots' odds (`fold_emit`)
    pub(in crate::deck::career::sim::plan) const fn pl(&self) -> bool {
        (1usize << self.nf) * self.na > self.ns()
    }
    pub(in crate::deck::career::sim::plan) const fn full_m(&self) -> TargetMask {
        TargetMask::from_bits((1usize << self.ntar) - 1)
    }
    /// a leaf's representative: every collapsed target's nonzero digit set to 1 (`sc_outcomes`)
    pub(in crate::deck::career::sim::plan) fn rep_of(&self, l: usize) -> usize {
        let cm: usize = (0..self.nu).filter(|&k| self.col[k]).map(|k| 3usize << (2 * k)).sum();
        let lo_bits = cm & (usize::MAX / 3);
        let z = l & cm;
        (l & !cm) | ((z | z >> 1) & lo_bits)
    }
}

/// The base a kept scoring was scored against (`ScoreBase` without the sides and C's umas, which only `her_trios` reads)
pub(in crate::deck::career::sim::plan) struct KeptBase {
    cands: Vec<usize>,
    c1: Vec<f64>,
    v0: f64,
    pnow: Vec<f64>,
    cont: Vec<Cont>,
    combos: Vec<Trio>,
    m23: Vec<f64>,
}

impl KeptBase {
    pub(in crate::deck::career::sim::plan) fn of(bs: &ScoreBase) -> Self {
        Self {
            cands: bs.cands.to_vec(),
            c1: bs.c1.to_vec(),
            v0: bs.v0,
            pnow: bs.pnow.to_vec(),
            cont: bs.cont.to_vec(),
            combos: bs.combos.to_vec(),
            m23: bs.m23.to_vec(),
        }
    }
    fn base(&self) -> ScoreBase<'_> {
        ScoreBase {
            cands: &self.cands,
            c1: &self.c1,
            m23: &self.m23,
            v0: self.v0,
            pnow: &self.pnow,
            cont: &self.cont,
            combos: &self.combos,
            sides: &[],
            all: &[],
        }
    }
}

/// A plan's roll state (`next.roll`), kept per plan job (`roll_key`): the pick's last scoring (`Scored`) and the base it
/// was scored against, the pick's career, and what its bound reads beyond them (`outcome_hi`): her own 8/8's trio sides as
/// the plan's end has them (`pick_her`) and the main search's found-career anchors (`found_anchors`)
pub(in crate::deck::career) struct RollKeep {
    pub(in crate::deck::career::sim::plan) key: u64,
    pub(in crate::deck::career::sim::plan) sd: Scored,
    pub(in crate::deck::career::sim::plan) base: KeptBase,
    pub(in crate::deck::career::sim::plan) x: usize,
    pub(in crate::deck::career::sim::plan) a: usize,
    pub(in crate::deck::career::sim::plan) b: usize,
    pub(in crate::deck::career::sim::plan) sel: Sel,
    pub(in crate::deck::career::sim::plan) her_hi: Vec<OwnTrio>,
    pub(in crate::deck::career::sim::plan) anc: Vec<Option<FoundAnchor>>,
    /// the bound's anchors per (candidate, partner), kept by its first roll for the next (`HiAnc`; not stored by `state_out`)
    pub(in crate::deck::career::sim::plan) hi_anc: Vec<PartnerAnchors>,
}

thread_local! { static ROLLS: std::cell::RefCell<Vec<RollKeep>> = const { std::cell::RefCell::new(Vec::new()) }; }

/// plans' roll states kept (the most recent last): the run's plan and the plan Compare makes with her added
const ROLL_KEEP: usize = 4;

/// a plan job's roll key: the whole job but its roll (`next.roll`) and the page's flags that change no plan
/// (`no_reports`, `compare`)
pub(in crate::deck::career::sim::plan) fn roll_key(job: &Value) -> u64 {
    h64(&crate::deck::career::json_without(job, &["no_reports", "compare", "roll"], &["roll"]))
}

pub(in crate::deck::career::sim::plan) fn keep_roll(roll: RollKeep) {
    ROLLS.with(|cache| {
        let mut cache = cache.borrow_mut();
        cache.retain(|entry| entry.key != roll.key);
        cache.push(roll);
        while cache.len() > ROLL_KEEP {
            cache.remove(0);
        }
    });
}

/// The plan's answer to a roll (`next.roll`: her stars per job target, 0 where it did not land) from its kept roll state,
/// with no search (None when the worker keeps none for this plan job: then the plan runs and answers it, `index_rule`).
/// y: the careers her veteran saves at that outcome, the pick's kept scoring evaluated at that one outcome (`sc_eval`,
/// the plan's own arithmetic in its order: the f64 the plan's full table held); where y < 0.5 the bound y_hi
/// (`outcome_hi` at that one cell). Rounded as the table was: y to 0.01, y_hi up to 0.01 (null where y >= 0.5)
pub(in crate::deck::career) fn roll_kept<'data>(
    env: &mut Env<'data>,
    input: &JobInputs<'data>,
    legs: &[Legacy],
    pool: &Pool,
    fix: &[usize],
    pj: &PlanJob,
) -> Option<Value> {
    let stars = pj.roll.as_ref()?;
    let key = roll_key(input.job);
    let mut kept = ROLLS.with(|cache| {
        let mut cache = cache.borrow_mut();
        let index = cache.iter().position(|roll| roll.key == key)?;
        Some(cache.remove(index))
    })?;
    // (the roll's own timers and counters, `prof`; what the job counted before goes back after)
    let started = now_ms();
    let prof_before = PROF.with(|prof| std::mem::take(&mut *prof.borrow_mut()));
    let answer = match sim_for(env, input, pj) {
        Ok(mut sim) => roll_answer(&mut sim, legs, pool, fix, &mut kept, stars),
        Err(error) => error,
    };
    keep_roll(kept);
    let mut answer = answer;
    let roll_prof = PROF.with(|prof| std::mem::replace(&mut *prof.borrow_mut(), prof_before));
    if answer.is_object() {
        answer["ms"] = json!(now_ms() - started);
        answer["prof"] = json!(roll_prof);
    }
    Some(answer)
}

fn roll_answer(s: &mut Sim, legs: &[Legacy], pool: &Pool, fix: &[usize], k: &mut RollKeep, stars: &[u8]) -> Value {
    let (uncv, fixed_t) = (s.env.uncv.clone(), s.env.fixed_t.clone());
    if stars.len() != s.env.ntar {
        return json!({"error": format!("roll: {} stars for {} targets", stars.len(), s.env.ntar)});
    }
    let outcome = OutcomeIndex::from_stars(stars, &uncv, &fixed_t);
    let mut ac = std::mem::take(&mut k.hi_anc);
    let bs = k.base.base();
    let y = sc_roll(&bs, &k.sd, outcome);
    let mut hi = None;
    if y < 0.5 {
        let context = ScoringContext { score_base: &bs, legacies: legs, pool };
        let career = Career { lineage: Lineage { trainee: k.x, first_parent: k.a, second_parent: k.b }, deck: &k.sel };
        let bounds = OutcomeBounds { fixed_parents: fix, found_anchors: &k.anc, own_trios: &k.her_hi };
        hi = Some(outcome_hi(s, &context, career, &bounds, OutcomeEstimate { index: outcome, saving: y }, &mut ac));
    }
    k.hi_anc = ac;
    // (y_hi >= y is derived, `outcome_hi` (1)-(3): checked here, within rounding, where the max below would hide a bound
    // that drifted under the estimate; the max stays for the page)
    debug_assert!(hi.is_none_or(|h| h >= y - 1e-9 * y.abs().max(1.0)), "roll: y_hi {hi:?} below y {y}");
    json!({"y": (y * 100.0).round() / 100.0, "y_hi": hi.map(|h| (h.max(y) * 100.0).ceil() / 100.0)})
}

/// the kept roll states for the page to store (`state_out`), every float as its bits
pub(in crate::deck::career) fn rolls_out() -> Value {
    use crate::deck::career::{fvec, hx, sel_out, ux};
    let tri = |v: &[OwnTrio]| {
        v.iter()
            .map(|&OwnTrio { candidate: CandidateIndex(i), uses_existing: r, finisher_cost: c }| json!([i, r, hx(c)]))
            .collect::<Vec<_>>()
    };
    ROLLS.with(|c| Value::Array(c.borrow().iter().map(|r| { let (sd, b) = (&r.sd, &r.base); json!({
        "k": ux(r.key), "x": [r.x, r.a, r.b], "sel": sel_out(&r.sel), "her": tri(&r.her_hi),
        "anc": r.anc.iter().map(|a| a.as_ref().map(|anchor| json!([anchor.deck_class, fvec(&anchor.inheritance_chances), hx(anchor.value)]))).collect::<Vec<_>>(),
        "base": {"cands": b.cands, "c1": fvec(&b.c1), "v0": hx(b.v0), "pnow": fvec(&b.pnow), "m23": fvec(&b.m23),
                 "cont": b.cont.iter().map(|c| json!([hx(c.lam), hx(c.phi), hx(c.pinf), !c.tab.is_empty()])).collect::<Vec<_>>(),
                 "combos": b.combos.iter().map(|&Trio { candidate_a: CandidateIndex(ia), candidate_b: CandidateIndex(ib), finisher_cost: c3, a_uses_existing: ra, b_uses_existing: rb }| json!([ia, ib, hx(c3), ra, rb])).collect::<Vec<_>>()},
        "sd": {"n": [sd.nu, sd.nf, sd.ntar, sd.na], "xi": sd.xi, "goal": sd.goal, "w": fvec(&sd.w), "uncv": sd.uncv, "fmm": sd.fmm,
               "x0": sd.x0.iter().map(|v| fvec(v)).collect::<Vec<_>>(), "xm": sd.xm.iter().map(|v| fvec(v)).collect::<Vec<_>>(),
               "pw": sd.pw.iter().map(|v| fvec(&v.iter().flatten().copied().collect::<Vec<f64>>())).collect::<Vec<_>>(),
               "tsi": sd.tsi, "t0": fvec(&sd.t0), "col": sd.col, "sslot": sd.sslot, "p0": fvec(&sd.p0), "os": fvec(&sd.os),
               "act": sd.act.iter().map(|&ActiveCandidate { candidate: CandidateIndex(ui), slot: sl, threshold: th, current_chance: pn }| json!([ui, sl, hx(th), hx(pn)])).collect::<Vec<_>>(), "her": tri(&sd.her)}}) }).collect()))
}

pub(in crate::deck::career) fn rolls_in(v: &Value) -> Option<Vec<RollKeep>> {
    use crate::deck::career::{STATE_MAX_N, fvec_r, fx, pvec_r, sel_in, usz, uxr};
    // (absent in a state stored before them: none)
    let Some(a) = v.as_array() else { return v.is_null().then(Vec::new) };
    let uv = |v: &Value| -> Option<Vec<usize>> { v.as_array()?.iter().map(usz).collect() };
    let tri = |v: &Value| -> Option<Vec<OwnTrio>> {
        v.as_array()?
            .iter()
            .map(|t| {
                Some(OwnTrio {
                    candidate: CandidateIndex(usz(&t[0])?),
                    uses_existing: t[1].as_bool()?,
                    finisher_cost: fx(&t[2])?,
                })
            })
            .collect()
    };
    let arr4 = |v: &Value| -> Option<[f64; 4]> {
        let x = fvec_r(v)?;
        x.try_into().ok()
    };
    a.iter()
        .map(|e| {
            let (b, s) = (&e["base"], &e["sd"]);
            let n = uv(&s["n"])?;
            // (the plan's limits, `sim_for`: at most 12 targets, 10 of them uncertain, the rest guaranteed; checked before
            // any table is sized by them)
            if n.len() != 4 || n[2] > 12 || n[0] > 10 || n[1] > 12 || n[0] + n[1] != n[2] {
                return None;
            }
            let cont = b["cont"]
                .as_array()?
                .iter()
                .map(|c| {
                    let (lam, phi, pinf) = (fx(&c[0])?, fx(&c[1])?, fx(&c[2])?);
                    // (a Cont without its table has no building: `Cont::default`'s lam 0; one that reads as building is refused)
                    Some(if c[3].as_bool()? {
                        Cont::new(lam, phi, pinf)
                    } else if lam > 0.0 && phi > 0.0 {
                        return None;
                    } else {
                        Cont { lam, phi, pinf, ..Cont::default() }
                    })
                })
                .collect::<Option<Vec<Cont>>>()?;
            let combos = b["combos"]
                .as_array()?
                .iter()
                .map(|t| {
                    Some(Trio {
                        candidate_a: CandidateIndex(usz(&t[0])?),
                        candidate_b: CandidateIndex(usz(&t[1])?),
                        finisher_cost: fx(&t[2])?,
                        a_uses_existing: t[3].as_bool()?,
                        b_uses_existing: t[4].as_bool()?,
                    })
                })
                .collect::<Option<Vec<Trio>>>()?;
            let base = KeptBase {
                cands: uv(&b["cands"])?,
                c1: fvec_r(&b["c1"])?,
                v0: fx(&b["v0"])?,
                pnow: fvec_r(&b["pnow"])?,
                cont,
                combos,
                m23: fvec_r(&b["m23"])?,
            };
            let w: [f64; 3] = pvec_r(&s["w"])?.try_into().ok()?;
            let pw = s["pw"]
                .as_array()?
                .iter()
                .map(|v| {
                    let f = fvec_r(v)?;
                    (f.len() == 2 * n[2]).then(|| f.chunks(2).map(|c| [c[0], c[1]]).collect())
                })
                .collect::<Option<Vec<Vec<[f64; 2]>>>>()?;
            let act = s["act"]
                .as_array()?
                .iter()
                .map(|t| {
                    Some(ActiveCandidate {
                        candidate: CandidateIndex(usz(&t[0])?),
                        slot: if t[1].is_null() { None } else { Some(usz(&t[1])?) },
                        threshold: fx(&t[2])?,
                        current_chance: fx(&t[3])?,
                    })
                })
                .collect::<Option<Vec<_>>>()?;
            let mut sd = Scored {
                nu: n[0],
                nf: n[1],
                ntar: n[2],
                na: n[3],
                xi: if s["xi"].is_null() { None } else { Some(usz(&s["xi"])?) },
                goal: s["goal"].as_bool()?,
                w,
                uncv: uv(&s["uncv"])?,
                fmm: uv(&s["fmm"])?.into_iter().map(TargetMask::from_bits).collect(),
                x0: s["x0"].as_array()?.iter().map(fvec_r).collect::<Option<_>>()?,
                xm: s["xm"].as_array()?.iter().map(arr4).collect::<Option<_>>()?,
                pw,
                tsi: uv(&s["tsi"])?,
                t0: fvec_r(&s["t0"])?,
                col: s["col"].as_array()?.iter().map(serde_json::Value::as_bool).collect::<Option<_>>()?,
                sslot: uv(&s["sslot"])?,
                p0: fvec_r(&s["p0"])?,
                os: pvec_r(&s["os"])?,
                act,
                her: tri(&s["her"])?,
                xs: vec![],
                xc: vec![],
                scf: vec![],
            };
            // (a store that does not fit together is refused whole, as any other: the vectors' lengths and every index
            // within what it indexes, as `score_career_` builds them, before `derive` sizes its tables)
            let (ns, nc) = (sd.x0.len(), base.cands.len());
            let fixed: Vec<usize> = (0..sd.ntar).filter(|i| !sd.uncv.contains(i)).collect();
            let fits_base = nc <= STATE_MAX_N
                && base.c1.len() == nc
                && base.pnow.len() == nc
                && base.m23.len() == nc
                && base.cont.len() <= nc
                && base.combos.iter().all(|t| t.candidate_a.0 < nc && t.candidate_b.0 < nc);
            // her setups (at most IDX_PARTNERS per candidate), the candidates the loop lifts (those with a slot in slot
            // order, one per slot), her own candidate index and trio sides: candidate indices
            let fits_cands = ns <= IDX_PARTNERS * nc
                && sd.act.iter().filter_map(|a| a.slot).eq(0..sd.na)
                && sd.act.iter().all(|a| a.candidate.0 < nc)
                && sd.xi.is_none_or(|x| x < nc)
                && sd.her.iter().all(|h| h.candidate.0 < nc);
            if !fits_base
                || !fits_cands
                || !sd.uncv.windows(2).all(|p| p[0] < p[1])
                || sd.uncv.last().is_some_and(|&i| i >= sd.ntar)
                || sd.fmm != fm_masks(&fixed)
                || sd.sslot.windows(2).any(|p| p[0] > p[1])
                || sd.uncv.len() != sd.nu
                || sd.col.len() != sd.nu
                || sd.xm.len() != ns
                || sd.pw.len() != ns
                || sd.sslot.len() != ns
                || sd.os.len() != 1usize << sd.ntar
                || sd.fmm.len() != 1usize << sd.nf
                || sd.t0.len() != sd.tsi.len() << sd.nu
                || sd.x0.iter().any(|v| v.len() != sd.nu)
                || sd.tsi.iter().any(|&si| si >= ns)
                || sd.sslot.iter().any(|&sl| sl >= sd.na)
                || sd.p0.len() != sd.na
            {
                return None;
            }
            sd.derive();
            let x = uv(&e["x"])?;
            if x.len() != 3 {
                return None;
            }
            let anc = e["anc"]
                .as_array()?
                .iter()
                .map(|a| {
                    if a.is_null() {
                        Some(None)
                    } else {
                        Some(Some(FoundAnchor {
                            deck_class: usz(&a[0])?,
                            inheritance_chances: pvec_r(&a[1])?,
                            value: fx(&a[2])?,
                        }))
                    }
                })
                .collect::<Option<Vec<_>>>()?;
            // (the anchors' chances: one per job target; her trio sides at the plan's end: candidate indices)
            let her_hi = tri(&e["her"])?;
            if anc.iter().flatten().any(|a| a.inheritance_chances.len() != sd.ntar)
                || her_hi.iter().any(|h| h.candidate.0 >= nc)
            {
                return None;
            }
            Some(RollKeep {
                key: uxr(&e["k"])?,
                sd,
                base,
                x: x[0],
                a: x[1],
                b: x[2],
                sel: sel_in(&e["sel"])?,
                her_hi,
                anc,
                hi_anc: vec![],
            })
        })
        .collect()
}

pub(in crate::deck::career) fn rolls_set(r: Vec<RollKeep>) {
    ROLLS.with(|c| *c.borrow_mut() = r);
}
