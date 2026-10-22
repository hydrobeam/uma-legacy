"""Recompute the solver's exact reference fixtures with an independent MILP (scipy/HiGHS).

usage: uv run val/milp_fixtures.py [--check]

pkg/loopopt/tests/fixtures/ holds best responses (br_milp.json, br_rest_milp.json) and pair ceilings (pb_milp.json)
on the frozen data_v1.json; `cargo test` checks the exact DP against them. This script solves each case as a MILP
written from the model (docs/MODEL.md, win chance in IT), not from the solver's code, and rewrites the fixtures' values
(the cases themselves are kept); --check only reports the largest difference. Run it after a change to the win model.

One uma's schedule: at each turn rest or one option; y[t, o, k] = 1 when option o at turn t is the k-th race in a row
(k = 1..6, 6 = 6th or later: any option counts, a rest resets). An option wins with clamp(bp - PEN[s], 0, 100) / 100, s
its scored position: k, one less for a goal option from the 2nd in a row. Single-turn goals force one of their options
at that turn; a multi-turn goal needs one of its options. Objective sum_r W[r] P(r), P(r) = 1 - prod(1 - p) over the
attempts at r: linear for a race offered once; for one offered twice (always a Classic/Senior pair) the product term
p1 p2 [both run at k1, k2] is a variable z >= y1 + y2 - 1, which the negative coefficient keeps at its lower bound.
Each value is the solved schedule's objective, evaluated directly.
"""

import json, sys
from pathlib import Path
from typing import Any, NamedTuple
import numpy as np
import scipy.sparse as sp
from scipy.optimize import Bounds, LinearConstraint, milp

FIXTURES = Path(__file__).resolve().parent.parent / "pkg/loopopt/tests/fixtures"
TURNS = 72
MAX_STREAK = 6


class Choice(NamedTuple):
    """one decision variable: option `option` at turn `turn` (0-based) as the `k`-th race in a row"""

    turn: int
    option: int
    k: int


def scored_position(goal: bool, k: int) -> int:
    """the streak position whose penalty an option at position k takes: a goal race from the 2nd in a row the previous"""
    return k - 1 if goal and k >= 2 else k


def win_chance(option: dict[str, Any], k: int, pen: list[int]) -> float:
    return min(100, max(0, option["bp"] - pen[scored_position(option.get("goal", False), k) - 1])) / 100


def schedule_value(uma: dict[str, Any], schedule: dict[int, int], weights: dict[int, float], pen: list[int]) -> float:
    """sum_r W[r] (1 - prod(1 - p)) over the schedule's attempts (turn -> option), streak positions counted directly"""
    miss: dict[int, float] = {}
    k = 0
    for turn in range(TURNS):
        if turn not in schedule:
            k = 0
            continue
        k = min(k + 1, MAX_STREAK)
        option = uma["turns"][turn][schedule[turn]]
        if option["race"] >= 0:
            miss[option["race"]] = miss.get(option["race"], 1.0) * (1 - win_chance(option, k, pen))
    return sum(weights.get(race, 0.0) * (1 - m) for race, m in miss.items())


