//! The model constants scripts read, printed as JSON by the native `loopopt constants`: each field is the constant the
//! solver itself uses (read here, never a copy), so a script that takes them from this command cannot drift from the
//! build. Native only; the page's copy is generated from it (`webapp/src/gen/constants.ts`, `crate::ts_gen`).
use crate::codes::{Aptitude, Grade, Kind};
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
    /// what each aptitude is of, by its code (codes.rs `Aptitude::kind`): where its grade is in a `Grades`
    aptitude_kinds: AptitudeKinds,
    /// what each grade costs a race's aptitude basis, by its code (codes.rs `Grade::cost`)
    grade_cost: GradeCosts,
}

/// `Aptitude::kind` of each aptitude, keyed by its code (a field per aptitude, so the page indexes it by `Aptitude`).
#[derive(Serialize, ts_rs::TS)]
struct AptitudeKinds {
    #[serde(rename = "11")]
    turf: Kind,
    #[serde(rename = "12")]
    dirt: Kind,
    #[serde(rename = "21")]
    front_runner: Kind,
    #[serde(rename = "22")]
    pace_chaser: Kind,
    #[serde(rename = "23")]
    late_surger: Kind,
    #[serde(rename = "24")]
    end_closer: Kind,
    #[serde(rename = "31")]
    sprint: Kind,
    #[serde(rename = "32")]
    mile: Kind,
    #[serde(rename = "33")]
    medium: Kind,
    #[serde(rename = "34")]
    long: Kind,
}

/// `Grade::cost` of each grade, keyed by its code (a field per grade, so the page indexes it by `Grade`).
#[derive(Serialize, ts_rs::TS)]
struct GradeCosts {
    #[serde(rename = "1")]
    g: i32,
    #[serde(rename = "2")]
    f: i32,
    #[serde(rename = "3")]
    e: i32,
    #[serde(rename = "4")]
    d: i32,
    #[serde(rename = "5")]
    c: i32,
    #[serde(rename = "6")]
    b: i32,
    #[serde(rename = "7")]
    a: i32,
    #[serde(rename = "8")]
    s: i32,
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
            aptitude_kinds: AptitudeKinds {
                turf: Aptitude::Turf.kind(),
                dirt: Aptitude::Dirt.kind(),
                front_runner: Aptitude::FrontRunner.kind(),
                pace_chaser: Aptitude::PaceChaser.kind(),
                late_surger: Aptitude::LateSurger.kind(),
                end_closer: Aptitude::EndCloser.kind(),
                sprint: Aptitude::Sprint.kind(),
                mile: Aptitude::Mile.kind(),
                medium: Aptitude::Medium.kind(),
                long: Aptitude::Long.kind(),
            },
            grade_cost: GradeCosts {
                g: Grade::G.cost(),
                f: Grade::F.cost(),
                e: Grade::E.cost(),
                d: Grade::D.cost(),
                c: Grade::C.cost(),
                b: Grade::B.cost(),
                a: Grade::A.cost(),
                s: Grade::S.cost(),
            },
        }
    }
}
