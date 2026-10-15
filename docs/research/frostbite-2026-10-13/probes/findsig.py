"""Read-only: find TOC-level chunks and bundle chunk spans whose FIRST block is stored uncompressed
(codec byte 0x00) and whose payload starts with a known Kettle-era signature.

Usage: python -I findsig.py <install_root> <root> <tag.jsonl>
"""
import collections
import json
import os
import struct
import sys

install, root, path = sys.argv[1:4]
SIGS = {b"DB\x00\x08": "TDB (LE, PS4-style)", b"\x89PNG": "PNG", b"RIFF": "RIFF", b"GNF ": "GNF", b"SBle": "SND10 SBle", b"DDS ": "DDS", b"<?xm": "XML", b"PK\x03\x04": "ZIP", b"\xef\xbb\xbf": "UTF-8 BOM text"}


def cas_path(cid):
    flag, pkg, cas = (cid >> 16) & 0xFF, (cid >> 8) & 0xFF, cid & 0xFF
    r = {0: "Data", 1: "Patch"}[flag]
    return os.path.join(install, r, "Gen4b", "superbundlelayout", f"nhl_installpackage_{pkg:02d}", f"cas_{cas:02d}.cas")


handles = {}
first_codec = collections.Counter()
hits = collections.defaultdict(list)
heads = collections.Counter()
rows = []
for line in open(path, encoding="utf-8"):
    if not line.startswith('{"k": "tocchunk"'):
        continue
    rows.append(json.loads(line))
rows.sort(key=lambda r: (r["cas"], r["off"]))  # near-sequential reads on a spinning disk
for r in rows:
    p = cas_path(r["cas"])
    f = handles.get(p) or handles.setdefault(p, open(p, "rb"))
    f.seek(r["off"])
    h = f.read(24)
    dsize, comp, _cs = struct.unpack_from(">IHH", h)
    codec = comp >> 8
    first_codec[(r["sb"], f"{codec:#04x}")] += 1
    if codec == 0:
        payload = h[8:]
        heads[payload[:4].hex()] += 1
        for sig, label in SIGS.items():
            if payload.startswith(sig):
                hits[label].append((r["sb"], r["guid"], f"{r['cas']:#06x}", r["off"], r["size"], dsize))
print("first-block codec per superbundle:")
for k, v in sorted(first_codec.items()):
    print("  ", k, v)
print("signature hits among uncompressed first blocks:")
for label, rows in hits.items():
    print(f"  {label}: {len(rows)}")
    for row in rows[:6]:
        print("     ", row)
print("most common uncompressed leading words:", heads.most_common(12))
