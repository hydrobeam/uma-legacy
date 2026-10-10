"""Audit the hand-entered pal and group card entries against GameTora's event files.

For every Global pal / group card, reads every event section GameTora stores (storyline, first meeting, outings,
card-specific events, per-card variants) through pkg/loopopt/export/gametora.py, and lists each target-skill hint the
card can give. Each is checked against the card's entry in
pkg/loopopt/data/global/card_events.json (always / chance / disabled). Exit code 1 if any target hint is unaccounted for.

usage (repo root): python3 val/event_audit.py [--all]     --all also prints the non-target hints
Needs game/global/master.mdb.
"""
import json, sqlite3, sys, os

R = os.path.dirname(os.path.abspath(__file__)) + '/../'
sys.path.insert(0, R + 'pkg/loopopt/export')
import gametora
c = sqlite3.connect(R + 'game/global/master.mdb')
q = lambda s, *a: c.execute(s, a).fetchall()
sk = dict(q("select [index], text from text_data where category = 47"))
title = dict(q("select [index], text from text_data where category = 181"))
cname = dict(q("select [index], text from text_data where category = 75"))
pals = {cid: ch for cid, ch in q("select id, chara_id from support_card_data where support_card_type in (2, 3)")}
TARGETS = {"Uma Stan", "Superstan", "Nimble Navigator", "No Stopping Me!", "Playtime's Over!", "See Ya Later!",
           "Ramp Up", "It's On!", "Tail Held High", "Tail Nine", "Racing Spirit: Stamina", "Slipstream",
           "Perfect Spot!", "Risky Business", "Nothing Ventured"}
SHOW_ALL = '--all' in sys.argv


def event_hints(choices):
    """(skill name, level or 'random') for every hint of an event's choices"""
    out = []
    for ch in gametora.choices(choices):
        for kind, x, lv in gametora.hints(ch[1]):
            if kind == 'hint': out.append((sk.get(x, f'(JP-only {x})'), lv))
            else: out += [(sk.get(d, '?'), 'random') for d in x]
    return out


def walk(row, kind):
    """Yield (event title, card id a variant is limited to or None, hints)."""
    for e, only, chs in gametora.events(row, kind):
        name = title.get(int(e[2]) if str(e[2]).isdigit() else 0, str(e[2]))
        yield name, only, event_hints(chs)


CE = json.load(open(R + 'pkg/loopopt/data/global/card_events.json'))['cards']
bad = 0
for kind in ('friend', 'group'):
    for row in gametora.load(f'training_events_{kind}'):
        owners = [cid for cid, ch in pals.items() if (ch == row[0] if kind == 'friend' else cid == row[0])]
        if not owners:
            continue
        evs = list(walk(row, kind))
        for cid in sorted(owners):
            e = CE.get(str(cid), {})
            covered = set(e.get('always', [])) | {n for n, _ in e.get('chance', [])} | {n for n, _ in e.get('disabled', [])}
            print(f"{cid} {cname.get(cid)}  [always={e.get('always')} chance={e.get('chance')} disabled={e.get('disabled')}]")
            for name, only, hs in evs:
                if only is not None and only != cid:
                    continue
                if only is None and any(o == cid and n == name for n, o, _ in evs):
                    continue   # this card has its own version of the event
                for h, lv in hs:
                    if h in TARGETS:
                        ok = h in covered
                        bad += not ok
                        print(f"   {'ok     ' if ok else 'MISSING'} {h} {lv}  ({name})")
                    elif SHOW_ALL:
                        print(f"           {h} {lv}  ({name})")
print(f"\n{bad} target hint(s) not accounted for" if bad else "\nevery target hint is accounted for")
sys.exit(1 if bad else 0)
