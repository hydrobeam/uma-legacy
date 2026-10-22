//! Deck-job setup and enumeration of the nondominated deck frontier (`run_deck`).
//! Loop structures, schedules and exact deck searches belong to `loop_search` and `enumerate`.
use crate::Instant;
use crate::data::Data;
use crate::deck::chains::{ChainEvents, ChainKind, chain_marginals};
use crate::deck::dominance::{
    DeckEval, DominanceIndex, Name, PDeck, dominates, k_filter, kfilter_order, sig_of, tie_rank,
};
use crate::deck::loop_search::{LoopJobInputs, run_multi};
use crate::deck::sources::{Entries, EventOutcome, Sources, TargetReward, TargetSources, TrainingHint, deck_entries};
use crate::deck::space::deck_space;
use crate::deck::{CardsRaw, DeckJob};
use crate::skills::{Model, SkillJob, SkillsRaw};
use serde::Deserialize;
use serde_json::{Value, json};
use std::collections::HashMap;
/// Deck + loop search. The character rule (a trainee cannot run a card of her own character, nor a group card she is
/// a member of, `Card::blocks`) is part of which decks are legal in the exact search (`DeckSpace`): per trainee with
/// per-uma decks, for every structure member with a shared deck.
pub(crate) fn run_deck(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    run_deck_relaxed(d, sk, cr, v)
}

pub(super) fn run_deck_relaxed(d: &Data, sk: &SkillsRaw, cr: &CardsRaw, v: &Value) -> Value {
    let sj: SkillJob = match SkillJob::deserialize(v) {
        Ok(x) => x,
        Err(e) => return json!({"error": e.to_string()}),
    };
    let dj: DeckJob = match DeckJob::deserialize(v) {
        Ok(x) => x,
        Err(e) => return json!({"error": e.to_string()}),
    };
    let model = Model::new(d, sk, &sj);
    let unc: Vec<u32> = model.uncertain.iter().map(|&i| sj.targets[i].group).collect();
    let target_count = unc.len();
    let pool_ids: Vec<u32> = d
        .umas
        .iter()
        .map(|u| u.id)
        .filter(|&id| {
            (sj.owned.is_empty() || sj.owned.contains(&id) || sj.require.contains(&id)) && !sj.exclude.contains(&id)
        })
        .collect();
    let mut ent = match deck_entries(cr, &dj, &unc, &pool_ids) {
        Ok(e) => e,
        Err(e) => return e,
    };
    // every entry, before card dominance drops any: the exact per-uma search enumerates decks from these
    let space = (!dj.deck_only).then(|| deck_space(cr, &dj, sk, &unc, &ent));
    let source_builder = Sources { dj: &dj, unc: &unc };
    let n_card_dropped = ent.drop_dominated_cards(&dj, target_count);
    let coordinates = dominance_coords(&ent.srcs, target_count);
    let en = match DeckEnumerator::new(&ent, &dj, target_count, &coordinates) {
        Ok(x) => x,
        Err(e) => return e,
    };
    let t0 = Instant::now();
    let mut ct = EnumerationCounts::default();
    // the decks no other deck beats on every dominance coordinate (the exact search scores every deck it needs from
    // `DeckSpace`, so one skyline serves any `top_decks`)
    let front: Vec<DeckEval> = en.run_enum(FrontierOptions { maximum_dominators: 1, first_frontier: None }, &mut ct);
    if dj.checks.deck_debug {
        eprintln!(
            "tests {} scans {} avg_lim {:.0} dom_in_scan {} ring {} swapped {} subtrees {} s1 {}",
            ct.dominance_tests,
            ct.full_scans,
            0.0,
            ct.dominated_decks,
            ct.recent_rejections,
            ct.decks_swapped,
            ct.subtrees_pruned,
            ct.first_frontier_rejections
        );
    }
    if dj.checks.deck_debug {
        eprintln!("rel {} decks {} enum {:.2}s", en.entry_count, ct.decks_built, t0.elapsed().as_secs_f64());
    }
    if dj.checks.deck_debug {
        eprintln!("front {} pareto {:.2}s", front.len(), t0.elapsed().as_secs_f64());
    }
    if front.is_empty() {
        return json!({"error": "No deck can be built from the allowed cards."});
    }
    let sky = skyline(&front);
    if dj.checks.deck_debug {
        eprintln!("skyline {} of {}", sky.len(), front.len());
    }
    let n_front = front.len();
    let pd = detach(front, &sky, &ent, &source_builder, sk);
    if dj.deck_only {
        return json!({"sky": pd});
    }
    let stats = json!({"decks_built": ct.decks_built, "decks_on_frontier": n_front, "relevant_cards": en.entry_count, "cards_dropped": n_card_dropped,
                       "decks_swapped": ct.decks_swapped, "plain_pals": ent.plain_names});
    let inputs = LoopJobInputs {
        data: d,
        cards: cr,
        model: &model,
        skill_job: &sj,
        deck_job: &dj,
        skyline: &pd,
        deck_space: space.as_ref().unwrap(),
    };
    run_multi(inputs, stats)
}

impl DeckJob {
    /// Whether the player owns a card (a locked card counts as owned).
    fn is_owned(&self, id: u32) -> bool {
        self.owned_cards.is_empty() || self.owned_cards.contains(&id) || self.lock_cards.contains(&id)
    }
}

/// Calls `out` on every multiset of at most `left` values from `vals[start..]` (as index-ordered lists)
fn multisets(
    vals: &[ChainKind],
    start: usize,
    left: usize,
    cur: &mut Vec<ChainKind>,
    out: &mut dyn FnMut(&[ChainKind]),
) {
    out(cur);
    if left == 0 {
        return;
    }
    for i in start..vals.len() {
        cur.push(vals[i]);
        multisets(vals, i, left - 1, cur, out);
        cur.pop();
    }
}

