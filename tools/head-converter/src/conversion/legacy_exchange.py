"""Legacy (Xbox 360 RenderWare RX2) donor exchange for the conversion side: `legacy-fit-exchange/1`.

Built from Codex's decoded native structure (Studio `tdb-rx2` strict native-skin decode through
`experiments/codex/native_probe`, dumped as `local/runs/codex/inventory-01/native-structure-v2.json`): per submesh the
decoded records (position, uv, 4 joints, 4 weights), ordered triangles, stride and Codex's topology/uv hashes, plus
the 122 palette matrices. Nothing is re-decoded here and nothing native is written: this module only re-shapes the
decoded evidence into the conversion-side layout, derives what can be derived without assumptions (roles from the
submesh names, open boundary loops, seam groups, connected components, attachment loop from the boundary records
weighted on body-side palettes, eye sphere descriptors) and validates it. numpy + stdlib only.

Conventions (measured on player_67, see research/claude/legacy-bridge-2026-09-12-report.md):
- native frame: metres, +Y up, +Z forward (eyes at z ~ +0.08), +X = character left (eyel_ at +x); the same
  `(x,y,z) -> (x,-z,y)` mapping to the Blender frame as the PS4 exchange; native UV origin top-left (crown v ~ 0.03).
- joints are shader registers (= palette index * 3); weights sum to 1; both are opaque here and must be written back
  unchanged (Codex's writer owns record layout, strips, fp16/u16 quantisation and the palette).
Only `positions` may change downstream (and, for eyes/mouthbag, only as whole-component rigid moves declared in the
handoff); `original_positions`, `uv`, `triangles`, `joints`, `weights`, `stride`, hashes and `palette_matrices`
are protected.
"""
import hashlib, json
from pathlib import Path
import numpy as np
from . import geometry as G, exchange as X
from .eye_fit import sphere_fit

SCHEMA = 'legacy-fit-exchange/1'
ROLE_BY_NAME = {'head_.Mesh': 'head', 'eyer_.Mesh': 'eye_r', 'eyel_.Mesh': 'eye_l', 'mouthbag_.Mesh': 'mouthbag', 'haircap_.Mesh': 'haircap'}
REQUIRED_ROLES = ('head', 'eye_r', 'eye_l', 'mouthbag')
PROTECTED_FIELDS = ('original_positions', 'uv', 'triangles', 'joints', 'weights', 'stride', 'topology_sha256', 'uv_ordered_sha256', 'name', 'role')
COORDINATE_SYSTEM = {'units': 'metres', 'up': '+Y', 'forward': '+Z', 'character_left': '+X', 'native_to_blender': '(x,y,z) -> (x,-z,y)', 'native_uv_to_blender': '(u,v) -> (u,1-v)',
                     'evidence': 'measured on player_67: eyel_ centre x=+0.0325, eye caps at z=+0.08, crown uv v=0.03 / neck v=0.97 (same orientation as the PS4 atlas-A exchange)'}
# Legacy facial-region palette buckets (Studio defs/appearance/facial-regions.toml, read-only; palette = joint // 3)
PALETTE_BUCKETS = {'eye_control_l': [23, 34], 'eye_control_r': [24, 35], 'upper_eyelid_l': [39], 'upper_eyelid_r': [41], 'lower_eyelid_l': [36], 'lower_eyelid_r': [37], 'upper_lip': [31, 44, 45], 'lower_lip': [17, 18, 19],
                   'mouth_corner_l': [29, 30], 'mouth_corner_r': [32, 33], 'tongue': [20, 21], 'jaw': [16], 'head': [8, 10, 15], 'neck': [4, 5, 6, 7, 14, 46, 47, 48]}
BODY_PALETTES = {0, 1, 2, 3, 4, 5, 48, 79}   # spine chain (Hips..Spine3) + RM_Left/RightSterno_End: body-side attachment (measured on player_67: 15 boundary records on 4/48/79)
NECK_PALETTES = {6, 7, 14, 46, 47}          # Neck, RM_AdamsApple, RM_Strap2, RM_NeckBack_Upper: back-of-neck records along the rear opening (not attachment)


def sha256_bytes(b):
    return hashlib.sha256(b).hexdigest()


