"""Deletes the recorded solver replies of every solver and data but the current ones (./build.sh prune-replies).

cache/solver-replies/ holds one directory per solver and data (webapp/test/replaystore.mjs). The argument is the
current one's path, which is kept; every other directory there is removed, file by file, and named. Only what a
recording writes is removed: a directory holding anything else, a symbolic link or any path outside
cache/solver-replies/ is refused and left as it is.

    uv run webapp/test/prune_replies.py CURRENT_STORE_DIR
"""

import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parents[2]
ROOT = REPO / "cache" / "solver-replies"
# the wasm's and both engines' fingerprints: Vite's (8 characters, base64url), or build.py's before it (hex)
STORE_DIR = re.compile(r"[\w-]{8}-[\w-]{8}-[\w-]{8}|[0-9a-f]+-[0-9a-f]+-[0-9a-f]+")
REPLY_FILE = re.compile(r"[0-9a-f]{64}\.json(\.\d+\.part)?|\.recorded-(quick|full)")


def refusal(store: Path) -> str | None:
    """Why a directory of the store must not be removed, or None when it holds only what a recording writes."""
    if store.is_symlink() or not store.is_dir() or not STORE_DIR.fullmatch(store.name):
        return "not a recording directory"
    if store.resolve().parent != ROOT.resolve():
        return "outside cache/solver-replies/"
    for entry in store.iterdir():
        if entry.is_symlink() or not entry.is_file() or not REPLY_FILE.fullmatch(entry.name):
            return f"holds {entry.name}, which no recording writes"
    return None


def remove(store: Path) -> str:
    """Removes a recording directory file by file; returns what it held."""
    files = sorted(store.iterdir())
    size = sum(f.stat().st_size for f in files)
    for f in files:
        f.unlink()
    store.rmdir()
    replies = sum(f.name.endswith(".json") for f in files)
    return f"{replies} replies, {size / 1e6:.1f} MB"


def main() -> int:
    if len(sys.argv) != 2:
        print(__doc__, file=sys.stderr)
        return 2
    current = Path(sys.argv[1])
    if current.resolve().parent != ROOT.resolve() or not STORE_DIR.fullmatch(current.name):
        print(f"prune-replies: refused, {current} is not a recording directory of {ROOT}", file=sys.stderr)
        return 1
    if not ROOT.is_dir():
        print("prune-replies: no recorded replies")
        return 0
    loose = 0
    for entry in sorted(ROOT.iterdir()):
        if entry.name == current.name:
            continue
        if not entry.is_dir() or entry.is_symlink():
            loose += 1
            continue
        where = entry.relative_to(REPO)
        why = refusal(entry)
        if why:
            print(f"left {where}: {why}")
        else:
            print(f"removed {where}: {remove(entry)}")
    if loose:
        print(f"left {loose} files directly in {ROOT.relative_to(REPO)} (no recording writes there)")
    state = "" if current.is_dir() else " (nothing recorded yet)"
    print(f"kept {current.resolve().relative_to(REPO)}{state}: this tree's solver and data")
    return 0


if __name__ == "__main__":
    sys.exit(main())
