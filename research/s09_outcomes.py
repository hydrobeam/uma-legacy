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
Intervals are 90% Wilson. Findings test each conclusion on them: the choice rule (prioritised against not), the measured
shares against the equal rule (k of n outcomes, from the game data) and the model's adopted shares (event_shares.json,
manual.json), Anshinzawa's choice 1 by scenario and her dates, the shares off or near the equal rule.
"""
import collections, json, os
import common

SEC = '9'
N_RECORDS = 2865
ANSHIN, FESTA, RIKO, HELIOS = 30080, 30108, 30036, 30116
HER_A = 'Sasami Anshinzawa event'
C1_LV = 3                                # her finale's Nothing Ventured at this level or above: choice 1
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


def equal(X, cid, event, choice, skill):
    """the equal rule's share for one outcome's marker skill (k of n outcomes, game data): the outcomes giving the skill,
    or those giving it at the marker's level when every outcome gives it (a split by level)"""
    x = next(x for x in X if x['cid'] == cid and x['event'].startswith(event) and x['choice'] == choice)
    gives = [o for o in x['outs'] if skill in o]
    if len(gives) == len(x['outs']):
        lv = next(m[1] for m in x['sigs'] if m and m[0] == skill); gives = [o for o in gives if o[skill] == lv]
    return len(gives) / len(x['outs'])


def adopted(cid, skill):
    """the model's share for one outcome: a measured split in event_shares.json, else the card's chance in manual.json"""
    for e in json.load(open(os.path.join(common.ROOT, 'pkg/loopopt/data/event_shares.json')))['cards'].get(str(cid), []):
        if skill in e['shares']: return e['shares'][skill]
    return dict(common.gjson('manual.json')['cards'][str(cid)]['chance'])[skill]


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
        out.append(dict(festa=FESTA in d, scen=r['scenario'], shown=pri is not None, nv_pri='Nothing Ventured' in (pri or []),
                        pri=['Risky Business'] if 'Risky Business' in (pri or []) else [],
                        NV=g.get('Nothing Ventured'), RB=g.get('Risky Business')))
    for o in out: o['c'] = classify(o)
    return out


def classify(o):
    her = lambda x: x and any('Anshinzawa' in s for s in x[1])
    nv, rb = o['NV'], o['RB']
    if nv: return 'c1_win' if nv[0] >= C1_LV else 'c2_win'
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
        nvp=collections.Counter(o['c'] for o in nvp), nei_c=collections.Counter(o['c'] for o in nei),
        c1_exc=(sum(o['c'] == 'c1_win' and bool(o['RB']) and o['RB'][0] >= 5 for o in rbp), c['c1_win']),
        c1_by=lambda gc: (sum(o['c'] == 'c1_win' for o in A if (o['scen'] == 3) == gc),
                          sum(o['c'] in ('c1_win', 'none', 'none_rb5') for o in A if (o['scen'] == 3) == gc)))


# ---------------------------------------------------------------- Riko's Lone Wolf, Helios's link 2, Festa's "Upping the Ante"
def riko_lone_wolf(R, prioritised=True):
    """runs showing a priority list with Lone Wolf on it (not Ramp Up), or without Lone Wolf: her bottom choice's Lone Wolf
    alone"""
    n = k = 0
    for r in R:
        p = r.get('priority')
        if RIKO not in deck_ids(r) or p is None or ('Lone Wolf' in p) != prioritised or prioritised and 'Ramp Up' in p: continue
        hs = hints(r); her = lambda s: s in hs and 'Riko Kashimoto event' in hs[s]['src']
        n += 1; k += her('Lone Wolf') and not her('Ramp Up') and not her('Maverick ○')
    return k, n


def riko_firings(R):
    """her "Unexpected Side": top choice Ramp Up +3 or Maverick ○ +5, bottom choice Lone Wolf +3 (its middle choice gives
    nothing and cannot be seen). Every credited outcome is one firing, counted where all three could show (each skill's
    level from other sources below 5). Returns (Ramp Up, Maverick, Lone Wolf firings, runs counted, runs with Ramp Up)"""
    sk = (('Ramp Up', 3), ('Maverick ○', 5), ('Lone Wolf', 3)); c = [0, 0, 0]; n = k = 0
    for r in R:
        if RIKO not in deck_ids(r) or r['chara'] == 9006: continue
        hs = hints(r); hit = [j for j, (s, _) in enumerate(sk) if s in hs and 'Riko Kashimoto event' in hs[s]['src']]
        if any(s in hs and hs[s]['lv'] - (lv if j in hit else 0) >= 5 for j, (s, lv) in enumerate(sk)): continue
        n += 1; k += 0 in hit
        for j in hit: c[j] += 1
    return (*c, n, k)


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


