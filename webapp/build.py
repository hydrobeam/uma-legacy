"""Assemble the web app in dist/: index.html (the career planner, the front page), loop.html (the loop planner: its
template, src/style.css, src/*.js bundled by esbuild, the page's data, skill catalog, precomputed example), about.html and assets/: the solver (loopopt.wasm), its data per era, the GM era's page data,
images (character icons, card art, skill icons, race banners, scenario logos, UI sprites from images.py) and fonts
(the Fontsource packages' Latin subsets). Every file in assets/ carries a fingerprint of its content in its name (images: one fingerprint over the
whole set, as a folder name), so it is served with a one-year immutable cache and an update simply gets new names;
the pages keep a 5-minute cache and always point at the current names. Favicons (icons.py) sit at the root.
Served over http(s): the page fetches the wasm, data and images, so it is not meant to be opened as a file."""

import json, subprocess, os, shutil, hashlib, re, io, string
from html import unescape
import brotli
from dataclasses import dataclass
from typing import NamedTuple

HERE = os.path.dirname(os.path.abspath(__file__))
GLOBAL_DATA = os.path.join(HERE, "..", "pkg", "loopopt", "data", "global")
JP_DATA = os.path.join(HERE, "..", "pkg", "loopopt", "data", "jp")
wasm = open(os.path.join(HERE, "loopopt.opt.wasm"), "rb").read()
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
    for kind in ("uma", "mob", "outfit", "vet", "card", "skill", "race", "scen", "scenart", "ui", "run"):
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
# the page's lists, exported from the game data by pkg/loopopt/export/export_page.py (./build.sh data, events, gametora
# and pb run it): each era's page.json, and page_raceids.json, the race id of each race name and chart name (both eras)
PAGE_DATA = os.path.join(HERE, "..", "pkg", "loopopt", "data")
race_ids = json.load(open(os.path.join(PAGE_DATA, "page_raceids.json")))


def js(o):
    return json.dumps(o, ensure_ascii=False, separators=(",", ":")).replace("</", "<\\/")


# each scenario's training-hint base (hints per card per run at hint frequency 0 and specialty 0), by the page's
# scenario key: its key in research/results.json's `values` (section 7's fit). Grand Masters has no runs: it takes the
# fit of one base over every other scenario's. The keys are pkg/loopopt/build.rs's, so the page shows the solver's base.
HINT_BASE_KEYS = {
    "unity": "7.specialty_base_unity_cup",
    "gc": "7.specialty_base_grand_concert",
    "ura": "7.specialty_base_ura_finale",
    "trackblazer": "7.specialty_base_trackblazer",
    "grandmasters": "7.one_base_and_one_d_base",
}


def research_value(key):
    """research/results.json's `values."{key}"` rounded to 3 decimals, as pkg/loopopt/build.rs (the solver's source)
    takes it. A missing or non-numeric key stops the build, naming it."""
    value = json.load(open(os.path.join(HERE, "..", "research", "results.json")))["values"].get(key)
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise SystemExit(
            f'research/results.json: `values."{key}"` is missing or not a number; `./build.sh research` computes it'
        )
    return round(value, 3)


def hint_bases():
    """the scenarios' training-hint bases by scenario key (`research_value`)"""
    return {scenario: research_value(key) for scenario, key in HINT_BASE_KEYS.items()}


# /about's statement of the career planner's robustness check: the all-target obs/pred ratio's 95% interval ends
# (pkg/loopopt/build.rs `plan_constants`, the same keys and rounding)
P_ALL_RATIO_ENDS = {
    "PALLLOW": "13.p_all_eight_ratio_all_95_pct_low",
    "PALLHIGH": "13.p_all_eight_ratio_all_95_pct_high",
}


def low_basis():
    """the career planner's low-basis cost, as pkg/loopopt/build.rs (the solver's source) reads it from
    research/results.json: (extra cost in points, the bucket's bound) from `4.low_basis_cost_fitted_on_every_run` (whole
    points) and `4.low_aptitude_basis_bucket` ("<=B"). Anything else stops the build, naming the key."""
    values = json.load(open(os.path.join(HERE, "..", "research", "results.json")))["values"]
    cost_key, bucket_key = "4.low_basis_cost_fitted_on_every_run", "4.low_aptitude_basis_bucket"
    cost, bucket = values.get(cost_key), values.get(bucket_key)
    if isinstance(cost, bool) or not isinstance(cost, (int, float)) or cost != int(cost) or cost < 0:
        raise SystemExit(f'research/results.json: `values."{cost_key}"` is {cost!r}, not whole points')
    if not isinstance(bucket, str) or not re.fullmatch(r"<=-?\d+", bucket):
        raise SystemExit(f'research/results.json: `values."{bucket_key}"` is {bucket!r}, not "<=" and a whole basis')
    return int(cost), int(bucket[2:])


