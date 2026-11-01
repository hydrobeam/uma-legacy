//! The model constants scripts read, printed as JSON by the native `loopopt constants`: each field is the constant the
//! solver itself uses (read here, never a copy), so a script that takes them from this command cannot drift from the
//! build. Native only; the page's copy is generated from it (`webapp/src/gen/constants.ts`, `crate::ts_gen`).
use serde::Serialize;

/// The model constants `loopopt constants` prints.
#[derive(Serialize, ts_rs::TS)]
pub(crate) struct SolverConstants {
    /// chain events per run: Binomial(`chain_n`, p) with logit p = `chain_a` + `chain_b` x pals (deck/chains.rs)
    chain_n: u32,
    chain_a: f64,
    chain_b: f64,
    /// the share of its firings on which a card-event stop risk ends its chain, [k, n] (deck/chains.rs)
    chain_stop_share: (u8, u8),
    /// the fewest targets at which the plan searches its builders exactly (career/plan/candidates.rs)
    exact_builders_targets: usize,
    /// a candidate's effective ceiling as a share of her full-loop odds (career/plan/cont.rs)
    ceil_share: f64,
    /// the exponent of the lift chance's decline as her odds near that ceiling (career/plan/cont.rs)
    lam_decay: f64,
    /// the 1/2/3-star shares of a lineage's sparks at each rank class a job may name (`RankClass::star_mix`,
    /// skills.rs; a job is below SS unless it says otherwise)
    star_mix: StarMixes,
    /// a gold / white skill's chance to become a spark on the veteran when all six ancestors carry it (skills.rs)
    gen_gold: f64,
    gen_white: f64,
    /// one inspiration's base chance per spark star (skills.rs)
    insp_per_star: f64,
}

/// Each rank class's star mix, by its name in a job (`RankClass`).
#[derive(Serialize, ts_rs::TS)]
struct StarMixes {
    below_ss: [f64; 3],
    ss_and_up: [f64; 3],
}

impl SolverConstants {
    /// The constants as this build has them.
    pub(crate) const fn now() -> Self {
        use crate::deck::chains::{CHAIN_A, CHAIN_B, CHAIN_N, CHAIN_STOP_SHARE};
        use crate::skills::RankClass;
        Self {
            chain_n: CHAIN_N,
            chain_a: CHAIN_A,
            chain_b: CHAIN_B,
            chain_stop_share: CHAIN_STOP_SHARE,
            exact_builders_targets: crate::career::EXACT_BUILDERS_TARGETS,
            ceil_share: crate::career::CEIL_SHARE,
            lam_decay: crate::career::LAM_DECAY,
            star_mix: StarMixes { below_ss: RankClass::BelowSs.star_mix(), ss_and_up: RankClass::SsAndUp.star_mix() },
            gen_gold: crate::skills::GEN_GOLD,
            gen_white: crate::skills::GEN_WHITE,
            insp_per_star: crate::skills::INSP_PER_STAR,
        }
    }
}
