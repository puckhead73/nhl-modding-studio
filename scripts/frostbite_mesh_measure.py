"""Qualify measured PS4 MeshSet streams against their exact bundle manifests.

Private research only: python scripts/frostbite_mesh_measure.py SAMPLES INVENTORY --decoder DLL
"""
import argparse
import json
import math
from pathlib import Path
import struct
from frostbite_head_probe import Oodle, decode_frames


def read_asset(row, decoder):
    span = row["span"]
    if span["size"] > 256 * 1024 * 1024:
        raise ValueError("Oversized asset")
    with open(row["physical"], "rb") as stream:
        stream.seek(span["offset"])
        raw = stream.read(span["size"])
    if len(raw) != span["size"]:
        raise ValueError("Truncated span")
    return decode_frames(raw, decoder)


def measure(data, chunk, metadata=None):
    u32 = lambda p: struct.unpack_from("<I", data, p)[0]
    u64 = lambda p: struct.unpack_from("<Q", data, p)[0]
    lods = struct.unpack_from("<H", data, 0x94)[0]
    assert 1 <= lods <= 6
    result = []
    for level in range(lods):
        lod = u64(0x20 + level * 8)
        assert u32(lod + 0x54) == 0x21  # Observed 16-bit index declaration.
        count, sections = u32(lod + 8), u64(lod + 12)
        index_bytes, vertex_bytes = u32(lod + 0x58), u32(lod + 0x5c)
        guid = data[lod + 0x60:lod + 0x70].hex()
        if guid == "0" * 32 and metadata:
            start, size = struct.unpack_from("<II", bytes.fromhex(metadata))
            offset = u32(lod + 0x70)
            buffer = data[start + offset:start + min(size, offset + vertex_bytes + index_bytes)]
        else:
            buffer = chunk(guid)
        assert vertex_bytes + index_bytes <= len(buffer)
        for index in range(count):
            section = sections + index * 0xe0
            triangles, first_index, vertex_start, vertices = struct.unpack_from("<4I", data, section + 0x20)
            name = data[u64(section + 8):].split(b"\0", 1)[0].decode()
            element_count, stream_count = data[section + 0xac:section + 0xae]
            assert 0 < element_count <= 16 and 0 < stream_count <= 16
            strides = [data[section + 0x8c + i * 2] for i in range(stream_count)]
            assert vertex_start + vertices * sum(strides) <= vertex_bytes
            attributes = {}
            for e in range(element_count):
                usage, fmt, offset, stream = data[section + 0x4c + e * 4:section + 0x50 + e * 4]
                if usage not in (1, 0x21):
                    continue
                assert stream < stream_count
                encoding = {1: {3: "<3f", 8: "<4e"}, 0x21: {6: "<2e", 2: "<2f"}}[usage][fmt]
                size = struct.calcsize(encoding)
                assert offset + size <= strides[stream]
                start = vertex_start + vertices * sum(strides[:stream]) + offset
                values = [struct.unpack_from(encoding, buffer, start + i * strides[stream]) for i in range(vertices)]
                assert all(math.isfinite(v) for row in values for v in row)
                attributes[usage] = values
            assert 1 in attributes
            offset = vertex_bytes + first_index * 2
            assert offset + triangles * 6 <= vertex_bytes + index_bytes
            indices = struct.unpack_from(f"<{triangles * 3}H", buffer, offset)
            assert all(i < vertices for i in indices)
            positions = attributes[1]
            result.append(dict(lod=level, section=index, name=name, guid=guid,
                               vertices=vertices, triangles=triangles, has_uv0=0x21 in attributes,
                               min=[min(v[a] for v in positions) for a in range(3)],
                               max=[max(v[a] for v in positions) for a in range(3)]))
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("samples", type=Path)
    parser.add_argument("inventory", type=Path)
    parser.add_argument("--decoder", required=True)
    args = parser.parse_args()
    samples = json.loads((args.samples / "samples.json").read_text())
    wanted = {s["bundle"] for s in samples}
    rows = []
    with args.inventory.open() as stream:
        for line in stream:
            if any(bundle in line for bundle in wanted):
                rows.append(json.loads(line))
    (args.samples / "bundle-members.json").write_text(json.dumps(rows, indent=2))
    decoder, results = Oodle(args.decoder), []
    for sample in samples:
        try:
            source = next(r for r in rows if r["kind"] == "res" and r["sha1"] == sample["sha1"]
                          and r["physical"] == sample["physical_archive"]
                          and r["span"]["offset"] == int(sample["offset"]) and r["toc"] == sample["toc"])
            def chunk(guid):
                candidates = [r for r in rows if r["kind"] == "chunk" and r["name"] == guid
                              and r["manifest_sha256"] == source["manifest_sha256"]]
                assert candidates and len({r["sha1"] for r in candidates}) == 1
                result = read_asset(candidates[0], decoder)
                (args.samples / (guid + ".chunk")).write_bytes(result)
                return result
            sections = measure((args.samples / sample["file"]).read_bytes(), chunk, sample["resource_metadata"])
            results.append(dict(file=sample["file"], sections=sections))
        except Exception as exc:
            results.append(dict(file=sample["file"], error=repr(exc)))
    (args.samples / "mesh-measurements.json").write_text(json.dumps(results, indent=2))
    print(json.dumps({"samples": len(results), "failures": [r for r in results if "error" in r],
                      "sections": sum(len(r.get("sections", [])) for r in results)}))
    return int(any("error" in r for r in results))


if __name__ == "__main__":
    raise SystemExit(main())
