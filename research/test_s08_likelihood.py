"""Synthetic likelihood checks; no run records or game database.

Run with: uv run --group research python research/test_s08_likelihood.py
"""

import collections
import itertools
import math

import numpy as np

import s08_chains as chains
from s08_chains import EPS, KMAX, Kind, Obs


def close(actual, expected) -> None:
    assert np.allclose(actual, expected, rtol=1e-11, atol=1e-12), (actual, expected)


def partition(markers: list[tuple[int, int]], bg: float = 0.0) -> list[list[Obs]]:
    """Every seen/unseen combination on the selected card-links, including inconsistent progress reports; every marker
    with background bg (the chance it shows when its link did not fire)."""
    return [
        [Obs(card, link, seen, bg) for (card, link), seen in zip(markers, flags)]
        for flags in itertools.product((False, True), repeat=len(markers))
    ]


def observation_probability(progress: tuple[int, ...], obs: list[Obs], eps: float) -> float:
    return math.prod(
        (1 - eps * (1 - bg) if seen else eps * (1 - bg)) if progress[card] >= link else (bg if seen else 1 - bg)
        for card, link, seen, bg in obs
    )


def forward_levels(kinds: list[Kind], rule: str) -> list[dict[tuple[int, ...], float]]:
    """Independent forward chain: draw stop outcomes when a link fires and carry absorbed states forward.
    Unlike paths(), this keeps a normalized distribution at every event count."""
    initial = tuple(0 for _ in kinds)
    layer = {}
    for stopped in itertools.product((False, True), repeat=len(kinds)):
        weight = 1.0
        for kind, ended in zip(kinds, stopped):
            risk = 1.0 if kind.last_link == 0 else next((k / n for link, k, n in kind.stop_risk if link == 0), 0.0)
            weight *= risk if ended else 1 - risk
        if weight:
            layer[(initial, stopped)] = weight
    levels = []
    for _ in range(KMAX + 3):
        marginal = collections.defaultdict(float)
        following = collections.defaultdict(float)
        for (progress, stopped), probability in layer.items():
            marginal[progress] += probability
            weights = [
                0 if stopped[i] else kind.links - progress[i] if rule == "slots" else 1 for i, kind in enumerate(kinds)
            ]
            total = sum(weights)
            if total == 0:
                following[(progress, stopped)] += probability
                continue
            for card, weight in enumerate(weights):
                if not weight:
                    continue
                advanced = list(progress)
                advanced[card] += 1
                kind = kinds[card]
                risk = (
                    1.0
                    if advanced[card] == kind.last_link
                    else next((k / n for link, k, n in kind.stop_risk if link == advanced[card]), 0.0)
                )
                for ended, chance in ((True, risk), (False, 1 - risk)):
                    if chance:
                        ends = list(stopped)
                        ends[card] = ended
                        following[(tuple(advanced), tuple(ends))] += probability * weight / total * chance
        close(sum(marginal.values()), 1.0)
        levels.append(dict(marginal))
        layer = following
    return levels


def check_partition(
    kinds: list[Kind], markers: list[tuple[int, int]], rule: str, eps: float, oracle: bool, bg: float = 0.0
) -> None:
    observations = partition(markers, bg)
    matrix = chains.lk_matrix([(kinds, obs) for obs in observations], eps, rule)
    assert matrix.shape == (2 ** len(markers), KMAX + 1)
    assert np.all(matrix >= -1e-12) and np.all(matrix <= 1 + 1e-12)
    close(matrix.sum(axis=0), np.ones(KMAX + 1))
    close(chains.lk_matrix([(kinds, [])], eps, rule), np.ones((1, KMAX + 1)))
    if oracle:
        levels = forward_levels(kinds, rule)
        for row, obs in zip(matrix, observations):
            for count, distribution in enumerate(levels):
                expected = sum(p * observation_probability(progress, obs, eps) for progress, p in distribution.items())
                close(row[min(count, KMAX)], expected)

    ck, positions = chains.canon(kinds)
    pmfs = [chains.binom_pmf(21, 0.35), chains.pois_pmf(7.1), chains.cmp_pmf(5, 0.8)]
    # Counts on both sides of the old cut, the new cut and beyond it; short pmfs also exercise final_w's indexing.
    pmfs += [[0.0] * count + [1.0] for count in (0, 2, 3, 16, 17, 18, 22)]
    for pmf in pmfs:
        states, weights = chains.final_w(ck, pmf, rule)
        assert np.all(weights >= 0)
        close(weights.sum(), 1.0)
        probabilities = matrix @ chains.lump(pmf)
        close(probabilities.sum(), 1.0)
        reached = chains.reach(kinds, pmf, rule)
        for card, link in markers:
            close(reached[(card, link)], weights[states[:, positions[card]] >= link].sum())
            seen = [obs[markers.index((card, link))].seen for obs in observations]
            close(probabilities[seen].sum(), chains.seen_chance(reached[(card, link)], bg, eps))
        for obs, probability in zip(observations, probabilities):
            observation_weights = weights.copy()
            for card, link, seen, bg in obs:
                fired = states[:, positions[card]] >= link
                observation_weights *= np.where(
                    fired, 1 - eps * (1 - bg) if seen else eps * (1 - bg), bg if seen else 1 - bg
                )
            close(probability, observation_weights.sum())
            a, b = chains.run_levels(kinds, obs, eps, rule)
            padded = np.pad(pmf, (0, max(0, KMAX + 1 - len(pmf))))[: KMAX + 1]
            close(probability, a @ padded + b @ chains.survival(padded))