impl Entries<'_> {
    /// ---- card dominance (exact) ----
    /// Card A can stand in for card B in the product search when both have the same chain kind (so the
    /// deck's chain odds do not change), A is owned whenever B is, and A at its weakest still beats B at its
    /// strongest on every target: A's miss chances with its chain links at their lowest possible odds (the most
    /// crowded deck) are no higher than B's with every link firing, for both "no gold hint" and "no hint at all".
    /// Swapping B for A then gives a deck at least as good on every dominance coordinate, with the same pal
    /// count when required. The swap is blocked if B is locked or A shares a character with one of the other
    /// deck_size - 1 cards, so B is dropped when kept dominators with at least deck_size - 1 + top_decks
    /// distinct characters exist: every deck with B then has top_decks distinct kept decks at least as good
    /// (swap the deck's other dropped cards first, then B for each unblocked dominator).
    /// Drops them from every entry column; how many it dropped.
    fn drop_dominated_cards(&mut self, deck_job: &DeckJob, target_count: usize) -> usize {
        let mut dropped_count = 0usize;
        // Pals and R cards both take no chain slot, so they can stand in for each other unless the job fixes the pal
        // count
        let pal_fixed = deck_job.pals.is_some();
        let original_count = self.rel.len();
        let bounds = miss_bounds(
            deck_job.chain_events(),
            &self.kind,
            &self.srcs,
            deck_job.deck_size.saturating_sub(1),
            target_count,
        );
        let strength = |j: usize| {
            (0..target_count)
                .map(|k| 2.0 - bounds.strongest[j].no_gold[k] - bounds.strongest[j].no_hint[k])
                .sum::<f64>()
        };
        let mut order: Vec<usize> = (0..original_count).collect();
        let tie_ranks = tie_rank(&self.rel, &self.cards);
        order.sort_by(|&dominator, &candidate| {
            strength(candidate)
                .partial_cmp(&strength(dominator))
                .unwrap()
                .then(tie_ranks[dominator].cmp(&tie_ranks[candidate]))
        });
        let need = deck_job.deck_size.saturating_sub(1) + 1;
        let mut keep = vec![true; original_count];
        let mut visited: Vec<usize> = vec![];
        for &candidate in &order {
            let candidate_card = self.cards[self.rel[candidate]];
            if !self.locked.contains(&self.rel[candidate]) {
                let mut characters: Vec<u32> = vec![];
                for &dominator in &visited {
                    if !keep[dominator] {
                        continue;
                    }
                    let dominator_card = self.cards[self.rel[dominator]];
                    if self.kind[dominator] != self.kind[candidate]
                        || (pal_fixed && dominator_card.pal != candidate_card.pal)
                    {
                        continue;
                    }
                    if deck_job.is_owned(candidate_card.id) && !deck_job.is_owned(dominator_card.id) {
                        continue;
                    }
                    if !(0..target_count).all(|k| {
                        bounds.weakest[dominator].no_gold[k] <= bounds.strongest[candidate].no_gold[k]
                            && bounds.weakest[dominator].no_hint[k] <= bounds.strongest[candidate].no_hint[k]
                    }) {
                        continue;
                    }
                    if !characters.contains(&dominator_card.chara) {
                        characters.push(dominator_card.chara);
                    }
                    if characters.len() >= need {
                        break;
                    }
                }
                if characters.len() >= need {
                    keep[candidate] = false;
                    dropped_count += 1;
                }
            }
            visited.push(candidate);
        }
        if deck_job.checks.deck_debug {
            for j in 0..original_count {
                if !keep[j] {
                    eprintln!("  drop {}", self.cards[self.rel[j]].name);
                }
            }
        }
        if deck_job.checks.deck_debug_pairs {
            debug_robust(self, &keep, &bounds, &tie_ranks, pal_fixed, target_count);
        }
        // All entry columns use the same original-index mask and keep their relative order.
        retain_entries(&mut self.rel, &keep);
        retain_entries(&mut self.srcs, &keep);
        retain_entries(&mut self.kind, &keep);
        retain_entries(&mut self.endk, &keep);
        retain_entries(&mut self.vark, &keep);
        dropped_count
    }
}

/// Each entry's (miss gold, miss any) per target at its strongest (every link firing) and weakest (its links at their
/// lowest odds over any `others` other chain cards)
fn miss_bounds(
    events: ChainEvents,
    kind: &[ChainKind],
    srcs: &[TargetSources],
    others: usize,
    target_count: usize,
) -> MissBounds {
    let nrel0 = kind.len();
    let mut kindset: Vec<ChainKind> = (0..nrel0).filter(|&j| kind[j] != ChainKind::NONE).map(|j| kind[j]).collect();
    kindset.push(ChainKind::FILLER);
    kindset.sort_unstable();
    kindset.dedup();
    // lowest odds of each link over every possible set of other chain cards
    let needs_marg = |key_data: ChainKind| {
        (0..nrel0).any(|j| kind[j] != ChainKind::NONE && kind[j] == key_data && !srcs[j].chain_outcomes.is_empty())
    };
    let mut min_marg: HashMap<ChainKind, Vec<f64>> = HashMap::new();
    for &key_data in &kindset {
        if !needs_marg(key_data) {
            continue;
        }
        let mut mins = vec![1.0f64; key_data.stop as usize];
        multisets(&kindset, 0, others, &mut vec![], &mut |o| {
            let mut key = vec![key_data];
            key.extend_from_slice(o);
            let m = chain_marginals(events, &key);
            for k in 0..key_data.stop as usize {
                mins[k] = mins[k].min(m[&key_data][k]);
            }
        });
        min_marg.insert(key_data, mins);
    }
    // per card: (miss gold, miss any) per target, at its strongest (lo) and weakest (hi)
    let miss = |j: usize, strongest: bool| -> MissProbabilities {
        let stop = kind[j].stop as usize;
        let m: Vec<f64> = if strongest {
            vec![1.0; stop]
        } else {
            min_marg.get(&kind[j]).cloned().unwrap_or_else(|| vec![0.0; stop])
        };
        let mut mw = vec![1.0f64; target_count];
        let mut mg = vec![1.0f64; target_count];
        srcs[j].src_miss(&m, &mut mw, &mut mg);
        let mh: Vec<f64> = (0..target_count).map(|k| mg[k] * mw[k]).collect();
        MissProbabilities { no_gold: mg, no_hint: mh }
    };
    let lo: Vec<MissProbabilities> = (0..nrel0).map(|j| miss(j, true)).collect();
    let hi: Vec<MissProbabilities> = (0..nrel0).map(|j| miss(j, false)).collect();
    MissBounds { strongest: lo, weakest: hi }
}
/// per entry: (miss gold, miss any) per target
type MissTab = Vec<MissProbabilities>;

struct MissProbabilities {
    /// One entry per uncertain target, in model order.
    no_gold: Vec<f64>,
    no_hint: Vec<f64>,
}

