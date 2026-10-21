"""Compile exported appearance catalogs into an indexed, exact-deduplicated library.

Usage: python scripts/headmesh_library.py sources.json NEW_LIBRARY
sources.json: [{"catalog": ".../catalog.json", "platform": "xbox360"}]
No source files are changed. Geometry keeps native coordinates and split UV records.
"""
import argparse
import collections
import copy
import hashlib
import json
import pathlib
import re
import shutil
import sqlite3
import struct


def sha(data):
    return hashlib.sha256(data).hexdigest()


def load_glb(path):
    data = path.read_bytes()
    if len(data) < 28 or struct.unpack_from("<4sII", data) != (b"glTF", 2, len(data)):
        raise ValueError(f"Invalid GLB: {path}")
    chunks, offset = {}, 12
    while offset < len(data):
        size, kind = struct.unpack_from("<II", data, offset)
        offset += 8
        if size % 4 or offset + size > len(data) or kind in chunks:
            raise ValueError(f"Invalid GLB chunk: {path}")
        chunks[kind] = data[offset:offset + size]
        offset += size
    doc = json.loads(chunks[0x4E4F534A])
    if doc.get("skins") or doc.get("animations"):
        raise ValueError("Rigged GLBs need the rig-preserving import route")
    for node in doc.get("nodes", []):
        if any(k in node for k in ("matrix", "translation", "rotation", "scale")):
            raise ValueError("Transformed nodes require an explicit native frame adapter")
    return doc, chunks[0x004E4942]


def accessor(doc, binary, number):
    a = doc["accessors"][number]
    if "sparse" in a:
        raise ValueError("Sparse accessors are not supported by this native-catalog compiler")
    view = doc["bufferViews"][a["bufferView"]]
    widths = {5120: 1, 5121: 1, 5122: 2, 5123: 2, 5125: 4, 5126: 4}
    components = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}
    width = widths[a["componentType"]] * components[a["type"]]
    stride = view.get("byteStride", width)
    start = view.get("byteOffset", 0) + a.get("byteOffset", 0)
    end = start + max(0, a["count"] - 1) * stride + width
    if (view.get("buffer", 0) != 0 or stride < width or
            end > min(len(binary), view.get("byteOffset", 0) + view["byteLength"])):
        raise ValueError("Accessor exceeds its buffer")
    values = (binary[start:start + a["count"] * width] if stride == width else
              b"".join(binary[start + i * stride:start + i * stride + width]
                       for i in range(a["count"])))
    meta = {k: a[k] for k in ("componentType", "count", "type", "normalized") if k in a}
    return meta, values


def role(name):
    name = name.lower()
    if any(t in name for t in ("facialhair", "beard", "moustache", "mustache", "goatee")):
        return "facialhair"
    if any(t in name for t in ("hair", "cards")):
        return "hair"
    if "eyelash" in name:
        return "eyelash"
    if "eye" in name:
        return "eye"
    if any(t in name for t in ("mouth", "teeth", "tongue")):
        return "mouth"
    return "head"


def pack_subset(doc, binary, selected):
    """Repack referenced accessor bytes only; retain submesh names and a/b cards."""
    result = {"asset": {"version": "2.0", "generator": "NHL Studio head library"},
              "accessors": [], "bufferViews": [], "buffers": [], "meshes": [],
              "nodes": [], "scene": 0, "scenes": [{"nodes": []}]}
    blob, remap = bytearray(), {}
    signatures = []

    def add(index):
        if index in remap:
            return remap[index]
        meta, raw = accessor(doc, binary, index)
        while len(blob) % 4:
            blob.append(0)
        view_id = len(result["bufferViews"])
        result["bufferViews"].append({"buffer": 0, "byteOffset": len(blob), "byteLength": len(raw)})
        blob.extend(raw)
        a = copy.deepcopy(doc["accessors"][index])
        a["bufferView"] = view_id
        a.pop("byteOffset", None)
        remap[index] = len(result["accessors"])
        result["accessors"].append(a)
        return remap[index]

    for index in selected:
        mesh = copy.deepcopy(doc["meshes"][index])
        sub_signatures = []
        for prim in mesh["primitives"]:
            if prim.get("targets") or prim.get("extensions"):
                raise ValueError("Morph/extension data requires a preserving adapter")
            signature = {"mode": prim.get("mode", 4), "attributes": {}}
            for attr, number in sorted(prim["attributes"].items()):
                meta, raw = accessor(doc, binary, number)
                signature["attributes"][attr] = [meta, sha(raw)]
                prim["attributes"][attr] = add(number)
            if "indices" in prim:
                meta, raw = accessor(doc, binary, prim["indices"])
                signature["indices"] = [meta, sha(raw)]
                prim["indices"] = add(prim["indices"])
            prim.pop("material", None)  # Native materials live in the texture-set ledger.
            sub_signatures.append(signature)
        signatures.append({"role": role(mesh.get("name", "")), "primitives": sub_signatures})
        mesh_id = len(result["meshes"])
        result["meshes"].append(mesh)
        result["nodes"].append({"mesh": mesh_id, "name": mesh.get("name", f"mesh_{mesh_id}")})
        result["scenes"][0]["nodes"].append(mesh_id)
    result["buffers"] = [{"byteLength": len(blob)}]
    text = json.dumps(result, separators=(",", ":")).encode()
    text += b" " * (-len(text) % 4)
    blob.extend(b"\0" * (-len(blob) % 4))
    output = (struct.pack("<4sII", b"glTF", 2, 28 + len(text) + len(blob)) +
              struct.pack("<II", len(text), 0x4E4F534A) + text +
              struct.pack("<II", len(blob), 0x004E4942) + blob)
    # Sort submesh signatures; exact values/topology/UVs are required, no rounding.
    canonical = sorted(json.dumps(s, sort_keys=True) for s in signatures)
    return output, sha(json.dumps(canonical).encode()), result


