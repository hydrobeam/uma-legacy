"""usage: add_pb.py LOOPOPT_BIN data.json...
Adds (or replaces) the "pb" field of each data.json: the exact pairwise affinity ceilings PB(a, b) for every pair of
umas, computed by `loopopt pbtable` (f64 bit patterns plus a fingerprint of the inputs). The solver uses the table
only while the fingerprint matches the file's umas, goals and streak penalties; otherwise it computes the ceilings
itself, so a stale table costs time, never correctness. Run after export_rs.py (./build.sh pb)."""

import json, subprocess, sys

binary, paths = sys.argv[1], sys.argv[2:]
for path in paths:
    pairwise_bounds = json.loads(
        subprocess.run([binary, "pbtable", path], check=True, capture_output=True, text=True).stdout
    )
    data = json.load(open(path))
    data["pb"] = pairwise_bounds
    json.dump(data, open(path, "w"))  # same layout as export_rs.py
    print(path, "pb", pairwise_bounds["fp"], len(pairwise_bounds["rows"]), "rows")
