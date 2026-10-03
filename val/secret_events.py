"""Every conditional character event (secret and race-result events (GameTora training_events/char, section 4) that gives a skill hint, with its
conditions decoded, and a feasibility check of the race conditions against the solver's Global data.json.
Writes docs/SECRET_EVENTS.md (per uma, then an index by skill). usage: uv run val/secret_events.py [OUT.md]"""
import json, glob, sqlite3, collections, sys
import os
REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
P = REPO + '/pkg/loopopt/'
ev = json.load(open(glob.glob(P + 'gametora/dict_evrew.*.json')[0])); OFF = 36
c = sqlite3.connect(f'file:{P}export/master.mdb?mode=ro', uri=True)
sk = dict(c.execute("select [index],text from text_data where category=47"))
grp = dict(c.execute("select id, group_id from skill_data"))
ttl = dict(c.execute("select [index],text from text_data where category=181"))
cname = dict(c.execute("select [index],text from text_data where category=6"))
cj = sqlite3.connect(f'file:{P}../../jp/export/master.mdb?mode=ro', uri=True)
for k, v in cj.execute("select [index],text from text_data where category=6"): cname.setdefault(k, v)
for k, v in cj.execute("select [index],text from text_data where category=181"): ttl.setdefault(k, v)
skj = dict(cj.execute("select [index],text from text_data where category=47"))
for k, v in skj.items(): sk.setdefault(k, v)
for k, g in cj.execute("select id, group_id from skill_data"): grp.setdefault(k, g)
rinst = {i: (rid, nm) for i, rid, nm in c.execute("select ri.id, ri.race_id, (select text from text_data where category=28 and [index]=ri.id) from race_instance ri")}
d = json.load(open(P + 'data.json'))
ridx = {r: k for k, r in enumerate(d['races'])}
U = {u['id']: u for u in d['umas']}
TARGETS = {20159, 20049, 20166, 20046, 20161, 21010, 20165, 20203}
YEAR = {1: 'Junior', 2: 'Classic', 3: 'Senior'}

def reward(k):
    r = ev[k - OFF] if isinstance(k, int) and 0 <= k - OFF < len(ev) else None
    if not r: return None
    if r[0] == 'sk': return ('hint', r[2], r[1])
    if r[0] == 'sr': return ('random', [x['d'] for x in r[2]], None)
    return None

def forced(u):
    """turn -> set of option indices allowed by a goal that has a single alternative (turns are 1-based in goals)"""
    out = {}
    for alts in u['goals']:
        if len(alts) == 1:
            t, o = alts[0]; out[t - 1] = {o}
    return out

def feasible(uid, inst, year):
    """'ok' / 'impossible: reason' / 'unknown: reason' for winning race instance `inst` in `year` (None = any)."""
    if uid not in U: return 'unknown: uma not in Global data'
    rid, nm = rinst.get(inst, (None, str(inst)))
    if rid not in ridx: return 'unknown: not a G1 (outside solver data)'
    u = U[uid]; f = forced(u); turns = []
    for t, opts in enumerate(u['turns']):
        if year and t // 24 != year - 1: continue
        for o, op in enumerate(opts):
            if op['race'] == ridx[rid]: turns.append((t, o))
    if not turns: return f'impossible: not offered{" in " + YEAR[year] if year else ""}'
    free = [(t, o) for t, o in turns if t not in f or o in f[t]]
    if not free:
        t, o = turns[0]; other = [u['turns'][t][x]['name'] for x in f[t]]
        return f'impossible: clashes with goal {"/".join(other)}'
    return 'ok'

def rref(v):
    v = str(v)
    return (int(v.split('|')[0]), int(v.split('|')[1])) if '|' in v else (int(v), None)
def rname(v):
    inst, yr = rref(v); nm = rinst.get(inst, (None, f'race {inst}'))[1] or f'race {inst}'
    return nm + (f' ({YEAR[yr]})' if yr else '')
def rfeas(uid, v):
    inst, yr = rref(v); return feasible(uid, inst, yr)
MUST_RUN = {'win', 'participate', 'beat_rival', 'lose_to_rival', 'rival_draw', 'race_w2', 'pick_and_win', 'dont_pick_and_win', 'win_on_streak',
            'race_pn', 'lose', 'win_as_strat', 'win_as_not_strat'}
def cond_text(uid, cs):
    out, bad = [], []
    for cnd in cs or []:
        kind, args = cnd[0], cnd[1:]
        try:
            if kind in MUST_RUN and args and not isinstance(args[0], list) and str(args[0]).replace('|', '').isdigit():
                fz = rfeas(uid, args[0]); rest = ' '.join(str(a) for a in args[1:])
                extra = f" vs {cname.get(int(args[1]), args[1])}" if kind in ('beat_rival', 'lose_to_rival', 'rival_draw') else (f' {rest}' if rest else '')
                out.append(f"{kind} {rname(args[0])}{extra}" + ('' if fz == 'ok' else f' [{fz}]'))
                if fz.startswith('impossible'): bad.append(rname(args[0]))
            elif kind in ('win_or', 'win_n_of', 'win_all'):
                n, lst = (1, args) if kind == 'win_or' else ((args[0], args[1]) if kind == 'win_n_of' else (len(args[0]), args[0]))
                ok = [v for v in lst if not rfeas(uid, v).startswith('impossible')]
                out.append(f"{kind} {n} of: " + ', '.join(rname(v) + ('' if not rfeas(uid, v).startswith('impossible') else ' [impossible]') for v in lst))
                if len(ok) < n: bad.append(kind)
            elif kind in ('do_not_participate', 'do_not_win'):
                out.append(f"{kind} {rname(args[0])}")
            elif kind == 'date':
                out.append(f"fires: {YEAR.get(args[0], args[0])} month {args[1]} {'early' if args[2] == 1 else 'late'}")
            elif kind.endswith('rival') and args and str(args[0]).isdigit():
                out.append(f"{kind} {cname.get(int(args[0]), args[0])}")
            else:
                nm = lambda a: (cname.get(a, a) if isinstance(a, int) and 1000 < a < 2000 else a)
                out.append(' '.join([kind] + [json.dumps([nm(x) for x in a] if isinstance(a, list) else nm(a), ensure_ascii=False) for a in args]))
        except Exception as ex:
            out.append(json.dumps(cnd, ensure_ascii=False) + f' (unparsed: {ex})')
    return out, bad

