"""The page's lists for one era, from the era's exports: the card picker's cards, the roster's umas and outfits, the skill
catalog and the career page's tables, with release dates from master.mdb. usage: export_page.py global|jp

Writes pkg/loopopt/data/<era>/page.json, {cards, umas, skills, career, inputs}, where `inputs` maps each file read
(relative to the repository) to its sha256: the page's build checks them (webapp/vite/pagedata.mjs), so a page is never
built from lists older than the exports they come from. The career tables keep every mob; the build keeps the ones with
a portrait. Each G1's distance and ground are checked here (`check_race_tracks`). Also writes
pkg/loopopt/data/page_raceids.json, one map for both eras: the race id of each race name and of its schedule-chart name.

Run by ./build.sh data, events, gametora and pb, which write its inputs."""

import argparse, hashlib, json, os, sqlite3
from typing import NamedTuple

# era.Era is not used: it opens master.mdb, which this export can do without (release dates from upcoming.json)
R = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
DATA = os.path.join(R, "pkg", "loopopt", "data")

# schedule rows name their race; the page maps names to race ids for the banners (same 34 G1s in both eras), keyed by
# both the full name and the chart's short name (pkg/loopopt/src/lib.rs chart_race_name: aliases, else 22 characters)
CHART_ALIAS = {
    "Tokyo Yushun (Japanese Derby)": "Japanese Derby",
    "Asahi Hai Futurity Stakes": "Asahi Hai FS",
    "Mile Championship Nambu Hai": "MCS Nambu Hai",
    "Hanshin Juvenile Fillies": "Hanshin JF",
    "Zen-Nippon Junior Yushun": "Zen-Nippon Jr Yushun",
}

inputs = {}  # each file read, relative to the repository -> the sha256 of its bytes (main() fills it)


def read_bytes(path):
    """the file's bytes, its hash recorded in `inputs`"""
    raw = open(path, "rb").read()
    inputs[os.path.relpath(path, R)] = hashlib.sha256(raw).hexdigest()
    return raw


def read_json(path):
    return json.loads(read_bytes(path))


class SkillReference(NamedTuple):
    """Skill group and rarity (1 white, 2 gold) for one localized skill name."""

    group: int
    rarity: int


def release_dates(mdb, out):
    """outfit and support card id -> release time (unix s), for the roster's and card picker's default order and the
    page's hiding of upcoming content. Without master.mdb: the dated outfits and cards of the tracked upcoming.json
    (Global only: the JP data has nothing upcoming); the page takes an undated id's own id, i.e. id order, released."""
    release_times = {}
    if os.path.exists(mdb):
        read_bytes(mdb)
        con = sqlite3.connect(f"file:{mdb}?mode=ro", uri=True)
        # an outfit's release = the start of the first gacha it appeared in; else its uma's start date
        release_times = dict(
            con.execute("select c.id, c2.start_date from card_data c join chara_data c2 on c2.id = c.chara_id")
        )
        release_times.update(
            con.execute(
                "select a.card_id, min(d.start_date) from gacha_available a join gacha_data d on d.id = a.gacha_id group by a.card_id"
            )
        )
        release_times.update(con.execute("select id, start_date from support_card_data"))  # cards carry their own date
        con.close()
    elif os.path.exists(os.path.join(out, "upcoming.json")):
        upcoming = read_json(os.path.join(out, "upcoming.json"))
        release_times = {item_id: start for kind in ("outfits", "cards") for item_id, start in upcoming[kind]}
    return release_times


def skill_references(skills):
    """Every name an event reward can carry (the ○ beside a catalog ◎ included): web_skills.json's `names`, with its
    group and rarity."""
    return {name: SkillReference(group=group_id, rarity=rarity) for name, (group_id, rarity) in skills["names"].items()}


def event_summary(skills_by_name, events):
    """card id -> the skills its events give, each [group, rarity, link (0: not a chain link), chance]"""
    event_skills = {}
    for card_id, card in events["cards"].items():
        out = []
        for key in ("chain", "end"):
            for link_number, names in card.get(key, []):
                for reward in names:
                    skill_name, chance = (reward[0], reward[1]) if isinstance(reward, list) else (reward, 1.0)
                    if skill_name in skills_by_name:
                        out.append(
                            [skills_by_name[skill_name].group, skills_by_name[skill_name].rarity, link_number, chance]
                        )
        for skill_name in card.get("always", []):
            if skill_name in skills_by_name:
                out.append([skills_by_name[skill_name].group, skills_by_name[skill_name].rarity, 0, 1.0])
        for skill_name, chance in card.get("chance", []):
            if skill_name in skills_by_name:
                out.append([skills_by_name[skill_name].group, skills_by_name[skill_name].rarity, 0, chance])
        if out:
            event_skills[int(card_id)] = out
    return event_skills


