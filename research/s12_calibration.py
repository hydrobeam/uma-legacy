"""Section 12 (and the inheritance test of section 3, the certainty checks of section 8): the career model against real
careers. Each run record is rebuilt as a career-planner job (trainee in her outfit, scenario, the six cards locked at
their own limit breaks, both parents with their recorded parents and every ancestor's white and pink sparks, race wins
laid out so every pair the model reads shares the record's shared-G1 count), run through the native solver, and
compared with what the run shows, per (run, target) row, pooled, with Wilson 95% intervals.

  uv run --group research research/s12_calibration.py [--wide]

Steps (outputs under research/data/out/calib/, reused while the jobs and the solver are unchanged):
  card variants   every card at limit breaks 0-3: hint frequency and increment read at that limit break's level cap
                  from master.mdb (effect values linear between milestone levels, floored; unique effect from its level)
  jobs            default eight targets (Unity Cup's two golds guaranteed there); --wide: every other skill a deck card
                  or an ancestor's spark could give, in jobs of at most 24 targets, on a seeded 30% sample of the runs
  solver          one process, 100 jobs per call (--wide: one job per call, 2 s limit each)
  analysis        the tests of RESEARCH.md section 12; the full tables go to report8.txt / reportW.txt

Dropped runs: a deck that is not six known cards, an R card in the deck, a trainee outfit not in web_skills.json, the
same parent twice, shared-G1 counts that do not fit the race list."""
import collections, hashlib, json, math, os, random, subprocess, sys, time
import common
from common import Claim, wilson, DEFAULT, UNITY_GOLDS

OUTD = os.path.join(common.OUT, 'calib')
GD = common.GD
SCN = {1: 'ura', 2: 'unity', 3: 'gc', 4: 'trackblazer'}
EXTRA = [[1078, 20165, 1], [1003, 20190, 1]]    # Yamanin Zephyr's Slipstream, Tokai Teio's Head-On (as the planner page sends)
PINK = {11, 12, 21, 22, 23, 24, 31, 32, 33, 34}
CHUNK = 24


# ---------------------------------------------------------------- card variants per limit break
def variant(cid, lb): return cid if lb is None or lb >= 4 else (lb + 1) * 1_000_000 + cid


