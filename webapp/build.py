"""Assemble the web app in dist/: index.html (template, src/style.css, src/*.js bundled by esbuild, the page's data,
skill catalog, precomputed example) and assets/: the solver (loopopt.wasm), its data per era, the GM era's page data,
images (character icons, card art, skill icons, race banners, scenario logos, UI sprites from images.py) and fonts
(the Fontsource packages' Latin subsets). Every file in assets/ carries a fingerprint of its content in its name (images: one fingerprint over the
whole set, as a folder name), so it is served with a one-year immutable cache and an update simply gets new names;
index.html keeps a 5-minute cache and always points at the current names. Favicons (icons.py) sit at the root.
Served over http(s): the page fetches the wasm, data and images, so it is not meant to be opened as a file."""

import json, subprocess, os, sqlite3, shutil, hashlib, re
from dataclasses import dataclass
from typing import NamedTuple

HERE = os.path.dirname(os.path.abspath(__file__))
GLOBAL_DATA = os.path.join(HERE, "..", "pkg", "loopopt", "data", "global")
JP_DATA = os.path.join(HERE, "..", "pkg", "loopopt", "data", "jp")
wasm = open(os.path.join(HERE, "loopopt.opt.wasm"), "rb").read()
worker = open(os.path.join(HERE, "worker.template.js")).read()
example = json.load(open(os.path.join(HERE, "example.json")))
DIST = os.path.join(HERE, "dist")
shutil.rmtree(DIST, ignore_errors=True)  # dist/ holds build output only: start clean, so nothing stale is deployed
os.makedirs(os.path.join(DIST, "assets"), exist_ok=True)


def fingerprint(name, data):
    """write assets/<stem>.<hash><ext> (hash of the content) and return its path relative to the page"""
    data = data.encode() if isinstance(data, str) else data
    stem, extension = os.path.splitext(name)
    relative_path = f"assets/{stem}.{hashlib.sha256(data).hexdigest()[:10]}{extension}"
    open(os.path.join(DIST, relative_path), "wb").write(data)
    return relative_path


# images are AVIF (UI sprites lossless WebP); one extension per kind, given to the page with the key list. They are
# copied to assets/img.<hash>/<kind>/<id>.<ext>, the hash taken over every file's name and content: any change gives
# the whole set a new folder (including runner strips from chibi/chibi.py)
class ImageFile(NamedTuple):
    """One source image and its destination name within its kind's folder."""

    kind: str
    name: str
    source: str


@dataclass
class ImageCatalog:
    """Copied images, their per-kind extension, total bytes and fingerprinted folder."""

    keys: list[str]
    extensions: dict[str, str]
    size_bytes: int
    base: str


def copy_images():
    images = []
    extensions = {}
    size_bytes = 0
    files = []
    image_hash = hashlib.sha256()
    for kind in ("uma", "mob", "outfit", "card", "skill", "race", "scen", "scenart", "ui", "run"):
        source_dir = os.path.join(HERE, "img", kind)
        for filename in sorted(os.listdir(source_dir)) if os.path.isdir(source_dir) else []:
            stem, extension = os.path.splitext(filename)
            if extension not in (".avif", ".webp"):
                continue
            assert extensions.setdefault(kind, extension[1:]) == extension[1:], (
                f"img/{kind} mixes formats ({filename}); rerun ./build.sh images"
            )
            source = os.path.join(source_dir, filename)
            image_hash.update(f"{kind}/{filename}".encode())
            image_hash.update(open(source, "rb").read())
            files.append(ImageFile(kind=kind, name=filename, source=source))
            images.append(f"{kind}:{stem}")
            size_bytes += os.path.getsize(source)
    image_base = f"assets/img.{image_hash.hexdigest()[:10]}/"
    for kind, filename, source in files:
        os.makedirs(os.path.join(DIST, image_base, kind), exist_ok=True)
        shutil.copyfile(source, os.path.join(DIST, image_base, kind, filename))
    return ImageCatalog(keys=images, extensions=extensions, size_bytes=size_bytes, base=image_base)


image_catalog = copy_images()


# skill group -> icon id of its white and of its gold version (images.py)
def _icons(f):
    f = os.path.join(HERE, "img", f)
    return json.load(open(f)) if os.path.exists(f) else {}


