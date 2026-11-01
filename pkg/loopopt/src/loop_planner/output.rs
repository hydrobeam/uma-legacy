//! What a deck job answers (`DeckReply`): the loop report, the deck skyline alone (`deck_only`), or a refusal. Built by
//! `job::run_deck` and `search::report`; the page's types are generated from these (`crate::ts_gen`). A report owns
//! what it holds (names and labels copied once per reported loop): it outlives the search and the job's model.
use crate::chart::{LoopMember, ScheduleRow};
use crate::deck::pdeck::{Name, PDeck, ResultDeck};
use crate::refusal::Refusal;
use crate::skills::AffinityDetail;
use serde::Serialize;
use ts_rs::TS;

/// A deck job's answer, by its `kind`.
#[derive(Serialize, TS)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum DeckReply {
    Refusal(Refusal),
    /// `deck_only`: the nondominated decks, before any loop search
    Skyline(Skyline),
    Loops(Box<LoopReport>),
}

/// The nondominated deck frontier of a `deck_only` job.
#[derive(Serialize, TS)]
pub(crate) struct Skyline {
    pub(crate) sky: Vec<PDeck>,
}

/// The deck enumeration's counts, reported with the loops.
#[derive(Clone, Serialize, TS)]
pub(crate) struct DeckStats {
    /// decks the enumeration built
    pub(crate) decks_built: usize,
    /// decks on its nondominated frontier
    pub(crate) decks_on_frontier: usize,
    /// cards that can hint a target
    pub(crate) relevant_cards: usize,
    /// cards another card dominates, dropped before the enumeration
    pub(crate) cards_dropped: usize,
    /// decks one card swap made from another
    pub(crate) decks_swapped: usize,
    /// the plain pals (pals that hint no target), by name
    pub(crate) plain_pals: Vec<String>,
}

/// The loop search's report: the best loops with their decks, the search's counts, and what the answer rests on.
#[derive(Serialize, TS)]
pub(crate) struct LoopReport {
    /// the deck enumeration's counts
    pub(crate) stats: DeckStats,
    /// the best loops, best first
    pub(crate) top: Vec<LoopResult>,
    /// a deck per member (false: one deck for the loop)
    pub(crate) per_uma: bool,
    /// loop structures the job's sizes and required umas allow
    pub(crate) foursomes: usize,
    /// structures ranked by their ceiling
    pub(crate) foursomes_ranked: usize,
    /// structures whose ceiling was read
    pub(crate) foursomes_scanned: usize,
    /// structures whose schedules were solved
    pub(crate) foursomes_solved: usize,
    /// the ceiling of everything left unsolved (0: nothing left)
    pub(crate) next_bound: f64,
    /// the search ended by its stop rule: no unexamined loop can win
    pub(crate) proven: bool,
    /// deck searches run: the first, and one per band
    pub(crate) searches: usize,
    pub(crate) exact: ExactSummary,
}

/// One result of the loop search: the loop and its deck.
#[derive(Clone, Serialize, TS)]
pub(crate) struct LoopResult {
    pub(crate) r#loop: ResultLoop,
    /// the deck most members share (one deck for the loop: that deck)
    pub(crate) deck: ResultDeck,
    /// per member, when each has her own
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub(crate) decks: Option<Vec<ResultDeck>>,
}

/// A loop in a result; member k is `members[k]`, with `outfits[k]` and `per_trainee[k]`.
#[derive(Clone, Serialize, TS)]
pub(crate) struct ResultLoop {
    pub(crate) members: Vec<LoopMember>,
    /// the chance every member lands every target, averaged over the loop's careers (what the results are ranked by)
    pub(crate) p_all: f64,
    /// the chance each lands all but at most one
    pub(crate) p_minus1: f64,
    /// the chance of landing exactly k targets, k = 0 ..= n, averaged over the loop's careers; null above the exact
    /// engine's landed-set limit
    pub(crate) landed: Option<Vec<f64>>,
    /// the search's product value of this loop with these decks
    pub(crate) rank_value: f64,
    /// per member: her outfits that cover a target, and what they cover
    pub(crate) outfits: Vec<MemberOutfits>,
    /// per member: her chance of landing every target
    pub(crate) per_trainee: Vec<f64>,
    /// the mean inheritance chance per career
    pub(crate) mean_proc: f64,
    /// the race schedule
    pub(crate) schedule: Vec<ScheduleRow>,
    /// the affinity detail
    pub(crate) detail: AffinityDetail,
}

/// A member's outfit choice: the outfits that give it (none when it covers no target) and what it covers.
#[derive(Clone, Serialize, TS)]
pub(crate) struct MemberOutfits {
    pub(crate) outfits: Vec<OutfitLabel>,
    pub(crate) covers: Vec<TargetCover>,
}

/// An outfit by card id and label (id 0: any outfit).
#[derive(Clone, Serialize, TS)]
pub(crate) struct OutfitLabel {
    pub(crate) id: u32,
    pub(crate) label: String,
}

/// What an outfit gives of a target: `level` 1 its white skill, 2 its gold.
#[derive(Clone, Serialize, TS)]
pub(crate) struct TargetCover {
    pub(crate) group: u32,
    pub(crate) level: u8,
}

/// What the answer rests on.
#[derive(Clone, Serialize, TS)]
pub(crate) struct ExactSummary {
    /// the highest exact / product odds cap certified
    pub(crate) rho: f64,
    /// the cap sought
    pub(crate) rho_target: f64,
    /// the assumed cap per structure class (each certified at most at `rho`)
    pub(crate) class_caps: [f64; 4],
    /// the largest exact / product odds seen on any deck scored here
    pub(crate) max_ratio: f64,
    /// the same per structure class
    pub(crate) class_max_ratio: [f64; 4],
    /// per structure class, the deck that reached `class_max_ratio` (null: none scored)
    pub(crate) class_max_deck: [Option<RatioDeck>; 4],
    /// the search ended by its stop rule (no unexamined loop or deck can win under the certified caps)
    pub(crate) complete: bool,
    pub(crate) decks_scored: usize,
    pub(crate) deck_searches: usize,
    pub(crate) loops_solved: usize,
    pub(crate) schedule_rejects: usize,
    /// a scored deck reached its class's assumed cap
    pub(crate) cap_hit: bool,
    /// an exact search stopped below the cap sought (its budget ran out)
    pub(crate) budget_out: bool,
    pub(crate) bound_filtered: usize,
}

/// A deck by its trainee and cards.
#[derive(Clone, Serialize, TS)]
pub(crate) struct RatioDeck {
    pub(crate) trainee: u32,
    pub(crate) cards: Vec<Name>,
}
