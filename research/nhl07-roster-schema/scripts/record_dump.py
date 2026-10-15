"""Dump decoded records of a TDB table using the meta XML field names (read-only).
Usage: python record_dump.py <db> <base> <meta.xml> <table raw id> [first] [count] [--only f1,f2] [--as <parent raw id>]
  --as: decode a difflist table (absent from the meta) with its parent table's field names
Integers: big-endian MSB-first bit fields (tdb-core read_bits). Strings: inline bytes, NUL-trimmed.
"""
import sys, os, re
sys.path.insert(0, os.path.dirname(__file__))
from tdb_scan import parse
def read_bits(data, off, width, be=True):
    bs = off // 8; e = (off + width + 7) // 8; span = data[bs:e]
    acc = int.from_bytes(span, "big" if be else "little")
    if be: return (acc >> (len(span) * 8 - (off - bs * 8) - width)) & ((1 << width) - 1)
    return (acc >> (off - bs * 8)) & ((1 << width) - 1)
if __name__ == "__main__":
  db = open(sys.argv[1], "rb").read(); base = int(sys.argv[2], 0); p = parse(db, base)
  x = open(sys.argv[3], encoding="utf-8-sig", errors="replace").read()
  names = {}
  for m in re.finditer(r'<table\b([^>]*)>(.*?)</table>', x, re.S):
      a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
      names[a["shortname"]] = (a["name"], {f["shortname"]: f for f in (dict(re.findall(r'(\w+)="([^"]*)"', fm.group(1))) for fm in re.finditer(r'<field\b([^>]*)/>', m.group(2)))})
  tid = sys.argv[4]; t = next(t for t in p["tables"] if t["raw"] == tid)
  first = int(sys.argv[5]) if len(sys.argv) > 5 else 0; count = int(sys.argv[6]) if len(sys.argv) > 6 else 3
  only = None
  if "--only" in sys.argv: only = set(sys.argv[sys.argv.index("--only") + 1].split(","))
  if "--as" in sys.argv: tname, fm = names[sys.argv[sys.argv.index("--as") + 1]]; tname = tid + " named as " + tname
  else: tname, fm = names.get(tid, (tid, {}))
  print("table %s (%s) block=%#x record_block=%#x rec=%dB cur=%d max=%d" % (tid, tname, t["block"], t["record_block"], t["rec_bytes"], t["cur"], t["max"]))
  for i in range(first, min(first + count, t["cur"])):
      s = t["record_block"] + i * t["rec_bytes"]; rec = db[s:s + t["rec_bytes"]]
      out = []
      for f in t["fields"]:
          fname = fm.get(f["raw"], {}).get("name", f["raw"])
          if only and fname not in only and f["raw"] not in only: continue
          if f["kind"] == 0:
              raw = rec[f["bit"] // 8: (f["bit"] + f["width"]) // 8]; v = repr(raw.split(b"\0")[0].decode("latin1"))
          else:
              v = read_bits(rec, f["bit"], f["width"], p["big_endian"])
          out.append("%s=%s" % (fname, v))
      print("  [%d @%#x] %s" % (i, s, " ".join(out)))
