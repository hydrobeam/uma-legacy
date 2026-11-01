"""The game era an export runs for: `global` or `jp` (the Grand Masters era: the JP game database cut at L'Arc's launch,
with English names taken from the Global database by id). Database-backed exporters take an `Era` (built from the
era's name, the first argument of every exporter) for their paths, names and release rules; nothing here runs at
import.

Global keeps everything the database dates up to AHEAD past the build (the page hides each entry until its start time); JP keeps what started before L'Arc's launch."""

import argparse
import functools
import os
import sqlite3
import time

ERAS = ("global", "jp")
R = os.path.dirname(os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))
GLOBAL_MDB = os.path.join(R, "game", "global", "master.mdb")
AHEAD = 60 * 86400  # Global keeps what the database dates up to this far past the build; the page hides it until then


def add_argument(parser: argparse.ArgumentParser) -> None:
    """the exporters' first argument, the era"""
    parser.add_argument("era", choices=ERAS, help="global, or jp: the Grand Masters era")


def read_only(path):
    return sqlite3.connect(f"file:{path}?mode=ro", uri=True)


class Era:
    """One era's database (read-only), output folder and release cut"""

    def __init__(self, name):
        assert name in ERAS, name
        self.name = name
        self.jp = name == "jp"
        self.mdb = os.path.join(R, "game", name, "master.mdb")
        self.out = os.path.join(R, "pkg", "loopopt", "data", name)  # the era's data files
        self.base = os.path.join(self.out, "base.json")  # the roster the exporters share
        self.connection = read_only(self.mdb)
        self.global_connection = read_only(GLOBAL_MDB)
        self.now = int(time.time())
        # JP: L'Arc's start
        self.cut = (
            self.q("select start_date from single_mode_scenario where id=6")[0][0] if self.jp else self.now + AHEAD
        )

    def q(self, sql, *args):
        return self.connection.execute(sql, args).fetchall()

    def names(self, category):
        """text_data names of one category by id, in English: the Global text where the id exists there, else the
        database's own (JP-only entries keep their Japanese text)."""
        own_names = dict(self.q('select "index", text from text_data where category=?', category))
        english_names = dict(
            self.global_connection.execute('select "index", text from text_data where category=?', (category,))
        )
        return {k: english_names.get(k, v) for k, v in own_names.items()}

    @functools.cached_property
    def outfit_starts(self):
        """outfit (trainee card) id -> its start: its first gacha or exchange-ticket listing; a main-story reward
        with no such listing counts from the start; otherwise its uma's start date."""
        first = {}
        for card_id, start_date in self.q(
            "select a.card_id, min(g.start_date) from gacha_available a join gacha_data g on g.id=a.gacha_id where a.card_type=1 group by a.card_id"
        ):
            first[card_id] = start_date
        if self.q("select 1 from sqlite_master where name='exchange_ticket_detail'"):
            for card_id, start_date in self.q(
                "select card_id, min(start_date) from exchange_ticket_detail where card_type=1 group by card_id"
            ):
                first[card_id] = min(first.get(card_id, start_date), start_date)
        for (card_id,) in self.q("select add_reward_id_1 from main_story_data where add_reward_category_1=50"):
            first.setdefault(card_id, 0)
        character_starts = dict(self.q("select id, start_date from chara_data"))
        return {
            card_id: first.get(card_id, character_starts.get(character_id, 0))
            for card_id, character_id in self.q("select id, chara_id from card_data where id<900000")
        }

    def outfit_released(self, card_id):
        return self.outfit_starts[card_id] < self.cut

    def released(self, start):
        """an uma's or support card's start date falls before the era's cut"""
        return start < self.cut
