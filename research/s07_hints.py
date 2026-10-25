"""Section 7 (training hints): reproduces docs/RESEARCH.md section 7 from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it) and master.mdb.

One observation per (run, non-pal deck card); its rows are the card's training-hint pool skills (master.mdb
single_mode_hint_gain), a row hit when the run's hint list names the card's character as a source of the skill. Hint
frequency (effect 18), hint levels per draw (1 + effect 17) and specialty priority (effect 19) are read at the card's
level: floored linear interpolation between milestone levels, plus the unique effect.

Every analysis runs on every record, on three row sets:
- the form study: every limit break, SR and SSR cards (R cards out, as everywhere), the rate without the specialty term (h = base_s (1 + hf/100)):
  it compares forms of the hf rate (Bernoulli rows, P(hit) = 1 - (1 - 1/n)^h), the stat option, repeats, per-scenario
  bases and the per-draw increment, and through the Lv-5 cap (uniform draws over the skills still below Lv 5, h read as
  floor or ceiling; pkg/loopopt/src/deck/hints.rs) on its max-limit-break rows no spark hinted the cap and joint draws;
- the fit: max-limit-break card-runs, rows no spark hinted, R cards out, through the Lv-5 cap, with
  h = base_s (1 + hf/100) (1 + d_s spec/100); its later-runs test fits on the earlier half of the records (by seq) and
  scores the later half;
- tests of the current model: the adopted rate (the same h with the fit's values rounded to 3 decimals, the solver's
  constants: pkg/loopopt/build.rs rounds the same results.json values the same way) on the fit's card-runs, for the
  other covariates (a free level per scenario in both fits), per-card differences, the spark deficit (every row),
  drop-out and priority;
- the same-card test: the clean cards (specialty differs between limit breaks, hint frequency does not), every limit
  break, a base per (card, scenario).
Rate forms, specialty choices, the stat option, SR/pal covariates and a factor per card are also tested held out:
5-fold cross-validation by common.fold(run), each fold scored by the fits on the other four, and (all but the factor per
card) on leave-cards-out folds (a card's rows all in one fold; CARD_FOLD_SEEDS deals). Hint frequency, specialty
priority, rarity and pool size are card properties and a card's rows share its residual, so each held-out difference's
standard error is clustered by run, by card and both ways (two_way_se); findings read the two-way z together with a
wild cluster bootstrap by card (wild_card_p), since the rows rest on few effective cards. The constants'
sampling spread is a bootstrap resampling cards and runs together (bootstrap_constants). The calibration checks of the
fit (the rows no other source hinted, the specialty bands and cells) are scored on held-out predictions; every other
result is in-sample on the pooled runs. Intervals on observed / predicted over rows are run-clustered
(common.cluster_ci) or, where a card-level term or card-level misfit enters, two-way (ratio_two_way); the per-card cells
are tested jointly by a run-clustered Wald chi2 (common.cluster_chi2), the specialty cells by a two-way one
(two_way_chi2). Inspired pool skills: the adopted rate by how many a card-run has (inspired_gradient, with a
placebo), and a removal share fitted with the bases and d's, held out (removal_model). Every test reads every run; the general collection's own numbers (batches
GENERAL_BATCHES) are context where they explain a result.
"""

import collections, functools, math, re, unicodedata
from collections.abc import Callable, Sequence
from dataclasses import dataclass, field
from itertools import pairwise
from typing import Any, NamedTuple
import numpy as np
from scipy.optimize import minimize, minimize_scalar
import common

SEC: str = "7"
MIL: list[int] = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]  # the milestone levels of support_card_effect_table
# master.mdb support effect types
EFFECT_INITIAL_BOND, EFFECT_HINT_LEVELS, EFFECT_HINT_FREQUENCY, EFFECT_SPECIALTY = 14, 17, 18, 19
SCENARIOS: tuple[tuple[int, str], ...] = ((1, "URA Finale"), (2, "Unity Cup"), (3, "Grand Concert"), (4, "Trackblazer"))
NELDER_MEAD: dict[str, float] = {"xatol": 1e-6, "fatol": 1e-7, "maxiter": 40000, "maxfev": 80000}
GENERAL_BATCHES: range = range(18)  # the general collection's batches (later ones: targeted collections)
CARD_ROWS: int = 200  # the rows a card needs for its own cell in the per-card tests
CHI1: float = 1.920729  # chi2(1) 95% / 2: a likelihood gain beyond it is detectable
SAME_CARD_D: tuple[float, float] = (-0.9, 1.5)  # the range searched for the same-card d (1 + d sp/100 stays positive)

Rate = Callable[[np.ndarray], np.ndarray]  # parameters -> h per card-run
Mask = np.ndarray  # a boolean mask over rows


def norm(s: str) -> str:
    return re.sub(r"\s+", " ", unicodedata.normalize("NFKC", s)).strip()


# ---------------------------------------------------------------- game data
@dataclass
class GameData:
    """the master.mdb tables the observations read"""

    level_cap: dict[int, tuple[int, ...]]  # rarity -> the level cap at limit breaks 0-4
    effects: dict[int, dict[int, tuple[int, ...]]]  # effect table -> type -> values at the milestones (-1: none)
    card_info: dict[int, tuple[int, int]]  # card -> (rarity, effect table)
    unique: dict[int, list[tuple[int, int, int]]]  # card -> its unique effects (level, type, value)
    hint_pool: dict[int, list[str]]  # card -> its training-hint pool's skill names
    cards: dict[int, dict[str, Any]]  # web_cards.json by card id


def _game() -> GameData:
    if "g" in common._cache:
        return common._cache["g"]
    db = common.mdb()

    def query(sql: str) -> list[tuple]:
        return db.execute(sql).fetchall()

    level_cap = {
        r[0]: r[1:] for r in query("select rarity, limit_0, limit_1, limit_2, limit_3, limit_4 from support_card_limit")
    }
    effects: collections.defaultdict[int, dict[int, tuple[int, ...]]] = collections.defaultdict(dict)
    for row in query(
        "select id, type, init,"
        + ",".join(f"limit_lv{lv}" for lv in range(5, 55, 5))
        + " from support_card_effect_table"
    ):
        effects[row[0]][row[1]] = row[2:]
    card_info = {i: (rar, et) for i, rar, et in query("select id, rarity, effect_table_id from support_card_data")}
    unique: collections.defaultdict[int, list[tuple[int, int, int]]] = collections.defaultdict(list)
    for cid, lv, t0, v0, t1, v1 in query(
        "select id, lv, type_0, value_0, type_1, value_1 from support_card_unique_effect"
    ):
        unique[cid] += [(lv, t0, v0), (lv, t1, v1)]
    skill_name = dict(query("select [index], text from text_data where category=47"))
    hint_pool: collections.defaultdict[int, list[str]] = collections.defaultdict(list)
    for cid, gain_type, v in query("select support_card_id, hint_gain_type, hint_value_1 from single_mode_hint_gain"):
        if gain_type == 0:
            hint_pool[cid].append(norm(skill_name[v]))
    g = common._cache["g"] = GameData(level_cap, effects, card_info, unique, hint_pool, common.cards())
    return g


def effect(g: GameData, cid: int, typ: int, level: int) -> int:
    """support effect `typ` at `level`: floored linear interpolation between milestones, plus the unique effect once its
    level is reached"""
    vals = g.effects[g.card_info[cid][1]].get(typ)
    x = 0
    if vals:
        pts = [(m, v) for m, v in zip(MIL, vals) if v != -1]
        if pts and level >= pts[0][0]:
            x = pts[-1][1]
            for (x0, y0), (x1, y1) in pairwise(pts):
                if level <= x1:
                    x = y0 + (y1 - y0) * (level - x0) // (x1 - x0)
                    break
    return x + sum(v for lv, t, v in g.unique.get(cid, []) if t == typ and level >= lv)


# ---------------------------------------------------------------- observations
@dataclass
class CardRun:
    """one (run, non-pal deck card) observation; effects read at the card's level"""

    run: int
    seq: int  # the record's order (common.runs)
    scen: int  # scenario (common.SCEN)
    cid: int  # card id
    name: str
    ch: str  # the card's character, as the hint sources name it
    rar: int  # rarity: 1 R, 2 SR, 3 SSR
    lb: int  # limit break
    hf: int  # hint frequency, floored-linear
    hlv: int  # effect 17: hint levels per draw minus 1
    spec: int  # specialty priority
    bond0: int  # initial bond
    hinc: int  # hint levels per draw at maximum limit break (web_cards.json)
    nskill: int  # training-hint pool size
    opts: int  # options a draw chooses from (the pool and the stat option)
    pool: list[str]  # the pool's skill names
    tr: dict[str, tuple[int, tuple[str, ...]]]  # skills the card's character is credited with: (level, sources)
    src_of: dict[str, tuple[str, ...]]  # every hinted skill's sources
    prio: list[str] | None  # the run's priority list; None when not shown
    npal: int  # pals in the deck
    kind: str  # card type (speed, stamina, ...; group)


@dataclass
class Data:
    """build()'s card-runs and pool-skill rows: arrays over rows, and per card-run (row-to-card-run index ci)"""

    obs: list[CardRun]
    ci: np.ndarray  # per row: its card-run
    skn: np.ndarray  # per row: the skill name
    kk: np.ndarray  # per row: draws that hit the skill (the level over the increment); 1 when censored
    kind: np.ndarray  # per row: 1 censored (other sources, or capped at Lv 5: at least kk draws), else 0
    cat: np.ndarray  # per row: the skill's other sources ('Inspiration', 'card event', ... joined; 'none')
    hit: np.ndarray  # per row: 1.0 when the card hinted the skill
    noinsp: np.ndarray  # per row: no spark hinted the skill
    hf: np.ndarray  # per card-run, the CardRun fields of the same names
    spec: np.ndarray
    bond0: np.ndarray
    scen: np.ndarray
    lb: np.ndarray
    rar: np.ndarray
    npal: np.ndarray
    nskill: np.ndarray
    opts: np.ndarray
    hinc: np.ndarray
    inc: np.ndarray  # hint levels per draw at the card's level (1 + hlv)
    ctype: np.ndarray  # card type
    seq: np.ndarray
    N: int  # card-runs
    form_classes: tuple[np.ndarray, np.ndarray] | None = field(default=None)  # form_classes(), once computed


def card_runs(g: GameData) -> list[CardRun]:
    """every record's non-pal deck cards"""
    C = g.cards
    obs: list[CardRun] = []
    for r in common.runs():
        hints = [(norm(h["name"]), h["lv"], tuple(h["src"])) for h in r["hints"]]
        npal = sum(1 for c, _, _ in r["deck"] if C.get(c, {}).get("pal"))
        for cid, lb, nm in r["deck"]:
            if cid not in C or C[cid]["pal"]:
                continue
            ch = common.bare(nm)
            rar = g.card_info[cid][0]
            level = g.level_cap[rar][lb]
            tr = {}
            src_of = {}
            for sk, lv, src in hints:
                src_of[sk] = src
                if ch in src:
                    tr[sk] = (lv, src)
            obs.append(
                CardRun(
                    run=r["id"],
                    seq=r["seq"],
                    scen=r["scenario"],
                    cid=cid,
                    name=nm,
                    ch=ch,
                    rar=rar,
                    lb=lb,
                    hf=effect(g, cid, EFFECT_HINT_FREQUENCY, level),
                    hlv=effect(g, cid, EFFECT_HINT_LEVELS, level),
                    spec=effect(g, cid, EFFECT_SPECIALTY, level),
                    bond0=effect(g, cid, EFFECT_INITIAL_BOND, level),
                    hinc=C[cid]["hinc"],
                    nskill=C[cid]["nskill"],
                    opts=C[cid]["opts"],
                    pool=g.hint_pool[cid],
                    tr=tr,
                    src_of=src_of,
                    prio=r.get("priority"),
                    npal=npal,
                    kind=C[cid]["kind"],
                )
            )
    return obs


def other_sources(x: CardRun, skill: str) -> str:
    """the kinds of the skill's sources other than the card's character, joined ('none' when there are none)"""
    others = [t for t in x.src_of.get(skill, ()) if t != x.ch]
    kinds = {
        "Inspiration"
        if t == "Inspiration"
        else "Events"
        if t == "Events"
        else "Pre-career"
        if t == "Pre-career"
        else "card event"
        if t.endswith(" event")
        else "other card"
        for t in others
    }
    return "+".join(sorted(kinds)) if kinds else "none"


def draws_of(x: CardRun, skill: str) -> tuple[int, int]:
    """(draws, censored) of one pool skill: exact when the card alone hinted it below Lv 5 at a multiple of its
    increment; at least ceil(5 / increment) at Lv 5; at least 1 with another source"""
    if skill not in x.tr:
        return 0, 0
    inc = 1 + x.hlv
    lv, src = x.tr[skill]
    if src == (x.ch,) and lv < 5 and lv % inc == 0:
        return lv // inc, 0
    if src == (x.ch,) and lv == 5:
        return math.ceil(5 / inc), 1
    return 1, 1


def build() -> Data:
    """card-runs and pool-skill rows of every record"""
    key = "obs"
    if key in common._cache:
        return common._cache[key]
    obs = card_runs(_game())
    ci, skn, kk, kind, cat = [], [], [], [], []
    for i, x in enumerate(obs):
        for s in x.pool:
            ci.append(i)
            skn.append(s)
            cat.append(other_sources(x, s))
            draws, censored = draws_of(x, s)
            kk.append(draws)
            kind.append(censored)
    kk_ = np.array(kk)
    cat_ = np.array(cat)

    def per_card_run(k: str, t: type = float) -> np.ndarray:
        return np.array([getattr(x, k) for x in obs], t)

    D = Data(
        obs=obs,
        ci=np.array(ci, int),
        skn=np.array(skn),
        kk=kk_,
        kind=np.array(kind),
        cat=cat_,
        hit=(kk_ >= 1).astype(float),
        noinsp=np.array(["Inspiration" not in c for c in cat]),
        hf=per_card_run("hf"),
        spec=per_card_run("spec"),
        bond0=per_card_run("bond0"),
        scen=per_card_run("scen", int),
        lb=per_card_run("lb", int),
        rar=per_card_run("rar", int),
        npal=per_card_run("npal", int),
        nskill=per_card_run("nskill", int),
        opts=per_card_run("opts", int),
        hinc=per_card_run("hinc", int),
        inc=1 + per_card_run("hlv", int),
        ctype=np.array([x.kind for x in obs]),
        seq=per_card_run("seq", int),
        N=len(obs),
    )
    common._cache[key] = D
    return D


def run_of_rows(D: Data, card_run: np.ndarray) -> np.ndarray:
    """the run of each of the given card-runs"""
    return np.array([x.run for x in D.obs])[card_run]


def cap_draws(hinc: np.ndarray) -> np.ndarray:
    """draws that take a skill to Lv 5 (it then leaves the pool)"""
    return np.ceil(5 / hinc).astype(int)


