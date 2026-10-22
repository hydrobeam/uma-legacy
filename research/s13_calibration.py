"""Section 13 (and the inheritance test of section 3, the certainty checks of section 9): the career model against real
careers. Each run record is rebuilt as a career-planner job (trainee in her outfit, scenario, the six cards locked at
their own limit breaks, both parents with their recorded parents and every ancestor's white and pink sparks, race wins
laid out so every pair the model reads shares the record's shared-G1 count), run through the native solver, and
compared with what the run shows, per (run, target) row, pooled. Intervals (95%): observed / predicted over rows by the
run-clustered ratio estimator (common.cluster_ci; a run's targets share its deck and lineage), the observed rate's
interval that ratio interval times the mean prediction; where every run gives one row, Wilson on the observed rate
(divided by the mean prediction for the ratio). Joint tests over buckets: the run-clustered Wald chi2
(common.cluster_chi2).

  uv run --group research research/s13_calibration.py [--wide]

Steps (outputs under research/data/out/calib/, reused while the jobs and the solver are unchanged):
  card variants   every card at limit breaks 0-3: hint frequency and increment read at that limit break's level cap
                  from master.mdb (effect values linear between milestone levels, floored; unique effect from its level)
  jobs            default eight targets (Unity Cup's two golds guaranteed there); --wide: every other skill a deck card
                  or an ancestor's spark could give, in jobs of at most 24 targets, on a seeded 30% sample of the runs
  solver          one process, 100 jobs per call (--wide: one job per call, 2 s limit each)
  analysis        the tests of RESEARCH.md section 13 and the conclusions drawn on them (Findings); the full tables go
                  to report8.txt / reportW.txt

Dropped runs: a deck that is not six known cards, an R card in the deck, a trainee outfit not in web_skills.json, the
same parent twice, shared-G1 counts that do not fit the race list."""

import collections, hashlib, json, math, os, random, subprocess, sys, time
from collections.abc import Callable, Iterable, Sequence
from dataclasses import dataclass
from itertools import pairwise
from typing import Any, NamedTuple, TypedDict
import common
from common import Value, Finding, Run, finding, wilson, DEFAULT, UNITY_GOLDS

OUTD: str = os.path.join(common.OUT, "calib")
GD: str = common.GD
SCENARIO_KEY: dict[int, str] = {1: "ura", 2: "unity", 3: "gc", 4: "trackblazer"}  # the solver's scenario names
# (character, skill group, 1 white / 2 gold): Yamanin Zephyr's Slipstream, Tokai Teio's Head-On (as the planner page
# sends them)
EXTRA: list[list[int]] = [[1078, 20165, 1], [1003, 20190, 1]]
PINK: set[int] = {11, 12, 21, 22, 23, 24, 31, 32, 33, 34}  # pink spark factor ids // 100 (track, distance, style)
WIDE_JOB_TARGETS: int = 24  # the most targets one --wide job carries
SOLVER_BATCH: int = 100  # jobs per solver call (default targets)
# master.mdb support effect types
EFFECT_HINT_LEVELS, EFFECT_HINT_FREQUENCY, EFFECT_SPECIALTY = 17, 18, 19
MILESTONES: list[int] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]  # the levels of support_card_effect_table's columns
# the generic spark chance of a learned white / gold at six carriers (docs/MODEL.md, spark generation: 20% / 40%,
# x1.1 per carrier); a row rescales it by 1.1 ** (carriers - 6)
SPARK_WHITE_6, SPARK_GOLD_6 = 0.2 * 1.1**6, 0.4 * 1.1**6
S3, S9, S13 = "3", "9", "13"  # the sections these values and findings belong to


class CalRow(TypedDict):
    """one (run, target) row of the analysis (rows_of); s10_random_events and s11_conditional read these keys"""

    run: int  # run id
    scen: int  # scenario (common.SCEN)
    g: int  # the target's skill group
    scen_gold: bool  # the scenario guarantees the target's gold (Unity Cup's two)
    outfit: int  # her outfit gives the skill: 0 no, 1 its white, 2 its gold
    zero: bool  # nothing can give the target: not sent to the solver, every prediction 0
    dg: float  # predicted P(a deck card hints the gold)
    dw: float  # predicted P(a deck card hints the white)
    q: float  # predicted P(an inspiration of the target in years 2 and 3)
    nc: int  # carriers: legacies holding the target's spark
    p: float  # the solver's P(the target lands: acquired and sparked)
    acq: float  # predicted P(acquired, gold or white); 1 for a scenario gold
    gold: float  # predicted P(the gold acquired); 1 for a scenario gold
    plegs: list[float]  # per legacy carrying the spark: its inspiration chance per roll
    o_any: bool  # observed: hinted from any source, or owned
    o_gold: bool  # observed: a gold hint
    o_deck: bool  # observed: a hint credited to a deck card
    o_deck_gold: bool  # observed: a gold hint credited to a deck card
    o_insp: bool  # observed: an inspiration of the target fired in year 2 or 3
    o_insp_years: int  # observed: years (of 2 and 3) with an inspiration of the target
    o_insp_hits: int  # observed: inspiration hits of the target over both years
    o_insp_src: bool  # observed: a hint credited to Inspiration
    o_events: bool  # observed: a hint credited to Events (no deck card's events could give it)
    o_other: bool  # observed: a hint credited to a source neither Inspiration, Events nor a deck card
    prio: bool | None  # the target is on the run's priority list; None when the run shows no list
    pool_lb: int | None  # the lowest limit break among deck cards whose training pool holds it; None: no card
    cards_pool: list[int]  # deck cards whose training pool holds the target
    cards_ev: list[int]  # deck cards whose events can hint the target
    p_pred: float  # predicted landing: p, or the gold's generic spark chance for a scenario gold
    l_obs: float  # observed-equivalent landing: the spark chance of what was hinted (gold, white or none)


class RunRows(TypedDict):
    """one run's rows (rows_of); s10_random_events and s11_conditional read these keys"""

    id: int
    scen: int
    p_all: float  # the solver's P(every target lands); 0 when a target is unreachable
    rows: list[CalRow]
    wide: bool  # from a --wide job (targets beyond the default eight)


class ManEntry(NamedTuple):
    """a manifest entry (build_jobs): a run and its targets nothing can give (written to manifest.json as a list)"""

    run_id: int
    unreachable: list[int]


SolverOut = dict[str, dict[str, Any]]  # a solver output per job file id
RowFn = Callable[[CalRow], Any]


# ---------------------------------------------------------------- card variants per limit break
def variant(cid: int, lb: int | None) -> int:
    """the card id of a card at limit break lb (lb 4 or none: the card itself)"""
    return cid if lb is None or lb >= 4 else (lb + 1) * 1_000_000 + cid


class HintEffects(NamedTuple):
    """a card's training-hint effects at one level: web_cards.json's hf, hinc and spec"""

    frequency: int  # hint frequency (effect 18)
    levels: int  # hint levels per draw (1 + effect 17)
    specialty: int  # specialty priority (effect 19)


def card_variants() -> list[int]:
    """write cards_lb.json and events_lb.json (every card also at limit breaks 0-3) under OUTD; returns the cards whose
    limit-break-4 effects read from master.mdb differ from web_cards.json"""
    db = common.mdb()

    def query(sql: str, *args: Any) -> list[Any]:
        return db.execute(sql, args).fetchall()

    level_cap: dict[int, list[int]] = {
        r: lv for r, *lv in query("select rarity, limit_0, limit_1, limit_2, limit_3, limit_4 from support_card_limit")
    }

    def effect(effect_table: int, typ: int, level: int) -> float:
        """the effect's value at level, linear between milestone levels (-1: no value at that milestone)"""
        row = query(
            "select init,limit_lv5,limit_lv10,limit_lv15,limit_lv20,limit_lv25,limit_lv30,limit_lv35,limit_lv40,"
            "limit_lv45,limit_lv50 from support_card_effect_table where id=? and type=?",
            effect_table,
            typ,
        )
        if not row:
            return 0.0
        pts = [(lv, v) for lv, v in zip(MILESTONES, row[0]) if v != -1]
        if not pts or level < pts[0][0]:
            return 0.0
        for (l0, v0), (l1, v1) in pairwise(pts):
            if l0 <= level <= l1:
                return v0 + (v1 - v0) * (level - l0) / (l1 - l0)
        return float(pts[-1][1])

    def unique(sid: int, typ: int, level: int) -> int:
        return sum(
            v
            for lv, t0, v0, t1, v1 in query(
                "select lv, type_0, value_0, type_1, value_1 from support_card_unique_effect where id=?", sid
            )
            for t, v in ((t0, v0), (t1, v1))
            if t == typ and level >= lv
        )

    def at_lb(sid: int, lb: int) -> HintEffects:
        effect_table, rarity = query("select effect_table_id, rarity from support_card_data where id=?", sid)[0]
        lv = level_cap[rarity][lb]
        return HintEffects(
            frequency=max(0, math.floor(effect(effect_table, EFFECT_HINT_FREQUENCY, lv)))
            + unique(sid, EFFECT_HINT_FREQUENCY, lv),
            levels=1
            + max(0, math.floor(effect(effect_table, EFFECT_HINT_LEVELS, lv)))
            + unique(sid, EFFECT_HINT_LEVELS, lv),
            specialty=max(0, math.floor(effect(effect_table, EFFECT_SPECIALTY, lv)))
            + unique(sid, EFFECT_SPECIALTY, lv),
        )

    web_cards = common.gjson("web_cards.json")
    card_events = common.gjson("card_events.json")
    out: list[dict[str, Any]] = list(web_cards["cards"])
    bad: list[int] = []
    for card in web_cards["cards"]:
        if at_lb(card["id"], 4) != (card["hf"], card["hinc"], card.get("spec", 0)):
            bad.append(card["id"])
        for lb in range(4):
            fx = at_lb(card["id"], lb)
            out.append(dict(card, id=variant(card["id"], lb), hf=fx.frequency, hinc=fx.levels, spec=fx.specialty))
    restrict = {sc: ids + [variant(i, lb) for i in ids for lb in range(4)] for sc, ids in web_cards["restrict"].items()}
    events = dict(card_events["cards"])
    for k, v in card_events["cards"].items():
        for lb in range(4):
            events[str(variant(int(k), lb))] = v
    os.makedirs(OUTD, exist_ok=True)
    json.dump({"cards": out, "restrict": restrict}, open(os.path.join(OUTD, "cards_lb.json"), "w"), ensure_ascii=False)
    json.dump(
        {"about": card_events["about"], "cards": events},
        open(os.path.join(OUTD, "events_lb.json"), "w"),
        ensure_ascii=False,
    )
    return bad


# ---------------------------------------------------------------- jobs
class RaceWins(NamedTuple):
    """the race indices each ancestor is laid out to have won (Model.wins)"""

    parent1: list[int]
    grandparent1a: list[int]
    grandparent1b: list[int]
    parent2: list[int]
    grandparent2a: list[int]
    grandparent2b: list[int]


