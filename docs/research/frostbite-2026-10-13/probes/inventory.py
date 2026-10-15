"""Read-only Frostbite (NHL 22-24 PS4) asset inventory.

Walks layout.toc -> every superbundle .toc (binary) -> every bundle record -> the bundle manifest
in its cas file, and every chunk entry. Writes one JSON-lines catalogue per (title, root) plus a
summary JSON into <out_dir>. Every structural assumption is checked per record; failures are
counted and sampled rather than skipped silently.

Usage: python -I inventory.py <install_root> <root: Data|Patch> <out_dir> <tag>
"""
import collections
import json
import os
import struct
import sys

HDR = 0x22C


def be(b, o):
    return struct.unpack_from(">I", b, o)[0]


def leb(buf, pos):
    result = shift = 0
    while True:
        c = buf[pos]
        pos += 1
        result |= (c & 0x7F) << shift
        if not c & 0x80:
            return result, pos
        shift += 7


def dbobj(buf, pos):
    t = buf[pos]
    pos += 1
    if t == 0:
        return None, None, pos
    name = None
    if not t & 0x80:
        end = buf.index(0, pos)
        name = buf[pos:end].decode()
        pos = end + 1
    k = t & 0x1F
    if k in (1, 2):
        size, pos = leb(buf, pos)
        end = pos + size
        items = []
        while pos < end:
            n, v, pos = dbobj(buf, pos)
            if n is None and v is None:
                break
            items.append((n, v))
        return name, (dict(items) if k == 2 else [v for _, v in items]), end
    if k == 6:
        return name, bool(buf[pos]), pos + 1
    if k == 7:
        size, pos = leb(buf, pos)
        return name, buf[pos:pos + size].rstrip(b"\0").decode(), pos + size
    if k == 8:
        return name, struct.unpack_from("<i", buf, pos)[0], pos + 4
    if k == 9:
        return name, struct.unpack_from("<q", buf, pos)[0], pos + 8
    if k == 0x0F:
        return name, buf[pos:pos + 16].hex(), pos + 16
    if k == 0x10:
        return name, buf[pos:pos + 20].hex(), pos + 20
    if k == 0x13:
        size, pos = leb(buf, pos)
        return name, buf[pos:pos + size], pos + size
    raise ValueError(f"DbObject type {t:#x}")


class CasFiles:
    """Open cas files lazily. A cas id word is read as: byte1 = root (0 Data, 1 Patch), byte2 = package, byte3 = cas."""

    def __init__(self, install, root):
        self.install, self.root, self.handles = install, root, {}

    def path(self, cas_id):
        flag, pkg, cas = (cas_id >> 16) & 0xFF, (cas_id >> 8) & 0xFF, cas_id & 0xFF
        root = {0: "Data", 1: "Patch"}[flag]  # absolute: verified on NHL 24 patch records
        return os.path.join(self.install, root, "Gen4b", "superbundlelayout", f"nhl_installpackage_{pkg:02d}", f"cas_{cas:02d}.cas")

    def read(self, cas_id, off, size):
        p = self.path(cas_id)
        if p not in self.handles:
            self.handles[p] = open(p, "rb")
        f = self.handles[p]
        f.seek(off)
        return f.read(size)


def cstr(b, o):
    return b[o:b.index(0, o)].decode("utf-8", "replace")


def manifest(b):
    size = be(b, 0)
    magic, total, ne, nr, nc, so, _mo, _ms = struct.unpack_from("<8I", b, 4)
    if total != ne + nr + nc or so >= size:
        raise ValueError("manifest counts inconsistent")
    p = 4 + 32 + total * 20
    ebx = [struct.unpack_from("<II", b, p + i * 8) for i in range(ne)]
    p += ne * 8
    res = [struct.unpack_from("<II", b, p + i * 8) for i in range(nr)]
    p += nr * 8
    rtypes = [struct.unpack_from("<I", b, p + i * 4)[0] for i in range(nr)]
    p += nr * 4 + nr * 16 + nr * 8
    chunks = [(b[p + i * 24:p + i * 24 + 16].hex(),) + struct.unpack_from("<II", b, p + i * 24 + 16) for i in range(nc)]
    sb = 4 + so
    return magic, [(cstr(b, sb + n), s) for n, s in ebx], [(cstr(b, sb + n), s, t) for (n, s), t in zip(res, rtypes)], chunks


