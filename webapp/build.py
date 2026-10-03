"""Assemble the web app in dist/: index.html (template, src/style.css, src/*.js bundled by esbuild, data, skill catalog,
precomputed example), loopopt.wasm, and img/ (character icons, card art, skill icons, race banners, UI sprites from
images.py). Served over http(s): the page fetches the wasm and the images, so it is not meant to be opened as a file."""
import json, subprocess, os, sqlite3, glob, shutil
H = os.path.dirname(os.path.abspath(__file__)); P = os.path.join(H, '..', 'pkg', 'loopopt')
J = os.path.join(H, '..', 'jp')
wasm = open(os.path.join(H, 'loopopt.opt.wasm'), 'rb').read()
worker = open(os.path.join(H, 'worker.template.js')).read()
example = json.load(open(os.path.join(H, 'example.json')))
# character icons and card art extracted by images.py, copied to dist/img/<kind>/<id>.webp; the page gets the list of
# keys ('card:30116', ...) so it asks only for images that exist. None is fine (the page shows initials).
DIST = os.path.join(H, 'dist')
shutil.rmtree(os.path.join(DIST, 'img'), ignore_errors=True)
# images are AVIF (UI sprites lossless WebP); one extension per kind, given to the page with the key list
images = []; ext = {}; isize = 0
for kind in ('uma', 'outfit', 'card', 'skill', 'race', 'ui'):
    d = os.path.join(H, 'img', kind)
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        stem, e = os.path.splitext(f)
        if e not in ('.avif', '.webp'): continue
        assert ext.setdefault(kind, e[1:]) == e[1:], f'img/{kind} mixes formats ({f}); rerun ./build.sh images'
        os.makedirs(os.path.join(DIST, 'img', kind), exist_ok=True)
        shutil.copyfile(os.path.join(d, f), os.path.join(DIST, 'img', kind, f)); images.append(f'{kind}:{stem}'); isize += os.path.getsize(os.path.join(d, f))
sip = os.path.join(H, 'img', 'skill_icons.json')   # skill group -> icon id (images.py)
skill_icons = json.load(open(sip)) if os.path.exists(sip) else {}
# schedule rows name their race; the page maps names to race ids for the banners (same 34 G1s in both eras)
# keyed by both the full name and the chart's short name (pkg/loopopt/src/lib.rs chart_race_name: aliases, else 22 chars)
CHART_ALIAS = {'Tokyo Yushun (Japanese Derby)': 'Japanese Derby', 'Asahi Hai Futurity Stakes': 'Asahi Hai FS', 'Mile Championship Nambu Hai': 'MCS Nambu Hai',
               'Hanshin Juvenile Fillies': 'Hanshin JF', 'Zen-Nippon Junior Yushun': 'Zen-Nippon Jr Yushun'}
race_ids = {}
for dd in (P, J):
    dj = json.load(open(os.path.join(dd, 'data.json')))
    for nm, rid in zip(dj['race_names'], dj['races']): race_ids[nm] = rid; race_ids[CHART_ALIAS.get(nm, nm[:22])] = rid