# ---------------------------------------------------------------- the draw model
@functools.lru_cache(None)
def miss(k: int, n: int, t: int, cap: int, extra: int = 0) -> float:
    """P(none of k given pool skills drawn) in t draws from an initial pool of n skills, each draw uniform over skills still below Lv 5; a skill
    leaves after `cap` draws (pkg/loopopt/src/deck/hints.rs miss_draws). `extra` options that never leave (a stat
    option, in the stat-option test) are drawn alongside; the model has none"""
    if k == 0:
        return 1.0
    if k >= n and not extra:
        return 1.0 if t == 0 else 0.0
    if t <= cap:
        return (1 - k / (n + extra)) ** t
    st = {tuple([n - k] + [0] * cap): 1.0}  # how many of the other skills have taken 0..cap draws
    for _ in range(t):
        nx = collections.defaultdict(float)
        for s, p in st.items():
            avail = n - s[cap] + extra  # a drawn extra option leaves the state as it is
            for j in range(cap):
                if s[j]:
                    u = list(s)
                    u[j] -= 1
                    u[j + 1] += 1
                    nx[tuple(u)] += p * s[j] / avail
            if extra:
                nx[s] += p * extra / avail
        st = nx
    return sum(st.values())


TMAX: int = 12  # the most draws a table covers


def miss_table(k: np.ndarray, n: np.ndarray, cap: np.ndarray, extra: int = 0) -> np.ndarray:
    """rows x (TMAX + 1): miss(k, n, t, cap, extra) for t = 0..TMAX (computed once per distinct (k, n, cap))"""
    u, inv = np.unique(
        np.stack([np.asarray(k), np.asarray(n), np.asarray(cap)], 1).astype(int), axis=0, return_inverse=True
    )
    t = np.array([[miss(int(a), int(b), t, int(c), extra) for t in range(TMAX + 1)] for a, b, c in u]).reshape(
        len(u), TMAX + 1
    )
    return t[inv.ravel()]


def mix(tab: np.ndarray, h: np.ndarray) -> np.ndarray:
    """h read as floor(h) or floor(h) + 1 draws, weighted by the fraction"""
    t0 = np.floor(h).astype(int)
    f = h - t0
    r = np.arange(len(h))
    return (1 - f) * tab[r, t0] + f * tab[r, t0 + 1]


def clip(h: np.ndarray) -> np.ndarray:
    """h capped below TMAX for interpolation between adjacent draw-table entries"""
    return np.minimum(h, TMAX - 1e-6)


def row_ll(p: np.ndarray, y: np.ndarray) -> np.ndarray:
    """each row's Bernoulli log-likelihood"""
    p = np.clip(p, 1e-12, 1 - 1e-12)
    return np.where(y > 0, np.log(p), np.log(1 - p))


