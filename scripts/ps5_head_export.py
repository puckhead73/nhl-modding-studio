"""Export measured PS5 head layouts from a read-only appearance inventory.

python scripts/ps5_head_export.py INVENTORY.jsonl NEW_OUTPUT --game nhl26
Uses the user's earlier NHL26 research readers for measured RDNA2 texture tiling.
Exports one explicit layer; patch chunks may reference the matching base data.
"""
import argparse
import collections
import hashlib
import json
from pathlib import Path
import struct
import sys
import numpy as np
from frostbite_head_probe import Oodle, decode_frames
from headmesh_library import pack_subset

PREFIXES = ("content/character/skater/shared/head/", "content/character/skater/referee/head/",
            "content/character/bench/shared/head/")

def associated_textures(name, textures):
    family = name.split("/components/")[0]
    return [path for (tex_name, _), (path, _) in textures.items()
            if tex_name.startswith(family + "/") or
            tex_name.startswith("content/character/common/textures/")]

class CompleteChunks:
    """Resolve streamed mip chains from measured TOC GUID/placement records."""
    def __init__(self, rows):
        self.folders = {}
        self.tocs = {}
        self.used = {}
        for row in rows:
            span = row["span"]
            self.folders[(span["layer"], span["persistent_index"])] = Path(row["physical"]).parent

    def lookup(self, row, guid):
        path = Path(row["toc_root"]) / row["toc"]
        if path not in self.tocs:
            data = path.read_bytes()
            if data[:4] == bytes.fromhex("00d1ce01"):
                data = data[0x22c:]
            words = struct.unpack_from(">15I", data)
            if words[0] != 60 or words[6] != words[4] + words[5] * 20:
                raise ValueError("Unsupported TOC chunk table")
            records = {}
            for index in range(words[5]):
                at = words[4] + index * 20
                key = data[at:at + 16][::-1].hex()
                token = struct.unpack_from(">I", data, at + 16)[0]
                if not token & 0x80000000 or token & 3:
                    raise ValueError("Invalid TOC chunk placement token")
                placement = (token & 0x7fffffff) // 4
                if placement >= words[5]:
                    raise ValueError("TOC chunk placement outside table")
                value, offset, size = struct.unpack_from(">QII", data, words[6] + placement * 16)
                span = dict(layer=value >> 48, persistent_index=(value >> 16) & 0xffffffff,
                            cas=value & 0xffff, offset=offset, size=size)
                if key in records and records[key] != span:
                    raise ValueError("Conflicting TOC chunk placements")
                records[key] = span
            self.tocs[path] = records
        span = self.tocs[path].get(guid)
        if span is None:
            return None
        folder = self.folders.get((span["layer"], span["persistent_index"]))
        if folder is None:
            raise ValueError("No measured CAS folder for streamed chunk")
        resolved = dict(span=span, physical=str(folder / f"cas_{span['cas']:02d}.cas"))
        self.used[f"{path}:{guid}"] = resolved
        return resolved


def read(row, decoder):
    if row["span"]["size"] > 256 * 1024 * 1024:
        raise ValueError("CAS span exceeds limit")
    with open(row["physical"], "rb") as stream:
        stream.seek(row["span"]["offset"])
        data = stream.read(row["span"]["size"])
    if len(data) != row["span"]["size"]:
        raise ValueError("Truncated CAS span")
    return decode_frames(data, decoder)


