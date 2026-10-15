"""Container-level checks for an NHL 08 Xbox 360 gen6-style save (ROSTER or DYNASTY). Read-only.
  python container_checks.py <save> <game.db> <meta.xml> [--group rosters]
Prints: container header words, checksum_a (CRC-32/IEEE over [0x1C..EOF]), every embedded TDB,
all four inner CRC scopes (tdb-core recompute_crcs), header/alloc/padding fields, record
totals, non-TDB preamble/tail statistics, and the capacity model (sum of declared difflist
capacities x record length vs the zero tail) for the save tables mapped by the meta.
"""
import os
import struct
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, verify_crcs, crc32_ieee, load_meta, roles, capacity

save_path, game_path, meta_path = sys.argv[1:4]
group = sys.argv[sys.argv.index("--group") + 1] if "--group" in sys.argv else None
b = open(save_path, "rb").read()
print("file", save_path)
print("size %d (%#x)" % (len(b), len(b)))
magic = b[:16].split(b"\0")[0].decode("latin1")
cs = struct.unpack_from(">I", b, 0x10)[0]
ver = struct.unpack_from(">I", b, 0x14)[0]
w18 = struct.unpack_from(">I", b, 0x18)[0]
print("magic %r  checksum_a stored %08x computed(crc32/ieee [0x1C..EOF]) %08x %s  version word %d  word@0x18 %d" % (
    magic, cs, crc32_ieee(b[0x1C:]), "MATCH" if cs == crc32_ieee(b[0x1C:]) else "MISMATCH", ver, w18))
hits = scan(b)
print("embedded TDB headers:", ["@%#x size %d tables %d" % h for h in hits])
if not hits:
    sys.exit("no TDB found")
base, size, _ = hits[0]
pre = b[0x1C:base]
print("bytes between container header and TDB: %d (zero: %d, nonzero: %d)" % (len(pre), pre.count(0), len(pre) - pre.count(0)))
p = parse(b, base)
print("inner TDB @%#x: version %d big_endian %s db_size %d (%#x) word@0x0C %#x tables %d" % (
    base, p["version"], p["big_endian"], p["db_size"], p["db_size"], p["hdr_0c"], p["table_count"]))
v = verify_crcs(b, p)
print("inner CRCs: file header %s; prior_crc ok %d/%d; table header_crc ok %d/%d; EOF crc stored %08x computed %08x %s" % (
    "ok" if v["header"] else "BAD", sum(v["prior"]), len(v["prior"]), sum(v["table_header"]), len(v["table_header"]),
    v["eof"][0], v["eof"][1], "ok" if v["eof"][0] == v["eof"][1] else "BAD"))
allocs = {}
unk = {}
for t in p["tables"]:
    allocs[t["alloc"]] = allocs.get(t["alloc"], 0) + 1
    k = (t["unk10"], t["unk18"], t["nindex"], t["unk1e"], t["unk20"])
    unk[k] = unk.get(k, 0) + 1
print("data_alloc_type distribution:", allocs)
print("(word@+0x10, word@+0x18, index_count, u16@+0x1E, word@+0x20) distribution:", unk)
print("records: sum(current) %d  sum(max) %d  tables with cur>0: %d  tables with max>cur: %d  max==cur everywhere: %s" % (
    sum(t["cur"] for t in p["tables"]), sum(t["max"] for t in p["tables"]), sum(1 for t in p["tables"] if t["cur"]),
    sum(1 for t in p["tables"] if t["max"] > t["cur"]), all(t["max"] == t["cur"] for t in p["tables"])))
rec_bits_ok = all(t["rec_bits"] == t["rec_bytes"] * 8 - 1 for t in p["tables"])
print("record_len_bits == record_len_bytes*8-1 for every table:", rec_bits_ok)
end = base + p["db_size"]
tail = b[end:]
print("tail after inner TDB: %d bytes (%#x..%#x), all zero: %s" % (len(tail), end, len(b), tail == b"\0" * len(tail)))
# layout check: tables contiguous in directory order, descriptors immediately followed by records
prev_end = base + 0x18 + 8 * p["table_count"]
gaps = []
for t in p["tables"]:
    if t["block"] != prev_end:
        gaps.append((t["raw"], t["block"] - prev_end))
    prev_end = t["block_end"]
print("table blocks contiguous in directory order:", "yes" if not gaps else "NO %s" % gaps,
      "; last block ends at %#x, db_size end %#x (EOF crc occupies the last 4 bytes: %s)" % (prev_end, end, prev_end + 4 == end))

# capacity model
game = parse(open(game_path, "rb").read(), 0)
gt = {t["raw"]: t for t in game["tables"]}
meta = load_meta(meta_path)
rl = roles(meta)
total = 0
unresolved = []
print("\n%-5s %-7s %-5s %-30s %5s %8s  %-26s %10s" % ("save", "role", "parent", "parent name", "recB", "capacity", "capacity source", "bytes"))
for t in p["tables"]:
    r = rl.get(t["raw"])
    if not r:
        unresolved.append(t["raw"])
        print("%-5s UNDECLARED" % t["raw"])
        continue
    parent, role = r
    a = meta[parent]
    cap, src = capacity(a, role, gt.get(parent))
    byt = (cap or 0) * t["rec_bytes"]
    total += byt
    print("%-5s %-7s %-5s %-30s %5d %8s  %-26s %10d" % (t["raw"], role, parent, a["name"], t["rec_bytes"], cap, src, byt))
print("sum(capacity x rec_bytes) = %d ; zero tail = %d ; difference = %d ; schema(db_size) + capacity = %d ; file - TDB base = %d" % (
    total, len(tail), len(tail) - total, p["db_size"] + total, len(b) - base))
if unresolved:
    print("undeclared save tables:", unresolved)
if group:
    missing = [(sn, a["name"]) for sn, a in meta.items() if group in a.get("savegroups", "").split(",")
               and not any(rl.get(t["raw"], ("", ""))[0] == sn for t in p["tables"])]
    print("parents in save group %r with no companion in this save: %s" % (group, missing or "none"))
