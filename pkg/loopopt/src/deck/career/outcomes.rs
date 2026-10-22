//! Career reports, outcome sets and corner tables.
use super::{Ancestor, Env, Found, Legacy, Pool, aff_factor, apt_pink, inherit, now_ms, pink_p, prof, set_ews};
use crate::deck::career::Lineage;
use crate::deck::{GEN_GOLD, JointExt, JointScratch, Kind, Sel, Src, Value, joint_eval_in, joint_prep, json};

/// Multilinear interpolation of corner values v (index bit k = coordinate k at 1) at the point x, with a buffer the
/// caller keeps (the simulator and the plan evaluate tables millions of times).
/// Folds coordinate 0 first, halving the table each step: entry i after k folds is the full sweep's entry i << k (the
/// same multiply-adds on the same values, so the same bits), and only those reach entry 0.
pub(in crate::deck::career) fn interp_in(v: &[f64], x: &[f64], cur: &mut Vec<f64>) -> f64 {
    cur.clear();
    cur.extend_from_slice(v);
    let mut n = cur.len();
    for &xk in x {
        n /= 2;
        for i in 0..n {
            cur[i] = cur[2 * i] * (1.0 - xk) + cur[2 * i + 1] * xk;
        }
    }
    cur[0]
}

impl Env<'_> {
    /// One career's report: trainee and outfit, parents, the six legacy slots, per target odds, aptitudes, deck, schedule.
    pub(in crate::deck::career) fn report(&mut self, p: &Pool, f: &Found, with_schedule: bool) -> Value {
        let d = self.d;
        let ntar = self.ntar;
        let nu = self.nu;
        let name = |id: u32| d.umas.iter().find(|u| u.id == id).map_or_else(|| format!("Uma {id}"), |u| u.name.clone());
        let (t, a, b) = (f.t, f.a, f.b);
        let uid = d.umas[t].id;
        let sl = self.slots(p, t, a, b);
        let (q, nc) = inherit(&sl, ntar);
        self.set_trainee(t, &q);
        let (j, ext) = self.exact(t, &f.sel, &nc, true, with_schedule && ntar <= 12);
        let o = j.2;
        let mut pt = vec![0.0f64; ntar];
        for (pos, &i) in self.uncv.iter().enumerate() {
            pt[i] = ext.marg[o * nu + pos];
        }
        for (pos, &i) in self.fixed_t.iter().enumerate() {
            pt[i] = GEN_GOLD * ext.sf[pos];
        }
        let sj = self.sj;
        let (st, cards) = &self.model.opts[t][o];
        let covers: Vec<Value> = st
            .iter()
            .zip(&self.uncv)
            .filter(|(lv, i)| **lv > 0 && !(**lv == 1 && sj.targets[**i].guaranteed == 1))
            .map(|(lv, i)| json!({"group": sj.targets[*i].group, "level": lv}))
            .collect();
        let default_st: Vec<u8> = self.uncv.iter().map(|&i| u8::from(sj.targets[i].guaranteed == 1)).collect();
        let outfits = if *st == default_st {
            vec![]
        } else {
            cards.iter().map(|c| json!({"id": c.0, "label": c.1})).collect::<Vec<_>>()
        };
        let slots: Vec<Value> = sl.iter().map(|s| json!({"kind": if s.parent { "parent" } else { "grandparent" }, "legacy": s.legacy, "gp": s.gp,
            "chara": s.chara, "name": name(s.chara), "rel": s.rel, "race": s.race, "aff": s.rel + s.race, "stars": &*s.stars, "pinks": &*s.pinks})).collect();
        // per target, buying it and landing it (MODEL.md, skills): bought as gold (her outfit's gold, or a deck gold hint)
        // or white (her outfit's white; else not gold and hinted by the deck, her own events or inheritance), each landing
        // with GEN_GOLD / GEN_WHITE x her spark scale; the deck's hint chances are its per-target marginals (`sel.dw/dg`), so
        // these products match `p` up to the deck chains' correlation. The scenario's golds: bought, landing GEN_GOLD.
        let (su, sf) = self.scale(&nc);
        let ewo = &self.ew0[t][o];
        let buy = |i: usize| -> Value {
            if let Some(pos) = self.uncv.iter().position(|&x| x == i) {
                let (v, dw, dg, e) = (st[pos], f.sel.dw[pos], f.sel.dg[pos], ewo[pos]);
                let gold = if v == 2 { 1.0 } else { dg };
                let white = if v == 2 {
                    0.0
                } else if v == 1 {
                    1.0 - dg
                } else {
                    (1.0 - dg) * (1.0 - (1.0 - dw) * (1.0 - e) * (1.0 - q[i]))
                };
                json!({"gold": gold, "white": white, "land_gold": gold * crate::skills::GEN_GOLD * su[pos], "land_white": white * crate::skills::GEN_WHITE * su[pos],
                       "deck_gold": dg, "deck_white": dw, "own_event": e, "inherit": q[i], "outfit": v})
            } else {
                let pos = self.fixed_t.iter().position(|&x| x == i).unwrap_or(0);
                json!({"gold": 1.0, "white": 0.0, "land_gold": crate::skills::GEN_GOLD * sf.get(pos).copied().unwrap_or(1.0), "land_white": 0.0, "scenario": true})
            }
        };
        let targets: Vec<Value> = (0..ntar)
            .map(|i| {
                json!({"group": sj.targets[i].group, "q": q[i], "carriers": nc[i], "p": pt[i],
            "gold": sj.targets[i].guaranteed == 2, "buy": buy(i)})
            })
            .collect();
        let pd = self.space.pdeck(&f.sel, &[uid]);
        // pink sparks: per aptitude the chance at least one of the two inheritance rolls passes it on (1/3/5% per star)
        let mut pg: Vec<u32> = sl.iter().flat_map(|s| s.pinks.iter().map(|x| x.0)).collect();
        pg.sort_unstable();
        pg.dedup();
        let pinks: Vec<Value> = pg
            .iter()
            .map(|&g| {
                let (mut miss, mut n) = (1.0f64, 0);
                for s in &sl {
                    if let Some(st) = s.pinks.iter().filter(|x| x.0 == g).map(|x| x.1.min(3)).max()
                        && st > 0
                    {
                        n += 1;
                        miss *= 1.0 - pink_p(st, aff_factor(s.rel, s.race));
                    }
                }
                json!({"group": g, "q": 1.0 - miss * miss, "carriers": n})
            })
            .collect();
        // her aptitudes and her veteran's pink spark (MODEL.md): base and start grades (data.json `apt` order, codes 1..8 =
        // G..S), each legacy pink's chance to fire at one April inspiration, and the chance of each pink type
        let apt = self.pink_career(p, t, a, b).map(|pc| {
            let pg8 = Self::pink_gen(&pc);
            json!({"base": pc.base, "start": pc.start,
                   "fire": pc.fire.iter().map(|f| json!({"group": apt_pink(f.apt), "apt": f.apt, "p": f.p, "slot": f.slot, "stars": f.stars})).collect::<Vec<_>>(),
                   "pink_gen": (0..10).map(|k| json!({"group": apt_pink(k), "p": pg8[k]})).collect::<Vec<_>>()})
        });
        let outcomes = if with_schedule && ntar <= 12 { json!(self.outcome_sets(j.2, &ext)) } else { Value::Null };
        let (sched, wp) = if with_schedule { self.schedule(p, t, a, b) } else { (Value::Null, vec![]) };
        json!({"trainee": {"id": uid, "name": d.umas[t].name, "outfits": outfits, "covers": covers},
               "parents": [a, b], "slots": slots, "targets": targets, "p_all": j.0, "p_minus1": j.1, "expected": pt.iter().sum::<f64>(),
               "bound": f.ub, "deck": pd.out(&self.unc), "pinks": pinks, "aptitudes": apt, "schedule": sched, "outcome_sets": outcomes, "g1_wins": if with_schedule { json!(wp.iter().sum::<f64>()) } else { Value::Null }})
    }

    /// Corner table of trainee t with deck `sel`: per outfit, P(all) with every spark-generation scale 1 at the 2^nu
    /// points q_k = 1 - (1 - q0_k) x_k, x_k in {0, 1} (index bit k = uncertain target k at x_k = 1). Derived: q_k enters
    /// every landing factor only through (1 - e)(1 - q_k), affinely, and P(all) per outfit is a sum of products with
    /// one factor per target, so it is multilinear in x and `interp` at any x in [0, 1]^nu gives it exactly.
    /// `corners` at q0 = 0 (a cached deck's table, `Cache::add`), memoised by (deck class, entries, filler). Exact: the
    /// table reads trainee t only through her model (outfit coverage, own-skill chances: `set_trainee` from `ew0`), which
    /// her deck class holds to the bit, and the deck only through its entries and filler; the memo is kept between jobs
    /// of one setup (`DECK_KEEP`: the same model and entries). `corners` leaves her model at its last corner, q = 0 for
    /// every target (1 - (1 - 0) x 1); a hit sets the same state
    pub(in crate::deck) fn corners_kept(&mut self, t: usize, sel: &Sel) -> Vec<Vec<f64>> {
        let key = (self.cls[t], sel.ents.clone(), sel.filler);
        let z = vec![0.0; self.ntar];
        if let Some(r) = self.corner_memo.get(&key) {
            let r = (**r).clone();
            self.set_trainee(t, &z);
            return r;
        }
        let tc = now_ms();
        let r = self.corners(t, sel, &z);
        prof("corner_ms", now_ms() - tc);
        prof("corner_builds", 1.0);
        self.corner_memo.insert(key, std::rc::Rc::new(r.clone()));
        r
    }
    fn corners(&mut self, t: usize, sel: &Sel, q0: &[f64]) -> Vec<Vec<f64>> {
        let (nu, ntar) = (self.nu, self.ntar);
        let no = self.model.n_outfits(t);
        let mut corners = vec![vec![0.0f64; 1 << nu]; no];
        // per corner `exact(t, sel, [6; ntar], false, false)`: the deck's part of `joint_fast` (`joint_prep`, which reads
        // no chance and no model state) once, its evaluation per corner (the same sums; the scale is 1 throughout)
        let (su, sf) = self.scale(&vec![6; ntar]);
        let ents: Vec<(&Src, Kind)> = sel.ents.iter().map(|&j| (&self.space.srcs[j], self.space.kind[j])).collect();
        let pre = joint_prep(self.model.uncertain.len(), &ents, sel.filler);
        // (one chance vector and one `JointExt` for every corner: joint_eval only reads su, sf; `marg` is not read here,
        // so not summed)
        let mut q = q0.to_vec();
        let mut ext = JointExt { su, sf, marg: vec![], sets: vec![], per_outfit: vec![] };
        // (and one set of joint_eval's buffers)
        let mut sx = JointScratch::default();
        for c in 0..(1usize << nu) {
            for (k, &i) in self.uncv.iter().enumerate() {
                let x = if c >> k & 1 == 1 { 1.0 } else { 0.0 };
                q[i] = 1.0 - (1.0 - q0[i]) * x;
            }
            set_ews(&mut self.model.ews[t], &self.ew0[t], &self.uncv, &q);
            joint_eval_in(&self.model, t, &[0.0], &pre, ntar, false, false, Some(&mut ext), &mut sx);
            for o in 0..no {
                corners[o][c] = ext.per_outfit[o];
            }
        }
        corners
    }

    /// Joint distribution of full target sets, including guaranteed gold generation.
    pub(in crate::deck::career) fn outcome_sets(&self, outfit: usize, ext: &JointExt) -> Vec<f64> {
        let su = &ext.sets[(outfit << self.nu)..((outfit + 1) << self.nu)];
        let mut ps = vec![0.0f64; 1 << self.ntar];
        for (mu, &x) in su.iter().enumerate() {
            if x == 0.0 {
                continue;
            }
            let mut base = 0usize;
            for (k, &i) in self.uncv.iter().enumerate() {
                if mu >> k & 1 == 1 {
                    base |= 1 << i;
                }
            }
            let mut part = vec![(base, x)];
            for (pos, &i) in self.fixed_t.iter().enumerate() {
                let g = GEN_GOLD * ext.sf[pos];
                part = part.into_iter().flat_map(|(m, w)| [(m, w * (1.0 - g)), (m | 1 << i, w * g)]).collect();
            }
            for (m, w) in part {
                ps[m] += w;
            }
        }
        ps
    }

    /// The veteran career (t, a, b) leaves, before her result is known: trainee t, a spark of every target with stars
    /// drawn from the rank mix (`star_dist`), her schedule's win chances `wp`, parents a and b.
    pub(in crate::deck::career) fn veteran(&self, legs: &[Legacy], lineage: Lineage, wp: Vec<f64>) -> Legacy {
        let Lineage { trainee: t, first_parent: a, second_parent: b } = lineage;
        let anc = |l: &Legacy| Ancestor {
            chara: l.chara,
            sparks: l.sparks.clone(),
            wins: l.wins.clone(),
            pinks: l.pinks.clone(),
        };
        Legacy {
            chara: self.d.umas[t].id,
            borrowed: false,
            sparks: self.sj.targets.iter().map(|x| (x.group, 2)).collect(),
            wins: vec![],
            pinks: vec![],
            parents: vec![anc(&legs[a]), anc(&legs[b])],
            win_p: wp,
            mix: Some(self.sj.star_dist),
        }
    }
}
