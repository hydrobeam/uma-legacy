//! Joint deck + loop search for skill targets.
//!
//! Deck model (per training run):
//!   - training hints: each card gives h = base(scenario) x (1 + hint_frequency/100) draws, each uniform over its
//!     skill options still below Lv 5 (the stat option is never drawn; a skill at Lv 5 leaves the pool, so a card
//!     giving +5 levels per draw never repeats one), `hint_miss`; a card's draws hint two of its targets together less
//!     often than independent odds would say;
//!   - chain events: the number in a run is Binomial(CHAIN_N, CHAIN_P); each advances a chain picked among the
//!     remaining chain slots of the non-pal cards (a card with more links left is picked more often; links fire in
//!     order). Pals and R cards have no chain slots. Both fitted on uma-it Independent Training runs (docs/MODEL.md);
//!   - event rewards come from the job (`events`): (card, skill group, 1 white | 2 gold, link; 0 = whenever the
//!     card is in the deck, e.g. pal golds).
//! Deck hint odds per target: 1 - prod(1 - p) over sources, separately for white and gold.
//!
//! Search (exact for the model under the assumed caps `EXACT_CAPS`): race schedules maximise affinity and do not
//! depend on the deck. Every deck legal for a trainee (the character rule included) is enumerated by chain-kind key
//! (`DeckSpace`) and ranked by the product of its per-target odds; decks and loops that could still win are scored
//! exactly (`joint_fast`), bounded by their structure class's cap on exact over product odds. Four-uma loops and
//! three-uma rotations (`sizes`) are generated in bands of a separable bound and solved (schedules, then exact
//! values) only while a ceiling (pairwise affinity ceilings first, then the structure's own schedules, with per-uma
//! tables of exact values by inheritance chance) can still beat the k-th best. A pass over its work budget falls back
//! to lower caps, then to product odds alone (`exact.rho`). Per-uma decks (the default, `run_multi`) give each trainee
//! her own best legal deck; a shared deck must be legal for every member. docs/SOLVER.md "Deck + loop search".
use super::*;
use crate::skills::{chart_rows, check_sizes, lins, member_json, Model, SkillJob, SkillsRaw, GEN_GOLD};
use serde_json::{json, Value};
use std::collections::HashMap;

