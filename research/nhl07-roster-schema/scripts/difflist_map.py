"""Map every table in a save's inner TDB to the game table whose metadata declares it as the
insert (difflist) or delete (key-only) shortname. Usage:
  python difflist_map.py <save> <inner_base> <game.db> <meta.xml>
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
save = parse(open(sys.argv[1], "rb").read(), int(sys.argv[2], 0))
game = parse(open(sys.argv[3], "rb").read(), 0)
x = open(sys.argv[4], encoding="utf-8-sig", errors="replace").read()
meta = {}
for m in re.finditer(r'<table\b([^>]*)>', x):
    a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
    meta[a["shortname"]] = a
ins = {a["insertshortname"]: (sn, "insert") for sn, a in meta.items() if "insertshortname" in a}
dele = {a["deleteshortname"]: (sn, "delete") for sn, a in meta.items() if "deleteshortname" in a}
gt = {t["raw"]: t for t in game["tables"]}
print("meta tables: %d, with insert ids: %d, with delete ids: %d" % (len(meta), len(ins), len(dele)))
groups = {}
for sn, a in meta.items():
    for g in a.get("savegroups", "").split(","):
        if g: groups.setdefault(g, []).append(sn)
print("savegroups:", {g: len(v) for g, v in groups.items()})
print()
print("%-5s %-6s %-7s %-34s %-6s %5s %5s %4s %4s | %s" % ("save", "role", "parent", "parent name", "inGame", "recB", "pRecB", "nf", "pnf", "maxins/maxdel savegroups"))
seen = set()
for t in save["tables"]:
    r = ins.get(t["raw"]) or dele.get(t["raw"])
    if not r:
        print("%-5s UNMAPPED" % t["raw"]); continue
    parent, role = r; seen.add(parent); a = meta[parent]; p = gt.get(parent)
    print("%-5s %-6s %-7s %-34s %-6s %5d %5s %4d %4s | %s/%s %s" % (
        t["raw"], role, parent, a["name"], "yes" if p else "NO", t["rec_bytes"],
        p["rec_bytes"] if p else "-", t["nfields"], p["nfields"] if p else "-",
        a.get("maxinsert", "?"), a.get("maxdelete", "?"), a.get("savegroups", "")))
print("\nGame tables whose savegroups include 'roster' but have no table in this save:")
for sn in groups.get("roster", []):
    if sn not in seen: print("  ", sn, meta[sn]["name"], meta[sn].get("insertshortname"), meta[sn].get("deleteshortname"))
print("\nAll savegroup memberships:")
for sn, a in sorted(meta.items(), key=lambda kv: kv[1]["name"]):
    if "savegroups" in a:
        print("  %-36s %-5s ins=%-5s del=%-5s maxins=%-4s maxdel=%-4s groups=%s" % (a["name"], sn, a.get("insertshortname"), a.get("deleteshortname"), a.get("maxinsert"), a.get("maxdelete"), a["savegroups"]))
