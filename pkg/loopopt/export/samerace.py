from model import PROG
for pid,p in PROG.items():
    p['race']={1025:1012,1027:1006,1028:1005,1029:1016}.get(p['race'],p['race'])
