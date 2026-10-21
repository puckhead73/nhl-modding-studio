"""Native-fit exchange JSON access (`native-fit-exchange/1`, Codex's NHL21 exchange schema), numpy only.

The exchange carries, per submesh, the untouched donor (`original_positions`), the editable `positions`, ordered
`triangles`, per-record `uv`/`normals`/`tangents`, `native_fields`/`streams`/`format`/`role`/`connected_components`.
Only `positions` of the head (and, when a qualified profile allows it, the eyes) may change; everything else must
stay byte-equal so the native packer can preserve adapter-private data. Helpers here never drop fields.
"""
import hashlib, json
from pathlib import Path
import numpy as np
from . import geometry as G

HEAD = 'geo_headShape'; EYES = 'geo_eyesShape'


def sha256_file(path):
    return hashlib.sha256(Path(path).read_bytes()).hexdigest()


def sha256_bytes(b):
    return hashlib.sha256(b).hexdigest()


def load(path):
    return json.loads(Path(path).read_text())


def save(ex, path):
    Path(path).write_text(json.dumps(ex))


def submesh(ex, name):
    return next(m for m in ex['submeshes'] if m['name'] == name)


def head(ex):
    return submesh(ex, HEAD)


def with_positions(ex, name, positions_blender):
    """Copy of the exchange with `name`'s positions replaced (Blender frame in -> native frame out)."""
    m = submesh(ex, name); m2 = dict(m); m2['positions'] = G.blender_to_native(positions_blender).tolist()
    nat = dict(ex); nat['submeshes'] = [m2 if x is m else x for x in ex['submeshes']]
    return nat


def head_arrays(ex, which='positions'):
    """(positions in Blender frame, triangles, uv) of the head."""
    h = head(ex)
    return G.native_to_blender(h[which]), np.array(h['triangles']), np.array(h['uv'])


def ordered_topology_sha256(triangles):
    """Ordered topology signature: sha256 of the int32 little-endian triangle index stream (winding preserved)."""
    return hashlib.sha256(np.asarray(triangles, np.int32).astype('<i4').tobytes()).hexdigest()


def corner_uv_sha256(uv, triangles, decimals=6):
    """Ordered per-corner UV signature (rounded to `decimals`)."""
    uv = np.asarray(uv, float); T = np.asarray(triangles)
    return hashlib.sha256(np.round(uv[T].reshape(-1, 2), decimals).astype('<f8').tobytes()).hexdigest()


def float32_roundtrip_max(positions):
    P = np.asarray(positions, float); return float(np.abs(P.astype(np.float32).astype(np.float64) - P).max())


def signature(ex):
    """Per-submesh counts and ordered signatures for donor-layout validation."""
    out = {'schema': ex.get('schema'), 'profile_id': ex.get('profile_id'), 'donor_sha256': ex.get('donor_sha256'), 'submeshes': {}}
    for m in ex['submeshes']:
        out['submeshes'][m['name']] = {'records': len(m['original_positions']), 'triangles': len(m['triangles']), 'format': m.get('format'),
                                       'ordered_topology_sha256': ordered_topology_sha256(m['triangles']), 'corner_uv_sha256': corner_uv_sha256(m['uv'], m['triangles']),
                                       'connected_components': len(m.get('connected_components', []))}
    return out


def validate_donor_layout(ex, expected):
    """Compare an exchange against an expected donor layout (from a profile / recipe): schema, profile id, donor hash,
    submesh names+order, record/triangle counts and ordered topology signatures. Returns (ok, problems)."""
    problems = []
    sig = signature(ex)
    for k in ('schema', 'profile_id', 'donor_sha256'):
        if expected.get(k) is not None and sig.get(k) != expected[k]: problems.append(f'{k}: {sig.get(k)} != {expected[k]}')
    exp_sub = expected.get('submeshes', {})
    if list(exp_sub.keys()) and list(sig['submeshes'].keys()) != list(exp_sub.keys()): problems.append(f"submesh names/order {list(sig['submeshes'].keys())} != {list(exp_sub.keys())}")
    for name, e in exp_sub.items():
        s = sig['submeshes'].get(name)
        if s is None: continue
        for k in ('records', 'triangles', 'ordered_topology_sha256', 'corner_uv_sha256'):
            if e.get(k) is not None and s.get(k) != e[k]: problems.append(f'{name}.{k}: {s.get(k)} != {e[k]}')
    cs = ex.get('coordinate_system', {})
    if cs.get('native_to_blender') != '(x,y,z) -> (x,-z,y)' or cs.get('native_uv_to_blender') != '(u,v) -> (u,1-v)': problems.append(f'coordinate_system unexpected: {cs}')
    return (not problems), problems


def untouched_fields_equal(cand, ref, eyes_may_move=False):
    """Every field except head positions (and eye positions when allowed) equals the reference exchange."""
    problems = []
    for k in ('schema', 'profile_id', 'coordinate_system', 'layout', 'rules', 'title', 'donor_sha256'):
        if cand.get(k) != ref.get(k): problems.append(f'top_level_{k}')
    if [m['name'] for m in cand['submeshes']] != [m['name'] for m in ref['submeshes']]: problems.append('submesh_names_order'); return False, problems
    for m, v in zip(cand['submeshes'], ref['submeshes']):
        for k in ('original_positions', 'triangles', 'uv', 'normals', 'tangents', 'streams', 'format', 'role', 'native_fields', 'connected_components'):
            if m.get(k) != v.get(k): problems.append(f'{m["name"]}.{k}')
        movable = m['name'] == HEAD or (eyes_may_move and m['name'] == EYES)
        if not movable and m['positions'] != v['positions']: problems.append(f'{m["name"]}.positions_moved')
    return (not problems), problems