# ---------------------------------------------------------------- classes: rows that a fit predicts alike
# A fit's prediction for a row reads only its card-run's covariates (scenario, hf, specialty, pool size, ...), so rows
# with equal covariates share P(hit): a fit sums hits * log p + misses * log(1 - p) over these classes instead of over
# rows (the same log-likelihood up to rounding, many times fewer terms).
def alike(*cols: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
    """rows alike in every column: (class of each row, a row of each class)"""
    _, first, inv = np.unique(
        np.stack([np.asarray(c, float) for c in cols], 1), axis=0, return_index=True, return_inverse=True
    )
    return inv.ravel(), first


def tally(cls: np.ndarray, y: np.ndarray, rows: Mask, ncls: int) -> tuple[np.ndarray, np.ndarray, np.ndarray]:
    """the classes of rows `rows` (a mask): (classes present, hits in each, rows in each)"""
    c = cls[rows]
    n = np.bincount(c, minlength=ncls)
    w = np.nonzero(n)[0]
    return w, np.bincount(c, weights=y[rows], minlength=ncls)[w], n[w].astype(float)


def class_ll(p: np.ndarray, hits: np.ndarray, n: np.ndarray) -> float:
    """Bernoulli log-likelihood of classes with `hits` hits of `n` rows, each row P(hit) = p (clipped as row_ll)"""
    p = np.clip(p, 1e-12, 1 - 1e-12)
    return float(hits @ np.log(p) + (n - hits) @ np.log(1 - p))


def on_rows(w: np.ndarray, v: np.ndarray, cls: np.ndarray, ncls: int) -> np.ndarray:
    """class values v (of classes w) on rows of classes `cls`"""
    full = np.zeros(ncls)
    full[w] = v
    return full[cls]


def best_base(
    tab: np.ndarray,
    mult: np.ndarray,
    hits: np.ndarray,
    n: np.ndarray,
    bounds: tuple[float, float],
    xatol: float,
    clipped: bool = False,
    grid: int = 0,
) -> Any:
    """the base b maximising the classes' likelihood with h = b * mult through the draw table tab (bounded
    minimize_scalar's result: x the base, fun minus the log-likelihood). With `grid` > 0 the search starts from the
    best of `grid` log-spaced bases over the bounds and refines between its neighbours: on wide bounds a bounded search
    alone can settle on a flat stretch (draws beyond what a small pool takes, where P(hit) = 1) far from the optimum."""

    def nll(b: float) -> float:
        h = b * mult
        return -class_ll(1 - mix(tab, clip(h) if clipped else h), hits, n)

    if grid:
        bs = np.geomspace(*bounds, grid)
        k = int(np.argmin([nll(b) for b in bs]))
        bounds = (bs[max(k - 1, 0)], bs[min(k + 1, grid - 1)])
    return minimize_scalar(nll, bounds=bounds, method="bounded", options={"xatol": xatol})


# ---------------------------------------------------------------- held out (5-fold cross-validation by run)
CV_FOLDS: list[int] | None = None  # None scores every fold; a list restricts the folds scored
FoldFn = Callable[[Mask, Mask], dict[str, np.ndarray]]
# Leave-cards-out folds: the cards (sorted by id) dealt to the 5 folds by a permutation of each seed. Every seed is
# reported, since one deal can put several heavily used cards in one fold.
CARD_FOLD_SEEDS: tuple[int, ...] = (1, 2, 3)


def cv_rows(fo: np.ndarray, fold_fn: FoldFn) -> dict[str, np.ndarray]:
    """5-fold cross-validation over the folds fo (one per row): fold_fn(train, test) fits every model on the rows in
    `train` (boolean masks over the rows) and returns {model: log-likelihood of each row in `test`, in row order}.
    Returns {model: held-out log-likelihood per row}"""
    ll: dict[str, np.ndarray] = {}
    for f in range(common.K_FOLDS) if CV_FOLDS is None else CV_FOLDS:
        te = fo == f
        for m, v in fold_fn(~te, te).items():
            ll.setdefault(m, np.zeros(len(fo)))[te] = v
    return ll


def run_folds(run_rows: np.ndarray) -> np.ndarray:
    """each row's fold by its run (common.fold): a run's rows all in one fold"""
    return np.array([common.fold(int(r)) for r in run_rows])


def card_folds(cid_rows: np.ndarray, seed: int) -> np.ndarray:
    """each row's fold by its card: a card's rows all in one fold, the cards dealt by the seed's permutation"""
    cards = np.unique(cid_rows)
    deal = np.random.default_rng(seed).permutation(len(cards)) % common.K_FOLDS
    return deal[np.searchsorted(cards, cid_rows)]


class Units(NamedTuple):
    """each row's run, card and card-run: the units a held-out difference or a ratio is clustered by"""

    run: np.ndarray
    card: np.ndarray
    card_run: np.ndarray


def units_of(D: Data, card_run: np.ndarray) -> Units:
    """the units of rows given by their card-runs"""
    return Units(run_of_rows(D, card_run), np.array([x.cid for x in D.obs])[card_run], card_run)


def unit_se(d: np.ndarray, clusters: np.ndarray) -> float:
    """the standard error of sum d with the rows' units summed first and treated as independent (common.compare_held_out)"""
    return common.compare_held_out(d, np.zeros(len(d)), clusters).se


def two_way_se(d: np.ndarray, u: Units) -> tuple[float, float, float]:
    """the standard error of sum d clustered by run, by card, and both ways (Cameron, Gelbach and Miller 2011: the run
    and card variances added, the card-run variance, counted in both, taken off; never below either one-way variance)"""
    se_run, se_card = unit_se(d, u.run), unit_se(d, u.card)
    v = se_run**2 + se_card**2 - unit_se(d, u.card_run) ** 2
    return se_run, se_card, math.sqrt(max(v, se_run**2, se_card**2))


WILD_REPS: int = 999  # wild cluster bootstrap replicates of a held-out difference
WILD_SEED: int = 5


def effective_cards(card: np.ndarray) -> float:
    """how many equally used cards the rows amount to: 1 / sum of each card's squared share of the rows (a cluster
    standard error behaves as if there were about this many clusters)"""
    _, n = np.unique(card, return_counts=True)
    share = n / n.sum()
    return float(1 / (share**2).sum())


def wild_card_p(d: np.ndarray, card: np.ndarray) -> float:
    """two-sided p of 'sum d = 0' by a wild cluster bootstrap by card: each card's summed difference, centred so the
    null holds, times a random sign (Rademacher); the statistic is the sum over its card-clustered SE (as unit_se). With
    few or uneven cards this is more trustworthy than the normal reference for the same z"""
    _, ix = np.unique(card, return_inverse=True)
    sums = np.bincount(ix, weights=d)
    G = len(sums)

    def t(m: np.ndarray) -> np.ndarray:
        se = np.sqrt(G / (G - 1) * ((m - m.mean(1, keepdims=True)) ** 2).sum(1))
        return m.sum(1) / se

    signs = np.random.default_rng(WILD_SEED).choice([-1.0, 1.0], size=(WILD_REPS, G))
    reps = t(signs * (sums - sums.mean()))
    return float((1 + (np.abs(reps) >= abs(t(sums[None])[0])).sum()) / (WILD_REPS + 1))


def compared(name: str, a: np.ndarray, b: np.ndarray, u: Units) -> list[common.Value]:
    """the Values of a held-out comparison, a (the more complex model) minus b, per row: nats, its SE and z clustered by
    run, z clustered by card with the effective number of cards behind it, SE and z clustered both ways, and the wild
    cluster bootstrap p by card"""
    d = np.asarray(a, float) - np.asarray(b, float)
    s = float(d.sum())
    se_run, se_card, se2 = two_way_se(d, u)
    return [
        V(f"held out: {name}: nats", s),
        V(f"held out: {name}: SE", se_run),
        V(f"held out: {name}: z", s / se_run),
        V(f"held out: {name}: z by card", s / se_card),
        V(f"held out: {name}: effective cards", effective_cards(u.card)),
        V(f"held out: {name}: SE two-way", se2),
        V(f"held out: {name}: z two-way", s / se2),
        V(
            f"held out: {name}: wild p",
            wild_card_p(d, u.card),
            note=f"wild cluster bootstrap by card, {WILD_REPS} reps",
        ),
    ]


def compared_by_card(name: str, by_seed: list[dict[str, np.ndarray]], a: str, b: str, u: Units) -> list[common.Value]:
    """a leave-cards-out comparison of models a and b over every seed's folds: the range of nats and two-way z"""
    nats, zs = [], []
    for ll in by_seed:
        d = ll[a] - ll[b]
        nats.append(float(d.sum()))
        zs.append(nats[-1] / two_way_se(d, u)[2])
    return [
        V(f"cards held out: {name}: nats low", min(nats), note=f"seeds {CARD_FOLD_SEEDS}"),
        V(f"cards held out: {name}: nats high", max(nats)),
        V(f"cards held out: {name}: z low", min(zs), note="two-way clustered"),
        V(f"cards held out: {name}: z high", max(zs)),
    ]


def held_out(
    fold_fn: FoldFn, u: Units, by_card: bool = True
) -> tuple[dict[str, np.ndarray], list[dict[str, np.ndarray]]]:
    """fold_fn's models held out on folds by run, and (by_card) on leave-cards-out folds for every seed"""
    by_run = cv_rows(run_folds(u.run), fold_fn)
    return by_run, [cv_rows(card_folds(u.card, s), fold_fn) for s in CARD_FOLD_SEEDS] if by_card else []


def ratio_two_way(obs: np.ndarray, exp: np.ndarray, u: Units) -> tuple[float, float, float]:
    """sum obs / sum exp and its 95% interval clustered both ways (the ratio estimator's linearised variance,
    common.cluster_ci, by run, by card and by card-run, combined as two_way_se); the low end is not clipped"""
    r = common.cluster_ci(obs, exp, u.run)
    se = [(common.cluster_ci(obs, exp, x).high - r.ratio) / 1.96 for x in (u.run, u.card, u.card_run)]
    se2 = math.sqrt(max(se[0] ** 2 + se[1] ** 2 - se[2] ** 2, se[0] ** 2, se[1] ** 2))
    return r.ratio, r.ratio - 1.96 * se2, r.ratio + 1.96 * se2


def form_classes(D: Data) -> tuple[np.ndarray, np.ndarray]:
    """card-runs alike for every rate of the form study (scenario, hf, pool size): (class of each card-run, a card-run of
    each class)"""
    if D.form_classes is None:
        D.form_classes = alike(D.scen, D.hf, D.nskill)
    return D.form_classes


def fitB(D: Data, hfun: Rate, x0: Sequence[float], rows: Mask | None = None) -> tuple[np.ndarray, float]:
    """the form study's fits: P(row hit) = 1 - (1 - 1/n)^h, h = hfun(theta) per card-run (reading only the scenario and
    hf); Nelder-Mead then L-BFGS-B. Returns (theta, log-likelihood)"""
    ci = D.ci
    cc, rep = form_classes(D)
    rows = np.ones(len(ci), bool) if rows is None else rows
    w, hits, nr = tally(cc[ci], D.hit, rows, len(rep))
    cr = rep[w]
    keep = 1 - 1 / D.nskill[cr].astype(float)

    def nll(th: np.ndarray) -> float:
        h = hfun(th)
        if np.any(h <= 0) or not np.all(np.isfinite(h)):
            return 1e12
        return -class_ll(1 - keep ** h[cr], hits, nr)

    r = minimize(nll, x0, method="Nelder-Mead", options=NELDER_MEAD)
    r2 = minimize(nll, r.x, method="L-BFGS-B")
    b = r2 if r2.fun <= r.fun else r
    return b.x, -b.fun


def fit_cap(D: Data, rows: Mask, cap: bool = True) -> tuple[dict[int, float], float, np.ndarray]:
    """per-scenario base for h = base (1 + hf/100), Bernoulli through the floor/ceiling mixture of
    miss(1, n, t, cap); cap=False draws with replacement. Returns bases, log-likelihood, per-row P(hit)"""
    ci = D.ci
    j = np.nonzero(rows)[0]
    c = ci[j]
    cc, rep = alike(D.scen, D.hf, D.nskill, D.hinc)
    rc = cc[c]
    nc = len(rep)
    capv = cap_draws(D.hinc[rep]) if cap else np.full(nc, 99)
    tab = miss_table(np.ones(nc, int), D.nskill[rep], capv)  # per class
    y = D.hit[j]
    g = D.scen[c]
    bases, ll, p = {}, 0.0, np.zeros(len(j))
    for s in sorted(set(g)):
        q = g == s
        w, hits, nr = tally(rc, y, q, nc)
        tq = tab[w]
        mult = 1 + D.hf[rep[w]] / 100
        r = best_base(tq, mult, hits, nr, bounds=(0.5, 5), xatol=1e-8)
        bases[int(s)] = r.x
        ll -= r.fun
        p[q] = on_rows(w, 1 - mix(tq, r.x * mult), rc[q], nc)
    return bases, ll, p


def B4(D: Data, th: Sequence[float] | np.ndarray) -> np.ndarray:
    """per card-run: its scenario's base among th[:4] (scenarios 1-4)"""
    return np.asarray(th[:4])[D.scen - 1]


class RateForms(NamedTuple):
    """the form study's rates h(theta) per card-run"""

    constant: Rate  # theta_0
    one_base: Rate  # theta_0 (1 + hf/100)
    per_scenario: Rate  # beta_s (1 + hf/100)
    free_divisor: Rate  # beta_s (1 + hf/c), c = theta_4


def rate_forms(D: Data) -> RateForms:
    hf = D.hf
    return RateForms(
        constant=lambda th: th[0] + 0 * hf,
        one_base=lambda th: th[0] * (1 + hf / 100),
        per_scenario=lambda th: B4(D, th) * (1 + hf / 100),
        free_divisor=lambda th: B4(D, th) * (1 + hf / th[4]),
    )


# ---------------------------------------------------------------- values
def V(name: str, got: float | np.number | None, **kw: str) -> common.Value:
    return common.Value(SEC, name, None if got is None else float(got), **kw)


Out = list[common.Value]  # the section's values, appended in order


def add(out: Out, name: str, got: float | np.number | None, **kw: str) -> None:
    out.append(V(name, got, **kw))


class Adopted(NamedTuple):
    """the solver's constants by scenario"""

    base: dict[int, float]  # hint_base
    spec: dict[int, float]  # hint_spec (d)


ADOPTED_DECIMALS: int = 3  # pkg/loopopt/build.rs rounds the fit in results.json to this many decimals


def adopted(fit: PerScenarioD, pooled: PooledFit) -> Adopted:
    """the solver's training-hint constants: the specialty fit's base and d per scenario, rounded as pkg/loopopt/build.rs
    rounds the same values in research/results.json; Grand Masters (5, no runs) takes the pooled fit's"""
    base = {k: round(b, ADOPTED_DECIMALS) for k, b in fit.bases.items()} | {5: round(pooled.base, ADOPTED_DECIMALS)}
    spec = {k: round(d, ADOPTED_DECIMALS) for k, d in fit.d.items()} | {5: round(pooled.d, ADOPTED_DECIMALS)}
    return Adopted(base, spec)


def median_card() -> tuple[float, float, int]:
    """the median hf and the median spec, taken separately, of the runs' SR/SSR non-pal, non-group deck cards at maximum
    limit break, and how many deck cards they are taken over"""
    wc = common.cards()
    deck_cards = [
        wc[c]
        for r in common.runs()
        for c, _, _ in r["deck"]
        if c in wc and wc[c]["rarity"] > 1 and not wc[c]["pal"] and wc[c].get("kind") != "group"
    ]
    return (
        float(np.median([c["hf"] for c in deck_cards])),
        float(np.median([c["spec"] for c in deck_cards])),
        len(deck_cards),
    )


def worked_example(A: Adopted, out: Out) -> None:
    """a worked example using separate median hf and spec values across the runs' SR/SSR non-pal, non-group cards
    at maximum limit break"""
    mhf, msp, n = median_card()
    add(out, "example: median hint frequency", mhf, note=f"{n:,} deck cards in the runs")
    add(out, "example: median specialty priority", msp)
    for s, nm in ((2, "Unity Cup"), (3, "Grand Concert"), (1, "URA Finale"), (4, "Trackblazer")):
        add(
            out,
            f"example: hints per career, median card, {nm}",
            A.base[s] * (1 + mhf / 100) * (1 + A.spec[s] * msp / 100),
        )


# ---------------------------------------------------------------- the form study: every record, every limit break, no specialty term
def sr_ssr(D: Data) -> Mask:
    """the rows of SR and SSR cards: R cards are left out of every measured result"""
    return D.rar[D.ci] != 1


def form_fits(D: Data, forms: RateForms, out: Out) -> None:
    """the per-scenario rate and the free divisor in-sample (SR and SSR rows)"""
    rows = sr_ssr(D)
    ci = D.ci[rows]
    add(out, "form: rows", len(ci))
    add(out, "form: runs", len({D.obs[i].run for i in ci}), note="runs that add a row")
    th4, ll4 = fitB(D, forms.per_scenario, [1.5, 2.6, 2.0, 1.7], rows=rows)
    add(out, "LL per-scenario base(1+hf/100)", ll4)
    thc, llc = fitB(D, forms.free_divisor, [*th4, 100.0], rows=rows)
    add(out, "LL free divisor", llc)
    add(out, "free divisor c", thc[4])


def stat_option_cells(D: Data, out: Out) -> None:
    """skills only: per (card, hf) with >= 15 card-runs whose every row is exact, var(T) against the floor a drawn stat
    option implies (E = mean T (o / n) draws over o = n + 1 options give var >= E (1/o)(1 - 1/o))"""
    ci, hf = D.ci, D.hf
    tot = np.bincount(ci, weights=D.kk, minlength=D.N)
    cens = np.bincount(ci, weights=D.kind, minlength=D.N)
    cid = np.array([x.cid for x in D.obs])
    o = D.opts.astype(float)
    cells = below_n = 0
    for c in sorted(set(cid[D.rar != 1])):  # SR and SSR cards
        for v in sorted(set(hf[cid == c])):
            m = (cid == c) & (hf == v) & (cens == 0)
            if m.sum() < 15:
                continue
            T = tot[m]
            oo = o[m][0]
            cells += oo > 1
            if oo <= 1:
                continue  # an empty pool (SSR Haru Urara) can't be tested
            below_n += T.var() < T.mean() / (oo - 1) * (1 - 1 / oo)
    add(out, "cells var below stat-option floor", below_n)
    add(out, "cells (card, hf)", cells, note="testable cells (an empty pool is left out)")


def repeats(D: Data, forms: RateForms, out: Out) -> np.ndarray:
    """repeats against a two-point and a Poisson count (bases: per-scenario, rows no spark hinted, every limit break);
    returns that rate's h per card-run"""
    ci, hf = D.ci, D.hf
    n = D.nskill.astype(float)
    bs = fitB(D, forms.per_scenario, [1.5, 2.7, 2.0, 1.8], rows=D.noinsp & sr_ssr(D))[0]
    h = B4(D, bs) * (1 + hf / 100)
    inc, kk, kind = D.inc[ci], D.kk, D.kind
    k1 = (kind == 0) & (kk == 1)
    k2 = ((kind == 0) & (kk >= 2)) | ((kind == 1) & (kk >= 2))
    use = (k1 | k2) & (inc <= 4) & sr_ssr(D)
    p = 1 / n[ci]
    hh = h[ci]
    f0 = np.floor(hh)
    fr = hh - f0

    def at_least(t: np.ndarray) -> tuple[np.ndarray, np.ndarray]:
        """P(k >= 1), P(k >= 2) draws of a row's skill in t draws"""
        return 1 - (1 - p) ** t, 1 - (1 - p) ** t - t * p * (1 - p) ** (t - 1)

    (a1, a2), (b1, b2) = at_least(f0), at_least(f0 + 1)
    r_two = ((1 - fr) * a2 + fr * b2) / ((1 - fr) * a1 + fr * b1)
    mu = hh / n[ci]
    r_poi = 1 - mu * np.exp(-mu) / (1 - np.exp(-mu))
    add(out, "repeat rate observed", k2[use].mean())
    add(out, "repeat rate two-point count", r_two[use].mean())
    add(out, "repeat rate Poisson count", r_poi[use].mean())
    return h


def held_out_forms(D: Data, forms: RateForms, out: Out) -> None:
    """the form study's rate forms held out: each fold's fits on the other four folds' rows, every row scored (SR and
    SSR rows; fold masks are over those rows)"""
    J = np.nonzero(sr_ssr(D))[0]
    ci, hit = D.ci[J], D.hit[J]
    n = D.nskill.astype(float)

    def scored(h: np.ndarray, te: Mask) -> np.ndarray:
        return row_ll(1 - (1 - 1 / n[ci][te]) ** h[ci][te], hit[te])

    def on_all_rows(q: Mask) -> Mask:
        full = np.zeros(len(D.ci), bool)
        full[J[q]] = True
        return full

    def fold_form(tr_: Mask, te: Mask) -> dict[str, np.ndarray]:
        tr = on_all_rows(tr_)
        t4 = fitB(D, forms.per_scenario, [1.5, 2.6, 2.0, 1.7], rows=tr)[0]
        return {
            "one base": scored(forms.one_base(fitB(D, forms.one_base, [2.0], rows=tr)[0]), te),
            "per-scenario": scored(forms.per_scenario(t4), te),
            "free divisor": scored(forms.free_divisor(fitB(D, forms.free_divisor, [*t4, 100.0], rows=tr)[0]), te),
            "constant": scored(forms.constant(fitB(D, forms.constant, [2.0], rows=tr)[0]), te),
        }

    u = units_of(D, ci)
    hof, by_card = held_out(fold_form, u)
    add(out, "held out: form study runs", len(np.unique(u.run)))
    for name, a, b in (
        ("free divisor vs 1/100", "free divisor", "per-scenario"),
        ("per-scenario bases vs one base", "per-scenario", "one base"),
        ("per-scenario bases vs constant", "per-scenario", "constant"),
    ):
        out.extend(compared(name, hof[a], hof[b], u))
        out.extend(compared_by_card(name, by_card, a, b, u))


def lv5_cap(D: Data, out: Out) -> Mask:
    """the Lv-5 cap against drawing with replacement (max limit break, rows no spark hinted), and obs / pred of each on
    the rows one draw takes to Lv 5; returns those max-limit-break rows"""
    ci, hit = D.ci, D.hit
    rows = (D.lb[ci] == 4) & D.noinsp & sr_ssr(D)
    _, ll_cap, p_cap = fit_cap(D, rows)
    _, ll_repl, p_repl = fit_cap(D, rows, cap=False)
    add(out, "cap gain, nats", ll_cap - ll_repl)
    j = np.nonzero(rows)[0]
    q = cap_draws(D.hinc[ci[j]]) == 1
    add(out, "cap table 1 draws replacement", hit[j][q].sum() / p_repl[q].sum())
    add(out, "cap table 1 draws cap", hit[j][q].sum() / p_cap[q].sum())
    return rows


def joint_draws(D: Data, rows: Mask, h: np.ndarray, out: Out) -> None:
    """pairs of no-spark pool skills on one max-limit-break card-run (URA, Unity, Grand Concert)"""
    ci, hit = D.ci, D.hit
    n = D.nskill.astype(float)
    m = rows & (D.scen[ci] != 4)
    by = collections.defaultdict(list)
    for r in np.nonzero(m)[0]:
        by[ci[r]].append(r)
    pairs = np.array([(i, a, b) for i, js in by.items() for x, a in enumerate(js) for b in js[x + 1 :]])
    I, a, b = pairs.T
    nI = n[I]
    hI = h[I]
    m1, m2 = (1 - 1 / nI) ** hI, (1 - 2 / nI) ** hI
    add(out, "joint pairs", len(pairs))
    add(out, "P(both) observed", np.mean(hit[a] * hit[b]))
    add(out, "P(both) joint", np.mean(1 - 2 * m1 + m2))
    add(out, "P(both) independent", np.mean((1 - m1) ** 2))


def increment(D: Data, out: Out) -> None:
    """per-draw increment: levels from this card's training alone"""
    ok = total = ok2 = total2 = 0  # every card, and cards with iota >= 2 (with iota = 1 every level is a multiple)
    for x in D.obs:
        if x.rar == 1:
            continue  # R cards out
        for lv, src in x.tr.values():
            if src == (x.ch,):
                hit = (lv % (1 + x.hlv) == 0) or lv == 5
                total += 1
                ok += hit
                if x.hlv >= 1:
                    total2 += 1
                    ok2 += hit
    add(out, "increment: multiples of iota", ok)
    add(out, "increment: single-source levels", total)
    add(out, "increment: multiples of iota, iota 2+", ok2)
    add(out, "increment: single-source levels, iota 2+", total2)


# ---------------------------------------------------------------- tests of the current model
ADOPTED_NOTE: str = "the adopted rate, Lv-5 cap, max limit break, R cards out"


@dataclass
class AdoptedRate:
    """the adopted rate (the solver's constants, specialty term in) through the Lv-5 cap, on max-limit-break card-runs with
    R cards out (the fit's card-runs; spark-hinted rows only where a test reads them)"""

    rows: Mask  # those rows
    jm: np.ndarray  # their indices
    cm: np.ndarray  # their card-runs
    tab: np.ndarray  # their draw tables
    h: np.ndarray  # h per card-run
    p: np.ndarray  # P(hit) per row (nan outside rows)


def adopted_rate(D: Data, A: Adopted) -> AdoptedRate:
    ci, sc, hf = D.ci, D.scen, D.hf
    M = (D.lb[ci] == 4) & (D.rar[ci] != 1)
    jm = np.nonzero(M)[0]
    cm = ci[jm]
    tab = miss_table(np.ones(len(jm), int), D.nskill[cm], cap_draws(D.hinc[cm]))
    bA = np.array([A.base[s] for s in (1, 2, 3, 4)])
    dA = np.array([A.spec[s] for s in (1, 2, 3, 4)])
    h = bA[sc - 1] * (1 + hf / 100) * (1 + dA[sc - 1] * D.spec / 100)  # per card-run
    p = np.full(len(ci), np.nan)
    p[jm] = 1 - mix(tab, clip(h[cm]))
    return AdoptedRate(M, jm, cm, tab, h, p)


class CovFit(NamedTuple):
    theta: np.ndarray  # a level per scenario, then a coefficient per covariate column
    ll: float


@dataclass
class CovariateRows:
    """the adopted rate's rows no spark hinted, for fits of the adopted rate times exp(X theta), a free level per
    scenario in both fits (so a covariate cannot gain by absorbing an offset of the adopted bases)"""

    y: np.ndarray
    tab: np.ndarray
    h: np.ndarray  # the adopted h per row
    scen: np.ndarray  # scenario - 1 per row
    card_run: np.ndarray
    cls: np.ndarray  # rows alike in everything a fit reads
    rep: np.ndarray

    def p(self, th: np.ndarray, X: np.ndarray, q: Mask) -> np.ndarray:
        """P(hit) of rows q (X: the covariates of each row)"""
        return 1 - mix(self.tab[q], clip(self.h[q] * np.exp(th[:4][self.scen[q]] + X[q] @ th[4:])))

    def fit(self, X: np.ndarray, q: Mask | None = None) -> CovFit:
        """the fit on rows q (default every row)"""
        k = X.shape[1]
        q = np.ones(len(self.y), bool) if q is None else q
        w, hits, nr = tally(self.cls, self.y, q, len(self.rep))
        r = self.rep[w]
        tw, hw, sw, Xw = self.tab[r], self.h[r], self.scen[r], X[r]

        def nll(th: np.ndarray) -> float:
            return -class_ll(1 - mix(tw, clip(hw * np.exp(th[:4][sw] + Xw @ th[4:]))), hits, nr)

        r1 = minimize(nll, np.zeros(4 + k), method="Nelder-Mead", options=NELDER_MEAD)
        r2 = minimize(nll, r1.x, method="L-BFGS-B")
        b = r2 if r2.fun <= r1.fun else r1
        return CovFit(b.x, -b.fun)

    def fit_many(self, X: np.ndarray, q: Mask, x0: np.ndarray) -> CovFit:
        """fit() for many columns (a factor per card): L-BFGS-B from x0 on the analytic gradient (the draw table is
        piecewise linear in h, its slope tab[t + 1] - tab[t] between whole draws)"""
        w, hits, nr = tally(self.cls, self.y, q, len(self.rep))
        r = self.rep[w]
        tw, hw = self.tab[r], self.h[r]
        Z = np.concatenate([np.eye(4)[self.scen[r]], X[r]], 1)  # the design: scenario levels, then the columns
        ix = np.arange(len(r))

        def nll(th: np.ndarray) -> tuple[float, np.ndarray]:
            h = hw * np.exp(Z @ th)
            hc = clip(h)
            t0 = np.floor(hc).astype(int)
            slope = tw[ix, t0 + 1] - tw[ix, t0]
            m = np.clip(tw[ix, t0] + (hc - t0) * slope, 1e-12, 1 - 1e-12)  # P(miss)
            ll = hits @ np.log(1 - m) + (nr - hits) @ np.log(m)
            dll_dm = (nr - hits) / m - hits / (1 - m)
            dll_deta = dll_dm * slope * np.where(h < TMAX - 1e-6, h, 0.0)  # dh / d(Z th) = h below the table's end
            return -ll, -(Z.T @ dll_deta)

        b = minimize(nll, x0, jac=True, method="L-BFGS-B", options={"maxiter": 2000})
        return CovFit(b.x, -b.fun)


HELD_OUT_COVARIATES: tuple[str, ...] = ("SR", "pals per deck")  # the in-sample detectable ones, also tested held out


def covariates(D: Data, AR: AdoptedRate, out: Out) -> None:
    hit, sc, n = D.hit, D.scen, D.nskill.astype(float)
    nim = D.noinsp[AR.jm]
    ck = AR.cm[nim]
    TY = ["stamina", "power", "guts", "wit"]  # against speed
    covs = {
        "card type": np.stack([(D.ctype == t).astype(float) for t in TY], 1),
        "SR": (D.rar == 2).astype(float)[:, None],
        "pals per deck": (D.npal - 2.5)[:, None].astype(float),
        "initial bond": (D.bond0 / 10)[:, None],
        "pool size": (n - 8)[:, None],
    }
    sk = sc[AR.cm][nim] - 1
    hk = AR.h[AR.cm][nim]
    cls, rep = alike(sk, hk, D.nskill[ck], D.hinc[ck], *np.concatenate([X[ck] for X in covs.values()], 1).T)
    rows = CovariateRows(hit[AR.jm][nim], AR.tab[nim], hk, sk, ck, cls, rep)
    l0 = rows.fit(np.zeros((len(ck), 0))).ll
    add(out, "covariate rows", nim.sum(), note=ADOPTED_NOTE)
    size = {len(r["deck"]) for r in common.runs()}
    add(out, "cards per deck min", min(size), note="a fixed size makes pals per deck the same covariate as non-pals")
    add(out, "cards per deck max", max(size))
    for nm, X in covs.items():
        f = rows.fit(X[ck])
        factors = np.exp(f.theta[4:])
        add(
            out,
            f"covariate gain {nm}, nats",
            f.ll - l0,
            note=f"{X.shape[1]} parameter(s), factor(s) " + ", ".join(f"{x:.3f}" for x in factors),
        )
        if nm in HELD_OUT_COVARIATES:
            add(out, f"covariate factor {nm}", factors[0])
    # the detectable covariates held out: the adopted rate with a free level per scenario, with and without each
    held = {"none": np.zeros((len(ck), 0))} | {nm: covs[nm][ck] for nm in HELD_OUT_COVARIATES}
    u = units_of(D, ck)
    hoc, by_card = held_out(
        lambda tr, te: {m: row_ll(rows.p(rows.fit(X, tr).theta, X, te), rows.y[te]) for m, X in held.items()}, u
    )
    add(out, "held out: covariate runs", len(np.unique(u.run)))
    for nm in HELD_OUT_COVARIATES:
        out.extend(compared(f"covariate {nm} vs none", hoc[nm], hoc["none"], u))
        out.extend(compared_by_card(f"covariate {nm} vs none", by_card, nm, "none", u))
    card_heterogeneity(D, rows, general_rows(D, ck), out)


def general_rows(D: Data, card_run: np.ndarray) -> Mask:
    """the given card-runs that belong to runs of the general collection"""
    batch = {r["id"]: r["batch"] for r in common.runs()}
    return np.isin([batch[int(r)] for r in run_of_rows(D, card_run)], GENERAL_BATCHES)


def card_heterogeneity(D: Data, rows: CovariateRows, general: Mask, out: Out) -> None:
    """hint rates by card beyond the adopted rate (with a free level per scenario): per-card obs / pred on the cards with
    CARD_ROWS+ rows, tested jointly (run-clustered Wald; the same on the general collection's rows as context), and
    held out a factor per such card against none (whether the cards' differences carry over to unseen runs)"""
    cid = np.array([x.cid for x in D.obs])[rows.card_run]
    run = run_of_rows(D, rows.card_run)
    count = collections.Counter(cid)
    cards = sorted(k for k, v in count.items() if v >= CARD_ROWS)
    pos = {k: i for i, k in enumerate(cards)}
    code = np.array([pos.get(k, -1) for k in cid])
    X = (code[:, None] == np.arange(len(cards))).astype(float)
    cls, rep = alike(rows.cls, code)  # rows alike in what the covariate fits read and in their card
    R = CovariateRows(rows.y, rows.tab, rows.h, rows.scen, rows.card_run, cls, rep)
    none = np.zeros((len(cid), 0))
    every = np.ones(len(cid), bool)
    p = R.p(R.fit(none).theta, none, every)
    O = np.bincount(code + 1, weights=rows.y)[1:]
    E = np.bincount(code + 1, weights=p)[1:]
    add(out, "per-card cards", len(cards), note=f"cards with {CARD_ROWS}+ covariate rows")
    add(out, "per-card obs/pred min", (O / E).min(), note="the adopted rate with a free level per scenario")
    add(out, "per-card obs/pred max", (O / E).max())
    for label, q in (("", every), (", general collection", general)):
        n_ = collections.Counter(cid[q])
        k = q & np.array([n_[c] >= CARD_ROWS for c in cid])
        x2, df, pv = common.cluster_chi2(list(zip(cid[k], rows.y[k] - p[k])), run[k])
        add(out, f"per-card Wald chi2{label}", x2, note="run-clustered, each card's obs - pred")
        add(out, f"per-card Wald df{label}", df)
        add(out, f"per-card Wald p{label}", pv)

    def fold(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        f0 = R.fit(none, tr)
        f1 = R.fit_many(X, tr, np.concatenate([f0.theta, np.zeros(len(cards))]))
        return {
            "none": row_ll(R.p(f0.theta, none, te), R.y[te]),
            "per card": row_ll(R.p(f1.theta, X, te), R.y[te]),
        }

    # held out by run only: a factor fitted to a card cannot be scored on folds that leave the card out
    u = units_of(D, rows.card_run)
    ho, _ = held_out(fold, u, by_card=False)
    out.extend(compared("a factor per card vs none", ho["per card"], ho["none"], u))


def clustered_ratio(D: Data, AR: AdoptedRate, q: Mask) -> tuple[float, float, float]:
    """observed / predicted on rows q and its 95% interval, run-clustered (common.cluster_ci: a run gives several rows)"""
    r, lo, hi, _ = common.cluster_ci(D.hit[q], AR.p[q], run_of_rows(D, D.ci)[q])
    return r, lo, hi


def observed_interval(D: Data, AR: AdoptedRate, q: Mask) -> tuple[float, float]:
    """the observed rate's 95% interval on rows q: the predicted rate times the interval of obs / pred"""
    _, lo, hi = clustered_ratio(D, AR, q)
    e = AR.p[q].mean()
    return lo * e, hi * e


def spark_deficit(D: Data, AR: AdoptedRate, out: Out) -> None:
    """the adopted rate by the other sources of the same skill (every row of those card-runs)"""
    hit = D.hit
    for c, key in (
        ("none", "none"),
        ("inspiration", "Inspiration"),
        ("card event", "card event"),
        ("other card training", "other card"),
    ):
        q = AR.rows & (D.cat == key)
        add(out, f"spark {c} rows", q.sum(), note=ADOPTED_NOTE)
        add(out, f"spark {c} observed", hit[q].mean())
        add(out, f"spark {c} predicted", AR.p[q].mean())
        lo, hi = observed_interval(D, AR, q)
        add(out, f"spark {c} observed 95% low", lo)
        add(out, f"spark {c} observed 95% high", hi)
        if c == "none":
            r, lo, hi = clustered_ratio(D, AR, q)
            add(out, "spark none obs/pred", r)
            add(out, "spark none obs/pred 95% low", lo)
            add(out, "spark none obs/pred 95% high", hi)
        else:
            add(out, f"spark {c} ratio", hit[q].mean() / AR.p[q].mean())
            u = units_of(D, D.ci[q])
            _, lo, hi = ratio_two_way(hit[q], AR.p[q], u)
            add(out, f"spark {c} ratio 95% low", lo, note="two-way clustered")
            add(out, f"spark {c} ratio 95% high", hi)

    # a card event's rows split by whose event could have given the skill: the card's own (alone or among others: the
    # records name "A event", or "A or B event" when several deck cards' events could) or only other cards'
    def own_event(x: CardRun, skill: str) -> bool:
        return any(t.endswith(" event") and x.ch in t[: -len(" event")].split(" or ") for t in x.src_of.get(skill, ()))

    own = np.array([own_event(D.obs[c], sk) for c, sk in zip(D.ci, D.skn)])
    u = units_of(D, D.ci)
    for c, mine in (("own event", True), ("other card event", False)):
        q = AR.rows & (D.cat == "card event") & (own == mine)
        r, lo, hi = ratio_two_way(hit[q], AR.p[q], Units(*(x[q] for x in u)))
        add(out, f"spark {c} rows", q.sum(), note=ADOPTED_NOTE)
        add(out, f"spark {c} obs/pred", r, note="two-way clustered interval")
        add(out, f"spark {c} obs/pred 95% low", lo)
        add(out, f"spark {c} obs/pred 95% high", hi)


def drop_out(D: Data, AR: AdoptedRate, out: Out) -> None:
    """card-runs with exactly two pool skills spark-hinted, their other (no-spark) rows: do the spark-hinted skills leave
    the draws?"""
    ci, NI = D.ci, D.noinsp
    sparked = np.bincount(ci, weights=(~NI).astype(float), minlength=D.N)[ci]
    jj = np.nonzero(NI & AR.rows & (sparked == 2))[0]
    c = ci[jj]
    capj = cap_draws(D.hinc[c])
    hj = clip(AR.h[c])
    keep = 1 - mix(miss_table(np.ones(len(jj), int), D.nskill[c], capj), hj)
    drop = 1 - mix(miss_table(np.ones(len(jj), int), D.nskill[c] - 2, capj), hj)
    y = D.hit[jj]
    u = units_of(D, c)
    add(out, "drop-out test observed", y.mean(), note=f"{len(jj):,} rows; " + ADOPTED_NOTE)
    add(out, "drop-out test no drop-out", keep.mean())
    add(out, "drop-out test drop-out", drop.mean())
    _, lo, hi = ratio_two_way(y, np.ones(len(y)), u)
    add(out, "drop-out test observed 95% low", lo, note="two-way clustered")
    add(out, "drop-out test observed 95% high", hi)
    # the share r of full removal that fits, P = keep + r (drop - keep): r = sum(y - keep) / sum(drop - keep), a ratio
    # of sums with its two-way clustered interval
    r, lo, hi = ratio_two_way(y - keep, drop - keep, u)
    add(out, "drop-out test removal share", r, note="0 keeps both skills, 1 removes them for the whole run")
    add(out, "drop-out test removal share 95% low", lo)
    add(out, "drop-out test removal share 95% high", hi)


INSPIRED_BINS: tuple[tuple[int, int, str], ...] = ((0, 1, "0"), (1, 2, "1"), (2, 3, "2"), (3, 99, "3+"))
PLACEBO_BINS: tuple[tuple[int, int, str], ...] = ((0, 1, "0"), (1, 3, "1-2"), (3, 6, "3-5"), (6, 999, "6+"))


def card_adjusted(y: np.ndarray, p: np.ndarray, card: np.ndarray, scen: np.ndarray) -> np.ndarray:
    """p times its (card, scenario)'s own obs / pred over the given rows: what is left is a departure within the card"""
    key = card.astype(np.int64) * 10 + scen
    _, ix = np.unique(key, return_inverse=True)
    factor = np.bincount(ix, weights=y) / np.bincount(ix, weights=p)
    return p * factor[ix]


def binned(out: Out, label: str, y: np.ndarray, p: np.ndarray, x: np.ndarray, bins: Sequence, u: Units) -> None:
    """obs / pred of rows by bins of x, two-way clustered intervals"""
    for lo_, hi_, nm in bins:
        q = (x >= lo_) & (x < hi_)
        r, lo, hi = ratio_two_way(y[q], p[q], Units(*(a[q] for a in u)))
        add(out, f"{label} {nm} rows", q.sum())
        add(out, f"{label} {nm}", r)
        add(out, f"{label} {nm} 95% low", lo)
        add(out, f"{label} {nm} 95% high", hi)


def inspired_gradient(D: Data, AR: AdoptedRate, out: Out) -> None:
    """the adopted rate on the rows no spark hinted, by how many of the card-run's pool skills an inspiration hinted:
    as is and card-adjusted (card_adjusted); and the placebo, card-runs with none inspired, by how many the run's other
    card-runs have. The count is of inspired pool skills whether or not this card drew them too (a split on that would
    select on the outcome)"""
    q = AR.rows & D.noinsp
    j = np.nonzero(q)[0]
    c = D.ci[j]
    k = inspired_in_pool(D)
    run = run_of_rows(D, np.arange(D.N))
    _, rix = np.unique(run, return_inverse=True)
    others = (np.bincount(rix, weights=k)[rix] - k)[c]  # inspired pool skills on the run's other card-runs
    k = k[c]
    u = units_of(D, c)
    y, p = D.hit[j], AR.p[j]
    adj = card_adjusted(y, p, u.card, D.scen[c])
    binned(out, "inspired in pool", y, p, k, INSPIRED_BINS, u)
    binned(out, "inspired in pool, card-adjusted", y, adj, k, INSPIRED_BINS, u)
    none = k == 0
    binned(
        out,
        "placebo: others inspired, card-adjusted",
        y[none],
        adj[none],
        others[none],
        PLACEBO_BINS,
        Units(*(a[none] for a in u)),
    )


def priority(D: Data, AR: AdoptedRate, out: Out) -> None:
    """prioritised skills against the adopted rate"""
    ci, hit = D.ci, D.hit
    prio = [None if x.prio is None else {norm(s) for s in x.prio} for x in D.obs]
    isp = np.array([prio[ci[r]] is not None and D.skn[r] in prio[ci[r]] for r in range(len(ci))]) & D.noinsp & AR.rows
    add(out, "priority runs", len({x.run for x in D.obs if x.prio is not None}))
    add(out, "priority rows", isp.sum())
    add(out, "priority observed", hit[isp].mean())
    add(out, "priority predicted", AR.p[isp].mean(), note=ADOPTED_NOTE)
    lo, hi = observed_interval(D, AR, isp)
    add(out, "priority observed 95% low", lo)
    add(out, "priority observed 95% high", hi)


def claims() -> list[common.Value | common.Finding]:
    if not common.have_runs() or common.mdb() is None:
        return []
    out: Out = []
    D = build()
    S = SpecialtyRows(D)
    fit = per_scenario_d(S, out)
    A = adopted(fit, pooled_fit(S, out))
    bootstrap_constants(S, out)
    worked_example(A, out)
    forms = rate_forms(D)
    form_fits(D, forms, out)
    stat_option_cells(D, out)
    h_form = repeats(D, forms, out)
    held_out_forms(D, forms, out)
    joint_draws(D, lv5_cap(D, out), h_form, out)
    increment(D, out)
    AR = adopted_rate(D, A)
    covariates(D, AR, out)
    spark_deficit(D, AR, out)
    inspired_gradient(D, AR, out)
    drop_out(D, AR, out)
    out += specialty(D, S, fit)
    priority(D, AR, out)
    return out + findings(out)


def findings(out: list[common.Value]) -> list[common.Finding]:
    """the section's conclusions, each tested on the numbers above. A held-out comparison of a card-level term (hf,
    specialty, rarity, the stat option, a factor per card) is tested on its two-way clustered z (by run and card), and
    where it has them on the leave-cards-out folds of every seed"""
    from scipy.stats import binom, chi2

    g: dict[str, Any] = {c.name: c.got for c in out}

    def F(*a: Any) -> common.Finding:
        return common.finding(SEC, *a)

    fs: list[common.Finding] = []
    lps = g["LL per-scenario base(1+hf/100)"]

    def z2(c: str) -> float:
        """the two-way clustered z of a held-out comparison"""
        return g[f"held out: {c}: z two-way"]

    def passes(c: str) -> bool:
        """a held-out gain is detectable: two-way z > 1.96 and the wild cluster bootstrap by card p < 0.05"""
        return z2(c) > 1.96 and g[f"held out: {c}: wild p"] < 0.05

    def cards(c: str) -> tuple[float, float]:
        """the leave-cards-out z over the seeds: (lowest, highest)"""
        return g[f"cards held out: {c}: z low"], g[f"cards held out: {c}: z high"]

    def hs(c: str) -> str:
        text = "{:+.1f} nats, z = {:.2f} by run, {:.2f} by card ({:.0f} effective cards), {:.2f} two-way, wild p = {:.3f}".format(
            g[f"held out: {c}: nats"],
            g[f"held out: {c}: z"],
            g[f"held out: {c}: z by card"],
            g[f"held out: {c}: effective cards"],
            z2(c),
            g[f"held out: {c}: wild p"],
        )
        if f"cards held out: {c}: z low" in g:
            text += "; cards held out {:+.1f} to {:+.1f} nats, z {:.2f} to {:.2f}".format(
                g[f"cards held out: {c}: nats low"], g[f"cards held out: {c}: nats high"], *cards(c)
            )
        return text

    def ci(c: str) -> tuple[float, float, float]:
        return g[c], g[f"{c} 95% low"], g[f"{c} 95% high"]

    def cis(c: str) -> str:
        return "{:.3f} [{:.3f}, {:.3f}]".format(*ci(c))

    fs.append(
        F(
            "h = beta_s (1 + hf/100) predicts unseen runs better than one constant count (form study, 5-fold held out, "
            "two-way z > 1.96, wild p < 0.05), and unseen cards (leave-cards-out z > 1.96 on every seed)",
            passes("per-scenario bases vs constant") and cards("per-scenario bases vs constant")[0] > 1.96,
            hs("per-scenario bases vs constant") + f" on {g['held out: form study runs']:,.0f} runs",
        )
    )
    gc = g["LL free divisor"] - lps
    fs.append(
        F(
            "Form study (without the specialty term): a free hf divisor passes the nominal in-sample bound (LR gain above "
            "1.92 nats, rows treated as independent) but is not detectable held out once clustered by card: it does not "
            "predict unseen runs better than 1/100 (two-way z <= 1.96) nor unseen cards (leave-cards-out z <= 1.96 on "
            "every seed); 1/100 stays assumed",
            gc > 1.92 and z2("free divisor vs 1/100") <= 1.96 and cards("free divisor vs 1/100")[1] <= 1.96,
            f"c = {g['free divisor c']:.0f}; in-sample +{gc:.1f} nats; held out " + hs("free divisor vs 1/100"),
        )
    )
    k, n = g["cells var below stat-option floor"], g["cells (card, hf)"]
    fs.append(
        F(
            "Consistent with no stat option (descriptive; it assumes a fixed count and keeps only exactly counted card-runs, so it "
            "does not establish the assumption): the draw-count variance is below the floor a drawn stat option implies in "
            "most (card, hf) cells (nominal sign test p < 0.05)",
            binom.sf(k - 1, n, 0.5) < 0.05,
            f"{k:.0f} of {n:.0f} cells, p = {binom.sf(k - 1, n, 0.5):.4f}",
        )
    )
    fs.append(
        F(
            "A stat option drawn with each skill's chance fits worse than none: with no extra parameter, the adopted model "
            "without it predicts unseen runs better (two-way z > 1.96, wild p < 0.05) and unseen cards (leave-cards-out z > 1.96 on every "
            "seed)",
            passes("stat option not drawn vs drawn") and cards("stat option not drawn vs drawn")[0] > 1.96,
            f"in-sample {-g['stat option drawn: in-sample, nats']:+.1f} nats; held out "
            + hs("stat option not drawn vs drawn"),
        )
    )
    o, t2, po = g["repeat rate observed"], g["repeat rate two-point count"], g["repeat rate Poisson count"]
    fs.append(
        F(
            "On the form study's rate (no specialty term, no Lv-5 cap), draws repeat skills as uniform draws with a near-fixed "
            "count do, not as a Poisson count",
            abs(o - t2) < abs(o - po),
            f"repeat rate observed {o:.3f}, two-point {t2:.3f}, Poisson {po:.3f}",
        )
    )
    fs.append(
        F(
            "Per-scenario bases predict unseen runs better than one base (form study, 5-fold held out, two-way z > 1.96, wild p < 0.05), "
            "and unseen cards (leave-cards-out z > 1.96 on every seed)",
            passes("per-scenario bases vs one base") and cards("per-scenario bases vs one base")[0] > 1.96,
            hs("per-scenario bases vs one base"),
        )
    )
    cov = {
        nm: (g[f"covariate gain {nm}, nats"], chi2.sf(2 * g[f"covariate gain {nm}, nats"], k_))
        for nm, k_ in (("card type", 4), ("SR", 1), ("pals per deck", 1), ("initial bond", 1), ("pool size", 1))
    }
    miss_c, none_c = ("SR", "pals per deck"), ("card type", "initial bond", "pool size")
    fs.append(
        F(
            "In-sample, rarity (SR against SSR) and pals per deck are nominally detectable on top of the adopted rate (each "
            "LR p < 0.05, rows treated as independent)",
            all(cov[nm][1] < 0.05 for nm in miss_c),
            "; ".join(f"{nm} +{cov[nm][0]:.1f} nats, p = {cov[nm][1]:.1e}" for nm in miss_c),
        )
    )
    fs.append(
        F(
            "Held out, neither is detectable once clustered by card and run: pals per deck predicts unseen runs better only "
            "with a run-clustered SE (z > 1.96; two-way z <= 1.96, leave-cards-out z <= 1.96 on every seed); rarity does "
            "not either way (two-way z <= 1.96)",
            g["held out: covariate pals per deck vs none: z"] > 1.96
            and z2("covariate pals per deck vs none") <= 1.96
            and cards("covariate pals per deck vs none")[1] <= 1.96
            and z2("covariate SR vs none") <= 1.96,
            "; ".join(f"{nm} " + hs(f"covariate {nm} vs none") for nm in miss_c)
            + f" on {g['held out: covariate runs']:,.0f} runs; factors SR {g['covariate factor SR']:.3f}, pals per deck "
            f"{g['covariate factor pals per deck']:.3f} per pal",
        )
    )
    fs.append(
        F(
            "Known miss (not modelled): hint rates differ by card beyond the adopted rate: per-card obs/pred on the cards with "
            f"{CARD_ROWS}+ rows are jointly inconsistent with it (run-clustered Wald, p < 0.001, on every run and on the general "
            "collection alone), and the cards' differences carry over to unseen runs (a factor per card against none, 5-fold "
            "held out, two-way z > 1.96, wild p < 0.05); which card properties explain them is open",
            g["per-card Wald p"] < 0.001
            and g["per-card Wald p, general collection"] < 0.001
            and passes("a factor per card vs none"),
            f"obs/pred {g['per-card obs/pred min']:.2f}-{g['per-card obs/pred max']:.2f} across {g['per-card cards']:.0f} cards; "
            f"chi2 {g['per-card Wald chi2']:.0f} on {g['per-card Wald df']:.0f}, p = {g['per-card Wald p']:.1e}; general "
            f"collection chi2 {g['per-card Wald chi2, general collection']:.0f} on "
            f"{g['per-card Wald df, general collection']:.0f}, p = {g['per-card Wald p, general collection']:.1e}; held out "
            + hs("a factor per card vs none"),
        )
    )
    fs.append(
        F(
            "Card type, initial bond and pool size add nothing nominally detectable to the adopted rate (each LR p > 0.05)",
            all(cov[nm][1] > 0.05 for nm in none_c),
            "; ".join(f"{nm} +{cov[nm][0]:.1f} nats, p = {cov[nm][1]:.3f}" for nm in none_c),
        )
    )
    fs.append(
        F(
            "The Lv-5 cap (a skill leaves the pool at Lv 5) beats drawing with replacement, with no parameter",
            g["cap gain, nats"] > 1.92,
            f"+{g['cap gain, nats']:.1f} nats on {g['form: runs']:,.0f} runs",
        )
    )
    b, j, i = g["P(both) observed"], g["P(both) joint"], g["P(both) independent"]
    fs.append(
        F(
            "On the form study's rate (no specialty term, continuous count, no Lv-5 cap), two skills of one card's pool are both "
            "hinted as joint draws imply, not as independent ones",
            abs(b - j) < abs(b - i),
            f"P(both) observed {b:.4f}, joint {j:.4f}, independent {i:.4f} ({g['joint pairs']:,.0f} pairs)",
        )
    )
    m, t = g["increment: multiples of iota, iota 2+"], g["increment: single-source levels, iota 2+"]
    fs.append(
        F(
            "Each draw adds the card's hint levels: on cards with iota >= 2 (where the check can fail) at least 99.5% of "
            "single-source levels are multiples of iota, or 5",
            m / t >= 0.995,
            f"{m:,.0f} of {t:,.0f}; every card {g['increment: multiples of iota']:,.0f} of "
            f"{g['increment: single-source levels']:,.0f}",
        )
    )
    _, lo, hi = ci("held out: spark none obs/pred")
    fs.append(
        F(
            "On runs left out of the fit, rows with no other source of the skill read within 2% of the adopted model's "
            "rate (two-way clustered interval inside 0.98-1.02, a stated tolerance)",
            lo >= 0.98 and hi <= 1.02,
            f"held out {cis('held out: spark none obs/pred')}; in-sample {g['spark none obs/pred']:.4f}",
        )
    )
    own, oth = ci("spark own event obs/pred"), ci("spark other card event obs/pred")
    fs.append(
        F(
            "No detectable shift where a card event could also have given the skill: the two-way interval covers 1 both "
            "where the card's own event could have and where only other cards' events could have",
            own[1] <= 1 <= own[2] and oth[1] <= 1 <= oth[2],
            f"own event {cis('spark own event obs/pred')} on {g['spark own event rows']:,.0f} rows; other cards' events "
            f"{cis('spark other card event obs/pred')} on {g['spark other card event rows']:,.0f} rows",
        )
    )
    oc, ins = ci("spark other card training ratio"), ci("spark inspiration ratio")
    fs.append(
        F(
            "Known miss (not modelled): a skill another card's training also hinted is hinted by this card's training less "
            "than predicted (two-way interval of obs/pred entirely below 1), by less than an inspired skill",
            oc[2] < 1 and oc[0] > ins[0],
            f"other card's training {cis('spark other card training ratio')}; inspiration {cis('spark inspiration ratio')}",
        )
    )
    fs.append(
        F(
            "Known miss (not modelled): a skill also hinted by inspiration is hinted by training less than predicted "
            "(two-way interval of obs/pred entirely below 1)",
            ins[2] < 1,
            cis("spark inspiration ratio"),
        )
    )
    _, lo, hi = ci("drop-out test removal share")
    fs.append(
        F(
            "On card-runs where an inspiration hinted two pool skills, the card's other rows are hinted more than with "
            "both skills kept in the draws and less than with both removed for the whole run: the share of full removal "
            "that fits lies strictly between 0 and 1 (two-way 95% interval inside (0, 1)), and its interval excludes the "
            "removal share fitted over every card-run; why is not tested",
            lo > 0 and hi < 1 and not lo <= g["removal model: r"] <= hi,
            f"share {cis('drop-out test removal share')} against r = {g['removal model: r']:.3f}; observed "
            f"{cis('drop-out test observed')}, no drop-out "
            f"{g['drop-out test no drop-out']:.3f}, drop-out {g['drop-out test drop-out']:.3f}",
        )
    )
    grad = [ci(f"inspired in pool, card-adjusted {nm}") for _, _, nm in INSPIRED_BINS]
    fs.append(
        F(
            "Known miss (not modelled): the more of a card's pool skills an inspiration hinted, the more its other pool "
            "skills are hinted than the adopted rate predicts. Card-adjusted (each card and scenario's own obs/pred taken "
            "out), obs/pred rises with every added inspired skill, and the two-way interval at none lies entirely below "
            "the one at two",
            all(a_[0] < b_[0] for a_, b_ in pairwise(grad)) and grad[0][2] < grad[2][1],
            "; ".join(
                f"{nm}: {cis(f'inspired in pool {nm}')}, card-adjusted {cis(f'inspired in pool, card-adjusted {nm}')}"
                for _, _, nm in INSPIRED_BINS
            ),
        )
    )
    placebo = [ci(f"placebo: others inspired, card-adjusted {nm}") for _, _, nm in PLACEBO_BINS]
    base0 = grad[0][0]
    fs.append(
        F(
            "Placebo: on card-runs with no inspired pool skill, the card-adjusted obs/pred does not move with how many "
            "inspired pool skills the run's other cards have (every bin's two-way interval covers the no-inspired average, "
            "and the bins' spread is under a quarter of the rise from none to two inspired skills), so a run-level cause is "
            "not detected",
            all(lo_ <= base0 <= hi_ for _, lo_, hi_ in placebo)
            and max(x[0] for x in placebo) - min(x[0] for x in placebo) < (grad[2][0] - grad[0][0]) / 4,
            "; ".join(f"{nm}: {cis(f'placebo: others inspired, card-adjusted {nm}')}" for _, _, nm in PLACEBO_BINS)
            + f"; no inspired skill {base0:.3f}",
        )
    )
    fs.append(
        F(
            "One removal share r, fitted jointly with a base and d per scenario (a card-run with k inspired pool skills "
            "draws a share r of the way from its full pool to the pool less those k), predicts unseen runs better than "
            "keeping them (two-way z > 1.96, wild p < 0.05) and unseen cards (leave-cards-out z > 1.96 on every seed); "
            "the solver keeps them (r = 0)",
            passes("removal share vs none") and cards("removal share vs none")[0] > 1.96,
            f"r = {g['removal model: r']:.3f}, in-sample +{g['removal model: in-sample gain, nats']:.1f} nats; held out "
            + hs("removal share vs none"),
        )
    )
    keep_b = [g[f"held out: keep model, inspired in pool {nm}"] for _, _, nm in INSPIRED_BINS]
    rem_b = [g[f"held out: removal model, inspired in pool {nm}"] for _, _, nm in INSPIRED_BINS]
    fs.append(
        F(
            "On held-out predictions the removal share narrows the gradient: the spread of obs/pred across the "
            "inspired-skill bins is smaller with it than without",
            max(rem_b) - min(rem_b) < max(keep_b) - min(keep_b),
            "keep " + ", ".join(f"{x:.3f}" for x in keep_b) + "; removal " + ", ".join(f"{x:.3f}" for x in rem_b),
        )
    )
    gs = g["specialty gain per scenario, nats"]
    fs.append(
        F(
            "In-sample, skills are hinted more often by cards with more specialty priority (an association): the four d's are "
            "jointly non-zero against their two-way "
            "bootstrap spread (Wald, 4 df, p < 0.001)",
            g["bootstrap d all zero: p"] < 0.001,
            f"chi2 {g['bootstrap d all zero: chi2']:.1f}, p = {g['bootstrap d all zero: p']:.1e}; in-sample +{gs:.1f} nats "
            f"(nominal LR p = {chi2.sf(2 * gs, 4):.1e}, rows treated as independent)",
        )
    )
    fs.append(
        F(
            "Held out, the specialty term (a d per scenario) predicts unseen runs better than no term (two-way z > 1.96, wild p < 0.05); on "
            "unseen cards its gain is positive on every seed but not detectable on every seed (lowest leave-cards-out "
            "z <= 1.96)",
            passes("specialty term vs none")
            and g["cards held out: specialty term vs none: nats low"] > 0
            and cards("specialty term vs none")[0] <= 1.96,
            hs("specialty term vs none") + f" on {g['held out: specialty runs']:,.0f} runs, "
            f"{g['held out: specialty cards']:,.0f} cards",
        )
    )
    fs.append(
        F(
            "A d per scenario is not shown to beat one d: held out it does not predict detectably better (two-way z <= "
            "1.96; leave-cards-out z <= 1.96 on every seed), though in-sample the four d's differ by more than their two-way "
            "bootstrap spread (equal-d Wald, 3 df, p < 0.05)",
            z2("per-scenario d vs one d") <= 1.96
            and cards("per-scenario d vs one d")[1] <= 1.96
            and g["bootstrap equal d: p"] < 0.05,
            hs("per-scenario d vs one d")
            + f"; equal-d chi2 {g['bootstrap equal d: chi2']:.1f}, p = {g['bootstrap equal d: p']:.3f}; one d against none "
            + hs("one d vs none"),
        )
    )
    bands = [b_ for _, _, b_ in SPEC_BANDS]
    after = [g[f"held out: specialty band {b_} after"] for b_ in bands]
    fs.append(
        F(
            "On runs left out of the fit, without the term the specialty-0 band reads below the model and the 60+ band "
            "above it (two-way intervals excluding 1); with the term every band reads within 0.98-1.02 (a stated tolerance)",
            g["held out: specialty band 0 before 95% high"] < 1
            and g["held out: specialty band 60+ before 95% low"] > 1
            and all(0.98 <= a_ <= 1.02 for a_ in after),
            "; ".join(
                f"{b_} {cis(f'held out: specialty band {b_} before')} -> {cis(f'held out: specialty band {b_} after')}"
                for b_ in bands
            ),
        )
    )
    seeds = len(CARD_FOLD_SEEDS)

    def covering(c: str) -> float:
        return g[f"cards held out: {c} seeds covering 1"]

    fs.append(
        F(
            "On unseen cards (leave-cards-out predictions), without the term the specialty-0 band reads below the model "
            "and the 60+ band above it (no seed's two-way interval covers 1); with the term every band's interval covers 1 "
            "on every seed, and so do the rows with no other source of the skill",
            covering("specialty band 0 before") == 0
            and covering("specialty band 60+ before") == 0
            and all(covering(f"specialty band {b_} after") == seeds for b_ in bands)
            and covering("spark none obs/pred") == seeds,
            "; ".join(
                f"{b_} {g[f'cards held out: specialty band {b_} before low']:.3f}-"
                f"{g[f'cards held out: specialty band {b_} before high']:.3f} -> "
                f"{g[f'cards held out: specialty band {b_} after low']:.3f}-"
                f"{g[f'cards held out: specialty band {b_} after high']:.3f}"
                for b_ in bands
            )
            + f"; no other source {g['cards held out: spark none obs/pred low']:.3f}-"
            f"{g['cards held out: spark none obs/pred high']:.3f}",
        )
    )
    fs.append(
        F(
            "With the term, the (scenario, band) cells with 200+ rows show no detectable misfit on held-out predictions "
            "(two-way clustered Wald on the cell count, p > 0.05)",
            g["held out: specialty cells p"] > 0.05,
            f"chi2 {g['held out: specialty cells chi2']:.1f} on {g['held out: specialty cells df']:.0f}, p = "
            f"{g['held out: specialty cells p']:.3f}; clustered by run alone chi2 "
            f"{g['held out: specialty cells chi2 by run']:.1f}, p = {g['held out: specialty cells p by run']:.3f}",
        )
    )
    fs.append(
        F(
            "The specialty effect is not an artefact of one card, one rarity or one period: one scale k on the adopted per-scenario "
            "d's (k = 1 on every row), refitted without each of the eight most-used cards, on SR and SSR alone and on each half "
            "of the records, stays positive and within a factor 2 everywhere (largest k under twice the smallest)",
            g["specialty scale k robustness min"] > 0
            and g["specialty scale k robustness max"] < 2 * g["specialty scale k robustness min"],
            f"k {g['specialty scale k robustness min']:.2f}-{g['specialty scale k robustness max']:.2f}",
        )
    )
    gd = g["specialty model: free divisor gain, nats"]
    fs.append(
        F(
            "On the adopted model (a base and d per scenario, the specialty fit's rows) a free hf divisor passes the nominal "
            "in-sample bound (LR gain above 1.92 nats, rows treated as independent) but is not detectable held out: it does "
            "not predict unseen runs better than 1/100 (two-way z <= 1.96) nor unseen cards (leave-cards-out z <= 1.96 on "
            "every seed); 1/100 stays assumed",
            gd > 1.92
            and z2("specialty model, free divisor vs 1/100") <= 1.96
            and cards("specialty model, free divisor vs 1/100")[1] <= 1.96,
            f"c = {g['specialty model: free divisor c']:.0f}; in-sample +{gd:.2f} nats, LR p = {chi2.sf(2 * gd, 1):.3f}; held out "
            + hs("specialty model, free divisor vs 1/100"),
        )
    )
    fs.append(
        F(
            "Later runs: fitted on the earlier half of the records, the specialty term predicts the later half better than no term "
            "(a test of drift over time; the cards are largely shared between the halves) "
            "(two-way z > 1.96, wild p < 0.05; the gain smaller than in-sample)",
            passes("later runs: specialty term vs none")
            and g["specialty held-out gain, nats"] < g["specialty gain per scenario, nats"],
            "held out "
            + hs("later runs: specialty term vs none")
            + f" on {g['specialty held-out rows']:,.0f} rows; in-sample +{g['specialty gain per scenario, nats']:.1f}",
        )
    )
    lab = "specialty same-card clean"
    lo, hi = g[f"{lab}: d 95% low"], g[f"{lab}: d 95% high"]
    clo, chi = g[f"{lab}: d 95% low by card"], g[f"{lab}: d 95% high by card"]
    d_one = g["one d with a base per scenario: d"]
    big = max(g["specialty d URA Finale"], g["specialty d Unity Cup"])
    fs.append(
        F(
            "Not shown to be causal: within the same card across limit breaks, on the clean cards (hint frequency the same at "
            "every limit break, so only the specialty term changes), d is not told apart from 0: both its profile interval "
            "(rows independent) and its card-clustered interval cover 0 and the one-d fit's d; the card-clustered interval "
            "excludes the larger of URA Finale's and Unity Cup's d",
            lo <= 0 <= hi and clo <= 0 <= chi and clo <= d_one <= chi and lo <= d_one <= hi and chi < big,
            f"clean cards d {g[f'{lab}: d']:.2f}, profile [{lo:.2f}, {hi:.2f}], by card [{clo:.2f}, {chi:.2f}] on "
            f"{g[f'{lab}: cards']:.0f} cards ({g[f'{lab}: row share grand concert']:.0%} of rows Grand Concert), "
            f"{g[f'{lab}: gain, nats']:+.2f} nats; one d {d_one:.2f}; per-scenario d "
            + ", ".join(f"{g[f'specialty d {nm}']:.2f}" for _, nm in SCENARIOS),
        )
    )
    rows, ob, pr = g["priority rows"], g["priority observed"], g["priority predicted"]
    lo, hi = g["priority observed 95% low"], g["priority observed 95% high"]
    fs.append(
        F(
            "No evidence that prioritising a skill raises its training hints (the observed rate is not above the prediction)",
            ob <= pr or lo <= pr,
            f"{ob:.3f} [{lo:.3f}, {hi:.3f}] vs {pr:.3f} ({rows:.0f} rows); a weak test",
        )
    )
    return fs


# ---------------------------------------------------------------- specialty priority
def inspired_in_pool(D: Data) -> np.ndarray:
    """per card-run: how many of the card's pool skills an inspiration hinted (whether or not the card hinted them too)"""
    return np.bincount(D.ci, weights=(~D.noinsp).astype(float), minlength=D.N).astype(int)


class SpecialtyRows:
    """the specialty fit's rows (max-limit-break card-runs, rows no spark hinted, Lv-5 cap, R cards out) and its fits:
    h = base_s m, a base per scenario on top of a multiplier m per row (of hf and specialty)"""

    def __init__(self, A: Data, stat_option: bool = False, removal: bool = False) -> None:
        """stat_option: every draw may also pick the card's stat option (the stat-option test; the model has none).
        removal: rows are also told apart by how many of their card-run's pool skills an inspiration hinted (k), and
        set_removal(r) draws each row as a share r of the way from its full pool to the pool less those k skills (the
        removal test; the model keeps them, r = 0)"""
        ci = A.ci
        rows = (A.lb[ci] == 4) & A.noinsp & (A.rar[ci] != 1)
        self.j = j = np.nonzero(rows)[0]
        self.c = c = ci[j]  # the rows, their card-runs
        self.tab = miss_table(np.ones(len(j), int), A.nskill[c], cap_draws(A.hinc[c]), int(stat_option))
        self.y = A.hit[j]
        self.hfm = 1 + A.hf[c] / 100
        self.sp = A.spec[c]
        self.sc = A.scen[c]
        self.hfr = A.hf[c]
        self.cid = np.array([x.cid for x in A.obs])[c]
        self.seq = A.seq[c]
        self.rar = A.rar[c]
        self.run = np.array([A.obs[i].run for i in c])
        self.units = Units(self.run, self.cid, c)
        # every multiplier below is a function of hf and specialty: rows alike in those, the pool size and the increment
        # (the draw table) are predicted alike
        self.inspired = inspired_in_pool(A)[c]
        extra = (self.inspired,) if removal else ()
        self.rc, self.rep = alike(A.hf[c], self.sp, A.nskill[c], A.hinc[c], *extra)
        self.tabc = self.tab[self.rep]
        if removal:
            self.tab_keep = self.tab
            self.tab_drop = miss_table(np.ones(len(j), int), A.nskill[c] - self.inspired, cap_draws(A.hinc[c]))
        self.tallies: dict[bytes, tuple[np.ndarray, np.ndarray, np.ndarray]] = {}
        self.every = np.ones(len(self.y), bool)

    def set_removal(self, r: float) -> None:
        """draw tables a share r of the way from keeping the inspired pool skills to removing them for the whole run
        (the miss chance is linear in the table, so the mixture of the two tables is the mixture of the predictions)"""
        self.tab = self.tab_keep + r * (self.tab_drop - self.tab_keep)
        self.tabc = self.tab[self.rep]

    def ll_removal(self, r: float, q: Mask | None = None) -> float:
        """LL of rows q with removal share r, a base and d per scenario fitted on them"""
        self.set_removal(r)
        q = self.every if q is None else q
        return sum(
            self.fit(q & (self.sc == k), self.with_d(self.d_of(q & (self.sc == k))))[0]
            for k in (1, 2, 3, 4)
            if (q & (self.sc == k)).any()
        )

    def removal_of(self, q: Mask | None = None) -> float:
        """the removal share fitted jointly with a base and d per scenario on rows q"""
        return minimize_scalar(
            lambda r: -self.ll_removal(r, q), bounds=(-1.0, 2.0), method="bounded", options={"xatol": 1e-3}
        ).x

    def fit(self, q: Mask, m: np.ndarray, rows: bool = False) -> tuple[float, float, np.ndarray | None]:
        """one scenario's base for multiplier m: (LL, base, P(hit) per row of q if `rows`, else None)"""
        key = q.tobytes()
        if key not in self.tallies:
            self.tallies[key] = tally(self.rc, self.y, q, len(self.rep))
        w, hits, nr = self.tallies[key]
        tq = self.tabc[w]
        mq = m[self.rep[w]]
        b = best_base(tq, mq, hits, nr, bounds=(0.3, 6), xatol=1e-7, clipped=True).x
        ll = class_ll(1 - mix(tq, np.minimum(b * mq, TMAX - 1e-6)), hits, nr)
        return (
            ll,
            b,
            on_rows(w, 1 - mix(tq, np.minimum(b * mq, TMAX - 1e-6)), self.rc[q], len(self.rep)) if rows else None,
        )

    def with_d(self, d: float, base: np.ndarray | None = None) -> np.ndarray:
        """the multiplier base (default 1 + hf/100) times the specialty term (1 + d sp/100)"""
        return (self.hfm if base is None else base) * (1 + d * self.sp / 100)

    def total(self, m: np.ndarray, q: Mask | None = None) -> float:
        """LL with a base per scenario"""
        q = self.every if q is None else q
        return sum(self.fit(q & (self.sc == k), m)[0] for k in (1, 2, 3, 4) if (q & (self.sc == k)).any())

    def pooled_d(self, q: Mask | None = None) -> tuple[float, float]:
        """one d over the scenarios, a base per scenario: (d, LL gain over no term)"""
        r = minimize_scalar(
            lambda d: -self.total(self.with_d(d), q), bounds=(-0.5, 1.5), method="bounded", options={"xatol": 1e-4}
        )
        return r.x, -r.fun - self.total(self.hfm, q)

    def d_of(self, q: Mask, xatol: float = 1e-5, base: np.ndarray | None = None) -> float:
        """the d of rows q (one scenario) on top of multiplier base"""
        return minimize_scalar(
            lambda d: -self.fit(q, self.with_d(d, base))[0],
            bounds=(-0.5, 1.5),
            method="bounded",
            options={"xatol": xatol},
        ).x

    def ll_ds(self, q: Mask, base: np.ndarray) -> float:
        """LL with a base and d per scenario on top of multiplier `base`"""
        sc = self.sc
        return sum(
            self.fit(q & (sc == k), self.with_d(self.d_of(q & (sc == k), base=base), base))[0]
            for k in (1, 2, 3, 4)
            if (q & (sc == k)).any()
        )

    def slope_of(self, q: Mask) -> float:
        """the adopted model with a free hf divisor: beta_s (1 + hf/c) (1 + d_s sp/100), one c, a base and d per
        scenario; fitted as the slope 1/c in [0, 0.05] (c >= 20), each slope's bases and d's profiled out"""
        return minimize_scalar(
            lambda s: -self.ll_ds(q, 1 + s * self.hfr), bounds=(0, 0.05), method="bounded", options={"xatol": 1e-6}
        ).x

    def predicted(self, qa: Mask, qb: Mask, m: np.ndarray) -> np.ndarray:
        """rows qb's P(hit) under the base fitted on rows qa for multiplier m"""
        return 1 - mix(self.tab[qb], np.minimum(self.fit(qa, m)[1] * m[qb], TMAX - 1e-6))

    def scored(self, qa: Mask, qb: Mask, m: np.ndarray) -> np.ndarray:
        """rows qb's log-likelihoods under the base fitted on rows qa for multiplier m"""
        return row_ll(self.predicted(qa, qb, m), self.y[qb])


SPEC_BANDS: tuple[tuple[int, int, str], ...] = ((0, 1, "0"), (1, 30, "1-29"), (30, 60, "30-59"), (60, 1000, "60+"))


def band_of(sp: np.ndarray) -> np.ndarray:
    """each row's specialty band name (SPEC_BANDS)"""
    names = np.empty(len(sp), object)
    for a, b, nm in SPEC_BANDS:
        names[(sp >= a) & (sp < b)] = nm
    return names


class PerScenarioD(NamedTuple):
    d: dict[int, float]
    bases: dict[int, float]
    p_without: np.ndarray  # P(hit) per row, no term
    p_with: np.ndarray  # P(hit) per row, a d per scenario
    gain: float
    ll: list[float]  # LL per scenario with the term


def per_scenario_d(S: SpecialtyRows, out: Out) -> PerScenarioD:
    d_s, p0, p1, bases, l1s = {}, np.zeros(len(S.y)), np.zeros(len(S.y)), {}, []
    gain_s = 0.0
    for k in (1, 2, 3, 4):
        q = S.sc == k
        d = S.d_of(q)
        l1, bases[k], p1[q] = S.fit(q, S.with_d(d), True)
        l0, _, p0[q] = S.fit(q, S.hfm, True)
        d_s[k] = d
        gain_s += l1 - l0
        l1s.append(l1)
    add(out, "specialty gain per scenario, nats", gain_s)
    for k, nm in SCENARIOS:
        add(out, f"specialty d {nm}", d_s[k])
        add(out, f"specialty base {nm}", bases[k])
    # Grand Masters has no runs: the scenarios' own spread bounds what its constants could be
    add(out, "scenario spread: base min", min(bases.values()), note="across the four scenarios' fits")
    add(out, "scenario spread: base max", max(bases.values()))
    add(out, "scenario spread: d min", min(d_s.values()))
    add(out, "scenario spread: d max", max(d_s.values()))
    return PerScenarioD(d_s, bases, p0, p1, gain_s, l1s)


class PooledFit(NamedTuple):
    """one base and one d over every scenario's rows"""

    base: float
    d: float


def pooled_fit(S: SpecialtyRows, out: Out) -> PooledFit:
    """one base and one d over every scenario's rows: Grand Masters' constants (it has no runs)"""
    d = S.d_of(S.every)
    fit = PooledFit(S.fit(S.every, S.with_d(d))[1], d)
    add(out, "one base and one d: base", fit.base, note="Grand Masters' hint_base (no runs)")
    add(out, "one base and one d: d", fit.d, note="Grand Masters' hint_spec")
    return fit


BOOT_REPS: int = 500  # cluster-bootstrap replicates of the fitted constants
BOOT_SEED: int = 11


def weighted_fit(S: SpecialtyRows, w: np.ndarray, groups: Sequence[Mask]) -> tuple[list[float], float]:
    """a base per group of rows and one d over them, each row counted w times: (bases, d)"""
    nc = len(S.rep)
    tallies = []
    for q in groups:
        hits = np.bincount(S.rc[q], weights=(S.y * w)[q], minlength=nc)
        n = np.bincount(S.rc[q], weights=w[q], minlength=nc)
        cl = np.nonzero(n)[0]
        tallies.append((cl, hits[cl], n[cl]))

    def best(d: float) -> list[Any]:
        m = S.with_d(d)
        return [
            best_base(S.tabc[cl], m[S.rep[cl]], hits, n, bounds=(0.3, 6), xatol=1e-6, clipped=True)
            for cl, hits, n in tallies
        ]

    d = minimize_scalar(
        lambda d: sum(r.fun for r in best(d)), bounds=(-0.5, 1.5), method="bounded", options={"xatol": 1e-4}
    ).x
    return [r.x for r in best(d)], d


def bootstrap_constants(S: SpecialtyRows, out: Out) -> None:
    """the constants' sampling spread: BOOT_REPS replicates resampling cards and runs together (each row counted its
    card's times its run's draw count, a two-way 'pigeonhole' bootstrap, Owen 2007, which keeps both clusterings'
    correlations). Per scenario the 95% percentile intervals of base, d and the median card's hints per career, and
    the correlation of base and d; the pooled (Grand Masters) base and d; one d with a base per scenario; and Wald
    tests on the replicates' covariance that the four d's are all zero and that they are equal"""
    from scipy.stats import chi2

    rng = np.random.default_rng(BOOT_SEED)
    cards, cix = np.unique(S.cid, return_inverse=True)
    runs, rix = np.unique(S.run, return_inverse=True)
    mhf, msp, _ = median_card()
    scen = [S.sc == k for k in (1, 2, 3, 4)]
    reps = []
    for _ in range(BOOT_REPS):
        mc = np.bincount(rng.integers(0, len(cards), len(cards)), minlength=len(cards))
        mr = np.bincount(rng.integers(0, len(runs), len(runs)), minlength=len(runs))
        w = (mc[cix] * mr[rix]).astype(float)
        per = [weighted_fit(S, w, [q & (w > 0)]) for q in scen]
        pooled = weighted_fit(S, w, [w > 0])
        one_d = weighted_fit(S, w, [q & (w > 0) for q in scen])[1]
        reps.append([v for bases, d in per for v in (bases[0], d)] + [pooled[0][0], pooled[1], one_d])
    R = np.array(reps)
    add(out, "bootstrap replicates", BOOT_REPS, note="cards and runs resampled together")

    def interval(label: str, x: np.ndarray) -> None:
        lo, hi = np.percentile(x, [2.5, 97.5])
        add(out, f"bootstrap {label} 95% low", lo)
        add(out, f"bootstrap {label} 95% high", hi)

    for i, (_, nm) in enumerate(SCENARIOS):
        base, d = R[:, 2 * i], R[:, 2 * i + 1]
        interval(f"base {nm}", base)
        interval(f"d {nm}", d)
        interval(f"median-card hints {nm}", base * (1 + mhf / 100) * (1 + d * msp / 100))
        add(out, f"bootstrap corr(base, d) {nm}", np.corrcoef(base, d)[0, 1])
    interval("pooled base", R[:, 8])
    interval("pooled d", R[:, 9])
    d_one = S.pooled_d()[0]
    add(out, "one d with a base per scenario: d", d_one)
    interval("one d", R[:, 10])
    add(out, "bootstrap one d: z", d_one / R[:, 10].std(ddof=1), note="d over its replicates' SD")
    d_hat = np.array([S.d_of(q) for q in scen])
    cov = np.cov(R[:, [1, 3, 5, 7]].T)
    x2 = float(d_hat @ np.linalg.solve(cov, d_hat))
    add(out, "bootstrap d all zero: chi2", x2, note="Wald, 4 df, the replicates' covariance")
    add(out, "bootstrap d all zero: p", chi2.sf(x2, 4))
    C = np.hstack([np.eye(3), -np.ones((3, 1))])  # d_1 - d_4, d_2 - d_4, d_3 - d_4: any full set of contrasts
    x2 = float(C @ d_hat @ np.linalg.solve(C @ cov @ C.T, C @ d_hat))
    add(out, "bootstrap equal d: chi2", x2, note="Wald, 3 df, the replicates' covariance")
    add(out, "bootstrap equal d: p", chi2.sf(x2, 3))


def specialty_residuals(S: SpecialtyRows, fit: PerScenarioD, out: Out) -> None:
    """obs / pred by specialty band before and after the term, in-sample (the held-out test is held_out_calibration)"""
    y, sp = S.y, S.sp
    for a, b, nm in SPEC_BANDS:
        q = (sp >= a) & (sp < b)
        add(out, f"specialty band {nm} rows", q.sum())
        add(out, f"specialty band {nm} before", y[q].sum() / fit.p_without[q].sum())
        add(out, f"specialty band {nm} after", y[q].sum() / fit.p_with[q].sum())


def specialty(A: Data, S: SpecialtyRows, fit: PerScenarioD) -> list[common.Value]:
    """specialty priority (effect 19, at the card's level) on the fit's setup over every record (S, its per-scenario fit
    `fit`): max-limit-break card-runs, rows no spark hinted, Lv-5 cap, R cards out. h = base_s (1 + hf/100) (1 + d
    spec/100)."""
    sc = S.sc
    out: Out = []
    add(out, "specialty rows", len(S.j))
    add(out, "specialty card-runs", len(set(S.c)))
    for k, nm in SCENARIOS:
        add(out, f"specialty rows {nm}", (sc == k).sum())
        add(out, f"specialty runs {nm}", len(set(S.run[sc == k])))
        add(out, f"specialty row share {nm}", (sc == k).mean(), note="the pooled (Grand Masters) fit's weight")
    specialty_residuals(S, fit, out)
    # robustness of the adopted per-scenario d's: one scale k on them, h = base_s (1 + hf/100) (1 + k d_s sp/100), the
    # d_s fitted on every row, a base per scenario refitted on each subset (halves of the records, rarity, without each
    # of the eight most-used cards); k = 1 on every row
    runs = common.runs()
    cut = runs[len(runs) // 2]["seq"]  # the later half of the records starts here
    rob = {"earlier half": S.seq < cut, "later half": S.seq >= cut, "SR": S.rar == 2, "SSR": S.rar == 3}
    top = [k for k, _ in collections.Counter(S.cid).most_common(8)]
    rob.update({f"without card {k}": S.cid != k for k in top})

    def scaled(k: float, q: Mask) -> float:
        """LL of rows q with a base per scenario and the specialty term k d_s (a multiplier per scenario: the fit's
        classes do not separate scenarios)"""
        return sum(S.fit(q & (sc == s), S.with_d(k * fit.d[s]))[0] for s in (1, 2, 3, 4) if (q & (sc == s)).any())

    ks = {
        nm: minimize_scalar(lambda k, q=q: -scaled(k, q), bounds=(-1, 3), method="bounded", options={"xatol": 1e-4}).x
        for nm, q in rob.items()
    }
    add(
        out,
        "specialty scale k robustness min",
        min(ks.values()),
        note="; ".join(f"{k} {v:.2f}" for k, v in ks.items() if not k.startswith("without")),
    )
    add(out, "specialty scale k robustness max", max(ks.values()))
    s_free = S.slope_of(S.every)
    add(out, "specialty model: free divisor c", 1 / s_free)
    # ll_ds(every, hfm) is the per-scenario fit above (the same d fits, the same rows)
    add(out, "specialty model: free divisor gain, nats", S.ll_ds(S.every, 1 + s_free * S.hfr) - sum(fit.ll))
    later_runs(S, cut, out)
    held_out_calibration(A, S, five_fold(S, out), out)
    removal_model(A, out)
    stat_option_drawn(A, S, out)
    out += same_card(A)
    return out


def stat_option_drawn(A: Data, S: SpecialtyRows, out: Out) -> None:
    """the adopted model (a base and d per scenario) against the same with a stat option drawn alongside the pool on
    every draw (it never leaves the pool): no extra parameter, in-sample and held out"""
    T = SpecialtyRows(A, stat_option=True)

    def ll_of(R: SpecialtyRows, q: Mask) -> float:
        return sum(R.fit(q & (R.sc == k), R.with_d(R.d_of(q & (R.sc == k))))[0] for k in (1, 2, 3, 4))

    add(out, "stat option drawn: in-sample, nats", ll_of(T, T.every) - ll_of(S, S.every), note="drawn minus not")

    def fold(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        r = {m: np.zeros(len(S.y)) for m in ("not drawn", "drawn")}
        for k in (1, 2, 3, 4):
            qa, qb = tr & (S.sc == k), te & (S.sc == k)
            if qb.any() and qa.any():
                r["not drawn"][qb] = S.scored(qa, qb, S.with_d(S.d_of(qa)))
                r["drawn"][qb] = T.scored(qa, qb, T.with_d(T.d_of(qa)))
        return {m: v[te] for m, v in r.items()}

    ho, by_card = held_out(fold, S.units)
    name = "stat option not drawn vs drawn"
    out.extend(compared(name, ho["not drawn"], ho["drawn"], S.units))
    out.extend(compared_by_card(name, by_card, "not drawn", "drawn", S.units))


def two_way_chi2(cell: np.ndarray, resid: np.ndarray, u: Units) -> tuple[common.WaldTest, bool]:
    """joint Wald test that every cell's residual sum is zero (common.cluster_chi2), the covariance of the cell sums
    clustered by run and by card (the two added, the card-run one taken off; when that is not positive definite, the
    two added alone, which is larger). Returns the test and whether that fallback was used"""
    from scipy.stats import chi2

    cells, ci = np.unique(cell, return_inverse=True)
    total = np.bincount(ci, weights=resid, minlength=len(cells))

    def V(unit: np.ndarray) -> np.ndarray:
        _, ui = np.unique(unit, return_inverse=True)
        M = np.zeros((ui.max() + 1, len(cells)))
        np.add.at(M, (ui, ci), resid)
        return len(M) / (len(M) - 1) * (M.T @ M)

    v_run, v_card = V(u.run), V(u.card)
    v = v_run + v_card - V(u.card_run)
    fallback = bool(np.linalg.eigvalsh(v).min() <= 0)
    if fallback:
        v = v_run + v_card
    x2 = float(total @ np.linalg.solve(v, total))
    return common.WaldTest(x2, len(cells), float(chi2.sf(x2, len(cells)))), fallback


def removal_model(A: Data, out: Out) -> None:
    """inspired pool skills partly leaving the card's draws: one removal share r (0 keeps them, 1 removes them for the
    whole run) fitted jointly with a base and d per scenario on the specialty fit's rows, in-sample and held out against
    r = 0 (the adopted form, refitted the same way); and the held-out obs / pred of both by inspired-skill count. The
    solver's constants are not these"""
    R = SpecialtyRows(A, removal=True)
    r_hat = R.removal_of()
    gain = R.ll_removal(r_hat) - R.ll_removal(0.0)
    add(out, "removal model: r", r_hat, note="jointly with a base and d per scenario")
    add(out, "removal model: in-sample gain, nats", gain, note="1 parameter")
    R.set_removal(r_hat)
    for k, nm in SCENARIOS:
        q = R.sc == k
        d = R.d_of(q)
        add(out, f"removal model: base {nm}", R.fit(q, R.with_d(d))[1], note="not read by the solver")
        add(out, f"removal model: d {nm}", d)

    def fold(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        res = {}
        for m, r in (("keep", 0.0), ("removal", R.removal_of(tr))):
            R.set_removal(r)
            ll, ph = np.zeros(len(R.y)), np.zeros(len(R.y))
            for k in (1, 2, 3, 4):
                qa, qb = tr & (R.sc == k), te & (R.sc == k)
                if qa.any() and qb.any():
                    ph[qb] = R.predicted(qa, qb, R.with_d(R.d_of(qa)))
                    ll[qb] = row_ll(ph[qb], R.y[qb])
            res[m], res[f"P(hit): {m}"] = ll[te], ph[te]
        return res

    ho, by_card = held_out(fold, R.units)
    name = "removal share vs none"
    out.extend(compared(name, ho["removal"], ho["keep"], R.units))
    out.extend(compared_by_card(name, by_card, "removal", "keep", R.units))
    for m in ("keep", "removal"):
        binned(
            out, f"held out: {m} model, inspired in pool", R.y, ho[f"P(hit): {m}"], R.inspired, INSPIRED_BINS, R.units
        )


def held_out_calibration(A: Data, S: SpecialtyRows, p_ho: list[np.ndarray], out: Out) -> None:
    """observed / predicted on held-out predictions, intervals clustered by run and card: the rows no other source
    hinted, and the specialty bands without and with the term. p_ho[0] holds folds by run (a card's other runs are in
    the fit); p_ho[1:] the leave-cards-out folds of each seed (the card is not in the fit), reported as the range of the
    ratio over the seeds and the widest interval"""
    p_spec = p_ho[0][1]

    def ratios(name: str, q: Mask, which: int) -> None:
        u = Units(*(x[q] for x in S.units))
        r, lo, hi = ratio_two_way(S.y[q], p_ho[0][which][q], u)
        add(out, f"held out: {name}", r, note="5-fold by run; two-way clustered interval")
        add(out, f"held out: {name} 95% low", lo)
        add(out, f"held out: {name} 95% high", hi)
        by_seed = [ratio_two_way(S.y[q], p[which][q], u) for p in p_ho[1:]]
        add(out, f"cards held out: {name} low", min(x[0] for x in by_seed), note=f"seeds {CARD_FOLD_SEEDS}")
        add(out, f"cards held out: {name} high", max(x[0] for x in by_seed))
        add(out, f"cards held out: {name} 95% low", min(x[1] for x in by_seed), note="the widest interval")
        add(out, f"cards held out: {name} 95% high", max(x[2] for x in by_seed))
        add(out, f"cards held out: {name} seeds covering 1", sum(lo_ <= 1 <= hi_ for _, lo_, hi_ in by_seed))

    q = A.cat[S.j] == "none"
    add(out, "held out: spark none rows", q.sum())
    ratios("spark none obs/pred", q, 1)
    for a, b, nm in SPEC_BANDS:
        q = (S.sp >= a) & (S.sp < b)
        add(out, f"held out: specialty band {nm} cards", len(np.unique(S.cid[q])))
        add(out, f"held out: specialty band {nm} effective cards", effective_cards(S.cid[q]))
        for which, label in enumerate(("before", "after")):
            ratios(f"specialty band {nm} {label}", q, which)
    # the (scenario, band) cells with 200+ rows jointly, on held-out predictions: no parameter was fitted on the
    # rows scored, so the reference is chi2 on the cell count
    cell = np.array([f"{k}:{nm}" for k, nm in zip(S.sc, band_of(S.sp))])
    n_cell = collections.Counter(cell)
    q = np.array([n_cell[c] >= 200 for c in cell])
    (x2, df, pv), fallback = two_way_chi2(cell[q], (S.y - p_spec)[q], Units(*(x[q] for x in S.units)))
    add(out, "held out: specialty cells chi2", x2, note="(scenario, band) cells with 200+ rows; two-way clustered Wald")
    add(out, "held out: specialty cells df", df)
    add(out, "held out: specialty cells p", pv)
    add(out, "held out: specialty cells fallback covariance", fallback, note="1: the card-run term was dropped")
    x2, _, pv = common.cluster_chi2(list(zip(cell[q], (S.y - p_spec)[q])), S.run[q])
    add(out, "held out: specialty cells chi2 by run", x2, note="the same, clustered by run alone")
    add(out, "held out: specialty cells p by run", pv)


def later_runs(S: SpecialtyRows, cut: int, out: Out) -> None:
    """held out: bases and d fitted on the earlier half of the records, scored on the later half, against the same
    without d"""
    tr, te = S.seq < cut, S.seq >= cut
    ho = {spec: np.zeros(len(S.y)) for spec in (False, True)}
    for k in (1, 2, 3, 4):
        qa, qb = tr & (S.sc == k), te & (S.sc == k)
        if not qb.any():
            continue
        ho[False][qb] = S.scored(qa, qb, S.hfm)
        ho[True][qb] = S.scored(qa, qb, S.with_d(S.d_of(qa, 1e-4)))
    add(out, "specialty held-out gain, nats", ho[True][te].sum() - ho[False][te].sum())
    add(out, "specialty held-out rows", te.sum())
    u = Units(*(x[te] for x in S.units))
    out.extend(compared("later runs: specialty term vs none", ho[True][te], ho[False][te], u))


HELD_OUT_P: tuple[str, ...] = ("none", "per-scenario d")  # models whose held-out P(hit) the calibration tests read


def five_fold(S: SpecialtyRows, out: Out) -> list[np.ndarray]:
    """5-fold: no term, one pooled d and a d per scenario (each with a base per scenario), and the free divisor, fitted
    on four folds; on folds by run and on leave-cards-out folds. Returns the held-out P(hit) per row of the models
    HELD_OUT_P, stacked: folds by run first, then each seed's leave-cards-out folds"""

    def fold_spec(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        r = {m: np.zeros(len(S.y)) for m in ("none", "one d", "per-scenario d", "free divisor")}
        p = {m: np.zeros(len(S.y)) for m in HELD_OUT_P}
        dp = S.pooled_d(tr)[0]
        hfc = 1 + S.slope_of(tr) * S.hfr
        for k in (1, 2, 3, 4):
            qa, qb = tr & (S.sc == k), te & (S.sc == k)
            if not qb.any() or not qa.any():
                continue
            m_spec = S.with_d(S.d_of(qa))
            p["none"][qb] = S.predicted(qa, qb, S.hfm)
            p["per-scenario d"][qb] = S.predicted(qa, qb, m_spec)
            r["none"][qb] = row_ll(p["none"][qb], S.y[qb])
            r["one d"][qb] = S.scored(qa, qb, S.with_d(dp))
            r["per-scenario d"][qb] = row_ll(p["per-scenario d"][qb], S.y[qb])
            r["free divisor"][qb] = S.scored(qa, qb, S.with_d(S.d_of(qa, base=hfc), hfc))
        return {m: v[te] for m, v in r.items()} | {f"P(hit): {m}": v[te] for m, v in p.items()}

    hos, by_card = held_out(fold_spec, S.units)
    add(out, "held out: specialty runs", len(np.unique(S.run)))
    add(out, "held out: specialty cards", len(np.unique(S.cid)))
    for name, a, b in (
        ("specialty term vs none", "per-scenario d", "none"),
        ("per-scenario d vs one d", "per-scenario d", "one d"),
        ("one d vs none", "one d", "none"),
        ("specialty model, free divisor vs 1/100", "free divisor", "per-scenario d"),
    ):
        out.extend(compared(name, hos[a], hos[b], S.units))
        out.extend(compared_by_card(name, by_card, a, b, S.units))
    return [np.stack([ho[f"P(hit): {m}"] for m in HELD_OUT_P]) for ho in [hos, *by_card]]


def clean_cards(g: GameData) -> set[int]:
    """SR and SSR cards (pals out) whose hint frequency is the same at every limit break while their specialty priority
    is not: across their limit breaks only the specialty term changes the predicted rate"""
    out = set()
    for cid, (rar, _) in g.card_info.items():
        if rar == 1 or cid not in g.cards or g.cards[cid]["pal"]:
            continue
        levels = g.level_cap[rar]
        hf = {effect(g, cid, EFFECT_HINT_FREQUENCY, lv) for lv in levels}
        sp = {effect(g, cid, EFFECT_SPECIALTY, lv) for lv in levels}
        if len(hf) == 1 and len(sp) > 1:
            out.add(cid)
    return out


class SameCardFit(NamedTuple):
    d: float
    gain: float  # LL gain over no term
    low: float  # 95% profile interval (rows independent; an end the interval reaches is the end of SAME_CARD_D)
    high: float
    card_low: float  # 95% sandwich interval clustered by card
    card_high: float
    gc_share: float  # the share of the rows from Grand Concert


def same_card_fit(A: Data, cards: Sequence[int]) -> SameCardFit:
    """the given cards at every limit break, rows no spark hinted: a base per (card, scenario), one pooled d, draws capped
    at the card's own increment at its level"""
    ci = A.ci
    cidall = np.array([x.cid for x in A.obs])
    jr = np.nonzero(A.noinsp & (A.rar[ci] != 1) & np.isin(cidall[ci], cards))[0]
    cw = ci[jr]
    tw = miss_table(np.ones(len(jr), int), A.nskill[cw], cap_draws(A.inc[cw]))
    yw = A.hit[jr]
    hw = 1 + A.hf[cw] / 100
    sw = A.spec[cw]
    gw = np.array([f"{a}-{b}" for a, b in zip(cidall[cw], A.scen[cw])])
    gu, gcode = np.unique(gw, return_inverse=True)  # (card, scenario) groups, in sorted order
    wc, wrep = alike(gcode, A.hf[cw], sw, A.nskill[cw], A.inc[cw])
    nwc = len(wrep)
    wh = np.bincount(wc, weights=yw, minlength=nwc)
    wn = np.bincount(wc, minlength=nwc).astype(float)
    twc = tw[wrep]
    gq = [np.nonzero(gcode[wrep] == u)[0] for u in range(len(gu))]  # each group's classes

    group_card = [int(g.split("-")[0]) for g in gu]

    def per_card(d: float) -> dict[int, float]:
        """each card's LL, a base per (card, scenario) group profiled out, for the specialty term with this d"""
        mc = (hw * (1 + d * sw / 100))[wrep]
        ll: dict[int, float] = collections.defaultdict(float)
        for card, q in zip(group_card, gq):
            ll[card] -= best_base(twc[q], mc[q], wh[q], wn[q], bounds=(0.05, 12), xatol=1e-6, clipped=True, grid=24).fun
        return ll

    def lw(d: float) -> float:
        """LL with a base per (card, scenario) group for the specialty term with this d"""
        return sum(per_card(d).values())

    from scipy.optimize import brentq

    rw = minimize_scalar(lambda d: -lw(d), bounds=SAME_CARD_D, method="bounded", options={"xatol": 1e-3})
    d, best = rw.x, -rw.fun

    def short(x: float) -> float:
        """how far LL(x) lies above the 95% profile bound"""
        return lw(x) - (best - CHI1)

    lo, hi = SAME_CARD_D
    lo = brentq(short, lo, d, xtol=1e-3) if short(lo) < 0 else lo
    hi = brentq(short, d, hi, xtol=1e-3) if short(hi) < 0 else hi
    # the profile interval treats rows as independent; the sandwich interval clusters by card (each card's bases are
    # its own, so its profiled LL is a separate term): common.robust_interval with cards in place of runs. Rows of one
    # card-run share a near-fixed number of draws, so they are negatively correlated and the sandwich can be narrower
    rob = common.robust_interval(per_card, d, h=0.01)
    return SameCardFit(d, best - lw(0.0), lo, hi, rob.low, rob.high, float((A.scen[cw] == 3).mean()))


def same_card(A: Data) -> list[common.Value]:
    """the same card at different limit breaks: the clean cards (clean_cards: only the specialty term changes) seen in
    the runs at two or more specialty values, R cards out"""
    out: Out = []
    lv = collections.defaultdict(set)
    for i, x in enumerate(A.obs):
        if x.rar != 1:
            lv[x.cid].add(A.spec[i])
    cc = clean_cards(_game())
    clean = sorted(k for k, v in lv.items() if len(v) > 1 and k in cc)
    f = same_card_fit(A, clean)
    label = "specialty same-card clean"
    add(out, f"{label}: cards", len(clean))
    add(out, f"{label}: d", f.d)
    add(out, f"{label}: d 95% low", f.low, note=f"profile likelihood; searched over {SAME_CARD_D}")
    add(out, f"{label}: d 95% high", f.high)
    add(out, f"{label}: d 95% low by card", f.card_low, note="sandwich, clustered by card")
    add(out, f"{label}: d 95% high by card", f.card_high)
    add(out, f"{label}: gain, nats", f.gain)
    add(out, f"{label}: row share grand concert", f.gc_share)
    return out


if __name__ == "__main__":
    common.main(claims)