/// Per-entry miss probabilities under the strongest and weakest possible chain progress.
struct MissBounds {
    strongest: MissTab,
    weakest: MissTab,
}

/// DECK_DEBUG_PAIRS: per kept card, its robust dominators among the kept ones (the same test as the drop, exact ties
/// by `tie_rank`)
fn debug_robust(
    entries: &Entries,
    keep: &[bool],
    bounds: &MissBounds,
    tr: &[usize],
    pal_fixed: bool,
    target_count: usize,
) {
    let nrel0 = entries.rel.len();
    // Per kept card, robust dominators reported by DECK_DEBUG_PAIRS (same test as above).
    let kept_old: Vec<usize> = (0..nrel0).filter(|&j| keep[j]).collect();
    let robust: Vec<Vec<usize>> = kept_old
        .iter()
        .map(|&b| {
            let cb = entries.cards[entries.rel[b]];
            kept_old
                .iter()
                .enumerate()
                .filter(|&(_, &a)| {
                    if a == b {
                        return false;
                    }
                    let ca = entries.cards[entries.rel[a]];
                    if entries.kind[a] != entries.kind[b] || (pal_fixed && ca.pal != cb.pal) {
                        return false;
                    }
                    let le = (0..target_count).all(|k| {
                        bounds.weakest[a].no_gold[k] <= bounds.strongest[b].no_gold[k]
                            && bounds.weakest[a].no_hint[k] <= bounds.strongest[b].no_hint[k]
                    });
                    let eq = (0..target_count).all(|k| {
                        bounds.weakest[a].no_gold[k] == bounds.strongest[b].no_gold[k]
                            && bounds.weakest[a].no_hint[k] == bounds.strongest[b].no_hint[k]
                    });
                    le && (!eq || tr[a] < tr[b])
                })
                .map(|(ni, _)| ni)
                .collect()
        })
        .collect();
    for (bi, &b) in kept_old.iter().enumerate() {
        let names: Vec<String> =
            robust[bi].iter().map(|&a| entries.cards[entries.rel[kept_old[a]]].name.clone()).collect();
        if !names.is_empty() {
            eprintln!("ROBUST {} <= {:?}", entries.cards[entries.rel[b]].name, names);
        }
    }
}

/// ---- dominance coordinates ----
/// For a target the trainee's outfit does not give, the metric's landing chance is
///   p = W + W*g - W*(1 - e)*(1 - q)*(1 - h)  (W = GEN_WHITE), e = P(own-event white hint),
///   g = P(gold hint), h = P(any deck hint) = 1 - (1 - w)*(1 - g);
/// with the white on the outfit p = W + W*g, and with the gold it is constant. So the metric only
/// rises with g and h, and a deck at least as high in every (g, h) is never worse. (w, g) dominance implies
/// (g, h) dominance, so this prunes at least as much. Coordinates no relevant card can move are dropped,
/// and h == g when no card gives the white.
fn dominance_coords(srcs: &[TargetSources], target_count: usize) -> Vec<crate::deck::space::HintCoordinate> {
    let mut has_w = vec![false; target_count];
    let mut has_g = vec![false; target_count];
    for sr in srcs {
        for &TrainingHint { target: k, .. } in &sr.training_hints {
            has_w[k] = true;
        }
        for g in sr.fixed_outcomes.iter().chain(sr.chain_outcomes.iter().map(|x| &x.outcomes)) {
            for EventOutcome { rewards: out, .. } in g {
                for &TargetReward { target: k, level: lvl } in out {
                    if lvl >= 2 {
                        has_g[k] = true;
                    } else {
                        has_w[k] = true;
                    }
                }
            }
        }
    }
    (0..target_count)
        .flat_map(|k| {
            let mut v = vec![];
            if has_g[k] {
                v.push(crate::deck::space::HintCoordinate { target: k, gold: true });
            }
            if has_w[k] {
                v.push(crate::deck::space::HintCoordinate { target: k, gold: false });
            }
            v
        })
        .collect()
}

/// Per chain-kind key: each entry's miss chances (white, gold) under the key's chain odds, and for each entry B the
/// entries that beat it on every target under those odds (same kind, so swapping keeps the odds; per target no
/// higher chance of missing the gold and no higher chance of missing every hint; exact ties go by `tie_rank`), as
/// owned / borrowed character masks.
struct ChainKeyData {
    white_misses: Vec<f64>,
    gold_misses: Vec<f64>,
    owned_dominators: Vec<u128>,
    borrowed_dominators: Vec<u128>,
}

/// Work counters of the enumeration (`decks_built`, `decks_swapped` and DECK_DEBUG's)
#[derive(Default)]
struct EnumerationCounts {
    decks_built: usize,
    decks_swapped: usize,
    subtrees_pruned: usize,
    first_frontier_rejections: usize,
    dominance_tests: usize,
    full_scans: usize,
    dominated_decks: usize,
    recent_rejections: usize,
}

/// Counts of relevant entries, plain pals and filler cards in each completed deck.
#[derive(Clone, Copy)]
struct DeckShape {
    entry_count: usize,
    plain_pals: usize,
    filler_cards: usize,
}

/// A previously computed single-dominator frontier and its deck identities.
struct FirstFrontier {
    decks: DominanceIndex,
    identities: crate::FxSet<Vec<u64>>,
}

#[derive(Clone, Copy)]
struct FrontierOptions<'frontier> {
    /// Reject a deck after this many distinct decks dominate it.
    maximum_dominators: usize,
    first_frontier: Option<&'frontier FirstFrontier>,
}

struct FrontierWorkspace<'counts> {
    groups: Groups,
    /// Reused coordinates of the candidate currently offered to the frontier.
    coordinates: Vec<f64>,
    counts: &'counts mut EnumerationCounts,
}

struct FrontierEntryGroup {
    entries: Vec<usize>,
    required: usize,
}

#[derive(Clone, Copy)]
struct DeckSelection<'selection> {
    entries: &'selection [usize],
    characters: u128,
    borrowed_count: usize,
}

struct FrontierCandidate<'selection> {
    entries: &'selection [usize],
    shape: DeckShape,
    pals: usize,
    borrowed: Option<usize>,
    white_hint_chances: &'selection [f64; 32],
    gold_hint_chances: &'selection [f64; 32],
    coordinate_sum: f64,
}
/// Every deck competes in the same frontier, regardless of its pal count.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum FrontierGroup {
    Global,
}

