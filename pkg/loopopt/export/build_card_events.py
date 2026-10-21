"""Build the era's card_events.json from the cached GameTora training-event files (cache/gametora/: ./build.sh gametora)
+ master.mdb. usage: build_card_events.py global|jp
Chain links come from master.mdb (single_mode_story_data.show_progress_1 for the event's story id). The files are
read and decoded by gametora.py.
A skill in every outcome of some choice is certain when the link fires; a skill in k of a choice's n outcomes
(split by 'di') is stored with chance k/n, outcomes taken as equally likely. Pal entries and bug notes are kept by hand.
Per card: `chain` [link, rewards of the choice IT takes by default]; `end` [link, rewards of a choice that ends the chain
early at that link] (always a link before the last: reaching the last link is the chain's normal end, not an `end`
entry); `stop_risk` [link, k, n] when IT's default choice there ends the chain on k of n outcomes; `alts` [link, every
choice's outcomes] where choices differ in targets."""

import json, os
import gametora
from event_choices import decode, fmt, manual_entry, apply_shares
from era import q, names, OUT, released

skill_names = names(47)
card_names = names(75)
story_links = dict(q("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0"))
released_cards = {i for i, sd in q("select id, start_date from support_card_data") if released(sd)}
out = {}
for key in ["training_events_sr", "training_events_ssr"]:
    for card_id, events in gametora.load(key):
        if card_id not in released_cards:
            continue
        chain = []
        chain_ends = []
        stop_risks = []
        alternatives = []
        for event in events:
            link_number = story_links.get(int(event[2]))
            if not link_number:
                continue
            decoded = decode(event[1], skill_names)
            if decoded.stop_risk:
                stop_risks.append([link_number, *decoded.stop_risk])
            if decoded.continuing:
                chain.append([link_number, fmt(decoded.continuing)])
            if decoded.has_ending:
                chain_ends.append([link_number, fmt(decoded.ending)])
            if decoded.alternatives:
                alternatives.append([link_number, decoded.alternatives])
        out[str(card_id)] = {
            "name": card_names.get(card_id, str(card_id)),
            "chain": sorted(chain),
            **({"end": sorted(chain_ends)} if chain_ends else {}),
            **({"stop_risk": sorted(stop_risks)} if stop_risks else {}),
            **({"alts": sorted(alternatives)} if alternatives else {}),
            "status": "complete",
            "src": "gametora training_events",
        }
apply_shares(
    out, os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "data", "event_shares.json")
)  # measured outcome shares replace the equal-outcome convention
# the hand-entered entries (pal and group cards, corrections): the era's manual.json
MANUAL = os.path.join(OUT, "manual.json")
DEST = os.path.join(OUT, "card_events.json")
manual = json.load(open(MANUAL))["cards"]
for k, v in manual.items():
    if int(k) not in released_cards:
        continue  # released after the cut
    if any(x in v for x in ("always", "disabled", "chance")):
        out[k] = {**out.get(k, {}), **manual_entry(k, v, skill_names)}
doc = {"about": json.load(open(MANUAL))["about"], "cards": dict(sorted(out.items()))}
json.dump(doc, open(DEST, "w"), ensure_ascii=False, indent=1)
print(len(out), "cards written")
