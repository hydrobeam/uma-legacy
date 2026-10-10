"""Build the era's card_events.json from the cached GameTora training-event files (cache/gametora/: ./build.sh gametora)
+ master.mdb. usage: build_card_events.py global|jp
Chain links come from master.mdb (single_mode_story_data.show_progress_1 for the event's story id). The files are
read and decoded by gametora.py.
A skill in every outcome of some choice is certain when the link fires; a skill in k of a choice's n outcomes
(split by 'di') is stored with chance k/n, outcomes taken as equally likely. Pal entries and bug notes are kept by hand."""
import json, os
import gametora
from event_choices import decode, fmt, manual_entry, apply_shares
from era import q, names, OUT, released
skname = names(47)
cname = names(75)
link = {sid: p for sid, p in q("select story_id,show_progress_1 from single_mode_story_data where show_progress_1>0")}
glob_cards = {i for i, sd in q("select id, start_date from support_card_data") if released(sd)}
out = {}
for key in ['training_events_sr', 'training_events_ssr']:
    for cid, evs in gametora.load(key):
        if cid not in glob_cards: continue
        chain = []; ends_out = []; risk = []; alts = []
        for e in evs:
            lk = link.get(int(e[2]))
            if not lk: continue
            best, endb, has_end, rk, alt = decode(e[1], skname)
            if rk: risk.append([lk, *rk])
            if best: chain.append([lk, fmt(best)])
            if has_end: ends_out.append([lk, fmt(endb)])
            if alt: alts.append([lk, alt])
        out[str(cid)] = {"name": cname.get(cid, str(cid)), "chain": sorted(chain), **({"end": sorted(ends_out)} if ends_out else {}), **({"stop_risk": sorted(risk)} if risk else {}), **({"alts": sorted(alts)} if alts else {}), "status": "complete", "src": "gametora training_events"}
apply_shares(out, os.path.join(os.path.dirname(os.path.abspath(__file__)), '..', 'data', 'event_shares.json'))   # measured outcome shares replace the equal-outcome convention
# the hand-entered entries (pal and group cards, corrections): the era's manual.json
MANUAL = os.path.join(OUT, 'manual.json')
DEST = os.path.join(OUT, 'card_events.json')
manual = json.load(open(MANUAL))['cards']
for k, v in manual.items():
    if int(k) not in glob_cards: continue   # released after the cut
    if any(x in v for x in ('always', 'disabled', 'chance')): out[k] = {**out.get(k, {}), **manual_entry(k, v, skname)}
doc = {"about": json.load(open(MANUAL))['about'], "cards": dict(sorted(out.items()))}
json.dump(doc, open(DEST, 'w'), ensure_ascii=False, indent=1)
print(len(out), 'cards written')
