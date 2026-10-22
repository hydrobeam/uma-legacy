"""Section 7 (training hints): reproduces docs/RESEARCH.md section 7 from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it) and master.mdb.

One observation per (run, non-pal deck card); its rows are the card's training-hint pool skills (master.mdb
single_mode_hint_gain), a row hit when the run's hint list names the card's character as a source of the skill. Hint
frequency (effect 18), hint levels per draw (1 + effect 17) and specialty priority (effect 19) are read at the card's
level: floored linear interpolation between milestone levels, plus the unique effect.

Every analysis runs on every record, on three row sets:
- the form study: every limit break, R cards included, the rate without the specialty term (h = base_s (1 + hf/100)):
  it compares forms of the hf rate (Bernoulli rows, P(hit) = 1 - (1 - 1/n)^h), the stat option, repeats and distinct
  counts, per-scenario bases, hf by card level and the per-draw increment, and through the Lv-5 cap (uniform draws over
  the skills still below Lv 5, h read as floor or ceiling; pkg/loopopt/src/deck/hints.rs) on its max-limit-break rows
  no spark hinted the cap, joint draws and pool size;
- the fit: max-limit-break card-runs, rows no spark hinted, R cards out, through the Lv-5 cap, with
  h = base_s (1 + hf/100) (1 + d_s spec/100); its later-runs test fits on the earlier half of the records (by seq) and
  scores the later half;
- tests of the current model: the adopted rate (the same h with hints.rs's constants) on the fit's card-runs, for the
  other covariates (a free level per scenario in both fits), per-card differences, the spark deficit (every row),
  drop-out and priority;
- the same-card test: cards whose specialty differs between limit breaks, every limit break, a base per (card,
  scenario); on every such card and on the clean ones (hint frequency the same at every limit break).
Rate forms, specialty choices, SR/pal covariates and a factor per card are also tested held out: 5-fold cross-validation by common.fold(run), each fold scored by the fits
on the other four, per-run held-out log-likelihoods compared by common.compare_held_out using a z > 1.96 threshold. Every other result is in-sample on the pooled runs. Intervals and z on observed / predicted over rows (the
spark deficit, priority, the specialty cells) are run-clustered (common.cluster_ci): one run gives many rows; the
specialty cells and the per-card cells are tested jointly by a run-clustered Wald chi2 (common.cluster_chi2), the
specialty cells' largest |z| only described. Every test reads every run; the general collection's own numbers (batches
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


def effect(g: GameData, cid: int, typ: int, level: int, mode: str = "lin") -> int:
    """support effect `typ` at `level`: floored linear interpolation between milestones (or the last milestone
    reached, mode 'step'), plus the unique effect once its level is reached"""
    vals = g.effects[g.card_info[cid][1]].get(typ)
    x = 0
    if vals:
        pts = [(m, v) for m, v in zip(MIL, vals) if v != -1]
        if pts and level >= pts[0][0]:
            if mode == "step":
                x = [v for m, v in pts if m <= level][-1]
            else:
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
    hf_step: int  # hint frequency, the last milestone reached
    hf_max: int  # hint frequency at maximum limit break (web_cards.json)
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
    hf_step: np.ndarray
    hf_max: np.ndarray
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
                    hf_step=effect(g, cid, EFFECT_HINT_FREQUENCY, level, "step"),
                    hf_max=C[cid]["hf"],
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
        hf_step=per_card_run("hf_step"),
        hf_max=per_card_run("hf_max"),
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
def miss(k: int, n: int, t: int, cap: int) -> float:
    """P(none of k given pool skills drawn) in t draws from an initial pool of n skills, each draw uniform over skills still below Lv 5; a skill
    leaves after `cap` draws (pkg/loopopt/src/deck/hints.rs miss_draws)"""
    if k == 0:
        return 1.0
    if k >= n:
        return 1.0 if t == 0 else 0.0
    if t <= cap:
        return (1 - k / n) ** t
    st = {tuple([n - k] + [0] * cap): 1.0}  # how many of the other skills have taken 0..cap draws
    for _ in range(t):
        nx = collections.defaultdict(float)
        for s, p in st.items():
            avail = n - s[cap]
            for j in range(cap):
                if s[j]:
                    u = list(s)
                    u[j] -= 1
                    u[j + 1] += 1
                    nx[tuple(u)] += p * s[j] / avail
        st = nx
    return sum(st.values())


TMAX: int = 12  # the most draws a table covers


def miss_table(k: np.ndarray, n: np.ndarray, cap: np.ndarray) -> np.ndarray:
    """rows x (TMAX + 1): miss(k, n, t, cap) for t = 0..TMAX (computed once per distinct (k, n, cap))"""
    u, inv = np.unique(
        np.stack([np.asarray(k), np.asarray(n), np.asarray(cap)], 1).astype(int), axis=0, return_inverse=True
    )
    t = np.array([[miss(int(a), int(b), t, int(c)) for t in range(TMAX + 1)] for a, b, c in u]).reshape(
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


def bern_ll(p: np.ndarray, y: np.ndarray) -> float:
    return row_ll(p, y).sum()


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


def cv(run_rows: np.ndarray, fold_fn: Callable[[Mask, Mask], dict[str, np.ndarray]]) -> dict[str, np.ndarray]:
    """5-fold cross-validation by common.fold(run), a run's rows all in one fold: fold_fn(train, test) fits every model on
    the rows in `train` (boolean masks over the rows) and returns {model: log-likelihood of each row in `test`, in row
    order}. Returns {model: held-out log-likelihood per run}, runs in sorted order"""
    runs = np.unique(run_rows)
    fo = np.array([common.fold(int(r)) for r in run_rows])
    ll = {}
    for f in range(common.K_FOLDS) if CV_FOLDS is None else CV_FOLDS:
        te = fo == f
        for m, v in fold_fn(~te, te).items():
            ll.setdefault(m, np.zeros(len(run_rows)))[te] = v
    ix = np.searchsorted(runs, run_rows)
    return {m: np.bincount(ix, weights=v, minlength=len(runs)) for m, v in ll.items()}


def compared(name: str, a: np.ndarray, b: np.ndarray) -> list[common.Value]:
    """the Values of a held-out comparison, a (the more complex model) minus b, summed over runs: nats, SE, z"""
    s, se, z = common.compare_held_out(a, b)
    return [V(f"held out: {name}: nats", s), V(f"held out: {name}: SE", se), V(f"held out: {name}: z", z)]


def form_classes(D: Data) -> tuple[np.ndarray, np.ndarray]:
    """card-runs alike for every rate of the form study (scenario, hf in its three readings, pool size): (class of each
    card-run, a card-run of each class)"""
    if D.form_classes is None:
        D.form_classes = alike(D.scen, D.hf, D.hf_step, D.hf_max, D.nskill)
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


def fit_cap(D: Data, rows: Mask, cap: bool = True, pooled: bool = False) -> tuple[dict[int, float], float, np.ndarray]:
    """per-scenario (or pooled) base for h = base (1 + hf/100), Bernoulli through the floor/ceiling mixture of
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
    g = np.zeros(len(j), int) if pooled else D.scen[c]
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


