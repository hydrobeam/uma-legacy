"""One GameTora training event -> the rewards the card-event builders store (both eras). The files are read and their
reward codes decoded by gametora.py (`choices`, `outcomes`, `hints`, `pal_event`); outcomes are taken as equally likely.
  - best:  skill -> chance over the choices that keep the chain going (k of n outcomes: k/n), the best choice per skill;
  - endb:  the same over the choices that end the chain on every outcome (a separate "end the chain here" option);
  - risk:  [k, n] when IT's default (first) choice ends the chain on k of its n outcomes (it is then not offered);
  - alts:  the full structure of the choices that keep the chain going, [choice][outcome] -> sorted skill names, when
           the event names two or more skills. `best` credits each skill on its own; the solver uses `alts` instead,
           so skills on different choices are never credited together and outcomes of one choice exclude each other."""


import gametora


def names_of(hs, skname):
    """the skill names of hints (`gametora.hint`), in order: a random hint gives every skill it can be"""
    out = []
    for kind, x, _ in hs:
        if kind == 'hint' and x in skname: out.append(skname[x])
        if kind == 'random': out.extend(skname[d] for d in x if d in skname)
    return out


def decode(event_choices, skname):
    best = {}; endb = {}; has_end = False; risk = None; normal = []; first = True
    for ch in gametora.choices(event_choices):
        top, first = first, False   # IT takes the first choice unless a prioritized skill is elsewhere
        outs = gametora.outcomes(ch[1])
        branches = [names_of(hs, skname) for hs, _ in outs]; ends = [e for _, e in outs]
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


def skills_of(ch, skname):
    """Every skill a choice ([label, rewards]) can hint, on any of its outcomes."""
    return set(names_of(gametora.hints(ch[1]), skname))


def prioritise(event_choices, k, skname):
    """IT takes an event's first choice unless a prioritized skill sits on another (docs/MODEL.md, Deck side, "IT event
    choices"): the skills that steer it to choice k (1-based) are those on choice k and on no choice above it. [] for
    k = 1; None when there is none (IT cannot be steered there)."""
    chs = gametora.choices(event_choices)
    if k == 1: return []
    above = set().union(*[skills_of(ch, skname) for ch in chs[:k - 1]])
    own = sorted(skills_of(chs[k - 1], skname) - above)
    return own or None


MANUAL_KEYS = ('name', 'always', 'disabled', 'chance', 'note', 'event')


def manual_entry(cid, v, skname):
    """A hand-entered (pal or group) card entry as the builders store it. `event: [story id, choice]` names the event and
    choice the entry models; `prioritise` is derived from it: the skills a player must prioritise for IT to take that choice."""
    e = {x: v[x] for x in v if x in MANUAL_KEYS}
    if 'event' in v:
        sid, k = v['event']
        chs = gametora.pal_event(sid, int(cid))
        if chs is None: raise SystemExit(f'card {cid}: event {sid} not in the GameTora files')
        p = prioritise(chs, k, skname)
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