rows = []
for r in json.load(open(P + 'gametora/' + os.path.basename(sorted(glob.glob(P + 'gametora/training_events_char.*.json'))[-1]) + '')):
    uid = r[0]; secret = r[4] if len(r) > 4 and isinstance(r[4], list) else []
    for e in secret:
        variants = [('current', e)]
        for x in e[3:]:
            if isinstance(x, list) and x and x[0] == 9:
                for tag, ve in x[1]: variants.append((tag, ve))
        for tag, ve in variants:
            hints = []; choices = []
            for ci, ch in enumerate(ve[1]):
                if isinstance(ch, list) and len(ch) == 2 and isinstance(ch[1], list):
                    hints_before = len(hints)
                    for k in ch[1]:
                        rw = reward(k)
                        if rw and rw[0] == 'hint': hints.append((sk.get(rw[1], rw[1]), rw[2], grp.get(rw[1]) in TARGETS))
                        if rw and rw[0] == 'random': hints.append(('random: ' + '/'.join(sk.get(x, '?') for x in rw[1]), '', any(grp.get(x) in TARGETS for x in rw[1])))
                    choices.append(hints[hints_before:])
            if not hints: continue
            cs = None
            for x in ve[3:]:
                if isinstance(x, list) and len(x) == 2 and x[0] == 0: cs = x[1]
            ctext, bad = cond_text(uid, cs)
            ver = [x[1] for x in ve[3:] if isinstance(x, list) and x and x[0] == 8]
            rows.append(dict(uma=cname.get(uid, uid), uid=uid, in_global=uid in U, title=str(ttl.get(int(ve[2]), '')).replace('\\n', ' ').replace('\n', ' '), version=tag + (f' [{ver[0]}]' if ver else ''),
                             hints=hints, choices=choices, conds=ctext, impossible=bad))
OUT = sys.argv[1] if len(sys.argv) > 1 else REPO + '/docs/SECRET_EVENTS.md'
print(len(rows), 'event variants with hints;', len({x['uid'] for x in rows}), 'umas;', sum(any(h[2] for h in x['hints']) for x in rows), 'touch a default target')

R = rows
md = ['# Conditional character events that give skill hints', '',
      'Source: GameTora `training_events/char` (the per-character section of events with trigger conditions, which holds the',
      'secret events and the race-result events). Rewards decoded from `dict_evrew` (offset 36), choice rewards only.',
      '`*` = one of the default 8 targets. With several choices, IT takes choice 1 unless a prioritized skill is on another.',
      'Race feasibility is checked against the solver\'s Global G1 data: "impossible" = never offered in that year, or on a',
      'turn forced to another career goal. Non-G1 races and other condition kinds are shown raw and not checked.',
      'Older versions (GameTora tags such as pre_2nd_anni) appear only where hints or conditions differ.', '']
by = {}
for r in R: by.setdefault((not r['in_global'], str(r['uma']), r['title']), []).append(r)
last = None
for (jp, uma, title), vs in sorted(by.items()):
    if (jp, uma) != last: md += ['', f'## {uma}' + (' (JP only)' if jp else ''), '']; last = (jp, uma)
    seen = set()
    for i, v in enumerate(vs):
        key = (json.dumps(v['choices']), json.dumps(v['conds']))
        if key in seen: continue
        seen.add(key)
        fmt = lambda hs: ', '.join(f'{h[0]} {h[1]}'.strip() + (' *' if h[2] else '') for h in hs) or 'no hint'
        ch = [c for c in v['choices']]
        rew = fmt(ch[0]) if len(ch) == 1 else ' | '.join(f'choice {k + 1}: {fmt(c)}' for k, c in enumerate(ch))
        ver = '' if i == 0 else f" ({v['version']})"
        md.append(f"- **{title or '(untitled)'}**{ver}: {rew}  ")
        md.append(f"  needs: {'; '.join(v['conds']) or 'no conditions listed'}" + (' — **impossible in this career**' if v['impossible'] else ''))
open(OUT, 'w').write('\n'.join(md) + '\n')
# index by skill: current versions only
idx = {}
for r in R:
    if not r['version'].startswith('current'): continue
    for k, c in enumerate(r['choices']):
        for h in c:
            idx.setdefault(h[0], []).append((r, k, h[1]))
out = ['', '# Index by skill (current versions)', '']
for skn in sorted(idx, key=str):
    ents = '; '.join(f"{r['uma']}{' (JP only)' if not r['in_global'] else ''}: {r['title'] or '(untitled)'} {lv}" + (f' (choice {k + 1})' if len(r['choices']) > 1 else '') + (' — impossible' if r['impossible'] else '') for r, k, lv in idx[skn])
    out.append(f'- **{skn}**: {ents}')
open(OUT, 'a').write('\n'.join(out) + '\n')
