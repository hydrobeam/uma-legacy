"""Every conditional character event (secret and race-result events; GameTora training_events_char, row index 4) that gives a skill hint, with its
conditions decoded, and a feasibility check of the race conditions against the solver's Global data.json.
Writes cache/secret_events.md (per uma, then an index by skill). usage: uv run val/secret_events.py [OUT.md]"""

import json
import sqlite3
import sys
import os
from dataclasses import dataclass
from typing import NamedTuple


class SkillHint(NamedTuple):
    name: str
    level: int | str
    is_target: bool


@dataclass(frozen=True)
class EventVariant:
    """One event version, with decoded hints and career-feasibility conditions."""

    uma: str
    character_id: int
    in_global: bool
    title: str
    version: str
    hints: list[SkillHint]
    choices: list[list[SkillHint]]
    conditions: list[str]
    impossible: bool


REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
sys.path.insert(0, REPO + "/pkg/loopopt/export")
import gametora

DATA_DIR = REPO + "/pkg/loopopt/data/global/"
c = sqlite3.connect(f"file:{REPO}/game/global/master.mdb?mode=ro", uri=True)
skill_names = dict(c.execute("select [index],text from text_data where category=47"))
skill_groups = dict(c.execute("select id, group_id from skill_data"))
event_titles = dict(c.execute("select [index],text from text_data where category=181"))
character_names = dict(c.execute("select [index],text from text_data where category=6"))
cj = sqlite3.connect(f"file:{REPO}/game/jp/master.mdb?mode=ro", uri=True)
for k, v in cj.execute("select [index],text from text_data where category=6"):
    character_names.setdefault(k, v)
for k, v in cj.execute("select [index],text from text_data where category=181"):
    event_titles.setdefault(k, v)
jp_skill_names = dict(cj.execute("select [index],text from text_data where category=47"))
for k, v in jp_skill_names.items():
    skill_names.setdefault(k, v)
for k, g in cj.execute("select id, group_id from skill_data"):
    skill_groups.setdefault(k, g)
race_instances = {
    i: (race_id, nm)
    for i, race_id, nm in c.execute(
        "select ri.id, ri.race_id, (select text from text_data where category=28 and [index]=ri.id) from race_instance ri"
    )
}
d = json.load(open(DATA_DIR + "data.json"))
race_indices = {r: k for k, r in enumerate(d["races"])}
umas = {u["id"]: u for u in d["umas"]}
TARGETS = {20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203}
YEAR = {1: "Junior", 2: "Classic", 3: "Senior"}


def forced(u):
    """zero-based turn -> allowed option indices for a single-alternative goal (input goal turns are one-based)"""
    out = {}
    for alts in u["goals"]:
        if len(alts) == 1:
            t, o = alts[0]
            out[t - 1] = {o}
    return out


def feasible(character_id, instance_id, year):
    """'ok' / 'impossible: reason' / 'unknown: reason' for winning race instance `instance_id` in `year` (None = any)."""
    if character_id not in umas:
        return "unknown: uma not in Global data"
    race_id, _name = race_instances.get(instance_id, (None, str(instance_id)))
    if race_id not in race_indices:
        return "unknown: not a G1 (outside solver data)"
    u = umas[character_id]
    f = forced(u)
    turns = []
    for t, opts in enumerate(u["turns"]):
        if year and t // 24 != year - 1:
            continue
        for o, op in enumerate(opts):
            if op["race"] == race_indices[race_id]:
                turns.append((t, o))
    if not turns:
        return f"impossible: not offered{' in ' + YEAR[year] if year else ''}"
    free = [(t, o) for t, o in turns if t not in f or o in f[t]]
    if not free:
        t, o = turns[0]
        other = [u["turns"][t][x]["name"] for x in f[t]]
        return f"impossible: clashes with goal {'/'.join(other)}"
    return "ok"


def rref(v):
    v = str(v)
    return (int(v.split("|")[0]), int(v.split("|")[1])) if "|" in v else (int(v), None)


def rname(v):
    instance_id, yr = rref(v)
    nm = race_instances.get(instance_id, (None, f"race {instance_id}"))[1] or f"race {instance_id}"
    return nm + (f" ({YEAR[yr]})" if yr else "")


def rfeas(character_id, v):
    instance_id, yr = rref(v)
    return feasible(character_id, instance_id, yr)


MUST_RUN = {
    "win",
    "participate",
    "beat_rival",
    "lose_to_rival",
    "rival_draw",
    "race_w2",
    "pick_and_win",
    "dont_pick_and_win",
    "win_on_streak",
    "race_pn",
    "lose",
    "win_as_strat",
    "win_as_not_strat",
}


