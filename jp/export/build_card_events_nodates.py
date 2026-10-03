"""Build ../card_events.json from the cached GameTora training-event files (pkg/loopopt/gametora/) + master.mdb.
Chain links come from master.mdb (single_mode_story_data.show_progress_1 for the event's story id).
Reward codes index dict/evrew with an offset of 36 (verified: every card's bond reward decodes to its own
character, and King Halo / Yaeno / Gold Ship / Light Hello / Anshinzawa match the wiki).
A skill in every outcome of some choice is certain when the link fires; a skill in k of a choice's n outcomes
(split by 'di') is stored with chance k/n, outcomes taken as equally likely. Pal entries and bug notes are kept by hand."""
import json, sqlite3, glob, os
from jpnames import names, CUT
G = '../../pkg/loopopt/gametora/'   # the same GameTora files as Global
OFF = 36
ev = json.load(open(glob.glob(G + 'dict_evrew.*.json')[0]))
c = sqlite3.connect('master.mdb'); q = lambda s, *a: c.execute(s, a).fetchall()
skname = names(47)
cname = names(75)
link = {sid: p for sid, p in q("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0")}
glob_cards = {i for (i,) in q("select id from support_card_data where start_date<?", CUT)}
def reward(k):
    i = k - OFF
    return ev[i] if 0 <= i < len(ev) else None
out = {}
for f in ['training_events_sr.*.json', 'training_events_ssr.*.json']:
    for cid, evs in json.load(open(glob.glob(G + f)[0])):
        if cid not in glob_cards: continue
        chain = []; ends_out = []
        for e in evs:
            lk = link.get(int(e[2]))
            if not lk: continue
            best = {}; endb = {}; has_end = False
            for ch in e[1]:
                if not (isinstance(ch, list) and len(ch) == 2 and isinstance(ch[1], list)): continue
                branches = [[]]; ends = [False]
                for k in ch[1]:
                    r = reward(k)
                    if r and r[0] == 'di': branches.append([]); ends.append(False); continue
                    if r and r[0] == 'ee': ends[-1] = True
                    if r and r[0] == 'sk' and r[2] in skname: branches[-1].append(skname[r[2]])
                    if r and r[0] == 'sr': branches[-1].extend(skname[x['d']] for x in r[2] if x['d'] in skname)
                # 'ee' ends the card's event chain. A choice that ends it on every outcome is a separate "end the
                # chain here" option; one that ends it on only some outcomes is treated as unsafe and left out.
                if any(ends) and not all(ends): continue
                tgt = endb if all(ends) else best
                if all(ends): has_end = True
                names = {n for b in branches for n in b}
                for n in names:
                    p = sum(n in b for b in branches) / len(branches)   # in k of n outcomes: k/n (outcomes equally likely)
                    tgt[n] = max(tgt.get(n, 0), p)
            if best: chain.append([lk, sorted([n if p == 1 else [n, p] for n, p in best.items()], key=str)])
            if has_end: ends_out.append([lk, sorted([n if p == 1 else [n, p] for n, p in endb.items()], key=str)])
        out[str(cid)] = {"name": cname.get(cid, str(cid)), "chain": sorted(chain), **({"end": sorted(ends_out)} if ends_out else {}), "status": "complete", "src": "gametora training_events"}
manual = json.load(open('manual_gm_nodates.json'))['cards']
for k, v in manual.items():
    if any(x in v for x in ('always', 'disabled', 'chance')): out[k] = {**out.get(k, {}), **{x: v[x] for x in v if x in ('name', 'always', 'disabled', 'chance', 'note')}, "status": "complete", "src": v.get('src', '')}
doc = {"about": json.load(open('manual_gm_nodates.json'))['about'], "cards": dict(sorted(out.items()))}
json.dump(doc, open('../card_events_nodates.json', 'w'), ensure_ascii=False, indent=1)
print(len(out), 'cards written')
