"""Section 9: event choices and outcome shares, counted on the run records (research/data/runs.jsonl, not included;
Independent Training runs from uma-it).

Every count uses every record.

- Outcome shares, and the equal rule on every readable choice (equal_scan(): each SR/SSR card event choice with two or
  more marked outcomes and at least MIN_READINGS readings; GameTora, through gametora.py). An outcome's marker is a
  (skill, level) only that outcome of the choice gives and no other event of the character (her card's events, her
  shared events) gives at all. Runs: the card in the deck, someone else training, and no other deck card whose events
  (or its character's shared events) can give one of the choice's markers, seen or not (a shared credit "A or B event"
  cannot say which card fired; Riko's firings and Helios's link 2 are read the same way). Outcomes that differ only by
  level count when the hint is credited solely to "<her name> event" at exactly that level; outcomes with different
  skills count when her event is credited at a level the outcome can give (its +level or more) and every marked outcome
  could show (each marker skill's level from other sources below 5). A share is among the marked outcomes' readings,
  against the equal rule's 1 / (marked outcomes).
- Sasami Anshinzawa [This Might Sting!] finale (common.classify): her Nothing Ventured at hint level C1_LV or above =
  choice 1 won, below it = choice 2 won; without it, her Risky Business credited to her alone: Lv 2 (or Lv 1 on a list
  without Risky Business) = choice 2 lost, Lv 1 with Risky Business prioritised = choice 1 gave nothing (her date-start
  event's +1 alone); a shared credit or one mixed with other sources is doubtful; nothing, with Risky Business at Lv 5,
  kept apart; runs with Nakayama Festa [Trusting the Dice] left out (joint credits). Daitaku Helios and Nakayama Festa:
  credits to her event on the skills of one link.
Intervals are 90% Wilson. Findings test each conclusion on them: the choice rule (prioritised against not; the maxed
case under each reading its unread runs allow), the measured shares against the equal rule (k of n outcomes, from the
game data) and the model's adopted shares (event_shares.json, manual.json; Anshinzawa's on the planner's premise, both
skills prioritised), Anshinzawa's dates, the scan of every readable choice.
"""

import collections, dataclasses, functools, json, os
from collections.abc import Callable
from dataclasses import dataclass
from typing import Any, NamedTuple
import common
from common import (
    ANSHIN,
    AnshinRun,
    C1_LV,
    FESTA,
    Outcome,
    STEER_CASES,
    anshin_runs,
    card_outcome_events,
    credited,
    deck_ids,
    hints,
    outcome_skills,
    steer_readings,
)
from common import Finding, Hint, Run, Value

# the marker an outcome is read by: (skill name, level), or None when it has none
Sig = tuple[str, int | str] | None
# a 90% Wilson interval
Wil = Callable[[float, float], tuple[float, float]]

SEC: str = "9"
RIKO: int = 30036  # Riko Kashimoto
HELIOS: int = 30116  # Daitaku Helios
Z90: float = 1.645


def wil(k: float, n: float) -> tuple[float, float]:
    """the section's 90% Wilson interval"""
    return common.wilson(k, n, Z90)


# ---------------------------------------------------------------- outcome shares (every split choice)
@dataclass
class SplitChoice:
    """one SR/SSR event choice with two or more outcomes, and its outcomes' credits in the runs (split_choices())"""

    card: str  # the card's name
    cid: int  # its id
    event: str  # the event's title
    link: int | None  # its chain link
    choice: int  # 1-based
    outs: list[Outcome]  # its outcomes
    sigs: list[Sig]  # each outcome's marker
    cnt: list[int]  # per outcome, runs crediting its marker to her event (at its level where outcomes
    # differ only by level)
    cntr: list[
        int
    ]  # per outcome, the same where every marked outcome could show (marked outcomes with distinct skills)
    cnt_any: list[int]  # cnt counting every credit, below the outcome's level included
    cntr_any: list[int]  # cntr counting every credit
    nrun: int  # runs with the card, someone else training
    low: int  # runs crediting a marker skill to her event below every level her events give it (impossible by the
    # game data)
    low_by: dict[str, int]  # ... by marker skill


def could_show(hs: dict[str, Hint], sig: Sig, was_credited: bool) -> bool:
    """an outcome would show: its marker skill's level from other sources is below 5"""
    h = hs.get(sig[0])
    if not h:
        return True
    return h["lv"] - (sig[1] if was_credited and isinstance(sig[1], int) else 0) < 5


def split_choices(R: list[Run]) -> list[SplitChoice]:
    """SR/SSR choices with differing skill outcomes and at least one observable marker, with their credits in the runs"""
    gt = common.gametora()
    db = common.mdb()
    sk = dict(db.execute("select [index], text from text_data where category = 47"))
    title = dict(db.execute("select [index], text from text_data where category = 181"))
    chn = dict(db.execute("select [index], text from text_data where category = 6"))
    cname = dict(db.execute("select [index], text from text_data where category = 75"))
    cchar = dict(db.execute("select id, chara_id from support_card_data"))
    crar = dict(db.execute("select id, rarity from support_card_data"))
    link = dict(db.execute("select story_id, show_progress_1 from single_mode_story_data where show_progress_1 > 0"))
    evs = card_outcome_events(gt, title, cchar)
    shared: dict[int, list[list[Any]]] = collections.defaultdict(list)  # character -> her shared events' choices
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            shared[chara].append([ch[1] for ch in gt.choices(e[1])])

    gives = other_card_skills()
    runs = [(r, set(deck_ids(r)), hints(r)) for r in R]
    out: list[SplitChoice] = []
    for cid, es in evs.items():
        if crar.get(cid, 0) < 2:
            continue  # SR / SSR only
        chara = cchar.get(cid)
        nm = chn.get(chara)
        if not nm:
            continue
        giver: set[tuple[int | str, str]] = set()  # (story id or 'sh' for a shared event, skill)
        for _, sid, chs in es:
            for ch in chs:
                for o in outcome_skills(ch, gt, sk):
                    giver |= {(sid, s) for s in o}
        for chs in shared.get(chara, []):
            for ch in chs:
                for o in outcome_skills(ch, gt, sk):
                    giver |= {("sh", s) for s in o}
        nsrc = collections.Counter(
            s for _, s in giver
        )  # skill -> card story sources plus one collective shared-event source
        mine = [(r, hs) for r, d, hs in runs if cid in d and r["chara"] != chara]
        for t, sid, chs in es:
            for ci, ch in enumerate(chs):
                outs = outcome_skills(ch, gt, sk)
                if len(outs) < 2 or len({tuple(sorted(o.items(), key=str)) for o in outs}) < 2:
                    continue
                sigs: list[Sig] = []
                for o in outs:
                    m = [
                        (k, v)
                        for k, v in o.items()
                        if v != "?" and nsrc[k] == 1 and sum(o2.get(k) == v for o2 in outs) == 1
                    ]
                    sigs.append(m[0] if m else None)
                if not any(sigs):
                    continue
                marks = {m[0] for m in sigs if m}
                read = [(r, hs) for r, hs in mine if not explained_elsewhere(r, cid, marks, gives)]
                floor = {
                    k: min(v for c2 in chs for o in outcome_skills(c2, gt, sk) for k2, v in o.items() if k2 == k)
                    for k in marks
                    if all(isinstance(o.get(k, 0), int) for c2 in chs for o in outcome_skills(c2, gt, sk))
                }
                cc = count_credits(read, outs, sigs, f"{nm} event", floor)
                out.append(
                    SplitChoice(
                        card=cname.get(cid, str(cid)),
                        cid=cid,
                        event=t,
                        link=link.get(sid),
                        choice=ci + 1,
                        outs=outs,
                        sigs=sigs,
                        cnt=[cc.cnt[j] for j in range(len(outs))],
                        cntr=[cc.cntr[j] for j in range(len(outs))],
                        cnt_any=[cc.cnt_any[j] for j in range(len(outs))],
                        cntr_any=[cc.cntr_any[j] for j in range(len(outs))],
                        nrun=len(mine),
                        low=cc.low,
                        low_by=dict(cc.low_by),
                    )
                )
    return out


@functools.cache
def other_card_skills() -> dict[int, set[str]]:
    """card -> every skill an event credited to its character can give: its own events (common.game().pool) and its
    character's shared events (GameTora training_events_shared)"""
    gt = common.gametora()
    db = common.mdb()
    sk = dict(db.execute("select [index], text from text_data where category = 47"))
    cchar = dict(db.execute("select id, chara_id from support_card_data"))
    by_chara: collections.defaultdict[int, set[str]] = collections.defaultdict(set)
    for chara, es in gt.load("training_events_shared"):
        for e in es:
            for ch in gt.choices(e[1]):
                by_chara[chara] |= {s for o in outcome_skills(ch[1], gt, sk) for s in o}
    own = common.game().pool
    return {cid: own.get(cid, set()) | by_chara.get(cchar.get(cid, -1), set()) for cid in set(own) | set(cchar)}


def explained_elsewhere(r: Run, cid: int, marks: set[str], gives: dict[int, set[str]]) -> bool:
    """another deck card's events can give one of the markers, seen or not: the record cannot tell the two sources
    apart (a shared credit "A or B event"), and dropping only the shared credits would lose one outcome's readings and
    not the others' (research/data rule: leave out decks that could explain the skill)"""
    return any(marks & gives.get(x, set()) for x in deck_ids(r) if x != cid)


def at_level(h: Hint, sig: Sig) -> bool:
    """the hint's level is one the outcome can give: at least its +level (capped at 5; other sources only add)"""
    return not isinstance(sig[1], int) or h["lv"] >= min(sig[1], 5)


@dataclass
class Credits:
    """count_credits(): per outcome index, the runs crediting its marker to the event, under two readings of a credit
    below the outcome's level: left out (the section's reading) or counted (every credit)"""

    cnt: collections.Counter[int]  # by exact level where the choice is read by level, else any credit at a level the
    # outcome can give
    cntr: collections.Counter[int]  # the same where every marked outcome could show (marked skills distinct)
    cnt_any: collections.Counter[int]  # cnt counting every credit
    cntr_any: collections.Counter[int]  # cntr counting every credit
    low: int  # credits below every level her events give the skill
    low_by: collections.Counter[str] = dataclasses.field(default_factory=collections.Counter)  # ... by skill


