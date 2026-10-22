"""GameTora's data files into cache/gametora/ (not committed): the event and skill tables card events are built from.

  uv run gametora.py            download the versions pinned in ../data/gametora.json that are not cached yet
  uv run gametora.py --latest   pin the newest versions from GameTora's manifest first, then download them

Files are saved as <key with / as _, static/ dropped>.<hash>.json (dict/evrew -> dict_evrew.8bda1ecc.json); a cached
file of the same key under another hash is removed, so each key has one file and readers can glob it. If a pinned version is no longer available online, rebuilding it needs a cached copy;
`--latest` updates the pins and may change the exported events.
Each run also writes ../data/gametora_slugs.json (tracked): the page's links to GameTora, so the page build needs no
download.

Imported, it is the one reader of those files (no other code decodes them). The format, as decoded here:
  - load(key) reads cache/gametora/<key>.<hash>.json (key as cached: 'dict_evrew', 'training_events_ssr', ...).
  - An event is [id, choices, story id, extras...]. Its title is master.mdb's text_data category 181 by story id.
  - A choice is [label, rewards]; only `rewards` is rewards: decoding the label produces hints that don't exist.
  - A reward code k indexes dict_evrew at k - 36. Kinds used: 'sk' [kind, level, skill id]
    a skill hint, 'sr' [kind, _, [{d: skill id}, ...]] a random hint among skills, 'di' starts the choice's next
    outcome (outcomes equally likely), 'ee' ends the card's event chain.
  - training_events_sr / _ssr: rows [card id, events] (the chain links come from master.mdb, show_progress_1 of the
    event's story id).
  - training_events_friend (pal cards): rows [chara id, section, section, section]: storyline, first meeting, outings
    and card-specific events; training_events_group: rows [card id, member chara ids, sections...]. An event of either
    may carry per-card variants in extras[0] = [_, [[card id, [_, choices]], ...]]: a card with its own variant sees
    that one (`events`, `pal_event`).
  - training_events_char: rows [chara id, meta, story events, date events, conditional events (secret and race-result
    events), localisation]. A conditional event's extras hold [0, conditions], [8, version] and [9, [[tag, older
    event], ...]] (`conditional_events`, `extra`).
"""

import glob, json, os, sys, urllib.request
from typing import NamedTuple

H = os.path.dirname(os.path.abspath(__file__))
R = os.path.dirname(os.path.dirname(os.path.dirname(H)))
PINS = os.path.join(H, "..", "data", "gametora.json")
CACHE = os.path.join(R, "cache", "gametora")
UA = "legacy-loop-planner build script"


def get(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={"User-Agent": UA}), timeout=60) as r:
        return r.read()


def local_name(key):
    return key.removeprefix("static/").replace("/", "_")


def main(argv):
    pins = json.load(open(PINS))
    if "--latest" in argv:
        manifest = json.loads(get(pins["manifest"]))
        moved = {k: manifest[k] for k in pins["files"] if manifest.get(k) and manifest[k] != pins["files"][k]}
        pins["files"].update(moved)
        with open(PINS, "w") as f:
            json.dump(pins, f, indent=1)
            f.write("\n")
        print(f"gametora: {len(moved)} newer: {', '.join(moved) or 'none'}")
    os.makedirs(CACHE, exist_ok=True)
    got = 0
    for key, h in pins["files"].items():
        name = local_name(key)
        dst = os.path.join(CACHE, f"{name}.{h}.json")
        for f in os.listdir(CACHE):
            if f.startswith(name + ".") and f.endswith(".json") and f != os.path.basename(dst) and f.count(".") == 2:
                os.remove(os.path.join(CACHE, f))
        if os.path.exists(dst):
            continue
        data = get(f"{pins['base']}/{key}.{h}.json")
        json.loads(data)
        with open(dst + ".part", "wb") as f:
            f.write(data)
        os.replace(dst + ".part", dst)
        got += 1
    print(f"gametora: {got} downloaded, {len(pins['files']) - got} cached in {os.path.relpath(CACHE, R)}")
    write_slugs()


SLUGS = os.path.join(H, "..", "data", "gametora_slugs.json")


def write_slugs():
    """GameTora's page names, for the page's links (webapp/build.py reads them): per support card its url_name (e.g.
    30067-the-thrones-assemblage, /supports/<url_name>), per character the slug of her character pages
    (/characters/<outfit id>-<slug>), taken from her support cards' url_name"""
    cards = load("support-cards")
    out = {
        "about": "GameTora page names for the page's links, from its support-card list (pkg/loopopt/export/gametora.py "
        "writes it with every download). Data (c) GameTora.",
        "cards": {str(c["support_id"]): c["url_name"] for c in sorted(cards, key=lambda c: c["support_id"])},
        "chars": {},
    }
    for c in sorted(cards, key=lambda c: c["support_id"]):
        if c.get("char_id") and "-" in c.get("url_name", ""):
            out["chars"][str(c["char_id"])] = c["url_name"].split("-", 1)[1]
    with open(SLUGS, "w") as f:
        json.dump(out, f, indent=1, ensure_ascii=False)
        f.write("\n")


