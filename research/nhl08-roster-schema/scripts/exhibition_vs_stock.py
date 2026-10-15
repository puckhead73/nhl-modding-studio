"""Relationship between exhibition*, stock*, default*, league* and custom* tables in NHL 08 nhlng.db.
  python exhibition_vs_stock.py <game.db> <meta.xml>
For each name family (suffix after the prefix) lists the member tables, schema identity and,
where schemas match, byte identity of the record blocks. Read-only.
"""
import os
import re
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, load_meta

game = parse(open(sys.argv[1], "rb").read(), 0)
b = open(sys.argv[1], "rb").read()
meta = load_meta(sys.argv[2])
gt = {t["raw"]: t for t in game["tables"]}
fam = {}
for sn, a in meta.items():
    m = re.match(r"(exhibition|stock|default|league|custom|draft)(.*)", a["name"])
    key = m.group(2) if m else a["name"]
    fam.setdefault(key, []).append((m.group(1) if m else "", sn))
for key in sorted(fam):
    members = fam[key]
    if len(members) < 2:
        continue
    print("family %r:" % key)
    for pre, sn in members:
        t = gt[sn]
        a = meta[sn]
        print("   %-10s %-5s rows %5d recB %4d nf %2d groups=%s" % (pre, sn, t["cur"], t["rec_bytes"], t["nfields"], a.get("savegroups", "")))
    for i in range(len(members)):
        for j in range(i + 1, len(members)):
            x, y = gt[members[i][1]], gt[members[j][1]]
            fx = [(f["raw"], f["kind"], f["bit"], f["width"]) for f in x["fields"]]
            fy = [(f["raw"], f["kind"], f["bit"], f["width"]) for f in y["fields"]]
            same_schema = fx == fy
            shared = {f[0] for f in fx} & {f[0] for f in fy}
            ident = None
            if same_schema and x["cur"] == y["cur"] and x["cur"]:
                ident = b[x["record_block"]:x["record_block"] + x["rec_bytes"] * x["cur"]] == b[y["record_block"]:y["record_block"] + y["rec_bytes"] * y["cur"]]
            print("     %s vs %s: identical descriptors=%s shared field ids=%d/%d|%d%s" % (
                members[i][1], members[j][1], same_schema, len(shared), len(fx), len(fy), "" if ident is None else " ; record blocks byte-identical=%s" % ident))
print("\ntables with no family partner:", [meta[sn]["name"] for k, v in fam.items() if len(v) == 1 for _, sn in v])
