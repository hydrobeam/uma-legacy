"""usage: owncheck.py BIN [K] [Required Uma,Other Uma]
Brute-force check of the character rule: every 6-card deck from a small owned set, each with its best legal
loop (deck locked, blocked umas excluded), against run_deck's branching answer. usage: owncheck.py BIN [K]"""
import tempfile, json, itertools, subprocess, sys, os, concurrent.futures as cf
S = os.path.dirname(os.path.abspath(__file__)) + '/../pkg/loopopt/'
BIN, K = sys.argv[1], int(sys.argv[2]) if len(sys.argv) > 2 else 3
REQ = sys.argv[3].split(',') if len(sys.argv) > 3 else []
data = json.load(open(S + 'data.json')); cards = {c['id']: c for c in json.load(open(S + 'web_cards.json'))['cards']}
uid = {u['name']: u['id'] for u in data['umas']}
UMAS = ['Agnes Digital', 'Tamamo Cross', 'Inari One', 'Oguri Cap', 'Yaeno Muteki', 'King Halo', 'Gold Ship', 'Special Week', 'Nice Nature']
def cid(frag): return next(i for i, c in cards.items() if frag in c['name'])
OWN = [cid('[Tracen Academy] Yaeno'), cid('Tonight, We Waltz'), cid('Sunlit Angel'), cid('From the Ground Up'), cid('This Might Sting'),
       cid('[Tracen Academy] Mr. C.B.'), cid('[Tracen Academy] Inari'), cid('Split the Sky'), cid('Team Sirius'), cid('[Tracen Academy] Gold Ship')]
base = json.load(open(S + 'jobs/pal_U.json')); base.update(owned=[uid[n] for n in UMAS], owned_cards=OWN, top_decks=K, require=[uid[n] for n in REQ])
def run(job):
    p = os.path.join(tempfile.gettempdir(), f'_oc_{os.getpid()}_{abs(hash(json.dumps(job, sort_keys=True)))}.json')
    json.dump(job, open(p, 'w')); out = subprocess.run([BIN, 'data.json', p], cwd=S, capture_output=True, text=True).stdout; os.remove(p)
    return json.loads(out[out.index('{'):])
def blocked(deck): return sorted({u for n in UMAS for u in [uid[n]] if any(cards[c]['chara'] == u or u in cards[c].get('charas', []) for c in deck)})
def one(deck):
    job = dict(base, lock_cards=list(deck), top_decks=1, exclude=blocked(deck))
    o = run(job)
    if 'error' in o or not o['top']: return (0.0, deck, None)
    t = o['top'][0]; return (t['loop']['p_all'], deck, [m['name'] for m in t['loop']['members']])
decks = [d for d in itertools.combinations(OWN, 6) if len({cards[c]['chara'] for c in d}) == 6]
with cf.ThreadPoolExecutor(4) as ex: res = sorted(ex.map(one, decks), key=lambda r: -r[0])
fast = run(base)
print('decks', len(decks), 'searches', fast.get('searches'))
for i in range(K):
    b, f = res[i], fast['top'][i] if i < len(fast['top']) else None
    fv = f['loop']['p_all'] if f else 0
    print(f'#{i+1} brute 1 in {1/b[0]:.2f} {b[2]} {[cards[c]["name"] for c in b[1]]}')
    print(f'    fast  1 in {1/fv:.2f} {[m["name"] for m in f["loop"]["members"]] if f else None} {f["deck"]["cards"] if f else None}')
    print('    ', 'OK' if abs(b[0] - fv) <= 1e-12 * b[0] else 'MISMATCH')
relaxed = run(dict(base, exclude=[]))
