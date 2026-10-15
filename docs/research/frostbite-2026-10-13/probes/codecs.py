"""Tally cas block-header compression words over a sample of spans (read-only).

Block header hypothesis (validated earlier against an ebx span whose sizes matched exactly):
  u32 BE decompressed size, u16 BE compression word, u16 BE compressed size.
Walks every block of each sampled span (a span is a run of blocks), tallies the compression word
per span kind, and checks the blocks tile the span exactly.

Usage: python -I codecs.py <install_root> <root> <tag.jsonl> [sample_every]
"""
import collections
import json
import os
import random
import struct
import sys

install, root, path = sys.argv[1:4]
every = int(sys.argv[4]) if len(sys.argv) > 4 else 50


def cas_path(cid):
    flag, pkg, cas = (cid >> 16) & 0xFF, (cid >> 8) & 0xFF, cid & 0xFF
    r = {0: "Data", 1: "Patch"}[flag]
    return os.path.join(install, r, "Gen4b", "superbundlelayout", f"nhl_installpackage_{pkg:02d}", f"cas_{cas:02d}.cas")


handles = {}


def blocks(cid, off, size):
    p = cas_path(cid)
    f = handles.get(p) or handles.setdefault(p, open(p, "rb"))
    f.seek(off)
    data = f.read(size)
    pos, out = 0, []
    while pos < len(data):
        if pos + 8 > len(data):
            return out, f"trailing {len(data) - pos} bytes"
        dsize, comp, csize = struct.unpack_from(">IHH", data, pos)
        # Hypothesis: low nibble of the compression word = bits 16-19 of the stored size; the
        # codec is the high byte; 0x7 in the high nibble of the low byte is constant.
        stored = ((comp & 0xF) << 16) | csize
        if (comp & 0xF0) != 0x70:
            return out, f"unexpected compression word {comp:#06x}"
        out.append((comp & 0xFFF0, dsize, csize))
        pos += 8 + stored
    return out, None if pos == len(data) else f"overrun {pos - len(data)}"


random.seed(1)
tally = collections.defaultdict(collections.Counter)
tiling = collections.Counter()
examples = []
n = 0
for line in open(path, encoding="utf-8"):
    r = json.loads(line)
    n += 1
    if n % every:
        continue
    if r["k"] == "tocchunk":
        spans = [("tocchunk", r["cas"], r["off"], r["size"])]
    else:
        spans = [("manifest" if i == 0 else "bundledata", c, o, s) for i, (c, o, s) in enumerate(r["spans"])]
        spans = spans[:1] + random.sample(spans[1:], min(3, len(spans) - 1))
    for kind, c, o, s in spans:
        if kind == "manifest":
            continue
        try:
            bl, err = blocks(c, o, s)
        except OSError as e:
            tiling[f"oserror {e.__class__.__name__}"] += 1
            continue
        tiling["tiles" if err is None else "mismatch"] += 1
        if err and len(examples) < 8:
            examples.append((kind, f"{c:#x}", o, s, err, [f"{x[0]:#06x}" for x in bl[:4]]))
        for comp, _d, _c in bl:
            tally[kind][f"{comp:#06x}"] += 1
for kind, c in tally.items():
    print(kind, dict(c.most_common(12)))
print("tiling:", dict(tiling))
for e in examples:
    print("  mismatch example:", e)