@dataclass
class EraBundle:
    """JSON strings embedded in page scripts or written as engine assets."""

    cards: str
    umas: str
    skills: str
    engine: str
    career: str


def page_lists(data_dir):
    """The era's page.json: the card picker's cards, the roster, the skill catalog and the career page's tables. Its
    `inputs` (each file it was made from, with its sha256) must match the files now, or the build stops; one under game/
    that is absent is skipped, so a checkout without the game files builds from the exported lists."""
    page = json.load(open(os.path.join(data_dir, "page.json")))
    for path, digest in page["inputs"].items():
        source = os.path.join(HERE, "..", path)
        if path.startswith("game/") and not os.path.exists(source):
            continue
        if not os.path.exists(source) or hashlib.sha256(open(source, "rb").read()).hexdigest() != digest:
            era_name = os.path.basename(os.path.normpath(data_dir))
            raise SystemExit(
                f"{path} changed since pkg/loopopt/data/{era_name}/page.json was made: run ./build.sh data"
            )
    return page


def era(data_dir):
    page = page_lists(data_dir)
    export = lambda name: json.load(open(os.path.join(data_dir, name)))
    # the solver's data, as text: the era's exports themselves
    engine = js(
        {
            "data": export("data.json"),
            "skills": export("web_skills.json"),
            "cards": export("web_cards.json"),
            "events": export("card_events.json"),
        }
    )
    career = page["career"]
    # only mobs with a portrait (the samples draw from them)
    career["mobs"] = [mob for mob in career["mobs"] if f"mob:{mob[0]}" in image_catalog.keys]
    return EraBundle(
        cards=js(page["cards"]), umas=js(page["umas"]), skills=js(page["skills"]), engine=engine, career=js(career)
    )


global_era = era(GLOBAL_DATA)
gm_era = era(JP_DATA)  # GM era: pal first-year date events left out of its card_events.json, as in Global
# the page scripts: src/loop/main.tsx, src/career/main.js and the modules they import, each bundled into one minified IIFE (esbuild
# from node_modules: npm ci); also the solver's worker script, src/worker.ts, minified and wrapped the same way
# (JSX compiles to Preact's jsx() calls, imported from preact/jsx-runtime)
ESBUILD = os.path.join(HERE, "node_modules", ".bin", "esbuild")
if not os.path.exists(ESBUILD):
    raise SystemExit("esbuild missing: run npm ci in webapp/")


def bundle(entry):  # entry: relative to webapp/
    return subprocess.run(
        [
            ESBUILD,
            os.path.join(HERE, entry),
            "--bundle",
            "--format=iife",
            "--charset=utf8",
            "--jsx=automatic",
            "--jsx-import-source=preact",
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


app = bundle("src/loop/main.tsx")
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
    # the solver's worker (engine.js `new Worker(ASSETS.worker)`)
    "worker": fingerprint("worker.js", bundle("src/worker.ts")),
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
    "gauge_bond",
    "friend_frame",
    "spark_white",
    "spark_blue",
    "spark_pink",
    "spark_green",
    "spark_dot",
    "spark_dot_gold",
    "tag_blue",
    "tag_pink",
    "tag_green",
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
# /about's formulas (MathML): STIX Two Math, subset to what the built page's <math> elements hold (the whole face is
# 403 KB; this, about 33 KB), its MATH table kept with the glyphs it reaches (stretched brackets, big operators). MathML
# draws a single-letter <mi> in math italic, so the italic Latin and Greek letters, ℎ and the invisible operators are
# kept too. The browser fetches it only on a page that draws math, so only /about gets the face.
MATH_EXTRA = [*range(0x1D434, 0x1D468), 0x210E, *range(0x1D6E2, 0x1D71C), *range(0x2061, 0x2065)]


def math_face(page):
    from fontTools import subset

    d = os.path.join(HERE, "node_modules", "@fontsource", "stix-two-math")
    formulas = re.findall(r"<math\b.*?</math>", page, re.S)
    chars = {ord(c) for f in formulas for c in unescape(re.sub(r"<[^>]+>", "", f))} | set(MATH_EXTRA)
    options = subset.Options(layout_features=["*"], flavor="woff2")
    font = subset.load_font(os.path.join(d, "files", "stix-two-math-latin-400-normal.woff2"), options)
    subsetter = subset.Subsetter(options)
    subsetter.populate(unicodes=sorted(chars))
    subsetter.subset(font)
    out = io.BytesIO()
    subset.save_font(font, out, options)
    url = fingerprint("stix-two-math.woff2", out.getvalue())
    face = open(os.path.join(d, "latin-400.css")).read()
    return re.sub(r"src: [^;]+;", f"src: url({url}) format('woff2');", face).strip()


# the planner pages' shared data (the Global era's lists, image keys, skill icons, race ids, the scenarios' hint bases),
# the career page's tables and each page's script: fingerprinted classic scripts at the end of the body, data first.
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
        "data-hintbase": js(hint_bases()),
    },
)
CAREER_DATA_JS = data_script("career-data.js", {"data-career": global_era.career})
MAIN_JS, CAREER_JS = fingerprint("main.js", app), fingerprint("career.js", bundle("src/career/main.js"))


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
PAGES = [("career", "./", "Career planner"), ("loop", "loop", "Loop planner"), ("about", "about", "How this works")]


