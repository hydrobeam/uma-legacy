"""Section 10, conditional character events: secret and race-result events that give skill hints (GameTora
training_events_char through gametora.py, current versions; skills from master.mdb), how many touch the default
targets and whether any gives a gold. Forcing them in the solver is an experiment, not redone here."""
import common
from common import Claim

S = '10'
TARGETS = {20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203}     # skill groups of the eight default targets


def claims():
    gt = common.gametora(); db = common.mdb()
    grp = dict(db.execute("select id, group_id from skill_data")); rar = dict(db.execute("select id, rarity from skill_data"))
    sk = dict(db.execute("select [index],text from text_data where category=47"))
    glob = {u['id'] for u in common.gjson('data.json')['umas']}
    ev = []
    for r in gt.load('training_events_char'):
        for tag, ve in gt.conditional_events(r):
            if not tag.startswith('current'): continue
            hs = [x for ch in gt.choices(ve[1]) for kind, x, _ in gt.hints(ch[1]) for x in ([x] if kind == 'hint' else x)]
            if hs: ev.append((r[0], hs))
    tg = [(u, hs) for u, hs in ev if any(grp.get(h) in TARGETS for h in hs)]
    names = sorted({sk[h] for _, hs in tg for h in hs if grp.get(h) in TARGETS})
    return [Claim(S, 'umas with a hint event', '81', len({u for u, _ in ev})),
            Claim(S, 'events with hints', '168', len(ev)),
            Claim(S, 'umas with a hint event, Global', '55', len({u for u, _ in ev if u in glob})),
            Claim(S, 'events with hints, Global', '110', sum(u in glob for u, _ in ev)),
            Claim(S, 'events touching a default target', '11', len(tg), note=', '.join(names)),
            Claim(S, 'of them giving a gold', '0', sum(any(grp.get(h) in TARGETS and rar.get(h, 1) >= 2 for h in hs) for _, hs in tg)),
            Claim(S, 'forced to fire: best loop unchanged', 'every setup', status='needs solver experiment')]


if __name__ == '__main__': common.main(claims)