def ps5_mesh(data, chunks, decoder, metadata=None):
    """Native 0x170 section declaration, relative pointers based at byte 16."""
    u32 = lambda p: struct.unpack_from("<I", data, p)[0]
    u64 = lambda p: struct.unpack_from("<Q", data, p)[0]
    if len(data) < 0xc0 or u32(8) != 0x170:
        raise ValueError("Unsupported PS5 MeshSet section layout")
    count = struct.unpack_from("<H", data, 0xac)[0]
    if not 1 <= count <= 6:
        raise ValueError(f"Unsupported PS5 LOD count {count}")
    output = []
    for level in range(count):
        lod = u64(0x30 + level * 8) + 16
        section_count, table = u32(lod + 8), u64(lod + 12) + 16
        if not 0 < section_count <= 128 or table + section_count * 0x170 > len(data):
            raise ValueError("Invalid section table")
        if u32(lod + 0x54) != 0x21:
            raise ValueError("Unsupported PS5 index declaration")
        index_bytes, vertex_bytes = u32(lod + 0x58), u32(lod + 0x5c)
        layout = u32(4)
        if layout not in (0xbc, 0xb8, 0xb0, 0xac):
            raise ValueError(f"Unsupported PS5 LOD layout {layout:#x}")
        is_nhl27 = layout in (0xb8, 0xac)
        guid_offset = 0x70 if is_nhl27 else 0x74
        guid = data[lod + guid_offset:lod + guid_offset + 16].hex()
        if guid == "0" * 32:
            if metadata is None:
                raise ValueError("Inline geometry requires resource metadata")
            meta = bytes.fromhex(metadata)
            header_size, relocation_size, inline_size = struct.unpack_from("<III", meta)
            start = (header_size + relocation_size + 15) & ~15
            offset = u32(lod + guid_offset + 16)
            if start + inline_size > len(data) or offset + vertex_bytes + index_bytes > inline_size:
                raise ValueError("Inline geometry exceeds resource bounds")
            geometry = data[start + offset:start + offset + vertex_bytes + index_bytes]
        else:
            if guid not in chunks:
                raise ValueError(f"Missing geometry chunk {guid}")
            geometry = read(chunks[guid], decoder)
        if vertex_bytes + index_bytes > len(geometry):
            raise ValueError("Geometry exceeds chunk")
        doc = {"asset": {"version": "2.0"}, "accessors": [], "bufferViews": [], "meshes": []}
        binary = bytearray()

        def add(array, kind, component=5126):
            array = np.asarray(array, dtype="<f4" if component == 5126 else "<u4")
            raw = array.tobytes()
            view = len(doc["bufferViews"])
            doc["bufferViews"].append({"buffer": 0, "byteOffset": len(binary), "byteLength": len(raw)})
            binary.extend(raw)
            index = len(doc["accessors"])
            accessor = {"bufferView": view, "componentType": component, "count": len(array), "type": kind}
            if kind == "VEC3":
                accessor.update(min=array.min(axis=0).tolist(), max=array.max(axis=0).tolist())
            doc["accessors"].append(accessor)
            return index

        for number in range(section_count):
            section = table + number * 0x170
            # This opaque section field is not a fixed tag: measured rigid
            # hair attachments have zero here. Do not infer a bone mapping.
            if data[section + 24:section + 26] not in (b"\xb2\x01", b"\0\0"):
                raise ValueError("Unsupported section declaration")
            triangles, first, start, vertices = (u32(section + x) for x in (0x20, 0x24, 0x28, 0x2c))
            if not 0 < vertices <= 1000000 or triangles > 2000000:
                raise ValueError("Excessive section size")
            name_ptr = u64(section + 8) + 16
            end = data.find(b"\0", name_ptr)
            if end < name_ptr or end - name_ptr > 1024:
                raise ValueError("Invalid material name")
            name = data[name_ptr:end].decode("utf8")
            attrs, offset = {}, start
            formats = {2: ("<f4", 2), 3: ("<f4", 3), 4: ("<f4", 4),
                       6: ("<f2", 2), 8: ("<f2", 4), 0x17: ("<u2", 4),
                       0x16: ("<u2", 2), 0x0d: ("u1", 4), 0x0c: ("u1", 4)}
            declaration = 0x70 if is_nhl27 else 0x64
            elements = data[section + declaration + 0x60]
            if not 1 <= elements <= 16:
                raise ValueError("Invalid vertex element count")
            for element in range(elements):
                usage, fmt, stream, ordinal = data[section + declaration + element * 4:section + declaration + 4 + element * 4]
                if fmt not in formats:
                    raise ValueError(f"Unsupported PS5 vertex format {fmt:#x}")
                dtype, width = formats[fmt]
                size = vertices * width * np.dtype(dtype).itemsize
                if offset + size > vertex_bytes:
                    raise ValueError("Vertex element exceeds declared buffer")
                arr = np.frombuffer(geometry, dtype=dtype, count=vertices * width, offset=offset).reshape(vertices, width).astype("<f4")
                if fmt == 0x0d:
                    arr /= 255
                if not np.isfinite(arr).all():
                    raise ValueError("Non-finite vertex attribute")
                offset += size
                key = {1: "POSITION", 6: "NORMAL", 0x21: "TEXCOORD_0", 0x22: "TEXCOORD_1",
                       0x23: "TEXCOORD_2"}.get(usage, f"_NATIVE_USAGE_{usage:02X}")
                if usage in (1, 6):
                    arr = arr[:, :3]
                attrs[key] = add(arr, f"VEC{arr.shape[1]}")
            if "POSITION" not in attrs:
                raise ValueError("Missing positions")
            if first * 2 + triangles * 6 > index_bytes:
                raise ValueError("Index range exceeds declared buffer")
            indices = np.frombuffer(geometry, dtype="<u2", count=triangles * 3, offset=vertex_bytes + first * 2)
            if len(indices) and indices.max() >= vertices:
                raise ValueError("Index outside section")
            doc["meshes"].append({"name": name, "primitives": [{"attributes": attrs,
                                   "indices": add(indices, "SCALAR", 5125), "mode": 4}]})
        packed, _, _ = pack_subset(doc, binary, list(range(section_count)))
        output.append((level, guid, packed, [m["name"] for m in doc["meshes"]]))
    return output


