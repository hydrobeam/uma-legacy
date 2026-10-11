"""Section 10, conditional character events: secret and race-result events that give skill hints (GameTora
training_events_char through gametora.py, current versions; skills from master.mdb), how many touch the default
targets and whether any gives a gold. Also checks the one exception modelled as always acquired, Tokai Teio's Head-On:
its event's condition (GameTora), her career goals (master.mdb single_mode_route_race; condition_value_1 read as the
place the goal requires) and, in the run records (research/data/runs.jsonl, not included; Independent Training runs
from uma-it), the runs training her.

Effect on real careers (section 13's calibration rows, read through s10_random_events.calib: each run rebuilt on its
own deck, default targets). Section 12's jobs carry no trainee event hints (e_k = 0 on every row) and the run records
credit every trainee and scenario event alike as "Events", so neither separates ordinary trainee events from
conditional ones; they are separated here by game data (s10_random_events.event_groups):
  rows        free rows (not on the trainee's outfit, not a scenario gold) whose trainee has a current conditional event
              that can hint the target; a row counts as conditional-only when none of her story or date events, her
              outfit's events, her shared random event, nor the run's scenario's events can hint it, so an "Events"
              credit there can only be a conditional event; rows where her shared random event can also hint it are
              kept apart (her events together, not separable)
  rate        per (trainee, target), and pooled over the conditional-only rows: "Events" credits / rows, Wilson 95%
  crediting   each row at its (trainee, target) rate, as in section 10 (m multiplied by (1 - r), section 12):
              relative change of each run's product of per-target odds, and acquisition observed against predicted
              without and with crediting (obs / pred, Wilson 95%; Bernoulli log-likelihood over the rows with a
              positive prediction, tested against the chi2 95% point of one degree of freedom per fitted rate)
"""
import collections
from scipy.stats import chi2
import common
from common import Value

S = '11'
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
        head_on(ev, sk) + careers()


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


