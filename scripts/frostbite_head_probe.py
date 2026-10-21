"""Decode bounded local MeshSet samples using an explicitly supplied Oodle DLL.

Usage: python scripts/frostbite_head_probe.py INVENTORY.csv OUTPUT --decoder DLL
The DLL is never copied into the output or repository. Game files are read-only.
"""
import argparse
import collections
import csv
import ctypes
import hashlib
import json
from pathlib import Path
import re
import struct


class Oodle:
    def __init__(self, path):
        self.dll = ctypes.CDLL(str(Path(path).resolve()))
        self.fn = self.dll.OodleLZ_Decompress
        self.fn.restype = ctypes.c_ssize_t
        self.fn.argtypes = [ctypes.c_void_p, ctypes.c_ssize_t, ctypes.c_void_p,
                           ctypes.c_ssize_t, ctypes.c_int, ctypes.c_int, ctypes.c_int,
                           ctypes.c_void_p, ctypes.c_ssize_t, ctypes.c_void_p,
                           ctypes.c_void_p, ctypes.c_void_p, ctypes.c_ssize_t, ctypes.c_int]

    def __call__(self, data, size):
        source = ctypes.create_string_buffer(data)
        output = ctypes.create_string_buffer(size)
        result = self.fn(source, len(data), output, size, 1, 0, 0, None, 0,
                         None, None, None, 0, 3)
        if result != size:
            raise ValueError(f"Oodle returned {result}, expected {size}")
        return output.raw


def decode_frames(data, decoder):
    offset, output = 0, bytearray()
    while offset < len(data):
        if offset + 8 > len(data):
            raise ValueError("Truncated frame")
        size = int.from_bytes(data[offset:offset + 4], "big")
        codec, flags, low = struct.unpack_from(">BBH", data, offset + 4)
        stored = (flags & 15) << 16 | low
        if flags & 0xF0 != 0x70 or not 0 < size <= 1024 * 1024 or not stored:
            raise ValueError("Unsupported frame header")
        if len(output) + size > 256 * 1024 * 1024:
            raise ValueError("Decoded asset exceeds limit")
        payload = data[offset + 8:offset + 8 + stored]
        if len(payload) != stored:
            raise ValueError("Truncated payload")
        if codec == 0:
            block = payload
        elif codec == 0x11:
            block = decoder(payload, size)
        else:
            raise ValueError(f"Unsupported codec {codec:#x}")
        if len(block) != size:
            raise ValueError("Decoded size mismatch")
        output.extend(block)
        offset += 8 + stored
    return bytes(output)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("inventory", type=Path)
    parser.add_argument("output", type=Path)
    parser.add_argument("--decoder", required=True, type=Path)
    parser.add_argument("--per-category", type=int, default=2)
    args = parser.parse_args()
    args.output.mkdir(parents=True, exist_ok=True)
    decoder = Oodle(args.decoder)
    counts, seen, records = collections.Counter(), set(), []
    with args.inventory.open(newline="", encoding="utf-8-sig") as source:
        for row in csv.DictReader(source):
            category = row["category"]
            if counts[category] >= args.per_category or row["sha1"] in seen:
                continue
            seen.add(row["sha1"])
            count = int(row["stored_bytes"])
            if count > 256 * 1024 * 1024:
                raise ValueError("Stored asset exceeds limit")
            with Path(row["physical_archive"]).open("rb") as archive:
                archive.seek(int(row["offset"]))
                raw = archive.read(count)
            if len(raw) != count:
                raise ValueError("Archive span beyond EOF")
            decoded = decode_frames(raw, decoder)
            name = f'{len(records):03}_{Path(row["name"]).name}.meshset'
            (args.output / name).write_bytes(decoded)
            strings = [{"offset": m.start(), "text": m.group().decode("ascii")}
                       for m in re.finditer(rb"[\x20-\x7e]{5,}", decoded)]
            records.append(dict(row, file=name, decoded_bytes=len(decoded),
                                sha256=hashlib.sha256(decoded).hexdigest(), strings=strings))
            counts[category] += 1
    (args.output / "samples.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
    print(json.dumps({"samples": len(records), "categories": counts}))


if __name__ == "__main__":
    main()
