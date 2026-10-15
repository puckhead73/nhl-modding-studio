"""Verify the NHL 08 roster-group key graph of nhlng.db by record contents (read-only).
  python key_graph.py <nhlng.db> <meta.xml>
Checks key uniqueness and foreign-key coverage along
  exhibitionrostertable.playerindex -> exhibitionplayers.index -> (table, playerid) -> exhibitionplayerbiotable.game_id
  exhibitionfreeagents.playerindex -> exhibitionplayers.index
  exhibitionrostertable.team -> exhibitionteams.index ; exhibitionteamstrategies/arena.index
  skater/goalie AI and equipment .game_id -> exhibitionplayerbiotable.game_id (partition by position)
  stockgoalieequipment.game_id/key vs exhibitiongoalieequipment ; team colour/equipment tables.
"""
import os
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, load_meta, column, table

db = open(sys.argv[1], "rb").read()
p = parse(db, 0)
meta = load_meta(sys.argv[2])
by_name = {a["name"]: sn for sn, a in meta.items()}
fid = {sn: {f["name"]: f["shortname"] for f in a["fields"]} for sn, a in meta.items()}


def col(tname, fname):
    sn = by_name[tname]
    return column(db, table(p, sn), fid[sn][fname])


def rng(v):
    return "%d..%d" % (min(v), max(v)) if v else "empty"


print("game tables (name, rows, record bytes):")
for t in p["tables"]:
    print("  %-5s %-34s %5d rows %4dB" % (t["raw"], meta.get(t["raw"], {}).get("name", "?"), t["cur"], t["rec_bytes"]))

pl_idx, pl_id, pl_tab = col("exhibitionplayers", "index"), col("exhibitionplayers", "playerid"), col("exhibitionplayers", "table")
print("\nexhibitionplayers: %d rows; index %s unique=%s; playerid %s unique=%s; table flag counts %s; index==playerid rows %d" % (
    len(pl_idx), rng(pl_idx), len(set(pl_idx)) == len(pl_idx), rng(pl_id), len(set(pl_id)) == len(pl_id), dict(Counter(pl_tab)),
    sum(a == b for a, b in zip(pl_idx, pl_id))))
dups = [k for k, c in Counter(pl_id).items() if c > 1]
print("  playerids used by more than one slot: %d (e.g. %s)" % (len(dups), sorted(dups)[:8]))
idx_sorted = pl_idx == sorted(pl_idx)
print("  index ascending in record order: %s; gaps in index sequence: %d" % (idx_sorted, (max(pl_idx) - min(pl_idx) + 1) - len(set(pl_idx))))

bio = col("exhibitionplayerbiotable", "game_id")
bio_team = col("exhibitionplayerbiotable", "team")
bio_pos = col("exhibitionplayerbiotable", "position")
bio_pro = col("exhibitionplayerbiotable", "proteam")
print("exhibitionplayerbiotable: %d rows; game_id %s unique=%s; game_id==record index: %d rows; position counts %s" % (
    len(bio), rng(bio), len(set(bio)) == len(bio), sum(i == g for i, g in enumerate(bio)), dict(Counter(bio_pos))))
print("  playerid set vs bio game_id set: slots whose playerid has no bio %d; bios never slotted %d" % (
    len(set(pl_id) - set(bio)), len(set(bio) - set(pl_id))))
for flag in sorted(set(pl_tab)):
    ids = [i for i, f in zip(pl_id, pl_tab) if f == flag]
    print("  table flag %d: %d slots, playerids %s, resolvable in exhibitionplayerbiotable: %d" % (flag, len(ids), rng(ids), sum(i in set(bio) for i in ids)))

ros_key = col("exhibitionrostertable", "key")
ros = col("exhibitionrostertable", "playerindex")
team = col("exhibitionrostertable", "team")
status = col("exhibitionrostertable", "rosterstatus")
rjersey = col("exhibitionrostertable", "jersey")
idx = set(pl_idx)
print("exhibitionrostertable: %d rows; key %s unique=%s key==playerindex rows %d key==record index rows %d; playerindex unique=%s all in exhibitionplayers.index=%s; team %s; rosterstatus counts %s" % (
    len(ros), rng(ros_key), len(set(ros_key)) == len(ros_key), sum(a == b for a, b in zip(ros_key, ros)), sum(i == k for i, k in enumerate(ros_key)),
    len(set(ros)) == len(ros), set(ros) <= idx, rng(team), dict(Counter(status))))
fa = col("exhibitionfreeagents", "playerindex")
print("exhibitionfreeagents: %d rows; all in exhibitionplayers.index=%s; overlap with roster %d; roster+FA cover %d of %d slots" % (
    len(fa), set(fa) <= idx, len(set(fa) & set(ros)), len(set(ros) | set(fa)), len(idx)))
unassigned = sorted(idx - set(ros) - set(fa))
print("  unassigned slots: %d (first %s)" % (len(unassigned), unassigned[:10]))

tt_idx = col("exhibitionteams", "index")
tt_name = col("exhibitionteams", "fullname")
tt_league = col("exhibitionteams", "league")
tt_tab = col("exhibitionteams", "table")
tt_active = col("exhibitionteams", "active")
print("exhibitionteams: %d rows; index %s unique=%s index==record index rows %d; league counts %s; table flag counts %s; active counts %s" % (
    len(tt_idx), rng(tt_idx), len(set(tt_idx)) == len(tt_idx), sum(i == k for i, k in enumerate(tt_idx)), dict(Counter(tt_league)), dict(Counter(tt_tab)), dict(Counter(tt_active))))
