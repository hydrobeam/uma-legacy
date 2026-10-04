"""Build ../card_events.json from the cached GameTora training-event files (pkg/loopopt/gametora/) + master.mdb.
Chain links come from master.mdb (single_mode_story_data.show_progress_1 for the event's story id).
Reward codes index dict/evrew with an offset of 36 (verified: every card's bond reward decodes to its own
character, and King Halo / Yaeno / Gold Ship / Light Hello / Anshinzawa match the wiki).
A skill in every outcome of some choice is certain when the link fires; a skill in k of a choice's n outcomes
(split by 'di') is stored with chance k/n, outcomes taken as equally likely. Pal entries and bug notes are kept by hand."""
import json, sqlite3, glob, os, sys
sys.path.insert(0, os.path.join(os.path.dirname(os.path.abspath(__file__)), '../../pkg/loopopt/export'))
from event_choices import decode, fmt
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
        chain = []; ends_out = []; risk = []; alts = []
        for e in evs:
            lk = link.get(int(e[2]))
            if not lk: continue
            best, endb, has_end, rk, alt = decode(e[1], reward, skname)
            if rk: risk.append([lk, *rk])
            if best: chain.append([lk, fmt(best)])
            if has_end: ends_out.append([lk, fmt(endb)])
            if alt: alts.append([lk, alt])
        out[str(cid)] = {"name": cname.get(cid, str(cid)), "chain": sorted(chain), **({"end": sorted(ends_out)} if ends_out else {}), **({"stop_risk": sorted(risk)} if risk else {}), **({"alts": sorted(alts)} if alts else {}), "status": "complete", "src": "gametora training_events"}
manual = json.load(open('manual_gm_nodates.json'))['cards']
for k, v in manual.items():
    if any(x in v for x in ('always', 'disabled', 'chance')): out[k] = {**out.get(k, {}), **{x: v[x] for x in v if x in ('name', 'always', 'disabled', 'chance', 'note')}, "status": "complete", "src": v.get('src', '')}
doc = {"about": json.load(open('manual_gm_nodates.json'))['about'], "cards": dict(sorted(out.items()))}
json.dump(doc, open('../card_events_nodates.json', 'w'), ensure_ascii=False, indent=1)
print(len(out), 'cards written')