def festa_numbers():
    """Festa's share on every research run with her card.
    Her exclusive credits ("Nakayama Festa event") are wins; a Risky Business credit shared with another card's event
    ("A or B event") is hers with chance r_F / (1 - (1 - r_F)(1 - r_o)), r_F her exclusive Risky Business rate per run,
    r_o the other card's rate in runs without her; blank runs with Risky Business at Lv 5 are split by the firing rate
    where both outcomes can show. {} without the run records."""
    R = common.runs()
    if R is None: return {}
    wil = lambda k, n: common.wilson(k, n, Z90)
    src = lambda hs, k: hs[k]['src'] if k in hs else []
    t = collections.Counter(); shared = collections.Counter(); n_f = 0
    for r in R:
        d = deck_ids(r)
        if FESTA not in d or ANSHIN in d or r['chara'] == 1049: continue
        hs = hints(r); n_f += 1
        nv, rb = 'Nakayama Festa event' in src(hs, 'Nothing Ventured'), 'Nakayama Festa event' in src(hs, 'Risky Business')
        co = [y.strip() for x in src(hs, 'Risky Business') if x.endswith(' event') and 'Nakayama Festa' in x and x != 'Nakayama Festa event'
              for y in x[:-6].split(' or ') if y.strip() != 'Nakayama Festa']
        if nv and not rb: t['NV'] += 1
        elif rb and not nv: t['RB'] += 1
        elif nv and rb: t['both'] += 1
        elif co: t['shared'] += 1; shared[co[0]] += 1
        elif 'Risky Business' in hs and hs['Risky Business']['lv'] >= 5: t['hidden'] += 1
        else: t['none'] += 1
    r_f = (t['RB'] + t['both']) / n_f

    def alone(nm):                                         # the co-source's own Risky Business rate, runs without Festa
        k = n = 0
        for r in R:
            if FESTA in deck_ids(r) or not any(common.bare(x) == nm for _, _, x in r['deck']): continue
            hs = hints(r); n += 1; k += f'{nm} event' in src(hs, 'Risky Business')
        return k / n if n else 0.0
    rates = {nm: alone(nm) for nm in shared}
    hers = sum(c * r_f / (1 - (1 - r_f) * (1 - rates[nm])) for nm, c in shared.items())
    nv, rb, hid = t['NV'], t['RB'], t['hidden']
    fire = (nv + rb + hers) / (nv + rb + t['shared'] + t['none'])
    split = nv + rb + hers + hid * fire
    sh = json.load(open(os.path.join(common.ROOT, 'pkg/loopopt/data/event_shares.json')))['cards'].get(str(FESTA), [{}])[0]
    out = {'Festa: runs with the card': sum(FESTA in deck_ids(r) for r in R), 'Festa: NV wins': nv, 'Festa: RB wins': rb,
           'Festa: RB shared credits': t['shared'], 'Festa: shared, with Team Sirius': shared.get('Team Sirius', 0),
           'Festa: Team Sirius alone RB rate': rates.get('Team Sirius', float('nan')),
           'Festa: Anshinzawa alone RB rate': rates.get('Sasami Anshinzawa', float('nan')), 'Festa: her RB rate': r_f,
           'Festa: shared credits hers': hers, 'Festa: blank, Risky Business at Lv 5': hid,
           'Festa: firing rate where both can show': fire, 'Festa: readings': split, 'Festa: share': nv / split,
           'Festa: share, shared none hers': nv / (nv + rb + hid * (nv + rb) / (nv + rb + t['shared'] + t['none'])),
           'Festa: share, shared all hers': nv / (nv + rb + t['shared'] + hid * (nv + rb + t['shared']) / (nv + rb + t['shared'] + t['none'])),
           'Festa: adopted': sh.get('shares', {}).get('Nothing Ventured', float('nan'))}
    out['Festa: share, 90% low'], out['Festa: share, 90% high'] = wil(nv, round(split))
    return out


