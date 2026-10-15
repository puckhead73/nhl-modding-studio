"""Test: are the save tables NOT declared as insert/delete shortnames the per-parent 'update'
difflists whose field set == {fields with update="True"} (+ keys) of a roster-savegroup parent?
Usage: python update_table_match.py <save> <base> <game.db> <meta.xml>
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
save = parse(open(sys.argv[1], "rb").read(), int(sys.argv[2], 0))
game = parse(open(sys.argv[3], "rb").read(), 0)
x = open(sys.argv[4], encoding="utf-8-sig", errors="replace").read()
meta = {}
for m in re.finditer(r'<table\b([^>]*)>(.*?)</table>', x, re.S):
    a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
    flds = []
    for fm in re.finditer(r'<field\b([^>]*)/>', m.group(2)):
        flds.append(dict(re.findall(r'(\w+)="([^"]*)"', fm.group(1))))
    a["fields"] = flds; meta[a["shortname"]] = a
declared = set()
for a in meta.values():
    declared.add(a.get("insertshortname")); declared.add(a.get("deleteshortname"))
gt = {t["raw"]: t for t in game["tables"]}
cands = {}
for sn, a in meta.items():
    if "rosters" not in a.get("savegroups", ""): continue
    upd = frozenset(f["shortname"] for f in a["fields"] if f.get("update") == "True")
    keys = frozenset(f["shortname"] for f in a["fields"] if f.get("key") == "True")
    allf = frozenset(f["shortname"] for f in a["fields"])
    cands[sn] = (a["name"], upd, keys, allf)
for t in save["tables"]:
    if t["raw"] in declared: continue
    fs = frozenset(f["raw"] for f in t["fields"])
    hits = []
    for sn, (name, upd, keys, allf) in cands.items():
        if fs == upd: hits.append("%s[%s] == update-fields" % (sn, name))
        elif fs == upd | keys: hits.append("%s[%s] == update+key" % (sn, name))
        elif fs == allf: hits.append("%s[%s] == all fields (upd=%d)" % (sn, name, len(upd)))
    print("%s %4dB nf=%2d -> %s" % (t["raw"], t["rec_bytes"], t["nfields"], "; ".join(hits) or "NO MATCH"))
    if not hits:
        for sn, (name, upd, keys, allf) in cands.items():
            if fs <= allf:
                print("     subset of %s[%s]: missing %s ; update-set differs by +%s -%s" % (sn, name, sorted(allf - fs), sorted(fs - upd), sorted(upd - fs)))
print("\nupdate-field counts for roster-group parents:")
for sn, (name, upd, keys, allf) in cands.items():
    print("  %-30s %s all=%d update=%d key=%s" % (name, sn, len(allf), len(upd), sorted(keys)))
