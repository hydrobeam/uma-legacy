"""JP master.mdb helpers: the Grand Masters-era cutoff (L'Arc's JP start) and English names taken from the
Global master.mdb by the same ids (JP-only entries keep their Japanese text).
Other cuts: JP_CUT=<unix time> (content released before it), JP_OUT=<folder> for the exported files (default ..),
JP_BASE=<file> for the intermediate roster file (default base.json)."""
import sqlite3, os
GLOBAL = '../../pkg/loopopt/export/master.mdb'
_jp = sqlite3.connect('master.mdb'); _gl = sqlite3.connect(GLOBAL)
CUT = int(os.environ.get('JP_CUT') or _jp.execute("select start_date from single_mode_scenario where id=6").fetchone()[0])   # L'Arc start
OUT = os.environ.get('JP_OUT', '..')
BASE = os.environ.get('JP_BASE', 'base.json')
def names(cat):
    jp = dict(_jp.execute('select "index", text from text_data where category=?', (cat,)).fetchall())
    en = dict(_gl.execute('select "index", text from text_data where category=?', (cat,)).fetchall())
    return {k: en.get(k, v) for k, v in jp.items()}
def is_en(cat, k):
    return _gl.execute('select 1 from text_data where category=? and "index"=?', (cat, k)).fetchone() is not None
# outfit release = first gacha that offered it (or first exchange-ticket listing)
_rel = {}
for cid, sd in _jp.execute("select a.card_id, min(g.start_date) from gacha_available a join gacha_data g on g.id=a.gacha_id where a.card_type=1 group by a.card_id"):
    _rel[cid] = sd
for cid, sd in _jp.execute("select card_id, min(start_date) from exchange_ticket_detail where card_type=1 group by card_id"):
    _rel[cid] = min(_rel.get(cid, sd), sd)
# outfits given as main-story rewards have no gacha or ticket date; count them as released (Special Week [Ruler of
# Japan], main story part 7 episode 13, belongs in the GM era)
for (cid,) in _jp.execute("select add_reward_id_1 from main_story_data where add_reward_category_1=50"):
    _rel.setdefault(cid, 0)
def outfit_released(cid): return _rel.get(cid, 10**12) < CUT
