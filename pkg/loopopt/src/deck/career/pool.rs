//! Veteran pools and legal parent pairs.
use super::{Ancestor, Env, Legacy};
use crate::UmaIndex;

/// What `Env::who` reads of a legacy or of one of her recorded parents.
struct VeteranTraits<'veteran> {
    chara: u32,
    /// (skill group, stars) of her white sparks.
    sparks: &'veteran [(u32, u8)],
    /// Races (data.json `races`) of the single-race G1s she won; read when `win_chances` is empty.
    wins: &'veteran [usize],
    /// A veteran before her result is known: her win chance per race index (empty: `wins` are certain).
    win_chances: &'veteran [f64],
    /// (aptitude factor group, stars) of her pink sparks.
    pinks: &'veteran [(u32, u8)],
    /// The star mix her sparks are drawn from (None: fixed stars).
    star_mix: Option<[f64; 3]>,
}

impl<'veteran> VeteranTraits<'veteran> {
    fn of_legacy(legacy: &'veteran Legacy) -> Self {
        Self {
            chara: legacy.chara,
            sparks: &legacy.sparks,
            wins: &legacy.wins,
            win_chances: &legacy.win_p,
            pinks: &legacy.pinks,
            star_mix: legacy.mix,
        }
    }
    /// A recorded parent: fixed stars and certain wins.
    fn of_ancestor(ancestor: &'veteran Ancestor) -> Self {
        Self {
            chara: ancestor.chara,
            sparks: &ancestor.sparks,
            wins: &ancestor.wins,
            win_chances: &[],
            pinks: &ancestor.pinks,
            star_mix: None,
        }
    }
}

/// What a lineage counts of one legacy: relation index, G1 wins as (race, chance) sorted by race, stars per job target,
/// pink sparks, and the star mix when the stars are random.
#[derive(Clone)]
/// (stars and pinks shared with the lineage slots made from her: `Env::slots` takes them without copying)
pub(crate) struct Who {
    /// Her index in Data::umas (None: a character the data does not list).
    pub(crate) uma_index: Option<UmaIndex>,
    pub(crate) chara: u32,
    /// Her G1 wins, sorted by race; chance 1 for a win on record.
    pub(crate) wins: Vec<RaceWin>,
    /// Stars (0..=3) of her spark of each job target, in job-target order.
    pub(crate) stars: std::rc::Rc<Vec<u8>>,
    /// (aptitude factor group, stars) of her pink sparks.
    pub(crate) pinks: std::rc::Rc<Vec<(u32, u8)>>,
    /// The star mix her sparks are drawn from (a veteran before her result is known); None: `stars` are fixed.
    pub(crate) star_mix: Option<[f64; 3]>,
}

/// One single-race G1 a legacy won, or may win.
#[derive(Clone, Copy)]
pub(crate) struct RaceWin {
    /// The race's index (data.json `races`).
    pub(crate) race: usize,
    /// Her chance of the win (1 for a win on record).
    pub(crate) chance: f64,
}

/// The veterans the parents come from, each with her own parents; per veteran and parent, the race part of that
/// grandparent slot's affinity, 3 x shared(veteran, parent) (constant per veteran: `Env::slots` reads it)
#[derive(Clone)]
pub(crate) struct Pool {
    /// Per veteran (legacy index), herself.
    pub(crate) veterans: Vec<Who>,
    /// Per veteran, her recorded parents (the trainee's grandparent slots under her), in record order.
    pub(crate) parents: Vec<Vec<Who>>,
    /// Per veteran, whether she is borrowed.
    pub(crate) borrowed: Vec<bool>,
    /// Per veteran and recorded parent, 3 x shared(veteran, parent): that grandparent slot's race affinity.
    pub(crate) grandparent_race_affinity: Vec<Vec<f64>>,
    /// How many borrowed veterans a pair of parents may hold (1; 0 under `borrowed_parent` "never", and for the pick's
    /// pool under "later").
    pub(crate) max_borrowed: u8,
}

impl Pool {
    /// whether veterans a and b hold at most `max_borrowed` borrowed ones between them
    #[inline]
    pub(crate) fn bor_ok(&self, a: usize, b: usize) -> bool {
        u8::from(self.borrowed[a]) + u8::from(self.borrowed[b]) <= self.max_borrowed
    }
    /// whether a veteran borrowed or not (`borrowed`) can be a parent at all: her share of `bor_ok`
    #[inline]
    pub(crate) fn bor_one(&self, borrowed: bool) -> bool {
        u8::from(borrowed) <= self.max_borrowed
    }
    /// The pair rules that hold for every trainee: two veterans, of different characters, at most
    /// `max_borrowed` borrowed
    #[inline]
    pub(crate) fn parents_ok(&self, a: usize, b: usize) -> bool {
        a != b && self.veterans[a].chara != self.veterans[b].chara && self.bor_ok(a, b)
    }
    /// The trainee's rule (character `tid`): neither parent is her character
    #[inline]
    pub(crate) fn trainee_ok(&self, tid: u32, a: usize, b: usize) -> bool {
        self.veterans[a].chara != tid && self.veterans[b].chara != tid
    }
    /// Legal parents a, b for a trainee of character `tid`: `parents_ok` and `trainee_ok`. The one rule every pair loop
    /// takes (`solve`'s candidates, the planner's and the simulator's scans), so a proof over its pairs covers `solve`'s
    #[inline]
    pub(crate) fn pair_ok(&self, tid: u32, a: usize, b: usize) -> bool {
        self.parents_ok(a, b) && self.trainee_ok(tid, a, b)
    }
}

