"""Section 8: event choices and outcome shares, counted on the run records (research/data/runs.jsonl, not included;
Independent Training runs from uma-it).

The shares were measured on the first 2,865 records (seq < 2865); the priority-list count uses all of them.

- Outcome shares (Riko Kashimoto, Seiun Sky, Smart Falcon, Tazuna Hayakawa, Agnes Tachyon, Marvelous Sunday, Gold Ship,
  King Halo, Light Hello): every SR/SSR card event choice with two or more outcomes (GameTora, through gametora.py).
  An outcome's marker is a (skill, level) only that outcome of the choice gives and no other event of the character
  (her card's events, her shared events) gives at all. Runs: the card in the deck, someone else training. Outcomes that
  differ only by level count when the hint is credited solely to "<her name> event" at exactly that level; outcomes
  with different skills count when her event is credited and every outcome could show (each marker skill's level from
  other sources below 5).
- Sasami Anshinzawa [This Might Sting!] finale: her Nothing Ventured +3 = choice 1 won, +1 = choice 2 won; her Risky
  Business without Nothing Ventured = choice 2 lost; runs with Nakayama Festa [Trusting the Dice] left out (joint
  credits). Daitaku Helios and Nakayama Festa: credits to her event on the skills of one link.
Intervals are 90% Wilson.
"""
import collections
import common

SEC = '8'
N_RECORDS = 2865
ANSHIN, FESTA, RIKO, HELIOS = 30080, 30108, 30036, 30116
HER_A = 'Sasami Anshinzawa event'
Z90 = 1.645


def deck_ids(r): return [d[0] for d in r['deck']]


def hints(r): return {h['name']: h for h in r['hints']}


# ---------------------------------------------------------------- outcome shares (every split choice)
def split_choices(R):
    """[dict(card, event, link, choice, outs, sigs, cnt, cntr, nrun)] for every SR/SSR split choice"""
    gt = common.gametora(); db = common.mdb()
    sk = dict(db.execute('select [index], text from text_data where category = 47'))
    title = dict(db.execute('select [index], text from text_data where category = 181'))
    chn = dict(db.execute('select [index], text from text_data where category = 6'))
    cname = dict(db.execute('select [index], text from text_data where category = 75'))
    cchar = dict(db.execute('select id, chara_id from support_card_data'))
    crar = dict(db.execute('select id, rarity from support_card_data'))
    link = dict(db.execute('select story_id, show_progress_1 from single_mode_story_data where show_progress_1 > 0'))

    def outcomes(rewards):   # one choice -> [ {skill name: level or '?'} ] per outcome
        out = []
        for hs, _ in gt.outcomes(rewards):
            o = {}
            for kind, x, lv in hs:
                if kind == 'hint' and x in sk:
                    s = str(lv).lstrip('+') if lv else '1'
                    o[sk[x]] = int(s) if s.isdigit() else '?'
                elif kind == 'random':
                    for i in x:
                        if i in sk: o[sk[i]] = '?'
            out.append(o)
        return out

    evs = collections.defaultdict(list)   # card -> [(title, story id, [choice rewards])]
    for key in ('training_events_sr', 'training_events_ssr'):
        for cid, es in gt.load(key):
            for e in es: evs[cid].append((title.get(int(e[2]), str(e[2])), int(e[2]), [ch[1] for ch in gt.choices(e[1])]))
    by_chara = collections.defaultdict(list)
    for cid, ch in cchar.items(): by_chara[ch].append(cid)
    for row in gt.load('training_events_friend'):
        base = {}; var = collections.defaultdict(dict)
        for e, only, chs in gt.events(row, 'friend'):
            if only is None: base[id(e)] = e
            else: var[id(e)][only] = chs
        for k, e in base.items():
            for cid in by_chara[row[0]]:
                chs = var[k].get(cid, e[1])
                evs[cid].append((title.get(int(e[2]), str(e[2])), int(e[2]), [ch[1] for ch in gt.choices(chs)]))
    shared = collections.defaultdict(list)
    for chara, es in gt.load('training_events_shared'):
        for e in es: shared[chara].append([ch[1] for ch in gt.choices(e[1])])

    runs = [(r, set(deck_ids(r)), hints(r)) for r in R]
    out = []
    for cid, es in evs.items():
        if crar.get(cid, 0) < 2: continue                       # SR / SSR only
        chara = cchar.get(cid); nm = chn.get(chara)
        if not nm: continue
        giver = set()
        for _, sid, chs in es:
            for ch in chs:
                for o in outcomes(ch): giver |= {(sid, s) for s in o}
        for chs in shared.get(chara, []):
            for ch in chs:
                for o in outcomes(ch): giver |= {('sh', s) for s in o}
        nsrc = collections.Counter(s for _, s in giver)
        mine = [(r, hs) for r, d, hs in runs if cid in d and r['chara'] != chara]
        for t, sid, chs in es:
            for ci, ch in enumerate(chs):
                outs = outcomes(ch)
                if len(outs) < 2 or len({tuple(sorted(o.items(), key=str)) for o in outs}) < 2: continue
                sigs = []
                for o in outs:
                    m = [(k, v) for k, v in o.items() if v != '?' and nsrc[k] == 1 and sum(o2.get(k) == v for o2 in outs) == 1]
                    sigs.append(m[0] if m else None)
                if not any(sigs): continue
                cnt = collections.Counter(); cntr = collections.Counter()
                ev = f'{nm} event'
                distinct = all(sigs) and len({m[0] for m in sigs}) == len(sigs)
                for r, hs in mine:
                    for j, m in enumerate(sigs):
                        h = m and hs.get(m[0])
                        if not h: continue
                        bylevel = sum(m[0] in o for o in outs) > 1
                        if (h['src'] == [ev] and h['lv'] == m[1]) if bylevel else ev in h['src']: cnt[j] += 1
                    hit = [j for j, m in enumerate(sigs) if m and hs.get(m[0]) and ev in hs[m[0]]['src']]

                    def room(j):   # outcome j would show: the skill's level from other sources is below 5
                        h = hs.get(sigs[j][0])
                        if not h: return True
                        return h['lv'] - (sigs[j][1] if j in hit and isinstance(sigs[j][1], int) else 0) < 5
                    if distinct and all(room(j) for j in range(len(sigs))):
                        for j in hit: cntr[j] += 1
                out.append(dict(card=cname.get(cid, str(cid)), cid=cid, event=t, link=link.get(sid), choice=ci + 1,
                                outs=outs, sigs=sigs, cnt=[cnt[j] for j in range(len(outs))],
                                cntr=[cntr[j] for j in range(len(outs))], nrun=len(mine)))
    return out


