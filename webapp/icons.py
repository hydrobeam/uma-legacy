"""Favicon files from webapp/icons/favicon.svg (drawn for the site, no game art): three sparkles styled on the URA Finale
logo's, a little off kilter, one per spark type in the game's own spark colours (stat blue, aptitude pink, unique green;
atlas/singlecommon utx_frm_factor_s_02..04 and the atlas/common utx_ico_factor_* badges):
favicon.ico (16/32/48), favicon-32.png, apple-touch-icon.png (180, opaque on the page background; iOS turns
transparency black), og.png (512x512 link-preview thumbnail). Renders through icons.mjs (Chromium via Playwright). usage: uv run webapp/icons.py"""

import os, subprocess
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ICON_DIR = os.path.join(HERE, "icons")
subprocess.run(["node", os.path.join(HERE, "icons.mjs")], check=True)
images_by_size = {
    size: Image.open(os.path.join(ICON_DIR, f"_{size}.png")).convert("RGBA") for size in (16, 32, 48, 180)
}
images_by_size[48].save(
    os.path.join(ICON_DIR, "favicon.ico"),
    sizes=[(16, 16), (32, 32), (48, 48)],
    append_images=[images_by_size[16], images_by_size[32]],
)
images_by_size[32].save(os.path.join(ICON_DIR, "favicon-32.png"))
background = Image.new("RGBA", (180, 180), (0xF3, 0xF5, 0xF8, 255))
background.alpha_composite(images_by_size[180].resize((150, 150), Image.LANCZOS), (15, 17))
background.convert("RGB").save(os.path.join(ICON_DIR, "apple-touch-icon.png"))
Image.open(os.path.join(ICON_DIR, "_og.png")).convert("RGB").save(
    os.path.join(ICON_DIR, "og.png"), optimize=True
)  # link previews
os.remove(os.path.join(ICON_DIR, "_og.png"))
for size in images_by_size:
    os.remove(os.path.join(ICON_DIR, f"_{size}.png"))
print({f: os.path.getsize(os.path.join(ICON_DIR, f)) for f in sorted(os.listdir(ICON_DIR))})
