"""Section 13 (and the inheritance test of section 3, the certainty checks of section 9): the career model against real
careers. Each run record is rebuilt as a career-planner job (trainee in her outfit, scenario, the six cards locked at
their own limit breaks, both parents with their recorded parents and every ancestor's white and pink sparks, race wins
laid out so every pair the model reads shares the record's shared-G1 count), run through the native solver, and
compared with what the run shows, per (run, target) row, pooled. Intervals (95%): observed / predicted over rows by the
run-clustered ratio estimator (common.cluster_ci; a run's targets share its deck and lineage), the observed rate's
interval that ratio interval times the mean prediction; where every run gives one row, Wilson on the observed rate
(divided by the mean prediction for the ratio). Joint tests over buckets: the run-clustered Wald chi2
(common.cluster_chi2).

  uv run --group research research/s13_calibration.py [--wide]

Steps (outputs under research/data/out/calib/, reused while the jobs and the solver are unchanged):
  card variants   every card at limit breaks 0-3: hint frequency and increment read at that limit break's level cap
                  from master.mdb (effect values linear between milestone levels, floored; unique effect from its level)
  jobs            default eight targets (Unity Cup's two golds guaranteed there); --wide: every other skill a deck card
                  or an ancestor's spark could give, in jobs of at most 24 targets, on a seeded 30% sample of the runs
  solver          one process, 100 jobs per call (--wide: one job per call, 2 s limit each)
  analysis        the tests of RESEARCH.md section 13 and the conclusions drawn on them (Findings); the full tables go
                  to report8.txt / reportW.txt

Dropped runs: a deck that is not six known cards, an R card in the deck, a trainee outfit not in web_skills.json, the
same parent twice, shared-G1 counts that do not fit the race list."""
import collections, hashlib, json, math, os, random, subprocess, sys, time
from collections.abc import Callable, Iterable, Sequence
from typing import Any
import common
from common import Value, Finding, Run, finding, wilson, DEFAULT, UNITY_GOLDS

OUTD: str = os.path.join(common.OUT, 'calib')
GD: str = common.GD
SCN: dict[int, str] = {1: 'ura', 2: 'unity', 3: 'gc', 4: 'trackblazer'}
EXTRA: list[list[int]] = [[1078, 20165, 1], [1003, 20190, 1]]    # Yamanin Zephyr's Slipstream, Tokai Teio's Head-On (as the planner page sends)
PINK: set[int] = {11, 12, 21, 22, 23, 24, 31, 32, 33, 34}
CHUNK: int = 24

# one (run, target) row of the analysis (rows_of): run, scen, g, scen_gold, outfit, zero, dg, dw, q, nc, p, acq, gold,
# plegs, o_any, o_gold, o_deck, o_deck_gold, o_insp, o_insp_years, o_insp_hits, o_insp_src, o_events, o_other, prio,
# pool_lb, cards_pool, cards_ev, p_pred, l_obs
CalRow = dict[str, Any]
# one run's rows (rows_of): id, scen, p_all, rows (its CalRows), wide
RunRows = dict[str, Any]
# a manifest entry (build_jobs): [run id, target groups nothing can give]
ManEntry = list[Any]
# a solver output per job file id
SolverOut = dict[str, dict[str, Any]]
# the counts and intervals of a set of rows (Report.stat): n, pred, obs, k, lo, hi, ratio, rlo, rhi
Stat = dict[str, float]
RowFn = Callable[[CalRow], Any]


# ---------------------------------------------------------------- card variants per limit break
def variant(cid: int, lb: int | None) -> int: return cid if lb is None or lb >= 4 else (lb + 1) * 1_000_000 + cid


def card_variants() -> list[int]:
    db = common.mdb(); q: Callable[..., list[Any]] = lambda s, *a: db.execute(s, a).fetchall()
    cap: dict[int, list[int]] = {r: lv for r, *lv in q("select rarity, limit_0, limit_1, limit_2, limit_3, limit_4 from support_card_limit")}
    ms = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]

    def effect(eff_id: int, typ: int, level: int) -> float:
        row = q("select init,limit_lv5,limit_lv10,limit_lv15,limit_lv20,limit_lv25,limit_lv30,limit_lv35,limit_lv40,limit_lv45,limit_lv50 "
                "from support_card_effect_table where id=? and type=?", eff_id, typ)
        if not row: return 0.0
        pts = [(lv, v) for lv, v in zip(ms, row[0]) if v != -1]
        if not pts or level < pts[0][0]: return 0.0
        for (l0, v0), (l1, v1) in zip(pts, pts[1:]):
            if l0 <= level <= l1: return v0 + (v1 - v0) * (level - l0) / (l1 - l0)
        return float(pts[-1][1])

    def unique(sid: int, typ: int, level: int) -> int:
        return sum(v for lv, t0, v0, t1, v1 in q("select lv, type_0, value_0, type_1, value_1 from support_card_unique_effect where id=?", sid)
                   for t, v in ((t0, v0), (t1, v1)) if t == typ and level >= lv)

    def at_lb(sid: int, lb: int) -> tuple[int, int, int]:
        eff, rar = q("select effect_table_id, rarity from support_card_data where id=?", sid)[0]
        lv = cap[rar][lb]
        return (max(0, math.floor(effect(eff, 18, lv))) + unique(sid, 18, lv),
                1 + max(0, math.floor(effect(eff, 17, lv))) + unique(sid, 17, lv),
                max(0, math.floor(effect(eff, 19, lv))) + unique(sid, 19, lv))

    W = common.gjson('web_cards.json'); E = common.gjson('card_events.json')
    out: list[dict[str, Any]]; bad: list[int]
    out, bad = list(W['cards']), []
    for card in W['cards']:
        if at_lb(card['id'], 4) != (card['hf'], card['hinc'], card.get('spec', 0)): bad.append(card['id'])
        for lb in range(4):
            hf, hinc, spec = at_lb(card['id'], lb)
            out.append(dict(card, id=variant(card['id'], lb), hf=hf, hinc=hinc, spec=spec))
    restrict = {sc: ids + [variant(i, lb) for i in ids for lb in range(4)] for sc, ids in W['restrict'].items()}
    ev = dict(E['cards'])
    for k, v in E['cards'].items():
        for lb in range(4): ev[str(variant(int(k), lb))] = v
    os.makedirs(OUTD, exist_ok=True)
    json.dump({'cards': out, 'restrict': restrict}, open(os.path.join(OUTD, 'cards_lb.json'), 'w'), ensure_ascii=False)
    json.dump({'about': E['about'], 'cards': ev}, open(os.path.join(OUTD, 'events_lb.json'), 'w'), ensure_ascii=False)
    return bad