class Model:
    """the game data the jobs and the analysis read"""

    cards: dict[int, dict[str, Any]]  # web_cards.json by card id
    web_skills: dict[str, Any]  # web_skills.json
    skill_groups: set[int]  # every skill group web_skills.json lists (a job can target)
    factor_group: dict[int, int]  # white spark factor id // 100 -> skill group
    race_count: int  # races in data.json
    card_events: dict[str, dict[str, Any]]  # card_events.json's cards, by card id as a string
    skill_level: dict[str, tuple[int, int]]  # skill name -> (group, 1 white / 2 gold)
    name_group: dict[str, int]  # skill name -> group

    def __init__(self) -> None:
        self.cards = common.cards()
        self.web_skills = common.gjson("web_skills.json")
        self.skill_groups = {g for g, w, gd in self.web_skills["skills"]}
        self.factor_group = common.factor_groups()
        self.race_count = len(common.gjson("data.json")["races"])
        self.card_events = common.gjson("card_events.json")["cards"]
        self.skill_level = common.skill_names()
        self.name_group = {n: g for n, (g, _) in self.skill_level.items()}

    def sparks(self, factors: Iterable[Sequence[int]]) -> list[list[int]]:
        """an ancestor's white sparks: [skill group, stars], the most stars per group"""
        out: dict[int, int] = {}
        for f, _ in factors:
            if 2000000 <= f < 3000000 and f // 100 in self.factor_group:
                g = self.factor_group[f // 100]
                out[g] = max(out.get(g, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    @staticmethod
    def pinks(factors: Iterable[Sequence[int]]) -> list[list[int]]:
        """an ancestor's pink sparks: [factor id // 100, stars]"""
        out: dict[int, int] = {}
        for f, _ in factors:
            if f < 10000 and f // 100 in PINK:
                out[f // 100] = max(out.get(f // 100, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    def ev_groups(self, c: int) -> set[int]:
        """skill groups a card's events can hint (every reward the solver reads)"""
        e = self.card_events.get(str(c), {})
        names: set[str] = set()
        for key in ("chain", "end"):
            for link in e.get(key, []):
                for n in link[1]:
                    names.add(n[0] if isinstance(n, list) else n)
        for alt in e.get("alts", []):
            for choice in alt[1]:
                for outcome in choice:
                    names.update(outcome)
        names.update(e.get("always", []))
        names.update(x[0] for x in e.get("chance", []))
        return {self.name_group[n] for n in names if n in self.name_group}

    def deck_groups(self, cards: Iterable[int]) -> set[int]:
        """skill groups a deck can hint: its cards' training pools and events"""
        gs: set[int] = set()
        for c in cards:
            gs.update(self.cards[c]["whites"])
            gs |= self.ev_groups(c)
        return gs

    def spark_groups(self, r: Run) -> set[int]:
        """skill groups any of the run's four ancestors carries a white spark of"""
        return {g for p in r["parents"] for a in [p] + p["gps"] for g, s in self.sparks(a["factors"]) if s > 0}

    def wins(self, affinity: dict[str, list[int]]) -> RaceWins | None:
        """race wins laid out so every pair the model reads shares the record's shared-G1 count: parent 1 wins races
        0..m1-1 (m1 the most she shares with anyone), her grandparents its first a1 / b1; parent 2 shares the first s12
        of them and wins new races after m1 for what her grandparents share beyond s12. None when that needs more
        races than the race list has."""

        def shared(pair: str) -> int:
            return affinity[pair][1]

        s12, a1, b1, a2, b2 = (
            shared("parent 1 × parent 2"),
            shared("parent 1 × grandparent 1a"),
            shared("parent 1 × grandparent 1b"),
            shared("parent 2 × grandparent 2a"),
            shared("parent 2 × grandparent 2b"),
        )
        m1 = max(s12, a1, b1)
        p1 = list(range(m1))
        p2 = list(range(s12)) + list(range(m1, m1 + max(0, max(a2, b2) - s12)))
        if m1 + max(0, max(a2, b2) - s12) > self.race_count:
            return None
        return RaceWins(p1, p1[:a1], p1[:b1], p2, p2[:a2], p2[:b2])


def drop_reason(r: Run, M: Model, outfits: list[int]) -> str | None:
    """why a run cannot be rebuilt as a job (outfits: her character's outfits in web_skills.json), or None"""
    chara, deck, parents = r["chara"], r["deck"], r["parents"]
    if len(deck) != 6:
        return "deck not 6 cards"
    if any(c not in M.cards for c, _, _ in deck):
        return "card not in web_cards"
    if any(M.cards[c]["rarity"] == 1 for c, _, _ in deck):
        return "R card in deck"
    if r["outfit"] not in outfits:
        return "outfit not in web_skills"
    if parents[0]["chara"] == parents[1]["chara"] or chara in (parents[0]["chara"], parents[1]["chara"]):
        return "same parent twice"
    return None


def legacy_tree(r: Run, M: Model, wins: RaceWins) -> list[dict[str, Any]]:
    """the job's two legacies, each with its two recorded parents, sparks and laid-out race wins"""

    def ancestor(a: dict[str, Any], won: list[int]) -> dict[str, Any]:
        return {"chara": a["chara"], "sparks": M.sparks(a["factors"]), "pinks": M.pinks(a["factors"]), "wins": won}

    parents = r["parents"]
    legs: list[dict[str, Any]] = []
    for parent, won, grandparent_wins in (
        (parents[0], wins.parent1, (wins.grandparent1a, wins.grandparent1b)),
        (parents[1], wins.parent2, (wins.grandparent2a, wins.grandparent2b)),
    ):
        leg = ancestor(parent, won)
        leg["borrowed"] = False
        leg["parents"] = [ancestor(g, gw) for g, gw in zip(parent["gps"], grandparent_wins)]
        legs.append(leg)
    return legs


def job_targets(r: Run, M: Model, wide: bool, scen: str) -> tuple[list[dict[str, Any]], list[int]]:
    """the job's targets and the candidate targets nothing can give. A target nothing can give has P = 0 in every
    mechanism; the career search drops careers with P(all) = 0, so such targets are not sent and are scored 0 by the
    analysis (listed in the manifest)"""
    chara, outfit = r["chara"], r["outfit"]
    deck_ids = [c for c, _, _ in r["deck"]]
    if wide:
        candidates = sorted(
            g for g in set(DEFAULT) | M.deck_groups(deck_ids) | M.spark_groups(r) if g in M.skill_groups
        )
    else:
        candidates = list(DEFAULT)
    outfit_gives = {g for o in M.web_skills["cards"][str(chara)] if o[0] == outfit for g, _ in o[2]} | {
        g for u, g, _ in EXTRA if u == chara
    }
    reachable = M.deck_groups(deck_ids) | M.spark_groups(r) | outfit_gives | (UNITY_GOLDS if scen == "unity" else set())
    unreachable = [g for g in candidates if g not in reachable]
    targets = [
        {"group": g, **({"guaranteed": 2} if scen == "unity" and g in UNITY_GOLDS else {})}
        for g in candidates
        if g in reachable
    ]
    return targets, unreachable


def build_jobs(
    R: list[Run], M: Model, wide: bool, jobdir: str
) -> tuple[list[ManEntry], list[str], collections.Counter[str]]:
    """one career-planner job file per usable run (--wide: per run and chunk of targets) and manifest.json; returns the
    manifest entries, the job file ids and the dropped runs by reason"""
    os.makedirs(jobdir, exist_ok=True)
    drop: collections.Counter[str] = collections.Counter()
    man: list[ManEntry] = []
    files: list[str] = []
    for r in R:
        chara, outfit = r["chara"], r["outfit"]
        outfits = [o[0] for o in M.web_skills["cards"].get(str(chara), [])]
        reason = drop_reason(r, M, outfits)
        if reason:
            drop[reason] += 1
            continue
        wins = M.wins(r["affinity"])
        if wins is None:
            drop["shared G1 counts exceed the race list"] += 1
            continue
        legs = legacy_tree(r, M, wins)
        scen = SCENARIO_KEY[r["scenario"]]
        targets, unreachable = job_targets(r, M, wide, scen)
        ids = [variant(c, lb) for c, lb, _ in r["deck"]]
        job = {
            "mode": "career",
            "iters": 30,
            "seed": 3,
            "skills_file": os.path.join(GD, "web_skills.json"),
            "cards_file": os.path.join(OUTD, "cards_lb.json"),
            "events_file": os.path.join(OUTD, "events_lb.json"),
            "scenario": scen,
            "targets": targets,
            "extra": EXTRA,
            "exclude_outfits": [o for o in outfits if o != outfit],
            "owned": [chara],
            "trainees": [chara],
            "owned_cards": ids,
            "lock_cards": ids,
            "borrow": False,
            "legacies": legs,
            "fix": [0, 1],
            "top": 1,
            "per_trainee": True,
        }
        if not wide:
            json.dump(job, open(os.path.join(jobdir, f"{r['id']}.json"), "w"))
            files.append(str(r["id"]))
        else:
            rest = [t for t in targets if t["group"] not in DEFAULT]
            for k in range(0, len(rest), WIDE_JOB_TARGETS):
                chunk = f"{r['id']}_{k // WIDE_JOB_TARGETS}"
                json.dump(
                    dict(job, targets=rest[k : k + WIDE_JOB_TARGETS]), open(os.path.join(jobdir, f"{chunk}.json"), "w")
                )
                files.append(chunk)
        man.append(ManEntry(r["id"], unreachable))
    json.dump({"runs": man, "files": files, "dropped": drop}, open(os.path.join(jobdir, "manifest.json"), "w"))
    return man, files, drop


def fingerprint(jobdir: str, files: Iterable[str]) -> str:
    """a hash of the job files and of everything else the solver reads"""
    h = hashlib.sha256()
    for f in sorted(files):
        h.update(open(os.path.join(jobdir, f + ".json"), "rb").read())
    for p in (
        common.SOLVER,
        os.path.join(GD, "data.json"),
        os.path.join(OUTD, "cards_lb.json"),
        os.path.join(OUTD, "events_lb.json"),
    ):
        h.update(open(p, "rb").read())
    return h.hexdigest()


def solver_lines(stdout: str) -> list[str]:
    """the solver's JSON output lines, one per job"""
    return [line for line in stdout.splitlines() if line.startswith("{")]


def solve_one_by_one(jobdir: str, files: list[str], timeout: float) -> tuple[SolverOut, int]:
    """one solver call per job, each with a time limit: (outputs, jobs over the limit; their output {'timeout': True})"""
    res: SolverOut = {}
    over = 0
    for x in files:
        try:
            p = subprocess.run(
                [common.SOLVER, "data.json", os.path.join(jobdir, x + ".json")],
                cwd=GD,
                capture_output=True,
                text=True,
                timeout=timeout,
            )
            lines = solver_lines(p.stdout)
            if p.returncode != 0 or len(lines) != 1:
                sys.exit(f"solver failed on {x}: rc {p.returncode}\n{p.stderr[-2000:]}")
            res[x] = json.loads(lines[0])
        except subprocess.TimeoutExpired:
            over += 1
            res[x] = {"timeout": True}
    return res, over


def solve_in_batches(jobdir: str, files: list[str]) -> SolverOut:
    """SOLVER_BATCH jobs per solver call"""
    res: SolverOut = {}
    for i in range(0, len(files), SOLVER_BATCH):
        part = files[i : i + SOLVER_BATCH]
        p = subprocess.run(
            [common.SOLVER, "data.json"] + [os.path.join(jobdir, x + ".json") for x in part],
            cwd=GD,
            capture_output=True,
            text=True,
        )
        lines = solver_lines(p.stdout)
        if p.returncode != 0 or len(lines) != len(part):
            sys.exit(f"solver failed on chunk starting {part[0]}: rc {p.returncode}\n{p.stderr[-2000:]}")
        for x, line in zip(part, lines):
            res[x] = json.loads(line)
    return res


def run_solver(
    jobdir: str,
    files: list[str],
    resf: str,
    timeout: float | None = None,
    share: float | None = None,
    man: list[ManEntry] | None = None,
) -> SolverOut:
    """one process at a time; returns {file id: output}; cached in resf with a fingerprint of everything read. share: the
    seeded fraction of the manifest's runs whose jobs run (run ids drawn in order)"""
    if share is not None:
        keep = set(random.Random(1).sample(sorted(m.run_id for m in man), round(share * len(man))))
        files = [x for x in files if int(x.split("_")[0]) in keep]
    fp = fingerprint(jobdir, files)
    if os.path.exists(resf):
        with open(resf) as f:
            head = json.loads(f.readline())
            if head.get("fingerprint") == fp:
                return {x["id"]: x["out"] for x in map(json.loads, f)}
    if not os.path.exists(common.SOLVER):
        sys.exit("native solver missing: cd pkg/loopopt && cargo build --release")
    t0 = time.time()
    print(f"solver: {len(files)} jobs, one process", file=sys.stderr, flush=True)
    if timeout:
        res, over = solve_one_by_one(jobdir, files, timeout)
    else:
        res, over = solve_in_batches(jobdir, files), 0
    print(f"solver: done in {time.time() - t0:.0f} s, {over} over the time limit", file=sys.stderr, flush=True)
    with open(resf, "w") as f:
        f.write(json.dumps({"fingerprint": fp}) + "\n")
        for x in files:
            f.write(json.dumps({"id": x, "out": res[x]}) + "\n")
    return res


# ---------------------------------------------------------------- analysis rows
class HintSeen(NamedTuple):
    """one hint record of a target"""

    tier: int  # 1 white, 2 gold
    sources: list[str]  # the record's credited sources ("<card> event", a card's character, "Inspiration", ...)
    level: int  # the hint level shown


@dataclass
class Inspirations:
    """a skill group's inspirations over years 2 and 3"""

    years: int = 0  # years with at least one
    hits: int = 0  # every hit, summed over both years


def hints_by_group(
    r: Run, M: Model, other_names: dict[str, tuple[int, int]], bad: collections.Counter[str]
) -> collections.defaultdict[int, list[HintSeen]]:
    """the run's hint records by skill group; a name in neither web_skills.json nor other_names counts in bad"""
    hints: collections.defaultdict[int, list[HintSeen]] = collections.defaultdict(list)
    for h in r["hints"]:
        if h["name"] in M.skill_level:
            g, tier = M.skill_level[h["name"]]
        elif h["name"] in other_names:
            g, tier = other_names[h["name"]]
        else:
            bad["hint name unmapped"] += 1
            continue
        hints[g].append(HintSeen(tier, h["src"], h["lv"]))
    return hints


def inspirations_of(r: Run, M: Model) -> collections.defaultdict[int, Inspirations]:
    """the run's white-spark inspirations in years 2 and 3, by skill group"""
    insp: collections.defaultdict[int, Inspirations] = collections.defaultdict(Inspirations)
    for y in ("2", "3"):
        seen: set[int] = set()
        for f, hit in r["years"].get(y, []):
            if 2000000 <= f < 3000000 and f // 100 in M.factor_group:
                g = M.factor_group[f // 100]
                insp[g].hits += hit
                if g not in seen:
                    insp[g].years += 1
                    seen.add(g)
    return insp


def credited(sources: Iterable[str], card_of: dict[str, int]) -> set[tuple[int, bool]]:
    """the deck cards a hint's sources credit (card_of: a deck card's character -> card): (card, by its event)"""
    cs: set[tuple[int, bool]] = set()
    for t in sources:
        for n in t[:-6].split(" or ") if t.endswith(" event") else [t]:
            if n in card_of:
                cs.add((card_of[n], t.endswith(" event")))
    return cs


def run_rows(
    r: Run, career: dict[str, Any], unreachable: list[int], wide: bool, M: Model, bad: collections.Counter[str]
) -> RunRows:
    """one run's rows: a row per target of the solver's best career (career), then (default targets) one per target
    nothing can give"""
    card_of = {common.bare(nm): c for c, lb, nm in r["deck"]}
    # skill names outside web_skills.json, from the run's own hint groups: name -> (group, 1 white / 2 gold)
    other_names = {v[1]: (gid, 2 if (v[2] or 1) >= 2 else 1) for gid, vs in r.get("hint_groups", []) for v in vs}
    hints = hints_by_group(r, M, other_names, bad)
    owned = {s // 10 for s in r.get("owned", [])}
    insp = inspirations_of(r, M)
    solved = {t["group"]: t for t in career["targets"]}
    order = [t["group"] for t in career["targets"]]
    slots = career["slots"]
    run: RunRows = {
        "id": r["id"],
        "scen": r["scenario"],
        "p_all": career["p_all"] * (0 if unreachable else 1),
        "rows": [],
        "wide": wide,
    }
    for g in order + ([] if wide else unreachable):
        t = solved.get(g)
        if t is None:
            # the solver's acquisition odds: gold, white, from the deck, from her outfit (0 none, 1 white, 2 gold)
            buy: dict[str, Any] = {"gold": 0.0, "white": 0.0, "deck_gold": 0.0, "deck_white": 0.0, "outfit": 0}
            q, nc, p, scen_g, plegs = 0.0, 0, 0.0, False, []
        else:
            buy = t["buy"]
            q, nc, p, scen_g = t["q"], t["carriers"], t["p"], bool(buy.get("scenario"))
            k = order.index(g)
            # per legacy carrying the spark: 3% per star, raised by its affinity, per roll
            plegs = [min(1.0, 0.03 * s["stars"][k] * (1 + s["aff"] / 100)) for s in slots if s["stars"][k] > 0]
        hs = hints.get(g, [])
        deck_sources = set().union(*[credited(h.sources, card_of) for h in hs]) if hs else set()
        prio = (
            None
            if r["priority"] is None
            else any(M.skill_level.get(n, other_names.get(n, (None,)))[0] == g for n in r["priority"])
        )
        row = CalRow(
            run=r["id"],
            scen=r["scenario"],
            g=g,
            scen_gold=scen_g,
            outfit=buy.get("outfit", 0),
            zero=t is None,
            dg=buy.get("deck_gold", 0.0),
            dw=buy.get("deck_white", 0.0),
            q=q,
            nc=nc,
            p=p,
            acq=1.0 if scen_g else buy["gold"] + buy["white"],
            gold=1.0 if scen_g else buy["gold"],
            plegs=plegs,
            o_any=bool(hs) or g in owned,
            o_gold=any(h.tier == 2 for h in hs),
            o_deck=bool(deck_sources),
            o_deck_gold=any(h.tier == 2 and credited(h.sources, card_of) for h in hs),
            o_insp=insp[g].years > 0,
            o_insp_years=insp[g].years,
            o_insp_hits=insp[g].hits,
            o_insp_src=any("Inspiration" in h.sources for h in hs),
            o_events=any("Events" in h.sources for h in hs),
            o_other=any(
                any(x not in ("Inspiration", "Events") and not credited([x], card_of) for x in h.sources) for h in hs
            ),
            prio=prio,
            pool_lb=min(
                [lb for c, lb, _ in r["deck"] if g in M.cards[c]["whites"]] or [None],
                key=lambda v: 9 if v is None else v,
            ),
            cards_pool=[c for c, _, _ in r["deck"] if g in M.cards[c]["whites"]],
            cards_ev=[c for c, _, _ in r["deck"] if g in M.ev_groups(c)],
        )
        carriers = 1.1 ** (nc - 6)
        row["p_pred"] = SPARK_GOLD_6 * carriers if scen_g else p
        row["l_obs"] = (
            SPARK_GOLD_6 * carriers
            if (row["o_gold"] or row["outfit"] == 2)
            else SPARK_WHITE_6 * carriers
            if (row["o_any"] or row["outfit"] == 1)
            else 0.0
        )
        run["rows"].append(row)
    return run


def rows_of(
    R: list[Run], M: Model, res: SolverOut, man: list[ManEntry]
) -> tuple[list[CalRow], list[RunRows], collections.Counter[str]]:
    """the (run, target) rows of every solved job, the rows by run, and the problems met (by kind)"""
    by_id = {r["id"]: r for r in R}
    unreachable = {m.run_id: m.unreachable for m in man}
    rows: list[CalRow] = []
    runrows: list[RunRows] = []
    bad: collections.Counter[str] = collections.Counter()
    for fid, o in res.items():
        wide = "_" in fid
        r = by_id[int(fid.split("_")[0])]
        if "results" not in o or not o["results"]:
            bad["no result"] += 1
            continue
        run = run_rows(r, o["results"][0], unreachable[r["id"]], wide, M, bad)
        rows.extend(run["rows"])
        runrows.append(run)
    return rows, runrows, bad


# ---------------------------------------------------------------- tables
class Stat(NamedTuple):
    """the counts and intervals of a set of rows (row_ci): the observed rate and observed / predicted, 95%"""

    rows: int
    predicted: float  # the mean prediction
    observed: float  # the observed rate
    hits: int
    low: float  # the observed rate's interval
    high: float
    ratio: float  # observed / predicted
    ratio_low: float
    ratio_high: float


class Bucket(NamedTuple):
    """one bucket of a reliability table: rows predicted in [low_edge, high_edge)"""

    low_edge: float
    high_edge: float
    rows: int
    observed: float
    predicted: float  # the mean prediction
    observed_low: float  # the observed rate's 95% interval (row_ci)
    observed_high: float


NAN: float = float("nan")
NO_BUCKET: Bucket = Bucket(NAN, NAN, 0, NAN, NAN, NAN, NAN)
ZERO_EDGE: float = 1e-12  # the high edge of the bucket predicted 0
EDGES: list[float] = [0, ZERO_EDGE, 0.2, 0.4, 0.6, 0.8, 0.95, 1 + 1e-9]


def row_ci(rs: list[CalRow], pred: Callable[[CalRow], float], obs: RowFn) -> Stat:
    """the Stat of rows rs: where a run gives several rows, the run-clustered ratio estimator (common.cluster_ci) and
    the rate's interval that ratio interval times the mean prediction (nothing predicted: the rate's own run-clustered
    interval); where every run gives one row, Wilson on the observed rate, divided by the mean prediction for the
    ratio"""
    n = len(rs)
    if n == 0:
        return Stat(0, NAN, NAN, 0, NAN, NAN, NAN, NAN, NAN)
    ps = [pred(r) for r in rs]
    hit = [1 if obs(r) else 0 for r in rs]
    mean_pred = sum(ps) / n
    k = sum(hit)
    if len({r["run"] for r in rs}) == n:
        lo, hi = wilson(k, n)
        return Stat(
            n,
            mean_pred,
            k / n,
            k,
            lo,
            hi,
            common.div(k / n, mean_pred),
            common.div(lo, mean_pred),
            common.div(hi, mean_pred),
        )
    if not mean_pred:  # nothing predicted (no ratio): the observed rate with its run-clustered interval
        _, lo, hi, _ = common.cluster_ci(hit, [1] * n, [r["run"] for r in rs])
        return Stat(n, mean_pred, k / n, k, lo, hi, NAN, NAN, NAN)
    ratio, rlo, rhi, _ = common.cluster_ci(hit, ps, [r["run"] for r in rs])
    return Stat(n, mean_pred, k / n, k, rlo * mean_pred, rhi * mean_pred, ratio, rlo, rhi)


class Report:
    """the analysis tables written to report8.txt / reportW.txt"""

    rows: list[CalRow]
    lines: list[str]

    def __init__(self, rows: list[CalRow]) -> None:
        self.rows, self.lines = rows, []

    def line(self, *a: object) -> None:
        self.lines.append(" ".join(str(x) for x in a))

    def stat(self, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn) -> Stat:
        """the rows sel picks: counts, the observed rate and obs / pred with their 95% intervals (row_ci)"""
        return row_ci([r for r in self.rows if sel(r)], pred, obs)

    def table(
        self,
        title: str,
        sel: RowFn,
        pred: Callable[[CalRow], float],
        obs: RowFn,
        key: RowFn | None = None,
        min_rows: int = 0,
    ) -> dict[Any, Stat]:
        """the stat of the rows sel picks per value of key, largest groups first, written as a table"""
        self.line(f"\n## {title}")
        groups: collections.defaultdict[Any, list[CalRow]] = collections.defaultdict(list)
        for r in self.rows:
            if sel(r):
                groups[key(r) if key else "all"].append(r)
        res: dict[Any, Stat] = {}
        for k in sorted(groups, key=lambda k: (-len(groups[k]), str(k))):
            if len(groups[k]) < min_rows:
                continue
            s = self.stat(lambda r, k=k: sel(r) and (key(r) if key else "all") == k, pred, obs)
            res[k] = s
            self.line(
                f"{str(k)[:60]:60s} {s.rows:6d} {s.predicted:7.3f} {s.observed:7.3f} [{s.low:6.3f}, {s.high:6.3f}] "
                f"{s.ratio:7.3f} [{s.ratio_low:5.2f}, {s.ratio_high:5.2f}]"
            )
        return res

    def reliability(
        self, title: str, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn, edges: Sequence[float]
    ) -> list[Bucket]:
        """the rows sel picks, per bucket of the prediction between successive edges, written as a table"""
        self.line(f"\n## Reliability: {title}")
        res: list[Bucket] = []
        for a, b in pairwise(edges):
            s = row_ci([r for r in self.rows if sel(r) and a <= pred(r) < b], pred, obs)
            res.append(Bucket(a, b, s.rows, s.observed, s.predicted, s.low, s.high))
            if s.rows:
                self.line(
                    f"[{a:.3g}, {b:.3g}) n {s.rows:6d} pred {s.predicted:.3f} obs {s.observed:.3f} "
                    f"[{s.low:.3f}, {s.high:.3f}]"
                )
        return res


def bootstrap_interval[T](
    statistic: Callable[[list[T]], float], xs: Sequence[T], draws: int = 1000, seed: int = 1
) -> tuple[float, float]:
    """the 95% percentile interval of statistic over seeded resamples of xs"""
    rnd = random.Random(seed)
    n = len(xs)
    v: list[float] = []
    for _ in range(draws):
        s = [xs[rnd.randrange(n)] for _ in range(n)]
        v.append(statistic(s))
    v.sort()
    return v[int(0.025 * draws)], v[int(0.975 * draws)]


# ---------------------------------------------------------------- row selections and predictions
def every_row(r: CalRow) -> bool:
    return True


def is_open(r: CalRow) -> bool:
    """neither her outfit nor the scenario gives the target: the deck, inspiration and events must"""
    return not r["outfit"] and not r["scen_gold"]


def in_deck_test(r: CalRow) -> bool:
    """an open target no inspiration touched: the deck's hints alone"""
    return is_open(r) and not r["o_insp"] and not r["o_insp_src"]


def deck_odds(r: CalRow) -> float:
    """predicted P(a deck card hints the target, gold or white)"""
    return r["dg"] + (1 - r["dg"]) * r["dw"]


def deck_hinted(r: CalRow) -> bool:
    return r["o_deck"]


def inspiration_odds(r: CalRow) -> float:
    return r["q"]


def inspired(r: CalRow) -> bool:
    return r["o_insp"]


def acquisition_odds(r: CalRow) -> float:
    return r["acq"]


def acquired(r: CalRow) -> bool:
    return r["o_any"]


def acquired_modelled(r: CalRow) -> bool:
    """acquired through a source the model has: a deck card's hint or an inspiration"""
    return r["o_deck"] or r["o_insp"] or r["o_insp_src"]


def source_kind(r: CalRow) -> str:
    """which deck sources could hint the target"""
    return (
        "pool+event"
        if r["cards_pool"] and r["cards_ev"]
        else "training pool only"
        if r["cards_pool"]
        else "events only"
        if r["cards_ev"]
        else "no deck source"
    )


def one_deck_source(r: CalRow) -> bool:
    """exactly one deck card can hint the target (by training or event)"""
    return len(set(r["cards_pool"]) | set(r["cards_ev"])) == 1


def deck_source(r: CalRow) -> int:
    """the one deck card that can hint the target (one_deck_source)"""
    return (set(r["cards_pool"]) | set(r["cards_ev"])).pop()


def where(*conditions: RowFn) -> RowFn:
    """the rows meeting every condition, tested in order"""
    return lambda r: all(c(r) for c in conditions)


def target_is(g: int) -> RowFn:
    return lambda r: r["g"] == g


def scenario_is(scenario: int) -> RowFn:
    return lambda r: r["scen"] == scenario


def kind_is(kind: str) -> RowFn:
    return lambda r: source_kind(r) == kind


def lowest_lb_is(lb: int) -> RowFn:
    return lambda r: r["pool_lb"] == lb


def priority_is(prio: bool | None) -> RowFn:
    return lambda r: r["prio"] is prio


def single_source_is(card: int | None) -> RowFn:
    return lambda r: one_deck_source(r) and deck_source(r) == card


def deck_test(rep: Report, *conditions: RowFn) -> Stat:
    """test 1 (deck hints, no inspiration seen) on the rows meeting the conditions"""
    return rep.stat(where(in_deck_test, *conditions), deck_odds, deck_hinted)


def inheritance_test(rep: Report, *conditions: RowFn) -> Stat:
    """test 2 (inspiration of the target) on the rows meeting the conditions"""
    return rep.stat(where(*conditions), inspiration_odds, inspired)


def acquisition_test(rep: Report, *conditions: RowFn) -> Stat:
    """test 3 (acquired from any source, open targets) on the rows meeting the conditions"""
    return rep.stat(where(is_open, *conditions), acquisition_odds, acquired)


def covers(s: Stat) -> bool:
    return s.ratio_low <= 1 <= s.ratio_high


def below(s: Stat) -> bool:
    return s.ratio_high < 1


def ratio_text(s: Stat) -> str:
    return f"{s.ratio:.3f} [{s.ratio_low:.3f}, {s.ratio_high:.3f}], {s.rows:,} rows"


def bucket_label(b: Bucket) -> str:
    return "0" if b.high_edge <= ZERO_EDGE else f"{b.low_edge:.2f}-{min(b.high_edge, 1):.2f}"


# ---------------------------------------------------------------- values
# the value names of a test and the Stat field each reads
STAT: dict[str, str] = {
    "rows": "rows",
    "predicted": "predicted",
    "observed": "observed",
    "obs / pred": "ratio",
    "ratio 95% low": "ratio_low",
    "ratio 95% high": "ratio_high",
}
RATIO: tuple[str, ...] = ("obs / pred", "ratio 95% low", "ratio 95% high")
# the single-source (card, target) pairs the document's table lists
SINGLE: list[tuple[str, int]] = [
    ("[This Might Sting!] Sasami Anshinzawa", 20203),
    ("[Planned Perfection] Riko Kashimoto", 20046),
    ("[Tonight, We Waltz] King Halo", 20159),
    ("[That Time I Became the Strongest] Gold Ship", 20159),
    ("[Cute + Cute = ?] Mayano Top Gun", 20049),
    ("[Sentimental Flare ♪] Maruzensky", 20165),
    ("[Let's Get This Party Lit!] Daitaku Helios", 20165),
    ("[Sunlit Angel ♪] Daitaku Helios", 20165),
    ("[Lamplit Training of a Witch-to-Be] Sweep Tosho", 20165),
    ("[Trusting the Dice] Nakayama Festa", 20203),
]
# the three event sources of Playtime's Over! (the skill on the chain-ending choice 2)
PO_CARDS: tuple[str, ...] = (
    "[Fiery Discipline] Yaeno Muteki",
    "[Dancing Light into the Night] Yukino Bijin",
    "[Escorte Étoile] Sirius Symboli",
)
PLAYTIMES_OVER: int = 20166
RACING_SPIRIT_STAMINA: int = 21010
NO_DECK_SOURCE: set[int] = {RACING_SPIRIT_STAMINA}  # no deck card can hint it: no deck test per target
INHERITANCE_INTERVAL_QUOTED: set[int] = {20161}  # the per-target intervals the document quotes
ACQUIRED_INTERVAL_QUOTED: set[int] = {20046, 20203}
GOLD_INTERVAL_QUOTED: set[int] = {20203, 20165, 20046}
# the golds whose acquisition is tested: (group, outside Unity Cup only)
GOLD_TESTS: tuple[tuple[int, bool], ...] = (
    (20203, False),
    (20165, False),
    (20046, True),
    (20049, True),
    (20166, False),
)
LARGE_PAIR: int = 40  # rows a single-source pair needs for the document's table


def test_values(name: str, s: Stat, fields: Iterable[str] = tuple(STAT), note: str = "") -> list[Value]:
    """the named values of one test (rows, predicted, observed, obs / pred, ratio 95% low / high), from its stat"""
    return [Value(S13, f"{name}: {f}", getattr(s, STAT[f]), note=note) for f in fields]


@dataclass
class Calibration:
    """what every test reads"""

    rows: list[CalRow]
    runs: list[RunRows]
    report: Report  # the tables go here (report8.txt)
    skill: dict[int, tuple[str, str | None]]  # group -> (white name, gold name)
    records: list[Run] | None  # the run records (their priority lists)
    card_id: dict[str, int]  # web_cards.json: card name -> id
    card_name: dict[int, str]  # id -> card name


def calibration(R: list[Run] | None, M: Model, why: str | None) -> Calibration:
    """the default-target jobs solved and read into rows; without data (why) no rows"""
    bad: list[int]
    drop: dict[str, int]
    rows: list[CalRow]
    runrows: list[RunRows]
    badrows: dict[str, int]
    if why:  # nothing computed: only the findings, as "needs data"
        bad, drop, rows, runrows, badrows = [], {}, [], [], {}
    else:
        os.makedirs(OUTD, exist_ok=True)
        bad = card_variants()
        jobdir = os.path.join(OUTD, "jobs8")
        man, files, drop = build_jobs(R, M, False, jobdir)
        res = run_solver(jobdir, files, os.path.join(OUTD, "res8.jsonl"))
        rows, runrows, badrows = rows_of(R, M, res, man)
    rep = Report(rows)
    rep.line(
        f"card variants: max-limit-break check {bad or 'all equal'}; dropped {dict(drop)}; problems {dict(badrows)}"
    )
    web_cards = common.gjson("web_cards.json")["cards"]
    return Calibration(
        rows,
        runrows,
        rep,
        common.skills(),
        R,
        card_id={c["name"]: c["id"] for c in web_cards},
        card_name={c["id"]: c["name"] for c in web_cards},
    )


def run_counts(cal: Calibration, out: list[Value]) -> None:
    rows = cal.rows
    by_scenario = collections.Counter(x["scen"] for x in cal.runs)
    out += [
        Value(S13, "runs used", len(cal.runs)),
        Value(S13, "URA Finale runs", by_scenario[1]),
        Value(S13, "Unity Cup runs", by_scenario[2]),
        Value(S13, "Grand Concert runs", by_scenario[3]),
        Value(S13, "Trackblazer runs", by_scenario[4]),
        Value(S13, "run-target rows", len(rows)),
        Value(S13, "rows on her outfit", sum(1 for r in rows if r["outfit"])),
        Value(S13, "scenario-gold rows", sum(1 for r in rows if r["scen_gold"])),
    ]


@dataclass
class DeckTests:
    """test 1, deck hints on rows no inspiration touched"""

    overall: Stat
    pool_only: Stat  # training pool only
    events_prioritised: Stat  # events only, on runs showing a priority list: the target on it
    events_not_prioritised: Stat  # ... the target not on it
    events_no_list: Stat  # events only, runs showing no list


def deck_tests(cal: Calibration, out: list[Value]) -> DeckTests:
    rep = cal.report
    overall = deck_test(rep)
    out += test_values("1. deck hint, no inspiration", overall)
    out += [
        Value(S13, "1. deck hint: observed 95% low", overall.low),
        Value(S13, "1. deck hint: observed 95% high", overall.high),
    ]
    pool_only = deck_test(rep, kind_is("training pool only"))
    out += test_values("1. training pool only", pool_only)
    out += test_values("1. events only", deck_test(rep, kind_is("events only")))

    def shows_list(r: CalRow) -> bool:
        return r["prio"] is not None

    prioritised = deck_test(rep, shows_list, kind_is("events only"), lambda r: r["prio"])
    not_prioritised = deck_test(rep, shows_list, kind_is("events only"), lambda r: not r["prio"])
    no_list = deck_test(rep, priority_is(None), kind_is("events only"))
    out += test_values("1. events only, prioritised", prioritised)
    out += test_values("1. events only, not prioritised", not_prioritised)
    out += [
        Value(S9, "event-only hints, prioritised targets: obs / pred", prioritised.ratio),
        Value(S9, "event-only hints, non-prioritised targets: obs / pred", not_prioritised.ratio),
    ]
    return DeckTests(overall, pool_only, prioritised, not_prioritised, no_list)


class CountTest(NamedTuple):
    """an observed count against its expectation, z run-clustered"""

    observed: int
    expected: float
    z: float


class Chi2(NamedTuple):
    statistic: float
    buckets: int
    p: float


@dataclass
class InheritanceTests:
    """test 2, inspiration of the target, and section 3's checks on it"""

    overall: Stat
    by_carriers: dict[int, Stat]
    spark_years: CountTest  # years with an inspiration of the target
    ancestor_hits: CountTest  # inspiration hits
    buckets: list[Bucket]  # reliability by q
    misses: list[Bucket]  # buckets with a positive prediction outside their own interval
    joint: Chi2  # the buckets jointly


def spark_year_odds(r: CalRow) -> float:
    """P(a year fires an inspiration of the target): a spark-year fires per roll with y = 1 - prod(1 - r_l)"""
    return 1 - math.prod(1 - x for x in r["plegs"])


def clustered_z(resid: list[float], rows: list[CalRow]) -> float:
    """z of a count's residuals (one per row), run-clustered: a run's targets share its lineage and years"""
    if not resid:
        return float("nan")
    x2, _, _ = common.cluster_chi2([(0, d) for d in resid], [r["run"] for r in rows])
    return math.copysign(math.sqrt(x2), sum(resid))


def reliability_chi2(rows: list[CalRow]) -> Chi2:
    """jointly over the buckets with a positive prediction (and some q strictly between 0 and 1): the run-clustered Wald
    chi2 of the buckets' observed - expected (a run's targets fall in several buckets), one degree of freedom per
    bucket, nothing fitted"""
    resid: list[tuple[int, float]] = []
    runs: list[int] = []
    for j, (a, b) in enumerate(pairwise(EDGES)):
        qs = [r for r in rows if a <= r["q"] < b]
        if b <= ZERO_EDGE or not any(0 < r["q"] < 1 for r in qs):
            continue
        resid += [(j, r["o_insp"] - r["q"]) for r in qs]
        runs += [r["run"] for r in qs]
    return Chi2(*common.cluster_chi2(resid, runs)) if resid else Chi2(0.0, 0, float("nan"))


def inheritance_tests(cal: Calibration, out: list[Value]) -> InheritanceTests:
    rep, rows = cal.report, cal.rows
    overall = inheritance_test(rep)
    out += test_values("2. inheritance", overall)
    out += [
        Value(S13, "2. inheritance: observed 95% low", overall.low),
        Value(S13, "2. inheritance: observed 95% high", overall.high),
    ]
    out += [
        Value(S3, "runs tested", len(cal.runs)),
        Value(S3, "run-target rows", overall.rows),
        Value(S3, "inheritance obs / pred", overall.ratio),
        Value(S3, "inheritance ratio 95% low", overall.ratio_low),
        Value(S3, "inheritance ratio 95% high", overall.ratio_high),
    ]
    by_carriers = rep.table("2. inheritance by carriers", every_row, inspiration_odds, inspired, lambda r: r["nc"])
    ratios = [by_carriers[k].ratio for k in range(1, 7) if k in by_carriers] or [float("nan")]
    out += [
        Value(S3, "by carriers 1-6: lowest ratio", min(ratios)),
        Value(S3, "by carriers 1-6: highest ratio", max(ratios)),
    ]
    # per roll: a spark-year fires with spark_year_odds, a legacy hits with r_l; two rolls (years 2 and 3)
    hits = CountTest(
        sum(r["o_insp_hits"] for r in rows),
        sum(2 * sum(r["plegs"]) for r in rows),
        clustered_z([r["o_insp_hits"] - 2 * sum(r["plegs"]) for r in rows], rows),
    )
    years = CountTest(
        sum(r["o_insp_years"] for r in rows),
        sum(2 * spark_year_odds(r) for r in rows),
        clustered_z([r["o_insp_years"] - 2 * spark_year_odds(r) for r in rows], rows),
    )
    out += [
        Value(S3, "per roll: spark-years obs / exp", common.div(years.observed, years.expected)),
        Value(S3, "per roll: ancestor hits obs / exp", common.div(hits.observed, hits.expected)),
        Value(S3, "per roll: spark-years, SDs below expectation", -years.z),
    ]
    buckets = rep.reliability("inheritance", every_row, inspiration_odds, inspired, EDGES)
    for b in buckets[:-1]:  # the document's table stops at 0.95
        lab = bucket_label(b)
        out += [
            Value(S3, f"reliability q {lab}: rows", b.rows),
            Value(S3, f"reliability q {lab}: observed", b.observed),
        ]
        if b.high_edge > ZERO_EDGE:
            out.append(Value(S3, f"reliability q {lab}: mean predicted", b.predicted))
    misses = [
        b
        for b in buckets
        if b.rows and b.high_edge > ZERO_EDGE and not b.observed_low <= b.predicted <= b.observed_high
    ]
    joint = reliability_chi2(rows)
    out += [
        Value(S3, "reliability chi2", joint.statistic),
        Value(S3, "reliability chi2 buckets", joint.buckets),
        Value(S3, "reliability chi2 p", joint.p),
    ]
    middle = next((b for b in buckets if b.low_edge == 0.4 and b.rows), NO_BUCKET)
    out += [
        Value(S3, "reliability q 0.40-0.60: observed 95% low", middle.observed_low),
        Value(S3, "reliability q 0.40-0.60: observed 95% high", middle.observed_high),
        Value(S3, "reliability q 0.40-0.60: mean predicted, three places", middle.predicted),
    ]
    return InheritanceTests(overall, by_carriers, years, hits, buckets, misses, joint)


@dataclass
class AcquisitionTests:
    """tests 3 (acquired) and 4 (landing) on open targets"""

    any_source: Stat
    modelled_source: Stat  # acquired through a deck hint or an inspiration
    prioritised: Stat
    not_prioritised: Stat
    open_rows: list[CalRow]
    unexplained: list[CalRow]  # acquired with no modelled source
    unexplained_events_only: int  # of them, credited to events only


def acquisition_tests(cal: Calibration, out: list[Value]) -> AcquisitionTests:
    rep = cal.report
    any_source = acquisition_test(rep)
    modelled_source = rep.stat(is_open, acquisition_odds, acquired_modelled)
    out += test_values("3. acquired, any source", any_source)
    out += [
        Value(S13, "3. acquired, any source: observed 95% low", any_source.low),
        Value(S13, "3. acquired, any source: observed 95% high", any_source.high),
    ]
    out += test_values("3. acquired, deck or inspiration only", modelled_source)
    prioritised = acquisition_test(rep, priority_is(True))
    out += test_values("3. acquired, prioritised", prioritised)
    not_prioritised = acquisition_test(rep, priority_is(False))
    out += test_values("3. acquired, not prioritised", not_prioritised)
    open_rows = [r for r in cal.rows if is_open(r)]
    predicted, observed = sum(r["p_pred"] for r in open_rows), sum(r["l_obs"] for r in open_rows)
    out += [
        Value(S13, "4. landing: rows", len(open_rows)),
        Value(S13, "4. landing: sum of p", predicted),
        Value(S13, "4. landing: sum observed-equivalent", observed),
        Value(S13, "4. landing: ratio", common.div(observed, predicted)),
    ]
    unexplained = [r for r in open_rows if r["o_any"] and not acquired_modelled(r)]
    events_only = sum(1 for r in unexplained if r["o_events"] and not r["o_other"])
    out += [
        Value(S13, "acquisitions from unmodelled events", events_only),
        Value(S13, "acquisitions from unmodelled events, share of rows", common.div(events_only, len(open_rows))),
    ]
    return AcquisitionTests(
        any_source, modelled_source, prioritised, not_prioritised, open_rows, unexplained, events_only
    )


def per_target_tests(cal: Calibration, open_rows: list[CalRow], out: list[Value]) -> dict[int, Stat]:
    """per default target: the deck test where a deck card can hint the target, inheritance, acquired and landing (the
    intervals the document quotes); returns the inheritance tests"""
    rep = cal.report
    inheritance: dict[int, Stat] = {}
    for g in DEFAULT:
        nm = cal.skill[g][0]
        if g not in NO_DECK_SOURCE:
            s = deck_test(rep, target_is(g))
            out += [
                Value(S13, f"{nm}: deck hint", s.ratio),
                Value(S13, f"{nm}: deck hint 95% low", s.ratio_low),
                Value(S13, f"{nm}: deck hint 95% high", s.ratio_high),
            ]
        s = inheritance[g] = inheritance_test(rep, target_is(g))
        out.append(Value(S13, f"{nm}: inheritance", s.ratio))
        if g in INHERITANCE_INTERVAL_QUOTED:
            out += [
                Value(S13, f"{nm}: inheritance 95% low", s.ratio_low),
                Value(S13, f"{nm}: inheritance 95% high", s.ratio_high),
            ]
        s = acquisition_test(rep, target_is(g))
        out.append(Value(S13, f"{nm}: acquired", s.ratio))
        if g in ACQUIRED_INTERVAL_QUOTED:
            out += [
                Value(S13, f"{nm}: acquired 95% low", s.ratio_low),
                Value(S13, f"{nm}: acquired 95% high", s.ratio_high),
            ]
        rg = [r for r in open_rows if r["g"] == g]
        out.append(Value(S13, f"{nm}: landing", common.div(sum(r["l_obs"] for r in rg), sum(r["p_pred"] for r in rg))))
    return inheritance


@dataclass
class PoolTests:
    """test 1 on targets only a training pool can hint"""

    by_scenario: dict[int, Stat]
    low_limit_breaks: list[Stat]  # by the pool's lowest limit break: 0, 1, 2


def pool_tests(cal: Calibration, out: list[Value]) -> PoolTests:
    rep = cal.report
    pool_only = kind_is("training pool only")
    by_scenario: dict[int, Stat] = {}
    for scn in (3, 2, 1, 4):
        by_scenario[scn] = deck_test(rep, pool_only, scenario_is(scn))
        out += test_values(f"1. training pool only, {common.SCEN[scn]}", by_scenario[scn], RATIO)
    for lb in (4, 3):
        out.append(
            Value(S13, f"1. training pool only, lowest LB {lb}", deck_test(rep, pool_only, lowest_lb_is(lb)).ratio)
        )
    low = [deck_test(rep, pool_only, lowest_lb_is(lb)) for lb in (0, 1, 2)]
    out += [
        Value(S13, "1. training pool only, LB0-2 lowest ratio", min(s.ratio for s in low)),
        Value(S13, "1. training pool only, LB0-2 highest ratio", max(s.ratio for s in low)),
        Value(S13, "1. training pool only, LB0-2 fewest rows", min(s.rows for s in low)),
        Value(S13, "1. training pool only, LB0-2 most rows", max(s.rows for s in low)),
    ]
    return PoolTests(by_scenario, low)


class UnityGolds(NamedTuple):
    runs: int  # Unity Cup runs
    with_both: int  # of them, both guaranteed golds hinted


def gold_tests(cal: Calibration, out: list[Value]) -> UnityGolds:
    """Unity Cup's guaranteed golds, and the acquisition of the golds the document lists"""
    unity = [x for x in cal.runs if x["scen"] == 2]
    golds = UnityGolds(len(unity), sum(1 for x in unity if all(r["o_gold"] for r in x["rows"] if r["scen_gold"])))
    out.append(Value(S13, "Unity Cup's guaranteed golds: runs with both", golds.with_both))
    for g, outside_unity in GOLD_TESTS:
        nm = cal.skill[g][1]
        s = cal.report.stat(
            where(
                is_open,
                lambda r: r["gold"] > 0,
                target_is(g),
                lambda r, outside_unity=outside_unity: not outside_unity or r["scen"] != 2,
            ),
            lambda r: r["gold"],
            lambda r: r["o_gold"],
        )
        out += [Value(S13, f"gold acquisition {nm}", s.ratio), Value(S13, f"gold acquisition {nm}: rows", s.rows)]
        if g in GOLD_INTERVAL_QUOTED:
            out += [
                Value(S13, f"gold acquisition {nm}: 95% low", s.ratio_low),
                Value(S13, f"gold acquisition {nm}: 95% high", s.ratio_high),
            ]
    return golds


@dataclass
class DeckReliability:
    """reliability of the deck test and of acquired, and where the deck test's shortfall (predicted minus observed
    hints) sits"""

    deck: list[Bucket]
    acquired: list[Bucket]
    deck_tested: list[Bucket]  # deck buckets with rows and a positive prediction
    shortfall: float  # over every deck-test row
    shortfall_events: float  # rows with an event source
    shortfall_top: float  # rows predicted 0.6 and above
    top_rows: int
    top_rows_with_event: int
    bucket_20_40: Bucket
    bucket_20_40_pool_rows: int  # its rows only a training pool can hint


def deck_shortfall(rows: list[CalRow], sel: RowFn) -> float:
    return sum(deck_odds(r) - r["o_deck"] for r in rows if in_deck_test(r) and sel(r))


def reliability_tests(cal: Calibration, out: list[Value]) -> DeckReliability:
    rep, rows = cal.report, cal.rows
    deck = rep.reliability("deck hint", in_deck_test, deck_odds, deck_hinted, EDGES)
    acq = rep.reliability("acquired", is_open, acquisition_odds, acquired, EDGES)
    for nm, buckets in (("deck hint", deck), ("acquired", acq)):
        for b in buckets:
            lab = bucket_label(b)
            out += [
                Value(S13, f"reliability {nm} {lab}: rows", b.rows),
                Value(S13, f"reliability {nm} {lab}: observed", b.observed),
            ]
            if b.high_edge > ZERO_EDGE:
                out.append(Value(S13, f"reliability {nm} {lab}: mean predicted", b.predicted))

    # the deck's shortfall in its buckets predicted 0.6 and above, and how many of their rows have an event source
    def top(r: CalRow) -> bool:
        return deck_odds(r) >= 0.6

    top_rows = [r for r in rows if in_deck_test(r) and top(r)]
    tested = [b for b in deck if b.rows and b.high_edge > ZERO_EDGE]
    b24 = next((b for b in tested if b.low_edge == 0.2), Bucket(0.2, 0.4, NAN, NAN, NAN, NAN, NAN))
    pool_rows_24 = sum(
        1 for r in rows if in_deck_test(r) and 0.2 <= deck_odds(r) < 0.4 and source_kind(r) == "training pool only"
    )
    rel = DeckReliability(
        deck,
        acq,
        tested,
        shortfall=deck_shortfall(rows, every_row),
        shortfall_events=deck_shortfall(rows, lambda r: source_kind(r) in ("events only", "pool+event")),
        shortfall_top=deck_shortfall(rows, top),
        top_rows=len(top_rows),
        top_rows_with_event=sum(1 for r in top_rows if r["cards_ev"]),
        bucket_20_40=b24,
        bucket_20_40_pool_rows=pool_rows_24,
    )
    out += [
        Value(S13, "deck shortfall in buckets predicted 0.6 and above", common.div(rel.shortfall_top, rel.shortfall)),
        Value(S13, "their rows with an event source", common.div(rel.top_rows_with_event, rel.top_rows)),
        Value(S13, "reliability deck hint 0.20-0.40: observed (sentence)", b24.observed),
        Value(S13, "reliability deck hint 0.20-0.40: observed 95% low", b24.observed_low),
        Value(S13, "reliability deck hint 0.20-0.40: observed 95% high", b24.observed_high),
        Value(S13, "reliability deck hint 0.20-0.40: mean predicted, three places", b24.predicted),
    ]
    return rel


@dataclass
class RunOdds:
    """one run's default targets: the open ones acquired, and landing every target"""

    open_targets: int
    acquired_expected: float  # sum of the open targets' acquisition odds
    acquired_observed: int
    all_acquired_odds: float  # the product of the open targets' acquisition odds
    all_acquired: bool
    p_all: float  # the solver's P(every target lands)
    landing_observed: float  # the product of the observed-equivalent landings
    scenario: int


class RatioInterval(NamedTuple):
    ratio: float
    low: float
    high: float


@dataclass
class PerRunTests:
    runs: list[RunOdds]  # in run-id order (the bootstrap draws)
    all_acquired_rate: float
    all_acquired_low: float  # Wilson 95%
    all_acquired_high: float
    all_acquired_odds_mean: float
    acquired_mean: float  # targets acquired per run: observed mean, its standard deviation over runs
    acquired_sd: float
    acquired_expected_mean: float
    landing: dict[str, RatioInterval]  # P(all eight) observed-equivalent / predicted, bootstrap 95%, by scenario
    exact_p_all: float  # the solver's exact P(all), summed over runs
    product_p_all: float  # the product of its per-target p, summed over runs


def per_run_tests(cal: Calibration, out: list[Value]) -> PerRunTests:
    runs: list[RunOdds] = []
    for pr in sorted(cal.runs, key=lambda x: x["id"]):  # bootstrap draws in run-id order
        targets = [r for r in pr["rows"] if r["g"] in DEFAULT]
        opened = [r for r in targets if is_open(r)]
        runs.append(
            RunOdds(
                len(opened),
                sum(r["acq"] for r in opened),
                sum(r["o_any"] for r in opened),
                math.prod(r["acq"] for r in opened),
                all(r["o_any"] for r in opened),
                pr["p_all"],
                math.prod(r["l_obs"] for r in targets),
                pr["scen"],
            )
        )
    n = len(runs) or float("nan")
    n_all = sum(x.all_acquired for x in runs)
    lo, hi = wilson(n_all, n)
    out += [
        Value(S13, "per run: open targets", sum(x.open_targets for x in runs) / n),
        Value(S13, "per run: acquired, expected", sum(x.acquired_expected for x in runs) / n),
        Value(S13, "per run: acquired, observed", sum(x.acquired_observed for x in runs) / n),
        Value(S13, "per run: all open acquired, product of odds", sum(x.all_acquired_odds for x in runs) / n),
        Value(S13, "per run: all open acquired, observed", sum(x.all_acquired for x in runs) / n),
        Value(S13, "per run: all open acquired, 95% low", lo),
        Value(S13, "per run: all open acquired, 95% high", hi),
        Value(S13, "runs acquiring every open target", n_all),
    ]
    landing: dict[str, RatioInterval] = {}
    for nm, sub in (
        ("all", runs),
        ("Unity Cup", [x for x in runs if x.scenario == 2]),
        ("Grand Concert", [x for x in runs if x.scenario == 3]),
        ("URA Finale", [x for x in runs if x.scenario == 1]),
    ):
        predicted = sum(x.p_all for x in sub)
        observed = sum(x.landing_observed for x in sub)
        blo, bhi = (
            (float("nan"),) * 2
            if not sub
            else bootstrap_interval(
                lambda s: sum(x.landing_observed for x in s) / max(1e-300, sum(x.p_all for x in s)), sub
            )
        )
        landing[nm] = RatioInterval(common.div(observed, predicted), blo, bhi)
        out += [
            Value(
                S13, f"P(all eight) ratio, {nm}", common.div(observed, predicted)
            ),  # bootstrap bounds: Monte Carlo (seeded)
            Value(S13, f"P(all eight) ratio, {nm}: 95% low", blo),
            Value(S13, f"P(all eight) ratio, {nm}: 95% high", bhi),
        ]
    out.append(
        Value(S13, "P(all eight): optimism the interval allows (1 / 95% low)", common.div(1, landing["all"].low))
    )
    out.append(Value(S13, "Trackblazer's model P(all)", sum(x.p_all for x in runs if x.scenario == 4)))
    # the solver's exact P(all) against the product of its per-target p, summed over runs (what 90 runs would have to separate)
    exact = sum(x["p_all"] for x in cal.runs)
    product = sum(math.prod(r["p_pred"] for r in x["rows"]) for x in cal.runs)
    # per run, acquired: the observed mean's 95% interval over runs
    mean = sum(x.acquired_observed for x in runs) / n
    sd = (
        math.sqrt(sum((x.acquired_observed - mean) ** 2 for x in runs) / max(1, len(runs) - 1))
        if runs
        else float("nan")
    )
    return PerRunTests(
        runs,
        n_all / n,
        lo,
        hi,
        sum(x.all_acquired_odds for x in runs) / n,
        mean,
        sd,
        sum(x.acquired_expected for x in runs) / n,
        landing,
        exact,
        product,
    )


class PairStat(NamedTuple):
    """the deck test of one (card, target) pair where the card is the target's one deck source"""

    card: int
    target: int
    stat: Stat


@dataclass
class SingleSourceTests:
    """the deck test per (card, target) pair where the card is the target's one deck source"""

    listed: set[tuple[int | None, int]]  # the pairs the document's table lists
    of_listed: dict[tuple[int | None, int], Stat]
    pairs: list[tuple[int, int]]  # every pair in the rows
    off: list[PairStat]  # pairs whose ratio interval excludes 1
    off_large: set[tuple[int, int]]  # of them, with LARGE_PAIR rows or more
    off_unlisted: list[PairStat]  # large, not in the table
    off_small: list[PairStat]  # under LARGE_PAIR rows
    gold_ship: tuple[int | None, int]  # the pair the table lists without its excluding 1
    playtime_small: list[PairStat]  # small, Playtime's Over!, below its prediction
    playtime_hits: int
    playtime_rows: int
    playtime_lowest_predicted: float


def single_source_tests(cal: Calibration, out: list[Value]) -> SingleSourceTests:
    rep = cal.report
    listed: set[tuple[int | None, int]] = set()
    of_listed: dict[tuple[int | None, int], Stat] = {}
    for card, g in SINGLE:
        cid = cal.card_id.get(card)
        listed.add((cid, g))
        s = of_listed[cid, g] = deck_test(rep, single_source_is(cid), target_is(g))
        nm = card + " > " + cal.skill[g][0]  # the full card name: two Helios cards share a target
        out += [
            Value(S13, f"{nm}: rows", s.rows),
            Value(S13, f"{nm}: predicted", s.predicted),
            Value(S13, f"{nm}: observed", s.observed),
            Value(S13, f"{nm}: ratio", s.ratio),
            Value(S13, f"{nm}: ratio 95% low", s.ratio_low),
            Value(S13, f"{nm}: ratio 95% high", s.ratio_high),
        ]
    # every single-source (card, target) pair whose ratio interval excludes 1
    pairs = sorted({(deck_source(r), r["g"]) for r in cal.rows if in_deck_test(r) and one_deck_source(r)})
    off: list[PairStat] = []
    for cid, g in pairs:
        s = deck_test(rep, single_source_is(cid), target_is(g))
        # a tolerance: k = n at a prediction of 1 is no miss
        if s.predicted and not s.ratio_low - 1e-9 <= 1 <= s.ratio_high + 1e-9:
            off.append(PairStat(cid, g, s))
    off_large = {(x.card, x.target) for x in off if x.stat.rows >= LARGE_PAIR}
    off_unlisted = [x for x in off if x.stat.rows >= LARGE_PAIR and (x.card, x.target) not in listed]
    small = [x for x in off if x.stat.rows < LARGE_PAIR]
    playtime = [x for x in small if x.target == PLAYTIMES_OVER and x.stat.ratio < 1]  # below its prediction
    hits, rows = sum(x.stat.hits for x in playtime), sum(x.stat.rows for x in playtime)
    lowest = min([x.stat.predicted for x in playtime] or [float("nan")])
    out += [
        Value(S13, "pairs under 40 rows excluding 1", len(small)),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! event sources", len(playtime)),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! event sources, runs", rows),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! event sources, hinted", hits),
        Value(S13, "pairs under 40 rows excluding 1: Playtime's Over! lowest predicted", lowest),
        Value(
            S13,
            "pairs under 40 rows excluding 1: Playtime's Over! highest predicted",
            max([x.stat.predicted for x in playtime] or [float("nan")]),
        ),
    ]
    gold_ship = (cal.card_id.get("[That Time I Became the Strongest] Gold Ship"), 20159)
    return SingleSourceTests(
        listed, of_listed, pairs, off, off_large, off_unlisted, small, gold_ship, playtime, hits, rows, lowest
    )


@dataclass
class PriorityTests:
    """the likely reasons the single-source table gives, read on the runs that show a priority list (priority by skill
    name); each a list over: the skill on the list, on a list without it[, no list shown]"""

    sasami: list[Stat]  # [This Might Sting!] Sasami Anshinzawa > Risky Business
    king_halo: list[Stat]  # [Tonight, We Waltz] King Halo > Uma Stan
    riko: list[Stat]  # [Planned Perfection] Riko Kashimoto > Ramp Up, by Lone Wolf
    playtime: list[Stat]  # the three Playtime's Over! event sources pooled, by the target's priority
    playtime_cards: set[int | None]


def priority_tests(cal: Calibration, out: list[Value]) -> PriorityTests:
    rep = cal.report
    priority_list: dict[int, list[str] | None] = {r["id"]: r["priority"] for r in (cal.records or [])}

    def on_list(skill: str, listed: bool | None) -> RowFn:
        """rows whose run lists skill (True), shows a list without it (False) or shows no list (None)"""

        def test(r: CalRow) -> bool:
            plist = priority_list.get(r["run"])
            return (None if plist is None else skill in plist) is listed

        return test

    def by_priority(card: str, g: int, skill: str, listed: bool | None) -> Stat:
        return deck_test(rep, single_source_is(cal.card_id.get(card)), target_is(g), on_list(skill, listed))

    sasami = [by_priority("[This Might Sting!] Sasami Anshinzawa", 20203, "Risky Business", v) for v in (True, False)]
    king_halo = [by_priority("[Tonight, We Waltz] King Halo", 20159, "Uma Stan", v) for v in (True, False)]
    riko = [by_priority("[Planned Perfection] Riko Kashimoto", 20046, "Lone Wolf", v) for v in (True, False, None)]
    playtime_cards = {cal.card_id.get(c) for c in PO_CARDS}

    def from_playtime_card(r: CalRow) -> bool:
        return one_deck_source(r) and deck_source(r) in playtime_cards

    po = [deck_test(rep, from_playtime_card, target_is(PLAYTIMES_OVER), priority_is(v)) for v in (True, False, None)]
    out += [
        Value(S13, "Playtime's Over! event sources: runs showing a priority list", po[0].rows + po[1].rows),
        Value(S13, "Playtime's Over! event sources: prioritised", po[0].rows),
        Value(S13, "Playtime's Over! event sources: hinted with a list shown", po[0].hits + po[1].hits),
        Value(S13, "Playtime's Over! event sources: runs without a list", po[2].rows),
        Value(S13, "Playtime's Over! event sources: hinted without a list", po[2].hits),
    ]
    return PriorityTests(sasami, king_halo, riko, po, playtime_cards)


def certainty_checks(cal: Calibration, out: list[Value]) -> dict[str, Stat]:
    """section 9's certainty checks: every run carrying the card credited, where it is the skill's one deck source;
    by the card's holder"""
    cert: dict[str, Stat] = {}
    for card, g in (("[From the Ground Up] Light Hello", 20166), ("[Team Sirius] Team Sirius", 20203)):
        holder = card.split("] ")[1]
        cid = cal.card_id.get(card) or next(
            (c["id"] for c in common.gjson("web_cards.json")["cards"] if holder in c["name"]), None
        )
        s = cert[holder] = deck_test(cal.report, single_source_is(cid), target_is(g))
        out += [
            Value(
                S9,
                f"{holder} credited: runs carrying her",
                s.rows,
                note="her the one deck source, no inspiration of it",
            ),
            Value(S9, f"{holder} credited: credited", s.hits),
        ]
    return cert


@dataclass
class Tests:
    """every step's results the findings read"""

    deck: DeckTests
    inheritance: InheritanceTests
    acquisition: AcquisitionTests
    target_inheritance: dict[int, Stat]
    pool: PoolTests
    unity_golds: UnityGolds
    reliability: DeckReliability
    per_run: PerRunTests
    single: SingleSourceTests
    priority: PriorityTests
    certainty: dict[str, Stat]


def computed_values(cal: Calibration, out: list[Value]) -> Tests:
    """every value of the section, in the document's order, appended to out"""
    run_counts(cal, out)
    deck = deck_tests(cal, out)
    inheritance = inheritance_tests(cal, out)
    acquisition = acquisition_tests(cal, out)
    target_inheritance = per_target_tests(cal, acquisition.open_rows, out)
    pool = pool_tests(cal, out)
    golds = gold_tests(cal, out)
    reliability = reliability_tests(cal, out)
    per_run = per_run_tests(cal, out)
    single = single_source_tests(cal, out)
    priority = priority_tests(cal, out)
    certainty = certainty_checks(cal, out)
    return Tests(
        deck,
        inheritance,
        acquisition,
        target_inheritance,
        pool,
        golds,
        reliability,
        per_run,
        single,
        priority,
        certainty,
    )


def findings(cal: Calibration, t: Tests) -> list[Finding]:
    """each conclusion, tested on the numbers above"""

    def tn(g: int) -> str:
        return cal.skill[g][0]

    def bucket_text(b: Bucket) -> str:
        return f"{bucket_label(b)}: {b.observed:.3f} [{b.observed_low:.3f}, {b.observed_high:.3f}] vs {b.predicted:.3f}"

    inh, deck, acq, rel, per_run, single, prio = (
        t.inheritance,
        t.deck,
        t.acquisition,
        t.reliability,
        t.per_run,
        t.single,
        t.priority,
    )
    byc = inh.by_carriers
    t_off = [g for g in DEFAULT if g in t.target_inheritance and not covers(t.target_inheritance[g])]
    half_width = (per_run.all_acquired_high - per_run.all_acquired_low) / 2 / per_run.all_acquired_rate

    def mean_interval(sign: int) -> float:
        return per_run.acquired_mean + sign * 1.96 * per_run.acquired_sd / math.sqrt(len(per_run.runs))

    def playtime_priority() -> bool:
        if prio.playtime[1].rows < 5:
            raise KeyError("too few rows on a list")  # common.finding: "needs data"
        return (prio.playtime[0].rows < 5 or covers(prio.playtime[0])) and below(prio.playtime[1])

    po = prio.playtime
    return [
        # section 3
        finding(
            S3,
            "inspiration odds q are calibrated: observed / predicted over all run-target rows has a 95% interval covering 1",
            lambda: covers(inh.overall),
            lambda: (
                ratio_text(inh.overall)
                + "; per target, intervals excluding 1: "
                + (", ".join(f"{tn(g)} {ratio_text(t.target_inheritance[g])}" for g in t_off) or "none")
            ),
        ),
        finding(
            S3,
            "by carriers 1-6, every interval covers 1",
            lambda: all(k in byc and covers(byc[k]) for k in range(1, 7)),
            lambda: "; ".join(
                f"{k}: {byc[k].ratio:.3f} [{byc[k].ratio_low:.3f}, {byc[k].ratio_high:.3f}]"
                for k in range(1, 7)
                if k in byc
            ),
        ),
        finding(
            S3,
            "per roll, spark-years and ancestor hits each sit within the model's 95% band (|z| < 1.96, z run-clustered)",
            lambda: abs(inh.spark_years.z) < 1.96 and abs(inh.ancestor_hits.z) < 1.96,
            lambda: (
                f"spark-years {inh.spark_years.observed:,} vs {inh.spark_years.expected:,.1f} expected (z {inh.spark_years.z:+.2f}); "
                f"hits {inh.ancestor_hits.observed:,} vs {inh.ancestor_hits.expected:,.1f} (z {inh.ancestor_hits.z:+.2f})"
            ),
        ),
        finding(
            S3,
            "reliability: calibrated jointly over the buckets with a positive prediction (chi2 over the buckets, one degree of "
            "freedom each, nothing fitted, p above 0.05); the buckets outside their own 95% interval are listed",
            lambda: inh.joint.p > 0.05,
            lambda: (
                (
                    "misses: " + "; ".join(f"{bucket_text(b)} ({b.rows:,} rows)" for b in inh.misses)
                    if inh.misses
                    else "no bucket misses"
                )
                + f" (of {sum(1 for b in inh.buckets if b.rows and b.high_edge > ZERO_EDGE)} buckets tested at 95% each; "
                f"q = 0 bucket observed {inh.buckets[0].observed:.4f} over {inh.buckets[0].rows:,} rows); "
                f"chi2 {inh.joint.statistic:.1f} on {inh.joint.buckets} buckets, p {inh.joint.p:.3f}"
            ),
        ),
        finding(
            S3,
            "reliability: the only bucket outside its own 95% interval (as the text names it) is 0.40-0.60",
            lambda: [(b.low_edge, b.high_edge) for b in inh.misses] == [(0.4, 0.6)],
            lambda: "; ".join(bucket_text(b) for b in inh.misses) or "none",
        ),
        finding(
            S13,
            "acquired: prioritised targets slightly above the model and non-prioritised below it (each run-clustered "
            "interval entirely on its side of 1), the two averaging out in the pooled test",
            lambda: acq.prioritised.ratio_low > 1 and acq.not_prioritised.ratio_high < 1,
            lambda: (
                f"prioritised {acq.prioritised.ratio:.3f} [{acq.prioritised.ratio_low:.3f}, {acq.prioritised.ratio_high:.3f}]; "
                f"not prioritised {acq.not_prioritised.ratio:.3f} [{acq.not_prioritised.ratio_low:.3f}, {acq.not_prioritised.ratio_high:.3f}]"
            ),
        ),
        # section 9
        finding(
            S9,
            "the choice rule shows in its consequences: event-only hints of prioritised targets land at the model's rate (interval covers 1), "
            "of non-prioritised targets below it (interval entirely below 1)",
            lambda: covers(deck.events_prioritised) and below(deck.events_not_prioritised),
            lambda: (
                f"prioritised {ratio_text(deck.events_prioritised)}; not prioritised {ratio_text(deck.events_not_prioritised)}"
            ),
        ),
        *[
            finding(
                S9,
                f"{nm} credits her finale skill in every run carrying her (the finale is certain)",
                lambda s=s: s.rows > 0 and s.hits == s.rows,
                lambda s=s: f"{s.hits} of {s.rows} (Wilson 95% low {s.low:.4f})",
            )
            for nm, s in t.certainty.items()
        ],
        # section 13
        finding(
            S13,
            "the composed acquisition odds are calibrated: acquired from any source, observed / predicted, 95% interval covers 1",
            lambda: covers(acq.any_source),
            lambda: (
                ratio_text(acq.any_source)
                + "; reliability buckets whose interval lies off their mean prediction: "
                + "; ".join(
                    f"{bucket_label(b)} {b.observed:.3f} vs {b.predicted:.3f}"
                    for b in rel.acquired
                    if b.rows and b.high_edge > ZERO_EDGE and not b.observed_low <= b.predicted <= b.observed_high
                )
            ),
        ),
        finding(
            S13,
            "inheritance is right: observed / predicted inspiration, 95% interval covers 1; one target of eight, Tail Held High, "
            "has its interval off 1",
            lambda: covers(inh.overall) and t_off == [20161],
            lambda: (
                ratio_text(inh.overall)
                + "; per-target exceptions: "
                + (", ".join(f"{tn(g)} {ratio_text(t.target_inheritance[g])}" for g in t_off) or "none")
            ),
        ),
        finding(
            S13,
            "deck hints read optimistic: the deck test's observed / predicted interval lies entirely below 1",
            lambda: below(deck.overall),
            lambda: ratio_text(deck.overall),
        ),
        finding(
            S13,
            "the deck's shortfall is almost all from the priority assumption: rows with an event source carry at least 80% of the shortfall "
            "(predicted minus observed hints), and on runs showing a priority list event-only hints land at the model's rate when the target "
            "is prioritised (interval covers 1) and below it when not (interval below 1)",
            lambda: (
                rel.shortfall_events / rel.shortfall >= 0.8
                and covers(deck.events_prioritised)
                and below(deck.events_not_prioritised)
            ),
            lambda: (
                f"event-source shortfall {rel.shortfall_events:.1f} of {rel.shortfall:.1f} hints ({rel.shortfall_events / rel.shortfall:.0%}); "
                f"training pool only {ratio_text(deck.pool_only)}; prioritised {ratio_text(deck.events_prioritised)}; "
                f"not prioritised {ratio_text(deck.events_not_prioritised)}; no priority list shown {ratio_text(deck.events_no_list)}"
            ),
        ),
        finding(
            S13,
            "unmodelled scenario and trainee events supply what the deck loses: acquired from any source covers 1, "
            "acquired from the deck or an inspiration lies below 1",
            lambda: covers(acq.any_source) and below(acq.modelled_source),
            lambda: (
                f"any source {ratio_text(acq.any_source)}; deck or inspiration {ratio_text(acq.modelled_source)}; "
                f"rows acquired with no modelled source {len(acq.unexplained)}, {acq.unexplained_events_only} of them credited to events only"
            ),
        ),
        finding(
            S13,
            "Racing Spirit: Stamina has no deck source: no deck card in any run can hint it",
            lambda: (
                any(r["g"] == RACING_SPIRIT_STAMINA for r in cal.rows)
                and not any(r["g"] == RACING_SPIRIT_STAMINA and (r["cards_pool"] or r["cards_ev"]) for r in cal.rows)
            ),
            lambda: (
                f"{sum(1 for r in cal.rows if r['g'] == RACING_SPIRIT_STAMINA and (r['cards_pool'] or r['cards_ev']))} of "
                f"{sum(1 for r in cal.rows if r['g'] == RACING_SPIRIT_STAMINA):,} rows with a deck source"
            ),
        ),
        finding(
            S13,
            "training-pool hints (beta): Grand Concert, Unity Cup and URA Finale each have an interval covering 1",
            lambda: all(covers(t.pool.by_scenario[k]) for k in (3, 2, 1)),
            lambda: "; ".join(f"{common.SCEN[k]} {ratio_text(t.pool.by_scenario[k])}" for k in (3, 2, 1)),
        ),
        finding(
            S13,
            "only Trackblazer's interval excludes 1, its beta over-predicting: its interval lies entirely below 1, the other "
            "three cover 1",
            lambda: below(t.pool.by_scenario[4]) and all(covers(t.pool.by_scenario[k]) for k in (3, 2, 1)),
            lambda: f"Trackblazer {ratio_text(t.pool.by_scenario[4])}",
        ),
        finding(
            S13,
            "training pool by lowest limit break: LB0, LB1 and LB2 each have an interval covering 1",
            lambda: all(covers(s) for s in t.pool.low_limit_breaks),
            lambda: "; ".join(f"LB{lb} {ratio_text(s)}" for lb, s in zip((0, 1, 2), t.pool.low_limit_breaks)),
        ),
        finding(
            S13,
            "Unity Cup's two golds are guaranteed: every Unity Cup run shows both hinted",
            lambda: t.unity_golds.runs > 0 and t.unity_golds.with_both == t.unity_golds.runs,
            lambda: f"{t.unity_golds.with_both} of {t.unity_golds.runs} runs",
        ),
        finding(
            S13,
            "the deck's over-prediction sits in its top buckets, the event rewards: buckets predicted 0.6 and above carry over half the "
            "shortfall, each lies below its mean prediction, and over half their rows have an event source",
            lambda: (
                rel.shortfall_top / rel.shortfall > 0.5
                and rel.top_rows_with_event / rel.top_rows > 0.5
                and all(b.observed_high < b.predicted for b in rel.deck_tested if b.low_edge >= 0.6)
            ),
            lambda: (
                f"top-bucket shortfall {rel.shortfall_top:.1f} of {rel.shortfall:.1f} ({rel.shortfall_top / rel.shortfall:.0%}); "
                f"event-source rows {rel.top_rows_with_event:,} of {rel.top_rows:,} ({rel.top_rows_with_event / rel.top_rows:.0%}); "
                "buckets lying below their mean prediction: "
                + "; ".join(
                    f"{bucket_label(b)} {b.observed:.3f} [{b.observed_low:.3f}, {b.observed_high:.3f}] vs {b.predicted:.3f}"
                    for b in rel.deck_tested
                    if b.observed_high < b.predicted
                )
            ),
        ),
        finding(
            S13,
            "per run, acquisition is calibrated: the all-open-acquired interval covers the product of the model's odds, and the mean "
            "targets acquired per run has a 95% interval covering the expected mean",
            lambda: (
                per_run.all_acquired_low <= per_run.all_acquired_odds_mean <= per_run.all_acquired_high
                and abs(per_run.acquired_mean - per_run.acquired_expected_mean)
                <= 1.96 * per_run.acquired_sd / math.sqrt(len(per_run.runs))
            ),
            lambda: (
                f"all open: {per_run.all_acquired_rate:.4f} [{per_run.all_acquired_low:.4f}, {per_run.all_acquired_high:.4f}] "
                f"vs {per_run.all_acquired_odds_mean:.4f}; targets acquired {per_run.acquired_mean:.3f} "
                f"[{mean_interval(-1):.3f}, {mean_interval(+1):.3f}] vs {per_run.acquired_expected_mean:.3f}"
            ),
        ),
        finding(
            S13,
            "landing P(all eight) is consistent with calibrated: the bootstrap interval covers 1, pooled and in each scenario with a model value",
            lambda: all(x.low <= 1 <= x.high for x in per_run.landing.values()),
            lambda: "; ".join(f"{k} {x.ratio:.3f} [{x.low:.2f}, {x.high:.2f}]" for k, x in per_run.landing.items()),
        ),
        finding(
            S13,
            "too few runs acquired every open target to separate the exact joint from the product: the solver's exact P(all) differs "
            "from the product of its per-target p by less than the relative half-width of the all-acquired interval",
            lambda: abs(per_run.exact_p_all / per_run.product_p_all - 1) < half_width,
            lambda: (
                f"exact / product {per_run.exact_p_all / per_run.product_p_all:.3f} (sums over runs {per_run.exact_p_all:.4f} / "
                f"{per_run.product_p_all:.4f}); all acquired {sum(x.all_acquired for x in per_run.runs)} runs, relative half-width {half_width:.0%}"
            ),
        ),
        finding(
            S13,
            "the single-source table holds every pair of 40+ rows whose ratio interval excludes 1, plus Gold Ship's, and nothing else",
            lambda: (
                bool(single.pairs)
                and not single.off_unlisted
                and single.listed <= single.off_large | {single.gold_ship}
                and single.gold_ship in single.listed
            ),
            lambda: (
                f"{len(single.off)} of {len(single.pairs)} pairs exclude 1; not listed: "
                + (
                    "; ".join(
                        f"{cal.card_name[x.card]} > {tn(x.target)} {x.stat.hits}/{x.stat.rows} vs pred {x.stat.predicted:.3f} = {x.stat.ratio:.2f} "
                        f"[{x.stat.ratio_low:.2f}, {x.stat.ratio_high:.2f}]"
                        for x in sorted(single.off_unlisted, key=lambda x: -x.stat.rows)
                    )
                    or "none"
                )
                + "; listed without excluding 1: "
                + (
                    ", ".join(
                        f"{cal.card_name[c]} > {tn(g)} [{single.of_listed[c, g].ratio_low:.2f}, {single.of_listed[c, g].ratio_high:.2f}]"
                        for c, g in sorted(single.listed - single.off_large)
                    )
                    or "none"
                )
            ),
        ),
        finding(
            S13,
            "below 40 rows more pairs exclude 1; among them, below their prediction, are exactly Playtime's Over!'s three event sources "
            "(Yaeno Muteki, Yukino Bijin, Sirius Symboli), whose pooled hint rate has a 95% interval entirely under their lowest prediction",
            lambda: (
                bool(single.off_small)
                and {x.card for x in single.playtime_small} == prio.playtime_cards
                and wilson(single.playtime_hits, single.playtime_rows)[1] < single.playtime_lowest_predicted
            ),
            lambda: (
                f"{len(single.off_small)} pairs; Playtime's Over! sources hinted {single.playtime_hits}/{single.playtime_rows} "
                f"[{wilson(single.playtime_hits, single.playtime_rows)[0]:.3f}, {wilson(single.playtime_hits, single.playtime_rows)[1]:.3f}] "
                f"vs lowest prediction {single.playtime_lowest_predicted:.3f}; "
                + "; ".join(
                    f"{cal.card_name[x.card]} > {tn(x.target)} {x.stat.hits}/{x.stat.rows} vs pred {x.stat.predicted:.3f} "
                    f"[{x.stat.ratio_low:.2f}, {x.stat.ratio_high:.2f}]"
                    for x in single.off_small
                )
            ),
        ),
        finding(
            S13,
            "the Playtime's Over! miss is the priority assumption (the skill sits on a chain-ending choice 2, taken only when "
            "prioritised): prioritised rows cover 1 or are too few to test (under 5), rows on a list without it lie below 1 "
            '(untestable, "needs data", when fewer than 5 rows are on a list without it)',
            playtime_priority,
            lambda: (
                f"prioritised {po[0].hits}/{po[0].rows} "
                + (ratio_text(po[0]) if po[0].rows else "")
                + f"; on a list, not prioritised "
                f"{po[1].hits}/{po[1].rows} {ratio_text(po[1])}; no list shown {po[2].hits}/{po[2].rows} {ratio_text(po[2])}"
            ),
        ),
        finding(
            S13,
            "the 0.20-0.40 deck bucket, almost all training-pool rows (at least 90%), is within its interval: the interval covers its mean prediction",
            lambda: (
                rel.bucket_20_40.observed_low <= rel.bucket_20_40.predicted <= rel.bucket_20_40.observed_high
                and rel.bucket_20_40_pool_rows / rel.bucket_20_40.rows >= 0.9
            ),
            lambda: (
                f"{rel.bucket_20_40.observed:.3f} [{rel.bucket_20_40.observed_low:.3f}, {rel.bucket_20_40.observed_high:.3f}] "
                f"vs {rel.bucket_20_40.predicted:.3f}; training-pool rows {rel.bucket_20_40_pool_rows:,} of {rel.bucket_20_40.rows:,}"
            ),
        ),
        finding(
            S13,
            "Sasami Anshinzawa [This Might Sting!] > Risky Business: the model assumes Risky Business prioritised: with it on the "
            "priority list her hint lands at the model's rate (interval covers 1), without it below (interval below 1)",
            lambda: covers(prio.sasami[0]) and below(prio.sasami[1]),
            lambda: (
                f"prioritised {ratio_text(prio.sasami[0])}; on a list, not prioritised {ratio_text(prio.sasami[1])}"
            ),
        ),
        finding(
            S13,
            "King Halo [Tonight, We Waltz] > Uma Stan sits on a choice that needs a priority: prioritised covers 1, not prioritised below 1",
            lambda: covers(prio.king_halo[0]) and below(prio.king_halo[1]),
            lambda: (
                f"prioritised {ratio_text(prio.king_halo[0])}; on a list, not prioritised {ratio_text(prio.king_halo[1])}"
            ),
        ),
        finding(
            S13,
            "Riko Kashimoto [Planned Perfection] > Ramp Up: her Lone Wolf choice when prioritised accounts for part: rows with Lone "
            "Wolf prioritised lie below 1, and rows from runs showing no priority list still lie below 1",
            lambda: below(prio.riko[0]) and below(prio.riko[2]),
            lambda: (
                f"Lone Wolf prioritised {ratio_text(prio.riko[0])}; on a list, not prioritised {ratio_text(prio.riko[1])}; "
                f"no list shown {ratio_text(prio.riko[2])}"
            ),
        ),
    ]


def claims(wide: bool | None = None) -> list[Value | Finding]:
    R = common.runs()
    why = "needs data" if R is None else "needs master.mdb" if common.mdb() is None else None
    M = Model()
    cal = calibration(R, M, why)
    values: list[Value] = []
    tests = computed_values(cal, values)
    fs = findings(cal, tests)
    if why:  # nothing computed: only the findings, as "needs data"
        for f in fs:
            f.holds, f.evidence = None, why
        return fs + wide_findings(None, None, why)
    out: list[Value | Finding] = [*values, *fs]
    open(os.path.join(OUTD, "report8.txt"), "w").write("\n".join(cal.report.lines) + "\n")
    if wide is None:
        wide = "--wide" in sys.argv
    out += wide_claims(R, M) if wide else wide_findings(None, None, "not run (--wide)")
    return out


def wide_claims(R: list[Run], M: Model) -> list[Value | Finding]:
    """inheritance and training-pool hints on every other skill a deck card or an ancestor's spark could give"""
    jobdir = os.path.join(OUTD, "jobsW")
    man, files, _ = build_jobs(R, M, True, jobdir)
    res = run_solver(jobdir, files, os.path.join(OUTD, "resW.jsonl"), timeout=2, share=0.3, man=man)
    over = sum(1 for o in res.values() if o.get("timeout"))
    res = {k: v for k, v in res.items() if not v.get("timeout")}
    rows, _, _ = rows_of(R, M, res, man)
    rows = [r for r in rows if r["g"] not in DEFAULT]
    rep = Report(rows)
    s_i = inheritance_test(rep)
    s_p = deck_test(rep, lambda r: r["cards_pool"] and not r["cards_ev"])
    open(os.path.join(OUTD, "reportW.txt"), "w").write(
        f"runs {len({r['run'] for r in rows})}, jobs {len(res)}, over the limit {over}\n"
    )
    sampled = len({m.run_id for m in man if any(f.startswith(f"{m.run_id}_") for f in res)})
    return [
        Value(S13, "other skills: runs sampled", sampled),
        Value(S13, "other skills: jobs over the 2 s limit", over),
        Value(S13, "other skills: inheritance rows", s_i.rows),
        Value(S13, "other skills: inheritance", s_i.ratio),
        Value(S13, "other skills: inheritance 95% low", s_i.ratio_low),
        Value(S13, "other skills: inheritance 95% high", s_i.ratio_high),
        Value(S13, "other skills: training-pool rows", s_p.rows),
        Value(S13, "other skills: training-pool hints", s_p.ratio),
        Value(S13, "other skills: training-pool 95% low", s_p.ratio_low),
        Value(S13, "other skills: training-pool 95% high", s_p.ratio_high),
        Value(S3, "non-target skills: inheritance rows", s_i.rows),
        Value(S3, "non-target skills: inheritance", s_i.ratio),
        Value(S3, "non-target skills: inheritance 95% low", s_i.ratio_low),
        Value(S3, "non-target skills: inheritance 95% high", s_i.ratio_high),
        *wide_findings(s_i, s_p),
    ]


def wide_findings(s_i: Stat | None, s_p: Stat | None, why: str | None = None) -> list[Finding]:
    """the conclusions drawn on the other skills (section 3's non-target rows, section 13's "Other skills"); not tested
    (why) when they were not computed"""
    statements = [
        (
            S3,
            "inspiration odds hold on non-target skills: interval covers 1",
            lambda: covers(s_i),
            lambda: ratio_text(s_i),
        ),
        (
            S13,
            "other skills: inheritance holds beyond the eight targets (interval covers 1)",
            lambda: covers(s_i),
            lambda: ratio_text(s_i),
        ),
        (
            S13,
            "other skills: training-pool hints under-predicted: interval entirely above 1",
            lambda: s_p.ratio_low > 1,
            lambda: ratio_text(s_p),
        ),
    ]
    if why:
        return [Finding(section, statement, None, why) for section, statement, _, _ in statements]
    return [finding(section, statement, test, evidence) for section, statement, test, evidence in statements]


if __name__ == "__main__":
    common.main(claims)
