//! Reusable buffers for configuration and node bounds.
use super::{DeckPath, ParentRatioBounds, ParentTargetBounds, SearchConfig};
use crate::deck::{gm_g, gm_w};

/// The node bound's buffers and per-parent rows (`Walk::hopeful`), and the dominance skip's, reused across
/// configurations
pub(in crate::deck::enumerate) struct NodeScratch {
    pub(in crate::deck::enumerate) dw: Vec<f64>,
    pub(in crate::deck::enumerate) dg: Vec<f64>,
    pub(in crate::deck::enumerate) av: Vec<f64>,
    // (the expanded configuration's clause sums and B's odds)
    pub(in crate::deck::enumerate) cvx: Vec<f64>,
    pub(in crate::deck::enumerate) pdwx: Vec<f64>,
    pub(in crate::deck::enumerate) pdgx: Vec<f64>,
    // the node bound's per-configuration constants, flat and reused (see `hopeful`)
    pub(in crate::deck::enumerate) kab: Vec<f64>,
    pub(in crate::deck::enumerate) kr: Vec<f64>,
    pub(in crate::deck::enumerate) kab_have: Vec<bool>,
    pub(in crate::deck::enumerate) kr_have: Vec<bool>,
    pub(in crate::deck::enumerate) hc: Vec<ParentTargetBounds>,
    pub(in crate::deck::enumerate) hb: Vec<ParentRatioBounds>,
    // (a child's touched targets: their odds and the parent's factors they replace)
    pub(in crate::deck::enumerate) tw: [f64; 32],
    pub(in crate::deck::enumerate) tg: [f64; 32],
    pub(in crate::deck::enumerate) told: [f64; 32],
    // (and for B: their odds 1 - pw, 1 - pg, and one row's factors with the touched ones in place)
    pub(in crate::deck::enumerate) bdw: [f64; 32],
    pub(in crate::deck::enumerate) bdg: [f64; 32],
    pub(in crate::deck::enumerate) brow: [f64; 32],
    // (a parent's odds for its A factors, the first nu slots written before each use)
    pub(in crate::deck::enumerate) bw: [f64; 32],
    pub(in crate::deck::enumerate) bg: [f64; 32],
    // (`Walk::dom`'s per-target ppg * rlo: written before read in each call)
    pub(in crate::deck::enumerate) dpr: [f64; 32],
    // (RB_CHECK: the node bounds on the current path, per depth its clause sums, the root's at depth 0)
    pub(in crate::deck::enumerate) rbs: Vec<Vec<f64>>,
}

impl NodeScratch {
    /// Grow the per-parent rows to `nhc` (A) and, with `a_rows`, `nrows` (B)
    pub(in crate::deck::enumerate) fn grow_parent_rows(
        &mut self,
        nhc: usize,
        nrows: usize,
        noq: usize,
        nu: usize,
        a_rows: bool,
    ) {
        while self.hc.len() < nhc {
            self.hc.push(ParentTargetBounds {
                pid: usize::MAX,
                fa: vec![0.0; noq * nu],
                pp: vec![1.0; noq * (nu + 1)],
            });
        }
        if a_rows {
            while self.hb.len() < nrows {
                self.hb.push(ParentRatioBounds {
                    pid: usize::MAX,
                    pdw: [0.0; 32],
                    pdg: [0.0; 32],
                    st: vec![usize::MAX; noq],
                    fb: vec![0.0; noq * nu],
                    pb: vec![1.0; noq * (nu + 1)],
                });
            }
        }
    }

    /// Clear the node bound's per-configuration constants for `nrow` rows (see `Walk::hopeful`)
    pub(in crate::deck::enumerate) fn reset_constants(&mut self, nrow: usize, nu: usize, nc: usize) {
        self.kab.clear();
        self.kab.resize(nrow * 2 * nu, 0.0);
        self.kr.clear();
        self.kr.resize(nrow * nc, 0.0);
        self.kab_have.clear();
        self.kab_have.resize(nrow, false);
        self.kr_have.clear();
        self.kr_have.resize(nrow, false);
    }
}