type Groups = crate::FxMap<FrontierGroup, DominanceIndex>;

// ---- enumerate decks ----
// Decks are enumerated one chain-kind key at a time (the key fixes every entry's chain odds), so each deck is
// generated once, under its own key, with that key's exact dominance relation:
//  - Swap rule. If card A outside the deck beats deck card B on every target under the key's odds and can
//    legally replace it (same kind, so the key is unchanged; not a second entry of a character already in
//    the deck; within the borrow limit), the swapped deck is generated too and is at least as good. A deck
//    with top_decks such swaps is not needed: by induction over the swap order (ties go to the lower index,
//    so it has no cycles), each swapped deck is kept or itself has top_decks kept decks at least as good.
//    Distinct (B, A) pairs give distinct decks; counting A's distinct characters undercounts pairs, which
//    only skips fewer decks.
//  - Partial decks. While a deck is built, each entry still to be added blocks at most one more character,
//    so a partial deck where one card B already has (entries still to add) + top_decks unblocked legal
//    swaps can only grow into decks the swap rule skips; the whole branch is dropped.
//  - Dead entries. The same with nothing chosen yet: an entry whose legal dominators under the key span
//    deck_size - 1 + top_decks characters never appears.
// Owned cards only count borrowed dominators when the deck has room for a borrowed card.

/// The enumeration's tables over the entries left after card dominance: characters as bits (one deck holds one entry
/// per character), borrowed and locked flags, the chain kinds with their key places and entries, and the entries
/// without chain slots.
struct DeckEnumerator<'env> {
    deck_job: &'env DeckJob,
    entries: &'env Entries<'env>,
    coordinates: &'env [crate::deck::space::HintCoordinate],
    target_count: usize,
    entry_count: usize,
    size: usize,
    allowed_borrow: usize,
    pal_fixed: bool,
    character_bits: Vec<u128>,
    borrowed_entries: Vec<bool>,
    is_locked: Vec<bool>,
    // Chain-kind key of a deck = multiset of its chain entries' kinds plus fillers (2, 2), coded as counts in base 7.
    chain_kinds: Vec<ChainKind>,
    kind_places: Vec<u64>,
    filler_kind_index: usize,
    entries_by_kind: Vec<Vec<usize>>,
    // entries without chain slots: pals, group cards and R cards
    no_chain_entries: Vec<usize>,
    tie_ranks: Vec<usize>,
}

impl<'env> DeckEnumerator<'env> {
    fn new(
        entries: &'env Entries<'env>,
        deck_job: &'env DeckJob,
        target_count: usize,
        coordinates: &'env [crate::deck::space::HintCoordinate],
    ) -> Result<Self, Value> {
        let (cards, rel, kind) = (&entries.cards, &entries.rel, &entries.kind);
        let allowed_borrow = usize::from(deck_job.borrow);
        let entry_count = rel.len();
        // characters as bits (one deck holds one entry per character)
        let mut chara_ix: HashMap<u32, usize> = HashMap::new();
        for &r in rel {
            let n = chara_ix.len();
            chara_ix.entry(cards[r].chara).or_insert(n);
        }
        if chara_ix.len() > 128 {
            return Err(json!({"error": "too many distinct characters among relevant cards"}));
        }
        let character_bits: Vec<u128> = rel.iter().map(|&r| 1u128 << chara_ix[&cards[r].chara]).collect();
        let borrowed_entries: Vec<bool> = rel.iter().map(|&r| !deck_job.is_owned(cards[r].id)).collect();
        let mut chain_kinds: Vec<ChainKind> =
            (0..entry_count).filter(|&j| kind[j] != ChainKind::NONE).map(|j| kind[j]).collect();
        chain_kinds.push(ChainKind::FILLER);
        chain_kinds.sort_unstable();
        chain_kinds.dedup();
        let kind_places: Vec<u64> = (0..chain_kinds.len()).map(|i| 7u64.pow(i as u32)).collect();
        let tie_ranks = tie_rank(rel, cards);
        assert!(target_count <= 32, "too many uncertain targets");
        let is_locked: Vec<bool> = (0..entry_count).map(|j| entries.locked.contains(&rel[j])).collect();
        let nk = chain_kinds.len();
        let filler_kind_index = chain_kinds.iter().position(|&k| k == ChainKind::FILLER).unwrap();
        let entries_by_kind: Vec<Vec<usize>> = (0..nk)
            .map(|i| (0..entry_count).filter(|&j| kind[j] != ChainKind::NONE && kind[j] == chain_kinds[i]).collect())
            .collect();
        let no_chain_entries: Vec<usize> = (0..entry_count).filter(|&j| kind[j] == ChainKind::NONE).collect();
        Ok(DeckEnumerator {
            deck_job,
            entries,
            coordinates,
            target_count,
            entry_count,
            size: deck_job.deck_size,
            allowed_borrow,
            pal_fixed: deck_job.pals.is_some(),
            character_bits,
            borrowed_entries,
            is_locked,
            chain_kinds,
            kind_places,
            filler_kind_index,
            entries_by_kind,
            no_chain_entries,
            tie_ranks,
        })
    }