# ---- reading and decoding the cached files ----
OFF = 36  # reward codes index dict_evrew at code - OFF
_cache = {}


def load(key):
    """the cached file of `key` ('dict_evrew', 'training_events_ssr', ...), parsed once"""
    if key not in _cache:
        files = sorted(glob.glob(os.path.join(CACHE, f"{key}.*.json")))
        if not files:
            raise SystemExit(f"cache/gametora has no {key}: run ./build.sh gametora")
        _cache[key] = json.load(open(files[-1]))
    return _cache[key]


def reward(code):
    """the dict_evrew entry of a reward code, or None"""
    ev = load("dict_evrew")
    i = code - OFF if isinstance(code, int) else -1
    return ev[i] if 0 <= i < len(ev) else None


def is_choice(ch):
    return isinstance(ch, list) and len(ch) == 2 and isinstance(ch[1], list)


def choices(event_choices):
    """the event's choices ([label, rewards]), in order; anything else in the list is skipped"""
    return [ch for ch in (event_choices if isinstance(event_choices, list) else []) if is_choice(ch)]


class SkillHint(NamedTuple):
    """A fixed or random skill hint; random hints have no fixed level."""

    kind: str
    skills: int | list[int]
    level: int | None


class ChoiceOutcome(NamedTuple):
    """Hints and chain termination for one equally likely event outcome."""

    hints: list[SkillHint]
    ends_chain: bool


class EventVariant(NamedTuple):
    """An event's choices, optionally restricted to one support card."""

    event: list
    card_id: int | None
    choices: list


def hint(code):
    """a reward code as a skill hint: ('hint', skill id, level), ('random', [skill ids], None), or None"""
    r = reward(code)
    if not r:
        return None
    if r[0] == "sk":
        return SkillHint(kind="hint", skills=r[2], level=r[1])
    if r[0] == "sr":
        return SkillHint(kind="random", skills=[x["d"] for x in r[2]], level=None)
    return None


def hints(rewards):
    """the skill hints of one choice's rewards, in order"""
    return [h for h in map(hint, rewards if isinstance(rewards, list) else []) if h]


def outcomes(rewards):
    """one choice's rewards split into its outcomes (at each 'di'): [(hints, ends the chain)], hints as `hint` gives them"""
    out = [ChoiceOutcome(hints=[], ends_chain=False)]
    for k in rewards if isinstance(rewards, list) else []:
        r = reward(k)
        if r and r[0] == "di":
            out.append(ChoiceOutcome(hints=[], ends_chain=False))
            continue
        if r and r[0] == "ee":
            out[-1] = ChoiceOutcome(hints=out[-1].hints, ends_chain=True)
        h = hint(k)
        if h:
            out[-1].hints.append(h)
    return out


def sections(row, kind):
    """the event sections of a training_events_friend / _group row"""
    return row[1:] if kind == "friend" else row[2:]


def events(row, kind):
    """(event, card id its variant is limited to or None, its choice list) for every event of a friend / group row: the
    event itself, then each per-card variant"""
    for part in sections(row, kind):
        for e in part if isinstance(part, list) else []:
            if not (isinstance(e, list) and len(e) >= 3):
                continue
            yield EventVariant(event=e, card_id=None, choices=e[1])
            if len(e) > 3 and isinstance(e[3], list) and len(e[3]) > 1 and isinstance(e[3][1], list):
                for z in e[3][1]:
                    if isinstance(z, list) and len(z) >= 2 and isinstance(z[1], list) and len(z[1]) >= 2:
                        yield EventVariant(event=e, card_id=z[0], choices=z[1][1])


def pal_event(sid, cid):
    """The choices of pal or group event `sid` (a story id) as card `cid` sees them: its per-card variant when it has
    one; None when no such event exists"""
    for kind in ("friend", "group"):
        for row in load(f"training_events_{kind}"):
            base = None
            for e, only, chs in events(row, kind):
                if base is not None and e is not base:
                    return base[1]
                if str(e[2]) != str(sid):
                    continue
                if only is None:
                    base = e
                elif only == cid:
                    return chs
            if base is not None:
                return base[1]
    return None


def conditional_events(row):
    """(tag, event) for every conditional event of a training_events_char row: 'current', then each older version"""
    for e in row[4] if len(row) > 4 and isinstance(row[4], list) else []:
        yield "current", e
        for x in e[3:]:
            if isinstance(x, list) and x and x[0] == 9:
                yield from ((tag, version_event) for tag, version_event in x[1])


def extra(event, code):
    """the value of an event's extra [code, value] (0: conditions, 8: version), or None"""
    for x in event[3:]:
        if isinstance(x, list) and len(x) == 2 and x[0] == code:
            return x[1]
    return None


if __name__ == "__main__":
    main(sys.argv[1:])