def count_credits(
    mine: list[tuple[Run, dict[str, Hint]]], outs: list[Outcome], sigs: list[Sig], ev: str, floor: dict[str, int]
) -> Credits:
    """per outcome index, the runs crediting its marker to the event `ev` (Credits). One rule per choice: where two
    marked outcomes share a skill every marker is read by its exact level, credited to her alone; otherwise a marker
    whose skill another outcome also gives is read by its level, the rest by any credit at a level the outcome can give
    (at_level()) or, the second reading, any credit at all"""
    c = Credits(*(collections.Counter() for _ in range(4)), low=0)
    marked = [j for j, m in enumerate(sigs) if m]
    distinct = len({sigs[j][0] for j in marked}) == len(marked)
    for _, hs in mine:
        hit: list[int] = []
        hit_any: list[int] = []
        for j in marked:
            m = sigs[j]
            h = hs.get(m[0])
            if not h or ev not in h["src"]:
                continue
            if isinstance(floor.get(m[0]), int) and h["lv"] < floor[m[0]]:
                c.low += 1
                c.low_by[m[0]] += 1
            if not distinct or sum(m[0] in o for o in outs) > 1:  # read by its level
                exact = h["src"] == [ev] and h["lv"] == m[1]
                c.cnt[j] += exact
                c.cnt_any[j] += exact
                hit.append(j)
                hit_any.append(j)
                continue
            hit_any.append(j)
            c.cnt_any[j] += 1
            if at_level(h, m):
                hit.append(j)
                c.cnt[j] += 1
        if distinct:
            for got, hits in ((c.cntr, hit), (c.cntr_any, hit_any)):
                if all(could_show(hs, sigs[j], j in hits) for j in marked):
                    for j in hits:
                        got[j] += 1
    return c


def choice_of(X: list[SplitChoice], cid: int, event: str, choice: int) -> SplitChoice:
    return next(x for x in X if x.cid == cid and x.event.startswith(event) and x.choice == choice)


def equal(X: list[SplitChoice], cid: int, event: str, choice: int, skill: str) -> float:
    """the equal rule's share of the outcomes giving `skill` at any level (k of n outcomes, game data): the share a
    reading of the skill against the choice's other skills tests (Anshinzawa's choice 2, Festa)"""
    x = choice_of(X, cid, event, choice)
    return sum(skill in o for o in x.outs) / len(x.outs)


def marked_counts(x: SplitChoice, every: bool = False) -> list[int]:
    """per outcome, the readings a share counts: cntr where the marked outcomes' marker skills differ, cnt (by level)
    where they share a skill; an unmarked outcome counts 0 (it cannot be seen). every: counting credits below the
    outcome's level too"""
    marks = [m for m in x.sigs if m]
    if len({m[0] for m in marks}) == len(marks):
        return x.cntr_any if every else x.cntr
    return x.cnt_any if every else x.cnt


def equal_marked(x: SplitChoice) -> float:
    """the equal rule's share of one marked outcome among the marked outcomes' readings: 1 / (marked outcomes), each
    marker being one outcome's own"""
    return 1 / sum(1 for m in x.sigs if m)


def adopted(cid: int, skill: str) -> float | None:
    """the model's share for one outcome: a measured split in event_shares.json, else the card's chance in manual.json;
    None when the model leaves it at the equal rule"""
    for e in json.load(open(os.path.join(common.ROOT, "pkg/loopopt/data/event_shares.json")))["cards"].get(
        str(cid), []
    ):
        if skill in e["shares"]:
            return e["shares"][skill]
    return dict(common.gjson("manual.json")["cards"].get(str(cid), {}).get("chance", [])).get(skill)


def find(
    X: list[SplitChoice], cid: int, event: str, choice: int, skill: str, every: bool = False
) -> tuple[int, int] | tuple[None, None]:
    """(count, total over the marked outcomes) for one outcome's marker skill (marked_counts()); its equal share is
    equal_marked()"""
    for x in X:
        if x.cid == cid and x.event.startswith(event) and x.choice == choice:
            for j, m in enumerate(x.sigs):
                if m and m[0] == skill:
                    c = marked_counts(x, every)
                    return c[j], sum(c)
    return None, None


def eq_of(X: list[SplitChoice], cid: int, event: str, choice: int, skill: str) -> float:
    """equal_marked() of the choice find() reads"""
    return equal_marked(choice_of(X, cid, event, choice))


# ---------------------------------------------------------------- the equal rule on every readable choice
MIN_READINGS: int = 30  # readings a choice needs to enter the scan (the threshold of section 9's review scan)


@dataclass
class ScanRow:
    """one readable choice of the scan (equal_scan())"""

    label: str  # card, event, link and choice, as the document's table names it
    marker: str  # the outcome read: its marker skill and level
    k: int  # readings with that outcome
    n: int  # readings over the marked outcomes
    eq: float  # the equal rule's share (equal_marked())
    level_split: bool  # the marked outcomes give one skill at different levels
    distinct: bool  # the marked outcomes' skills all differ (no reading by level)
    larger: bool | None  # a level split: the read outcome is the larger hint (None otherwise)
    low: int  # credits of a marker skill below every level her events give it, left out (SplitChoice.low)
    k_any: int  # k counting every credit
    n_any: int  # n counting every credit
    pair: (
        tuple[int, int] | None
    )  # a skill read at two levels among the marked outcomes: (larger hint's count, smaller's)

    @property
    def off(self) -> bool:
        """the 90% interval excludes the equal share"""
        lo, hi = wil(self.k, self.n)
        return not lo <= self.eq <= hi

    @property
    def off_any(self) -> bool:
        """the same counting every credit"""
        lo, hi = wil(self.k_any, self.n_any)
        return not lo <= self.eq <= hi


def level_pair(x: SplitChoice, c: list[int]) -> tuple[int, int] | None:
    """a skill two marked outcomes give at different levels: (the larger hint's count, the smaller's), else None"""
    by: collections.defaultdict[str, list[tuple[int, int]]] = collections.defaultdict(list)
    for j, m in enumerate(x.sigs):
        if m and isinstance(m[1], int):
            by[m[0]].append((m[1], c[j]))
    for lv in by.values():
        if len(lv) == 2 and lv[0][0] != lv[1][0]:
            hi, lo = sorted(lv, reverse=True)
            return hi[1], lo[1]
    return None


def equal_scan(X: list[SplitChoice]) -> tuple[list[ScanRow], collections.Counter[str]]:
    """every SR/SSR choice with two or more marked outcomes and at least MIN_READINGS readings: one outcome's share
    (the larger hint of a level split, else the first marked outcome) against the equal rule; and the choices left
    out, by reason"""
    out: list[ScanRow] = []
    skipped: collections.Counter[str] = collections.Counter()
    for x in X:
        marked = [j for j, m in enumerate(x.sigs) if m]
        if len(marked) < 2:
            skipped["one marked outcome"] += 1
            continue
        c = marked_counts(x)
        ca = marked_counts(x, every=True)
        if sum(c) < MIN_READINGS:
            skipped[f"under {MIN_READINGS} readings"] += 1
            continue
        split = len({x.sigs[j][0] for j in marked}) == 1
        j = max(marked, key=lambda i: x.sigs[i][1]) if split else marked[0]
        sig = x.sigs[j]
        link = f"link {x.link}" if x.link else "random"
        out.append(
            ScanRow(
                label=f"{x.card} {x.event.replace(chr(92) + 'n', ' ')} ({link}, choice {x.choice})",
                marker=f"{sig[0]} +{sig[1]}",
                k=c[j],
                n=sum(c),
                eq=equal_marked(x),
                level_split=split,
                distinct=len({x.sigs[i][0] for i in marked}) == len(marked),
                larger=(sig[1] > min(x.sigs[i][1] for i in marked)) if split else None,
                low=x.low,
                k_any=ca[j],
                n_any=sum(ca),
                pair=level_pair(x, c),
            )
        )
    return out, skipped


# a count and its total
Count = tuple[int, int]


# a run's priority list as Anshinzawa's finale reads it (anshin_list())
ANSHIN_LISTS: tuple[str, ...] = (
    "both prioritised",
    "Risky Business prioritised, not Nothing Ventured",
    "Nothing Ventured prioritised, not Risky Business",
    "neither prioritised",
)
# the finale's readings (classify), named for the document
ANSHIN_READINGS: dict[str, str] = {
    "c1_win": "choice 1, Nothing Ventured",
    "c1_blank": "choice 1, nothing",
    "c2_win": "choice 2, Nothing Ventured",
    "c2_lose": "choice 2, Risky Business",
    "c2_lose?": "choice 2 or not, unreadable",
    "none": "nothing shown",
    "none_rb5": "nothing shown, Risky Business at Lv 5",
}
TAKEN: dict[str, bool] = {"c1_win": False, "c1_blank": False, "c2_win": True, "c2_lose": True}  # readings: choice 2?


def anshin_list(o: AnshinRun) -> str:
    return ANSHIN_LISTS[0 if o.rb_pri and o.nv_pri else 1 if o.rb_pri else 2 if o.nv_pri else 3]


@dataclass
class AnshinSummary:
    """anshin(): her finale on the runs without Festa (readings by common.classify)"""

    n: int  # runs
    rbp: int  # runs with Risky Business prioritised
    choice2: Count  # Risky Business prioritised: readings taking choice 2, of the readings (TAKEN)
    nv_rbp: int  # Nothing Ventured hinted with Risky Business prioritised
    by_list: dict[str, Count]  # per ANSHIN_LISTS (list shown): runs with her Nothing Ventured, runs
    readings_by_list: dict[str, collections.Counter[str]]  # per ANSHIN_LISTS: runs by the finale's reading (classify)
    c2_outcome: Count  # Risky Business prioritised alone, her events its only source: choice-2 wins, choice-2 readings
    nei: int  # runs with neither skill prioritised (list shown)
    nei_c2: int  # of them, choice 2 won
    shows: int  # Risky Business prioritised and the finale's outcome read (a reading in TAKEN)
    after_bottom: int  # ... showing both the date-start's bottom choice (her +1) and a finale outcome
    doubtful: int  # Risky Business prioritised, her Risky Business not readable ('c2_lose?')
    rb_c2wins: Count  # her extra Risky Business in choice-2 wins, prioritised
    rb_c2wins_nei: Count  # the same, Risky Business not prioritised
    rb_not: Count  # choice 2 won, Risky Business not prioritised
    nvp: collections.Counter[str]  # readings with Nothing Ventured prioritised (not Risky Business)
    nei_c: collections.Counter[str]  # readings with neither prioritised
    c1: tuple[int, int, int]  # Risky Business prioritised, choice-1 readings: all; Nothing Ventured not prioritised and
    # Risky Business at Lv 5 before the finale (the maxed case's readings); Nothing Ventured prioritised too
    maxed: dict[
        str, int
    ]  # Risky Business prioritised alone and at Lv 5 at the end, none of it credited to her (at Lv 5
    # before her date-start event, so before the finale): runs by reading
    maxed_after: dict[
        str, int
    ]  # ... at Lv 5 at the end with her Risky Business credited (below Lv 5 at the date-start)
    nv_other: (
        Count  # runs (no Festa) whose Nothing Ventured is credited to a source other than her events, runs with it
    )
    blank_alone: Count  # Risky Business prioritised alone: runs with her date-start +1 and no finale outcome, runs with
    # her +1 (the premise of reading 'c1_blank': the finale follows the date-start)