def chain_ends(skills_by_name, card_events):
    """The card's chain-ending choices, one per link that has one: the skills on the ending choice ("end") and on the
    event's other choices ("go"), each [group, rarity]. IT takes the ending choice only for a prioritised skill on it,
    so the page names it."""

    def references(rewards):
        names = [reward[0] if isinstance(reward, list) else reward for reward in rewards]
        return [list(skills_by_name[name]) for name in names if name in skills_by_name]

    continuing = dict(card_events.get("chain", []))  # link -> the skills on its other choices
    return [
        {"link": link_number, "end": references(rewards), "go": references(continuing.get(link_number, []))}
        for link_number, rewards in card_events.get("end", [])
    ]


# Phase 1: the era's exports, and GameTora's page names for the links (support cards: url_name, e.g.
# 30067-the-thrones-assemblage; characters: the slug of /characters/<outfit id>-<slug>)
def check_race_tracks(career, era):
    """Each G1's distance (0-3, sprint to long) and ground (0 turf, 1 dirt), one of each per race: the career page reads
    them as the solver's `Distance` and `Surface` (webapp/src/model/data.ts `tracksOf`, which fails the page's data load on any
    other), so one out of range, or lists of the wrong length, stops the export here, naming it."""
    races, distances, grounds = career["races"], career.get("race_cat"), career.get("race_ground")
    where = f"pkg/loopopt/data/{era}/data.json"
    if not isinstance(distances, list) or not isinstance(grounds, list):
        raise SystemExit(f"{where}: race_cat and race_ground missing")
    if not len(distances) == len(grounds) == len(races):
        raise SystemExit(f"{where}: {len(races)} races, {len(distances)} distances, {len(grounds)} grounds")
    code_in = lambda value, codes: type(value) is int and value in codes  # (a bool or a float is no code)
    for name, distance, ground in zip(races, distances, grounds, strict=True):
        if not (code_in(distance, range(4)) and code_in(ground, range(2))):
            raise SystemExit(f"{where}: {name}'s distance {distance!r} (0-3) or ground {ground!r} (0-1) is not one")


