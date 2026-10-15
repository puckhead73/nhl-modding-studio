"""Dump decoded records of a TDB table with meta field names (read-only).
  python record_dump.py <file> <base|auto> <meta.xml> <table raw id> [first] [count] [--only f1,f2] [--as <parent raw id>] [--where field=value]
--as decodes a difflist companion table (absent from the meta) with its parent's field names.
--where keeps only records whose decoded field equals the value (string compare).
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan, load_meta, record, decode, field_names

if __name__ == "__main__":
    b = open(sys.argv[1], "rb").read()
    base = scan(b)[0][0] if sys.argv[2] == "auto" else int(sys.argv[2], 0)
    p = parse(b, base)
    meta = load_meta(sys.argv[3])
    tid = sys.argv[4]
    t = next(t for t in p["tables"] if t["raw"] == tid)
    rest = [a for a in sys.argv[5:] if not a.startswith("--")]
    first = int(rest[0]) if rest else 0
    count = int(rest[1]) if len(rest) > 1 else 3
    only = set(sys.argv[sys.argv.index("--only") + 1].split(",")) if "--only" in sys.argv else None
    where = sys.argv[sys.argv.index("--where") + 1].split("=") if "--where" in sys.argv else None
    src = sys.argv[sys.argv.index("--as") + 1] if "--as" in sys.argv else tid
    a = meta.get(src)
    names = field_names(a) if a else {}
    label = (a["name"] if a else "?") + ("" if src == tid else " (decoded as parent %s)" % src)
    print("table %s = %s block=%#x record_block=%#x rec=%dB cur=%d max=%d alloc=%d" % (
        tid, label, t["block"], t["record_block"], t["rec_bytes"], t["cur"], t["max"], t["alloc"]))
    shown = 0
    for i in range(t["cur"]):
        d = decode(record(b, t, i), t, p["big_endian"])
        named = [(names.get(k, k), v) for k, v in d.items()]
        if where:
            if not any(n == where[0] and str(v) == where[1] for n, v in named):
                continue
        if i < first:
            continue
        out = ["%s=%r" % (n, v) if isinstance(v, str) else "%s=%d" % (n, v) for n, v in named if not only or n in only or n in names and n in only]
        print("  [%d @%#x] %s" % (i, t["record_block"] + i * t["rec_bytes"], " ".join(out)))
        shown += 1
        if shown >= count:
            break
