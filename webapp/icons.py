"""Favicon files from webapp/icons/favicon.svg (drawn for the site, no game art): three sparkles styled on the URA Finale
logo's, a little off kilter, one per spark type in the game's own spark colours (stat blue, aptitude pink, unique green;
atlas/singlecommon utx_frm_factor_s_02..04 and the atlas/common utx_ico_factor_* badges):
favicon.ico (16/32/48), favicon-32.png, apple-touch-icon.png (180, opaque on the page background; iOS turns
transparency black), og.png (1200x630 link-preview image). Renders through icons.mjs (Chromium via Playwright). usage: uv run webapp/icons.py"""
import os, subprocess
from PIL import Image
H = os.path.dirname(os.path.abspath(__file__)); I = os.path.join(H, 'icons')
subprocess.run(['node', os.path.join(H, 'icons.mjs')], check=True)
ims = {n: Image.open(os.path.join(I, f'_{n}.png')).convert('RGBA') for n in (16, 32, 48, 180)}
ims[48].save(os.path.join(I, 'favicon.ico'), sizes=[(16, 16), (32, 32), (48, 48)], append_images=[ims[16], ims[32]])
ims[32].save(os.path.join(I, 'favicon-32.png'))
bg = Image.new('RGBA', (180, 180), (0xf3, 0xf5, 0xf8, 255)); bg.alpha_composite(ims[180].resize((150, 150), Image.LANCZOS), (15, 17))
bg.convert('RGB').save(os.path.join(I, 'apple-touch-icon.png'))
Image.open(os.path.join(I, '_og.png')).convert('RGB').save(os.path.join(I, 'og.png'), optimize=True)   # link previews
os.remove(os.path.join(I, '_og.png'))
for n in ims: os.remove(os.path.join(I, f'_{n}.png'))
print({f: os.path.getsize(os.path.join(I, f)) for f in sorted(os.listdir(I))})
