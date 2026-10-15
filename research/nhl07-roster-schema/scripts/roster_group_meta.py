"""Dump the field metadata of every roster-savegroup table in a meta XML, cross-checked against the
game DB descriptors (bit offset/width), plus the difflist capacity arithmetic.
Usage: python roster_group_meta.py <game.db> <meta.xml> [save] [save_base]
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
game = parse(open(sys.argv[1], "rb").read(), 0)
x = open(sys.argv[2], encoding="utf-8-sig", errors="replace").read()
gt = {t["raw"]: t for t in game["tables"]}
meta = {}
for m in re.finditer(r'<table\b([^>]*)>(.*?)</table>', x, re.S):
    a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
    a["fields"] = [dict(re.findall(r'(\w+)="([^"]*)"', fm.group(1))) for fm in re.finditer(r'<field\b([^>]*)/>', m.group(2))]
    meta[a["shortname"]] = a
for sn, a in sorted(meta.items(), key=lambda kv: kv[1]["name"]):
    if "rosters" not in a.get("savegroups", ""): continue
    t = gt[sn]
    print("\n== %s (%s)  rec=%dB max=%d cur=%d  insert=%s delete=%s maxinsert=%s maxupdate=%s savegroups=%s" % (
        a["name"], sn, t["rec_bytes"], t["max"], t["cur"], a.get("insertshortname"), a.get("deleteshortname"), a.get("maxinsert"), a.get("maxupdate"), a["savegroups"]))
    desc = {f["raw"]: f for f in t["fields"]}
    for f in a["fields"]:
        d = desc.get(f["shortname"])
        flags = []
        if f.get("key") == "True": flags.append("KEY")
        if f.get("update") == "True": flags.append("upd")
        if f.get("fkparenttable"): flags.append("FK->" + f["fkparenttable"])
        print("   %-5s %-26s %-24s depth=%-5s range=%s..%s bit=%s w=%s %s" % (
            f["shortname"], f["name"], f["type"].replace("DBOFIELDTYPE_", ""), f.get("depth"), f.get("rangelow"), f.get("rangehigh"),
            d["bit"] if d else "?", d["width"] if d else "?", " ".join(flags)))
if len(sys.argv) > 3:
    save = parse(open(sys.argv[3], "rb").read(), int(sys.argv[4], 0))
    ins = {a["insertshortname"]: sn for sn, a in meta.items() if "insertshortname" in a}
    tot = 0
    for t in save["tables"]:
        if t["raw"] in ins:
            cap = int(meta[ins[t["raw"]]]["maxinsert"]) * t["rec_bytes"]; tot += cap
    print("\nsum(maxinsert * insert rec_bytes) over save insert tables =", tot)
