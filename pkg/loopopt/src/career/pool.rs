//! Veteran pools and legal parent pairs.
use crate::career::schedule::PinkChance;
use crate::career::{Ancestor, CareerSetup, Legacy};
use crate::uma::UmaIndex;
/// What `CareerSetup::who` reads of a legacy or of one of her recorded parents.
struct VeteranTraits<'veteran> {
    character_id: u32,
    /// (skill group, stars) of her white sparks.
    sparks: &'veteran [(u32, u8)],
    /// Races (data.json `races`) of the single-race G1s she won; read when `win_chances` is empty.
    wins: &'veteran [usize],
    /// A veteran before her result is known: her win chance per race index (empty: `wins` are certain).
    win_chances: &'veteran [f64],
    /// (aptitude factor group, stars) of her pink sparks.
    pinks: &'veteran [(u32, u8)],
    /// A veteran before her result is known: the pinks she may carry, one at most (empty: `pinks` are hers).
    pink_chances: &'veteran [PinkChance],
    /// The star mix her sparks are drawn from (None: fixed stars).
    star_mix: Option<[f64; 3]>,
}

impl<'veteran> VeteranTraits<'veteran> {
    fn of_legacy(legacy: &'veteran Legacy) -> Self {
        Self {
            character_id: legacy.character_id,
            sparks: &legacy.sparks,
            wins: &legacy.wins,
            win_chances: &legacy.win_chances,
            pinks: &legacy.pinks,
            pink_chances: &legacy.pink_chances,
            star_mix: legacy.mix,
        }
    }
    /// A legacy's parent: a recorded one's fixed stars and certain wins; a hypothetical one's star mix and win chances,
    /// and (the parent of a veteran before her result is known) her possible pinks (`Ancestor::of_legacy`).
    fn of_ancestor(ancestor: &'veteran Ancestor) -> Self {
        Self {
            character_id: ancestor.character_id,
            sparks: &ancestor.sparks,
            wins: &ancestor.wins,
            win_chances: &ancestor.win_chances,
            pinks: &ancestor.pinks,
            pink_chances: &ancestor.pink_chances,
            star_mix: ancestor.mix,
        }
    }
}

/// What a lineage counts of one legacy: relation index, G1 wins as (race, chance) sorted by race, stars per job target,
/// pink sparks, and the star mix when the stars are random (stars and pinks shared with the lineage slots made from
/// her: `CareerSetup::slots` takes them without copying).
#[derive(Clone)]
pub(crate) struct Who {
    /// Her index in Data::umas (None: a character the data does not list).
    pub(crate) uma_index: Option<UmaIndex>,
    pub(crate) character_id: u32,
    /// Her G1 wins, sorted by race; chance 1 for a win on record.
    pub(crate) wins: Vec<RaceWin>,
    /// Stars (0..=3) of her spark of each job target, in job-target order.
    pub(crate) stars: std::rc::Rc<Vec<u8>>,
    /// (aptitude factor group, stars) of her pink sparks.
    pub(crate) pinks: std::rc::Rc<Vec<(u32, u8)>>,
    /// A veteran before her result is known: the pinks she may carry, one at most (empty: `pinks` are hers).
    pub(crate) pink_chances: std::rc::Rc<Vec<PinkChance>>,
    /// The star mix her sparks are drawn from (a veteran before her result is known); None: `stars` are fixed.
    pub(crate) star_mix: Option<[f64; 3]>,
}

