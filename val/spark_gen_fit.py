"""Measure spark generation (the chance a skill learned in a career becomes a white spark on the veteran) from a real
umadump trained_chara_data.json, by the version bought and by how many of the career's six ancestors carried that
spark. Fits "x m per carrier" against "+a points per carrier".

usage: spark_gen_fit.py TRAINED_CHARA_DATA_JSON [MASTER_MDB] [--scenario N]   (MASTER_MDB default pkg/loopopt/export/master.mdb)
Ancestors: positions 10/20 are the veteran's parents. Its grandparents are positions 11/12/21/22 when the dump has
them; otherwise a parent is matched to an owned veteran with the same card, sparks and win saddles, and that
veteran's parents are used. Skill-observations whose six ancestors are not all known are reported but not fitted.
"""
import json, math, sqlite3, sys, collections, os
args = [a for a in sys.argv[1:] if not a.startswith('--')]
scen = int(sys.argv[sys.argv.index('--scenario') + 1]) if '--scenario' in sys.argv else None
V = json.load(open(args[0]))
db = sqlite3.connect(args[1] if len(args) > 1 else os.path.dirname(os.path.abspath(__file__)) + '/../pkg/loopopt/export/master.mdb')
q = lambda s, *a: db.execute(s, a).fetchall()

# skill -> white spark factor group, and the tier it was bought at
sk_group = dict(q("select id, group_id from skill_data"))
sk_rarity = dict(q("select id, rarity from skill_data"))
sk_name = dict(q("select [index], text from text_data where category = 47"))
white_fg = {}   # skill group -> factor group of its white spark
for fg, sid in q("select distinct f.factor_group_id, e.value_1 from succession_factor f join succession_factor_effect e"
                 " on e.factor_group_id = f.factor_group_id where f.factor_type = 4 and e.target_type = 41"):
    if sid in sk_group: white_fg.setdefault(sk_group[sid], fg)
def spark_of(sid):
    g = sk_group.get(sid)
    if g is None or g not in white_fg or sk_rarity.get(sid, 9) > 2: return None
    tier = 'gold' if sk_rarity[sid] == 2 else 'dbl' if '◎' in sk_name.get(sid, '') else 'white'
    return white_fg[g], tier
