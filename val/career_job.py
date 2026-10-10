"""Build a career-planner job from a veteran export (umadump's trained_chara_data.json), the way the career page
reads one: each veteran's white skill sparks (factor_id = factor group * 100 + stars, factor group -> skill group from
career.json), its single-race G1 wins (win saddle -> race index, career.json) and the same for its two parents
(succession_chara_array positions 10 and 20). A veteran counts as borrowed when owner_viewer_id is set and is not the
exporting player's viewer_id.

usage: career_job.py TRAINED_CHARA_JSON BASE_JOB OUT_JOB [TABLES]
  BASE_JOB  a deck job (targets, cards, events files...); its mode becomes "career"
  TABLES    career.json (default pkg/loopopt/data/global/career.json)"""
import json, sys, os
S = os.path.dirname(os.path.abspath(__file__)) + '/../'
vets, base_path, out_path = json.load(open(sys.argv[1])), sys.argv[2], sys.argv[3]
T = json.load(open(sys.argv[4] if len(sys.argv) > 4 else S + 'pkg/loopopt/data/global/career.json'))
F, W = {int(k): v for k, v in T['factors'].items()}, {int(k): v for k, v in T['saddles'].items()}

def sparks(fs):
    out = {}
    for f in fs:
        g = F.get(f['factor_id'] // 100)
        if g is not None: out[g] = max(out.get(g, 0), f['factor_id'] % 100)
    return sorted(out.items())
P = {int(k) for k in T.get('pinks', {})}
def pinks(fs): return sorted({f['factor_id'] // 100: f['factor_id'] % 100 for f in fs if f['factor_id'] // 100 in P}.items())
def wins(ws): return sorted({W[w] for w in ws if w in W})
def anc(x): return {'chara': x['card_id'] // 100, 'sparks': sparks(x['factor_info_array']), 'pinks': pinks(x['factor_info_array']), 'wins': wins(x['win_saddle_id_array'])}

legs = []
for v in vets:
    a = anc(v)
    a['borrowed'] = bool(v.get('owner_viewer_id')) and v.get('owner_viewer_id') != v.get('viewer_id')
    a['parents'] = [anc(p) for p in sorted(v['succession_chara_array'], key=lambda p: p['position_id']) if p['position_id'] in (10, 20)]
    legs.append(a)
job = json.load(open(base_path)); job['mode'] = 'career'; job['legacies'] = legs
json.dump(job, open(out_path, 'w'))
print(len(legs), 'veterans,', sum(l['borrowed'] for l in legs), 'borrowed')
