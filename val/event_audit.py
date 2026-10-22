"""Audit the hand-entered pal and group card entries against GameTora's event files.

For every Global pal / group card, reads every event section GameTora stores (storyline, first meeting, outings,
card-specific events, per-card variants) through pkg/loopopt/export/gametora.py, and lists each target-skill hint the
card can give. Each is checked against the card's entry in
pkg/loopopt/data/global/card_events.json (always / chance / disabled). Exit code 1 if any target hint is unaccounted for.

usage (repo root): uv run val/event_audit.py [--all]     --all also prints the non-target hints
Needs game/global/master.mdb.
"""

import json
import sqlite3
import sys
import os
from typing import NamedTuple


class EventHint(NamedTuple):
    skill_name: str
    level: int | str


class EventHints(NamedTuple):
    title: str
    card_variant: int | None  # None means the event applies to every owning card.
    hints: list[EventHint]


REPO = os.path.dirname(os.path.abspath(__file__)) + "/../"
sys.path.insert(0, REPO + "pkg/loopopt/export")
import gametora

connection = sqlite3.connect(REPO + "game/global/master.mdb")
query = lambda s, *a: connection.execute(s, a).fetchall()
skill_names = dict(query("select [index], text from text_data where category = 47"))
event_titles = dict(query("select [index], text from text_data where category = 181"))
card_names = dict(query("select [index], text from text_data where category = 75"))
pal_characters = dict(query("select id, chara_id from support_card_data where support_card_type in (2, 3)"))
TARGETS = {
    "Uma Stan",
    "Superstan",
    "Nimble Navigator",
    "No Stopping Me!",
    "Playtime's Over!",
    "See Ya Later!",
    "Ramp Up",
    "It's On!",
    "Tail Held High",
    "Tail Nine",
    "Racing Spirit: Stamina",
    "Slipstream",
    "Perfect Spot!",
    "Risky Business",
    "Nothing Ventured",
}
SHOW_ALL = "--all" in sys.argv


def event_hints(choices):
    """(skill name, level or 'random') for every hint of an event's choices"""
    out = []
    for choice in gametora.choices(choices):
        for kind, skill, level in gametora.hints(choice[1]):
            if kind == "hint":
                out.append(EventHint(skill_name=skill_names.get(skill, f"(JP-only {skill})"), level=level))
            else:
                out += [EventHint(skill_name=skill_names.get(d, "?"), level="random") for d in skill]
    return out


def event_hints_by_variant(row, kind):
    """Yield (event title, card id a variant is limited to or None, hints)."""
    for event, card_variant, choices in gametora.events(row, kind):
        event_name = event_titles.get(int(event[2]) if str(event[2]).isdigit() else 0, str(event[2]))
        yield EventHints(title=event_name, card_variant=card_variant, hints=event_hints(choices))


card_events = json.load(open(REPO + "pkg/loopopt/data/global/card_events.json"))["cards"]
missing = 0
for kind in ("friend", "group"):
    for row in gametora.load(f"training_events_{kind}"):
        owners = [
            card_id
            for card_id, character_id in pal_characters.items()
            if (character_id == row[0] if kind == "friend" else card_id == row[0])
        ]
        if not owners:
            continue
        events = list(event_hints_by_variant(row, kind))
        for card_id in sorted(owners):
            event = card_events.get(str(card_id), {})
            covered = (
                set(event.get("always", []))
                | {n for n, _ in event.get("chance", [])}
                | {n for n, _ in event.get("disabled", [])}
            )
            print(
                f"{card_id} {card_names.get(card_id)}  [always={event.get('always')} chance={event.get('chance')} disabled={event.get('disabled')}]"
            )
            for event_name, card_variant, hints in events:
                if card_variant is not None and card_variant != card_id:
                    continue
                if card_variant is None and any(o == card_id and n == event_name for n, o, _ in events):
                    continue  # this card has its own version of the event
                for h, level in hints:
                    if h in TARGETS:
                        ok = h in covered
                        missing += not ok
                        print(f"   {'ok     ' if ok else 'MISSING'} {h} {level}  ({event_name})")
                    elif SHOW_ALL:
                        print(f"           {h} {level}  ({event_name})")
print(f"\n{missing} target hint(s) not accounted for" if missing else "\nevery target hint is accounted for")
sys.exit(1 if missing else 0)