def find(X, cid, event, choice, skill):
    """(count, total) for one outcome's marker skill: cntr where every outcome could show, cnt for level splits"""
    for x in X:
        if x['cid'] == cid and x['event'].startswith(event) and x['choice'] == choice:
            for j, m in enumerate(x['sigs']):
                if m and m[0] == skill:
                    c = x['cntr'] if all(x['sigs']) and len({s[0] for s in x['sigs']}) == len(x['sigs']) else x['cnt']
                    return c[j], sum(c)
    return None, None


# ---------------------------------------------------------------- Sasami Anshinzawa's finale
def anshin_runs(R):
    out = []
    for r in R:
        d = deck_ids(r)
        if ANSHIN not in d: continue
        hs = hints(r); pri = r.get('priority')
        g = {s: (hs[s]['lv'], hs[s]['src']) for s in ('Nothing Ventured', 'Risky Business') if s in hs}
        out.append(dict(festa=FESTA in d, shown=pri is not None, nv_pri='Nothing Ventured' in (pri or []),
                        pri=['Risky Business'] if 'Risky Business' in (pri or []) else [],
                        NV=g.get('Nothing Ventured'), RB=g.get('Risky Business')))
    for o in out: o['c'] = classify(o)
    return out


def classify(o):
    her = lambda x: x and any('Anshinzawa' in s for s in x[1])
    nv, rb = o['NV'], o['RB']
    if nv: return 'c1_win' if nv[0] >= 3 else 'c2_win'
    if her(rb):
        if any(' or ' in s for s in rb[1]): return 'c2_lose?'
        if o['pri'] and HER_A not in rb[1]: return 'c2_lose?'
        return 'c2_lose'
    if rb and rb[0] >= 5: return 'none_rb5'
    return 'none'