print("  roster.team all in exhibitionteams.index: %s; teams with rostered players: %d" % (set(team) <= set(tt_idx), len(set(team))))
print("  sample teams: %s" % [(i, n) for i, n in zip(tt_idx, tt_name)][:4], "...", [(i, n) for i, n in zip(tt_idx, tt_name)][-3:])
ar_idx = col("exhibitionarena", "index")
st_idx = col("exhibitionteamstrategies", "index")
print("exhibitionarena.index == exhibitionteams.index set: %s; exhibitionteamstrategies.index == exhibitionteams.index set: %s" % (set(ar_idx) == set(tt_idx), set(st_idx) == set(tt_idx)))

id_by_idx = dict(zip(pl_idx, pl_id))
team_by_id = dict(zip(bio, bio_team))
mism = [(pi, tm, team_by_id.get(id_by_idx.get(pi))) for pi, tm in zip(ros, team) if team_by_id.get(id_by_idx.get(pi)) != tm]
print("bio.team vs roster.team (via slot): mismatches %d of %d (first %s)" % (len(mism), len(ros), mism[:5]))
jersey_by_id = dict(zip(bio, col("exhibitionplayerbiotable", "jersey")))
jm = sum(1 for pi, j in zip(ros, rjersey) if jersey_by_id.get(id_by_idx.get(pi)) != j)
print("bio.jersey vs roster.jersey (via slot): mismatches %d of %d" % (jm, len(ros)))

sk = col("exhibitionskaterai", "game_id")
gk = col("exhibitiongoalieai", "game_id")
skq = col("exhibitionskaterequipment", "game_id")
gkq = col("exhibitiongoalieequipment", "game_id")
print("AI/equipment keyed by game_id: skaterai %d unique=%s; goalieai %d unique=%s; skaterai+goalieai == bio ids: %s (disjoint %s)" % (
    len(sk), len(set(sk)) == len(sk), len(gk), len(set(gk)) == len(gk), set(sk) | set(gk) == set(bio), not (set(sk) & set(gk))))
pos_of_goalies = Counter(pos for g, pos in zip(bio, bio_pos) if g in set(gk))
pos_of_skaters = Counter(pos for g, pos in zip(bio, bio_pos) if g in set(sk))
print("  bio.position of goalieai ids: %s ; of skaterai ids: %s" % (dict(pos_of_goalies), dict(pos_of_skaters)))
custom_ids = {i for i, f in zip(pl_id, pl_tab) if f == 1}
print("  custom-slot (table=1) ids in skaterai: %d, in goalieai: %d, in skaterequipment: %d, in goalieequipment: %d" % (
    len(custom_ids & set(sk)), len(custom_ids & set(gk)), len(custom_ids & set(skq)), len(custom_ids & set(gkq))))
t30 = Counter(tm for _, tm, _ in mism)
print("  roster.team values on the bio/roster team mismatches: %s" % dict(t30.most_common(5)))
print("  skaterequipment ids == skaterai ids: %s; goalieequipment ids == goalieai ids: %s" % (set(skq) == set(sk), set(gkq) == set(gk)))
print("  record order aligned (skaterai[i].game_id == skaterequipment[i].game_id): %s; goalie: %s" % (sk == skq, gk == gkq))
sg_key = col("stockgoalieequipment", "key")
sg_id = col("stockgoalieequipment", "game_id")
sg_team = col("stockgoalieequipment", "team")
sg_fn = col("stockgoalieequipment", "firstname")
sg_ln = col("stockgoalieequipment", "lastname")
print("stockgoalieequipment: %d rows; key %s unique=%s; game_id in goalieai ids: %d; game_id==0 rows %d; sample %s" % (
    len(sg_key), rng(sg_key), len(set(sg_key)) == len(sg_key), sum(g in set(gk) for g in sg_id), sg_id.count(0),
    list(zip(sg_key, sg_id, sg_fn, sg_ln, sg_team))[:3]))
ste_idx = col("stockteamequipmenttable", "index")
ste_team = col("stockteamequipmenttable", "team")
ste_era = col("stockteamequipmenttable", "era")
print("stockteamequipmenttable: %d rows; index unique=%s %s; team %s (in exhibitionteams: %s); era counts %s" % (
    len(ste_idx), len(set(ste_idx)) == len(ste_idx), rng(ste_idx), rng(ste_team), set(ste_team) <= set(tt_idx), dict(Counter(ste_era))))
cte_idx = col("customteamequipmenttable", "index")
cte_team = col("customteamequipmenttable", "team")
print("customteamequipmenttable: %d rows; index %s unique=%s; team values %s" % (len(cte_idx), rng(cte_idx), len(set(cte_idx)) == len(cte_idx), dict(Counter(cte_team).most_common(4))))
sc_idx = col("stockteamcolorstable", "index")
sc_team = col("stockteamcolorstable", "team")
sc_cid = col("stockteamcolorstable", "colorid")
print("stockteamcolorstable: %d rows; index %s unique=%s; team %s; colorid counts %s" % (len(sc_idx), rng(sc_idx), len(set(sc_idx)) == len(sc_idx), rng(sc_team), dict(Counter(sc_cid))))
cf = col("customfacetable", "playerid")
cp = col("custompalettetable", "index")
print("customfacetable: %d rows playerid %s unique=%s (in bio ids: %d); custompalettetable: %d rows index %s" % (
    len(cf), rng(cf), len(set(cf)) == len(cf), sum(x in set(bio) for x in cf), len(cp), rng(cp)))