def anshin(R: list[Run]) -> AnshinSummary:
    A = [o for o in anshin_runs(R) if not o.festa]
    rbp = [o for o in A if o.rb_pri]
    nei = [o for o in A if o.shown and not o.rb_pri and not o.nv_pri]
    nvp = [o for o in A if o.nv_pri and not o.rb_pri]
    read = [o for o in rbp if o.c in TAKEN]
    her_rb: Callable[[AnshinRun], bool] = lambda o: bool(o.RB and any("Anshinzawa" in s for s in o.RB[1]))
    lists = {ls: [o for o in A if o.shown and anshin_list(o) == ls] for ls in ANSHIN_LISTS}
    alone = lists[ANSHIN_LISTS[1]]
    sole = [o for o in alone if o.RB and o.RB[1] == [common.HER_A] and o.c in ("c2_win", "c2_lose")]
    c1 = [o for o in read if not TAKEN[o.c]]
    at5: Callable[[AnshinRun], bool] = lambda o: bool(o.RB) and o.RB[0] >= 5
    pre5: Callable[[AnshinRun], bool] = lambda o: at5(o) and not her_rb(o)
    return AnshinSummary(
        n=len(A),
        rbp=len(rbp),
        choice2=(sum(TAKEN[o.c] for o in read), len(read)),
        nv_rbp=sum(o.NV is not None for o in rbp),
        by_list={ls: (sum(o.NV is not None for o in xs), len(xs)) for ls, xs in lists.items()},
        readings_by_list={ls: collections.Counter(o.c for o in xs) for ls, xs in lists.items()},
        c2_outcome=(sum(o.c == "c2_win" for o in sole), len(sole)),
        nei=len(nei),
        nei_c2=sum(o.c == "c2_win" for o in nei),
        shows=len(read),
        after_bottom=sum(her_rb(o) and o.c in ("c1_win", "c2_win", "c2_lose") for o in read),
        doubtful=sum(o.c == "c2_lose?" for o in rbp),
        rb_c2wins=(sum(her_rb(o) for o in rbp if o.c == "c2_win"), sum(o.c == "c2_win" for o in rbp)),
        rb_c2wins_nei=(sum(her_rb(o) for o in nei + nvp if o.c == "c2_win"), sum(o.c == "c2_win" for o in nei + nvp)),
        rb_not=(sum(o.c == "c2_win" for o in nei + nvp), len(nei + nvp)),
        nvp=collections.Counter(o.c for o in nvp),
        nei_c=collections.Counter(o.c for o in nei),
        c1=(len(c1), sum(pre5(o) and not o.nv_pri for o in c1), sum(o.nv_pri for o in c1)),
        maxed=dict(collections.Counter(o.c for o in alone if pre5(o))),
        maxed_after=dict(collections.Counter(o.c for o in alone if at5(o) and her_rb(o))),
        nv_other=(
            sum(bool(o.NV) and any("Anshinzawa" not in x for x in o.NV[1]) for o in A),
            sum(bool(o.NV) for o in A),
        ),
        blank_alone=(
            sum(her_rb(o) and o.c in ("c1_blank", "c2_lose?") and o.RB[0] == 1 for o in alone),
            sum(her_rb(o) for o in alone),
        ),
    )


# ---------------------------------------------------------------- Riko's Lone Wolf, Helios's link 2, Festa's "Upping the Ante"
RIKO_SKILLS: tuple[tuple[str, int], ...] = (("Ramp Up", 3), ("Maverick ○", 5), ("Lone Wolf", 3))
RIKO_EV: str = "Riko Kashimoto event"


def riko_hit(hs: dict[str, Hint], skill: str) -> bool:
    """her event credited with one of her "Unexpected Side" skills at a level that outcome can give (at_level(): Maverick
    ○ only at Lv 5, since her one event giving it gives +5)"""
    return credited(hs, skill, RIKO_EV) and at_level(hs[skill], (skill, dict(RIKO_SKILLS)[skill]))


def riko_lone_wolf(R: list[Run], prioritised: bool = True) -> Count:
    """runs showing a priority list with Lone Wolf on it (not Ramp Up), or without Lone Wolf: her bottom choice's Lone Wolf
    alone (riko_hit())"""
    n = k = 0
    for r in R:
        p = r.get("priority")
        if (
            RIKO not in deck_ids(r)
            or p is None
            or ("Lone Wolf" in p) != prioritised
            or (prioritised and "Ramp Up" in p)
        ):
            continue
        hs = hints(r)
        n += 1
        k += riko_hit(hs, "Lone Wolf") and not riko_hit(hs, "Ramp Up") and not riko_hit(hs, "Maverick ○")
    return k, n


# a run's priority list, as the per-run Ramp Up counts split it
RIKO_LISTS: tuple[str, ...] = ("Ramp Up prioritised", "Lone Wolf prioritised", "neither prioritised", "no list shown")


class RikoFirings(NamedTuple):
    ramp_up: int  # top choice, Ramp Up firings (decks where no other card can give any of her three skills)
    maverick: int  # top choice, Maverick firings (the same decks)
    lone_wolf: int  # bottom choice firings (the same decks)
    runs: int  # those decks' runs counted
    per_run: dict[str, Count]  # by the run's list (RIKO_LISTS): (runs with her Ramp Up, runs where it could show)
    per_run_any: dict[str, Count]  # the same counting her Ramp Up credited at any level (below Lv 3 included)
    by_maverick: dict[bool, Count]  # list shown, Lone Wolf not prioritised: by whether another deck card can give
    # Maverick ○, (runs with her Ramp Up, runs)
    low_maverick: tuple[int, int, int]  # runs on decks clean of all three crediting Maverick ○ to her below Lv 5 (no
    # event of hers gives that): (of them also crediting her Ramp Up, those runs, the clean decks' runs)


def riko_list(p: list[str] | None) -> str:
    """the run's priority list as RIKO_LISTS reads it (Lone Wolf first: it moves IT to the bottom choice)"""
    if p is None:
        return "no list shown"
    if "Lone Wolf" in p:
        return "Lone Wolf prioritised"
    return "Ramp Up prioritised" if "Ramp Up" in p else "neither prioritised"


def riko_firings(R: list[Run]) -> RikoFirings:
    """her "Unexpected Side": top choice Ramp Up +3 or Maverick ○ +5, bottom choice Lone Wolf +3 (its middle choice gives
    nothing and cannot be seen). Credits are counted once per skill per run at a level the outcome can give (riko_hit()),
    on decks where no other card's events can give the skill read (explained_elsewhere()) and where it could show (its
    level from other sources below 5): the firings on decks clear of all three, the per-run Ramp Up on decks clear of
    Ramp Up"""
    gives = other_card_skills()
    c = [0, 0, 0]
    n = 0
    per: dict[str, list[int]] = {k: [0, 0] for k in RIKO_LISTS}
    per_any: dict[str, list[int]] = {k: [0, 0] for k in RIKO_LISTS}
    bym: dict[bool, list[int]] = {True: [0, 0], False: [0, 0]}
    low = [0, 0, 0]
    for r in R:
        d = deck_ids(r)
        hs = hints(r)
        if RIKO not in d or r["chara"] == 9006:
            continue
        hit = [j for j, (s, _) in enumerate(RIKO_SKILLS) if riko_hit(hs, s)]
        shows = [not (s in hs and hs[s]["lv"] - (lv if j in hit else 0) >= 5) for j, (s, lv) in enumerate(RIKO_SKILLS)]
        if shows[0] and not explained_elsewhere(r, RIKO, {"Ramp Up"}, gives):
            ls = riko_list(r.get("priority"))
            per[ls][0] += 0 in hit
            per[ls][1] += 1
            per_any[ls][0] += credited(hs, "Ramp Up", RIKO_EV)
            per_any[ls][1] += 1
            if ls in ("Ramp Up prioritised", "neither prioritised"):
                m = explained_elsewhere(r, RIKO, {"Maverick ○"}, gives)
                bym[m][0] += 0 in hit
                bym[m][1] += 1
        if not explained_elsewhere(r, RIKO, {s for s, _ in RIKO_SKILLS}, gives):
            low[2] += 1
            if credited(hs, "Maverick ○", RIKO_EV) and 1 not in hit:
                low[0] += 0 in hit
                low[1] += 1
            if all(shows):
                n += 1
                for j in hit:
                    c[j] += 1
    return RikoFirings(
        *c,
        n,
        {k: (v[0], v[1]) for k, v in per.items()},
        {k: (v[0], v[1]) for k, v in per_any.items()},
        {k: (v[0], v[1]) for k, v in bym.items()},
        (low[0], low[1], low[2]),
    )