skill_icons = {"white": _icons("skill_icons.json"), "gold": _icons("skill_icons_gold.json")}
# schedule rows name their race; the page maps names to race ids for the banners (same 34 G1s in both eras)
# keyed by both the full name and the chart's short name (pkg/loopopt/src/lib.rs chart_race_name: aliases, else 22 chars)
CHART_ALIAS = {
    "Tokyo Yushun (Japanese Derby)": "Japanese Derby",
    "Asahi Hai Futurity Stakes": "Asahi Hai FS",
    "Mile Championship Nambu Hai": "MCS Nambu Hai",
    "Hanshin Juvenile Fillies": "Hanshin JF",
    "Zen-Nippon Junior Yushun": "Zen-Nippon Jr Yushun",
}
race_ids = {}
for data_dir in (GLOBAL_DATA, JP_DATA):
    data = json.load(open(os.path.join(data_dir, "data.json")))
    for race_name, race_id in zip(data["race_names"], data["races"]):
        race_ids[race_name] = race_id
        race_ids[CHART_ALIAS.get(race_name, race_name[:22])] = race_id


def js(o):
    return json.dumps(o, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")


# GameTora's page names for the links (support cards: url_name, e.g. 30067-the-thrones-assemblage; characters: the slug
# of /characters/<outfit id>-<slug>), tracked (pkg/loopopt/data/gametora_slugs.json, written by gametora.py): no download
_gts = json.load(open(os.path.join(HERE, "..", "pkg", "loopopt", "data", "gametora_slugs.json")))
GT_URL = {int(k): v for k, v in _gts["cards"].items()}
GT_CHAR = {int(k): v for k, v in _gts["chars"].items()}


@dataclass
class EraBundle:
    """JSON strings embedded in page scripts or written as engine assets."""

    cards: str
    umas: str
    skills: str
    engine: str
    career: str


class SkillReference(NamedTuple):
    """Skill group and rarity (1 white, 2 gold) for one localized skill name."""

    group: int
    rarity: int


def release_dates(data_dir):
    # release dates for the roster's and card picker's default order; without master.mdb, id order
    mdb = os.path.join(
        HERE, "..", "game", "jp" if os.path.basename(os.path.normpath(data_dir)) == "jp" else "global", "master.mdb"
    )
    release_times = {}
    if os.path.exists(mdb):
        con = sqlite3.connect(f"file:{mdb}?mode=ro", uri=True)
        # an outfit's release = the start of the first gacha it appeared in; else its uma's start date
        release_times = dict(
            con.execute("select c.id, c2.start_date from card_data c join chara_data c2 on c2.id = c.chara_id")
        )
        release_times.update(
            con.execute(
                "select a.card_id, min(d.start_date) from gacha_available a join gacha_data d on d.id = a.gacha_id group by a.card_id"
            )
        )
        release_times.update(
            con.execute("select id, start_date from support_card_data")
        )  # support cards carry their own date
        con.close()
    return release_times


def event_summary(skills, events):
    skills_by_name = {}
    for group_id, white_name, gold_name in skills["skills"]:
        skills_by_name[white_name] = SkillReference(group=group_id, rarity=1)
        if gold_name:
            skills_by_name[gold_name] = SkillReference(group=group_id, rarity=2)
    event_skills = {}
    for card_id, card in events["cards"].items():
        out = []
        for key in ("chain", "end"):
            for link_number, names in card.get(key, []):
                for reward in names:
                    skill_name, chance = (reward[0], reward[1]) if isinstance(reward, list) else (reward, 1.0)
                    if skill_name in skills_by_name:
                        out.append(
                            [skills_by_name[skill_name].group, skills_by_name[skill_name].rarity, link_number, chance]
                        )
        for skill_name in card.get("always", []):
            if skill_name in skills_by_name:
                out.append([skills_by_name[skill_name].group, skills_by_name[skill_name].rarity, 0, 1.0])
        for skill_name, chance in card.get("chance", []):
            if skill_name in skills_by_name:
                out.append([skills_by_name[skill_name].group, skills_by_name[skill_name].rarity, 0, chance])
        if out:
            event_skills[int(card_id)] = out
    return event_skills


def era(data_dir, events_file):
    data = json.load(open(os.path.join(data_dir, "data.json")))
    skills = json.load(open(os.path.join(data_dir, "web_skills.json")))
    cards = json.load(open(os.path.join(data_dir, "web_cards.json")))
    events = json.load(open(os.path.join(data_dir, events_file)))
    release_times = release_dates(data_dir)
    event_skills = event_summary(skills, events)
    ui_cards = []
    for card in cards["cards"]:
        # pals, group cards, R cards (no chain events: hints only, no chain slot) and SR/SSR cards with chains
        if not card["pal"] and card["links"] == 0 and card["rarity"] != 1:
            continue
        if not card["pal"] and card["links"] > 0 and card["rarity"] < 2:
            continue
        ui_cards.append(
            {
                "id": card["id"],
                "name": card["name"],
                "rar": card["rarity"],
                "pal": card["pal"],
                "kind": card.get("kind", ""),
                "rel": release_times.get(card["id"], card["id"]),
                "gt": GT_URL.get(card["id"]),
                "whites": card["whites"],
                "ev": event_skills.get(card["id"], []),
                **(
                    {"pri": pri} if (pri := events["cards"].get(str(card["id"]), {}).get("prioritise")) else {}
                ),  # skills to prioritise for IT to take the event choice the model credits
                **(
                    {"ban": ban}
                    if (ban := [scenario for scenario, ids in cards.get("restrict", {}).items() if card["id"] in ids])
                    else {}
                ),
            }
        )  # scenarios that bar it from the deck
    umas = [
        {
            "id": uma["id"],
            "name": uma["name"],
            "gt": GT_CHAR.get(uma["id"]),
            **({"apt": uma["apt"]} if uma.get("apt") else {}),
            "outfits": [
                {"id": card[0], "label": card[1], "rel": release_times.get(card[0], card[0])}
                for card in skills["cards"].get(str(uma["id"]), [])
            ],
        }
        for uma in data["umas"]
    ]
    engine = js({"data": data, "skills": skills, "cards": cards, "events": events})
    # the career page's veteran import: spark factor group -> skill group, win saddle -> race index, race names by index
    career_tables = json.load(open(os.path.join(data_dir, "career.json")))
    # and the turns each G1 is held at (0-71: year x 24 + half-month; Classic and Senior runnings share one race index)
    race_turns = [
        sorted(
            {
                turn_index
                for uma in data["umas"]
                for turn_index, choices in enumerate(uma["turns"])
                for choice in choices
                if choice["race"] == race_index
            }
        )
        for race_index in range(len(data["race_names"]))
    ]
    # skill groups something besides inheritance can give: a card's hints or events, an outfit's own skills (the career
    # planner's sample veterans carry only the targets outside this: the ones a loop must inherit)
    sourced = sorted(
        {group_id for card in cards["cards"] for group_id in card["whites"]}
        | {event[0] for card_events in event_skills.values() for event in card_events}
        | {group_id for outfits in skills["cards"].values() for outfit in outfits for group_id, _ in outfit[2]}
    )
    career = js(
        {
            "factors": career_tables["factors"],
            "saddles": career_tables["saddles"],
            "pinks": career_tables["pinks"],
            "blues": career_tables.get("blues", {}),
            "uniques": career_tables.get("uniques", {}),
            "races": data["race_names"],
            "race_turns": race_turns,
            "mobs": [m for m in career_tables.get("mobs", []) if f"mob:{m[0]}" in image_catalog.keys],
            "sourced": sourced,
        }
    )  # only mobs with a portrait (the samples draw from them)
    return EraBundle(cards=js(ui_cards), umas=js(umas), skills=js(skills["skills"]), engine=engine, career=career)


global_era = era(GLOBAL_DATA, "card_events.json")
gm_era = era(JP_DATA, "card_events.json")  # GM era: pal first-year date events left out, as in Global
# the page script: src/main.js and the modules it imports, bundled into one IIFE (esbuild from node_modules: npm ci)
ESBUILD = os.path.join(HERE, "node_modules", ".bin", "esbuild")
if not os.path.exists(ESBUILD):
    raise SystemExit("esbuild missing: run npm ci in webapp/")


def bundle(entry):
    return subprocess.run(
        [
            ESBUILD,
            os.path.join(HERE, "src", entry),
            "--bundle",
            "--format=iife",
            "--charset=utf8",
            "--minify",
            "--log-level=warning",
        ],
        check=True,
        capture_output=True,
        text=True,
    ).stdout


# Minify CSS whitespace; esbuild may retain legal comments.
def css_min(text):
    return subprocess.run(
        [ESBUILD, "--loader=css", "--minify-whitespace", "--charset=utf8", "--log-level=warning"],
        input=text,
        check=True,
        capture_output=True,
        text=True,
    ).stdout


app = bundle("main.js")
css = open(os.path.join(HERE, "src", "style.css")).read()
assert "</style" not in css.lower()
html = open(os.path.join(HERE, "index.template.html")).read()
# the partials both planner pages share (panels.template.html: the setup rows, the Enhance buttons, the settings editors and
# their dialogs): each __PANEL_NAME__ gets the block marked panel:name; a block may hold others, so this runs until none is left
PANELS = dict(
    re.findall(
        r"<!--panel:(\w+)-->(.*?)<!--/panel:\1-->", open(os.path.join(HERE, "panels.template.html")).read(), re.S
    )
)


def panels(page):
    while m := re.search(r"__PANEL_([A-Z]+)__", page):
        page = page.replace(m.group(0), PANELS[m.group(1).lower()], 1)
    return page


html = panels(html)


# version shown in the footer: the git tag (and commits since it), "-dirty" with uncommitted changes to what the page
# is built from (webapp/, pkg/; edits elsewhere, e.g. docs, don't count)
def _git(*a):
    return subprocess.run(["git", *a], cwd=HERE, capture_output=True, text=True, check=True).stdout.strip()


try:
    VERSION = _git("describe", "--tags", "--always") + (
        "-dirty" if _git("status", "--porcelain", "--untracked-files=no", "--", "../webapp", "../pkg") else ""
    )
except Exception:
    VERSION = "unknown"
# the GM era's data, fetched only when that era is selected: the page's lists, and the solver's data as text
gm_ui = (
    '{"cards":'
    + gm_era.cards
    + ',"umas":'
    + gm_era.umas
    + ',"skills":'
    + gm_era.skills
    + ',"career":'
    + gm_era.career
    + "}"
)
ASSETS = {
    "wasm": fingerprint("loopopt.wasm", wasm),
    "engine": {
        "global": fingerprint("engine-global.json", global_era.engine),
        "gm": fingerprint("engine-gm.json", gm_era.engine),
    },
    "dataGm": fingerprint("data-gm.json", gm_ui),
    "img": image_catalog.base,
}
# fonts: the Fontsource npm packages (webapp/package.json), self-hosted. Each weight's Latin subset as Fontsource writes
# it (its own @font-face rule; the woff2 only, fingerprinted); anything else (symbols, the JP era's untranslated names)
# falls back to the system font per glyph. The UI font's 400 and 700 are preloaded.
FONTS = (
    [("zen-kaku-gothic-new", w) for w in (400, 500, 700, 900)]
    + [("ibm-plex-mono", w) for w in (400, 500, 600)]
    + [("zen-maru-gothic", w) for w in (500, 700, 900)]
)  # rounded, the career page's game-style veteran card (fetched only where used)
faces = []
preloads = []
for pkg, w in FONTS:
    d = os.path.join(HERE, "node_modules", "@fontsource", pkg)
    rule = open(os.path.join(d, f"latin-{w}.css")).read()
    woff2 = re.search(r"url\(\./files/([^)]+\.woff2)\)", rule).group(1)
    url = fingerprint(woff2, open(os.path.join(d, "files", woff2), "rb").read())
    faces.append(re.sub(r"src: [^;]+;", f"src: url({url}) format('woff2');", rule).strip())
    if pkg == "zen-kaku-gothic-new" and w in (400, 700):
        preloads.append(f'<link rel="preload" href="{url}" as="font" type="font/woff2" crossorigin>')
os.makedirs(
    os.path.join(DIST, "licenses"), exist_ok=True
)  # the fonts' SIL Open Font Licenses (not fingerprinted: kept out of assets/)
for pkg in ("zen-kaku-gothic-new", "ibm-plex-mono", "stix-two-math", "zen-maru-gothic"):
    shutil.copyfile(
        os.path.join(HERE, "node_modules", "@fontsource", pkg, "LICENSE"),
        os.path.join(DIST, "licenses", f"OFL-{pkg}.txt"),
    )
# the UI sprites as CSS variables (--btn-w, --spark-white, ...), in the stylesheet itself so the game's buttons have their
# art at the first paint; a sprite the build lacks is left unset (the rules fall back to plain styles). A url() inside a
# custom property resolves against the stylesheet that uses it (Chromium and Firefox, checked), so the paths are given
# relative to where the stylesheet is served from
SPRITES = [
    "friend_frame",
    "spark_white",
    "spark_blue",
    "spark_pink",
    "spark_green",
    "spark_dot",
    "spark_dot_gold",
    "skill_white",
    "skill_gold",
    "skill_unique",
    "skill_pink",
    "pill",
    "star_on",
    "star_off",
    "spark_white_dark",
    "skill_white_dark",
    "star_off_dark",
    "enh_umas",
    "enh_cards",
    "enh_vets",
    "enh_umas_dark",
    "enh_cards_dark",
    "enh_vets_dark",
    "btn_w",
    "btn_g",
    "pill_w",
    "pill_g",
    "chk_off",
    "chk_on",
    "btn_w_dark",
    "pill_w_dark",
    "chk_off_dark",
    "tog_off",
    "tog_on",
    "tog_off_dark",
    "slot_frame",
    "slot_empty",
    "sched_badge",
    "plus",
    "slot_frame_dark",
    "slot_empty_dark",
    "close",
]


def sprite_rule(base):
    vs = [
        f"--{k.replace('_', '-')}:url({base}ui/{k}.{image_catalog.extensions['ui']})"
        for k in SPRITES
        if f"ui:{k}" in image_catalog.keys
    ]
    return ":root{" + ";".join(vs) + "}" if vs else ""


# One fingerprinted stylesheet for the three pages; font and sprite paths are relative to assets/.
css = css_min(
    "\n".join(f.replace("url(assets/", "url(") for f in faces)
    + "\n"
    + sprite_rule(image_catalog.base[len("assets/") :])
    + "\n"
    + css
)
CSS_LINK = f'<link rel="stylesheet" href="{fingerprint("style.css", css)}">'
# /about's formulas (MathML): STIX Two Math, whole (its MATH table stretches brackets and big operators); the browser
# fetches it only on a page that draws math, so only /about gets the face
_d = os.path.join(HERE, "node_modules", "@fontsource", "stix-two-math")
MATHFACE = re.sub(
    r"src: [^;]+;",
    "src: url("
    + fingerprint(
        "stix-two-math-latin-400-normal.woff2",
        open(os.path.join(_d, "files", "stix-two-math-latin-400-normal.woff2"), "rb").read(),
    )
    + ") format('woff2');",
    open(os.path.join(_d, "latin-400.css")).read(),
).strip()


# the planner pages' shared data (the Global era's lists, image keys, skill icons, race ids), the career page's tables
# and each page's script: fingerprinted classic scripts at the end of the body, data first.
# data.js reads the data from self.__LLP_DATA ('data-cards', ...).
def data_script(name, d):
    payload = "{" + ",".join(f'"{k}":{v}' for k, v in d.items()) + "}"
    return fingerprint(
        name,
        "Object.assign(self.__LLP_DATA=self.__LLP_DATA||{},JSON.parse("
        + json.dumps(payload, ensure_ascii=False)
        + "));",
    )


DATA_JS = data_script(
    "data-global.js",
    {
        "data-cards": global_era.cards,
        "data-umas": global_era.umas,
        "data-skills": global_era.skills,
        "data-img": js({"ext": image_catalog.extensions, "keys": image_catalog.keys, "base": image_catalog.base}),
        "data-skillicon": js(skill_icons),
        "data-raceids": js(race_ids),
    },
)
CAREER_DATA_JS = data_script("career-data.js", {"data-career": global_era.career})
MAIN_JS, CAREER_JS = fingerprint("main.js", app), fingerprint("career.js", bundle("career.js"))


def script_preloads(*srcs):
    return "\n".join(f'<link rel="preload" href="{x}" as="script">' for x in srcs)


# the site's address for the link-preview tags (og:url, og:image); a preview deploy passes its own (SITE_URL=...)
SITE = os.environ.get("SITE_URL", "https://uma-legacy.pages.dev").rstrip("/")
# light / dark: the visitor's choice (the header's button, saved in this browser), else the system's preference, followed
# live until the button is used; set on <html> before the first paint (no light flash), on all three pages
THEMEJS = """
(function () {
  var K = 'llp:theme', t = null, d = document.documentElement, m = window.matchMedia ? matchMedia('(prefers-color-scheme: dark)') : null;
  try { t = localStorage.getItem(K); } catch (e) {}
  if (t !== 'dark' && t !== 'light') t = null;
  function set() { var v = t || (m && m.matches ? 'dark' : 'light'), b = document.getElementById('theme-btn'); d.setAttribute('data-theme', v);
    if (b) { b.setAttribute('aria-pressed', String(v === 'dark')); b.title = v === 'dark' ? 'Switch to light mode' : 'Switch to dark mode'; } }
  set(); if (m && m.addEventListener) m.addEventListener('change', function () { if (!t) set(); });
  document.addEventListener('DOMContentLoaded', set);
  document.addEventListener('click', function (e) { if (!e.target.closest || !e.target.closest('#theme-btn')) return;
    t = d.getAttribute('data-theme') === 'dark' ? 'light' : 'dark'; try { localStorage.setItem(K, t); } catch (x) {}
    d.classList.add('themeswap'); set(); requestAnimationFrame(function () { requestAnimationFrame(function () { d.classList.remove('themeswap'); }); }); });
})();
"""
THEMEBTN = (
    '<button type="button" class="themebtn" id="theme-btn" aria-label="Dark mode" aria-pressed="false" title="Switch to dark mode">'
    '<svg class="i-moon" viewBox="0 0 24 24" aria-hidden="true"><path d="M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z" fill="currentColor"/></svg>'
    '<svg class="i-sun" viewBox="0 0 24 24" aria-hidden="true"><circle cx="12" cy="12" r="4.5" fill="currentColor"/><g stroke="currentColor" stroke-width="2" stroke-linecap="round">'
    '<path d="M12 2v2.5M12 19.5V22M2 12h2.5M19.5 12H22M4.9 4.9l1.8 1.8M17.3 17.3l1.8 1.8M4.9 19.1l1.8-1.8M17.3 6.7l1.8-1.8"/></g></svg></button>'
)
# the header's page tabs, the same on every page in the same order, the page you are on selected (the game's tab bar)
PAGES = [("loop", "./", "Loop planner"), ("career", "career", "Career planner"), ("about", "about", "How this works")]


def NAV(cur):
    tabs = "".join(f'<a href="{h}"' + (' aria-current="page"' if k == cur else "") + f">{t}</a>" for k, h, t in PAGES)
    return f'<nav class="tnav" aria-label="Pages"><div class="ptabs">{tabs}</div> {THEMEBTN}</nav>'


for k, v in [
    ("__THEMEJS__", THEMEJS),
    ("__NAV__", NAV("loop")),
    ("__VERSION__", VERSION),
    ("__SITE__", SITE),
    ("__SITE__", SITE),
    ("__CSS__", CSS_LINK),
    ("__APP__", MAIN_JS),
    ("__DATAJS__", DATA_JS),
    ("__EXAMPLE__", js(example)),
    ("__ASSETS__", js(ASSETS)),
    ("__ASSETS__", js(ASSETS)),
    ("__FONTPRELOAD__", "\n".join(preloads) + "\n" + script_preloads(DATA_JS, MAIN_JS)),
    ("__WORKER__", worker),
]:
    assert "</script" not in v.lower(), k
    html = html.replace(k, v, 1)
# the logo: the favicon's three sparkles inline in the title (gradient ids prefixed so they can't clash)
LOGO = re.sub(r'\bid="g(\d)"', r'id="lg\1"', open(os.path.join(HERE, "icons", "favicon.svg")).read()).replace(
    "url(#g", "url(#lg"
)
LOGO = LOGO.replace("<svg ", '<svg class="logo" aria-hidden="true" focusable="false" ', 1).strip()
html = html.replace("__LOGO__", LOGO, 1)
assert not re.search(r"__(?!PURE__)[A-Z]+__", html), re.findall(
    r"__(?!PURE__)[A-Z]+__", html
)  # every placeholder filled (esbuild's @__PURE__ marks aside)
open(os.path.join(DIST, "index.html"), "w").write(html)
# "How this works" (/about; Cloudflare Pages serves about.html there): shared styles and fonts.
# Its disclosure script opens the "Show …" box a link lands on.
about = open(os.path.join(HERE, "about.template.html")).read()
for k, v in [
    ("__THEMEJS__", THEMEJS),
    ("__NAV__", NAV("about")),
    ("__VERSION__", VERSION),
    ("__SITE__", SITE),
    ("__SITE__", SITE),
    ("__CSS__", CSS_LINK + "\n<style>\n" + MATHFACE + "\n</style>"),
    ("__FONTPRELOAD__", "\n".join(preloads)),
    ("__LOGO__", LOGO),
]:
    about = about.replace(k, v, 1)
assert not re.search(r"__[A-Z]+__", about), re.findall(r"__[A-Z]+__", about)
open(os.path.join(DIST, "about.html"), "w").write(about)
# the career planner (/career): the planner's settings, styles, data and engine, its own script and veteran data
career = open(os.path.join(HERE, "career.template.html")).read()
career = panels(career)  # the shared setup rows, Enhance buttons and settings editors
for k, v in [
    ("__THEMEJS__", THEMEJS),
    ("__NAV__", NAV("career")),
    ("__VERSION__", VERSION),
    ("__SITE__", SITE),
    ("__SITE__", SITE),
    ("__CSS__", CSS_LINK),
    ("__APP__", CAREER_JS),
    ("__DATAJS__", DATA_JS),
    ("__CAREERDATAJS__", CAREER_DATA_JS),
    ("__ASSETS__", js(ASSETS)),
    ("__ASSETS__", js(ASSETS)),
    ("__FONTPRELOAD__", "\n".join(preloads) + "\n" + script_preloads(DATA_JS, CAREER_DATA_JS, CAREER_JS)),
    ("__WORKER__", worker),
    ("__LOGO__", LOGO),
]:
    assert "</script" not in v.lower(), k
    career = career.replace(k, v, 1)
assert not re.search(r"__(?!PURE__)[A-Z]+__", career), re.findall(r"__(?!PURE__)[A-Z]+__", career)
open(os.path.join(DIST, "career.html"), "w").write(career)
for f in ("favicon.ico", "favicon.svg", "favicon-32.png", "apple-touch-icon.png", "og.png"):
    shutil.copyfile(os.path.join(HERE, "icons", f), os.path.join(DIST, f))
# hosting (Cloudflare Pages reads _headers): indexable (the page names its canonical address); the page cached 5 minutes so a
# new deploy shows quickly; fingerprinted assets a year, immutable (their names change with their content)
open(os.path.join(DIST, "_headers"), "w").write(
    "/*\n  Cache-Control: public, max-age=300\n"
    "/assets/*\n  ! Cache-Control\n  Cache-Control: public, max-age=31536000, immutable\n"
)
open(os.path.join(DIST, "robots.txt"), "w").write("User-agent: *\nAllow: /\n")
# /loop is the loop planner too (the header calls it "Loop planner"; Cloudflare Pages reads _redirects)
open(os.path.join(DIST, "_redirects"), "w").write("/loop / 301\n/loop/ / 301\n")
print(
    f"{VERSION}: dist/index.html {len(html.encode()) / 1e6:.2f} MB, solver data {len(global_era.engine.encode()) / 1e6:.2f} MB, loopopt.wasm {len(wasm) / 1e6:.2f} MB, GM era {(len(gm_ui.encode()) + len(gm_era.engine.encode())) / 1e6:.2f} MB, {image_catalog.base} {len(image_catalog.keys)} files {image_catalog.size_bytes / 1e6:.2f} MB, {len(faces)} font files"
)
