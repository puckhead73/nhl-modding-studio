"""Structural comparison of two TDB schemas: pairs tables by field-descriptor signature
(sequence of (kind, bit_offset, bit_width)), ignoring the 4-char ids entirely.
Usage: python schema_compare.py <tdb A> <baseA> <tdb B> <baseB> [--fields]
"""
import sys, os
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse

def sig(t): return tuple((f["kind"], f["bit"], f["width"]) for f in t["fields"])
def sig_loose(t): return (t["rec_bytes"], tuple((f["kind"], f["width"]) for f in t["fields"]))

a = parse(open(sys.argv[1], "rb").read(), int(sys.argv[2], 0))
b = parse(open(sys.argv[3], "rb").read(), int(sys.argv[4], 0))
bs = {}
for t in b["tables"]: bs.setdefault(sig(t), []).append(t)
bl = {}
for t in b["tables"]: bl.setdefault(sig_loose(t), []).append(t)
print("%-6s %-6s %5s %3s | exact-signature matches in B (raw/canon rec max cur)" % ("A.raw", "A.can", "recB", "nf"))
for t in a["tables"]:
    m = bs.get(sig(t), [])
    desc = ", ".join("%s/%s %dB max=%d cur=%d" % (x["raw"], x["canonical"], x["rec_bytes"], x["max"], x["cur"]) for x in m)
    if not m:
        m2 = bl.get(sig_loose(t), [])
        desc = "(no exact) loose: " + ", ".join("%s/%s" % (x["raw"], x["canonical"]) for x in m2) if m2 else "NONE"
    print("%-6s %-6s %5d %3d | %s" % (t["raw"], t["canonical"], t["rec_bytes"], t["nfields"], desc))
    if "--fields" in sys.argv and m:
        for fa, fb in zip(t["fields"], m[0]["fields"]):
            print("      %s/%s  <->  %s/%s  kind=%d bit=%d w=%d" % (fa["raw"], fa["canonical"], fb["raw"], fb["canonical"], fa["kind"], fa["bit"], fa["width"]))
# field-id overlap, any direction
fa = {f["raw"] for t in a["tables"] for f in t["fields"]}
fb = {f["raw"] for t in b["tables"] for f in t["fields"]}
ta = {t["raw"] for t in a["tables"]}; tb = {t["raw"] for t in b["tables"]}
print("\nraw table-id overlap:", sorted(ta & tb))
print("raw field-id overlap:", sorted(fa & fb))
print("A field ids:", len(fa), "B field ids:", len(fb))
