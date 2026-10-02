"""Best 4-uma loop for landing a target set of skill sparks on one veteran.

Model (per training run = one SCF lineage with trainee T):
  - every target skill's spark sits on all 6 legacies at STARS stars (steady state);
  - a skill is acquired if it is guaranteed (deck/scenario), innate to T's card (need_rank <= AWAKENING),
    or its inherited spark procs at least once over INSPIRATIONS inspiration events;
  - proc chance per legacy = base[stars] * (1 + slot affinity/100), grandparents x GP_RATE;
  - acquired skill becomes a white spark with GEN_GOLD if the gold version is bought, else GEN_WHITE
    (uma.guide table at 6 appearances).
  Metric: P(all targets land on the veteran), averaged over the 12 lineages of the loop.
Search: exact upper bound per foursome (pair overlaps replaced by the pairwise ceilings PB), then
best-first: optimize real schedules (loopopt, gp_weight 0.5) in descending-bound order until the
next bound is below the k-th best achieved value.
"""
import json, sqlite3, subprocess, itertools, sys, os
import numpy as np

HERE = os.path.dirname(os.path.abspath(__file__))
CRATE = os.path.join(HERE, '..', 'pkg', 'loopopt')
MDB = os.path.join(HERE, '..', 'master.mdb')

AWAKENING = 5
STARS = int(os.environ.get("STARS", 2))
GP_STARS = int(os.environ.get("GP_STARS", STARS))       # grandparents' spark stars (default: same as parents)
BASE_WHITE = {1: 0.03, 2: 0.06, 3: 0.09}[STARS]
BASE_WHITE_GP = {1: 0.03, 2: 0.06, 3: 0.09}[GP_STARS]
# optional: legacies' spark stars drawn from a rating band's distribution, e.g. "0.45,0.50,0.05" (B-SS).
# Slots are independent and proc is linear in the base rate, so the expected base rate is exact for P(no proc).
if os.environ.get('STAR_DIST'):
    sd = [float(x) for x in os.environ['STAR_DIST'].split(',')]
    BASE_WHITE = BASE_WHITE_GP = sum(p * b for p, b in zip(sd, (0.03, 0.06, 0.09)))
GP_RATE = 0.5
INSPIRATIONS = 2
GEN_WHITE, GEN_GOLD = 0.35, 0.70
TOP = int(os.environ.get('TOP', 5))

# (label, skill group, guaranteed version or None)
TARGETS = [
    ("Uma Stan", 20159, None),
    ("Nimble Navigator", 20049, "gold"),
    ("Playtime's Over!", 20166, "gold"),
    ("Ramp Up", 20046, "gold"),
    ("Tail Held High", 20161, None),
    ("Racing Spirit: Stamina", 21010, None),
    ("Slipstream", 20165, "gold"),
    ("Risky Business", 20203, None),
]
# optional override: python3 skills.py targets.json  ([[label, group, null|"white"|"gold"], ...])
if len(sys.argv) > 1: TARGETS = [tuple(t) for t in json.load(open(sys.argv[1]))]
OUT = os.environ.get('OUT', os.path.join(HERE, 'skills_result.json'))

data = json.load(open(os.path.join(CRATE, 'data.json')))
ids = [u['id'] for u in data['umas']]
names = {u['id']: u['name'] for u in data['umas']}
n = len(ids)
P = np.array(data['pair'], dtype=float)
T3 = np.array(data['tri'], dtype=float)
pbj = json.load(open(os.path.join(HERE, 'pb.json')))
assert pbj['ids'] == ids
PB = np.array(pbj['pb'])

# ---- innate skills per card -------------------------------------------------------------
c = sqlite3.connect(MDB)
sk = {i: (r, g) for i, r, g in c.execute("select id, rarity, group_id from skill_data")}
cardname = dict(c.execute('select "index", text from text_data where category=5'))
# guaranteed golds are the same for every trainee; everything else depends on the trainee's card
uncertain = [t[1] for t in TARGETS if t[2] != 'gold']
guaranteed_white = {t[1] for t in TARGETS if t[2] == 'white'}
fixed = [t[2] for t in TARGETS if t[2] == 'gold']
cards = {i: [] for i in ids}           # chara id -> [(card_id, label, {group: 'gold'|'white'})]
for card_id, chara, ass in c.execute("select id, chara_id, available_skill_set_id from card_data"):
    if chara not in cards: continue
    cov = {}
    for sid, rank in c.execute("select skill_id, need_rank from available_skill_set where available_skill_set_id=?", (ass,)):
        if rank > AWAKENING or sid not in sk: continue
        r, g = sk[sid]
        if g in uncertain:
            v = 'gold' if r >= 2 else 'white'
            if cov.get(g) != 'gold': cov[g] = v
    for g in guaranteed_white: cov.setdefault(g, 'white')   # deck guarantees the white; an innate gold still upgrades it
    cards[chara].append((card_id, cardname.get(card_id, str(card_id)), cov))