def riko_maverick_sources() -> tuple[list[tuple[str, int | str]], bool]:
    """game data: every event of Riko Kashimoto's (her cards' events and her shared events, GameTora through
    gametora.py) whose outcomes give Maverick ○, with the level; and whether master.mdb lists "Unexpected Side" as an
    event of hers (it holds no event rewards, so the levels come from GameTora)"""
    gt = common.gametora()
    db = common.mdb()
    sk = dict(db.execute("select [index], text from text_data where category = 47"))
    title = dict(db.execute("select [index], text from text_data where category = 181"))
    cchar = dict(db.execute("select id, chara_id from support_card_data"))
    chara = cchar[RIKO]
    evs = card_outcome_events(gt, title, cchar)
    found: list[tuple[str, int | str]] = []
    for cid, es in evs.items():
        if cchar.get(cid) != chara:
            continue
        for t, _, chs in es:
            found += [(t, o["Maverick ○"]) for ch in chs for o in outcome_skills(ch, gt, sk) if "Maverick ○" in o]
    for c, es in gt.load("training_events_shared"):
        if c == chara:
            for e in es:
                found += [
                    ("shared", o["Maverick ○"])
                    for ch in gt.choices(e[1])
                    for o in outcome_skills(ch[1], gt, sk)
                    if "Maverick ○" in o
                ]
    sid = next((sid for t, sid, _ in evs[RIKO] if t == "Unexpected Side"), None)
    listed = bool(
        db.execute(
            "select count(*) from single_mode_story_data where story_id = ? and support_chara_id = ?", (sid, chara)
        ).fetchone()[0]
    )
    return sorted(set(found), key=str), listed


def helios(R: list[Run]) -> Count:
    """Tail Held High (link 2) among runs where link 3 visibly fired (her Perfect Spot!), on decks where no other card's
    events can give either (explained_elsewhere())"""
    gives = other_card_skills()
    k = n = 0
    for r in R:
        if HELIOS not in deck_ids(r) or explained_elsewhere(r, HELIOS, {"Perfect Spot!", "Tail Held High"}, gives):
            continue
        hs = hints(r)
        if credited(hs, "Perfect Spot!", "Daitaku Helios event"):
            n += 1
            k += credited(hs, "Tail Held High", "Daitaku Helios event")
    return k, n


def festa(R: list[Run]) -> collections.Counter[str]:
    """'Upping the Ante': her Nothing Ventured against her Risky Business; blank runs with Risky Business at 5 may hide one"""
    t: collections.Counter[str] = collections.Counter()
    for r in R:
        d = deck_ids(r)
        if FESTA not in d or ANSHIN in d or r["chara"] == 1049:
            continue
        hs = hints(r)
        nv, rb = (
            credited(hs, "Nothing Ventured", "Nakayama Festa event"),
            credited(hs, "Risky Business", "Nakayama Festa event"),
        )
        if nv and not rb:
            t["NV"] += 1
        elif rb and not nv:
            t["RB"] += 1
        elif not nv and not rb:
            t["hidden" if "Risky Business" in hs and hs["Risky Business"]["lv"] >= 5 else "none"] += 1
    return t


def festa_numbers() -> dict[str, float]:
    """Festa's share on research runs with her card, excluding her own careers and decks with Anshinzawa.
    Her exclusive credits ("Nakayama Festa event") are wins; a Risky Business credit shared with another card's event
    ("A or B event") is hers with chance r_F / (1 - (1 - r_F)(1 - r_o)), r_F her exclusive Risky Business rate per run,
    r_o the other card's rate in runs without her; blank runs with Risky Business at Lv 5 are split by the firing rate
    where both outcomes can show. {} without the run records."""
    R = common.runs()
    if R is None:
        return {}
    src: Callable[[dict[str, Hint], str], list[str]] = lambda hs, k: hs[k]["src"] if k in hs else []
    t: collections.Counter[str] = collections.Counter()
    shared: collections.Counter[str] = collections.Counter()
    n_f = 0
    for r in R:
        d = deck_ids(r)
        if FESTA not in d or ANSHIN in d or r["chara"] == 1049:
            continue
        hs = hints(r)
        n_f += 1
        nv, rb = (
            "Nakayama Festa event" in src(hs, "Nothing Ventured"),
            "Nakayama Festa event" in src(hs, "Risky Business"),
        )
        co = [
            y.strip()
            for x in src(hs, "Risky Business")
            if x.endswith(" event") and "Nakayama Festa" in x and x != "Nakayama Festa event"
            for y in x[:-6].split(" or ")
            if y.strip() != "Nakayama Festa"
        ]
        if nv and not rb:
            t["NV"] += 1
        elif rb and not nv:
            t["RB"] += 1
        elif nv and rb:
            t["both"] += 1
        elif co:
            t["shared"] += 1
            shared[co[0]] += 1
        elif "Risky Business" in hs and hs["Risky Business"]["lv"] >= 5:
            t["hidden"] += 1
        else:
            t["none"] += 1
    r_f = (t["RB"] + t["both"]) / n_f

    def alone(nm: str) -> float:  # the co-source's own Risky Business rate, runs without Festa
        k = n = 0
        for r in R:
            if FESTA in deck_ids(r) or not any(common.bare(x) == nm for _, _, x in r["deck"]):
                continue
            hs = hints(r)
            n += 1
            k += f"{nm} event" in src(hs, "Risky Business")
        return k / n if n else 0.0

    rates = {nm: alone(nm) for nm in shared}
    # Exclusive-credit rates supply an attribution weight, not an identified conditional firing probability.
    hers = sum(c * r_f / (1 - (1 - r_f) * (1 - rates[nm])) for nm, c in shared.items())
    nv, rb, hid = t["NV"], t["RB"], t["hidden"]
    fire = (nv + rb + hers) / (nv + rb + t["shared"] + t["none"])
    split = nv + rb + hers + hid * fire
    sh = json.load(open(os.path.join(common.ROOT, "pkg/loopopt/data/event_shares.json")))["cards"].get(
        str(FESTA), [{}]
    )[0]
    out: dict[str, float] = {
        "Festa: runs with the card": sum(FESTA in deck_ids(r) for r in R),
        "Festa: NV wins": nv,
        "Festa: RB wins": rb,
        "Festa: RB shared credits": t["shared"],
        "Festa: shared, with Team Sirius": shared.get("Team Sirius", 0),
        "Festa: Team Sirius alone RB rate": rates.get("Team Sirius", float("nan")),
        "Festa: Anshinzawa alone RB rate": rates.get("Sasami Anshinzawa", float("nan")),
        "Festa: her RB rate": r_f,
        "Festa: shared credits hers": hers,
        "Festa: blank, Risky Business at Lv 5": hid,
        "Festa: firing rate where both can show": fire,
        "Festa: readings": split,
        "Festa: share": nv / split,
        "Festa: share, shared none hers": nv / (nv + rb + hid * (nv + rb) / (nv + rb + t["shared"] + t["none"])),
        "Festa: share, shared all hers": nv
        / (nv + rb + t["shared"] + hid * (nv + rb + t["shared"]) / (nv + rb + t["shared"] + t["none"])),
        "Festa: adopted": sh.get("shares", {}).get("Nothing Ventured", float("nan")),
    }
    # Rounded imputed trials omit attribution uncertainty; interval coverage is unestablished.
    out["Festa: share, 90% low"], out["Festa: share, 90% high"] = wil(nv, round(split))
    return out


# ---------------------------------------------------------------- values
def claims() -> list[Value | Finding]:
    R = common.runs()
    if R is None:
        return [finale_rule(split_choices([])), riko_game_finding()]
    a = anshin(R)
    X = split_choices(R)
    got: dict[str, Any] = {}  # value name -> value, in the order they are listed
    got.update(anshin_values(a))
    got.update(marker_share_values(X))
    got.update(riko_values(R))
    got.update(scan_values(X))
    k, n = helios(R)
    got["Helios link 2 Tail Held High (given link 3)"] = k / n
    got["Helios link 2: Tail Held High"], got["Helios link 2: readings"] = k, n
    fe = festa_numbers()
    if fe:
        got.update(fe)
    got["runs showing a priority list"] = sum(r.get("priority") is not None for r in R)
    got["runs"] = len(R)
    note = {
        "Anshinzawa: neither prioritised, choice 2 won": f"Risky Business alone not prioritised: {a.rb_not[0]}/{a.rb_not[1]}"
    }
    out: list[Value | Finding] = [common.Value(SEC, n, v, note=note.get(n, "")) for n, v in got.items()]
    return out + findings(R, a, X, got)


def anshin_values(a: AnshinSummary) -> dict[str, Any]:
    got: dict[str, Any] = {}
    got["Anshinzawa: RB prioritised, took choice 2"] = a.choice2[0] / a.choice2[1]
    got["Anshinzawa: RB prioritised, readings"] = a.choice2[1]
    got["Anshinzawa: neither prioritised, choice 2 won"] = a.nei_c2
    got["Anshinzawa: neither prioritised, runs"] = a.nei
    got["Anshinzawa: runs with the card (no Festa)"] = a.n
    got["Anshinzawa: NV with RB prioritised"] = a.nv_rbp
    got["Anshinzawa: RB prioritised runs"] = a.rbp
    for ls, c in a.readings_by_list.items():
        for reading in ANSHIN_READINGS:
            got[f"Anshinzawa readings, {ls}: {ANSHIN_READINGS[reading]}"] = c.get(reading, 0)
    for ls, (k, n) in a.by_list.items():
        lo, hi = wil(k, n) if n else (None, None)
        got[f"Anshinzawa NV per run, {ls}: runs with NV"] = k
        got[f"Anshinzawa NV per run, {ls}: runs"] = n
        got[f"Anshinzawa NV per run, {ls}: share"] = k / n if n else None
        got[f"Anshinzawa NV per run, {ls}: 90% low"], got[f"Anshinzawa NV per run, {ls}: 90% high"] = lo, hi
    k, n = a.c2_outcome
    lo, hi = wil(k, n)
    got.update(
        {
            "Anshinzawa choice 2: NV wins": k,
            "Anshinzawa choice 2: readings": n,
            "Anshinzawa choice 2: NV share": k / n,
            "Anshinzawa choice 2: 90% low": lo,
            "Anshinzawa choice 2: 90% high": hi,
        }
    )
    got["Anshinzawa: RB prioritised runs showing the finale"] = a.shows
    got["Anshinzawa: RB prioritised runs showing the bottom choice and the finale"] = a.after_bottom
    got["Anshinzawa: RB prioritised runs with her RB unreadable"] = a.doubtful
    (
        got["Anshinzawa: RB prioritised alone, her +1 without a finale outcome"],
        got["Anshinzawa: RB prioritised alone, her +1"],
    ) = a.blank_alone
    got["Anshinzawa: her extra RB in choice-2 wins, prioritised"], got["Anshinzawa: choice-2 wins, prioritised"] = (
        a.rb_c2wins
    )
    (
        got["Anshinzawa: her extra RB in choice-2 wins, RB not prioritised"],
        got["Anshinzawa: choice-2 wins, RB not prioritised"],
    ) = a.rb_c2wins_nei
    (
        got["Anshinzawa: choice-1 readings, RB prioritised"],
        got["Anshinzawa: choice-1 readings with RB at Lv 5"],
        got["Anshinzawa: choice-1 readings with NV prioritised too"],
    ) = a.c1
    got["Anshinzawa maxed before the finale: runs"] = sum(a.maxed.values())
    got["Anshinzawa maxed before the finale: choice-1 readings"] = a.maxed.get("c1_win", 0)
    got["Anshinzawa maxed before the finale: choice-2 wins"] = a.maxed.get("c2_win", 0)
    got["Anshinzawa maxed before the finale: nothing shown"] = a.maxed.get("none_rb5", 0)
    got["Anshinzawa Lv 5 at the end, her RB credited: runs"] = sum(a.maxed_after.values())
    got["Anshinzawa Lv 5 at the end, her RB credited: choice-2 wins"] = a.maxed_after.get("c2_win", 0)
    got["Anshinzawa Lv 5 at the end, her RB credited: choice-1 readings"] = a.maxed_after.get("c1_win", 0)
    got["Anshinzawa Lv 5 at the end, her RB credited: unreadable"] = a.maxed_after.get("c2_lose?", 0)
    w = a.c2_outcome[0] / a.c2_outcome[1]  # choice 2 shows Nothing Ventured on this share of its firings
    nrun = sum(a.maxed.values())
    got["Anshinzawa maxed before the finale: take-rate 90% bound, the finale firing in each"] = (
        (1 - 0.1 ** (1 / nrun)) / w if nrun and not a.maxed.get("c2_win") else None
    )
    return got


