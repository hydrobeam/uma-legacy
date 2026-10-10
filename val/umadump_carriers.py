"""Two test veterans (umadump format) whose 8 target sparks sit on 1..6 of the trainee's six ancestors, so a lineage
planner's "Potential Inheritance" table shows its spark-generation chance for each carrier count.

usage: umadump_carriers.py LOOP_VETERANS OUT_A OUT_B
  LOOP_VETERANS  output of umadump_lineage.py for the four-uma loop Oguri Cap, Agnes Digital, Tamamo Cross and Inari
                 One (any outfits; a deck output whose first loop is those four, e.g. with `sizes: [4]`)
  OUT_A  P1 = Inari (parents Digital, Tamamo), P2 = Tamamo (parents Digital, Inari); pick Oguri as the target
  OUT_B  the same plus great-grandparents (positions 11/12/21/22 inside each veteran) carrying Nimble Navigator,
         to see whether a tool counts them as carriers or for affinity
"""
import json, sys, copy
src, out_a, out_b = sys.argv[1:4]
V = json.load(open(src))
by = {v['memo']: v for v in V}
p1 = copy.deepcopy(by['Inari <- Digital + Tamamo']); p2 = copy.deepcopy(by['Tamamo <- Digital + Inari'])
# legacy slots: P1, its parents (positions 10, 20), P2, its parents
slot = {'P1': (p1, None), 'GP1-1': (p1, 0), 'GP1-2': (p1, 1), 'P2': (p2, None), 'GP2-1': (p2, 0), 'GP2-2': (p2, 1)}
TEST = [  # (spark, factor group, carrier slots)
    ("Nimble Navigator", 20049, ['P1']),
    ("Playtime's Over!", 20166, ['P1', 'P2']),
    ("Ramp Up", 20046, ['P1', 'P2', 'GP1-1']),
    ("Tail Held High", 20161, ['P1', 'P2', 'GP1-1', 'GP2-1']),
    ("Racing Spirit: Stamina", 21009, ['P1', 'P2', 'GP1-1', 'GP1-2', 'GP2-1']),
    ("Slipstream", 20165, ['P1', 'P2', 'GP1-1', 'GP1-2', 'GP2-1', 'GP2-2']),
    ("Uma Stan", 20159, ['GP2-2']),                  # 1 carrier, a grandparent (vs Nimble Navigator on a parent)
    ("Risky Business", 20203, ['GP1-1', 'GP1-2']),   # 2 carriers, both grandparents (vs Playtime's Over! on parents)
]
WHITE = {g for _, g, _ in TEST}
def holder(s):
    v, k = slot[s]; return v if k is None else v['succession_chara_array'][k]
for s in slot:   # strip every target spark, keep blue/pink/green
    h = holder(s); h['factor_info_array'] = [f for f in h['factor_info_array'] if f['factor_id'] // 100 not in WHITE]
for _, g, carriers in TEST:
    for s in carriers: holder(s)['factor_info_array'].append({"factor_id": g * 100 + 2, "level": 0})
for i, (v, name) in enumerate(((p1, 'P1 Inari'), (p2, 'P2 Tamamo'))):
    v['trained_chara_id'] = 901 + i; v['memo'] = f"TEST carriers {name}"; v['is_locked'] = 0
json.dump([p1, p2], open(out_a, 'w'), indent=1, ensure_ascii=False)

# B: great-grandparents. Inari's parents' parents and Tamamo's parents' parents, from loop members other than Oguri.
gg_of = {'Digital': ('Tamamo', 'Inari'), 'Tamamo': ('Digital', 'Inari'), 'Inari': ('Digital', 'Tamamo')}
card = {'Digital': 101901, 'Tamamo': 102101, 'Inari': 103401}
proto = {e['card_id']: copy.deepcopy(e) for m in by for e in by[m]['succession_chara_array']}
name_of = {v: k for k, v in card.items()}
for v in (p1, p2):
    add = []
    for k, base in ((0, 10), (1, 20)):
        gp = name_of[v['succession_chara_array'][k]['card_id']]
        for j, gg in enumerate(gg_of[gp]):
            e = copy.deepcopy(proto[card[gg]]); e['position_id'] = base + 1 + j
            e['factor_info_array'] = [f for f in e['factor_info_array'] if f['factor_id'] // 100 not in WHITE] + [{"factor_id": 2004902, "level": 0}]
            add.append(e)
    v['succession_chara_array'] += add
for i, v in enumerate((p1, p2)): v['trained_chara_id'] = 911 + i; v['memo'] = v['memo'].replace('TEST', 'TEST+GGP')
json.dump([p1, p2], open(out_b, 'w'), indent=1, ensure_ascii=False)

print("carriers per spark (A):")
for n, _, c in TEST: print(f"  {n:24s} {len(c)}  {', '.join(c)}")
print("\nchance a learned skill becomes a spark, by carrier count")
print("  n   x1.1 per carrier (white / dbl-circle / gold)   +2 points per carrier (white)")
for n in range(7):
    m = 1.1 ** n
    print(f"  {n}   {20 * m:5.2f} / {25 * m:5.2f} / {40 * m:5.2f}                     {20 + 2 * n:5.2f}")
