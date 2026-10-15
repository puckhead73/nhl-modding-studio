"""Read-only TDB v6/v8 walker used for the NHL 07 roster-schema research.

Usage:
  python tdb_scan.py <file> [--scan]   # --scan: find every "DB" header in the file
Prints: file header, every table (raw id, canonical id, rec len, max/cur, fields).
Layout taken from crates/tdb-core/src/lib.rs (FILE_HEADER 0x18, TABLE_HEADER 0x28,
DESCRIPTOR 0x10). Nothing is written.
"""
import struct, sys, json

def u32(b, o, be): return struct.unpack_from(">I" if be else "<I", b, o)[0]
def u16(b, o, be): return struct.unpack_from(">H" if be else "<H", b, o)[0]

def parse(b, base=0):
    h = b[base:]
    if h[:2] != b"DB": raise ValueError("no DB magic at %#x" % base)
    be = h[4] == 1
    ver = u16(h, 2, be)
    db_size = u32(h, 8, be); zero = u32(h, 0xC, be)
    ntab = u32(h, 0x10, be); hcrc = u32(h, 0x14, be)
    info = dict(base=base, version=ver, big_endian=be, db_size=db_size, hdr_0c=zero,
                table_count=ntab, header_crc=hcrc, tables=[])
    dir_end = 0x18 + 8 * ntab
    for i in range(ntab):
        o = 0x18 + 8 * i
        rid = h[o:o+4].decode("latin1"); off = u32(h, o+4, be)
        abs_ = dir_end + off
        t = dict(raw=rid, canonical=rid[::-1] if be else rid, block=base+abs_,
                 prior_crc=u32(h, abs_, be), alloc=u32(h, abs_+4, be),
                 rec_bytes=u32(h, abs_+8, be), rec_bits=u32(h, abs_+12, be),
                 max=u16(h, abs_+0x14, be), cur=u16(h, abs_+0x16, be),
                 unk18=u32(h, abs_+0x18, be), nfields=h[abs_+0x1C], nindex=h[abs_+0x1D],
                 fields=[])
        d = abs_ + 0x28
        for _ in range(t["nfields"]):
            fid = h[d+8:d+12].decode("latin1")
            t["fields"].append(dict(kind=u32(h, d, be), bit=u32(h, d+4, be), raw=fid,
                                    canonical=fid[::-1] if be else fid, width=u32(h, d+12, be)))
            d += 0x10
        t["record_block"] = base + d
        t["block_end"] = base + d + t["rec_bytes"] * t["max"]
        info["tables"].append(t)
    return info

def scan(b):
    hits = []
    i = 0
    while True:
        i = b.find(b"DB\x00", i)
        if i < 0: break
        if i + 0x18 <= len(b) and b[i+4] in (0, 1):
            try:
                p = parse(b, i)
                if 0 < p["table_count"] < 500 and 0x18 < p["db_size"] <= len(b) - i:
                    hits.append((i, p["db_size"], p["table_count"]))
            except Exception: pass
        i += 1
    return hits

if __name__ == "__main__":
    path = sys.argv[1]; b = open(path, "rb").read()
    if "--scan" in sys.argv:
        print("file size", len(b))
        for off, sz, nt in scan(b):
            print("  TDB at %#x size %d (%#x) tables %d ends %#x" % (off, sz, sz, nt, off+sz))
        sys.exit(0)
    base = int(sys.argv[2], 0) if len(sys.argv) > 2 and not sys.argv[2].startswith("--") else 0
    p = parse(b, base)
    if "--json" in sys.argv:
        print(json.dumps(p, indent=1)); sys.exit(0)
    print("TDB @%#x version=%d BE=%s db_size=%d tables=%d hdr_crc=%08x" %
          (p["base"], p["version"], p["big_endian"], p["db_size"], p["table_count"], p["header_crc"]))
    for t in p["tables"]:
        print("  %s (canon %s) block=%#x rec=%dB/%dbits max=%d cur=%d alloc=%d unk18=%#x nf=%d ni=%d" %
              (t["raw"], t["canonical"], t["block"], t["rec_bytes"], t["rec_bits"], t["max"], t["cur"],
               t["alloc"], t["unk18"], t["nfields"], t["nindex"]))
        if "--fields" in sys.argv:
            for f in t["fields"]:
                print("      %s (canon %s) kind=%d bit=%d width=%d" % (f["raw"], f["canonical"], f["kind"], f["bit"], f["width"]))
