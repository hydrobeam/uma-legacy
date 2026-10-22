//! Deck dominance: signatures, buckets and the beaten-deck bookkeeping (`DominanceIndex`), and the decks a search returns.
//! Frontier enumeration belongs to `frontier`; cached career plans belong to `career::plan::kept`.
use crate::deck::Card;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
/// Slack for float noise in dominance tests: per coordinate, and summed over all coordinates.
pub(super) const DOM_EPS: f64 = 1e-12;
pub(super) const SUM_EPS: f64 = 1e-9;

#[inline]
pub(super) fn dominates(dominator: &[f64], coordinates: &[f64]) -> bool {
    dominator
        .iter()
        .zip(coordinates)
        .all(|(dominator_coordinate, coordinate)| *dominator_coordinate >= *coordinate - DOM_EPS)
}

/// Coarse signature for a fast necessary test of dominance: the first SIG_FIELDS coordinates as levels
/// floor(LEVELS * v) (capped at LEVELS - 1), each in a field of FB bits whose top bit is a guard. `entry`
/// signatures use the value itself, `query` ones the value less DOM_EPS, so `by` dominating `x` implies every
/// field of entry(by) >= the matching field of query(x). A 64-bit word holds 16 fields of 8 levels, enough for
/// 8 targets x (gold, any hint), and is cheap in WebAssembly as well as natively.
pub(super) type Sig = u64;
pub(super) const FB: usize = 4;
pub(super) const SIG_FIELDS: usize = 64 / FB;
pub(super) const LEVELS: Sig = 1 << (FB - 1);
pub(super) const SIG_GUARD: Sig = {
    let mut guard: Sig = 0;
    let mut field = 0;
    while field < SIG_FIELDS {
        guard |= (1 as Sig) << (FB * field + FB - 1);
        field += 1;
    }
    guard
};
pub(super) fn sig_of(coordinates: &[f64], query: bool) -> Sig {
    let mut signature: Sig = 0;
    for (field, &coordinate) in coordinates.iter().take(SIG_FIELDS).enumerate() {
        let value = if query { coordinate - DOM_EPS } else { coordinate };
        // via u32: wasm2js lacks the 64-bit float conversion
        let level = Sig::from((value * LEVELS as f64).floor().max(0.0) as u32).min(LEVELS - 1);
        signature |= level << (FB * field);
    }
    signature
}
/// every field of `entry` >= the matching field of `query` (SWAR: guard bits survive the subtraction)
#[inline]
pub(super) const fn sig_ge(entry: Sig, query: Sig) -> bool {
    ((entry | SIG_GUARD) - query) & SIG_GUARD == SIG_GUARD
}

pub(super) const NBUCKET: usize = 128;

