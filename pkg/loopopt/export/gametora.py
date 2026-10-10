"""GameTora's data files into cache/gametora/ (not committed): the event and skill tables card events are built from.

  uv run gametora.py            download the versions pinned in ../gametora.json that are not cached yet
  uv run gametora.py --latest   pin the newest versions from GameTora's manifest first, then download them

Files are saved as <key with / as _, static/ dropped>.<hash>.json (dict/evrew -> dict_evrew.8bda1ecc.json); a cached
file of the same key under another hash is removed, so each key has one file and readers can glob it. GameTora keeps
older versions online, so a pinned set rebuilds the same events.
"""
import json, os, sys, urllib.request

H = os.path.dirname(os.path.abspath(__file__)); R = os.path.dirname(os.path.dirname(os.path.dirname(H)))
PINS = os.path.join(H, '..', 'gametora.json')
CACHE = os.path.join(R, 'cache', 'gametora')
UA = 'legacy-loop-planner build script'


def get(url):
    with urllib.request.urlopen(urllib.request.Request(url, headers={'User-Agent': UA}), timeout=60) as r: return r.read()


def local_name(key): return key.removeprefix('static/').replace('/', '_')


def main(argv):
    pins = json.load(open(PINS))
    if '--latest' in argv:
        manifest = json.loads(get(pins['manifest']))
        moved = {k: manifest[k] for k in pins['files'] if manifest.get(k) and manifest[k] != pins['files'][k]}
        pins['files'].update(moved)
        with open(PINS, 'w') as f: json.dump(pins, f, indent=1); f.write('\n')
        print(f'gametora: {len(moved)} newer: {", ".join(moved) or "none"}')
    os.makedirs(CACHE, exist_ok=True)
    got = 0
    for key, h in pins['files'].items():
        name = local_name(key); dst = os.path.join(CACHE, f'{name}.{h}.json')
        for f in os.listdir(CACHE):
            if f.startswith(name + '.') and f.endswith('.json') and f != os.path.basename(dst) and f.count('.') == 2:
                os.remove(os.path.join(CACHE, f))
        if os.path.exists(dst): continue
        data = get(f"{pins['base']}/{key}.{h}.json"); json.loads(data)
        with open(dst + '.part', 'wb') as f: f.write(data)
        os.replace(dst + '.part', dst); got += 1
    print(f'gametora: {got} downloaded, {len(pins["files"]) - got} cached in {os.path.relpath(CACHE, R)}')


if __name__ == '__main__': main(sys.argv[1:])
