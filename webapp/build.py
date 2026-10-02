"""Assemble the single-file web app: wasm (base64), pure-JS fallback, data, skill catalog, precomputed example."""
import json, base64, subprocess, os, re
H = os.path.dirname(os.path.abspath(__file__)); P = os.path.join(H, '..', 'pkg', 'loopopt')
J = os.path.join(H, '..', 'jp')
wasm = open(os.path.join(H, 'loopopt.opt.wasm'), 'rb').read()
fb = open(os.path.join(H, 'loopopt.wasm2js.mjs')).read()
fb = re.sub(r'^export var .*$', '', fb, flags=re.M)
worker = open(os.path.join(H, 'worker.template.js')).read().replace('/*FALLBACK*/', fb)
example = json.load(open(os.path.join(H, 'example.json')))
def js(o): return json.dumps(o, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
# Random events that hint a target (GameTora training_events: chara 'shared', char_card, friend, group), at the rate
# "always fires"; the page scales them. Trainee: (chara id or outfit id, group, chance). Deck: (card, group, level, chance).
NN, RU, US, PO = 20049, 20046, 20159, 20166
TRAINEE_HINTS = [[1008, NN, 1.0], [1060, RU, 1.0], [1042, US, 1.0], [101202, NN, 0.5], [101303, PO, 1 / 3]]
DECK_HINTS = [[10060, RU, 1, 0.5], [30036, RU, 1, 0.5], [30137, RU, 1, 1.0]]
def era(dirp, events_file):
    data = json.load(open(os.path.join(dirp, 'data.json'))); skills = json.load(open(os.path.join(dirp, 'web_skills.json')))
    cards = json.load(open(os.path.join(dirp, 'web_cards.json'))); events = json.load(open(os.path.join(dirp, events_file)))
    look = {}
    for g, w, gd in skills['skills']:
        look[w] = (g, 1)
        if gd: look[gd] = (g, 2)
    evsum = {}
    for cid, c in events['cards'].items():
        out = []
        for key in ('chain', 'end'):
            for lk, names in c.get(key, []):
                for n in names:
                    nm, p = (n[0], n[1]) if isinstance(n, list) else (n, 1.0)
                    if nm in look: out.append([look[nm][0], look[nm][1], lk, p])
        for nm in c.get('always', []):
            if nm in look: out.append([look[nm][0], look[nm][1], 0, 1.0])
        for nm, pr in c.get('chance', []):
            if nm in look: out.append([look[nm][0], look[nm][1], 0, pr])
        if out: evsum[int(cid)] = out
    ui_cards = []
    for c in cards['cards']:
        # pals, group cards, R cards (no chain events: hints only, no chain slot) and SR/SSR cards with chains
        if not c['pal'] and c['links'] == 0 and c['rarity'] != 1: continue
        if not c['pal'] and c['links'] > 0 and c['rarity'] < 2: continue
        ui_cards.append({'id': c['id'], 'name': c['name'], 'rar': c['rarity'], 'pal': c['pal'], 'links': c['links'], 'hf': c['hf'], 'kind': c.get('kind', ''),
                         'whites': c['whites'], 'ev': evsum.get(c['id'], [])})
    umas = [{'id': u['id'], 'name': u['name'], 'outfits': [{'id': c[0], 'label': c[1]} for c in skills['cards'].get(str(u['id']), [])]} for u in data['umas']]
    uma_ids = {u['id'] for u in umas}; outfit_ids = {o['id'] for u in umas for o in u['outfits']}; card_ids = {c['id'] for c in cards['cards']}
    hints = {'trainee': [h for h in TRAINEE_HINTS if h[0] in uma_ids or h[0] in outfit_ids],
             'deck': [h for h in DECK_HINTS if h[0] in card_ids]}
    engine = js({'data': data, 'skills': skills, 'cards': cards, 'events': events})
    return {'cards': js(ui_cards), 'umas': js(umas), 'skills': js(skills['skills']), 'engine': engine, 'hints': js(hints)}
G = era(P, 'card_events.json')
M = era(J, 'card_events_nodates.json')      # GM era: pal first-year dates are added by the page's toggle
html = open(os.path.join(H, 'index.template.html')).read()
for k, v in [('__CARDS__', G['cards']), ('__UMAS__', G['umas']), ('__SKILLCAT__', G['skills']), ('__HINTS__', G['hints']),
             ('__CARDS_GM__', M['cards']), ('__UMAS_GM__', M['umas']), ('__SKILLCAT_GM__', M['skills']), ('__HINTS_GM__', M['hints']),
             ('__EXAMPLE__', js(example)), ('__DATA_GM__', M['engine']), ('__DATA__', G['engine']),
             ('__WASM__', base64.b64encode(wasm).decode()), ('__WORKER__', worker)]:
    assert '</script' not in v.lower(), k
    html = html.replace(k, v, 1)
os.makedirs(os.path.join(H, 'dist'), exist_ok=True)
open(os.path.join(H, 'dist', 'index.html'), 'w').write(html)
print('dist/index.html', round(len(html.encode()) / 1e6, 2), 'MB')