# ---------------------------------------------------------------- values
def claims():
    R = common.runs()
    if R is None: return [finale_rule(split_choices([]))]
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
    for lab, cid, ev, ch, skill, kn in (('Riko', RIKO, 'Unexpected Side', 1, 'Ramp Up', 'Riko: Ramp Up (top choice taken)'),
                                        ('Seiun Sky', 30008, "It's All to Win", 1, 'Fast-Paced', 'Seiun Sky link 3: Fast-Paced'),
                                        ('Smart Falcon', 30017, 'My Umadol Way!', 1, 'Center Stage', 'Smart Falcon link 3: Center Stage')):
        k, n = find(X, cid, ev, ch, skill); lo, hi = wil(k, n) if n else (None, None)
        nn = 'Riko: readings' if lab == 'Riko' else f'{lab} link 3: readings'
        for key, v in zip((kn, nn, f'{lab}: share', f'{lab}: 90% low', f'{lab}: 90% high'), (k, n, k / n if n else None, lo, hi)):
            got[key] = v
    k, n = riko_lone_wolf(S); got['Riko: Lone Wolf prioritised, bottom choice'] = k; got['Riko: Lone Wolf prioritised runs'] = n
    ru, mv, lw, nr, rr = riko_firings(S); lo, hi = wil(rr, nr)
    got.update({'Riko per run: runs with Ramp Up': rr, 'Riko per run: runs': nr, 'Riko per run: share': rr / nr, 'Riko per run: 90% low': lo,
                'Riko per run: 90% high': hi, 'Riko firings: top choice': ru + mv, 'Riko firings: all': ru + mv + lw,
                'Riko firings: top-choice rate': (ru + mv) / (ru + mv + lw), 'Riko firings: bottom choice': lw})
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
    fe = festa_numbers()
    if fe: got.update(fe)
    got['Anshinzawa: choice-1 exceptions with RB at Lv 5'], got['Anshinzawa: choice-1 exceptions, RB prioritised'] = a['c1_exc']
    got['runs showing a priority list'] = sum(r.get('priority') is not None for r in R); got['runs'] = len(R)
    note = {'Anshinzawa: neither prioritised, choice 2 won': 'Risky Business alone not prioritised: %d/%d' % a['rb_not']}
    out = [common.Value(SEC, n, v, note=note.get(n, '')) for n, v in got.items()]
    return out + findings(R, S, a, X, got)


def finale_rule(X):
    """game data: the finale's hint levels identify the choice as classify() reads them"""
    fin = [x['outs'] for x in X if x['cid'] == ANSHIN and x['link'] == 3]
    nv = lambda ch: {o['Nothing Ventured'] for o in ch if 'Nothing Ventured' in o}
    rb = lambda ch: any('Risky Business' in o for o in ch)
    return common.finding(SEC, 'Game data: the hint levels identify the finale\'s choice (choice 1 Nothing Ventured at the level '
                          'read as choice 1 or nothing, choice 2 Nothing Ventured below it or Risky Business)',
                          lambda: len(fin) == 2 and min(nv(fin[0])) >= C1_LV > max(nv(fin[1])) and any(not o for o in fin[0])
                          and not rb(fin[0]) and rb(fin[1]), f'outcomes by choice {fin}; level read as choice 1: {C1_LV} or above')


