import json,glob,sys,os
sys.path.insert(0,os.path.dirname(os.path.abspath(__file__))+'/export')
os.chdir(os.path.dirname(os.path.abspath(__file__))+'/export')
from jpnames import names
G='../gametora/'; OFF=36
ev=json.load(open(glob.glob(G+'dict_evrew.*.json')[0])); sk=names(47)
T={'Uma Stan','Superstan','Nimble Navigator','No Stopping Me!',"Playtime's Over!",'See Ya Later!','Ramp Up',"It's On!",'Tail Held High','Tail Nine','Racing Spirit: Stamina','Slipstream','Perfect Spot!','Risky Business','Nothing Ventured'}
def dec(k):
    r=ev[k-OFF] if 0<=k-OFF<len(ev) else None
    if r is None: return f'?{k}'
    if r[0]=='sk': return 'SK:'+str(sk.get(r[2],r[2]))
    if r[0]=='sr': return 'SR:'+'|'.join(str(sk.get(x['d'])) for x in r[2])
    return r[0]
def events(obj):
    """yield (event list entry) = [id, choices, story, ...]"""
    if isinstance(obj,list):
        if len(obj)>=3 and isinstance(obj[0],int) and isinstance(obj[1],list) and isinstance(obj[2],(int,str)) and obj[1] and all(isinstance(c,list) and len(c)==2 and isinstance(c[1],list) for c in obj[1]):
            yield obj
        else:
            for x in obj: yield from events(x)
