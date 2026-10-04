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