for ch in cards:
    if not cards[ch]: cards[ch] = [(0, 'no card', {g: 'white' for g in guaranteed_white})]
# undocumented guarantees: EXTRA="chara:group:white|gold,..." applies to every outfit of that chara
for item in filter(None, os.environ.get('EXTRA', '').split(',')):
    ch, g, lvl = item.split(':'); ch, g = int(ch), int(g)
    if g in uncertain:
        for _, _, cv in cards[ch]:
            if cv.get(g) != 'gold': cv[g] = lvl

# per chara: distinct coverage tuples over the uncertain skills -> the cards that give them
DECK = {t[1]: (t[3] if len(t) > 3 and t[3] else 0.0) for t in TARGETS}   # optional 4th field: chance the deck hints the white
DECK_GOLD = {t[1]: (t[4] if len(t) > 4 and t[4] else 0.0) for t in TARGETS}  # optional 5th field: chance the deck hints the gold
STAT = {'gold': 2, 'white': 1}
chara_opts = []
for ch in ids:
    opts = {}
    for cid, lab, cv in cards[ch]:
        opts.setdefault(tuple(STAT.get(cv.get(g), 0) for g in uncertain), []).append((cid, lab))
    chara_opts.append(opts)
fixed_factor = np.prod([GEN_GOLD if v == 'gold' else GEN_WHITE for v in fixed])
NU = len(uncertain)

def skill_probs(st, q):
    """per-uncertain-skill P(spark lands) for coverage tuple st; q = per-career inheritance proc."""
    out = []
    for v, g in zip(st, uncertain):
        if v == 2: out.append(GEN_GOLD); continue
        white = GEN_WHITE if v == 1 else GEN_WHITE * (1 - (1 - DECK[g]) * (1 - q))
        out.append(DECK_GOLD[g] * GEN_GOLD + (1 - DECK_GOLD[g]) * white)
    return out

def trainee_factor(t_idx, q, with_choice=False):
    """max over T's cards of prod over uncertain skills; q = per-career proc (array)."""
    best = None; arg = None
    for st in chara_opts[t_idx]:
        v = 1.0
        for p in skill_probs(st, q): v = v * p
        if not np.ndim(q): v = float(v)
        if best is None: best, arg = v, st
        elif np.ndim(q): best = np.maximum(best, v)
        elif v > best: best, arg = v, st
    return (best, arg) if with_choice else best

# SCF lineages over positions 0..3: (t, p1, p2, x); P1 bred from (P2, X), P2 from (P1, X)
LIN = []
for t in range(4):
    for x in range(4):
        if x == t: continue
        p1, p2 = [i for i in range(4) if i not in (t, x)]
        LIN.append((t, p1, p2, x))

def lineage_q(F, O):
    """F: (N,4) uma indices, O(a,b) -> (N,) overlap arrays. Returns q (N,12) per-career proc of a 2* x6 spark."""
    qs = []
    for t, p1, p2, x in LIN:
        a, b1, b2, xx = F[:, t], F[:, p1], F[:, p2], F[:, x]
        o12 = O(b1, b2)
        shared = P[b1, b2] / 2 + 1.5 * o12
        miss = np.ones(len(F))
        for par, other in ((b1, b2), (b2, b1)):
            ap = P[a, par] + shared
            miss *= 1 - np.minimum(1, BASE_WHITE * (1 + ap / 100))
            for g in (other, xx):
                ag = T3[a, par, g] + 3 * O(par, g)
                miss *= 1 - np.minimum(1, GP_RATE * BASE_WHITE_GP * (1 + ag / 100))
        qs.append(1 - miss ** INSPIRATIONS)
    return np.stack(qs, 1)

def metric_from_q(F, q):
    vals = np.zeros(len(F))
    for li, (t, p1, p2, x) in enumerate(LIN):
        tf = np.zeros(len(F))
        for ti in np.unique(F[:, t]):
            m = F[:, t] == ti
            tf[m] = trainee_factor(ti, q[m, li])
        vals += tf
    return fixed_factor * vals / len(LIN)

# ---- 1. bound every foursome ----------------------------------------------------------------
allF = np.array(list(itertools.combinations(range(n), 4)), dtype=np.int16)
bounds = np.zeros(len(allF))
CH = 100000
for s in range(0, len(allF), CH):
    F = allF[s:s + CH].astype(np.int64)
    q = lineage_q(F, lambda a, b: PB[a, b])
    bounds[s:s + CH] = metric_from_q(F, q)
EXCL = [int(x) for x in os.environ.get('EXCLUDE', '').split(',') if x]
for e in EXCL:
    bounds[(allF == ids.index(e)).any(1)] = -1.0
