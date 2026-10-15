"""List every TDB in a file, or the tables of one. Read-only.
  python tdb_scan.py <file> --scan              # every embedded "DB" header
  python tdb_scan.py <file> [<base>] [--fields] # tables (and fields) of the TDB at <base>
"""
import os
import sys

sys.path.insert(0, os.path.dirname(__file__))
from tdblib import parse, scan

if __name__ == "__main__":
    path = sys.argv[1]
    b = open(path, "rb").read()
    if "--scan" in sys.argv:
        print("file size", len(b))
        for off, sz, nt in scan(b):
            print("  TDB at %#x size %d (%#x) tables %d ends %#x" % (off, sz, sz, nt, off + sz))
        sys.exit(0)
    base = int(sys.argv[2], 0) if len(sys.argv) > 2 and not sys.argv[2].startswith("--") else 0
    p = parse(b, base)
    print("TDB @%#x version=%d BE=%s db_size=%d tables=%d hdr_crc=%08x" %
          (p["base"], p["version"], p["big_endian"], p["db_size"], p["table_count"], p["header_crc"]))
    for t in p["tables"]:
        print("  %s (canon %s) block=%#x rec=%dB/%dbits max=%d cur=%d alloc=%d unk18=%#x nf=%d ni=%d" %
              (t["raw"], t["canonical"], t["block"], t["rec_bytes"], t["rec_bits"], t["max"], t["cur"],
               t["alloc"], t["unk18"], t["nfields"], t["nindex"]))
        if "--fields" in sys.argv:
            for f in t["fields"]:
                print("      %s (canon %s) kind=%d bit=%d width=%d" % (f["raw"], f["canonical"], f["kind"], f["bit"], f["width"]))