class Page(string.Template):
    """A page template's markers, `__NAME__` (capitals and digits; `__PANEL_X__` are filled before, by `panels`): one
    pass, so inserted text is never searched again; `$`, which the pages' scripts use, stays literal."""

    delimiter = "__"
    pattern = r"__(?:(?P<named>[A-Z][A-Z0-9]*)__|(?P<escaped>(?!))|(?P<braced>(?!))|(?P<invalid>(?!)))"


def fill(page, values):
    """`page` with every marker filled from `values`: a marker without a value, or a value without its marker, stops the
    build; no value may close the page's inline script early."""
    named = {m.group("named") for m in Page.pattern.finditer(page) if m.group("named")}
    assert named == set(values), (sorted(named - set(values)), sorted(set(values) - named))
    for name, value in values.items():
        assert "</script" not in value.lower(), name
    return Page(page).substitute(values)


def NAV(cur):
    tabs = "".join(f'<a href="{h}"' + (' aria-current="page"' if k == cur else "") + f">{t}</a>" for k, h, t in PAGES)
    return f'<nav class="tnav" aria-label="Pages"><div class="ptabs">{tabs}</div> {THEMEBTN}</nav>'


# the logo: the favicon's three sparkles inline in the title (gradient ids prefixed so they can't clash)
LOGO = re.sub(r'\bid="g(\d)"', r'id="lg\1"', open(os.path.join(HERE, "icons", "favicon.svg")).read()).replace(
    "url(#g", "url(#lg"
)
LOGO = LOGO.replace("<svg ", '<svg class="logo" aria-hidden="true" focusable="false" ', 1).strip()
html = fill(
    html,
    {
        "THEMEJS": THEMEJS,
        "NAV": NAV("loop"),
        "VERSION": VERSION,
        "SITE": SITE,
        "CSS": CSS_LINK,
        "APP": MAIN_JS,
        "DATAJS": DATA_JS,
        "EXAMPLE": js(example),
        "ASSETS": js(ASSETS),
        "FONTPRELOAD": "\n".join(preloads) + "\n" + script_preloads(DATA_JS, MAIN_JS),
        "LOGO": LOGO,
    },
)
open(os.path.join(DIST, "loop.html"), "w").write(html)
# "How this works" (/about; Cloudflare Pages serves about.html there): shared styles and fonts.
# Its disclosure script opens the "Show …" box a link lands on.
about = open(os.path.join(HERE, "about.template.html")).read()
LOW_BASIS_COST, LOW_BASIS_MAX = low_basis()
about_values = {
    "LOWBASISCOST": str(LOW_BASIS_COST),
    "LOWBASISMAX": str(LOW_BASIS_MAX),
    "THEMEJS": THEMEJS,
    "NAV": NAV("about"),
    "VERSION": VERSION,
    "SITE": SITE,
    "FONTPRELOAD": "\n".join(preloads),
    "LOGO": LOGO,
    **{name: f"{research_value(key):.3f}" for name, key in P_ALL_RATIO_ENDS.items()},
}
# (the math face is subset to the filled page's formulas, so the page is filled once without it to read them)
math_css = math_face(fill(about, {**about_values, "CSS": ""}))
about = fill(about, {**about_values, "CSS": CSS_LINK + "\n<style>\n" + math_css + "\n</style>"})
open(os.path.join(DIST, "about.html"), "w").write(about)
# the career planner (/career): the planner's settings, styles, data and engine, its own script and veteran data
career = open(os.path.join(HERE, "career.template.html")).read()
career = panels(career)  # the shared setup rows, Enhance buttons and settings editors
career = fill(
    career,
    {
        "THEMEJS": THEMEJS,
        "NAV": NAV("career"),
        "VERSION": VERSION,
        "SITE": SITE,
        "CSS": CSS_LINK,
        "APP": CAREER_JS,
        "DATAJS": DATA_JS,
        "CAREERDATAJS": CAREER_DATA_JS,
        "ASSETS": js(ASSETS),
        "FONTPRELOAD": "\n".join(preloads) + "\n" + script_preloads(DATA_JS, CAREER_DATA_JS, CAREER_JS),
        "LOGO": LOGO,
    },
)
open(os.path.join(DIST, "index.html"), "w").write(career)
for f in ("favicon.ico", "favicon.svg", "favicon-32.png", "apple-touch-icon.png", "og.png"):
    shutil.copyfile(os.path.join(HERE, "icons", f), os.path.join(DIST, f))
