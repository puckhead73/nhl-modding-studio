"""C0-POR1 / C0-POR1-R1 inspection parsers and decoders.

Preserved so the portrait findings can be re-derived or challenged. This module reads bytes and
returns header fields; it never writes to any source location.

WHAT THIS IS AND IS NOT
-----------------------
Every routine here is an *inspection* parser written for the C0-POR1 package. Accepting a file
means only that these routines found the fields they look for at the offsets they expect. It is
NOT semantic validation, and it is NOT a qualification of this repository's own parsers
(`tdb-rx2`, `tdb-archive-big`, `tdb-archive-codec`), which were not exercised — with one
documented exception:

  `xbox360_tiled_offset` is TRANSCRIBED from this repository's
  `crates/tdb-rx2/src/texlib.rs::xg_address_2d_tiled`. It is therefore NOT independent of the
  repository. It is kept here so the transcription is reviewable.

INDEPENDENCE OF STAGES (see the R1 receipt, "shared versus independent stages")
------------------------------------------------------------------------------
  PS3   chain: parse_bigf -> refpack_decode -> parse_dds  -> decode_dxt5
  X360  chain: parse_bigf -> refpack_decode -> parse_xpr2 -> swap16 -> untile -> decode_dxt5

`parse_bigf`, `refpack_decode` and `decode_dxt5` are the SAME code on both chains. Agreement
between a PS3 and an Xbox 360 file therefore tests only the stages that differ between them
(`parse_xpr2`, `swap16`, the tiling formula); it cannot detect a fault shared by both.
"""

import struct

REFPACK_MAGIC = bytes([0x10, 0xFB])


# --------------------------------------------------------------------------- RefPack

def refpack_decode(data, limit=None):
    """Decompress an EA RefPack stream.

    Returns (declared_size, output). With `limit`, stops once that many bytes have been produced
    -- used by the census to reach a texture header without expanding the whole payload.

    Written for this package from the published RefPack command encoding; not derived from this
    repository's `tdb-archive-codec::refpack`.
    """
    i = 0
    flags, magic = data[i], data[i + 1]
    i += 2
    if magic != 0xFB:
        raise ValueError("not a refpack stream")
    if flags & 0x01:
        i += 3  # compressed-size field present
    declared = int.from_bytes(data[i:i + 3], "big")
    i += 3

    out = bytearray()
    while i < len(data) and (limit is None or len(out) < limit):
        b0 = data[i]
        if b0 < 0x80:
            b1 = data[i + 1]
            i += 2
            n = b0 & 0x03
            out += data[i:i + n]
            i += n
            off = ((b0 & 0x60) << 3) | b1
            cnt = ((b0 >> 2) & 0x07) + 3
            s = len(out) - off - 1
            for k in range(cnt):
                out.append(out[s + k])
        elif b0 < 0xC0:
            b1, b2 = data[i + 1], data[i + 2]
            i += 3
            n = (b1 >> 6) & 0x03
            out += data[i:i + n]
            i += n
            off = ((b1 & 0x3F) << 8) | b2
            cnt = (b0 & 0x3F) + 4
            s = len(out) - off - 1
            for k in range(cnt):
                out.append(out[s + k])
        elif b0 < 0xE0:
            b1, b2, b3 = data[i + 1], data[i + 2], data[i + 3]
            i += 4
            n = b0 & 0x03
            out += data[i:i + n]
            i += n
            off = ((b0 & 0x10) << 12) | (b1 << 8) | b2
            cnt = (((b0 & 0x0C) << 6) | b3) + 5
            s = len(out) - off - 1
            for k in range(cnt):
                out.append(out[s + k])
        elif b0 < 0xFC:
            n = ((b0 & 0x1F) << 2) + 4
            i += 1
            out += data[i:i + n]
            i += n
        else:
            n = b0 & 0x03
            i += 1
            out += data[i:i + n]
            i += n
            break
    return declared, bytes(out)


# --------------------------------------------------------------------------- BIGF

def parse_bigf(data):
    """Parse an EA `BIGF` archive header.

    Returns None when the magic is absent -- the caller decides whether that is an error.

    Observed layout (see the R1 receipt): the u32 at 0x04 is LITTLE-endian while the member count,
    header size and every member offset/length are BIG-endian.
    """
    if data[:4] != b"BIGF":
        return None
    size_le = struct.unpack("<I", data[4:8])[0]
    count = struct.unpack(">I", data[8:12])[0]
    header_size = struct.unpack(">I", data[12:16])[0]
    p = 16
    members = []
    for _ in range(count):
        if p + 8 > len(data):
            raise ValueError("member table runs past end of file")
        off, ln = struct.unpack(">II", data[p:p + 8])
        p += 8
        end = data.index(b"\0", p)
        members.append({"name": data[p:end].decode("latin1"), "offset": off, "length": ln})
        p = end + 1
    return {
        "size_field_le": size_le,
        "member_count": count,
        "header_size": header_size,
        "name_table_end": p,
        "members": members,
    }