def boundary_loops(triangles):
    """Connected components of the open-boundary edge graph (edges used by exactly one triangle). Returns
    (list of vertex-id lists, non-manifold edge count)."""
    T = np.asarray(triangles); cnt = {}
    for t in T:
        for a, b in ((t[0], t[1]), (t[1], t[2]), (t[2], t[0])):
            k = (int(min(a, b)), int(max(a, b))); cnt[k] = cnt.get(k, 0) + 1
    adj = {}
    for (a, b), c in cnt.items():
        if c == 1: adj.setdefault(a, set()).add(b); adj.setdefault(b, set()).add(a)
    seen = set(); loops = []
    for v in sorted(adj):
        if v in seen: continue
        st = [v]; comp = []
        while st:
            x = st.pop()
            if x in seen: continue
            seen.add(x); comp.append(x); st.extend(adj[x] - seen)
        loops.append(sorted(comp))
    return loops, sum(1 for c in cnt.values() if c > 2)


def connected_components(n, triangles):
    T = np.asarray(triangles); parent = list(range(n))

    def find(a):
        while parent[a] != a: parent[a] = parent[parent[a]]; a = parent[a]
        return a
    for t in T:
        for a, b in ((t[0], t[1]), (t[1], t[2])):
            ra, rb = find(int(a)), find(int(b))
            if ra != rb: parent[ra] = rb
    comps = {}
    for i in range(n): comps.setdefault(find(i), []).append(i)
    return [{'vertex_ids': v} for v in sorted(comps.values(), key=lambda v: (-len(v), v[0]))]