def walk_toc(install, root, sbname, cas, out, stats):
    path = os.path.join(install, root, sbname + ".toc")
    if not os.path.exists(path):
        stats["missing_toc"].append(sbname)
        return
    raw = open(path, "rb").read()
    body = raw[HDR:] if raw[:3] == b"\x00\xd1\xce" else raw
    w = [be(body, i * 4) for i in range(15)]
    bh, bd, bc, ch, cg, cc, ce = w[:7]
    ndata = w[10]
    st = stats["superbundles"].setdefault(sbname, collections.Counter())
    st["bundles"] = bc
    st["chunks_toc"] = cc
    if ndata != cc * 3:
        stats["anomalies"].append(f"{sbname}: chunk data words {ndata} != 3*{cc}")
    # chunks listed directly in the toc
    for i in range(cc):
        guid = body[cg + i * 20:cg + i * 20 + 16].hex()
        ref = be(body, cg + i * 20 + 16)
        idx = ref & 0xFFFFFF
        cid, off, size = (be(body, ce + (idx + k) * 4) for k in range(3))
        out.write(json.dumps({"k": "tocchunk", "sb": sbname, "guid": guid, "flags": ref >> 24, "cas": cid, "off": off, "size": size}) + "\n")
    # bundles
    for i in range(bc):
        nm, szf, off = struct.unpack_from(">IIQ", body, bd + i * 16)
        rec = off + 8
        try:
            rsize, count = be(body, rec), be(body, rec + 4)
            # The span stream (8 bytes per span plus a 4-byte cas id before every span whose flag
            # is set) is followed by one flag byte per span. Solve for the number of cas ids k:
            # the flags at rec+0x18+8*count+4*k must have exactly k non-zero bytes, the first set.
            # The bundle entry's size word (low 24 bits) ends exactly at the last flag byte:
            # off + size == flags_off + count. That fixes k directly; the flag count is checked.
            end = off + (szf & 0xFFFFFF)
            k, rem = divmod(end - count - (rec + 0x18 + 8 * count), 4)
            flags_off = end - count
            fl = body[flags_off:end]
            if rem or k < 1 or not fl or not fl[0] or sum(1 for x in fl if x) != k:
                raise ValueError(f"record shape: k={k} rem={rem} flags={fl[:8].hex()} (rsize {rsize:#x}, entry size {szf:#x})")
            flags = body[flags_off:flags_off + count]
            p = rec + 0x18
            spans, cur = [], None
            for j in range(count):
                if flags[j]:
                    cur = be(body, p)
                    p += 4
                spans.append((cur, be(body, p), be(body, p + 4)))
                p += 8
            if p != flags_off:
                raise ValueError(f"record shape: stream end {p:#x} vs flags {flags_off:#x}")
            stats["flag_values"].update(str(x) for x in flags)
            cid, moff, msize = spans[0]
            mb = cas.read(cid, moff, msize)
            magic, ebx, res, chunks = manifest(mb)
            st["ebx"] += len(ebx)
            st["res"] += len(res)
            st["bundle_chunks"] += len(chunks)
            stats["magics"][f"{magic:#x}"] += 1
            stats["cas_files"][cas.path(cid)] += 1
            out.write(json.dumps({"k": "bundle", "sb": sbname, "i": i, "nameword": nm, "spans": spans, "ebx": ebx, "res": res, "chunks": chunks}) + "\n")
        except Exception as e:  # noqa: BLE001 - every failure is counted and sampled
            st["bundle_errors"] += 1
            if len(stats["errors"]) < 20:
                stats["errors"].append(f"{sbname}#{i}: {e}")


def main():
    install, root, out_dir, tag = sys.argv[1:5]
    raw = open(os.path.join(install, root, "layout.toc"), "rb").read()
    _, layout, _ = dbobj(raw[HDR:], 0)
    stats = {"superbundles": {}, "anomalies": [], "errors": [], "missing_toc": [], "magics": collections.Counter(), "cas_files": collections.Counter(), "flag_values": collections.Counter()}
    cas = CasFiles(install, root)
    os.makedirs(out_dir, exist_ok=True)
    with open(os.path.join(out_dir, f"{tag}.jsonl"), "w", encoding="utf-8") as out:
        for sb in layout["superBundles"]:
            walk_toc(install, root, sb["name"], cas, out, stats)
    stats["layout_base"] = layout.get("base")
    stats["cas_files"] = dict(stats["cas_files"])
    json.dump(stats, open(os.path.join(out_dir, f"{tag}.summary.json"), "w"), indent=1, default=str)
    for sb, c in stats["superbundles"].items():
        print(tag, sb, dict(c))
    print(tag, "errors:", stats["errors"][:5], "anomalies:", stats["anomalies"][:5], "magics:", dict(stats["magics"]), "base:", stats["layout_base"])


if __name__ == "__main__":
    main()