/// Decks whose coordinate sums fall in one slice of [0, nd]: flat arrays (entry and query signatures,
/// coordinates, ids) plus a level index of the entry signatures: for each word of 64 decks and each signature
/// field f and level l >= 1, the bits of the decks whose field f is at least l. ANDing one word per field of a
/// query gives the decks that pass the signature test, 64 at a time; they are then visited in index order, the
/// same order as a plain scan.
#[derive(Default)]
/// `lq` is the mirror index of the query signatures (bit set when field f is at most l, l < LEVELS - 1), for the
/// removal test "x's entry signature over the deck's query one".
pub(super) struct Bucket {
    // These columns share deck indices. Compaction must move every column together.
    pub(super) list: Vec<DeckEval>,
    pub(super) entry_signatures: Vec<Sig>,
    pub(super) query_signatures: Vec<Sig>,
    pub(super) flat: Vec<f64>,
    pub(super) deck_ids: Vec<u64>,
    pub(super) entry_levels: Vec<u64>,
    pub(super) query_levels: Vec<u64>,
}
pub(super) const LVW: usize = SIG_FIELDS * (LEVELS as usize - 1); // index words per 64 decks
impl Bucket {
    #[inline]
    pub(super) fn index_set(&mut self, i: usize, entry_signature: Sig, query_signature: Sig) {
        let word = i / 64;
        if self.entry_levels.len() < (word + 1) * LVW {
            self.entry_levels.resize((word + 1) * LVW, 0);
            self.query_levels.resize((word + 1) * LVW, 0);
        }
        let bit = 1u64 << (i % 64);
        let base = word * LVW;
        const L1: usize = LEVELS as usize - 1;
        for f in 0..SIG_FIELDS {
            let l = ((entry_signature >> (FB * f)) & (LEVELS - 1)) as usize;
            for k in 0..l {
                self.entry_levels[base + f * L1 + k] |= bit;
            }
            let l = ((query_signature >> (FB * f)) & (LEVELS - 1)) as usize;
            for k in l..L1 {
                self.query_levels[base + f * L1 + k] |= bit;
            }
        }
    }
    /// rebuild the level index from deck `from` on (after the arrays were compacted)
    pub(super) fn reindex_from(&mut self, from: usize) {
        let first_word = from / 64;
        self.entry_levels.truncate((first_word + 1) * LVW);
        self.query_levels.truncate((first_word + 1) * LVW);
        if self.entry_levels.len() == (first_word + 1) * LVW {
            let keep_bits = if from.is_multiple_of(64) { 0 } else { u64::MAX >> (64 - from % 64) };
            for word in &mut self.entry_levels[first_word * LVW..] {
                *word &= keep_bits;
            }
            for word in &mut self.query_levels[first_word * LVW..] {
                *word &= keep_bits;
            }
        }
        if self.entry_signatures.len() <= first_word * 64 {
            self.entry_levels.truncate(first_word * LVW);
            self.query_levels.truncate(first_word * LVW);
        }
        for deck in from..self.entry_signatures.len() {
            let (entry_sig, query_sig) = (self.entry_signatures[deck], self.query_signatures[deck]);
            self.index_set(deck, entry_sig, query_sig);
        }
    }
    /// Deck bits in `word` that pass all of the query's level-index tests.
    #[inline]
    pub(super) fn candidate_word(&self, word: usize, query_offsets: &[usize]) -> u64 {
        let remaining = self.entry_signatures.len() - word * 64;
        let mut candidates = if remaining >= 64 { u64::MAX } else { (1u64 << remaining) - 1 };
        let base = word * LVW;
        for &offset in query_offsets {
            candidates &= self.entry_levels[base + offset];
            if candidates == 0 {
                break;
            }
        }
        candidates
    }
}
/// Level-index offsets of a query signature: one per field whose level is at least 1.
pub(super) fn query_offsets(query_signature: Sig, out: &mut [usize; SIG_FIELDS]) -> usize {
    let mut n = 0;
    for f in 0..SIG_FIELDS {
        let l = ((query_signature >> (FB * f)) & (LEVELS - 1)) as usize;
        if l > 0 {
            out[n] = f * (LEVELS as usize - 1) + l - 1;
            n += 1;
        }
    }
    n
}

/// One kept deck located by the most recent dominance scan.
struct DominatorLocation {
    bucket: usize,
    deck: usize,
}