def findings(R, S, a, X, got):
    """the conclusions of section 9, each on its 90% Wilson intervals (the section's)"""
    F = lambda st, test, ev: common.finding(SEC, st, test, ev)
    wil = lambda k, n: common.wilson(k, n, Z90)
    iv = lambda k, n: f'{k}/{n} = {k / n:.2f} [{wil(k, n)[0]:.2f}, {wil(k, n)[1]:.2f}]'
    out = []
    L = collections.Counter(len(r['priority']) for r in R if r.get('priority') is not None)
    out.append(F('A priority list has a cap that lists fill: none is longer than the commonest length', max(L) == L.most_common(1)[0][0],
                 f'cap {max(L)}: ' + ', '.join(f'{n} skills: {L[n]}' for n in sorted(L, reverse=True))))
    out.append(finale_rule(X))
    # the choice rule: a prioritised skill moves IT off the top choice, and nothing else does
    k2, n2 = a['shows'] - a['c1_exc'][1], a['shows']; k0, n0 = a['nei_c2'], a['nei']
    out.append(F('Choice rule, Anshinzawa\'s finale: with Risky Business prioritised IT takes choice 2, with neither skill '
                 'prioritised it does not (the two 90% intervals are disjoint)', wil(k2, n2)[0] > wil(k0, n0)[1],
                 f'prioritised {iv(k2, n2)}; neither {iv(k0, n0)}'))
    ke, ne = a['c1_exc']
    out.append(F('Most of the prioritised runs that took choice 1 anyway had Risky Business at hint Lv 5 already '
                 '(more than half)', ke > ne / 2, f'{ke} of {ne} choice-1 runs with Risky Business prioritised'))
    k1, n1 = riko_lone_wolf(S); kn, nn = riko_lone_wolf(S, prioritised=False)
    out.append(F('Choice rule, Riko Kashimoto: with Lone Wolf prioritised IT takes the bottom choice, without it never '
                 '(disjoint 90% intervals)', wil(k1, n1)[0] > wil(kn, nn)[1],
                 f'Lone Wolf prioritised {iv(k1, n1)}; priority list without it {iv(kn, nn)}'))
    # measured shares: why they leave the equal rule, and whether the adopted share sits in the measured interval
    # (each: k, n, the choice (card, event, choice, marker skill))
    rows = {'Anshinzawa finale, Nothing Ventured': (got['Anshinzawa: NV with RB prioritised'], got['Anshinzawa: RB prioritised runs'],
                                                    (ANSHIN, 'The Final Race', 2, 'Nothing Ventured')),
            'Riko "Unexpected Side", Ramp Up': (got['Riko: Ramp Up (top choice taken)'], got['Riko: readings'],
                                                (RIKO, 'Unexpected Side', 1, 'Ramp Up')),
            'Seiun Sky link 3, Fast-Paced': (got['Seiun Sky link 3: Fast-Paced'], got['Seiun Sky link 3: readings'],
                                             (30008, "It's All to Win", 1, 'Fast-Paced')),
            'Smart Falcon link 3, Center Stage': (got['Smart Falcon link 3: Center Stage'], got['Smart Falcon link 3: readings'],
                                                  (30017, 'My Umadol Way!', 1, 'Center Stage'))}
    out.append(F('The four measured shares depart from the equal rule: every 90% interval lies above its equal share '
                 '(k of n outcomes)', all(wil(k, n)[0] > equal(X, *c) for k, n, c in rows.values()),
                 '; '.join(f'{t} {iv(k, n)} (equal {equal(X, *c):g})' for t, (k, n, c) in rows.items())))
    c2k, c2n = a['c2_clean']
    for t, (k, n, c) in rows.items():
        if t.startswith('Riko'): continue
        ex = f'; choice 2 alone {iv(c2k, c2n)}' if t.startswith('Anshinzawa') else ''
        v = adopted(c[0], c[3]); out.append(common.Value(SEC, f'adopted: {t}', v))
        out.append(F(f'Adopted {v:g} for {t} lies inside its measured 90% interval', wil(k, n)[0] <= v <= wil(k, n)[1], iv(k, n) + ex))
    ru, mv, lw, nr, rr = riko_firings(S); fire = ru + mv + lw; v = adopted(RIKO, 'Ramp Up')
    out.append(common.Value(SEC, 'adopted: Riko "Unexpected Side", Ramp Up', v))
    out.append(F(f'Riko\'s adopted {v:g} is the chance a run gets her Ramp Up hint (the model\'s per-run chance), whichever choice '
                 f'IT took: {v:g} inside the per-run 90% interval',
                 wil(rr, nr)[0] <= v <= wil(rr, nr)[1],
                 f'per run {rr}/{nr} = {rr / nr:.3f} [{wil(rr, nr)[0]:.3f}, {wil(rr, nr)[1]:.3f}]; per firing {ru}/{fire} = {ru / fire:.3f} '
                 f'(Maverick {mv}, bottom-choice Lone Wolf {lw}): top-choice share {iv(ru, ru + mv)} x top-choice rate {iv(ru + mv, fire)}'))
    # Anshinzawa's choice 1 by scenario; her date-start event and the finale
    g, o = a['c1_by'](True), a['c1_by'](False)
    out.append(F('Anshinzawa\'s choice 1 (Nothing Ventured +3 or nothing) is lower in Grand Concert than elsewhere '
                 '(disjoint 90% intervals)', wil(*g)[1] < wil(*o)[0], f'Grand Concert {iv(*g)}; other scenarios {iv(*o)}'))
    kp, np_ = a['rb_c2wins']; kq, nq = a['rb_c2wins_nei']
    out.append(F('With Risky Business prioritised IT also takes the bottom choice of "A Pleasant Practice Partner?": her '
                 'extra Risky Business +1 in choice-2 wins with the priority, against without it (disjoint 90% intervals)',
                 wil(kp, np_)[0] > wil(kq, nq)[1], f'with {iv(kp, np_)}; without {iv(kq, nq)}'))
    ks, ns = a['shows'], a['rbp']
    out.append(F('IT keeps her dates either way: with Risky Business prioritised the finale shows its outcome in nearly '
                 'every run (90% lower bound above 0.9, so at most 10% of runs could have lost it)', wil(ks, ns)[0] > 0.9, iv(ks, ns)))
    # off the equal rule but left at it; consistent with it
    off = {'Tazuna Hayakawa finale, Concentration': (30021, 'A Surprise Breather', 1, 'Concentration'),
           'Agnes Tachyon link 1 choice 2, Murmur': (30101, 'Researching the Limits', 2, 'Murmur'),
           'Marvelous Sunday link 3, Position Pilfer': (30074, 'Pure White Marvelousness', 1, 'Position Pilfer'),
           'Light Hello finale, See Ya Later! +3': (30052, 'At Rainbow Cove', 1, 'See Ya Later!')}
    on = {'Gold Ship link 3, Innate Experience': (30057, 'Assembling the Uma Lord', 1, 'Innate Experience'),
          'King Halo link 3, Gap Closer': (30056, 'Glittering Star of the Ball', 1, 'Gap Closer'),
          'Daitaku Helios link 2, Tail Held High (given link 3)': (HELIOS, "Helios's Hype Heaven Week", 1, 'Tail Held High')}
    kv = {t: helios(S) if x[0] == HELIOS else find(X, *x) for t, x in {**off, **on}.items()}   # Helios: given link 3 (helios())
    eq = {t: equal(X, *x) for t, x in {**off, **on}.items()}
    out.append(F('Off the equal rule (left at it, a known miss): each 90% interval excludes the equal share (k of n outcomes)',
                 all(not wil(*kv[t])[0] <= eq[t] <= wil(*kv[t])[1] for t in off),
                 '; '.join(f'{t} {iv(*kv[t])} (equal {eq[t]:g})' for t in off)))
    out.append(F('Consistent with the equal rule: each 90% interval contains the equal share (k of n outcomes)',
                 all(wil(*kv[t])[0] <= eq[t] <= wil(*kv[t])[1] for t in on), '; '.join(f'{t} {iv(*kv[t])} (equal {eq[t]:g})' for t in on)))
    fe = festa_numbers()
    if fe:
        e, v = equal(X, FESTA, 'Upping the Ante', 1, 'Nothing Ventured'), fe['Festa: adopted']
        out.append(F(f'Nakayama Festa\'s Nothing Ventured share is above {e:g} and the adopted {v:g} fits: with shared credits attributed '
                     f'and blank runs split by the firing rate, {e:g} (the equal rule) lies below the 90% interval and {v:g} inside it',
                     fe['Festa: share, 90% low'] > e and fe['Festa: share, 90% low'] <= fe['Festa: adopted'] <= fe['Festa: share, 90% high'],
                     f"{fe['Festa: share']:.3f} [{fe['Festa: share, 90% low']:.3f}, {fe['Festa: share, 90% high']:.3f}]; shared credits hers "
                     f"{fe['Festa: shared credits hers']:.1f} of {fe['Festa: RB shared credits']}; bounds {fe['Festa: share, shared all hers']:.3f}-"
                     f"{fe['Festa: share, shared none hers']:.3f}"))
    return out


def detail():
    S = [r for r in common.runs() if r['seq'] < N_RECORDS]; a = anshin(S)
    print('Anshinzawa finale by priority (no Festa): neither shown-prioritised', a['nei'], dict(a['nei_c']),
          '| Nothing Ventured prioritised (not Risky Business)', sum(a['nvp'].values()), dict(a['nvp']))


if __name__ == '__main__':
    common.main(claims)
    if common.have_runs(): print(); detail()
