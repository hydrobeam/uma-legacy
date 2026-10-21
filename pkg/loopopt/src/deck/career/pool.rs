//! Veteran pools and legal parent pairs.
use super::{Env, Legacy};

/// What a lineage counts of one legacy: relation index, G1 wins as (race, chance) sorted by race, stars per job target,
/// pink sparks, and the star mix when the stars are random.
#[derive(Clone)]
/// (stars and pinks shared with the lineage slots made from her: `Env::slots` takes them without copying)
pub(in crate::deck::career) struct Who {
    pub(in crate::deck::career) ix: Option<usize>,
    pub(in crate::deck::career) chara: u32,
    pub(in crate::deck::career) wins: Vec<(usize, f64)>,
    pub(in crate::deck::career) stars: std::rc::Rc<Vec<u8>>,
    pub(in crate::deck::career) pinks: std::rc::Rc<Vec<(u32, u8)>>,
    pub(in crate::deck::career) mix: Option<[f64; 3]>,
}

/// The veterans the parents come from, each with her own parents; per veteran and parent, the race part of that
/// grandparent slot's affinity, 3 x shared(veteran, parent) (constant per veteran: `Env::slots` reads it)
/// `max_bor`: how many borrowed veterans a pair of parents may hold (1; 0 under `borrowed_parent` "never", and for the
/// pick's pool under "later")
#[derive(Clone)]
pub(in crate::deck::career) struct Pool {
    pub(in crate::deck::career) me: Vec<Who>,
    pub(in crate::deck::career) gp: Vec<Vec<Who>>,
    pub(in crate::deck::career) borrowed: Vec<bool>,
    pub(in crate::deck::career) gpc: Vec<Vec<f64>>,
    pub(in crate::deck::career) max_bor: u8,
}

impl Pool {
    /// whether veterans a and b hold at most `max_bor` borrowed ones between them
    #[inline]
    pub(in crate::deck::career) fn bor_ok(&self, a: usize, b: usize) -> bool {
        u8::from(self.borrowed[a]) + u8::from(self.borrowed[b]) <= self.max_bor
    }
    /// whether a veteran borrowed or not (`borrowed`) can be a parent at all: her share of `bor_ok`
    #[inline]
    pub(in crate::deck::career) fn bor_one(&self, borrowed: bool) -> bool {
        u8::from(borrowed) <= self.max_bor
    }
    /// The pair rules that hold for every trainee: two veterans, of different characters, at most `max_bor` borrowed
    #[inline]
    pub(in crate::deck::career) fn parents_ok(&self, a: usize, b: usize) -> bool {
        a != b && self.me[a].chara != self.me[b].chara && self.bor_ok(a, b)
    }
    /// The trainee's rule (character `tid`): neither parent is her character
    #[inline]
    pub(in crate::deck::career) fn trainee_ok(&self, tid: u32, a: usize, b: usize) -> bool {
        self.me[a].chara != tid && self.me[b].chara != tid
    }
    /// Legal parents a, b for a trainee of character `tid`: `parents_ok` and `trainee_ok`. The one rule every pair loop
    /// takes (`solve`'s candidates, the planner's and the simulator's scans), so a proof over its pairs covers `solve`'s
    #[inline]
    pub(in crate::deck::career) fn pair_ok(&self, tid: u32, a: usize, b: usize) -> bool {
        self.parents_ok(a, b) && self.trainee_ok(tid, a, b)
    }
}