def js(o): return json.dumps(o, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
# Random events that hint a target (GameTora training_events: chara 'shared', char_card, friend, group), at the rate
# "always fires"; the page scales them. Trainee: (chara id or outfit id, group, chance). Deck: (card, group, level, chance).
NN, RU, US, PO = 20049, 20046, 20159, 20166
TRAINEE_HINTS = [[1008, NN, 1.0], [1060, RU, 1.0], [1042, US, 1.0], [101202, NN, 0.5], [101303, PO, 1 / 3]]
DECK_HINTS = [[10060, RU, 1, 0.5], [30036, RU, 1, 0.5], [30137, RU, 1, 1.0]]
# GameTora's card page names (url_name, e.g. 30067-the-thrones-assemblage), from its cached support-card list
_gt = sorted(glob.glob(os.path.join(P, 'gametora', 'support-cards.*.json')))
GT_URL = {c['support_id']: c['url_name'] for c in json.load(open(_gt[-1]))} if _gt else {}
def era(dirp, events_file):
    data = json.load(open(os.path.join(dirp, 'data.json'))); skills = json.load(open(os.path.join(dirp, 'web_skills.json')))
    cards = json.load(open(os.path.join(dirp, 'web_cards.json'))); events = json.load(open(os.path.join(dirp, events_file)))
    # release dates for the roster's and card picker's default order; without master.mdb, id order
    mdb = os.path.join(dirp, 'export', 'master.mdb'); rel = {}
    if os.path.exists(mdb):
        con = sqlite3.connect(f'file:{mdb}?mode=ro', uri=True)
        # an outfit's release = the start of the first gacha it appeared in; else its uma's start date
        rel = dict(con.execute('select c.id, c2.start_date from card_data c join chara_data c2 on c2.id = c.chara_id'))
        rel.update(con.execute('select a.card_id, min(d.start_date) from gacha_available a join gacha_data d on d.id = a.gacha_id group by a.card_id'))
        rel.update(con.execute('select id, start_date from support_card_data'))   # support cards carry their own date
        con.close()
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
        ui_cards.append({'id': c['id'], 'name': c['name'], 'rar': c['rarity'], 'pal': c['pal'], 'links': c['links'], 'hf': c['hf'], 'kind': c.get('kind', ''), 'rel': rel.get(c['id'], c['id']), 'gt': GT_URL.get(c['id']),
                         'whites': c['whites'], 'ev': evsum.get(c['id'], [])})
    umas = [{'id': u['id'], 'name': u['name'], 'outfits': [{'id': c[0], 'label': c[1], 'rel': rel.get(c[0], c[0])} for c in skills['cards'].get(str(u['id']), [])]} for u in data['umas']]
    uma_ids = {u['id'] for u in umas}; outfit_ids = {o['id'] for u in umas for o in u['outfits']}; card_ids = {c['id'] for c in cards['cards']}
    hints = {'trainee': [h for h in TRAINEE_HINTS if h[0] in uma_ids or h[0] in outfit_ids],
             'deck': [h for h in DECK_HINTS if h[0] in card_ids]}
    engine = js({'data': data, 'skills': skills, 'cards': cards, 'events': events})
    return {'cards': js(ui_cards), 'umas': js(umas), 'skills': js(skills['skills']), 'engine': engine, 'hints': js(hints)}
G = era(P, 'card_events.json')
M = era(J, 'card_events_nodates.json')      # GM era: pal first-year dates are added by the page's toggle
# the page script: src/main.js and the modules it imports, bundled into one IIFE (esbuild from node_modules: npm ci)
ESBUILD = os.path.join(H, 'node_modules', '.bin', 'esbuild')
if not os.path.exists(ESBUILD): raise SystemExit('esbuild missing: run npm ci in webapp/')
app = subprocess.run([ESBUILD, os.path.join(H, 'src', 'main.js'), '--bundle', '--format=iife', '--charset=utf8', '--log-level=warning'],
                     check=True, capture_output=True, text=True).stdout
css = open(os.path.join(H, 'src', 'style.css')).read()
assert '</style' not in css.lower()
html = open(os.path.join(H, 'index.template.html')).read()
for k, v in [('__CSS__', css), ('__APP__', app), ('__CARDS__', G['cards']), ('__UMAS__', G['umas']), ('__SKILLCAT__', G['skills']), ('__HINTS__', G['hints']),
             ('__EXAMPLE__', js(example)), ('__IMG__', js({'ext': ext, 'keys': images})), ('__SKILLICON__', js(skill_icons)), ('__RACEIDS__', js(race_ids)), ('__DATA__', G['engine']),
             ('__WORKER__', worker)]:
    assert '</script' not in v.lower(), k
    html = html.replace(k, v, 1)
os.makedirs(DIST, exist_ok=True)
open(os.path.join(DIST, 'index.html'), 'w').write(html)
open(os.path.join(DIST, 'loopopt.wasm'), 'wb').write(wasm)
# the GM era's data, fetched only when that era is selected: the page's lists, and the solver's data as text
gm_ui = '{"cards":' + M['cards'] + ',"umas":' + M['umas'] + ',"skills":' + M['skills'] + ',"hints":' + M['hints'] + '}'
open(os.path.join(DIST, 'data-gm.json'), 'w').write(gm_ui)
open(os.path.join(DIST, 'engine-gm.json'), 'w').write(M['engine'])
print(f"dist/index.html {len(html.encode()) / 1e6:.2f} MB, loopopt.wasm {len(wasm) / 1e6:.2f} MB, GM era {(len(gm_ui.encode()) + len(M['engine'].encode())) / 1e6:.2f} MB, img/ {len(images)} files {isize / 1e6:.2f} MB")