def best_response(
    uma: dict[str, Any], weights: dict[int, float], pen: list[int], force: tuple[int, int | None] | None = None
) -> float:
    """the best schedule's value; `force` = (turn, option), option None for a forced rest"""
    turns = uma["turns"]
    choices = [Choice(t, o, k) for t in range(TURNS) for o in range(len(turns[t])) for k in range(1, MAX_STREAK + 1)]
    index = {c: i for i, c in enumerate(choices)}
    offered: dict[int, list[tuple[int, int]]] = {}
    for t in range(TURNS):
        for o, option in enumerate(turns[t]):
            if option["race"] >= 0:
                offered.setdefault(option["race"], []).append((t, o))
    pairs = [(race, k1, k2) for race, at in offered.items() if len(at) == 2 for k1 in range(1, 7) for k2 in range(1, 7)]
    n = len(choices) + len(pairs)
    objective = np.zeros(n)
    for i, c in enumerate(choices):
        option = turns[c.turn][c.option]
        if option["race"] >= 0:
            objective[i] = weights.get(option["race"], 0.0) * win_chance(option, c.k, pen)
    rows: list[dict[int, float]] = []
    low: list[float] = []
    high: list[float] = []

    def add(row: dict[int, float], lo: float, hi: float) -> None:
        rows.append(row)
        low.append(lo)
        high.append(hi)

    at_position = lambda t, k: [index[Choice(t, o, k)] for o in range(len(turns[t]))]
    at_turn = lambda t: [i for k in range(1, MAX_STREAK + 1) for i in at_position(t, k)]
    for t in range(TURNS):
        add(dict.fromkeys(at_turn(t), 1.0), 0, 1)
        if t == 0:
            for k in range(2, MAX_STREAK + 1):
                add(dict.fromkeys(at_position(t, k), 1.0), 0, 0)
            continue
        add(dict.fromkeys(at_position(t, 1), 1.0) | dict.fromkeys(at_turn(t - 1), 1.0), 0, 1)  # 1st only after a rest
        for k in range(2, MAX_STREAK + 1):
            previous = at_position(t - 1, k - 1) + (at_position(t - 1, k) if k == MAX_STREAK else [])
            add(dict.fromkeys(at_position(t, k), 1.0) | dict.fromkeys(previous, -1.0), -np.inf, 0)
    single: dict[int, set[int]] = {}
    for goal in uma["goals"]:
        goal_turns = {t for t, _ in goal}
        if len(goal_turns) == 1:
            t = goal_turns.pop() - 1
            allowed = {o for _, o in goal}
            single[t] = single[t] & allowed if t in single else allowed
        else:
            add({index[Choice(t - 1, o, k)]: 1.0 for t, o in goal for k in range(1, MAX_STREAK + 1)}, 1, np.inf)
    for t, allowed in single.items():
        add({index[Choice(t, o, k)]: 1.0 for o in allowed for k in range(1, MAX_STREAK + 1)}, 1, 1)
    if force is not None:
        t, o = force
        if o is None:
            add(dict.fromkeys(at_turn(t), 1.0), 0, 0)
        else:
            add({index[Choice(t, o, k)]: 1.0 for k in range(1, MAX_STREAK + 1)}, 1, 1)
    for j, (race, k1, k2) in enumerate(pairs):
        (t1, o1), (t2, o2) = offered[race]
        first, second = turns[t1][o1], turns[t2][o2]
        objective[len(choices) + j] = -weights.get(race, 0.0) * win_chance(first, k1, pen) * win_chance(second, k2, pen)
        add({len(choices) + j: 1.0, index[Choice(t1, o1, k1)]: -1.0, index[Choice(t2, o2, k2)]: -1.0}, -1, np.inf)
    entries = [(r, col, value) for r, row in enumerate(rows) for col, value in row.items()]
    r_, c_, v_ = zip(*entries)
    matrix = sp.csr_array((v_, (r_, c_)), shape=(len(rows), n))
    result = milp(
        -objective,
        constraints=LinearConstraint(matrix, low, high),
        integrality=np.r_[np.ones(len(choices)), np.zeros(len(pairs))],
        bounds=Bounds(0, 1),
        options={"mip_rel_gap": 1e-12, "time_limit": 600},
    )
    assert result.status == 0, f"{uma['name']}: {result.message}"
    schedule = {c.turn: c.option for c, x in zip(choices, result.x) if x > 0.5}
    return schedule_value(uma, schedule, weights, pen)


def best_case(uma: dict[str, Any], pen: list[int]) -> dict[int, float]:
    """per race, 1 - prod(1 - p at the 1st race in a row) over every offering: the partner weights of a pair ceiling"""
    miss: dict[int, float] = {}
    for options in uma["turns"]:
        for option in options:
            if option["race"] >= 0:
                miss[option["race"]] = miss.get(option["race"], 1.0) * (1 - win_chance(option, 1, pen))
    return {race: 1 - m for race, m in miss.items()}


def option_of(uma: dict[str, Any], turn: int, program: int) -> int:
    return next(o for o, option in enumerate(uma["turns"][turn - 1]) if option["prog"] == program)


def main() -> None:
    check = "--check" in sys.argv[1:]
    data = json.loads((FIXTURES / "data_v1.json").read_text())
    pen, umas = data["pen"], {u["id"]: u for u in data["umas"]}
    worst = 0.0
    for name in ("br_milp.json", "br_rest_milp.json"):
        cases = json.loads((FIXTURES / name).read_text())
        for case in cases:
            uma = umas[case["uma"]]
            weights = dict(case["weights"])
            if "force_rest" in case:
                force = (case["force_rest"] - 1, None)
            elif case["force"]:
                force = (case["force"][0] - 1, option_of(uma, *case["force"]))
            else:
                force = None
            value = round(best_response(uma, weights, pen, force), 8)
            worst = max(worst, abs(value - case["milp_value"]))
            case["milp_value"] = value
        print(f"{name}: {len(cases)} cases")
        if not check:
            (FIXTURES / name).write_text(json.dumps(cases))
    ceilings = json.loads((FIXTURES / "pb_milp.json").read_text())
    for entry in ceilings:
        a, b = umas[entry[0]], umas[entry[1]]
        value = min(best_response(a, best_case(b, pen), pen), best_response(b, best_case(a, pen), pen))
        worst = max(worst, abs(value - entry[2]))
        entry[2] = value
    print(f"pb_milp.json: {len(ceilings)} pairs; largest change {worst:.6g}")
    if not check:
        (FIXTURES / "pb_milp.json").write_text(json.dumps(ceilings))


if __name__ == "__main__":
    main()
