"""Verify the roster-group key graph of NHL07NG.db by record contents (read-only).
exhibitionrostertable.playerindex -> exhibitionplayers.index -> (table, playerid) -> stock/custom bio game_id;
exhibitionfreeagents.playerindex -> exhibitionplayers.index; exhibitionrostertable.team -> exhibitionteams.index -> stockteamtable.index
Usage: python key_graph.py <NHL07NG.db> <meta.xml>
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
from record_dump import read_bits  # safe: record_dump guards main
db = open(sys.argv[1], "rb").read(); p = parse(db, 0)
x = open(sys.argv[2], encoding="utf-8-sig").read()
fm = {}
for m in re.finditer(r'<table\b([^>]*)>(.*?)</table>', x, re.S):
    a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
    fm[a["shortname"]] = {f["name"]: f["shortname"] for f in (dict(re.findall(r'(\w+)="([^"]*)"', q.group(1))) for q in re.finditer(r'<field\b([^>]*)/>', m.group(2)))}
T = {t["raw"]: t for t in p["tables"]}
def col(tid, name):
    t = T[tid]; f = next(f for f in t["fields"] if f["raw"] == fm[tid][name])
    return [read_bits(db[t["record_block"] + i * t["rec_bytes"]: t["record_block"] + (i + 1) * t["rec_bytes"]], f["bit"], f["width"]) for i in range(t["cur"])]
pl_idx, pl_id, pl_tab = col("caBZ", "index"), col("caBZ", "playerid"), col("caBZ", "table")
print("exhibitionplayers: %d rows; table flag counts %s; index range %d..%d; playerid range %d..%d; rows where index==playerid: %d" % (
    len(pl_idx), {v: pl_tab.count(v) for v in set(pl_tab)}, min(pl_idx), max(pl_idx), min(pl_id), max(pl_id), sum(a == b for a, b in zip(pl_idx, pl_id))))
print("  index unique:", len(set(pl_idx)) == len(pl_idx), " playerid unique:", len(set(pl_id)) == len(pl_id))
bio = col("cPbu", "game_id"); stock = col("aUhv", "game_id")
print("exhibitionplayerbiotable game_id: %d rows, unique %s, equals stockplayerbiotable game_id column: %s" % (len(bio), len(set(bio)) == len(bio), bio == stock))
print("  caBZ.playerid set == cPbu.game_id set:", set(pl_id) == set(bio), "| playerids not in bio:", len(set(pl_id) - set(bio)), "| bio ids not in caBZ:", len(set(bio) - set(pl_id)))
ros = col("ulGe", "playerindex"); fa = col("QEoV", "playerindex")
idx = set(pl_idx)
print("exhibitionrostertable.playerindex: %d rows, all in caBZ.index: %s, unique: %s" % (len(ros), set(ros) <= idx, len(set(ros)) == len(ros)))
print("exhibitionfreeagents.playerindex: %d rows, all in caBZ.index: %s, overlap with roster: %d" % (len(fa), set(fa) <= idx, len(set(fa) & set(ros))))
print("  roster + free agents cover caBZ.index:", len(set(ros) | set(fa)), "of", len(idx))
team = col("ulGe", "team"); tt_idx = col("ttOk", "index"); tt_id = col("ttOk", "teamid"); tt_tab = col("ttOk", "table")
print("exhibitionrostertable.team range %d..%d, all in exhibitionteams.index: %s" % (min(team), max(team), set(team) <= set(tt_idx)))
print("exhibitionteams: %d rows, index==teamid everywhere: %s, table flags %s" % (len(tt_idx), tt_idx == tt_id, {v: tt_tab.count(v) for v in set(tt_tab)}))
bt = col("cPbu", "team")
print("exhibitionplayerbiotable.team equals roster team for the same player (via caBZ): ", end="")
id_by_idx = dict(zip(pl_idx, pl_id)); team_by_id = dict(zip(bio, bt))
mism = sum(1 for pi, tm in zip(ros, team) if team_by_id.get(id_by_idx.get(pi)) != tm)
print("mismatches %d of %d" % (mism, len(ros)))
