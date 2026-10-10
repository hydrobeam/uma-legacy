"""The game era an export runs for, given as the script's argument: `global` or `jp` (the Grand Masters era: the JP
game database cut at L'Arc's launch, with English names taken from the Global database by id). Every exporter here
reads its database, output folder, names and release rules from this module.

The cut: Global keeps everything the database dates up to upcoming.AHEAD past the build (the page hides each entry
until its start time); JP keeps what started before L'Arc's launch, an outfit counting from its first gacha or
exchange-ticket listing (outfits given as main-story rewards count as released)."""
import os, sqlite3, sys

ERA = next((a for a in sys.argv[1:] if a in ('global', 'jp')), None)
if ERA is None: sys.exit(f'usage: {os.path.basename(sys.argv[0])} global|jp ...')
JP = ERA == 'jp'
R = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
GLOBAL_MDB = os.path.join(R, 'game', 'global', 'master.mdb')
MDB = os.path.join(R, 'game', ERA, 'master.mdb')
OUT = os.path.join(R, 'jp') if JP else os.path.join(R, 'pkg', 'loopopt')   # the era's data files
BASE = os.path.join(OUT, 'export', 'base.json')                              # the roster the exporters share
c = sqlite3.connect(f'file:{MDB}?mode=ro', uri=True)
q = lambda s, *a: c.execute(s, a).fetchall()
_gl = sqlite3.connect(f'file:{GLOBAL_MDB}?mode=ro', uri=True) if JP else c


def names(cat):
    """text_data names of one category by id; for JP the Global English text where the id exists there."""
    own = dict(q('select "index", text from text_data where category=?', cat))
    if not JP: return own
    en = dict(_gl.execute('select "index", text from text_data where category=?', (cat,)).fetchall())
    return {k: en.get(k, v) for k, v in own.items()}


if JP:
    CUT = q("select start_date from single_mode_scenario where id=6")[0][0]   # L'Arc's start
    _rel = {}
    for cid, sd in q("select a.card_id, min(g.start_date) from gacha_available a join gacha_data g on g.id=a.gacha_id where a.card_type=1 group by a.card_id"):
        _rel[cid] = sd
    for cid, sd in q("select card_id, min(start_date) from exchange_ticket_detail where card_type=1 group by card_id"):
        _rel[cid] = min(_rel.get(cid, sd), sd)
    for (cid,) in q("select add_reward_id_1 from main_story_data where add_reward_category_1=50"):
        _rel.setdefault(cid, 0)
    def outfit_released(cid): return _rel.get(cid, 10**12) < CUT
    def card_released(start): return start < CUT
else:
    from upcoming import CUT, outfit_starts
    _starts = None
    def outfit_released(cid):
        global _starts
        if _starts is None: _starts = outfit_starts(q)
        return _starts[cid] <= CUT
    def card_released(start): return start <= CUT