impl Who {
    /// Whether she is at least as good a parent as `other` in her own right (`Pool::dominates`): the same character,
    /// fixed stars at least `other`'s on every job target, certain wins only, and a superset of `other`'s wins.
    fn at_least_as_good_as(&self, other: &Self) -> bool {
        self.character_id == other.character_id
            && self.star_mix.is_none()
            && other.star_mix.is_none()
            && self.stars.iter().zip(other.stars.iter()).all(|(own, theirs)| own >= theirs)
            && self.wins.iter().chain(&other.wins).all(|win| win.chance == 1.0)
            && other.wins.iter().all(|their_win| self.wins.iter().any(|own_win| own_win.race == their_win.race))
    }
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
/// grandparent slot's affinity, 3 x shared(veteran, parent) (constant per veteran: `CareerSetup::slots` reads it)
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
    pub(crate) fn within_borrow_limit(&self, a: usize, b: usize) -> bool {
        u8::from(self.borrowed[a]) + u8::from(self.borrowed[b]) <= self.max_borrowed
    }
    /// whether a veteran borrowed or not (`borrowed`) can be a parent at all: her share of `within_borrow_limit`
    #[inline]
    pub(crate) fn alone_within_borrow_limit(&self, borrowed: bool) -> bool {
        u8::from(borrowed) <= self.max_borrowed
    }
    /// The pair rules that hold for every trainee: two veterans, of different characters, at most
    /// `max_borrowed` borrowed
    #[inline]
    pub(crate) fn parents_ok(&self, a: usize, b: usize) -> bool {
        a != b && self.veterans[a].character_id != self.veterans[b].character_id && self.within_borrow_limit(a, b)
    }
    /// The trainee's rule (character `trainee_chara`): neither parent is her character
    #[inline]
    pub(crate) fn trainee_ok(&self, trainee_chara: u32, a: usize, b: usize) -> bool {
        self.veterans[a].character_id != trainee_chara && self.veterans[b].character_id != trainee_chara
    }
    /// Legal parents a, b for a trainee of character `trainee_chara`: `parents_ok` and `trainee_ok`. The one rule every
    /// pair loop takes (`solve`'s candidates, the planner's and the simulator's scans), so a proof over its pairs covers
    /// `solve`'s
    #[inline]
    pub(crate) fn pair_ok(&self, trainee_chara: u32, a: usize, b: usize) -> bool {
        self.parents_ok(a, b) && self.trainee_ok(trainee_chara, a, b)
    }

    /// Whether veteran `x` is at least as good a parent as veteran `y` in every career (`Env::solve`, dominated
    /// veterans): she is at least as good in her own right (`Who::at_least_as_good_as`: the same character, stars at
    /// least y's on every job target, G1 wins a superset of y's), and she has the same number of parents, each (in
    /// order) at least as good as y's the same way. Only fixed stars and certain wins (a veteran before her result is
    /// known, with random stars and win chances, never compares).
    /// Derived: with partner z and trainee t, swapping y for x keeps every relation term (same characters,
    /// grandparents in the same slots) and raises or keeps every shared-win count (x with z, x with her parents;
    /// nothing else changes), so every slot's affinity and stars are at least as high: each q_k is at least as high
    /// (1 - prod (1 - min(1, 0.03 s (1 + A/100)))^2 rises with every s and A), each carrier count n_k too (so the spark
    /// scale C), and with them the ceiling, the `rise` bounds and every deck's product and exact value (each landing
    /// factor rises with q_k). The same sums and products in the same order, so this holds for the floats as well
    /// (correctly rounded operations are monotone).
    pub(crate) fn dominates(&self, x: usize, y: usize) -> bool {
        self.veterans[x].at_least_as_good_as(&self.veterans[y])
            && self.parents[x].len() == self.parents[y].len()
            && self.parents[x].iter().zip(&self.parents[y]).all(|(own, theirs)| own.at_least_as_good_as(theirs))
    }

    /// Per veteran y, the veterans x at least as good in every career (`dominates`), ascending. Only veterans of one
    /// character can dominate each other (`dominates` tests the characters first), so the other pairs are skipped
    /// unasked.
    pub(crate) fn dominance_lists(&self) -> Vec<Vec<usize>> {
        let veteran_count = self.veterans.len();
        let mut veterans_by_chara: crate::FxMap<u32, Vec<usize>> = crate::FxMap::default();
        for x in 0..veteran_count {
            veterans_by_chara.entry(self.veterans[x].character_id).or_default().push(x);
        }
        (0..veteran_count)
            .map(|y| {
                veterans_by_chara[&self.veterans[y].character_id]
                    .iter()
                    .copied()
                    .filter(|&x| x != y && self.dominates(x, y))
                    .collect()
            })
            .collect()
    }

    /// Undo one `CareerSetup::pool_push`, keeping all four per-veteran arrays aligned.
    pub(crate) fn pop_veteran(&mut self) {
        self.veterans.pop();
        self.parents.pop();
        self.borrowed.pop();
        self.grandparent_race_affinity.pop();
    }
}

/// Whether parents a and b hold every fixed legacy of `fix` (a job's parent places)
#[inline]
pub(crate) fn holds_fix(fix: &[usize], a: usize, b: usize) -> bool {
    fix.iter().all(|&fixed_legacy| fixed_legacy == a || fixed_legacy == b)
}

