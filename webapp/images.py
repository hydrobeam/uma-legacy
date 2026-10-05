"""Card art, character icons, skill icons, race banners and scenario logos for the page, taken from the game's locally
decrypted asset bundles.

Collects every uma, outfit and support card id the page can show (both eras' exports), finds its bundle, decodes the
texture with UnityPy and writes a small AVIF (quality 55) to webapp/img/{kind}/{id}.avif (UI sprites: lossless WebP),
plus img/manifest.json (the bundle each image came from, and every id with no bundle). Images already extracted are
kept unless --force; an older WebP of the same image is removed when its AVIF is written.

  uma      chara/chr{chara}/chr_icon_{chara}                 round default icon ("any outfit")
  outfit   chara/chr{chara}/chr_icon_{chara}_{dress}_01      dress = card_rarity_data.race_dress_id of the outfit
                                                             card at its highest rarity (100102 wears dress 100130)
  The round uma icon is cropped to a square inside its ring; the crop starts from the visible area (alpha bounding
  box) and keeps the top. Outfit icons are cut to a fixed square inside their frame (OUTFIT_BOX).
  card     supportcard/support{id}/support_card_s_{id}       card thumbnail with its rarity frame
  race     race/racetitle/thumb/thum_race_rt_000_{race}_00   race banner (title art, 512x256), for the schedule view
  ui       UI sprites, lossless: the friend-slot frame (atlas/singlecommon utx_frm_friend_support_00_sl, a 9-slice for
           CSS border-image) and the support-type icons (atlas/common utx_ico_obtain_00..06: speed, stamina, power,
           guts, wit, pal, group)
  skill    outgame/skillicon/utx_ico_skill_{icon}            skill icon of each catalogue skill's white version
                                                             (skill_data.icon_id, lowest rarity in the group) and of
                                                             its gold (rarity 2); img/skill_icons.json maps skill
                                                             group -> white icon, img/skill_icons_gold.json -> gold
  scen     single/banner/bnr_{image:03d}_text                scenario logo (the scenario picker's title art, Global:
                                                             1 URA Finale, 2 Unity Cup, 3 Grand Concert, 4 Trackblazer,
                                                             5 Grand Masters (not decrypted locally yet);
                                                             image = single_mode_scenario.scenario_image_id). The
                                                             512x512 texture is shown 1.25x as wide as tall in the
                                                             game (SCEN_ASPECT), so it is widened to 640x512 first,
                                                             then cropped to its visible area and fitted in 288x160

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
SIZE = {'uma': 96, 'outfit': 192, 'card': 192, 'skill': 48, 'race': (256, 128), 'scen': (288, 160)}   # 2x the largest display (race banners ~128x64; scenario logos fit 144x80)
# the scenario logos' textures are square, but the game draws them 1.25x as wide as tall (resources.assets:
# PartsSingleModeStartScenarioLogo/bnr_00N RawImage 642x514, DialogSingleModeStartScenarioDetail/scenarioImage 446x355);
# the square texture alone looks squeezed (round letters come out tall)
SCEN_ASPECT = 1.25
INSET = {'uma': 26}   # px of the 256 px icon cut inside the visible area: the white ring
# Outfit icons (chr_icon_{chara}_{dress}_01) are a portrait in a silver frame that sits at the same place in every
# texture (outer edge x 10-245, y 34/35-248/249; inner picture x 23-232, y 47-235, checked on all 137 icons); ears and
# hair often stick out above it by 0-20 px. Cropping from the visible area kept the frame's top edge at a different
# height per uma, with the page's tile colour showing through above it. A fixed square inside the frame is the same
# for every icon: the picture and its striped background only.
OUTFIT_BOX = (34, 47, 222, 235)
ERAS = [('pkg/loopopt', 'pkg/loopopt/export/master.mdb'), ('jp', 'jp/export/master.mdb')]


def wanted():
    """{kind: {id: chara}} over both eras, plus the catalogue's skill groups."""
    w = {'uma': {}, 'outfit': {}, 'card': {}, 'race': {}, 'groups': set()}
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
        w['groups'].update(g for g, *_ in skills['skills'])
        for r in data['races']: w['race'][r] = None   # thumbnail id = race id for every G1 (checked against master.mdb)
    w['scen'] = scenario_images()
    return w


def scenario_images():
    """{scenario id: its logo's image id} for the page's five scenarios (single_mode_scenario: 1 URA Finale, 2 Unity Cup,
    3 Grand Concert, 4 Trackblazer, 5 Grand Masters, the last offered with the JP Grand Masters-era data; text_data
    category 119 names them in that order)."""
    path = os.path.join(R, ERAS[0][1]); ids = {1: 1, 2: 2, 3: 3, 4: 4, 5: 5}
    if os.path.exists(path):
        con = sqlite3.connect(f'file:{path}?mode=ro', uri=True)
        ids.update((i, img) for i, img in con.execute('select id, scenario_image_id from single_mode_scenario where id <= 5'))
    return ids


def skill_icons(groups):
    """({group: icon id of its white (lowest-rarity) skill}, {group: icon id of its gold (rarity 2)}), Global master.mdb
    first, then JP."""
    out, gold = {}, {}
    for _, mdb in ERAS:
        path = os.path.join(R, mdb)
        if not os.path.exists(path): continue
        con = sqlite3.connect(f'file:{path}?mode=ro', uri=True)
        for g in groups:
            if g not in out:
                row = con.execute('select icon_id from skill_data where group_id = ? order by rarity, id limit 1', (g,)).fetchone()
                if row: out[g] = row[0]
            if g not in gold:
                row = con.execute('select icon_id from skill_data where group_id = ? and rarity = 2 order by id limit 1', (g,)).fetchone()
                if row: gold[g] = row[0]
    return out, gold


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


