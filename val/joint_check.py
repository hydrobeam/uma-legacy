"""Independent check of the solver's exact landing odds (`joint_fast` in src/deck/joint/mod.rs).

For every deck result of a validation run, recompute P(all targets land) from the job, the card data and the
reported decks and loops, jointly over the chain process (Binomial(CHAIN_N, p) events with logit p = CHAIN_A + CHAIN_B x
the deck's pals, group cards not counted, or the job's chain_events; each event advancing a running chain card, all
equally likely, or weighted by links left under job chain_rule "slots"; a card-event stop risk ending its chain on
CHAIN_STOP_SHARE of its firings, its own k/n under "slots"; the constants read from src/deck/chains.rs), over each event's outcomes and over one card's training draws
(P(no hint for a set A of its targets) by forward simulation of the draws, each uniform over the skills below Lv 5, a
skill leaving after ceil(5 / hint levels per draw) draws; floor/ceil(h) draws, h = base(scenario) x (1 + hf/100) x
(1 + d(scenario) x specialty/100), the two constants read from research/results.json as the solver's build reads
them), and compare with the
reported p_all. Written separately from the Rust code: final chain states by plain forward iteration, and the joint
expectation by inclusion-exclusion over per-target "no gold" / "no hint" requirements.

usage: uv run val/joint_check.py TAG        (val/TAG__*.out and val/TAGpu__*.out)"""

import functools
import itertools
import json
import math
import os
import re
import sys
from collections.abc import Callable
from dataclasses import dataclass
from typing import NamedTuple


class SkillReference(NamedTuple):
    """A named skill's group and hint tier (1 white, 2 gold)."""

    group: int
    tier: int


def chain_constants():
    """CHAIN_N, CHAIN_A, CHAIN_B and CHAIN_STOP_SHARE (as k, n) from the solver's source, read as its build reads them"""
    src = open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "pkg/loopopt/src/deck/chains.rs")).read()
    number = lambda name, kind: float(re.search(rf"const {name}: {kind} = (-?[\d._]+);", src).group(1).replace("_", ""))
    share = re.search(r"const CHAIN_STOP_SHARE: \(u\d+, u\d+\) = \((\d+), (\d+)\);", src)
    return (
        int(number("CHAIN_N", "u32")),
        number("CHAIN_A", "f64"),
        number("CHAIN_B", "f64"),
        (int(share[1]), int(share[2])),
    )


CHAIN_N, CHAIN_A, CHAIN_B, CHAIN_STOP_SHARE = chain_constants()


class StopRisk(NamedTuple):
    link: int
    stopped: int
    observed: int


class TargetSkill(NamedTuple):
    """An uncertain target's index and hint tier (1 white, 2 gold)."""

    index: int
    tier: int


@dataclass(frozen=True)
class TrainingHints:
    targets: list[int]
    draws: float  # Expected training draws, mixed over the adjacent integers.
    skill_count: int
    levels_per_draw: int


class ChainKind(NamedTuple):
    """A chain's original length, selected stopping link, and observed stop risks."""

    links: int
    stop: int
    risks: tuple[StopRisk, ...]


@dataclass(frozen=True)
class CardSources:
    training: TrainingHints
    fixed: list  # Independent outcome groups, available without advancing a chain.
    chain: dict  # Link number -> independent outcome groups unlocked at that link.
    kind: ChainKind | None


def _hint_constants():
    """hint_base and hint_spec as the solver's build.rs takes them: research/results.json's section 7 fit, rounded to 3
    decimals; Grand Masters (no runs) takes one base and one d fitted over every scenario, an unknown scenario reads as
    Unity Cup (constants, not code: the check stays independent)"""
    path = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "research", "results.json")
    values = json.load(open(path, encoding="utf-8"))["values"]
    fitted_names = {"gc": "grand_concert", "ura": "ura_finale", "trackblazer": "trackblazer"}

    def constant(part):
        """a scenario's base or specialty weight d (`part`: "base" or "d")"""
        return lambda s: round(
            values[
                f"7.one_base_and_one_d_{part}"
                if s == "grandmasters"
                else f"7.specialty_{part}_{fitted_names.get(s, 'unity_cup')}"
            ],
            3,
        )

    return {"hint_base": constant("base"), "hint_spec": constant("d")}


