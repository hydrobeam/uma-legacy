"""Section 3: the inheritance roll's parts, tested on single-ancestor trials from the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it).

A trial (built by s05_affinity.trials) is one (ancestor, white skill spark, career year 2 or 3) where the spark's skill
is carried by exactly one of the six ancestors, so a year's inspiration of it can only come from her; it passes when the
factor shows among that year's inspirations. The model gives one roll per year, r = min(1, b(s) (1 + A / 100)),
b(s) = 3 / 6 / 9% by stars, A the ancestor's affinity (section 5, from master.mdb relation points and the record's
shared G1 wins). Ordinary ancestors only (no self-grandparent or grandparent equal to the parent).
Fitted parameters carry run-clustered (sandwich) 95% intervals, since one run gives many trials.
Tests: the grandparent weight (r scaled by kappa_G: full base kappa_G = 1 against halved 0.5), the affinity slope
(r = b (1 + s A / 100), s free; the model has s = 1), and the base by stars (observed / predicted per star count and per
affinity band). A sum's interval is observed / predicted by the run-clustered ratio estimator (common.cluster_ci: a
run's trials share its lineage, so the variance comes from each run's residual against the model); the bands' joint
test is the run-clustered Wald chi2 (common.cluster_chi2), since one run's trials fall in several bands.
"""
import collections, math
from scipy.optimize import minimize_scalar
import common
from common import Value
import s05_affinity

SEC = '3'
CHI1 = 1.920729                          # chi2(1) 95% / 2
STARS = {0.03: 1, 0.06: 2, 0.09: 3}
BANDS = [(0, 50), (50, 100), (100, 150), (150, 200), (200, 10 ** 6)]


def ll(ts, kp=1.0, kg=1.0, s=1.0):
    t = 0.0
    for cat, a, b, f, _ in ts:
        p = min(1.0, (kg if cat == 'G' else kp) * b * (1 + s * a / 100))
        p = min(max(p, 1e-12), 1 - 1e-12)
        t += math.log(p if f else 1 - p)
    return t


def llr(ts, kp=1.0, kg=1.0, s=1.0):
    """{run: log-likelihood of that run's trials}"""
    out = collections.defaultdict(float)
    for cat, a, b, f, run in ts:
        p = min(max(min(1.0, (kg if cat == 'G' else kp) * b * (1 + s * a / 100)), 1e-12), 1 - 1e-12)
        out[run] += math.log(p if f else 1 - p)
    return out


def fit1(run_ll, lo, hi):
    """(maximum-likelihood estimate on [lo, hi], run-clustered 95% low, high): a run gives many trials, so the
    interval is the sandwich one (common.robust_interval), not the likelihood interval of independent trials"""
    r = minimize_scalar(lambda x: -sum(run_ll(x).values()), bounds=(lo, hi), method='bounded', options={'xatol': 1e-5})
    a, b, _ = common.robust_interval(run_ll, float(r.x))
    return float(r.x), a, b


def profile(f, grid):
    """(best x, LL at best, 95% likelihood interval) over a grid"""
    L = [f(x) for x in grid]
    i = max(range(len(grid)), key=L.__getitem__)
    ok = [x for x, v in zip(grid, L) if L[i] - v <= CHI1]
    return grid[i], L[i], (min(ok), max(ok))


def pred(a, b): return min(1.0, b * (1 + a / 100))


def ratio(ts):
    """(trials, observed, predicted, obs / pred, low, high) under the model; the interval is run-clustered
    (common.cluster_ci)"""
    o = sum(f for _, _, _, f, _ in ts); e = sum(pred(a, b) for _, a, b, _, _ in ts)
    _, lo, hi, _ = common.cluster_ci([f for *_, f, _ in ts], [pred(a, b) for _, a, b, _, _ in ts], [run for *_, run in ts])
    return len(ts), o, e, common.div(o, e), lo, hi


