"""Shared read-only helpers for the NHL 08 roster-schema research (standard library only).

Binary layout follows crates/tdb-core/src/lib.rs: file header 0x18 bytes, directory of 8-byte
(id, offset) entries, 0x28-byte table headers, 0x10-byte field descriptors, then the packed
record block. Nothing here writes anything.
"""
import re
import struct
import zlib
from collections import OrderedDict

FILE_HEADER = 0x18
TABLE_HEADER = 0x28
DESCRIPTOR = 0x10


def u32(b, o, be):
    return struct.unpack_from(">I" if be else "<I", b, o)[0]


def u16(b, o, be):
    return struct.unpack_from(">H" if be else "<H", b, o)[0]


# ---------------------------------------------------------------- CRC-32/MPEG-2 (tdb-core crc::crc32_mpeg2)
_MPEG2 = []
for _i in range(256):
    _c = _i << 24
    for _ in range(8):
        _c = ((_c << 1) ^ 0x04C11DB7) & 0xFFFFFFFF if _c & 0x80000000 else (_c << 1) & 0xFFFFFFFF
    _MPEG2.append(_c)


def crc32_mpeg2(data):
    c = 0xFFFFFFFF
    for x in data:
        c = ((c << 8) & 0xFFFFFFFF) ^ _MPEG2[((c >> 24) ^ x) & 0xFF]
    return c


def crc32_ieee(data):
    return zlib.crc32(data) & 0xFFFFFFFF


# ---------------------------------------------------------------- TDB parsing
def parse(b, base=0):
    """Parse one TDB at `base`. Returns a dict with header fields and a `tables` list."""
    h = b[base:]
    if h[:2] != b"DB":
        raise ValueError("no DB magic at %#x" % base)
    be = h[4] == 1
    ver = u16(h, 2, be)
    info = dict(base=base, version=ver, big_endian=be, db_size=u32(h, 8, be), hdr_0c=u32(h, 0xC, be),
                table_count=u32(h, 0x10, be), header_crc=u32(h, 0x14, be), tables=[])
    ntab = info["table_count"]
    dir_end = FILE_HEADER + 8 * ntab
    for i in range(ntab):
        o = FILE_HEADER + 8 * i
        rid = h[o:o + 4].decode("latin1")
        off = u32(h, o + 4, be)
        a = dir_end + off
        t = dict(raw=rid, canonical=rid[::-1] if be else rid, dir_index=i, block=base + a,
                 prior_crc=u32(h, a, be), alloc=u32(h, a + 4, be), rec_bytes=u32(h, a + 8, be),
                 rec_bits=u32(h, a + 12, be), unk10=u32(h, a + 0x10, be), max=u16(h, a + 0x14, be),
                 cur=u16(h, a + 0x16, be), unk18=u32(h, a + 0x18, be), nfields=h[a + 0x1C],
                 nindex=h[a + 0x1D], unk1e=u16(h, a + 0x1E, be), unk20=u32(h, a + 0x20, be),
                 header_crc=u32(h, a + 0x24, be), fields=[])
        d = a + TABLE_HEADER
        for _ in range(t["nfields"]):
            fid = h[d + 8:d + 12].decode("latin1")
            t["fields"].append(dict(kind=u32(h, d, be), bit=u32(h, d + 4, be), raw=fid,
                                    canonical=fid[::-1] if be else fid, width=u32(h, d + 12, be)))
            d += DESCRIPTOR
        t["record_block"] = base + d
        t["block_end"] = base + d + t["rec_bytes"] * t["max"]
        info["tables"].append(t)
    return info


def scan(b):
    """Every plausible embedded TDB header: list of (offset, db_size, table_count)."""
    hits = []
    i = 0
    while True:
        i = b.find(b"DB\x00", i)
        if i < 0:
            break
        if i + FILE_HEADER <= len(b) and b[i + 4] in (0, 1):
            try:
                p = parse(b, i)
                if 0 < p["table_count"] < 500 and FILE_HEADER < p["db_size"] <= len(b) - i:
                    hits.append((i, p["db_size"], p["table_count"]))
            except Exception:
                pass
        i += 1
    return hits