/// Kept decks of one pal count. A dominator's coordinate sum is at least the deck's (less float slack), so a
/// query only visits buckets at or above the deck's sum.
/// `recent`: coordinates of the last RECENT decks that dominated a new deck.
///   top_decks = 1: a deck on it may since have been dropped, but then a kept deck dominates it, so it still
///   proves that any deck it dominates is not needed.
///   top_decks > 1: kept decks are only dropped at compaction, which starts a fresh `DominanceIndex`, so every deck on it
///   is still kept; `recent_ids` holds their ids so each counts once.
pub(super) struct DominanceIndex {
    pub(super) coordinate_count: usize,
    pub(super) bucket_width: f64,
    pub(super) buckets: Vec<Bucket>,
    pub(super) deck_count: usize,
    pub(super) next_uid: u64,
    pub(super) last_dominator: Vec<f64>,
    pub(super) has_last_dominator: bool,
    pub(super) cap: usize,
    pub(super) recent: Vec<f64>,
    pub(super) recent_position: usize,
    pub(super) recent_count: usize,
    pub(super) recent_ids: Vec<u64>,
    found: Vec<DominatorLocation>,
}
pub(super) const RECENT: usize = 32;
impl DominanceIndex {
    pub(super) fn new(coordinate_count: usize) -> Self {
        Self {
            coordinate_count,
            bucket_width: (coordinate_count.max(1) as f64) / NBUCKET as f64,
            buckets: (0..NBUCKET).map(|_| Bucket::default()).collect(),
            deck_count: 0,
            next_uid: 0,
            last_dominator: vec![0.0; coordinate_count],
            has_last_dominator: false,
            cap: 512,
            recent: vec![0.0; RECENT * coordinate_count],
            recent_position: 0,
            recent_count: 0,
            recent_ids: vec![u64::MAX; RECENT],
            found: vec![],
        }
    }
    pub(super) const fn len(&self) -> usize {
        self.deck_count
    }
    #[inline]
    fn bucket_of(&self, coordinate_sum: f64) -> usize {
        ((coordinate_sum / self.bucket_width).floor().max(0.0) as usize).min(NBUCKET - 1)
    }
    /// number of kept decks dominating `coordinates` (stops at `need`)
    pub(super) fn count(
        &mut self,
        coordinates: &[f64],
        query_signature: Sig,
        coordinate_sum: f64,
        need: usize,
        tests: &mut usize,
    ) -> usize {
        let coordinate_count = self.coordinate_count;
        let first_bucket = self.bucket_of(coordinate_sum - SUM_EPS);
        let mut n = 0;
        self.found.clear();
        let mut query_word_offsets = [0usize; SIG_FIELDS];
        let query_word_count = query_offsets(query_signature, &mut query_word_offsets);
        for bucket_index in (first_bucket..NBUCKET).rev() {
            let bucket = &self.buckets[bucket_index];
            for word in 0..bucket.entry_signatures.len().div_ceil(64) {
                let mut candidate_mask = bucket.candidate_word(word, &query_word_offsets[..query_word_count]);
                while candidate_mask != 0 {
                    let i = word * 64 + candidate_mask.trailing_zeros() as usize;
                    candidate_mask &= candidate_mask - 1;
                    *tests += 1;
                    let row = &bucket.flat[i * coordinate_count..(i + 1) * coordinate_count];
                    if dominates(row, coordinates) {
                        n += 1;
                        self.found.push(DominatorLocation { bucket: bucket_index, deck: i });
                        if n >= need {
                            self.last_dominator.copy_from_slice(row);
                            self.has_last_dominator = true;
                            return n;
                        }
                    }
                }
            }
        }
        n
    }
    /// like `count`, without touching the remembered-dominator state (for a fixed reference set)
    pub(super) fn count_ro(
        &self,
        coordinates: &[f64],
        query_signature: Sig,
        coordinate_sum: f64,
        need: usize,
    ) -> usize {
        let coordinate_count = self.coordinate_count;
        let first_bucket = self.bucket_of(coordinate_sum - SUM_EPS);
        let mut n = 0;
        let mut query_word_offsets = [0usize; SIG_FIELDS];
        let query_word_count = query_offsets(query_signature, &mut query_word_offsets);
        for bucket_index in (first_bucket..NBUCKET).rev() {
            let bucket = &self.buckets[bucket_index];
            for word in 0..bucket.entry_signatures.len().div_ceil(64) {
                let mut candidate_mask = bucket.candidate_word(word, &query_word_offsets[..query_word_count]);
                while candidate_mask != 0 {
                    let i = word * 64 + candidate_mask.trailing_zeros() as usize;
                    candidate_mask &= candidate_mask - 1;
                    if dominates(&bucket.flat[i * coordinate_count..(i + 1) * coordinate_count], coordinates) {
                        n += 1;
                        if n >= need {
                            return n;
                        }
                    }
                }
            }
        }
        n
    }
    pub(super) fn count_recent(&self, coordinates: &[f64]) -> usize {
        let coordinate_count = self.coordinate_count;
        (0..self.recent_count)
            .filter(|&slot| {
                dominates(&self.recent[slot * coordinate_count..(slot + 1) * coordinate_count], coordinates)
            })
            .count()
    }
    /// put the dominators the last count found on `recent`
    pub(super) fn remember_found(&mut self) {
        let coordinate_count = self.coordinate_count;
        // (the buffer put back after, its capacity kept for the next `count`)
        let mut found = std::mem::take(&mut self.found);
        for &DominatorLocation { bucket: bucket_index, deck: i } in &found {
            let id = self.buckets[bucket_index].deck_ids[i];
            if self.recent_ids[..self.recent_count].contains(&id) {
                continue;
            }
            let slot = self.recent_position;
            let (recent, flat) = (&mut self.recent, &self.buckets[bucket_index].flat);
            recent[slot * coordinate_count..(slot + 1) * coordinate_count]
                .copy_from_slice(&flat[i * coordinate_count..(i + 1) * coordinate_count]);
            self.recent_ids[slot] = id;
            self.recent_position = (slot + 1) % RECENT;
            self.recent_count = (self.recent_count + 1).min(RECENT);
        }
        found.clear();
        self.found = found;
    }
    pub(super) fn insert(&mut self, deck: DeckEval) {
        self.insert_at(deck, None);
    }
    /// `insert` with the deck's coordinates given apart (`deck.dv` then left as it is: the enumeration's single-result
    /// path, where nothing reads a kept deck's `dv` again, leaves it empty)
    pub(super) fn insert_dv(&mut self, deck: DeckEval, dominance_values: &[f64]) {
        self.insert_at(deck, Some(dominance_values));
    }
    fn insert_at(&mut self, deck: DeckEval, dominance_values: Option<&[f64]>) {
        let bucket_index = self.bucket_of(deck.coordinate_sum);
        let id = self.next_uid;
        self.next_uid += 1;
        let bucket = &mut self.buckets[bucket_index];
        let i = bucket.list.len();
        {
            let dominance_values = dominance_values.unwrap_or(&deck.dominance_values);
            let (entry_signature, query_signature) = (sig_of(dominance_values, false), sig_of(dominance_values, true));
            bucket.index_set(i, entry_signature, query_signature);
            bucket.entry_signatures.push(entry_signature);
            bucket.query_signatures.push(query_signature);
            bucket.flat.extend_from_slice(dominance_values);
        }
        bucket.deck_ids.push(id);
        bucket.list.push(deck);
        self.deck_count += 1;
    }
    /// drop kept decks that `coordinates` dominates (only decks with sum <= `coordinate_sum` + slack can be), in place,
    /// keeping the order of the rest
    pub(super) fn remove_dominated_by(&mut self, coordinates: &[f64], entry_signature: Sig, coordinate_sum: f64) {
        let coordinate_count = self.coordinate_count;
        let top = self.bucket_of(coordinate_sum + SUM_EPS);
        // lq offsets of the dominator's entry signature: fields below the top level constrain the deck's query field
        let mut entry_offsets = [0usize; SIG_FIELDS];
        let mut entry_offset_count = 0;
        for f in 0..SIG_FIELDS {
            let l = ((entry_signature >> (FB * f)) & (LEVELS - 1)) as usize;
            if l < LEVELS as usize - 1 {
                entry_offsets[entry_offset_count] = f * (LEVELS as usize - 1) + l;
                entry_offset_count += 1;
            }
        }
        for bucket_index in 0..=top {
            let bucket = &mut self.buckets[bucket_index];
            let len = bucket.query_signatures.len();
            // the first deck it dominates: entry test (its entry signature over the deck's query one) first
            let mut first = None;
            'scan: for start in (0..len).step_by(64) {
                let n = len - start;
                let mut candidates = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
                let base = start / 64 * LVW;
                for &offset in &entry_offsets[..entry_offset_count] {
                    if candidates == 0 {
                        break;
                    }
                    candidates &= bucket.query_levels[base + offset];
                }
                while candidates != 0 {
                    let i = start + candidates.trailing_zeros() as usize;
                    candidates &= candidates - 1;
                    if dominates(coordinates, &bucket.flat[i * coordinate_count..(i + 1) * coordinate_count]) {
                        first = Some(i);
                        break 'scan;
                    }
                }
            }
            let Some(first_removed) = first else { continue };
            // Compact from the first removed deck, preserving survivor order. Dropped decks
            // end up at or beyond write_index and are truncated after all columns move.
            let mut write_index = first_removed;
            for read_index in first_removed + 1..len {
                if sig_ge(entry_signature, bucket.query_signatures[read_index])
                    && dominates(
                        coordinates,
                        &bucket.flat[read_index * coordinate_count..(read_index + 1) * coordinate_count],
                    )
                {
                    continue;
                }
                bucket.list.swap(write_index, read_index);
                bucket.entry_signatures[write_index] = bucket.entry_signatures[read_index];
                bucket.query_signatures[write_index] = bucket.query_signatures[read_index];
                bucket.deck_ids[write_index] = bucket.deck_ids[read_index];
                bucket.flat.copy_within(
                    read_index * coordinate_count..(read_index + 1) * coordinate_count,
                    write_index * coordinate_count,
                );
                write_index += 1;
            }
            self.deck_count -= len - write_index;
            bucket.list.truncate(write_index);
            bucket.entry_signatures.truncate(write_index);
            bucket.query_signatures.truncate(write_index);
            bucket.deck_ids.truncate(write_index);
            bucket.flat.truncate(write_index * coordinate_count);
            bucket.reindex_from(first_removed);
        }
    }
    pub(super) fn into_list(self) -> Vec<DeckEval> {
        self.buckets.into_iter().flat_map(|bucket| bucket.list).collect()
    }
}