def anshin(R):
    A = [o for o in anshin_runs(R) if not o['festa']]
    rbp = [o for o in A if 'Risky Business' in o['pri']]
    nei = [o for o in A if o['shown'] and not o['pri'] and not o['nv_pri']]
    nvp = [o for o in A if o['nv_pri'] and not o['pri']]
    c = collections.Counter(o['c'] for o in rbp)
    c2 = c['c2_win'] + c['c2_lose'] + c['c2_lose?']
    inh = lambda o: bool(o['RB'] and 'Inspiration' in o['RB'][1])
    her_rb = lambda o: bool(o['RB'] and any('Anshinzawa' in s for s in o['RB'][1]))
    w_clean = sum(o['c'] == 'c2_win' and not inh(o) for o in rbp)
    l_clean = sum(o['c'] == 'c2_lose' and o['RB'] == (2, [HER_A]) for o in rbp)
    l_mixed = sum(o['c'] == 'c2_lose' and inh(o) for o in rbp)
    ca = collections.Counter(o['c'] for o in A)
    return dict(
        n=len(A), rbp=len(rbp), choice2_share=c2 / (c2 + c['c1_win']),
        nv_rbp=c['c1_win'] + c['c2_win'],
        c2_clean=(w_clean, w_clean + l_clean),
        c2_mixed=(c['c2_win'] / (c['c2_win'] + l_clean + l_mixed), c['c2_win'] / (c['c2_win'] + l_clean)),
        nei=len(nei), nei_c2=sum(o['c'] == 'c2_win' for o in nei),
        c1=(ca['c1_win'], ca['c1_win'] + ca['none'] + ca['none_rb5']),
        shows=c['c1_win'] + c2,
        rb_c2wins=(sum(her_rb(o) for o in rbp if o['c'] == 'c2_win'), c['c2_win']),
        rb_c2wins_nei=(sum(her_rb(o) for o in nei + nvp if o['c'] == 'c2_win'), sum(o['c'] == 'c2_win' for o in nei + nvp)),
        rb_not=(sum(o['c'] == 'c2_win' for o in nei + nvp), len(nei + nvp)),
        nvp=collections.Counter(o['c'] for o in nvp), nei_c=collections.Counter(o['c'] for o in nei))


# ---------------------------------------------------------------- Riko's Lone Wolf, Helios's link 2, Festa's "Upping the Ante"
def riko_lone_wolf(R):
    n = k = 0
    for r in R:
        p = r.get('priority')
        if RIKO not in deck_ids(r) or p is None or 'Lone Wolf' not in p or 'Ramp Up' in p: continue
        hs = hints(r); her = lambda s: s in hs and 'Riko Kashimoto event' in hs[s]['src']
        n += 1; k += her('Lone Wolf') and not her('Ramp Up') and not her('Maverick ○')
    return k, n


def helios(R):
    """Tail Held High (link 2) among runs where link 3 visibly fired (her Perfect Spot!)"""
    k = n = 0
    for r in R:
        if HELIOS not in deck_ids(r) or r['chara'] == 1071: continue
        hs = hints(r); her = lambda s: s in hs and 'Daitaku Helios event' in hs[s]['src']
        if her('Perfect Spot!'): n += 1; k += her('Tail Held High')
    return k, n


def festa(R):
    """'Upping the Ante': her Nothing Ventured against her Risky Business; blank runs with Risky Business at 5 may hide one"""
    t = collections.Counter()
    for r in R:
        d = deck_ids(r)
        if FESTA not in d or ANSHIN in d or r['chara'] == 1049: continue
        hs = hints(r); her = lambda s: s in hs and 'Nakayama Festa event' in hs[s]['src']
        nv, rb = her('Nothing Ventured'), her('Risky Business')
        if nv and not rb: t['NV'] += 1
        elif rb and not nv: t['RB'] += 1
        elif not nv and not rb: t['hidden' if 'Risky Business' in hs and hs['Risky Business']['lv'] >= 5 else 'none'] += 1
    return t


