#!/bin/bash
# usage: runall.sh BIN TAG
python3 - "$1" "$2" <<'PY'
import json,subprocess,sys,time
b,tag=sys.argv[1],sys.argv[2]
for d,j in json.load(open(S+'val/jobs.json')):
    name=(d.replace('/','_')+'__'+j.split('/')[-1]).replace('.json','')
    t=time.time(); out=subprocess.run([b,'data.json',j],cwd=S+d,capture_output=True,text=True).stdout
    open(S+f'val/{tag}__{name}.out','w').write(out); print(name, round(time.time()-t,1), flush=True)
PY