/// The order k_filter(list, 1, nd).into_list() produces when it keeps every deck: stable by
/// (bucket of the sum ascending, sum descending), ties in input order.
pub(super) fn kfilter_order(mut decks: Vec<DeckEval>, coordinate_count: usize) -> Vec<DeckEval> {
    decks.sort_by(|a, b| b.coordinate_sum.partial_cmp(&a.coordinate_sum).unwrap());
    let kept = DominanceIndex::new(coordinate_count);
    decks.sort_by_key(|deck| kept.bucket_of(deck.coordinate_sum));
    decks
}
/// Keep each deck unless `kmax` decks scoring at least as high in every coordinate are kept before it (sorted
/// pass). A dominator's coordinate sum is at least the deck's, so only kept decks above that sum are checked.
pub(super) fn k_filter(mut decks: Vec<DeckEval>, kmax: usize, coordinate_count: usize) -> DominanceIndex {
    decks.sort_by(|a, b| b.coordinate_sum.partial_cmp(&a.coordinate_sum).unwrap());
    let mut kept = DominanceIndex::new(coordinate_count);
    let mut tests = 0usize;
    for deck in decks {
        let query_signature = sig_of(&deck.dominance_values, true);
        if kept.count(&deck.dominance_values, query_signature, deck.coordinate_sum, kmax, &mut tests) < kmax {
            kept.insert(deck);
        }
    }
    kept.has_last_dominator = false;
    kept
}