def marker_share_values(X: list[SplitChoice]) -> dict[str, Any]:
    """Riko's top choice and two link-3 shares: count, readings, share and its 90% interval"""
    got: dict[str, Any] = {}
    for lab, cid, ev, ch, skill, kn in (
        ("Riko", RIKO, "Unexpected Side", 1, "Ramp Up", "Riko: Ramp Up (top choice taken)"),
        ("Seiun Sky", 30008, "It's All to Win", 1, "Fast-Paced", "Seiun Sky link 3: Fast-Paced"),
        ("Smart Falcon", 30017, "My Umadol Way!", 1, "Center Stage", "Smart Falcon link 3: Center Stage"),
    ):
        k, n = find(X, cid, ev, ch, skill)
        lo, hi = wil(k, n) if n else (None, None)
        if lab == "Riko":
            ka, na = find(X, cid, ev, ch, skill, every=True)
            got["Riko every credit: Ramp Up (top choice taken)"], got["Riko every credit: readings"] = ka, na
            got["Riko every credit: share"] = ka / na
            got["Riko every credit: 90% low"], got["Riko every credit: 90% high"] = wil(ka, na)
        nn = "Riko: readings" if lab == "Riko" else f"{lab} link 3: readings"
        for key, v in zip(
            (kn, nn, f"{lab}: share", f"{lab}: 90% low", f"{lab}: 90% high"), (k, n, k / n if n else None, lo, hi)
        ):
            got[key] = v
    return got


def riko_values(S: list[Run]) -> dict[str, Any]:
    got: dict[str, Any] = {}
    k, n = riko_lone_wolf(S)
    got["Riko: Lone Wolf prioritised, bottom choice"] = k
    got["Riko: Lone Wolf prioritised runs"] = n
    f = riko_firings(S)
    rr, nr = (sum(v[i] for v in f.per_run.values()) for i in (0, 1))
    lo, hi = wil(rr, nr)
    fire = f.ramp_up + f.maverick + f.lone_wolf
    got.update(
        {
            "Riko per run: runs with Ramp Up": rr,
            "Riko per run: runs": nr,
            "Riko per run: share": rr / nr,
            "Riko per run: 90% low": lo,
            "Riko per run: 90% high": hi,
        }
    )
    k, n = f.per_run_any["Ramp Up prioritised"]
    got["Riko per run, Ramp Up prioritised, any level: runs with Ramp Up"] = k
    got["Riko per run, Ramp Up prioritised, any level: share"] = k / n
    (
        got["Riko per run, Ramp Up prioritised, any level: 90% low"],
        got["Riko per run, Ramp Up prioritised, any level: 90% high"],
    ) = wil(k, n)
    for ls, (k, n) in f.per_run.items():
        got[f"Riko per run, {ls}: runs with Ramp Up"] = k
        got[f"Riko per run, {ls}: runs"] = n
        got[f"Riko per run, {ls}: share"] = k / n if n else None
        got[f"Riko per run, {ls}: 90% low"], got[f"Riko per run, {ls}: 90% high"] = wil(k, n) if n else (None, None)
    for m, (k, n) in f.by_maverick.items():
        lab = "another card gives Maverick" if m else "no other card gives Maverick"
        got[f"Riko per run, Lone Wolf not prioritised, {lab}: runs with Ramp Up"] = k
        got[f"Riko per run, Lone Wolf not prioritised, {lab}: runs"] = n
    got.update(
        {
            "Riko firings: clean decks": f.runs,
            "Riko firings: top choice, Ramp Up": f.ramp_up,
            "Riko firings: top choice, Maverick": f.maverick,
            "Riko firings: bottom choice": f.lone_wolf,
            "Riko firings: top choice": f.ramp_up + f.maverick,
            "Riko firings: all": fire,
            "Riko firings: top-choice rate": (f.ramp_up + f.maverick) / fire if fire else None,
            "Riko: Maverick below Lv 5, runs": f.low_maverick[1],
            "Riko: Maverick below Lv 5, runs also crediting Ramp Up": f.low_maverick[0],
            "Riko: decks clean of her three skills, runs": f.low_maverick[2],
        }
    )
    return got


def scan_values(X: list[SplitChoice]) -> dict[str, Any]:
    """the equal-rule scan (equal_scan()): per readable choice its outcome's count, readings, share, 90% interval and
    equal share, each also counting every credit; the scan's totals"""
    rows, skipped = equal_scan(X)
    got: dict[str, Any] = {}
    for x in rows:
        lo, hi = wil(x.k, x.n)
        la, ha = wil(x.k_any, x.n_any)
        got.update(
            {
                f"scan, {x.label}: k": x.k,
                f"scan, {x.label}: n": x.n,
                f"scan, {x.label}: share": x.k / x.n,
                f"scan, {x.label}: 90% low": lo,
                f"scan, {x.label}: 90% high": hi,
                f"scan, {x.label}: equal": x.eq,
                f"scan, {x.label}: credits below her levels": x.low,
                f"scan, {x.label}: every credit, k": x.k_any,
                f"scan, {x.label}: every credit, n": x.n_any,
                f"scan, {x.label}: every credit, share": x.k_any / x.n_any,
                f"scan, {x.label}: every credit, 90% low": la,
                f"scan, {x.label}: every credit, 90% high": ha,
            }
        )
    pairs = [x for x in rows if x.pair]
    dist = [x for x in rows if x.distinct]
    got.update(
        {
            "scan: readings a choice needs": MIN_READINGS,
            "scan: readable choices": len(rows),
            "scan: off the equal rule": sum(x.off for x in rows),
            "scan: within it": sum(not x.off for x in rows),
            "scan: off the equal rule, every credit counted": sum(x.off_any for x in rows),
            "scan: verdicts that depend on the reading": sum(x.off != x.off_any for x in rows),
            "scan: distinct-skill choices": len(dist),
            "scan: distinct-skill choices off the equal rule": sum(x.off for x in dist),
            "scan: distinct-skill choices off the equal rule, every credit counted": sum(x.off_any for x in dist),
            "scan: choices with one marked outcome": skipped["one marked outcome"],
            f"scan: choices under {MIN_READINGS} readings": skipped[f"under {MIN_READINGS} readings"],
            "scan: level splits": len(pairs),
            "scan: level splits leaning to the larger hint": sum(x.pair[0] > x.pair[1] for x in pairs),
            "scan: credits below every level her events give": sum(x.low for x in X),
        }
    )
    return got


def anshin_rb_sources() -> list[tuple[str, int, int | str]]:
    """game data: every (event, choice, level) of Anshinzawa's events (her card's events and her character's shared
    events, GameTora) that gives Risky Business"""
    gt = common.gametora()
    db = common.mdb()
    sk = dict(db.execute("select [index], text from text_data where category = 47"))
    title = dict(db.execute("select [index], text from text_data where category = 181"))
    cchar = dict(db.execute("select id, chara_id from support_card_data"))
    out: list[tuple[str, int, int | str]] = []
    for t, _, chs in card_outcome_events(gt, title, cchar)[ANSHIN]:
        for j, ch in enumerate(chs, 1):
            out += [(t, j, o["Risky Business"]) for o in outcome_skills(ch, gt, sk) if "Risky Business" in o]
    for c, es in gt.load("training_events_shared"):
        if c == cchar[ANSHIN]:
            for e in es:
                for j, ch in enumerate(gt.choices(e[1]), 1):
                    out += [
                        ("shared", j, o["Risky Business"])
                        for o in outcome_skills(ch[1], gt, sk)
                        if "Risky Business" in o
                    ]
    return sorted(set(out), key=str)


def finale_rule(X: list[SplitChoice]) -> Finding:
    """game data: the finale's hint levels identify the choice as classify() reads them"""
    fin = [x.outs for x in X if x.cid == ANSHIN and x.link == 3]
    rbs = anshin_rb_sources()
    nv: Callable[[list[Outcome]], set[int | str]] = lambda ch: {
        o["Nothing Ventured"] for o in ch if "Nothing Ventured" in o
    }
    rb: Callable[[list[Outcome]], bool] = lambda ch: any("Risky Business" in o for o in ch)
    return common.finding(
        SEC,
        "Game data: the hint levels identify the finale's choice (choice 1 Nothing Ventured at the level "
        "read as choice 1 or nothing, choice 2 Nothing Ventured below it or Risky Business), and her events give "
        "Risky Business only twice, +1 each: the date-start event's bottom choice and the finale's choice 2",
        lambda: (
            len(fin) == 2
            and len(rbs) == 2
            and all(lv == 1 for _, _, lv in rbs)
            and {(t.startswith("A Pleasant Practice Partner"), j) for t, j, _ in rbs} == {(True, 3), (False, 2)}
            and min(nv(fin[0])) >= C1_LV > max(nv(fin[1]))
            and any(not o for o in fin[0])
            and not rb(fin[0])
            and rb(fin[1])
        ),
        f"outcomes by choice {fin}; level read as choice 1: {C1_LV} or above; her Risky Business sources {rbs}",
    )