def describe_conditions(character_id, cs):
    out, bad = [], []
    for cnd in cs or []:
        kind, args = cnd[0], cnd[1:]
        try:
            if kind in MUST_RUN and args and not isinstance(args[0], list) and str(args[0]).replace("|", "").isdigit():
                fz = rfeas(character_id, args[0])
                rest = " ".join(str(a) for a in args[1:])
                extra = (
                    f" vs {character_names.get(int(args[1]), args[1])}"
                    if kind in ("beat_rival", "lose_to_rival", "rival_draw")
                    else (f" {rest}" if rest else "")
                )
                out.append(f"{kind} {rname(args[0])}{extra}" + ("" if fz == "ok" else f" [{fz}]"))
                if fz.startswith("impossible"):
                    bad.append(rname(args[0]))
            elif kind in ("win_or", "win_n_of", "win_all"):
                n, lst = (
                    (1, args)
                    if kind == "win_or"
                    else ((args[0], args[1]) if kind == "win_n_of" else (len(args[0]), args[0]))
                )
                ok = [v for v in lst if not rfeas(character_id, v).startswith("impossible")]
                out.append(
                    f"{kind} {n} of: "
                    + ", ".join(
                        rname(v) + ("" if not rfeas(character_id, v).startswith("impossible") else " [impossible]")
                        for v in lst
                    )
                )
                if len(ok) < n:
                    bad.append(kind)
            elif kind in ("do_not_participate", "do_not_win"):
                out.append(f"{kind} {rname(args[0])}")
            elif kind == "date":
                out.append(f"fires: {YEAR.get(args[0], args[0])} month {args[1]} {'early' if args[2] == 1 else 'late'}")
            elif kind.endswith("rival") and args and str(args[0]).isdigit():
                out.append(f"{kind} {character_names.get(int(args[0]), args[0])}")
            else:
                nm = lambda a: character_names.get(a, a) if isinstance(a, int) and 1000 < a < 2000 else a
                out.append(
                    " ".join(
                        [kind]
                        + [
                            json.dumps([nm(x) for x in a] if isinstance(a, list) else nm(a), ensure_ascii=False)
                            for a in args
                        ]
                    )
                )
        except Exception as ex:
            out.append(json.dumps(cnd, ensure_ascii=False) + f" (unparsed: {ex})")
    return out, bad


rows = []
for r in gametora.load("training_events_char"):
    character_id = r[0]
    for tag, event_version in gametora.conditional_events(r):
        hints = []
        choices = []
        for ch in gametora.choices(event_version[1]):
            hints_before = len(hints)
            for kind, x, level in gametora.hints(ch[1]):
                if kind == "hint":
                    hints.append(
                        SkillHint(name=skill_names.get(x, x), level=level, is_target=skill_groups.get(x) in TARGETS)
                    )
                else:
                    hints.append(
                        SkillHint(
                            name="random: " + "/".join(skill_names.get(d, "?") for d in x),
                            level="",
                            is_target=any(skill_groups.get(d) in TARGETS for d in x),
                        )
                    )
            choices.append(hints[hints_before:])
        if not hints:
            continue
        conditions, bad = describe_conditions(character_id, gametora.extra(event_version, 0))
        version = gametora.extra(event_version, 8)
        rows.append(
            EventVariant(
                uma=character_names.get(character_id, character_id),
                character_id=character_id,
                in_global=character_id in umas,
                title=str(event_titles.get(int(event_version[2]), "")).replace("\\n", " ").replace("\n", " "),
                version=tag + (f" [{version}]" if version is not None else ""),
                hints=hints,
                choices=choices,
                conditions=conditions,
                impossible=bad,
            )
        )
output_path = sys.argv[1] if len(sys.argv) > 1 else REPO + "/cache/secret_events.md"
print(
    len(rows),
    "event variants with hints;",
    len({x.character_id for x in rows}),
    "umas;",
    sum(any(h.is_target for h in x.hints) for x in rows),
    "touch a default target",
)

R = rows
md = [
    "# Conditional character events that give skill hints",
    "",
    "Source: GameTora `training_events/char` (the per-character section of events with trigger conditions, which holds the",
    "secret events and the race-result events). Rewards decoded from `dict_evrew` (offset 36), choice rewards only.",
    "`*` = one of the default 8 targets. With several choices, IT takes choice 1 unless a prioritized skill is on another.",
    'Race feasibility is checked against the solver\'s Global G1 data: "impossible" = never offered in that year, or on a',
    "turn forced to another career goal. Non-G1 races and other condition kinds are shown raw and not checked.",
    "Older versions (GameTora tags such as pre_2nd_anni) appear only where hints or conditions differ.",
    "",
]
by = {}
for r in R:
    by.setdefault((not r.in_global, str(r.uma), r.title), []).append(r)
last = None
for (jp, uma, title), variants in sorted(by.items()):
    if (jp, uma) != last:
        md += ["", f"## {uma}" + (" (JP only)" if jp else ""), ""]
        last = (jp, uma)
    seen = set()
    for i, v in enumerate(variants):
        key = (json.dumps(v.choices), json.dumps(v.conditions))
        if key in seen:
            continue
        seen.add(key)
        fmt = lambda hs: (
            ", ".join(f"{h.name} {h.level}".strip() + (" *" if h.is_target else "") for h in hs) or "no hint"
        )
        ch = list(v.choices)
        rewards = fmt(ch[0]) if len(ch) == 1 else " | ".join(f"choice {k + 1}: {fmt(c)}" for k, c in enumerate(ch))
        version = "" if i == 0 else f" ({v.version})"
        md.append(f"- **{title or '(untitled)'}**{version}: {rewards}  ")
        md.append(
            f"  needs: {'; '.join(v.conditions) or 'no conditions listed'}"
            + (" — **impossible in this career**" if v.impossible else "")
        )
os.makedirs(os.path.dirname(output_path), exist_ok=True)
open(output_path, "w").write("\n".join(md) + "\n")
# index by skill: current versions only
by_skill = {}
for r in R:
    if not r.version.startswith("current"):
        continue
    for k, c in enumerate(r.choices):
        for h in c:
            by_skill.setdefault(h.name, []).append((r, k, h.level))
out = ["", "# Index by skill (current versions)", ""]
for skill_name in sorted(by_skill, key=str):
    ents = "; ".join(
        f"{r.uma}{' (JP only)' if not r.in_global else ''}: {r.title or '(untitled)'} {level}"
        + (f" (choice {k + 1})" if len(r.choices) > 1 else "")
        + (" — impossible" if r.impossible else "")
        for r, k, level in by_skill[skill_name]
    )
    out.append(f"- **{skill_name}**: {ents}")
open(output_path, "a").write("\n".join(out) + "\n")
