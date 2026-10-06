"""Assemble the web app in dist/: index.html (template, src/style.css, src/*.js bundled by esbuild, the page's data,
skill catalog, precomputed example) and assets/: the solver (loopopt.wasm), its data per era, the GM era's page data,
images (character icons, card art, skill icons, race banners, scenario logos, UI sprites from images.py) and fonts
(the Fontsource packages' Latin subsets). Every file in assets/ carries a fingerprint of its content in its name (images: one fingerprint over the
whole set, as a folder name), so it is served with a one-year immutable cache and an update simply gets new names;
index.html keeps a 5-minute cache and always points at the current names. Favicons (icons.py) sit at the root.
Served over http(s): the page fetches the wasm, data and images, so it is not meant to be opened as a file."""
import json, subprocess, os, sqlite3, glob, shutil, hashlib, re
H = os.path.dirname(os.path.abspath(__file__)); P = os.path.join(H, '..', 'pkg', 'loopopt')
J = os.path.join(H, '..', 'jp')
wasm = open(os.path.join(H, 'loopopt.opt.wasm'), 'rb').read()
worker = open(os.path.join(H, 'worker.template.js')).read()
example = json.load(open(os.path.join(H, 'example.json')))
# character icons and card art extracted by images.py, copied to dist/img/<kind>/<id>.webp; the page gets the list of
# keys ('card:30116', ...) so it asks only for images that exist. None is fine (the page shows initials).
DIST = os.path.join(H, 'dist')
shutil.rmtree(DIST, ignore_errors=True)   # dist/ holds build output only: start clean, so nothing stale is deployed
os.makedirs(os.path.join(DIST, 'assets'), exist_ok=True)
def fingerprint(name, data):
    """write assets/<stem>.<hash><ext> (hash of the content) and return its path relative to the page"""
    data = data.encode() if isinstance(data, str) else data
    stem, e = os.path.splitext(name); rel = f'assets/{stem}.{hashlib.sha256(data).hexdigest()[:10]}{e}'
    open(os.path.join(DIST, rel), 'wb').write(data); return rel
# images are AVIF (UI sprites lossless WebP); one extension per kind, given to the page with the key list. They are
# copied to assets/img.<hash>/<kind>/<id>.<ext>, the hash taken over every file's name and content: any change gives
# the whole set a new folder (images change only when images.py is rerun)
images = []; ext = {}; isize = 0; files = []; ih = hashlib.sha256()
for kind in ('uma', 'outfit', 'card', 'skill', 'race', 'scen', 'ui'):
    d = os.path.join(H, 'img', kind)
    for f in sorted(os.listdir(d)) if os.path.isdir(d) else []:
        stem, e = os.path.splitext(f)
        if e not in ('.avif', '.webp'): continue
        assert ext.setdefault(kind, e[1:]) == e[1:], f'img/{kind} mixes formats ({f}); rerun ./build.sh images'
        src = os.path.join(d, f); ih.update(f'{kind}/{f}'.encode()); ih.update(open(src, 'rb').read())
        files.append((kind, f, src)); images.append(f'{kind}:{stem}'); isize += os.path.getsize(src)
IMG_BASE = f'assets/img.{ih.hexdigest()[:10]}/'
for kind, f, src in files:
    os.makedirs(os.path.join(DIST, IMG_BASE, kind), exist_ok=True); shutil.copyfile(src, os.path.join(DIST, IMG_BASE, kind, f))
# skill group -> icon id of its white and of its gold version (images.py)
def _icons(f): f = os.path.join(H, 'img', f); return json.load(open(f)) if os.path.exists(f) else {}
skill_icons = {'white': _icons('skill_icons.json'), 'gold': _icons('skill_icons_gold.json')}
# schedule rows name their race; the page maps names to race ids for the banners (same 34 G1s in both eras)
# keyed by both the full name and the chart's short name (pkg/loopopt/src/lib.rs chart_race_name: aliases, else 22 chars)
CHART_ALIAS = {'Tokyo Yushun (Japanese Derby)': 'Japanese Derby', 'Asahi Hai Futurity Stakes': 'Asahi Hai FS', 'Mile Championship Nambu Hai': 'MCS Nambu Hai',
               'Hanshin Juvenile Fillies': 'Hanshin JF', 'Zen-Nippon Junior Yushun': 'Zen-Nippon Jr Yushun'}
race_ids = {}
for dd in (P, J):
    dj = json.load(open(os.path.join(dd, 'data.json')))
    for nm, rid in zip(dj['race_names'], dj['races']): race_ids[nm] = rid; race_ids[CHART_ALIAS.get(nm, nm[:22])] = rid
