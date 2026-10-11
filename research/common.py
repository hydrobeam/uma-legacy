"""Shared helpers for the research scripts: paths, the run records, game data, intervals and the claim table.

Each section script defines `claims()`, returning a list of Claim, and prints its own detail when run alone.
The run records (research/data/runs.jsonl, not included) are optional: without them a script returns
its data claims as "needs data" and still checks the derived ones."""
import json, math, os, re, sqlite3, sys
from dataclasses import dataclass

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(HERE)
DATA = os.path.join(HERE, 'data')
OUT = os.path.join(DATA, 'out')            # intermediate outputs of the scripts (job files, solver results)
RUNS = os.path.join(DATA, 'runs.jsonl')
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


def run_list():
    """the run list's rows (every completed run, with uma-it's rank-score estimates; more runs than the records),
    one dict per run; None without it"""
    f = os.path.join(DATA, 'summary.jsonl')
    if not os.path.exists(f): return None
    if 'list' not in _cache: _cache['list'] = [json.loads(l) for l in open(f, encoding='utf-8')]
    return _cache['list']


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
def key_of(section, name):
    """a value's key in results.json and in the document template: section.slug_of_the_name"""
    for a_, b_ in (('+-', ' pm '), ('±', ' pm '), ('%', ' pct '), ('>=', ' ge '), ('<', ' lt '), ('>', ' gt '), ('/', ' per ')):
        name = name.replace(a_, b_)
    return f"{section}.{re.sub(r'[^a-z0-9]+', '_', name.lower()).strip('_')}"


@dataclass
class Value:
    """One number a section computes: its name (the key in results.json and in the document template is
    key_of(section, name)) and the computed value; None when it could not be computed (`status` says why, e.g.
    'needs data' without the run records)."""
    section: str
    name: str
    got: float | None = None
    status: str = ''
    note: str = ''

    def missing(self):
        return self.got is None or (isinstance(self.got, float) and math.isnan(self.got))

    def shown(self):
        return '-' if self.missing() else f'{self.got:,.6g}'


@dataclass
class Finding:
    """One conclusion RESEARCH.md draws, tested on the reproduced numbers: `holds` is the test's outcome (None when the
    numbers could not be computed), `evidence` the numbers it rests on, as printed."""
    section: str
    statement: str
    holds: bool | None
    evidence: str = ''

    def verdict(self):
        return 'needs data' if self.holds is None else 'holds' if self.holds else 'FAILS'


def finding(section, statement, test, evidence=''):
    """a Finding from a test that may need missing numbers: `test` is a bool or a function returning one; any error
    computing it (a missing number) gives holds = None"""
    try: holds = bool(test() if callable(test) else test)
    except (TypeError, KeyError, ValueError, ZeroDivisionError): holds = None
    if callable(evidence):
        try: evidence = evidence()
        except (TypeError, KeyError, ValueError, ZeroDivisionError): evidence = ''
    return Finding(section, statement, holds, evidence)


def say_missing():
    print('run records not found (research/data/runs.jsonl, not included): data claims skipped')


def findings_table(fs, out=print):
    if not fs: return
    out('\nFindings (each conclusion, tested on the reproduced numbers):')
    for f in fs:
        out(f'{f.section:7s}  {f.verdict():10s}  {f.statement}')
        if f.evidence: out(f'{"":19s}{f.evidence}')


def table(items, out=print):
    vals = [c for c in items if isinstance(c, Value)]
    w = max([len(c.name) for c in vals] + [10])
    out(f'{"section":7s}  {"value":{w}s}  {"computed":>14s}')
    for c in vals:
        out(f'{c.section:7s}  {c.name:{w}s}  {c.shown():>14s}' + (f'  ({c.status or c.note})' if (c.status or c.note) else ''))
    findings_table([f for f in items if isinstance(f, Finding)], out)


def main(claims_fn):
    """run one section script alone: print its claims"""
    if not have_runs(): say_missing()
    table(claims_fn())
