//! The deck cache the simulator and the plan value careers through: per trainee her decks, each with its corner
//! tables. For a trainee and a fixed deck, P(all) per outfit is multilinear in the per-target inheritance chances
//! (derived; `corners`), so 2^nu exact evaluations per (trainee, deck) give it at any chances. Decks come from the
//! searches' fully checked decks (`Ref`, also kept for `rise`) and from deck searches where a bound says the cached
//! decks may fall short.
use super::*;

/// A cached deck of one trainee and per outfit its corner table (P(all), every spark scale 1, at q_k in {0, 1} per
/// uncertain target; index bit k set = q_k = 0, as `corners` builds them from q0 = 0)
#[derive(Clone)]
pub(super) struct CDeck { pub(super) sel: Sel, pub(super) tab: Vec<Vec<f64>> }
/// The deck cache of the simulator and the plan: per trainee her decks with corner tables and her fully checked decks (`Ref`, for `rise`)
#[derive(Clone)]
pub(super) struct Cache { pub(super) decks: Vec<Vec<CDeck>>, pub(super) refs: Vec<Vec<Ref>>, pub tables: usize }

impl Cache {
    pub(super) fn n_decks(&self) -> usize { self.decks.iter().map(|d| d.len()).sum() }
    pub(super) fn new(n: usize) -> Cache { Cache { decks: (0..n).map(|_| vec![]).collect(), refs: vec![vec![]; n], tables: 0 } }
    /// a deck of trainee t (its corner table built now); a deck already cached is skipped
    pub(super) fn add(&mut self, env: &mut Env, t: usize, sel: &Sel) {
        if self.decks[t].iter().any(|d| d.sel.ents == sel.ents && d.sel.plain == sel.plain && d.sel.filler == sel.filler) { return; }
        let tab = env.corners_kept(t, sel);
        self.tables += 1;
        self.decks[t].push(CDeck { sel: sel.clone(), tab });
    }
    /// a search's fully checked decks
    pub(super) fn absorb(&mut self, env: &mut Env, refs: &[Vec<Ref>]) {
        for (t, rs) in refs.iter().enumerate() { for r in rs { self.add(env, t, &r.sel); self.refs[t].push(r.clone()); } }
    }
}