    /// The key data of a chain-kind key code (`ChainKeyData`)
    fn make_key(&self, code: u64) -> ChainKeyData {
        let (entry_count, target_count) = (self.entry_count, self.target_count);
        let (cards, rel, kind, srcs) = (&self.entries.cards, &self.entries.rel, &self.entries.kind, &self.entries.srcs);
        let mut ks: Vec<ChainKind> = vec![];
        let mut c = code;
        for &key_data in &self.chain_kinds {
            for _ in 0..(c % 7) {
                ks.push(key_data);
            }
            c /= 7;
        }
        let marg = chain_marginals(self.deck_job.chain_events(), &ks);
        let empty: Vec<f64> = vec![];
        let mut white_misses = vec![1.0f64; entry_count * target_count];
        let mut gold_misses = vec![1.0f64; entry_count * target_count];
        for j in 0..entry_count {
            srcs[j].src_miss(
                marg.get(&kind[j]).unwrap_or(&empty),
                &mut white_misses[j * target_count..(j + 1) * target_count],
                &mut gold_misses[j * target_count..(j + 1) * target_count],
            );
        }
        let hint_misses: Vec<f64> = (0..entry_count * target_count).map(|i| gold_misses[i] * white_misses[i]).collect();
        let mut owned_dominators = vec![0u128; entry_count];
        let mut borrowed_dominators = vec![0u128; entry_count];
        for b in 0..entry_count {
            for a in 0..entry_count {
                // same kind keeps the chain odds; with a fixed pal count (`pals`), same pal status keeps it
                if a == b || kind[a] != kind[b] || (self.pal_fixed && cards[rel[a]].pal != cards[rel[b]].pal) {
                    continue;
                }
                let (ra, rb) = (a * target_count..(a + 1) * target_count, b * target_count..(b + 1) * target_count);
                let le = gold_misses[ra.clone()].iter().zip(&gold_misses[rb.clone()]).all(|(x, y)| x <= y)
                    && hint_misses[ra.clone()].iter().zip(&hint_misses[rb.clone()]).all(|(x, y)| x <= y);
                if !le {
                    continue;
                }
                let eq = gold_misses[ra.clone()] == gold_misses[rb.clone()] && hint_misses[ra] == hint_misses[rb];
                if eq && self.tie_ranks[a] > self.tie_ranks[b] {
                    continue;
                }
                if self.borrowed_entries[a] {
                    borrowed_dominators[b] |= self.character_bits[a];
                } else {
                    owned_dominators[b] |= self.character_bits[a];
                }
            }
        }
        ChainKeyData { white_misses, gold_misses, owned_dominators, borrowed_dominators }
    }

    /// a deck's identity from a selection of entries
    fn ident_sel(&self, sel: &[usize], plain: usize, filler: usize) -> Vec<u64> {
        let ident_of = |c: usize, e: u8, x: u32| ((c as u64 + 3) << 40) | (u64::from(x) << 8) | u64::from(e);
        let mut v: Vec<u64> = Vec::with_capacity(sel.len() + 2);
        v.extend(sel.iter().map(|&j| ident_of(self.entries.rel[j], self.entries.endk[j], self.entries.vark[j])));
        v.sort_unstable();
        v.push((1 << 40) + plain as u64);
        v.push((2 << 40) + filler as u64);
        v
    }

    /// Keep decks with fewer than the requested number of dominators. An optional first frontier can
    /// reject candidates early, but a deck cannot count itself among its dominators.
    fn run_enum(&self, options: FrontierOptions, counts: &mut EnumerationCounts) -> Vec<DeckEval> {
        let mut workspace =
            FrontierWorkspace { groups: Groups::default(), coordinates: vec![0.0; self.coordinates.len()], counts };
        for counts in &self.count_vectors() {
            self.enum_key(counts, options, &mut workspace);
        }
        let mut front: Vec<DeckEval> = vec![];
        let mut group_keys: Vec<FrontierGroup> = workspace.groups.keys().copied().collect();
        group_keys.sort_unstable();
        for group in &group_keys {
            let list = workspace.groups.remove(group).unwrap().into_list();
            // With one dominator the online set is already an antichain: k_filter would only reorder it.
            let kept = if options.maximum_dominators == 1 {
                kfilter_order(list, self.coordinates.len())
            } else {
                k_filter(list, options.maximum_dominators, self.coordinates.len()).into_list()
            };
            front.extend(kept);
        }
        front
    }

    /// every chain-kind count vector (fillers included in the (2, 2) count) with at most deck_size chain slots
    fn count_vectors(&self) -> Vec<Vec<usize>> {
        let size = self.size;
        let mut counts: Vec<Vec<usize>> = vec![vec![]];
        for _ in 0..self.chain_kinds.len() {
            counts = counts
                .into_iter()
                .flat_map(|c| {
                    let used: usize = c.iter().sum();
                    (0..=size - used).map(move |x| {
                        let mut d = c.clone();
                        d.push(x);
                        d
                    })
                })
                .collect();
        }
        counts
    }

    /// The decks of one count vector: its key's dead entries left out, every split of the (2, 2) count into real
    /// entries and fillers and of the rest into pal-like entries and plain pals
    fn enum_key(&self, counts: &[usize], options: FrontierOptions, workspace: &mut FrontierWorkspace) {
        let (nk, filler_kind_index, size) = (self.chain_kinds.len(), self.filler_kind_index, self.size);
        let (entries_by_kind, borrowed_entries) = (&self.entries_by_kind, &self.borrowed_entries);
        let chain_slots: usize = counts.iter().sum();
        // a key needs real entries of each non-filler kind
        if (0..nk).any(|i| i != filler_kind_index && counts[i] > entries_by_kind[i].len()) {
            return;
        }
        let code: u64 = counts.iter().zip(&self.kind_places).map(|(&c, &p)| c as u64 * p).sum();
        // (built per count vector and dropped after it: each one has its own code, so no key repeats)
        let key_data: &ChainKeyData = &self.make_key(code);
        let alive = |j: usize| {
            self.is_locked[j]
                || (key_data.owned_dominators[j]
                    | if borrowed_entries[j] { key_data.borrowed_dominators[j] } else { 0 })
                .count_ones()
                    < (size - 1 + options.maximum_dominators) as u32
        };
        let alive_k: Vec<Vec<usize>> =
            (0..nk).map(|i| entries_by_kind[i].iter().copied().filter(|&j| alive(j)).collect()).collect();
        let alive_p: Vec<usize> = self.no_chain_entries.iter().copied().filter(|&j| alive(j)).collect();
        for real22 in 0..=counts[filler_kind_index].min(entries_by_kind[filler_kind_index].len()) {
            let filler = counts[filler_kind_index] - real22;
            if (0..nk).any(|i| (if i == filler_kind_index { real22 } else { counts[i] }) > alive_k[i].len()) {
                continue;
            }
            let rem = size - chain_slots;
            for np in 0..=rem.min(alive_p.len()) {
                let plain = rem - np;
                if plain > self.entries.plain_chars.len() {
                    continue;
                }
                let target = chain_slots - filler + np; // entries in the finished deck
                let mut entry_groups: Vec<FrontierEntryGroup> = vec![];
                for i in 0..nk {
                    let c = if i == filler_kind_index { real22 } else { counts[i] };
                    if c > 0 {
                        entry_groups.push(FrontierEntryGroup { entries: alive_k[i].clone(), required: c });
                    }
                }
                if np > 0 {
                    entry_groups.push(FrontierEntryGroup { entries: alive_p.clone(), required: np });
                }
                self.enum_decks(
                    key_data,
                    &entry_groups,
                    DeckShape { entry_count: target, plain_pals: plain, filler_cards: filler },
                    options,
                    workspace,
                );
            }
        }
    }

