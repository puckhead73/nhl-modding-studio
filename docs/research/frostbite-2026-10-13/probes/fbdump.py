"""Read-only Frostbite structure probe for NHL 22-24 PS4 installs.

Usage: python -I fbdump.py <mode> <path> [...]
  obj <file>       decode a DbObject file (D1CE01-wrapped or bare) and print it as JSON-ish text
  tochdr <file>    print the big-endian u32 header words of a binary superbundle TOC payload
Nothing is written anywhere except stdout.
"""
import json
import struct
import sys

HDR = 0x22C


def payload(data):
    if data[:4] == b"\x00\xd1\xce\x01":
        return data[HDR:], "D1CE01"
    if data[:4] == b"\x00\xd1\xce\x00":
        return data[HDR:], "D1CE00"
    return data, "bare"


def leb(buf, pos):
    result = shift = 0
    while True:
        b = buf[pos]
        pos += 1
        result |= (b & 0x7F) << shift
        if not b & 0x80:
            return result, pos
        shift += 7


def read_entry(buf, pos):
    """Returns (name, value, newpos) or (None, None, pos+1) on terminator."""
    t = buf[pos]
    pos += 1
    if t == 0:
        return None, None, pos
    name = None
    if not t & 0x80:
        end = buf.index(0, pos)
        name = buf[pos:end].decode("utf-8", "replace")
        pos = end + 1
    kind = t & 0x1F
    if kind in (0x01, 0x02):  # list / object
        size, pos = leb(buf, pos)
        end = pos + size
        items = []
        while pos < end:
            n, v, pos = read_entry(buf, pos)
            if n is None and v is None:
                break
            items.append((n, v))
        pos = end
        if kind == 0x02:
            return name, {n: v for n, v in items}, pos
        return name, [v for _, v in items], pos
    if kind == 0x06:
        return name, bool(buf[pos]), pos + 1
    if kind == 0x07:
        size, pos = leb(buf, pos)
        return name, buf[pos:pos + size].rstrip(b"\x00").decode("utf-8", "replace"), pos + size
    if kind == 0x08:
        return name, struct.unpack_from("<i", buf, pos)[0], pos + 4
    if kind == 0x09:
        return name, struct.unpack_from("<q", buf, pos)[0], pos + 8
    if kind in (0x0B, 0x0C):
        return name, struct.unpack_from("<d", buf, pos)[0], pos + 8
    if kind == 0x0F:
        return name, "guid:" + buf[pos:pos + 16].hex(), pos + 16
    if kind == 0x10:
        return name, "sha1:" + buf[pos:pos + 20].hex(), pos + 20
    if kind == 0x13:
        size, pos = leb(buf, pos)
        return name, f"<blob {size} bytes, head {buf[pos:pos + 16].hex()}>", pos + size
    raise ValueError(f"unknown DbObject type 0x{t:02x} at 0x{pos - 1:x}")


def mode_obj(path):
    data = open(path, "rb").read()
    body, kind = payload(data)
    name, value, end = read_entry(body, 0)
    print(f"# {path}: wrapper={kind} payload={len(body)} consumed={end} root_name={name!r}")
    print(json.dumps(value, indent=1, default=str)[:200000])


def mode_tochdr(path):
    data = open(path, "rb").read()
    body, kind = payload(data)
    words = struct.unpack_from(">" + "I" * 24, body, 0)
    print(f"# {path}: wrapper={kind} payload={len(body)}")
    for i, w in enumerate(words):
        print(f"  +0x{i * 4:02x}  {w:#010x}  {w}")


if __name__ == "__main__":
    mode, *paths = sys.argv[1:]
    for p in paths:
        {"obj": mode_obj, "tochdr": mode_tochdr}[mode](p)