# hosting (Cloudflare Pages reads _headers): indexable (the page names its canonical address); the page cached 5 minutes so a
# new deploy shows quickly; fingerprinted assets a year, immutable (their names change with their content)
open(os.path.join(DIST, "_headers"), "w").write(
    "/*\n  Cache-Control: public, max-age=300\n"
    "/assets/*\n  ! Cache-Control\n  Cache-Control: public, max-age=31536000, immutable\n"
)
open(os.path.join(DIST, "robots.txt"), "w").write("User-agent: *\nAllow: /\n")
# the career planner is the front page (index.html), the loop planner /loop (loop.html); an old /career link lands on
# the front page (a temporary redirect: Cloudflare Pages reads _redirects)
open(os.path.join(DIST, "_redirects"), "w").write("/career / 302\n/career/ / 302\n")
print(
    f"{VERSION}: GM era {(len(gm_ui.encode()) + len(gm_era.engine.encode())) / 1e6:.2f} MB, {image_catalog.base} {len(image_catalog.keys)} files {image_catalog.size_bytes / 1e6:.2f} MB, {len(faces)} font files"
)


# What a first visit to each page fetches before it can work: the page, every local file it names (scripts, the
# stylesheet, preloaded fonts, its inline style's fonts), and on the planner pages the solver's worker, the solver and its
# Global data (the page starts the solver at once). Raw and at brotli quality 4, which reproduces the host's transfer of the wasm to
# within 0.1% (measured against v50's deploy, 2026-10-31). Checked against webapp/budgets.json: a page over its budget
# fails the build; a budget is raised in the commit that needs it, saying why (art is left out: it loads as seen).
def page_cost(name, solver):
    page = open(os.path.join(DIST, name)).read()
    files = set(re.findall(r'(?:src|href)="(assets/[^"]+)"', page)) | set(re.findall(r"url\((assets/[^)]+)\)", page))
    if solver:
        files |= {ASSETS["wasm"], ASSETS["engine"]["global"], ASSETS["worker"]}
    blobs = [page.encode()] + [open(os.path.join(DIST, f), "rb").read() for f in sorted(files)]
    return sum(map(len, blobs)), sum(len(brotli.compress(b, quality=4)) for b in blobs)


budgets = json.load(open(os.path.join(HERE, "budgets.json")))
over = []
for name, solver in (("index.html", True), ("loop.html", True), ("about.html", False)):
    raw, sent = page_cost(name, solver)
    budget = budgets["pages_kb_sent"][name]
    print(f"  {name}: {raw / 1e3:,.0f} KB, about {sent / 1e3:,.0f} KB sent (budget {budget:,} KB)")
    if sent / 1e3 > budget:
        over.append(f"{name} sends about {sent / 1e3:,.0f} KB, over its budget of {budget:,} KB (webapp/budgets.json)")
assert not over, "; ".join(over)
