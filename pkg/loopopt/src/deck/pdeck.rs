//! The decks a search returns (`PDeck`: detached from the search, names and ids instead of indices), their shared
//! name strings (`Name`) and identity key (`DeckId`).
use crate::deck::Card;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
/// A deck detached from the search that built it (names and ids instead of indices), for per-uma mode. Its names are
/// shared strings (`Name`: a skyline of 50k decks repeats about a hundred distinct ones), written as plain strings
#[derive(Clone, Serialize, Deserialize)]
pub(crate) struct PDeck {
    pub(crate) names: Vec<Name>,
    pub(crate) ids: Vec<u32>,
    pub(crate) ends: Vec<u8>,
    #[serde(default)]
    #[serde(rename = "vars")]
    pub(crate) variants: Vec<u32>,
    #[serde(default)]
    pub(crate) choices: Vec<Name>,
    pub(crate) plain_ids: Vec<u32>,
    pub(crate) pal_names: Vec<Name>,
    pub(crate) pals: usize,
    pub(crate) borrowed: Option<Name>,
    #[serde(rename = "dw")]
    pub(crate) white_hint_chances: Vec<f64>,
    #[serde(rename = "dg")]
    pub(crate) gold_hint_chances: Vec<f64>,
}
/// A card or deck name shared between decks (serde's `rc`: serialised as the string)
pub(crate) type Name = std::rc::Rc<str>;
impl PDeck {
    pub(crate) fn out(&self, uncertain_groups: &[u32]) -> Value {
        let odds: Vec<Value> = (0..uncertain_groups.len())
            .map(|k| json!({"group": uncertain_groups[k], "white": self.white_hint_chances[k], "gold": self.gold_hint_chances[k]}))
            .collect();
        json!({"pals": self.pals, "cards": self.names, "odds": odds, "pal_cards": self.pal_names, "borrowed": self.borrowed,
               "card_ids": self.ids, "end_chain_at": self.ends, "choices": self.choices, "plain_ids": self.plain_ids})
    }
    pub(crate) fn blocks(&self, uma: u32, card_of: &crate::FxMap<u32, &Card>, checks: &crate::checks::Checks) -> bool {
        self.ids.iter().chain(&self.plain_ids).any(|id| card_of.get(id).is_some_and(|card| card.blocks(uma, checks)))
    }
    /// the deck's identity (cards, end links, variants, plain pals) as a fixed key: at most `DECK_ID_N` cards (a deck
    /// holds at most 6: the key codes count kinds in base 7), the unused places 0
    pub(crate) fn id(&self) -> DeckId {
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
/// `PDeck::id`'s key: equal exactly when the (ids, ends, variants, plain pal count) are (the lengths kept)
const DECK_ID_N: usize = 8;
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct DeckId {
    card_count: u8,
    end_count: u8,
    variant_count: u8,
    ids: [u32; DECK_ID_N],
    ends: [u8; DECK_ID_N],
    variants: [u32; DECK_ID_N],
    plain_pal_count: usize,
}
