"""Section 10, conditional character events: secret and race-result events that give skill hints (GameTora
training_events_char through gametora.py, current versions; skills from master.mdb), how many touch the default
targets and whether any gives a gold. Forcing them in the solver is an experiment, not redone here. Also checks the one
exception modelled as always acquired, Tokai Teio's Head-On: its event's condition (GameTora), her career goals
(master.mdb single_mode_route_race; condition_value_1 read as the place the goal requires) and, in the run records
(research/data/runs.jsonl, not included; Independent Training runs from uma-it), the runs training her."""
import common
from common import Value

S = '10'
TARGETS = set(common.DEFAULT)                                          # skill groups of the eight default targets


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
    return [Value(S, 'umas with a hint event', len({u for u, _ in ev})),
            Value(S, 'events with hints', len(ev)),
            Value(S, 'umas with a hint event, Global', len({u for u, _ in ev if u in glob})),
            Value(S, 'events with hints, Global', sum(u in glob for u, _ in ev)),
            Value(S, 'events touching a default target', len(tg), note=', '.join(names)),
            Value(S, 'of them giving a gold', sum(any(grp.get(h) in TARGETS and rar.get(h, 1) >= 2 for h in hs) for _, hs in tg))] + \
        head_on(ev, sk)


TEIO, CROWN = 1003, ('Satsuki Sho', 'Tokyo Yushun (Japanese Derby)', 'Kikuka Sho')   # the Triple Crown's three races


def head_on(ev, sk):
    """Tokai Teio's Head-On: the Triple Crown event, her goals, and the runs training her"""
    gt = common.gametora(); db = common.mdb()
    conds = [gt.extra(ve, 0) for r in gt.load('training_events_char') if r[0] == TEIO
             for tag, ve in gt.conditional_events(r) if tag.startswith('current')
             and 'Head-On' in {sk.get(x) for ch in gt.choices(ve[1]) for kind, x, _ in gt.hints(ch[1]) for x in ([x] if kind == 'hint' else x)}]
    race = dict(db.execute('select [index], text from text_data where category = 32'))
    goals = {race.get(rid): place for rid, place in db.execute(
        'select ri.race_id, r.condition_value_1 from single_mode_route_race r join single_mode_route m on m.race_set_id = r.race_set_id '
        'join single_mode_program p on p.id = r.condition_id join race_instance ri on ri.id = p.race_instance_id '
        'where m.chara_id = ? and m.scenario_id = 0 and r.condition_type = 1', (TEIO,))}
    out = [common.finding(S, 'Game data: Tokai Teio\'s Head-On event needs the Triple Crown, and all three of its races are '
                          'her career goals', conds == [[['triple_crown']]] and all(c in goals for c in CROWN),
                          f'conditions {conds}; goal places required ' + ', '.join(f'{c} {goals.get(c)}' for c in CROWN))]
    R = common.runs()
    if R is None: return out
    n = won = ev_ok = room = 0
    for r in R:
        if r['chara'] != TEIO: continue
        n += 1; w = {x['race'] for x in r['races'] if x['won']}; won += all(c in w for c in CROWN)
        h = next((h for h in r['hints'] if h['name'] == 'Head-On'), None)
        if h and 'Events' not in h['src'] and h['lv'] >= 5: continue   # at Lv 5 from elsewhere: her event cannot show
        room += 1; ev_ok += bool(h and 'Events' in h['src'])
    lo, _ = common.wilson(ev_ok, room)
    out += [Value(S, 'runs training Tokai Teio', n), Value(S, 'of them winning all three', won)]
    out.append(common.finding(S, 'The goals only require placings, so certainty rests on IT winning them: in every run training '
                              'Tokai Teio (any outfit) she won all three, and Head-On is credited to her events wherever it '
                              'could show', n > 0 and won == n and ev_ok == room,
                              f'won all three {won}/{n}; Head-On from events {ev_ok}/{room} (95% lower {lo:.3f}; '
                              f'{n - room} run at Lv 5 from inspiration)'))
    return out


if __name__ == '__main__': common.main(claims)
