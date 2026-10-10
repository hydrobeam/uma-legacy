"""Write a loop's veterans in umadump's trained_chara_data.json format (github.com/Werseter/umadump), so another
tool (uma.moe's lineage planner) can compute affinity and spark chances for the same lineages, and print what our
model gives for exactly those veterans.

usage: umadump_lineage.py OUT_JSON LOOP_OUTPUT [EXPECT_MD]
  LOOP_OUTPUT  a deck/skills job output whose first loop has four members (e.g. a deck job with `sizes: [4]`; the
               reference setups' best results are three-uma rotations, which this script does not handle)
Every member gets one veteran per pair of the other three as its parents (12 veterans): lineage (T, P1, P2, X) uses
P1 bred from (P2, X) and P2 bred from (P1, X), as in the solver's `scf` scheme. Each veteran and each of its parents carries
all 8 target skill sparks at fixed stars (STARS). A scheduled G1 counts as won when its win chance is >= 50%.
"""
import json, sqlite3, sys, os, itertools
S = os.path.dirname(os.path.abspath(__file__)) + '/../'
out_path = sys.argv[1]
loop_path = sys.argv[2]
md_path = sys.argv[3] if len(sys.argv) > 3 else None
D = json.load(open(S + 'pkg/loopopt/data.json'))
db = sqlite3.connect(S + 'pkg/loopopt/export/master.mdb'); q = lambda s, *a: db.execute(s, a).fetchall()
TOP = json.load(open(loop_path))['top'][0]; L = TOP['loop']
assert len(L['members']) == 4, f"{loop_path}: the first loop has {len(L['members'])} members; this script needs four"
DECKS = TOP.get('decks') or ([TOP['deck']] * 4 if 'deck' in TOP else [{'card_ids': []}] * 4)
members = [m['id'] for m in L['members']]
UI = {u['id']: i for i, u in enumerate(D['umas'])}
idx = [UI[m] for m in members]
short = [m['short'] for m in L['members']]

# ---- races won, from the schedule chart ----
ABBR = {"Tokyo Yushun (Japanese Derby)": "Japanese Derby", "Asahi Hai Futurity Stakes": "Asahi Hai FS",
        "Mile Championship Nambu Hai": "MCS Nambu Hai", "Hanshin Juvenile Fillies": "Hanshin JF",
        "Zen-Nippon Junior Yushun": "Zen-Nippon Jr Yushun"}
chart = lambda n: ABBR.get(n, n[:22])
MON = {m: i + 1 for i, m in enumerate("Jan Feb Mar Apr May Jun Jul Aug Sep Oct Nov Dec".split())}
def turn_of(when):
    y, eh = when.split(); e, m = eh.split('-')
    return {'Jr': 0, 'Cl': 24, 'Sr': 48}[y] + (MON[m] - 1) * 2 + (1 if e == 'E' else 2)
ri_of = dict(q("select id, race_instance_id from single_mode_program"))
g1_saddle = {}   # race_instance -> (saddle id, saddle group) for single-race G1 saddles
for sid, grp, ri in q("""select s.id, s.group_id, s.race_instance_id_1 from single_mode_wins_saddle s
        join race_instance r on r.id = s.race_instance_id_1 join race x on x.id = r.race_id
        where s.race_instance_id_2 = 0 and x.grade = 100"""):
    g1_saddle[ri] = (sid, grp)
titles = [(r[0], [x for x in r[5:13] if x]) for r in q("select * from single_mode_wins_saddle where race_instance_id_2 != 0")]

won = [[] for _ in members]; ran = [[] for _ in members]   # (turn, program)
for row in L['schedule']:
    t = turn_of(row['when'])
    names = row['race'].split('/'); prim = [n for n in names if n not in row['alts']]
    prim = prim[0] if len(names) > 1 else names[0]
    for i, c in enumerate(row['cells']):
        if c is None: continue
        ops = [o for o in D['umas'][idx[i]]['turns'][t - 1] if o['race'] >= 0 and chart(D['race_names'][o['race']]) == prim]
        assert len(ops) == 1, (short[i], row)
        ran[i].append((t, ops[0]['prog'], c['p']))
        if c['p'] >= 50: won[i].append((t, ops[0]['prog']))