fgs = lambda factors: {f['factor_id'] // 100 for f in factors}

# owned veterans by fingerprint, to recover grandparents
fp = lambda e: (e['card_id'], tuple(sorted(f['factor_id'] for f in e['factor_info_array'])), tuple(sorted(e['win_saddle_id_array'])))
own = collections.defaultdict(list)
for v in V: own[fp(v)].append(v)

obs = []; partial = 0; skipped_v = 0
for v in V:
    if scen is not None and v.get('scenario_id') != scen: continue
    anc = {e['position_id']: e for e in v['succession_chara_array']}
    if 10 not in anc or 20 not in anc: skipped_v += 1; continue
    for p, (a, b) in ((10, (11, 12)), (20, (21, 22))):
        if a in anc and b in anc: continue
        m = own.get(fp(anc[p]), [])
        if len(m) == 1:
            ga = {e['position_id']: e for e in m[0]['succession_chara_array']}
            if 10 in ga and 20 in ga: anc[a], anc[b] = ga[10], ga[20]
    full = all(k in anc for k in (10, 11, 12, 20, 21, 22))
    mine = fgs(v['factor_info_array'])
    seen = set()
    for s in v['skill_array']:
        r = spark_of(s['skill_id'])
        if not r or r[0] in seen: continue
        seen.add(r[0]); fg, tier = r
        if not full: partial += 1; continue
        npar = sum(fg in fgs(anc[k]['factor_info_array']) for k in (10, 20))
        ngp = sum(fg in fgs(anc[k]['factor_info_array']) for k in (11, 12, 21, 22))
        obs.append((tier, npar, ngp, fg in mine))
print(f"veterans: {len(V)}; skill-observations with all six ancestors known: {len(obs)}; without: {partial}; "
      f"veterans with no parents stored: {skipped_v}")
if not obs: sys.exit()

def wilson(k, n, z=1.96):
    if n == 0: return (0, 0)
    p = k / n; d = 1 + z * z / n; c = p + z * z / (2 * n); h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return ((c - h) / d, (c + h) / d)
T = collections.defaultdict(lambda: [0, 0])
for tier, npar, ngp, hit in obs: T[(tier, npar + ngp)][0] += hit; T[(tier, npar + ngp)][1] += 1
print("\ncarriers  tier    n      sparked  rate    95% CI          x1.1^n model  +2/carrier model")
base = {'white': 0.20, 'dbl': 0.25, 'gold': 0.40}
for tier in ('white', 'dbl', 'gold'):
    for n in range(7):
        k, N = T[(tier, n)]
        if N == 0: continue
        lo, hi = wilson(k, N)
        print(f"{n:8d}  {tier:6s} {N:6d}  {k:7d}  {k / N:6.3f}  [{lo:.3f}, {hi:.3f}]  {base[tier] * 1.1 ** n:12.3f}  {base[tier] + 0.02 * n:16.3f}")

# maximum likelihood on the (tier, carriers) table: p = b_tier * m^n  vs  p = b_tier + a*n (grid search)
def llp(k, N, p):
    p = min(max(p, 1e-9), 1 - 1e-9); return k * math.log(p) + (N - k) * math.log(1 - p)
tiers = sorted({o[0] for o in obs})
def ll(f): return sum(llp(k, N, f(t, n)) for (t, n), (k, N) in T.items() if N)
def fit(kind):
    best = None
    steps = [x / 1000 for x in range(0, 301, 2)] if kind == 'add' else [1 + x / 1000 for x in range(0, 301, 2)]
    for st in steps:
        g = (lambda b, n: b * st ** n) if kind == 'mul' else (lambda b, n: b + st * n)
        bs = {t: max((b / 1000 for b in range(50, 801, 2)),
                     key=lambda b: sum(llp(*T[(t, n)], g(b, n)) for n in range(7) if T[(t, n)][1])) for t in tiers}
        v = ll(lambda t, n: g(bs[t], n))
        if best is None or v > best[0]: best = (v, st, bs)
    return best
for kind, label in (('mul', 'x m per carrier'), ('add', '+a per carrier')):
    v, st, bs = fit(kind)
    print(f"\n{label}: step {st:.3f}, bases " + ", ".join(f"{t} {b:.3f}" for t, b in bs.items()) + f", log-likelihood {v:.1f}")
fixed = ll(lambda t, n: base[t] * 1.1 ** n); fixed2 = ll(lambda t, n: base[t] + 0.02 * n)
print(f"\nlog-likelihood of the published curves: 20/25/40% x 1.1^n {fixed:.1f}; 20/25/40% + 2 points per carrier {fixed2:.1f}")
print("(a log-likelihood gap above ~2 per extra parameter is a real preference; below that the data can't tell them apart)")

# do parents and grandparents count the same? white tier, by (parents carrying, grandparents carrying)
print("\nwhite tier by parents carrying / grandparents carrying (rate, n):")
P = collections.defaultdict(lambda: [0, 0])
for tier, a, g, hit in obs:
    if tier == 'white': P[(a, g)][0] += hit; P[(a, g)][1] += 1
for a in range(3):
    print("  parents %d: " % a + "  ".join(f"gp{g} {P[(a, g)][0] / P[(a, g)][1]:.3f} ({P[(a, g)][1]})" for g in range(5) if P[(a, g)][1]))

# star mix of skill sparks by the veteran's rank (the inheritance base rate is 3/6/9% for 1/2/3 stars)
print("\nwhite skill spark stars by veteran rank code (share of 1/2/3 stars, n; expected base rate per spark):")
R = collections.defaultdict(lambda: [0, 0, 0])
for v in V:
    if scen is not None and v.get('scenario_id') != scen: continue
    for f in v['factor_info_array']:
        if 2000000 <= f['factor_id'] < 3000000 and f['factor_id'] % 100 in (1, 2, 3): R[v['rank']][f['factor_id'] % 100 - 1] += 1
for r in sorted(R):
    c = R[r]; n = sum(c)
    print(f"  rank {r:3d}: {c[0] / n:.2f} / {c[1] / n:.2f} / {c[2] / n:.2f}  (n {n}; base {(3 * c[0] + 6 * c[1] + 9 * c[2]) / n:.2f}%)")