# ---------------------------------------------------------------- findings
def F(st: str, test: Any, ev: Any) -> Finding:
    return common.finding(SEC, st, test, ev)


def iv(k: float, n: float) -> str:
    """k/n with its 90% interval, as the findings print it"""
    return f"{k}/{n} = {k / n:.2f} [{wil(k, n)[0]:.2f}, {wil(k, n)[1]:.2f}]"


def findings(R: list[Run], a: AnshinSummary, X: list[SplitChoice], got: dict[str, Any]) -> list[Value | Finding]:
    """the conclusions of section 9, each on its 90% Wilson intervals (the section's)"""
    out: list[Value | Finding] = []
    L = collections.Counter(len(r["priority"]) for r in R if r.get("priority") is not None)
    out.append(
        F(
            "A priority list has a cap that lists fill: none is longer than the commonest length",
            max(L) == L.most_common(1)[0][0],
            f"cap {max(L)}: " + ", ".join(f"{n} skills: {L[n]}" for n in sorted(L, reverse=True)),
        )
    )
    out.append(finale_rule(X))
    out.append(riko_game_finding())
    out += choice_rule_findings(R, a)
    out += measured_share_findings(R, X, got, a)
    out += anshin_findings(a)
    out += equal_rule_findings(R, X)
    out += festa_findings(X)
    return out


def steer_claims(S: list[Run], a: AnshinSummary) -> list[Value | Finding]:
    """the choice rule's three cases (steer_readings): per case and event, how often IT took the steered choice; the
    maxed case under each reading its unread runs allow; the ranking test on Riko's both-prioritised runs"""
    xs = steer_readings(S)
    out: list[Value | Finding] = []
    sh = {}
    for case in STEER_CASES:
        k = sum(x.taken for x in xs if x.case == case)
        n = sum(1 for x in xs if x.case == case)
        sh[case] = (k, n)
        lo, hi = wil(k, n) if n else (float("nan"), float("nan"))
        out += [
            common.Value(SEC, f"steer {case}: taken", k),
            common.Value(SEC, f"steer {case}: runs", n),
            common.Value(SEC, f"steer {case}: share", k / n if n else None),
            common.Value(SEC, f"steer {case}: 90% low", lo),
            common.Value(SEC, f"steer {case}: 90% high", hi),
        ]
        for event in sorted({x.event for x in xs}):
            ke = sum(x.taken for x in xs if x.case == case and x.event == event)
            ne = sum(1 for x in xs if x.case == case and x.event == event)
            out += [
                common.Value(SEC, f"steer {case}, {event}: taken", ke),
                common.Value(SEC, f"steer {case}, {event}: runs", ne),
            ]
    # Riko, both prioritised: does the ranking decide? Lone Wolf ranked above Maverick ○ / below it (Ramp Up absent)
    rank = {"above": [0, 0], "below": [0, 0]}
    for r in S:
        p = r.get("priority")
        if p is None or RIKO not in deck_ids(r) or "Lone Wolf" not in p or "Maverick ○" not in p or "Ramp Up" in p:
            continue
        hs = hints(r)
        bottom = riko_hit(hs, "Lone Wolf")
        top = riko_hit(hs, "Ramp Up") or riko_hit(hs, "Maverick ○")
        if bottom == top:
            continue
        side = "above" if p.index("Lone Wolf") < p.index("Maverick ○") else "below"
        rank[side][0] += bottom
        rank[side][1] += 1
    for side, (k, n) in rank.items():
        out += [
            common.Value(SEC, f"steer both, Riko, Lone Wolf ranked {side} Maverick: bottom choice", k),
            common.Value(SEC, f"steer both, Riko, Lone Wolf ranked {side} Maverick: runs", n),
        ]
    (k1, n1), (km, nm), (kb, nb) = sh["one-sided"], sh["maxed"], sh["both"]
    (ka, na), (kw, nw) = rank["above"], rank["below"]
    kr, nr = ka + kw, na + nw
    n_finale = sum(1 for x in xs if x.case == "maxed" and x.event.startswith("Anshinzawa"))
    mb = a.maxed
    w = a.c2_outcome[0] / a.c2_outcome[1]
    bound = (1 - 0.1 ** (1 / sum(mb.values()))) / w if mb else float("nan")
    out += [
        common.Value(SEC, "steer both, Riko: bottom choice", kr),
        common.Value(SEC, "steer both, Riko: runs", nr),
        common.Value(SEC, "steer both, Riko: share", kr / nr if nr else None),
    ]
    hi_m = wil(km, nm)[1] if nm else float("nan")
    out += [
        F(
            "Choice rule, one-sided case: with a prioritised skill on one choice only, not at hint Lv 5 from another "
            "source, IT takes that choice (90% lower bound at least 0.95)",
            n1 > 0 and wil(k1, n1)[0] >= 0.95,
            f"{iv(k1, n1)}",
        ),
        F(
            "Choice rule, both-prioritised case: with a prioritised skill on the default choice too, IT takes the "
            "steered choice less often than in the one-sided case (disjoint 90% intervals)",
            nb > 0 and wil(kb, nb)[1] < wil(k1, n1)[0],
            f"both {iv(kb, nb)}; one-sided {iv(k1, n1)}",
        ),
        F(
            "Choice rule, both-prioritised case: no difference between Anshinzawa's finale and Riko Kashimoto's "
            "event is detectable (her steered choice against Riko's bottom choice with Lone Wolf and Maverick ○ both "
            "prioritised: 90% intervals overlap)",
            nb > 0 and nr > 0 and wil(kb, nb)[0] <= wil(kr, nr)[1] and wil(kr, nr)[0] <= wil(kb, nb)[1],
            f"Anshinzawa {iv(kb, nb)}; Riko {iv(kr, nr)}",
        ),
        F(
            "Choice rule, both-prioritised case: no effect of the ranking is detectable (Riko: Lone Wolf ranked above "
            "Maverick ○ against below, 90% intervals overlap; not a test that rank has no effect)",
            na > 0 and nw > 0 and wil(ka, na)[0] <= wil(kw, nw)[1] and wil(kw, nw)[0] <= wil(ka, na)[1],
            f"above {iv(ka, na)}; below {iv(kw, nw)}",
        ),
        F(
            "Choice rule, maxed case, read before the event (the steered skill at hint Lv 5 from other sources and "
            "never credited to the card's event, whose credit would show it below Lv 5 when it fired): every reading is "
            "Anshinzawa's finale, the generic scan's being left out because its maxed readings could only be ones "
            "where IT kept the top choice, and IT took the steered choice in none of them, below the one-sided case "
            "(disjoint 90% intervals)",
            nm > 0 and km == 0 and n_finale == nm and hi_m < wil(k1, n1)[0],
            f"maxed {iv(km, nm)}; one-sided {iv(k1, n1)}",
        ),
        F(
            "No run with Risky Business at Lv 5 before Anshinzawa's finale shows a choice-2 win; with choice 2 showing "
            "Nothing Ventured on most of its firings, that bounds IT's take rate there (90%, if the finale fired in "
            "each run; the bound is in the evidence)",
            sum(mb.values()) > 0 and not mb.get("c2_win"),
            f"{sum(mb.values())} runs: {mb}; choice 2 shows Nothing Ventured on {w:.2f} of its firings; take rate below "
            f"{bound:.3f}",
        ),
        F(
            "Runs ending with Risky Business at Lv 5 whose Risky Business is credited to her were below Lv 5 at the "
            "date-start event and read as one-sided: IT took choice 2 in every one whose choice shows",
            sum(a.maxed_after.values()) > 0 and a.maxed_after.get("c2_win", 0) > 0 and not a.maxed_after.get("c1_win"),
            f"{a.maxed_after}",
        ),
    ]
    return out


def choice_rule_findings(S: list[Run], a: AnshinSummary) -> list[Value | Finding]:
    """the choice rule: a prioritised skill moves IT off the top choice, and nothing else does"""
    k2, n2 = a.choice2
    k0, n0 = a.nei_c2, a.nei
    c1, c1_5, c1_nv = a.c1
    k1, n1 = riko_lone_wolf(S)
    kn, nn = riko_lone_wolf(S, prioritised=False)
    return [
        *steer_claims(S, a),
        F(
            "Choice rule, Anshinzawa's finale: with Risky Business prioritised IT mostly takes choice 2, with neither skill "
            "prioritised it does not (the two 90% intervals are disjoint)",
            wil(k2, n2)[0] > wil(k0, n0)[1],
            f"prioritised {iv(k2, n2)}; neither {iv(k0, n0)}",
        ),
        F(
            "Every reading with Risky Business prioritised that took choice 1 is in the maxed case (Risky Business at "
            "hint Lv 5 before the finale, Nothing Ventured not prioritised) or the both case (Nothing Ventured "
            "prioritised too)",
            c1 > 0 and c1 == c1_5 + c1_nv,
            f"{c1} choice-1 readings: {c1_5} maxed, {c1_nv} with Nothing Ventured prioritised",
        ),
        F(
            "Choice rule, Riko Kashimoto: with Lone Wolf prioritised IT takes the bottom choice in most runs, without it almost "
            "never (disjoint 90% intervals)",
            wil(k1, n1)[0] > wil(kn, nn)[1],
            f"Lone Wolf prioritised {iv(k1, n1)}; priority list without it {iv(kn, nn)}",
        ),
    ]


def riko_game_finding() -> Finding:
    """game data: Riko Kashimoto's one event giving Maverick ○ gives +5"""
    found, listed = riko_maverick_sources()
    return F(
        'Game data: "Unexpected Side" is the only event of Riko Kashimoto\'s that gives Maverick ○, at +5, so a credit '
        "of it to her event below Lv 5 is not that event's outcome (GameTora; master.mdb lists the event as hers and "
        "holds no event rewards)",
        found == [("Unexpected Side", 5)] and listed,
        f"events giving Maverick ○: {found}; listed in master.mdb as hers: {listed}",
    )