def js(o): return json.dumps(o, ensure_ascii=False, separators=(',', ':')).replace('</', '<\\/')
# GameTora's card page names (url_name, e.g. 30067-the-thrones-assemblage), from its cached support-card list
_gt = sorted(glob.glob(os.path.join(P, 'gametora', 'support-cards.*.json')))
_gtc = json.load(open(_gt[-1])) if _gt else []
GT_URL = {c['support_id']: c['url_name'] for c in _gtc}
# GameTora's character slug (its character pages are /characters/<outfit id>-<slug>), from her support cards' url_name
GT_CHAR = {c['char_id']: c['url_name'].split('-', 1)[1] for c in _gtc if c.get('char_id') and '-' in c.get('url_name', '')}
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
        ui_cards.append({'id': c['id'], 'name': c['name'], 'rar': c['rarity'], 'pal': c['pal'], 'kind': c.get('kind', ''), 'rel': rel.get(c['id'], c['id']), 'gt': GT_URL.get(c['id']),
                         'whites': c['whites'], 'ev': evsum.get(c['id'], []),
                         **({'pri': pri} if (pri := events['cards'].get(str(c['id']), {}).get('prioritise')) else {}),   # skills to prioritise for IT to take the event choice the model credits
                         **({'ban': ban} if (ban := [sc for sc, ids in cards.get('restrict', {}).items() if c['id'] in ids]) else {})})   # scenarios that bar it from the deck
    umas = [{'id': u['id'], 'name': u['name'], 'gt': GT_CHAR.get(u['id']), 'outfits': [{'id': c[0], 'label': c[1], 'rel': rel.get(c[0], c[0])} for c in skills['cards'].get(str(u['id']), [])]} for u in data['umas']]
    engine = js({'data': data, 'skills': skills, 'cards': cards, 'events': events})
    return {'cards': js(ui_cards), 'umas': js(umas), 'skills': js(skills['skills']), 'engine': engine}
G = era(P, 'card_events.json')
M = era(J, 'card_events_nodates.json')      # GM era: pal first-year date events left out, as in Global
# the page script: src/main.js and the modules it imports, bundled into one IIFE (esbuild from node_modules: npm ci)
ESBUILD = os.path.join(H, 'node_modules', '.bin', 'esbuild')
if not os.path.exists(ESBUILD): raise SystemExit('esbuild missing: run npm ci in webapp/')
app = subprocess.run([ESBUILD, os.path.join(H, 'src', 'main.js'), '--bundle', '--format=iife', '--charset=utf8', '--log-level=warning'],
                     check=True, capture_output=True, text=True).stdout
css = open(os.path.join(H, 'src', 'style.css')).read()
assert '</style' not in css.lower()
html = open(os.path.join(H, 'index.template.html')).read()
# version shown in the footer: the git tag (and commits since it), "-dirty" with uncommitted changes to what the page
# is built from (webapp/, pkg/, jp/; edits elsewhere, e.g. docs, don't count)
def _git(*a): return subprocess.run(['git', *a], cwd=H, capture_output=True, text=True, check=True).stdout.strip()
try: VERSION = _git('describe', '--tags', '--always') + ('-dirty' if _git('status', '--porcelain', '--untracked-files=no', '--', '../webapp', '../pkg', '../jp') else '')
except Exception: VERSION = 'unknown'
# the GM era's data, fetched only when that era is selected: the page's lists, and the solver's data as text
gm_ui = '{"cards":' + M['cards'] + ',"umas":' + M['umas'] + ',"skills":' + M['skills'] + '}'
ASSETS = {'wasm': fingerprint('loopopt.wasm', wasm), 'engine': {'global': fingerprint('engine-global.json', G['engine']), 'gm': fingerprint('engine-gm.json', M['engine'])},
          'dataGm': fingerprint('data-gm.json', gm_ui), 'img': IMG_BASE}
# fonts: the Fontsource npm packages (webapp/package.json), self-hosted. Each weight's Latin subset as Fontsource writes
# it (its own @font-face rule; the woff2 only, fingerprinted); anything else (symbols, the JP era's untranslated names)
# falls back to the system font per glyph. The UI font's 400 and 700 are preloaded.
FONTS = [('zen-kaku-gothic-new', w) for w in (400, 500, 700, 900)] + [('ibm-plex-mono', w) for w in (400, 500, 600)]
faces = []; preloads = []
for pkg, w in FONTS:
    d = os.path.join(H, 'node_modules', '@fontsource', pkg)
    rule = open(os.path.join(d, f'latin-{w}.css')).read()
    woff2 = re.search(r"url\(\./files/([^)]+\.woff2)\)", rule).group(1)
    url = fingerprint(woff2, open(os.path.join(d, 'files', woff2), 'rb').read())
    faces.append(re.sub(r"src: [^;]+;", f"src: url({url}) format('woff2');", rule).strip())
    if pkg == 'zen-kaku-gothic-new' and w in (400, 700): preloads.append(f'<link rel="preload" href="{url}" as="font" type="font/woff2" crossorigin>')