/// Whether parents a and b hold every fixed legacy of `fix` (a job's parent places)
#[inline]
pub(crate) fn holds_fix(fix: &[usize], a: usize, b: usize) -> bool {
    fix.iter().all(|&f| f == a || f == b)
}

/// Whether legacy `x` is at least as good a parent as legacy `y` in every career (`Env::solve`, dominated veterans):
/// the same character, stars at least y's on every job target, G1 wins a superset of y's, and the same number of
/// parents, each (in order) the same character as y's, with stars and wins at least as good the same way. Only fixed
/// stars and certain wins (a veteran before her result is known, with random stars and win chances, never compares).
/// Derived: with partner z and trainee t, swapping y for x keeps every relation term (same characters, grandparents
/// in the same slots) and raises or keeps every shared-win count (x with z, x with her parents; nothing else changes),
/// so every slot's affinity and stars are at least as high: each q_k is at least as high (1 - prod (1 - min(1,
/// 0.03 s (1 + A/100)))^2 rises with every s and A), each carrier count n_k too (so the spark scale C), and with them
/// the ceiling, the `rise` bounds and every deck's product and exact value (each landing factor rises with q_k). The
/// same sums and products in the same order, so this holds for the floats as well (correctly rounded operations are
/// monotone).
pub(crate) fn dominates(p: &Pool, x: usize, y: usize) -> bool {
    let ge = |a: &Who, b: &Who| {
        a.chara == b.chara
            && a.star_mix.is_none()
            && b.star_mix.is_none()
            && a.stars.iter().zip(b.stars.iter()).all(|(s, t)| s >= t)
            && a.wins.iter().chain(&b.wins).all(|w| w.chance == 1.0)
            && b.wins.iter().all(|w| a.wins.iter().any(|v| v.race == w.race))
    };
    ge(&p.veterans[x], &p.veterans[y])
        && p.parents[x].len() == p.parents[y].len()
        && p.parents[x].iter().zip(&p.parents[y]).all(|(a, b)| ge(a, b))
}

/// G1s two legacies both won (expected count when a win is a chance)
pub(crate) fn shared(a: &Who, b: &Who) -> f64 {
    let (mut i, mut j, mut s) = (0, 0, 0.0f64);
    while i < a.wins.len() && j < b.wins.len() {
        match a.wins[i].race.cmp(&b.wins[j].race) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                s += a.wins[i].chance * b.wins[j].chance;
                i += 1;
                j += 1;
            }
        }
    }
    s
}

impl Env<'_> {
    pub(crate) fn uix(&self, id: u32) -> Option<UmaIndex> {
        self.data.umas.iter().position(|u| u.id == id).map(UmaIndex)
    }
    fn stars_of(&self, sp: &[(u32, u8)]) -> Vec<u8> {
        self.skill_job
            .targets
            .iter()
            .map(|t| sp.iter().filter(|s| s.0 == t.group).map(|s| s.1.min(3)).max().unwrap_or(0))
            .collect()
    }
    fn who(&self, traits: &VeteranTraits) -> Who {
        let mut w: Vec<RaceWin> = if traits.win_chances.is_empty() {
            let mut v = traits.wins.to_vec();
            v.sort_unstable();
            v.dedup();
            v.into_iter().map(|race| RaceWin { race, chance: 1.0 }).collect()
        } else {
            traits
                .win_chances
                .iter()
                .enumerate()
                .filter(|x| *x.1 > 0.0)
                .map(|(race, &chance)| RaceWin { race, chance })
                .collect()
        };
        w.sort_by_key(|x| x.race);
        Who {
            uma_index: self.uix(traits.chara),
            chara: traits.chara,
            wins: w,
            stars: std::rc::Rc::new(self.stars_of(traits.sparks)),
            pinks: std::rc::Rc::new(traits.pinks.to_vec()),
            star_mix: traits.star_mix,
        }
    }
    /// `pool` of the legacies so far plus `l` (pop with `pool_pop`)
    pub(crate) fn pool_push(&self, p: &mut Pool, l: &Legacy) {
        let me = self.who(&VeteranTraits::of_legacy(l));
        let gp: Vec<Who> = l.parents.iter().map(|g| self.who(&VeteranTraits::of_ancestor(g))).collect();
        p.grandparent_race_affinity.push(gp.iter().map(|g| 3.0 * shared(&me, g)).collect());
        p.veterans.push(me);
        p.parents.push(gp);
        p.borrowed.push(l.borrowed);
    }
    /// Undo one `pool_push`, keeping all four per-veteran arrays aligned.
    pub(crate) fn pool_pop(p: &mut Pool) {
        p.veterans.pop();
        p.parents.pop();
        p.borrowed.pop();
        p.grandparent_race_affinity.pop();
    }
    /// the solver's pool of `legs`: each one's entry as `pool_push` builds it (the one construction, the simulator's too)
    pub(crate) fn pool(&self, legs: &[Legacy]) -> Pool {
        let n = legs.len();
        let mut p = Pool {
            veterans: Vec::with_capacity(n),
            parents: Vec::with_capacity(n),
            borrowed: Vec::with_capacity(n),
            grandparent_race_affinity: Vec::with_capacity(n),
            max_borrowed: self.max_borrowed,
        };
        for l in legs {
            self.pool_push(&mut p, l);
        }
        p
    }
}
