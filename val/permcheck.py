"""Brute-force check of per-uma decks on a small instance: every foursome x every 6-card deck, each lineage's value
read from the shared solver (deck locked, loop required, character rule off), then per uma the best legal deck
over her three lineages. usage: permcheck.py BIN [K]"""
import json, itertools, subprocess, sys, os, concurrent.futures as cf
S = os.path.dirname(os.path.abspath(__file__)) + '/../pkg/loopopt/'
SC = os.path.dirname(os.path.abspath(__file__)) + '/../'
BIN, K = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 3
data = json.load(open(S + 'data.json')); cards = {c['id']: c for c in json.load(open(S + 'web_cards.json'))['cards']}
ends = {k for k, v in json.load(open(S + 'card_events.json'))['cards'].items() if 'end' in v}
uid = {u['name']: u['id'] for u in data['umas']}
UMAS = ['Agnes Digital', 'Tamamo Cross', 'Inari One', 'Oguri Cap', 'Gold Ship', 'King Halo']
def cid(frag): return next(i for i, c in cards.items() if frag in c['name'])
OWN = [cid('[Tracen Academy] Yaeno'), cid('Tonight, We Waltz'), cid('Sunlit Angel'), cid('From the Ground Up'), cid('This Might Sting'),
       cid('[Tracen Academy] Inari'), cid('Split the Sky'), cid('[Tracen Academy] Gold Ship')]
assert not any(str(c) in ends for c in OWN), 'pick cards without chain-ending choices'
base = json.load(open(S + 'jobs/pal_U.json')); base.update(owned=[uid[n] for n in UMAS], owned_cards=OWN)
env = dict(os.environ, DECK_RELAXED='1')
def run(job, e=None):
    p = SC + f'_pc_{os.getpid()}_{abs(hash(json.dumps(job, sort_keys=True)))}.json'
    json.dump(job, open(p, 'w')); out = subprocess.run([BIN, 'data.json', p], cwd=S, capture_output=True, text=True, env=e).stdout; os.remove(p)
    return json.loads(out[out.index('{'):])
def blocks(c, u): return cards[c]['chara'] == u or u in cards[c].get('charas', [])
decks = [d for d in itertools.combinations(OWN, 6) if len({cards[c]['chara'] for c in d}) == 6]
fours = list(itertools.combinations(UMAS, 4))
def lin_vals(args):
    F, D = args
    o = run(dict(base, require=[uid[n] for n in F], lock_cards=list(D), top_decks=1), env)
    t = o['top'][0]; L = t['loop']['detail']['lineages']; mem = [m['id'] for m in t['loop']['members']]
    return F, D, mem, [(l['t'], l['p_all']) for l in L]
with cf.ThreadPoolExecutor(2) as ex: rows = list(ex.map(lin_vals, [(F, D) for F in fours for D in decks]))
best = {}
for F, D, mem, L in rows:
    for t in range(4):
        if any(blocks(c, mem[t]) for c in D): continue
        s = sum(p for tt, p in L if tt == t) / 12
        key = (F, t)
        if s > best.get(key, (-1,))[0]: best[key] = (s, D, mem[t])
vals = sorted(((sum(best[(F, t)][0] for t in range(4)), F) for F in fours if all((F, t) in best for t in range(4))), reverse=True)
fast = run(dict(base, per_uma=True, top_decks=K))
print('foursomes', len(fours), 'decks', len(decks), 'per-uma searches', fast.get('searches'))
for i in range(K):
    b, f = vals[i], fast['top'][i]
    print(f'#{i+1} brute 1 in {1/b[0]:.3f} {b[1]}  fast 1 in {1/f["loop"]["p_all"]:.3f} {[m["name"] for m in f["loop"]["members"]]}',
          'OK' if abs(b[0] - f['loop']['p_all']) <= 1e-9 * b[0] else 'MISMATCH')
    if i == 0:
        names = {uid[n]: n for n in UMAS}
        for t in range(4):
            s, D, u = best[(b[1], t)]
            fd = f['decks'][[m['id'] for m in f['loop']['members']].index(u)]['card_ids'] if u in [m['id'] for m in f['loop']['members']] else None
            print('   ', names[u], 'brute', sorted(D) == sorted(fd or []), [cards[c]['name'] for c in D])