HINT = _hint_constants()
V = os.path.dirname(os.path.abspath(__file__))
S = os.path.dirname(V)
GW, GG = 0.2 * 1.771561, 0.4 * 1.771561


def draws_miss(k, n, t, cap):
    """P(none of k given skills drawn in t draws over n, each uniform over the skills drawn fewer than cap times), by
    recursion over the other skills' draw counts (sorted tuple; written independently of the solver's histogram DP)"""

    @functools.lru_cache(None)
    def go(state, t):
        if t == 0:
            return 1.0
        available = [i for i, c in enumerate(state) if c < cap]
        if not available and not k:
            return 1.0
        tot = 0.0
        for i in available:
            c = list(state)
            c[i] += 1
            tot += go(tuple(sorted(c)), t - 1)
        return tot / (len(available) + k)

    return go(tuple([0] * (n - k)), t)


def hint_miss(k, n, h, hinc):
    cap = -(-5 // max(1, hinc))
    t0 = math.floor(h)
    f = h - t0
    return (1 - f) * draws_miss(k, n, t0, cap) + (f * draws_miss(k, n, t0 + 1, cap) if f else 0.0)


def chain_distribution(kinds, event_probability, slots=False):
    n = len(kinds)
    res = {}
    alts = []
    for _links, stop, risk in kinds:
        v = []
        go = 1.0
        for l, k, nn in sorted(risk):
            q = k / nn
            v.append((l, go * q))
            go *= 1 - q
        v.append((stop, go))
        alts.append(v)
    for combo in itertools.product(*alts):
        w = math.prod(p for _, p in combo)
        stops = [s for s, _ in combo]
        if w == 0:
            continue
        cur = {tuple([0] * n): 1.0}
        e = 0
        while True:
            pe = event_probability(e)
            nxt = {}
            for s, p in cur.items():
                ws = [((kinds[i].links - s[i]) if slots else 1) if s[i] < stops[i] else 0 for i in range(n)]
                if not sum(ws):
                    res[s] = res.get(s, 0) + w * p * (1 - sum(event_probability(x) for x in range(e)))
                    continue
                res[s] = res.get(s, 0) + w * p * pe
                for i in range(n):
                    if ws[i]:
                        t = list(s)
                        t[i] += 1
                        t = tuple(t)
                        nxt[t] = nxt.get(t, 0) + p * ws[i] / sum(ws)
            if not nxt:
                break
            cur = nxt
            e += 1
    return res


class DeckModel(NamedTuple):
    """A job's deck model, restated (`deck_model`)."""

    deck_miss_probability: Callable  # a reported deck -> (req -> P(no outcome breaks req)), req per uncertain target
    lineage: Callable  # (miss probability, outfit levels, q, own-event chances) -> P(all) at the loop's spark scale
    skills_db: dict
    uncertain_targets: list  # the uncertain targets' skill groups, in job order
    fixed_target_count: int


def deck_model(job, d):
    """The solver's deck model for `job` (data folder `d`): per reported deck, P(no outcome of the deck's sources breaks a
    requirement), jointly over the chain process; and a loop lineage's P(all) from it (`lineage`)"""
    read_data = lambda f: json.load(open(os.path.join(d, f)))
    skill_targets = {}
    for g, w, gd in read_data(job["skills_file"])["skills"]:
        skill_targets[w] = SkillReference(group=g, tier=1)
        if gd:
            skill_targets[gd] = SkillReference(group=g, tier=2)
    cards = {c["id"]: c for c in read_data(job["cards_file"])["cards"]}
    card_events = read_data(job.get("events_file", "card_events.json"))["cards"]
    # the solver's model, restated: chain events ~ Binomial(n, p), p by the deck's pals unless the job fixes (n, p);
    # training draws h = base(scenario) x (1 + hf/100) x (1 + d(scenario) x specialty/100) over the card's skill options
    slots = job.get("chain_rule", "card") == "slots"
    base, spec_d = HINT["hint_base"](job.get("scenario", "unity")), HINT["hint_spec"](job.get("scenario", "unity"))

    def event_law(pals):
        """the deck's chain-event pmf"""
        count, chance = job.get("chain_events") or (CHAIN_N, 1 / (1 + math.exp(-(CHAIN_A + CHAIN_B * pals))))
        return lambda e: math.comb(count, e) * chance**e * (1 - chance) ** (count - e) if 0 <= e <= count else 0.0

    uncertain_targets = [t["group"] for t in job["targets"] if t.get("guaranteed", 0) != 2]
    fixed_target_count = len(job["targets"]) - len(uncertain_targets)
    target_count = len(uncertain_targets)
    nm = lambda x: (x, 1.0) if isinstance(x, str) else (x[0], x[1])
    target_skill = lambda n: (
        TargetSkill(index=uncertain_targets.index(skill_targets[n].group), tier=skill_targets[n].tier)
        if n in skill_targets and skill_targets[n].group in uncertain_targets
        else None
    )

    def choice_groups(cid):
        """per alts link: the choices with targets, as outcome groups [(p, [(k, lvl)])], dominated ones dropped"""
        res = {}
        aw = dict(card_events.get(str(cid), {}).get("alt_w", []))  # measured outcome shares, else equal
        for lk, chs in card_events.get(str(cid), {}).get("alts", []):
            gs = []
            for ci, ch in enumerate(chs):
                wt = aw[lk][ci] if lk in aw else [1 / len(ch)] * len(ch)
                g = [(wt[oi], sorted({x for x in map(target_skill, o) if x})) for oi, o in enumerate(ch)]
                gs.append([x for x in g if x[1]])

            def prof(g):
                p = [[0.0, 0.0] for _ in range(target_count)]
                for w, o in g:
                    for k in {k for k, _ in o}:
                        p[k][1] += w
                        if any(kk == k and l == 2 for kk, l in o):
                            p[k][0] += w
                return p

            P = [prof(g) for g in gs]
            ge = lambda a, b: all(x[0] >= y[0] and x[1] >= y[1] for x, y in zip(a, b))
            keep = [
                g
                for i, g in enumerate(gs)
                if g
                and not any(
                    j != i and gs[j] and ge(P[j], P[i]) and (P[j] != P[i] or (gs[j] == g and j < i))
                    for j in range(len(gs))
                )
            ]
            res[lk] = keep
        return res

    def card_sources(cid, end, label):
        c = cards[cid]
        e = card_events.get(str(cid), {})
        tr = [uncertain_targets.index(g) for g in c["whites"] if g in uncertain_targets] if c["nskill"] else []
        fixed = []
        for n in e.get("always", []):
            if target_skill(n):
                fixed.append([(1.0, [target_skill(n)])])
        for n, p in e.get("chance", []):
            if target_skill(n):
                fixed.append([(p, [target_skill(n)])])
        for c_, g, lvl, p in job.get("odds_events", []):  # inline per-run events of the job
            if c_ == cid and g in uncertain_targets:
                fixed.append([(p, [(uncertain_targets.index(g), lvl)])])
        chain = {}
        kind = None
        if not c["pal"] and c["links"] > 0:
            stop = end or c["links"]
            alts = choice_groups(cid)
            picks = dict(x.split(": ", 1) for x in label.split("; ")) if label else {}
            for lk, names in e.get("chain", []):
                if (end and lk >= stop) or lk in alts:
                    continue
                for n, p in map(nm, names):
                    if target_skill(n):
                        chain.setdefault(lk, []).append([(p, [target_skill(n)])])
            for lk, keep in alts.items():
                if (end and lk >= stop) or not keep:
                    continue
                if len(keep) == 1:
                    g = keep[0]
                else:
                    want = set(picks[f"link {lk}"].split(", "))
                    g = next(g for g in keep if {n for _, o in g for k, l in o for n in [skill_name(k, l)]} == want)
                chain.setdefault(lk, []).append(g)
            if end:
                for lk, names in e.get("end", []):
                    if lk != end:
                        continue
                    for n, p in map(nm, names):
                        if target_skill(n):
                            chain.setdefault(lk, []).append([(p, [target_skill(n)])])
            kind = ChainKind(
                links=c["links"],
                stop=stop,
                risks=tuple(
                    StopRisk(r[0], *(r[1:3] if slots else CHAIN_STOP_SHARE))
                    for r in e.get("stop_risk", [])
                    if r[0] < stop and r[0] <= 4
                ),
            )
        return CardSources(
            training=TrainingHints(
                targets=tr,
                draws=base * (1 + c["hf"] / 100) * (1 + spec_d * c.get("spec", 0) / 100),
                skill_count=c["nskill"],
                levels_per_draw=c.get("hinc", 1),
            ),
            fixed=fixed,
            chain=chain,
            kind=kind,
        )

    skills_db = read_data(job["skills_file"])
    names = {g: (w, gd) for g, w, gd in skills_db["skills"]}
    skill_name = lambda k, l: (
        names[uncertain_targets[k]][1] if l == 2 and names[uncertain_targets[k]][1] else names[uncertain_targets[k]][0]
    )

    def groups_satisfy(groups, req):
        """P(no outcome of these independent groups breaks req); req[k]: 0 any, 1 no gold, 2 no hint"""
        pr = 1.0
        for g in groups:
            bad = sum(p for p, o in g if any(req[k] == 2 or (req[k] == 1 and l == 2) for k, l in o))
            pr *= 1 - bad
        return pr

    def deck_miss_probability(dk, deck_size=6):
        ch = dk.get("choices") or [""] * len(dk["card_ids"])
        sources = [card_sources(cid, e, lb) for cid, e, lb in zip(dk["card_ids"], dk["end_chain_at"], ch)]
        filler = deck_size - len(dk["card_ids"]) - len(dk.get("plain_ids", []))
        chs = [s for s in sources if s.kind]
        pals = sum(1 for cid in dk["card_ids"] if cards[cid]["pal"] and cards[cid]["kind"] != "group")
        dist = chain_distribution(
            [s.kind for s in chs] + [ChainKind(links=2, stop=2, risks=())] * filler,
            event_probability=event_law(pals + len(dk.get("plain_ids", []))),
            slots=slots,
        )
        memo = {}

        def miss_probability(req):
            if req in memo:
                return memo[req]
            pr = 1.0
            for s in sources:
                n = sum(1 for k in s.training.targets if req[k] == 2)
                if n:
                    pr *= hint_miss(k=n, n=s.training.skill_count, h=s.training.draws, hinc=s.training.levels_per_draw)
                pr *= groups_satisfy(s.fixed, req)
            tot = 0.0
            for state, p in dist.items():
                x = p
                for i, s in enumerate(chs):
                    for lk, groups in s.chain.items():
                        if state[i] >= lk:
                            x *= groups_satisfy(groups, req)
                tot += x
            memo[req] = pr * tot
            return memo[req]

        return miss_probability

    def lineage(miss_probability, lv, q, ew):
        coef = [
            (GG, 0, 0) if v == 2 else (GG, GG - GW, 0) if v == 1 else (GG, GG - GW, GW * (1 - ew[k]) * (1 - q))
            for k, v in enumerate(lv)
        ]
        tot = 0.0
        for req in itertools.product(range(3), repeat=target_count):
            c = 1.0
            for k, r in enumerate(req):
                c *= coef[k][0] if r == 0 else -coef[k][r]
                if c == 0:
                    break
            if c:
                tot += c * miss_probability(req)
        return tot * GG**fixed_target_count

    return DeckModel(
        deck_miss_probability=deck_miss_probability,
        lineage=lineage,
        skills_db=skills_db,
        uncertain_targets=uncertain_targets,
        fixed_target_count=fixed_target_count,
    )


def check(out, job, d, alt_q=None):
    """max |recomputed / reported p_all - 1| over the result's loops; with `alt_q` (top -> list of per-lineage q
    vectors) also, per loop, the mean P(all) at each of those q vectors: returns worst alone, or (worst, [[P per vector] per loop]) with alt_q"""
    deck_miss_probability, lineage, skills_db, uncertain_targets, _ = deck_model(job, d)
    worst = 0.0
    alt = []
    for top in out.get("top", []):
        loop = top["loop"]
        decks = top.get("decks") or [top["deck"]] * 4
        pm = [deck_miss_probability(dk) for dk in decks]

        # each lineage at the trainee's best outfit, as the solver does: per outfit the targets it carries (and the job's
        # `extra` / white guarantees), and the trainee events that hint a target's white (`trainee_hints`)
        def options(t, loop=loop):
            uid = loop["members"][t]["id"]
            outs = [o for o in skills_db["cards"].get(str(uid), []) if o[0] not in job.get("exclude_outfits", [])] or [
                [0, "any", []]
            ]
            res = []
            for o in outs:
                lv = []
                for g in uncertain_targets:
                    l = max(
                        [x[1] for x in o[2] if x[0] == g]
                        + [x[2] for x in job.get("extra", []) if x[0] == uid and x[1] == g]
                        + [0]
                    )
                    if job["targets"][[t_["group"] for t_ in job["targets"]].index(g)].get("guaranteed", 0) == 1:
                        l = max(l, 1)
                    lv.append(l)
                ids = {uid, o[0]}
                ew = [
                    1 - math.prod(1 - h[2] for h in job.get("trainee_hints", []) if h[0] in ids and h[1] == g)
                    for g in uncertain_targets
                ]
                res.append((lv, ew))
            return res

        # the mean over the structure's lineages: 12 in a four-uma loop, 3 in a three-uma rotation
        lins = loop["detail"]["lineages"]
        mine = sum(max(lineage(pm[L["t"]], lv, L["q"], ew) for lv, ew in options(L["t"])) for L in lins) / len(lins)
        worst = max(worst, abs(mine / loop["p_all"] - 1))
        if alt_q is not None:
            alt.append(
                [
                    sum(max(lineage(pm[L["t"]], lv, q, ew) for lv, ew in options(L["t"])) for L, q in zip(lins, qs))
                    / len(lins)
                    for qs in alt_q(top)
                ]
            )
    return worst if alt_q is None else (worst, alt)


if __name__ == "__main__":
    tag = sys.argv[1]
    bad = 0
    for era, j in json.load(open(os.path.join(V, "jobs.json"))):
        job = json.load(open(os.path.join(V, "jobs", j + ".json")))
        if job.get("mode") != "deck":
            continue
        name = era + "__" + j
        for m in ("", "pu"):
            f = os.path.join(V, f"{tag}{m}__{name}.out")
            if not os.path.exists(f):
                continue
            w = check(json.load(open(f)), job, os.path.join(S, "pkg", "loopopt", "data", era))
            flag = w > 1e-3
            bad += flag
            print(f"{m or 'sh'} {name}: max |python / solver - 1| = {w:.1e}{'  <<' if flag else ''}", flush=True)
    print("all within 1e-3" if not bad else f"{bad} outside 1e-3")
