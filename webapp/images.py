"""Card art and character icons for the page, taken from the game's locally decrypted asset bundles.

Collects every uma, outfit and support card id the page can show (both eras' exports), finds its bundle, decodes the
texture with UnityPy and writes a small WebP to webapp/img/{uma,outfit,card}/{id}.webp, plus img/manifest.json
(the bundle each image came from, and every id with no bundle). Images already extracted are kept unless --force.

  uma      chara/chr{chara}/chr_icon_{chara}                 round default icon ("any outfit")
  outfit   chara/chr{chara}/chr_icon_{chara}_{dress}_01      dress = card_rarity_data.race_dress_id of the outfit
                                                             card at its highest rarity (100102 wears dress 100130)
  Both icon kinds are cropped to a square inside their ring or frame (the page draws them as circles). The icon sits
  low in its 256 px texture, so the crop starts from the visible area (alpha bounding box) and keeps the top.
  card     supportcard/support{id}/support_card_s_{id}       card thumbnail with its rarity frame

Bundles: $UMA_DECRYPTED, default the Steam (Proton) install's LocalLow/Cygames/Umamusume/decrypted. They must already
be decrypted; this script only reads them. Dress ids come from the Global master.mdb, then the JP one for JP-only
outfits. usage: uv run webapp/images.py [--force]
"""
import io, json, os, sqlite3, sys
import UnityPy
from PIL import Image

H = os.path.dirname(os.path.abspath(__file__)); R = os.path.dirname(H)
ASSETS = os.environ.get('UMA_DECRYPTED', os.path.expanduser(
    '~/.local/share/Steam/steamapps/compatdata/3224770/pfx/drive_c/users/steamuser/AppData/LocalLow/Cygames/Umamusume/decrypted'))
OUT = os.path.join(H, 'img')
SIZE = 96                 # 2x the largest avatar on the page (38 px)
INSET = {'uma': 26, 'outfit': 22}   # px of the 256 px icon cut inside the visible area: the white ring / the frame
ERAS = [('pkg/loopopt', 'pkg/loopopt/export/master.mdb'), ('jp', 'jp/export/master.mdb')]


def wanted():
    """{kind: {id: chara}} over both eras."""
    w = {'uma': {}, 'outfit': {}, 'card': {}}
    for d, _ in ERAS:
        data = json.load(open(os.path.join(R, d, 'data.json')))
        skills = json.load(open(os.path.join(R, d, 'web_skills.json')))
        cards = json.load(open(os.path.join(R, d, 'web_cards.json')))
        for u in data['umas']:
            w['uma'][u['id']] = u['id']
            for o in skills['cards'].get(str(u['id']), []):
                w['outfit'][o[0]] = u['id']
        for c in cards['cards']:
            w['card'][c['id']] = c['chara']
    return w


def dress_ids(outfits):
    out = {}
    for _, mdb in ERAS:
        path = os.path.join(R, mdb)
        if not os.path.exists(path): continue
        con = sqlite3.connect(f'file:{path}?mode=ro', uri=True)
        for cid in outfits:
            if cid in out: continue
            row = con.execute('select race_dress_id from card_rarity_data where card_id = ? order by rarity desc limit 1', (cid,)).fetchone()
            if row: out[cid] = row[0]
    return out


def icon_crop(im, inset):
    x0, y0, x1, y1 = im.getchannel('A').getbbox()
    x0 += inset; y0 += inset; x1 -= inset
    side = min(x1 - x0, y1 - y0); left = (x0 + x1) // 2 - side // 2
    return im.crop((left, y0, left + side, y0 + side))


def texture(path):
    for obj in UnityPy.load(path).objects:
        if obj.type.name == 'Texture2D':
            return obj.read().image.convert('RGBA')
    return None


def main(force):
    if not os.path.isdir(ASSETS):
        sys.exit(f'no decrypted asset folder at {ASSETS} (set UMA_DECRYPTED)')
    w = wanted(); dress = dress_ids(w['outfit'])
    manifest = {'source': 'game asset bundles (decrypted locally); images (c) Cygames', 'images': {}, 'missing': {}}
    done = skipped = 0
    for kind, ids in w.items():
        os.makedirs(os.path.join(OUT, kind), exist_ok=True)
        for i, chara in sorted(ids.items()):
            if kind == 'uma': rel = f'chara/chr{i}/chr_icon_{i}'
            elif kind == 'outfit':
                if i not in dress: manifest['missing'][f'{kind}:{i}'] = 'no dress id in either master.mdb'; continue
                rel = f'chara/chr{chara}/chr_icon_{chara}_{dress[i]}_01'
            else: rel = f'supportcard/support{i}/support_card_s_{i}'
            dst = os.path.join(OUT, kind, f'{i}.webp')
            src = os.path.join(ASSETS, rel)
            if not os.path.exists(src): manifest['missing'][f'{kind}:{i}'] = rel; continue
            manifest['images'][f'{kind}:{i}'] = rel
            if os.path.exists(dst) and not force: skipped += 1; continue
            im = texture(src)
            if im is None: manifest['missing'][f'{kind}:{i}'] = rel + ' (no texture)'; del manifest['images'][f'{kind}:{i}']; continue
            if kind in INSET: im = icon_crop(im, INSET[kind])
            im.resize((SIZE, SIZE), Image.LANCZOS).save(dst, 'WEBP', quality=82, method=6)
            done += 1
    json.dump(manifest, open(os.path.join(OUT, 'manifest.json'), 'w'), indent=1, sort_keys=True)
    n = sum(len(v) for v in w.values())
    print(f'{n} ids: {done} extracted, {skipped} kept, {len(manifest["missing"])} missing (see img/manifest.json)')


if __name__ == '__main__':
    main('--force' in sys.argv)
