"""One GameTora training event -> the rewards the card-event builders store (both eras). The files are read and their
reward codes decoded by gametora.py (`choices`, `outcomes`, `hints`, `pal_event`); outcomes are taken as equally likely.
  - continuing:  skill -> chance over the choices that keep the chain going (k of n outcomes: k/n), the best choice per skill;
  - ending:  the same over the choices that end the chain on every outcome (a separate "end the chain here" option);
  - stop_risk:  [k, n] when IT's default (first) choice ends the chain on k of its n outcomes (it is then not offered);
  - alternatives:  the full structure of the choices that keep the chain going, [choice][outcome] -> sorted skill names, when
           the continuing choices name two or more skills. Stored as `alts`, these replace the separate `continuing` chances,
           so skills on different choices are never credited together and outcomes of one choice exclude each other."""

from typing import NamedTuple

import gametora


def names_of(hints, skill_names):
    """the skill names of hints (`gametora.hint`), in order: a random hint gives every skill it can be"""
    out = []
    for kind, x, _ in hints:
        if kind == "hint" and x in skill_names:
            out.append(skill_names[x])
        if kind == "random":
            out.extend(skill_names[d] for d in x if d in skill_names)
    return out


class DecodedEvent(NamedTuple):
    """Per-skill chances and joint outcomes for continuing and ending a chain."""

    continuing: dict[str, float]
    ending: dict[str, float]
    has_ending: bool
    stop_risk: list[int] | None  # ending outcomes, total outcomes on the default choice
    alternatives: list[list[list[str]]] | None  # choice -> outcome -> skill names


def decode(event_choices, skill_names):
    continuing = {}
    ending = {}
    has_ending = False
    risk = None
    normal = []
    first = True
    for choice in gametora.choices(event_choices):
        is_default, first = first, False  # IT takes the first choice unless a prioritized skill is elsewhere
        outcomes = gametora.outcomes(choice[1])
        branches = [names_of(hints, skill_names) for hints, _ in outcomes]
        ends = [entry for _, entry in outcomes]
        # A choice that ends the chain on only some outcomes is not offered as a choice; when it is IT's default
        # choice, the chain stops at this link on k of its n outcomes (stop_risk).
        if any(ends) and not all(ends):
            if is_default:
                risk = [sum(ends), len(ends)]
            continue
        chances = ending if all(ends) else continuing
        if all(ends):
            has_ending = True
        else:
            normal.append([sorted(set(branch)) for branch in branches])
        names = {skill_name for branch in branches for skill_name in branch}
        for skill_name in names:
            chance = sum(skill_name in branch for branch in branches) / len(branches)
            chances[skill_name] = max(chances.get(skill_name, 0), chance)
    alts = normal if len({skill_name for choice in normal for branch in choice for skill_name in branch}) >= 2 else None
    return DecodedEvent(continuing=continuing, ending=ending, has_ending=has_ending, stop_risk=risk, alternatives=alts)


def fmt(d):
    return sorted([skill_name if chance == 1 else [skill_name, chance] for skill_name, chance in d.items()], key=str)


def skills_of(choice, skill_names):
    """Every skill a choice ([label, rewards]) can hint, on any of its outcomes."""
    return set(names_of(gametora.hints(choice[1]), skill_names))


def prioritise(event_choices, choice_number, skill_names):
    """IT takes an event's first choice unless a prioritized skill sits on another (docs/MODEL.md, Deck side, "IT event
    choices"): the skills that steer it to choice k (1-based) are those on choice k and on no choice above it. [] for
    k = 1; None when there is none (IT cannot be steered there)."""
    choices = gametora.choices(event_choices)
    if choice_number == 1:
        return []
    above = set().union(*[skills_of(choice, skill_names) for choice in choices[: choice_number - 1]])
    own = sorted(skills_of(choices[choice_number - 1], skill_names) - above)
    return own or None


MANUAL_KEYS = ("name", "always", "disabled", "chance", "note", "event")


def manual_entry(card_id, manual, skill_names):
    """A hand-entered (pal or group) card entry as the builders store it. `event: [story id, choice]` names the event and
    choice the entry models; `prioritise` is derived from it: the skills a player must prioritise for IT to take that choice."""
    entry = {x: manual[x] for x in manual if x in MANUAL_KEYS}
    if "event" in manual:
        story_id, choice_number = manual["event"]
        choices = gametora.pal_event(story_id, int(card_id))
        if choices is None:
            raise SystemExit(f"card {card_id}: event {story_id} not in the GameTora files")
        chance = prioritise(choices, choice_number, skill_names)
        if chance is None:
            raise SystemExit(f"card {card_id}: no skill steers IT to choice {choice_number} of event {story_id}")
        if chance:
            entry["prioritise"] = chance
    return {**entry, "status": "complete", "src": manual.get("src", "")}


def apply_shares(out, path):
    """Measured outcome shares (event_shares.json) onto generated entries: each listed skill's `chain` share at that link
    becomes its measured share, and `alt_w` gives the solver each choice's outcome weights (equal where none is listed)."""
    import json

    for card_id, share_entries in json.load(open(path))["cards"].items():
        entry = out.get(card_id)
        if not entry:
            continue
        for share_entry in share_entries:
            link_number, choice_number, shares = share_entry["link"], share_entry["choice"], share_entry["shares"]
            if abs(sum(shares.values()) - 1) > 1e-9:
                raise SystemExit(f"card {card_id} link {link_number}: shares do not add to 1")
            for chain_link, names in entry.get("chain", []):
                if chain_link != link_number:
                    continue
                for i, reward in enumerate(names):
                    skill_name = reward[0] if isinstance(reward, list) else reward
                    if skill_name in shares:
                        names[i] = skill_name if shares[skill_name] >= 1 else [skill_name, shares[skill_name]]
            alternatives = next((a for chain_link, a in entry.get("alts", []) if chain_link == link_number), None)
            if alternatives is None or choice_number > len(alternatives):
                raise SystemExit(f"card {card_id} link {link_number}: no choice {choice_number} in alts")
            weights = [[1 / len(choice)] * len(choice) for choice in alternatives]
            outcome_counts = {
                skill_name: sum(skill_name in outcome for outcome in alternatives[choice_number - 1])
                for skill_name in shares
            }  # a skill on several outcomes splits its share
            for j, outcome in enumerate(alternatives[choice_number - 1]):
                hits = [
                    shares[skill_name] / outcome_counts[skill_name] for skill_name in outcome if skill_name in shares
                ]
                if len(hits) != 1:
                    raise SystemExit(
                        f"card {card_id} link {link_number} outcome {j + 1}: needs exactly one listed skill, has {outcome}"
                    )
                weights[choice_number - 1][j] = hits[0]
            entry.setdefault("alt_w", []).append([link_number, weights])
