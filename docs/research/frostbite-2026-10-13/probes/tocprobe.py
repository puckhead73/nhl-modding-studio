"""Exploratory read-only probe of the binary superbundle TOC payload (NHL 22-24 PS4).

Hypothesis under test (derived from region arithmetic, not assumed):
  +00 bundle hash map offset   +04 bundle data offset   +08 bundle count
  +0C chunk hash map offset    +10 chunk guid offset    +14 chunk count
  +18 chunk entry offset  ...
Prints header words, region-size arithmetic and hex of the first entries of each region.
"""
import struct
import sys

HDR = 0x22C


def u32(b, o):
    return struct.unpack_from(">I", b, o)[0]


def show(path):
    data = open(path, "rb").read()
    body = data[HDR:] if data[:3] == b"\x00\xd1\xce" else data
    n = len(body)
    w = [u32(body, i * 4) for i in range(15)]
    print(f"# {path}  payload={n:#x} ({n})")
    print("  header:", " ".join(f"{x:#x}" for x in w))
    bh, bd, bc, ch, cg, cc, ce = w[0], w[1], w[2], w[3], w[4], w[5], w[6]
    print(f"  bundle hashmap {bh:#x}+{bc}*4={bh + bc * 4:#x} (next region {bd:#x})")
    if bc:
        print(f"  bundle data    {bd:#x}..{ch:#x}: {(ch - bd)} bytes = {(ch - bd) / bc:.3f} per bundle")
    if cc:
        print(f"  chunk hashmap  {ch:#x}+{cc}*4={ch + cc * 4:#x} (chunk guid at {cg:#x})")
        print(f"  chunk guids    {cg:#x}..{ce:#x}: {(ce - cg) / cc:.3f} per chunk")
    for i, x in enumerate(w[7:], 7):
        print(f"  word +{i * 4:#04x} = {x:#x} ({x})")
    for label, off, size in (("bundle[0..3]", bd, 16 * 4), ("chunkguid[0..3]", cg, 20 * 4), ("chunkentry[0..5]", ce, 12 * 6)):
        if off < n:
            print(f"  {label} @ {off:#x}:")
            chunk = body[off:off + size]
            for r in range(0, len(chunk), 16):
                print("     ", chunk[r:r + 16].hex(" "))
    tail = w[8] if len(w) > 8 else 0
    for off in sorted(set(x for x in w[7:] if 0x3C < x < n)):
        print(f"  @ {off:#x}: {body[off:off + 48].hex(' ')}  |{body[off:off + 48].decode('latin-1').encode('ascii', 'replace').decode()}|")


for p in sys.argv[1:]:
    show(p)