#[derive(Deserialize, Clone)]
pub(crate) struct Card { pub id: u32, pub name: String, pub chara: u32, pub rarity: u8, pub pal: bool, pub links: u8, pub hf: f64,
    /// specialty priority at max limit break (unique effect included): more training hints (`hint_spec`)
    #[serde(default)] pub spec: f64,
    /// training-hint options that are skills (the stat option is never drawn in Independent Training)
    #[serde(default)] pub nskill: u32, pub whites: Vec<u32>,
    /// hint levels per training-hint draw at max limit break (1 + hint-level effect); a skill at Lv 5 leaves the pool
    #[serde(default = "d_hinc")] pub hinc: u32,
    /// characters who cannot train with this card: its own character and, for a group card, every member
    #[serde(default)] pub charas: Vec<u32> }
impl Card {
    /// The character rule. `RULE_OFF=1` (brute-force checks only, val/permcheck.py and owncheck.py) switches it off.
    pub(crate) fn blocks(&self, uma: u32) -> bool {
        static OFF: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
        !*OFF.get_or_init(|| std::env::var("RULE_OFF").is_ok()) && (self.chara == uma || self.charas.contains(&uma))
    }
}
#[derive(Deserialize)]
pub(crate) struct CardsRaw { pub cards: Vec<Card>,
    /// cards the game bans from a scenario's deck, by scenario name (master.mdb single_mode_restrict_support via
    /// export_cards.py: Team Sirius in "unity" and "gc")
    #[serde(default)] pub restrict: HashMap<String, Vec<u32>> }

#[derive(Deserialize)]
pub(crate) struct DeckJob {
    /// chain events per run: Binomial(n, p); defaults CHAIN_N, CHAIN_P (for experiments)
    #[serde(default)] pub chain_events: Option<(u32, f64)>,
    /// Independent Training scenario: "unity" (default), "gc" (Grand Concert), "ura", "trackblazer", "grandmasters" (JP); sets how many
    /// training-hint draws a card gives (`hint_base`)
    #[serde(default = "d_scenario")] pub scenario: String,
    #[serde(default = "d_size")] pub deck_size: usize,
    /// (card id, skill group, 1 white | 2 gold, chain link 1..; 0 = whenever the card is in the deck)
    #[serde(default)] pub events: Vec<(u32, u32, u8, u8)>,
    /// chain events whose reward depends on the outcome: (card, group, level, link, chance given the link fires)
    #[serde(default)] pub events_p: Vec<(u32, u32, u8, u8, f64)>,
    /// non-chain events with a fixed chance per run when the card is in the deck: (card, group, 1 | 2, chance)
    #[serde(default)] pub odds_events: Vec<(u32, u32, u8, f64)>,
    /// choices that end a card's chain: (card, link) for every such choice, and their skill rewards
    /// (card, link, group, 1 | 2, chance given the link fires)
    #[serde(default)] pub end_links: Vec<(u32, u8)>,
    #[serde(default)] pub ends: Vec<(u32, u8, u32, u8, f64)>,
    /// links where IT's default choice ends a card's chain on some outcomes: (card, link, k outcomes, of n)
    #[serde(default)] pub stop_risk: Vec<(u32, u8, u8, u8)>,
    /// events that name two or more skills, in full: (card, link, [choice][outcome] -> (skill group, 1 | 2)), the
    /// choices that keep the chain going, outcomes equally likely. At these links they replace `events` /
    /// `events_p`, which credit every skill on its own.
    #[serde(default)] pub alts: Vec<(u32, u8, Vec<Vec<Vec<(u32, u8)>>>)>,
    /// measured outcome weights for `alts` events: (card, link, [choice][outcome] -> share); an event not listed here
    /// keeps equally likely outcomes (event_shares.json, applied by the event builders)
    #[serde(default)] pub alt_w: Vec<(u32, u8, Vec<Vec<f64>>)>,
    #[serde(default)] pub owned_cards: Vec<u32>,
    /// allow one card the player does not own (a friend's card) in the deck
    #[serde(default)] pub borrow: bool,
    #[serde(default)] pub lock_cards: Vec<u32>,
    #[serde(default)] pub exclude_cards: Vec<u32>,
    /// lowest rarity allowed for cards with chain events (R cards have none and always count: no chain slots)
    #[serde(default = "d_rar")] pub min_rarity: u8,
    /// only decks with exactly this many pals
    #[serde(default)] pub pals: Option<usize>,
    /// how many distinct decks to return (each with its best loop)
    #[serde(default = "d_one")] pub top_decks: usize,
    /// experiment: fixed per-uma race schedules instead of loop-optimised ones. "solo" = each uma maximises her
    /// own expected race wins; "goals" = career objectives only; "none" = shared wins count for nothing.
    #[serde(default)] pub schedule: Option<String>,
    /// one deck per uma instead of one shared deck: each trainee runs her own best legal deck (across her three
    /// lineages); `top_decks` then counts loops
    #[serde(default)] pub per_uma: bool,
    /// internal: return the skyline decks only (used for a uma's own-card-free deck set in per-uma mode)
    #[serde(default)] pub deck_only: bool,
    /// per-uma: rank loops and decks by exact odds (EXACT_RHO); false = prune by product odds, as a fast first pass
    /// (what is reported is still scored exactly, `exact.complete` false)
    #[serde(default = "d_true")] pub exact: bool,
    /// audit: every class cap (`EXACT_CAPS`) at 2.0, so a deck whose exact / product exceeds its class's cap is still
    /// found and reported (`val/capaudit.py`; slow)
    #[serde(default)] pub cap_audit: bool,
}

/// Fixed schedule for one uma (see DeckJob::schedule).
fn prebuilt(d: &Data, u: &Uma, mode: &str) -> Sched {
    let solo = best_response(d, u, &vec![1.0; d.nr], None).expect("infeasible goals").0;
    if mode != "goals" { return solo; }
    let mut s = solo; let mut mask = 0u32;
    for t in 0..NT {
        if let Choice::Opt(o) = s[t] {
            let b = u.bits[t][o as usize];
            if u.forced[t].is_some() || b & !mask != 0 { mask |= b; } else { s[t] = Choice::Rest; }
        }
    }
    if feasible(u, &s) { s } else { solo }
}
fn d_one() -> usize { 1 }
fn d_true() -> bool { true }
/// Skyline depth of the deck enumeration: 1 (the exact search scores every deck it needs from `DeckSpace`)
fn kmax_top(_dj: &DeckJob) -> usize { 1 }
fn d_scenario() -> String { "unity".into() }
fn d_hinc() -> u32 { 1 }
fn d_size() -> usize { 6 }
fn d_rar() -> u8 { 2 }   // SR: R cards have no chain events, so this only filters chain cards

mod hints;
mod chains;
mod sources;
mod joint;
mod caps;
mod kept;
mod space;
mod enumerate;
mod loops;
mod multi;
#[cfg(test)]
mod tests;
use self::{caps::*, chains::*, hints::*, joint::*, kept::*, multi::*, sources::*, space::*};
pub(crate) use self::caps::JCOUNT;
pub(crate) use self::joint::{SBCHK, SBN, SB_ON, SB_USED};
pub(crate) use self::enumerate::RBCHK;
pub(crate) use self::loops::run_deck;
pub(crate) use self::sources::apply_events_db;

mod career;
pub(crate) use career::run_career;
#[cfg(target_arch = "wasm32")]
pub(crate) use career::forget_kept_decks;