# ---------------------------------------------------------------- claims
ITEMS = [
    ('Anshinzawa: RB prioritised, took choice 2', '95%'),
    ('Anshinzawa: neither prioritised, choice 2 won', '0'), ('Anshinzawa: neither prioritised, runs', '35'),
    ('Anshinzawa: runs with the card (no Festa)', '254'),
    ('Anshinzawa: NV with RB prioritised', '58'), ('Anshinzawa: RB prioritised runs', '77'),
    ('Anshinzawa: NV share', '0.75'), ('Anshinzawa: NV share 90% low', '0.66'), ('Anshinzawa: NV share 90% high', '0.82'),
    ('Anshinzawa: choice 2, no inherited RB, won', '37'), ('Anshinzawa: choice 2, no inherited RB, runs', '45'),
    ('Anshinzawa: choice 2 share', '0.82'), ('Anshinzawa: choice 2 90% low', '0.71'), ('Anshinzawa: choice 2 90% high', '0.90'),
    ('Anshinzawa: choice 2 with inherited, low', '0.77'), ('Anshinzawa: choice 2 with inherited, high', '0.87'),
    ('Anshinzawa: choice 1 won', '69'), ('Anshinzawa: choice 1 runs', '127'), ('Anshinzawa: choice 1 share', '0.54'),
    ('Anshinzawa: RB prioritised runs showing the finale', '75'),
    ('Anshinzawa: her extra RB in choice-2 wins, prioritised', '53'), ('Anshinzawa: choice-2 wins, prioritised', '54'),
    ('Anshinzawa: her extra RB in choice-2 wins, RB not prioritised', '1'), ('Anshinzawa: choice-2 wins, RB not prioritised', '20'),
    ('Riko: Ramp Up (top choice taken)', '527'), ('Riko: readings', '585'), ('Riko: share', '0.90'),
    ('Riko: 90% low', '0.88'), ('Riko: 90% high', '0.92'),
    ('Riko: Lone Wolf prioritised, bottom choice', '55'), ('Riko: Lone Wolf prioritised runs', '62'),
    ('Seiun Sky link 3: Fast-Paced', '33'), ('Seiun Sky link 3: readings', '40'), ('Seiun Sky: share', '0.82'),
    ('Seiun Sky: 90% low', '0.71'), ('Seiun Sky: 90% high', '0.90'),
    ('Smart Falcon link 3: Center Stage', '37'), ('Smart Falcon link 3: readings', '51'), ('Smart Falcon: share', '0.73'),
    ('Smart Falcon: 90% low', '0.61'), ('Smart Falcon: 90% high', '0.81'),
    ('Tazuna finale: Concentration', '0.70'), ('Tachyon link 1 choice 2: Murmur', '0.70'),
    ('Marvelous Sunday link 3: Position Pilfer', '0.58'),
    ('Light Hello finale: See Ya Later! +3', '1,497'), ('Light Hello finale: readings', '2,103'),
    ('Light Hello finale: share', '0.71'),
    ('Gold Ship link 3: Innate Experience', '0.61'), ('King Halo link 3: Gap Closer', '0.57'),
    ('Helios link 2 Tail Held High (given link 3)', '0.47'),
    ('Festa: NV share, low', '0.48'), ('Festa: NV share, high', '0.65'),
    ('Festa: readings, low', '37'), ('Festa: readings, high', '50'),
    ('runs showing a priority list', '358'), ('runs', '3,179'),
]


