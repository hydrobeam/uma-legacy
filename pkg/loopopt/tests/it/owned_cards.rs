//! `owned_cards` is the set of cards the player owns, every one listed: on val/jobs/pal_U, a job owning none uses no
//! card of its own. With `borrow` each deck holds at most one card, the borrowed one, beside the fillers; without it,
//! only fillers.
use crate::common::{run_job, val_job};
use serde_json::{Value, json};

/// pal_U's three best decks owning no card, with a borrowed card allowed or not.
fn owning_none(borrow: bool) -> Value {
    let mut job = val_job("jobs/pal_U.json");
    job["job"]["deck"]["owned_cards"] = json!([]);
    job["job"]["deck"]["borrow"] = json!(borrow);
    job["job"]["deck"]["top_decks"] = json!(3);
    run_job("owned-cards", &job)
}

#[test]
fn owning_none_leaves_the_borrow_and_fillers() {
    for borrow in [true, false] {
        let out = owning_none(borrow);
        let top = out["top"].as_array().unwrap_or_else(|| panic!("borrow {borrow}: {out}"));
        assert!(!top.is_empty(), "borrow {borrow}: no deck");
        for row in top {
            let deck = &row["deck"];
            let cards = deck["card_ids"].as_array().unwrap().len() + deck["plain_ids"].as_array().unwrap().len();
            if borrow {
                assert!(cards <= 1, "more than the borrowed card: {deck}");
                assert_eq!(cards == 1, !deck["borrowed"].is_null(), "the one card is the borrowed one: {deck}");
            } else {
                assert_eq!(cards, 0, "a card without owning or borrowing it: {deck}");
            }
        }
    }
}