def measured_share_findings(
    S: list[Run], X: list[SplitChoice], got: dict[str, Any], a: AnshinSummary
) -> list[Value | Finding]:
    """measured shares: why they leave the equal rule, and whether the adopted share sits in the measured interval"""
    out: list[Value | Finding] = []
    # each: k, n, the choice (card, event, choice, marker skill)
    rows = {
        'Riko "Unexpected Side", Ramp Up': (
            got["Riko: Ramp Up (top choice taken)"],
            got["Riko: readings"],
            (RIKO, "Unexpected Side", 1, "Ramp Up"),
        ),
        "Seiun Sky link 3, Fast-Paced": (
            got["Seiun Sky link 3: Fast-Paced"],
            got["Seiun Sky link 3: readings"],
            (30008, "It's All to Win", 1, "Fast-Paced"),
        ),
        "Smart Falcon link 3, Center Stage": (
            got["Smart Falcon link 3: Center Stage"],
            got["Smart Falcon link 3: readings"],
            (30017, "My Umadol Way!", 1, "Center Stage"),
        ),
    }
    ka, na = a.c2_outcome
    ea = equal(X, ANSHIN, "The Final Race", 2, "Nothing Ventured")
    t = "Seiun Sky link 3, Fast-Paced"
    k, n, c = rows[t]
    out.append(
        F(
            "Anshinzawa's finale and Seiun Sky's link 3 depart from the equal rule: each 90% interval lies above its "
            "equal share (k of n outcomes; Anshinzawa's choice 2 read on runs prioritising Risky Business alone whose "
            "only source of it is her events, so a loss can show)",
            wil(ka, na)[0] > ea and wil(k, n)[0] > eq_of(X, *c),
            f"Anshinzawa choice 2, Nothing Ventured {iv(ka, na)} (equal {ea:g}); {t} {iv(k, n)} (equal {eq_of(X, *c):g})",
        )
    )
    t = "Smart Falcon link 3, Center Stage"
    k, n, c = rows[t]
    out.append(
        F(
            "Smart Falcon's link 3 is consistent with the equal rule: its 90% interval contains the equal share",
            n > 0 and wil(k, n)[0] <= eq_of(X, *c) <= wil(k, n)[1],
            f"{iv(k, n)} (equal {eq_of(X, *c):g})",
        )
    )
    f = riko_firings(S)
    t = 'Riko "Unexpected Side", Ramp Up'
    k, n, c = rows[t]
    kl, nl, nc = f.low_maverick
    out += [
        F(
            "Riko's top choice against the equal rule depends on her credits below an outcome's level: read at each "
            "outcome's own level (Ramp Up from Lv 3, Maverick ○ only at Lv 5) its 90% interval lies above the equal "
            "share; counting every credit it contains it",
            n > 0
            and wil(k, n)[0] > eq_of(X, *c)
            and wil(got["Riko every credit: Ramp Up (top choice taken)"], got["Riko every credit: readings"])[0]
            <= eq_of(X, *c)
            <= wil(got["Riko every credit: Ramp Up (top choice taken)"], got["Riko every credit: readings"])[1],
            f"at its level {iv(k, n)}; every credit "
            f"{iv(got['Riko every credit: Ramp Up (top choice taken)'], got['Riko every credit: readings'])} "
            f"(equal {eq_of(X, *c):g})",
        ),
        F(
            "The records credit Maverick ○ to Riko's event below Lv 5, a level no event GameTora lists for her gives, "
            "on some clean decks; most of those runs also credit her Ramp Up, which her top choice gives in place of "
            "Maverick ○, so these credits come from somewhere else (an event GameTora does not list is the likelier "
            "source)",
            nl > 0 and kl > nl / 2,
            f"{nl} of {nc} runs on decks clean of her three skills; {kl} of them credit Ramp Up too",
        ),
    ]
    for t, (k, n, c) in rows.items():
        if t.startswith("Riko"):
            continue
        v = adopted(c[0], c[3])
        if v is None:  # left at the equal rule, which the equal-rule findings above test
            continue
        out.append(common.Value(SEC, f"adopted: {t}", v))
        out.append(
            F(
                f"Adopted {v:g} for {t} lies inside its measured 90% interval (a consistency check: the value was set "
                "from an earlier subset of these runs)",
                wil(k, n)[0] <= v <= wil(k, n)[1],
                iv(k, n),
            )
        )
    v = adopted(ANSHIN, "Nothing Ventured")
    out.append(common.Value(SEC, "adopted: Anshinzawa finale, Nothing Ventured", v))
    # Lists holding both skills against the both case's choice rate: the finale shows on a share of runs measured on
    # lists with Risky Business alone (where IT takes choice 2 every time), choice 2 gives Nothing Ventured at its read
    # share, choice 1 on one of its two outcomes (the equal rule, assumed)
    k2, n2 = a.c2_outcome
    ka, na = a.by_list[ANSHIN_LISTS[1]]
    kt, nt = common.steer_shares()["both"]
    shows = (ka / na) / (k2 / n2)
    expected = shows * ((kt / nt) * (k2 / n2) + (1 - kt / nt) * 0.5)
    kb, nb = a.by_list[ANSHIN_LISTS[0]]
    both_lists = [
        r["priority"]
        for r in S
        if r.get("priority")
        and {"Nothing Ventured", "Risky Business"} <= set(r["priority"])
        and ANSHIN in common.deck_ids(r)
        and common.FESTA not in common.deck_ids(r)
    ]
    out += [
        common.Value(
            SEC,
            "Anshinzawa, both prioritised: runs ranking Nothing Ventured above Risky Business",
            sum(p.index("Nothing Ventured") < p.index("Risky Business") for p in both_lists),
            note=f"of {len(both_lists)}",
        ),
        common.Value(SEC, "Anshinzawa, both prioritised: finale shown, from Risky Business alone", shows),
        common.Value(SEC, "Anshinzawa, both prioritised: expected from the both case", expected),
        common.Value(
            SEC,
            "Anshinzawa, both prioritised: a target's landing chance against the adopted share",
            (1 + kb / nb) / (1 + v),
            note="her card the only gold source, the white certain: landing is proportional to 1 + the gold chance",
        ),
    ]
    one = [a.by_list[ls] for ls in ANSHIN_LISTS[1:3]]
    out.append(
        F(
            f"Anshinzawa's adopted {v:g} lies inside the 90% interval on lists holding exactly one of the two skills, "
            "Risky Business or Nothing Ventured",
            all(n > 0 and wil(k, n)[0] <= v <= wil(k, n)[1] for k, n in one),
            "; ".join(iv(k, n) for k, n in one),
        )
    )
    out.append(
        F(
            "On lists holding both, the share is consistent with IT taking choice 2 only at the both case's rate: the "
            "share that rate implies (the finale shown as on lists with Risky Business alone, choice 1 giving Nothing "
            "Ventured on one of its two outcomes) lies inside the share's 90% interval (a consistency check, not a test "
            "of the cause)",
            nb > 0 and wil(kb, nb)[0] <= expected <= wil(kb, nb)[1],
            f"expected {expected:.3f}; observed {iv(kb, nb)}",
        )
    )
    per = "; ".join(f"{ls} {iv(*kn)}" for ls, kn in a.by_list.items() if kn[1])
    kb, nb = a.by_list[ANSHIN_LISTS[0]]
    out.append(
        F(
            f"Known miss: Anshinzawa's adopted {v:g} (the per-run chance of her Nothing Ventured) lies above the 90% "
            "interval on the planner's premise, lists prioritising both skills (Nothing Ventured as a target, Risky "
            "Business as her deck entry's instruction)",
            nb > 0 and wil(kb, nb)[1] < v,
            per,
        )
    )
    v = adopted(RIKO, "Ramp Up")
    out.append(common.Value(SEC, 'adopted: Riko "Unexpected Side", Ramp Up', v))
    k, n = (sum(x[i] for x in f.per_run.values()) for i in (0, 1))
    kp, np_ = f.per_run["Ramp Up prioritised"]
    ka, na = f.per_run_any["Ramp Up prioritised"]
    per = "; ".join(f"{ls} {iv(*kn)}" for ls, kn in f.per_run.items() if kn[1])
    out += [
        F(
            f"Riko's adopted {v:g}, the per-run chance of her Ramp Up hint on runs whose shown list prioritises Ramp Up "
            "and not Lone Wolf (a target prioritised, as the model assumes), rests on how a credit below Lv 3 is read: "
            f"counting every credit, as when it was set, {v:g} lies inside the 90% interval; counting only credits at "
            "a level her event gives (Lv 3 or more), it lies just above it",
            np_ > 0 and wil(ka, na)[0] <= v <= wil(ka, na)[1] and wil(kp, np_)[1] < v,
            f"every credit {iv(ka, na)}; Lv 3 or more {iv(kp, np_)}",
        ),
        F(
            f"Pooled over every list, Riko's per-run chance of her Ramp Up hint is below the adopted {v:g} (a "
            "prioritised Lone Wolf takes her bottom choice): its 90% interval lies below it",
            n > 0 and wil(k, n)[1] < v,
            f"pooled {iv(k, n)}; {per}",
        ),
    ]
    return out


def anshin_findings(a: AnshinSummary) -> list[Value | Finding]:
    """Anshinzawa's date-start event and the finale"""
    kp, np_ = a.rb_c2wins
    kq, nq = a.rb_c2wins_nei
    kz, nz = a.blank_alone
    ko, no = a.nv_other
    return [
        F(
            "On the runs her finale is read from (no Nakayama Festa), Nothing Ventured is credited only to her events, "
            "so its level is her finale's alone",
            no > 0 and ko == 0,
            f"{ko} of {no} runs with Nothing Ventured credit another source",
        ),
        F(
            'With Risky Business prioritised IT also takes the bottom choice of "A Pleasant Practice Partner?": her '
            "extra Risky Business +1 in choice-2 wins with the priority, against without it (disjoint 90% intervals)",
            wil(kp, np_)[0] > wil(kq, nq)[1],
            f"with {iv(kp, np_)}; without {iv(kq, nq)}",
        ),
        F(
            "With Risky Business prioritised alone, her date-start +1 never shows without a finale outcome, so a lone "
            "+1 is read as the finale's choice 1 giving nothing (the both case's 'c1_blank')",
            nz > 0 and kz == 0,
            f"{kz} of {nz} runs with her +1",
        ),
        F(
            "The date-start event's bottom choice does not stop her finale: with Risky Business prioritised, runs show "
            "its +1 together with a finale outcome (whether her later dates still fire is not tested)",
            a.after_bottom > 0,
            f"{a.after_bottom} of {a.rbp} runs; the finale's outcome read in {a.shows}, unreadable in {a.doubtful}, "
            f"not seen in {a.rbp - a.shows - a.doubtful}",
        ),
    ]