/// The expanded configuration's tables, flat and reused across configurations
pub(in crate::deck::enumerate) struct ConfigScratch {
    // the suffix tables: per target (g * nu + t) the products of the groups' lowest misses from group g on (white,
    // gold), per (clause, chance, outfit) (g * noq + i) those of the best ratios
    pub(in crate::deck::enumerate) sufw: Vec<f64>,
    pub(in crate::deck::enumerate) sufg: Vec<f64>,
    pub(in crate::deck::enumerate) spf: Vec<f64>,
    // (the class masks: csuf[goff[g] + p] = the `cv_mask`s ORed of group g's entries from position p on and of every
    // later group's)
    pub(in crate::deck::enumerate) csuf: Vec<u64>,
    pub(in crate::deck::enumerate) goff: Vec<usize>,
    // locked cards as bits; avail[g][p]: locked cards with an entry at position >= p of group g or in a later group
    pub(in crate::deck::enumerate) avail: Vec<Vec<u8>>,
    pub(in crate::deck::enumerate) path: DeckPath,
}

impl ConfigScratch {
    /// The suffix tables of `cf` (its groups' key tables `gmin`, `rp`): the same products in the same order as the
    /// per-configuration tables they replace
    pub(in crate::deck::enumerate) fn fill_suffixes(
        &mut self,
        cf: &SearchConfig,
        gmin: &[&[f64]],
        rp: &[&[f64]],
        nu: usize,
        nc: usize,
        size: usize,
    ) {
        let Self { sufw, sufg, spf, .. } = self;
        let ng = cf.gs.len();
        sufw.clear();
        sufw.resize((ng + 1) * nu, 1.0);
        sufg.clear();
        sufg.resize((ng + 1) * nu, 1.0);
        spf.clear();
        spf.resize((ng + 1) * nc, 1.0);
        for g in (0..ng).rev() {
            let (gw, gg) = (gm_w(gmin[g], nu, cf.gs[g].1), gm_g(gmin[g], nu, cf.gs[g].1));
            for t in 0..nu {
                sufw[g * nu + t] = sufw[(g + 1) * nu + t] * gw[t];
                sufg[g * nu + t] = sufg[(g + 1) * nu + t] * gg[t];
            }
        }
        for g in (0..ng).rev() {
            let rpg = rp[g];
            for i in 0..nc {
                spf[g * nc + i] = spf[(g + 1) * nc + i] * rpg[i * (size + 1) + cf.gs[g].1];
            }
        }
    }

    /// The class masks of groups `gs` (entries' `cv_mask`s `cvm`)
    pub(in crate::deck::enumerate) fn fill_class_suffixes(&mut self, gs: &[(&[usize], usize)], cvm: &[u64]) {
        let Self { csuf, goff, .. } = self;
        let ng = gs.len();
        goff.clear();
        goff.push(0);
        for g in 0..ng {
            goff.push(goff[g] + gs[g].0.len());
        }
        csuf.clear();
        csuf.resize(goff[ng] + 1, 0);
        for g in (0..ng).rev() {
            for (p, &j) in gs[g].0.iter().enumerate().rev() {
                csuf[goff[g] + p] = csuf[goff[g] + p + 1] | cvm[j];
            }
        }
    }

    /// The locked-card table of groups `gs`; false when they cannot place every locked card (`all_locked`)
    pub(in crate::deck::enumerate) fn fill_avail(
        &mut self,
        gs: &[(&[usize], usize)],
        lbit: &[u8],
        all_locked: u8,
    ) -> bool {
        let avail = &mut self.avail;
        let ng = gs.len();
        avail.truncate(ng + 1);
        avail.resize_with(ng + 1, Vec::new);
        avail[ng].clear();
        avail[ng].push(0);
        for g in (0..ng).rev() {
            let l = gs[g].0;
            let next = avail[g + 1][0];
            let v = &mut avail[g];
            v.clear();
            v.resize(l.len() + 1, 0u8);
            v[l.len()] = next;
            for p in (0..l.len()).rev() {
                v[p] = v[p + 1] | lbit[l[p]];
            }
        }
        all_locked & !avail[0][0] == 0
    }
}