def adopted() -> tuple[dict[int, float], dict[int, float]]:
    """the solver's training-hint bases and specialty weights by scenario (hint_base, hint_spec in
    pkg/loopopt/src/deck/hints.rs)"""
    src = open(f"{common.ROOT}/pkg/loopopt/src/deck/hints.rs", encoding="utf-8").read()

    def table(fn: str) -> dict[int, float]:
        body = re.search(r"fn " + fn + r"[^{]*\{.*?match scenario \{(.*?)\}", src, re.S).group(1)
        v = {k: float(x) for k, x in re.findall(r'"(\w+)" => ([\d.]+)', body)}
        v["unity"] = float(re.search(r"_ => ([\d.]+)", body).group(1))
        return {1: v["ura"], 2: v["unity"], 3: v["gc"], 4: v["trackblazer"], 5: v["grandmasters"]}

    return table("hint_base"), table("hint_spec")


def derived() -> list[common.Value]:
    """the h-reading example: n = 5, h = 2.74; smooth (1 - 1/n)^h against floor/ceiling draws"""
    n, h = 5, 2.74
    caps = [V(f"draws to Lv 5, +{i} card", math.ceil(5 / i), note="derived") for i in range(1, 6)]
    return [
        *caps,
        V("miss (1-1/n)^h, n=5 h=2.74", (1 - 1 / n) ** h, note="derived"),
        V(
            "miss floor/ceiling, n=5 h=2.74",
            mix(np.array([[(1 - 1 / n) ** t for t in range(TMAX + 1)]]), np.array([h]))[0],
            note="derived",
        ),
    ]


class Adopted(NamedTuple):
    """the solver's constants by scenario"""

    base: dict[int, float]  # hint_base
    spec: dict[int, float]  # hint_spec (d)


def adopted_values(A: Adopted, out: Out) -> None:
    """the solver's constants (read, not fitted)"""
    for s, nm in ((2, "Unity Cup"), (3, "Grand Concert"), (1, "URA Finale"), (4, "Trackblazer"), (5, "Grand Masters")):
        add(out, f"adopted {nm}", A.base[s], note="the solver's hint_base (pkg/loopopt/src/deck/hints.rs)")
        add(out, f"adopted d {nm}", A.spec[s], note="the solver's hint_spec")


def worked_example(A: Adopted, out: Out) -> None:
    """a worked example using separate median hf and spec values across the runs' SR/SSR non-pal, non-group cards
    at maximum limit break"""
    wc = common.cards()
    deck_cards = [
        wc[c]
        for r in common.runs()
        for c, _, _ in r["deck"]
        if c in wc and wc[c]["rarity"] > 1 and not wc[c]["pal"] and wc[c].get("kind") != "group"
    ]
    mhf = float(np.median([c["hf"] for c in deck_cards]))
    msp = float(np.median([c["spec"] for c in deck_cards]))
    add(out, "example: median hint frequency", mhf, note=f"{len(deck_cards):,} deck cards in the runs")
    add(out, "example: median specialty priority", msp)
    for s, nm in ((2, "Unity Cup"), (3, "Grand Concert"), (1, "URA Finale"), (4, "Trackblazer")):
        add(
            out,
            f"example: hints per career, median card, {nm}",
            A.base[s] * (1 + mhf / 100) * (1 + A.spec[s] * msp / 100),
        )
        add(out, f"example: hints per career, no effects, {nm}", A.base[s])


# ---------------------------------------------------------------- the form study: every record, every limit break, no specialty term
class FormFits(NamedTuple):
    one_base_ll: float
    per_scenario_ll: float


def form_fits(D: Data, forms: RateForms, out: Out) -> FormFits:
    """the rate forms in-sample"""
    ci, hit, hf = D.ci, D.hit, D.hf
    n = D.nskill.astype(float)
    add(out, "form: rows", len(ci))
    add(out, "form: rows without a spark", D.noinsp.sum())
    add(out, "form: runs", len({D.obs[i].run for i in ci}), note="runs that add a row")

    def fixed_ll(h: np.ndarray, nn: np.ndarray = n) -> float:
        return bern_ll(1 - (1 - 1 / nn[ci]) ** h[ci], hit)

    add(out, "LL 1+hf/25 over options", fixed_ll(1 + hf / 25, D.opts.astype(float)))
    add(out, "LL 1+hf/25 over skills", fixed_ll(1 + hf / 25))
    add(out, "LL constant", fitB(D, forms.constant, [2.0])[1])
    ll1 = fitB(D, forms.one_base, [2.0])[1]
    add(out, "LL base(1+hf/100)", ll1)
    th4, ll4 = fitB(D, forms.per_scenario, [1.5, 2.6, 2.0, 1.7])
    add(out, "LL per-scenario base(1+hf/100)", ll4)
    thc, llc = fitB(D, forms.free_divisor, [*th4, 100.0])
    add(out, "LL free divisor", llc)
    add(out, "free divisor c", thc[4])
    return FormFits(ll1, ll4)


