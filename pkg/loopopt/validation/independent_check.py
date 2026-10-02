"""Independent verifier for URA-scenario parent-loop affinity scoring.

Usage:
    from check import Checker
    ck = Checker()                       # loads master.mdb
    res = ck.score_foursome({1008: [[turn, pid], ...], 1009: [...], ...})
    print(ck.report(res))

CLI:  python3 check.py schedules.json   (JSON: {chara_id: [[turn, program_id], ...]})
      python3 check.py --selftest
"""
import json
import os
import sqlite3
import sys
from itertools import combinations

DB = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "master.mdb")

OFF = {8: 0, 7: 0, 6: 1, 5: 2, 4: 3, 3: 4, 2: 5, 1: 6}   # S,A=0 B=1 C=2 D=3 E=4 F=5 G=6
LETTER = {1: "G", 2: "F", 3: "E", 4: "D", 5: "C", 6: "B", 7: "A", 8: "S"}
PEN = {1: 0, 2: 0, 3: 10, 4: 25, 5: 35}  # 6+ = 50
RELOC = {1025: 1012, 1027: 1006, 1028: 1005}
YB = {"J": 0, "C": 24, "S": 48}
JBC_GOAL = {20002: 1025, 20003: 1026}   # pseudo condition_id -> base program family (Senior? turn given by goal row)


def pen(k):
    return PEN.get(k, 50)


def cat_of(distance):
    if distance <= 1400:
        return "short"
    if distance <= 1800:
        return "mile"
    if distance <= 2400:
        return "middle"
    return "long"


def turn_of(year, month, half):
    return YB[year] + (month - 1) * 2 + half


def years_for_perm(perm):
    return {1: ["J"], 2: ["C"], 3: ["C", "S"], 4: ["S"]}.get(perm, [])


