"""Populated-difflist evidence from the NHL 08 DYNASTY1 save (read-only).
  python dynasty_evidence.py <dynasty save> <game.db> <meta.xml>
Separates container facts (preamble, TDB location, tail) from the difflist tables, maps every
table to its parent/role via the meta, and reports which insert / delete / update tables are
populated, their alloc fields, and whether max_records == current_records. Decodes samples.
"""
import os
import sys
from collections import Counter

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta, roles, capacity, record, decode, field_names, keys_of

dyn_path, game_path, meta_path = sys.argv[1:4]
b = open(dyn_path, "rb").read()
hits = scan(b)
print("file size %d; embedded TDBs: %s" % (len(b), ["@%#x size %d tables %d" % h for h in hits]))
base = hits[0][0]
pre = b[0x1C:base]
print("non-TDB preamble [0x1C..%#x): %d bytes, %d zero, %d nonzero" % (base, len(pre), pre.count(0), len(pre) - pre.count(0)))
print("  first 64 bytes: %s" % pre[:64].hex(" "))
# crude structure probe: positions of 0xFFFF words in the first 256 bytes, stride guess
ff = [i for i in range(0, 256, 2) if pre[i:i + 2] == b"\xff\xff"]
print("  0xFFFF word positions in first 256 bytes: %s" % ff)
p = parse(b, base)
meta = load_meta(meta_path)
rl = roles(meta)
game = parse(open(game_path, "rb").read(), 0)
gt = {t["raw"]: t for t in game["tables"]}
end = base + p["db_size"]
tail = b[end:]
print("inner TDB @%#x: tables %d db_size %d; tail %d bytes all zero: %s" % (base, p["table_count"], p["db_size"], len(tail), tail == b"\0" * len(tail)))

print("\n%-5s %-7s %-5s %-32s %5s %5s %5s %-9s %8s %s" % ("save", "role", "parent", "parent name", "recB", "max", "cur", "alloc", "capacity", "groups"))
pop = Counter()
alloc_by_state = Counter()
total_cap = 0
for t in p["tables"]:
    r = rl.get(t["raw"])
    if not r:
        print("%-5s UNDECLARED nf=%d recB=%d max=%d cur=%d" % (t["raw"], t["nfields"], t["rec_bytes"], t["max"], t["cur"]))
        continue
    parent, role = r
    a = meta[parent]
    cap, src = capacity(a, role, gt.get(parent))
    total_cap += (cap or 0) * t["rec_bytes"]
    state = "populated" if t["cur"] else "empty"
    pop[(role, state)] += 1
    alloc_by_state[(state, t["alloc"], t["max"] == t["cur"])] += 1
    print("%-5s %-7s %-5s %-32s %5d %5d %5d %-9s %8s %s" % (t["raw"], role, parent, a["name"], t["rec_bytes"], t["max"], t["cur"], t["alloc"], cap, a.get("savegroups", "")))
print("\ntables by (role, state):", dict(pop))
print("(state, data_alloc_type, max==cur) distribution:", dict(alloc_by_state))
rec_bytes_used = sum(t["cur"] * t["rec_bytes"] for t in p["tables"])
schema_only = p["db_size"] - rec_bytes_used
print("sum(capacity x recB) over mapped tables = %d ; db_size = %d of which record data %d, schema-only %d" % (total_cap, p["db_size"], rec_bytes_used, schema_only))
print("allocation test: schema-only + sum(capacity x recB) = %d ; file - TDB base = %d ; difference = %d" % (
    schema_only + total_cap, len(b) - base, (len(b) - base) - (schema_only + total_cap)))
print("alternative: db_size + sum((capacity - current) x recB) = %d" % (p["db_size"] + sum(((capacity(meta[rl[t["raw"]][0]], rl[t["raw"]][1], gt.get(rl[t["raw"]][0]))[0] or 0) - t["cur"]) * t["rec_bytes"] for t in p["tables"] if t["raw"] in rl)))
groups_present = Counter()
for t in p["tables"]:
    if t["raw"] in rl:
        for g in meta[rl[t["raw"]][0]].get("savegroups", "").split(","):
            groups_present[g] += 1
print("save-group membership of the parents present:", dict(groups_present))
dyn_parents = [sn for sn, a in meta.items() if "dynasty" in a.get("savegroups", "").split(",")]
present = {rl[t["raw"]][0] for t in p["tables"] if t["raw"] in rl}
print("dynasty-group parents: %d; present in save: %d; missing: %s" % (len(dyn_parents), len([x for x in dyn_parents if x in present]), [meta[x]["name"] for x in dyn_parents if x not in present]))
print("save table parents not in dynasty group: %s" % [meta[x]["name"] for x in present if x not in dyn_parents])

# directory order vs meta order
order = [meta[rl[t["raw"]][0]]["order"] for t in p["tables"] if t["raw"] in rl]
print("directory order follows meta document order of parents (non-decreasing): %s" % (order == sorted(order)))
triples = []
for t in p["tables"]:
    if t["raw"] in rl:
        triples.append(rl[t["raw"]][1])
print("role sequence (first 24): %s" % triples[:24])

print("\n== populated tables: decoded samples ==")
for t in p["tables"]:
    if not t["cur"] or t["raw"] not in rl:
        continue
    parent, role = rl[t["raw"]]
    a = meta[parent]
    names = field_names(a)
    keys = keys_of(a)
    print("%s %s of %s (%s): cur=%d max=%d alloc=%d" % (t["raw"], role, parent, a["name"], t["cur"], t["max"], t["alloc"]))
    kcol = []
    for i in range(t["cur"]):
        d = decode(record(b, t, i), t, p["big_endian"])
        if i < 2:
            items = list(d.items())
            show = [(names.get(k, k), v) for k, v in items if k in keys] + [(names.get(k, k), v) for k, v in items if k not in keys][:8]
            print("   [%d] %s" % (i, " ".join("%s=%r" % kv for kv in show)))
        kcol.append(tuple(d[k] for k in keys if k in d))
    if kcol and kcol[0]:
        ks = [k[0] for k in kcol]
        print("   key column: unique=%s range %d..%d ascending=%s" % (len(set(kcol)) == len(kcol), min(ks), max(ks), ks == sorted(ks)))