/// Whether parents a and b hold every fixed legacy of `fix` (a job's parent places)
#[inline]
pub(in crate::deck::career) fn holds_fix(fix: &[usize], a: usize, b: usize) -> bool {
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
pub(in crate::deck::career) fn dominates(p: &Pool, x: usize, y: usize) -> bool {
    let ge = |a: &Who, b: &Who| {
        a.chara == b.chara
            && a.mix.is_none()
            && b.mix.is_none()
            && a.stars.iter().zip(b.stars.iter()).all(|(s, t)| s >= t)
            && a.wins.iter().chain(&b.wins).all(|w| w.1 == 1.0)
            && b.wins.iter().all(|w| a.wins.iter().any(|v| v.0 == w.0))
    };
    ge(&p.me[x], &p.me[y]) && p.gp[x].len() == p.gp[y].len() && p.gp[x].iter().zip(&p.gp[y]).all(|(a, b)| ge(a, b))
}

/// G1s two legacies both won (expected count when a win is a chance)
pub(in crate::deck::career) fn shared(a: &Who, b: &Who) -> f64 {
    let (mut i, mut j, mut s) = (0, 0, 0.0f64);
    while i < a.wins.len() && j < b.wins.len() {
        match a.wins[i].0.cmp(&b.wins[j].0) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                s += a.wins[i].1 * b.wins[j].1;
                i += 1;
                j += 1;
            }
        }
    }
    s
}

impl Env<'_> {
    pub(in crate::deck::career) fn uix(&self, id: u32) -> Option<usize> {
        self.d.umas.iter().position(|u| u.id == id)
    }
    fn stars_of(&self, sp: &[(u32, u8)]) -> Vec<u8> {
        self.sj
            .targets
            .iter()
            .map(|t| sp.iter().filter(|s| s.0 == t.group).map(|s| s.1.min(3)).max().unwrap_or(0))
            .collect()
    }
    fn who(
        &self,
        chara: u32,
        sparks: &[(u32, u8)],
        wins: &[usize],
        win_p: &[f64],
        pinks: &[(u32, u8)],
        mix: Option<[f64; 3]>,
    ) -> Who {
        let mut w: Vec<(usize, f64)> = if win_p.is_empty() {
            let mut v = wins.to_vec();
            v.sort_unstable();
            v.dedup();
            v.into_iter().map(|r| (r, 1.0)).collect()
        } else {
            win_p.iter().enumerate().filter(|x| *x.1 > 0.0).map(|(r, &p)| (r, p)).collect()
        };
        w.sort_by_key(|x| x.0);
        Who {
            ix: self.uix(chara),
            chara,
            wins: w,
            stars: std::rc::Rc::new(self.stars_of(sparks)),
            pinks: std::rc::Rc::new(pinks.to_vec()),
            mix,
        }
    }
    /// `pool` of the legacies so far plus `l` (pop with `pool_pop`)
    pub(in crate::deck::career) fn pool_push(&self, p: &mut Pool, l: &Legacy) {
        let me = self.who(l.chara, &l.sparks, &l.wins, &l.win_p, &l.pinks, l.mix);
        let gp: Vec<Who> =
            l.parents.iter().map(|g| self.who(g.chara, &g.sparks, &g.wins, &[], &g.pinks, None)).collect();
        p.gpc.push(gp.iter().map(|g| 3.0 * shared(&me, g)).collect());
        p.me.push(me);
        p.gp.push(gp);
        p.borrowed.push(l.borrowed);
    }
    /// Undo one `pool_push`, keeping all four per-veteran arrays aligned.
    pub(in crate::deck::career) fn pool_pop(p: &mut Pool) {
        p.me.pop();
        p.gp.pop();
        p.borrowed.pop();
        p.gpc.pop();
    }
    /// the solver's pool of `legs`: each one's entry as `pool_push` builds it (the one construction, the simulator's too)
    pub(in crate::deck::career) fn pool(&self, legs: &[Legacy]) -> Pool {
        let n = legs.len();
        let mut p = Pool {
            me: Vec::with_capacity(n),
            gp: Vec::with_capacity(n),
            borrowed: Vec::with_capacity(n),
            gpc: Vec::with_capacity(n),
            max_bor: self.max_bor,
        };
        for l in legs {
            self.pool_push(&mut p, l);
        }
        p
    }
}