# the scan's choices off the equal rule, as the document's table lists them, and within it; the choices whose verdict
# depends on the reading of a credit below its outcome's level
SCAN_OFF: tuple[str, ...] = (
    "[Foolproof Plan] Seiun Sky It's All to Win (link 3, choice 1)",
    "[Fire at My Heels] Kitasan Black We Walk Together (link 3, choice 1)",
    "[Fiery Discipline] Yaeno Muteki Yaeno Muteki's Love Song Nightmare (link 3, choice 1)",
    "[That Time I Became the Strongest] Gold Ship Assembling the Uma Lord's Minions (link 3, choice 1)",
    "[Touching Sleeves Is Good Luck! ♪] Matikanefukukitaru Finding Miraculous Joy! (link 3, choice 1)",
    "[Trusting the Dice] Nakayama Festa Upping the Ante (link 3, choice 1)",
    "[Q≠0] Agnes Tachyon Researching the Limits of Running: Rapport (link 1, choice 2)",
    "[Q≠0] Agnes Tachyon Researching the Limits of Running: Velocity (link 3, choice 1)",
    "[Sunlit Angel ♪] Daitaku Helios Hype Heaven, Part Two! (link 3, choice 1)",
    "[Danke Schön] Eishin Flash One Should Share Friends' Hometown Love (link 3, choice 1)",
    "[Tracen Reception] Tazuna Hayakawa A Surprise Breather (link 5, choice 1)",
    "[Trainers' Teamwork] Aoi Kiryuin Chasing Dreams (link 5, choice 1)",
    "[Planned Perfection] Riko Kashimoto Unexpected Side (random, choice 1)",
    "[Planned Perfection] Riko Kashimoto A Peek at Her Heart (link 5, choice 1)",
    "[From the Ground Up] Light Hello At Rainbow Cove (link 5, choice 1)",
)
SCAN_FLIPS: tuple[str, ...] = (
    "[Dazzling Day in the Snow] Marvelous Sunday Pure White Marvelousness ☆ (link 3, choice 1)",
    "[Planned Perfection] Riko Kashimoto Unexpected Side (random, choice 1)",
)
SCAN_ON: tuple[str, ...] = (
    "[Fairest Fleur] Grass Wonder Hidden Petals (link 3, choice 1)",
    "[My Umadol Way! ☆] Smart Falcon My Umadol Way! ☆ (link 3, choice 1)",
    "[Tonight, We Waltz] King Halo Glittering Star of the Ball (link 3, choice 1)",
    "[Dazzling Day in the Snow] Marvelous Sunday Pure White Marvelousness ☆ (link 3, choice 1)",
    "[My Way] Tosen Jordan If You Got It, Own It (link 3, choice 1)",
)


def unlisted_stories(cids: set[int]) -> tuple[dict[str, list[str]], dict[str, float]]:
    """per card, the stories master.mdb (single_mode_story_data) lists for its character that GameTora's decoded events
    (her cards' events and her shared events) do not include, by title; and per such title, the share of support
    characters master.mdb lists it for. master.mdb holds no event rewards"""
    gt = common.gametora()
    db = common.mdb()
    title = dict(db.execute("select [index], text from text_data where category = 181"))
    cchar = dict(db.execute("select id, chara_id from support_card_data"))
    cname = dict(db.execute("select [index], text from text_data where category = 75"))
    known: collections.defaultdict[int, set[int]] = collections.defaultdict(set)
    for cid, es in card_outcome_events(gt, title, cchar).items():
        known[cchar.get(cid, -1)] |= {sid for _, sid, _ in es}
    for chara, es in gt.load("training_events_shared"):
        known[chara] |= {int(e[2]) for e in es}
    rows = db.execute(
        "select story_id, support_chara_id from single_mode_story_data where support_chara_id > 0"
    ).fetchall()
    chars = {c for _, c in rows}
    by_title: collections.defaultdict[str, set[int]] = collections.defaultdict(set)
    for sid, c in rows:
        by_title[title.get(sid, str(sid))].add(c)
    out: dict[str, list[str]] = {}
    for cid in sorted(cids):
        ch = cchar.get(cid)
        ids = {sid for sid, c in rows if c == ch}
        out[cname.get(cid, str(cid))] = sorted({title.get(sid, str(sid)) for sid in ids - known[ch]})
    share = {t: len(by_title[t]) / len(chars) for ts in out.values() for t in ts}
    return out, share


def equal_rule_findings(S: list[Run], X: list[SplitChoice]) -> list[Value | Finding]:
    """the scan of every readable choice against the equal rule, under both readings of a credit below its outcome's
    level; Helios's link 2 given link 3"""
    from scipy.stats import binom

    rows, _ = equal_scan(X)
    off = [x for x in rows if x.off]
    dist = [x for x in rows if x.distinct]
    doff = [x for x in dist if x.off]
    pairs = [x for x in rows if x.pair]
    lean = [x for x in pairs if x.pair[0] > x.pair[1]]
    flips = sorted(x.label for x in rows if x.off != x.off_any)
    kh, nh = helios(S)
    eh = equal(X, HELIOS, "Helios's Hype Heaven Week", 1, "Tail Held High")
    low = {x.cid for x in X if x.low}
    un, common_share = unlisted_stories(low)
    lowc = [x for x in X if x.low]
    p_all = binom.sf(len(off) - 1, len(rows), 0.1)
    p_dist = binom.sf(len(doff) - 1, len(dist), 0.1)
    return [
        F(
            "The document's equal-rule table lists every choice the scan reads, each with the verdict the scan gives "
            "under each reading of a credit below its outcome's level",
            {x.label for x in off} == set(SCAN_OFF)
            and {x.label for x in rows if not x.off} == set(SCAN_ON)
            and flips == sorted(SCAN_FLIPS),
            f"off: {sorted(x.label for x in off)}; within: {sorted(x.label for x in rows if not x.off)}; verdict "
            f"depends on the reading: {flips}",
        ),
        F(
            "Known misses: on most readable choices the 90% interval excludes the equal share, far more than the one in "
            "ten a 90% interval misses by chance; on the choices read without levels alone (each marked outcome its own "
            "skill), still far more than chance",
            len(off) > len(rows) / 2 and p_all < 0.05 and p_dist < 0.05,
            f"all {len(off)} of {len(rows)} (P by chance {p_all:.1e}); without levels {len(doff)} of {len(dist)} (P "
            f"{p_dist:.1e}); "
            + "; ".join(
                f"{x.label} {x.marker} {iv(x.k, x.n)}, every credit {iv(x.k_any, x.n_any)} (equal {x.eq:g})"
                for x in rows
            ),
        ),
        F(
            "Most level splits read (one skill at two hint levels among the marked outcomes) lean to the larger hint",
            len(lean) > len(pairs) / 2,
            f"{len(lean)} of {len(pairs)}: "
            + "; ".join(f"{x.label} larger {x.pair[0]}, smaller {x.pair[1]}" for x in pairs),
        ),
        F(
            "Credits below every level her events give fall mostly on one marker skill of a choice (more than half of "
            "them in every such choice, all of them in all but one), so they are not spread like noise in the records; "
            "another source of that skill credited to her event (one GameTora does not list) would give this pattern",
            bool(lowc)
            and all(max(x.low_by.values()) > sum(x.low_by.values()) / 2 for x in lowc)
            and sum(len(x.low_by) > 1 for x in lowc) <= 1,
            "; ".join(f"{x.card} {x.event[:30]} choice {x.choice}: {x.low_by}" for x in lowc),
        ),
        F(
            "The game data cannot name that source: master.mdb holds no event rewards, and the only stories it lists "
            "for these characters that GameTora does not decode are ones it lists for most support characters",
            all(v > 0.5 for v in common_share.values()),
            "; ".join(f"{k}: {v or 'none'}" for k, v in un.items())
            + "; share of support characters: "
            + ", ".join(f"{t} {v:.2f}" for t, v in common_share.items()),
        ),
        F(
            "Daitaku Helios's link 2, where link 3 visibly fired, is consistent with the equal rule: its 90% interval "
            "contains the equal share of Tail Held High",
            nh > 0 and wil(kh, nh)[0] <= eh <= wil(kh, nh)[1],
            f"{iv(kh, nh)} (equal {eh:g})",
        ),
    ]


def festa_findings(X: list[SplitChoice]) -> list[Value | Finding]:
    """Festa's Nothing Ventured share against the equal rule and the adopted share"""
    fe = festa_numbers()
    if not fe:
        return []
    e, v = equal(X, FESTA, "Upping the Ante", 1, "Nothing Ventured"), fe["Festa: adopted"]
    return [
        F(
            f"Nakayama Festa's Nothing Ventured share is above {e:g} and the adopted {v:g} fits: with shared credits attributed "
            f"and blank runs split by the firing rate, {e:g} (the equal rule) lies below the 90% interval and {v:g} inside it",
            fe["Festa: share, 90% low"] > e
            and fe["Festa: share, 90% low"] <= fe["Festa: adopted"] <= fe["Festa: share, 90% high"],
            f"{fe['Festa: share']:.3f} [{fe['Festa: share, 90% low']:.3f}, {fe['Festa: share, 90% high']:.3f}]; shared credits hers "
            f"{fe['Festa: shared credits hers']:.1f} of {fe['Festa: RB shared credits']}; bounds {fe['Festa: share, shared all hers']:.3f}-"
            f"{fe['Festa: share, shared none hers']:.3f}",
        )
    ]


def detail() -> None:
    S = common.runs()
    a = anshin(S)
    print(
        "Anshinzawa finale by priority (no Festa): neither shown-prioritised",
        a.nei,
        dict(a.nei_c),
        "| Nothing Ventured prioritised (not Risky Business)",
        sum(a.nvp.values()),
        dict(a.nvp),
        "| maxed case's runs by reading",
        a.maxed,
    )
    rows, skipped = equal_scan(split_choices(S))
    print(f"equal-rule scan: {len(rows)} readable choices; left out {dict(skipped)}")
    for x in rows:
        print(f"  {'off' if x.off else 'on '} {x.label}: {x.marker} {iv(x.k, x.n)} (equal {x.eq:g})")


if __name__ == "__main__":
    common.main(claims)
    if common.have_runs():
        print()
        detail()