UI_SPRITES = {'friend_frame': ('atlas/singlecommon', 'utx_frm_friend_support_00_sl'),
              # support-card type icons, shown on the art's top-right corner as in the game (atlas/common utx_ico_obtain_00..06)
              **{'type_' + k: ('atlas/common', f'utx_ico_obtain_0{i}') for i, k in enumerate(['speed', 'stamina', 'power', 'guts', 'wit', 'pal', 'group'])}}


def ui_sprites(force, manifest):
    os.makedirs(os.path.join(OUT, 'ui'), exist_ok=True); n = 0
    for key, (atlas, name) in UI_SPRITES.items():
        dst = os.path.join(OUT, 'ui', key + '.webp')
        if os.path.exists(dst) and not force: manifest['images']['ui:' + key] = f'{atlas} {name}'; continue
        for f in (atlas + '/' + atlas.split('/')[-1], atlas + '/' + atlas.split('/')[-1] + '_tex'):
            path = os.path.join(ASSETS, f)
            if not os.path.exists(path): continue
            for obj in UnityPy.load(path).objects:
                if obj.type.name == 'Sprite' and obj.read().m_Name == name:
                    obj.read().image.convert('RGBA').save(dst, 'WEBP', lossless=True); manifest['images']['ui:' + key] = f'{atlas} {name}'; n += 1; break
            if os.path.exists(dst): break
        if not os.path.exists(dst): manifest['missing']['ui:' + key] = f'{atlas} {name}'
    return n


def texture(path):
    for obj in UnityPy.load(path).objects:
        if obj.type.name == 'Texture2D':
            return obj.read().image.convert('RGBA')
    return None


def main(force):
    if not os.path.isdir(ASSETS):
        sys.exit(f'no decrypted asset folder at {ASSETS} (set UMA_DECRYPTED)')
    w = wanted(); dress = dress_ids(w['outfit'])
    icons, gold = skill_icons(sorted(w.pop('groups')))
    w['skill'] = {i: None for i in sorted(set(icons.values()) | set(gold.values()))}
    json.dump({str(g): i for g, i in sorted(icons.items())}, open(os.path.join(OUT, 'skill_icons.json'), 'w'), separators=(',', ':'))
    json.dump({str(g): i for g, i in sorted(gold.items())}, open(os.path.join(OUT, 'skill_icons_gold.json'), 'w'), separators=(',', ':'))
    manifest = {'source': 'game asset bundles (decrypted locally); images (c) Cygames', 'images': {}, 'missing': {}}
    done = skipped = 0
    for kind, ids in w.items():
        os.makedirs(os.path.join(OUT, kind), exist_ok=True)
        for i, chara in sorted(ids.items()):
            if kind == 'uma': rel = f'chara/chr{i}/chr_icon_{i}'
            elif kind == 'outfit':
                if i not in dress: manifest['missing'][f'{kind}:{i}'] = 'no dress id in either master.mdb'; continue
                rel = f'chara/chr{chara}/chr_icon_{chara}_{dress[i]}_01'
            elif kind == 'skill': rel = f'outgame/skillicon/utx_ico_skill_{i}'
            elif kind == 'race': rel = f'race/racetitle/thumb/thum_race_rt_000_{i}_00'
            elif kind == 'scen': rel = f'single/banner/bnr_{chara:03d}_text'   # chara holds the scenario's image id
            else: rel = f'supportcard/support{i}/support_card_s_{i}'
            dst = os.path.join(OUT, kind, f'{i}.avif')
            src = os.path.join(ASSETS, rel)
            if not os.path.exists(src): manifest['missing'][f'{kind}:{i}'] = rel; continue
            manifest['images'][f'{kind}:{i}'] = rel
            if os.path.exists(dst) and not force: skipped += 1; continue
            im = texture(src)
            if im is None: manifest['missing'][f'{kind}:{i}'] = rel + ' (no texture)'; del manifest['images'][f'{kind}:{i}']; continue
            if kind in INSET: im = icon_crop(im, INSET[kind])
            elif kind == 'outfit': im = im.crop(OUTFIT_BOX)
            if kind == 'scen':   # the game's width (SCEN_ASPECT), then the visible area, fitted to the box with its aspect kept
                im = im.resize((round(im.width * SCEN_ASPECT), im.height), Image.LANCZOS)
                im = im.crop(im.getchannel('A').getbbox()); im.thumbnail(SIZE[kind], Image.LANCZOS); im.save(dst, 'AVIF', quality=55, speed=6)
            else:
                sz = SIZE[kind] if isinstance(SIZE[kind], tuple) else (SIZE[kind], SIZE[kind])
                im.resize(sz, Image.LANCZOS).save(dst, 'AVIF', quality=55, speed=6)
            old = dst[:-5] + '.webp'   # the earlier WebP of the same image
            if os.path.exists(old): os.remove(old)
            done += 1
    done += ui_sprites(force, manifest)
    json.dump(manifest, open(os.path.join(OUT, 'manifest.json'), 'w'), indent=1, sort_keys=True)
    n = sum(len(v) for v in w.values())
    print(f'{n} ids: {done} extracted, {skipped} kept, {len(manifest["missing"])} missing (see img/manifest.json)')


if __name__ == '__main__':
    main('--force' in sys.argv)
