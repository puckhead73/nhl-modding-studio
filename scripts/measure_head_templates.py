"""Measure shared-UV correspondence across actual catalog templates, without writes to games.

Run with Blender's Python (mathutils required):
blender --background --factory-startup --python-exit-code 1 --python scripts/measure_head_templates.py -- sources.json report.json
"""
import json
from pathlib import Path
import struct
import sys

sys.path.insert(0, str(Path(__file__).resolve().parent))
sys.path.insert(0, str(Path(__file__).resolve().parent / "blender"))
from headmesh_library import accessor, load_glb, role
from uv_template_match import match_templates


def array(doc, binary, index):
    meta, raw = accessor(doc, binary, index)
    fmt = {5126: "f", 5125: "I", 5123: "H", 5121: "B"}[meta["componentType"]]
    width = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}[meta["type"]]
    return [list(v) for v in struct.iter_unpack("<" + fmt * width, raw)]


def sample(source):
    path = Path(source["catalog"])
    catalog = json.loads(path.read_text(encoding="utf-8-sig"))
    candidates = [e for e in catalog["entries"] if e["class"] == "head" and
                  e["origin"] == "player" and e.get("lod", 0) == 0]
    # Same-player comparisons where available; record any fallback explicitly.
    entry = next((e for e in candidates if "ovechkin" in e["name"]), candidates[0])
    doc, binary = load_glb(path.parent / entry["mesh_file"])
    primitives = [p for m in doc["meshes"] if role(m.get("name", "")) == "head"
                  for p in m["primitives"] if "TEXCOORD_0" in p["attributes"]]
    p = max(primitives, key=lambda p: doc["accessors"][p["attributes"]["POSITION"]]["count"])
    indices = [v[0] for v in array(doc, binary, p["indices"])]
    return dict(game=catalog["game"], platform=source["platform"], source=entry["source"],
                same_player="ovechkin" in entry["name"], mesh=entry["mesh_file"],
                geometry=dict(positions=array(doc, binary, p["attributes"]["POSITION"]),
                              uv=array(doc, binary, p["attributes"]["TEXCOORD_0"]),
                              triangles=[indices[i:i+3] for i in range(0, len(indices), 3)]))

def source_surface(mesh):
    """Collapse exact source position+UV duplicates only; never weld the donor."""
    unique, positions, uv, remap, original_ids = {}, [], [], [], []
    for index, (point, texcoord) in enumerate(zip(mesh["positions"], mesh["uv"])):
        key = tuple(point + texcoord)
        if key not in unique:
            unique[key] = len(positions)
            positions.append(point); uv.append(texcoord); original_ids.append(index)
        remap.append(unique[key])
    triangles = [[remap[i] for i in face] for face in mesh["triangles"]]
    return dict(positions=positions, uv=uv, triangles=triangles), original_ids


def main(args):
    sources = json.loads(Path(args[0]).read_text(encoding="utf-8-sig"))
    output = Path(args[1])
    if output.exists():
        raise ValueError("Choose a fresh report path")
    samples = [sample(source) for source in sources]
    surfaces = [source_surface(s["geometry"]) for s in samples]
    pairs = []
    for destination in samples:
        print("Measuring destination", destination["game"], destination["platform"], flush=True)
        for source, (surface, original_ids) in zip(samples, surfaces):
            result = match_templates(destination["geometry"], surface)
            # Ledger IDs refer back to the source GLB records, not temporary welded IDs.
            for key in ("mapping", "uv_matches"):
                result[key] = {i:original_ids[j] for i,j in result[key].items()}
            pairs.append(dict(source=f"{source['game']}-{source['platform']}",
                              destination=f"{destination['game']}-{destination['platform']}",
                              donor_vertices=len(destination["geometry"]["positions"]),
                              source_vertices=len(source["geometry"]["positions"]),
                              source_surface_vertices=len(surface["positions"]),
                              evidence=result))
    output.write_text(json.dumps(dict(
        scope="Correspondence evidence only; does not qualify native packing, UV baking, rig transfer or animation.",
        samples=[{k:v for k,v in s.items() if k != "geometry"} for s in samples],
        pairs=pairs), indent=2), encoding="utf8")


if __name__ == "__main__":
    main(sys.argv[sys.argv.index("--") + 1:] if "--" in sys.argv else sys.argv[1:])