# --------------------------------------------------------------------------- DDS

def parse_dds(data):
    """Parse a 124-byte `DDS ` header. Fields are read verbatim; nothing is inferred."""
    if data[:4] != b"DDS ":
        return None
    size, flags, height, width, pitch_or_linear, depth, mips = struct.unpack("<IIIIIII", data[4:32])
    pf_size, pf_flags = struct.unpack("<II", data[76:84])
    fourcc = data[84:88].decode("latin1")
    rgb_bits = struct.unpack("<I", data[88:92])[0]
    caps, caps2 = struct.unpack("<II", data[108:116])
    return {
        "header_size": size,
        "flags": flags,
        "width": width,
        "height": height,
        "pitch_or_linear_size": pitch_or_linear,
        "depth": depth,
        "mip_count": mips,
        "pf_size": pf_size,
        "pf_flags": pf_flags,
        "fourcc": fourcc,
        "rgb_bit_count": rgb_bits,
        "caps": caps,
        "caps2": caps2,
        "payload_offset": 128,
    }


# --------------------------------------------------------------------------- XPR2

# Offsets located by inspection on NHL Legacy Xbox 360 portrait payloads. The GPU fetch constant
# is six big-endian dwords at 0x48; the pixel payload begins at 0x60.
XPR2_FETCH_CONSTANT_OFFSET = 0x48
XPR2_PAYLOAD_OFFSET = 0x60


def parse_xpr2(data):
    """Parse an `XPR2` package header and its embedded GPU texture fetch constant.

    The bitfield readings below are INTERPRETATIONS of the Xenos texture fetch constant, not
    self-describing fields. They are reported alongside the raw dwords so a reviewer can check
    them.
    """
    if data[:4] != b"XPR2":
        return None
    header_size = struct.unpack(">I", data[4:8])[0]
    data_size = struct.unpack(">I", data[8:12])[0]
    count = struct.unpack(">I", data[12:16])[0]
    kind = data[16:20].decode("latin1")
    dwords = struct.unpack(">6I", data[XPR2_FETCH_CONSTANT_OFFSET:XPR2_FETCH_CONSTANT_OFFSET + 24])
    size2d = dwords[2]
    return {
        "header_size_field": header_size,
        "data_size": data_size,
        "resource_count": count,
        "resource_type": kind,
        "fetch_constant": [f"0x{d:08x}" for d in dwords],
        # --- interpreted fields ---
        "width": (size2d & 0x1FFF) + 1,
        "height": ((size2d >> 13) & 0x1FFF) + 1,
        "tiled_bit": dwords[0] >> 31,
        "pitch_texels": ((dwords[0] >> 22) & 0x1FF) * 32,
        "data_format_bits": dwords[1] & 0x3F,
        "mip_descriptor_dword": f"0x{dwords[3]:08x}",
        "payload_offset": XPR2_PAYLOAD_OFFSET,
    }


# --------------------------------------------------------------------------- GNF

GNF_PAYLOAD_OFFSET = 0x100


def parse_gnf(data):
    """Parse a PS4 `GNF ` header and its GNM T# descriptor.

    Every field below the raw dwords is an INTERPRETATION of the T# bit layout (the same layout
    documented in this repository's `crates/tdb-rx2/src/gnf.rs`), not a self-describing field.
    `data_format` and `num_format` are raw numeric codes here; mapping them to a codec name is a
    table lookup the caller must make explicitly.
    """
    if data[:4] != b"GNF ":
        return None
    contents_size = struct.unpack("<I", data[4:8])[0]
    version, alignment = data[8], data[9]
    t = struct.unpack("<8I", data[16:48])
    return {
        "contents_size": contents_size,
        "version": version,
        "alignment": alignment,
        "tsharp": [f"0x{d:08x}" for d in t],
        # --- interpreted fields ---
        "data_format": (t[1] >> 20) & 0x3F,
        "num_format": (t[1] >> 26) & 0x0F,
        "width": (t[2] & 0x3FFF) + 1,
        "height": ((t[2] >> 14) & 0x3FFF) + 1,
        "dst_sel_x": (t[2] >> 28) & 0x07,
        "tiling_index": (t[3] >> 20) & 0x1F,
        "last_level": (t[3] >> 16) & 0x0F,
        "payload_offset": GNF_PAYLOAD_OFFSET,
    }