/// One candidate deck. `white_hint_chances`/`gold_hint_chances` are the per-target hint odds the metric uses;
/// `dominance_values` are the coordinates used for dominance (see `run_deck`), all higher-is-better, and
/// `coordinate_sum` is their sum.
#[derive(Clone)]
pub(super) struct DeckEval {
    pub(super) pals: usize,
    pub(super) borrowed: Option<usize>,
    pub(super) cards: Vec<usize>,
    pub(super) ends: Vec<u8>,
    pub(super) variants: Vec<u32>,
    pub(super) plain: usize,
    pub(super) filler: usize,
    pub(super) white_hint_chances: Vec<f64>,
    pub(super) gold_hint_chances: Vec<f64>,
    pub(super) dominance_values: Vec<f64>,
    pub(super) coordinate_sum: f64,
}

/// A deck's per-target (white, gold) hint odds.
pub(super) trait Odds {
    fn white_hint_chances(&self) -> &[f64];
    fn gold_hint_chances(&self) -> &[f64];
}
impl Odds for DeckEval {
    fn white_hint_chances(&self) -> &[f64] {
        &self.white_hint_chances
    }
    fn gold_hint_chances(&self) -> &[f64] {
        &self.gold_hint_chances
    }
}
impl Odds for PDeck {
    fn white_hint_chances(&self) -> &[f64] {
        &self.white_hint_chances
    }
    fn gold_hint_chances(&self) -> &[f64] {
        &self.gold_hint_chances
    }
}
/// A cluster of decks (indices into the list it was built from) with their componentwise maximum odds.
pub(super) struct DeckCluster {
    pub(super) max_white_chances: Vec<f64>,
    pub(super) max_gold_chances: Vec<f64>,
    pub(super) members: Vec<usize>,
}
/// Clusters of at most 16 decks by recursive median splits on the widest odds coordinate.
pub(super) fn split<D: Odds>(skyline: &[D], members: &mut [usize], uncertain_count: usize, out: &mut Vec<DeckCluster>) {
    if members.len() <= 16 {
        let mut max_white_chances = vec![0.0f64; uncertain_count];
        let mut max_gold_chances = vec![0.0f64; uncertain_count];
        for &j in members.iter() {
            for k in 0..uncertain_count {
                max_white_chances[k] = max_white_chances[k].max(skyline[j].white_hint_chances()[k]);
                max_gold_chances[k] = max_gold_chances[k].max(skyline[j].gold_hint_chances()[k]);
            }
        }
        out.push(DeckCluster { max_white_chances, max_gold_chances, members: members.to_vec() });
        return;
    }
    let coord = |deck: usize, coordinate: usize| {
        if coordinate < uncertain_count {
            skyline[deck].white_hint_chances()[coordinate]
        } else {
            skyline[deck].gold_hint_chances()[coordinate - uncertain_count]
        }
    };
    let mut lo = vec![f64::INFINITY; 2 * uncertain_count];
    let mut hi = vec![f64::NEG_INFINITY; 2 * uncertain_count];
    for &j in members.iter() {
        for k in 0..uncertain_count {
            lo[k] = lo[k].min(skyline[j].white_hint_chances()[k]);
            hi[k] = hi[k].max(skyline[j].white_hint_chances()[k]);
            lo[uncertain_count + k] = lo[uncertain_count + k].min(skyline[j].gold_hint_chances()[k]);
            hi[uncertain_count + k] = hi[uncertain_count + k].max(skyline[j].gold_hint_chances()[k]);
        }
    }
    let widest =
        (0..2 * uncertain_count).max_by(|&a, &b| (hi[a] - lo[a]).partial_cmp(&(hi[b] - lo[b])).unwrap()).unwrap();
    let mid = members.len() / 2;
    members.select_nth_unstable_by(mid, |&a, &b| coord(a, widest).partial_cmp(&coord(b, widest)).unwrap());
    let (lower, upper) = members.split_at_mut(mid);
    split(skyline, lower, uncertain_count, out);
    split(skyline, upper, uncertain_count, out);
}