def year(game):
    return 2016 if game == "nhllegacy" else 2000 + int(re.search(r"\d+", game)[0])


def identity(entry):
    match = re.match(r"^(.*?_\d{1,2}_\d{1,2}_\d{4})(?:_|$)", entry["name"])
    return match[1] if match else None


def compile_library(sources, destination):
    destination = pathlib.Path(destination)
    if destination.exists():
        raise ValueError("Choose a new library directory; existing libraries are never overwritten")
    catalogs, known_names = [], collections.defaultdict(set)
    for source in sources:
        path = pathlib.Path(source["catalog"]).resolve()
        doc = json.loads(path.read_text(encoding="utf-8-sig"))
        if doc.get("validate_only"):
            raise ValueError(f"Catalog has no physical assets: {path}")
        catalogs.append((source, path, doc))
        for entry in doc["entries"]:
            slug = identity(entry)
            if slug:
                known_names["_".join(slug.split("_")[:-3])].add(slug)
    destination.mkdir(parents=True)
    db = sqlite3.connect(destination / "headmeshes.sqlite")
    db.executescript("""
      PRAGMA user_version=1;
      CREATE TABLE assets(id TEXT PRIMARY KEY, class TEXT NOT NULL, stem TEXT NOT NULL,
        mesh_file TEXT NOT NULL UNIQUE, game TEXT NOT NULL, platform TEXT NOT NULL,
        origin TEXT NOT NULL, lod INTEGER NOT NULL, submeshes TEXT NOT NULL, geometry_sha256 TEXT NOT NULL);
      CREATE TABLE appearances(id INTEGER PRIMARY KEY, asset_id TEXT NOT NULL REFERENCES assets(id),
        identity_slug TEXT, game TEXT NOT NULL, platform TEXT NOT NULL, origin TEXT NOT NULL,
        source TEXT NOT NULL, source_catalog TEXT NOT NULL, entry_json TEXT NOT NULL,
        textures_json TEXT NOT NULL, lod INTEGER NOT NULL);
      CREATE INDEX by_identity ON appearances(identity_slug, asset_id);
      CREATE INDEX by_asset ON appearances(asset_id);
      CREATE INDEX by_class ON assets(class, stem);
      CREATE TABLE selections(person_key TEXT PRIMARY KEY, asset_id TEXT NOT NULL,
        appearance_id INTEGER NOT NULL, hair_asset_id TEXT, facial_asset_id TEXT);
    """)
    texture_cache, geometry, ordinals = {}, {}, collections.Counter()
    records, unresolved, totals = [], [], collections.Counter()
    # Most recent title wins; Xbox wins against PS3 within the same title.
    catalogs.sort(key=lambda c: (year(c[2]["game"]),
                  {"xbox360": 3, "ps4": 2, "ps5": 4, "ps3": 1}.get(c[0]["platform"], 0),
                  c[2].get("layer") == "Patch"), reverse=True)
    for source, catalog_path, catalog in catalogs:
        print(f"Compiling {catalog['game']} {source['platform']}: {len(catalog['entries'])} assets", flush=True)
        for entry in catalog["entries"]:
            input_mesh = catalog_path.parent / entry["mesh_file"]
            doc, binary = load_glb(input_mesh)
            slug = identity(entry)
            if not slug and "/licensed/" in entry["source"]:
                name = entry["source"].split("/licensed/")[1].split("/")[0]
                # Resource names use first_last; only accept an unambiguous known full name.
                candidates = set()
                parts = name.split("_")
                for split in range(1, len(parts)):
                    candidates.update(known_names["_".join(parts[split:] + parts[:split])])
                if len(candidates) == 1:
                    slug = candidates.pop()
            textures = []
            for rel in entry.get("textures", []):
                texture_key = (catalog_path, rel)
                if texture_key not in texture_cache:
                    path = (catalog_path.parent / rel).resolve()
                    if not path.is_relative_to(catalog_path.parent):
                        raise ValueError(f"Texture escapes source catalog: {rel}")
                    data = path.read_bytes()
                    if data[:4] != b"DDS ":
                        raise ValueError(f"Not DDS: {path}")
                    digest = sha(data)
                    target = pathlib.Path("textures") / digest[:2] / f"{digest}.dds"
                    out = destination / target
                    if not out.exists():
                        out.parent.mkdir(parents=True, exist_ok=True)
                        out.write_bytes(data)
                    texture_cache[texture_key] = target.as_posix()
                textures.append({"name": pathlib.Path(rel).name, "file": texture_cache[texture_key]})
            groups = collections.defaultdict(list)
            for index, mesh in enumerate(doc["meshes"]):
                kind = role(mesh.get("name", ""))
                # A standalone hair/facial-hair container may have generic section labels.
                asset_class = entry["class"] if entry["class"] != "head" else (
                    kind if kind in ("hair", "facialhair") else "head")
                groups[asset_class].append(index)
            for asset_class, indices in groups.items():
                packed, fingerprint, subset = pack_subset(doc, binary, indices)
                key = f"{asset_class}:{fingerprint}"
                if key not in geometry:
                    stem = slug or re.sub(r"[^a-z0-9_]", "_", entry["name"].lower())
                    ordinals[stem, asset_class] += 1
                    folder = asset_class if slug or entry["origin"] != "player" else f"Unresolved/{asset_class}"
                    relative = f"{folder}/{stem[:2]}/{stem}_{ordinals[stem, asset_class]:02d}.glb"
                    output = destination / relative
                    output.parent.mkdir(parents=True, exist_ok=True)
                    output.write_bytes(packed)
                    asset_id = sha(key.encode())
                    names = [m.get("name", "") for m in subset["meshes"]]
                    row = (asset_id, asset_class, stem, relative, entry["game"], source["platform"],
                           entry["origin"], entry.get("lod", 0), json.dumps(names), fingerprint)
                    db.execute("INSERT INTO assets VALUES (?,?,?,?,?,?,?,?,?,?)", row)
                    geometry[key] = asset_id
                    totals[f"unique_{asset_class}"] += 1
                else:
                    totals["duplicate_meshes"] += 1
                asset_id = geometry[key]
                db.execute("INSERT INTO appearances(asset_id,identity_slug,game,platform,origin,source,source_catalog,entry_json,textures_json,lod) VALUES (?,?,?,?,?,?,?,?,?,?)",
                           (asset_id, slug, entry["game"], source["platform"], entry["origin"],
                            entry["source"], str(catalog_path), json.dumps(entry), json.dumps(textures),
                            entry.get("lod", 0)))
                if not slug and entry["origin"] == "player":
                    unresolved.append({"asset_id": asset_id, "name": entry["name"], "source": entry["source"]})
                totals["source_appearances"] += 1
        db.commit()
        records.append({"catalog": str(catalog_path), "sha256": sha(catalog_path.read_bytes()),
                        "platform": source["platform"], "game": catalog["game"],
                        "layer": catalog.get("layer"), "source_entries": len(catalog["entries"]),
                        "rig_policy": "Native geometry; animation skeletons are not inferred",
                        "texture_mips": catalog.get("texture_mips", "base")})
    totals["unique_texture_files"] = len(set(texture_cache.values()))
    totals["unresolved_appearances"] = len(unresolved)
    (destination / "sources.json").write_text(json.dumps(records, indent=2), encoding="utf-8")
    (destination / "unresolved.json").write_text(json.dumps(unresolved, indent=2), encoding="utf-8")
    (destination / "summary.json").write_text(json.dumps(dict(totals), indent=2), encoding="utf-8")
    db.execute("ANALYZE")
    db.close()
    print(json.dumps(dict(totals), indent=2), flush=True)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("sources", type=pathlib.Path)
    parser.add_argument("destination", type=pathlib.Path)
    args = parser.parse_args()
    compile_library(json.loads(args.sources.read_text(encoding="utf-8-sig")), args.destination)
