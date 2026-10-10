"""The runner: each uma's mini model running, as one sprite strip per uma (webapp/img/run/{chara}.avif, 18 frames of
128 px side by side, a 0.6 s loop the page steps through with CSS: webapp/src/images.js runnerHTML).

From the game's own files (the decrypted bundles images.py reads, found by name through the asset index
game/global/meta.db): her mini body (mbdy{id}_00), hair (mchr{id}_00), the shared mini face (mchr0001_00_face0, her
skin tone's texture), her tail model with her texture (chara_data.tail_model_id, .skin from master.mdb), and the
game's mini run motion (3d/motion/mini/event/body/type00/anm_min_eve_type00_run00_loop). AssetStudio's CLI exports
the models to FBX, FBX2glTF makes glb; the run (a generic clip: dense and constant curves, its bones named by the
CRC32 of their path) is decoded here; webapp/chibi/render.mjs renders the frames (chibi.html, three.js); the strip is
cropped to her box over the loop and written as AVIF.

Third-party tools, fetched on first use into cache/chibi/tools (never committed): AssetStudioModCLI (aelurum/AssetStudio
v0.19.0, net9 linux64) with a .NET 9 runtime, FBX2glTF 0.9.7, and libxml2.so.2 (Arch's libxml2-legacy) when the system
has only the newer libxml2.

usage: uv run webapp/chibi/chibi.py [all | ID ...] [--force]     (./build.sh runners: all)
"""
import glob, io, json, os, shutil, sqlite3, struct, subprocess, sys, tarfile, urllib.request, zipfile, zlib

H = os.path.dirname(os.path.abspath(__file__)); W = os.path.dirname(H); R = os.path.dirname(W)
CACHE = os.path.join(R, 'cache', 'chibi'); TOOLS = os.path.join(CACHE, 'tools'); WORK = os.path.join(CACHE, 'work')
OUT = os.path.join(W, 'img', 'run')
META = os.path.join(R, 'game', 'global', 'meta.db')
MDB = os.path.join(R, 'game', 'global', 'master.mdb')
ASSETS = os.path.join(R, 'game', 'global', 'decrypted')   # as webapp/images.py
RUN = '3d/motion/mini/event/body/type00/anm_min_eve_type00_run00_loop'
TILE, FRAMES_Q = 128, 60
ASMOD = os.path.join(TOOLS, 'asmod', 'AssetStudioModCLI_net9_linux64', 'AssetStudioModCLI')
FBX2GLTF = os.path.join(TOOLS, 'FBX2glTF-linux-x64')
DOTNET = os.path.join(TOOLS, 'dotnet'); XML = os.path.join(TOOLS, 'xml', 'usr', 'lib')
DOWNLOADS = {
    'asmod': 'https://github.com/aelurum/AssetStudio/releases/download/v0.19.0/AssetStudioModCLI_net9_linux64.zip',
    'fbx2gltf': 'https://github.com/facebookincubator/FBX2glTF/releases/download/v0.9.7/FBX2glTF-linux-x64',
    'dotnet': 'https://builds.dotnet.microsoft.com/dotnet/Runtime/9.0.20/dotnet-runtime-9.0.20-linux-x64.tar.gz',
    'libxml2': 'https://geo.mirror.pkgbuild.com/extra/os/x86_64/libxml2-legacy-2.13.9-2-x86_64.pkg.tar.zst',
}


