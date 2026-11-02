//! The veterans the planner imagines: a career's veteran before her result is known (`CareerSetup::veteran`), the
//! synthetic full veterans of a loop's ceiling and of the hint ranking (`plan::search`), and an audit's veteran at one
//! outcome (`plan::audit`). A real veteran is a record (`Legacy`), which a job sends and the planner keys and identifies;
//! an imagined one has none of that, only what the pool reads of her (`Imagined`, `CareerSetup::imagined_who`) and her
//! two parents as the pool holds them.
use crate::career::Spark;
use crate::career::pool::Who;
use crate::career::schedule::PinkChance;

/// What the pool reads of an imagined veteran.
#[derive(Clone)]
pub(crate) struct Imagined {
    pub(crate) character_id: u32,
    /// Her white sparks: fixed stars, or (with `star_mix`) her targets, their stars drawn from the mix.
    pub(crate) sparks: Vec<Spark>,
    /// The star mix her sparks are drawn from (None: `sparks`' stars are hers).
    pub(crate) star_mix: Option<[f64; 3]>,
    /// Her chance of having won each race, by race index (empty: none won).
    pub(crate) win_chances: Vec<f64>,
    /// The pinks she may carry, one at most, each at its chance (empty: no pink modelled for her).
    pub(crate) pink_chances: Vec<PinkChance>,
}

/// An imagined veteran with her two parents, as the pool holds them (`CareerSetup::pool_push_imagined`).
#[derive(Clone)]
pub(crate) struct ImaginedVeteran {
    pub(crate) her: Imagined,
    pub(crate) parents: [Who; 2],
}

#[cfg(test)]
impl ImaginedVeteran {
    /// A veteran for a test that never reads her (a route's lift slot, a fresh side never placed).
    pub(crate) fn unread() -> Self {
        let parent = Who {
            uma_index: None,
            character_id: 0,
            wins: vec![],
            stars: std::rc::Rc::new(vec![]),
            pink: None,
            pink_chances: std::rc::Rc::new(vec![]),
            star_mix: None,
        };
        let her =
            Imagined { character_id: 0, sparks: vec![], star_mix: None, win_chances: vec![], pink_chances: vec![] };
        Self { her, parents: [parent.clone(), parent] }
    }
}
