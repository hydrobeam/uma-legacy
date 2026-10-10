"""Section 2, star mix of legacy sparks: the expected base per roll (derived) and each scenario's share of runs
reaching rank score 17,500 if every skill point is spent (the summary records' `score_ceiling`), which picks each
scenario's default band. The star-mix shares themselves are an external fit with no data here."""
import statistics
import common
from common import Claim

S = '2'
MIX = {'below 17,500': (0.50, 0.45, 0.05), '17,500+': (0.20, 0.70, 0.10)}


def claims():
    out = [Claim(S, f'b-bar {band}', st, 0.03 * (p1 + 2 * p2 + 3 * p3)) for (band, (p1, p2, p3)), st in zip(MIX.items(), ('0.0465', '0.057'))]
    stated = {1: ('0%', None, None), 2: ('47%', '17,428', None), 3: ('79%', '18,094', None), 4: ('0%', '12,386', '282')}
    rows = common.summary()
    for sc, (share, med, n) in stated.items():
        name = common.SCEN[sc]
        if rows is None:
            out += common.need_data(S, [(f'{name}: share at 17,500+', share)] + ([(f'{name}: median score', med)] if med else []))
            continue
        v = [r['score_ceiling'] for r in rows if r['scenario_id'] == sc and r['score_ceiling'] is not None]
        out.append(Claim(S, f'{name}: share at 17,500+', share, sum(x >= 17500 for x in v) / len(v)))
        if med: out.append(Claim(S, f'{name}: median score', med, statistics.median(v)))
        if n: out.append(Claim(S, f'{name}: runs', n, len(v)))
    return out


if __name__ == '__main__': common.main(claims)