def verify_crcs(b, p):
    """Recompute the four inner-DB CRC scopes exactly as tdb-core::recompute_crcs does.
    Returns dict(header=bool, prior=[bool...], table_header=[bool...], eof=(stored, computed))."""
    base = p["base"]
    be = p["big_endian"]
    db = b[base:base + p["db_size"]]
    tables = sorted(p["tables"], key=lambda t: t["block"])
    n = len(db)
    res = dict(header=crc32_mpeg2(db[:0x14]) == p["header_crc"], prior=[], table_header=[], eof=None)
    dir_range = db[FILE_HEADER:FILE_HEADER + 8 * p["table_count"]]
    for k, t in enumerate(tables):
        blk = t["block"] - base
        nxt = (tables[k + 1]["block"] - base) if k + 1 < len(tables) else n - 4
        if k == 0:
            scope = crc32_mpeg2(dir_range)
        else:
            pb = tables[k - 1]["block"] - base
            scope = crc32_mpeg2(db[pb + TABLE_HEADER:blk])
        res["prior"].append(scope == t["prior_crc"])
        res["table_header"].append(crc32_mpeg2(db[blk + 4:blk + 36]) == t["header_crc"])
        if k == len(tables) - 1:
            stored = u32(db, n - 4, be)
            res["eof"] = (stored, crc32_mpeg2(db[blk + TABLE_HEADER:nxt]))
    return res


# ---------------------------------------------------------------- record decoding
def read_bits(data, off, width, be=True):
    bs = off // 8
    e = (off + width + 7) // 8
    span = data[bs:e]
    acc = int.from_bytes(span, "big" if be else "little")
    if be:
        return (acc >> (len(span) * 8 - (off - bs * 8) - width)) & ((1 << width) - 1)
    return (acc >> (off - bs * 8)) & ((1 << width) - 1)


def record(b, t, i):
    s = t["record_block"] + i * t["rec_bytes"]
    return b[s:s + t["rec_bytes"]]


def decode(rec, t, be=True):
    out = OrderedDict()
    for f in t["fields"]:
        if f["kind"] == 0:
            raw = rec[f["bit"] // 8:(f["bit"] + f["width"]) // 8]
            out[f["raw"]] = raw.split(b"\0")[0].decode("latin1")
        else:
            out[f["raw"]] = read_bits(rec, f["bit"], f["width"], be)
    return out


def column(b, t, raw_field, be=True):
    f = next(f for f in t["fields"] if f["raw"] == raw_field)
    if f["kind"] == 0:
        return [record(b, t, i)[f["bit"] // 8:(f["bit"] + f["width"]) // 8].split(b"\0")[0].decode("latin1")
                for i in range(t["cur"])]
    return [read_bits(record(b, t, i), f["bit"], f["width"], be) for i in range(t["cur"])]


def table(p, raw):
    return next(t for t in p["tables"] if t["raw"] == raw)


# ---------------------------------------------------------------- meta XML
def load_meta(path):
    """OrderedDict shortname -> table attrs (+ 'fields': list of field attr dicts in document order,
    'order': document index). Plain regex parsing keeps the exact attribute strings."""
    x = open(path, encoding="utf-8-sig", errors="replace").read()
    meta = OrderedDict()
    for i, m in enumerate(re.finditer(r'<table\b([^>]*)>(.*?)</table>', x, re.S)):
        a = dict(re.findall(r'(\w+)="([^"]*)"', m.group(1)))
        a["fields"] = [dict(re.findall(r'(\w+)="([^"]*)"', q.group(1)))
                       for q in re.finditer(r'<field\b([^>]*)/>', m.group(2))]
        a["order"] = i
        meta[a["shortname"]] = a
    return meta


def roles(meta):
    """save table id -> (parent shortname, role) from the meta's own declarations."""
    out = {}
    for sn, a in meta.items():
        for attr, role in (("insertshortname", "insert"), ("deleteshortname", "delete"), ("updateshortname", "update")):
            sid = a.get(attr)
            if sid:
                if sid in out:
                    raise ValueError("save id %s declared twice: %s and %s" % (sid, out[sid], (sn, role)))
                out[sid] = (sn, role)
    return out


def keys_of(a):
    return [f["shortname"] for f in a["fields"] if f.get("key") == "True"]


def update_fields_of(a):
    return [f["shortname"] for f in a["fields"] if f.get("update") == "True"]


def field_names(a):
    return {f["shortname"]: f["name"] for f in a["fields"]}


def capacity(a, role, parent_table):
    """Declared capacity for a role; -1 means the parent's max_records."""
    attr = {"insert": "maxinsert", "delete": "maxdelete", "update": "maxupdate"}[role]
    v = a.get(attr)
    if v is None:
        return None, "undeclared"
    v = int(v)
    if v == -1:
        return (parent_table["max"] if parent_table else None), "-1 (parent max_records)"
    return v, "declared"
