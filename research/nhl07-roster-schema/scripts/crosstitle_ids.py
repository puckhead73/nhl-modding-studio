"""H2 check: are difflist table ids (insert/delete/update) a stable function of the parent table id
across NHL 07 and NHL 08 metas/saves? Usage: python crosstitle_ids.py
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
def load_meta(path):
    x = open(path, encoding="utf-8-sig", errors="replace").read(); meta = {}
    for m in re.finditer(r'<table\b([^>]*)>(.*?)</table>', x, re.S):
        a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
        a["fields"] = [dict(re.findall(r'(\w+)="([^"]*)"', fm.group(1))) for fm in re.finditer(r'<field\b([^>]*)/>', m.group(2))]
        meta[a["shortname"]] = a
    return meta
def update_ids(save, base, meta):
    s = parse(open(save, "rb").read(), base); declared = {a.get("insertshortname") for a in meta.values()} | {a.get("deleteshortname") for a in meta.values()}
    out = {}
    for t in s["tables"]:
        if t["raw"] in declared: continue
        fs = frozenset(f["raw"] for f in t["fields"])
        hits = [sn for sn, a in meta.items() if "rosters" in a.get("savegroups", "") and fs == frozenset(f["shortname"] for f in a["fields"] if f.get("update") == "True" or f.get("key") == "True")]
        out[t["raw"]] = hits
    return out
m7 = load_meta("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"); m8 = load_meta("anchors/xbox360/nhl08/databases/nhlng-meta.xml")
print("declared insert/delete ids for parents present in BOTH metas (same raw id):")
same = diff = 0
for sn in sorted(set(m7) & set(m8)):
    a, b = m7[sn], m8[sn]
    i7, d7, i8, d8 = a.get("insertshortname"), a.get("deleteshortname"), b.get("insertshortname"), b.get("deleteshortname")
    if i7 or i8 or d7 or d8:
        ok = (i7 == i8) and (d7 == d8); same += ok; diff += (not ok)
        print("  %-5s %-32s/%-32s ins %s/%s del %s/%s %s" % (sn, a["name"], b["name"], i7, i8, d7, d8, "SAME" if ok else "DIFF"))
print("same:", same, "diff:", diff)
u7 = update_ids("anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1", 0x1c, m7)
u8 = update_ids("anchors/xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658", 0x1c, m8)
print("\nupdate-table ids resolved by field set, NHL 07 vs NHL 08 (same save table id -> parents):")
for k in sorted(set(u7) | set(u8)):
    print("  %-5s nhl07->%s nhl08->%s" % (k, u7.get(k), u8.get(k)))