def stat_option_cells(D: Data, out: Out) -> None:
    """skills only: per (card, hf) with >= 15 card-runs whose every row is exact, var(T) against the floor a drawn stat
    option implies (E = mean T (o / n) draws over o = n + 1 options give var >= E (1/o)(1 - 1/o))"""
    ci, hf = D.ci, D.hf
    tot = np.bincount(ci, weights=D.kk, minlength=D.N)
    cens = np.bincount(ci, weights=D.kind, minlength=D.N)
    cid = np.array([x.cid for x in D.obs])
    o = D.opts.astype(float)
    cells = below_n = 0
    for c in sorted(set(cid)):
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


class NoSparkFit(NamedTuple):
    """per-scenario bases fitted on the rows no spark hinted, every limit break"""

    bases: np.ndarray
    ll: float
    h: np.ndarray  # h per card-run


def repeats_and_distinct(D: Data, forms: RateForms, out: Out) -> NoSparkFit:
    """repeats and distinct counts (bases: per-scenario, rows no spark hinted, every limit break)"""
    ci, hit, hf = D.ci, D.hit, D.hf
    n = D.nskill.astype(float)
    bs, ll_ni = fitB(D, forms.per_scenario, [1.5, 2.7, 2.0, 1.8], rows=D.noinsp)
    h = B4(D, bs) * (1 + hf / 100)
    inc, kk, kind = D.inc[ci], D.kk, D.kind
    k1 = (kind == 0) & (kk == 1)
    k2 = ((kind == 0) & (kk >= 2)) | ((kind == 1) & (kk >= 2))
    use = (k1 | k2) & (inc <= 4)
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
    has = n > 0
    dist = np.bincount(ci, weights=hit, minlength=D.N).astype(int)
    model = np.zeros(TMAX + 1)
    for i in np.nonzero(has)[0]:  # distinct skills after t uniform draws over n: C(n,d) d! S(t,d) / n^t
        t0 = int(h[i])
        w = h[i] - t0
        for t, wt in ((t0, 1 - w), (t0 + 1, w)):
            model[: t + 1] += wt * distinct_pmf(int(n[i]), t)
    model /= has.sum()
    for k in (1, 2, 3, 4):
        add(out, f"distinct {k} observed", np.mean(dist[has] == k))
        add(out, f"distinct {k} model", model[k], note="exact")
    return NoSparkFit(bs, ll_ni, h)