def palettes_of(joints, weights):
    """Set of palette indices with non-zero weight per record."""
    J = np.asarray(joints); W = np.asarray(weights)
    return [sorted({int(j) // 3 for j, w in zip(jr, wr) if w > 0}) for jr, wr in zip(J, W)]


def derive_head_structure(sub):
    """Open loops, attachment loop (boundary records weighted only on body/neck palettes, i.e. never on Head), scalp
    opening (remaining boundary records of the loop that contains the attachment) and mouth opening (other loops).
    Nothing here uses vertex ids of any other donor."""
    P = np.asarray(sub['positions'], float); T = np.asarray(sub['triangles']); pal = palettes_of(sub['joints'], sub['weights'])
    loops, nonmanifold = boundary_loops(T)
    att = sorted(i for L in loops for i in L if pal[i] and all(p in BODY_PALETTES for p in pal[i]))
    att_loop = next((L for L in loops if att and att[0] in L), [])
    rear = sorted(set(att_loop) - set(att)); neck_rear = sorted(i for i in rear if pal[i] and all(p in BODY_PALETTES | NECK_PALETTES for p in pal[i]))
    other = [L for L in loops if L is not att_loop]
    mouth = max(other, key=lambda L: float(P[L, 2].mean())) if other else []      # mouth opening = front-most (+Z) remaining loop
    dup = G.seam_groups(P)
    return {'open_loops': [{'count': len(L), 'y_min': float(P[L, 1].min()), 'y_max': float(P[L, 1].max())} for L in loops], 'nonmanifold_edges': int(nonmanifold),
            'attachment_loop': att, 'attachment_rule': 'open-boundary records whose non-zero palettes are all body-side palettes (spine chain / sterno ends; never Head or neck bucket)',
            'rear_opening': rear, 'rear_opening_rule': 'remaining records of the open loop that contains the attachment (Legacy heads are open at the back/scalp under the native haircap)', 'rear_opening_neck_weighted': neck_rear,
            'mouth_opening': mouth, 'other_loops': [L for L in other if L is not mouth], 'seam_groups': [list(map(int, g)) for g in dup], 'attachment_palettes': sorted({p for i in att for p in pal[i]})}


def eye_descriptor(sub):
    P = np.asarray(sub['positions'], float); c, r, res = sphere_fit(P); d = P - c; ax = d.mean(0); ax /= max(np.linalg.norm(ax), 1e-12)
    ang = np.degrees(np.arccos(np.clip((d @ ax) / np.maximum(np.linalg.norm(d, axis=1), 1e-12), -1, 1)))
    loops, _ = boundary_loops(sub['triangles'])
    return {'centre': c.tolist(), 'radius_m': float(r), 'rms_m': float(np.sqrt((res ** 2).mean())), 'forward_axis': ax.tolist(), 'cap_half_angle_deg': float(ang.max()), 'open_loops': len(loops), 'kind': 'spherical_cap' if loops else 'closed', 'pivot_source': 'unknown (palette matrices are bind data, not pivots)'}


def from_native_structure(entry, structure_path=None, structure_sha256=None):
    """Re-shape one decoded asset entry (native-structure-v2.json item) into the exchange. Copies every decoded
    field; adds roles, hashes, components, structure and provenance."""
    subs = []
    for s in entry['submeshes']:
        role = ROLE_BY_NAME.get(s['name'], 'unknown'); n = len(s['positions'])
        sub = {'name': s['name'], 'role': role, 'original_positions': [list(map(float, p)) for p in s['positions']], 'positions': [list(map(float, p)) for p in s['positions']], 'uv': [list(map(float, u)) for u in s['uv']],
               'triangles': [list(map(int, t)) for t in s['triangles']], 'joints': [list(map(int, j)) for j in s['joints']], 'weights': [list(map(float, w)) for w in s['weights']], 'stride': s.get('stride'),
               'topology_sha256': s.get('topology_sha256'), 'uv_ordered_sha256': s.get('uv_ordered_sha256'), 'has_blend_indices': s.get('has_blend_indices'), 'has_blend_weights': s.get('has_blend_weights'),
               'ordered_topology_sha256': X.ordered_topology_sha256(s['triangles']), 'corner_uv_sha256': X.corner_uv_sha256(s['uv'], s['triangles']), 'connected_components': connected_components(n, s['triangles'])}
        if role == 'head': sub['structure'] = derive_head_structure(sub)
        if role in ('eye_r', 'eye_l'): sub['eye'] = eye_descriptor(sub)
        subs.append(sub)
    return {'schema': SCHEMA, 'title': 'NHL Legacy Edition (Xbox 360)', 'profile_id': 'nhllegacy-xbox360-player67-v0 (proposed; Codex owns profiles)', 'coordinate_system': dict(COORDINATE_SYSTEM),
            'donor': {'path': entry.get('path'), 'sha256': entry.get('sha256'), 'bytes': entry.get('bytes')}, 'donor_sha256': entry.get('sha256'),
            'decode': {'mode': entry.get('decode_mode'), 'container_roundtrip_equal': entry.get('container_roundtrip_equal'), 'decoder': 'Studio tdb-rx2 playermesh::parse_player_mesh via experiments/codex/native_probe (Codex; read-only)',
                       'structure_file': str(structure_path) if structure_path else None, 'structure_sha256': structure_sha256},
            'palette_matrices': entry.get('palette_matrices'), 'palette_buckets': PALETTE_BUCKETS,
            'rules': ['Only positions may change. original_positions, uv, triangles, joints, weights, stride, hashes and palette_matrices are written back unchanged by the native writer.',
                      'Eyes and mouthbag move only as whole rigid components (translation; scale only when declared) and the move is reported in the handoff.',
                      'The attachment loop (body-weighted boundary records) stays exact; the scalp opening is covered by the native haircap and its clearance must be measured, not assumed.',
                      'Native normals/tangents are not carried in the decode; the writer regenerates or preserves them (Codex).', 'Vertex counts are fixed (donor topology); the writer refuses >= 65535 records.'],
            'submeshes': subs}


def submesh(ex, role):
    return next(m for m in ex['submeshes'] if m['role'] == role)


def validate(ex, expected=None):
    """Structural validation of a Legacy exchange. Returns (ok, problems, summary). `expected` may carry per-role
    {records, triangles, ordered_topology_sha256, corner_uv_sha256} and 'donor_sha256'."""
    problems = []; summary = {}
    if not isinstance(ex, dict): return False, ['exchange must be a JSON object'], summary
    if ex.get('schema') != SCHEMA: problems.append(f"schema {ex.get('schema')!r} != {SCHEMA}")
    cs = ex.get('coordinate_system') or {}
    for k, v in (('native_to_blender', '(x,y,z) -> (x,-z,y)'), ('native_uv_to_blender', '(u,v) -> (u,1-v)'), ('up', '+Y'), ('forward', '+Z'), ('character_left', '+X')):
        if cs.get(k) != v: problems.append(f'coordinate_system.{k} {cs.get(k)!r} != {v!r}')
    if not (ex.get('donor') or {}).get('sha256'): problems.append('donor.sha256 missing (provenance)')
    if not (ex.get('decode') or {}).get('decoder'): problems.append('decode.decoder missing (provenance)')
    pm = ex.get('palette_matrices')
    if not isinstance(pm, list) or not pm: problems.append('palette_matrices missing')
    subs = ex.get('submeshes') or []
    roles = [s.get('role') for s in subs]
    for r in REQUIRED_ROLES:
        if roles.count(r) != 1: problems.append(f'role {r} must appear exactly once (found {roles.count(r)})')
    for s in subs:
        name = s.get('name', '?'); n = len(s.get('positions') or [])
        for k in ('original_positions', 'positions', 'uv', 'joints', 'weights'):
            if len(s.get(k) or []) != n: problems.append(f'{name}: {k} length {len(s.get(k) or [])} != {n}')
        if n == 0: problems.append(f'{name}: no records'); continue
        P = np.asarray(s['positions'], float); T = np.asarray(s.get('triangles') or [], int)
        if not np.isfinite(P).all(): problems.append(f'{name}: non-finite positions')
        if T.ndim != 2 or T.shape[1] != 3 or len(T) == 0: problems.append(f'{name}: triangles must be an Nx3 list'); continue
        if T.min() < 0 or T.max() >= n: problems.append(f'{name}: triangle index out of range ({T.min()}..{T.max()} vs {n} records)'); continue
        U = np.asarray(s['uv'], float)
        tol = 1e-2 if s.get('role') == 'haircap' else 1e-6                      # haircap UVs tile slightly past 1 (measured 1.003 on player_67)
        if U.shape != (n, 2) or (U < -tol).any() or (U > 1 + tol).any(): problems.append(f'{name}: uv must be per-record (u,v) in [0,1]')
        J = np.asarray(s['joints']); W = np.asarray(s['weights'], float)
        if J.shape != (n, 4) or W.shape != (n, 4): problems.append(f'{name}: joints/weights must be n x 4')
        else:
            if (W < -1e-6).any() or np.abs(W.sum(1) - 1).max() > 2e-2: problems.append(f'{name}: weights must be non-negative and sum to 1 (max |sum-1| {np.abs(W.sum(1) - 1).max():.4f})')
            if (J % 3).any(): problems.append(f'{name}: joints must be shader registers (multiples of 3)')
            if pm and (J // 3 >= len(pm)).any(): problems.append(f'{name}: joint palette index beyond palette_matrices ({int((J // 3).max())} >= {len(pm)})')
        if s.get('ordered_topology_sha256') != X.ordered_topology_sha256(T): problems.append(f'{name}: ordered_topology_sha256 does not match triangles')
        if s.get('corner_uv_sha256') != X.corner_uv_sha256(U, T): problems.append(f'{name}: corner_uv_sha256 does not match uv')
        summary[name] = {'role': s.get('role'), 'records': n, 'triangles': int(len(T))}
        if s.get('role') == 'head':
            st = s.get('structure') or derive_head_structure(s)
            if not st['attachment_loop']: problems.append(f'{name}: no attachment loop (boundary records weighted only on body/neck palettes)')
            if st['nonmanifold_edges']: problems.append(f"{name}: {st['nonmanifold_edges']} non-manifold edges")
            summary[name].update({'open_loops': st['open_loops'], 'attachment_records': len(st['attachment_loop']), 'rear_opening_records': len(st['rear_opening']), 'mouth_opening_records': len(st['mouth_opening']), 'seam_groups': len(st['seam_groups'])})
        if s.get('role') in ('eye_r', 'eye_l'):
            ed = s.get('eye') or eye_descriptor(s)
            if ed['rms_m'] > 5e-4: problems.append(f"{name}: eye records are not a sphere (rms {ed['rms_m'] * 1000:.2f} mm)")
            summary[name].update({'sphere_radius_mm': ed['radius_m'] * 1000, 'kind': ed['kind']})
    if expected:
        if expected.get('donor_sha256') and ex.get('donor_sha256') != expected['donor_sha256']: problems.append('donor_sha256 != expected')
        for role, e in (expected.get('roles') or {}).items():
            s = next((m for m in subs if m.get('role') == role), None)
            if s is None: problems.append(f'expected role {role} absent'); continue
            for k in ('records', 'triangles'):
                v = len(s['positions']) if k == 'records' else len(s['triangles'])
                if e.get(k) is not None and v != e[k]: problems.append(f'{role}.{k} {v} != expected {e[k]}')
            for k in ('ordered_topology_sha256', 'corner_uv_sha256', 'topology_sha256'):
                if e.get(k) is not None and s.get(k) != e[k]: problems.append(f'{role}.{k} != expected')
    return (not problems), problems, summary


def untouched_fields_equal(cand, ref, movable_roles=('head',), rigid_roles=('eye_r', 'eye_l', 'mouthbag'), tol_m=1e-7, declared_scales=None):
    """Protected fields equal the reference; positions unchanged except for movable roles and rigid whole-component
    translations of rigid roles (returned per role). `tol_m` is the per-record tolerance around the rigid
    translation (default exact; a native-reparsed file needs the declared fp16 quantisation, e.g. 0.5e-3, and the
    caller must report which tolerance it used). `declared_scales` = {role: s} (2026-09-16 cap-radius study): for those
    rigid roles ONLY, the permitted move is the DECLARED similarity P = s * Q + t (isotropic scale s about any centre plus
    a translation; s must be a declared number, never inferred), reported as {'translation': t_of_centroid, 'scale': s}.
    Roles without a declared scale keep the historical translation-only rule, so an undeclared scale still fails.
    Returns (ok, problems, rigid_moves)."""
    problems = []; moves = {}; declared_scales = dict(declared_scales or {})
    for role, sc in declared_scales.items():
        if role not in rigid_roles: problems.append(f'declared_scale_for_non_rigid_role_{role}')
        if not isinstance(sc, (int, float)) or not np.isfinite(sc) or sc <= 0: problems.append(f'declared_scale_{role}_invalid')
    for k in ('schema', 'coordinate_system', 'donor_sha256', 'palette_matrices'):
        if cand.get(k) != ref.get(k): problems.append(f'top_level_{k}')
    if [m['name'] for m in cand['submeshes']] != [m['name'] for m in ref['submeshes']]: return False, ['submesh_names_order'], moves
    for m, v in zip(cand['submeshes'], ref['submeshes']):
        for k in PROTECTED_FIELDS:
            if m.get(k) != v.get(k): problems.append(f"{m['name']}.{k}")
        P = np.asarray(m['positions'], float); Q = np.asarray(v['positions'], float)
        if m['role'] in movable_roles: continue
        if m['role'] in rigid_roles:
            if m['role'] in declared_scales and isinstance(declared_scales[m['role']], (int, float)) and declared_scales[m['role']] > 0:
                sc = float(declared_scales[m['role']]); d = P - sc * Q; t = d.mean(0)
                if np.abs(d - t).max() > tol_m: problems.append(f"{m['name']}: positions differ from the declared similarity (scale {sc:.6f} + translation) by max {np.abs(d - t).max() * 1000:.4f} mm beyond tol {tol_m * 1000:.4f} mm")
                moves[m['role']] = {'translation': (P.mean(0) - Q.mean(0)).tolist(), 'scale': sc, 'kind': 'declared_similarity'}
            else:
                d = P - Q; t = d.mean(0)
                if np.abs(d - t).max() > tol_m: problems.append(f"{m['name']}: positions changed by more than a rigid translation (max {np.abs(d - t).max() * 1000:.4f} mm beyond tol {tol_m * 1000:.4f} mm)")
                moves[m['role']] = t.tolist()
        elif not (np.abs(P - Q).max() <= tol_m if tol_m > 1e-7 else np.array_equal(P, Q)): problems.append(f"{m['name']}.positions_moved")
    return (not problems), problems, moves


def head_arrays(ex, which='positions'):
    h = submesh(ex, 'head'); return G.native_to_blender(h[which]), np.array(h['triangles']), np.array(h['uv'])


def with_positions(ex, role, positions_blender):
    m = submesh(ex, role); m2 = dict(m); m2['positions'] = G.blender_to_native(positions_blender).tolist()
    out = dict(ex); out['submeshes'] = [m2 if x is m else x for x in ex['submeshes']]; return out


def load(path):
    return json.loads(Path(path).read_text(encoding='utf-8'))


def save(ex, path):
    p = Path(path)
    if p.exists(): raise FileExistsError(f'refusing to overwrite {p}')
    p.write_text(json.dumps(ex), encoding='utf-8'); return p