def check_background() -> None:
    """a marker with background 1 carries no information on N; with background, a later link seen over an unseen
    earlier one is possible even without the allowance"""
    kinds = [Kind(3, 3, ()), Kind(3, 3, ())]
    blind = chains.lk_matrix([(kinds, [Obs(0, 3, True, 1.0), Obs(1, 1, False, 1.0)])], 0.0)
    close(blind, np.zeros_like(blind))  # an unseen marker that always shows is impossible: no information, likelihood 0
    blind = chains.lk_matrix([(kinds, [Obs(0, 3, True, 1.0)])], 0.0)
    close(blind, np.ones_like(blind))  # a marker that always shows: every N alike
    jump = chains.lk_matrix([(kinds, [Obs(0, 1, False, 0.0), Obs(0, 3, True, 0.3)])], 0.0)
    assert jump.max() > 0, "a seen later link with background must not make an unseen earlier link impossible"
    close(jump[0, 0], 0.3)  # N = 0: no link fired, the later marker shows from background alone


def check_fit_cells() -> None:
    kinds = [Kind(3, 3, ())] * 6
    transient, absorbed = chains.run_levels(kinds, [])
    close(transient[16:18], [1.0, 1.0])
    close(absorbed[18], 1.0)
    close(chains.lk_matrix([(kinds, [])])[0, -1], 1.0)  # the old tail was 3
    # One full-progress observation vector per count gives a triangular, full-rank minor.
    # Thus the cut retains 19 structurally distinguishable cells; particular run samples may identify fewer.
    observations = [
        [Obs(card, link, 3 * card + link <= count) for card in range(6) for link in (1, 2, 3)]
        for count in range(KMAX + 1)
    ]
    matrix = chains.lk_matrix([(kinds, obs) for obs in observations])
    assert np.linalg.matrix_rank(matrix) == KMAX + 1
    fit = chains.em(matrix, iters=20)
    close(fit.sum(), 1.0)
    assert np.all(fit >= 0)
    close(chains.ll(fit, matrix), np.log(matrix @ fit).sum())
    close(chains.lump([1.0]), [1.0] + [0.0] * KMAX)
    close(chains.survival([0.7, 0.3000000000000002, 0.0]), [1.0, 0.3, 0.0])
    close(chains.mean_var(np.asarray(chains.lump([0.0] * 22 + [1.0]))), (KMAX, 0.0))
    for n, p in ((21, 0.35), (30, 0.8)):
        moments = chains.mean_var(np.asarray(chains.lump(chains.binom_pmf(n, p))))
        fitted_n, fitted_p = chains.binomial_moments(*moments)
        assert fitted_n == n
        close(fitted_p, p)
    try:
        chains.lk_matrix([([Kind(3, 3, ())] * 7, [])])
    except ValueError as error:
        assert "absorption bound" in str(error)
    else:
        raise AssertionError("oversize decks must not silently lose transient tail states")


def main() -> None:
    small_decks = [
        [],
        [Kind(3, 3, ())],
        [Kind(3, 3, ((1, 1, 4), (2, 1, 3))), Kind(2, 2, ((1, 1, 2),))],
        [Kind(2, 2, ()), Kind(3, 2, ((1, 1, 3),))],
        list(chains.with_activation([Kind(3, 3, ()), Kind(2, 2, ((1, 1, 2),))], 0.7)),
        list(chains.with_activation([Kind(3, 3, ())], 0.0)),
        [Kind(3, 3, ((2, 0.215, 1),)), Kind(2, 2, ())],  # a chosen end taken with a measured share (kind_in_run)
    ]
    for rule in ("slots", "cards"):
        for kinds in small_decks:
            markers = [(card, link) for card, kind in enumerate(kinds) for link in range(1, kind.links + 1)]
            for eps in (0.0, EPS):
                for bg in (0.0, 0.3, 1.0):
                    check_partition(kinds, markers, rule, eps, oracle=True, bg=bg)
        # All 18 markers would require 2**18 rows. Each of these three complete partitions has 2**6 rows.
        for link in (1, 2, 3):
            check_partition([Kind(3, 3, ())] * 6, [(card, link) for card in range(6)], rule, EPS, oracle=False)
    check_background()
    check_fit_cells()
    print("s08 likelihood checks passed")


if __name__ == "__main__":
    main()