# ---------------------------------------------------------------- jobs
class Model:
    """the game data the jobs and the analysis read"""
    cards: dict[int, dict[str, Any]]
    sk: dict[str, Any]
    skg: set[int]
    fac: dict[int, int]
    nr: int
    ev: dict[str, dict[str, Any]]
    name2g: dict[str, int]

    def __init__(self) -> None:
        self.cards = common.cards()
        self.sk = common.gjson('web_skills.json')
        self.skg = {g for g, w, gd in self.sk['skills']}
        self.fac = common.factor_groups()
        self.nr = len(common.gjson('data.json')['races'])
        self.ev = common.gjson('card_events.json')['cards']
        self.name2g = {n: g for n, (g, _) in common.skill_names().items()}

    def sparks(self, fs: Iterable[Sequence[int]]) -> list[list[int]]:
        out: dict[int, int] = {}
        for f, _ in fs:
            if 2000000 <= f < 3000000 and f // 100 in self.fac:
                g = self.fac[f // 100]; out[g] = max(out.get(g, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    @staticmethod
    def pinks(fs: Iterable[Sequence[int]]) -> list[list[int]]:
        out: dict[int, int] = {}
        for f, _ in fs:
            if f < 10000 and f // 100 in PINK: out[f // 100] = max(out.get(f // 100, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    def ev_groups(self, c: int) -> set[int]:
        """skill groups a card's events can hint (every reward the solver reads)"""
        e = self.ev.get(str(c), {}); ns: set[str] = set()
        for key in ('chain', 'end'):
            for link in e.get(key, []):
                for n in link[1]: ns.add(n[0] if isinstance(n, list) else n)
        for alt in e.get('alts', []):
            for ch in alt[1]:
                for o in ch: ns.update(o)
        ns.update(e.get('always', [])); ns.update(x[0] for x in e.get('chance', []))
        return {self.name2g[n] for n in ns if n in self.name2g}

    def deck_groups(self, cards: Iterable[int]) -> set[int]:
        gs: set[int] = set()
        for c in cards: gs.update(self.cards[c]['whites']); gs |= self.ev_groups(c)
        return gs

    def spark_groups(self, r: Run) -> set[int]:
        return {g for p in r['parents'] for a in [p] + p['gps'] for g, s in self.sparks(a['factors']) if s > 0}

    def wins(self, aff: dict[str, list[int]]) -> tuple[list[int], list[int], list[int], list[int], list[int], list[int]] | None:
        g: Callable[[str], int] = lambda k: aff[k][1]
        s12, a1, b1, a2, b2 = (g('parent 1 × parent 2'), g('parent 1 × grandparent 1a'), g('parent 1 × grandparent 1b'),
                               g('parent 2 × grandparent 2a'), g('parent 2 × grandparent 2b'))
        m1 = max(s12, a1, b1)
        p1 = list(range(m1))
        p2 = list(range(s12)) + list(range(m1, m1 + max(0, max(a2, b2) - s12)))
        if m1 + max(0, max(a2, b2) - s12) > self.nr: return None
        return p1, p1[:a1], p1[:b1], p2, p2[:a2], p2[:b2]


def build_jobs(R: list[Run], M: Model, wide: bool, jobdir: str) -> tuple[list[ManEntry], list[str], collections.Counter[str]]:
    os.makedirs(jobdir, exist_ok=True)
    drop: collections.Counter[str] = collections.Counter(); man: list[ManEntry] = []; files: list[str] = []
    for r in R:
        chara, outfit, deck = r['chara'], r['outfit'], r['deck']
        if len(deck) != 6: drop['deck not 6 cards'] += 1; continue
        if any(c not in M.cards for c, _, _ in deck): drop['card not in web_cards'] += 1; continue
        if any(M.cards[c]['rarity'] == 1 for c, _, _ in deck): drop['R card in deck'] += 1; continue
        outs = [o[0] for o in M.sk['cards'].get(str(chara), [])]
        if outfit not in outs: drop['outfit not in web_skills'] += 1; continue
        pa = r['parents']
        if pa[0]['chara'] == pa[1]['chara'] or chara in (pa[0]['chara'], pa[1]['chara']): drop['same parent twice'] += 1; continue
        w = M.wins(r['affinity'])
        if w is None: drop['shared G1 counts exceed the race list'] += 1; continue
        p1, g1a, g1b, p2, g2a, g2b = w
        anc: Callable[[dict[str, Any], list[int]], dict[str, Any]] = lambda a, ws: {'chara': a['chara'], 'sparks': M.sparks(a['factors']), 'pinks': M.pinks(a['factors']), 'wins': ws}
        legs: list[dict[str, Any]] = []
        for p, pw, gws in ((pa[0], p1, (g1a, g1b)), (pa[1], p2, (g2a, g2b))):
            l = anc(p, pw); l['borrowed'] = False
            l['parents'] = [anc(g, gw) for g, gw in zip(p['gps'], gws)]
            legs.append(l)
        scen = SCN[r['scenario']]
        ids0 = [c for c, _, _ in deck]
        if wide:
            tg = sorted(g for g in set(DEFAULT) | M.deck_groups(ids0) | M.spark_groups(r) if g in M.skg)
        else:
            tg = list(DEFAULT)
        # a target nothing can give has P = 0 in every mechanism; the career search drops careers with P(all) = 0, so
        # such targets are not sent and are scored 0 by the analysis (listed in the manifest)
        cover = {g for o in M.sk['cards'][str(chara)] if o[0] == outfit for g, _ in o[2]} | {g for u, g, _ in EXTRA if u == chara}
        live = M.deck_groups(ids0) | M.spark_groups(r) | cover | (UNITY_GOLDS if scen == 'unity' else set())
        zero = [g for g in tg if g not in live]
        tg = [g for g in tg if g in live]
        targets = [dict({'group': g}, **({'guaranteed': 2} if scen == 'unity' and g in UNITY_GOLDS else {})) for g in tg]
        ids = [variant(c, lb) for c, lb, _ in deck]
        job = {'mode': 'career', 'iters': 30, 'seed': 3, 'skills_file': os.path.join(GD, 'web_skills.json'),
               'cards_file': os.path.join(OUTD, 'cards_lb.json'), 'events_file': os.path.join(OUTD, 'events_lb.json'),
               'scenario': scen, 'targets': targets, 'extra': EXTRA,
               'exclude_outfits': [o for o in outs if o != outfit], 'owned': [chara], 'trainees': [chara],
               'owned_cards': ids, 'lock_cards': ids, 'borrow': False, 'legacies': legs, 'fix': [0, 1], 'top': 1,
               'per_trainee': True}
        if not wide:
            json.dump(job, open(os.path.join(jobdir, f'{r["id"]}.json'), 'w')); files.append(str(r['id']))
        else:
            rest = [t for t in targets if t['group'] not in DEFAULT]
            for k in range(0, len(rest), CHUNK):
                json.dump(dict(job, targets=rest[k:k + CHUNK]), open(os.path.join(jobdir, f'{r["id"]}_{k // CHUNK}.json'), 'w'))
                files.append(f'{r["id"]}_{k // CHUNK}')
        man.append([r['id'], zero])
    json.dump({'runs': man, 'files': files, 'dropped': drop}, open(os.path.join(jobdir, 'manifest.json'), 'w'))
    return man, files, drop


def fingerprint(jobdir: str, files: Iterable[str]) -> str:
    h = hashlib.sha256()
    for f in sorted(files): h.update(open(os.path.join(jobdir, f + '.json'), 'rb').read())
    for p in (common.SOLVER, os.path.join(GD, 'data.json'), os.path.join(OUTD, 'cards_lb.json'), os.path.join(OUTD, 'events_lb.json')):
        h.update(open(p, 'rb').read())
    return h.hexdigest()


def run_solver(jobdir: str, files: list[str], resf: str, timeout: float | None = None, share: float | None = None,
               man: list[ManEntry] | None = None) -> SolverOut:
    """one process at a time; returns {file id: output}; cached in resf with a fingerprint of everything read"""
    if share is not None:
        keep = set(random.Random(1).sample(sorted(x[0] for x in man), round(share * len(man))))   # seeded, over run ids in order
        files = [x for x in files if int(x.split('_')[0]) in keep]
    fp = fingerprint(jobdir, files)
    if os.path.exists(resf):
        with open(resf) as f:
            head = json.loads(f.readline())
            if head.get('fingerprint') == fp: return {x['id']: x['out'] for x in map(json.loads, f)}
    if not os.path.exists(common.SOLVER): sys.exit('native solver missing: cd pkg/loopopt && cargo build --release')
    cwd = GD; t0 = time.time(); res: SolverOut = {}; nt = 0
    print(f'solver: {len(files)} jobs, one process', file=sys.stderr, flush=True)
    if timeout:
        for x in files:
            try:
                p = subprocess.run([common.SOLVER, 'data.json', os.path.join(jobdir, x + '.json')], cwd=cwd, capture_output=True, text=True, timeout=timeout)
                lines = [l for l in p.stdout.splitlines() if l.startswith('{')]
                if p.returncode != 0 or len(lines) != 1: sys.exit(f'solver failed on {x}: rc {p.returncode}\n{p.stderr[-2000:]}')
                res[x] = json.loads(lines[0])
            except subprocess.TimeoutExpired:
                nt += 1; res[x] = {'timeout': True}
    else:
        for i in range(0, len(files), 100):
            part = files[i:i + 100]
            p = subprocess.run([common.SOLVER, 'data.json'] + [os.path.join(jobdir, x + '.json') for x in part], cwd=cwd, capture_output=True, text=True)
            lines = [l for l in p.stdout.splitlines() if l.startswith('{')]
            if p.returncode != 0 or len(lines) != len(part):
                sys.exit(f'solver failed on chunk starting {part[0]}: rc {p.returncode}\n{p.stderr[-2000:]}')
            for x, l in zip(part, lines): res[x] = json.loads(l)
    print(f'solver: done in {time.time() - t0:.0f} s, {nt} over the time limit', file=sys.stderr, flush=True)
    with open(resf, 'w') as f:
        f.write(json.dumps({'fingerprint': fp}) + '\n')
        for x in files: f.write(json.dumps({'id': x, 'out': res[x]}) + '\n')
    return res


# ---------------------------------------------------------------- analysis rows
def rows_of(R: list[Run], M: Model, res: SolverOut, man: list[ManEntry]) -> tuple[list[CalRow], list[RunRows], collections.Counter[str]]:
    gen_w, gen_g = 0.2 * 1.1 ** 6, 0.4 * 1.1 ** 6
    names = common.skill_names(); cards = M.cards
    byid = {r['id']: r for r in R}
    zero = {i: z for i, z in man}
    rows: list[CalRow]; runrows: list[RunRows]; bad: collections.Counter[str]
    rows, runrows, bad = [], [], collections.Counter()
    for fid, o in res.items():
        wide = '_' in fid; r = byid[int(fid.split('_')[0])]
        if 'results' not in o or not o['results']: bad['no result'] += 1; continue
        out = o['results'][0]
        deckc = [(c, lb, nm) for c, lb, nm in r['deck']]
        deckname = {common.bare(nm): c for c, lb, nm in deckc}
        pn = {v[1]: (gid, 2 if (v[2] or 1) >= 2 else 1) for gid, vs in r.get('hint_groups', []) for v in vs}
        hints: collections.defaultdict[int, list[tuple[int, list[str], int]]] = collections.defaultdict(list)
        for h in r['hints']:
            if h['name'] in names: g, lv = names[h['name']]
            elif h['name'] in pn: g, lv = pn[h['name']]
            else: bad['hint name unmapped'] += 1; continue
            hints[g].append((lv, h['src'], h['lv']))
        owned = {s // 10 for s in r.get('owned', [])}
        insp: collections.defaultdict[int, list[int]] = collections.defaultdict(lambda: [0, 0])     # group -> [years fired, hits], years 2 and 3
        for y in ('2', '3'):
            seen: set[int] = set()
            for f, hit in r['years'].get(y, []):
                if 2000000 <= f < 3000000 and f // 100 in M.fac:
                    g = M.fac[f // 100]; insp[g][1] += hit
                    if g not in seen: insp[g][0] += 1; seen.add(g)

        def credited(srcs: Iterable[str]) -> set[tuple[int, bool]]:
            cs: set[tuple[int, bool]] = set()
            for t in srcs:
                for n in (t[:-6].split(' or ') if t.endswith(' event') else [t]):
                    if n in deckname: cs.add((deckname[n], t.endswith(' event')))
            return cs
        tg = {t['group']: t for t in out['targets']}
        order = [t['group'] for t in out['targets']]
        slots = out['slots']
        prun: RunRows = {'id': r['id'], 'scen': r['scenario'], 'p_all': out['p_all'] * (0 if zero[r['id']] else 1), 'rows': [], 'wide': wide}
        for g in order + ([] if wide else zero[r['id']]):
            t = tg.get(g)
            if t is None:
                b: dict[str, Any] = {'gold': 0.0, 'white': 0.0, 'deck_gold': 0.0, 'deck_white': 0.0, 'outfit': 0}
                q, nc, p, scen_g, plegs = 0.0, 0, 0.0, False, []
            else:
                b = t['buy']; q, nc, p, scen_g = t['q'], t['carriers'], t['p'], bool(b.get('scenario'))
                k = order.index(g)
                plegs = [min(1.0, 0.03 * s['stars'][k] * (1 + s['aff'] / 100)) for s in slots if s['stars'][k] > 0]
            hs = hints.get(g, [])
            dsrc = set().union(*[credited(s) for _, s, _ in hs]) if hs else set()
            prio = None if r['priority'] is None else any(names.get(n, pn.get(n, (None,)))[0] == g for n in r['priority'])
            row: CalRow = dict(run=r['id'], scen=r['scenario'], g=g, scen_gold=scen_g, outfit=b.get('outfit', 0), zero=t is None,
                       dg=b.get('deck_gold', 0.0), dw=b.get('deck_white', 0.0), q=q, nc=nc, p=p,
                       acq=1.0 if scen_g else b['gold'] + b['white'], gold=1.0 if scen_g else b['gold'], plegs=plegs,
                       o_any=bool(hs) or g in owned, o_gold=any(lv == 2 for lv, _, _ in hs), o_deck=bool(dsrc),
                       o_deck_gold=any(lv == 2 and credited(s) for lv, s, _ in hs),
                       o_insp=insp[g][0] > 0, o_insp_years=insp[g][0], o_insp_hits=insp[g][1],
                       o_insp_src=any('Inspiration' in s for _, s, _ in hs),
                       o_events=any('Events' in s for _, s, _ in hs),
                       o_other=any(any(x not in ('Inspiration', 'Events') and not credited([x]) for x in s) for _, s, _ in hs),
                       prio=prio,
                       pool_lb=min([lb for c, lb, _ in deckc if g in cards[c]['whites']] or [None], key=lambda v: 9 if v is None else v),
                       cards_pool=[c for c, _, _ in deckc if g in cards[c]['whites']], cards_ev=[c for c, _, _ in deckc if g in M.ev_groups(c)])
            su = 1.1 ** (nc - 6)
            row['p_pred'] = gen_g * su if scen_g else p
            row['l_obs'] = gen_g * su if (row['o_gold'] or row['outfit'] == 2) else gen_w * su if (row['o_any'] or row['outfit'] == 1) else 0.0
            rows.append(row); prun['rows'].append(row)
        runrows.append(prun)
    return rows, runrows, bad


# ---------------------------------------------------------------- tables
def row_ci(rs: list[CalRow], pred: Callable[[CalRow], float], obs: RowFn) -> tuple[int, float, float, int, float, float, float, float, float]:
    """(n, mean predicted, observed rate, hits, rate low, rate high, obs / pred, ratio low, ratio high) over rows: where
    a run gives several rows, the run-clustered ratio estimator (common.cluster_ci) and the rate's interval that ratio
    interval times the mean prediction (nothing predicted: the rate's own run-clustered interval); where every run gives
    one row, Wilson on the observed rate, divided by the mean prediction for the ratio"""
    nan = float('nan'); n = len(rs)
    if n == 0: return 0, nan, nan, 0, nan, nan, nan, nan, nan
    ps = [pred(r) for r in rs]; os_ = [1 if obs(r) else 0 for r in rs]
    ep = sum(ps) / n; k = sum(os_)
    if len({r['run'] for r in rs}) == n:
        lo, hi = wilson(k, n)
        return n, ep, k / n, k, lo, hi, common.div(k / n, ep), common.div(lo, ep), common.div(hi, ep)
    if not ep:   # nothing predicted (no ratio): the observed rate with its run-clustered interval
        _, lo, hi, _ = common.cluster_ci(os_, [1] * n, [r['run'] for r in rs])
        return n, ep, k / n, k, lo, hi, nan, nan, nan
    ratio, rlo, rhi, _ = common.cluster_ci(os_, ps, [r['run'] for r in rs])
    return n, ep, k / n, k, rlo * ep, rhi * ep, ratio, rlo, rhi


class Report:
    """the analysis tables written to report8.txt / reportW.txt"""
    rows: list[CalRow]
    lines: list[str]

    def __init__(self, rows: list[CalRow]) -> None: self.rows, self.lines = rows, []

    def out(self, *a: object) -> None: self.lines.append(' '.join(str(x) for x in a))

    def stat(self, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn) -> Stat:
        """the rows sel picks: counts, the observed rate and obs / pred with their 95% intervals (row_ci)"""
        return dict(zip(('n', 'pred', 'obs', 'k', 'lo', 'hi', 'ratio', 'rlo', 'rhi'), row_ci([r for r in self.rows if sel(r)], pred, obs)))

    def table(self, title: str, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn, key: RowFn | None = None,
              minn: int = 0) -> dict[Any, Stat]:
        self.out(f'\n## {title}')
        groups: collections.defaultdict[Any, list[CalRow]] = collections.defaultdict(list)
        for r in self.rows:
            if sel(r): groups[key(r) if key else 'all'].append(r)
        res: dict[Any, Stat] = {}
        for k in sorted(groups, key=lambda k: (-len(groups[k]), str(k))):
            if len(groups[k]) < minn: continue
            s = self.stat(lambda r, k=k: sel(r) and (key(r) if key else 'all') == k, pred, obs); res[k] = s
            self.out(f'{str(k)[:60]:60s} {s["n"]:6d} {s["pred"]:7.3f} {s["obs"]:7.3f} [{s["lo"]:6.3f}, {s["hi"]:6.3f}] {s["ratio"]:7.3f} [{s["rlo"]:5.2f}, {s["rhi"]:5.2f}]')
        return res

    def reliab(self, title: str, sel: RowFn, pred: Callable[[CalRow], float], obs: RowFn,
               edges: Sequence[float]) -> list[tuple[float, float, int, float, float, float, float]]:
        """per bucket of the prediction: (low edge, high edge, rows, observed, mean predicted, observed 95% low, high),
        the interval by row_ci"""
        self.out(f'\n## Reliability: {title}')
        res: list[tuple[float, float, int, float, float, float, float]] = []
        for a, b in zip(edges, edges[1:]):
            n, ep, ob, _, lo, hi, *_ = row_ci([r for r in self.rows if sel(r) and a <= pred(r) < b], pred, obs)
            res.append((a, b, n, ob, ep, lo, hi))
            if n: self.out(f'[{a:.3g}, {b:.3g}) n {n:6d} pred {ep:.3f} obs {ob:.3f} [{lo:.3f}, {hi:.3f}]')
        return res


def boot[T](f: Callable[[list[T]], float], xs: Sequence[T], B: int = 1000, seed: int = 1) -> tuple[float, float]:
    rnd = random.Random(seed); n = len(xs); v: list[float] = []
    for _ in range(B):
        s = [xs[rnd.randrange(n)] for _ in range(n)]; v.append(f(s))
    v.sort(); return v[int(0.025 * B)], v[int(0.975 * B)]


# ---------------------------------------------------------------- values
STAT: dict[str, str] = {'rows': 'n', 'predicted': 'pred', 'observed': 'obs', 'obs / pred': 'ratio', 'ratio 95% low': 'rlo', 'ratio 95% high': 'rhi'}
RATIO: tuple[str, ...] = ('obs / pred', 'ratio 95% low', 'ratio 95% high')
# the single-source (card, target) pairs the document's table lists
SINGLE: list[tuple[str, int]] = [('[This Might Sting!] Sasami Anshinzawa', 20203), ('[Planned Perfection] Riko Kashimoto', 20046),
          ('[Tonight, We Waltz] King Halo', 20159), ('[That Time I Became the Strongest] Gold Ship', 20159),
          ('[Cute + Cute = ?] Mayano Top Gun', 20049), ('[Sentimental Flare ♪] Maruzensky', 20165),
          ("[Let's Get This Party Lit!] Daitaku Helios", 20165), ('[Sunlit Angel ♪] Daitaku Helios', 20165),
          ('[Lamplit Training of a Witch-to-Be] Sweep Tosho', 20165), ('[Trusting the Dice] Nakayama Festa', 20203)]
# the three event sources of Playtime's Over! (the skill on the chain-ending choice 2)
PO_CARDS: tuple[str, ...] = ('[Fiery Discipline] Yaeno Muteki', '[Dancing Light into the Night] Yukino Bijin', '[Escorte Étoile] Sirius Symboli')


def claims(wide: bool | None = None) -> list[Value | Finding]:
    S3, S8, S12 = '3', '9', '13'
    R = common.runs()
    out: list[Value | Finding] = []
    why = 'needs data' if R is None else 'needs master.mdb' if common.mdb() is None else None
    M = Model()
    bad: list[int]; drop: dict[str, int]; rows: list[CalRow]; runrows: list[RunRows]; badrows: dict[str, int]
    if why:   # nothing computed: only the findings, as "needs data"
        bad, drop, rows, runrows, badrows = [], {}, [], [], {}
    else:
        os.makedirs(OUTD, exist_ok=True)
        bad = card_variants()
        jobdir = os.path.join(OUTD, 'jobs8')
        man, files, drop = build_jobs(R, M, False, jobdir)
        res = run_solver(jobdir, files, os.path.join(OUTD, 'res8.jsonl'))
        rows, runrows, badrows = rows_of(R, M, res, man)
    rep = Report(rows)
    rep.out(f'card variants: max-limit-break check {"all equal" if not bad else bad}; dropped {dict(drop)}; problems {dict(badrows)}')
    sc = collections.Counter(x['scen'] for x in runrows)
    out += [Value(S12, 'runs used', len(runrows)), Value(S12, 'URA Finale runs', sc[1]),
            Value(S12, 'Unity Cup runs', sc[2]), Value(S12, 'Grand Concert runs', sc[3]),
            Value(S12, 'Trackblazer runs', sc[4]), Value(S12, 'run-target rows', len(rows)),
            Value(S12, 'rows on her outfit', sum(1 for r in rows if r['outfit'])),
            Value(S12, 'scenario-gold rows', sum(1 for r in rows if r['scen_gold']))]
    free: RowFn = lambda r: not r['outfit'] and not r['scen_gold']
    pdeck: Callable[[CalRow], float] = lambda r: r['dg'] + (1 - r['dg']) * r['dw']
    d_sel: RowFn = lambda r: free(r) and not r['o_insp'] and not r['o_insp_src']
    kind: Callable[[CalRow], str] = lambda r: ('pool+event' if r['cards_pool'] and r['cards_ev'] else 'training pool only' if r['cards_pool'] else 'events only' if r['cards_ev'] else 'no deck source')
    gn = common.skills(); tname: Callable[[CalRow], str] = lambda r: gn[r['g']][0]
    o_deck: RowFn = lambda r: r['o_deck']
    acq: Callable[[CalRow], float] = lambda r: r['acq']; o_any: RowFn = lambda r: r['o_any']
    modelled: RowFn = lambda r: r['o_deck'] or r['o_insp'] or r['o_insp_src']

    def add(name: str, s: Stat, fields: Iterable[str] = tuple(STAT), note: str = '') -> None:
        """the named values of one test (rows, predicted, observed, obs / pred, ratio 95% low / high), from its stat"""
        out.extend(Value(S12, f'{name}: {f}', s[STAT[f]], note=note) for f in fields)

    s_deck = s = rep.stat(d_sel, pdeck, o_deck); add('1. deck hint, no inspiration', s)
    out += [Value(S12, '1. deck hint: observed 95% low', s['lo']), Value(S12, '1. deck hint: observed 95% high', s['hi'])]
    s_pool = rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only', pdeck, o_deck)
    add('1. training pool only', s_pool)
    add('1. events only', rep.stat(lambda r: d_sel(r) and kind(r) == 'events only', pdeck, o_deck))
    pl = lambda r: d_sel(r) and r['prio'] is not None and kind(r) == 'events only'
    s_evp = rep.stat(lambda r: pl(r) and r['prio'], pdeck, o_deck); s_evn = rep.stat(lambda r: pl(r) and not r['prio'], pdeck, o_deck)
    s_evu = rep.stat(lambda r: d_sel(r) and r['prio'] is None and kind(r) == 'events only', pdeck, o_deck)
    add('1. events only, prioritised', s_evp)
    add('1. events only, not prioritised', s_evn)
    out += [Value(S8, 'event-only hints, prioritised targets: obs / pred', s_evp['ratio']),
            Value(S8, 'event-only hints, non-prioritised targets: obs / pred', s_evn['ratio'])]
    s_inh = s = rep.stat(lambda r: True, lambda r: r['q'], lambda r: r['o_insp'])
    add('2. inheritance', s)
    out += [Value(S12, '2. inheritance: observed 95% low', s['lo']), Value(S12, '2. inheritance: observed 95% high', s['hi'])]
    out += [Value(S3, 'runs tested', len(runrows)), Value(S3, 'run-target rows', s['n']),
            Value(S3, 'inheritance obs / pred', s['ratio']), Value(S3, 'inheritance ratio 95% low', s['rlo']),
            Value(S3, 'inheritance ratio 95% high', s['rhi'])]
    byc = rep.table('2. inheritance by carriers', lambda r: True, lambda r: r['q'], lambda r: r['o_insp'], lambda r: r['nc'])
    rat = [byc[k]['ratio'] for k in range(1, 7) if k in byc] or [float('nan')]
    out += [Value(S3, 'by carriers 1-6: lowest ratio', min(rat)), Value(S3, 'by carriers 1-6: highest ratio', max(rat))]
    e_h = sum(2 * sum(r['plegs']) for r in rows); o_h = sum(r['o_insp_hits'] for r in rows)
    e_y = sum(2 * (1 - math.prod(1 - x for x in r['plegs'])) for r in rows); o_y = sum(r['o_insp_years'] for r in rows)
    # z of each count, run-clustered (a run's targets share its lineage and years): a spark-year fires per roll with
    # y = 1 - prod(1 - r_l), a legacy hits with r_l; each row's residual is its count minus that expectation
    def zc(resid: list[float]) -> float:
        if not resid: return float('nan')
        x2, _, _ = common.cluster_chi2([(0, d) for d in resid], [r['run'] for r in rows])
        return math.copysign(math.sqrt(x2), sum(resid))
    z_y = zc([r['o_insp_years'] - 2 * (1 - math.prod(1 - x for x in r['plegs'])) for r in rows])
    z_h = zc([r['o_insp_hits'] - 2 * sum(r['plegs']) for r in rows])
    out += [Value(S3, 'per roll: spark-years obs / exp', common.div(o_y, e_y)), Value(S3, 'per roll: ancestor hits obs / exp', common.div(o_h, e_h)),
            Value(S3, 'per roll: spark-years, SDs below expectation', -z_y)]
    edges = [0, 1e-12, 0.2, 0.4, 0.6, 0.8, 0.95, 1 + 1e-9]
    rel = rep.reliab('inheritance', lambda r: True, lambda r: r['q'], lambda r: r['o_insp'], edges)
    for a, b, n, ob, ep, _, _ in rel[:-1]:               # the document's table stops at 0.95
        lab = '0' if b <= 1e-12 else f'{a:.2f}-{b:.2f}'
        out += [Value(S3, f'reliability q {lab}: rows', n), Value(S3, f'reliability q {lab}: observed', ob)]
        if b > 1e-12: out.append(Value(S3, f'reliability q {lab}: mean predicted', ep))
    rel_miss = [x for x in rel if x[2] and x[1] > 1e-12 and not x[5] <= x[4] <= x[6]]   # buckets with a positive prediction
    # jointly over the buckets with a positive prediction (and some q strictly between 0 and 1): the run-clustered Wald
    # chi2 of the buckets' observed - expected (a run's targets fall in several buckets), one degree of freedom per
    # bucket, nothing fitted
    resid: list[tuple[int, float]]; rruns: list[int]
    resid, rruns = [], []
    for j, (a, b) in enumerate(zip(edges, edges[1:])):
        qs = [r for r in rows if a <= r['q'] < b]
        if b <= 1e-12 or not any(0 < r['q'] < 1 for r in qs): continue
        resid += [(j, r['o_insp'] - r['q']) for r in qs]; rruns += [r['run'] for r in qs]
    rel_chi2, rel_k, rel_p = common.cluster_chi2(resid, rruns) if resid else (0.0, 0, float('nan'))
    out += [Value(S3, 'reliability chi2', rel_chi2), Value(S3, 'reliability chi2 buckets', rel_k), Value(S3, 'reliability chi2 p', rel_p)]
    b46 = next(((n, ob, ep, lo, hi) for a, b, n, ob, ep, lo, hi in rel if a == 0.4 and n), (0,) + (float('nan'),) * 4)
    out += [Value(S3, 'reliability q 0.40-0.60: observed 95% low', b46[3]), Value(S3, 'reliability q 0.40-0.60: observed 95% high', b46[4]),
            Value(S3, 'reliability q 0.40-0.60: mean predicted, three places', b46[2])]
    c_sel = free
    s_any = rep.stat(c_sel, acq, o_any); s_mod = rep.stat(c_sel, acq, modelled)
    add('3. acquired, any source', s_any)
    out += [Value(S12, '3. acquired, any source: observed 95% low', s_any['lo']), Value(S12, '3. acquired, any source: observed 95% high', s_any['hi'])]
    add('3. acquired, deck or inspiration only', s_mod)
    s_pr = rep.stat(lambda r: c_sel(r) and r['prio'] is True, acq, o_any); add('3. acquired, prioritised', s_pr)
    s_np = rep.stat(lambda r: c_sel(r) and r['prio'] is False, acq, o_any); add('3. acquired, not prioritised', s_np)
    rs = [r for r in rows if c_sel(r)]
    a, b = sum(r['p_pred'] for r in rs), sum(r['l_obs'] for r in rs)
    out += [Value(S12, '4. landing: rows', len(rs)), Value(S12, '4. landing: sum of p', a),
            Value(S12, '4. landing: sum observed-equivalent', b), Value(S12, '4. landing: ratio', common.div(b, a))]
    miss = [r for r in rs if r['o_any'] and not modelled(r)]
    n_unm = sum(1 for r in miss if r['o_events'] and not r['o_other'])
    out += [Value(S12, 'acquisitions from unmodelled events', n_unm),
            Value(S12, 'acquisitions from unmodelled events, share of rows', common.div(n_unm, len(rs)))]
    # where the deck test's shortfall (predicted minus observed hints) sits: by source kind, by reliability bucket
    short: Callable[[RowFn], float] = lambda sel: sum(pdeck(r) - r['o_deck'] for r in rows if d_sel(r) and sel(r))
    sh_all = short(lambda r: True); sh_ev = short(lambda r: kind(r) in ('events only', 'pool+event'))
    # per target: the deck test where a deck card can hint the target (not Racing Spirit: Stamina), the intervals the
    # document quotes
    no_deck = {21010}
    ci_ti = {20161}; ci_ta = {20046, 20203}               # inheritance, acquired
    t_inh: dict[int, Stat] = {}
    for g in DEFAULT:
        nm = gn[g][0]
        if g not in no_deck:
            s = rep.stat(lambda r: d_sel(r) and r['g'] == g, pdeck, o_deck)
            out += [Value(S12, f'{nm}: deck hint', s['ratio']), Value(S12, f'{nm}: deck hint 95% low', s['rlo']),
                    Value(S12, f'{nm}: deck hint 95% high', s['rhi'])]
        s = t_inh[g] = rep.stat(lambda r: r['g'] == g, lambda r: r['q'], lambda r: r['o_insp'])
        out.append(Value(S12, f'{nm}: inheritance', s['ratio']))
        if g in ci_ti: out += [Value(S12, f'{nm}: inheritance 95% low', s['rlo']), Value(S12, f'{nm}: inheritance 95% high', s['rhi'])]
        s = rep.stat(lambda r: c_sel(r) and r['g'] == g, acq, o_any)
        out.append(Value(S12, f'{nm}: acquired', s['ratio']))
        if g in ci_ta: out += [Value(S12, f'{nm}: acquired 95% low', s['rlo']), Value(S12, f'{nm}: acquired 95% high', s['rhi'])]
        rg = [r for r in rs if r['g'] == g]
        out.append(Value(S12, f'{nm}: landing', common.div(sum(r['l_obs'] for r in rg), sum(r['p_pred'] for r in rg))))
    # training pool by scenario, by limit break
    s_scn: dict[int, Stat] = {}
    for scn in (3, 2, 1, 4):
        s_scn[scn] = rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only' and r['scen'] == scn, pdeck, o_deck)
        add(f'1. training pool only, {common.SCEN[scn]}', s_scn[scn], RATIO)
    for lb in (4, 3):
        out.append(Value(S12, f'1. training pool only, lowest LB {lb}', rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only' and r['pool_lb'] == lb, pdeck, o_deck)['ratio']))
    low = [rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only' and r['pool_lb'] == lb, pdeck, o_deck) for lb in (0, 1, 2)]
    out += [Value(S12, '1. training pool only, LB0-2 lowest ratio', min(s['ratio'] for s in low)),
            Value(S12, '1. training pool only, LB0-2 highest ratio', max(s['ratio'] for s in low)),
            Value(S12, '1. training pool only, LB0-2 fewest rows', min(s['n'] for s in low)),
            Value(S12, '1. training pool only, LB0-2 most rows', max(s['n'] for s in low))]
    n_uc = sum(1 for x in runrows if x['scen'] == 2)
    k_uc = sum(1 for x in runrows if x['scen'] == 2 and all(r['o_gold'] for r in x['rows'] if r['scen_gold']))
    out.append(Value(S12, "Unity Cup's guaranteed golds: runs with both", k_uc))
    ci_g = {20203, 20165, 20046}                          # the golds whose interval the document quotes
    for g, cond in ((20203, None), (20165, None), (20046, 'nu'), (20049, 'nu'), (20166, None)):   # nu: outside Unity Cup
        nm = gn[g][1]
        s = rep.stat(lambda r: c_sel(r) and r['gold'] > 0 and r['g'] == g and (cond is None or r['scen'] != 2), lambda r: r['gold'], lambda r: r['o_gold'])
        out += [Value(S12, f'gold acquisition {nm}', s['ratio']), Value(S12, f'gold acquisition {nm}: rows', s['n'])]
        if g in ci_g: out += [Value(S12, f'gold acquisition {nm}: 95% low', s['rlo']), Value(S12, f'gold acquisition {nm}: 95% high', s['rhi'])]
    # reliability, deck hint and acquired
    edges = [0, 1e-12, 0.2, 0.4, 0.6, 0.8, 0.95, 1 + 1e-9]
    rd = rep.reliab('deck hint', d_sel, pdeck, o_deck, edges); ra = rep.reliab('acquired', c_sel, acq, o_any, edges)
    for nm, rr in (('deck hint', rd), ('acquired', ra)):
        for a, b, n, ob, ep, _, _ in rr:
            lab = '0' if b <= 1e-12 else f'{a:.2f}-{min(b, 1):.2f}'
            out += [Value(S12, f'reliability {nm} {lab}: rows', n), Value(S12, f'reliability {nm} {lab}: observed', ob)]
            if b > 1e-12: out.append(Value(S12, f'reliability {nm} {lab}: mean predicted', ep))
    # the deck's shortfall in its buckets predicted 0.6 and above, and how many of their rows have an event source
    top: RowFn = lambda r: pdeck(r) >= 0.6
    sh_top = short(top)
    n_top = sum(1 for r in rows if d_sel(r) and top(r)); n_top_ev = sum(1 for r in rows if d_sel(r) and top(r) and r['cards_ev'])
    rd_w = [x for x in rd if x[2] and x[1] > 1e-12]
    d24 = next((x for x in rd_w if x[0] == 0.2), (0.2, 0.4) + (float('nan'),) * 5)
    n24_pool = sum(1 for r in rows if d_sel(r) and 0.2 <= pdeck(r) < 0.4 and kind(r) == 'training pool only')
    out += [Value(S12, 'deck shortfall in buckets predicted 0.6 and above', common.div(sh_top, sh_all)),
            Value(S12, 'their rows with an event source', common.div(n_top_ev, n_top)),
            Value(S12, 'reliability deck hint 0.20-0.40: observed (sentence)', d24[3]),
            Value(S12, 'reliability deck hint 0.20-0.40: observed 95% low', d24[5]),
            Value(S12, 'reliability deck hint 0.20-0.40: observed 95% high', d24[6]),
            Value(S12, 'reliability deck hint 0.20-0.40: mean predicted, three places', d24[4])]
    # per run
    P: list[dict[str, Any]] = []   # per run: n, e, o, ap, ao, pa, po, scen
    for pr in sorted(runrows, key=lambda x: x['id']):   # bootstrap draws in run-id order
        rs_ = [r for r in pr['rows'] if r['g'] in DEFAULT]; fr = [r for r in rs_ if not r['outfit'] and not r['scen_gold']]
        P.append(dict(n=len(fr), e=sum(r['acq'] for r in fr), o=sum(r['o_any'] for r in fr), ap=math.prod(r['acq'] for r in fr),
                      ao=all(r['o_any'] for r in fr), pa=pr['p_all'], po=math.prod(r['l_obs'] for r in rs_), scen=pr['scen']))
    n = len(P) or float('nan')
    lo, hi = wilson(sum(x['ao'] for x in P), n)
    n_run, lo_ao, hi_ao, ao_rate = n, lo, hi, sum(x['ao'] for x in P) / n
    out += [Value(S12, 'per run: open targets', sum(x['n'] for x in P) / n),
            Value(S12, 'per run: acquired, expected', sum(x['e'] for x in P) / n),
            Value(S12, 'per run: acquired, observed', sum(x['o'] for x in P) / n),
            Value(S12, 'per run: all open acquired, product of odds', sum(x['ap'] for x in P) / n),
            Value(S12, 'per run: all open acquired, observed', sum(x['ao'] for x in P) / n),
            Value(S12, 'per run: all open acquired, 95% low', lo), Value(S12, 'per run: all open acquired, 95% high', hi),
            Value(S12, 'runs acquiring every open target', sum(x['ao'] for x in P))]
    pall: dict[str, tuple[float, float, float]] = {}
    for nm, sub in (('all', P), ('Unity Cup', [x for x in P if x['scen'] == 2]), ('Grand Concert', [x for x in P if x['scen'] == 3]),
                    ('URA Finale', [x for x in P if x['scen'] == 1])):
        pp = sum(x['pa'] for x in sub); po = sum(x['po'] for x in sub)
        blo, bhi = (float('nan'),) * 2 if not sub else boot(lambda s: sum(x['po'] for x in s) / max(1e-300, sum(x['pa'] for x in s)), sub)
        pall[nm] = (common.div(po, pp), blo, bhi)
        out += [Value(S12, f'P(all eight) ratio, {nm}', common.div(po, pp)),   # bootstrap bounds: Monte Carlo (seeded)
                Value(S12, f'P(all eight) ratio, {nm}: 95% low', blo), Value(S12, f'P(all eight) ratio, {nm}: 95% high', bhi)]
    out.append(Value(S12, 'P(all eight): optimism the interval allows (1 / 95% low)', common.div(1, pall['all'][1])))
    out.append(Value(S12, "Trackblazer's model P(all)", sum(x['pa'] for x in P if x['scen'] == 4)))
    # the solver's exact P(all) against the product of its per-target p, summed over runs (what 90 runs would have to separate)
    pa_exact = sum(x['p_all'] for x in runrows); pa_prod = sum(math.prod(r['p_pred'] for r in x['rows']) for x in runrows)
    # per run, acquired: the observed mean's 95% interval over runs
    m_o = sum(x['o'] for x in P) / n_run; sd_o = math.sqrt(sum((x['o'] - m_o) ** 2 for x in P) / max(1, len(P) - 1)) if P else float('nan')
    m_e = sum(x['e'] for x in P) / n_run; ap_mean = sum(x['ap'] for x in P) / n_run
    # single-source cards
    one: RowFn = lambda r: len(set(r['cards_pool']) | set(r['cards_ev'])) == 1
    cn: Callable[[CalRow], int] = lambda r: (set(r['cards_pool']) | set(r['cards_ev'])).pop()
    byname = {c['name']: c['id'] for c in common.gjson('web_cards.json')['cards']}
    listed: set[tuple[int | None, int]]; s_one: dict[tuple[int | None, int], Stat]
    listed, s_one = set(), {}
    for card, g in SINGLE:
        cid = byname.get(card); listed.add((cid, g))
        s = s_one[cid, g] = rep.stat(lambda r: d_sel(r) and one(r) and cn(r) == cid and r['g'] == g, pdeck, o_deck)
        nm = card + ' > ' + gn[g][0]                     # the full card name: two Helios cards share a target
        out += [Value(S12, f'{nm}: rows', s['n']), Value(S12, f'{nm}: predicted', s['pred']), Value(S12, f'{nm}: observed', s['obs']),
                Value(S12, f'{nm}: ratio', s['ratio']), Value(S12, f'{nm}: ratio 95% low', s['rlo']), Value(S12, f'{nm}: ratio 95% high', s['rhi'])]
    # every single-source (card, target) pair whose ratio interval excludes 1
    cname = {c['id']: c['name'] for c in common.gjson('web_cards.json')['cards']}
    pairs = sorted({(cn(r), r['g']) for r in rows if d_sel(r) and one(r)})
    off: list[tuple[int, int, Stat]] = []
    for cid, g in pairs:
        s = rep.stat(lambda r: d_sel(r) and one(r) and cn(r) == cid and r['g'] == g, pdeck, o_deck)
        if s['pred'] and not s['rlo'] - 1e-9 <= 1 <= s['rhi'] + 1e-9: off.append((cid, g, s))   # a tolerance: k = n at a prediction of 1 is no miss
    gship = (byname.get('[That Time I Became the Strongest] Gold Ship'), 20159)
    off40 = {(cid, g) for cid, g, s in off if s['n'] >= 40}
    off_unlisted = [(cid, g, s) for cid, g, s in off if s['n'] >= 40 and (cid, g) not in listed]
    small = [(cid, g, s) for cid, g, s in off if s['n'] < 40]
    po_small = [(cid, g, s) for cid, g, s in small if g == 20166 and s['ratio'] < 1]     # Playtime's Over! below its prediction
    po_k, po_n = sum(s['k'] for _, _, s in po_small), sum(s['n'] for _, _, s in po_small)
    po_lo = min([s['pred'] for _, _, s in po_small] or [float('nan')])
    out += [Value(S12, 'pairs under 40 rows excluding 1', len(small)),
            Value(S12, "pairs under 40 rows excluding 1: Playtime's Over! event sources", len(po_small)),
            Value(S12, "pairs under 40 rows excluding 1: Playtime's Over! event sources, runs", po_n),
            Value(S12, "pairs under 40 rows excluding 1: Playtime's Over! event sources, hinted", po_k),
            Value(S12, "pairs under 40 rows excluding 1: Playtime's Over! lowest predicted", po_lo),
            Value(S12, "pairs under 40 rows excluding 1: Playtime's Over! highest predicted", max([s['pred'] for _, _, s in po_small] or [float('nan')]))]
    # the likely reasons the table gives, read on the runs that show a priority list (priority by skill name)
    plist: dict[int, list[str] | None] = {r['id']: r['priority'] for r in (R or [])}
    onp = lambda r, skill: None if plist.get(r['run']) is None else skill in plist[r['run']]
    by_prio = lambda card, g, skill, v: rep.stat(lambda r: d_sel(r) and one(r) and cn(r) == byname.get(card) and r['g'] == g and onp(r, skill) is v, pdeck, o_deck)
    sas = [by_prio('[This Might Sting!] Sasami Anshinzawa', 20203, 'Risky Business', v) for v in (True, False)]
    kh = [by_prio('[Tonight, We Waltz] King Halo', 20159, 'Uma Stan', v) for v in (True, False)]
    rk = [by_prio('[Planned Perfection] Riko Kashimoto', 20046, 'Lone Wolf', v) for v in (True, False, None)]
    # the three Playtime's Over! event sources, pooled, by whether the target is prioritised
    po_ids = {byname.get(c) for c in PO_CARDS}
    po = [rep.stat(lambda r: d_sel(r) and one(r) and cn(r) in po_ids and r['g'] == 20166 and r['prio'] is v, pdeck, o_deck) for v in (True, False, None)]
    out += [Value(S12, "Playtime's Over! event sources: runs showing a priority list", po[0]['n'] + po[1]['n']),
            Value(S12, "Playtime's Over! event sources: prioritised", po[0]['n']),
            Value(S12, "Playtime's Over! event sources: hinted with a list shown", po[0]['k'] + po[1]['k']),
            Value(S12, "Playtime's Over! event sources: runs without a list", po[2]['n']),
            Value(S12, "Playtime's Over! event sources: hinted without a list", po[2]['k'])]
    # section 9 certainty checks: every credited, where the card is the skill's one deck source
    cert: dict[str, Stat] = {}
    for card, g in (('[From the Ground Up] Light Hello', 20166), ('[Team Sirius] Team Sirius', 20203)):
        cid = byname.get(card) or next((c['id'] for c in common.gjson('web_cards.json')['cards'] if card.split('] ')[1] in c['name']), None)
        s = cert[card.split('] ')[1]] = rep.stat(lambda r: d_sel(r) and one(r) and cn(r) == cid and r['g'] == g, pdeck, o_deck)
        out += [Value(S8, f'{card.split("] ")[1]} credited: runs carrying her', s['n'], note='her the one deck source, no inspiration of it'),
                Value(S8, f'{card.split("] ")[1]} credited: credited', s['k'])]

    # ------------------------------------------------------------ findings: each conclusion, tested on the numbers above
    covers: Callable[[Stat], bool] = lambda s: s['rlo'] <= 1 <= s['rhi']
    below: Callable[[Stat], bool] = lambda s: s['rhi'] < 1
    rt: Callable[[Stat], str] = lambda s: f"{s['ratio']:.3f} [{s['rlo']:.3f}, {s['rhi']:.3f}], {s['n']:,} rows"
    lab = lambda a, b: f'{a:.2f}-{min(b, 1):.2f}'
    tn = lambda g: gn[g][0]
    t_off = [g for g in DEFAULT if g in t_inh and not covers(t_inh[g])]
    F = [
        # section 3
        finding(S3, 'inspiration odds q are calibrated: observed / predicted over all run-target rows has a 95% interval covering 1',
                lambda: covers(s_inh), lambda: rt(s_inh) + '; per target, intervals excluding 1: '
                + (', '.join(f'{tn(g)} {rt(t_inh[g])}' for g in t_off) or 'none')),
        finding(S3, 'by carriers 1-6, every interval covers 1',
                lambda: all(k in byc and covers(byc[k]) for k in range(1, 7)),
                lambda: '; '.join(f'{k}: {byc[k]["ratio"]:.3f} [{byc[k]["rlo"]:.3f}, {byc[k]["rhi"]:.3f}]' for k in range(1, 7) if k in byc)),
        finding(S3, 'per roll, spark-years and ancestor hits each sit within the model\'s 95% band (|z| < 1.96, z run-clustered)',
                lambda: abs(z_y) < 1.96 and abs(z_h) < 1.96,
                lambda: f'spark-years {o_y:,} vs {e_y:,.1f} expected (z {z_y:+.2f}); hits {o_h:,} vs {e_h:,.1f} (z {z_h:+.2f})'),
        finding(S3, 'reliability: calibrated jointly over the buckets with a positive prediction (chi2 over the buckets, one degree of '
                    'freedom each, nothing fitted, p above 0.05); the buckets outside their own 95% interval are listed',
                lambda: rel_p > 0.05,
                lambda: ('misses: ' + '; '.join(f'{lab(a, b)}: {ob:.3f} [{lo:.3f}, {hi:.3f}] vs {ep:.3f} ({n:,} rows)' for a, b, n, ob, ep, lo, hi in rel_miss)
                         if rel_miss else 'no bucket misses') + f' (of {sum(1 for _, b, n, *_ in rel if n and b > 1e-12)} buckets tested at 95% each; '
                f'q = 0 bucket observed {rel[0][3]:.4f} over {rel[0][2]:,} rows); chi2 {rel_chi2:.1f} on {rel_k} buckets, p {rel_p:.3f}'),
        finding(S3, 'reliability: the only bucket outside its own 95% interval (as the text names it) is 0.40-0.60',
                lambda: [(a, b) for a, b, *_ in rel_miss] == [(0.4, 0.6)],
                lambda: '; '.join(f'{lab(a, b)}: {ob:.3f} [{lo:.3f}, {hi:.3f}] vs {ep:.3f}' for a, b, n, ob, ep, lo, hi in rel_miss) or 'none'),
        finding(S12, 'acquired: prioritised targets slightly above the model and non-prioritised below it (each run-clustered '
                     'interval entirely on its side of 1), the two averaging out in the pooled test',
                lambda: s_pr['rlo'] > 1 and s_np['rhi'] < 1,
                lambda: f'prioritised {s_pr["ratio"]:.3f} [{s_pr["rlo"]:.3f}, {s_pr["rhi"]:.3f}]; not prioritised {s_np["ratio"]:.3f} [{s_np["rlo"]:.3f}, {s_np["rhi"]:.3f}]'),
        # section 9
        finding(S8, 'the choice rule shows in its consequences: event-only hints of prioritised targets land at the model\'s rate (interval covers 1), '
                    'of non-prioritised targets below it (interval entirely below 1)',
                lambda: covers(s_evp) and below(s_evn), lambda: f'prioritised {rt(s_evp)}; not prioritised {rt(s_evn)}'),
        *[finding(S8, f'{nm} credits her finale skill in every run carrying her (the finale is certain)',
                  lambda s=s: s['n'] > 0 and s['k'] == s['n'], lambda s=s: f'{s["k"]} of {s["n"]} (Wilson 95% low {s["lo"]:.4f})') for nm, s in cert.items()],
        # section 13
        finding(S12, 'the composed acquisition odds are calibrated: acquired from any source, observed / predicted, 95% interval covers 1',
                lambda: covers(s_any), lambda: rt(s_any) + '; reliability buckets whose interval lies off their mean prediction: '
                + '; '.join(f'{lab(a, b)} {ob:.3f} vs {ep:.3f}' for a, b, n, ob, ep, lo, hi in ra if n and b > 1e-12
                            and not lo <= ep <= hi)),
        finding(S12, 'inheritance is right: observed / predicted inspiration, 95% interval covers 1; one target of eight, Tail Held High, '
                     'has its interval off 1',
                lambda: covers(s_inh) and t_off == [20161], lambda: rt(s_inh) + '; per-target exceptions: ' + (', '.join(f'{tn(g)} {rt(t_inh[g])}' for g in t_off) or 'none')),
        finding(S12, 'deck hints read optimistic: the deck test\'s observed / predicted interval lies entirely below 1',
                lambda: below(s_deck), lambda: rt(s_deck)),
        finding(S12, 'the deck\'s shortfall is almost all from the priority assumption: rows with an event source carry at least 80% of the shortfall '
                     '(predicted minus observed hints), and on runs showing a priority list event-only hints land at the model\'s rate when the target '
                     'is prioritised (interval covers 1) and below it when not (interval below 1)',
                lambda: sh_ev / sh_all >= 0.8 and covers(s_evp) and below(s_evn),
                lambda: f'event-source shortfall {sh_ev:.1f} of {sh_all:.1f} hints ({sh_ev / sh_all:.0%}); training pool only {rt(s_pool)}; '
                        f'prioritised {rt(s_evp)}; not prioritised {rt(s_evn)}; no priority list shown {rt(s_evu)}'),
        finding(S12, 'unmodelled scenario and trainee events supply what the deck loses: acquired from any source covers 1, '
                     'acquired from the deck or an inspiration lies below 1',
                lambda: covers(s_any) and below(s_mod),
                lambda: f'any source {rt(s_any)}; deck or inspiration {rt(s_mod)}; rows acquired with no modelled source {len(miss)}, '
                        f'{n_unm} of them credited to events only'),
        finding(S12, 'Racing Spirit: Stamina has no deck source: no deck card in any run can hint it',
                lambda: any(r['g'] == 21010 for r in rows) and not any(r['g'] == 21010 and (r['cards_pool'] or r['cards_ev']) for r in rows),
                lambda: f'{sum(1 for r in rows if r["g"] == 21010 and (r["cards_pool"] or r["cards_ev"]))} of {sum(1 for r in rows if r["g"] == 21010):,} rows with a deck source'),
        finding(S12, 'training-pool hints (beta): Grand Concert, Unity Cup and URA Finale each have an interval covering 1',
                lambda: all(covers(s_scn[k]) for k in (3, 2, 1)), lambda: '; '.join(f'{common.SCEN[k]} {rt(s_scn[k])}' for k in (3, 2, 1))),
        finding(S12, 'only Trackblazer\'s interval excludes 1, its beta over-predicting: its interval lies entirely below 1, the other '
                     'three cover 1',
                lambda: below(s_scn[4]) and all(covers(s_scn[k]) for k in (3, 2, 1)), lambda: f'Trackblazer {rt(s_scn[4])}'),
        finding(S12, 'training pool by lowest limit break: LB0, LB1 and LB2 each have an interval covering 1',
                lambda: all(covers(s) for s in low), lambda: '; '.join(f'LB{lb} {rt(s)}' for lb, s in zip((0, 1, 2), low))),
        finding(S12, 'Unity Cup\'s two golds are guaranteed: every Unity Cup run shows both hinted',
                lambda: n_uc > 0 and k_uc == n_uc, lambda: f'{k_uc} of {n_uc} runs'),
        finding(S12, 'the deck\'s over-prediction sits in its top buckets, the event rewards: buckets predicted 0.6 and above carry over half the '
                     'shortfall, each lies below its mean prediction, and over half their rows have an event source',
                lambda: sh_top / sh_all > 0.5 and n_top_ev / n_top > 0.5 and all(hi < ep for a, b, n, ob, ep, lo, hi in rd_w if a >= 0.6),
                lambda: f'top-bucket shortfall {sh_top:.1f} of {sh_all:.1f} ({sh_top / sh_all:.0%}); event-source rows {n_top_ev:,} of {n_top:,} '
                        f'({n_top_ev / n_top:.0%}); buckets lying below their mean prediction: '
                        + '; '.join(f'{lab(a, b)} {ob:.3f} [{lo:.3f}, {hi:.3f}] vs {ep:.3f}' for a, b, n, ob, ep, lo, hi in rd_w if hi < ep)),
        finding(S12, 'per run, acquisition is calibrated: the all-open-acquired interval covers the product of the model\'s odds, and the mean '
                     'targets acquired per run has a 95% interval covering the expected mean',
                lambda: lo_ao <= ap_mean <= hi_ao and abs(m_o - m_e) <= 1.96 * sd_o / math.sqrt(len(P)),
                lambda: f'all open: {ao_rate:.4f} [{lo_ao:.4f}, {hi_ao:.4f}] vs {ap_mean:.4f}; targets acquired {m_o:.3f} '
                        f'[{m_o - 1.96 * sd_o / math.sqrt(len(P)):.3f}, {m_o + 1.96 * sd_o / math.sqrt(len(P)):.3f}] vs {m_e:.3f}'),
        finding(S12, 'landing P(all eight) is consistent with calibrated: the bootstrap interval covers 1, pooled and in each scenario with a model value',
                lambda: all(b0 <= 1 <= b1 for _, b0, b1 in pall.values()),
                lambda: '; '.join(f'{k} {v:.3f} [{b0:.2f}, {b1:.2f}]' for k, (v, b0, b1) in pall.items())),
        finding(S12, 'too few runs acquired every open target to separate the exact joint from the product: the solver\'s exact P(all) differs '
                     'from the product of its per-target p by less than the relative half-width of the all-acquired interval',
                lambda: abs(pa_exact / pa_prod - 1) < (hi_ao - lo_ao) / 2 / ao_rate,
                lambda: f'exact / product {pa_exact / pa_prod:.3f} (sums over runs {pa_exact:.4f} / {pa_prod:.4f}); all acquired '
                        f'{sum(x["ao"] for x in P)} runs, relative half-width {(hi_ao - lo_ao) / 2 / ao_rate:.0%}'),
        finding(S12, 'the single-source table holds every pair of 40+ rows whose ratio interval excludes 1, plus Gold Ship\'s, and nothing else',
                lambda: bool(pairs) and not off_unlisted and listed <= off40 | {gship} and gship in listed,
                lambda: f'{len(off)} of {len(pairs)} pairs exclude 1; not listed: ' + ('; '.join(
                    f'{cname[c]} > {tn(g)} {s["k"]}/{s["n"]} vs pred {s["pred"]:.3f} = {s["ratio"]:.2f} [{s["rlo"]:.2f}, {s["rhi"]:.2f}]'
                    for c, g, s in sorted(off_unlisted, key=lambda x: -x[2]['n'])) or 'none') + '; listed without excluding 1: '
                    + (', '.join(f'{cname[c]} > {tn(g)} [{s_one[c, g]["rlo"]:.2f}, {s_one[c, g]["rhi"]:.2f}]' for c, g in sorted(listed - off40)) or 'none')),
        finding(S12, 'below 40 rows more pairs exclude 1; among them, below their prediction, are exactly Playtime\'s Over!\'s three event sources '
                     '(Yaeno Muteki, Yukino Bijin, Sirius Symboli), whose pooled hint rate has a 95% interval entirely under their lowest prediction',
                lambda: bool(small) and {c for c, _, _ in po_small} == po_ids and wilson(po_k, po_n)[1] < po_lo,
                lambda: f'{len(small)} pairs; Playtime\'s Over! sources hinted {po_k}/{po_n} [{wilson(po_k, po_n)[0]:.3f}, {wilson(po_k, po_n)[1]:.3f}] '
                        f'vs lowest prediction {po_lo:.3f}; ' + '; '.join(f'{cname[c]} > {tn(g)} {s["k"]}/{s["n"]} vs pred {s["pred"]:.3f} '
                                                                          f'[{s["rlo"]:.2f}, {s["rhi"]:.2f}]' for c, g, s in small)),
        finding(S12, 'the Playtime\'s Over! miss is the priority assumption (the skill sits on a chain-ending choice 2, taken only when '
                     'prioritised): prioritised rows cover 1 or are too few to test (under 5), rows on a list without it lie below 1 '
                     '(untestable, "needs data", when fewer than 5 rows are on a list without it)',
                lambda: (po[0]['n'] < 5 or covers(po[0])) and below(po[1]) if po[1]['n'] >= 5 else {}['too few rows on a list'],
                lambda: f'prioritised {po[0]["k"]}/{po[0]["n"]} ' + (rt(po[0]) if po[0]['n'] else '') + f'; on a list, not prioritised '
                        f'{po[1]["k"]}/{po[1]["n"]} {rt(po[1])}; no list shown {po[2]["k"]}/{po[2]["n"]} {rt(po[2])}'),
        finding(S12, 'the 0.20-0.40 deck bucket, almost all training-pool rows (at least 90%), is within its interval: the interval covers its mean prediction',
                lambda: d24[5] <= d24[4] <= d24[6] and n24_pool / d24[2] >= 0.9,
                lambda: f'{d24[3]:.3f} [{d24[5]:.3f}, {d24[6]:.3f}] vs {d24[4]:.3f}; training-pool rows {n24_pool:,} of {d24[2]:,}'),
        finding(S12, 'Sasami Anshinzawa [This Might Sting!] > Risky Business: the model assumes Risky Business prioritised: with it on the '
                     'priority list her hint lands at the model\'s rate (interval covers 1), without it below (interval below 1)',
                lambda: covers(sas[0]) and below(sas[1]), lambda: f'prioritised {rt(sas[0])}; on a list, not prioritised {rt(sas[1])}'),
        finding(S12, 'King Halo [Tonight, We Waltz] > Uma Stan sits on a choice that needs a priority: prioritised covers 1, not prioritised below 1',
                lambda: covers(kh[0]) and below(kh[1]), lambda: f'prioritised {rt(kh[0])}; on a list, not prioritised {rt(kh[1])}'),
        finding(S12, 'Riko Kashimoto [Planned Perfection] > Ramp Up: her Lone Wolf choice when prioritised accounts for part: rows with Lone '
                     'Wolf prioritised lie below 1, and rows from runs showing no priority list still lie below 1',
                lambda: below(rk[0]) and below(rk[2]),
                lambda: f'Lone Wolf prioritised {rt(rk[0])}; on a list, not prioritised {rt(rk[1])}; no list shown {rt(rk[2])}'),
    ]
    if why:
        for f in F: f.holds, f.evidence = None, why
        return F + wide_findings(None, None, why)
    out += F
    open(os.path.join(OUTD, 'report8.txt'), 'w').write('\n'.join(rep.lines) + '\n')
    if wide is None: wide = '--wide' in sys.argv
    out += wide_claims(R, M) if wide else wide_findings(None, None, 'not run (--wide)')
    return out


def wide_claims(R: list[Run], M: Model) -> list[Value | Finding]:
    S = '13'
    jobdir = os.path.join(OUTD, 'jobsW')
    man, files, drop = build_jobs(R, M, True, jobdir)
    res = run_solver(jobdir, files, os.path.join(OUTD, 'resW.jsonl'), timeout=2, share=0.3, man=man)
    nt = sum(1 for o in res.values() if o.get('timeout'))
    res = {k: v for k, v in res.items() if not v.get('timeout')}
    rows, runrows, _ = rows_of(R, M, res, man)
    rows = [r for r in rows if r['g'] not in DEFAULT]
    rep = Report(rows)
    free: RowFn = lambda r: not r['outfit'] and not r['scen_gold']
    pdeck: Callable[[CalRow], float] = lambda r: r['dg'] + (1 - r['dg']) * r['dw']
    d_sel: RowFn = lambda r: free(r) and not r['o_insp'] and not r['o_insp_src']
    s_i = rep.stat(lambda r: True, lambda r: r['q'], lambda r: r['o_insp'])
    s_p = rep.stat(lambda r: d_sel(r) and r['cards_pool'] and not r['cards_ev'], pdeck, lambda r: r['o_deck'])
    open(os.path.join(OUTD, 'reportW.txt'), 'w').write(f'runs {len({r["run"] for r in rows})}, jobs {len(res)}, over the limit {nt}\n')
    return [Value(S, 'other skills: runs sampled', len({x[0] for x in man if any(f.startswith(f'{x[0]}_') for f in res)})),
            Value(S, 'other skills: jobs over the 2 s limit', nt),
            Value(S, 'other skills: inheritance rows', s_i['n']), Value(S, 'other skills: inheritance', s_i['ratio']),
            Value(S, 'other skills: inheritance 95% low', s_i['rlo']), Value(S, 'other skills: inheritance 95% high', s_i['rhi']),
            Value(S, 'other skills: training-pool rows', s_p['n']), Value(S, 'other skills: training-pool hints', s_p['ratio']),
            Value(S, 'other skills: training-pool 95% low', s_p['rlo']), Value(S, 'other skills: training-pool 95% high', s_p['rhi']),
            Value('3', 'non-target skills: inheritance rows', s_i['n']), Value('3', 'non-target skills: inheritance', s_i['ratio']),
            Value('3', 'non-target skills: inheritance 95% low', s_i['rlo']), Value('3', 'non-target skills: inheritance 95% high', s_i['rhi'])] \
        + wide_findings(s_i, s_p)


def wide_findings(s_i: Stat | None, s_p: Stat | None, why: str | None = None) -> list[Finding]:
    """the conclusions drawn on the other skills (section 3's non-target rows, section 13's "Other skills")"""
    covers: Callable[[Stat], bool] = lambda s: s['rlo'] <= 1 <= s['rhi']
    rt: Callable[[Stat], str] = lambda s: f"{s['ratio']:.3f} [{s['rlo']:.3f}, {s['rhi']:.3f}], {s['n']:,} rows"
    fs = [finding('3', 'inspiration odds hold on non-target skills: interval covers 1', lambda: covers(s_i), lambda: rt(s_i)),
          finding('13', 'other skills: inheritance holds beyond the eight targets (interval covers 1)', lambda: covers(s_i), lambda: rt(s_i)),
          finding('13', 'other skills: training-pool hints under-predicted: interval entirely above 1',
                  lambda: s_p['rlo'] > 1, lambda: rt(s_p))]
    if why:
        for f in fs: f.holds, f.evidence = None, why
    return fs


if __name__ == '__main__': common.main(claims)