class Checker:
    def __init__(self, db=DB):
        self.c = sqlite3.connect(f"file:{os.path.abspath(db)}?mode=ro", uri=True)
        q = self.c.execute
        self.name = {i: t for i, t in q("select [index],text from text_data where category=6")}
        self.race_name = {i: t for i, t in q("select [index],text from text_data where category=32")}
        # programs
        self.prog = {}
        for pid, base, perm, month, half, rid, grade, dist, ground in q(
            """select p.id,p.base_program_id,p.race_permission,p.month,p.half,r.id,r.grade,cs.distance,cs.ground
               from single_mode_program p join race_instance ri on p.race_instance_id=ri.id
               join race r on ri.race_id=r.id join race_course_set cs on r.course_set=cs.id"""):
            self.prog[pid] = dict(id=pid, base=base, perm=perm, month=month, half=half, race=rid,
                                  rkey=RELOC.get(rid, rid), grade=grade, dist=dist,
                                  cat=cat_of(dist), surf="turf" if ground == 1 else "dirt",
                                  turns=[turn_of(y, month, half) for y in years_for_perm(perm)])
        # G1 calendar: turn -> [pid]
        self.g1 = {}
        for p in self.prog.values():
            if p["grade"] == 100 and p["base"] == 0 and 1 <= p["perm"] <= 4:
                for t in p["turns"]:
                    self.g1.setdefault(t, []).append(p["id"])
        self.g1_bases = sorted({pid for v in self.g1.values() for pid in v})
        # relations
        self.rel_point = dict(q("select relation_type,relation_point from succession_relation"))
        self.rel_types = {}
        for rt, ch in q("select relation_type,chara_id from succession_relation_member"):
            self.rel_types.setdefault(ch, set()).add(rt)
        self._apt = {}

    # ---------- aptitude ----------
    def aptitude(self, chara):
        if chara in self._apt:
            return self._apt[chara]
        row = self.c.execute(
            "select id from card_data where chara_id=? and id<900000 order by id limit 1", (chara,)).fetchone()
        if not row:
            raise KeyError(f"no card for chara {chara}")
        r = self.c.execute(
            """select proper_distance_short,proper_distance_mile,proper_distance_middle,proper_distance_long,
                      proper_ground_turf,proper_ground_dirt from card_rarity_data
               where card_id=? order by rarity limit 1""", (row[0],)).fetchone()
        a = dict(card=row[0], short=r[0], mile=r[1], middle=r[2], long=r[3], turf=r[4], dirt=r[5])
        self._apt[chara] = a
        return a

    def apt_str(self, chara):
        a = self.aptitude(chara)
        return " ".join(f"{k}:{LETTER[a[k]]}" for k in ("short", "mile", "middle", "long", "turf", "dirt"))

    # ---------- goals ----------
    def goals(self, chara):
        """Return list of goal groups: [(sort_id, turn, [condition_id, ...]), ...] (determine_race=2 excluded)."""
        groups = {}
        for sort_id, turn, cid in self.c.execute(
            """select sort_id,turn,condition_id from single_mode_route_race
               where race_set_id=? and scenario_group_id=100 and condition_type=1 and turn<=72
                 and determine_race!=2 order by sort_id,id""", (chara,)):
            g = groups.setdefault(sort_id, [])
            g.append((turn, cid))
        out = []
        for sid, rows in sorted(groups.items()):
            turns = sorted({t for t, _ in rows})
            out.append((sid, turns, rows))
        return out

    def _goal_row_matches(self, gturn, cid, turn, pid):
        if turn != gturn:
            return False
        if cid in JBC_GOAL:
            fam = JBC_GOAL[cid]
            p = self.prog.get(pid)
            return p is not None and (pid == fam or p["base"] == fam)
        return pid == cid

    # ---------- feasibility ----------
    def available(self, chara, turn, pid):
        """(ok, how) whether program pid can be run by chara at turn."""
        p = self.prog.get(pid)
        if p is None:
            return False, "unknown program"
        for _, _, rows in self.goals(chara):
            for gt, cid in rows:
                if self._goal_row_matches(gt, cid, turn, pid):
                    return True, "goal"
        if turn not in p["turns"]:
            return False, f"program {pid} not held at turn {turn} (held at {p['turns']})"
        if p["base"] != 0:
            return False, f"program {pid} is a character-specific variant (base {p['base']}) not in this chara's goals"
        if p["grade"] == 100 and pid in self.g1.get(turn, []):
            return True, "G1"
        return True, "other"

    def feasible(self, chara, schedule):
        """schedule: list of (turn, program_id). Returns (ok, [problems])."""
        probs = []
        sched = sorted((int(t), int(p)) for t, p in schedule)
        seen = {}
        for t, pid in sched:
            if not 1 <= t <= 72:
                probs.append(f"turn {t} out of 1..72")
            if t in seen:
                probs.append(f"two races at turn {t}: {seen[t]} and {pid}")
            seen[t] = pid
            ok, how = self.available(chara, t, pid)
            if not ok:
                probs.append(f"turn {t} pid {pid}: {how}")
        for sid, turns, rows in self.goals(chara):
            if not any(self._goal_row_matches(gt, cid, t, pid) for gt, cid in rows for t, pid in sched):
                probs.append(f"goal sort {sid} unmet (needs one of {rows})")
        return (not probs), probs

    # ---------- probabilities ----------
    def attempts(self, chara, schedule):
        """List of (turn, pid, race_key, k, p)."""
        a = self.aptitude(chara)
        sched = sorted((int(t), int(p)) for t, p in schedule)
        out = []
        k = 0
        prev = None
        for t, pid in sched:
            k = k + 1 if prev is not None and t == prev + 1 else 1
            prev = t
            pr = self.prog[pid]
            v = 110 - 10 * OFF[a[pr["cat"]]] - 10 * OFF[a[pr["surf"]]] - pen(k)
            v = max(0, min(100, v)) / 100.0
            out.append((t, pid, pr["rkey"], k, v))
        return out

    def win_probs(self, chara, schedule, g1_only=False):
        miss = {}
        for t, pid, rk, k, p in self.attempts(chara, schedule):
            if g1_only and self.prog[pid]["grade"] != 100:
                continue
            miss[rk] = miss.get(rk, 1.0) * (1 - p)
        return {r: 1 - m for r, m in miss.items()}

    # ---------- relations ----------
    def pair(self, a, b):
        if a == b:
            return 0
        s = self.rel_types.get(a, set()) & self.rel_types.get(b, set())
        return sum(self.rel_point[t] for t in s)

    def tri(self, a, b, c):
        if len({a, b, c}) < 3:
            return 0
        s = self.rel_types.get(a, set()) & self.rel_types.get(b, set()) & self.rel_types.get(c, set())
        return sum(self.rel_point[t] for t in s)

    def base_score(self, F):
        F = list(F)
        sp = sum(self.pair(a, b) for a, b in combinations(F, 2))
        st = sum(self.tri(a, b, c) for a, b, c in combinations(F, 3))
        return 2 * sp + 4 * st, sp, st

    # ---------- foursome score ----------
    def score_foursome(self, schedules, g1_only=False):
        """schedules: {chara_id: [[turn, program_id], ...]} with exactly 4 members.
        Returns dict with SCF, components, per-member feasibility and per-pair overlaps."""
        S = {int(k): v for k, v in schedules.items()}
        F = sorted(S)
        assert len(F) == 4, "need exactly 4 members"
        feas = {c: self.feasible(c, S[c]) for c in F}
        P = {c: self.win_probs(c, S[c], g1_only) for c in F}
        ov = {}
        for a, b in combinations(F, 2):
            ov[(a, b)] = sum(P[a][r] * P[b][r] for r in P[a] if r in P[b])
        base, sp, st = self.base_score(F)
        so = sum(ov.values())
        return dict(members=F, feasible=all(f[0] for f in feas.values()), feas=feas, P=P,
                    overlap=ov, sum_pair=sp, sum_tri=st, base=base, sum_overlap=so,
                    SCF=base + 10 * so)

    def report(self, res, detail=True):
        L = []
        F = res["members"]
        L.append("Foursome: " + ", ".join(f"{c} {self.name.get(c, '?')}" for c in F))
        L.append(f"SCF = {res['SCF']:.4f}  (base {res['base']} = 2*{res['sum_pair']} + 4*{res['sum_tri']}; "
                 f"10*sum O = {10 * res['sum_overlap']:.4f})")
        L.append(f"All feasible: {res['feasible']}")
        for c in F:
            ok, probs = res["feas"][c]
            L.append(f"  {c} {self.name.get(c)} [{self.apt_str(c)}] feasible={ok}")
            for p in probs:
                L.append(f"     ! {p}")
        for (a, b), o in res["overlap"].items():
            L.append(f"  O({self.name.get(a)},{self.name.get(b)}) = {o:.4f}")
        if detail:
            for c in F:
                rows = ", ".join(f"{self.race_name.get(r, r)}:{p:.3f}" for r, p in sorted(res["P"][c].items()))
                L.append(f"  P[{self.name.get(c)}]: {rows}")
        return "\n".join(L)