def claims():
    R = common.runs()
    if R is None: return []
    full = [t for t in s05_affinity.trials(R, race=True) if t[0] in ('P', 'G')]
    ts = [(c, a0, b, f, run) for c, a0, _, b, f, run, _ in full]
    out = []
    P = [t for t in ts if t[0] == 'P']; G = [t for t in ts if t[0] == 'G']
    for lab, sub in (('parents', P), ('grandparents', G)):
        n, o, e, r, lo, hi = ratio(sub)
        out += [Value(SEC, f'trials, {lab}', n), Value(SEC, f'{lab}: passes observed', o),
                Value(SEC, f'{lab}: predicted, full base', e), Value(SEC, f'{lab}: obs / pred', r),
                Value(SEC, f'{lab}: obs / pred 95% low', lo), Value(SEC, f'{lab}: obs / pred 95% high', hi)]
    eh = sum(min(1.0, 0.5 * b * (1 + a / 100)) for _, a, b, _, _ in G)
    d_half = ll(ts) - ll(ts, kg=0.5)
    grid = [i / 100 for i in range(20, 201)]
    kg, kg_lo, kg_hi = fit1(lambda k: llr(ts, kg=k), 0.2, 2.0)
    out += [Value(SEC, 'grandparents: predicted, halved base', eh), Value(SEC, 'grandparents full over halved, nats', d_half),
            Value(SEC, 'grandparent weight fitted', kg), Value(SEC, 'grandparent weight 95% low', kg_lo),
            Value(SEC, 'grandparent weight 95% high', kg_hi)]
    out.append(common.finding(SEC, 'Grandparents take the full base: the fitted grandparent weight\'s 95% interval covers 1 and '
                              'excludes 0.5, and the full base beats the halved one by more than 1.92 nats',
                              kg_lo <= 1 <= kg_hi and not kg_lo <= 0.5 <= kg_hi and d_half > CHI1,
                              f'weight {kg:.2f} [{kg_lo:.2f}, {kg_hi:.2f}] on {len(G):,} trials; full over halved {d_half:.1f} nats'))
    sg = [i / 100 for i in range(0, 301)]
    s_hat, s_lo, s_hi = fit1(lambda s: llr(ts, s=s), 0.0, 3.0)
    d0 = ll(ts) - ll(ts, s=0)
    out += [Value(SEC, 'affinity slope fitted', s_hat), Value(SEC, 'affinity slope 95% low', s_lo),
            Value(SEC, 'affinity slope 95% high', s_hi), Value(SEC, 'affinity over none, nats', d0)]
    out.append(common.finding(SEC, 'Affinity scales the roll as (1 + A/100): the fitted slope\'s 95% interval covers 1, and '
                              'the term beats no affinity by more than 1.92 nats',
                              s_lo <= 1 <= s_hi and d0 > CHI1, f'slope {s_hat:.2f} [{s_lo:.2f}, {s_hi:.2f}]; over none {d0:.1f} nats'))
    # the two parts of A apart: relation points (rel) and 3 x shared G1 wins (race), each with its own slope
    def ll2(sr, sc):
        t = 0.0
        for c, a, b, f, rc in ((c, a0, b, f, rc) for c, a0, _, b, f, _, rc in full):
            p = min(max(min(1.0, b * (1 + (sr * (a - rc) + sc * rc) / 100)), 1e-12), 1 - 1e-12)
            t += math.log(p if f else 1 - p)
        return t
    def ll2r(sr, sc):
        out_ = collections.defaultdict(float)
        for c, a, _, b, f, run, rc in full:
            p = min(max(min(1.0, b * (1 + (sr * (a - rc) + sc * rc) / 100)), 1e-12), 1 - 1e-12)
            out_[run] += math.log(p if f else 1 - p)
        return out_
    sc_hat, sc_lo, sc_hi = fit1(lambda x: ll2r(1.0, x), 0.0, 3.0)
    sr_hat, sr_lo, sr_hi = fit1(lambda x: ll2r(x, 1.0), 0.0, 3.0)
    out += [Value(SEC, 'race part slope fitted', sc_hat), Value(SEC, 'race part slope 95% low', sc_lo),
            Value(SEC, 'race part slope 95% high', sc_hi), Value(SEC, 'relation part slope fitted', sr_hat),
            Value(SEC, 'relation part slope 95% low', sr_lo), Value(SEC, 'relation part slope 95% high', sr_hi),
            Value(SEC, 'trials with shared wins', sum(1 for t in full if t[6] > 0))]
    out.append(common.finding(SEC, 'Each part of A carries its assumed weight: with the other part fixed, the slope of the '
                              'relation points and of 3 x shared G1 wins each has a 95% interval covering 1',
                              sr_lo <= 1 <= sr_hi and sc_lo <= 1 <= sc_hi,
                              f'relation {sr_hat:.2f} [{sr_lo:.2f}, {sr_hi:.2f}]; shared wins {sc_hat:.2f} [{sc_lo:.2f}, {sc_hi:.2f}]'))
    miss = []
    for b, st in STARS.items():
        n, o, e, r, lo, hi = ratio([t for t in ts if t[2] == b])
        out += [Value(SEC, f'{st} star trials', n), Value(SEC, f'{st} star obs / pred', r),
                Value(SEC, f'{st} star 95% low', lo), Value(SEC, f'{st} star 95% high', hi)]
        if n and not lo <= 1 <= hi: miss.append(f'{st} star {r:.3f} [{lo:.2f}, {hi:.2f}]')
    out.append(common.finding(SEC, 'The base by stars (3 / 6 / 9%) holds for each star count: every interval covers 1',
                              not miss, '; '.join(miss) or 'all cover 1'))
    miss = []; resid = []
    for j, (a, b) in enumerate(BANDS):
        sub = [t for t in ts if a <= t[1] < b]
        n, o, e, r, lo, hi = ratio(sub)
        resid += [((j, f - pred(aa, bb)), run) for _, aa, bb, f, run in sub]
        lab = f'{a}+' if b > 10 ** 5 else f'{a}-{b - 1}'
        out += [Value(SEC, f'affinity {lab} trials', n), Value(SEC, f'affinity {lab} obs / pred', r),
                Value(SEC, f'affinity {lab} 95% low', lo), Value(SEC, f'affinity {lab} 95% high', hi)]
        if n >= 200 and not lo <= 1 <= hi: miss.append(f'{lab} {r:.3f} [{lo:.2f}, {hi:.2f}]')
    chi2, k, chi2_p = common.cluster_chi2([x for x, _ in resid], [run for _, run in resid])
    out += [Value(SEC, 'affinity bands chi2', chi2), Value(SEC, 'affinity bands chi2 p', chi2_p)]
    out.append(common.finding(SEC, 'By affinity band the model holds jointly: chi2 over the five bands (5 degrees of freedom, '
                              'no parameter fitted) has p above 0.05', chi2_p > 0.05,
                              f'chi2 {chi2:.1f} on {k} bands, p {chi2_p:.3f}; outside their own 95% interval: '
                              + ('; '.join(miss) or 'none')))
    out.append(common.finding(SEC, 'The bands outside their own 95% interval (as the text names them) are 150-199 and 200+',
                              [m.split(' ')[0] for m in miss] == ['150-199', '200+'], '; '.join(miss) or 'none'))
    return out


if __name__ == '__main__': common.main(claims)