    /// Choose the required entries from each group, pruning partial decks by the swap rule.
    fn enum_decks(
        &self,
        key_data: &ChainKeyData,
        entry_groups: &[FrontierEntryGroup],
        shape: DeckShape,
        options: FrontierOptions,
        workspace: &mut FrontierWorkspace,
    ) {
        let (character_bits, borrowed_entries, is_locked) =
            (&self.character_bits, &self.borrowed_entries, &self.is_locked);
        let mut n_pruned = 0usize;
        // (a card added later could be the deck's borrowed card, so before the deck is complete an owned
        // card only counts owned dominators)
        let mut prune = |selection: DeckSelection| -> bool {
            let need = (shape.entry_count - selection.entries.len() + options.maximum_dominators) as u32;
            for &b in selection.entries {
                if is_locked[b] {
                    continue;
                }
                let m = key_data.owned_dominators[b]
                    | if borrowed_entries[b] { key_data.borrowed_dominators[b] } else { 0 };
                if (m & !(selection.characters & !character_bits[b])).count_ones() >= need {
                    n_pruned += 1;
                    return true;
                }
            }
            false
        };
        let mut out = |selection: DeckSelection| self.emit(key_data, selection, shape, options, workspace);
        let mut scratch = PickScratch { selection: Vec::with_capacity(self.size), cut_subtrees: 0 };
        if entry_groups.is_empty() {
            out(DeckSelection { entries: &scratch.selection, characters: 0, borrowed_count: 0 });
        } else {
            // later[gi][p]: characters of entries after position p in group gi and of every later group
            let mut later: Vec<Vec<u128>> = vec![vec![]; entry_groups.len()];
            let mut tail = 0u128;
            for gi in (0..entry_groups.len()).rev() {
                let l = &entry_groups[gi].entries;
                let mut v = vec![0u128; l.len()];
                let mut m = tail;
                for p in (0..l.len()).rev() {
                    v[p] = m;
                    m |= character_bits[l[p]];
                }
                tail = m;
                later[gi] = v;
            }
            // legal dominators whatever else the deck holds: owned ones, or any for a borrowed card
            let dm: Vec<u128> = (0..self.entry_count)
                .map(|j| {
                    if is_locked[j] {
                        0
                    } else {
                        key_data.owned_dominators[j]
                            | if borrowed_entries[j] { key_data.borrowed_dominators[j] } else { 0 }
                    }
                })
                .collect();
            let context = PickContext {
                groups: entry_groups,
                later_characters: &later,
                dominators: &dm,
                character_bits,
                borrowed: borrowed_entries,
                allowed_borrow: self.allowed_borrow,
                maximum_dominators: options.maximum_dominators as u32,
            };
            let cursor = PickCursor {
                group: 0,
                start: 0,
                remaining: entry_groups[0].required,
                characters: 0,
                borrowed_count: 0,
                guaranteed_swaps: 0,
            };
            pick(&context, cursor, &mut scratch, &mut prune, &mut out);
        }
        workspace.counts.subtrees_pruned += n_pruned + scratch.cut_subtrees;
    }

    /// A finished deck: kept on the frontier unless it breaks the job's pal count or locks, the swap rule skips it,
    /// or enough other decks dominate it.
    fn emit(
        &self,
        key_data: &ChainKeyData,
        selection: DeckSelection,
        shape: DeckShape,
        options: FrontierOptions,
        workspace: &mut FrontierWorkspace,
    ) {
        let (target_count, coordinates) = (self.target_count, self.coordinates);
        let Some(pals) = self.pal_count(selection.entries, shape.plain_pals) else { return };
        if self.swaps_skip(key_data, selection, options) {
            workspace.counts.decks_swapped += 1;
            return;
        }
        let mut white_hint_chances = [1.0f64; 32];
        let mut gold_hint_chances = [1.0f64; 32];
        for &j in selection.entries {
            let (w, g) = (
                &key_data.white_misses[j * target_count..(j + 1) * target_count],
                &key_data.gold_misses[j * target_count..(j + 1) * target_count],
            );
            for t in 0..target_count {
                white_hint_chances[t] *= w[t];
                gold_hint_chances[t] *= g[t];
            }
        }
        for t in 0..target_count {
            white_hint_chances[t] = 1.0 - white_hint_chances[t];
            gold_hint_chances[t] = 1.0 - gold_hint_chances[t];
        }
        let borrowed =
            selection.entries.iter().copied().find(|&j| self.borrowed_entries[j]).map(|j| self.entries.rel[j]);
        workspace.counts.decks_built += 1;
        let mut coordinate_sum = 0.0;
        for (c, &crate::deck::space::HintCoordinate { target: k, gold: is_g }) in coordinates.iter().enumerate() {
            workspace.coordinates[c] = if is_g {
                gold_hint_chances[k]
            } else {
                1.0 - (1.0 - white_hint_chances[k]) * (1.0 - gold_hint_chances[k])
            };
            coordinate_sum += workspace.coordinates[c];
        }
        self.offer(
            FrontierCandidate {
                entries: selection.entries,
                shape,
                pals,
                borrowed,
                white_hint_chances: &white_hint_chances,
                gold_hint_chances: &gold_hint_chances,
                coordinate_sum,
            },
            options,
            workspace,
        );
    }

    /// The deck's pal count, None when it breaks the job's pal count or leaves out a locked card
    fn pal_count(&self, sel: &[usize], plain: usize) -> Option<usize> {
        let (cards, rel) = (&self.entries.cards, &self.entries.rel);
        let pals = sel.iter().filter(|&&j| cards[rel[j]].pal).count() + plain;
        if let Some(p) = self.deck_job.pals
            && p != pals
        {
            return None;
        }
        if !self.entries.locked.iter().all(|l| sel.iter().any(|&j| rel[j] == *l)) {
            return None;
        }
        Some(pals)
    }