def careers():
    """the conditional events touching default targets on real careers: rates, effect on the odds, calibration"""
    R = common.runs()
    if R is None: return []
    import s10_random_events as s10
    F = lambda st, test, ev: common.finding(S, st, test, ev)
    E = s10.event_groups(); rows, runrows = s10.calib(); byid = {r['id']: r for r in R}
    name = dict(common.mdb().execute('select [index], text from text_data where category = 6')); gn = common.skills()
    sel, skipped = [], collections.Counter()
    for row in rows:
        r = byid[row['run']]; u, g = r['chara'], row['g']
        if not any(g in gs for gs, _ in E['cond'].get(u, [])): continue
        if not s10.free(row): skipped['outfit or scenario gold'] += 1; continue
        if g in E['ordinary'].get(u, set()) | E['outfit'].get(r['outfit'], set()) | E['scen'].get(r['scenario'], set()):
            skipped['story, date, outfit or scenario event can hint it'] += 1; continue
        hit = any('Events' in h['src'] for h in s10.hint_srcs(r)[g])
        sel.append(dict(row=row, key=(u, g), mixed=g in E['shared'].get(u, set()), hit=hit))
    groups = collections.defaultdict(list)
    for x in sel: groups[x['key']].append(x)
    rate = {k: sum(x['hit'] for x in xs) / len(xs) for k, xs in groups.items()}
    lab = lambda k: f'{name.get(k[0])}, any outfit, {gn[k[1]][0]}'
    cond = [x for x in sel if not x['mixed']]; mixed = [x for x in sel if x['mixed']]
    kc = sum(x['hit'] for x in cond); lo, hi = common.wilson(kc, len(cond))
    out = [Value(S, 'careers: conditional-only rows', len(cond)), Value(S, 'careers: conditional-only, from her events', kc),
           Value(S, 'careers: conditional-only rate', common.div(kc, len(cond))),
           Value(S, 'careers: conditional-only rate 95% low', lo), Value(S, 'careers: conditional-only rate 95% high', hi),
           Value(S, 'careers: rows with her random event too', len(mixed)),
           Value(S, 'careers: rows with her random event too, from her events', sum(x['hit'] for x in mixed)),
           Value(S, 'careers: rows left out, another event can hint it', skipped['story, date, outfit or scenario event can hint it']),
           Value(S, 'careers: rows left out, outfit or scenario gold', skipped['outfit or scenario gold'])]
    for k in sorted(groups, key=lab):
        xs = groups[k]; h = sum(x['hit'] for x in xs); l_, h_ = common.wilson(h, len(xs))
        out += [Value(S, f'{lab(k)}: rows', len(xs)), Value(S, f'{lab(k)}: from her events', h),
                Value(S, f'{lab(k)}: rate 95% low', l_), Value(S, f'{lab(k)}: rate 95% high', h_)]
    rr = {id(x['row']): rate[x['key']] for x in sel}
    ch, tch, lifted, pos = s10.odds_change(runrows, lambda row: rr.get(id(row), 0.0))
    for lb, xs in (('', ch), (', runs with a credited row', tch), (', targets with a positive chance, runs with a credited row', pos)):
        m, md, mx = s10.summary(xs)
        out += [Value(S, f'odds change{lb}: runs', len(xs)), Value(S, f'odds change{lb}: mean', m),
                Value(S, f'odds change{lb}: median', md), Value(S, f'odds change{lb}: max', mx)]
    out.append(Value(S, 'odds change: runs lifted from zero', lifted))
    t = s10.calib_test([x['row'] for x in sel], lambda row: rr[id(row)])
    tc = s10.calib_test([x['row'] for x in cond], lambda row: rr[id(row)])
    for pre, tt in (('calibration', t), ('calibration, conditional-only', tc)):
        out += [Value(S, f'{pre}: rows', tt['n']), Value(S, f'{pre}: observed', tt['obs']),
                Value(S, f'{pre}: obs / pred, left out', tt['ratio0']), Value(S, f'{pre}: ratio 95% low, left out', tt['rlo0']),
                Value(S, f'{pre}: ratio 95% high, left out', tt['rhi0']), Value(S, f'{pre}: obs / pred, credited', tt['ratio1']),
                Value(S, f'{pre}: ratio 95% low, credited', tt['rlo1']), Value(S, f'{pre}: ratio 95% high, credited', tt['rhi1']),
                Value(S, f'{pre}: LL credited minus left out, positive rows', tt['ll1'] - tt['ll0'])]
    zc = [x for x in cond if s10.credit(x['row'], 0.0)[1] <= 1e-12]
    out += [Value(S, 'careers: conditional-only rows the model gives no chance', len(zc)),
            Value(S, 'careers: of them acquired', sum(x['row']['o_any'] for x in zc)),
            Value(S, 'careers: of them from her events', sum(x['hit'] for x in zc))]
    rt = lambda tt, k: f'{tt["ratio" + k]:.3f} [{tt["rlo" + k]:.2f}, {tt["rhi" + k]:.2f}]'
    covers = lambda tt, k: tt[f'rlo{k}'] <= 1 <= tt[f'rhi{k}']
    lim = lambda keys: chi2.ppf(0.95, len(keys)) / 2
    kall = set(groups); kcond = {x['key'] for x in cond}
    out += [F('Conditional events touching default targets hint them on real careers: on the conditional-only rows, the '
              'pooled rate\'s 95% interval excludes 0', lambda: lo > 0,
              lambda: f'{kc} of {len(cond)} rows, {kc / len(cond):.3f} [{lo:.3f}, {hi:.3f}]; by trainee and target: ' +
                      '; '.join(f'{lab(k)} {sum(x["hit"] for x in xs)}/{len(xs)}' for k, xs in sorted(groups.items(), key=lambda kv: lab(kv[0])))),
            F('Conditional-only rows, left out: acquisition obs / pred has a 95% interval covering 1',
              lambda: covers(tc, '0'), lambda: f'{tc["n"]} rows; left out {rt(tc, "0")}; credited {rt(tc, "1")}'),
            F('Conditional-only rows: crediting each (trainee, target) at its measured rate does not improve the fit significantly: '
              'the gain on the rows with a positive prediction is below the chi2 95% point (one degree of freedom per fitted rate, halved)',
              lambda: tc['ll1'] - tc['ll0'] < lim(kcond),
              lambda: f'{tc["ll1"] - tc["ll0"]:+.1f} nats against {lim(kcond):.2f} ({len(kcond)} rates; {tc["n_zero"]} rows at 0 apart)'),
            F('Conditional-only rows, left out: the model gives some of them no chance, and her events hinted the target on at least '
              'one of them', lambda: len(zc) > 0 and sum(x['hit'] for x in zc) > 0,
              lambda: f'{len(zc)} rows at 0; {sum(x["row"]["o_any"] for x in zc)} acquired, {sum(x["hit"] for x in zc)} from her events'),
            F('With Vodka\'s rows included (her random event can hint the target too), leaving these trainees\' events out '
              'under-predicts acquisition: obs / pred has a 95% interval entirely above 1', lambda: t['rlo0'] > 1, lambda: f'{t["n"]} rows; left out {rt(t, "0")}; credited {rt(t, "1")}'),
            F('All these trainees\' rows: crediting at the measured rates improves the fit by more than the chi2 95% point (one '
              'degree of freedom per fitted rate, halved) on the rows with a positive prediction',
              lambda: t['ll1'] - t['ll0'] > lim(kall),
              lambda: f'{t["ll1"] - t["ll0"]:+.1f} nats against {lim(kall):.2f} ({len(kall)} rates; {t["n_zero"]} rows at 0 apart)')]
    return out


if __name__ == '__main__': common.main(claims)