os.makedirs(os.path.join(DIST, 'licenses'), exist_ok=True)   # the fonts' SIL Open Font Licenses (not fingerprinted: kept out of assets/)
for pkg in ('zen-kaku-gothic-new', 'ibm-plex-mono', 'stix-two-math'): shutil.copyfile(os.path.join(H, 'node_modules', '@fontsource', pkg, 'LICENSE'), os.path.join(DIST, 'licenses', f'OFL-{pkg}.txt'))
css = '\n'.join(faces) + '\n' + css
# /about's formulas (MathML): STIX Two Math, whole (its MATH table stretches brackets and big operators); the browser
# fetches it only on a page that draws math, so only /about gets the face
_d = os.path.join(H, 'node_modules', '@fontsource', 'stix-two-math')
MATHFACE = re.sub(r"src: [^;]+;", "src: url(" + fingerprint('stix-two-math-latin-400-normal.woff2', open(os.path.join(_d, 'files', 'stix-two-math-latin-400-normal.woff2'), 'rb').read()) + ") format('woff2');",
                  open(os.path.join(_d, 'latin-400.css')).read()).strip()
# the site's address for the link-preview tags (og:url, og:image); a preview deploy passes its own (SITE_URL=...)
SITE = os.environ.get('SITE_URL', 'https://uma-legacy.pages.dev').rstrip('/')
for k, v in [('__VERSION__', VERSION), ('__SITE__', SITE), ('__SITE__', SITE), ('__CSS__', css), ('__APP__', app), ('__CARDS__', G['cards']), ('__UMAS__', G['umas']), ('__SKILLCAT__', G['skills']),
             ('__EXAMPLE__', js(example)), ('__IMG__', js({'ext': ext, 'keys': images, 'base': IMG_BASE})), ('__SKILLICON__', js(skill_icons)), ('__RACEIDS__', js(race_ids)),
             ('__ASSETS__', js(ASSETS)), ('__ASSETS__', js(ASSETS)), ('__FONTPRELOAD__', '\n'.join(preloads)), ('__WORKER__', worker)]:
    assert '</script' not in v.lower(), k
    html = html.replace(k, v, 1)
# the logo: the favicon's three sparkles inline in the title (gradient ids prefixed so they can't clash)
LOGO = re.sub(r'\bid="g(\d)"', r'id="lg\1"', open(os.path.join(H, 'icons', 'favicon.svg')).read()).replace('url(#g', 'url(#lg')
LOGO = LOGO.replace('<svg ', '<svg class="logo" aria-hidden="true" focusable="false" ', 1).strip()
html = html.replace('__LOGO__', LOGO, 1)
assert not re.search(r'__(?!PURE__)[A-Z]+__', html), re.findall(r'__(?!PURE__)[A-Z]+__', html)   # every placeholder filled (esbuild's @__PURE__ marks aside)
open(os.path.join(DIST, 'index.html'), 'w').write(html)
# "How this works" (/about; Cloudflare Pages serves about.html there): the same styles and fonts, no script
about = open(os.path.join(H, 'about.template.html')).read()
for k, v in [('__VERSION__', VERSION), ('__SITE__', SITE), ('__SITE__', SITE), ('__CSS__', MATHFACE + '\n' + css), ('__FONTPRELOAD__', '\n'.join(preloads)), ('__LOGO__', LOGO)]: about = about.replace(k, v, 1)
assert not re.search(r'__[A-Z]+__', about), re.findall(r'__[A-Z]+__', about)
open(os.path.join(DIST, 'about.html'), 'w').write(about)
for f in ('favicon.ico', 'favicon.svg', 'favicon-32.png', 'apple-touch-icon.png', 'og.png'): shutil.copyfile(os.path.join(H, 'icons', f), os.path.join(DIST, f))
# hosting (Cloudflare Pages reads _headers): indexable (owner, 2026-10-05; the page names its canonical address); the page 5 minutes so a new deploy shows
# quickly; fingerprinted assets a year, immutable (their names change with their content)
open(os.path.join(DIST, '_headers'), 'w').write('/*\n  Cache-Control: public, max-age=300\n'
                                                 '/assets/*\n  ! Cache-Control\n  Cache-Control: public, max-age=31536000, immutable\n')
open(os.path.join(DIST, 'robots.txt'), 'w').write('User-agent: *\nAllow: /\n')
print(f"{VERSION}: dist/index.html {len(html.encode()) / 1e6:.2f} MB, solver data {len(G['engine'].encode()) / 1e6:.2f} MB, loopopt.wasm {len(wasm) / 1e6:.2f} MB, GM era {(len(gm_ui.encode()) + len(M['engine'].encode())) / 1e6:.2f} MB, {IMG_BASE} {len(images)} files {isize / 1e6:.2f} MB, {len(faces)} font files")