    /// Whether the complete deck has enough legal swaps for better cards to be skipped.
    fn swaps_skip(&self, key_data: &ChainKeyData, selection: DeckSelection, options: FrontierOptions) -> bool {
        let (character_bits, borrowed_entries) = (&self.character_bits, &self.borrowed_entries);
        let dmask = |b: usize, borrowed_count: usize| -> u128 {
            key_data.owned_dominators[b]
                | if borrowed_count - usize::from(borrowed_entries[b]) < self.allowed_borrow {
                    key_data.borrowed_dominators[b]
                } else {
                    0
                }
        };
        let mut swaps = 0u32;
        for &b in selection.entries {
            if self.is_locked[b] {
                continue;
            }
            swaps += (dmask(b, selection.borrowed_count) & !(selection.characters & !character_bits[b])).count_ones();
            if swaps as usize >= options.maximum_dominators {
                break;
            }
        }
        swaps as usize >= options.maximum_dominators
    }

    /// Reject a dominated candidate using recent winners, the first frontier, then the current frontier.
    /// Accepted candidates retain their coordinates only when later compaction needs them.
    fn offer(&self, candidate: FrontierCandidate, options: FrontierOptions, workspace: &mut FrontierWorkspace) {
        let (target_count, coordinates, rel) = (self.target_count, self.coordinates, &self.entries.rel);
        // One global frontier: pals have already passed the job's count constraint. A deck
        // matched or beaten on every coordinate is no better for any loop.
        let nd = coordinates.len();
        let g = workspace.groups.entry(FrontierGroup::Global).or_insert_with(|| DominanceIndex::new(nd));
        if options.maximum_dominators == 1
            && g.has_last_dominator
            && dominates(&g.last_dominator, &workspace.coordinates)
        {
            return;
        }
        if options.maximum_dominators == 1
            && nd > 0
            && (0..g.recent_count).any(|r| dominates(&g.recent[r * nd..(r + 1) * nd], &workspace.coordinates))
        {
            workspace.counts.recent_rejections += 1;
            return;
        }
        if options.maximum_dominators > 1
            && g.recent_count >= options.maximum_dominators
            && g.count_recent(&workspace.coordinates) >= options.maximum_dominators
        {
            workspace.counts.recent_rejections += 1;
            return;
        }
        let xq = sig_of(&workspace.coordinates, true);
        if let Some(first) = options.first_frontier {
            // A candidate already in the first frontier needs one extra match to exclude self-dominance.
            if first.decks.count_ro(&workspace.coordinates, xq, candidate.coordinate_sum, options.maximum_dominators)
                >= options.maximum_dominators
            {
                let own = first.identities.contains(&self.ident_sel(
                    candidate.entries,
                    candidate.shape.plain_pals,
                    candidate.shape.filler_cards,
                ));
                if !own
                    || first.decks.count_ro(
                        &workspace.coordinates,
                        xq,
                        candidate.coordinate_sum,
                        options.maximum_dominators + 1,
                    ) > options.maximum_dominators
                {
                    workspace.counts.first_frontier_rejections += 1;
                    return;
                }
            }
        }
        let n = g.count(
            &workspace.coordinates,
            xq,
            candidate.coordinate_sum,
            options.maximum_dominators,
            &mut workspace.counts.dominance_tests,
        );
        workspace.counts.full_scans += 1;
        if n > 0 && (options.maximum_dominators > 1 || n >= options.maximum_dominators) && nd > 0 {
            g.remember_found();
        }
        if n >= options.maximum_dominators {
            workspace.counts.dominated_decks += 1;
            return;
        }
        // entry order, for display (distinct entries: the unstable sort is the sort), on the stack
        let mut sa = [0usize; 32];
        let so = &mut sa[..candidate.entries.len()];
        so.copy_from_slice(candidate.entries);
        so.sort_unstable();
        let x = DeckEval {
            pals: candidate.pals,
            borrowed: candidate.borrowed,
            cards: so.iter().map(|&j| rel[j]).collect(),
            ends: so.iter().map(|&j| self.entries.endk[j]).collect(),
            vars: so.iter().map(|&j| self.entries.vark[j]).collect(),
            plain: candidate.shape.plain_pals,
            filler: candidate.shape.filler_cards,
            dw: candidate.white_hint_chances[..target_count].to_vec(),
            dg: candidate.gold_hint_chances[..target_count].to_vec(),
            dv: if options.maximum_dominators > 1 { workspace.coordinates.clone() } else { vec![] },
            s: candidate.coordinate_sum,
        };
        if options.maximum_dominators == 1 {
            g.remove_dominated_by(
                &workspace.coordinates,
                sig_of(&workspace.coordinates, false),
                candidate.coordinate_sum,
            );
        }
        g.insert_dv(x, &workspace.coordinates);
        if options.maximum_dominators > 1 && g.len() > g.cap {
            // compaction drops decks, so the remembered dominators are forgotten (DominanceIndex::new clears them)
            let list = std::mem::replace(g, DominanceIndex::new(nd)).into_list();
            *g = k_filter(list, options.maximum_dominators, nd);
            g.cap = (2 * g.len()).max(512);
        }
    }
}

// Pick each group's required entries, enforcing character uniqueness and the borrow limit.
// Passed-over dominators: when entry j is added at position p of its group, a dominator of j whose character
// is not in the deck and belongs to no entry that can still be added (`later_characters[group][position]`) will never be in the
// finished deck, so it is a legal swap for j there. `guaranteed_swaps` counts such guaranteed swaps along the branch; at
// top_decks the branch is cut (the swap rule would skip every deck in it).
struct PickContext<'tables> {
    groups: &'tables [FrontierEntryGroup],
    /// At each group position, characters available strictly after that position.
    later_characters: &'tables [Vec<u128>],
    dominators: &'tables [u128],
    character_bits: &'tables [u128],
    borrowed: &'tables [bool],
    allowed_borrow: usize,
    maximum_dominators: u32,
}

#[derive(Clone, Copy)]
struct PickCursor {
    group: usize,
    start: usize,
    remaining: usize,
    characters: u128,
    borrowed_count: usize,
    guaranteed_swaps: u32,
}

struct PickScratch {
    selection: Vec<usize>,
    cut_subtrees: usize,
}

