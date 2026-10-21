"""Audit every compiled GLB/DDS reference and export a portable provenance CSV.

Usage: python scripts/verify_headmesh_library.py LIBRARY
Writes verification.json and provenance.csv inside the library. Never edits assets.
"""
import argparse
import collections
import csv
import hashlib
import json
from pathlib import Path
import sqlite3
import struct

import numpy as np
from headmesh_library import accessor, load_glb, pack_subset


def verify(root):
    root = Path(root).resolve()
    db = sqlite3.connect(f"{root.as_uri()}/headmeshes.sqlite?mode=ro", uri=True)
    db.row_factory = sqlite3.Row
    if db.execute("PRAGMA integrity_check").fetchone()[0] != "ok":
        raise ValueError("SQLite integrity check failed")
    if db.execute("PRAGMA foreign_key_check").fetchall():
        raise ValueError("Orphaned provenance records")
    totals, errors, seen_textures = collections.Counter(), [], set()

    def contained(relative):
        path = (root / relative).resolve()
        if not path.is_relative_to(root):
            raise ValueError(f"Path escapes library: {relative}")
        return path

    for row in db.execute("SELECT * FROM assets ORDER BY id"):
        try:
            doc, binary = load_glb(contained(row["mesh_file"]))
            _, fingerprint, _ = pack_subset(doc, binary, list(range(len(doc["meshes"]))))
            if fingerprint != row["geometry_sha256"]:
                raise ValueError("Geometry fingerprint mismatch")
            for mesh in doc["meshes"]:
                for primitive in mesh["primitives"]:
                    positions = doc["accessors"][primitive["attributes"]["POSITION"]]["count"]
                    for key, index in primitive["attributes"].items():
                        meta, raw = accessor(doc, binary, index)
                        if meta["count"] != positions:
                            raise ValueError(f"{key} count differs from positions")
                        if meta["componentType"] == 5126 and not np.isfinite(np.frombuffer(raw, "<f4")).all():
                            raise ValueError(f"Non-finite {key}")
                    meta, raw = accessor(doc, binary, primitive["indices"])
                    indices = np.frombuffer(raw, {5121: "u1", 5123: "<u2", 5125: "<u4"}[meta["componentType"]])
                    if len(indices) % 3 or (len(indices) and indices.max() >= positions):
                        raise ValueError("Invalid triangle indices")
            totals["meshes"] += 1
        except Exception as exc:
            errors.append({"asset": row["id"], "file": row["mesh_file"], "error": str(exc)})
        if (totals["meshes"] + len(errors)) % 1000 == 0:
            print(f"Verified {totals['meshes']} meshes; {len(errors)} errors", flush=True)
    with (root / "provenance.csv").open("w", encoding="utf-8-sig", newline="") as stream:
        writer = csv.writer(stream)
        writer.writerow(["asset_id", "mesh", "class", "identity_slug", "game", "platform",
                         "origin", "lod", "source", "texture_files", "layer", "patch_version"])
        for row in db.execute("""SELECT p.*,a.mesh_file,a.class FROM appearances p
                              JOIN assets a ON a.id=p.asset_id ORDER BY a.mesh_file,p.game,p.platform"""):
            textures = json.loads(row["textures_json"])
            entry = json.loads(row["entry_json"])
            writer.writerow([row["asset_id"], row["mesh_file"], row["class"],
                             row["identity_slug"], row["game"], row["platform"], row["origin"],
                             row["lod"], row["source"], "|".join(t["file"] for t in textures),
                             entry.get("layer", "Data"), entry.get("patch_version", "")])
            totals["appearances"] += 1
            for texture in textures:
                relative = texture["file"]
                if relative in seen_textures:
                    continue
                seen_textures.add(relative)
                try:
                    path = contained(relative)
                    data = path.read_bytes()
                    if hashlib.sha256(data).hexdigest() != path.stem:
                        raise ValueError("Texture content hash mismatch")
                    if len(data) < 128 or data[:4] != b"DDS ":
                        raise ValueError("Invalid DDS header")
                    height, width = struct.unpack_from("<II", data, 12)
                    if not 0 < min(width, height) <= max(width, height) <= 16384:
                        raise ValueError("Invalid DDS dimensions")
                    totals["textures"] += 1
                except Exception as exc:
                    errors.append({"file": relative, "error": str(exc)})
    totals["duplicate_geometry_groups"] = db.execute(
        "SELECT count(*) FROM (SELECT asset_id FROM appearances GROUP BY asset_id HAVING count(*)>1)").fetchone()[0]
    report = {"passed": not errors, "counts": dict(totals), "errors": errors,
              "scope": "Structural integrity and exact fingerprints; not an in-game animation qualification."}
    (root / "verification.json").write_text(json.dumps(report, indent=2), encoding="utf8")
    db.close()
    print(json.dumps(report, indent=2), flush=True)
    return not errors


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("library", type=Path)
    raise SystemExit(0 if verify(parser.parse_args().library) else 1)
