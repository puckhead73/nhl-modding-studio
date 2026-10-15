"""Container-level checks for the NHL 07 ROSTER1 save: checksum_a, inner header CRC, zero tail,
and the capacity model (declared difflist capacities vs the zero tail). Read-only.
Usage: python container_checks.py [save path]   (run from the repository root)
"""
import sys, os, re, zlib, struct
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse

R = sys.argv[1] if len(sys.argv) > 1 else "anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1"
G = "anchors/xbox360/nhl07/databases/NHL07NG.db"
M = "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml"
# Resolved update-table parents (see README section 3.3): declared ids come from the meta.
UPDATE_PARENTS = dict(CvSu="cPbu", MVOx="EhOn", HHLk="MqdK", RosT="yvSd", uyGF="yuHm", vozd="ulGe",
                      mASv="oYeE", Iblm="bZvB", rcpY="YzeG", zrsU="oDHy", DZfz="dDrd", YvhR="Raym")


def mpeg2(d):
    c = 0xffffffff
    for x in d:
        c ^= x << 24
        for _ in range(8):
            c = ((c << 1) ^ 0x04c11db7) & 0xffffffff if c & 0x80000000 else (c << 1) & 0xffffffff
    return c


b = open(R, "rb").read()
save = parse(b, 0x1c)
end = 0x1c + save["db_size"]
print("file size", len(b), "magic", b[:16].rstrip(b"\0"), "version word", struct.unpack_from(">I", b, 0x14)[0],
      "word@0x18", struct.unpack_from(">I", b, 0x18)[0])
print("checksum_a stored %08x  crc32/ieee over [0x1C..EOF] %08x" % (struct.unpack_from(">I", b, 0x10)[0], zlib.crc32(b[0x1c:]) & 0xffffffff))
print("inner TDB @0x1C: db_size %d tables %d header crc stored %08x computed %08x" % (
    save["db_size"], save["table_count"], struct.unpack_from(">I", b, 0x1c + 0x14)[0], mpeg2(b[0x1c:0x1c + 0x14])))
print("records in any table:", sum(t["cur"] for t in save["tables"]), " max_records in any table:", sum(t["max"] for t in save["tables"]))
print("tail after inner TDB:", len(b) - end, "bytes, all zero:", b[end:] == b"\0" * (len(b) - end))

game = parse(open(G, "rb").read(), 0)
gt = {t["raw"]: t for t in game["tables"]}
x = open(M, encoding="utf-8-sig").read()
meta = {}
for m in re.finditer(r'<table\b([^>]*)>', x):
    a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
    meta[a["shortname"]] = a
role = {}
for sn, a in meta.items():
    if a.get("insertshortname"):
        role[a["insertshortname"]] = (sn, "insert")
    if a.get("deleteshortname"):
        role[a["deleteshortname"]] = (sn, "delete")
for k, v in UPDATE_PARENTS.items():
    role[k] = (v, "update")
total = 0
print("\n%-5s %-7s %-30s %6s %8s %10s" % ("save", "role", "parent", "recB", "capacity", "bytes"))
for t in save["tables"]:
    parent, r = role[t["raw"]]
    a = meta[parent]
    cap = int(a["maxinsert"]) if r == "insert" else int(a["maxdelete"]) if r == "delete" else int(a.get("maxupdate", "-1"))
    if cap == -1:
        cap = gt[parent]["max"]
    total += cap * t["rec_bytes"]
    print("%-5s %-7s %-30s %6d %8d %10d" % (t["raw"], r, a["name"], t["rec_bytes"], cap, cap * t["rec_bytes"]))
print("sum(capacity * rec_bytes) = %d ; zero tail = %d ; schema + capacity = %d ; file - 0x1C = %d" % (
    total, len(b) - end, save["db_size"] + total, len(b) - 0x1c))
