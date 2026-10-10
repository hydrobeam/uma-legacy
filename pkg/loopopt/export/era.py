"""The game era an export runs for, given as the script's argument: `global` or `jp` (the Grand Masters era: the JP
game database cut at L'Arc's launch, with English names taken from the Global database by id). Every exporter here
reads its database, output folder, names and release rules from this module.

The only differences: the database, the output folder and the cut. Global keeps everything the database dates up to
AHEAD past the build (the page hides each entry until its start time); JP keeps what started before L'Arc's launch."""
import os, sqlite3, sys, time

ERA = next((a for a in sys.argv[1:] if a in ('global', 'jp')), None)
if ERA is None: sys.exit(f'usage: {os.path.basename(sys.argv[0])} global|jp ...')
JP = ERA == 'jp'
R = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
GLOBAL_MDB = os.path.join(R, 'game', 'global', 'master.mdb')
MDB = os.path.join(R, 'game', ERA, 'master.mdb')
OUT = os.path.join(R, 'pkg', 'loopopt', 'data', ERA)   # the era's data files
BASE = os.path.join(OUT, 'base.json')                  # the roster the exporters share
c = sqlite3.connect(f'file:{MDB}?mode=ro', uri=True)
q = lambda s, *a: c.execute(s, a).fetchall()
_gl = sqlite3.connect(f'file:{GLOBAL_MDB}?mode=ro', uri=True)

AHEAD = 60 * 86400   # Global keeps what the database dates up to this far past the build; the page hides it until then
NOW = int(time.time())
CUT = q("select start_date from single_mode_scenario where id=6")[0][0] if JP else NOW + AHEAD   # JP: L'Arc's start


def names(cat):
    """text_data names of one category by id, in English: the Global text where the id exists there, else the
    database's own (JP-only entries keep their Japanese text)."""
    own = dict(q('select "index", text from text_data where category=?', cat))
    en = dict(_gl.execute('select "index", text from text_data where category=?', (cat,)).fetchall())
    return {k: en.get(k, v) for k, v in own.items()}


def outfit_starts():
    """outfit (trainee card) id -> its start: its first gacha or exchange-ticket listing; an outfit given as a
    main-story reward counts from the start; otherwise its uma's start date."""
    first = {}
    for cid, sd in q("select a.card_id, min(g.start_date) from gacha_available a join gacha_data g on g.id=a.gacha_id where a.card_type=1 group by a.card_id"):
        first[cid] = sd
    if q("select 1 from sqlite_master where name='exchange_ticket_detail'"):   # the JP database has exchange tickets
        for cid, sd in q("select card_id, min(start_date) from exchange_ticket_detail where card_type=1 group by card_id"):
            first[cid] = min(first.get(cid, sd), sd)
    for (cid,) in q("select add_reward_id_1 from main_story_data where add_reward_category_1=50"):
        first.setdefault(cid, 0)
    chara = dict(q('select id, start_date from chara_data'))
    return {cid: first.get(cid, chara.get(ch, 0)) for cid, ch in q('select id, chara_id from card_data where id<900000')}


_starts = None
def outfit_released(cid):
    global _starts
    if _starts is None: _starts = outfit_starts()
    return _starts[cid] < CUT


def released(start):
    """an uma's or support card's start date falls before the era's cut"""
    return start < CUT
