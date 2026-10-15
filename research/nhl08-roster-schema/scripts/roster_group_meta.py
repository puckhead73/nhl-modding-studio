"""Field metadata of every rosters-group parent and the binary layout of its save companions.
  python roster_group_meta.py <game.db> <meta.xml> <save> [--group rosters]
Per field: name, raw id, type, depth, range, key, update flag, FK, bit offset in the parent
(game DB descriptor) and bit offset in the insert / update table of the save (if present).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta

game_path, meta_path, save_path = sys.argv[1:4]
group = sys.argv[sys.argv.index("--group") + 1] if "--group" in sys.argv else "rosters"
game = parse(open(game_path, "rb").read(), 0)
gt = {t["raw"]: t for t in game["tables"]}
sb = open(save_path, "rb").read()
save = parse(sb, scan(sb)[0][0])
st = {t["raw"]: t for t in save["tables"]}
meta = load_meta(meta_path)
for sn, a in meta.items():
    if group not in a.get("savegroups", "").split(","):
        continue
    pt = gt.get(sn)
    ins, dele, upd = a.get("insertshortname"), a.get("deleteshortname"), a.get("updateshortname")
    print("\n%s %s  rows=%s recB=%s  insert=%s(%s) delete=%s(%s) update=%s(%s) groups=%s" % (
        sn, a["name"], pt["cur"] if pt else "-", pt["rec_bytes"] if pt else "-", ins, a.get("maxinsert"), dele, a.get("maxdelete"), upd, a.get("maxupdate"), a.get("savegroups")))
    poff = {f["raw"]: f for f in pt["fields"]} if pt else {}
    ioff = {f["raw"]: f["bit"] for f in st[ins]["fields"]} if ins in st else {}
    uoff = {f["raw"]: f["bit"] for f in st[upd]["fields"]} if upd in st else {}
    doff = {f["raw"]: f["bit"] for f in st[dele]["fields"]} if dele in st else {}
    print("  %-26s %-5s %-8s %5s %-13s %-3s %-3s %-20s %7s %7s %7s %7s" % ("name", "id", "type", "depth", "range", "key", "upd", "fk", "p.bit", "ins.bit", "upd.bit", "del.bit"))
    for f in a["fields"]:
        q = poff.get(f["shortname"])
        typ = f["type"].replace("DBOFIELDTYPE_", "")[:8]
        print("  %-26s %-5s %-8s %5s %-13s %-3s %-3s %-20s %7s %7s %7s %7s" % (
            f["name"], f["shortname"], typ, f["depth"], f["rangelow"] + ".." + f["rangehigh"], "K" if f.get("key") == "True" else "", "U" if f.get("update") == "True" else "-",
            f.get("fkparenttable", ""), q["bit"] if q else "-", ioff.get(f["shortname"], "-"), uoff.get(f["shortname"], "-"), doff.get(f["shortname"], "-")))
        if q and int(f["depth"]) != q["width"]:
            print("      !! meta depth %s != binary width %d" % (f["depth"], q["width"]))
    if pt:
        binset = {f["raw"] for f in pt["fields"]}
        metaset = {f["shortname"] for f in a["fields"]}
        if binset != metaset:
            print("  !! field set differs: binary-only %s meta-only %s" % (sorted(binset - metaset), sorted(metaset - binset)))
        if [f["raw"] for f in pt["fields"]] != [f["shortname"] for f in a["fields"]]:
            print("  note: binary descriptor order differs from meta document order")
