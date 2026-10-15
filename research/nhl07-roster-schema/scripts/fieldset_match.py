"""Match tables between two TDBs by exact field-id multiset + descriptor layout.
Usage: python fieldset_match.py <A> <baseA> <B> <baseB> [meta.xml]
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
a = parse(open(sys.argv[1], "rb").read(), int(sys.argv[2], 0))
b = parse(open(sys.argv[3], "rb").read(), int(sys.argv[4], 0))
names = {}
if len(sys.argv) > 5:
    x = open(sys.argv[5], encoding="utf-8", errors="replace").read()
    for m in re.finditer(r'<table\b[^>]*>', x):
        tag = m.group(0)
        sn = re.search(r'shortname="([^"]+)"', tag); nm = re.search(r'\bname="([^"]+)"', tag)
        if sn and nm: names[sn.group(1)] = nm.group(1)
def key(t): return tuple((f["raw"], f["kind"], f["bit"], f["width"]) for f in t["fields"])
def ids(t): return frozenset(f["raw"] for f in t["fields"])
exact = {}; loose = {}
for t in b["tables"]:
    exact.setdefault(key(t), []).append(t); loose.setdefault(ids(t), []).append(t)
for t in a["tables"]:
    e = exact.get(key(t), [])
    if e: how = "EXACT(ids+layout)"
    else:
        e = loose.get(ids(t), []); how = "same field-id set, different layout" if e else "NONE"
    print("%s/%s %4dB nf=%2d -> %s: %s" % (t["raw"], t["canonical"], t["rec_bytes"], t["nfields"], how,
          ", ".join("%s/%s[%s] cur=%d" % (x["raw"], x["canonical"], names.get(x["raw"], "?"), x["cur"]) for x in e)))
    if not e:
        # best partial overlap
        best = sorted(((len(ids(t) & ids(x)), x) for x in b["tables"]), key=lambda p: -p[0])[:3]
        print("      partial:", ", ".join("%s/%s[%s] shares %d/%d" % (x["raw"], x["canonical"], names.get(x["raw"], "?"), n, len(ids(t))) for n, x in best))
        print("      fields:", " ".join(f["raw"] for f in t["fields"]))
fa = {f["raw"] for t in a["tables"] for f in t["fields"]}; fb = {f["raw"] for t in b["tables"] for f in t["fields"]}
print("\nA-only field ids:", sorted(fa - fb)); print("overlap %d of %d" % (len(fa & fb), len(fa)))
