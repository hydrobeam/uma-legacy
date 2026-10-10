"""Shared helpers for the research scripts: paths, the run records, game data, intervals and the claim table.

Each section script defines `claims()`, returning a list of Claim, and prints its own detail when run alone.
The run records (research/data/runs.jsonl and summary.jsonl, not included) are optional: without them a script returns
its data claims as "needs data" and still checks the derived ones."""
import json, math, os, sqlite3, sys
from dataclasses import dataclass

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DATA = os.path.join(HERE, 'data')
OUT = os.path.join(DATA, 'out')            # intermediate outputs of the scripts (job files, solver results)
RUNS = os.path.join(DATA, 'runs.jsonl')
SUMMARY = os.path.join(DATA, 'summary.jsonl')
GD = os.path.join(ROOT, 'pkg', 'loopopt', 'data', 'global')
MDB = os.path.join(ROOT, 'game', 'global', 'master.mdb')
SOLVER = os.path.join(ROOT, 'pkg', 'loopopt', 'target', 'release', 'loopopt')

SCEN = {1: 'URA Finale', 2: 'Unity Cup', 3: 'Grand Concert', 4: 'Trackblazer'}
DEFAULT = [20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203]   # skill groups of the default eight targets
UNITY_GOLDS = {20049, 20046}                                          # No Stopping Me!, It's On! (guaranteed in Unity Cup)


# ---------------------------------------------------------------- data
_cache = {}


def have_runs():
    return os.path.exists(RUNS)


def runs():
    """the run records, one dict per run, in record order (seq); None without them"""
    if not have_runs(): return None
    if 'runs' not in _cache:
        _cache['runs'] = sorted((json.loads(l) for l in open(RUNS, encoding='utf-8')), key=lambda r: r['seq'])
    return _cache['runs']


def summary():
    if not os.path.exists(SUMMARY): return None
    if 'summary' not in _cache:
        _cache['summary'] = [json.loads(l) for l in open(SUMMARY, encoding='utf-8')]
    return _cache['summary']


def gjson(name):
    """a tracked Global data file (pkg/loopopt/data/global/NAME)"""
    if name not in _cache: _cache[name] = json.load(open(os.path.join(GD, name), encoding='utf-8'))
    return _cache[name]


def cards():
    return {c['id']: c for c in gjson('web_cards.json')['cards']}


def skills():
    """group -> (white name, gold name or None)"""
    return {g: (w, gd) for g, w, gd in gjson('web_skills.json')['skills']}


def skill_names():
    """skill name -> (group, 1 white / 2 gold)"""
    out = {}
    for g, w, gd in gjson('web_skills.json')['skills']:
        out[w] = (g, 1)
        if gd: out[gd] = (g, 2)
    return out


def factor_groups():
    """skill factor id // 100 -> skill group"""
    return {int(k): v for k, v in gjson('career.json')['factors'].items()}


def mdb():
    """master.mdb (game/global/master.mdb), or None"""
    if not os.path.exists(MDB): return None
    if 'mdb' not in _cache: _cache['mdb'] = sqlite3.connect(f'file:{MDB}?mode=ro', uri=True)
    return _cache['mdb']


def gametora():
    """the repo's one GameTora reader (pkg/loopopt/export/gametora.py)"""
    sys.path.insert(0, os.path.join(ROOT, 'pkg', 'loopopt', 'export'))
    import gametora
    return gametora


def bare(card_name):
    """'[Title] Character' or 'SSR Character' -> 'Character' (as hint sources name a card)"""
    s = card_name.split('] ', 1)[1] if card_name.startswith('[') else card_name
    return s.split(' ', 1)[1] if s.split(' ', 1)[0] in ('SSR', 'SR', 'R') else s


# ---------------------------------------------------------------- statistics
def wilson(k, n, z=1.96):
    if n == 0: return (float('nan'),) * 2
    p = k / n; d = 1 + z * z / n; c = p + z * z / (2 * n); h = z * math.sqrt(p * (1 - p) / n + z * z / (4 * n * n))
    return (c - h) / d, (c + h) / d


def div(a, b):
    return a / b if b else float('nan')


def binom_pmf(n, p):
    return [math.comb(n, e) * p ** e * (1 - p) ** (n - e) for e in range(n + 1)]


# ---------------------------------------------------------------- claims
@dataclass
class Claim:
    """One number RESEARCH.md states. `stated` is the text as written (its decimals set the precision); `got` the
    reproduced value (None when it could not be computed: `status` says why). Verdicts: match (equal at the stated
    precision; exact for counts), within tolerance (|got - stated| <= tol), differs, needs data, external."""
    section: str
    name: str
    stated: str
    got: float | None = None
    tol: float = 0.0
    status: str = ''
    note: str = ''

    def missing(self):
        return self.got is None or (isinstance(self.got, float) and math.isnan(self.got))

    def verdict(self):
        if self.missing(): return self.status or 'needs data'
        s = self.stated.replace(',', '').replace('−', '-').rstrip('%')
        try: v = float(s)
        except ValueError: return self.status or 'differs'
        dec = len(s.split('.')[1]) if '.' in s else 0
        g = self.got * 100 if self.stated.endswith('%') else self.got
        if round(g, dec) == round(v, dec): return 'match'
        if self.tol and abs(g - v) <= self.tol + 1e-12: return 'within tolerance'
        return 'differs'

    def shown(self):
        if self.missing(): return '-'
        g = self.got * 100 if self.stated.endswith('%') else self.got
        s = self.stated.replace(',', '').replace('−', '-').rstrip('%')
        dec = len(s.split('.')[1]) if '.' in s else 0
        return f'{g:,.{max(dec, 0)}f}' + ('%' if self.stated.endswith('%') else '')


def need_data(section, items, why='needs data'):
    """claims that need the run records, when they are missing: [(name, stated), ...]"""
    return [Claim(section, n, s, status=why) for n, s in items]


def say_missing():
    print('run records not found (research/data/runs.jsonl, not included): data claims skipped')


def table(claims, out=print):
    w = max([len(c.name) for c in claims] + [10])
    out(f'{"section":7s}  {"claim":{w}s}  {"stated":>12s}  {"reproduced":>12s}  verdict')
    for c in claims:
        out(f'{c.section:7s}  {c.name:{w}s}  {c.stated:>12s}  {c.shown():>12s}  {c.verdict()}' + (f'  ({c.note})' if c.note else ''))


def main(claims_fn):
    """run one section script alone: print its claims"""
    if not have_runs(): say_missing()
    table(claims_fn())