fn pick(
    context: &PickContext,
    cursor: PickCursor,
    scratch: &mut PickScratch,
    prune: &mut dyn FnMut(DeckSelection) -> bool,
    emit: &mut dyn FnMut(DeckSelection),
) {
    if cursor.remaining == 0 {
        if cursor.group + 1 >= context.groups.len() {
            emit(DeckSelection {
                entries: &scratch.selection,
                characters: cursor.characters,
                borrowed_count: cursor.borrowed_count,
            });
        } else {
            let next = PickCursor {
                group: cursor.group + 1,
                start: 0,
                remaining: context.groups[cursor.group + 1].required,
                ..cursor
            };
            pick(context, next, scratch, prune, emit);
        }
        return;
    }
    let entries = &context.groups[cursor.group].entries;
    for position in cursor.start..entries.len() {
        if entries.len() - position < cursor.remaining {
            break;
        }
        let entry = entries[position];
        if cursor.characters & context.character_bits[entry] != 0
            || (context.borrowed[entry] && cursor.borrowed_count >= context.allowed_borrow)
        {
            continue;
        }
        let characters = cursor.characters | context.character_bits[entry];
        let borrowed_count = cursor.borrowed_count + usize::from(context.borrowed[entry]);
        let guaranteed_swaps = cursor.guaranteed_swaps
            + (context.dominators[entry] & !characters & !context.later_characters[cursor.group][position])
                .count_ones();
        if guaranteed_swaps >= context.maximum_dominators {
            scratch.cut_subtrees += 1;
            continue;
        }
        scratch.selection.push(entry);
        if !prune(DeckSelection { entries: &scratch.selection, characters, borrowed_count }) {
            let next = PickCursor {
                start: position + 1,
                remaining: cursor.remaining - 1,
                characters,
                borrowed_count,
                guaranteed_swaps,
                ..cursor
            };
            pick(context, next, scratch, prune, emit);
        }
        scratch.selection.pop();
    }
}

/// Decks that no other frontier deck (of any pal count) dominates. The metric rises with every dominance
/// coordinate, so for any loop the best value over the frontier is reached on this skyline; the ceilings
/// in run_multi (which only bound that best value) are built from it. Exact scoring obtains
/// any other needed legal decks from DeckSpace.
/// (with top_decks = 1 the frontier is already an antichain, so it is its own skyline)
fn skyline(front: &[DeckEval]) -> Vec<usize> {
    (0..front.len()).collect()
}

/// A card entry's display identity, including its chain and event choices.
#[derive(PartialEq, Eq, Hash)]
struct DisplayEntry {
    card: usize,
    end_link: u8,
    variant: u32,
}

/// Shared labels, with the borrowed label built only when requested.
struct EntryLabels {
    name: Name,
    choices: Name,
    borrowed: Option<Name>,
}

/// The skyline's decks detached from the search (names, ids and choice labels instead of indices): the frontier
/// consumed, the skyline's decks (ascending indices, `sky`) in order: each deck's lists move into its PDeck and the
/// rest is freed before `run_multi`
fn detach(front: Vec<DeckEval>, sky: &[usize], ent: &Entries, source_builder: &Sources, sk: &SkillsRaw) -> Vec<PDeck> {
    let cards = &ent.cards;
    // per (card, end link, variant): its name with the choice and end-link notes, its choice label, and the name
    // marked borrowed (built at first need)
    let mut shown: crate::FxMap<DisplayEntry, EntryLabels> = crate::FxMap::default();
    // (and the shared strings of the card names, the plain pals and the filler)
    let mut cname: crate::FxMap<usize, Name> = crate::FxMap::default();
    let plain_rc: Vec<Name> = ent.plain_names.iter().map(|n| Name::from(n.as_str())).collect();
    let filler_rc: Name = Name::from("(any SR without target hints)");
    let mut skx = sky.iter().copied().peekable();
    front
        .into_iter()
        .enumerate()
        .filter(|&(i, _)| {
            if skx.peek() == Some(&i) {
                skx.next();
                true
            } else {
                false
            }
        })
        .map(|(_, x)| {
            let mut names: Vec<Name> = Vec::with_capacity(x.cards.len() + x.plain + x.filler);
            let mut choices: Vec<Name> = Vec::with_capacity(x.cards.len());
            for ((&i, &e), &v) in x.cards.iter().zip(&x.ends).zip(&x.vars) {
                let labels = shown.entry(DisplayEntry { card: i, end_link: e, variant: v }).or_insert_with(|| {
                    let mut n = cards[i].name.clone();
                    let lb = source_builder.label(cards[i], (e > 0).then_some(e), v, sk);
                    if !lb.is_empty() {
                        n = format!("{n} ({lb})");
                    }
                    if e > 0 {
                        n = format!("{n} (end chain at link {e})");
                    }
                    EntryLabels { name: Name::from(n), choices: Name::from(lb), borrowed: None }
                });
                names.push(if Some(i) == x.borrowed {
                    std::rc::Rc::clone(
                        labels.borrowed.get_or_insert_with(|| Name::from(format!("{} (borrowed)", labels.name))),
                    )
                } else {
                    std::rc::Rc::clone(&labels.name)
                });
                choices.push(std::rc::Rc::clone(&labels.choices));
            }
            names.extend(plain_rc.iter().take(x.plain).cloned());
            for _ in 0..x.filler {
                names.push(std::rc::Rc::clone(&filler_rc));
            }
            let mut cn =
                |i: usize| std::rc::Rc::clone(cname.entry(i).or_insert_with(|| Name::from(cards[i].name.as_str())));
            PDeck {
                names,
                ids: x.cards.iter().map(|&i| cards[i].id).collect(),
                choices,
                plain_ids: ent.plain_ids.iter().take(x.plain).copied().collect(),
                pal_names: x
                    .cards
                    .iter()
                    .filter(|&&i| cards[i].pal)
                    .map(|&i| cn(i))
                    .chain(plain_rc.iter().take(x.plain).cloned())
                    .collect(),
                pals: x.pals,
                borrowed: x.borrowed.map(&mut cn),
                ends: x.ends,
                vars: x.vars,
                dw: x.dw,
                dg: x.dg,
            }
        })
        .collect()
}

/// Apply an original-entry mask to one column of the parallel entry arrays.
/// Each column starts with the mask's length; Vec::retain preserves its surviving entries' order.
fn retain_entries<T>(entries: &mut Vec<T>, keep: &[bool]) {
    let mut original_index = 0;
    entries.retain(|_| {
        let retain = keep[original_index];
        original_index += 1;
        retain
    });
}
