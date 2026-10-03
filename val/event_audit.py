"""Audit the hand-entered pal and group card entries against GameTora's event files.

For every Global pal / group card, decodes every event section GameTora stores (storyline, first meeting, outings,
card-specific events, per-card variants), reading reward lists only (a choice's first element is its label, not a
reward), and lists each target-skill hint the card can give. Each is checked against the card's entry in
pkg/loopopt/card_events.json (always / chance / disabled). Exit code 1 if any target hint is unaccounted for.

usage (repo root): python3 val/event_audit.py [--all]     --all also prints the non-target hints
Needs pkg/loopopt/export/master.mdb.
"""
import json, glob, sqlite3, sys, os

R = os.path.dirname(os.path.abspath(__file__)) + '/../'
G = R + 'pkg/loopopt/gametora/'
c = sqlite3.connect(R + 'pkg/loopopt/export/master.mdb')
q = lambda s, *a: c.execute(s, a).fetchall()
sk = dict(q("select [index], text from text_data where category = 47"))
title = dict(q("select [index], text from text_data where category = 181"))
cname = dict(q("select [index], text from text_data where category = 75"))
pals = {cid: ch for cid, ch in q("select id, chara_id from support_card_data where support_card_type in (2, 3)")}
ev = json.load(open(glob.glob(G + 'dict_evrew.*.json')[0]))
OFF = 36   # reward codes index dict_evrew with this offset (see pkg/loopopt/export/build_card_events.py)
TARGETS = {"Uma Stan", "Superstan", "Nimble Navigator", "No Stopping Me!", "Playtime's Over!", "See Ya Later!",
           "Ramp Up", "It's On!", "Tail Held High", "Tail Nine", "Racing Spirit: Stamina", "Slipstream",
           "Perfect Spot!", "Risky Business", "Nothing Ventured"}
SHOW_ALL = '--all' in sys.argv


def hints(rewards):
    out = []
    for k in rewards if isinstance(rewards, list) else []:
        r = ev[k - OFF] if isinstance(k, int) and 0 <= k - OFF < len(ev) else None
        if r and r[0] == 'sk':
            out.append((sk.get(r[2], f'(JP-only {r[2]})'), r[1]))
        if r and r[0] == 'sr':
            out += [(sk.get(z['d'], '?'), 'random') for z in r[2]]
    return out


def event_hints(choices):
    return [h for ch in (choices if isinstance(choices, list) else [])
            if isinstance(ch, list) and len(ch) == 2 and isinstance(ch[1], list) for h in hints(ch[1])]


def walk(row, kind):
    """Yield (event title, card id a variant is limited to or None, hints)."""
    parts = row[1:] if kind == 'friend' else row[2:]
    for part in parts:
        for e in part if isinstance(part, list) else []:
            if not (isinstance(e, list) and len(e) >= 3):
                continue
            name = title.get(int(e[2]) if str(e[2]).isdigit() else 0, str(e[2]))
            yield name, None, event_hints(e[1])
            if len(e) > 3 and isinstance(e[3], list) and len(e[3]) > 1 and isinstance(e[3][1], list):
                for z in e[3][1]:
                    if isinstance(z, list) and len(z) >= 2 and isinstance(z[1], list) and len(z[1]) >= 2:
                        yield name, z[0], event_hints(z[1][1])


CE = json.load(open(R + 'pkg/loopopt/card_events.json'))['cards']
bad = 0
for kind in ('friend', 'group'):
    for row in json.load(open(glob.glob(G + f'training_events_{kind}.*.json')[0])):
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
