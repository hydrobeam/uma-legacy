"""Compare a validation tag with a reference when every value is expected to scale: same loops and decks, and the
ratio of each top result's p_all / p_minus1. usage: cmp_ratio.py TAG REF"""
import json, sys, glob, os
V = os.path.dirname(os.path.abspath(__file__)) + '/'
tag, ref = sys.argv[1], sys.argv[2]
def load(p):
    t = open(p).read()
    try: return json.loads(t[t.index('{'):])
    except Exception: return None
rows = []
for rf in sorted(glob.glob(V + ref + '__*.out')):
    name = rf.split(ref + '__')[1]; nf = V + f'{tag}__{name}'
    if not os.path.exists(nf): continue
    r, x = load(rf), load(nf)
    if r is None or x is None: print(name, 'PARSE FAIL'); continue
    if 'top' in r:
        A = [(t['loop'], t['deck']) for t in r['top']]; B = [(t['loop'], t['deck']) for t in x['top']]
        mem = lambda l: tuple(m['name'] if isinstance(m, dict) else m for m in l['members'])
        same = [mem(a[0]) for a in A] == [mem(b[0]) for b in B] and [sorted(a[1]['card_ids']) for a in A] == [sorted(b[1]['card_ids']) for b in B]
        if 'decks' in r['top'][0]:
            same = same and [[sorted(d['card_ids']) for d in t['decks']] for t in r['top']] == [[sorted(d['card_ids']) for d in t['decks']] for t in x['top']]
        pa = B[0][0]['p_all'] / A[0][0]['p_all']; pm = B[0][0]['p_minus1'] / A[0][0]['p_minus1']
        rows.append((name.replace('.out', ''), same, 1 / A[0][0]['p_all'], 1 / B[0][0]['p_all'], pa, 1 / A[0][0]['p_minus1'], 1 / B[0][0]['p_minus1'], pm, x.get('proven')))
    else:
        mem = lambda l: tuple(m['name'] if isinstance(m, dict) else m for m in l['members'])
        A, B = r.get('loops', []), x.get('loops', [])
        same = [mem(l) for l in A] == [mem(l) for l in B]
        rows.append((name.replace('.out', ''), same, 1 / A[0]['p_all'], 1 / B[0]['p_all'], B[0]['p_all'] / A[0]['p_all'], 0, 0, 0, x.get('proven_top')))
print(f"{'setup':34s} same  all: ref -> new (ratio)        n-1: ref -> new (ratio)   proven")
for n, s, a0, a1, ra, m0, m1, rm, pv in rows:
    print(f"{n:34s} {'yes' if s else 'NO ':4s} 1 in {a0:6.0f} -> {a1:6.0f} (x{ra:.4f})   " + (f"1 in {m0:5.1f} -> {m1:5.1f} (x{rm:.4f})" if m0 else " " * 33) + f"  {pv}")
print(f"{len(rows)} compared; {sum(not r[1] for r in rows)} with different loops or decks; "
      f"p_all ratio range {min(r[4] for r in rows):.5f}..{max(r[4] for r in rows):.5f}")
