from era import c

pt = {r[0]: r[1] for r in c.execute("select relation_type,relation_point from succession_relation")}
mem = {}
for relation_type, character_id in c.execute("select relation_type,chara_id from succession_relation_member"):
    mem.setdefault(character_id, set()).add(relation_type)


def pair(a, b):
    return 0 if a == b else sum(pt[relation_type] for relation_type in mem.get(a, set()) & mem.get(b, set()))


def tri(a, b, c_):
    return (
        0
        if len({a, b, c_}) < 3
        else sum(pt[relation_type] for relation_type in mem.get(a, set()) & mem.get(b, set()) & mem.get(c_, set()))
    )
