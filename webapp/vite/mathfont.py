"""/about's formulas (MathML) in STIX Two Math, subset to what the page's <math> elements hold (the whole face is 403
KB; this, about 33 KB), its MATH table kept with the glyphs it reaches (stretched brackets, big operators). MathML draws
a single-letter <mi> in math italic, so the italic Latin and Greek letters, ℎ and the invisible operators are kept too.
Run by vite/mathfont.mjs. usage: python mathfont.py PAGE.html FONT.woff2 > SUBSET.woff2"""

import argparse, io, re, sys
from html import unescape
from fontTools import subset

MATH_EXTRA = [*range(0x1D434, 0x1D468), 0x210E, *range(0x1D6E2, 0x1D71C), *range(0x2061, 0x2065)]

parser = argparse.ArgumentParser(description="/about's math font subset to its formulas' glyphs, as woff2 on stdout")
parser.add_argument("page", help="the page whose <math> elements the subset covers")
parser.add_argument("font", help="the whole face (woff2)")
args = parser.parse_args()
formulas = re.findall(r"<math\b.*?</math>", open(args.page, encoding="utf-8").read(), re.S)
chars = {ord(c) for f in formulas for c in unescape(re.sub(r"<[^>]+>", "", f))} | set(MATH_EXTRA)
options = subset.Options(layout_features=["*"], flavor="woff2")
font = subset.load_font(args.font, options)
subsetter = subset.Subsetter(options)
subsetter.populate(unicodes=sorted(chars))
subsetter.subset(font)
out = io.BytesIO()
subset.save_font(font, out, options)
sys.stdout.buffer.write(out.getvalue())