def card_variants():
    db = common.mdb(); q = lambda s, *a: db.execute(s, a).fetchall()
    cap = {r: lv for r, *lv in q("select rarity, limit_0, limit_1, limit_2, limit_3, limit_4 from support_card_limit")}
    ms = [1, 5, 10, 15, 20, 25, 30, 35, 40, 45, 50]

    def effect(eff_id, typ, level):
        row = q("select init,limit_lv5,limit_lv10,limit_lv15,limit_lv20,limit_lv25,limit_lv30,limit_lv35,limit_lv40,limit_lv45,limit_lv50 "
                "from support_card_effect_table where id=? and type=?", eff_id, typ)
        if not row: return 0.0
        pts = [(lv, v) for lv, v in zip(ms, row[0]) if v != -1]
        if not pts or level < pts[0][0]: return 0.0
        for (l0, v0), (l1, v1) in zip(pts, pts[1:]):
            if l0 <= level <= l1: return v0 + (v1 - v0) * (level - l0) / (l1 - l0)
        return float(pts[-1][1])

    def unique(sid, typ, level):
        return sum(v for lv, t0, v0, t1, v1 in q("select lv, type_0, value_0, type_1, value_1 from support_card_unique_effect where id=?", sid)
                   for t, v in ((t0, v0), (t1, v1)) if t == typ and level >= lv)

    def at_lb(sid, lb):
        eff, rar = q("select effect_table_id, rarity from support_card_data where id=?", sid)[0]
        lv = cap[rar][lb]
        return (max(0, math.floor(effect(eff, 18, lv))) + unique(sid, 18, lv),
                1 + max(0, math.floor(effect(eff, 17, lv))) + unique(sid, 17, lv))

    W = common.gjson('web_cards.json'); E = common.gjson('card_events.json')
    out, bad = list(W['cards']), []
    for card in W['cards']:
        if at_lb(card['id'], 4) != (card['hf'], card['hinc']): bad.append(card['id'])
        for lb in range(4):
            hf, hinc = at_lb(card['id'], lb)
            out.append(dict(card, id=variant(card['id'], lb), hf=hf, hinc=hinc))
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
    def __init__(self):
        self.cards = common.cards()
        self.sk = common.gjson('web_skills.json')
        self.skg = {g for g, w, gd in self.sk['skills']}
        self.fac = common.factor_groups()
        self.nr = len(common.gjson('data.json')['races'])
        self.ev = common.gjson('card_events.json')['cards']
        self.name2g = {n: g for n, (g, _) in common.skill_names().items()}

    def sparks(self, fs):
        out = {}
        for f, _ in fs:
            if 2000000 <= f < 3000000 and f // 100 in self.fac:
                g = self.fac[f // 100]; out[g] = max(out.get(g, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    @staticmethod
    def pinks(fs):
        out = {}
        for f, _ in fs:
            if f < 10000 and f // 100 in PINK: out[f // 100] = max(out.get(f // 100, 0), f % 100)
        return sorted([g, s] for g, s in out.items())

    def ev_groups(self, c):
        """skill groups a card's events can hint (every reward the solver reads)"""
        e = self.ev.get(str(c), {}); ns = set()
        for key in ('chain', 'end'):
            for link in e.get(key, []):
                for n in link[1]: ns.add(n[0] if isinstance(n, list) else n)
        for alt in e.get('alts', []):
            for ch in alt[1]:
                for o in ch: ns.update(o)
        ns.update(e.get('always', [])); ns.update(x[0] for x in e.get('chance', []))
        return {self.name2g[n] for n in ns if n in self.name2g}

    def deck_groups(self, cards):
        gs = set()
        for c in cards: gs.update(self.cards[c]['whites']); gs |= self.ev_groups(c)
        return gs

    def spark_groups(self, r):
        return {g for p in r['parents'] for a in [p] + p['gps'] for g, s in self.sparks(a['factors']) if s > 0}

    def wins(self, aff):
        g = lambda k: aff[k][1]
        s12, a1, b1, a2, b2 = (g('parent 1 × parent 2'), g('parent 1 × grandparent 1a'), g('parent 1 × grandparent 1b'),
                               g('parent 2 × grandparent 2a'), g('parent 2 × grandparent 2b'))
        m1 = max(s12, a1, b1)
        p1 = list(range(m1))
        p2 = list(range(s12)) + list(range(m1, m1 + max(0, max(a2, b2) - s12)))
        if m1 + max(0, max(a2, b2) - s12) > self.nr: return None
        return p1, p1[:a1], p1[:b1], p2, p2[:a2], p2[:b2]


def build_jobs(R, M, wide, jobdir):
    os.makedirs(jobdir, exist_ok=True)
    drop = collections.Counter(); man = []; files = []
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
        anc = lambda a, ws: {'chara': a['chara'], 'sparks': M.sparks(a['factors']), 'pinks': M.pinks(a['factors']), 'wins': ws}
        legs = []
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


def fingerprint(jobdir, files):
    h = hashlib.sha256()
    for f in sorted(files): h.update(open(os.path.join(jobdir, f + '.json'), 'rb').read())
    for p in (common.SOLVER, os.path.join(GD, 'data.json'), os.path.join(OUTD, 'cards_lb.json'), os.path.join(OUTD, 'events_lb.json')):
        h.update(open(p, 'rb').read())
    return h.hexdigest()


def run_solver(jobdir, files, resf, timeout=None, share=None, man=None):
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
    cwd = GD; t0 = time.time(); res = {}; nt = 0
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
def rows_of(R, M, res, man):
    gen_w, gen_g = 0.2 * 1.1 ** 6, 0.4 * 1.1 ** 6
    names = common.skill_names(); cards = M.cards
    byid = {r['id']: r for r in R}
    zero = {i: z for i, z in man}
    rows, runrows, bad = [], [], collections.Counter()
    for fid, o in res.items():
        wide = '_' in fid; r = byid[int(fid.split('_')[0])]
        if 'results' not in o or not o['results']: bad['no result'] += 1; continue
        out = o['results'][0]
        deckc = [(c, lb, nm) for c, lb, nm in r['deck']]
        deckname = {common.bare(nm): c for c, lb, nm in deckc}
        pn = {v[1]: (gid, 2 if (v[2] or 1) >= 2 else 1) for gid, vs in r.get('hint_groups', []) for v in vs}
        hints = collections.defaultdict(list)
        for h in r['hints']:
            if h['name'] in names: g, lv = names[h['name']]
            elif h['name'] in pn: g, lv = pn[h['name']]
            else: bad['hint name unmapped'] += 1; continue
            hints[g].append((lv, h['src'], h['lv']))
        owned = {s // 10 for s in r.get('owned', [])}
        insp = collections.defaultdict(lambda: [0, 0])     # group -> [years fired, hits], years 2 and 3
        for y in ('2', '3'):
            seen = set()
            for f, hit in r['years'].get(y, []):
                if 2000000 <= f < 3000000 and f // 100 in M.fac:
                    g = M.fac[f // 100]; insp[g][1] += hit
                    if g not in seen: insp[g][0] += 1; seen.add(g)

        def credited(srcs):
            cs = set()
            for t in srcs:
                for n in (t[:-6].split(' or ') if t.endswith(' event') else [t]):
                    if n in deckname: cs.add((deckname[n], t.endswith(' event')))
            return cs
        tg = {t['group']: t for t in out['targets']}
        order = [t['group'] for t in out['targets']]
        slots = out['slots']
        prun = {'id': r['id'], 'scen': r['scenario'], 'p_all': out['p_all'] * (0 if zero[r['id']] else 1), 'rows': [], 'wide': wide}
        for g in order + ([] if wide else zero[r['id']]):
            t = tg.get(g)
            if t is None:
                b = {'gold': 0.0, 'white': 0.0, 'deck_gold': 0.0, 'deck_white': 0.0, 'outfit': 0}
                q, nc, p, scen_g, plegs = 0.0, 0, 0.0, False, []
            else:
                b = t['buy']; q, nc, p, scen_g = t['q'], t['carriers'], t['p'], bool(b.get('scenario'))
                k = order.index(g)
                plegs = [min(1.0, 0.03 * s['stars'][k] * (1 + s['aff'] / 100)) for s in slots if s['stars'][k] > 0]
            hs = hints.get(g, [])
            dsrc = set().union(*[credited(s) for _, s, _ in hs]) if hs else set()
            prio = None if r['priority'] is None else any(names.get(n, pn.get(n, (None,)))[0] == g for n in r['priority'])
            row = dict(run=r['id'], scen=r['scenario'], g=g, scen_gold=scen_g, outfit=b.get('outfit', 0), zero=t is None,
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
class Report:
    def __init__(self, rows): self.rows, self.lines = rows, []

    def out(self, *a): self.lines.append(' '.join(str(x) for x in a))

    def stat(self, sel, pred, obs):
        rs = [r for r in self.rows if sel(r)]
        n = len(rs)
        if n == 0: return dict(n=0, pred=float('nan'), obs=float('nan'), k=0, lo=float('nan'), hi=float('nan'), ratio=float('nan'), rlo=float('nan'), rhi=float('nan'))
        ep = sum(pred(r) for r in rs) / n; k = sum(1 for r in rs if obs(r)); lo, hi = wilson(k, n)
        nan = float('nan')
        return dict(n=n, pred=ep, obs=k / n, k=k, lo=lo, hi=hi, ratio=(k / n) / ep if ep else nan, rlo=lo / ep if ep else nan, rhi=hi / ep if ep else nan)

    def table(self, title, sel, pred, obs, key=None, minn=0):
        self.out(f'\n## {title}')
        groups = collections.defaultdict(list)
        for r in self.rows:
            if sel(r): groups[key(r) if key else 'all'].append(r)
        res = {}
        for k in sorted(groups, key=lambda k: (-len(groups[k]), str(k))):
            if len(groups[k]) < minn: continue
            s = self.stat(lambda r, k=k: sel(r) and (key(r) if key else 'all') == k, pred, obs); res[k] = s
            self.out(f'{str(k)[:60]:60s} {s["n"]:6d} {s["pred"]:7.3f} {s["obs"]:7.3f} [{s["lo"]:6.3f}, {s["hi"]:6.3f}] {s["ratio"]:7.3f} [{s["rlo"]:5.2f}, {s["rhi"]:5.2f}]')
        return res

    def reliab(self, title, sel, pred, obs, edges):
        self.out(f'\n## Reliability: {title}')
        res = []
        for a, b in zip(edges, edges[1:]):
            rs = [r for r in self.rows if sel(r) and a <= pred(r) < b]
            if not rs: res.append((a, b, 0, float('nan'), float('nan'))); continue
            n = len(rs); ep = sum(pred(r) for r in rs) / n; k = sum(1 for r in rs if obs(r))
            res.append((a, b, n, k / n, ep)); self.out(f'[{a:.3g}, {b:.3g}) n {n:6d} pred {ep:.3f} obs {k / n:.3f}')
        return res


def boot(f, xs, B=1000, seed=1):
    rnd = random.Random(seed); n = len(xs); v = []
    for _ in range(B):
        s = [xs[rnd.randrange(n)] for _ in range(n)]; v.append(f(s))
    v.sort(); return v[int(0.025 * B)], v[int(0.975 * B)]


# ---------------------------------------------------------------- claims
def claims(wide=None):
    S3, S8, S12 = '3', '8', '12'
    R = common.runs()
    out = []
    why = 'needs data' if R is None else 'needs master.mdb' if common.mdb() is None else None
    M = Model()
    if why:   # every claim listed, none computed
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
    out += [Claim(S12, 'runs used', '2,936', len(runrows)), Claim(S12, 'URA Finale runs', '598', sc[1]),
            Claim(S12, 'Unity Cup runs', '780', sc[2]), Claim(S12, 'Grand Concert runs', '1,477', sc[3]),
            Claim(S12, 'Trackblazer runs', '81', sc[4]), Claim(S12, 'run-target rows', '23,488', len(rows)),
            Claim(S12, 'rows on her outfit', '1,005', sum(1 for r in rows if r['outfit'])),
            Claim(S12, 'scenario-gold rows', '1,560', sum(1 for r in rows if r['scen_gold']))]
    free = lambda r: not r['outfit'] and not r['scen_gold']
    pdeck = lambda r: r['dg'] + (1 - r['dg']) * r['dw']
    d_sel = lambda r: free(r) and not r['o_insp'] and not r['o_insp_src']
    kind = lambda r: ('pool+event' if r['cards_pool'] and r['cards_ev'] else 'training pool only' if r['cards_pool'] else 'events only' if r['cards_ev'] else 'no deck source')
    gn = common.skills(); tname = lambda r: gn[r['g']][0]
    o_deck = lambda r: r['o_deck']
    acq = lambda r: r['acq']; o_any = lambda r: r['o_any']
    modelled = lambda r: r['o_deck'] or r['o_insp'] or r['o_insp_src']

    def add(name, s, st_rows=None, st_pred=None, st_obs=None, st_ratio=None, ci=None, note=''):
        new = []
        if st_rows: new.append(Claim(S12, f'{name}: rows', st_rows, s['n']))
        if st_pred: new.append(Claim(S12, f'{name}: predicted', st_pred, s['pred']))
        if st_obs: new.append(Claim(S12, f'{name}: observed', st_obs, s['obs']))
        if st_ratio: new.append(Claim(S12, f'{name}: obs / pred', st_ratio, s['ratio']))
        if ci: new += [Claim(S12, f'{name}: ratio 95% low', ci[0], s['rlo']), Claim(S12, f'{name}: ratio 95% high', ci[1], s['rhi'])]
        for c in new: c.note = note
        out.extend(new)

    s = rep.stat(d_sel, pdeck, o_deck); add('1. deck hint, no inspiration', s, '14,015', '0.247', '0.231', '0.932', ('0.90', '0.96'))
    out += [Claim(S12, '1. deck hint: observed 95% low', '0.224', s['lo']), Claim(S12, '1. deck hint: observed 95% high', '0.238', s['hi'])]
    add('1. training pool only', rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only', pdeck, o_deck), '2,982', '0.372', '0.357', '0.959', ('0.91', '1.01'))
    add('1. events only', rep.stat(lambda r: d_sel(r) and kind(r) == 'events only', pdeck, o_deck), '1,864', '0.887', '0.807', '0.910', ('0.89', '0.93'))
    pl = lambda r: d_sel(r) and r['prio'] is not None and kind(r) == 'events only'
    add('1. events only, prioritised', rep.stat(lambda r: pl(r) and r['prio'], pdeck, o_deck), '151', '0.889', '0.907', '1.021', ('0.96', '1.06'))
    add('1. events only, not prioritised', rep.stat(lambda r: pl(r) and not r['prio'], pdeck, o_deck), '193', '0.849', '0.549', '0.647', ('0.56', '0.73'))
    s = rep.stat(lambda r: True, lambda r: r['q'], lambda r: r['o_insp'])
    add('2. inheritance', s, '23,488', '0.350', '0.345', '0.985', ('0.97', '1.00'))
    out += [Claim(S3, 'inheritance obs / pred (23,488 rows)', '0.985', s['ratio']), Claim(S3, 'inheritance ratio 95% low', '0.97', s['rlo']),
            Claim(S3, 'inheritance ratio 95% high', '1.00', s['rhi'])]
    byc = rep.table('2. inheritance by carriers', lambda r: True, lambda r: r['q'], lambda r: r['o_insp'], lambda r: r['nc'])
    rat = [byc[k]['ratio'] for k in range(1, 7) if k in byc] or [float('nan')]
    out += [Claim(S3, 'by carriers 1-6: lowest ratio', '0.957', min(rat)), Claim(S3, 'by carriers 1-6: highest ratio', '1.001', max(rat)),
            Claim(S3, 'by carriers 1-6: intervals covering 1', '6', sum(1 for k in range(1, 7) if k in byc and byc[k]['rlo'] <= 1 <= byc[k]['rhi']))]
    e_h = sum(2 * sum(r['plegs']) for r in rows); o_h = sum(r['o_insp_hits'] for r in rows)
    e_y = sum(2 * (1 - math.prod(1 - x for x in r['plegs'])) for r in rows); o_y = sum(r['o_insp_years'] for r in rows)
    out += [Claim(S3, 'per roll: spark-years obs / exp', '0.985', common.div(o_y, e_y)), Claim(S3, 'per roll: ancestor hits obs / exp', '0.987', common.div(o_h, e_h))]
    edges = [0, 1e-12, 0.2, 0.4, 0.6, 0.8, 0.95, 1 + 1e-9]
    stated_rel = [('7,785', '0.002', None), ('2,278', '0.154', '0.16'), ('2,868', '0.299', '0.30'), ('3,585', '0.487', '0.50'),
                  ('4,851', '0.691', '0.70'), ('2,121', '0.835', '0.84')]
    rel = rep.reliab('inheritance', lambda r: True, lambda r: r['q'], lambda r: r['o_insp'], edges)
    for (a, b, n, ob, ep), (sn, so, sp) in zip(rel, stated_rel):
        lab = '0' if b <= 1e-12 else f'{a:.2f}-{b:.2f}'
        out += [Claim(S3, f'reliability q {lab}: rows', sn, n), Claim(S3, f'reliability q {lab}: observed', so, ob)]
        if sp: out.append(Claim(S3, f'reliability q {lab}: mean predicted', sp, ep))
    c_sel = free
    add('3. acquired, any source', rep.stat(c_sel, acq, o_any), '20,923', '0.498', '0.495', '0.993', ('0.98', '1.01'))
    add('3. acquired, deck or inspiration only', rep.stat(c_sel, acq, modelled), None, None, '0.485', '0.972', ('0.96', '0.99'))
    add('3. acquired, prioritised', rep.stat(lambda r: c_sel(r) and r['prio'] is True, acq, o_any), '1,052', '0.711', '0.736', '1.036', ('1.00', '1.07'))
    add('3. acquired, not prioritised', rep.stat(lambda r: c_sel(r) and r['prio'] is False, acq, o_any), '1,316', '0.535', '0.487', '0.910', ('0.86', '0.96'))
    rs = [r for r in rows if c_sel(r)]
    a, b = sum(r['p_pred'] for r in rs), sum(r['l_obs'] for r in rs)
    out += [Claim(S12, '4. landing: sum of p', '3,669.8', a), Claim(S12, '4. landing: sum observed-equivalent', '3,631.5', b), Claim(S12, '4. landing: ratio', '0.990', common.div(b, a))]
    miss = [r for r in rs if r['o_any'] and not modelled(r)]
    out.append(Claim(S12, 'acquisitions from unmodelled events', '208', sum(1 for r in miss if r['o_events'] and not r['o_other'])))
    # per target
    stated_t = {20159: ('0.79', '1.005', '0.996', '0.997'), 20049: ('0.95', '0.977', '1.057', '1.035'), 20166: ('0.973', '0.993', '0.984', '0.993'),
                20046: ('0.886', '1.003', '0.938', '0.924'), 20161: ('0.948', '0.928', '0.959', '0.962'), 21010: (None, '0.997', '1.060', '1.048'),
                20165: ('1.004', '0.993', '1.018', '0.976'), 20203: ('0.833', '0.968', '0.918', '0.917')}
    for g, (sd, si, sa, sl) in stated_t.items():
        nm = gn[g][0]
        if sd: out.append(Claim(S12, f'{nm}: deck hint', sd, rep.stat(lambda r: d_sel(r) and r['g'] == g, pdeck, o_deck)['ratio']))
        out.append(Claim(S12, f'{nm}: inheritance', si, rep.stat(lambda r: r['g'] == g, lambda r: r['q'], lambda r: r['o_insp'])['ratio']))
        out.append(Claim(S12, f'{nm}: acquired', sa, rep.stat(lambda r: c_sel(r) and r['g'] == g, acq, o_any)['ratio']))
        rg = [r for r in rs if r['g'] == g]
        out.append(Claim(S12, f'{nm}: landing', sl, common.div(sum(r['l_obs'] for r in rg), sum(r['p_pred'] for r in rg))))
    # training pool by scenario, by limit break
    for scn, st, lo, hi in ((3, '1.000', '0.94', '1.06'), (2, '0.935', '0.83', '1.05'), (1, '0.893', '0.78', '1.02'), (4, '0.747', '0.58', '0.94')):
        add(f'1. training pool only, {common.SCEN[scn]}', rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only' and r['scen'] == scn, pdeck, o_deck), None, None, None, st, (lo, hi))
    for lb, st in ((4, '0.960'), (3, '0.953')):
        out.append(Claim(S12, f'1. training pool only, lowest LB {lb}', st, rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only' and r['pool_lb'] == lb, pdeck, o_deck)['ratio']))
    low = [rep.stat(lambda r: d_sel(r) and kind(r) == 'training pool only' and r['pool_lb'] == lb, pdeck, o_deck) for lb in (0, 1, 2)]
    out += [Claim(S12, '1. training pool only, LB0-2 lowest ratio', '0.83', min(s['ratio'] for s in low)),
            Claim(S12, '1. training pool only, LB0-2 highest ratio', '1.06', max(s['ratio'] for s in low)),
            Claim(S12, '1. training pool only, LB0-2 fewest rows', '33', min(s['n'] for s in low)),
            Claim(S12, '1. training pool only, LB0-2 most rows', '87', max(s['n'] for s in low))]
    out.append(Claim(S12, "Unity Cup's guaranteed golds: runs with both", '780', sum(1 for x in runrows if x['scen'] == 2 and
                                                                                      all(r['o_gold'] for r in x['rows'] if r['scen_gold']))))
    for g, st, n, cond in ((20203, '0.899', '449', None), (20165, '0.833', '366', None), (20046, '0.437', '109', 'nu'), (20049, '0.894', '62', 'nu'), (20166, '1.000', '2,336', None)):
        nm = gn[g][1]
        s = rep.stat(lambda r: c_sel(r) and r['gold'] > 0 and r['g'] == g and (cond is None or r['scen'] != 2), lambda r: r['gold'], lambda r: r['o_gold'])
        out += [Claim(S12, f'gold acquisition {nm}', st, s['ratio']), Claim(S12, f'gold acquisition {nm}: rows', n, s['n'])]
    # reliability, deck hint and acquired
    edges = [0, 1e-12, 0.2, 0.4, 0.6, 0.8, 0.95, 1 + 1e-9]
    rd = rep.reliab('deck hint', d_sel, pdeck, o_deck, edges); ra = rep.reliab('acquired', c_sel, acq, o_any, edges)
    st_d = [('8,247', '0.001'), ('81', '0.247'), ('2,151', '0.300'), ('1,011', '0.465'), ('736', '0.632'), ('403', '0.722'), ('1,386', '0.957')]
    st_a = [('4,958', '0.015'), ('1,238', '0.164'), ('2,343', '0.316'), ('2,718', '0.501'), ('3,899', '0.703'), ('2,672', '0.826'), ('3,095', '0.975')]
    for nm, rr, st in (('deck hint', rd, st_d), ('acquired', ra, st_a)):
        for (a, b, n, ob, ep), (sn, so) in zip(rr, st):
            lab = '0' if b <= 1e-12 else f'{a:.2f}-{min(b, 1):.2f}'
            out += [Claim(S12, f'reliability {nm} {lab}: rows', sn, n), Claim(S12, f'reliability {nm} {lab}: observed', so, ob)]
    # per run
    P = []
    for pr in sorted(runrows, key=lambda x: x['id']):   # bootstrap draws in run-id order
        rs_ = [r for r in pr['rows'] if r['g'] in DEFAULT]; fr = [r for r in rs_ if not r['outfit'] and not r['scen_gold']]
        P.append(dict(n=len(fr), e=sum(r['acq'] for r in fr), o=sum(r['o_any'] for r in fr), ap=math.prod(r['acq'] for r in fr),
                      ao=all(r['o_any'] for r in fr), pa=pr['p_all'], po=math.prod(r['l_obs'] for r in rs_), scen=pr['scen']))
    n = len(P) or float('nan')
    lo, hi = wilson(sum(x['ao'] for x in P), n)
    out += [Claim(S12, 'per run: open targets', '7.13', sum(x['n'] for x in P) / n),
            Claim(S12, 'per run: acquired, expected', '3.551', sum(x['e'] for x in P) / n),
            Claim(S12, 'per run: acquired, observed', '3.525', sum(x['o'] for x in P) / n),
            Claim(S12, 'per run: all open acquired, product of odds', '0.035', sum(x['ap'] for x in P) / n),
            Claim(S12, 'per run: all open acquired, observed', '0.031', sum(x['ao'] for x in P) / n),
            Claim(S12, 'per run: all open acquired, 95% low', '0.025', lo), Claim(S12, 'per run: all open acquired, 95% high', '0.038', hi),
            Claim(S12, 'runs acquiring every open target', '90', sum(x['ao'] for x in P))]
    for nm, sub, st, ci in (('all', P, '0.847', ('0.66', '1.05')), ('Unity Cup', [x for x in P if x['scen'] == 2], '0.834', ('0.61', '1.07')),
                            ('Grand Concert', [x for x in P if x['scen'] == 3], '1.30', ('0.78', '1.83')),
                            ('URA Finale', [x for x in P if x['scen'] == 1], '0.81', ('0.23', '1.58'))):
        pp = sum(x['pa'] for x in sub); po = sum(x['po'] for x in sub)
        blo, bhi = (float('nan'),) * 2 if not sub else boot(lambda s: sum(x['po'] for x in s) / max(1e-300, sum(x['pa'] for x in s)), sub)
        out += [Claim(S12, f'P(all eight) ratio, {nm}', st, common.div(po, pp)),   # bootstrap bounds: Monte Carlo, tolerance 0.01
                Claim(S12, f'P(all eight) ratio, {nm}: 95% low', ci[0], blo, tol=0.01), Claim(S12, f'P(all eight) ratio, {nm}: 95% high', ci[1], bhi, tol=0.01)]
    out.append(Claim(S12, "Trackblazer's model P(all)", '0', sum(x['pa'] for x in P if x['scen'] == 4)))
    # single-source cards
    one = lambda r: len(set(r['cards_pool']) | set(r['cards_ev'])) == 1
    cn = lambda r: (set(r['cards_pool']) | set(r['cards_ev'])).pop()
    byname = {c['name']: c['id'] for c in common.gjson('web_cards.json')['cards']}
    for card, g, sp, so, sr, sn in (('[This Might Sting!] Sasami Anshinzawa', 20203, '1.000', '0.730', '0.73', '185'),
                                    ('[Planned Perfection] Riko Kashimoto', 20046, '0.750', '0.596', '0.79', '359'),
                                    ('[Tonight, We Waltz] King Halo', 20159, '0.937', '0.726', '0.77', '73'),
                                    ('[That Time I Became the Strongest] Gold Ship', 20159, '0.205', '0.122', '0.59', '74'),
                                    ('[Cute + Cute = ?] Mayano Top Gun', 20049, '0.298', '0.070', '0.23', '43'),
                                    ('[Sentimental Flare ♪] Maruzensky', 20165, '0.240', '0.299', '1.25', '224'),
                                    ("[Let's Get This Party Lit!] Daitaku Helios", 20165, '0.339', '0.491', '1.45', '57')):
        cid = byname.get(card)
        s = rep.stat(lambda r: d_sel(r) and one(r) and cn(r) == cid and r['g'] == g, pdeck, o_deck)
        nm = card.split('] ')[1] + ' > ' + gn[g][0]
        out += [Claim(S12, f'{nm}: rows', sn, s['n']), Claim(S12, f'{nm}: predicted', sp, s['pred']), Claim(S12, f'{nm}: observed', so, s['obs']),
                Claim(S12, f'{nm}: ratio', sr, s['ratio'])]
    # section 8 certainty checks: every credited, where the card is the skill's one deck source
    for card, g, st in (('[From the Ground Up] Light Hello', 20166, '852'), ('[Team Sirius] Team Sirius', 20203, '64')):
        cid = byname.get(card) or next((c['id'] for c in common.gjson('web_cards.json')['cards'] if card.split('] ')[1] in c['name']), None)
        s = rep.stat(lambda r: d_sel(r) and one(r) and cn(r) == cid and r['g'] == g, pdeck, o_deck)
        out += [Claim(S8, f'{card.split("] ")[1]} credited: runs carrying her', st, s['n'], note='her the one deck source, no inspiration of it'),
                Claim(S8, f'{card.split("] ")[1]} credited: credited', st, s['k'])]
    if why:
        for c in out: c.got, c.status = None, why
        return out + [Claim(S12, 'other skills: inheritance', '0.993', status=why), Claim(S12, 'other skills: training-pool hints', '1.051', status=why)]
    open(os.path.join(OUTD, 'report8.txt'), 'w').write('\n'.join(rep.lines) + '\n')
    if wide is None: wide = '--wide' in sys.argv
    out += wide_claims(R, M) if wide else [Claim(S12, 'other skills: inheritance', '0.993', status='not run (--wide)'),
                                           Claim(S12, 'other skills: training-pool hints', '1.051', status='not run (--wide)')]
    return out


def wide_claims(R, M):
    S = '12'
    jobdir = os.path.join(OUTD, 'jobsW')
    man, files, drop = build_jobs(R, M, True, jobdir)
    res = run_solver(jobdir, files, os.path.join(OUTD, 'resW.jsonl'), timeout=2, share=0.3, man=man)
    nt = sum(1 for o in res.values() if o.get('timeout'))
    res = {k: v for k, v in res.items() if not v.get('timeout')}
    rows, runrows, _ = rows_of(R, M, res, man)
    rows = [r for r in rows if r['g'] not in DEFAULT]
    rep = Report(rows)
    free = lambda r: not r['outfit'] and not r['scen_gold']
    pdeck = lambda r: r['dg'] + (1 - r['dg']) * r['dw']
    d_sel = lambda r: free(r) and not r['o_insp'] and not r['o_insp_src']
    s_i = rep.stat(lambda r: True, lambda r: r['q'], lambda r: r['o_insp'])
    s_p = rep.stat(lambda r: d_sel(r) and r['cards_pool'] and not r['cards_ev'], pdeck, lambda r: r['o_deck'])
    open(os.path.join(OUTD, 'reportW.txt'), 'w').write(f'runs {len({r["run"] for r in rows})}, jobs {len(res)}, over the limit {nt}\n')
    return [Claim(S, 'other skills: runs sampled', '881', len({x[0] for x in man if any(f.startswith(f'{x[0]}_') for f in res)})),
            Claim(S, 'other skills: jobs over the 2 s limit', '8', nt),
            Claim(S, 'other skills: inheritance rows', '37,785', s_i['n']), Claim(S, 'other skills: inheritance', '0.993', s_i['ratio']),
            Claim(S, 'other skills: inheritance 95% low', '0.97', s_i['rlo']), Claim(S, 'other skills: inheritance 95% high', '1.01', s_i['rhi']),
            Claim(S, 'other skills: training-pool rows', '14,609', s_p['n']), Claim(S, 'other skills: training-pool hints', '1.051', s_p['ratio']),
            Claim(S, 'other skills: training-pool 95% low', '1.03', s_p['rlo']), Claim(S, 'other skills: training-pool 95% high', '1.07', s_p['rhi'])]


if __name__ == '__main__': common.main(claims)