def selftest():
    ck = Checker()
    V, D, P, A = 1008, 1009, 1014, 1019
    lab = {V: "V", D: "D", P: "P", A: "A"}
    exp_pair = {(V, D): 39, (V, P): 26, (V, A): 34, (D, P): 31, (D, A): 37, (P, A): 33}
    exp_tri = {(V, D, P): 23, (V, D, A): 28, (V, P, A): 25, (D, P, A): 30}
    allok = True
    for (a, b), e in exp_pair.items():
        g = ck.pair(a, b)
        allok &= g == e
        print(f"pair {lab[a]}-{lab[b]}: {g} (exp {e}) {'OK' if g == e else 'MISMATCH'}")
    for (a, b, c), e in exp_tri.items():
        g = ck.tri(a, b, c)
        allok &= g == e
        print(f"tri {lab[a]}{lab[b]}{lab[c]}: {g} (exp {e}) {'OK' if g == e else 'MISMATCH'}")
    base, sp, st = ck.base_score([V, D, P, A])
    print(f"base = 2*{sp} + 4*{st} = {base} (exp 824) {'OK' if base == 824 else 'MISMATCH'}")
    print("all relation checks:", "PASS" if allok and base == 824 else "FAIL")
    print(f"G1 calendar distinct base programs: {len(ck.g1_bases)}; calendar entries (turn,pid): "
          f"{sum(len(v) for v in ck.g1.values())}")
    for t in sorted(ck.g1):
        print(f"  t{t:2d}: " + ", ".join(f"{p} {ck.race_name.get(ck.prog[p]['race'])}" for p in ck.g1[t]))
    for c in (V, D, P, A):
        print(f"{c} {ck.name[c]} aptitudes: {ck.apt_str(c)} (card {ck.aptitude(c)['card']})")
    print("Daiwa Scarlet goals:")
    for sid, turns, rows in ck.goals(D):
        print(f"  sort {sid}: " + " | ".join(
            f"t{t} pid {cid} {ck.race_name.get(ck.prog[cid]['race']) if cid in ck.prog else cid}" for t, cid in rows))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] != "--selftest":
        with open(sys.argv[1]) as f:
            data = json.load(f)
        ck = Checker()
        g1o = "--g1only" in sys.argv
        print(ck.report(ck.score_foursome(data, g1_only=g1o)))
    else:
        selftest()