order = np.argsort(-bounds)
print(f"bounded {len(allF)} foursomes", file=sys.stderr)

# ---- 2. best-first with real schedules ---------------------------------------------------------
def run_crate(job):
    p = '/tmp/skills_job.json'
    json.dump(job, open(p, 'w'))
    r = subprocess.run([os.path.join(CRATE, 'target/release/loopopt'), os.path.join(CRATE, 'data.json'), p], capture_output=True, text=True, check=True)
    return json.loads(r.stdout)

def actual(Fs):
    jobs = [{"members": [ids[i] for i in f]} for f in Fs]
    res = run_crate({"scheme": "scf", "gp_weight": 0.5, "iters": 30, "seed": 3, "threads": 2, "jobs": jobs})
    out = []
    for f, r in zip(Fs, res):
        mem = [m['id'] for m in r['members']]
        init = [m['schedule'] for m in r['members']]
        leg = run_crate({"mode": "legacy", "scheme": "scf", "iters": 1, "seed": 1, "jobs": [{"members": mem, "init": [init]}]})[0]
        # q per lineage from the real slot affinities
        idx = {ids[i]: i for i in range(n)}
        qs, tfs, dist_ps, picks = [], [], [], {}
        for L in leg['lineages']:
            miss = 1.0
            for s_ in L['slots']:
                rr = (GP_RATE * BASE_WHITE_GP if s_['grandparent'] else BASE_WHITE) * (1 + s_['affinity'] / 100)
                miss *= 1 - min(1, rr)
            q = 1 - miss ** INSPIRATIONS
            ti = idx[L['trainee']]
            v, st = trainee_factor(ti, q, with_choice=True)
            qs.append(q); tfs.append(v)
            dist_ps.append([GEN_GOLD] * len(fixed) + skill_probs(st, q)); picks.setdefault(L['trainee'], []).append(st)
        def atleast(ps, k):
            d = [1.0]
            for p in ps:
                nn = [0.0] * (len(d) + 1)
                for i, x in enumerate(d): nn[i] += x * (1 - p); nn[i + 1] += x * p
                d = nn
            return sum(d[k:])
        nt = len(TARGETS)
        outfits = {}
        for ch, sts in picks.items():
            st = max(set(sts), key=sts.count)
            cs = chara_opts[idx[ch]][st]
            outfits[names[ch]] = 'any outfit' if st == tuple(1 if g in guaranteed_white else 0 for g in uncertain) else ' or '.join(lab + ('' if cid % 100 == 1 else ' (alt)') for cid, lab in cs)
        out.append(dict(members=mem, value=fixed_factor * sum(tfs) / len(tfs), p_all_minus1=sum(atleast(p, nt - 1) for p in dist_ps) / len(dist_ps),
                        q=qs, trainees=[L['trainee'] for L in leg['lineages']], outfits=outfits, affinity=r['value'], schedules=init))
    return out

done = []
nxt = 0
BATCH = 8
def kth():
    if len(done) < TOP: return -1
    return sorted((d['value'] for d in done), reverse=True)[TOP - 1]
while nxt < len(order) and bounds[order[nxt]] >= kth():
    batch = [allF[order[i]].astype(int).tolist() for i in range(nxt, min(nxt + BATCH, len(order))) if bounds[order[i]] >= kth()]
    if not batch: break
    bmap = {tuple(sorted(ids[i] for i in allF[order[j]])): float(bounds[order[j]]) for j in range(nxt, nxt + len(batch))}
    nxt += len(batch)
    for r in actual(batch):
        r['bound'] = bmap[tuple(sorted(r['members']))]
        assert r['value'] <= r['bound'] + 1e-12, (r['members'], r['value'], r['bound'])
        done.append(r)
        print(f"  bound {r['bound']:.3e}  actual {r['value']:.3e} (1 in {1/r['value']:.0f}, >=n-1: 1 in {1/r['p_all_minus1']:.1f})  {' / '.join(names[m] + ' ' + r['outfits'][names[m]] for m in r['members'])}", file=sys.stderr)
done.sort(key=lambda d: -d['value'])
json.dump({"targets": TARGETS, "awakening": AWAKENING, "stars": STARS, "searched": len(done), "foursomes": len(allF),
           "next_bound": float(bounds[order[nxt]]) if nxt < len(order) else None,
           "cards": {names[ch]: [(cid, lab, cv) for cid, lab, cv in cards[ch] if any(v == 'gold' or g not in guaranteed_white for g, v in cv.items())] for ch in ids},
           "top": done[:max(TOP, 10)]}, open(OUT, 'w'), indent=1, default=str)
print(json.dumps({names[ch]: [(lab, cv) for _, lab, cv in cards[ch] if cv] for ch in ids if any(cv for _, _, cv in cards[ch])}, indent=0))
