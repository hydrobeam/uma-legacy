"""Self-hosted font subsets for the page -> webapp/fonts/ (committed, SIL Open Font License; licences alongside).

The page uses Zen Kaku Gothic New (UI text; 400/500/700/900) and IBM Plex Mono (numbers; 400/500/600). Google Fonts
served them through a 117 KB render-blocking stylesheet (the Japanese font is split into ~120 subsets). Here each
weight gets:
  - a Latin subset: Basic Latin, Latin-1, general punctuation, arrows, plus every other non-CJK character the page's
    template, scripts and data contain (☆ ♪ ○ ∞ ...); loaded on every visit;
  - for Zen Kaku Gothic New, a CJK subset with the kana/kanji the data contain (only the JP Grand Masters-era data has
    any: untranslated names), declared with unicode-range so a browser fetches it only when such text is on screen.
fonts.json lists the files and their unicode ranges for build.py, which fingerprints them and writes the @font-face
rules. Sources: the TTFs in github.com/google/fonts (ofl/zenkakugothicnew, ofl/ibmplexmono), downloaded to
cache/fonts/ on first run. Rerun after a data update that adds new characters (build.py warns when one is missing).
usage: uv run webapp/fonts.py"""
import glob, json, os, subprocess, sys, urllib.request
H = os.path.dirname(os.path.abspath(__file__)); R = os.path.dirname(H)
SRC = os.path.join(R, 'cache', 'fonts'); OUT = os.path.join(H, 'fonts')
FONTS = [('Zen Kaku Gothic New', 400, 'zenkakugothicnew/ZenKakuGothicNew-Regular'), ('Zen Kaku Gothic New', 500, 'zenkakugothicnew/ZenKakuGothicNew-Medium'),
         ('Zen Kaku Gothic New', 700, 'zenkakugothicnew/ZenKakuGothicNew-Bold'), ('Zen Kaku Gothic New', 900, 'zenkakugothicnew/ZenKakuGothicNew-Black'),
         ('IBM Plex Mono', 400, 'ibmplexmono/IBMPlexMono-Regular'), ('IBM Plex Mono', 500, 'ibmplexmono/IBMPlexMono-Medium'),
         ('IBM Plex Mono', 600, 'ibmplexmono/IBMPlexMono-SemiBold')]
LICENSES = {'OFL-ZenKakuGothicNew.txt': 'zenkakugothicnew', 'OFL-IBMPlexMono.txt': 'ibmplexmono'}
BASE = [(0x20, 0x7e), (0xa0, 0xff), (0x2010, 0x2027), (0x2030, 0x205e), (0x2190, 0x21ff)]
cjk = lambda c: 0x2e80 <= c <= 0x9fff or 0xf900 <= c <= 0xfaff or 0xff00 <= c <= 0xffef


def page_chars():
    """every character in the page's template, scripts and both eras' displayed data"""
    files = [os.path.join(H, 'index.template.html')] + glob.glob(os.path.join(H, 'src', '*.js'))
    for era in ('pkg/loopopt', 'jp'):
        files += [os.path.join(R, era, f) for f in ('web_cards.json', 'web_skills.json', 'data.json', 'card_events.json')]
    chars = set()
    for f in files:
        if os.path.exists(f): chars |= {ord(c) for c in open(f, encoding='utf-8').read()}
    return chars


def ranges(cps):
    """code points -> 'U+20-7E,U+A0-FF,...'"""
    cps = sorted(cps); out = []; i = 0
    while i < len(cps):
        j = i
        while j + 1 < len(cps) and cps[j + 1] == cps[j] + 1: j += 1
        out.append(f'U+{cps[i]:X}' + (f'-{cps[j]:X}' if j > i else '')); i = j + 1
    return ','.join(out)


def main():
    os.makedirs(SRC, exist_ok=True); os.makedirs(OUT, exist_ok=True)
    for _, _, rel in FONTS:
        dst = os.path.join(SRC, os.path.basename(rel) + '.ttf')
        if not os.path.exists(dst): urllib.request.urlretrieve(f'https://github.com/google/fonts/raw/main/ofl/{rel}.ttf', dst)
    for name, d in LICENSES.items():
        dst = os.path.join(OUT, name)
        if not os.path.exists(dst): urllib.request.urlretrieve(f'https://github.com/google/fonts/raw/main/ofl/{d}/OFL.txt', dst)
    chars = page_chars()
    latin = {c for a, b in BASE for c in range(a, b + 1)} | {c for c in chars if c > 0x7e and not cjk(c) and c not in (0xfe0e, 0xfe0f)}
    kanji = {c for c in chars if cjk(c)}
    manifest = []
    for fam, w, rel in FONTS:
        src = os.path.join(SRC, os.path.basename(rel) + '.ttf'); short = 'zen' if fam.startswith('Zen') else 'plex'
        for part, cps in (('latin', latin), ('cjk', kanji if short == 'zen' else set())):
            if not cps: continue
            out = os.path.join(OUT, f'{short}-{w}-{part}.woff2')
            subprocess.run([sys.executable, '-m', 'fontTools.subset', src, '--unicodes=' + ranges(cps), '--flavor=woff2', '--layout-features=*',
                            '--no-hinting', '--desubroutinize', '--output-file=' + out], check=True)
            manifest.append({'family': fam, 'weight': w, 'file': os.path.basename(out), 'range': ranges(cps), 'preload': short == 'zen' and part == 'latin' and w in (400, 700)})
            print(f'{os.path.basename(out):24s} {os.path.getsize(out) / 1e3:6.1f} KB')
    json.dump({'chars': sorted(chars), 'fonts': manifest}, open(os.path.join(OUT, 'fonts.json'), 'w'), separators=(',', ':'))


if __name__ == '__main__':
    main()
