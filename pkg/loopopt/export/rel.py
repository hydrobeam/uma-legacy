"""Relation points from master.mdb: each relation type's points, and the relation types each character belongs to."""


def relation_points(era):
    """(relation type -> its points, character id -> the relation types she belongs to)"""
    points = {r[0]: r[1] for r in era.q("select relation_type,relation_point from succession_relation")}
    members = {}
    for relation_type, character_id in era.q("select relation_type,chara_id from succession_relation_member"):
        members.setdefault(character_id, set()).add(relation_type)
    return points, members
