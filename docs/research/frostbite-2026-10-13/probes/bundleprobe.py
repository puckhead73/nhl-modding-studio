"""Read-only probe: decode a candidate binary bundle manifest at <cas> <offset>.

Hypothesised layout (to be validated by the result being self-consistent and the strings being
readable asset names):
  u32 BE manifest size, then (little-endian): magic, total, ebx_count, res_count, chunk_count,
  strings_off, meta_off, meta_size; total * sha1(20); ebx_count * (name_off, size);
  res_count * (name_off, size); res_count * res_type; res_count * meta(16); res_count * rid(u64);
  chunk_count * (guid16, logical_off, logical_size); strings at 4 + strings_off.
"""
import struct
import sys


def cstr(buf, off):
    end = buf.index(0, off)
    return buf[off:end].decode("utf-8", "replace")


def probe(path, off, limit=40):
    with open(path, "rb") as f:
        f.seek(off)
        head = f.read(8)
        size = struct.unpack(">I", head[:4])[0]
        f.seek(off)
        b = f.read(4 + size)
    magic, total, ne, nr, nc, so, mo, ms = struct.unpack_from("<8I", b, 4)
    print(f"# {path} @ {off:#x}: size={size:#x} magic={magic:#010x} total={total} ebx={ne} res={nr} chunks={nc} strings_off={so:#x} meta=({mo:#x},{ms})")
    assert total == ne + nr + nc, "total != ebx+res+chunks"
    p = 4 + 32 + total * 20
    ebx = [struct.unpack_from("<II", b, p + i * 8) for i in range(ne)]
    p += ne * 8
    res = [struct.unpack_from("<II", b, p + i * 8) for i in range(nr)]
    p += nr * 8
    rtypes = [struct.unpack_from("<I", b, p + i * 4)[0] for i in range(nr)]
    p += nr * 4 + nr * 16 + nr * 8
    chunks = [(b[p + i * 24:p + i * 24 + 16].hex(),) + struct.unpack_from("<II", b, p + i * 24 + 16) for i in range(nc)]
    sbase = 4 + so
    print(f"  ebx[{ne}]:")
    for n, s in ebx[:limit]:
        print(f"    {cstr(b, sbase + n)}  ({s} bytes)")
    print(f"  res[{nr}]:")
    for (n, s), t in list(zip(res, rtypes))[:limit]:
        print(f"    {cstr(b, sbase + n)}  type={t:#010x} ({s} bytes)")
    print(f"  chunks[{nc}]: first {chunks[:3]}")
    return 4 + size


if __name__ == "__main__":
    path, off = sys.argv[1], int(sys.argv[2], 0)
    n = int(sys.argv[3]) if len(sys.argv) > 3 else 1
    for _ in range(n):
        off += probe(path, off)
