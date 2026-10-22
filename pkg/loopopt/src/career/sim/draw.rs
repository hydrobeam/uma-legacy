//! Keyed uniforms and failed-career veteran draws, including the pinned random-number test.
//! Policy selection and path execution live in the sibling modules.

use crate::career::sim::{Cur, Sim};
use crate::career::{Ancestor, Legacy, Lineage};

// ---- random numbers ----
const fn fmix(mut hash: u32) -> u32 {
    hash ^= hash >> 16;
    hash = hash.wrapping_mul(0x85eb_ca6b);
    hash ^= hash >> 13;
    hash = hash.wrapping_mul(0xc2b2_ae35);
    hash ^= hash >> 16;
    hash
}
/// 53 bits in [0, 1) from the keys
fn u01(keys: &[u32]) -> f64 {
    let (mut high_bits, mut low_bits) = (0x9e37_79b9u32, 0x7f4a_7c15u32);
    for &key in keys {
        high_bits = fmix((high_bits ^ key).wrapping_add(0x632b_e5ab));
        low_bits = fmix(low_bits.wrapping_add(key.wrapping_mul(0x27d4_eb2f)) ^ high_bits);
    }
    (f64::from(high_bits) * 2f64.powi(21) + f64::from(low_bits >> 11)) / 2f64.powi(53)
}
const STREAM_SET: u32 = 1;
const STREAM_STAR: u32 = 2;
const STREAM_WIN: u32 = 3;
/// Whose uniforms: a run's own path
#[derive(Clone, Copy)]
pub(super) enum Keys {
    Main { seed: u32, run: u32 },
}
impl Keys {
    pub(super) fn uniform(&self, career_index: usize, stream: u32, item: usize) -> f64 {
        match *self {
            Self::Main { seed, run } => u01(&[seed, run, career_index as u32, stream, item as u32]),
        }
    }
}

impl Sim<'_, '_> {
    /// the veteran a failed career leaves: her landed set drawn from the failed sets, her stars from the
    /// rank's mix, her wins from her schedule's chances
    pub(super) fn draw(&self, career: &Cur, legacies: &[Legacy], keys: Keys, career_index: usize) -> Legacy {
        let failed_probability: f64 = self.failed_sets.iter().map(|&outcome_mask| career.os[outcome_mask]).sum();
        let mut remaining_mass = keys.uniform(career_index, STREAM_SET, 0) * failed_probability;
        let (mut landed_mask, mut last_failed_mask) = (usize::MAX, usize::MAX);
        for &outcome_mask in &self.failed_sets {
            if career.os[outcome_mask] <= 0.0 {
                continue;
            }
            last_failed_mask = outcome_mask;
            if remaining_mass < career.os[outcome_mask] {
                landed_mask = outcome_mask;
                break;
            }
            remaining_mass -= career.os[outcome_mask];
        }
        if landed_mask == usize::MAX {
            landed_mask = last_failed_mask;
        }
        let mut sparks = vec![];
        for (i, target) in self.env.setup.skill_job.targets.iter().enumerate() {
            if landed_mask >> i & 1 == 0 {
                continue;
            }
            let star_draw = keys.uniform(career_index, STREAM_STAR, i);
            let stars = if star_draw < self.star_mix[0] {
                1
            } else if star_draw < self.star_mix[0] + self.star_mix[1] {
                2
            } else {
                3
            };
            sparks.push((target.group, stars));
        }
        let wins: Vec<usize> = career
            .wp
            .iter()
            .enumerate()
            .filter(|&(race_index, &win_chance)| {
                win_chance > 0.0 && keys.uniform(career_index, STREAM_WIN, race_index) < win_chance
            })
            .map(|(race_index, _)| race_index)
            .collect();
        let Lineage { trainee, first_parent, second_parent } = career.lineage;
        Legacy {
            chara: self.d.umas[trainee].id,
            borrowed: false,
            sparks,
            wins,
            pinks: vec![],
            parents: vec![Ancestor::of_legacy(&legacies[first_parent]), Ancestor::of_legacy(&legacies[second_parent])],
            ..Default::default()
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    /// the random numbers are pinned: changing them would change every recorded run
    fn u01_values() {
        assert_eq!(super::u01(&[1, 3, 0, 1, 0]), 0.006_101_473_502_044_041);
        assert_eq!(super::u01(&[1, 0, 17, 2, 5]), 0.786_756_654_747_679);
        assert_eq!(super::u01(&[7, 123_456, 99_999, 3, 33]), 0.923_524_891_523_141_8);
    }
}
