"""Measure preserved PS5 vertex channels against a decoded named bind-pose roster.

python scripts/audit_ps5_head_weights.py CATALOG BIND_POSE_JSON NEW_REPORT
Reports evidence only; does not invent a parent hierarchy or rewrite skinning.
"""
import json
from pathlib import Path
import sys
import numpy as np
from headmesh_library import accessor, load_glb


def audit(catalog_path, skeleton_path, output):
    catalog_path, skeleton_path, output = map(Path, (catalog_path, skeleton_path, output))
    if output.exists():
        raise ValueError("Choose a new report")
    catalog = json.loads(catalog_path.read_text())
    skeleton = json.loads(skeleton_path.read_text())
    count = len(skeleton["bones"])
    reports = []
    for index, entry in enumerate(catalog["entries"]):
        doc, binary = load_glb(catalog_path.parent / entry["mesh_file"])
        for mesh in doc["meshes"]:
            for primitive in mesh["primitives"]:
                attrs = primitive["attributes"]
                arrays = {}
                other_layout = None
                for usage in ("02", "03", "04", "05"):
                    key = "_NATIVE_USAGE_" + usage
                    if key in attrs:
                        meta, raw = accessor(doc, binary, attrs[key])
                        if meta["componentType"] != 5126 or meta["type"] != "VEC4":
                            other_layout = meta
                            break
                        arrays[usage] = np.frombuffer(raw, "<f4").reshape(-1, 4)
                row = dict(mesh=entry["mesh_file"], submesh=mesh["name"], source=entry["source"])
                if other_layout:
                    row.update(status="other_skin_channel_layout", declaration=other_layout)
                elif "02" not in arrays or "04" not in arrays:
                    row["status"] = "no_complete_skin_channels"
                else:
                    joints = np.concatenate([arrays["02"], arrays.get("03", np.zeros_like(arrays["02"]))], axis=1)
                    weights = np.concatenate([arrays["04"], arrays.get("05", np.zeros_like(arrays["04"]))], axis=1)
                    active = weights > 0
                    sums = weights.sum(axis=1)
                    bad = (~np.isfinite(joints)).any() or (~np.isfinite(weights)).any()
                    bad |= bool((joints[active] < 0).any() or (joints[active] >= count).any())
                    bad |= bool((joints[active] != np.floor(joints[active])).any())
                    bad |= bool((weights < 0).any() or (weights > 1).any() or (np.abs(sums - 1) > 1 / 255 + 1e-6).any())
                    used = sorted(int(v) for v in np.unique(joints[active]))
                    row.update(status="invalid_or_other_rig" if bad else "channels_within_named_roster",
                               vertices=len(joints), weight_sum_min=float(sums.min()),
                               weight_sum_max=float(sums.max()), raw_joint_ids=used,
                               bone_names=[skeleton["bones"][j]["name"] for j in used if 0 <= j < count])
                reports.append(row)
        if index % 500 == 0:
            print(f"Audited {index}/{len(catalog['entries'])} meshes", flush=True)
    from collections import Counter
    result = dict(game=catalog["game"], skeleton_sha256=skeleton["sha256"], named_bones=count,
                  counts=dict(Counter(row["status"] for row in reports)), submeshes=reports,
                  scope="Channel range and normalization evidence. Parent hierarchy and animated deformations are not qualified.")
    output.write_text(json.dumps(result, indent=2), encoding="utf8")
    print(result["counts"], flush=True)


if __name__ == "__main__":
    audit(*sys.argv[1:])