def main():
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("inventory", type=Path)
    ap.add_argument("output", type=Path)
    ap.add_argument("--game", choices=["nhl26", "nhl27"], required=True)
    ap.add_argument("--layer", choices=["Data", "Patch"], default="Data")
    ap.add_argument("--research", type=Path, default=Path("S:/NHL26/research/src"))
    ap.add_argument("--decoder", type=Path, default=Path("E:/Repositories/UEViewer/oo2core_8_win64.dll"))
    ap.add_argument("--reuse-textures", type=Path, help="Reuse immutable DDS outputs from an earlier run")
    args = ap.parse_args()
    sys.dont_write_bytecode = True
    sys.path.insert(0, str(args.research))
    from frostbite import texture, gfx10
    args.output.mkdir(parents=True, exist_ok=False)
    (args.output / "heads").mkdir()
    (args.output / "textures").mkdir()
    decoder = Oodle(args.decoder)
    resources, chunks, base_chunks = {}, {}, {}
    print(f"Indexing {args.layer} resources and exact GUID chunk references…", flush=True)
    with args.inventory.open(encoding="utf8") as stream:
        for line in stream:
            row = json.loads(line)
            layer = row["toc"].split("/")[0]
            if row["kind"] == "chunk":
                if layer == args.layer:
                    chunks.setdefault(row["name"].replace("-", ""), row)
                elif layer == "Data":
                    base_chunks.setdefault(row["name"].replace("-", ""), row)
            elif layer == args.layer and row["kind"] == "res" and (row["name"].startswith(PREFIXES) or
                                          row["name"].startswith("content/character/common/textures/")):
                resources.setdefault((row["name"], row["sha1"]), row)
    for guid, row in base_chunks.items():
        chunks.setdefault(guid, row)
    if args.layer == "Patch":
        sys.path.insert(0, str(Path(__file__).resolve().parent / "nhl27"))
        from dbobject import parse
        for root in {Path(r["toc_root"]) for r in resources.values()}:
            base = parse(root / "Data/layout.toc")
            patch = parse(root / "Patch/layout.toc")
            if patch.get("base") is None or patch["base"] != base.get("head"):
                raise ValueError(f"Patch does not match its base installation: {root}")
    complete = CompleteChunks(list(resources.values()) + list(chunks.values()))
    textures, failures, entries, mesh_rows = {}, [], [], []
    print(f"{len(resources)} resources; {len(chunks)} chunks", flush=True)
    for row in resources.values():
        name, digest = row["name"], row["sha1"]
        leaf = name.rsplit("/", 1)[-1][:100] + "_" + digest
        if row["resource_type"] == "0x49b156d4":
            mesh_rows.append(row)
            continue
        if row["resource_type"] != "0x6bde20ba":
            continue
        try:
            relative = f"textures/{leaf}.dds"
            if args.reuse_textures and (args.reuse_textures / relative).is_file():
                import os, shutil
                try:
                    os.link(args.reuse_textures / relative, args.output / relative)
                except OSError:
                    shutil.copyfile(args.reuse_textures / relative, args.output / relative)
                textures[name, digest] = (relative, row)
                continue
            res = read(row, decoder)
            info = texture.parse_res_header(res)
            width, height = struct.unpack_from("<HH", res, 0x16)
            if all(0 < n <= 8192 and n & (n - 1) == 0 for n in (width, height)):
                info.update(width=width, height=height)
            guid = texture.chunk_guid(res).hex()
            if guid not in chunks:
                raise ValueError(f"Missing pixel chunk {guid}")
            pixels = read(complete.lookup(row, guid) or chunks[guid], decoder)
            fmt = "BC4" if info["format"] == 0x3e else texture.resolve_bcn(info, len(pixels))
            if fmt is None:
                raise ValueError("Unknown texture block format")
            block = 8 if fmt in ("BC1", "BC4") else 16
            tile_width, tile_height = gfx10._tile_dims(block)
            blocks_w, blocks_h = max(1, width // 4), max(1, height // 4)
            if blocks_w % tile_width or blocks_h % tile_height:
                if res[0x1e] != 1:
                    raise ValueError("Small packed mip chains require a measured mip-tail adapter")
                padded_w = (blocks_w + tile_width - 1) // tile_width * tile_width
                padded_h = (blocks_h + tile_height - 1) // tile_height * tile_height
                padded = gfx10.deswizzle(pixels, padded_w, padded_h, block)
                linear = b"".join(padded[y * padded_w * block:(y * padded_w + blocks_w) * block]
                                  for y in range(blocks_h))
            else:
                linear = gfx10.deswizzle_texture(pixels, width, height, fmt)
            dds = texture.dds_dx10(width, height, "BC1" if fmt == "BC4" else fmt, linear)
            if fmt == "BC4":
                dds = dds[:128] + struct.pack("<I", 80) + dds[132:]
            (args.output / relative).write_bytes(dds)
            textures[name, digest] = (relative, row)
        except Exception as exc:
            failures.append({"source": name, "kind": "texture", "error": str(exc)})
    print(f"Textures: {len(textures)}; failures: {len(failures)}", flush=True)
    for index, row in enumerate(mesh_rows):
        name = row["name"]
        try:
            data = read(row, decoder)
            for lod, guid, glb, names in ps5_mesh(data, chunks, decoder, row["resource_meta"]):
                leaf = name.rsplit("/", 1)[-1][:100] + "_" + row["sha1"]
                relative = f"heads/{leaf}_lod{lod}_{args.game}.glb"
                (args.output / relative).write_bytes(glb)
                associated = associated_textures(name, textures)
                kind = "facialhair" if any(t in leaf for t in ("beard", "facial", "goatee", "moustache")) else "hair" if "hair" in leaf else "head"
                origin = "coach" if "/bench/" in name else "referee" if "/referee/" in name else "player" if "/licensed/" in name else "generic"
                entries.append(dict(name=Path(relative).stem, origin=origin, **{"class": kind},
                                    game=args.game, layer=args.layer, mesh_file=relative, submeshes=names, textures=associated,
                                    texture_sources=[], source=name, lod=lod, chunk=guid, resource_sha1=row["sha1"]))
        except Exception as exc:
            failures.append({"source": name, "kind": "mesh", "error": str(exc)})
        if index % 100 == 0:
            print(f"Meshes {index}/{len(mesh_rows)}; exports {len(entries)}; failures {len(failures)}", flush=True)
    catalog = dict(version=1, game=args.game, layer=args.layer, entries=entries,
                   geometry_only=True, native_vertex_channels=True, texture_mips="base",
                   head_count=sum(e["class"] == "head" for e in entries),
                   hair_count=sum(e["class"] == "hair" for e in entries),
                   facial_hair_count=sum(e["class"] == "facialhair" for e in entries))
    for file, value in [("catalog.json", catalog), ("failures.json", failures),
                        ("resource-provenance.json", list(resources.values())),
                        ("streamed-chunk-provenance.json", complete.used),
                        ("reader-provenance.json", {str(p): hashlib.sha256(p.read_bytes()).hexdigest()
                         for p in [Path(texture.__file__), Path(gfx10.__file__), Path(__file__)]}),
                        ("summary.json", dict(meshes=len(entries), textures=len(textures), failures=len(failures)))]:
        (args.output / file).write_text(json.dumps(value, indent=2), encoding="utf8")
    print(f"Done: {len(entries)} meshes, {len(textures)} textures, {len(failures)} failures", flush=True)


if __name__ == "__main__":
    main()
