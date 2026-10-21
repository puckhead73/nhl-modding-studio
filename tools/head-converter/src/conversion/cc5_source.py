"""CC5 (Character Creator / Headshot) source geometry records (`source_geometry.json`, Studio matcher schema), numpy only.

Schema: a list of meshes {name, positions (metres, Blender frame: Z up, -Y forward, +X = character's left), uv (V-up),
triangles (into the per-(vertex, uv) deduplicated records), triangle_materials}. The head mesh is named '0' and
carries a single material (Std_Skin_Head); cc_base_eye has four materials in the order Std_Eye_R, Std_Cornea_R,
Std_Eye_L, Std_Cornea_L. Positions differ per character and per export session; the tessellation and atlas are
shared by every CC5 Headshot head export of this base (measured on four exports, research/claude/inventory.md), so
a UV registration calibrated once can be reused family-wide while positions/fits never can.
"""
import hashlib, json
from pathlib import Path
import numpy as np

HEAD = '0'; EYE = 'cc_base_eye'; TEETH = 'cc_base_teeth'; TONGUE = 'cc_base_tongue'
EYE_MATERIALS = ['Std_Eye_R', 'Std_Cornea_R', 'Std_Eye_L', 'Std_Cornea_L']
# Measured CC5 head-only family (Demitra, Turgeon, TurgeonTest, Fedorov): 4119 vertices / 8196 triangles before the
# per-(vertex, uv) record split; the record split yields 4282 records with the Studio/Blender dedup rule.
FAMILY_CC5_HEADSHOT = {'id': 'cc5-headshot-head-4119-8196', 'head_records': 4282, 'head_triangles': 8196, 'eye_records': 660, 'eye_triangles': 1280}


def load(path):
    return json.loads(Path(path).read_text())


def mesh(src, name):
    return next(m for m in src if m['name'] == name)


def head_arrays(src):
    cc = mesh(src, HEAD); return np.array(cc['positions'], float), np.array(cc['triangles']), np.array(cc['uv'], float)


def eye_material_ids(src):
    """{material name: unique record ids} for the CC5 eye mesh."""
    e = mesh(src, EYE); mats = np.array(e['triangle_materials']); et = np.array(e['triangles'])
    return {name: np.unique(et[mats == mi]) for mi, name in enumerate(EYE_MATERIALS)}


def ordered_topology_sha256(triangles):
    return hashlib.sha256(np.asarray(triangles, np.int32).astype('<i4').tobytes()).hexdigest()


def corner_uv_sha256(uv, triangles, decimals=6):
    uv = np.asarray(uv, float); T = np.asarray(triangles)
    return hashlib.sha256(np.round(uv[T].reshape(-1, 2), decimals).astype('<f8').tobytes()).hexdigest()


def signature(src):
    out = {}
    for m in src:
        out[m['name']] = {'records': len(m['positions']), 'triangles': len(m['triangles']), 'materials': int(max(m['triangle_materials']) + 1) if m.get('triangle_materials') else None,
                          'ordered_topology_sha256': ordered_topology_sha256(m['triangles']), 'corner_uv_sha256': corner_uv_sha256(m['uv'], m['triangles'])}
    return out


def validate(src, expected_family=FAMILY_CC5_HEADSHOT, expected_signature=None):
    """Structural validation of a CC5 source: required meshes, family counts, optional exact signatures."""
    problems = []
    names = [m['name'] for m in src]
    for n in (HEAD, EYE):
        if n not in names: problems.append(f'missing mesh {n}')
    if problems: return False, problems
    sig = signature(src)
    if expected_family:
        if sig[HEAD]['records'] != expected_family['head_records'] or sig[HEAD]['triangles'] != expected_family['head_triangles']:
            problems.append(f"head records/triangles {sig[HEAD]['records']}/{sig[HEAD]['triangles']} != family {expected_family['head_records']}/{expected_family['head_triangles']}")
        if sig[EYE]['records'] != expected_family['eye_records'] or sig[EYE]['triangles'] != expected_family['eye_triangles']:
            problems.append(f"eye records/triangles {sig[EYE]['records']}/{sig[EYE]['triangles']} != family {expected_family['eye_records']}/{expected_family['eye_triangles']}")
        if sig[EYE]['materials'] != 4: problems.append(f"eye materials {sig[EYE]['materials']} != 4")
    if expected_signature:
        for n, e in expected_signature.items():
            for k in ('ordered_topology_sha256', 'corner_uv_sha256'):
                if e.get(k) and sig.get(n, {}).get(k) != e[k]: problems.append(f'{n}.{k} mismatch')
    P = np.array(mesh(src, HEAD)['positions'], float)
    if not np.isfinite(P).all(): problems.append('non-finite head positions')
    ext = P.max(0) - P.min(0)
    if not (0.15 < ext[2] < 0.6): problems.append(f'head z extent {ext[2]:.3f} m outside 0.15-0.6 (units/scale?)')
    return (not problems), problems
