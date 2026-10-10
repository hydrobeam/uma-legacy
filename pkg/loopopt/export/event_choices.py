"""One GameTora training event -> the rewards the card-event builders store (shared by the Global and JP builders).

A choice is [label, rewards]; only `rewards` is rewards. 'di' splits a choice into outcomes (taken as equally likely),
'ee' ends the card's event chain, 'sk' / 'sr' give skill hints.
  - best:  skill -> chance over the choices that keep the chain going (k of n outcomes: k/n), the best choice per skill;
  - endb:  the same over the choices that end the chain on every outcome (a separate "end the chain here" option);
  - risk:  [k, n] when IT's default (first) choice ends the chain on k of its n outcomes (it is then not offered);
  - alts:  the full structure of the choices that keep the chain going, [choice][outcome] -> sorted skill names, when
           the event names two or more skills. `best` credits each skill on its own; the solver uses `alts` instead,
           so skills on different choices are never credited together and outcomes of one choice exclude each other."""


def decode(choices, reward, skname):
    best = {}; endb = {}; has_end = False; risk = None; normal = []; first = True
    for ch in choices:
        if not (isinstance(ch, list) and len(ch) == 2 and isinstance(ch[1], list)): continue
        top, first = first, False   # IT takes the first choice unless a prioritized skill is elsewhere
        branches = [[]]; ends = [False]
        for k in ch[1]:
            r = reward(k)
            if r and r[0] == 'di': branches.append([]); ends.append(False); continue
            if r and r[0] == 'ee': ends[-1] = True
            if r and r[0] == 'sk' and r[2] in skname: branches[-1].append(skname[r[2]])
            if r and r[0] == 'sr': branches[-1].extend(skname[x['d']] for x in r[2] if x['d'] in skname)
        # A choice that ends the chain on only some outcomes is not offered as a choice; when it is IT's default
        # choice, the chain stops at this link on k of its n outcomes (stop_risk).
        if any(ends) and not all(ends):
            if top: risk = [sum(ends), len(ends)]
            continue
        tgt = endb if all(ends) else best
        if all(ends): has_end = True
        else: normal.append([sorted(set(b)) for b in branches])
        names = {n for b in branches for n in b}
        for n in names:
            p = sum(n in b for b in branches) / len(branches)   # in k of n outcomes: k/n (outcomes equally likely)
            tgt[n] = max(tgt.get(n, 0), p)
    alts = normal if len({n for c in normal for b in c for n in b}) >= 2 else None
    return best, endb, has_end, risk, alts


def fmt(d):
    return sorted([n if p == 1 else [n, p] for n, p in d.items()], key=str)


def skills_of(ch, reward, skname):
    """Every skill a choice ([label, rewards]) can hint, on any of its outcomes."""
    out = set()
    for k in ch[1]:
        r = reward(k)
        if r and r[0] == 'sk' and r[2] in skname: out.add(skname[r[2]])
        if r and r[0] == 'sr': out.update(skname[x['d']] for x in r[2] if x['d'] in skname)
    return out


def pal_event(gdir, sid, cid):
    """The choices of pal or group event `sid` (a story id) as card `cid` sees them: its per-card variant when it has one."""
    import glob, json
    for f in ('training_events_friend.*.json', 'training_events_group.*.json'):
        for ent in json.load(open(glob.glob(gdir + f)[0])):
            for sec in ent[1:]:
                for e in sec or []:
                    if not e or str(e[2]) != str(sid): continue
                    var = e[3][1] if len(e) > 3 and len(e[3]) > 1 and isinstance(e[3][1], list) else []
                    return next((v[1][1] for v in var if v[0] == cid), e[1])
    return None


def prioritise(choices, k, reward, skname):
    """IT takes an event's first choice unless a prioritized skill sits on another (docs/MODEL.md, Deck side, "IT event
    choices"): the skills that steer it to choice k (1-based) are those on choice k and on no choice above it. [] for
    k = 1; None when there is none (IT cannot be steered there)."""
    chs = [ch for ch in choices if isinstance(ch, list) and len(ch) == 2 and isinstance(ch[1], list)]
    if k == 1: return []
    above = set().union(*[skills_of(ch, reward, skname) for ch in chs[:k - 1]])
    own = sorted(skills_of(chs[k - 1], reward, skname) - above)
    return own or None


MANUAL_KEYS = ('name', 'always', 'disabled', 'chance', 'note', 'event')


def manual_entry(cid, v, gdir, reward, skname):
    """A hand-entered (pal or group) card entry as the builders store it. `event: [story id, choice]` names the event and
    choice the entry models; `prioritise` is derived from it: the skills a player must prioritise for IT to take that choice."""
    e = {x: v[x] for x in v if x in MANUAL_KEYS}
    if 'event' in v:
        sid, k = v['event']
        chs = pal_event(gdir, sid, int(cid))
        if chs is None: raise SystemExit(f'card {cid}: event {sid} not in the GameTora files')
        p = prioritise(chs, k, reward, skname)
        if p is None: raise SystemExit(f'card {cid}: no skill steers IT to choice {k} of event {sid}')
        if p: e['prioritise'] = p
    return {**e, "status": "complete", "src": v.get('src', '')}


def apply_shares(out, path):
    """Measured outcome shares (event_shares.json) onto generated entries: each listed skill's `chain` share at that link
    becomes its measured share, and `alt_w` gives the solver each choice's outcome weights (equal where none is listed)."""
    import json
    for cid, lst in json.load(open(path))['cards'].items():
        e = out.get(cid)
        if not e: continue
        for s in lst:
            lk, k, sh = s['link'], s['choice'], s['shares']
            if abs(sum(sh.values()) - 1) > 1e-9: raise SystemExit(f'card {cid} link {lk}: shares do not add to 1')
            for l, names in e.get('chain', []):
                if l != lk: continue
                for i, n in enumerate(names):
                    nm = n[0] if isinstance(n, list) else n
                    if nm in sh: names[i] = nm if sh[nm] >= 1 else [nm, sh[nm]]
            alt = next((a for l, a in e.get('alts', []) if l == lk), None)
            if alt is None or k > len(alt): raise SystemExit(f'card {cid} link {lk}: no choice {k} in alts')
            w = [[1 / len(ch)] * len(ch) for ch in alt]
            for j, o in enumerate(alt[k - 1]):
                hits = [sh[n] for n in o if n in sh]
                if len(hits) != 1: raise SystemExit(f'card {cid} link {lk} outcome {j + 1}: needs exactly one listed skill, has {o}')
                w[k - 1][j] = hits[0]
            e.setdefault('alt_w', []).append([lk, w])
