"""Section 2: the star mix the loop model gives its legacies.

The mixes themselves (1/2/3-star shares 0.50/0.45/0.05 below rank score 17,500, 0.20/0.70/0.10 from 17,500) are
assumed inputs: the run records show only the legacies players chose, which are picked for their sparks, so the mix of
an unselected veteran cannot be read from them. What is computed here:
- the band each scenario's careers reach: the run list's rank-score estimate with every skill point spent
  (research/data/summary.jsonl, not included; Independent Training runs from uma-it), share at 17,500 or more per
  scenario, against the band the page picks for the scenario (webapp/src/data.js, `ss`);
- the expected base per roll b = 0.03 (pi_1 + 2 pi_2 + 3 pi_3) for each mix (the mixes read from webapp/src/data.js
  RANKS, the solver's default from pkg/loopopt/src/skills.rs);
- what replacing a legacy's stars by that expected base costs: per run of the records (research/data/runs.jsonl), with
  its six real affinities (s05_affinity), q with b in each of the two rolls against the exact
  1 - prod_l E_s[(1 - b(s)(1 + A_l / 100))^2] (a legacy's stars are the same in both rolls), for one target carried by
  all six legacies.
"""
import math, os, re, statistics
import common
from common import Finding, Value
import s05_affinity

SEC: str = '2'
CUT: int = 17500

# a star mix: the shares of 1, 2 and 3-star legacies
Mix = tuple[float, ...]


def mixes() -> tuple[dict[str, Mix], dict[int, bool], Mix]:
    src = open(os.path.join(common.ROOT, 'webapp', 'src', 'data.js')).read()
    rk = dict(re.findall(r"(\w+): '([\d.]+,[\d.]+,[\d.]+)'", re.search(r'RANKS = \{(.*?)\}', src).group(1)))
    mix = {k: tuple(float(x) for x in v.split(',')) for k, v in rk.items()}
    ss = {int(i): b == 'true' for i, b in re.findall(r"id: (\d+),[^}]*?ss: (true|false)", src)}
    rs = open(os.path.join(common.ROOT, 'pkg', 'loopopt', 'src', 'skills.rs')).read()
    dflt = tuple(float(x) for x in re.search(r'fn dstar\(\) -> \[f64; 3\] \{ \[([^\]]*)\]', rs).group(1).split(','))
    return mix, ss, dflt


def bbar(pi: Mix) -> float: return 0.03 * (pi[0] + 2 * pi[1] + 3 * pi[2])


def claims() -> list[Value | Finding]:
    mix, ss, dflt = mixes()
    out: list[Value | Finding] = [Value(SEC, 'base per roll, below 17,500', bbar(mix['below'])), Value(SEC, 'base per roll, 17,500 and up', bbar(mix['ss']))]
    out.append(common.finding(SEC, 'The solver\'s default mix is the page\'s below-17,500 mix', dflt == mix['below'],
                              f'solver {dflt}, page {mix["below"]}'))
    L = common.run_list()
    if L is not None:
        bad: list[str] = []
        for sid, name in common.SCEN.items():
            v = [x['score_ceiling'] for x in L if x['scenario_id'] == sid and x['score_ceiling']]
            k = sum(x >= CUT for x in v); lo, hi = common.wilson(k, len(v))
            out += [Value(SEC, f'{name}: runs', len(v)), Value(SEC, f'{name}: median rank score', statistics.median(v)),
                    Value(SEC, f'{name}: share at 17,500 or more', k / len(v)),
                    Value(SEC, f'{name}: share 95% low', lo), Value(SEC, f'{name}: share 95% high', hi)]
            if (lo > 0.5) != ss[sid] or (hi < 0.5) == ss[sid]: bad.append(f'{name} {k / len(v):.3f} [{lo:.3f}, {hi:.3f}], page SS {ss[sid]}')
        out.append(common.finding(SEC, 'Each scenario\'s band is the one most of its careers reach: the share at 17,500 or more is '
                                  'above one half (95% interval) exactly for the scenarios the page puts at 17,500 and up',
                                  not bad, '; '.join(bad) or 'every scenario agrees'))
    R = common.runs()
    if R is None: return out
    S, _ = s05_affinity.relations()
    ratios: dict[str, list[float]] = {k: [] for k in mix}
    for r in R:
        anc = s05_affinity.lineage(r); aff = r.get('affinity') or {}
        if anc is None or any(l not in aff for l in list(s05_affinity.GLBL.values()) + ['parent 1 × parent 2']): continue
        T = r['chara']; A = [a['chara'] for a in anc]
        if len({T, A[0], A[3]}) < 3: continue
        Aff = s05_affinity.six_affinities(T, A, aff, S)
        for k, pi in mix.items():
            b = bbar(pi)
            miss_loop = math.prod((1 - min(1.0, b * (1 + a / 100))) ** 2 for a in Aff.values())
            miss_ex = math.prod(sum(w * (1 - min(1.0, 0.03 * (s + 1) * (1 + a / 100))) ** 2 for s, w in enumerate(pi)) for a in Aff.values())
            ratios[k].append((1 - miss_loop) / (1 - miss_ex))
    lab = {'below': 'below 17,500', 'ss': '17,500 and up'}
    out.append(Value(SEC, 'lineages', len(ratios['below'])))
    for k, v in ratios.items():
        out += [Value(SEC, f'q overstatement, {lab[k]}: mean', statistics.mean(v) - 1),
                Value(SEC, f'q overstatement, {lab[k]}: max', max(v) - 1)]
    out.append(common.finding(SEC, 'Replacing stars by their expected base never understates q (convexity): every lineage\'s '
                              'ratio is at least 1', all(x >= 1 - 1e-12 for v in ratios.values() for x in v),
                              ', '.join(f'{lab[k]} min {min(v):.6f}' for k, v in ratios.items())))
    return out


if __name__ == '__main__': common.main(claims)