def main():
    parser = argparse.ArgumentParser(
        description="The page's lists for one era -> the era's page.json, and page_raceids.json.", allow_abbrev=False
    )
    parser.add_argument("era", choices=("global", "jp"), help="global, or jp: the Grand Masters era")
    parser.add_argument(
        "--fallback", action="store_true", help="without master.mdb, replace dates a page.json read from it"
    )
    arguments = parser.parse_args()
    era = arguments.era
    out = os.path.join(DATA, era)
    mdb = os.path.join(R, "game", era, "master.mdb")
    data = read_json(os.path.join(out, "data.json"))
    skills = read_json(os.path.join(out, "web_skills.json"))
    cards = read_json(os.path.join(out, "web_cards.json"))
    events = read_json(os.path.join(out, "card_events.json"))
    career_tables = read_json(os.path.join(out, "career.json"))
    gametora_slugs = read_json(os.path.join(DATA, "gametora_slugs.json"))
    gametora_cards = {int(k): v for k, v in gametora_slugs["cards"].items()}
    gametora_characters = {int(k): v for k, v in gametora_slugs["chars"].items()}
    release_times = release_dates(mdb, out)
    skills_by_name = skill_references(skills)
    event_skills = event_summary(skills_by_name, events)

    # Phase 2: the card picker's cards: pals, group cards, R cards (no chain events: hints only, no chain slot) and SR/SSR
    # cards with chains
    page_cards = []
    for card in cards["cards"]:
        if not card["pal"] and card["links"] == 0 and card["rarity"] != 1:
            continue
        if not card["pal"] and card["links"] > 0 and card["rarity"] < 2:
            continue
        card_events = events["cards"].get(str(card["id"]), {})
        page_cards.append(
            {
                "id": card["id"],
                "name": card["name"],
                "rar": card["rarity"],
                "pal": card["pal"],
                "kind": card.get("kind", ""),
                "rel": release_times.get(card["id"], card["id"]),
                "gt": gametora_cards.get(card["id"]),
                "whites": card["whites"],
                "ev": event_skills.get(card["id"], []),
                # skills to prioritise for IT to take the event choice the model credits
                **({"pri": pri} if (pri := card_events.get("prioritise")) else {}),
                **({"end": ends} if (ends := chain_ends(skills_by_name, card_events)) else {}),
                # scenarios that bar it from the deck
                **(
                    {"ban": ban}
                    if (ban := [scenario for scenario, ids in cards.get("restrict", {}).items() if card["id"] in ids])
                    else {}
                ),
            }
        )

    # every gold a card gives (an event's skill at rarity 2, a chain end's on either choice) has a gold name in the skill
    # list: the page names it by that (state/cards.ts `skillName`, a must()), so a gap fails here and not in the card list
    gold_names = {group_id: gold for group_id, _, gold in skills["skills"]}
    for page_card in page_cards:
        golds = [group_id for group_id, rarity, *_ in page_card["ev"] if rarity == 2] + [
            group_id for end in page_card.get("end", []) for group_id, rarity in end["end"] + end["go"] if rarity == 2
        ]
        for group_id in golds:
            if group_id in gold_names and not gold_names[group_id]:
                raise SystemExit(f"card {page_card['id']} gives skill group {group_id} at gold, which has no gold name")

    # Phase 3: the roster's umas and their outfits
    page_umas = [
        {
            "id": uma["id"],
            "name": uma["name"],
            "gt": gametora_characters.get(uma["id"]),
            **({"apt": uma["apt"]} if uma.get("apt") else {}),
            "outfits": [
                {"id": card[0], "label": card[1], "rel": release_times.get(card[0], card[0])}
                for card in skills["cards"].get(str(uma["id"]), [])
            ],
        }
        for uma in data["umas"]
    ]

    # Phase 4: the career page's veteran import: spark factor group -> skill group, win saddle -> race index, race names by
    # index, and the turns each G1 is held at (0-71: year x 24 + half-month; Classic and Senior runnings share one race index)
    race_turns = [
        sorted(
            {
                turn_index
                for uma in data["umas"]
                for turn_index, choices in enumerate(uma["turns"])
                for choice in choices
                if choice["race"] == race_index
            }
        )
        for race_index in range(len(data["race_names"]))
    ]
    # skill groups something besides inheritance can give: a card's hints or events, an outfit's own skills (the career
    # planner's sample veterans carry only the targets outside this: the ones a loop must inherit)
    sourced = sorted(
        {group_id for card in cards["cards"] for group_id in card["whites"]}
        | {event[0] for card_events in event_skills.values() for event in card_events}
        | {group_id for outfits in skills["cards"].values() for outfit in outfits for group_id, _ in outfit[2]}
    )
    career = {
        "factors": career_tables["factors"],
        "saddles": career_tables["saddles"],
        "pinks": career_tables["pinks"],
        "blues": career_tables.get("blues", {}),
        "uniques": career_tables.get("uniques", {}),
        "races": data["race_names"],
        "race_cat": data["race_cat"],  # each race's distance (0-3 sprint to long) and ground (0 turf, 1 dirt)
        "race_ground": data["race_ground"],
        "race_turns": race_turns,
        "mobs": career_tables.get("mobs", []),  # every mob; the page's build keeps the ones with a portrait
        "sourced": sourced,
        "ranks": career_tables.get("ranks", []),
    }
    check_race_tracks(career, era)
    if era == "jp":  # GM era: the spark factors Global has, the only ones uma.moe (a Global site) can list
        global_factors = read_json(os.path.join(DATA, "global", "career.json"))["factors"]
        career["moe"] = sorted(map(int, global_factors))

    # Phase 5: the files
    # A checkout without game/ would replace release dates read from master.mdb with id order, a tracked change a commit
    # could carry: refuse unless asked.
    old = os.path.join(out, "page.json")
    if not os.path.exists(mdb) and not arguments.fallback and os.path.exists(old):
        made_from = json.load(open(old))["inputs"]
        if any(path.startswith("game" + os.sep) for path in made_from):
            raise SystemExit(f"{old} was made from master.mdb, which is missing here: --fallback to replace its dates")
    json.dump(
        {
            "cards": page_cards,
            "umas": page_umas,
            "skills": skills["skills"],
            "career": career,
            "inputs": dict(sorted(inputs.items())),
        },
        open(os.path.join(out, "page.json"), "w"),
        ensure_ascii=False,
        separators=(",", ":"),
    )
    race_ids = {}
    for era_dir in ("global", "jp"):
        era_data = json.load(open(os.path.join(DATA, era_dir, "data.json")))
        for race_name, race_id in zip(era_data["race_names"], era_data["races"]):
            race_ids[race_name] = race_id
            race_ids[CHART_ALIAS.get(race_name, race_name[:22])] = race_id
    json.dump(race_ids, open(os.path.join(DATA, "page_raceids.json"), "w"), ensure_ascii=False, separators=(",", ":"))
    print(
        f"{era} page.json: {len(page_cards)} cards, {len(page_umas)} umas, release dates from",
        "master.mdb" if os.path.exists(mdb) else "upcoming.json and id order",
    )


if __name__ == "__main__":
    main()