def saddles(i):
    ris = {ri_of[p] for _, p in won[i]}
    single = sorted({g1_saddle[r][0] for r in ris})
    by_year = [{ri_of[p] for t, p in won[i] if (t - 1) // 24 == y} for y in range(3)]
    title = sorted(sid for sid, rs in titles if any(set(rs) <= yr for yr in by_year))   # titles: all in one year
    return single, title
SAD = [saddles(i) for i in range(4)]
G1G = {sid: grp for ri, (sid, grp) in g1_saddle.items()}
def shared_g1(a, b):   # uma-it-optimizer / our rule: single-race G1 saddles, by saddle group
    return len({G1G[s] for s in SAD[a][0]} & {G1G[s] for s in SAD[b][0]})
def shared_all(a, b):  # every shared saddle id, titles included
    return len(set(SAD[a][0] + SAD[a][1]) & set(SAD[b][0] + SAD[b][1]))

# ---- veterans ----
outfit = []
for i, o in enumerate(L['outfits']):
    outfit.append(o['outfits'][0]['id'] if o['outfits'] else members[i] * 100 + 1)
# unique skill of an outfit: its rarity-3 skill set's first skill (falls back to the base-outfit id pattern)
unique = []
for c in outfit:
    r = q("select s.skill_id1 from card_rarity_data c join skill_set s on s.id = c.skill_set where c.card_id = ? and c.rarity = 3", c)
    unique.append(r[0][0] if r else 100000 + (c // 100 - 1000) * 10 + c % 100)
SKILLS = [  # (target, learned skill id, spark factor group, stars)
    ("Uma Stan", 201591, 20159, 2), ("No Stopping Me!", 200491, 20049, 2), ("Playtime's Over!", 201661, 20166, 2),
    ("It's On!", 200461, 20046, 2), ("Tail Held High", 201611, 20161, 1), ("Racing Spirit: Stamina", 210101, 21009, 3),
    ("Slipstream", 201651, 20165, 2), ("Risky Business", 202032, 20203, 2)]
STAR_BASE = {1: 0.03, 2: 0.06, 3: 0.09}
BLUE = [103, 203, 103, 203]; PINK = [3202, 1202, 3302, 3202]
def factors(i):
    return ([{"factor_id": BLUE[i], "level": 0}, {"factor_id": PINK[i], "level": 0},
             {"factor_id": outfit[i] * 100 + 2, "level": 0}]
            + [{"factor_id": g * 100 + s, "level": 0} for _, _, g, s in SKILLS])
def anc(pos, i):
    return {"position_id": pos, "card_id": outfit[i], "rank": 16, "rarity": 3, "talent_level": 5,
            "factor_info_array": factors(i), "factor_extend_array": [],
            "win_saddle_id_array": SAD[i][0] + SAD[i][1], "owner_viewer_id": 0}
vets = []; vid = {}
for m in range(4):
    others = [x for x in range(4) if x != m]
    for a, b in itertools.combinations(others, 2):
        tid = len(vets) + 1; vid[(m, frozenset((a, b)))] = tid
        vets.append({
            "viewer_id": 100000000001, "trained_chara_id": tid, "owner_viewer_id": 0, "owner_trained_chara_id": 0,
            "single_mode_chara_id": 0, "chara_seed": 0, "card_id": outfit[m],
            "succession_trained_chara_id_1": 0, "succession_trained_chara_id_2": 0, "use_type": 0,
            "speed": 1100, "stamina": 800, "power": 900, "wiz": 600, "guts": 500, "fans": 300000,
            "rank_score": 12500, "rank": 16, "scenario_id": 2, "route_id": 0, "arrive_route_race_id": 0,
            "proper_ground_turf": 7, "proper_ground_dirt": 7, "proper_running_style_nige": 5,
            "proper_running_style_senko": 7, "proper_running_style_sashi": 7, "proper_running_style_oikomi": 5,
            "proper_distance_short": 7, "proper_distance_mile": 7, "proper_distance_middle": 7, "proper_distance_long": 7,
            "succession_num": 0, "rarity": 3, "is_saved": 1, "is_locked": 1, "talent_level": 5, "race_cloth_id": 0,
            "chara_grade": 8, "running_style": 2, "nickname_id": 0, "wins": len(won[m]),
            "register_time": "2026-10-02 00:00:%02d" % tid, "create_time": "2026-10-02 00:00:%02d" % tid,
            "skill_array": [{"skill_id": unique[m], "level": 1}]
                           + [{"skill_id": s, "level": 1} for _, s, _, _ in SKILLS],
            "support_card_list": [{"position": k + 1, "support_card_id": c, "exp": 0, "limit_break_count": 4}
                                  for k, c in enumerate(DECKS[m]["card_ids"])],
            "race_result_list": [{"turn": t, "program_id": p, "weather": 1, "ground_condition": 1, "running_style": 2,
                                  "popularity": 0, "result_rank": 1 if pc >= 50 else 2, "result_time": 0, "prize_money": 0}
                                 for t, p, pc in ran[m]],
            "win_saddle_id_array": SAD[m][0] + SAD[m][1], "nickname_id_array": [],
            "factor_info_array": factors(m), "factor_extend_array": [],
            "succession_chara_array": [anc(10, a), anc(20, b)],
            "icon_type": 0, "memo": f"{short[m]} <- {short[a]} + {short[b]}",
        })
json.dump(vets, open(out_path, 'w'), indent=1, ensure_ascii=False)

# ---- our model on exactly these veterans ----
def rel_tables(d):
    """pair / trio relation tables from data.json: stored tables (old files) or rebuilt from its relation groups."""
    if 'tri' in d: return d['pair'], d['tri']
    n = len(d['umas']); P = [[0] * n for _ in range(n)]; T = [[[0] * n for _ in range(n)] for _ in range(n)]
    for pts, m in d['relations']:
        for a in m:
            for b in m:
                if a == b: continue
                P[a][b] += pts
                for c in m:
                    if c != a and c != b: T[a][b][c] += pts
    return P, T
PAIR, TRI = rel_tables(D)
pr = lambda a, b: PAIR[idx[a]][idx[b]]
tr = lambda a, b, c: TRI[idx[a]][idx[b]][idx[c]]
def lineage(t, p1, p2, x, o):
    slots = []
    for par, other in ((p1, p2), (p2, p1)):
        rel = pr(t, par) + pr(p1, p2) + tr(t, par, other) + tr(t, par, x)
        race = 3 * o(par, other) + 3 * o(par, other) + 3 * o(par, x)
        slots.append(("parent", short[par], rel, race))
        for g in (other, x):
            slots.append((f"gp {short[g]} under {short[par]}", short[g], tr(t, par, g), 3 * o(par, g)))
    return slots
def q_of(slots, base):
    miss = 1.0
    for *_, rel, race in slots: miss *= 1 - min(1.0, base * (1 + (rel + race) / 100))
    return 1 - miss ** 2
LIN = [(t, *[p for p in range(4) if p not in (t, x)], x) for t in range(4) for x in range(4) if x != t]
lines = [f"# Model check: {', '.join(short)} (veterans in {os.path.basename(out_path)})", "",
         "Wins (chance >= 50% counts as won), single-race G1 saddles / title saddles:", ""]
for i in range(4):
    lines.append(f"- {short[i]} ({outfit[i]}): {len(SAD[i][0])} G1 saddles {SAD[i][0]}; titles {SAD[i][1]}")
lines += ["", "Shared G1s by pair (our rule: single-race G1 saddles by group) and every shared saddle id:", ""]
for a, b in itertools.combinations(range(4), 2):
    lines.append(f"- {short[a]}-{short[b]}: pair {pr(a, b)}, shared G1 {shared_g1(a, b)}, all saddles {shared_all(a, b)}")
lines += ["", "Per legacy: affinity = relation points + race points; spark chance per inspiration = base x (1 + aff/100);",
          "q = chance a given target fires from at least one of the 6 legacies over 2 inspirations.", ""]
for t, p1, p2, x in LIN:
    a = vid[(p1, frozenset((p2, x)))]; b = vid[(p2, frozenset((p1, x)))]
    lines.append(f"## Trainee {short[t]}; parents veteran #{a} ({short[p1]} <- {short[p2]} + {short[x]}) "
                 f"and #{b} ({short[p2]} <- {short[p1]} + {short[x]})")
    tot = lambda o: (pr(t, p1) + pr(t, p2) + pr(p1, p2) + tr(t, p1, p2) + tr(t, p1, x) + tr(t, p2, p1) + tr(t, p2, x)
                     + 3 * (o(p1, p2) + o(p1, x) + o(p2, p1) + o(p2, x)) + 3 * o(p1, p2))
    sym = lambda v: "double circle" if v > 150 else "circle" if v > 50 else "triangle"
    lines.append("")
    dl = next((l for l in L['detail']['lineages'] if l['t'] == t and l['x'] == x), None)
    if dl: lines.append(f"Optimizer's own figure for this lineage (win chances instead of wins, star mix 45/50/5, "
                        f"base {L['detail']['base']:.3f}): q = {dl['q']:.4f}; parents' affinity "
                        f"{dl['slots'][0]['aff']:.1f} / {dl['slots'][3]['aff']:.1f}. Same formula at base 0.048 "
                        f"on these veterans: q = {q_of(lineage(t, p1, p2, x, shared_g1), L['detail']['base']):.4f}.")
    lines.append(f"Displayed lineage total (game / calculators): {tot(shared_g1)} ({sym(tot(shared_g1))}) under the G1 rule, "
                 f"{tot(shared_all)} under the all-saddle rule.")
    lines.append("")
    lines.append("| legacy | relation | race (G1 rule) | affinity | race (all-saddle rule) | affinity |")
    lines.append("|---|---|---|---|---|---|")
    s1 = lineage(t, p1, p2, x, shared_g1); s2 = lineage(t, p1, p2, x, shared_all)
    for (k, who, rel, r1), (_, _, _, r2) in zip(s1, s2):
        lines.append(f"| {who if k == 'parent' else k} | {rel} | {r1} | {rel + r1} | {r2} | {rel + r2} |")
    lines.append("")
    lines.append("| spark | stars | per-inspiration chance, parents (G1 rule) | q (G1 rule) | q (all-saddle rule) |")
    lines.append("|---|---|---|---|---|")
    for name, _, _, st in SKILLS:
        b0 = STAR_BASE[st]
        pp = [f"{b0 * (1 + (rel + r) / 100):.4f}" for k, _, rel, r in s1 if k == 'parent']
        lines.append(f"| {name} | {st} | {' / '.join(pp)} | {q_of(s1, b0):.4f} | {q_of(s2, b0):.4f} |")
    lines.append("")
txt = "\n".join(lines) + "\n"
if md_path: open(md_path, 'w').write(txt)
print(txt)