/// A deck detached from the search that built it (names and ids instead of indices), for per-uma mode. Its names are
/// shared strings (`Name`: a skyline of 50k decks repeats about a hundred distinct ones), written as plain strings
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PDeck {
    pub(crate) names: Vec<Name>,
    pub(super) ids: Vec<u32>,
    pub(super) ends: Vec<u8>,
    #[serde(default)]
    #[serde(rename = "vars")]
    pub(super) variants: Vec<u32>,
    #[serde(default)]
    pub(super) choices: Vec<Name>,
    pub(super) plain_ids: Vec<u32>,
    pub(super) pal_names: Vec<Name>,
    pub(super) pals: usize,
    pub(super) borrowed: Option<Name>,
    #[serde(rename = "dw")]
    pub(super) white_hint_chances: Vec<f64>,
    #[serde(rename = "dg")]
    pub(super) gold_hint_chances: Vec<f64>,
}
/// A card or deck name shared between decks (serde's `rc`: serialised as the string)
pub(super) type Name = std::rc::Rc<str>;
impl PDeck {
    pub(crate) fn out(&self, uncertain_groups: &[u32]) -> Value {
        let odds: Vec<Value> = (0..uncertain_groups.len())
            .map(|k| json!({"group": uncertain_groups[k], "white": self.white_hint_chances[k], "gold": self.gold_hint_chances[k]}))
            .collect();
        json!({"pals": self.pals, "cards": self.names, "odds": odds, "pal_cards": self.pal_names, "borrowed": self.borrowed,
               "card_ids": self.ids, "end_chain_at": self.ends, "choices": self.choices, "plain_ids": self.plain_ids})
    }
    pub(super) fn blocks(&self, uma: u32, card_of: &crate::FxMap<u32, &Card>, checks: &crate::checks::Checks) -> bool {
        self.ids.iter().chain(&self.plain_ids).any(|id| card_of.get(id).is_some_and(|card| card.blocks(uma, checks)))
    }
    /// the deck's identity (cards, end links, variants, plain pals) as a fixed key: at most `DECK_ID_N` cards (a deck
    /// holds at most 6: the key codes count kinds in base 7), the unused places 0
    pub(super) fn id(&self) -> DeckId {
        let card_count = self.ids.len();
        assert!(
            card_count <= DECK_ID_N && self.ends.len() <= DECK_ID_N && self.variants.len() <= DECK_ID_N,
            "a deck of more than {DECK_ID_N} cards"
        );
        let mut key = DeckId {
            card_count: card_count as u8,
            end_count: self.ends.len() as u8,
            variant_count: self.variants.len() as u8,
            ids: [0; DECK_ID_N],
            ends: [0; DECK_ID_N],
            variants: [0; DECK_ID_N],
            plain_pal_count: self.plain_ids.len(),
        };
        key.ids[..card_count].copy_from_slice(&self.ids);
        key.ends[..self.ends.len()].copy_from_slice(&self.ends);
        key.variants[..self.variants.len()].copy_from_slice(&self.variants);
        key
    }
}
/// `PDeck::id`'s key: equal exactly when the (ids, ends, vars, plain pal count) are (the lengths kept)
pub(super) const DECK_ID_N: usize = 8;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct DeckId {
    card_count: u8,
    end_count: u8,
    variant_count: u8,
    ids: [u32; DECK_ID_N],
    ends: [u8; DECK_ID_N],
    variants: [u32; DECK_ID_N],
    plain_pal_count: usize,
}

/// Order for exact ties between deck entries: card order, except that one character's entries take her positions in
/// order of rarity (highest first; stable), so an SSR wins an exact tie with her own R card ([Planned Perfection] over
/// [Tracen Academy] Riko Kashimoto, the same event) and ties between different characters keep card order.
pub(super) fn tie_rank(relevant: &[usize], cards: &[&Card]) -> Vec<usize> {
    let mut rank: Vec<usize> = (0..relevant.len()).collect();
    let mut by_character: Vec<(u32, Vec<usize>)> = vec![];
    for (j, &card) in relevant.iter().enumerate() {
        match by_character.iter_mut().find(|group| group.0 == cards[card].character_id) {
            Some(group) => group.1.push(j),
            None => by_character.push((cards[card].character_id, vec![j])),
        }
    }
    for (_, positions) in &by_character {
        let mut by_rarity = positions.clone();
        by_rarity.sort_by_key(|&j| std::cmp::Reverse(cards[relevant[j]].rarity));
        for (&position, &j) in positions.iter().zip(&by_rarity) {
            rank[j] = position;
        }
    }
    rank
}