def claims():
    R = common.runs()
    if R is None: return common.need_data(SEC, ITEMS)
    S = [r for r in R if r['seq'] < N_RECORDS]
    a = anshin(S); X = split_choices(S)
    wil = lambda k, n: common.wilson(k, n, Z90)
    got = {}
    got['Anshinzawa: RB prioritised, took choice 2'] = a['choice2_share']
    got['Anshinzawa: neither prioritised, choice 2 won'] = a['nei_c2']; got['Anshinzawa: neither prioritised, runs'] = a['nei']
    got['Anshinzawa: runs with the card (no Festa)'] = a['n']
    k, n = a['nv_rbp'], a['rbp']; lo, hi = wil(k, n)
    got.update({'Anshinzawa: NV with RB prioritised': k, 'Anshinzawa: RB prioritised runs': n, 'Anshinzawa: NV share': k / n,
                'Anshinzawa: NV share 90% low': lo, 'Anshinzawa: NV share 90% high': hi})
    k, n = a['c2_clean']; lo, hi = wil(k, n)
    got.update({'Anshinzawa: choice 2, no inherited RB, won': k, 'Anshinzawa: choice 2, no inherited RB, runs': n,
                'Anshinzawa: choice 2 share': k / n, 'Anshinzawa: choice 2 90% low': lo, 'Anshinzawa: choice 2 90% high': hi})
    got['Anshinzawa: choice 2 with inherited, low'], got['Anshinzawa: choice 2 with inherited, high'] = a['c2_mixed']
    k, n = a['c1']
    got.update({'Anshinzawa: choice 1 won': k, 'Anshinzawa: choice 1 runs': n, 'Anshinzawa: choice 1 share': k / n})
    got['Anshinzawa: RB prioritised runs showing the finale'] = a['shows']
    got['Anshinzawa: her extra RB in choice-2 wins, prioritised'], got['Anshinzawa: choice-2 wins, prioritised'] = a['rb_c2wins']
    got['Anshinzawa: her extra RB in choice-2 wins, RB not prioritised'], got['Anshinzawa: choice-2 wins, RB not prioritised'] = a['rb_c2wins_nei']
    for lab, cid, ev, ch, skill in (('Riko', RIKO, 'Unexpected Side', 1, 'Ramp Up'), ('Seiun Sky', 30008, "It's All to Win", 1, 'Fast-Paced'),
                                    ('Smart Falcon', 30017, 'My Umadol Way!', 1, 'Center Stage')):
        k, n = find(X, cid, ev, ch, skill); lo, hi = wil(k, n) if n else (None, None)
        keys = [x for x, _ in ITEMS if x.startswith(lab)]
        for key, v in zip(keys, (k, n, k / n if n else None, lo, hi)): got[key] = v
    k, n = riko_lone_wolf(S); got['Riko: Lone Wolf prioritised, bottom choice'] = k; got['Riko: Lone Wolf prioritised runs'] = n
    for key, (cid, ev, ch, skill) in {'Tazuna finale: Concentration': (30021, 'A Surprise Breather', 1, 'Concentration'),
                                      'Tachyon link 1 choice 2: Murmur': (30101, 'Researching the Limits', 2, 'Murmur'),
                                      'Marvelous Sunday link 3: Position Pilfer': (30074, 'Pure White Marvelousness', 1, 'Position Pilfer'),
                                      'Gold Ship link 3: Innate Experience': (30057, "Assembling the Uma Lord", 1, 'Innate Experience'),
                                      'King Halo link 3: Gap Closer': (30056, 'Glittering Star of the Ball', 1, 'Gap Closer')}.items():
        k, n = find(X, cid, ev, ch, skill); got[key] = k / n if n else None
    k, n = find(X, 30052, 'At Rainbow Cove', 1, 'See Ya Later!')
    got.update({'Light Hello finale: See Ya Later! +3': k, 'Light Hello finale: readings': n,
                'Light Hello finale: share': k / n if n else None})
    k, n = helios(S); got['Helios link 2 Tail Held High (given link 3)'] = k / n
    t = festa(S); m = t['NV'] + t['RB']
    got.update({'Festa: NV share, low': t['NV'] / (m + t['hidden']), 'Festa: NV share, high': t['NV'] / m,
                'Festa: readings, low': m, 'Festa: readings, high': m + t['hidden']})
    got['runs showing a priority list'] = sum(r.get('priority') is not None for r in R); got['runs'] = len(R)
    note = {'Anshinzawa: neither prioritised, choice 2 won': 'Risky Business alone not prioritised: %d/%d' % a['rb_not']}
    return [common.Claim(SEC, n, s, got.get(n), note=note.get(n, '')) for n, s in ITEMS]


def detail():
    S = [r for r in common.runs() if r['seq'] < N_RECORDS]; a = anshin(S)
    print('Anshinzawa finale by priority (no Festa): neither shown-prioritised', a['nei'], dict(a['nei_c']),
          '| Nothing Ventured prioritised (not Risky Business)', sum(a['nvp'].values()), dict(a['nvp']))


if __name__ == '__main__':
    common.main(claims)
    if common.have_runs(): print(); detail()