# --------------------------------------------------------------------------- Xbox 360 layout

def swap16(data):
    """Byte-swap each 16-bit word. Written for this package."""
    out = bytearray(data)
    for i in range(0, len(out) - 1, 2):
        out[i], out[i + 1] = out[i + 1], out[i]
    return bytes(out)


def xbox360_tiled_offset(x, y, width, element_pitch):
    """TRANSCRIBED from `crates/tdb-rx2/src/texlib.rs::xg_address_2d_tiled`.

    NOT independent of this repository. `x`, `y` and `width` are in elements (4x4 blocks for DXT);
    the return value is a byte offset.
    """
    aligned_w = (width + 31) & ~31
    log_bpp = (element_pitch >> 2) + ((element_pitch >> 1) >> (element_pitch >> 2))
    macro = ((x >> 5) + (y >> 5) * (aligned_w >> 5)) << (log_bpp + 7)
    micro = ((x & 7) + ((y & 6) << 2)) << log_bpp
    offset = (macro
              + ((micro & ~0xF) << 1)
              + (micro & 0xF)
              + ((y & 8) << (3 + log_bpp))
              + ((y & 1) << 4))
    return (((offset & ~0x1FF) << 3)
            + ((y & 16) << 7)
            + ((offset & 0x1C0) << 2)
            + (((((y & 8) >> 2) + (x >> 3)) & 3) << 6)
            + (offset & 0x3F))


def untile_dxt(tiled, width, height, bytes_per_block=16):
    """Reorder a tiled block stream into row-major order using `xbox360_tiled_offset`."""
    bw, bh = width // 4, height // 4
    out = bytearray(bw * bh * bytes_per_block)
    for by in range(bh):
        for bx in range(bw):
            src = xbox360_tiled_offset(bx, by, bw, bytes_per_block)
            dst = (by * bw + bx) * bytes_per_block
            out[dst:dst + bytes_per_block] = tiled[src:src + bytes_per_block]
    return bytes(out)


# --------------------------------------------------------------------------- DXT5

def decode_dxt5(data, width, height):
    """Decode a row-major DXT5 block stream to RGBA8. Written for this package."""
    px = bytearray(width * height * 4)
    bw = width // 4
    o = 0
    for by in range(height // 4):
        for bx in range(bw):
            a0, a1 = data[o], data[o + 1]
            abits = int.from_bytes(data[o + 2:o + 8], "little")
            c0, c1 = struct.unpack("<HH", data[o + 8:o + 12])
            cbits = struct.unpack("<I", data[o + 12:o + 16])[0]
            o += 16

            def rgb(c):
                return (((c >> 11) & 31) * 255 // 31,
                        ((c >> 5) & 63) * 255 // 63,
                        (c & 31) * 255 // 31)

            r0, g0, b0 = rgb(c0)
            r1, g1, b1 = rgb(c1)
            cols = [(r0, g0, b0), (r1, g1, b1),
                    ((2 * r0 + r1) // 3, (2 * g0 + g1) // 3, (2 * b0 + b1) // 3),
                    ((r0 + 2 * r1) // 3, (g0 + 2 * g1) // 3, (b0 + 2 * b1) // 3)]
            if a0 > a1:
                al = [a0, a1] + [((6 - i) * a0 + (i + 1) * a1) // 7 for i in range(6)]
            else:
                al = [a0, a1] + [((4 - i) * a0 + (i + 1) * a1) // 5 for i in range(4)] + [0, 255]

            for py in range(4):
                for pxx in range(4):
                    i = py * 4 + pxx
                    c = cols[(cbits >> (2 * i)) & 3]
                    a = al[(abits >> (3 * i)) & 7]
                    q = ((by * 4 + py) * width + (bx * 4 + pxx)) * 4
                    px[q:q + 4] = bytes((c[0], c[1], c[2], a))
    return bytes(px)


# --------------------------------------------------------------------------- helpers

def texture_payload(data, header):
    """Return (member, declared_size, decoded_bytes) for the largest RefPack member of a BIGF.

    Picking the LARGEST RefPack member is a heuristic of this tool, recorded because it matters:
    smaller RefPack members carrying APT resources also exist. The census cross-checks the choice
    by requiring the decoded result to begin with a `DDS ` or `XPR2` magic.
    """
    best = None
    for m in header["members"]:
        if m["length"] >= 64 and data[m["offset"]:m["offset"] + 2] == REFPACK_MAGIC:
            if best is None or m["length"] > best["length"]:
                best = m
    if best is None:
        return None, None, None
    declared, out = refpack_decode(data[best["offset"]:best["offset"] + best["length"]])
    return best, declared, out