def get(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': 'uma-legacy chibi.py'}), timeout=300) as r: return r.read()


def tools():
    os.makedirs(TOOLS, exist_ok=True)
    if not os.path.exists(ASMOD):
        print('fetching AssetStudioModCLI'); zipfile.ZipFile(io.BytesIO(get(DOWNLOADS['asmod']))).extractall(os.path.join(TOOLS, 'asmod'))
        os.chmod(ASMOD, 0o755)
    if not os.path.exists(FBX2GLTF):
        print('fetching FBX2glTF'); open(FBX2GLTF, 'wb').write(get(DOWNLOADS['fbx2gltf'])); os.chmod(FBX2GLTF, 0o755)
    if not os.path.exists(os.path.join(DOTNET, 'dotnet')):
        print('fetching the .NET 9 runtime'); tarfile.open(fileobj=io.BytesIO(get(DOWNLOADS['dotnet']))).extractall(DOTNET)
    if not os.path.exists(os.path.join(XML, 'libxml2.so.2')):   # always the copy in cache/chibi/tools, never the system's
        print('fetching libxml2.so.2 (libxml2-legacy)'); d = os.path.join(TOOLS, 'xml'); os.makedirs(d, exist_ok=True)
        pkg = os.path.join(d, 'libxml2-legacy.pkg.tar.zst'); open(pkg, 'wb').write(get(DOWNLOADS['libxml2']))
        subprocess.run(['tar', '--zstd', '-xf', pkg, '-C', d], check=True)
    env = dict(os.environ, DOTNET_ROOT=DOTNET, LD_LIBRARY_PATH=XML + (':' + os.environ['LD_LIBRARY_PATH'] if os.environ.get('LD_LIBRARY_PATH') else ''))
    return env


def body_of(meta, cid):
    """Her mini body's variant: the lowest-numbered one (mbdy{id}_00 for most; a few have only _01 and later)."""
    vs = sorted(r[0].rsplit('_', 1)[1] for r in meta.execute('select n from a where n like ?', (f'3d/chara/mini/body/mbdy{cid}_%/pfb_mbdy{cid}_%',)) if '/clothes/' not in r[0])
    return vs[0] if vs else '00'


def bundles(meta, cid, skin, tail, body):
    """Her bundles by name, with their prerequisites (the materials live in sourceresources/...)."""
    like = lambda p: [r[0] for r in meta.execute('select n from a where n like ?', (p,))]
    names = set(like(f'3d/chara/mini/body/mbdy{cid}_{body}/%') + like(f'3d/chara/mini/head/mchr{cid}_00/%'))
    names |= {'3d/chara/mini/head/mchr0001_00/pfb_mchr0001_00_face0', f'3d/chara/mini/head/mchr0001_00/textures/tex_mchr0001_00_face0_{skin}_diff',
              f'3d/chara/mini/head/mchr0001_00/textures/tex_mchr0001_00_face0_{skin}_cheek0', f'3d/chara/mini/tail/mtail{tail}_00/pfb_mtail{tail}_00',
              f'3d/chara/mini/tail/mtail{tail}_00/textures/tex_mtail{tail}_00_{cid}_diff', RUN}
    names |= set(like(f'3d/chara/mini/tail/mtail{tail}_00/clothes/%'))
    deps = set()
    for n in names:
        for (d,) in meta.execute('select d from a where n = ?', (n,)): deps |= {x for x in (d or '').split(';') if x}
    return sorted(names | deps)


def export(cid, skin, tail, body, meta, env):
    w = os.path.join(WORK, str(cid))
    if os.path.exists(os.path.join(w, 'run.json')): return None   # exported before (the game files don't change under an id; delete cache/chibi/work/ID to redo)
    os.makedirs(os.path.join(w, 'in'), exist_ok=True); missing = []
    for n in bundles(meta, cid, skin, tail, body):
        src = os.path.join(ASSETS, n)
        if os.path.exists(src): shutil.copyfile(src, os.path.join(w, 'in', n.replace('/', '__')))
        else: missing.append(n)
    if any(f'pfb_mbdy{cid}_{body}' in m or f'pfb_mchr{cid}_00_hair' in m for m in missing): return f'no mini model in the game files ({len(missing)} missing)'
    run = lambda *a: subprocess.run([ASMOD, os.path.join(w, 'in'), *a], env=env, capture_output=True, text=True)
    run('-m', 'animator', '--fbx-animation', 'skip', '-o', os.path.join(w, 'fbx')); run('-t', 'tex2d', '-o', os.path.join(w, 'tex'))
    os.makedirs(os.path.join(w, 't'), exist_ok=True); os.makedirs(os.path.join(w, 'glb'), exist_ok=True)
    for f in glob.glob(os.path.join(w, 'tex', '**', '*.png'), recursive=True) + glob.glob(os.path.join(w, 'fbx', '**', '*.png'), recursive=True):
        shutil.copyfile(f, os.path.join(w, 't', os.path.basename(f)))
    for f in glob.glob(os.path.join(w, 'fbx', '**', '*.fbx'), recursive=True):
        subprocess.run([FBX2GLTF, '-b', '-i', f, '-o', os.path.join(w, 'glb', os.path.splitext(os.path.basename(f))[0])], capture_output=True)
    for f in glob.glob(os.path.join(w, 't', '*.png')): shutil.copyfile(f, os.path.join(w, 'glb', os.path.basename(f)))   # the glb files name their textures beside them
    need = [f'pfb_mbdy{cid}_{body}.glb', f'pfb_mchr{cid}_00_hair.glb', 'pfb_mchr0001_00_face0.glb', f'pfb_mtail{tail}_00.glb']
    lack = [f for f in need if not os.path.exists(os.path.join(w, 'glb', f))]
    if lack: return 'export failed: ' + ', '.join(lack)
    return decode_run(w, cid, body)


def decode_run(w, cid, body):
    """The run clip as per-bone keyframes, its bones named through the CRC32 of their path in her body's skeleton."""
    import UnityPy
    env = UnityPy.load(os.path.join(w, 'in', RUN.replace('/', '__')))
    clip = next(o.read() for o in env.objects if o.type.name == 'AnimationClip')
    mc = clip.m_MuscleClip.m_Clip; mc = getattr(mc, 'data', mc)
    dense, const = mc.m_DenseClip, mc.m_ConstantClip.data
    fc, cc, samples = dense.m_FrameCount, dense.m_CurveCount, list(dense.m_SampleArray)
    curve = lambda i: [samples[f * cc + i] for f in range(fc)] if i < cc else [const[i - cc]] * fc   # (this clip streams no curves)
    b = open(os.path.join(w, 'glb', f'pfb_mbdy{cid}_{body}.glb'), 'rb').read(); n = struct.unpack('<I', b[12:16])[0]; g = json.loads(b[20:20 + n])
    parent = {c: i for i, nd in enumerate(g['nodes']) for c in nd.get('children', [])}
    hashes = {}
    for i in range(len(g['nodes'])):
        p, j = [], i
        while j is not None: p.append(g['nodes'][j]['name']); j = parent.get(j)
        p.reverse()
        for k in range(len(p)): hashes.setdefault(zlib.crc32('/'.join(p[k:]).encode()), i)
    size, kind = {1: 3, 2: 4, 3: 3, 4: 3}, {1: 'position', 2: 'quaternion', 3: 'scale', 4: 'euler'}
    out, ci = {'fps': dense.m_SampleRate, 'frames': fc, 'tracks': []}, 0
    for bnd in clip.m_ClipBindingConstant.genericBindings:
        vals = [curve(ci + j) for j in range(size[bnd.attribute])]; ci += size[bnd.attribute]
        if bnd.path in hashes:
            out['tracks'].append({'node': g['nodes'][hashes[bnd.path]]['name'], 'kind': kind[bnd.attribute], 'values': [[v[f] for v in vals] for f in range(fc)]})
    # the loop: the first frame back at the first pose (the clip stores a frame or two past its end)
    out['loop'] = next((k for k in range(1, fc) if max(abs(a - c) for t in out['tracks'] for a, c in zip(t['values'][0], t['values'][k])) < 1e-3), fc)
    json.dump(out, open(os.path.join(w, 'run.json'), 'w'))
    return None


def strip(cid):
    from PIL import Image
    fs = [Image.open(f) for f in sorted(glob.glob(os.path.join(WORK, str(cid), 'frames', 'f*.png')))]
    if not fs: return 'no frames'
    box = fs[0].getbbox()
    for f in fs[1:]:
        b = f.getbbox(); box = (min(box[0], b[0]), min(box[1], b[1]), max(box[2], b[2]), max(box[3], b[3]))
    side = max(box[2] - box[0], box[3] - box[1]) + 4; cx, cy = (box[0] + box[2]) // 2, (box[1] + box[3]) // 2
    sq = (cx - side // 2, cy - side // 2, cx - side // 2 + side, cy - side // 2 + side)
    s = Image.new('RGBA', (TILE * len(fs), TILE), (0, 0, 0, 0))
    for k, f in enumerate(fs): s.paste(f.crop(sq).resize((TILE, TILE), Image.LANCZOS), (k * TILE, 0))
    os.makedirs(OUT, exist_ok=True); s.save(os.path.join(OUT, f'{cid}.avif'), 'AVIF', quality=FRAMES_Q, speed=6)
    return None


def roster():
    ids = set()
    for f in (os.path.join(R, 'pkg', 'loopopt', 'export', 'base.json'), os.path.join(R, 'jp', 'export', 'base.json')):
        if os.path.exists(f): ids |= set(json.load(open(f))['roster'])
    return sorted(ids)


def main(argv):
    force = '--force' in argv; args = [a for a in argv if not a.startswith('--')]
    ids = roster() if not args or args == ['all'] else [int(a) for a in args]
    if not ids: sys.exit('no umas: build the data first (./build.sh data)')
    for f, what in ((META, 'the asset index (game/global/meta.db: the game\'s asset index as plain SQLite)'), (MDB, 'the Global master.mdb'), (ASSETS, 'the decrypted game files (game/global/decrypted)')):
        if not os.path.exists(f): sys.exit(f'missing {what}')
    todo = [c for c in ids if force or not os.path.exists(os.path.join(OUT, f'{c}.avif'))]
    if not todo: print(f'runners: all {len(ids)} present'); return
    env = tools(); meta = sqlite3.connect(f'file:{META}?mode=ro', uri=True); mdb = sqlite3.connect(f'file:{MDB}?mode=ro', uri=True)
    q = f'select id, skin, tail_model_id from chara_data where id in ({",".join(map(str, todo))})'
    info = {c: s for c, *s in mdb.execute(q)}
    jp = os.path.join(R, 'game', 'jp', 'master.mdb')   # umas not yet on Global (their mini models already ship in its files)
    if os.path.exists(jp): info = {**{c: s for c, *s in sqlite3.connect(f'file:{jp}?mode=ro', uri=True).execute(q)}, **info}
    ready = []
    for c in todo:
        if c not in info: print(f'{c}: not in master.mdb'); continue
        skin, tail = max(0, (info[c][0] or 1) - 1), f'{info[c][1] or 1:04d}'
        body = body_of(meta, c)
        err = export(c, skin, tail, body, meta, env)
        if err: print(f'{c}: {err}'); continue
        ready.append(f'{c}:{skin}:{tail}:{body}')
    if ready: subprocess.run(['node', os.path.join(H, 'render.mjs'), WORK, *ready])
    made = 0
    for job in ready:
        c = int(job.split(':')[0]); err = strip(c)
        if err: print(f'{c}: {err}')
        else: made += 1
    print(f'runners: {made} written to webapp/img/run ({len(ids) - len(todo)} kept, {len(todo) - made} not made)')


if __name__ == '__main__':
    main(sys.argv[1:])