/// G1s two legacies both won (expected count when a win is a chance)
pub(crate) fn shared(a: &Who, b: &Who) -> f64 {
    let (mut i, mut j, mut shared_wins) = (0, 0, 0.0f64);
    while i < a.wins.len() && j < b.wins.len() {
        match a.wins[i].race.cmp(&b.wins[j].race) {
            std::cmp::Ordering::Less => i += 1,
            std::cmp::Ordering::Greater => j += 1,
            std::cmp::Ordering::Equal => {
                shared_wins += a.wins[i].chance * b.wins[j].chance;
                i += 1;
                j += 1;
            }
        }
    }
    shared_wins
}

impl CareerSetup<'_> {
    /// The index in Data::umas of character `character_id` (None: a character the data does not list).
    pub(crate) fn uma_index(&self, character_id: u32) -> Option<UmaIndex> {
        self.data.umas.iter().position(|uma| uma.id == character_id).map(UmaIndex)
    }
    /// Stars (0..=3) of the best white spark of each job target among `sparks` ((skill group, stars) pairs), in
    /// job-target order.
    fn job_target_stars(&self, sparks: &[(u32, u8)]) -> Vec<u8> {
        self.skill_job
            .targets
            .iter()
            .map(|target| {
                sparks
                    .iter()
                    .filter(|&&(group, _)| group == target.group)
                    .map(|&(_, stars)| stars.min(3))
                    .max()
                    .unwrap_or(0)
            })
            .collect()
    }
    fn who(&self, traits: &VeteranTraits) -> Who {
        let mut wins: Vec<RaceWin> = if traits.win_chances.is_empty() {
            let mut races = traits.wins.to_vec();
            races.sort_unstable();
            races.dedup();
            races.into_iter().map(|race| RaceWin { race, chance: 1.0 }).collect()
        } else {
            traits
                .win_chances
                .iter()
                .enumerate()
                .filter(|&(_, &chance)| chance > 0.0)
                .map(|(race, &chance)| RaceWin { race, chance })
                .collect()
        };
        wins.sort_by_key(|win| win.race);
        Who {
            uma_index: self.uma_index(traits.character_id),
            character_id: traits.character_id,
            wins,
            stars: std::rc::Rc::new(self.job_target_stars(traits.sparks)),
            pinks: std::rc::Rc::new(traits.pinks.to_vec()),
            pink_chances: std::rc::Rc::new(traits.pink_chances.to_vec()),
            star_mix: traits.star_mix,
        }
    }
    /// `pool` of the legacies so far plus `legacy` (pop with `Pool::pop_veteran`)
    pub(crate) fn pool_push(&self, pool: &mut Pool, legacy: &Legacy) {
        let veteran = self.who(&VeteranTraits::of_legacy(legacy));
        let parents: Vec<Who> =
            legacy.parents.iter().map(|parent| self.who(&VeteranTraits::of_ancestor(parent))).collect();
        pool.grandparent_race_affinity.push(parents.iter().map(|parent| 3.0 * shared(&veteran, parent)).collect());
        pool.veterans.push(veteran);
        pool.parents.push(parents);
        pool.borrowed.push(legacy.borrowed);
    }
    /// the solver's pool of `legacies`: each one's entry as `pool_push` builds it (the one construction, the
    /// simulator's too)
    pub(crate) fn pool(&self, legacies: &[Legacy]) -> Pool {
        let veteran_count = legacies.len();
        let mut pool = Pool {
            veterans: Vec::with_capacity(veteran_count),
            parents: Vec::with_capacity(veteran_count),
            borrowed: Vec::with_capacity(veteran_count),
            grandparent_race_affinity: Vec::with_capacity(veteran_count),
            max_borrowed: self.max_borrowed,
        };
        for legacy in legacies {
            self.pool_push(&mut pool, legacy);
        }
        pool
    }
}

#[cfg(test)]
mod tests {
    use crate::career::Legacy;
    use serde_json::json;

    #[test]
    fn a_legacy_records_at_most_two_parents() {
        let legacy = |parent_count: usize| {
            serde_json::from_value::<Legacy>(
                json!({"chara": 1001, "parents": vec![json!({"chara": 1002}); parent_count]}),
            )
        };
        assert_eq!(legacy(2).unwrap().parents.len(), 2);
        let Err(error) = legacy(3) else { panic!("three parents accepted") };
        assert!(error.to_string().contains("at most 2 parents"), "{error}");
    }
}
