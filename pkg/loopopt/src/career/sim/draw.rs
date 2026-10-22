//! Keyed uniforms and failed-career veteran draws, including the pinned random-number test.
//! Policy selection and path execution live in the sibling modules.

use crate::career::sim::{Cur, Sim};
use crate::career::{Ancestor, Legacy, Lineage};

// ---- random numbers ----
const fn fmix(mut h: u32) -> u32 {
    h ^= h >> 16;
    h = h.wrapping_mul(0x85eb_ca6b);
    h ^= h >> 13;
    h = h.wrapping_mul(0xc2b2_ae35);
    h ^= h >> 16;
    h
}
/// 53 bits in [0, 1) from the keys
fn u01(keys: &[u32]) -> f64 {
    let (mut a, mut b) = (0x9e37_79b9u32, 0x7f4a_7c15u32);
    for &k in keys {
        a = fmix((a ^ k).wrapping_add(0x632b_e5ab));
        b = fmix(b.wrapping_add(k.wrapping_mul(0x27d4_eb2f)) ^ a);
    }
    (f64::from(a) * 2f64.powi(21) + f64::from(b >> 11)) / 2f64.powi(53)
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
    pub(super) fn u(self, t: usize, stream: u32, item: usize) -> f64 {
        match self {
            Self::Main { seed, run } => u01(&[seed, run, t as u32, stream, item as u32]),
        }
    }
}

impl Sim<'_, '_> {
    /// the veteran career number t leaves when it fails: her landed set drawn from the failed sets, her stars from the
    /// rank's mix, her wins from her schedule's chances
    pub(super) fn draw(&self, c: &Cur, legs: &[Legacy], keys: Keys, t: usize) -> Legacy {
        let tot: f64 = self.failed_sets.iter().map(|&m| c.os[m]).sum();
        let mut u = keys.u(t, STREAM_SET, 0) * tot;
        let (mut mask, mut last) = (usize::MAX, usize::MAX);
        for &m in &self.failed_sets {
            if c.os[m] <= 0.0 {
                continue;
            }
            last = m;
            if u < c.os[m] {
                mask = m;
                break;
            }
            u -= c.os[m];
        }
        if mask == usize::MAX {
            mask = last;
        }
        let mut sparks = vec![];
        for (i, tg) in self.env.skill_job.targets.iter().enumerate() {
            if mask >> i & 1 == 0 {
                continue;
            }
            let x = keys.u(t, STREAM_STAR, i);
            let s = if x < self.star_mix[0] {
                1
            } else if x < self.star_mix[0] + self.star_mix[1] {
                2
            } else {
                3
            };
            sparks.push((tg.group, s));
        }
        let wins: Vec<usize> =
            c.wp.iter()
                .enumerate()
                .filter(|&(ri, &p)| p > 0.0 && keys.u(t, STREAM_WIN, ri) < p)
                .map(|(ri, _)| ri)
                .collect();
        let Lineage { trainee, first_parent, second_parent } = c.lineage;
        Legacy {
            chara: self.d.umas[trainee].id,
            borrowed: false,
            sparks,
            wins,
            pinks: vec![],
            parents: vec![Ancestor::of_legacy(&legs[first_parent]), Ancestor::of_legacy(&legs[second_parent])],
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