def held_out_forms(D: Data, forms: RateForms, out: Out) -> None:
    """the form study's rate forms held out: each fold's fits on the other four folds' rows, every row scored"""
    ci, hit, hf = D.ci, D.hit, D.hf
    n = D.nskill.astype(float)

    def scored(h: np.ndarray, te: Mask, nn: np.ndarray = n) -> np.ndarray:
        return row_ll(1 - (1 - 1 / nn[ci][te]) ** h[ci][te], hit[te])

    def fold_form(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        t4 = fitB(D, forms.per_scenario, [1.5, 2.6, 2.0, 1.7], rows=tr)[0]
        return {
            "one base": scored(forms.one_base(fitB(D, forms.one_base, [2.0], rows=tr)[0]), te),
            "per-scenario": scored(forms.per_scenario(t4), te),
            "free divisor": scored(forms.free_divisor(fitB(D, forms.free_divisor, [*t4, 100.0], rows=tr)[0]), te),
            "constant": scored(forms.constant(fitB(D, forms.constant, [2.0], rows=tr)[0]), te),
            "1+hf/25 over options": scored(1 + hf / 25, te, D.opts.astype(float)),
            "1+hf/25 over skills": scored(1 + hf / 25, te),
        }

    hof = cv(run_of_rows(D, ci), fold_form)
    add(out, "held out: form study runs", len(hof["per-scenario"]))
    out.extend(compared("free divisor vs 1/100", hof["free divisor"], hof["per-scenario"]))
    out.extend(compared("per-scenario bases vs one base", hof["per-scenario"], hof["one base"]))
    for m in ("constant", "1+hf/25 over options", "1+hf/25 over skills"):
        out.extend(compared(f"per-scenario bases vs {m}", hof["per-scenario"], hof[m]))


class CapFit(NamedTuple):
    """the Lv-5 cap fit on max-limit-break rows no spark hinted"""

    rows: Mask
    j: np.ndarray  # those rows' indices
    p: np.ndarray  # P(hit) per row of j


def lv5_cap(D: Data, out: Out) -> CapFit:
    """the Lv-5 cap against drawing with replacement (max limit break, rows no spark hinted)"""
    ci, hit = D.ci, D.hit
    rows = (D.lb[ci] == 4) & D.noinsp
    _, ll_cap, p_cap = fit_cap(D, rows)
    _, ll_repl, p_repl = fit_cap(D, rows, cap=False)
    add(out, "cap gain, nats", ll_cap - ll_repl)
    j = np.nonzero(rows)[0]
    capd = cap_draws(D.hinc[ci[j]])
    for d in (5, 3, 2, 1):
        q = capd == d
        add(out, f"cap table {d} draws rows", q.sum())
        add(out, f"cap table {d} draws replacement", hit[j][q].sum() / p_repl[q].sum())
        add(out, f"cap table {d} draws cap", hit[j][q].sum() / p_cap[q].sum())
    return CapFit(rows, j, p_cap)


def joint_draws(D: Data, cap: CapFit, h: np.ndarray, out: Out) -> None:
    """pairs of no-spark pool skills on one max-limit-break card-run (URA, Unity, Grand Concert)"""
    ci, hit = D.ci, D.hit
    n = D.nskill.astype(float)
    m = cap.rows & (D.scen[ci] != 4)
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
    add(out, "P(neither) observed", np.mean((1 - hit[a]) * (1 - hit[b])))
    add(out, "P(neither) joint", np.mean(m2))


def pool_size(D: Data, cap: CapFit, out: Out) -> None:
    """obs / pred by n (cap fit above), Unity Cup and Grand Concert, n with >= 150 rows"""
    ci, hit, sc, j = D.ci, D.hit, D.scen, cap.j
    n = D.nskill.astype(float)
    ratios = []
    for s in (2, 3):
        for v in sorted(set(n[ci[j]])):
            q = (n[ci[j]] == v) & (sc[ci[j]] == s)
            if q.sum() >= 150:
                ratios.append(hit[j][q].sum() / cap.p[q].sum())
    note = "Lv-5 cap, max limit break, no spark, per (scenario, n)"
    add(out, "pool size obs/pred min", min(ratios), note=note)
    add(out, "pool size obs/pred max", max(ratios), note=note)


def increment(D: Data, out: Out) -> None:
    """per-draw increment: levels from this card's training alone"""
    ok = total = 0
    for x in D.obs:
        for lv, src in x.tr.values():
            if src == (x.ch,):
                total += 1
                ok += (lv % (1 + x.hlv) == 0) or lv == 5
    add(out, "increment: multiples of iota", ok)
    add(out, "increment: single-source levels", total)


def hf_by_level(D: Data, nospark: NoSparkFit, out: Out) -> None:
    """hint frequency by card level (rows no spark hinted)"""
    hf = D.hf
    n = D.nskill.astype(float)
    add(out, "hf max in the data", hf[n > 0].max(), note=f"min {hf[n > 0].min():.0f}")
    add(out, "LL hf floored-linear", nospark.ll)
    for nm, H in (("LL hf step", D.hf_step), ("LL hf max-LB for all", D.hf_max)):
        add(out, nm, fitB(D, lambda th, H=H: B4(D, th) * (1 + H / 100), list(nospark.bases), rows=D.noinsp)[1])


# ---------------------------------------------------------------- tests of the current model
ADOPTED_NOTE: str = "the adopted rate, Lv-5 cap, max limit break, R cards out"


@dataclass
class AdoptedRate:
    """the adopted rate (hints.rs constants, specialty term in) through the Lv-5 cap, on max-limit-break card-runs with
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
    facs = []
    add(out, "covariate rows", nim.sum(), note=ADOPTED_NOTE)
    for nm, X in covs.items():
        f = rows.fit(X[ck])
        factors = np.exp(f.theta[4:])
        facs += list(factors)
        add(
            out,
            f"covariate gain {nm}, nats",
            f.ll - l0,
            note=f"{X.shape[1]} parameter(s), factor(s) " + ", ".join(f"{x:.3f}" for x in factors),
        )
        if X.shape[1] == 1:
            add(out, f"covariate factor {nm}", factors[0])
    add(out, "covariate factor min", min(facs))
    add(out, "covariate factor max", max(facs))
    # the detectable covariates held out: the adopted rate with a free level per scenario, with and without each
    held = {"none": np.zeros((len(ck), 0)), "SR": covs["SR"][ck], "pals per deck": covs["pals per deck"][ck]}
    hoc = cv(
        run_of_rows(D, ck),
        lambda tr, te: {m: row_ll(rows.p(rows.fit(X, tr).theta, X, te), rows.y[te]) for m, X in held.items()},
    )
    add(out, "held out: covariate runs", len(hoc["none"]))
    for nm in ("SR", "pals per deck"):
        out.extend(compared(f"covariate {nm} vs none", hoc[nm], hoc["none"]))
    # context: the SR factor on the general collection and on the targeted ones (decks chosen for their cards)
    general = general_rows(D, ck)
    for label, q in (("general collection", general), ("targeted collections", ~general)):
        add(out, f"covariate factor SR, {label}", np.exp(rows.fit(covs["SR"][ck], q).theta[4]))
    card_heterogeneity(D, rows, general, out)


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
    add(out, "per-card rows", (code >= 0).sum())
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

    ho = cv(run, fold)
    out.extend(compared("a factor per card vs none", ho["per card"], ho["none"]))


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
        ("inspiration + card event", "Inspiration+card event"),
        ("card event", "card event"),
        ("other card training", "other card"),
    ):
        q = AR.rows & (D.cat == key)
        add(out, f"spark {c} rows", q.sum(), note=ADOPTED_NOTE)
        add(out, f"spark {c} observed", hit[q].mean())
        add(out, f"spark {c} predicted", AR.p[q].mean())
        add(out, f"spark {c} ratio", hit[q].mean() / AR.p[q].mean())
        lo, hi = observed_interval(D, AR, q)
        add(out, f"spark {c} observed 95% low", lo)
        add(out, f"spark {c} observed 95% high", hi)
        if c == "none":
            r, lo, hi = clustered_ratio(D, AR, q)
            add(out, "spark none obs/pred", r)
            add(out, "spark none obs/pred 95% low", lo)
            add(out, "spark none obs/pred 95% high", hi)


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
    add(out, "drop-out test rows", len(jj), note=ADOPTED_NOTE)
    add(out, "drop-out test observed", D.hit[jj].mean())
    add(out, "drop-out test no drop-out", keep.mean())
    add(out, "drop-out test drop-out", drop.mean())


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
        return derived()
    out: Out = []
    A = Adopted(*adopted())
    adopted_values(A, out)
    worked_example(A, out)
    D = build()
    forms = rate_forms(D)
    fits = form_fits(D, forms, out)
    stat_option_cells(D, out)
    nospark = repeats_and_distinct(D, forms, out)
    add(out, "per-scenario bases gain, nats", fits.per_scenario_ll - fits.one_base_ll)  # against one base
    held_out_forms(D, forms, out)
    cap = lv5_cap(D, out)
    joint_draws(D, cap, nospark.h, out)
    pool_size(D, cap, out)
    increment(D, out)
    hf_by_level(D, nospark, out)
    AR = adopted_rate(D, A)
    covariates(D, AR, out)
    spark_deficit(D, AR, out)
    drop_out(D, AR, out)
    out += specialty(D)
    priority(D, AR, out)
    return derived() + out + findings(out)


def findings(out: list[common.Value]) -> list[common.Finding]:
    """the section's conclusions, each tested on the numbers above"""
    from scipy.stats import binom, chi2

    g: dict[str, Any] = {c.name: c.got for c in out}

    def F(*a: Any) -> common.Finding:
        return common.finding(SEC, *a)

    fs: list[common.Finding] = []
    lps = g["LL per-scenario base(1+hf/100)"]

    def H(c: str) -> tuple[float, float, float]:
        return g[f"held out: {c}: nats"], g[f"held out: {c}: SE"], g[f"held out: {c}: z"]

    def hs(c: str) -> str:
        return "{:+.1f} +- {:.1f} nats, z = {:.2f}".format(*H(c))

    rf = ("constant", "1+hf/25 over options", "1+hf/25 over skills")
    fs.append(
        F(
            "h = beta_s (1 + hf/100) predicts unseen runs better than the other rate forms tried without a free slope (a constant, "
            "1 + hf/25 over options or over skills; 5-fold held out, each z > 1.96)",
            all(H(f"per-scenario bases vs {m}")[2] > 1.96 for m in rf),
            "; ".join(f"against {m} " + hs(f"per-scenario bases vs {m}") for m in rf)
            + f" on {g['held out: form study runs']:,.0f} runs",
        )
    )
    gc = g["LL free divisor"] - lps
    fs.append(
        F(
            "Known miss (form study, without the specialty term): the slope 1/100 is assumed; a free divisor is detectable (LR gain above 1.92 nats, the 1-parameter 5% "
            "bound) and predicts unseen runs better than 1/100 (5-fold held out, z > 1.96)",
            gc > 1.92 and H("free divisor vs 1/100")[2] > 1.96,
            f"c = {g['free divisor c']:.0f}; in-sample +{gc:.1f} nats; held out "
            + hs("free divisor vs 1/100")
            + f" on {g['held out: form study runs']:,.0f} runs",
        )
    )
    k, n = g["cells var below stat-option floor"], g["cells (card, hf)"]
    fs.append(
        F(
            "The stat option is never drawn: the draw-count variance is below the floor a drawn stat option implies in most (card, hf) "
            "cells (sign test p < 0.05)",
            binom.sf(k - 1, n, 0.5) < 0.05,
            f"{k:.0f} of {n:.0f} cells, p = {binom.sf(k - 1, n, 0.5):.4f}",
        )
    )
    o, t2, po = g["repeat rate observed"], g["repeat rate two-point count"], g["repeat rate Poisson count"]
    fs.append(
        F(
            "Draws repeat skills as uniform draws with a near-fixed count do, not as a Poisson count",
            abs(o - t2) < abs(o - po),
            f"repeat rate observed {o:.3f}, two-point {t2:.3f}, Poisson {po:.3f}",
        )
    )
    gb = g["per-scenario bases gain, nats"]
    fs.append(
        F(
            "Bases differ by scenario (per-scenario bases against one base, LR p < 0.001 on 3 parameters)",
            chi2.sf(2 * gb, 3) < 0.001,
            f"+{gb:.0f} nats, LR p = {chi2.sf(2 * gb, 3):.1e}",
        )
    )
    fs.append(
        F(
            "Per-scenario bases predict unseen runs better than one base (form study, 5-fold held out, z > 1.96)",
            H("per-scenario bases vs one base")[2] > 1.96,
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
            "Known miss (not modelled): rarity (SR against SSR) and pals per deck are detectable on top of the adopted rate "
            "(each LR p < 0.05)",
            all(cov[nm][1] < 0.05 for nm in miss_c),
            "; ".join(f"{nm} +{cov[nm][0]:.1f} nats, p = {cov[nm][1]:.1e}" for nm in miss_c),
        )
    )
    sr, pals = H("covariate SR vs none"), H("covariate pals per deck vs none")
    fs.append(
        F(
            "Known miss (not modelled): pals per deck predicts unseen runs better than the adopted rate without it (5-fold held "
            "out, z > 1.96); rarity (SR against SSR), detectable in-sample, does not (z <= 1.96)",
            pals[2] > 1.96 and sr[2] <= 1.96,
            "; ".join(f"{nm} " + hs(f"covariate {nm} vs none") for nm in miss_c)
            + f" on {g['held out: covariate runs']:,.0f} runs; SR factor {g['covariate factor SR']:.3f}, general collection "
            f"{g['covariate factor SR, general collection']:.3f}, targeted collections "
            f"{g['covariate factor SR, targeted collections']:.3f}",
        )
    )
    fs.append(
        F(
            "Known miss (not modelled): hint rates differ by card beyond the adopted rate: per-card obs/pred on the cards with "
            f"{CARD_ROWS}+ rows are jointly inconsistent with it (run-clustered Wald, p < 0.001, on every run and on the general "
            "collection alone), and the cards' differences carry over to unseen runs (a factor per card against none, 5-fold "
            "held out, z > 1.96); which card properties explain them is open",
            g["per-card Wald p"] < 0.001
            and g["per-card Wald p, general collection"] < 0.001
            and H("a factor per card vs none")[2] > 1.96,
            f"obs/pred {g['per-card obs/pred min']:.2f}-{g['per-card obs/pred max']:.2f} across {g['per-card cards']:.0f} cards; "
            f"chi2 {g['per-card Wald chi2']:.0f} on {g['per-card Wald df']:.0f}, p = {g['per-card Wald p']:.1e}; general "
            f"collection chi2 {g['per-card Wald chi2, general collection']:.0f} on "
            f"{g['per-card Wald df, general collection']:.0f}, p = {g['per-card Wald p, general collection']:.1e}; held out "
            + hs("a factor per card vs none"),
        )
    )
    fs.append(
        F(
            "Card type, initial bond and pool size add nothing significant to the adopted rate (each LR p > 0.05)",
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
            "Two targets in one card's pool land jointly as the draws imply, not independently",
            abs(b - j) < abs(b - i),
            f"P(both) observed {b:.4f}, joint {j:.4f}, independent {i:.4f} ({g['joint pairs']:,.0f} pairs)",
        )
    )
    m, t = g["increment: multiples of iota"], g["increment: single-source levels"]
    fs.append(
        F(
            "Each draw adds the card's hint levels (at least 99.5% of single-source levels are multiples of it, capped at 5)",
            m / t >= 0.995,
            f"{m:,.0f} of {t:,.0f}",
        )
    )
    fl, stp, mx = g["LL hf floored-linear"], g["LL hf step"], g["LL hf max-LB for all"]
    fs.append(
        F(
            "hf at a card's level is the floored linear interpolation between milestone levels (it fits at least as well as a step "
            "or the max-limit-break value)",
            fl >= stp and fl >= mx,
            f"floored-linear {fl:.1f}, step {stp:.1f}, max-LB {mx:.1f}",
        )
    )
    sp = {}  # by other source: (prediction inside the interval, observed, predicted, observed 95% low, high)
    for c in ("none", "inspiration", "card event", "other card training"):
        ob, pr, lo, hi = (
            g[f"spark {c} {k}"] for k in ("observed", "predicted", "observed 95% low", "observed 95% high")
        )
        sp[c] = (lo <= pr <= hi, ob, pr, lo, hi)
    v = sp["none"]
    fs.append(
        F(
            "With no other source of the skill the adopted rate slightly under-predicts (the run-clustered interval lies entirely "
            "above the prediction)",
            v[3] > v[2],
            f"{v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}; obs/pred {g['spark none obs/pred']:.3f} "
            f"[{g['spark none obs/pred 95% low']:.3f}, {g['spark none obs/pred 95% high']:.3f}]",
        )
    )
    fs.append(
        F(
            "A card's own event or another card's training leaves the adopted rate calibrated (prediction inside the interval)",
            sp["card event"][0] and sp["other card training"][0],
            "; ".join(
                f"{c} {v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}"
                for c, v in sp.items()
                if c in ("card event", "other card training")
            ),
        )
    )
    v = sp["inspiration"]
    fs.append(
        F(
            "Known miss (not modelled): a skill also hinted by inspiration is hinted by training less than predicted",
            v[4] < v[2],
            f"{v[1]:.3f} [{v[3]:.3f}, {v[4]:.3f}] vs {v[2]:.3f}",
        )
    )
    o, nd, dd = g["drop-out test observed"], g["drop-out test no drop-out"], g["drop-out test drop-out"]
    fs.append(
        F(
            "Spark-hinted skills do not drop out of the draws (observed nearer no drop-out than full drop-out)",
            abs(o - nd) < abs(o - dd),
            f"observed {o:.3f}, no drop-out {nd:.3f}, drop-out {dd:.3f}",
        )
    )
    gs = g["specialty gain per scenario, nats"]
    fs.append(
        F(
            "Specialty priority raises training hints: (1 + d_s sp/100) is detectable (4 parameters, LR p < 0.001)",
            chi2.sf(2 * gs, 4) < 0.001,
            f"+{gs:.1f} nats, LR p = {chi2.sf(2 * gs, 4):.1e}",
        )
    )
    fs.append(
        F(
            "The specialty term (a d per scenario) predicts unseen runs better than no term (5-fold held out, z > 1.96)",
            H("specialty term vs none")[2] > 1.96,
            hs("specialty term vs none") + f" on {g['held out: specialty runs']:,.0f} runs",
        )
    )
    fs.append(
        F(
            "A d per scenario predicts unseen runs better than one pooled d (5-fold held out, z > 1.96)",
            H("per-scenario d vs one d")[2] > 1.96,
            hs("per-scenario d vs one d") + "; one d against none " + hs("one d vs none"),
        )
    )
    bf = [
        (b_, g[f"specialty band {b_} before"], g[f"specialty band {b_} after"]) for b_ in ("0", "1-29", "30-59", "60+")
    ]
    fs.append(
        F(
            "With it the residual by specialty is flat: every band within 0.98-1.02",
            all(0.98 <= a_ <= 1.02 for _, _, a_ in bf),
            "; ".join(f"{b_} {x:.3f} -> {y:.3f}" for b_, x, y in bf),
        )
    )
    x2, dfc, pc = g["specialty cells: joint chi2"], g["specialty cells: df"], g["specialty cells: p"]
    fs.append(
        F(
            "After the specialty term the (scenario, band) cells with 200+ rows are jointly consistent with the model (run-clustered "
            "Wald chi2, p > 0.05)",
            pc > 0.05,
            f"chi2 {x2:.1f} on {dfc:.0f} cells, p = {pc:.3f}; largest single |z| {g['specialty cells max |z| after']:.2f} (descriptive)",
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
            "On the adopted model (a base and d per scenario, the specialty fit's rows) a free divisor is detectable in-sample (LR "
            "gain above 1.92 nats, the 1-parameter 5% bound) but does not predict unseen runs better than 1/100 (5-fold held out, "
            "z <= 1.96): 1/100 stays",
            gd > 1.92 and H("specialty model, free divisor vs 1/100")[2] <= 1.96,
            f"c = {g['specialty model: free divisor c']:.0f}; in-sample +{gd:.2f} nats, LR p = {chi2.sf(2 * gd, 1):.3f}; held out "
            + hs("specialty model, free divisor vs 1/100")
            + f" on {g['held out: specialty runs']:,.0f} runs",
        )
    )
    fs.append(
        F(
            "Later runs: fitted on the earlier half of the records, the specialty term predicts the later half better than no term "
            "(z > 1.96; the gain smaller than in-sample)",
            H("later runs: specialty term vs none")[2] > 1.96
            and g["specialty held-out gain, nats"] < g["specialty gain per scenario, nats"],
            "held out "
            + hs("later runs: specialty term vs none")
            + f" on {g['specialty held-out rows']:,.0f} rows; in-sample +{g['specialty gain per scenario, nats']:.1f}",
        )
    )
    fs.append(
        F(
            "Not shown to be causal: within the same card across limit breaks, on the clean cards (hint frequency the same at "
            "every limit break, so only the specialty term changes), d is not decisive either way: its 95% profile interval "
            "covers both 0 and every per-scenario d",
            g["specialty same-card clean: d 95% low"] <= 0 <= g["specialty same-card clean: d 95% high"]
            and all(
                g["specialty same-card clean: d 95% low"]
                <= g[f"specialty d {nm}"]
                <= g["specialty same-card clean: d 95% high"]
                for _, nm in SCENARIOS
            ),
            f"clean cards d {g['specialty same-card clean: d']:.2f} [{g['specialty same-card clean: d 95% low']:.2f}, "
            f"{g['specialty same-card clean: d 95% high']:.2f}] on {g['specialty same-card clean: cards']:.0f} cards, "
            f"{g['specialty same-card clean: gain, nats']:+.2f} nats; per-scenario d "
            + ", ".join(f"{g[f'specialty d {nm}']:.2f}" for _, nm in SCENARIOS)
            + f"; every card (hf also changes with limit break, confounded) d {g['specialty same-card: d']:.2f} "
            f"[{g['specialty same-card: d 95% low']:.2f}, {g['specialty same-card: d 95% high']:.2f}] on "
            f"{g['specialty same-card: cards']:.0f} cards",
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
class SpecialtyRows:
    """the specialty fit's rows (max-limit-break card-runs, rows no spark hinted, Lv-5 cap, R cards out) and its fits:
    h = base_s m, a base per scenario on top of a multiplier m per row (of hf and specialty)"""

    def __init__(self, A: Data) -> None:
        ci = A.ci
        rows = (A.lb[ci] == 4) & A.noinsp & (A.rar[ci] != 1)
        self.j = j = np.nonzero(rows)[0]
        self.c = c = ci[j]  # the rows, their card-runs
        self.tab = miss_table(np.ones(len(j), int), A.nskill[c], cap_draws(A.hinc[c]))
        self.y = A.hit[j]
        self.hfm = 1 + A.hf[c] / 100
        self.sp = A.spec[c]
        self.sc = A.scen[c]
        self.hfr = A.hf[c]
        self.cid = np.array([x.cid for x in A.obs])[c]
        self.seq = A.seq[c]
        self.rar = A.rar[c]
        self.run = np.array([A.obs[i].run for i in c])
        # every multiplier below is a function of hf and specialty: rows alike in those, the pool size and the increment
        # (the draw table) are predicted alike
        self.rc, self.rep = alike(A.hf[c], self.sp, A.nskill[c], A.hinc[c])
        self.tabc = self.tab[self.rep]
        self.tallies: dict[bytes, tuple[np.ndarray, np.ndarray, np.ndarray]] = {}
        self.every = np.ones(len(self.y), bool)

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

    def exp_gain(self, k: int) -> float:
        """scenario k's LL gain of the exponential form exp(g sp/10) over no term"""
        q = self.sc == k
        best = minimize_scalar(
            lambda g: -self.fit(q, self.hfm * np.exp(g * self.sp / 10))[0], bounds=(-0.1, 0.1), method="bounded"
        )
        return -best.fun - self.fit(q, self.hfm)[0]

    def scored(self, qa: Mask, qb: Mask, m: np.ndarray) -> np.ndarray:
        """rows qb's log-likelihoods under the base fitted on rows qa for multiplier m"""
        return row_ll(1 - mix(self.tab[qb], np.minimum(self.fit(qa, m)[1] * m[qb], TMAX - 1e-6)), self.y[qb])


SPEC_BANDS: tuple[tuple[int, int, str], ...] = ((0, 1, "0"), (1, 30, "1-29"), (30, 60, "30-59"), (60, 1000, "60+"))


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
    return PerScenarioD(d_s, bases, p0, p1, gain_s, l1s)


def specialty_residuals(S: SpecialtyRows, fit: PerScenarioD, out: Out) -> None:
    """obs / pred by specialty band before and after the term, and the (scenario, band) cells jointly"""
    y, sp, sc = S.y, S.sp, S.sc
    for a, b, nm in SPEC_BANDS:
        q = (sp >= a) & (sp < b)
        add(out, f"specialty band {nm} rows", q.sum())
        add(out, f"specialty band {nm} before", y[q].sum() / fit.p_without[q].sum())
        add(out, f"specialty band {nm} after", y[q].sum() / fit.p_with[q].sum())
    zcell = []
    cres, crun = [], []
    for k in (1, 2, 3, 4):
        for a, b, nm in SPEC_BANDS:
            q = (sc == k) & (sp >= a) & (sp < b)
            if q.sum() >= 200:  # z of obs / pred against 1, its standard error run-clustered (common.cluster_ci)
                r, _, hi, _ = common.cluster_ci(y[q], fit.p_with[q], S.run[q])
                zcell.append((r - 1) / ((hi - r) / 1.96))
                cres += [((k, nm), d) for d in y[q] - fit.p_with[q]]
                crun += list(S.run[q])
    add(
        out,
        "specialty cells max |z| after",
        max(abs(z) for z in zcell),
        note=f"{len(zcell)} (scenario, band) cells with 200+ rows, run-clustered; descriptive",
    )
    # the cells jointly: one Wald test, the covariance run-clustered (one run's rows fall in several cells)
    x2c, dfc, pc = common.cluster_chi2(cres, crun)
    add(
        out,
        "specialty cells: joint chi2",
        x2c,
        note="run-clustered Wald, (scenario, band) cells with 200+ rows, after the term",
    )
    add(out, "specialty cells: df", dfc)
    add(out, "specialty cells: p", pc)


def specialty(A: Data) -> list[common.Value]:
    """specialty priority (effect 19, at the card's level) on the fit's setup over every record: max-limit-break
    card-runs, rows no spark hinted, Lv-5 cap, R cards out. h = base_s (1 + hf/100) (1 + d spec/100)."""
    S = SpecialtyRows(A)
    y, sc, sp = S.y, S.sc, S.sp
    out: Out = []
    add(out, "specialty rows", len(S.j))
    add(out, "specialty card-runs", len(set(S.c)))
    for k, nm in SCENARIOS:
        add(out, f"specialty rows {nm}", (sc == k).sum())
        add(out, f"specialty runs {nm}", len(set(S.run[sc == k])))
    fit = per_scenario_d(S, out)
    dp, gp = S.pooled_d()
    add(out, "specialty gain one d, nats", gp)
    add(out, "specialty d pooled", dp)
    d1 = S.d_of(S.every)
    add(out, "one base and one d: base", S.fit(S.every, S.with_d(d1))[1])
    add(out, "one base and one d: d", d1)
    add(out, "specialty gain exp form, nats", sum(S.exp_gain(k) for k in (1, 2, 3, 4)))
    specialty_residuals(S, fit, out)
    u = (sc == 2) & (S.hfr == 0) & (sp == 0)
    add(out, "Unity hf-0 spec-0 before", y[u].sum() / fit.p_without[u].sum())
    add(out, "Unity hf-0 spec-0 after", y[u].sum() / fit.p_with[u].sum())
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
    five_fold(S, out)
    out += same_card(A)
    return out


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
    ix = np.searchsorted(np.unique(S.run[te]), S.run[te])
    add(out, "specialty held-out gain, nats", ho[True][te].sum() - ho[False][te].sum())
    add(out, "specialty held-out rows", te.sum())
    add(out, "specialty held-out fit rows", tr.sum())
    add(out, "specialty held-out first later record", cut, note="seq; records before it fit")
    out.extend(
        compared("later runs: specialty term vs none", *(np.bincount(ix, weights=ho[s][te]) for s in (True, False)))
    )


def five_fold(S: SpecialtyRows, out: Out) -> None:
    """5-fold: no term, one pooled d and a d per scenario (each with a base per scenario), and the free divisor, fitted
    on four folds"""

    def fold_spec(tr: Mask, te: Mask) -> dict[str, np.ndarray]:
        r = {m: np.zeros(len(S.y)) for m in ("none", "one d", "per-scenario d", "free divisor")}
        dp = S.pooled_d(tr)[0]
        hfc = 1 + S.slope_of(tr) * S.hfr
        for k in (1, 2, 3, 4):
            qa, qb = tr & (S.sc == k), te & (S.sc == k)
            if not qb.any():
                continue
            r["none"][qb] = S.scored(qa, qb, S.hfm)
            r["one d"][qb] = S.scored(qa, qb, S.with_d(dp))
            r["per-scenario d"][qb] = S.scored(qa, qb, S.with_d(S.d_of(qa)))
            r["free divisor"][qb] = S.scored(qa, qb, S.with_d(S.d_of(qa, base=hfc), hfc))
        return {m: v[te] for m, v in r.items()}

    hos = cv(S.run, fold_spec)
    add(out, "held out: specialty runs", len(hos["none"]))
    out.extend(compared("specialty term vs none", hos["per-scenario d"], hos["none"]))
    out.extend(compared("per-scenario d vs one d", hos["per-scenario d"], hos["one d"]))
    out.extend(compared("one d vs none", hos["one d"], hos["none"]))
    out.extend(compared("specialty model, free divisor vs 1/100", hos["free divisor"], hos["per-scenario d"]))


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


def same_card_fit(A: Data, cards: Sequence[int]) -> tuple[float, float, float, float]:
    """the given cards at every limit break, rows no spark hinted: a base per (card, scenario), one pooled d, draws capped
    at the card's own increment at its level. Returns (d, LL gain over no term, 95% profile interval low, high; an
    end the interval reaches is the end of SAME_CARD_D)"""
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

    def lw(d: float) -> float:
        """LL with a base per (card, scenario) group for the specialty term with this d"""
        mc = (hw * (1 + d * sw / 100))[wrep]
        return sum(
            -best_base(twc[q], mc[q], wh[q], wn[q], bounds=(0.05, 12), xatol=1e-6, clipped=True, grid=24).fun
            for q in gq
        )

    from scipy.optimize import brentq

    rw = minimize_scalar(lambda d: -lw(d), bounds=SAME_CARD_D, method="bounded", options={"xatol": 1e-3})
    d, best = rw.x, -rw.fun

    def short(x: float) -> float:
        """how far LL(x) lies above the 95% profile bound"""
        return lw(x) - (best - CHI1)

    lo, hi = SAME_CARD_D
    lo = brentq(short, lo, d, xtol=1e-3) if short(lo) < 0 else lo
    hi = brentq(short, d, hi, xtol=1e-3) if short(hi) < 0 else hi
    return d, best - lw(0.0), lo, hi


def same_card(A: Data) -> list[common.Value]:
    """the same card at different limit breaks (specialty seen at two or more values in the runs, R cards out): every
    such card, whose hint frequency mostly changes with limit break too (so its d also absorbs any misfit of the hf
    slope), and the clean cards among them (clean_cards: only the specialty term changes)"""
    out: Out = []
    lv = collections.defaultdict(set)
    for i, x in enumerate(A.obs):
        if x.rar != 1:
            lv[x.cid].add(A.spec[i])
    vary = sorted(k for k, v in lv.items() if len(v) > 1)
    clean = [k for k in vary if k in clean_cards(_game())]
    for label, cards in (("specialty same-card", vary), ("specialty same-card clean", clean)):
        d, gain, lo, hi = same_card_fit(A, cards)
        add(out, f"{label}: cards", len(cards))
        add(out, f"{label}: d", d)
        add(out, f"{label}: d 95% low", lo, note=f"profile likelihood; searched over {SAME_CARD_D}")
        add(out, f"{label}: d 95% high", hi)
        add(out, f"{label}: gain, nats", gain)
    return out


@functools.lru_cache(None)
def distinct_pmf(n: int, t: int) -> np.ndarray:
    """P(d distinct of n skills after t uniform draws with replacement), d = 0..t"""
    S = [[0] * (t + 1) for _ in range(t + 1)]
    S[0][0] = 1  # Stirling numbers of the second kind
    for i in range(1, t + 1):
        for k in range(1, i + 1):
            S[i][k] = k * S[i - 1][k] + S[i - 1][k - 1]
    return np.array([math.comb(n, d) * math.factorial(d) * S[t][d] / n**t for d in range(t + 1)])


if __name__ == "__main__":
    common.main(claims)
